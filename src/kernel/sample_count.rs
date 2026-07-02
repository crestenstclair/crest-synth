// path: src/kernel/sample_count.rs

//! `SampleCount` — an absolute sample position measured from stream start.
//!
//! This is a newtype over `u64` so that raw sample indices are never
//! silently confused with other numeric quantities (e.g. buffer sizes,
//! frame counts, or milliseconds). All arithmetic is saturating so the
//! type can never panic or wrap on the audio thread.

use std::fmt;
use std::ops::{Add, Sub};

/// An absolute sample position since stream start.
///
/// # Examples
///
/// ```
/// use crest_synth::kernel::sample_count::SampleCount;
///
/// let start = SampleCount::new(0);
/// let advanced = start.advance_by(512);
/// assert_eq!(advanced.value(), 512);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SampleCount(u64);

impl SampleCount {
    /// The zero sample position (stream start).
    pub const ZERO: SampleCount = SampleCount(0);

    /// Constructs a new `SampleCount` from a raw sample index.
    pub fn new(samples: u64) -> Self {
        SampleCount(samples)
    }

    /// Returns the raw sample index.
    pub fn value(self) -> u64 {
        self.0
    }

    /// Advances this position by `delta` samples, saturating at `u64::MAX`.
    pub fn advance_by(self, delta: u64) -> Self {
        SampleCount(self.0.saturating_add(delta))
    }

    /// Moves this position back by `delta` samples, saturating at zero.
    pub fn retreat_by(self, delta: u64) -> Self {
        SampleCount(self.0.saturating_sub(delta))
    }

    /// Returns the number of samples elapsed between `self` and an earlier
    /// (or equal) position `earlier`. Saturates to zero if `earlier` is
    /// actually later than `self`.
    pub fn elapsed_since(self, earlier: SampleCount) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

impl fmt::Display for SampleCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for SampleCount {
    fn from(samples: u64) -> Self {
        SampleCount(samples)
    }
}

impl From<SampleCount> for u64 {
    fn from(count: SampleCount) -> Self {
        count.0
    }
}

impl Add<u64> for SampleCount {
    type Output = SampleCount;

    fn add(self, rhs: u64) -> Self::Output {
        self.advance_by(rhs)
    }
}

impl Sub<u64> for SampleCount {
    type Output = SampleCount;

    fn sub(self, rhs: u64) -> Self::Output {
        self.retreat_by(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_value_round_trip() {
        let count = SampleCount::new(4_096);
        assert_eq!(count.value(), 4_096);
    }

    #[test]
    fn zero_is_default_and_zero_value() {
        assert_eq!(SampleCount::default(), SampleCount::ZERO);
        assert_eq!(SampleCount::ZERO.value(), 0);
    }

    #[test]
    fn advance_by_adds_samples() {
        let count = SampleCount::new(100);
        assert_eq!(count.advance_by(50).value(), 150);
    }

    #[test]
    fn advance_by_saturates_at_max() {
        let count = SampleCount::new(u64::MAX - 1);
        assert_eq!(count.advance_by(10).value(), u64::MAX);
    }

    #[test]
    fn retreat_by_subtracts_samples() {
        let count = SampleCount::new(100);
        assert_eq!(count.retreat_by(30).value(), 70);
    }

    #[test]
    fn retreat_by_saturates_at_zero() {
        let count = SampleCount::new(5);
        assert_eq!(count.retreat_by(10).value(), 0);
    }

    #[test]
    fn elapsed_since_computes_difference() {
        let earlier = SampleCount::new(1_000);
        let later = SampleCount::new(1_500);
        assert_eq!(later.elapsed_since(earlier), 500);
    }

    #[test]
    fn elapsed_since_saturates_when_reversed() {
        let earlier = SampleCount::new(1_000);
        let later = SampleCount::new(1_500);
        assert_eq!(earlier.elapsed_since(later), 0);
    }

    #[test]
    fn ordering_compares_by_value() {
        assert!(SampleCount::new(1) < SampleCount::new(2));
        assert!(SampleCount::new(2) > SampleCount::new(1));
    }

    #[test]
    fn from_u64_and_into_u64_round_trip() {
        let count: SampleCount = 42_u64.into();
        let raw: u64 = count.into();
        assert_eq!(raw, 42);
    }

    #[test]
    fn add_and_sub_operators() {
        let count = SampleCount::new(10) + 5;
        assert_eq!(count.value(), 15);
        let count = count - 3;
        assert_eq!(count.value(), 12);
    }

    #[test]
    fn display_formats_raw_value() {
        let count = SampleCount::new(777);
        assert_eq!(format!("{}", count), "777");
    }
}
