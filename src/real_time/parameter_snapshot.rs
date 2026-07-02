// path: src/real_time/parameter_snapshot.rs

//! `ParameterSnapshot` is the payload type carried across the parameter
//! thread boundary. Non-real-time code (UI, MIDI) builds a new snapshot
//! whenever a parameter changes and publishes it through the
//! `ParameterBridge`; the audio thread reads the latest published snapshot
//! by value. `ParameterSnapshot` itself never allocates and never crosses
//! the boundary by any path other than the bridge — see the "all parameter
//! changes cross the thread boundary via the ParameterBridge or the
//! EventRing" invariant.
//!
//! Every field is a plain `Copy` newtype so the whole snapshot is `Copy`:
//! publishing a new snapshot is a value move/copy, never a heap allocation,
//! satisfying the "audio thread never allocates" invariant for the
//! read side.

/// Master output gain, expressed in decibels.
///
/// Defined locally: `kernel::decibel` is not yet a declared dependency of
/// this resource, so `ParameterSnapshot` does not assume its API.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasterGainDb(f32);

/// Errors constructing a [`MasterGainDb`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterGainDbError {
    /// The supplied value was NaN or outside the supported range.
    OutOfRange,
}

impl MasterGainDb {
    /// Silence: -inf dB is not representable, so unity-minus-infinity is
    /// approximated by a large negative floor used as the effective mute
    /// point.
    pub const MIN_DB: f32 = -96.0;
    /// Maximum permitted master gain, chosen to prevent runaway output.
    pub const MAX_DB: f32 = 12.0;
    /// Unity gain (0 dB).
    pub const UNITY: MasterGainDb = MasterGainDb(0.0);

    /// Constructs a validated master gain value.
    pub fn try_new(db: f32) -> Result<Self, MasterGainDbError> {
        if db.is_nan() || !(Self::MIN_DB..=Self::MAX_DB).contains(&db) {
            return Err(MasterGainDbError::OutOfRange);
        }
        Ok(MasterGainDb(db))
    }

    /// The raw decibel value.
    pub fn db(&self) -> f32 {
        self.0
    }
}

impl Default for MasterGainDb {
    fn default() -> Self {
        MasterGainDb::UNITY
    }
}

/// Master output pan position, from `-1.0` (full left) to `1.0` (full
/// right), `0.0` being center.
///
/// Defined locally: `kernel::pan` is not yet a declared dependency of this
/// resource.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanPosition(f32);

/// Errors constructing a [`PanPosition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanPositionError {
    /// The supplied value was NaN or outside `-1.0..=1.0`.
    OutOfRange,
}

impl PanPosition {
    /// Center pan (equal left/right).
    pub const CENTER: PanPosition = PanPosition(0.0);

    /// Constructs a validated pan position.
    pub fn try_new(value: f32) -> Result<Self, PanPositionError> {
        if value.is_nan() || !(-1.0..=1.0).contains(&value) {
            return Err(PanPositionError::OutOfRange);
        }
        Ok(PanPosition(value))
    }

    /// The raw pan value.
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl Default for PanPosition {
    fn default() -> Self {
        PanPosition::CENTER
    }
}

/// Global filter cutoff frequency, in Hz, readable by the audio thread.
///
/// Defined locally: `kernel::frequency` is not yet a declared dependency of
/// this resource.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterCutoffHz(f32);

/// Errors constructing a [`FilterCutoffHz`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterCutoffHzError {
    /// The supplied value was NaN or outside the audible-and-safe range.
    OutOfRange,
}

impl FilterCutoffHz {
    /// Lowest permitted cutoff frequency.
    pub const MIN_HZ: f32 = 20.0;
    /// Highest permitted cutoff frequency.
    pub const MAX_HZ: f32 = 20_000.0;

    /// Constructs a validated cutoff frequency.
    pub fn try_new(hz: f32) -> Result<Self, FilterCutoffHzError> {
        if hz.is_nan() || !(Self::MIN_HZ..=Self::MAX_HZ).contains(&hz) {
            return Err(FilterCutoffHzError::OutOfRange);
        }
        Ok(FilterCutoffHz(hz))
    }

    /// The raw frequency value in Hz.
    pub fn hz(&self) -> f32 {
        self.0
    }
}

impl Default for FilterCutoffHz {
    fn default() -> Self {
        // A generous default cutoff that passes essentially the full
        // audible spectrum until a patch narrows it.
        FilterCutoffHz(Self::MAX_HZ)
    }
}

