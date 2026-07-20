/// Fixed-size measurements from one completed mixer block.
///
/// The value contains only numeric callback-local data. It never owns or
/// borrows mixer buffers and cannot influence rendering decisions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MixObservation {
    left_peak: f32,
    right_peak: f32,
    output_rms: f32,
    reverb_input_rms: f32,
    delay_input_rms: f32,
    wet_output_rms: f32,
    non_finite_samples: u64,
    clipped_samples: u64,
}

impl MixObservation {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
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

#[cfg(test)]
mod tests {
    use super::MixObservation;

    #[test]
    fn mix_observation_is_fixed_size_copyable_numeric_data() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<MixObservation>();
        assert!(!core::mem::needs_drop::<MixObservation>());
        assert_eq!(MixObservation::default().non_finite_samples(), 0);
    }
}
