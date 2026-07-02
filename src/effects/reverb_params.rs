// path: src/effects/reverb_params.rs

//! Algorithmic reverb settings.
//!
//! `ReverbParams` bundles the tunable parameters for an algorithmic reverb
//! effect. All fields are immutable once constructed; `wet_dry` is the sole
//! field carrying a hard invariant (must lie within `0.0..=1.0`), enforced
//! by [`ReverbParams::try_new`].

use std::fmt;

/// Error returned when constructing a [`ReverbParams`] with an out-of-range
/// or non-finite value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReverbParamsError {
    /// `wet_dry` was NaN or outside `0.0..=1.0`.
    WetDryOutOfRange(f64),
}

impl fmt::Display for ReverbParamsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReverbParamsError::WetDryOutOfRange(value) => {
                write!(f, "wet_dry must be within 0.0..=1.0, got {value}")
            }
        }
    }
}

impl std::error::Error for ReverbParamsError {}

/// Algorithmic reverb settings.
///
/// Constructed only through [`ReverbParams::try_new`] so the `wet_dry`
/// invariant can never be bypassed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbParams {
    damping: f64,
    pre_delay_ms: f64,
    room_size: f64,
    wet_dry: f64,
    width: f64,
}

impl ReverbParams {
    /// Constructs a new [`ReverbParams`], validating that `wet_dry` lies
    /// within `0.0..=1.0`.
    ///
    /// # Errors
    ///
    /// Returns [`ReverbParamsError::WetDryOutOfRange`] if `wet_dry` is NaN
    /// or outside `0.0..=1.0`.
    pub fn try_new(
        damping: f64,
        pre_delay_ms: f64,
        room_size: f64,
        wet_dry: f64,
        width: f64,
    ) -> Result<Self, ReverbParamsError> {
        if wet_dry.is_nan() || !(0.0..=1.0).contains(&wet_dry) {
            return Err(ReverbParamsError::WetDryOutOfRange(wet_dry));
        }
        Ok(Self {
            damping,
            pre_delay_ms,
            room_size,
            wet_dry,
            width,
        })
    }

    /// Damping coefficient: higher values roll off high frequencies faster
    /// within the reverb tail.
    pub fn damping(&self) -> f64 {
        self.damping
    }

    /// Pre-delay in milliseconds before the reverb tail begins.
    pub fn pre_delay_ms(&self) -> f64 {
        self.pre_delay_ms
    }

    /// Simulated room size.
    pub fn room_size(&self) -> f64 {
        self.room_size
    }

    /// Wet/dry mix, guaranteed to lie within `0.0..=1.0`.
    pub fn wet_dry(&self) -> f64 {
        self.wet_dry
    }

    /// Stereo width of the reverb tail.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Returns a copy of `self` with `wet_dry` replaced, re-validating the
    /// invariant.
    ///
    /// # Errors
    ///
    /// Returns [`ReverbParamsError::WetDryOutOfRange`] if `wet_dry` is NaN
    /// or outside `0.0..=1.0`.
    pub fn with_wet_dry(&self, wet_dry: f64) -> Result<Self, ReverbParamsError> {
        Self::try_new(
            self.damping,
            self.pre_delay_ms,
            self.room_size,
            wet_dry,
            self.width,
        )
    }

    /// Returns a copy of `self` with `damping` replaced.
    pub fn with_damping(&self, damping: f64) -> Self {
        Self { damping, ..*self }
    }

    /// Returns a copy of `self` with `pre_delay_ms` replaced.
    pub fn with_pre_delay_ms(&self, pre_delay_ms: f64) -> Self {
        Self {
            pre_delay_ms,
            ..*self
        }
    }

    /// Returns a copy of `self` with `room_size` replaced.
    pub fn with_room_size(&self, room_size: f64) -> Self {
        Self { room_size, ..*self }
    }

    /// Returns a copy of `self` with `width` replaced.
    pub fn with_width(&self, width: f64) -> Self {
        Self { width, ..*self }
    }
}

impl Default for ReverbParams {
    /// A neutral, safe default: moderate damping and room size, no
    /// pre-delay, an even wet/dry mix, and full stereo width.
    fn default() -> Self {
        Self {
            damping: 0.5,
            pre_delay_ms: 0.0,
            room_size: 0.5,
            wet_dry: 0.5,
            width: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_wet_dry_within_bounds() {
        let params = ReverbParams::try_new(0.5, 20.0, 0.5, 0.0, 1.0).expect("min bound");
        assert_eq!(params.wet_dry(), 0.0);

        let params = ReverbParams::try_new(0.5, 20.0, 0.5, 1.0, 1.0).expect("max bound");
        assert_eq!(params.wet_dry(), 1.0);
    }

    #[test]
    fn try_new_rejects_wet_dry_below_zero() {
        let err = ReverbParams::try_new(0.5, 20.0, 0.5, -0.01, 1.0).unwrap_err();
        assert_eq!(err, ReverbParamsError::WetDryOutOfRange(-0.01));
    }

    #[test]
    fn try_new_rejects_wet_dry_above_one() {
        let err = ReverbParams::try_new(0.5, 20.0, 0.5, 1.01, 1.0).unwrap_err();
        assert_eq!(err, ReverbParamsError::WetDryOutOfRange(1.01));
    }

    #[test]
    fn try_new_rejects_nan_wet_dry() {
        let err = ReverbParams::try_new(0.5, 20.0, 0.5, f64::NAN, 1.0).unwrap_err();
        assert!(matches!(err, ReverbParamsError::WetDryOutOfRange(v) if v.is_nan()));
    }

    #[test]
    fn accessors_round_trip_all_fields() {
        let params = ReverbParams::try_new(0.3, 15.0, 0.7, 0.4, 0.9).expect("valid params");
        assert_eq!(params.damping(), 0.3);
        assert_eq!(params.pre_delay_ms(), 15.0);
        assert_eq!(params.room_size(), 0.7);
        assert_eq!(params.wet_dry(), 0.4);
        assert_eq!(params.width(), 0.9);
    }

    #[test]
    fn with_wet_dry_revalidates_invariant() {
        let params = ReverbParams::default();
        let updated = params.with_wet_dry(0.2).expect("valid update");
        assert_eq!(updated.wet_dry(), 0.2);

        let err = params.with_wet_dry(2.0).unwrap_err();
        assert_eq!(err, ReverbParamsError::WetDryOutOfRange(2.0));
    }

    #[test]
    fn with_field_helpers_preserve_other_fields() {
        let params = ReverbParams::default();
        let updated = params.with_damping(0.9);
        assert_eq!(updated.damping(), 0.9);
        assert_eq!(updated.wet_dry(), params.wet_dry());
    }

    #[test]
    fn default_is_valid() {
        let params = ReverbParams::default();
        assert!((0.0..=1.0).contains(&params.wet_dry()));
    }
}
