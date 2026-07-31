use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::patch_id::PatchId;
use crate::mixer::patch_output::PatchOutput;
use crate::synth::effect_slot_id::{EffectSlotIndex, MAX_EFFECT_SLOTS};
use crate::synth::instrument_capability::{
    CapabilityDescriptor, CapabilityError, InstrumentConfig, ParameterUpdate,
};
use crate::synth::parameter_id::ParameterId;
use crate::synth::voice_envelope::{VoiceEnvelope, VoiceEnvelopeParameter};
use crate::synth::{EffectSlotId, PostEffectConfig};
use core::fmt;
use serde::{Deserialize, Serialize};

/// One entry in the canonical schema-derived editable Patch surface.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "parameter", rename_all = "camelCase")]
pub enum PatchEditableTarget {
    Envelope(VoiceEnvelopeParameter),
    Instrument(ParameterId),
}

impl PatchEditableTarget {
    /// Returns the stable semantic identifier used by projection and coverage.
    pub fn name(&self) -> &str {
        match self {
            Self::Envelope(parameter) => parameter.name(),
            Self::Instrument(parameter) => parameter.as_str(),
        }
    }
}

/// Resolves the only editable Patch ordering from immutable schema and config.
pub fn resolve_patch_editable_targets(
    descriptor: &CapabilityDescriptor,
    config: &InstrumentConfig,
) -> Result<Vec<PatchEditableTarget>, CapabilityError> {
    if descriptor.id() != config.capability_id() {
        return Err(CapabilityError::ProviderRegistryMismatch(
            config.capability_id().clone(),
        ));
    }
    let canonical = descriptor.create_config(config.values(), config.asset_references())?;
    if canonical != *config {
        return Err(CapabilityError::ConfigOrderMismatch(
            config.capability_id().clone(),
        ));
    }

    let mut targets = Vec::with_capacity(
        VoiceEnvelope::surface_descriptor().len() + descriptor.scalar_parameter_count(),
    );
    targets.extend(
        VoiceEnvelope::surface_descriptor()
            .iter()
            .map(|descriptor| PatchEditableTarget::Envelope(descriptor.parameter())),
    );
    targets.extend(
        descriptor
            .parameters()
            .filter(|parameter| parameter.update() == ParameterUpdate::Scalar)
            .map(|parameter| PatchEditableTarget::Instrument(parameter.id().clone())),
    );
    Ok(targets)
}

/// One installed, playable instrument capability configuration.
///
/// A patch's identity, display name, instrument, and assigned MIDI channel are
/// fixed at construction time. Output, envelope, and descriptor-classified
/// values can change only through the canonical reducer.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    id: PatchId,
    name: String,
    instrument: InstrumentConfig,
    channel: MidiChannel,
    envelope: VoiceEnvelope,
    output: PatchOutput,
    /// The canonical bounded effect chain: exactly `MAX_EFFECT_SLOTS` ordered
    /// positions, each independently empty or occupied. Slot order is render
    /// order, positions are stable addresses, and a fourth effect is
    /// unrepresentable in the type.
    effects: [Option<PostEffectConfig>; MAX_EFFECT_SLOTS],
    /// Derived occupied-slots-in-index-order projection of `effects`.
    ///
    /// Transitional: it exists only so `post_effects()` can keep returning a
    /// slice to the legacy zero-or-one callers (snapshot projection,
    /// persistence, demos) until WP05/WP06 migrate them to `effect_slots()`.
    /// It is rebuilt after every mutation and never carries independent state.
    effects_compact: Vec<PostEffectConfig>,
}

impl Patch {
    /// Creates an installed patch from validated domain values.
    pub fn new(
        id: PatchId,
        name: String,
        instrument: InstrumentConfig,
        channel: MidiChannel,
        output: PatchOutput,
    ) -> Self {
        Self {
            id,
            name,
            instrument,
            channel,
            envelope: VoiceEnvelope::default(),
            output,
            effects: std::array::from_fn(|_| None),
            effects_compact: Vec::new(),
        }
    }

    /// Returns this patch's stable process-lifetime identity.
    pub const fn id(&self) -> PatchId {
        self.id
    }

