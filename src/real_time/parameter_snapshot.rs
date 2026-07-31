use crate::kernel::patch_id::PatchId;
use crate::mixer::bus_id::{BusId, MAX_BUS_RETURNS};
use crate::mixer::global_parameters::GlobalParameters;
use crate::mixer::mixer_state::MixerState;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::MixerTrackParameters;
use crate::mixer::patch_output::PatchOutput;
use crate::real_time::graph_revision::GraphRevision;
use crate::synth::effect_slot_id::MAX_EFFECT_SLOTS;
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

/// Fixed live values for one bus return: the occupying instance's
/// descriptor-ordered scalars plus the return-owned output level.
///
/// Modelled directly on [`RtPostEffectParameters`] because a return plays the
/// same role as a Patch effect slot: zero or one prepared registry instance
/// whose live scalars cross the real-time boundary each block. The one
/// addition is the return-owned level, which scales the wet contribution and
/// survives occupancy changes on the domain side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RtBusReturnParameters {
    effect: RtPostEffectParameters,
    return_level: f32,
}

impl RtBusReturnParameters {
    /// The canonical unoccupied return entry.
    pub const EMPTY: Self = Self {
        effect: RtPostEffectParameters::EMPTY,
        return_level: 0.0,
    };

    /// Copies one occupied return's descriptor-ordered scalars and level.
    ///
    /// Scalar validation reuses the effect-projection error paths. The return
    /// level is validated as the entry's final scalar-position value: a
    /// non-finite level reports `NonFiniteEffectScalar` with the index one
    /// past the last declared scalar.
    pub fn new(
        slot_id: EffectSlotId,
        scalars: &[f32],
        return_level: f32,
    ) -> Result<Self, ParameterSnapshotError> {
        let effect = RtPostEffectParameters::new(slot_id, scalars)?;
        if !return_level.is_finite() {
            return Err(ParameterSnapshotError::NonFiniteEffectScalar {
                index: scalars.len(),
            });
        }
        Ok(Self {
            effect,
            return_level,
        })
    }

    pub const fn is_active(&self) -> bool {
        self.effect.is_active()
    }

    pub const fn slot_id(&self) -> Option<EffectSlotId> {
        self.effect.slot_id()
    }

    pub const fn scalar_count(&self) -> usize {
        self.effect.scalar_count()
    }

    pub fn scalars(&self) -> &[f32] {
        self.effect.scalars()
    }

    pub fn scalar(&self, index: usize) -> Option<f32> {
        self.effect.scalar(index)
    }

    /// Returns the inner effect-shaped view handed to the prepared instance.
    pub const fn effect_parameters(&self) -> &RtPostEffectParameters {
        &self.effect
    }

    pub const fn return_level(&self) -> f32 {
        self.return_level
    }
}

impl Default for RtBusReturnParameters {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Serialize for RtBusReturnParameters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializableBusReturnParameters<'a> {
            active: bool,
            slot_id: Option<EffectSlotId>,
            scalar_count: usize,
            scalars: &'a [f32],
            return_level: f32,
        }
        SerializableBusReturnParameters {
            active: self.is_active(),
            slot_id: self.slot_id(),
            scalar_count: self.scalar_count(),
            scalars: self.scalars(),
            return_level: self.return_level(),
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
    output: PatchOutput,
    envelope: VoiceEnvelope,
    instrument: RtInstrumentParameters,
    effects: [RtPostEffectParameters; MAX_EFFECT_SLOTS],
}

impl RtPatchParameters {
    /// Copies one active Patch's identity and validated mixer parameters into a
    /// real-time-safe value.
    pub const fn new(patch_id: PatchId, output: PatchOutput) -> Self {
        Self {
            patch_id: Some(patch_id),
            output,
            envelope: VoiceEnvelope::DEFAULT,
            instrument: RtInstrumentParameters::EMPTY,
            effects: [RtPostEffectParameters::EMPTY; MAX_EFFECT_SLOTS],
        }
    }

    /// Copies the full live projection for one active Patch.
    pub const fn projected(
        patch_id: PatchId,
        output: PatchOutput,
        envelope: VoiceEnvelope,
        instrument: RtInstrumentParameters,
    ) -> Self {
        Self {
            patch_id: Some(patch_id),
            output,
            envelope,
            instrument,
            effects: [RtPostEffectParameters::EMPTY; MAX_EFFECT_SLOTS],
        }
    }

