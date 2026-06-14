// path: src/sample_library/key_velocity_range.rs

use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;

/// The note and velocity range a sample zone responds to.
///
/// `KeyVelocityRange` enforces that `key_low <= key_high` and
/// `velocity_low <= velocity_high`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyVelocityRange {
    key_low: NoteNumber,
    key_high: NoteNumber,
    velocity_low: Velocity,
    velocity_high: Velocity,
}

/// Error returned when constructing an invalid `KeyVelocityRange`.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyVelocityRangeError {
    /// `key_low` is greater than `key_high`.
    KeyRangeInverted,
    /// `velocity_low` is greater than `velocity_high`.
    VelocityRangeInverted,
}

impl std::fmt::Display for KeyVelocityRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyVelocityRangeError::KeyRangeInverted => {
                write!(f, "key_low must be <= key_high")
            }
            KeyVelocityRangeError::VelocityRangeInverted => {
                write!(f, "velocity_low must be <= velocity_high")
            }
        }
    }
}

impl std::error::Error for KeyVelocityRangeError {}

impl KeyVelocityRange {
    /// Construct a `KeyVelocityRange`.
    ///
    /// Returns `Err(KeyVelocityRangeError::KeyRangeInverted)` if `key_low > key_high`.
    /// Returns `Err(KeyVelocityRangeError::VelocityRangeInverted)` if `velocity_low > velocity_high`.
    ///
    /// ```
    /// use crest_synth::kernel::note_number::NoteNumber;
    /// use crest_synth::kernel::velocity::Velocity;
    /// use crest_synth::sample_library::key_velocity_range::KeyVelocityRange;
    ///
    /// let low = NoteNumber::try_new(60).unwrap();
    /// let high = NoteNumber::try_new(72).unwrap();
    /// let vel_lo = Velocity::try_new(0.0).unwrap();
    /// let vel_hi = Velocity::try_new(1.0).unwrap();
    /// assert!(KeyVelocityRange::try_new(low, high, vel_lo, vel_hi).is_ok());
    /// ```
    pub fn try_new(
        key_low: NoteNumber,
        key_high: NoteNumber,
        velocity_low: Velocity,
        velocity_high: Velocity,
    ) -> Result<Self, KeyVelocityRangeError> {
        if key_low > key_high {
            return Err(KeyVelocityRangeError::KeyRangeInverted);
        }
        if velocity_low > velocity_high {
            return Err(KeyVelocityRangeError::VelocityRangeInverted);
        }
        Ok(Self {
            key_low,
            key_high,
            velocity_low,
            velocity_high,
        })
    }

    /// Returns the lowest note number in the key range.
    #[inline]
    pub fn key_low(self) -> NoteNumber {
        self.key_low
    }

    /// Returns the highest note number in the key range.
    #[inline]
    pub fn key_high(self) -> NoteNumber {
        self.key_high
    }

    /// Returns the lowest velocity in the velocity range.
    #[inline]
    pub fn velocity_low(self) -> Velocity {
        self.velocity_low
    }

    /// Returns the highest velocity in the velocity range.
    #[inline]
    pub fn velocity_high(self) -> Velocity {
        self.velocity_high
    }

    /// Returns `true` if `note` falls within `[key_low, key_high]`.
    #[inline]
    pub fn contains_note(self, note: NoteNumber) -> bool {
        (self.key_low..=self.key_high).contains(&note)
    }

    /// Returns `true` if `velocity` falls within `[velocity_low, velocity_high]`.
    #[inline]
    pub fn contains_velocity(self, velocity: Velocity) -> bool {
        (self.velocity_low..=self.velocity_high).contains(&velocity)
    }

    /// Returns `true` if both `note` and `velocity` fall within their respective ranges.
    #[inline]
    pub fn contains(self, note: NoteNumber, velocity: Velocity) -> bool {
        self.contains_note(note) && self.contains_velocity(velocity)
    }

