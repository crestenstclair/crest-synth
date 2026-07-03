// path: src/sample/zone_resolver.rs
//
// ZoneResolver — domain service for the Sample context.
//
// Given a note (key) and velocity, finds every zone in a `SampleSet` whose
// key range and velocity range both match. `SampleSet` already owns the
// authoritative implementation of this query (`SampleSet::resolve`); this
// service exists as the named domain concept callers reach for, and keeps
// the query decoupled from any single `SampleSet` instance — it takes the
// aggregate to query as an explicit parameter rather than owning one, so it
// carries no state of its own and never instantiates a `SampleSet` itself.

use crate::sample::sample_set::{SampleSet, Zone};

/// Stateless domain service that resolves a note+velocity address against a
/// `SampleSet`, returning every zone whose key range and velocity range both
/// match.
#[derive(Debug, Default, Clone, Copy)]
pub struct ZoneResolver;

impl ZoneResolver {
    /// Construct a resolver. `ZoneResolver` holds no state, so this is a
    /// zero-cost convenience over `ZoneResolver::default()`.
    pub fn new() -> Self {
        Self
    }

    /// Resolve `(key, velocity)` against `sample_set`, returning every zone
    /// whose key range and velocity range both match.
    ///
    /// Delegates to `SampleSet::resolve`, the aggregate's own authoritative
    /// implementation of the "resolving a note returns every matching zone"
    /// invariant. Multiple matches are expected and intentional (layered
    /// zones); this never short-circuits on the first hit.
    pub fn resolve<'a>(&self, sample_set: &'a SampleSet, key: u8, velocity: u8) -> Vec<&'a Zone> {
        sample_set.resolve(key, velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::sample_set::{InterpolationMode, KeyRange, VelocityRange, Zone};

    fn zone(key_lo: u8, key_hi: u8, vel_lo: u8, vel_hi: u8) -> Zone {
        Zone::new(
            KeyRange::try_new(key_lo, key_hi).unwrap(),
            VelocityRange::try_new(vel_lo, vel_hi).unwrap(),
            "test-sample".to_string(),
        )
    }

    #[test]
    fn resolve_delegates_to_sample_set_and_returns_all_matches() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(60, 72, 0, 127));
        set.apply_add_zone(zone(60, 72, 0, 127));
        set.apply_add_zone(zone(73, 84, 0, 127));

        let resolver = ZoneResolver::new();
        let matches = resolver.resolve(&set, 65, 100);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn resolve_returns_empty_when_no_zone_matches() {
        let set = SampleSet::new(InterpolationMode::Linear);
        let resolver = ZoneResolver::new();
        assert!(resolver.resolve(&set, 65, 100).is_empty());
    }

    #[test]
    fn resolve_excludes_zone_with_non_matching_key_range() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(60, 72, 0, 127));

        let resolver = ZoneResolver::new();
        assert!(resolver.resolve(&set, 80, 100).is_empty());
    }

    #[test]
    fn resolve_excludes_zone_with_non_matching_velocity_range() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(60, 72, 0, 50));

        let resolver = ZoneResolver::new();
        assert!(resolver.resolve(&set, 65, 100).is_empty());
    }

    #[test]
    fn default_constructs_equivalent_resolver() {
        let set = SampleSet::new(InterpolationMode::None);
        let resolver = ZoneResolver;
        assert!(resolver.resolve(&set, 0, 0).is_empty());
    }
}
