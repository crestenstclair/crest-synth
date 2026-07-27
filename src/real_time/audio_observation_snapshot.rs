use crate::kernel::patch_id::PatchId;
use crate::mixer::mix_observation::MixObservation;
use crate::real_time::{GraphRevision, PatchEffectObservation};
use serde::Serialize;

/// Fixed-size numeric evidence from one completed real-time render block.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioObservationSnapshot {
    sequence: u64,
    rendered_blocks: u64,
    rendered_frames: u64,
    parameter_generation: u64,
    active_graph_revision: GraphRevision,
    commands_consumed: u64,
    active_notes: u32,
    routing_failures: u64,
    last_unknown_patch_id: Option<PatchId>,
    primary_patch_id: Option<PatchId>,
    primary_patch_rms: f32,
    primary_active_notes: u32,
    patch_effect: PatchEffectObservation,
    left_peak: f32,
    right_peak: f32,
    output_rms: f32,
    reverb_input_rms: f32,
    delay_input_rms: f32,
    wet_output_rms: f32,
    non_finite_samples: u64,
    clipped_samples: u64,
}

impl AudioObservationSnapshot {
    pub const fn from_mix(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        commands_consumed: u64,
        active_notes: u32,
        mix: MixObservation,
    ) -> Self {
        Self::from_mix_with_routing(
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            commands_consumed,
            active_notes,
            0,
            None,
            mix,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn from_mix_with_routing(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        commands_consumed: u64,
        active_notes: u32,
        routing_failures: u64,
        last_unknown_patch_id: Option<PatchId>,
        mix: MixObservation,
    ) -> Self {
        Self::from_mix_with_graph_and_routing(
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            GraphRevision::INITIAL,
            commands_consumed,
            active_notes,
            routing_failures,
            last_unknown_patch_id,
            mix,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn from_mix_with_graph_and_routing(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        active_graph_revision: GraphRevision,
        commands_consumed: u64,
        active_notes: u32,
        routing_failures: u64,
        last_unknown_patch_id: Option<PatchId>,
        mix: MixObservation,
    ) -> Self {
        Self::from_mix_with_graph_routing_and_primary(
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            active_graph_revision,
            commands_consumed,
            active_notes,
            routing_failures,
            last_unknown_patch_id,
            None,
            0.0,
            0,
            mix,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn from_mix_with_graph_routing_and_primary(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        active_graph_revision: GraphRevision,
        commands_consumed: u64,
        active_notes: u32,
        routing_failures: u64,
        last_unknown_patch_id: Option<PatchId>,
        primary_patch_id: Option<PatchId>,
        primary_patch_rms: f32,
        primary_active_notes: u32,
        mix: MixObservation,
    ) -> Self {
        Self::from_mix_with_graph_routing_primary_and_effect(
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            active_graph_revision,
            commands_consumed,
            active_notes,
            routing_failures,
            last_unknown_patch_id,
            primary_patch_id,
            primary_patch_rms,
            primary_active_notes,
            PatchEffectObservation::EMPTY,
            mix,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn from_mix_with_graph_routing_primary_and_effect(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        active_graph_revision: GraphRevision,
        commands_consumed: u64,
        active_notes: u32,
        routing_failures: u64,
        last_unknown_patch_id: Option<PatchId>,
        primary_patch_id: Option<PatchId>,
        primary_patch_rms: f32,
        primary_active_notes: u32,
        patch_effect: PatchEffectObservation,
        mix: MixObservation,
    ) -> Self {
        Self {
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            active_graph_revision,
            commands_consumed,
            active_notes,
            routing_failures,
            last_unknown_patch_id,
            primary_patch_id,
            primary_patch_rms,
            primary_active_notes,
            patch_effect,
            left_peak: mix.left_peak(),
            right_peak: mix.right_peak(),
            output_rms: mix.output_rms(),
            reverb_input_rms: mix.reverb_input_rms(),
            delay_input_rms: mix.delay_input_rms(),
            wet_output_rms: mix.wet_output_rms(),
            non_finite_samples: mix.non_finite_samples(),
            clipped_samples: mix.clipped_samples(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(crate) const fn from_parts(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        commands_consumed: u64,
        active_notes: u32,
        left_peak: f32,
        right_peak: f32,
        output_rms: f32,
        reverb_input_rms: f32,
        delay_input_rms: f32,
        wet_output_rms: f32,
        non_finite_samples: u64,
        clipped_samples: u64,
    ) -> Self {
        Self::from_parts_with_routing(
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            commands_consumed,
            active_notes,
            0,
            None,
            left_peak,
            right_peak,
            output_rms,
            reverb_input_rms,
            delay_input_rms,
            wet_output_rms,
            non_finite_samples,
            clipped_samples,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(crate) const fn from_parts_with_routing(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        commands_consumed: u64,
        active_notes: u32,
        routing_failures: u64,
        last_unknown_patch_id: Option<PatchId>,
        left_peak: f32,
        right_peak: f32,
        output_rms: f32,
        reverb_input_rms: f32,
        delay_input_rms: f32,
        wet_output_rms: f32,
        non_finite_samples: u64,
        clipped_samples: u64,
    ) -> Self {
        Self::from_parts_with_graph_and_routing(
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            GraphRevision::INITIAL,
            commands_consumed,
            active_notes,
            routing_failures,
            last_unknown_patch_id,
            left_peak,
            right_peak,
            output_rms,
            reverb_input_rms,
            delay_input_rms,
            wet_output_rms,
            non_finite_samples,
            clipped_samples,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_parts_with_graph_and_routing(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        active_graph_revision: GraphRevision,
        commands_consumed: u64,
        active_notes: u32,
        routing_failures: u64,
        last_unknown_patch_id: Option<PatchId>,
        left_peak: f32,
        right_peak: f32,
        output_rms: f32,
        reverb_input_rms: f32,
        delay_input_rms: f32,
        wet_output_rms: f32,
        non_finite_samples: u64,
        clipped_samples: u64,
    ) -> Self {
        Self::from_parts_with_graph_routing_and_primary(
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            active_graph_revision,
            commands_consumed,
            active_notes,
            routing_failures,
            last_unknown_patch_id,
            None,
            0.0,
            0,
            left_peak,
            right_peak,
            output_rms,
            reverb_input_rms,
            delay_input_rms,
            wet_output_rms,
            non_finite_samples,
            clipped_samples,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_parts_with_graph_routing_and_primary(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        active_graph_revision: GraphRevision,
        commands_consumed: u64,
        active_notes: u32,
        routing_failures: u64,
        last_unknown_patch_id: Option<PatchId>,
        primary_patch_id: Option<PatchId>,
        primary_patch_rms: f32,
        primary_active_notes: u32,
        left_peak: f32,
        right_peak: f32,
        output_rms: f32,
        reverb_input_rms: f32,
        delay_input_rms: f32,
        wet_output_rms: f32,
        non_finite_samples: u64,
        clipped_samples: u64,
    ) -> Self {
        Self::from_parts_with_graph_routing_primary_and_effect(
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            active_graph_revision,
            commands_consumed,
            active_notes,
            routing_failures,
            last_unknown_patch_id,
            primary_patch_id,
            primary_patch_rms,
            primary_active_notes,
            PatchEffectObservation::EMPTY,
            left_peak,
            right_peak,
            output_rms,
            reverb_input_rms,
            delay_input_rms,
            wet_output_rms,
            non_finite_samples,
            clipped_samples,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_parts_with_graph_routing_primary_and_effect(
        sequence: u64,
        rendered_blocks: u64,
        rendered_frames: u64,
        parameter_generation: u64,
        active_graph_revision: GraphRevision,
        commands_consumed: u64,
        active_notes: u32,
        routing_failures: u64,
        last_unknown_patch_id: Option<PatchId>,
        primary_patch_id: Option<PatchId>,
        primary_patch_rms: f32,
        primary_active_notes: u32,
        patch_effect: PatchEffectObservation,
        left_peak: f32,
        right_peak: f32,
        output_rms: f32,
        reverb_input_rms: f32,
        delay_input_rms: f32,
        wet_output_rms: f32,
        non_finite_samples: u64,
        clipped_samples: u64,
    ) -> Self {
        Self {
            sequence,
            rendered_blocks,
            rendered_frames,
            parameter_generation,
            active_graph_revision,
            commands_consumed,
            active_notes,
            routing_failures,
            last_unknown_patch_id,
            primary_patch_id,
            primary_patch_rms,
            primary_active_notes,
            patch_effect,
            left_peak,
            right_peak,
            output_rms,
            reverb_input_rms,
            delay_input_rms,
            wet_output_rms,
            non_finite_samples,
            clipped_samples,
        }
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }
    pub const fn rendered_blocks(self) -> u64 {
        self.rendered_blocks
    }
    pub const fn rendered_frames(self) -> u64 {
        self.rendered_frames
    }
    pub const fn parameter_generation(self) -> u64 {
        self.parameter_generation
    }
    pub const fn active_graph_revision(self) -> GraphRevision {
        self.active_graph_revision
    }
    pub const fn commands_consumed(self) -> u64 {
        self.commands_consumed
    }
    pub const fn active_notes(self) -> u32 {
        self.active_notes
    }
    pub const fn routing_failures(self) -> u64 {
        self.routing_failures
    }
    pub const fn last_unknown_patch_id(self) -> Option<PatchId> {
        self.last_unknown_patch_id
    }
    pub const fn primary_patch_id(self) -> Option<PatchId> {
        self.primary_patch_id
    }
    pub const fn primary_patch_rms(self) -> f32 {
        self.primary_patch_rms
    }
    pub const fn primary_active_notes(self) -> u32 {
        self.primary_active_notes
    }
    pub const fn patch_effect(self) -> PatchEffectObservation {
        self.patch_effect
    }
    pub const fn left_peak(self) -> f32 {
        self.left_peak
    }
    pub const fn right_peak(self) -> f32 {
        self.right_peak
    }
    pub const fn output_rms(self) -> f32 {
        self.output_rms
    }
    pub const fn reverb_input_rms(self) -> f32 {
        self.reverb_input_rms
    }
    pub const fn delay_input_rms(self) -> f32 {
        self.delay_input_rms
    }
    pub const fn wet_output_rms(self) -> f32 {
        self.wet_output_rms
    }
    pub const fn non_finite_samples(self) -> u64 {
        self.non_finite_samples
    }
    pub const fn clipped_samples(self) -> u64 {
        self.clipped_samples
    }
}

impl Default for AudioObservationSnapshot {
    fn default() -> Self {
        Self::from_parts_with_graph_and_routing(
            0,
            0,
            0,
            0,
            GraphRevision::INITIAL,
            0,
            0,
            0,
            None,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AudioObservationSnapshot;

    #[test]
    fn snapshot_is_copyable_fixed_size_numeric_data() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<AudioObservationSnapshot>();
        assert!(!core::mem::needs_drop::<AudioObservationSnapshot>());
        assert_eq!(AudioObservationSnapshot::default().sequence(), 0);
    }
}
