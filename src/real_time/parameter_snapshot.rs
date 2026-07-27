use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::graph_revision::GraphRevision;
use crate::synth::instrument_capability::{CapabilityRegistry, MAX_INSTRUMENT_SCALAR_PARAMETERS};
use crate::synth::patch::Patch;
use crate::synth::voice_envelope::VoiceEnvelope;
use crate::synth::{EffectCapabilityRegistry, EffectSlotId, MAX_EFFECT_SCALAR_PARAMETERS};
use core::fmt;
use serde::{Serialize, Serializer};

/// The maximum number of Patch parameter values carried across the real-time
/// boundary.
///
/// SoundFont playback is addressed through MIDI's sixteen bounded channels, so
/// the callback never needs dynamically sized Patch storage.
pub const MAX_PATCHES: usize = 16;

/// Fixed descriptor-ordered live instrument values for one Patch.
///
/// Choice values are encoded as descriptor indices, toggles as 0/1, and
/// numeric values directly. Unused storage is always zeroed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RtInstrumentParameters {
    count: u8,
    values: [f32; MAX_INSTRUMENT_SCALAR_PARAMETERS],
}

impl RtInstrumentParameters {
    pub const EMPTY: Self = Self {
        count: 0,
        values: [0.0; MAX_INSTRUMENT_SCALAR_PARAMETERS],
    };

    /// Copies a complete descriptor-ordered scalar prefix into fixed storage.
    pub fn new(values: &[f32]) -> Result<Self, ParameterSnapshotError> {
        if values.len() > MAX_INSTRUMENT_SCALAR_PARAMETERS {
            return Err(ParameterSnapshotError::TooManyInstrumentScalars {
                count: values.len(),
                capacity: MAX_INSTRUMENT_SCALAR_PARAMETERS,
            });
        }
        if let Some(index) = values.iter().position(|value| !value.is_finite()) {
            return Err(ParameterSnapshotError::NonFiniteInstrumentScalar { index });
        }
        let mut storage = [0.0; MAX_INSTRUMENT_SCALAR_PARAMETERS];
        storage[..values.len()].copy_from_slice(values);
        Ok(Self {
            count: values.len() as u8,
            values: storage,
        })
    }

    pub const fn count(&self) -> usize {
        self.count as usize
    }

    pub fn values(&self) -> &[f32] {
        &self.values[..self.count()]
    }

    pub fn value(&self, index: usize) -> Option<f32> {
        self.values().get(index).copied()
    }

    pub const fn storage(&self) -> &[f32; MAX_INSTRUMENT_SCALAR_PARAMETERS] {
        &self.values
    }
}

impl Serialize for RtInstrumentParameters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct SerializableInstrumentParameters<'a> {
            count: usize,
            values: &'a [f32],
        }

        SerializableInstrumentParameters {
            count: self.count(),
            values: self.values(),
        }
        .serialize(serializer)
    }
}

impl Default for RtInstrumentParameters {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Fixed descriptor-ordered live values for one optional Patch post-effect slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RtPostEffectParameters {
    slot_id: Option<EffectSlotId>,
    scalar_count: u8,
    scalars: [f32; MAX_EFFECT_SCALAR_PARAMETERS],
}

impl RtPostEffectParameters {
    pub const EMPTY: Self = Self {
        slot_id: None,
        scalar_count: 0,
        scalars: [0.0; MAX_EFFECT_SCALAR_PARAMETERS],
    };

    pub fn new(slot_id: EffectSlotId, scalars: &[f32]) -> Result<Self, ParameterSnapshotError> {
        if scalars.len() > MAX_EFFECT_SCALAR_PARAMETERS {
            return Err(ParameterSnapshotError::TooManyEffectScalars {
                count: scalars.len(),
                capacity: MAX_EFFECT_SCALAR_PARAMETERS,
            });
        }
        if let Some(index) = scalars.iter().position(|value| !value.is_finite()) {
            return Err(ParameterSnapshotError::NonFiniteEffectScalar { index });
        }
        let mut storage = [0.0; MAX_EFFECT_SCALAR_PARAMETERS];
        storage[..scalars.len()].copy_from_slice(scalars);
        Ok(Self {
            slot_id: Some(slot_id),
            scalar_count: scalars.len() as u8,
            scalars: storage,
        })
    }

