// path: src/mixer/send_tap.rs

use crate::kernel::amplitude::Amplitude;

/// Identifier for an aux bus that a `SendTap` routes to.
///
/// Defined locally because no dedicated `BusId` module exists yet in the
/// kernel module tree; if one is introduced later, this type should be
/// re-pointed to import it instead of defining its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BusId(u32);

impl BusId {
    /// Construct a `BusId` from a raw index.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return the underlying raw index.
    #[inline]
    pub fn value(self) -> u32 {
        self.0
    }
}

/// One send from a channel strip to an aux bus.
///
/// Send taps are post-fader by default, matching mixing-console
/// convention; pre-fader is an explicit opt-in per send.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SendTap {
    bus: BusId,
    level: Amplitude,
    pre_fader: bool,
}

impl SendTap {
    /// Construct a post-fader send tap (the default per mixing-console
    /// convention).
    pub fn new(bus: BusId, level: Amplitude) -> Self {
        Self {
            bus,
            level,
            pre_fader: false,
        }
    }

    /// Construct a pre-fader send tap. Pre-fader is an explicit opt-in
    /// per send, never a default.
    pub fn pre_fader(bus: BusId, level: Amplitude) -> Self {
        Self {
            bus,
            level,
            pre_fader: true,
        }
    }

    /// The aux bus this send routes to.
    #[inline]
    pub fn bus(self) -> BusId {
        self.bus
    }

    /// The send level.
    #[inline]
    pub fn level(self) -> Amplitude {
        self.level
    }

    /// Whether this send taps the signal pre-fader (`true`) or
    /// post-fader (`false`, the default).
    #[inline]
    pub fn is_pre_fader(self) -> bool {
        self.pre_fader
    }

    /// Return a copy of this send tap with a new level.
    pub fn with_level(self, level: Amplitude) -> Self {
        Self { level, ..self }
    }

    /// Return a copy of this send tap with the pre-fader flag set
    /// explicitly.
    pub fn with_pre_fader(self, pre_fader: bool) -> Self {
        Self { pre_fader, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_to_post_fader() {
        let tap = SendTap::new(BusId::new(1), Amplitude::try_new(1.0).unwrap());
        assert!(!tap.is_pre_fader());
    }

    #[test]
    fn pre_fader_is_explicit_opt_in() {
        let tap = SendTap::pre_fader(BusId::new(1), Amplitude::try_new(1.0).unwrap());
        assert!(tap.is_pre_fader());
    }

    #[test]
    fn bus_and_level_accessors() {
        let bus = BusId::new(7);
        let level = Amplitude::try_new(0.5).unwrap();
        let tap = SendTap::new(bus, level);
        assert_eq!(tap.bus(), bus);
        assert_eq!(tap.level(), level);
    }

    #[test]
    fn with_level_updates_only_level() {
        let tap = SendTap::new(BusId::new(2), Amplitude::try_new(0.0).unwrap());
        let updated = tap.with_level(Amplitude::try_new(1.0).unwrap());
        assert_eq!(updated.level(), Amplitude::try_new(1.0).unwrap());
        assert_eq!(updated.bus(), tap.bus());
        assert_eq!(updated.is_pre_fader(), tap.is_pre_fader());
    }

    #[test]
    fn with_pre_fader_toggles_flag() {
        let tap = SendTap::new(BusId::new(3), Amplitude::try_new(1.0).unwrap());
        let pre = tap.with_pre_fader(true);
        assert!(pre.is_pre_fader());
        let post = pre.with_pre_fader(false);
        assert!(!post.is_pre_fader());
    }

    #[test]
    fn bus_id_round_trips_value() {
        let bus = BusId::new(42);
        assert_eq!(bus.value(), 42);
    }
}