    /// Returns `true` if this range overlaps with `other` in both key and velocity space.
    ///
    /// Two ranges overlap if their key intervals intersect **and** their velocity
    /// intervals intersect.
    #[inline]
    pub fn overlaps(self, other: KeyVelocityRange) -> bool {
        self.key_low <= other.key_high
            && other.key_low <= self.key_high
            && self.velocity_low <= other.velocity_high
            && other.velocity_low <= self.velocity_high
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::velocity::Velocity;

    fn nn(v: u8) -> NoteNumber {
        NoteNumber::try_new(v).unwrap()
    }

    fn vel(v: f64) -> Velocity {
        Velocity::try_new(v).unwrap()
    }

    #[test]
    fn key_velocity_range_valid_construction() {
        let range = KeyVelocityRange::try_new(nn(60), nn(72), vel(0.2), vel(0.8));
        assert!(range.is_ok());
        let r = range.unwrap();
        assert_eq!(r.key_low().value(), 60);
        assert_eq!(r.key_high().value(), 72);
        assert!((r.velocity_low().value() - 0.2).abs() < f64::EPSILON);
        assert!((r.velocity_high().value() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn key_velocity_range_equal_keys_is_valid() {
        // key_low == key_high is valid (single-note range)
        let range = KeyVelocityRange::try_new(nn(60), nn(60), vel(0.0), vel(1.0));
        assert!(range.is_ok());
    }

    #[test]
    fn key_velocity_range_equal_velocities_is_valid() {
        // velocity_low == velocity_high is valid (single-velocity range)
        let range = KeyVelocityRange::try_new(nn(0), nn(127), vel(0.5), vel(0.5));
        assert!(range.is_ok());
    }

    #[test]
    fn key_velocity_range_inverted_keys_rejected() {
        let result = KeyVelocityRange::try_new(nn(72), nn(60), vel(0.0), vel(1.0));
        assert_eq!(result, Err(KeyVelocityRangeError::KeyRangeInverted));
    }

    #[test]
    fn key_velocity_range_inverted_velocity_rejected() {
        let result = KeyVelocityRange::try_new(nn(60), nn(72), vel(0.8), vel(0.2));
        assert_eq!(result, Err(KeyVelocityRangeError::VelocityRangeInverted));
    }

    #[test]
    fn key_velocity_range_contains_note_inside() {
        let r = KeyVelocityRange::try_new(nn(60), nn(72), vel(0.0), vel(1.0)).unwrap();
        assert!(r.contains_note(nn(60)));
        assert!(r.contains_note(nn(66)));
        assert!(r.contains_note(nn(72)));
    }

    #[test]
    fn key_velocity_range_contains_note_outside() {
        let r = KeyVelocityRange::try_new(nn(60), nn(72), vel(0.0), vel(1.0)).unwrap();
        assert!(!r.contains_note(nn(59)));
        assert!(!r.contains_note(nn(73)));
    }

    #[test]
    fn key_velocity_range_contains_velocity_inside() {
        let r = KeyVelocityRange::try_new(nn(0), nn(127), vel(0.2), vel(0.8)).unwrap();
        assert!(r.contains_velocity(vel(0.2)));
        assert!(r.contains_velocity(vel(0.5)));
        assert!(r.contains_velocity(vel(0.8)));
    }

    #[test]
    fn key_velocity_range_contains_velocity_outside() {
        let r = KeyVelocityRange::try_new(nn(0), nn(127), vel(0.2), vel(0.8)).unwrap();
        assert!(!r.contains_velocity(vel(0.0)));
        assert!(!r.contains_velocity(vel(1.0)));
    }

    #[test]
    fn key_velocity_range_contains_both_true() {
        let r = KeyVelocityRange::try_new(nn(60), nn(72), vel(0.25), vel(0.75)).unwrap();
        assert!(r.contains(nn(65), vel(0.5)));
    }

    #[test]
    fn key_velocity_range_contains_both_false_when_note_out() {
        let r = KeyVelocityRange::try_new(nn(60), nn(72), vel(0.0), vel(1.0)).unwrap();
        assert!(!r.contains(nn(50), vel(0.5)));
    }

    #[test]
    fn key_velocity_range_contains_both_false_when_velocity_out() {
        let r = KeyVelocityRange::try_new(nn(60), nn(72), vel(0.5), vel(1.0)).unwrap();
        assert!(!r.contains(nn(65), vel(0.2)));
    }

    #[test]
    fn key_velocity_range_full_range() {
        let r = KeyVelocityRange::try_new(nn(0), nn(127), vel(0.0), vel(1.0)).unwrap();
        assert!(r.contains(nn(0), vel(0.0)));
        assert!(r.contains(nn(127), vel(1.0)));
        assert!(r.contains(nn(64), vel(0.5)));
    }

    #[test]
    fn key_velocity_range_error_display_key() {
        let err = KeyVelocityRangeError::KeyRangeInverted;
        assert!(err.to_string().contains("key_low"));
    }

    #[test]
    fn key_velocity_range_error_display_velocity() {
        let err = KeyVelocityRangeError::VelocityRangeInverted;
        assert!(err.to_string().contains("velocity_low"));
    }
}
