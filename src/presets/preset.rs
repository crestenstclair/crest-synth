// path: src/presets/preset.rs

//! Preset aggregate — serialized snapshot of a single patch's complete sound
//! and routing configuration.
//!
//! A preset captures every parameter needed to reproduce a saved sound:
//! oscillator, filter, amp envelope, sample player, modulation matrix, and
//! effect chain. Saving a preset reads state from the live `Patch`; loading
//! one writes state back.

use crate::effects::effect_processor::{EffectParams, EffectType};
use crate::modulation::lfo_config::LfoConfig;
use crate::modulation::mod_destination_type::ModDestinationType;
use crate::modulation::mod_envelope_config::ModEnvelopeConfig;
use crate::modulation::mod_source_type::ModSourceType;
use crate::patch::patch_id::PatchId;
use crate::presets::preset_id::PresetId;
use crate::presets::preset_metadata::PresetMetadata;
use crate::synth::amp_envelope_config::AmpEnvelopeConfig;
use crate::synth::filter_config::FilterConfig;
use crate::synth::oscillator_config::OscillatorConfig;
use crate::synth::sample_player_config::SamplePlayerConfig;

// ── EngineType ────────────────────────────────────────────────────────────────

/// Which sound-generation engine a preset uses.
///
/// The engine type determines which sub-configuration fields are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EngineType {
    /// Wavetable / analogue-modelling oscillator engine.
    #[default]
    Oscillator,
    /// Sample playback engine.
    SamplePlayer,
}

// ── SerializedEffectSlot ──────────────────────────────────────────────────────

/// A portable snapshot of one effect slot for preset serialization.
///
/// Captures the type, parameters, and bypass state; the live `EffectProcessor`
/// DSP state is not included because it would be wrong after deserialization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SerializedEffectSlot {
    /// Which algorithm this slot runs.
    pub effect_type: EffectType,
    /// Parameters frozen at save time.
    pub params: EffectParams,
    /// Whether this slot was bypassed at save time.
    pub bypass: bool,
}

// ── SerializedEffectChain ─────────────────────────────────────────────────────

/// A portable snapshot of an effect chain for preset serialization.
///
/// Serializes the ordered list of slots and the chain-level bypass flag.
/// The live DSP buffers and processor state are not included.
///
/// # Invariant
///
/// `slots` are in signal-flow order (index 0 is processed first).
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedEffectChain {
    /// Whether the chain was bypassed at save time.
    pub bypass: bool,
    /// Ordered list of effect slots (signal flow: index 0 → … → last).
    pub slots: Vec<SerializedEffectSlot>,
}

impl SerializedEffectChain {
    /// Returns an empty, non-bypassed chain snapshot.
    pub fn empty() -> Self {
        Self {
            bypass: false,
            slots: Vec::new(),
        }
    }
}

impl Default for SerializedEffectChain {
    fn default() -> Self {
        Self::empty()
    }
}

// ── SerializedModRouting ──────────────────────────────────────────────────────

/// A single modulation routing frozen for serialization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SerializedModRouting {
    pub source: ModSourceType,
    pub destination: ModDestinationType,
    /// Signed depth in `[-1.0, 1.0]`.
    pub depth: f64,
}

// ── SerializedModMatrix ───────────────────────────────────────────────────────

/// A portable snapshot of the modulation matrix for preset serialization.
///
/// Captures all LFO configs, mod envelope configs, and routings.
/// The snapshot is complete: loading it must reproduce the saved modulation
/// routing in full.
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedModMatrix {
    /// LFO configurations (indexed; index must match the engine's LFO index).
    pub lfo_configs: Vec<LfoConfig>,
    /// Mod envelope configurations (indexed).
    pub mod_envelopes: Vec<ModEnvelopeConfig>,
    /// Ordered modulation routings.
    pub routings: Vec<SerializedModRouting>,
}

impl SerializedModMatrix {
    /// Returns an empty mod matrix snapshot with no routings.
    pub fn empty() -> Self {
        Self {
            lfo_configs: Vec::new(),
            mod_envelopes: Vec::new(),
            routings: Vec::new(),
        }
    }
}

