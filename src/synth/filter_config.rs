// path: src/synth/filter_config.rs

/// Audible frequency range lower bound in Hz.
const FREQ_MIN_HZ: f64 = 20.0;
/// Audible frequency range upper bound in Hz.
const FREQ_MAX_HZ: f64 = 20_000.0;

/// A frequency value constrained to the audible range (20 Hz – 20 000 Hz).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Frequency(f64);

/// Error returned when a `Frequency` value is out of audible range or NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyError(f64);

impl std::fmt::Display for FrequencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Frequency value {} Hz is outside audible range {}-{} Hz",
            self.0, FREQ_MIN_HZ, FREQ_MAX_HZ
        )
    }
}

impl std::error::Error for FrequencyError {}

impl Frequency {
    /// Construct a `Frequency` from a raw `f64` (Hz).
    ///
    /// Returns `Err` if the value is NaN or outside the audible range
    /// (20.0–20 000.0 Hz inclusive).
    ///
    /// ```
    /// use crest_synth::synth::filter_config::Frequency;
    /// assert!(Frequency::try_new(440.0).is_ok());
    /// assert!(Frequency::try_new(20.0).is_ok());
    /// assert!(Frequency::try_new(20_000.0).is_ok());
    /// assert!(Frequency::try_new(10.0).is_err());
    /// assert!(Frequency::try_new(25_000.0).is_err());
    /// ```
    pub fn try_new(hz: f64) -> Result<Self, FrequencyError> {
        if hz.is_nan() || !(FREQ_MIN_HZ..=FREQ_MAX_HZ).contains(&hz) {
            return Err(FrequencyError(hz));
        }
        Ok(Self(hz))
    }

    /// Return the frequency in Hz.
    #[inline]
    pub fn hz(self) -> f64 {
        self.0
    }
}

impl Default for Frequency {
    /// Returns 1 000 Hz (a neutral mid-range default).
    fn default() -> Self {
        Self(1_000.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// The topology of the resonant filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    /// Low-pass filter — attenuates frequencies above the cutoff.
    #[default]
    LowPass,
    /// High-pass filter — attenuates frequencies below the cutoff.
    HighPass,
    /// Band-pass filter — passes frequencies near the cutoff.
    BandPass,
}

// ─────────────────────────────────────────────────────────────────────────────

/// Error returned when `FilterConfig` construction fails.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterConfigError {
    /// The resonance value was NaN or outside 0.0–1.0.
    InvalidResonance(f64),
    /// The cutoff frequency was NaN or outside the audible range.
    InvalidCutoff(FrequencyError),
}

impl std::fmt::Display for FilterConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterConfigError::InvalidResonance(v) => {
                write!(f, "resonance value {} is out of range 0.0-1.0", v)
            }
            FilterConfigError::InvalidCutoff(e) => write!(f, "invalid cutoff: {}", e),
        }
    }
}

impl std::error::Error for FilterConfigError {}

impl From<FrequencyError> for FilterConfigError {
    fn from(e: FrequencyError) -> Self {
        FilterConfigError::InvalidCutoff(e)
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Resonant filter parameters.
///
/// Invariants:
/// - `resonance` is in the range 0.0–1.0 (inclusive, not NaN).
/// - `cutoff` is within the audible range 20 Hz – 20 000 Hz.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterConfig {
    /// Cutoff frequency in Hz (20–20 000 Hz).
    pub cutoff: Frequency,
    /// Filter topology (low-pass, high-pass, band-pass).
    pub filter_type: FilterType,
    /// Resonance / Q factor (0.0 = no resonance, 1.0 = self-oscillation).
    resonance: f64,
}

impl FilterConfig {
    /// Construct a `FilterConfig`.
    ///
    /// Returns `Err` if `resonance` is NaN or outside 0.0–1.0, or if
    /// `cutoff_hz` is NaN or outside the audible range.
    ///
    /// ```
    /// use crest_synth::synth::filter_config::{FilterConfig, FilterType};
    /// let cfg = FilterConfig::try_new(1_000.0, FilterType::LowPass, 0.5).unwrap();
    /// assert!((cfg.resonance() - 0.5).abs() < f64::EPSILON);
    /// ```
    pub fn try_new(
        cutoff_hz: f64,
        filter_type: FilterType,
        resonance: f64,
    ) -> Result<Self, FilterConfigError> {
        if resonance.is_nan() || !(0.0..=1.0).contains(&resonance) {
            return Err(FilterConfigError::InvalidResonance(resonance));
        }
        let cutoff = Frequency::try_new(cutoff_hz)?;
        Ok(Self {
            cutoff,
            filter_type,
            resonance,
        })
    }

