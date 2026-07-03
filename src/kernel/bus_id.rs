// path: src/kernel/bus_id.rs

//! `BusId` identifies a mixer bus.
//!
//! Bus `0` is reserved for the master bus (see the signal-flow invariant:
//! engine output → channel strip inserts → volume and pan → send taps and
//! bus routing → aux bus inserts → master bus inserts → limiter → output).
//! All other bus ids identify aux/send buses that route into the master bus.

use std::fmt;

/// Identifies a mixer bus.
///
/// `BusId(0)` is reserved for the master bus. Every other value identifies
/// an auxiliary (send) bus.
///
/// # Examples
///
/// ```
/// use crest_synth::kernel::bus_id::BusId;
///
/// let master = BusId::MASTER;
/// assert!(master.is_master());
///
/// let aux = BusId::new(1);
/// assert!(!aux.is_master());
/// assert_eq!(aux.value(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BusId(u32);

impl BusId {
    /// The reserved id for the master bus.
    pub const MASTER: BusId = BusId(0);

    /// Constructs a new `BusId` from a raw `u32`.
    ///
    /// Passing `0` yields [`BusId::MASTER`]; any other value identifies an
    /// auxiliary bus.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        BusId(value)
    }

    /// Returns the underlying raw value.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Returns `true` if this id refers to the reserved master bus.
    #[must_use]
    pub const fn is_master(self) -> bool {
        self.0 == Self::MASTER.0
    }
}

impl Default for BusId {
    /// Defaults to the master bus, matching the canonical signal path's
    /// terminal destination.
    fn default() -> Self {
        Self::MASTER
    }
}

impl fmt::Display for BusId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for BusId {
    fn from(value: u32) -> Self {
        BusId::new(value)
    }
}

impl From<BusId> for u32 {
    fn from(id: BusId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_bus_is_zero() {
        assert_eq!(BusId::MASTER.value(), 0);
        assert!(BusId::MASTER.is_master());
    }

    #[test]
    fn default_is_master() {
        assert_eq!(BusId::default(), BusId::MASTER);
    }

    #[test]
    fn new_zero_is_master() {
        let id = BusId::new(0);
        assert!(id.is_master());
    }

    #[test]
    fn new_nonzero_is_not_master() {
        let id = BusId::new(7);
        assert!(!id.is_master());
        assert_eq!(id.value(), 7);
    }

    #[test]
    fn from_u32_round_trips() {
        let id: BusId = 42u32.into();
        let raw: u32 = id.into();
        assert_eq!(raw, 42);
    }

    #[test]
    fn ordering_matches_raw_value() {
        assert!(BusId::new(1) < BusId::new(2));
        assert!(BusId::MASTER < BusId::new(1));
    }

    #[test]
    fn equality_and_hash_are_value_based() {
        assert_eq!(BusId::new(3), BusId::new(3));
        assert_ne!(BusId::new(3), BusId::new(4));
    }

    #[test]
    fn display_shows_raw_value() {
        assert_eq!(format!("{}", BusId::new(5)), "5");
        assert_eq!(format!("{}", BusId::MASTER), "0");
    }
}
