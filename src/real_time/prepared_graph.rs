use crate::kernel::patch_id::PatchId;
use crate::mixer::bus_id::{BusId, MAX_BUS_RETURNS};
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::callback_safety::record_callback_owned_destruction;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use crate::real_time::parameter_snapshot::MAX_PATCHES;
use crate::real_time::patch_audio_block::PatchAudioBlock;
use crate::real_time::prepared_bus_return_rack::PreparedBusReturnRack;
use crate::real_time::prepared_engine_rack::PreparedEngineRack;
use crate::real_time::PreparedPostEffectRack;
use crate::synth::capability_id::CapabilityId;
use crate::synth::effect_slot_id::MAX_EFFECT_SLOTS;
use crate::synth::EffectCapabilityId;
use crate::synth::EffectSlotId;
use core::fmt;

/// Fixed byte capacity of one recorded per-position capability identity.
///
/// The bound keeps [`PreparedGraphLayout`] and every rack slot fixed-size and
/// copyable; graph preparation refuses an identity it cannot record exactly —
/// it never truncates one.
pub const MAX_CAPABILITY_IDENTITY_BYTES: usize = 64;

/// The exact engine or effect capability identity recorded for one prepared
/// position — fixed-size, copyable, and comparable without heap access.
///
/// Every occupied prepared position (engine per Patch, effect per Patch-slot,
/// occupant per bus return) records the capability identity of the validated
/// candidate configuration it was prepared from. Carrying a live instance
/// into a replacement requires exact per-position identity agreement in
/// addition to patch, slot, and scalar-layout agreement; any mismatch keeps
/// the freshly prepared instance. The comparison is a bounded fixed-size
/// equality made at carry-over decision time, never per render block, and it
/// allocates, locks, and destroys nothing.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PositionCapabilityIdentity {
    length: u8,
    bytes: [u8; MAX_CAPABILITY_IDENTITY_BYTES],
}

impl PositionCapabilityIdentity {
    /// Records one engine capability identity exactly, or refuses one that
    /// exceeds the fixed capacity.
    pub fn from_capability_id(id: &CapabilityId) -> Option<Self> {
        Self::from_identifier(id.as_str())
    }

    /// Records one effect capability identity exactly, or refuses one that
    /// exceeds the fixed capacity.
    pub fn from_effect_capability_id(id: &EffectCapabilityId) -> Option<Self> {
        Self::from_identifier(id.as_str())
    }

    fn from_identifier(value: &str) -> Option<Self> {
        let raw = value.as_bytes();
        if raw.is_empty() || raw.len() > MAX_CAPABILITY_IDENTITY_BYTES {
            return None;
        }
        let mut bytes = [0_u8; MAX_CAPABILITY_IDENTITY_BYTES];
        bytes[..raw.len()].copy_from_slice(raw);
        Some(Self {
            length: raw.len() as u8,
            bytes,
        })
    }

    /// Returns the recorded identity text.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.length)]).unwrap_or("")
    }
}

impl fmt::Debug for PositionCapabilityIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PositionCapabilityIdentity")
            .field(&self.as_str())
            .finish()
    }
}

/// One complete callback-ready synthesis and mixing topology.
///
/// The graph is the sole owner of prepared instruments, Patch stems, the
/// per-Patch effect grid, the bus-return rack (owned through the mix engine
/// as the post-effect rack's peer), mixer scratch, routing order, and
/// compatible initial scalar parameters for one structural revision.
pub struct PreparedGraph {
    inner: Box<PreparedGraphState>,
}

struct PreparedGraphState {
    revision: GraphRevision,
    sample_rate: f32,
    max_frames: usize,
    initial_parameters: ParameterSnapshot,
    engine_rack: PreparedEngineRack,
    effect_rack: PreparedPostEffectRack,
    patch_audio: PatchAudioBlock,
    mixer: MixEngine,
    /// The exact correlated delta this replacement declares, set on worker
    /// ownership after preparation. `Some` authorizes the block-boundary
    /// voice carry-over exchange at activation; `None` (a fresh initial
    /// graph, or a graph built outside the correlated worker path) keeps the
    /// full-reset swap semantics.
    carry_over: Option<GraphReplacementScope>,
}