    /// Returns the immutable display name assigned at installation.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the current generic instrument configuration.
    pub const fn instrument_config(&self) -> &InstrumentConfig {
        &self.instrument
    }

    /// Returns the immutable MIDI channel assigned by the input adapter.
    pub const fn channel(&self) -> MidiChannel {
        self.channel
    }

    /// Returns the canonical Patch-owned ADSR value.
    pub const fn envelope(&self) -> &VoiceEnvelope {
        &self.envelope
    }

    /// Returns this Patch's validated pre-track trim and destination.
    pub const fn output(&self) -> PatchOutput {
        self.output
    }

    /// Returns the occupied post-effect configurations in slot index order.
    ///
    /// Transitional compact view for legacy zero-or-one callers; it erases
    /// empty positions. Position-aware code must use `effect_slots()`.
    pub fn post_effects(&self) -> &[PostEffectConfig] {
        &self.effects_compact
    }

    /// Returns the canonical ordered effect chain, one entry per position.
    ///
    /// Slot order is render order. Empty positions stay in place: clearing
    /// slot 1 leaves slot 2 occupied at index 2, never compacted down.
    pub const fn effect_slots(&self) -> &[Option<PostEffectConfig>; MAX_EFFECT_SLOTS] {
        &self.effects
    }

    /// Returns the configuration occupying one validated position, if any.
    pub fn effect_slot(&self, index: EffectSlotIndex) -> Option<&PostEffectConfig> {
        self.effects[index.index()].as_ref()
    }

    /// Replaces the complete bounded Patch output value through the reducer.
    pub(crate) fn set_output(&mut self, output: PatchOutput) {
        self.output = output;
    }

    /// Supplies a non-default envelope while constructing a Patch fixture.
    pub fn with_envelope(mut self, envelope: VoiceEnvelope) -> Self {
        self.envelope = envelope;
        self
    }

    /// Supplies the ordered post-effect list while constructing a Patch,
    /// occupying positions `0..len` in order. Installation validates registry
    /// identity and config; the type itself bounds the chain, so more than
    /// `MAX_EFFECT_SLOTS` entries is a construction-time programmer error.
    ///
    /// # Panics
    ///
    /// Panics when given more than `MAX_EFFECT_SLOTS` configurations; a
    /// fourth effect is unrepresentable in the slot array.
    pub fn with_post_effects(mut self, post_effects: Vec<PostEffectConfig>) -> Self {
        assert!(
            post_effects.len() <= MAX_EFFECT_SLOTS,
            "a Patch holds at most {MAX_EFFECT_SLOTS} ordered effect slots"
        );
        self.effects = std::array::from_fn(|_| None);
        for (position, config) in post_effects.into_iter().enumerate() {
            self.effects[position] = Some(config);
        }
        self.rebuild_compact_view();
        self
    }

    /// Applies the structural `SetSlotOccupancy` domain transition: occupies,
    /// replaces, or clears exactly one validated position, leaving every other
    /// position untouched. Occupancy changes what exists, so it travels the
    /// prepared-structural-change path, never the scalar snapshot.
    ///
    /// The occupant carries its own stable `EffectSlotId` instance identity;
    /// an identity already occupying another position is rejected, never
    /// silently replaced.
    pub(crate) fn set_slot_occupancy(
        &mut self,
        index: EffectSlotIndex,
        occupant: Option<PostEffectConfig>,
    ) -> Result<(), EffectSlotOccupancyError> {
        if let Some(config) = &occupant {
            let duplicate = self
                .effects
                .iter()
                .enumerate()
                .filter(|(position, _)| *position != index.index())
                .filter_map(|(_, slot)| slot.as_ref())
                .any(|other| other.slot_id() == config.slot_id());
            if duplicate {
                return Err(EffectSlotOccupancyError::DuplicateSlotId(config.slot_id()));
            }
        }
        self.effects[index.index()] = occupant;
        self.rebuild_compact_view();
        Ok(())
    }

    /// Resolves this Patch's common ADSR and live instrument targets.
    pub fn editable_targets(
        &self,
        descriptor: &CapabilityDescriptor,
    ) -> Result<Vec<PatchEditableTarget>, CapabilityError> {
        resolve_patch_editable_targets(descriptor, &self.instrument)
    }

