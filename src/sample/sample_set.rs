// path: src/sample/sample_set.rs
//
// SampleSet — aggregate root for the Sample context.
//
// A named collection of zones backing one instrument sound. Each zone maps a
// region of key+velocity space to an underlying sample reference. Resolving a
// note returns every zone that matches, in key range *and* velocity range —
// layering (multiple zones sounding for one note) is intentional, so
// `resolve` never short-circuits on the first hit.
//
// This is domain/editing-side state (non-realtime): commands mutate the
// aggregate and produce events describing what happened. It does not touch
// the audio thread directly and is free to allocate.
//
// No Kernel-context value objects (e.g. NoteNumber, Velocity) are available
// yet in this project, so key and velocity addresses are represented here as
// plain `u8` and the ranges are defined locally.

use std::fmt;

/// Resampling algorithm applied when a zone's native pitch differs from the
/// pitch being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterpolationMode {
    /// No resampling; the nearest recorded frame is used.
    #[default]
    None,
    /// Linear interpolation between adjacent frames.
    Linear,
    /// Cubic (4-point) interpolation for higher fidelity.
    Cubic,
}

/// Inclusive range of MIDI key numbers that a [`Zone`] responds to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRange {
    low: u8,
    high: u8,
}

impl KeyRange {
    /// Build a key range. Fails if `low > high`.
    pub fn try_new(low: u8, high: u8) -> Result<Self, SampleSetError> {
        if low > high {
            return Err(SampleSetError::InvalidRange { low, high });
        }
        Ok(Self { low, high })
    }

    pub fn low(&self) -> u8 {
        self.low
    }

    pub fn high(&self) -> u8 {
        self.high
    }

    /// True when `key` falls within `[low, high]`.
    pub fn contains(&self, key: u8) -> bool {
        (self.low..=self.high).contains(&key)
    }
}

/// Inclusive range of MIDI velocity values that a [`Zone`] responds to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VelocityRange {
    low: u8,
    high: u8,
}

impl VelocityRange {
    /// Build a velocity range. Fails if `low > high`.
    pub fn try_new(low: u8, high: u8) -> Result<Self, SampleSetError> {
        if low > high {
            return Err(SampleSetError::InvalidRange { low, high });
        }
        Ok(Self { low, high })
    }

    pub fn low(&self) -> u8 {
        self.low
    }

    pub fn high(&self) -> u8 {
        self.high
    }

    /// True when `velocity` falls within `[low, high]`.
    pub fn contains(&self, velocity: u8) -> bool {
        (self.low..=self.high).contains(&velocity)
    }
}

/// One region of key+velocity space, mapped to a single underlying sample.
///
/// `sample_ref` names the underlying sample asset (e.g. a file stem or
/// content-addressed id) that this zone plays back; the aggregate only
/// tracks which zones exist and where they sit in key/velocity space, not
/// how the referenced audio is decoded or stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    key_range: KeyRange,
    velocity_range: VelocityRange,
    sample_ref: String,
}

impl Zone {
    pub fn new(key_range: KeyRange, velocity_range: VelocityRange, sample_ref: String) -> Self {
        Self {
            key_range,
            velocity_range,
            sample_ref,
        }
    }

    pub fn key_range(&self) -> &KeyRange {
        &self.key_range
    }

    pub fn velocity_range(&self) -> &VelocityRange {
        &self.velocity_range
    }

    pub fn sample_ref(&self) -> &str {
        &self.sample_ref
    }

    /// True when both the key range and the velocity range cover `(key, velocity)`.
    pub fn matches(&self, key: u8, velocity: u8) -> bool {
        self.key_range.contains(key) && self.velocity_range.contains(velocity)
    }
}

/// Commands accepted by the [`SampleSet`] aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleSetCommand {
    AddZone { zone: Zone },
    RemoveZone { index: u32 },
}

/// Events emitted by the [`SampleSet`] aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleSetEvent {
    ZoneAdded { index: u32 },
    ZoneRemoved { index: u32 },
}

/// Errors returned when a [`SampleSetCommand`] cannot be applied, or a range
/// cannot be constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleSetError {
    /// A key or velocity range was constructed with `low > high`.
    InvalidRange { low: u8, high: u8 },
    /// `RemoveZone` referenced an index past the end of the zone list.
    ZoneIndexOutOfRange { index: u32, len: u32 },
}

impl fmt::Display for SampleSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SampleSetError::InvalidRange { low, high } => {
                write!(
                    f,
                    "invalid range: low ({low}) is greater than high ({high})"
                )
            }
            SampleSetError::ZoneIndexOutOfRange { index, len } => {
                write!(f, "zone index {index} out of range (set has {len} zones)")
            }
        }
    }
}

impl std::error::Error for SampleSetError {}

/// Aggregate root: a named collection of zones backing one instrument sound.
///
/// # Invariants
///
/// - Resolving a note returns every zone whose key range and velocity range
///   both match (see [`SampleSet::resolve`]) — layering across zones is
///   intentional, leakage from a non-matching zone is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleSet {
    interpolation: InterpolationMode,
    zones: Vec<Zone>,
}

impl SampleSet {
    /// Create an empty sample set using the given interpolation algorithm.
    pub fn new(interpolation: InterpolationMode) -> Self {
        Self {
            interpolation,
            zones: Vec::new(),
        }
    }

    pub fn interpolation(&self) -> InterpolationMode {
        self.interpolation
    }

