// path: src/modulation/mod_matrix.rs

use crate::modulation::lfo_config::LfoConfig;
use crate::modulation::mod_destination_type::ModDestinationType;
use crate::modulation::mod_envelope_config::ModEnvelopeConfig;
use crate::modulation::mod_routing::{ModRouting, ModRoutingError};
use crate::modulation::mod_source_type::ModSourceType;
use crate::patch::patch_id::PatchId;

// ── Commands ─────────────────────────────────────────────────────────────────────────────

/// Commands that can be applied to a [`ModMatrix`].
#[derive(Debug, Clone)]
pub enum ModMatrixCommand {
    /// Add an LFO routing slot for the given index.
    ConfigureLfo { lfo_index: u8, config: LfoConfig },
    /// Configure the mod envelope at the given index.
    ConfigureModEnvelope {
        env_index: u8,
        config: ModEnvelopeConfig,
    },
    /// Add a new routing from `source` to `destination` with `depth`.
    AddRouting {
        source: ModSourceType,
        destination: ModDestinationType,
        depth: f64,
    },
    /// Remove the routing at `routing_index`.
    RemoveRouting { routing_index: u8 },
    /// Update the depth of the routing at `routing_index`.
    UpdateRoutingDepth { routing_index: u8, depth: f64 },
}

// ── Events ─────────────────────────────────────────────────────────────────────────────

/// Domain events emitted by [`ModMatrix`] after a command is applied.
#[derive(Debug, Clone, PartialEq)]
pub enum ModMatrixEvent {
    /// A routing was added.
    RoutingAdded {
        source: ModSourceType,
        destination: ModDestinationType,
        depth: f64,
    },
    /// A routing was removed.
    RoutingRemoved { routing_index: u8 },
    /// A routing's depth was changed.
    RoutingDepthChanged { routing_index: u8, depth: f64 },
    /// An LFO was configured.
    LfoConfigured { lfo_index: u8 },
    /// A mod envelope was configured.
    ModEnvelopeConfigured { env_index: u8 },
}

// ── Errors ────────────────────────────────────────────────────────────────────────────

/// Errors that can arise when applying a command to [`ModMatrix`].
#[derive(Debug, Clone, PartialEq)]
pub enum ModMatrixError {
    /// The routing depth was outside `[-1.0, 1.0]`.
    InvalidDepth(ModRoutingError),
    /// The routing index is out of bounds.
    RoutingIndexOutOfBounds { index: u8, len: usize },
}

impl From<ModRoutingError> for ModMatrixError {
    fn from(e: ModRoutingError) -> Self {
        ModMatrixError::InvalidDepth(e)
    }
}

impl std::fmt::Display for ModMatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModMatrixError::InvalidDepth(e) => write!(f, "invalid depth: {e}"),
            ModMatrixError::RoutingIndexOutOfBounds { index, len } => {
                write!(f, "routing index {index} out of bounds (len = {len})")
            }
        }
    }
}

impl std::error::Error for ModMatrixError {}

// ── Aggregate ─────────────────────────────────────────────────────────────────────────────

/// Per-patch modulation routing aggregate.
///
/// Maps modulation sources (LFOs, envelopes, per-note expression, etc.) to
/// destinations (filter cutoff, oscillator pitch, amplitude…) with an
/// adjustable signed depth per routing.
///
/// # Audio-thread safety
///
/// `ModMatrix` is a pure state value — all heap allocation happens off the
/// audio thread (when commands are applied) and the resulting routing list is
/// passed across the boundary via `ParameterBridge` / `EventRingBuffer`.
/// The audio thread reads the snapshot; it never holds a `ModMatrix` directly.
///
/// # Invariants
///
/// - Every stored routing has depth in `[-1.0, 1.0]`.
/// - LFOs and macros are per-patch: they are configured once in the matrix and
///   shared by every voice in the patch.
/// - Per-note expression sources (`PerNoteBendX`, `PerNoteTimbreY`,
///   `PerNotePressureZ`) are per-voice: the engine delivers their values to
///   the individual voice, never to the patch as a whole.
#[derive(Debug, Clone)]
pub struct ModMatrix {
    /// The patch this matrix belongs to.
    patch_id: PatchId,
    /// LFO configurations (per-patch, indexed).
    lfo_configs: Vec<LfoConfig>,
    /// Mod envelope configurations (per-patch, indexed).
    mod_envelopes: Vec<ModEnvelopeConfig>,
    /// Ordered list of modulation routings.
    routings: Vec<ModRouting>,
}