    /// Copies the full live projection including every ordered effect position.
    ///
    /// One entry per position: empty positions carry the canonical inactive
    /// value at their exact index, never compacted down.
    pub const fn projected_with_effects(
        patch_id: PatchId,
        output: PatchOutput,
        envelope: VoiceEnvelope,
        instrument: RtInstrumentParameters,
        effects: [RtPostEffectParameters; MAX_EFFECT_SLOTS],
    ) -> Self {
        Self {
            patch_id: Some(patch_id),
            output,
            envelope,
            instrument,
            effects,
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

    /// Returns the Patch's copied, validated output route and trim.
    pub const fn output(&self) -> PatchOutput {
        self.output
    }

    pub const fn envelope(&self) -> &VoiceEnvelope {
        &self.envelope
    }

    pub const fn instrument(&self) -> &RtInstrumentParameters {
        &self.instrument
    }

    /// Returns every ordered effect position, one entry per slot index.
    pub const fn effects(&self) -> &[RtPostEffectParameters; MAX_EFFECT_SLOTS] {
        &self.effects
    }

    /// Returns the live entry at one exact slot position.
    pub fn effect(&self, slot: usize) -> Option<&RtPostEffectParameters> {
        self.effects.get(slot)
    }

    fn inactive() -> Self {
        Self {
            patch_id: None,
            output: PatchOutput::default(),
            envelope: VoiceEnvelope::DEFAULT,
            instrument: RtInstrumentParameters::EMPTY,
            effects: [RtPostEffectParameters::EMPTY; MAX_EFFECT_SLOTS],
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
            effects: &'a [RtPostEffectParameters; MAX_EFFECT_SLOTS],
            output: PatchOutput,
        }

        SerializablePatchParameters {
            patch_id: self.patch_id(),
            envelope: self.envelope(),
            instrument: self.instrument(),
            effects: self.effects(),
            output: self.output(),
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
    mixer_tracks: [MixerTrackParameters; MixerTrackId::COUNT],
    returns: [RtBusReturnParameters; MAX_BUS_RETURNS],
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
        "patches[].effects[].active",
        "patches[].effects[].slotId",
        "patches[].effects[].scalarCount",
        "patches[].effects[].scalars[]",
        "patches[].output.trackId",
        "patches[].output.trimGainDb",
        "mixerTracks[].levelDb",
        "mixerTracks[].pan",
        "mixerTracks[].mute",
        "mixerTracks[].solo",
        "mixerTracks[].sends[]",
        "returns[].active",
        "returns[].slotId",
        "returns[].scalarCount",
        "returns[].scalars[]",
        "returns[].returnLevel",
        "global.masterGainDb",
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
        mixer: MixerState,
        patches: &[RtPatchParameters],
    ) -> Result<Self, ParameterSnapshotError> {
        Self::for_graph(generation, GraphRevision::INITIAL, global, mixer, patches)
    }

    /// Copies one complete projection for a specific prepared graph revision.
    ///
    /// The eight bus-return entries are all inactive: live return values come
    /// only from a projection of the canonical `BusReturnBank` supplied
    /// through [`Self::for_graph_with_returns`] or
    /// [`Self::project_returns`], never derived from global state.
    pub fn for_graph(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        mixer: MixerState,
        patches: &[RtPatchParameters],
    ) -> Result<Self, ParameterSnapshotError> {
        Self::for_graph_with_returns(
            generation,
            graph_revision,
            global,
            mixer,
            patches,
            [RtBusReturnParameters::EMPTY; MAX_BUS_RETURNS],
        )
    }

    /// Copies one complete projection carrying an explicit return bank.
    pub fn for_graph_with_returns(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        mixer: MixerState,
        patches: &[RtPatchParameters],
        returns: [RtBusReturnParameters; MAX_BUS_RETURNS],
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
            mixer_tracks: *mixer.tracks(),
            returns,
            patch_count: patches.len(),
            patches: storage,
        })
    }

    /// Replaces the eight live return entries on an already-complete snapshot.
    #[must_use]
    pub const fn with_returns(mut self, returns: [RtBusReturnParameters; MAX_BUS_RETURNS]) -> Self {
        self.returns = returns;
        self
    }

    /// Projects the canonical bus-return bank into the eight fixed live
    /// entries: each occupied return contributes its instance identity, its
    /// descriptor-ordered scalar values, and the return-owned level; each
    /// unoccupied return stays the canonical inactive value at its exact
    /// index.
    ///
    /// This is the one derivation of live return values. The prepared graph
    /// builder installs occupancy from the same bank, so the snapshot and the
    /// prepared return rack cannot drift.
    pub fn project_returns(
        effect_registry: &EffectCapabilityRegistry,
        bank: &crate::mixer::bus_return::BusReturnBank,
    ) -> Result<[RtBusReturnParameters; MAX_BUS_RETURNS], ParameterSnapshotError> {
        let mut returns = [RtBusReturnParameters::EMPTY; MAX_BUS_RETURNS];
        for bus_return in bank.returns() {
            let Some(config) = bus_return.effect() else {
                continue;
            };
            let index = bus_return.id().index();
            let descriptor = effect_registry
                .descriptor(config.capability_id())
                .ok_or(ParameterSnapshotError::InvalidEffectConfig { index })?;
            let scalars = descriptor
                .scalar_parameters()
                .map(|spec| {
                    let value = config
                        .value(spec.id())
                        .ok_or(ParameterSnapshotError::InvalidEffectConfig { index })?;
                    spec.scalar_value(value)
                        .map_err(|_| ParameterSnapshotError::InvalidEffectConfig { index })
                })
                .collect::<Result<Vec<_>, _>>()?;
            returns[index] =
                RtBusReturnParameters::new(config.slot_id(), &scalars, bus_return.return_level())?;
        }
        Ok(returns)
    }

    /// Projects the canonical Patch/config values into the one fixed real-time shape.
    ///
    /// Control state projection and off-callback graph preparation share this
    /// implementation so descriptor ordering and Scalar encoding cannot drift.
    pub fn project_patches(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        mixer: MixerState,
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
                    patch.output(),
                    *patch.envelope(),
                    instrument,
                ))
            })
            .collect::<Result<Vec<_>, ParameterSnapshotError>>()?;

        Self::for_graph(generation, graph_revision, global, mixer, &projected)
    }

    /// Projects instrument and post-effect values into their separate fixed layouts.
    pub fn project_patches_with_effects(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        mixer: MixerState,
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
                let mut effects = [RtPostEffectParameters::EMPTY; MAX_EFFECT_SLOTS];
                for (position, occupancy) in patch.effect_slots().iter().enumerate() {
                    let Some(config) = occupancy else {
                        continue;
                    };
                    let effect_descriptor = effect_registry
                        .descriptor(config.capability_id())
                        .ok_or(ParameterSnapshotError::InvalidEffectConfig { index })?;
                    let effect_values = effect_descriptor
                        .scalar_parameters()
                        .map(|spec| {
                            let value = config
                                .value(spec.id())
                                .ok_or(ParameterSnapshotError::InvalidEffectConfig { index })?;
                            spec.scalar_value(value)
                                .map_err(|_| ParameterSnapshotError::InvalidEffectConfig { index })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    effects[position] =
                        RtPostEffectParameters::new(config.slot_id(), &effect_values)?;
                }
                Ok(RtPatchParameters::projected_with_effects(
                    patch.id(),
                    patch.output(),
                    *patch.envelope(),
                    instrument,
                    effects,
                ))
            })
            .collect::<Result<Vec<_>, ParameterSnapshotError>>()?;
        Self::for_graph(generation, graph_revision, global, mixer, &projected)
    }

    /// Projects Patch, effect, and bus-return values in one complete pass.
    ///
    /// This is the canonical production projection: control state projection
    /// and off-callback graph preparation both derive the eight live return
    /// entries from the same `BusReturnBank` that drives prepared occupancy.
    #[allow(clippy::too_many_arguments)]
    pub fn project_patches_with_effects_and_returns(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        mixer: MixerState,
        patches: &[Patch],
        registry: &CapabilityRegistry,
        effect_registry: &EffectCapabilityRegistry,
        bank: &crate::mixer::bus_return::BusReturnBank,
    ) -> Result<Self, ParameterSnapshotError> {
        let returns = Self::project_returns(effect_registry, bank)?;
        Ok(Self::project_patches_with_effects(
            generation,
            graph_revision,
            global,
            mixer,
            patches,
            registry,
            effect_registry,
        )?
        .with_returns(returns))
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

    /// Returns all fixed track values in `MixerTrackId` order.
    pub const fn mixer_tracks(&self) -> &[MixerTrackParameters; MixerTrackId::COUNT] {
        &self.mixer_tracks
    }

    pub const fn mixer_track(&self, id: MixerTrackId) -> &MixerTrackParameters {
        &self.mixer_tracks[id.index()]
    }

    /// Returns all eight live bus-return entries in ascending `BusId` order.
    pub const fn returns(&self) -> &[RtBusReturnParameters; MAX_BUS_RETURNS] {
        &self.returns
    }

    /// Returns the live entry for one bus return.
    pub const fn bus_return(&self, bus: BusId) -> &RtBusReturnParameters {
        &self.returns[bus.index()]
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

    /// Compares only data observed by the audio thread, excluding the
    /// control-generation correlation carried by this immutable projection.
    pub fn audio_values_equal(&self, other: &Self) -> bool {
        self.graph_revision == other.graph_revision
            && self.global == other.global
            && self.mixer_tracks == other.mixer_tracks
            && self.returns == other.returns
            && self.patch_count == other.patch_count
            && self.patches == other.patches
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
        /// The retired reverb/delay fields are no longer serialized: their
        /// values cross the boundary as the generic indexed `returns` entries.
        /// Master gain is the one genuinely global leaf.
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializableGlobalParameters {
            master_gain_db: f32,
        }

        impl From<&GlobalParameters> for SerializableGlobalParameters {
            fn from(parameters: &GlobalParameters) -> Self {
                Self {
                    master_gain_db: parameters.master_gain_db(),
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
            mixer_tracks: &'a [MixerTrackParameters; MixerTrackId::COUNT],
            returns: &'a [RtBusReturnParameters; MAX_BUS_RETURNS],
            global: SerializableGlobalParameters,
        }

        SerializableParameterSnapshot {
            generation: self.generation(),
            graph_revision: self.graph_revision(),
            patch_count: self.patch_count(),
            patches: self.patches(),
            mixer_tracks: self.mixer_tracks(),
            returns: self.returns(),
            global: SerializableGlobalParameters::from(self.global()),
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ParameterSnapshot, ParameterSnapshotError, RtBusReturnParameters, RtInstrumentParameters,
        RtPatchParameters, RtPostEffectParameters, MAX_PATCHES,
    };
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::bus_id::{BusId, MAX_BUS_RETURNS};
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mixer_state::MixerState;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::mixer_track_parameters::MixerTrackParameters;
    use crate::mixer::patch_output::PatchOutput;
    use crate::real_time::graph_revision::GraphRevision;
    use crate::synth::effect_slot_id::MAX_EFFECT_SLOTS;
    use crate::synth::voice_envelope::VoiceEnvelope;
    use crate::synth::{EffectSlotId, MAX_EFFECT_SCALAR_PARAMETERS};
    use serde_json::Value;
    use std::collections::BTreeSet;

    fn global() -> GlobalParameters {
        GlobalParameters::new(-3.0).unwrap()
    }

    fn patch(id: u32, gain_db: f32) -> RtPatchParameters {
        RtPatchParameters::new(
            PatchId::new(id).unwrap(),
            PatchOutput::new(MixerTrackId::new(((id - 1) % 16) as u8).unwrap(), gain_db).unwrap(),
        )
    }

    #[test]
    fn copies_one_complete_accepted_control_projection() {
        let patches = [patch(1, -6.0), patch(2, -12.0)];
        let revision = GraphRevision::new(7).unwrap();
        let snapshot =
            ParameterSnapshot::for_graph(42, revision, global(), MixerState::default(), &patches)
                .unwrap();

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
            PatchOutput::default(),
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
            MixerState::default(),
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

        let mut effects = [RtPostEffectParameters::EMPTY; MAX_EFFECT_SLOTS];
        effects[1] =
            RtPostEffectParameters::new(EffectSlotId::new(1).unwrap(), &[0.5, 0.75]).unwrap();
        let projected = RtPatchParameters::projected_with_effects(
            PatchId::new(7).unwrap(),
            PatchOutput::new(MixerTrackId::new(7).unwrap(), -3.0).unwrap(),
            VoiceEnvelope::new(12.0, 34.0, 0.56, 78.0).unwrap(),
            RtInstrumentParameters::new(&[2.0, 0.35, 0.65]).unwrap(),
            effects,
        );
        let mut returns = [RtBusReturnParameters::EMPTY; MAX_BUS_RETURNS];
        returns[2] =
            RtBusReturnParameters::new(EffectSlotId::new(3).unwrap(), &[0.25, 0.5], 0.75).unwrap();
        let snapshot = ParameterSnapshot::for_graph(
            9,
            GraphRevision::new(4).unwrap(),
            global(),
            MixerState::default(),
            &[projected],
        )
        .unwrap()
        .with_returns(returns);
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
        let snapshot =
            ParameterSnapshot::new(1, global(), MixerState::default(), &[patch(1, 0.0)]).unwrap();

        assert!(snapshot.storage()[0].is_active());
        assert!(snapshot.storage()[1..]
            .iter()
            .all(|entry| !entry.is_active()));
    }

    #[test]
    fn rejects_state_larger_than_the_compile_time_bound() {
        let patches = [patch(1, 0.0); MAX_PATCHES + 1];
        let error =
            ParameterSnapshot::new(1, global(), MixerState::default(), &patches).unwrap_err();

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
        assert_copy::<RtBusReturnParameters>();
        assert!(!core::mem::needs_drop::<ParameterSnapshot>());
        assert!(!core::mem::needs_drop::<RtPatchParameters>());
        assert!(!core::mem::needs_drop::<RtInstrumentParameters>());
        assert!(!core::mem::needs_drop::<RtBusReturnParameters>());
        assert_eq!(
            core::mem::size_of::<ParameterSnapshot>(),
            core::mem::size_of_val(
                &ParameterSnapshot::new(0, global(), MixerState::default(), &[]).unwrap(),
            )
        );
    }

    fn occupied_effects(seed: u32) -> [RtPostEffectParameters; MAX_EFFECT_SLOTS] {
        std::array::from_fn(|position| {
            let slot =
                EffectSlotId::new((seed * MAX_EFFECT_SLOTS as u32) as u16 + position as u16 + 1)
                    .unwrap();
            let scalars: Vec<f32> = (0..MAX_EFFECT_SCALAR_PARAMETERS)
                .map(|index| (seed as f32 + position as f32 + index as f32) * 0.01)
                .collect();
            RtPostEffectParameters::new(slot, &scalars).unwrap()
        })
    }

    /// T026: a fully occupied 16 x 3 snapshot constructs with every slot
    /// correctly initialized at its exact position — no construction site
    /// leaves a position at `EMPTY` by accident.
    #[test]
    fn fully_occupied_snapshot_initializes_every_slot_at_its_exact_position() {
        let patches: Vec<RtPatchParameters> = (1..=MAX_PATCHES as u32)
            .map(|id| {
                RtPatchParameters::projected_with_effects(
                    PatchId::new(id).unwrap(),
                    PatchOutput::to_track(MixerTrackId::new(((id - 1) % 16) as u8).unwrap()),
                    VoiceEnvelope::DEFAULT,
                    RtInstrumentParameters::new(&[id as f32]).unwrap(),
                    occupied_effects(id),
                )
            })
            .collect();
        let snapshot =
            ParameterSnapshot::new(3, global(), MixerState::default(), &patches).unwrap();

        assert_eq!(snapshot.patch_count(), MAX_PATCHES);
        for (index, patch) in snapshot.patches().iter().enumerate() {
            let expected = occupied_effects(index as u32 + 1);
            assert_eq!(patch.effects(), &expected);
            for position in 0..MAX_EFFECT_SLOTS {
                let entry = patch.effect(position).unwrap();
                assert!(entry.is_active(), "patch {index} position {position}");
                assert_eq!(entry.scalar_count(), MAX_EFFECT_SCALAR_PARAMETERS);
            }
            assert!(patch.effect(MAX_EFFECT_SLOTS).is_none());
        }
    }

    /// T028/WP06: the eight return entries derive only from the canonical
    /// `BusReturnBank` — an unsupplied bank projects all-inactive, and the
    /// production default occupancy projects reverb/delay at returns 0 and 1
    /// with their descriptor default scalars and levels.
    #[test]
    fn returns_project_only_from_the_canonical_bus_return_bank() {
        let snapshot = ParameterSnapshot::new(1, global(), MixerState::default(), &[]).unwrap();
        assert_eq!(snapshot.returns().len(), MAX_BUS_RETURNS);
        assert!(snapshot.returns().iter().all(|entry| !entry.is_active()));

        let registry = crate::adapter::production_effects::production_effect_registry().unwrap();
        let bank =
            crate::adapter::production_effects::production_default_bus_returns(&registry).unwrap();
        let returns = ParameterSnapshot::project_returns(&registry, &bank).unwrap();

        assert!(returns[0].is_active());
        assert_eq!(returns[0].slot_id(), Some(EffectSlotId::new(1).unwrap()));
        assert_eq!(returns[0].scalars(), &[0.5, 0.5]);
        assert_eq!(returns[0].return_level(), 0.5);
        assert!(returns[1].is_active());
        assert_eq!(returns[1].slot_id(), Some(EffectSlotId::new(2).unwrap()));
        assert_eq!(returns[1].scalars(), &[250.0, 0.5]);
        assert_eq!(returns[1].return_level(), 0.5);
        assert!(returns[2..].iter().all(|entry| !entry.is_active()));

        let projected = snapshot.with_returns(returns);
        assert_eq!(projected.bus_return(BusId::new(1).unwrap()), &returns[1]);
    }

    /// T028: capacity and finiteness errors fire through the reused
    /// effect-projection error paths, including the return level convention.
    #[test]
    fn bus_return_projection_rejects_capacity_and_finiteness_violations() {
        let slot = EffectSlotId::new(1).unwrap();

        assert_eq!(
            RtBusReturnParameters::new(slot, &[0.0; MAX_EFFECT_SCALAR_PARAMETERS + 1], 0.5),
            Err(ParameterSnapshotError::TooManyEffectScalars {
                count: MAX_EFFECT_SCALAR_PARAMETERS + 1,
                capacity: MAX_EFFECT_SCALAR_PARAMETERS,
            })
        );
        assert_eq!(
            RtBusReturnParameters::new(slot, &[0.5, f32::NAN], 0.5),
            Err(ParameterSnapshotError::NonFiniteEffectScalar { index: 1 })
        );
        // The return level is validated as the entry's final scalar-position
        // value: its index is the declared scalar count.
        assert_eq!(
            RtBusReturnParameters::new(slot, &[0.5, 0.25], f32::INFINITY),
            Err(ParameterSnapshotError::NonFiniteEffectScalar { index: 2 })
        );

        let entry = RtBusReturnParameters::new(slot, &[0.5, 0.25], 0.75).unwrap();
        assert!(entry.is_active());
        assert_eq!(entry.scalar_count(), 2);
        assert_eq!(entry.scalar(1), Some(0.25));
        assert_eq!(entry.effect_parameters().scalars(), &[0.5, 0.25]);
        assert!(!RtBusReturnParameters::EMPTY.is_active());
        assert_eq!(RtBusReturnParameters::EMPTY.return_level(), 0.0);
    }

    /// Explicit returns are the only source of live return entries, and they
    /// change exactly the audio-observed return values.
    #[test]
    fn explicit_returns_are_the_only_live_return_source() {
        let mut returns = [RtBusReturnParameters::EMPTY; MAX_BUS_RETURNS];
        returns[5] =
            RtBusReturnParameters::new(EffectSlotId::new(6).unwrap(), &[0.1, 0.2, 0.3], 1.0)
                .unwrap();
        let bare = ParameterSnapshot::new(1, global(), MixerState::default(), &[]).unwrap();
        let explicit = ParameterSnapshot::for_graph_with_returns(
            1,
            GraphRevision::INITIAL,
            global(),
            MixerState::default(),
            &[],
            returns,
        )
        .unwrap();

        assert!(bare.returns().iter().all(|entry| !entry.is_active()));
        assert!(!explicit.returns()[0].is_active());
        assert!(explicit.returns()[5].is_active());
        assert_eq!(explicit.returns()[5].scalar_count(), 3);
        assert_eq!(bare.with_returns(returns).returns(), &returns);
        assert!(!bare.audio_values_equal(&explicit));
    }

    /// T027: all eight sends cross the boundary per track and are observable
    /// in `BusId` order.
    #[test]
    fn every_track_carries_all_eight_indexed_sends() {
        let mut sends = [0.0; MAX_BUS_RETURNS];
        for (index, send) in sends.iter_mut().enumerate() {
            *send = index as f32 * 0.1;
        }
        let track = MixerTrackParameters::from_values(0.0, 0.0, false, false, sends).unwrap();
        let snapshot = ParameterSnapshot::new(
            1,
            global(),
            MixerState::default().with_track(MixerTrackId::new(3).unwrap(), track),
            &[],
        )
        .unwrap();

        assert_eq!(
            snapshot.mixer_track(MixerTrackId::new(3).unwrap()).sends(),
            sends
        );
    }
}