    pub(crate) fn set_envelope(&mut self, envelope: VoiceEnvelope) {
        self.envelope = envelope;
    }

    pub(crate) fn set_instrument_config(&mut self, config: InstrumentConfig) {
        self.instrument = config;
    }

    /// Replaces the configuration at one compact-view index (the nth occupied
    /// slot in position order), preserving its position. Transitional peer of
    /// `post_effects()` for the legacy scalar-edit path.
    pub(crate) fn set_post_effect_config(&mut self, index: usize, config: PostEffectConfig) {
        let position = self
            .effects
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.is_some())
            .map(|(position, _)| position)
            .nth(index)
            .expect("compact post-effect index addresses an occupied slot");
        self.effects[position] = Some(config);
        self.rebuild_compact_view();
    }

    fn rebuild_compact_view(&mut self) {
        self.effects_compact = self.effects.iter().flatten().cloned().collect();
    }
}

/// The reason a `SetSlotOccupancy` transition was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectSlotOccupancyError {
    /// The occupant's instance identity already occupies another position.
    DuplicateSlotId(EffectSlotId),
}

impl fmt::Display for EffectSlotOccupancyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSlotId(slot_id) => {
                write!(
                    formatter,
                    "effect instance {slot_id} already occupies another slot"
                )
            }
        }
    }
}

