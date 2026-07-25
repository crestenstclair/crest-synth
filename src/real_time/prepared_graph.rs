use crate::adapter::global_reverb_delay::GlobalReverbDelay;
use crate::kernel::patch_id::PatchId;
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use crate::real_time::parameter_snapshot::MAX_PATCHES;
use crate::real_time::patch_audio_block::PatchAudioBlock;
use crate::real_time::prepared_engine_rack::PreparedEngineRack;
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
    patch_audio: PatchAudioBlock,
    mixer: MixEngine<GlobalReverbDelay>,
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
}

impl PreparedGraph {
    pub(crate) fn new(
        revision: GraphRevision,
        sample_rate: f32,
        max_frames: usize,
        initial_parameters: ParameterSnapshot,
        engine_rack: PreparedEngineRack,
        patch_audio: PatchAudioBlock,
        mixer: MixEngine<GlobalReverbDelay>,
    ) -> Self {
        Self {
            inner: Box::new(PreparedGraphState {
                revision,
                sample_rate,
                max_frames,
                initial_parameters,
                engine_rack,
                patch_audio,
                mixer,
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

    /// Returns the fixed replacement contract without borrowing graph-owned
    /// engine, effect, or scratch state.
    pub fn layout(&self) -> PreparedGraphLayout {
        let mut patch_ids = [None; MAX_PATCHES];
        let mut index = 0;
        while index < self.inner.engine_rack.patch_count() {
            patch_ids[index] = self.inner.engine_rack.patch_id(index);
            index += 1;
        }
        PreparedGraphLayout {
            sample_rate_bits: self.inner.sample_rate.to_bits(),
            max_frames: self.inner.max_frames,
            patch_count: self.inner.engine_rack.patch_count(),
            patch_ids,
        }
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
}

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