pub(crate) struct PreparedGraphResources {
    engine_rack: PreparedEngineRack,
    effect_rack: PreparedPostEffectRack,
    patch_audio: PatchAudioBlock,
    mixer: MixEngine,
}

impl PreparedGraphResources {
    pub(crate) fn new(
        engine_rack: PreparedEngineRack,
        effect_rack: PreparedPostEffectRack,
        patch_audio: PatchAudioBlock,
        mixer: MixEngine,
    ) -> Self {
        Self {
            engine_rack,
            effect_rack,
            patch_audio,
            mixer,
        }
    }
}

/// Fixed-size identity and callback-capacity contract shared by every graph in
/// one running renderer. Replacements may change implementation-owned state,
/// but not the device capacity or canonical ordered Patch layout.
///
/// Every prepared position additionally records its capability identity —
/// the engine per Patch, the effect per Patch-slot, the occupant per bus
/// return — populated at build time from the validated candidate
/// configuration, with an explicit empty for every unoccupied position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedGraphLayout {
    sample_rate_bits: u32,
    max_frames: usize,
    patch_count: usize,
    patch_ids: [Option<PatchId>; MAX_PATCHES],
    scalar_counts: [u8; MAX_PATCHES],
    engine_capability_identities: [Option<PositionCapabilityIdentity>; MAX_PATCHES],
    effect_slot_ids: [[Option<EffectSlotId>; MAX_EFFECT_SLOTS]; MAX_PATCHES],
    effect_scalar_counts: [[u8; MAX_EFFECT_SLOTS]; MAX_PATCHES],
    effect_capability_identities:
        [[Option<PositionCapabilityIdentity>; MAX_EFFECT_SLOTS]; MAX_PATCHES],
    return_slot_ids: [Option<EffectSlotId>; MAX_BUS_RETURNS],
    return_scalar_counts: [u8; MAX_BUS_RETURNS],
    return_capability_identities: [Option<PositionCapabilityIdentity>; MAX_BUS_RETURNS],
}

impl PreparedGraph {
    pub(crate) fn new(
        revision: GraphRevision,
        sample_rate: f32,
        max_frames: usize,
        initial_parameters: ParameterSnapshot,
        resources: PreparedGraphResources,
    ) -> Self {
        Self {
            inner: Box::new(PreparedGraphState {
                revision,
                sample_rate,
                max_frames,
                initial_parameters,
                engine_rack: resources.engine_rack,
                effect_rack: resources.effect_rack,
                patch_audio: resources.patch_audio,
                mixer: resources.mixer,
                carry_over: None,
            }),
        }
    }

    pub const fn revision(&self) -> GraphRevision {
        self.inner.revision
    }

    pub const fn sample_rate(&self) -> f32 {
        self.inner.sample_rate
    }

    pub const fn max_frames(&self) -> usize {
        self.inner.max_frames
    }

    pub const fn initial_parameters(&self) -> &ParameterSnapshot {
        &self.inner.initial_parameters
    }

    pub const fn engine_rack(&self) -> &PreparedEngineRack {
        &self.inner.engine_rack
    }

    pub const fn patch_audio(&self) -> &PatchAudioBlock {
        &self.inner.patch_audio
    }

    pub const fn effect_rack(&self) -> &PreparedPostEffectRack {
        &self.inner.effect_rack
    }

