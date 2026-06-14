// path: src/kernel/sample_rate.rs

/// Audio sample rate in Hz.
///
/// # Invariants
/// - Must be positive (> 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SampleRate(u32);

impl SampleRate {
    /// Create a new `SampleRate`.
    ///
    /// Returns `Err` if `value` is zero.
    ///
    /// # Example
    ///
    /// ```
    /// use crest_synth::kernel::sample_rate::SampleRate;
    ///
    /// assert!(SampleRate::try_new(44100).is_ok());
    /// assert!(SampleRate::try_new(0).is_err());
    /// ```
    pub fn try_new(value: u32) -> Result<Self, &'static str> {
        if value == 0 {
            return Err("SampleRate must be positive (> 0)");
        }
        Ok(Self(value))
    }

    /// Returns the sample rate value in Hz.
    pub fn value(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for SampleRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_sample_rates_are_accepted() {
        assert!(SampleRate::try_new(44100).is_ok());
        assert!(SampleRate::try_new(48000).is_ok());
        assert!(SampleRate::try_new(96000).is_ok());
        assert!(SampleRate::try_new(1).is_ok());
        assert!(SampleRate::try_new(u32::MAX).is_ok());
    }

    #[test]
    fn zero_is_rejected() {
        assert!(SampleRate::try_new(0).is_err());
    }

    #[test]
    fn value_roundtrips() {
        let sr = SampleRate::try_new(44100).unwrap();
        assert_eq!(sr.value(), 44100);
    }

    #[test]
    fn ordering_works() {
        let low = SampleRate::try_new(44100).unwrap();
        let high = SampleRate::try_new(96000).unwrap();
        assert!(low < high);
    }

    #[test]
    fn display_includes_hz() {
        let sr = SampleRate::try_new(48000).unwrap();
        assert_eq!(sr.to_string(), "48000 Hz");
    }
}