impl ModMatrix {
    /// Construct an empty `ModMatrix` for a given patch.
    pub fn new(patch_id: PatchId) -> Self {
        Self {
            patch_id,
            lfo_configs: Vec::new(),
            mod_envelopes: Vec::new(),
            routings: Vec::new(),
        }
    }

    /// Returns the patch this matrix belongs to.
    pub fn patch_id(&self) -> PatchId {
        self.patch_id
    }

    /// Returns a slice of all LFO configurations.
    ///
    /// LFOs are per-patch — one LFO state is shared across all voices.
    pub fn lfo_configs(&self) -> &[LfoConfig] {
        &self.lfo_configs
    }

    /// Returns a slice of all mod envelope configurations.
    pub fn mod_envelopes(&self) -> &[ModEnvelopeConfig] {
        &self.mod_envelopes
    }

    /// Returns a slice of all modulation routings.
    pub fn routings(&self) -> &[ModRouting] {
        &self.routings
    }

    /// Apply a command to the aggregate.
    ///
    /// Returns the domain event to be stored/broadcast, or an error if the
    /// command violates an invariant.
    ///
    /// No heap allocation occurs inside the audio thread; this method is
    /// expected to be called from the UI / command thread, not the audio thread.
    pub fn apply(&mut self, command: ModMatrixCommand) -> Result<ModMatrixEvent, ModMatrixError> {
        match command {
            ModMatrixCommand::ConfigureLfo { lfo_index, config } => {
                let idx = lfo_index as usize;
                if idx >= self.lfo_configs.len() {
                    self.lfo_configs.resize_with(idx + 1, LfoConfig::default);
                }
                self.lfo_configs[idx] = config;
                Ok(ModMatrixEvent::LfoConfigured { lfo_index })
            }

            ModMatrixCommand::ConfigureModEnvelope { env_index, config } => {
                let idx = env_index as usize;
                if idx >= self.mod_envelopes.len() {
                    self.mod_envelopes
                        .resize_with(idx + 1, ModEnvelopeConfig::default);
                }
                self.mod_envelopes[idx] = config;
                Ok(ModMatrixEvent::ModEnvelopeConfigured { env_index })
            }

            ModMatrixCommand::AddRouting {
                source,
                destination,
                depth,
            } => {
                let routing = ModRouting::try_new(source, destination, depth)?;
                self.routings.push(routing);
                Ok(ModMatrixEvent::RoutingAdded {
                    source,
                    destination,
                    depth,
                })
            }

            ModMatrixCommand::RemoveRouting { routing_index } => {
                let idx = routing_index as usize;
                if idx >= self.routings.len() {
                    return Err(ModMatrixError::RoutingIndexOutOfBounds {
                        index: routing_index,
                        len: self.routings.len(),
                    });
                }
                self.routings.remove(idx);
                Ok(ModMatrixEvent::RoutingRemoved { routing_index })
            }

            ModMatrixCommand::UpdateRoutingDepth {
                routing_index,
                depth,
            } => {
                let idx = routing_index as usize;
                if idx >= self.routings.len() {
                    return Err(ModMatrixError::RoutingIndexOutOfBounds {
                        index: routing_index,
                        len: self.routings.len(),
                    });
                }
                let updated = ModRouting::try_new(
                    self.routings[idx].source(),
                    self.routings[idx].destination(),
                    depth,
                )?;
                self.routings[idx] = updated;
                Ok(ModMatrixEvent::RoutingDepthChanged {
                    routing_index,
                    depth,
                })
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modulation::lfo_waveform::LfoWaveform;

    fn patch() -> PatchId {
        PatchId::new(1)
    }

    fn empty_matrix() -> ModMatrix {
        ModMatrix::new(patch())
    }

    // ── AddRouting ────────────────────────────────────────────────────────────────────────

    #[test]
    fn add_routing_appends_and_emits_event() {
        let mut m = empty_matrix();
        let evt = m
            .apply(ModMatrixCommand::AddRouting {
                source: ModSourceType::Lfo,
                destination: ModDestinationType::FilterCutoff,
                depth: 0.5,
            })
            .unwrap();

        assert_eq!(
            evt,
            ModMatrixEvent::RoutingAdded {
                source: ModSourceType::Lfo,
                destination: ModDestinationType::FilterCutoff,
                depth: 0.5,
            }
        );
        assert_eq!(m.routings().len(), 1);
        assert!((m.routings()[0].depth() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn add_routing_rejects_depth_out_of_range() {
        let mut m = empty_matrix();
        let result = m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 1.5,
        });
        assert!(result.is_err());
    }

    #[test]
    fn add_routing_negative_depth_accepted() {
        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Envelope,
            destination: ModDestinationType::Amplitude,
            depth: -1.0,
        })
        .unwrap();
        assert!((m.routings()[0].depth() - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn add_routing_nan_depth_rejected() {
        let mut m = empty_matrix();
        assert!(m
            .apply(ModMatrixCommand::AddRouting {
                source: ModSourceType::Lfo,
                destination: ModDestinationType::FilterCutoff,
                depth: f64::NAN,
            })
            .is_err());
    }

    // ── RemoveRouting ────────────────────────────────────────────────────────────────────────

    #[test]
    fn remove_routing_removes_correct_index() {
        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.5,
        })
        .unwrap();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Envelope,
            destination: ModDestinationType::Amplitude,
            depth: -0.3,
        })
        .unwrap();
        assert_eq!(m.routings().len(), 2);

        let evt = m
            .apply(ModMatrixCommand::RemoveRouting { routing_index: 0 })
            .unwrap();
        assert_eq!(evt, ModMatrixEvent::RoutingRemoved { routing_index: 0 });
        assert_eq!(m.routings().len(), 1);
        // Remaining routing is the second one (Envelope → Amplitude)
        assert_eq!(m.routings()[0].source(), ModSourceType::Envelope);
    }

    #[test]
    fn remove_routing_out_of_bounds_returns_error() {
        let mut m = empty_matrix();
        let result = m.apply(ModMatrixCommand::RemoveRouting { routing_index: 5 });
        assert!(matches!(
            result,
            Err(ModMatrixError::RoutingIndexOutOfBounds { .. })
        ));
    }

    // ── UpdateRoutingDepth ──────────────────────────────────────────────────────────────────────

    #[test]
    fn update_routing_depth_changes_value() {
        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.5,
        })
        .unwrap();