    /// Returns the fixed replacement contract without borrowing graph-owned
    /// engine, effect, or scratch state.
    pub fn layout(&self) -> PreparedGraphLayout {
        let mut patch_ids = [None; MAX_PATCHES];
        let mut scalar_counts = [0; MAX_PATCHES];
        let mut engine_capability_identities = [None; MAX_PATCHES];
        let mut effect_slot_ids = [[None; MAX_EFFECT_SLOTS]; MAX_PATCHES];
        let mut effect_scalar_counts = [[0; MAX_EFFECT_SLOTS]; MAX_PATCHES];
        let mut effect_capability_identities = [[None; MAX_EFFECT_SLOTS]; MAX_PATCHES];
        let mut index = 0;
        while index < self.inner.engine_rack.patch_count() {
            patch_ids[index] = self.inner.engine_rack.patch_id(index);
            scalar_counts[index] =
                self.inner
                    .engine_rack
                    .scalar_count(index)
                    .expect("active rack slots have a Scalar count") as u8;
            engine_capability_identities[index] = self.inner.engine_rack.capability_identity(index);
            let mut position = 0;
            while position < MAX_EFFECT_SLOTS {
                effect_slot_ids[index][position] =
                    self.inner.effect_rack.slot_id_at(index, position);
                effect_scalar_counts[index][position] = self
                    .inner
                    .effect_rack
                    .scalar_count_at(index, position)
                    .unwrap_or(0) as u8;
                effect_capability_identities[index][position] = self
                    .inner
                    .effect_rack
                    .capability_identity_at(index, position);
                position += 1;
            }
            index += 1;
        }
        let mut return_slot_ids = [None; MAX_BUS_RETURNS];
        let mut return_scalar_counts = [0; MAX_BUS_RETURNS];
        let mut return_capability_identities = [None; MAX_BUS_RETURNS];
        for bus in BusId::ALL {
            return_slot_ids[bus.index()] = self.inner.mixer.bus_returns().slot_id(bus);
            return_scalar_counts[bus.index()] = self
                .inner
                .mixer
                .bus_returns()
                .scalar_count(bus)
                .unwrap_or(0) as u8;
            return_capability_identities[bus.index()] =
                self.inner.mixer.bus_returns().capability_identity(bus);
        }
        PreparedGraphLayout {
            sample_rate_bits: self.inner.sample_rate.to_bits(),
            max_frames: self.inner.max_frames,
            patch_count: self.inner.engine_rack.patch_count(),
            patch_ids,
            scalar_counts,
            engine_capability_identities,
            effect_slot_ids,
            effect_scalar_counts,
            effect_capability_identities,
            return_slot_ids,
            return_scalar_counts,
            return_capability_identities,
        }
    }

    /// Returns the graph-owned bus-return rack, the post-effect rack's peer.
    pub const fn bus_return_rack(&self) -> &PreparedBusReturnRack {
        self.inner.mixer.bus_returns()
    }

    /// Rebinds the graph's activation fallback to the exact committed state.
    ///
    /// This runs only on control ownership before publication. The replacement
    /// must target this graph and match its complete ordered engine layout.
    pub fn refresh_initial_parameters(
        &mut self,
        parameters: ParameterSnapshot,
    ) -> Result<(), PreparedGraphRefreshError> {
        if parameters.graph_revision() != self.revision() {
            return Err(PreparedGraphRefreshError::RevisionMismatch);
        }
        if !self.inner.engine_rack.matches_parameters(&parameters)
            || !self.inner.effect_rack.matches_parameters(&parameters)
            || !self
                .inner
                .mixer
                .bus_returns()
                .matches_parameters(&parameters)
        {
            return Err(PreparedGraphRefreshError::LayoutMismatch);
        }
        self.inner.initial_parameters = parameters;
        Ok(())
    }

    /// Borrows the three mutable callback projections together while retaining
    /// their ownership inside this complete graph.
    pub fn callback_parts_mut(
        &mut self,
    ) -> (
        &mut PreparedEngineRack,
        &mut PatchAudioBlock,
        &mut MixEngine,
    ) {
        (
            &mut self.inner.engine_rack,
            &mut self.inner.patch_audio,
            &mut self.inner.mixer,
        )
    }

    /// Borrows the complete callback pipeline in processing order.
    pub fn callback_parts_with_effects_mut(
        &mut self,
    ) -> (
        &mut PreparedEngineRack,
        &mut PreparedPostEffectRack,
        &mut PatchAudioBlock,
        &mut MixEngine,
    ) {
        (
            &mut self.inner.engine_rack,
            &mut self.inner.effect_rack,
            &mut self.inner.patch_audio,
            &mut self.inner.mixer,
        )
    }

