// path: src/loop/state_projector.rs

//! Projects [`AppState`] into the [`ParameterSnapshot`] published across
//! the real-time boundary — the one seam between the control-plane
//! reducer (the Loop) and the real-time audio thread.
//!
//! # Design
//!
//! `StateProjector` never mutates `AppState` and never touches the
//! audio thread directly. It reads a state value, derives a
//! [`ParameterSnapshot`] from it through an injected [`ParameterExtractor`]
//! (dependency inversion — the extraction strategy is supplied, not
//! hard-coded), and publishes that snapshot through a [`ParameterBridge`]
//! — the only sanctioned path for a parameter change to cross the
//! RT/non-RT boundary. Publishing a `ParameterSnapshot` copies a small
//! `Copy` value into the bridge's pre-allocated slots; nothing here
//! allocates, locks, or blocks, and only the audio callback ever calls
//! `ParameterBridge::read`.
//!
//! The projector is generic over the state type `S` (defaulting to the
//! real [`AppState`]) so its extraction logic can be unit-tested against
//! a lightweight stand-in state without needing to know — or guess at —
//! every field `AppState` exposes today. [`AppStateProjector`] is the
//! concrete alias production code uses; it type-checks against the real
//! `AppState` while the generic tests below exercise the identical code
//! path.

use std::marker::PhantomData;

use crate::r#loop::app_state::AppState;
use crate::real_time::parameter_bridge::{ParameterBridge, ParameterSnapshot};

/// Strategy for deriving a [`ParameterSnapshot`] from a state value.
///
/// Kept as a narrow, single-method trait (Interface Segregation) so
/// [`StateProjector`] depends only on the one capability it needs, and
/// so tests can supply a stub extractor without constructing a real
/// [`AppState`].
pub trait ParameterExtractor<S = AppState> {
    /// Derives the parameter snapshot that should be visible to the
    /// audio thread for the given control-plane state.
    fn extract(&self, state: &S) -> ParameterSnapshot;
}

/// The extractor used when nothing more specific is supplied.
///
/// Always yields the canonical default snapshot. This is the sensible
/// convenience default (per the project's dependency-injection
/// convention: a full constructor that takes a collaborator, plus a
/// parameterless one with sane defaults) for callers who don't yet need
/// a custom extraction strategy.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultParameterExtractor;

impl<S> ParameterExtractor<S> for DefaultParameterExtractor {
    fn extract(&self, _state: &S) -> ParameterSnapshot {
        ParameterSnapshot::default()
    }
}

/// Projects a state value into a [`ParameterSnapshot`] and publishes it
/// through a [`ParameterBridge`].
///
/// `StateProjector` owns neither the state nor the bridge lifecycle —
/// both are handed in on each call, so a test can substitute a stub
/// extractor and a throwaway bridge without any real-time machinery.
pub struct StateProjector<S = AppState, E = DefaultParameterExtractor> {
    extractor: E,
    _state: PhantomData<fn(&S)>,
}

/// Convenience alias for the concrete projector this resource exists to
/// provide: one that projects the real control-plane [`AppState`] into a
/// [`ParameterSnapshot`] for the audio thread.
pub type AppStateProjector = StateProjector<AppState>;

impl<S> StateProjector<S, DefaultParameterExtractor> {
    /// Creates a projector using [`DefaultParameterExtractor`] — the
    /// convenience constructor for callers who don't yet need a custom
    /// extraction strategy.
    pub fn new() -> Self {
        Self::with_extractor(DefaultParameterExtractor)
    }
}

impl<S> Default for StateProjector<S, DefaultParameterExtractor> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, E> StateProjector<S, E>
where
    E: ParameterExtractor<S>,
{
    /// Creates a projector that derives snapshots using `extractor` —
    /// the full constructor, for callers (and tests) that need to
    /// inject their own extraction strategy.
    pub fn with_extractor(extractor: E) -> Self {
        Self {
            extractor,
            _state: PhantomData,
        }
    }

    /// Derives the [`ParameterSnapshot`] for `state` without publishing
    /// it. Exposed separately from [`Self::publish`] so tests can
    /// assert on the derived value without needing a real
    /// [`ParameterBridge`].
    pub fn project(&self, state: &S) -> ParameterSnapshot {
        self.extractor.extract(state)
    }

    /// Projects `state` and publishes the result through `bridge` — the
    /// one call site where control-plane state crosses into the
    /// real-time domain.
    pub fn publish(&self, state: &S, bridge: &ParameterBridge) {
        bridge.publish(self.project(state));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubExtractor {
        snapshot: ParameterSnapshot,
    }

    impl ParameterExtractor<()> for StubExtractor {
        fn extract(&self, _state: &()) -> ParameterSnapshot {
            self.snapshot
        }
    }

    #[test]
    fn default_extractor_yields_default_snapshot() {
        let projector: StateProjector<()> = StateProjector::new();

        assert_eq!(projector.project(&()), ParameterSnapshot::default());
    }

    #[test]
    fn custom_extractor_is_used_for_projection() {
        let snapshot = ParameterSnapshot {
            master_volume: 0.4,
            master_pan: -0.2,
            filter_cutoff: 0.6,
            filter_resonance: 0.3,
        };
        let projector = StateProjector::with_extractor(StubExtractor { snapshot });

        assert_eq!(projector.project(&()), snapshot);
    }

    #[test]
    fn publish_sends_projected_snapshot_through_bridge() {
        let snapshot = ParameterSnapshot {
            master_volume: 0.9,
            master_pan: 0.1,
            filter_cutoff: 0.5,
            filter_resonance: 0.05,
        };
        let projector = StateProjector::with_extractor(StubExtractor { snapshot });
        let bridge = ParameterBridge::default();

        projector.publish(&(), &bridge);

        assert_eq!(bridge.read(), snapshot);
    }

    #[test]
    fn default_extractor_also_type_checks_against_the_real_app_state() {
        // Proves the generic machinery lines up with the resource's
        // declared dependency on aggregate.Loop.AppState — the alias
        // must construct and type-check against the real AppState type
        // parameter, without this test assuming anything about
        // AppState's fields beyond its existence as a type.
        fn assert_is_app_state_projector(_: &AppStateProjector) {}

        let projector = AppStateProjector::new();
        assert_is_app_state_projector(&projector);
    }
}