/// Global filter resonance, normalized `0.0` (no resonance) to `1.0`
/// (maximum resonance before self-oscillation).
///
/// Defined locally: no existing dependency covers this quantity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterResonance(f32);

/// Errors constructing a [`FilterResonance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResonanceError {
    /// The supplied value was NaN or outside `0.0..=1.0`.
    OutOfRange,
}

impl FilterResonance {
    /// No resonance.
    pub const NONE: FilterResonance = FilterResonance(0.0);

    /// Constructs a validated resonance value.
    pub fn try_new(value: f32) -> Result<Self, FilterResonanceError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(FilterResonanceError::OutOfRange);
        }
        Ok(FilterResonance(value))
    }

    /// The raw normalized resonance value.
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl Default for FilterResonance {
    fn default() -> Self {
        FilterResonance::NONE
    }
}

/// A complete, immutable snapshot of every audio-thread-readable parameter.
///
/// `ParameterSnapshot` is a plain-old-data value object: `Copy`, no heap
/// allocation, no interior mutability. The audio thread only ever reads a
/// snapshot handed to it across the `ParameterBridge`; it never mutates one
/// in place. To change a parameter, non-real-time code builds a new
/// snapshot (via [`ParameterSnapshot::with_master_gain`] and friends, or by
/// constructing one directly) and publishes the whole snapshot as the next
/// latest-wins value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterSnapshot {
    master_gain: MasterGainDb,
    master_pan: PanPosition,
    filter_cutoff: FilterCutoffHz,
    filter_resonance: FilterResonance,
}

impl ParameterSnapshot {
    /// Constructs a snapshot from already-validated component values.
    pub fn new(
        master_gain: MasterGainDb,
        master_pan: PanPosition,
        filter_cutoff: FilterCutoffHz,
        filter_resonance: FilterResonance,
    ) -> Self {
        ParameterSnapshot {
            master_gain,
            master_pan,
            filter_cutoff,
            filter_resonance,
        }
    }

    /// The master output gain at the moment of this snapshot.
    pub fn master_gain(&self) -> MasterGainDb {
        self.master_gain
    }

    /// The master output pan at the moment of this snapshot.
    pub fn master_pan(&self) -> PanPosition {
        self.master_pan
    }

    /// The global filter cutoff at the moment of this snapshot.
    pub fn filter_cutoff(&self) -> FilterCutoffHz {
        self.filter_cutoff
    }

    /// The global filter resonance at the moment of this snapshot.
    pub fn filter_resonance(&self) -> FilterResonance {
        self.filter_resonance
    }

    /// Returns a new snapshot identical to this one except for master gain.
    ///
    /// Snapshots are immutable value objects: producing an updated
    /// parameter set never mutates `self`, it returns a fresh value to be
    /// published as the next latest-wins snapshot.
    pub fn with_master_gain(&self, master_gain: MasterGainDb) -> Self {
        ParameterSnapshot {
            master_gain,
            ..*self
        }
    }

    /// Returns a new snapshot identical to this one except for master pan.
    pub fn with_master_pan(&self, master_pan: PanPosition) -> Self {
        ParameterSnapshot {
            master_pan,
            ..*self
        }
    }

    /// Returns a new snapshot identical to this one except for filter
    /// cutoff.
    pub fn with_filter_cutoff(&self, filter_cutoff: FilterCutoffHz) -> Self {
        ParameterSnapshot {
            filter_cutoff,
            ..*self
        }
    }

    /// Returns a new snapshot identical to this one except for filter
    /// resonance.
    pub fn with_filter_resonance(&self, filter_resonance: FilterResonance) -> Self {
        ParameterSnapshot {
            filter_resonance,
            ..*self
        }
    }
}

