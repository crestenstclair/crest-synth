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
use crate::synth::effect_slot_id::MAX_EFFECT_SLOTS;
use crate::synth::EffectSlotId;
use core::fmt;

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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedGraphLayout {
    sample_rate_bits: u32,
    max_frames: usize,
    patch_count: usize,
    patch_ids: [Option<PatchId>; MAX_PATCHES],
    scalar_counts: [u8; MAX_PATCHES],
    effect_slot_ids: [[Option<EffectSlotId>; MAX_EFFECT_SLOTS]; MAX_PATCHES],
    effect_scalar_counts: [[u8; MAX_EFFECT_SLOTS]; MAX_PATCHES],
    return_slot_ids: [Option<EffectSlotId>; MAX_BUS_RETURNS],
    return_scalar_counts: [u8; MAX_BUS_RETURNS],
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
        let mut effect_slot_ids = [[None; MAX_EFFECT_SLOTS]; MAX_PATCHES];
        let mut effect_scalar_counts = [[0; MAX_EFFECT_SLOTS]; MAX_PATCHES];
        let mut index = 0;
        while index < self.inner.engine_rack.patch_count() {
            patch_ids[index] = self.inner.engine_rack.patch_id(index);
            scalar_counts[index] =
                self.inner
                    .engine_rack
                    .scalar_count(index)
                    .expect("active rack slots have a Scalar count") as u8;
            let mut position = 0;
            while position < MAX_EFFECT_SLOTS {
                effect_slot_ids[index][position] =
                    self.inner.effect_rack.slot_id_at(index, position);
                effect_scalar_counts[index][position] = self
                    .inner
                    .effect_rack
                    .scalar_count_at(index, position)
                    .unwrap_or(0) as u8;
                position += 1;
            }
            index += 1;
        }
        let mut return_slot_ids = [None; MAX_BUS_RETURNS];
        let mut return_scalar_counts = [0; MAX_BUS_RETURNS];
        for bus in BusId::ALL {
            return_slot_ids[bus.index()] = self.inner.mixer.bus_returns().slot_id(bus);
            return_scalar_counts[bus.index()] = self
                .inner
                .mixer
                .bus_returns()
                .scalar_count(bus)
                .unwrap_or(0) as u8;
        }
        PreparedGraphLayout {
            sample_rate_bits: self.inner.sample_rate.to_bits(),
            max_frames: self.inner.max_frames,
            patch_count: self.inner.engine_rack.patch_count(),
            patch_ids,
            scalar_counts,
            effect_slot_ids,
            effect_scalar_counts,
            return_slot_ids,
            return_scalar_counts,
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

    /// WP10 voice carry-over (T057 mechanism decision, DIRECTIVE_003).
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
    /// owning pointer guarded by exact identity agreement — no allocation, no
    /// deallocation, no locking, no blocking, no destruction. The fresh
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
    /// Admits one selected capability/scalar-layout change and nothing else.
    pub fn permits_selected_replacement(self, candidate: Self, selected_patch_id: PatchId) -> bool {
        self.permits_replacement(
            candidate,
            GraphReplacementScope::SelectedEngine(selected_patch_id),
        )
    }

    /// Admits exactly the layout delta the scope declares and nothing else.
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
                    || self.return_slot_ids != candidate.return_slot_ids
                    || self.return_scalar_counts != candidate.return_scalar_counts
                {
                    return false;
                }
                let mut selected_seen = false;
                let mut index = 0;
                while index < self.patch_count {
                    if self.patch_ids[index] == Some(selected_patch_id) {
                        selected_seen = true;
                    } else if self.scalar_counts[index] != candidate.scalar_counts[index] {
                        return false;
                    }
                    index += 1;
                }
                selected_seen
            }
            GraphReplacementScope::PatchSlot { patch_id, slot } => {
                if self.scalar_counts != candidate.scalar_counts
                    || self.return_slot_ids != candidate.return_slot_ids
                    || self.return_scalar_counts != candidate.return_scalar_counts
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
                                    != candidate.effect_scalar_counts[index][position])
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
                    || self.effect_slot_ids != candidate.effect_slot_ids
                    || self.effect_scalar_counts != candidate.effect_scalar_counts
                {
                    return false;
                }
                let mut index = 0;
                while index < MAX_BUS_RETURNS {
                    if index != bus.index()
                        && (self.return_slot_ids[index] != candidate.return_slot_ids[index]
                            || self.return_scalar_counts[index]
                                != candidate.return_scalar_counts[index])
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
    use super::PreparedGraphLayout;
    use crate::kernel::PatchId;
    use crate::mixer::bus_id::MAX_BUS_RETURNS;
    use crate::real_time::MAX_PATCHES;
    use crate::synth::effect_slot_id::MAX_EFFECT_SLOTS;
    use crate::synth::EffectSlotId;

    fn layout(scalar_counts: [u8; 2]) -> PreparedGraphLayout {
        let mut patch_ids = [None; MAX_PATCHES];
        patch_ids[0] = PatchId::new(1).ok();
        patch_ids[1] = PatchId::new(2).ok();
        let mut counts = [0; MAX_PATCHES];
        counts[..2].copy_from_slice(&scalar_counts);
        PreparedGraphLayout {
            sample_rate_bits: 48_000.0_f32.to_bits(),
            max_frames: 512,
            patch_count: 2,
            patch_ids,
            scalar_counts: counts,
            effect_slot_ids: [[None; MAX_EFFECT_SLOTS]; MAX_PATCHES],
            effect_scalar_counts: [[0; MAX_EFFECT_SLOTS]; MAX_PATCHES],
            return_slot_ids: [None; MAX_BUS_RETURNS],
            return_scalar_counts: [0; MAX_BUS_RETURNS],
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
}
