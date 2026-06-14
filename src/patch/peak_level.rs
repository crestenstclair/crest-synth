// path: src/patch/peak_level.rs

/// A channel's live peak meter level.
///
/// `PeakLevel` represents the peak signal amplitude observed on a channel.
/// A value of `0.0` means silence.  The level reflects the channel's **own**
/// signal and is independent of mute/solo gating — a silenced channel still
/// carries a meaningful `PeakLevel`.
///
/// # Invariants
/// - The inner `f32` value is always non-negative (`>= 0.0`).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PeakLevel(f32);

impl PeakLevel {
    /// The silent (minimum) peak level.
    pub const SILENT: Self = Self(0.0);

    /// Constructs a `PeakLevel` from a raw `f32`.
    ///
    /// Returns an error if `value` is negative or NaN.
    ///
    /// # Errors
    /// Returns `Err(value)` when the value is negative or NaN.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::patch::peak_level::PeakLevel;
    ///
    /// assert!(PeakLevel::try_new(0.0).is_ok());
    /// assert!(PeakLevel::try_new(1.0).is_ok());
    /// assert!(PeakLevel::try_new(-0.1).is_err());
    /// assert!(PeakLevel::try_new(f32::NAN).is_err());
    /// ```
    pub fn try_new(value: f32) -> Result<Self, f32> {
        if value.is_nan() || !(0.0..=f32::INFINITY).contains(&value) {
            Err(value)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the raw `f32` value.
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl Default for PeakLevel {
    fn default() -> Self {
        Self::SILENT
    }
}

impl TryFrom<f32> for PeakLevel {
    type Error = f32;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<PeakLevel> for f32 {
    fn from(peak: PeakLevel) -> f32 {
        peak.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_is_zero() {
        assert_eq!(PeakLevel::SILENT.value(), 0.0);
    }

    #[test]
    fn default_is_silent() {
        assert_eq!(PeakLevel::default(), PeakLevel::SILENT);
    }

    #[test]
    fn zero_is_valid() {
        assert!(PeakLevel::try_new(0.0).is_ok());
    }

    #[test]
    fn positive_is_valid() {
        assert!(PeakLevel::try_new(0.5).is_ok());
        assert!(PeakLevel::try_new(1.0).is_ok());
        assert!(PeakLevel::try_new(10.0).is_ok());
    }

    #[test]
    fn negative_is_rejected() {
        assert!(PeakLevel::try_new(-0.001).is_err());
        assert!(PeakLevel::try_new(-1.0).is_err());
    }

    #[test]
    fn nan_is_rejected() {
        assert!(PeakLevel::try_new(f32::NAN).is_err());
    }

    #[test]
    fn round_trips_via_f32() {
        let level = PeakLevel::try_new(0.75).unwrap();
        let raw: f32 = level.into();
        assert_eq!(raw, 0.75);
    }

    #[test]
    fn try_from_f32_works() {
        assert!(PeakLevel::try_from(0.0).is_ok());
        assert!(PeakLevel::try_from(-1.0).is_err());
    }

    #[test]
    fn ordering_holds() {
        let low = PeakLevel::try_new(0.1).unwrap();
        let high = PeakLevel::try_new(0.9).unwrap();
        assert!(low < high);
    }
}