    /// Return the resonance value (0.0–1.0).
    #[inline]
    pub fn resonance(self) -> f64 {
        self.resonance
    }
}

impl Default for FilterConfig {
    /// Returns a neutral low-pass config: cutoff 1 000 Hz, resonance 0.0.
    fn default() -> Self {
        Self {
            cutoff: Frequency::default(),
            filter_type: FilterType::default(),
            resonance: 0.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Frequency ────────────────────────────────────────────────────────────

    #[test]
    fn frequency_accepts_audible_range_boundaries() {
        assert!(Frequency::try_new(20.0).is_ok());
        assert!(Frequency::try_new(20_000.0).is_ok());
    }

    #[test]
    fn frequency_accepts_mid_value() {
        let f = Frequency::try_new(440.0).unwrap();
        assert!((f.hz() - 440.0).abs() < f64::EPSILON);
    }

    #[test]
    fn frequency_rejects_below_audible() {
        assert!(Frequency::try_new(19.99).is_err());
        assert!(Frequency::try_new(0.0).is_err());
    }

    #[test]
    fn frequency_rejects_above_audible() {
        assert!(Frequency::try_new(20_001.0).is_err());
        assert!(Frequency::try_new(44_100.0).is_err());
    }

    #[test]
    fn frequency_rejects_nan() {
        assert!(Frequency::try_new(f64::NAN).is_err());
    }

    #[test]
    fn frequency_rejects_negative() {
        assert!(Frequency::try_new(-100.0).is_err());
    }

    #[test]
    fn frequency_default_is_in_range() {
        let f = Frequency::default();
        assert!((FREQ_MIN_HZ..=FREQ_MAX_HZ).contains(&f.hz()));
    }

    #[test]
    fn frequency_error_display_contains_value() {
        let err = Frequency::try_new(5.0).unwrap_err();
        assert!(err.to_string().contains("5"));
    }

    // ── FilterType ───────────────────────────────────────────────────────────

    #[test]
    fn filter_type_default_is_low_pass() {
        assert_eq!(FilterType::default(), FilterType::LowPass);
    }

    #[test]
    fn filter_type_variants_are_copy() {
        let a = FilterType::HighPass;
        let b = a;
        assert_eq!(a, b);
    }

    // ── FilterConfig ─────────────────────────────────────────────────────────

    #[test]
    fn filter_config_valid_construction() {
        let cfg = FilterConfig::try_new(1_000.0, FilterType::LowPass, 0.5).unwrap();
        assert!((cfg.resonance() - 0.5).abs() < f64::EPSILON);
        assert!((cfg.cutoff.hz() - 1_000.0).abs() < f64::EPSILON);
        assert_eq!(cfg.filter_type, FilterType::LowPass);
    }

    #[test]
    fn filter_config_resonance_zero_is_valid() {
        assert!(FilterConfig::try_new(440.0, FilterType::BandPass, 0.0).is_ok());
    }

    #[test]
    fn filter_config_resonance_one_is_valid() {
        assert!(FilterConfig::try_new(440.0, FilterType::HighPass, 1.0).is_ok());
    }

    #[test]
    fn filter_config_resonance_above_one_rejected() {
        let err = FilterConfig::try_new(440.0, FilterType::LowPass, 1.001).unwrap_err();
        assert!(matches!(err, FilterConfigError::InvalidResonance(_)));
    }

    #[test]
    fn filter_config_resonance_below_zero_rejected() {
        let err = FilterConfig::try_new(440.0, FilterType::LowPass, -0.001).unwrap_err();
        assert!(matches!(err, FilterConfigError::InvalidResonance(_)));
    }

    #[test]
    fn filter_config_resonance_nan_rejected() {
        let err = FilterConfig::try_new(440.0, FilterType::LowPass, f64::NAN).unwrap_err();
        assert!(matches!(err, FilterConfigError::InvalidResonance(_)));
    }

    #[test]
    fn filter_config_cutoff_out_of_range_rejected() {
        let err = FilterConfig::try_new(5.0, FilterType::LowPass, 0.5).unwrap_err();
        assert!(matches!(err, FilterConfigError::InvalidCutoff(_)));
    }

    #[test]
    fn filter_config_cutoff_nan_rejected() {
        let err = FilterConfig::try_new(f64::NAN, FilterType::LowPass, 0.5).unwrap_err();
        assert!(matches!(err, FilterConfigError::InvalidCutoff(_)));
    }

    #[test]
    fn filter_config_default_is_valid() {
        let cfg = FilterConfig::default();
        assert!((0.0..=1.0).contains(&cfg.resonance()));
        assert!((FREQ_MIN_HZ..=FREQ_MAX_HZ).contains(&cfg.cutoff.hz()));
    }

    #[test]
    fn filter_config_copy_semantics() {
        let a = FilterConfig::try_new(2_000.0, FilterType::HighPass, 0.7).unwrap();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn filter_config_error_display_resonance() {
        let err = FilterConfigError::InvalidResonance(2.5);
        assert!(err.to_string().contains("2.5"));
    }

    #[test]
    fn filter_config_boundary_cutoff_20hz() {
        assert!(FilterConfig::try_new(20.0, FilterType::LowPass, 0.0).is_ok());
    }

    #[test]
    fn filter_config_boundary_cutoff_20khz() {
        assert!(FilterConfig::try_new(20_000.0, FilterType::LowPass, 1.0).is_ok());
    }
}