        let evt = m
            .apply(ModMatrixCommand::UpdateRoutingDepth {
                routing_index: 0,
                depth: -0.8,
            })
            .unwrap();
        assert_eq!(
            evt,
            ModMatrixEvent::RoutingDepthChanged {
                routing_index: 0,
                depth: -0.8
            }
        );
        assert!((m.routings()[0].depth() - (-0.8)).abs() < f64::EPSILON);
    }

    #[test]
    fn update_routing_depth_invalid_depth_rejected() {
        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.0,
        })
        .unwrap();
        assert!(m
            .apply(ModMatrixCommand::UpdateRoutingDepth {
                routing_index: 0,
                depth: 2.0,
            })
            .is_err());
    }

    #[test]
    fn update_routing_depth_out_of_bounds_returns_error() {
        let mut m = empty_matrix();
        let result = m.apply(ModMatrixCommand::UpdateRoutingDepth {
            routing_index: 10,
            depth: 0.5,
        });
        assert!(matches!(
            result,
            Err(ModMatrixError::RoutingIndexOutOfBounds { .. })
        ));
    }

    // ── ConfigureLfo ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn configure_lfo_stores_config_and_emits_event() {
        let mut m = empty_matrix();
        let cfg = LfoConfig::try_new(2.0, 0.5, 0.0, false, LfoWaveform::Sine).unwrap();
        let evt = m
            .apply(ModMatrixCommand::ConfigureLfo {
                lfo_index: 0,
                config: cfg.clone(),
            })
            .unwrap();

        assert_eq!(evt, ModMatrixEvent::LfoConfigured { lfo_index: 0 });
        assert_eq!(m.lfo_configs().len(), 1);
        assert!((m.lfo_configs()[0].rate - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn configure_lfo_expands_vec_for_non_zero_index() {
        let mut m = empty_matrix();
        let cfg = LfoConfig::default();
        m.apply(ModMatrixCommand::ConfigureLfo {
            lfo_index: 2,
            config: cfg,
        })
        .unwrap();
        // Slots 0, 1, and 2 should all exist after gapping
        assert_eq!(m.lfo_configs().len(), 3);
    }

    #[test]
    fn configure_lfo_is_per_patch_shared_comment() {
        // LFOs are per-patch: configuring them at the matrix level affects all voices
        let mut m = empty_matrix();
        let cfg = LfoConfig::try_new(4.0, 1.0, 0.0, false, LfoWaveform::Square).unwrap();
        m.apply(ModMatrixCommand::ConfigureLfo {
            lfo_index: 0,
            config: cfg,
        })
        .unwrap();
        // All voices in this patch see the same LFO config
        assert!((m.lfo_configs()[0].rate - 4.0).abs() < f64::EPSILON);
    }

    // ── ConfigureModEnvelope ───────────────────────────────────────────────────────────────────

    #[test]
    fn configure_mod_envelope_stores_config_and_emits_event() {
        let mut m = empty_matrix();
        let cfg = ModEnvelopeConfig::try_new(0.01, 0.1, 0.7, 0.4).unwrap();
        let evt = m
            .apply(ModMatrixCommand::ConfigureModEnvelope {
                env_index: 0,
                config: cfg,
            })
            .unwrap();
        assert_eq!(evt, ModMatrixEvent::ModEnvelopeConfigured { env_index: 0 });
        assert_eq!(m.mod_envelopes().len(), 1);
        assert!((m.mod_envelopes()[0].attack - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn configure_mod_envelope_expands_vec_for_non_zero_index() {
        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::ConfigureModEnvelope {
            env_index: 3,
            config: ModEnvelopeConfig::default(),
        })
        .unwrap();
        assert_eq!(m.mod_envelopes().len(), 4);
    }

    // ── Accessors ──────────────────────────────────────────────────────────────────────────

    #[test]
    fn patch_id_accessor() {
        let m = ModMatrix::new(PatchId::new(42));
        assert_eq!(m.patch_id(), PatchId::new(42));
    }

    #[test]
    fn empty_matrix_has_no_routings() {
        let m = empty_matrix();
        assert!(m.routings().is_empty());
        assert!(m.lfo_configs().is_empty());
        assert!(m.mod_envelopes().is_empty());
    }

    // ── Per-note expression routing ────────────────────────────────────────────────

    #[test]
    fn per_note_expression_routing_tracked_correctly() {
        let mut m = empty_matrix();
        // PerNoteBendX → OscillatorPitch is a per-note expression routing
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::PerNoteBendX,
            destination: ModDestinationType::OscillatorPitch,
            depth: 1.0,
        })
        .unwrap();
        let routing = &m.routings()[0];
        assert!(routing.is_per_note_expression_routing());
    }

    #[test]
    fn per_note_expression_routing_distinct_from_patch_level() {
        // Adding a per-note expression routing alongside a patch-level LFO routing
        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo, // per-patch
            destination: ModDestinationType::FilterCutoff,
            depth: 0.7,
        })
        .unwrap();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::PerNotePressureZ, // per-voice
            destination: ModDestinationType::Amplitude,
            depth: 0.5,
        })
        .unwrap();
        assert!(!m.routings()[0].is_per_note_expression_routing());
        assert!(m.routings()[1].is_per_note_expression_routing());
    }

    // ── Error display ──────────────────────────────────────────────────────────────────

    #[test]
    fn error_display_depth_out_of_range() {
        use crate::modulation::mod_routing::ModRoutingError;
        let e = ModMatrixError::InvalidDepth(ModRoutingError::DepthOutOfRange(3.0));
        assert!(e.to_string().contains("3"));
    }

    #[test]
    fn error_display_index_out_of_bounds() {
        let e = ModMatrixError::RoutingIndexOutOfBounds { index: 7, len: 3 };
        let s = e.to_string();
        assert!(s.contains("7"), "expected index 7 in: {s}");
        assert!(s.contains("3"), "expected len 3 in: {s}");
    }
}
