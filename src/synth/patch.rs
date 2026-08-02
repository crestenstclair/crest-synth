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
    ///
    /// This array is the aggregate's only chain representation. No compacted
    /// or otherwise position-erasing projection is stored beside it, so every
    /// consumer reads positions exactly as they were written and a gapped
    /// chain can never be silently renumbered by a round trip.
    effects: [Option<PostEffectConfig>; MAX_EFFECT_SLOTS],
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

    /// Returns the canonical ordered effect chain, one entry per position.
    ///
    /// This is the only chain view the aggregate exposes. Slot order is render
    /// order and empty positions stay in place: clearing slot 1 leaves slot 2
    /// occupied at index 2, never compacted down.
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

    /// Occupies one validated position while constructing a Patch, leaving
    /// every other position exactly as it stands.
    ///
    /// Construction is position-explicit: the caller names the address, so a
    /// list order is never reinterpreted as a chain layout and a fixture can
    /// state a gapped chain directly. Installation validates registry identity
    /// and config; the type itself bounds the chain, so a fourth position is
    /// unrepresentable rather than refused.
    ///
    /// # Panics
    ///
    /// Panics when the occupant's instance identity already occupies another
    /// position. A duplicate identity is a construction-time programmer error;
    /// the reducer path reports it as `EffectSlotOccupancyError` instead.
    pub fn with_effect_slot(mut self, index: EffectSlotIndex, occupant: PostEffectConfig) -> Self {
        self.set_slot_occupancy(index, Some(occupant))
            .expect("a constructed occupant carries an identity unique to its Patch");
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
            let patch =
                configs
                    .iter()
                    .enumerate()
                    .fold(test_patch(), |patch, (position, occupant)| {
                        patch.with_effect_slot(slot(position), occupant.clone())
                    });
            assert_eq!(patch.effect_slots().len(), MAX_EFFECT_SLOTS);
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
    fn a_fourth_position_is_unrepresentable() {
        assert_eq!(test_patch().effect_slots().len(), MAX_EFFECT_SLOTS);
        assert_eq!(
            EffectSlotIndex::new(MAX_EFFECT_SLOTS),
            Err(
                crate::synth::effect_slot_id::EffectSlotIndexError::OutOfRange {
                    value: MAX_EFFECT_SLOTS
                }
            )
        );
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
        let mut patch = test_patch()
            .with_effect_slot(slot(0), config(1))
            .with_effect_slot(slot(1), config(2))
            .with_effect_slot(slot(2), config(3));
        patch.set_slot_occupancy(slot(1), None).unwrap();

        assert_eq!(patch.effect_slot(slot(0)), Some(&config(1)));
        assert_eq!(patch.effect_slot(slot(1)), None);
        assert_eq!(patch.effect_slot(slot(2)), Some(&config(3)));
        // The survivors keep their addresses and their instance identities:
        // nothing slid down into the hole the cleared position left.
        assert_eq!(
            patch.effect_slots()[2]
                .as_ref()
                .map(PostEffectConfig::slot_id),
            Some(EffectSlotId::new(3).unwrap())
        );
    }

    #[test]
    fn an_occupied_later_position_leaves_the_earlier_one_empty() {
        let patch = test_patch().with_effect_slot(slot(1), config(2));

        assert_eq!(patch.effect_slot(slot(0)), None);
        assert_eq!(patch.effect_slot(slot(1)), Some(&config(2)));
        assert_eq!(patch.effect_slot(slot(2)), None);
        assert_eq!(
            patch.effect_slots(),
            &[None, Some(config(2)), None],
            "an occupied second position is reported at index 1, never at index 0"
        );
    }

    #[test]
    fn instance_identities_stay_stable_across_occupancy_changes_elsewhere() {
        let mut patch = test_patch()
            .with_effect_slot(slot(0), config(1))
            .with_effect_slot(slot(1), config(2))
            .with_effect_slot(slot(2), config(3));
        let identities_before: Vec<_> = patch
            .effect_slots()
            .iter()
            .map(|occupant| occupant.as_ref().map(PostEffectConfig::slot_id))
            .collect();

        patch.set_slot_occupancy(slot(1), None).unwrap();
        patch.set_slot_occupancy(slot(1), Some(config(9))).unwrap();

        let identities_after: Vec<_> = patch
            .effect_slots()
            .iter()
            .map(|occupant| occupant.as_ref().map(PostEffectConfig::slot_id))
            .collect();
        assert_eq!(identities_before[0], identities_after[0]);
        assert_eq!(identities_after[1], Some(EffectSlotId::new(9).unwrap()));
        assert_eq!(identities_before[2], identities_after[2]);
        let occupied: Vec<_> = identities_after.iter().flatten().collect();
        let mut unique = occupied.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            occupied.len(),
            unique.len(),
            "every occupied position holds a distinct instance identity"
        );
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

    /// The fluent constructor must route through the same duplicate-identity
    /// guard as the reducer path, not write the slot array directly.
    ///
    /// Nothing else pins that routing: a constructor that assigned
    /// `self.effects[index] = Some(occupant)` would let one instance identity
    /// occupy two positions and leave every other test green, which is exactly
    /// the bypass the retired compact chain constructor allowed. The panic
    /// message is asserted so the pin fails on a silent bypass rather than on
    /// any incidental panic.
    #[test]
    #[should_panic(expected = "a constructed occupant carries an identity unique to its Patch")]
    fn with_effect_slot_refuses_one_instance_identity_at_two_positions() {
        let _duplicate = test_patch()
            .with_effect_slot(slot(0), config(7))
            .with_effect_slot(slot(2), config(7));
    }

    #[test]
    fn rerouting_the_output_leaves_the_effect_chain_untouched() {
        let mut patch = test_patch()
            .with_effect_slot(slot(0), config(1))
            .with_effect_slot(slot(1), config(2));
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
    fn replacing_a_gapped_occupant_addresses_its_own_position() {
        let mut patch = test_patch()
            .with_effect_slot(slot(0), config(1))
            .with_effect_slot(slot(2), config(3));

        let replacement = PostEffectConfig::from_parts(
            EffectSlotId::new(3).unwrap(),
            EffectCapabilityId::new("effect.other").unwrap(),
            Vec::new(),
            Vec::new(),
        );
        patch
            .set_slot_occupancy(slot(2), Some(replacement.clone()))
            .unwrap();

        assert_eq!(patch.effect_slot(slot(2)), Some(&replacement));
        assert_eq!(patch.effect_slot(slot(1)), None);
        assert_eq!(patch.effect_slot(slot(0)), Some(&config(1)));
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
        let _: for<'a> fn(&'a Patch) -> &'a [Option<PostEffectConfig>; MAX_EFFECT_SLOTS] =
            Patch::effect_slots;
        let _: fn(Patch, EffectSlotIndex, PostEffectConfig) -> Patch = Patch::with_effect_slot;

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
