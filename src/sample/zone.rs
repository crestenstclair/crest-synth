// path: src/sample/zone.rs

use std::fmt;

use crate::kernel::amplitude::Amplitude;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::pan::Pan;
use crate::kernel::velocity::Velocity;

/// Cents of fine-tune detune are bounded to one octave in either direction;
/// coarse transposition belongs elsewhere (e.g. picking a different
/// `rootKey`), so a zone's own fine tune stays a small trim.
const FINE_TUNE_CENTS_BOUND: f64 = 1200.0;

/// Reasons a [`Zone`] or one of its supporting ranges failed to validate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneError {
    /// `KeyRange::low` was greater than `KeyRange::high`.
    InvertedKeyRange,
    /// `VelocityRange::low` was greater than `VelocityRange::high`.
    InvertedVelocityRange,
    /// `fineTuneCents` was non-finite or outside `+/-1200.0`.
    InvalidFineTuneCents,
}

impl fmt::Display for ZoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZoneError::InvertedKeyRange => {
                write!(f, "key range low must not exceed key range high")
            }
            ZoneError::InvertedVelocityRange => {
                write!(f, "velocity range low must not exceed velocity range high")
            }
            ZoneError::InvalidFineTuneCents => write!(
                f,
                "fine tune cents must be finite and within +/-{FINE_TUNE_CENTS_BOUND}"
            ),
        }
    }
}

impl std::error::Error for ZoneError {}

/// An inclusive range of MIDI key numbers a [`Zone`] responds to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyRange {
    low: NoteNumber,
    high: NoteNumber,
}

impl KeyRange {
    /// Builds a key range. Fails when `low` is above `high`.
    pub fn try_new(low: NoteNumber, high: NoteNumber) -> Result<Self, ZoneError> {
        if low > high {
            return Err(ZoneError::InvertedKeyRange);
        }
        Ok(Self { low, high })
    }

    pub fn low(&self) -> NoteNumber {
        self.low
    }

    pub fn high(&self) -> NoteNumber {
        self.high
    }

    /// True when `key` falls within `[low, high]` inclusive.
    pub fn contains(&self, key: NoteNumber) -> bool {
        self.low <= key && key <= self.high
    }
}

/// An inclusive range of note-on velocities a [`Zone`] responds to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityRange {
    low: Velocity,
    high: Velocity,
}

impl VelocityRange {
    /// Builds a velocity range. Fails when `low` is above `high`.
    pub fn try_new(low: Velocity, high: Velocity) -> Result<Self, ZoneError> {
        if low > high {
            return Err(ZoneError::InvertedVelocityRange);
        }
        Ok(Self { low, high })
    }

    pub fn low(&self) -> Velocity {
        self.low
    }

    pub fn high(&self) -> Velocity {
        self.high
    }

    /// True when `velocity` falls within `[low, high]` inclusive.
    pub fn contains(&self, velocity: Velocity) -> bool {
        self.low <= velocity && velocity <= self.high
    }
}

/// How playback loops (if at all) once it reaches the end of the sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    /// Play through once with no looping.
    #[default]
    None,
    /// Loop forward from loop-start to loop-end.
    Forward,
    /// Alternate forward/backward between loop-start and loop-end.
    PingPong,
    /// Loop backward from loop-end to loop-start.
    Reverse,
}

/// Maps a key range and a velocity range to one sample with per-zone
/// playback settings (fine tune, gain, pan, loop behavior, and the root
/// key used to compute pitch-shift for keys other than the root).
///
/// A `Zone` is pure data: it holds no reference to sample audio and
/// performs no I/O or allocation. Higher-level services use `matches` to
/// decide which zones a given note trigger selects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    fine_tune_cents: f64,
    gain: Amplitude,
    keys: KeyRange,
    loop_mode: LoopMode,
    pan: Pan,
    root_key: NoteNumber,
    velocities: VelocityRange,
}