    /// Declares the exact correlated delta this replacement carries, enabling
    /// voice carry-over at activation. Set only on worker ownership after a
    /// correlated preparation succeeds; never on the callback.
    pub(crate) fn set_carry_over_scope(&mut self, scope: GraphReplacementScope) {
        self.inner.carry_over = Some(scope);
    }

    /// Returns the correlated delta this replacement declares, if any.
    pub fn carry_over_scope(&self) -> Option<GraphReplacementScope> {
        self.inner.carry_over
    }

    /// Voice carry-over across a correlated structural replacement.
    ///
    /// Chosen shape: **unchanged-position live-instance exchange at the block
    /// boundary** — a variant of "reuse unchanged prepared components". The
    /// worker still builds a complete fresh replacement graph (preparation,
    /// atomic failure, and `matches_parameters` exactness are untouched); the
    /// replacement carries the correlated [`GraphReplacementScope`], and at
    /// activation the callback `mem::swap`s the still-live prepared instances
    /// from the superseded graph into the replacement at every position the
    /// scope leaves unchanged:
    ///
    /// - engine rack: every Patch except a `SelectedEngine` target — voices,
    ///   envelopes, channel state, and partial-block engine state move as a
    ///   pointer-sized ownership exchange;
    /// - Patch effect grid: every position except a `PatchSlot` target —
    ///   unchanged instances keep their tails;
    /// - bus returns: every return except a `BusReturn` target.
    ///
    /// Rejected alternatives: (b) bounded voice-state transfer is infeasible
    /// because rustysynth and the pinned C++ Braids engines do not expose
    /// envelope-phase extraction/injection across the prepared capability
    /// boundary; (c) MIDI replay is a retrigger and cannot satisfy the
    /// sample-continuity proof.
    ///
    /// Callback discipline: every exchange is a bounded `mem::swap` of an
    /// owning pointer guarded by exact identity agreement — patch, slot,
    /// scalar layout, and recorded per-position capability identity — no
    /// allocation, no deallocation, no locking, no blocking, no destruction.
    /// A position whose recorded capability identity disagrees keeps its
    /// freshly prepared instance. The fresh
    /// never-sounded instances ride into the superseded graph, which retires
    /// through the existing return queue and is destroyed off-callback
    /// exactly as before. A graph without a declared scope exchanges nothing.
    pub(crate) fn carry_live_state_from(&mut self, superseded: &mut Self) {
        let Some(scope) = self.inner.carry_over else {
            return;
        };
        let (engine_exclude, slot_exclude, return_exclude) = match scope {
            GraphReplacementScope::SelectedEngine(patch_id) => (Some(patch_id), None, None),
            GraphReplacementScope::PatchSlot { patch_id, slot } => {
                (None, Some((patch_id, slot.index())), None)
            }
            GraphReplacementScope::BusReturn(bus) => (None, None, Some(bus)),
        };
        self.inner
            .engine_rack
            .carry_live_instruments_from(&mut superseded.inner.engine_rack, engine_exclude);
        self.inner
            .effect_rack
            .carry_live_effects_from(&mut superseded.inner.effect_rack, slot_exclude);
        self.inner
            .mixer
            .bus_returns_mut()
            .carry_live_returns_from(superseded.inner.mixer.bus_returns_mut(), return_exclude);
    }
}

impl Drop for PreparedGraph {
    fn drop(&mut self) {
        record_callback_owned_destruction();
    }
}

/// The one position a correlated structural replacement is permitted to
/// change. Everything outside the scope must stay layout-identical, so a
/// replacement can never smuggle in an unrelated topology change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphReplacementScope {
    /// The selected Patch's engine capability and scalar shape may change.
    SelectedEngine(PatchId),
    /// Exactly one Patch effect-slot position may change occupancy.
    PatchSlot {
        patch_id: PatchId,
        slot: crate::synth::effect_slot_id::EffectSlotIndex,
    },
    /// Exactly one bus return may change occupancy.
    BusReturn(crate::mixer::bus_id::BusId),
}