    pub const fn is_active(&self) -> bool {
        self.slot_id.is_some()
    }

    pub const fn slot_id(&self) -> Option<EffectSlotId> {
        self.slot_id
    }

    pub const fn scalar_count(&self) -> usize {
        self.scalar_count as usize
    }

    pub fn scalars(&self) -> &[f32] {
        &self.scalars[..self.scalar_count()]
    }

    pub fn scalar(&self, index: usize) -> Option<f32> {
        self.scalars().get(index).copied()
    }

    pub const fn storage(&self) -> &[f32; MAX_EFFECT_SCALAR_PARAMETERS] {
        &self.scalars
    }
}

impl Default for RtPostEffectParameters {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Serialize for RtPostEffectParameters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializableEffectParameters<'a> {
            active: bool,
            slot_id: Option<EffectSlotId>,
            scalar_count: usize,
            scalars: &'a [f32],
        }
        SerializableEffectParameters {
            active: self.is_active(),
            slot_id: self.slot_id(),
            scalar_count: self.scalar_count(),
            scalars: self.scalars(),
        }
        .serialize(serializer)
    }
}

/// The fixed-size audio parameters for one active Patch.
///
/// The value is copyable and owns no heap storage. An absent Patch identity is
/// the canonical inactive value used for unused ParameterSnapshot entries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RtPatchParameters {
    patch_id: Option<PatchId>,
    parameters: ChannelParameters,
    envelope: VoiceEnvelope,
    instrument: RtInstrumentParameters,
    effect: RtPostEffectParameters,
}

impl RtPatchParameters {
    /// Copies one active Patch's identity and validated mixer parameters into a
    /// real-time-safe value.
    pub const fn new(patch_id: PatchId, parameters: ChannelParameters) -> Self {
        Self {
            patch_id: Some(patch_id),
            parameters,
            envelope: VoiceEnvelope::DEFAULT,
            instrument: RtInstrumentParameters::EMPTY,
            effect: RtPostEffectParameters::EMPTY,
        }
    }

    /// Copies the full live projection for one active Patch.
    pub const fn projected(
        patch_id: PatchId,
        parameters: ChannelParameters,
        envelope: VoiceEnvelope,
        instrument: RtInstrumentParameters,
    ) -> Self {
        Self {
            patch_id: Some(patch_id),
            parameters,
            envelope,
            instrument,
            effect: RtPostEffectParameters::EMPTY,
        }
    }

    /// Copies the full live projection including one optional post-effect slot.
    pub const fn projected_with_effect(
        patch_id: PatchId,
        parameters: ChannelParameters,
        envelope: VoiceEnvelope,
        instrument: RtInstrumentParameters,
        effect: RtPostEffectParameters,
    ) -> Self {
        Self {
            patch_id: Some(patch_id),
            parameters,
            envelope,
            instrument,
            effect,
        }
    }

    /// Returns whether this entry contains one active Patch.
    pub const fn is_active(&self) -> bool {
        self.patch_id.is_some()
    }

    /// Returns the active Patch identity, or None for unused storage.
    pub const fn patch_id(&self) -> Option<PatchId> {
        self.patch_id
    }

    /// Returns the Patch's copied, validated channel parameters.
    pub const fn parameters(&self) -> &ChannelParameters {
        &self.parameters
    }

    pub const fn envelope(&self) -> &VoiceEnvelope {
        &self.envelope
    }

    pub const fn instrument(&self) -> &RtInstrumentParameters {
        &self.instrument
    }

    pub const fn effect(&self) -> &RtPostEffectParameters {
        &self.effect
    }