impl Zone {
    /// Builds a zone, validating `fine_tune_cents` is finite and within
    /// `+/-1200.0` cents. The key range and velocity range are validated by
    /// their own constructors before a `Zone` can be built.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        fine_tune_cents: f64,
        gain: Amplitude,
        keys: KeyRange,
        loop_mode: LoopMode,
        pan: Pan,
        root_key: NoteNumber,
        velocities: VelocityRange,
    ) -> Result<Self, ZoneError> {
        if fine_tune_cents.is_nan()
            || !(-FINE_TUNE_CENTS_BOUND..=FINE_TUNE_CENTS_BOUND).contains(&fine_tune_cents)
        {
            return Err(ZoneError::InvalidFineTuneCents);
        }

        Ok(Self {
            fine_tune_cents,
            gain,
            keys,
            loop_mode,
            pan,
            root_key,
            velocities,
        })
    }

    pub fn fine_tune_cents(&self) -> f64 {
        self.fine_tune_cents
    }

    pub fn gain(&self) -> Amplitude {
        self.gain
    }

    pub fn keys(&self) -> KeyRange {
        self.keys
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn pan(&self) -> Pan {
        self.pan
    }

    pub fn root_key(&self) -> NoteNumber {
        self.root_key
    }

    pub fn velocities(&self) -> VelocityRange {
        self.velocities
    }

    /// True when both the key range and the velocity range admit the given
    /// note trigger, i.e. this zone is one of the zones that should sound.
    pub fn matches(&self, key: NoteNumber, velocity: Velocity) -> bool {
        self.keys.contains(key) && self.velocities.contains(velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(n: u8) -> NoteNumber {
        NoteNumber::try_new(n).expect("valid note number")
    }

    fn vel(v: f64) -> Velocity {
        Velocity::try_new(v).expect("valid velocity")
    }

    fn amp(a: f64) -> Amplitude {
        Amplitude::try_new(a).expect("valid amplitude")
    }

    fn pan(p: f64) -> Pan {
        Pan::try_new(p).expect("valid pan")
    }

    #[test]
    fn key_range_rejects_inverted_bounds() {
        assert_eq!(
            KeyRange::try_new(note(60), note(48)),
            Err(ZoneError::InvertedKeyRange)
        );
    }

    #[test]
    fn key_range_accepts_equal_bounds() {
        assert!(KeyRange::try_new(note(60), note(60)).is_ok());
    }

    #[test]
    fn key_range_contains_bounds_inclusive() {
        let range = KeyRange::try_new(note(48), note(72)).unwrap();
        assert!(range.contains(note(48)));
        assert!(range.contains(note(72)));
        assert!(!range.contains(note(47)));
        assert!(!range.contains(note(73)));
    }

    #[test]
    fn velocity_range_rejects_inverted_bounds() {
        assert_eq!(
            VelocityRange::try_new(vel(0.8), vel(0.2)),
            Err(ZoneError::InvertedVelocityRange)
        );
    }

    #[test]
    fn velocity_range_contains_bounds_inclusive() {
        let range = VelocityRange::try_new(vel(0.2), vel(0.8)).unwrap();
        assert!(range.contains(vel(0.2)));
        assert!(range.contains(vel(0.8)));
        assert!(!range.contains(vel(0.1)));
        assert!(!range.contains(vel(0.9)));
    }

    fn full_range_zone(loop_mode: LoopMode, fine_tune_cents: f64) -> Result<Zone, ZoneError> {
        let keys = KeyRange::try_new(note(0), note(127)).unwrap();
        let velocities = VelocityRange::try_new(vel(0.0), vel(1.0)).unwrap();
        Zone::try_new(
            fine_tune_cents,
            amp(1.0),
            keys,
            loop_mode,
            pan(0.0),
            note(60),
            velocities,
        )
    }

    #[test]
    fn zone_rejects_non_finite_fine_tune() {
        assert_eq!(
            full_range_zone(LoopMode::None, f64::NAN),
            Err(ZoneError::InvalidFineTuneCents)
        );
    }

    #[test]
    fn zone_rejects_out_of_bound_fine_tune() {
        assert_eq!(
            full_range_zone(LoopMode::None, 1300.0),
            Err(ZoneError::InvalidFineTuneCents)
        );
        assert_eq!(
            full_range_zone(LoopMode::None, -1300.0),
            Err(ZoneError::InvalidFineTuneCents)
        );
    }

    #[test]
    fn zone_accepts_boundary_fine_tune() {
        assert!(full_range_zone(LoopMode::Forward, 1200.0).is_ok());
        assert!(full_range_zone(LoopMode::Forward, -1200.0).is_ok());
    }

    #[test]
    fn zone_matches_when_key_and_velocity_are_in_range() {
        let keys = KeyRange::try_new(note(48), note(72)).unwrap();
        let velocities = VelocityRange::try_new(vel(0.2), vel(0.8)).unwrap();
        let zone = Zone::try_new(
            0.0,
            amp(1.0),
            keys,
            LoopMode::Forward,
            pan(0.0),
            note(60),
            velocities,
        )
        .unwrap();

        assert!(zone.matches(note(60), vel(0.5)));
        assert!(!zone.matches(note(90), vel(0.5)));
        assert!(!zone.matches(note(60), vel(0.9)));
    }

    #[test]
    fn loop_mode_defaults_to_none() {
        assert_eq!(LoopMode::default(), LoopMode::None);
    }
}
