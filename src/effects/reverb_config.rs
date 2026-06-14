// path: src/effects/reverb_config.rs

/// Reverb parameters.
///
/// Invariants enforced at construction:
/// - `room_size`, `damping`, and `dry_wet` must be in `[0.0, 1.0]`
/// - `pre_delay` must be non-negative (`>= 0.0`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbConfig {
    room_size: f64,
    damping: f64,
    dry_wet: f64,
    pre_delay: f64,
}

/// Error returned when [`ReverbConfig`] construction fails due to an
/// out-of-range argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReverbConfigError {
    /// `room_size` is outside `[0.0, 1.0]`.
    RoomSizeOutOfRange,
    /// `damping` is outside `[0.0, 1.0]`.
    DampingOutOfRange,
    /// `dry_wet` is outside `[0.0, 1.0]`.
    DryWetOutOfRange,
    /// `pre_delay` is negative.
    PreDelayNegative,
}

impl std::fmt::Display for ReverbConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoomSizeOutOfRange => write!(f, "room_size must be in [0.0, 1.0]"),
            Self::DampingOutOfRange => write!(f, "damping must be in [0.0, 1.0]"),
            Self::DryWetOutOfRange => write!(f, "dry_wet must be in [0.0, 1.0]"),
            Self::PreDelayNegative => write!(f, "pre_delay must be non-negative"),
        }
    }
}

impl std::error::Error for ReverbConfigError {}

impl ReverbConfig {
    /// Constructs a [`ReverbConfig`] with default parameters:
    /// `room_size = 0.5`, `damping = 0.5`, `dry_wet = 0.3`, `pre_delay = 0.0`.
    pub fn new() -> Self {
        // Safety: all defaults are within their valid ranges.
        Self {
            room_size: 0.5,
            damping: 0.5,
            dry_wet: 0.3,
            pre_delay: 0.0,
        }
    }

    /// Constructs a [`ReverbConfig`], validating every parameter.
    ///
    /// # Errors
    ///
    /// Returns an error if any invariant is violated:
    /// - `room_size`, `damping`, `dry_wet` must be in `[0.0, 1.0]`
    /// - `pre_delay` must be `>= 0.0`
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::effects::reverb_config::ReverbConfig;
    ///
    /// let cfg = ReverbConfig::try_new(0.8, 0.4, 0.5, 0.02).unwrap();
    /// assert_eq!(cfg.room_size(), 0.8);
    /// ```
    pub fn try_new(
        room_size: f64,
        damping: f64,
        dry_wet: f64,
        pre_delay: f64,
    ) -> Result<Self, ReverbConfigError> {
        if room_size.is_nan() || !(0.0..=1.0).contains(&room_size) {
            return Err(ReverbConfigError::RoomSizeOutOfRange);
        }
        if damping.is_nan() || !(0.0..=1.0).contains(&damping) {
            return Err(ReverbConfigError::DampingOutOfRange);
        }
        if dry_wet.is_nan() || !(0.0..=1.0).contains(&dry_wet) {
            return Err(ReverbConfigError::DryWetOutOfRange);
        }
        if pre_delay.is_nan() || pre_delay < 0.0 {
            return Err(ReverbConfigError::PreDelayNegative);
        }
        Ok(Self {
            room_size,
            damping,
            dry_wet,
            pre_delay,
        })
    }

    /// Returns the room size in `[0.0, 1.0]`.
    pub fn room_size(&self) -> f64 {
        self.room_size
    }

    /// Returns the damping in `[0.0, 1.0]`.
    pub fn damping(&self) -> f64 {
        self.damping
    }

    /// Returns the dry/wet mix in `[0.0, 1.0]`.
    pub fn dry_wet(&self) -> f64 {
        self.dry_wet
    }

    /// Returns the pre-delay (seconds), always `>= 0.0`.
    pub fn pre_delay(&self) -> f64 {
        self.pre_delay
    }
}

impl Default for ReverbConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod reverb_config_tests {
    use super::*;

    // ── construction ──────────────────────────────────────────────────────

    #[test]
    fn default_values_are_valid() {
        let cfg = ReverbConfig::new();
        assert!((0.0..=1.0).contains(&cfg.room_size()));
        assert!((0.0..=1.0).contains(&cfg.damping()));
        assert!((0.0..=1.0).contains(&cfg.dry_wet()));
        assert!(cfg.pre_delay() >= 0.0);
    }