    fn inactive() -> Self {
        Self {
            patch_id: None,
            parameters: ChannelParameters::default(),
            envelope: VoiceEnvelope::DEFAULT,
            instrument: RtInstrumentParameters::EMPTY,
            effect: RtPostEffectParameters::EMPTY,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializableChannelParameters {
    gain_db: f32,
    pan: f32,
    reverb_send: f32,
    delay_send: f32,
}

impl From<&ChannelParameters> for SerializableChannelParameters {
    fn from(parameters: &ChannelParameters) -> Self {
        Self {
            gain_db: parameters.gain_db(),
            pan: parameters.pan(),
            reverb_send: parameters.reverb_send(),
            delay_send: parameters.delay_send(),
        }
    }
}

impl Serialize for RtPatchParameters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializablePatchParameters<'a> {
            patch_id: Option<PatchId>,
            envelope: &'a VoiceEnvelope,
            instrument: &'a RtInstrumentParameters,
            effect: &'a RtPostEffectParameters,
            parameters: SerializableChannelParameters,
        }

        SerializablePatchParameters {
            patch_id: self.patch_id(),
            envelope: self.envelope(),
            instrument: self.instrument(),
            effect: self.effect(),
            parameters: SerializableChannelParameters::from(self.parameters()),
        }
        .serialize(serializer)
    }
}

/// The reason a complete real-time parameter snapshot could not be built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterSnapshotError {
    /// The control state contains more Patch values than the fixed capacity.
    TooManyPatches { count: usize, capacity: usize },
    /// An inactive value was supplied inside the active Patch prefix.
    InactivePatch { index: usize },
    /// A descriptor exceeded the fixed live instrument scalar capacity.
    TooManyInstrumentScalars { count: usize, capacity: usize },
    /// A scalar could not be represented as a finite real-time value.
    NonFiniteInstrumentScalar { index: usize },
    /// An effect descriptor exceeded the fixed live effect scalar capacity.
    TooManyEffectScalars { count: usize, capacity: usize },
    /// An effect scalar could not be represented as a finite real-time value.
    NonFiniteEffectScalar { index: usize },
    /// A Patch config did not resolve through the immutable registry.
    InvalidInstrumentConfig { index: usize },
    /// A Patch effect config did not resolve through the immutable effect registry.
    InvalidEffectConfig { index: usize },
}

impl fmt::Display for ParameterSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooManyPatches { count, capacity } => write!(
                formatter,
                "parameter snapshot has {count} patches; maximum is {capacity}"
            ),
            Self::InactivePatch { index } => {
                write!(formatter, "parameter snapshot patch {index} is inactive")
            }
            Self::TooManyInstrumentScalars { count, capacity } => write!(
                formatter,
                "instrument projection has {count} Scalars; maximum is {capacity}"
            ),
            Self::NonFiniteInstrumentScalar { index } => {
                write!(formatter, "instrument Scalar {index} is not finite")
            }
            Self::TooManyEffectScalars { count, capacity } => write!(
                formatter,
                "effect projection has {count} Scalars; maximum is {capacity}"
            ),
            Self::NonFiniteEffectScalar { index } => {
                write!(formatter, "effect Scalar {index} is not finite")
            }
            Self::InvalidInstrumentConfig { index } => {
                write!(formatter, "Patch {index} has an invalid instrument config")
            }
            Self::InvalidEffectConfig { index } => {
                write!(formatter, "Patch {index} has an invalid effect config")
            }
        }
    }
}

impl std::error::Error for ParameterSnapshotError {}

/// The newest complete control state required for rendering.
///
/// Every field is fully owned, fixed-size, and copyable. Audio-thread readers
/// can therefore consume one coherent value without allocation, locking,
/// blocking, I/O, logging, or destruction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterSnapshot {
    generation: u64,
    graph_revision: GraphRevision,
    global: GlobalParameters,
    patch_count: usize,
    patches: [RtPatchParameters; MAX_PATCHES],
}