impl PreparedGraphLayout {
    /// Returns the recorded engine capability identity at one Patch position.
    pub fn engine_capability_identity(&self, index: usize) -> Option<PositionCapabilityIdentity> {
        self.engine_capability_identities.get(index).copied()?
    }

    /// Returns the recorded effect capability identity at one grid position.
    pub fn effect_capability_identity(
        &self,
        patch_index: usize,
        slot_index: usize,
    ) -> Option<PositionCapabilityIdentity> {
        self.effect_capability_identities
            .get(patch_index)?
            .get(slot_index)
            .copied()?
    }

    /// Returns the recorded occupant capability identity at one bus return.
    pub fn return_capability_identity(
        &self,
        bus_index: usize,
    ) -> Option<PositionCapabilityIdentity> {
        self.return_capability_identities.get(bus_index).copied()?
    }

    /// Admits one selected capability/scalar-layout change and nothing else.
    pub fn permits_selected_replacement(self, candidate: Self, selected_patch_id: PatchId) -> bool {
        self.permits_replacement(
            candidate,
            GraphReplacementScope::SelectedEngine(selected_patch_id),
        )
    }

    /// Admits exactly the layout delta the scope declares and nothing else.
    ///
    /// Per-position capability identity is part of the contract: outside the
    /// scoped position, every prepared position's recorded identity must
    /// agree exactly — a same-scalar-shape candidate carrying a different
    /// capability at an unscoped position is refused.
    pub fn permits_replacement(self, candidate: Self, scope: GraphReplacementScope) -> bool {
        if self.sample_rate_bits != candidate.sample_rate_bits
            || self.max_frames != candidate.max_frames
            || self.patch_count != candidate.patch_count
            || self.patch_ids != candidate.patch_ids
        {
            return false;
        }
        match scope {
            GraphReplacementScope::SelectedEngine(selected_patch_id) => {
                if self.effect_slot_ids != candidate.effect_slot_ids
                    || self.effect_scalar_counts != candidate.effect_scalar_counts
                    || self.effect_capability_identities != candidate.effect_capability_identities
                    || self.return_slot_ids != candidate.return_slot_ids
                    || self.return_scalar_counts != candidate.return_scalar_counts
                    || self.return_capability_identities != candidate.return_capability_identities
                {
                    return false;
                }
                let mut selected_seen = false;
                let mut index = 0;
                while index < self.patch_count {
                    if self.patch_ids[index] == Some(selected_patch_id) {
                        selected_seen = true;
                    } else if self.scalar_counts[index] != candidate.scalar_counts[index]
                        || self.engine_capability_identities[index]
                            != candidate.engine_capability_identities[index]
                    {
                        return false;
                    }
                    index += 1;
                }
                selected_seen
            }
            GraphReplacementScope::PatchSlot { patch_id, slot } => {
                if self.scalar_counts != candidate.scalar_counts
                    || self.engine_capability_identities != candidate.engine_capability_identities
                    || self.return_slot_ids != candidate.return_slot_ids
                    || self.return_scalar_counts != candidate.return_scalar_counts
                    || self.return_capability_identities != candidate.return_capability_identities
                {
                    return false;
                }
                let Some(selected_index) = self
                    .patch_ids
                    .iter()
                    .position(|entry| *entry == Some(patch_id))
                else {
                    return false;
                };
                let mut index = 0;
                while index < MAX_PATCHES {
                    let mut position = 0;
                    while position < MAX_EFFECT_SLOTS {
                        let selected_position = index == selected_index && position == slot.index();
                        if !selected_position
                            && (self.effect_slot_ids[index][position]
                                != candidate.effect_slot_ids[index][position]
                                || self.effect_scalar_counts[index][position]
                                    != candidate.effect_scalar_counts[index][position]
                                || self.effect_capability_identities[index][position]
                                    != candidate.effect_capability_identities[index][position])
                        {
                            return false;
                        }
                        position += 1;
                    }
                    index += 1;
                }
                true
            }
            GraphReplacementScope::BusReturn(bus) => {
                if self.scalar_counts != candidate.scalar_counts
                    || self.engine_capability_identities != candidate.engine_capability_identities
                    || self.effect_slot_ids != candidate.effect_slot_ids
                    || self.effect_scalar_counts != candidate.effect_scalar_counts
                    || self.effect_capability_identities != candidate.effect_capability_identities
                {
                    return false;
                }
                let mut index = 0;
                while index < MAX_BUS_RETURNS {
                    if index != bus.index()
                        && (self.return_slot_ids[index] != candidate.return_slot_ids[index]
                            || self.return_scalar_counts[index]
                                != candidate.return_scalar_counts[index]
                            || self.return_capability_identities[index]
                                != candidate.return_capability_identities[index])
                    {
                        return false;
                    }
                    index += 1;
                }
                true
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreparedGraphRefreshError {
    RevisionMismatch,
    LayoutMismatch,
}

impl fmt::Display for PreparedGraphRefreshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RevisionMismatch => {
                "refreshed parameters do not target the prepared graph revision"
            }
            Self::LayoutMismatch => "refreshed parameters do not match the prepared engine layout",
        })
    }
}