    #[test]
    fn try_new_accepts_boundary_values() {
        ReverbConfig::try_new(0.0, 0.0, 0.0, 0.0).expect("all-zero boundary must be valid");
        ReverbConfig::try_new(1.0, 1.0, 1.0, 100.0).expect("all-one/max boundary must be valid");
    }

    #[test]
    fn try_new_rejects_room_size_below_zero() {
        assert_eq!(
            ReverbConfig::try_new(-0.01, 0.5, 0.5, 0.0),
            Err(ReverbConfigError::RoomSizeOutOfRange)
        );
    }

    #[test]
    fn try_new_rejects_room_size_above_one() {
        assert_eq!(
            ReverbConfig::try_new(1.01, 0.5, 0.5, 0.0),
            Err(ReverbConfigError::RoomSizeOutOfRange)
        );
    }

    #[test]
    fn try_new_rejects_room_size_nan() {
        assert_eq!(
            ReverbConfig::try_new(f64::NAN, 0.5, 0.5, 0.0),
            Err(ReverbConfigError::RoomSizeOutOfRange)
        );
    }

    #[test]
    fn try_new_rejects_damping_out_of_range() {
        assert_eq!(
            ReverbConfig::try_new(0.5, -0.1, 0.5, 0.0),
            Err(ReverbConfigError::DampingOutOfRange)
        );
        assert_eq!(
            ReverbConfig::try_new(0.5, 1.1, 0.5, 0.0),
            Err(ReverbConfigError::DampingOutOfRange)
        );
    }

    #[test]
    fn try_new_rejects_damping_nan() {
        assert_eq!(
            ReverbConfig::try_new(0.5, f64::NAN, 0.5, 0.0),
            Err(ReverbConfigError::DampingOutOfRange)
        );
    }

    #[test]
    fn try_new_rejects_dry_wet_out_of_range() {
        assert_eq!(
            ReverbConfig::try_new(0.5, 0.5, -0.01, 0.0),
            Err(ReverbConfigError::DryWetOutOfRange)
        );
        assert_eq!(
            ReverbConfig::try_new(0.5, 0.5, 1.01, 0.0),
            Err(ReverbConfigError::DryWetOutOfRange)
        );
    }

    #[test]
    fn try_new_rejects_dry_wet_nan() {
        assert_eq!(
            ReverbConfig::try_new(0.5, 0.5, f64::NAN, 0.0),
            Err(ReverbConfigError::DryWetOutOfRange)
        );
    }

    #[test]
    fn try_new_rejects_negative_pre_delay() {
        assert_eq!(
            ReverbConfig::try_new(0.5, 0.5, 0.5, -0.001),
            Err(ReverbConfigError::PreDelayNegative)
        );
    }

    #[test]
    fn try_new_rejects_pre_delay_nan() {
        assert_eq!(
            ReverbConfig::try_new(0.5, 0.5, 0.5, f64::NAN),
            Err(ReverbConfigError::PreDelayNegative)
        );
    }

    // ── accessors ─────────────────────────────────────────────────────────

    #[test]
    fn accessors_return_constructed_values() {
        let cfg = ReverbConfig::try_new(0.8, 0.3, 0.6, 0.02).unwrap();
        assert_eq!(cfg.room_size(), 0.8);
        assert_eq!(cfg.damping(), 0.3);
        assert_eq!(cfg.dry_wet(), 0.6);
        assert_eq!(cfg.pre_delay(), 0.02);
    }

    // ── derived traits ────────────────────────────────────────────────────

    #[test]
    fn clone_and_copy_work() {
        let a = ReverbConfig::try_new(0.7, 0.2, 0.4, 0.01).unwrap();
        let b = a; // Copy
        let c = a; // Clone
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(ReverbConfig::default(), ReverbConfig::new());
    }

    // ── error display ─────────────────────────────────────────────────────

    #[test]
    fn error_display_is_human_readable() {
        assert!(ReverbConfigError::RoomSizeOutOfRange
            .to_string()
            .contains("room_size"));
        assert!(ReverbConfigError::DampingOutOfRange
            .to_string()
            .contains("damping"));
        assert!(ReverbConfigError::DryWetOutOfRange
            .to_string()
            .contains("dry_wet"));
        assert!(ReverbConfigError::PreDelayNegative
            .to_string()
            .contains("pre_delay"));
    }
}