impl Default for SerializedModMatrix {
    fn default() -> Self {
        Self::empty()
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Commands that can be applied to a [`Preset`] aggregate.
///
/// Commands are the only way to mutate preset state. Each returns an event
/// on success or an error on invariant violation.
#[derive(Debug, Clone)]
pub enum PresetCommand {
    /// Capture a new preset from a patch's current state.
    SavePreset {
        patch_id: PatchId,
        metadata: PresetMetadata,
    },
    /// Mark this preset as loaded onto a target patch.
    LoadPreset { preset_id: PresetId },
    /// Overwrite the display metadata without re-capturing patch state.
    UpdateMetadata {
        preset_id: PresetId,
        metadata: PresetMetadata,
    },
    /// Delete this preset.
    DeletePreset { preset_id: PresetId },
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Domain events emitted by [`Preset`] after a command succeeds.
#[derive(Debug, Clone, PartialEq)]
pub enum PresetEvent {
    /// A preset was saved (created from a patch).
    PresetSaved { id: PresetId, name: String },
    /// A preset was loaded onto a patch.
    PresetLoaded {
        id: PresetId,
        target_patch_id: PatchId,
    },
    /// A preset's metadata was updated in-place.
    PresetMetadataUpdated { id: PresetId },
    /// A preset was deleted.
    PresetDeleted { id: PresetId },
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors that can arise when applying a command to a [`Preset`].
#[derive(Debug, Clone, PartialEq)]
pub enum PresetError {
    /// The command references a preset id that does not match this aggregate.
    IdMismatch { expected: PresetId, got: PresetId },
}

impl std::fmt::Display for PresetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresetError::IdMismatch { expected, got } => {
                write!(f, "preset id mismatch: expected {}, got {}", expected, got)
            }
        }
    }
}

impl std::error::Error for PresetError {}

// ── Preset aggregate ──────────────────────────────────────────────────────────

/// Serialized snapshot of a single patch's complete sound and routing
/// configuration.
///
/// # State
///
/// All fields are plain data values; no audio-thread primitives, no locks.
/// The `Preset` aggregate lives entirely outside the audio thread: it is
/// created/mutated by UI commands, persisted to disk, and reconstructed on
/// load. The audio thread never holds a `Preset` directly.
///
/// # Preset serialization completeness invariant
///
/// Every field required to reproduce the saved sound is captured:
/// - oscillator / sample player (engine selection + config)
/// - amp envelope (ADSR shape)
/// - filter (cutoff + resonance + type)
/// - modulation matrix (all routings, LFO configs, mod envelopes)
/// - effect chain (all slots in signal-flow order)
///
/// Loading a preset must be able to restore the original sound exactly.
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    /// Unique identifier for this preset.
    pub id: PresetId,
    /// Human-readable metadata (name, author, tags).
    pub metadata: PresetMetadata,
    /// Which sound engine this preset targets.
    pub engine_type: EngineType,
    /// Oscillator configuration (active when `engine_type == EngineType::Oscillator`).
    pub oscillator: OscillatorConfig,
    /// Amplitude envelope (ADSR).
    pub amp_envelope: AmpEnvelopeConfig,
    /// Filter parameters.
    pub filter: FilterConfig,
    /// Sample player configuration (active when `engine_type == EngineType::SamplePlayer`).
    pub sample_player: Option<SamplePlayerConfig>,
    /// Complete modulation matrix snapshot.
    pub mod_matrix: SerializedModMatrix,
    /// Complete effect chain snapshot.
    pub effect_chain: SerializedEffectChain,
}

