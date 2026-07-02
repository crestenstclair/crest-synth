// path: src/modulation/mod_matrix.rs

//! The `ModMatrix` aggregate: the set of active modulation routes and LFO
//! configurations for one patch.
//!
//! `ModMatrix` is a pure state value — no heap allocation performed here ever
//! reaches the audio thread directly. Commands are applied on the UI/command
//! thread; the resulting snapshot crosses the real-time boundary via the
//! `ParameterBridge` / `EventRing`, never by handing this aggregate itself to
//! the audio callback.

/// Number of LFO slots a `ModMatrix` always holds.
///
/// The matrix has exactly this many LFOs at all times — never more, never
/// fewer. Modeling `lfos` as a fixed-size array (rather than a `Vec`) makes
/// "there are exactly 4 LFOs" a structural guarantee instead of a runtime
/// check.
pub const LFO_COUNT: usize = 4;

// ── Value objects ────────────────────────────────────────────────────────
//
// `ModRoute` and `LfoConfig` are not yet published by another module in this
// workspace, so they are defined locally here per the resource's state
// declaration (`routes: list<ModRoute>`, `lfos: list<LfoConfig>`). If a
// dedicated `mod_route` / `lfo_config` module is generated later, these
// definitions should move there and this module should import them instead.

/// A single modulation routing: a source identifier and destination
/// identifier connected with a signed depth.
///
/// `ModRoute` is a plain data value — `Copy`, no heap allocation, safe to
/// pass across the real-time boundary inside a snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModRoute {
    /// Identifier of the modulation source (LFO index, envelope, MPE
    /// dimension, etc.). Opaque to the matrix — interpreted by the
    /// modulation processor.
    pub source_id: u32,
    /// Identifier of the modulation destination (parameter being modulated).
    /// Opaque to the matrix — interpreted by the modulation processor.
    pub destination_id: u32,
    /// Signed modulation depth. Not range-constrained by this aggregate;
    /// destination-specific clamping happens downstream.
    pub depth: f64,
}

impl ModRoute {
    /// Construct a new `ModRoute`.
    pub fn new(source_id: u32, destination_id: u32, depth: f64) -> Self {
        Self {
            source_id,
            destination_id,
            depth,
        }
    }
}

impl Default for ModRoute {
    fn default() -> Self {
        Self {
            source_id: 0,
            destination_id: 0,
            depth: 0.0,
        }
    }
}

/// Configuration for a single LFO slot.
///
/// `LfoConfig` is a plain data value — `Copy`, no heap allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfoConfig {
    /// Oscillation rate in Hz.
    pub rate_hz: f64,
    /// Modulation depth in `[0.0, 1.0]`. Not enforced by this aggregate;
    /// callers are expected to supply valid values.
    pub depth: f64,
    /// Initial phase in radians.
    pub phase: f64,
}

impl LfoConfig {
    /// Construct a new `LfoConfig`.
    pub fn new(rate_hz: f64, depth: f64, phase: f64) -> Self {
        Self {
            rate_hz,
            depth,
            phase,
        }
    }
}

impl Default for LfoConfig {
    fn default() -> Self {
        Self {
            rate_hz: 1.0,
            depth: 0.0,
            phase: 0.0,
        }
    }
}

// ── Commands ─────────────────────────────────────────────────────────────

/// Commands that can be applied to a [`ModMatrix`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModMatrixCommand {
    /// Append a new route to the matrix.
    AddRoute { route: ModRoute },
    /// Remove the route at `index`.
    RemoveRoute { index: u32 },
    /// Replace the LFO configuration at `index` (must be `< LFO_COUNT`).
    SetLfo { index: u8, config: LfoConfig },
}

// ── Events ───────────────────────────────────────────────────────────────

/// Domain events emitted by [`ModMatrix`] after a command is applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModMatrixEvent {
    /// A route was added at `index`.
    RouteAdded { index: u32 },
    /// The route at `index` was removed.
    RouteRemoved { index: u32 },
    /// The LFO at `index` was reconfigured.
    LfoSet { index: u8 },
}

// ── Errors ───────────────────────────────────────────────────────────────