impl Default for ParameterSnapshot {
    /// The snapshot in effect before any parameter has been touched:
    /// unity gain, centered pan, filter fully open, no resonance.
    fn default() -> Self {
        ParameterSnapshot {
            master_gain: MasterGainDb::default(),
            master_pan: PanPosition::default(),
            filter_cutoff: FilterCutoffHz::default(),
            filter_resonance: FilterResonance::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_snapshot_is_unity_centered_and_open() {
        let snapshot = ParameterSnapshot::default();

        assert_eq!(snapshot.master_gain(), MasterGainDb::UNITY);
        assert_eq!(snapshot.master_pan(), PanPosition::CENTER);
        assert_eq!(snapshot.filter_cutoff().hz(), FilterCutoffHz::MAX_HZ);
        assert_eq!(snapshot.filter_resonance(), FilterResonance::NONE);
    }

    #[test]
    fn master_gain_rejects_out_of_range() {
        assert!(MasterGainDb::try_new(f32::NAN).is_err());
        assert!(MasterGainDb::try_new(MasterGainDb::MIN_DB - 1.0).is_err());
        assert!(MasterGainDb::try_new(MasterGainDb::MAX_DB + 1.0).is_err());
        assert!(MasterGainDb::try_new(0.0).is_ok());
    }

    #[test]
    fn pan_position_rejects_out_of_range() {
        assert!(PanPosition::try_new(f32::NAN).is_err());
        assert!(PanPosition::try_new(-1.01).is_err());
        assert!(PanPosition::try_new(1.01).is_err());
        assert!(PanPosition::try_new(-1.0).is_ok());
        assert!(PanPosition::try_new(1.0).is_ok());
    }

    #[test]
    fn filter_cutoff_rejects_out_of_range() {
        assert!(FilterCutoffHz::try_new(f32::NAN).is_err());
        assert!(FilterCutoffHz::try_new(FilterCutoffHz::MIN_HZ - 1.0).is_err());
        assert!(FilterCutoffHz::try_new(FilterCutoffHz::MAX_HZ + 1.0).is_err());
    }

    #[test]
    fn filter_resonance_rejects_out_of_range() {
        assert!(FilterResonance::try_new(f32::NAN).is_err());
        assert!(FilterResonance::try_new(-0.01).is_err());
        assert!(FilterResonance::try_new(1.01).is_err());
        assert!(FilterResonance::try_new(0.0).is_ok());
        assert!(FilterResonance::try_new(1.0).is_ok());
    }

    #[test]
    fn with_master_gain_returns_new_value_and_leaves_original_unchanged() {
        let original = ParameterSnapshot::default();
        let louder = original.with_master_gain(MasterGainDb::try_new(6.0).unwrap());

        assert_eq!(original.master_gain(), MasterGainDb::UNITY);
        assert_eq!(louder.master_gain().db(), 6.0);
        // Every other field is carried over unchanged.
        assert_eq!(louder.master_pan(), original.master_pan());
        assert_eq!(louder.filter_cutoff(), original.filter_cutoff());
        assert_eq!(louder.filter_resonance(), original.filter_resonance());
    }

    #[test]
    fn with_master_pan_returns_new_value_and_leaves_original_unchanged() {
        let original = ParameterSnapshot::default();
        let panned = original.with_master_pan(PanPosition::try_new(-0.5).unwrap());

        assert_eq!(original.master_pan(), PanPosition::CENTER);
        assert_eq!(panned.master_pan().value(), -0.5);
    }

    #[test]
    fn with_filter_cutoff_returns_new_value_and_leaves_original_unchanged() {
        let original = ParameterSnapshot::default();
        let darker = original.with_filter_cutoff(FilterCutoffHz::try_new(400.0).unwrap());

        assert_eq!(
            original.filter_cutoff().hz(),
            FilterCutoffHz::MAX_HZ
        );
        assert_eq!(darker.filter_cutoff().hz(), 400.0);
    }

    #[test]
    fn with_filter_resonance_returns_new_value_and_leaves_original_unchanged() {
        let original = ParameterSnapshot::default();
        let resonant = original.with_filter_resonance(FilterResonance::try_new(0.8).unwrap());

        assert_eq!(original.filter_resonance(), FilterResonance::NONE);
        assert_eq!(resonant.filter_resonance().value(), 0.8);
    }

    #[test]
    fn snapshot_is_copy() {
        let a = ParameterSnapshot::default();
        let b = a;
        // Both remain usable: `ParameterSnapshot` is `Copy`, not moved.
        assert_eq!(a, b);
    }

    #[test]
    fn new_composes_component_values_directly() {
        let snapshot = ParameterSnapshot::new(
            MasterGainDb::try_new(-3.0).unwrap(),
            PanPosition::try_new(0.25).unwrap(),
            FilterCutoffHz::try_new(1_000.0).unwrap(),
            FilterResonance::try_new(0.5).unwrap(),
        );

        assert_eq!(snapshot.master_gain().db(), -3.0);
        assert_eq!(snapshot.master_pan().value(), 0.25);
        assert_eq!(snapshot.filter_cutoff().hz(), 1_000.0);
        assert_eq!(snapshot.filter_resonance().value(), 0.5);
    }
}
