use crate::mixer::global_parameters::GlobalParameters;
use core::fmt;

/// Failures that can occur while preparing global effect storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectError {
    /// The sample rate is not finite and strictly positive.
    InvalidSampleRate,
    /// The configured maximum block size is zero.
    InvalidMaxFrames,
    /// The maximum delay is not finite and strictly positive.
    InvalidMaxDelayMilliseconds,
    /// The implementation could not reserve all storage before processing.
    StorageAllocationFailed,
}

impl fmt::Display for EffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidSampleRate => "sample rate must be finite and greater than zero",
            Self::InvalidMaxFrames => "maximum frame count must be greater than zero",
            Self::InvalidMaxDelayMilliseconds => {
                "maximum delay must be finite and greater than zero milliseconds"
            }
            Self::StorageAllocationFailed => "global effect storage allocation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for EffectError {}

/// Replaceable boundary for the single shared reverb and single shared delay.
///
/// Implementations allocate every delay line and scratch buffer in prepare.
/// process runs on the audio thread and must remain allocation-free, lock-free,
/// non-blocking, and free of I/O, logging, and destruction.
pub trait GlobalEffectsProcessor {
    /// Allocates and initializes all storage required by process.
    fn prepare(
        &mut self,
        sample_rate: f32,
        max_frames: usize,
        max_delay_milliseconds: f32,
    ) -> Result<(), EffectError>;

    /// Adds the one shared reverb and one shared delay return to output.
    ///
    /// Inputs and output are interleaved stereo samples. Implementations use only
    /// storage established by prepare. Production and verification implementations
    /// derive wet excitation exclusively from `reverb_input` and `delay_input`.
    /// They never treat samples already in `output` as an implicit send, and zero
    /// effect inputs cannot create a wet return.
    fn process(
        &mut self,
        reverb_input: &[f32],
        delay_input: &[f32],
        output: &mut [f32],
        parameters: &GlobalParameters,
    );
}

#[cfg(test)]
mod tests {
    use super::{EffectError, GlobalEffectsProcessor};
    use crate::mixer::global_parameters::GlobalParameters;

    struct TestProcessor;

    impl GlobalEffectsProcessor for TestProcessor {
        fn prepare(
            &mut self,
            _sample_rate: f32,
            _max_frames: usize,
            _max_delay_milliseconds: f32,
        ) -> Result<(), EffectError> {
            Ok(())
        }

        fn process(
            &mut self,
            _reverb_input: &[f32],
            _delay_input: &[f32],
            _output: &mut [f32],
            _parameters: &GlobalParameters,
        ) {
        }
    }

    fn accept_trait_object(_processor: &mut dyn GlobalEffectsProcessor) {}

    #[test]
    fn processor_contract_is_object_safe() {
        let mut processor = TestProcessor;
        accept_trait_object(&mut processor);
    }

    #[test]
    fn preparation_errors_have_actionable_messages() {
        assert_eq!(
            EffectError::InvalidSampleRate.to_string(),
            "sample rate must be finite and greater than zero"
        );
        assert_eq!(
            EffectError::InvalidMaxFrames.to_string(),
            "maximum frame count must be greater than zero"
        );
        assert_eq!(
            EffectError::InvalidMaxDelayMilliseconds.to_string(),
            "maximum delay must be finite and greater than zero milliseconds"
        );
        assert_eq!(
            EffectError::StorageAllocationFailed.to_string(),
            "global effect storage allocation failed"
        );
    }
}