/// Errors that can arise when applying a command to [`ModMatrix`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModMatrixError {
    /// `AddRoute` was rejected because the matrix already holds `maxRoutes`
    /// routes.
    MaxRoutesExceeded { max_routes: u8 },
    /// `RemoveRoute` was given an index that does not exist.
    RouteIndexOutOfBounds { index: u32, len: usize },
    /// `SetLfo` was given an index `>= LFO_COUNT`.
    LfoIndexOutOfBounds { index: u8 },
}

impl std::fmt::Display for ModMatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModMatrixError::MaxRoutesExceeded { max_routes } => {
                write!(
                    f,
                    "cannot add route: matrix already at maxRoutes ({max_routes})"
                )
            }
            ModMatrixError::RouteIndexOutOfBounds { index, len } => {
                write!(f, "route index {index} out of bounds (len = {len})")
            }
            ModMatrixError::LfoIndexOutOfBounds { index } => {
                write!(
                    f,
                    "lfo index {index} out of bounds (there are exactly {LFO_COUNT} lfos)"
                )
            }
        }
    }
}

impl std::error::Error for ModMatrixError {}

// ── Aggregate ────────────────────────────────────────────────────────────

/// The set of active modulation routes and LFO configurations for one patch.
///
/// # Audio-thread safety
///
/// `ModMatrix` is a pure state value. All mutation happens off the audio
/// thread via [`ModMatrix::apply`]; the resulting snapshot is handed to the
/// audio thread only through the `ParameterBridge` / `EventRing` boundary.
/// The audio thread never calls `apply` and never owns a mutable
/// `ModMatrix`.
///
/// # Invariants
///
/// - The matrix never holds more than `max_routes` routes.
/// - There are always exactly [`LFO_COUNT`] LFOs — enforced structurally by
///   storing `lfos` as a fixed-size array rather than a growable list.
#[derive(Debug, Clone, PartialEq)]
pub struct ModMatrix {
    lfos: [LfoConfig; LFO_COUNT],
    max_routes: u8,
    routes: Vec<ModRoute>,
}

impl ModMatrix {
    /// Construct an empty `ModMatrix` with default LFO configurations and
    /// the given route capacity.
    pub fn new(max_routes: u8) -> Self {
        Self {
            lfos: [LfoConfig::default(); LFO_COUNT],
            max_routes,
            routes: Vec::new(),
        }
    }

    /// Returns the configured route capacity.
    #[inline]
    pub fn max_routes(&self) -> u8 {
        self.max_routes
    }

    /// Returns the current routes, in order.
    #[inline]
    pub fn routes(&self) -> &[ModRoute] {
        &self.routes
    }

    /// Returns the four LFO configurations.
    #[inline]
    pub fn lfos(&self) -> &[LfoConfig; LFO_COUNT] {
        &self.lfos
    }

    /// Apply a command to the aggregate, returning the resulting event or an
    /// error if the command would violate an invariant.
    ///
    /// No heap allocation performed here ever executes on the audio thread;
    /// this method is called only from the UI / command thread.
    pub fn apply(&mut self, command: ModMatrixCommand) -> Result<ModMatrixEvent, ModMatrixError> {
        match command {
            ModMatrixCommand::AddRoute { route } => {
                if self.routes.len() >= self.max_routes as usize {
                    return Err(ModMatrixError::MaxRoutesExceeded {
                        max_routes: self.max_routes,
                    });
                }
                self.routes.push(route);
                let index = (self.routes.len() - 1) as u32;
                Ok(ModMatrixEvent::RouteAdded { index })
            }

            ModMatrixCommand::RemoveRoute { index } => {
                let idx = index as usize;
                if idx >= self.routes.len() {
                    return Err(ModMatrixError::RouteIndexOutOfBounds {
                        index,
                        len: self.routes.len(),
                    });
                }
                self.routes.remove(idx);
                Ok(ModMatrixEvent::RouteRemoved { index })
            }

            ModMatrixCommand::SetLfo { index, config } => {
                let idx = index as usize;
                if idx >= LFO_COUNT {
                    return Err(ModMatrixError::LfoIndexOutOfBounds { index });
                }
                self.lfos[idx] = config;
                Ok(ModMatrixEvent::LfoSet { index })
            }
        }
    }
}