impl std::error::Error for EffectSlotOccupancyError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::synth::capability_id::CapabilityId;
    use crate::synth::EffectCapabilityId;

    fn test_patch() -> Patch {
        Patch::new(
            PatchId::new(1).unwrap(),
            "Test".to_owned(),
            InstrumentConfig::from_parts(
                CapabilityId::new("instrument.test").unwrap(),
                Vec::new(),
                Vec::new(),
            ),
            MidiChannel::new(0).unwrap(),
            PatchOutput::default(),
        )
    }

    fn config(slot_id: u16) -> PostEffectConfig {
        PostEffectConfig::from_parts(
            EffectSlotId::new(slot_id).unwrap(),
            EffectCapabilityId::new("effect.test").unwrap(),
            Vec::new(),
            Vec::new(),
        )
    }

    fn slot(index: usize) -> EffectSlotIndex {
        EffectSlotIndex::new(index).unwrap()
    }

    #[test]
    fn patch_holds_zero_through_three_ordered_effects() {
        for count in 0..=MAX_EFFECT_SLOTS {
            let configs: Vec<_> = (0..count).map(|index| config(index as u16 + 1)).collect();
            let patch = test_patch().with_post_effects(configs.clone());
            assert_eq!(patch.effect_slots().len(), MAX_EFFECT_SLOTS);
            assert_eq!(patch.post_effects(), configs.as_slice());
            for position in 0..MAX_EFFECT_SLOTS {
                assert_eq!(
                    patch.effect_slot(slot(position)),
                    configs.get(position),
                    "position {position} with {count} installed"
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "at most 3 ordered effect slots")]
    fn a_fourth_effect_is_refused_at_construction() {
        let configs = (1..=4).map(config).collect();
        let _ = test_patch().with_post_effects(configs);
    }

    #[test]
    fn each_position_sets_replaces_and_clears_independently() {
        for position in 0..MAX_EFFECT_SLOTS {
            let mut patch = test_patch();
            for occupied in 0..MAX_EFFECT_SLOTS {
                patch
                    .set_slot_occupancy(slot(occupied), Some(config(occupied as u16 + 1)))
                    .unwrap();
            }

            let replacement = PostEffectConfig::from_parts(
                EffectSlotId::new(position as u16 + 1).unwrap(),
                EffectCapabilityId::new("effect.other").unwrap(),
                Vec::new(),
                Vec::new(),
            );
            patch
                .set_slot_occupancy(slot(position), Some(replacement.clone()))
                .unwrap();
            assert_eq!(patch.effect_slot(slot(position)), Some(&replacement));

            patch.set_slot_occupancy(slot(position), None).unwrap();
            assert_eq!(patch.effect_slot(slot(position)), None);
            for other in (0..MAX_EFFECT_SLOTS).filter(|other| *other != position) {
                assert_eq!(
                    patch.effect_slot(slot(other)),
                    Some(&config(other as u16 + 1)),
                    "clearing position {position} must preserve position {other}"
                );
            }
        }
    }

    #[test]
    fn clearing_a_slot_never_compacts_the_others() {
        let mut patch = test_patch().with_post_effects(vec![config(1), config(2), config(3)]);
        patch.set_slot_occupancy(slot(1), None).unwrap();

        assert_eq!(patch.effect_slot(slot(0)), Some(&config(1)));
        assert_eq!(patch.effect_slot(slot(1)), None);
        assert_eq!(patch.effect_slot(slot(2)), Some(&config(3)));
        assert_eq!(patch.post_effects(), &[config(1), config(3)]);
    }

    #[test]
    fn duplicate_instance_identity_is_rejected_not_replaced() {
        let mut patch = test_patch();
        patch.set_slot_occupancy(slot(0), Some(config(7))).unwrap();

        assert_eq!(
            patch.set_slot_occupancy(slot(2), Some(config(7))),
            Err(EffectSlotOccupancyError::DuplicateSlotId(
                EffectSlotId::new(7).unwrap()
            ))
        );
        assert_eq!(patch.effect_slot(slot(2)), None);
        assert_eq!(patch.effect_slot(slot(0)), Some(&config(7)));

        // Replacing an instance in place with its own identity stays valid.
        patch.set_slot_occupancy(slot(0), Some(config(7))).unwrap();
        assert_eq!(patch.effect_slot(slot(0)), Some(&config(7)));
    }

    #[test]
    fn rerouting_the_output_leaves_the_effect_chain_untouched() {
        let mut patch = test_patch().with_post_effects(vec![config(1), config(2)]);
        patch.set_slot_occupancy(slot(1), None).unwrap();
        let chain_before = patch.effect_slots().clone();

        patch.set_output(PatchOutput::new(MixerTrackId::new(9).unwrap(), -4.5).unwrap());

        assert_eq!(patch.effect_slots(), &chain_before);
        assert_eq!(
            patch.output(),
            PatchOutput::new(MixerTrackId::new(9).unwrap(), -4.5).unwrap()
        );
    }

    #[test]
    fn compact_index_updates_address_the_nth_occupied_position() {
        let mut patch = test_patch();
        patch.set_slot_occupancy(slot(0), Some(config(1))).unwrap();
        patch.set_slot_occupancy(slot(2), Some(config(3))).unwrap();

        let replacement = PostEffectConfig::from_parts(
            EffectSlotId::new(3).unwrap(),
            EffectCapabilityId::new("effect.other").unwrap(),
            Vec::new(),
            Vec::new(),
        );
        patch.set_post_effect_config(1, replacement.clone());

        assert_eq!(patch.effect_slot(slot(2)), Some(&replacement));
        assert_eq!(patch.effect_slot(slot(1)), None);
        assert_eq!(patch.effect_slot(slot(0)), Some(&config(1)));
        assert_eq!(patch.post_effects(), &[config(1), replacement]);
    }

    #[test]
    fn public_api_keeps_configuration_read_only() {
        let _: fn(PatchId, String, InstrumentConfig, MidiChannel, PatchOutput) -> Patch =
            Patch::new;
        let _: fn(&Patch) -> PatchId = Patch::id;
        let _: for<'a> fn(&'a Patch) -> &'a str = Patch::name;
        let _: for<'a> fn(&'a Patch) -> &'a InstrumentConfig = Patch::instrument_config;
        let _: fn(&Patch) -> MidiChannel = Patch::channel;
        let _: for<'a> fn(&'a Patch) -> &'a VoiceEnvelope = Patch::envelope;
        let _: fn(&Patch) -> PatchOutput = Patch::output;

        let config = InstrumentConfig::from_parts(
            CapabilityId::new("instrument.test").unwrap(),
            Vec::new(),
            Vec::new(),
        );
        let patch = Patch::new(
            PatchId::new(1).unwrap(),
            "Test".to_owned(),
            config.clone(),
            MidiChannel::new(0).unwrap(),
            PatchOutput::default(),
        );
        assert_eq!(patch.instrument_config(), &config);
        assert_eq!(patch.envelope(), &VoiceEnvelope::default());
    }
}