impl std::error::Error for PreparedGraphRefreshError {}

impl fmt::Debug for PreparedGraph {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedGraph")
            .field("revision", &self.inner.revision)
            .field("sample_rate", &self.inner.sample_rate)
            .field("max_frames", &self.inner.max_frames)
            .field("initial_parameters", &self.inner.initial_parameters)
            .field("engine_rack", &self.inner.engine_rack)
            .field(
                "patch_audio_max_frames",
                &self.inner.patch_audio.max_frames(),
            )
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GraphReplacementScope, PositionCapabilityIdentity, PreparedGraphLayout,
        MAX_CAPABILITY_IDENTITY_BYTES,
    };
    use crate::kernel::PatchId;
    use crate::mixer::bus_id::{BusId, MAX_BUS_RETURNS};
    use crate::real_time::MAX_PATCHES;
    use crate::synth::capability_id::CapabilityId;
    use crate::synth::effect_slot_id::{EffectSlotIndex, MAX_EFFECT_SLOTS};
    use crate::synth::EffectCapabilityId;
    use crate::synth::EffectSlotId;

    fn identity(value: &str) -> Option<PositionCapabilityIdentity> {
        PositionCapabilityIdentity::from_identifier(value)
    }

    /// Builds one valid namespaced capability identifier of an exact byte
    /// length. Neither `CapabilityId` nor `EffectCapabilityId` bounds length,
    /// so identifiers on either side of the fixed record are constructible.
    fn namespaced_identifier(bytes: usize) -> String {
        const PREFIX: &str = "capability.";
        format!("{PREFIX}{}", "a".repeat(bytes - PREFIX.len()))
    }

    fn layout(scalar_counts: [u8; 2]) -> PreparedGraphLayout {
        let mut patch_ids = [None; MAX_PATCHES];
        patch_ids[0] = PatchId::new(1).ok();
        patch_ids[1] = PatchId::new(2).ok();
        let mut counts = [0; MAX_PATCHES];
        counts[..2].copy_from_slice(&scalar_counts);
        let mut engine_capability_identities = [None; MAX_PATCHES];
        engine_capability_identities[0] = identity("instrument.alpha");
        engine_capability_identities[1] = identity("instrument.beta");
        PreparedGraphLayout {
            sample_rate_bits: 48_000.0_f32.to_bits(),
            max_frames: 512,
            patch_count: 2,
            patch_ids,
            scalar_counts: counts,
            engine_capability_identities,
            effect_slot_ids: [[None; MAX_EFFECT_SLOTS]; MAX_PATCHES],
            effect_scalar_counts: [[0; MAX_EFFECT_SLOTS]; MAX_PATCHES],
            effect_capability_identities: [[None; MAX_EFFECT_SLOTS]; MAX_PATCHES],
            return_slot_ids: [None; MAX_BUS_RETURNS],
            return_scalar_counts: [0; MAX_BUS_RETURNS],
            return_capability_identities: [None; MAX_BUS_RETURNS],
        }
    }

    #[test]
    fn replacement_layout_changes_only_the_selected_patch_scalar_shape() {
        let active = layout([0, 3]);
        assert!(active.permits_selected_replacement(layout([3, 3]), PatchId::new(1).unwrap()));
        assert!(!active.permits_selected_replacement(layout([3, 0]), PatchId::new(1).unwrap()));
        assert!(!active.permits_selected_replacement(layout([3, 3]), PatchId::new(9).unwrap()));

        let mut wrong_order = layout([3, 3]);
        wrong_order.patch_ids.swap(0, 1);
        assert!(!active.permits_selected_replacement(wrong_order, PatchId::new(1).unwrap()));

        let mut wrong_capacity = layout([3, 3]);
        wrong_capacity.max_frames += 1;
        assert!(!active.permits_selected_replacement(wrong_capacity, PatchId::new(1).unwrap()));

        // A replacement may change only the selected Patch's engine scalar
        // shape: any per-position effect-grid change and any return-rack
        // occupancy change is rejected.
        let mut wrong_effect_position = layout([3, 3]);
        wrong_effect_position.effect_slot_ids[0][1] = EffectSlotId::new(4).ok();
        assert!(
            !active.permits_selected_replacement(wrong_effect_position, PatchId::new(1).unwrap())
        );
        let mut wrong_effect_scalars = layout([3, 3]);
        wrong_effect_scalars.effect_scalar_counts[1][2] = 5;
        assert!(
            !active.permits_selected_replacement(wrong_effect_scalars, PatchId::new(1).unwrap())
        );
        let mut wrong_return = layout([3, 3]);
        wrong_return.return_slot_ids[3] = EffectSlotId::new(4).ok();
        assert!(!active.permits_selected_replacement(wrong_return, PatchId::new(1).unwrap()));
        let mut wrong_return_scalars = layout([3, 3]);
        wrong_return_scalars.return_scalar_counts[7] = 2;
        assert!(
            !active.permits_selected_replacement(wrong_return_scalars, PatchId::new(1).unwrap())
        );
    }

    /// The layout records per-position capability identity, and admission is
    /// identity-exact outside the scoped position: a same-scalar-shape
    /// candidate carrying a different capability anywhere the scope leaves
    /// unchanged is refused.
    #[test]
    fn carry_over_capability_identity_layout_admission_is_position_exact() {
        let active = layout([3, 3]);
        let selected = PatchId::new(1).unwrap();

        // The selected engine position may change identity; any other engine
        // position may not, even with an identical scalar shape.
        let mut selected_changed = layout([3, 3]);
        selected_changed.engine_capability_identities[0] = identity("instrument.gamma");
        assert!(active.permits_selected_replacement(selected_changed, selected));
        let mut unselected_changed = layout([3, 3]);
        unselected_changed.engine_capability_identities[1] = identity("instrument.gamma");
        assert!(!active.permits_selected_replacement(unselected_changed, selected));

        // An effect-grid identity change is admitted only at the scoped slot.
        let mut occupied = layout([3, 3]);
        occupied.effect_slot_ids[0][1] = EffectSlotId::new(4).ok();
        occupied.effect_scalar_counts[0][1] = 2;
        occupied.effect_capability_identities[0][1] = identity("effect.chorus");
        let scope = GraphReplacementScope::PatchSlot {
            patch_id: selected,
            slot: EffectSlotIndex::new(1).unwrap(),
        };
        let mut scoped_change = occupied;
        scoped_change.effect_capability_identities[0][1] = identity("effect.reverb");
        assert!(occupied.permits_replacement(scoped_change, scope));
        let mut unscoped_change = occupied;
        unscoped_change.effect_capability_identities[1][2] = identity("effect.reverb");
        assert!(!occupied.permits_replacement(unscoped_change, scope));

        // A bus-return identity change is admitted only at the scoped return.
        let mut returned = layout([3, 3]);
        returned.return_slot_ids[2] = EffectSlotId::new(9).ok();
        returned.return_capability_identities[2] = identity("effect.reverb");
        let return_scope = GraphReplacementScope::BusReturn(BusId::new(2).unwrap());
        let mut scoped_return = returned;
        scoped_return.return_capability_identities[2] = identity("effect.delay");
        assert!(returned.permits_replacement(scoped_return, return_scope));
        let mut unscoped_return = returned;
        unscoped_return.return_capability_identities[5] = identity("effect.delay");
        assert!(!returned.permits_replacement(unscoped_return, return_scope));

        // The recorded identity is exact and readable per position.
        assert_eq!(
            active.engine_capability_identity(0).unwrap().as_str(),
            "instrument.alpha"
        );
        assert_eq!(
            occupied.effect_capability_identity(0, 1).unwrap().as_str(),
            "effect.chorus"
        );
        assert_eq!(occupied.effect_capability_identity(0, 0), None);
        assert_eq!(
            returned.return_capability_identity(2).unwrap().as_str(),
            "effect.reverb"
        );
        assert_eq!(returned.return_capability_identity(3), None);
    }

    /// The fixed identity record is exact at its own edge: an identity of
    /// exactly `MAX_CAPABILITY_IDENTITY_BYTES` is recorded in full, and one
    /// byte beyond it is refused outright rather than truncated.
    ///
    /// Truncation is the specific hazard this pins. The two overlong
    /// identifiers below are distinct capabilities that share their first
    /// `MAX_CAPABILITY_IDENTITY_BYTES` bytes, so a record that truncated
    /// instead of refusing would make them compare equal and silently readmit
    /// a wrong-capability carry-over — the exact hole per-position identity
    /// exists to close. Asserting only the overflow would still pass under a
    /// truncating record that happened to reject overlong input elsewhere, so
    /// both sides of the edge are asserted.
    #[test]
    fn carry_over_capability_identity_records_at_capacity_and_refuses_one_byte_beyond() {
        let at_capacity = namespaced_identifier(MAX_CAPABILITY_IDENTITY_BYTES);
        let first_beyond = format!("{at_capacity}b");
        let second_beyond = format!("{at_capacity}c");
        assert_eq!(at_capacity.len(), MAX_CAPABILITY_IDENTITY_BYTES);
        assert_eq!(first_beyond.len(), MAX_CAPABILITY_IDENTITY_BYTES + 1);
        assert_eq!(second_beyond.len(), MAX_CAPABILITY_IDENTITY_BYTES + 1);
        assert_ne!(first_beyond, second_beyond);
        assert_eq!(
            &first_beyond[..MAX_CAPABILITY_IDENTITY_BYTES],
            &second_beyond[..MAX_CAPABILITY_IDENTITY_BYTES]
        );

        // Engine capability identities: recorded in full at exactly the
        // capacity, refused one byte beyond it.
        let engine = |value: &str| CapabilityId::new(value).unwrap();
        let recorded =
            PositionCapabilityIdentity::from_capability_id(&engine(&at_capacity)).unwrap();
        assert_eq!(recorded.as_str(), at_capacity);
        assert_eq!(
            PositionCapabilityIdentity::from_capability_id(&engine(&first_beyond)),
            None
        );
        assert_eq!(
            PositionCapabilityIdentity::from_capability_id(&engine(&second_beyond)),
            None
        );

        // Effect capability identities record through the same fixed bound.
        let effect = |value: &str| EffectCapabilityId::new(value).unwrap();
        let recorded_effect =
            PositionCapabilityIdentity::from_effect_capability_id(&effect(&at_capacity)).unwrap();
        assert_eq!(recorded_effect.as_str(), at_capacity);
        assert_eq!(
            PositionCapabilityIdentity::from_effect_capability_id(&effect(&first_beyond)),
            None
        );
        assert_eq!(
            PositionCapabilityIdentity::from_effect_capability_id(&effect(&second_beyond)),
            None
        );

        // An empty identity is refused too, so an unoccupied position's
        // `None` can never collide with a recorded identity.
        assert_eq!(identity(""), None);
    }
}
