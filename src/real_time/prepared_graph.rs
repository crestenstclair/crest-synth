use crate::adapter::global_reverb_delay::GlobalReverbDelay;
use crate::kernel::patch_id::PatchId;
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::callback_safety::record_callback_owned_destruction;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use crate::real_time::parameter_snapshot::MAX_PATCHES;
use crate::real_time::patch_audio_block::PatchAudioBlock;
use crate::real_time::prepared_engine_rack::PreparedEngineRack;
use crate::real_time::PreparedPostEffectRack;
use crate::synth::EffectSlotId;
use core::fmt;

/// One complete callback-ready synthesis and mixing topology.
///
/// The graph is the sole owner of prepared instruments, Patch stems, global
/// reverb/delay state, mixer scratch, routing order, and compatible initial
/// scalar parameters for one structural revision.
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
    mixer: MixEngine<GlobalReverbDelay>,
}

pub(crate) struct PreparedGraphResources {
    engine_rack: PreparedEngineRack,
    effect_rack: PreparedPostEffectRack,
    patch_audio: PatchAudioBlock,
    mixer: MixEngine<GlobalReverbDelay>,
}

impl PreparedGraphResources {
    pub(crate) fn new(
        engine_rack: PreparedEngineRack,
        effect_rack: PreparedPostEffectRack,
        patch_audio: PatchAudioBlock,
        mixer: MixEngine<GlobalReverbDelay>,
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
    effect_slot_ids: [Option<EffectSlotId>; MAX_PATCHES],
    effect_scalar_counts: [u8; MAX_PATCHES],
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
        let mut effect_slot_ids = [None; MAX_PATCHES];
        let mut effect_scalar_counts = [0; MAX_PATCHES];
        let mut index = 0;
        while index < self.inner.engine_rack.patch_count() {
            patch_ids[index] = self.inner.engine_rack.patch_id(index);
            scalar_counts[index] =
                self.inner
                    .engine_rack
                    .scalar_count(index)
                    .expect("active rack slots have a Scalar count") as u8;
            effect_slot_ids[index] = self.inner.effect_rack.slot_id(index);
            effect_scalar_counts[index] =
                self.inner.effect_rack.scalar_count(index).unwrap_or(0) as u8;
            index += 1;
        }
        PreparedGraphLayout {
            sample_rate_bits: self.inner.sample_rate.to_bits(),
            max_frames: self.inner.max_frames,
            patch_count: self.inner.engine_rack.patch_count(),
            patch_ids,
            scalar_counts,
            effect_slot_ids,
            effect_scalar_counts,
        }
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
        &mut MixEngine<GlobalReverbDelay>,
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
        &mut MixEngine<GlobalReverbDelay>,
    ) {
        (
            &mut self.inner.engine_rack,
            &mut self.inner.effect_rack,
            &mut self.inner.patch_audio,
            &mut self.inner.mixer,
        )
    }
}

impl Drop for PreparedGraph {
    fn drop(&mut self) {
        record_callback_owned_destruction();
    }
}

impl PreparedGraphLayout {
    /// Admits one selected capability/scalar-layout change and nothing else.
    pub fn permits_selected_replacement(self, candidate: Self, selected_patch_id: PatchId) -> bool {
        if self.sample_rate_bits != candidate.sample_rate_bits
            || self.max_frames != candidate.max_frames
            || self.patch_count != candidate.patch_count
            || self.patch_ids != candidate.patch_ids
            || self.effect_slot_ids != candidate.effect_slot_ids
            || self.effect_scalar_counts != candidate.effect_scalar_counts
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
    use crate::real_time::MAX_PATCHES;

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
            effect_slot_ids: [None; MAX_PATCHES],
            effect_scalar_counts: [0; MAX_PATCHES],
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
    }
}