impl ParameterSnapshot {
    pub const SERIALIZED_LEAF_DESCRIPTOR: &'static [&'static str] = &[
        "generation",
        "graphRevision",
        "patchCount",
        "patches[].patchId",
        "patches[].envelope.attackMilliseconds",
        "patches[].envelope.decayMilliseconds",
        "patches[].envelope.sustain",
        "patches[].envelope.releaseMilliseconds",
        "patches[].instrument.count",
        "patches[].instrument.values[]",
        "patches[].effect.active",
        "patches[].effect.slotId",
        "patches[].effect.scalarCount",
        "patches[].effect.scalars[]",
        "patches[].parameters.gainDb",
        "patches[].parameters.pan",
        "patches[].parameters.reverbSend",
        "patches[].parameters.delaySend",
        "global.masterGainDb",
        "global.reverbRoomSize",
        "global.reverbDamping",
        "global.reverbReturn",
        "global.delayMilliseconds",
        "global.delayFeedback",
        "global.delayReturn",
    ];

    /// Returns the canonical control-side serialization surface mirrored in StateTree.
    pub const fn serialized_leaf_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_LEAF_DESCRIPTOR
    }

    /// Copies a complete accepted control projection into bounded storage.
    ///
    /// Patch values in the input slice must all be active. Remaining array
    /// entries are initialized to the canonical inactive value.
    pub fn new(
        generation: u64,
        global: GlobalParameters,
        patches: &[RtPatchParameters],
    ) -> Result<Self, ParameterSnapshotError> {
        Self::for_graph(generation, GraphRevision::INITIAL, global, patches)
    }

    /// Copies one complete projection for a specific prepared graph revision.
    pub fn for_graph(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        patches: &[RtPatchParameters],
    ) -> Result<Self, ParameterSnapshotError> {
        if patches.len() > MAX_PATCHES {
            return Err(ParameterSnapshotError::TooManyPatches {
                count: patches.len(),
                capacity: MAX_PATCHES,
            });
        }
        if let Some(index) = patches.iter().position(|patch| !patch.is_active()) {
            return Err(ParameterSnapshotError::InactivePatch { index });
        }

        let mut storage = [RtPatchParameters::inactive(); MAX_PATCHES];
        storage[..patches.len()].copy_from_slice(patches);

        Ok(Self {
            generation,
            graph_revision,
            global,
            patch_count: patches.len(),
            patches: storage,
        })
    }

    /// Projects the canonical Patch/config values into the one fixed real-time shape.
    ///
    /// Control state projection and off-callback graph preparation share this
    /// implementation so descriptor ordering and Scalar encoding cannot drift.
    pub fn project_patches(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        patches: &[Patch],
        registry: &CapabilityRegistry,
    ) -> Result<Self, ParameterSnapshotError> {
        if patches.len() > MAX_PATCHES {
            return Err(ParameterSnapshotError::TooManyPatches {
                count: patches.len(),
                capacity: MAX_PATCHES,
            });
        }

        let projected = patches
            .iter()
            .enumerate()
            .map(|(index, patch)| {
                registry
                    .validate_config(patch.instrument_config())
                    .map_err(|_| ParameterSnapshotError::InvalidInstrumentConfig { index })?;
                let descriptor = registry
                    .descriptor(patch.instrument_config().capability_id())
                    .ok_or(ParameterSnapshotError::InvalidInstrumentConfig { index })?;
                let values = descriptor
                    .scalar_parameters()
                    .map(|spec| {
                        let value = patch
                            .instrument_config()
                            .value(spec.id())
                            .ok_or(ParameterSnapshotError::InvalidInstrumentConfig { index })?;
                        spec.scalar_value(value)
                            .map_err(|_| ParameterSnapshotError::InvalidInstrumentConfig { index })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let instrument = RtInstrumentParameters::new(&values)?;
                Ok(RtPatchParameters::projected(
                    patch.id(),
                    *patch.parameters(),
                    *patch.envelope(),
                    instrument,
                ))
            })
            .collect::<Result<Vec<_>, ParameterSnapshotError>>()?;

        Self::for_graph(generation, graph_revision, global, &projected)
    }

    /// Projects instrument and post-effect values into their separate fixed layouts.
    pub fn project_patches_with_effects(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        patches: &[Patch],
        registry: &CapabilityRegistry,
        effect_registry: &EffectCapabilityRegistry,
    ) -> Result<Self, ParameterSnapshotError> {
        if patches.len() > MAX_PATCHES {
            return Err(ParameterSnapshotError::TooManyPatches {
                count: patches.len(),
                capacity: MAX_PATCHES,
            });
        }
        let projected = patches
            .iter()
            .enumerate()
            .map(|(index, patch)| {
                registry
                    .validate_config(patch.instrument_config())
                    .map_err(|_| ParameterSnapshotError::InvalidInstrumentConfig { index })?;
                let descriptor = registry
                    .descriptor(patch.instrument_config().capability_id())
                    .ok_or(ParameterSnapshotError::InvalidInstrumentConfig { index })?;
                let instrument_values = descriptor
                    .scalar_parameters()
                    .map(|spec| {
                        let value = patch
                            .instrument_config()
                            .value(spec.id())
                            .ok_or(ParameterSnapshotError::InvalidInstrumentConfig { index })?;
                        spec.scalar_value(value)
                            .map_err(|_| ParameterSnapshotError::InvalidInstrumentConfig { index })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let instrument = RtInstrumentParameters::new(&instrument_values)?;
                effect_registry
                    .validate_patch_effects(patch.post_effects())
                    .map_err(|_| ParameterSnapshotError::InvalidEffectConfig { index })?;
                let effect = match patch.post_effects().first() {
                    None => RtPostEffectParameters::EMPTY,
                    Some(config) => {
                        let effect_descriptor = effect_registry
                            .descriptor(config.capability_id())
                            .ok_or(ParameterSnapshotError::InvalidEffectConfig { index })?;
                        let effect_values = effect_descriptor
                            .scalar_parameters()
                            .map(|spec| {
                                let value = config
                                    .value(spec.id())
                                    .ok_or(ParameterSnapshotError::InvalidEffectConfig { index })?;
                                spec.scalar_value(value).map_err(|_| {
                                    ParameterSnapshotError::InvalidEffectConfig { index }
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        RtPostEffectParameters::new(config.slot_id(), &effect_values)?
                    }
                };
                Ok(RtPatchParameters::projected_with_effect(
                    patch.id(),
                    *patch.parameters(),
                    *patch.envelope(),
                    instrument,
                    effect,
                ))
            })
            .collect::<Result<Vec<_>, ParameterSnapshotError>>()?;
        Self::for_graph(generation, graph_revision, global, &projected)
    }

    /// Returns the AppState generation from which this snapshot was projected.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the prepared graph revision targeted by this snapshot.
    pub const fn graph_revision(&self) -> GraphRevision {
        self.graph_revision
    }

    /// Returns the copied parameters for the one shared global mix.
    pub const fn global(&self) -> &GlobalParameters {
        &self.global
    }

    /// Returns the number of active entries in the fixed Patch array.
    pub const fn patch_count(&self) -> usize {
        self.patch_count
    }

    /// Returns exactly the active Patch parameter prefix.
    pub fn patches(&self) -> &[RtPatchParameters] {
        &self.patches[..self.patch_count]
    }

    /// Returns the complete fixed storage, including inactive entries.
    pub const fn storage(&self) -> &[RtPatchParameters; MAX_PATCHES] {
        &self.patches
    }

    /// Finds one active Patch without allocation.
    pub fn patch(&self, patch_id: PatchId) -> Option<&RtPatchParameters> {
        self.patches()
            .iter()
            .find(|patch| patch.patch_id() == Some(patch_id))
    }

    /// Returns whether this complete snapshot targets an exact graph revision
    /// and ordered Patch layout.
    pub fn is_compatible(
        &self,
        graph_revision: GraphRevision,
        ordered_patch_ids: &[PatchId],
    ) -> bool {
        self.graph_revision == graph_revision
            && self.patch_count == ordered_patch_ids.len()
            && self
                .patches()
                .iter()
                .zip(ordered_patch_ids)
                .all(|(patch, patch_id)| patch.patch_id() == Some(*patch_id))
    }

    /// Reuses identical bounded parameter values for a MIDI-only generation.
    pub(crate) const fn with_generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }
}

impl Serialize for ParameterSnapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializableGlobalParameters {
            master_gain_db: f32,
            reverb_room_size: f32,
            reverb_damping: f32,
            reverb_return: f32,
            delay_milliseconds: f32,
            delay_feedback: f32,
            delay_return: f32,
        }

        impl From<&GlobalParameters> for SerializableGlobalParameters {
            fn from(parameters: &GlobalParameters) -> Self {
                Self {
                    master_gain_db: parameters.master_gain_db(),
                    reverb_room_size: parameters.reverb_room_size(),
                    reverb_damping: parameters.reverb_damping(),
                    reverb_return: parameters.reverb_return(),
                    delay_milliseconds: parameters.delay_milliseconds(),
                    delay_feedback: parameters.delay_feedback(),
                    delay_return: parameters.delay_return(),
                }
            }
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializableParameterSnapshot<'a> {
            generation: u64,
            graph_revision: GraphRevision,
            patch_count: usize,
            patches: &'a [RtPatchParameters],
            global: SerializableGlobalParameters,
        }

        SerializableParameterSnapshot {
            generation: self.generation(),
            graph_revision: self.graph_revision(),
            patch_count: self.patch_count(),
            patches: self.patches(),
            global: SerializableGlobalParameters::from(self.global()),
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ParameterSnapshot, ParameterSnapshotError, RtInstrumentParameters, RtPatchParameters,
        RtPostEffectParameters, MAX_PATCHES,
    };
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::graph_revision::GraphRevision;
    use crate::synth::voice_envelope::VoiceEnvelope;
    use crate::synth::EffectSlotId;
    use serde_json::Value;
    use std::collections::BTreeSet;

    fn global() -> GlobalParameters {
        GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2).unwrap()
    }

    fn patch(id: u32, gain_db: f32) -> RtPatchParameters {
        RtPatchParameters::new(
            PatchId::new(id).unwrap(),
            ChannelParameters::new(gain_db, 0.0, 0.2, 0.1).unwrap(),
        )
    }

    #[test]
    fn copies_one_complete_accepted_control_projection() {
        let patches = [patch(1, -6.0), patch(2, -12.0)];
        let revision = GraphRevision::new(7).unwrap();
        let snapshot = ParameterSnapshot::for_graph(42, revision, global(), &patches).unwrap();

        assert_eq!(snapshot.generation(), 42);
        assert_eq!(snapshot.graph_revision(), revision);
        assert_eq!(snapshot.global(), &global());
        assert_eq!(snapshot.patch_count(), 2);
        assert_eq!(snapshot.patches(), &patches);
        assert_eq!(snapshot.patch(PatchId::new(2).unwrap()), Some(&patches[1]));
    }

    #[test]
    fn patch_projection_carries_bounded_envelope_and_descriptor_ordered_scalars() {
        let envelope = VoiceEnvelope::new(5.0, 120.0, 0.6, 450.0).unwrap();
        let instrument = RtInstrumentParameters::new(&[3.0, 0.25, 0.75]).unwrap();
        let projected = RtPatchParameters::projected(
            PatchId::new(7).unwrap(),
            ChannelParameters::default(),
            envelope,
            instrument,
        );

        assert_eq!(projected.envelope(), &envelope);
        assert_eq!(projected.instrument().count(), 3);
        assert_eq!(projected.instrument().values(), &[3.0, 0.25, 0.75]);
        assert!(projected.instrument().storage()[3..]
            .iter()
            .all(|value| *value == 0.0));
        assert_eq!(
            RtInstrumentParameters::new(&[0.0; 17]),
            Err(ParameterSnapshotError::TooManyInstrumentScalars {
                count: 17,
                capacity: 16,
            })
        );
        assert_eq!(
            RtInstrumentParameters::new(&[f32::NAN]),
            Err(ParameterSnapshotError::NonFiniteInstrumentScalar { index: 0 })
        );
    }

    #[test]
    fn compatibility_requires_revision_count_and_exact_patch_order() {
        let revision = GraphRevision::new(8).unwrap();
        let snapshot = ParameterSnapshot::for_graph(
            42,
            revision,
            global(),
            &[patch(1, -6.0), patch(2, -12.0)],
        )
        .unwrap();

        assert!(snapshot.is_compatible(
            revision,
            &[PatchId::new(1).unwrap(), PatchId::new(2).unwrap()]
        ));
        assert!(!snapshot.is_compatible(
            GraphRevision::new(9).unwrap(),
            &[PatchId::new(1).unwrap(), PatchId::new(2).unwrap()]
        ));
        assert!(!snapshot.is_compatible(revision, &[PatchId::new(1).unwrap()]));
        assert!(!snapshot.is_compatible(
            revision,
            &[PatchId::new(2).unwrap(), PatchId::new(1).unwrap()]
        ));
    }

    #[test]
    fn serialized_leaf_descriptor_exactly_matches_the_active_projection() {
        fn leaves(value: &Value, prefix: &str, output: &mut BTreeSet<String>) {
            match value {
                Value::Object(object) => {
                    for (name, child) in object {
                        let path = if prefix.is_empty() {
                            name.to_owned()
                        } else {
                            format!("{prefix}.{name}")
                        };
                        leaves(child, &path, output);
                    }
                }
                Value::Array(array) => {
                    for child in array {
                        leaves(child, &format!("{prefix}[]"), output);
                    }
                }
                _ => {
                    output.insert(prefix.to_owned());
                }
            }
        }

        let projected = RtPatchParameters::projected_with_effect(
            PatchId::new(7).unwrap(),
            ChannelParameters::new(-3.0, 0.25, 0.4, 0.2).unwrap(),
            VoiceEnvelope::new(12.0, 34.0, 0.56, 78.0).unwrap(),
            RtInstrumentParameters::new(&[2.0, 0.35, 0.65]).unwrap(),
            RtPostEffectParameters::new(EffectSlotId::new(1).unwrap(), &[0.5, 0.75]).unwrap(),
        );
        let snapshot =
            ParameterSnapshot::for_graph(9, GraphRevision::new(4).unwrap(), global(), &[projected])
                .unwrap();
        let mut discovered = BTreeSet::new();
        leaves(
            &serde_json::to_value(snapshot).unwrap(),
            "",
            &mut discovered,
        );
        let descriptor = ParameterSnapshot::serialized_leaf_descriptor();
        let described = descriptor
            .iter()
            .map(|path| (*path).to_owned())
            .collect::<BTreeSet<_>>();

        assert_eq!(descriptor.len(), described.len());
        assert_eq!(described, discovered);
    }

    #[test]
    fn unused_fixed_entries_are_inactive() {
        let snapshot = ParameterSnapshot::new(1, global(), &[patch(1, 0.0)]).unwrap();

        assert!(snapshot.storage()[0].is_active());
        assert!(snapshot.storage()[1..]
            .iter()
            .all(|entry| !entry.is_active()));
    }

    #[test]
    fn rejects_state_larger_than_the_compile_time_bound() {
        let patches = [patch(1, 0.0); MAX_PATCHES + 1];
        let error = ParameterSnapshot::new(1, global(), &patches).unwrap_err();

        assert_eq!(
            error,
            ParameterSnapshotError::TooManyPatches {
                count: MAX_PATCHES + 1,
                capacity: MAX_PATCHES
            }
        );
    }

    #[test]
    fn snapshot_and_patch_values_need_no_drop_or_dynamic_storage() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<ParameterSnapshot>();
        assert_copy::<RtPatchParameters>();
        assert_copy::<RtInstrumentParameters>();
        assert!(!core::mem::needs_drop::<ParameterSnapshot>());
        assert!(!core::mem::needs_drop::<RtPatchParameters>());
        assert!(!core::mem::needs_drop::<RtInstrumentParameters>());
        assert_eq!(
            core::mem::size_of::<ParameterSnapshot>(),
            core::mem::size_of_val(&ParameterSnapshot::new(0, global(), &[]).unwrap())
        );
    }
}