    pub fn zones(&self) -> &[Zone] {
        &self.zones
    }

    /// Handle a command, mutating state and returning the resulting event.
    ///
    /// This is the single entry point for command dispatch; `apply_add_zone`
    /// and `apply_remove_zone` are also exposed directly for callers that
    /// already know which operation they want.
    pub fn handle(&mut self, command: SampleSetCommand) -> Result<SampleSetEvent, SampleSetError> {
        match command {
            SampleSetCommand::AddZone { zone } => Ok(self.apply_add_zone(zone)),
            SampleSetCommand::RemoveZone { index } => self.apply_remove_zone(index),
        }
    }

    /// Append a zone to the set and return the `ZoneAdded` event.
    pub fn apply_add_zone(&mut self, zone: Zone) -> SampleSetEvent {
        let index = self.zones.len() as u32;
        self.zones.push(zone);
        SampleSetEvent::ZoneAdded { index }
    }

    /// Remove the zone at `index` and return the `ZoneRemoved` event.
    ///
    /// Returns `Err` if `index` is out of range; state is left unchanged.
    pub fn apply_remove_zone(&mut self, index: u32) -> Result<SampleSetEvent, SampleSetError> {
        let len = self.zones.len() as u32;
        if index >= len {
            return Err(SampleSetError::ZoneIndexOutOfRange { index, len });
        }
        self.zones.remove(index as usize);
        Ok(SampleSetEvent::ZoneRemoved { index })
    }

    /// Resolve a note address to every zone that covers it.
    ///
    /// Multiple matches are expected and intentional (layered zones); this
    /// scans the full list rather than stopping at the first hit.
    pub fn resolve(&self, key: u8, velocity: u8) -> Vec<&Zone> {
        self.zones
            .iter()
            .filter(|zone| zone.matches(key, velocity))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(key_lo: u8, key_hi: u8, vel_lo: u8, vel_hi: u8) -> Zone {
        Zone::new(
            KeyRange::try_new(key_lo, key_hi).unwrap(),
            VelocityRange::try_new(vel_lo, vel_hi).unwrap(),
            "test-sample".to_string(),
        )
    }

    #[test]
    fn new_set_has_no_zones() {
        let set = SampleSet::new(InterpolationMode::Linear);
        assert!(set.zones().is_empty());
    }

    #[test]
    fn add_zone_appends_and_emits_zone_added_with_index() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        let event = set.apply_add_zone(zone(0, 60, 0, 127));
        assert_eq!(event, SampleSetEvent::ZoneAdded { index: 0 });
        assert_eq!(set.zones().len(), 1);

        let event2 = set.apply_add_zone(zone(61, 127, 0, 127));
        assert_eq!(event2, SampleSetEvent::ZoneAdded { index: 1 });
        assert_eq!(set.zones().len(), 2);
    }

    #[test]
    fn remove_zone_removes_and_emits_zone_removed() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(0, 60, 0, 127));
        set.apply_add_zone(zone(61, 127, 0, 127));

        let event = set.apply_remove_zone(0).unwrap();
        assert_eq!(event, SampleSetEvent::ZoneRemoved { index: 0 });
        assert_eq!(set.zones().len(), 1);
        assert_eq!(set.zones()[0].key_range().low(), 61);
    }

    #[test]
    fn remove_zone_out_of_range_returns_error_and_leaves_state_unchanged() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(0, 60, 0, 127));

        let err = set.apply_remove_zone(5).unwrap_err();
        assert_eq!(
            err,
            SampleSetError::ZoneIndexOutOfRange { index: 5, len: 1 }
        );
        assert_eq!(set.zones().len(), 1);
    }

    #[test]
    fn resolve_returns_every_zone_whose_key_and_velocity_range_both_match() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        // Two layered zones covering the same key+velocity space.
        set.apply_add_zone(zone(60, 72, 0, 127));
        set.apply_add_zone(zone(60, 72, 0, 127));
        // A non-matching zone (different key range).
        set.apply_add_zone(zone(73, 84, 0, 127));

        let matches = set.resolve(65, 100);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn resolve_excludes_zones_whose_key_range_does_not_match() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(60, 72, 0, 127));

        assert!(set.resolve(80, 100).is_empty());
    }

    #[test]
    fn resolve_excludes_zones_whose_velocity_range_does_not_match() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(60, 72, 0, 50));

        assert!(set.resolve(65, 100).is_empty());
    }

    #[test]
    fn key_range_rejects_low_greater_than_high() {
        assert!(KeyRange::try_new(10, 5).is_err());
    }

    #[test]
    fn velocity_range_rejects_low_greater_than_high() {
        assert!(VelocityRange::try_new(10, 5).is_err());
    }

    #[test]
    fn handle_add_zone_command_matches_direct_call() {
        let mut set = SampleSet::new(InterpolationMode::None);
        let event = set
            .handle(SampleSetCommand::AddZone {
                zone: zone(0, 127, 0, 127),
            })
            .unwrap();
        assert_eq!(event, SampleSetEvent::ZoneAdded { index: 0 });
    }

    #[test]
    fn handle_remove_zone_command_matches_direct_call() {
        let mut set = SampleSet::new(InterpolationMode::None);
        set.apply_add_zone(zone(0, 127, 0, 127));
        let event = set
            .handle(SampleSetCommand::RemoveZone { index: 0 })
            .unwrap();
        assert_eq!(event, SampleSetEvent::ZoneRemoved { index: 0 });
    }
}