impl Default for ModMatrix {
    /// Default matrix: no route capacity, four default LFOs, no routes.
    fn default() -> Self {
        Self::new(0)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn route(source_id: u32, destination_id: u32, depth: f64) -> ModRoute {
        ModRoute::new(source_id, destination_id, depth)
    }

    // ── Construction ─────────────────────────────────────────────────────

    #[test]
    fn new_matrix_has_exactly_four_lfos() {
        let m = ModMatrix::new(8);
        assert_eq!(m.lfos().len(), LFO_COUNT);
        assert_eq!(LFO_COUNT, 4);
    }

    #[test]
    fn new_matrix_has_no_routes() {
        let m = ModMatrix::new(8);
        assert!(m.routes().is_empty());
    }

    #[test]
    fn max_routes_accessor_reports_configured_value() {
        let m = ModMatrix::new(3);
        assert_eq!(m.max_routes(), 3);
    }

    #[test]
    fn default_matrix_has_zero_capacity_and_four_lfos() {
        let m = ModMatrix::default();
        assert_eq!(m.max_routes(), 0);
        assert_eq!(m.lfos().len(), LFO_COUNT);
    }

    // ── AddRoute ─────────────────────────────────────────────────────────

    #[test]
    fn add_route_appends_and_emits_event_with_index() {
        let mut m = ModMatrix::new(2);
        let evt = m
            .apply(ModMatrixCommand::AddRoute {
                route: route(1, 2, 0.5),
            })
            .unwrap();
        assert_eq!(evt, ModMatrixEvent::RouteAdded { index: 0 });
        assert_eq!(m.routes().len(), 1);
        assert_eq!(m.routes()[0], route(1, 2, 0.5));
    }

    #[test]
    fn add_route_second_route_gets_index_one() {
        let mut m = ModMatrix::new(2);
        m.apply(ModMatrixCommand::AddRoute {
            route: route(1, 2, 0.5),
        })
        .unwrap();
        let evt = m
            .apply(ModMatrixCommand::AddRoute {
                route: route(3, 4, -0.5),
            })
            .unwrap();
        assert_eq!(evt, ModMatrixEvent::RouteAdded { index: 1 });
        assert_eq!(m.routes().len(), 2);
    }

    #[test]
    fn add_route_never_exceeds_max_routes() {
        let mut m = ModMatrix::new(1);
        m.apply(ModMatrixCommand::AddRoute {
            route: route(1, 2, 0.5),
        })
        .unwrap();
        let result = m.apply(ModMatrixCommand::AddRoute {
            route: route(3, 4, 0.1),
        });
        assert_eq!(
            result,
            Err(ModMatrixError::MaxRoutesExceeded { max_routes: 1 })
        );
        // The matrix truly never exceeds maxRoutes.
        assert_eq!(m.routes().len(), 1);
    }

    #[test]
    fn add_route_at_zero_capacity_always_rejected() {
        let mut m = ModMatrix::new(0);
        let result = m.apply(ModMatrixCommand::AddRoute {
            route: route(1, 2, 0.5),
        });
        assert!(result.is_err());
        assert!(m.routes().is_empty());
    }

    // ── RemoveRoute ──────────────────────────────────────────────────────

    #[test]
    fn remove_route_removes_correct_index() {
        let mut m = ModMatrix::new(4);
        m.apply(ModMatrixCommand::AddRoute {
            route: route(1, 2, 0.1),
        })
        .unwrap();
        m.apply(ModMatrixCommand::AddRoute {
            route: route(3, 4, 0.2),
        })
        .unwrap();

        let evt = m.apply(ModMatrixCommand::RemoveRoute { index: 0 }).unwrap();
        assert_eq!(evt, ModMatrixEvent::RouteRemoved { index: 0 });
        assert_eq!(m.routes().len(), 1);
        assert_eq!(m.routes()[0], route(3, 4, 0.2));
    }

    #[test]
    fn remove_route_out_of_bounds_returns_error() {
        let mut m = ModMatrix::new(4);
        let result = m.apply(ModMatrixCommand::RemoveRoute { index: 5 });
        assert_eq!(
            result,
            Err(ModMatrixError::RouteIndexOutOfBounds { index: 5, len: 0 })
        );
    }

    #[test]
    fn remove_route_then_add_route_reuses_freed_capacity() {
        let mut m = ModMatrix::new(1);
        m.apply(ModMatrixCommand::AddRoute {
            route: route(1, 2, 0.1),
        })
        .unwrap();
        m.apply(ModMatrixCommand::RemoveRoute { index: 0 }).unwrap();
        // Capacity was freed, so adding again must succeed.
        let evt = m
            .apply(ModMatrixCommand::AddRoute {
                route: route(5, 6, 0.3),
            })
            .unwrap();
        assert_eq!(evt, ModMatrixEvent::RouteAdded { index: 0 });
    }

    // ── SetLfo ───────────────────────────────────────────────────────────

    #[test]
    fn set_lfo_replaces_config_and_emits_event() {
        let mut m = ModMatrix::new(4);
        let cfg = LfoConfig::new(2.0, 0.5, 0.0);
        let evt = m
            .apply(ModMatrixCommand::SetLfo {
                index: 0,
                config: cfg,
            })
            .unwrap();
        assert_eq!(evt, ModMatrixEvent::LfoSet { index: 0 });
        assert_eq!(m.lfos()[0], cfg);
    }

    #[test]
    fn set_lfo_accepts_all_four_valid_indices() {
        let mut m = ModMatrix::new(4);
        for i in 0..LFO_COUNT as u8 {
            let cfg = LfoConfig::new(1.0 + i as f64, 0.1, 0.0);
            let evt = m
                .apply(ModMatrixCommand::SetLfo {
                    index: i,
                    config: cfg,
                })
                .unwrap();
            assert_eq!(evt, ModMatrixEvent::LfoSet { index: i });
            assert_eq!(m.lfos()[i as usize], cfg);
        }
    }

    #[test]
    fn set_lfo_index_four_is_out_of_bounds() {
        let mut m = ModMatrix::new(4);
        let result = m.apply(ModMatrixCommand::SetLfo {
            index: 4,
            config: LfoConfig::default(),
        });
        assert_eq!(
            result,
            Err(ModMatrixError::LfoIndexOutOfBounds { index: 4 })
        );
    }

    #[test]
    fn set_lfo_out_of_bounds_leaves_lfos_unchanged() {
        let mut m = ModMatrix::new(4);
        let before = *m.lfos();
        let _ = m.apply(ModMatrixCommand::SetLfo {
            index: 200,
            config: LfoConfig::new(9.0, 1.0, 1.0),
        });
        assert_eq!(*m.lfos(), before);
    }

    #[test]
    fn set_lfo_never_changes_lfo_count() {
        let mut m = ModMatrix::new(4);
        m.apply(ModMatrixCommand::SetLfo {
            index: 2,
            config: LfoConfig::new(3.0, 0.2, 0.0),
        })
        .unwrap();
        // Structurally guaranteed, but assert explicitly for the invariant.
        assert_eq!(m.lfos().len(), 4);
    }

    // ── Error display ────────────────────────────────────────────────────

    #[test]
    fn error_display_max_routes_exceeded() {
        let e = ModMatrixError::MaxRoutesExceeded { max_routes: 6 };
        assert!(e.to_string().contains('6'));
    }

    #[test]
    fn error_display_route_index_out_of_bounds() {
        let e = ModMatrixError::RouteIndexOutOfBounds { index: 9, len: 2 };
        let s = e.to_string();
        assert!(s.contains('9'));
        assert!(s.contains('2'));
    }

    #[test]
    fn error_display_lfo_index_out_of_bounds() {
        let e = ModMatrixError::LfoIndexOutOfBounds { index: 7 };
        let s = e.to_string();
        assert!(s.contains('7'));
        assert!(s.contains('4'));
    }

    // ── Value objects ────────────────────────────────────────────────────

    #[test]
    fn mod_route_default_is_valid_zeroed_route() {
        let r = ModRoute::default();
        assert_eq!(r.source_id, 0);
        assert_eq!(r.destination_id, 0);
        assert_eq!(r.depth, 0.0);
    }

    #[test]
    fn lfo_config_default_has_positive_rate() {
        let c = LfoConfig::default();
        assert!(c.rate_hz > 0.0);
    }
}