impl Preset {
    /// Construct a `Preset` with explicit state values.
    ///
    /// This constructor is used when loading a preset from persistent storage.
    /// For creating a new preset from live patch state, use [`Preset::apply`]
    /// with a [`PresetCommand::SavePreset`] command.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PresetId,
        metadata: PresetMetadata,
        engine_type: EngineType,
        oscillator: OscillatorConfig,
        amp_envelope: AmpEnvelopeConfig,
        filter: FilterConfig,
        sample_player: Option<SamplePlayerConfig>,
        mod_matrix: SerializedModMatrix,
        effect_chain: SerializedEffectChain,
    ) -> Self {
        Self {
            id,
            metadata,
            engine_type,
            oscillator,
            amp_envelope,
            filter,
            sample_player,
            mod_matrix,
            effect_chain,
        }
    }

    /// Construct a default `Preset` with the given id and name.
    ///
    /// Produces an oscillator preset with factory defaults for all configs.
    pub fn default_for(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: PresetId::new(id),
            metadata: PresetMetadata::new(name, "", "Other", "", vec![]),
            engine_type: EngineType::Oscillator,
            oscillator: OscillatorConfig::default(),
            amp_envelope: AmpEnvelopeConfig::default(),
            filter: FilterConfig::default(),
            sample_player: None,
            mod_matrix: SerializedModMatrix::empty(),
            effect_chain: SerializedEffectChain::empty(),
        }
    }

    /// Apply a command, returning the emitted event on success.
    ///
    /// The `Preset` aggregate does not fetch live patch state itself — callers
    /// are responsible for providing the current `OscillatorConfig`,
    /// `AmpEnvelopeConfig`, etc. via field mutation before issuing a
    /// `SavePreset` command.
    pub fn apply(&mut self, command: PresetCommand) -> Result<PresetEvent, PresetError> {
        match command {
            PresetCommand::SavePreset {
                patch_id: _,
                metadata,
            } => {
                let name = metadata.name.clone();
                self.metadata = metadata;
                Ok(PresetEvent::PresetSaved {
                    id: self.id.clone(),
                    name,
                })
            }

            PresetCommand::LoadPreset { preset_id } => {
                if preset_id != self.id {
                    return Err(PresetError::IdMismatch {
                        expected: self.id.clone(),
                        got: preset_id,
                    });
                }
                // The caller supplies the target patch id via `load_onto`.
                // Here we record intent; target_patch_id is a sentinel.
                Ok(PresetEvent::PresetLoaded {
                    id: self.id.clone(),
                    target_patch_id: PatchId::new(0),
                })
            }

            PresetCommand::UpdateMetadata {
                preset_id,
                metadata,
            } => {
                if preset_id != self.id {
                    return Err(PresetError::IdMismatch {
                        expected: self.id.clone(),
                        got: preset_id,
                    });
                }
                self.metadata = metadata;
                Ok(PresetEvent::PresetMetadataUpdated {
                    id: self.id.clone(),
                })
            }

            PresetCommand::DeletePreset { preset_id } => {
                if preset_id != self.id {
                    return Err(PresetError::IdMismatch {
                        expected: self.id.clone(),
                        got: preset_id,
                    });
                }
                Ok(PresetEvent::PresetDeleted {
                    id: self.id.clone(),
                })
            }
        }
    }

    /// Produce a `PresetLoaded` event that names the target patch.
    ///
    /// The caller is responsible for reading back the state fields from this
    /// `Preset` and writing them to the target patch.
    pub fn load_onto(&self, target_patch_id: PatchId) -> PresetEvent {
        PresetEvent::PresetLoaded {
            id: self.id.clone(),
            target_patch_id,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::filter_config::FilterType;
    use crate::synth::oscillator_config::Waveform;

    fn make_preset(id: &str) -> Preset {
        Preset::default_for(id, "Test Preset")
    }

    fn make_metadata(name: &str) -> PresetMetadata {
        PresetMetadata::new(name, "", "Other", "", vec![])
    }

    // ── EngineType ────────────────────────────────────────────────────────────

    #[test]
    fn engine_type_default_is_oscillator() {
        assert_eq!(EngineType::default(), EngineType::Oscillator);
    }

    #[test]
    fn engine_type_copy_semantics() {
        let a = EngineType::SamplePlayer;
        let b = a;
        assert_eq!(a, b);
    }

    // ── SerializedEffectChain ─────────────────────────────────────────────────

    #[test]
    fn serialized_effect_chain_empty_has_no_slots() {
        let chain = SerializedEffectChain::empty();
        assert!(!chain.bypass);
        assert!(chain.slots.is_empty());
    }

    #[test]
    fn serialized_effect_chain_preserves_slot_order() {
        let slot0 = SerializedEffectSlot {
            effect_type: EffectType::Gain,
            params: EffectParams::default(),
            bypass: false,
        };
        let slot1 = SerializedEffectSlot {
            effect_type: EffectType::Delay,
            params: EffectParams::default(),
            bypass: true,
        };
        let chain = SerializedEffectChain {
            bypass: false,
            slots: vec![slot0, slot1],
        };
        assert_eq!(chain.slots[0].effect_type, EffectType::Gain);
        assert_eq!(chain.slots[1].effect_type, EffectType::Delay);
        assert!(chain.slots[1].bypass);
    }

    // ── SerializedModMatrix ───────────────────────────────────────────────────

    #[test]
    fn serialized_mod_matrix_empty_has_no_routings() {
        let m = SerializedModMatrix::empty();
        assert!(m.lfo_configs.is_empty());
        assert!(m.mod_envelopes.is_empty());
        assert!(m.routings.is_empty());
    }

    #[test]
    fn serialized_mod_matrix_routing_fields_preserved() {
        let routing = SerializedModRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.5,
        };
        let m = SerializedModMatrix {
            lfo_configs: Vec::new(),
            mod_envelopes: Vec::new(),
            routings: vec![routing],
        };
        assert_eq!(m.routings[0].source, ModSourceType::Lfo);
        assert!((m.routings[0].depth - 0.5).abs() < f64::EPSILON);
    }

    // ── Preset::default_for ───────────────────────────────────────────────────

    #[test]
    fn preset_default_for_has_correct_id_and_name() {
        let p = make_preset("bright-pad");
        assert_eq!(p.id, PresetId::new("bright-pad"));
        assert_eq!(p.metadata.name, "Test Preset");
    }

    #[test]
    fn preset_default_for_is_oscillator_engine() {
        let p = make_preset("p1");
        assert_eq!(p.engine_type, EngineType::Oscillator);
    }

    #[test]
    fn preset_default_for_has_no_sample_player() {
        let p = make_preset("p1");
        assert!(p.sample_player.is_none());
    }

    #[test]
    fn preset_default_for_has_empty_effect_chain() {
        let p = make_preset("p1");
        assert!(p.effect_chain.slots.is_empty());
    }

    #[test]
    fn preset_default_for_has_empty_mod_matrix() {
        let p = make_preset("p1");
        assert!(p.mod_matrix.routings.is_empty());
    }

    // ── Preset::new ───────────────────────────────────────────────────────────

    #[test]
    fn preset_new_round_trips_all_fields() {
        let id = PresetId::new("warm-pad");
        let metadata = PresetMetadata::new(
            "Warm Pad",
            "Alice",
            "Pad",
            "2025-01-01T00:00:00Z",
            vec!["ambient".to_string()],
        );
        let osc = OscillatorConfig::try_new(0.0, 0.5, Waveform::Saw).unwrap();
        let env = AmpEnvelopeConfig::try_new(0.1, 0.2, 0.7, 0.4).unwrap();
        let filter = FilterConfig::try_new(2_000.0, FilterType::LowPass, 0.3).unwrap();
        let mod_m = SerializedModMatrix::empty();
        let fx = SerializedEffectChain::empty();

        let p = Preset::new(
            id.clone(),
            metadata.clone(),
            EngineType::Oscillator,
            osc,
            env,
            filter,
            None,
            mod_m,
            fx,
        );

        assert_eq!(p.id, id);
        assert_eq!(p.metadata, metadata);
        assert_eq!(p.engine_type, EngineType::Oscillator);
        assert_eq!(p.oscillator, osc);
        assert_eq!(p.amp_envelope, env);
        assert!(p.sample_player.is_none());
    }

    // ── Preset::apply – SavePreset ────────────────────────────────────────────

    #[test]
    fn apply_save_preset_emits_saved_event_with_name() {
        let mut p = make_preset("lead-001");
        let cmd = PresetCommand::SavePreset {
            patch_id: PatchId::new(0),
            metadata: make_metadata("Lead"),
        };
        let evt = p.apply(cmd).unwrap();
        assert_eq!(
            evt,
            PresetEvent::PresetSaved {
                id: PresetId::new("lead-001"),
                name: "Lead".to_owned(),
            }
        );
        assert_eq!(p.metadata.name, "Lead");
    }

    #[test]
    fn apply_save_preset_updates_metadata_in_place() {
        let mut p = make_preset("p2");
        p.apply(PresetCommand::SavePreset {
            patch_id: PatchId::new(1),
            metadata: make_metadata("New Name"),
        })
        .unwrap();
        assert_eq!(p.metadata.name, "New Name");
    }

    // ── Preset::apply – LoadPreset ────────────────────────────────────────────

    #[test]
    fn apply_load_preset_emits_loaded_event() {
        let mut p = make_preset("pad-005");
        let cmd = PresetCommand::LoadPreset {
            preset_id: PresetId::new("pad-005"),
        };
        let evt = p.apply(cmd).unwrap();
        assert!(
            matches!(evt, PresetEvent::PresetLoaded { id, .. } if id == PresetId::new("pad-005"))
        );
    }

    #[test]
    fn apply_load_preset_id_mismatch_returns_error() {
        let mut p = make_preset("pad-005");
        let cmd = PresetCommand::LoadPreset {
            preset_id: PresetId::new("different-id"),
        };
        let err = p.apply(cmd).unwrap_err();
        assert!(matches!(err, PresetError::IdMismatch { .. }));
    }

    // ── Preset::apply – UpdateMetadata ────────────────────────────────────────

    #[test]
    fn apply_update_metadata_changes_name_and_emits_event() {
        let mut p = make_preset("bass-007");
        let cmd = PresetCommand::UpdateMetadata {
            preset_id: PresetId::new("bass-007"),
            metadata: make_metadata("Renamed Bass"),
        };
        let evt = p.apply(cmd).unwrap();
        assert_eq!(
            evt,
            PresetEvent::PresetMetadataUpdated {
                id: PresetId::new("bass-007"),
            }
        );
        assert_eq!(p.metadata.name, "Renamed Bass");
    }

    #[test]
    fn apply_update_metadata_id_mismatch_returns_error() {
        let mut p = make_preset("bass-007");
        let cmd = PresetCommand::UpdateMetadata {
            preset_id: PresetId::new("other"),
            metadata: make_metadata("Whatever"),
        };
        let err = p.apply(cmd).unwrap_err();
        assert!(matches!(err, PresetError::IdMismatch { .. }));
    }

    // ── Preset::apply – DeletePreset ──────────────────────────────────────────

    #[test]
    fn apply_delete_preset_emits_deleted_event() {
        let mut p = make_preset("del-preset");
        let cmd = PresetCommand::DeletePreset {
            preset_id: PresetId::new("del-preset"),
        };
        let evt = p.apply(cmd).unwrap();
        assert_eq!(
            evt,
            PresetEvent::PresetDeleted {
                id: PresetId::new("del-preset"),
            }
        );
    }

    #[test]
    fn apply_delete_preset_id_mismatch_returns_error() {
        let mut p = make_preset("del-preset");
        let cmd = PresetCommand::DeletePreset {
            preset_id: PresetId::new("wrong-id"),
        };
        let err = p.apply(cmd).unwrap_err();
        assert!(matches!(err, PresetError::IdMismatch { .. }));
    }

    // ── Preset::load_onto ─────────────────────────────────────────────────────

    #[test]
    fn load_onto_returns_loaded_event_with_target_patch_id() {
        let p = make_preset("sunny-lead");
        let target = PatchId::new(7);
        let evt = p.load_onto(target);
        assert_eq!(
            evt,
            PresetEvent::PresetLoaded {
                id: PresetId::new("sunny-lead"),
                target_patch_id: PatchId::new(7),
            }
        );
    }

    // ── Preset completeness: all state fields round-trip ─────────────────────

    #[test]
    fn preset_captures_complete_state_including_mod_and_effects() {
        let routing = SerializedModRouting {
            source: ModSourceType::PerNoteTimbreY,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.8,
        };
        let slot = SerializedEffectSlot {
            effect_type: EffectType::LowPassFilter,
            params: EffectParams {
                effect_type: EffectType::LowPassFilter,
                cutoff_hz: 2_000.0,
                resonance: 0.5,
                ..EffectParams::default()
            },
            bypass: false,
        };

        let p = Preset::new(
            PresetId::new("full-preset"),
            make_metadata("Full"),
            EngineType::Oscillator,
            OscillatorConfig::default(),
            AmpEnvelopeConfig::default(),
            FilterConfig::default(),
            None,
            SerializedModMatrix {
                lfo_configs: Vec::new(),
                mod_envelopes: Vec::new(),
                routings: vec![routing],
            },
            SerializedEffectChain {
                bypass: false,
                slots: vec![slot],
            },
        );

        assert_eq!(
            p.mod_matrix.routings[0].source,
            ModSourceType::PerNoteTimbreY
        );
        assert_eq!(
            p.effect_chain.slots[0].effect_type,
            EffectType::LowPassFilter
        );
        assert!(!p.effect_chain.bypass);
    }

    // ── Preset with SamplePlayer engine ──────────────────────────────────────

    #[test]
    fn preset_sample_player_engine_stores_config() {
        use crate::synth::sample_player_config::{InterpolationMode, LoopMode, SamplePlayerConfig};
        let sp_cfg =
            SamplePlayerConfig::try_new(1, InterpolationMode::Linear, LoopMode::Sustain).unwrap();
        let p = Preset::new(
            PresetId::new("samp-001"),
            make_metadata("Sample Preset"),
            EngineType::SamplePlayer,
            OscillatorConfig::default(),
            AmpEnvelopeConfig::default(),
            FilterConfig::default(),
            Some(sp_cfg),
            SerializedModMatrix::empty(),
            SerializedEffectChain::empty(),
        );
        assert_eq!(p.engine_type, EngineType::SamplePlayer);
        assert!(p.sample_player.is_some());
        assert_eq!(p.sample_player.unwrap(), sp_cfg);
    }

    // ── Error display ─────────────────────────────────────────────────────────

    #[test]
    fn preset_error_id_mismatch_display_contains_mismatch() {
        let e = PresetError::IdMismatch {
            expected: PresetId::new("expected-id"),
            got: PresetId::new("got-id"),
        };
        let s = e.to_string();
        assert!(s.contains("mismatch"), "expected 'mismatch' in: {s}");
    }
}
