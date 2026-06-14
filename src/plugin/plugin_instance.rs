// path: src/plugin/plugin_instance.rs

//! `PluginInstance` aggregate — wraps the engine library as a plugin.
//!
//! Maintains the full plugin state: format, registered parameters, patch count,
//! and sample rate. Provides command dispatch for `Initialize`, `Reset`, and
//! `SetParameter`, persists state via the same [`PresetCodec`] used by the
//! standalone app, and normalizes host MIDI through the same
//! [`MidiNormalizerPort`] / [`StandardMidiNormalizer`] pipeline.
//!
//! # Audio-thread safety
//!
//! [`PluginInstance`] lives on the **control thread**. No heap allocation,
//! mutex acquisition, or blocking I/O may occur on the audio thread.

use crate::kernel::sample_rate::SampleRate;
use crate::plugin::parameter_id::ParameterId;
use crate::plugin::parameter_range::ParameterRange;
use crate::plugin::plugin_format::PluginFormat;
use crate::plugin::plugin_parameter::PluginParameter;
use crate::presets::preset::Preset;
use crate::presets::preset_codec::{CodecError, PresetCodec};
use crate::presets::setup::Setup;
use crate::shell::midi_normalizer::{MidiNormalizerPort, RawMidiMessage, StandardMidiNormalizer};

// ── Commands ──────────────────────────────────────────────────────────────────

/// Commands that can be dispatched to [`PluginInstance`].
#[derive(Debug, Clone)]
pub enum PluginCommand {
    /// Initialize the plugin with the host's audio configuration.
    Initialize {
        /// Maximum audio block size in samples.
        max_block_size: u32,
        /// Host sample rate.
        sample_rate: SampleRate,
    },
    /// Reset the plugin to its default state (all parameters → default).
    Reset,
    /// Update one parameter value (clamped to that parameter's range).
    SetParameter {
        /// The stable ID of the parameter to update.
        id: ParameterId,
        /// Requested new value (will be clamped to the parameter's range).
        value: f64,
    },
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Events emitted by [`PluginInstance`] in response to commands.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginEvent {
    /// Emitted after successful initialization.
    PluginInitialized {
        /// Sample rate the engine was initialized at.
        sample_rate: SampleRate,
    },
    /// Emitted after all parameters have been reset to their defaults.
    PluginReset,
    /// Emitted when a parameter value changes.
    ParameterChanged {
        /// Stable parameter identifier.
        id: ParameterId,
        /// New (clamped) value.
        value: f64,
    },
}

// ── PluginInstance ────────────────────────────────────────────────────────────

/// Aggregate that wraps the engine library as a plugin.
///
/// Responsibilities:
/// - Maintain the list of [`PluginParameter`]s (1:1 with engine parameters)
/// - Route parameter changes from the host to the engine via [`PluginCommand`]
/// - Persist and restore state via the shared [`PresetCodec`]
/// - Normalize host MIDI bytes through [`MidiNormalizerPort`]
///
/// # Dependency injection
///
/// [`PresetCodec`] and [`StandardMidiNormalizer`] are injected via the
/// constructor, making the aggregate testable without I/O or audio drivers.
/// Use [`PluginInstance::with_defaults`] when injection is not needed.
pub struct PluginInstance {
    /// Current plugin format (CLAP or VST3).
    pub format: PluginFormat,
    /// Registered parameters, in stable ID order.
    pub parameters: Vec<PluginParameter>,
    /// Number of patches available in the engine.
    pub patch_count: u8,
    /// Sample rate the engine was initialized at.
    pub sample_rate: SampleRate,
    /// Codec for preset/setup serialization — same instance as standalone.
    codec: PresetCodec,
    /// MIDI normalizer for translating raw host MIDI bytes.
    normalizer: StandardMidiNormalizer,
}

impl PluginInstance {
    /// Create a new [`PluginInstance`] with explicit dependency injection.
    pub fn new(
        format: PluginFormat,
        parameters: Vec<PluginParameter>,
        patch_count: u8,
        sample_rate: SampleRate,
        codec: PresetCodec,
        normalizer: StandardMidiNormalizer,
    ) -> Self {
        Self {
            format,
            parameters,
            patch_count,
            sample_rate,
            codec,
            normalizer,
        }
    }

    /// Create a [`PluginInstance`] with default [`PresetCodec`] and
    /// [`StandardMidiNormalizer`].
    ///
    /// Suitable for production use when callers do not need to inject custom
    /// implementations.
    pub fn with_defaults(
        format: PluginFormat,
        parameters: Vec<PluginParameter>,
        patch_count: u8,
        sample_rate: SampleRate,
    ) -> Self {
        Self::new(
            format,
            parameters,
            patch_count,
            sample_rate,
            PresetCodec::new(),
            StandardMidiNormalizer::new(),
        )
    }

    // ── Command dispatch ──────────────────────────────────────────────────────

    /// Process a [`PluginCommand`] and return zero or more [`PluginEvent`]s.
    pub fn handle(&mut self, cmd: PluginCommand) -> Vec<PluginEvent> {
        match cmd {
            PluginCommand::Initialize {
                max_block_size: _,
                sample_rate,
            } => {
                self.sample_rate = sample_rate;
                vec![PluginEvent::PluginInitialized { sample_rate }]
            }

            PluginCommand::Reset => {
                for param in &mut self.parameters {
                    param.reset();
                }
                vec![PluginEvent::PluginReset]
            }

            PluginCommand::SetParameter { id, value } => {
                if let Some(param) = self.parameters.iter_mut().find(|p| p.id() == id) {
                    let before = param.current_value();
                    param.set_value(value);
                    let after = param.current_value();
                    // Only emit an event when the clamped value actually changed.
                    if (after - before).abs() > f64::EPSILON {
                        return vec![PluginEvent::ParameterChanged { id, value: after }];
                    }
                }
                vec![]
            }
        }
    }

    // ── MIDI normalization ────────────────────────────────────────────────────

    /// Normalize a raw MIDI byte message from the host into a kernel
    /// [`crate::kernel::midi_event::MidiEvent`].
    ///
    /// Returns `None` for messages that cannot be mapped (SysEx, active-sense,
    /// truncated messages, etc.).  Uses the same normalizer as the standalone app.
    pub fn normalize_midi(
        &self,
        raw: &RawMidiMessage,
    ) -> Option<crate::kernel::midi_event::MidiEvent> {
        self.normalizer.normalize(raw)
    }

    // ── State persistence ─────────────────────────────────────────────────────

    /// Serialize a [`Preset`] to bytes using the shared [`PresetCodec`].
    ///
    /// The bytes can be stored by the host and restored via
    /// [`PluginInstance::restore_preset`].  The format is identical to the
    /// standalone app's preset format.
    pub fn save_preset(&self, preset: Preset) -> Vec<u8> {
        self.codec.serialize(preset)
    }

    /// Deserialize a [`Preset`] from bytes using the shared [`PresetCodec`].
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] on parse failure.
    pub fn restore_preset(&self, bytes: Vec<u8>) -> Result<Preset, CodecError> {
        self.codec.deserialize(bytes)
    }

    /// Serialize a [`Setup`] to bytes.
    pub fn save_setup(&self, setup: Setup) -> Vec<u8> {
        self.codec.serialize_setup(setup)
    }

    /// Deserialize a [`Setup`] from bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] on parse failure.
    pub fn restore_setup(&self, bytes: Vec<u8>) -> Result<Setup, CodecError> {
        self.codec.deserialize_setup(bytes)
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Find a parameter by its stable [`ParameterId`].
    pub fn parameter(&self, id: ParameterId) -> Option<&PluginParameter> {
        self.parameters.iter().find(|p| p.id() == id)
    }

    /// Return the current value of a parameter by its stable ID.
    pub fn parameter_value(&self, id: ParameterId) -> Option<f64> {
        self.parameter(id).map(|p| p.current_value())
    }
}

// ── Well-known parameter IDs ──────────────────────────────────────────────────

/// Stable raw `u32` parameter ID constants.
///
/// These values **must not change** across plugin versions; changing them
/// breaks saved DAW automation.  Construct a [`ParameterId`] from these with
/// `ParameterId::new(param_id::FILTER_CUTOFF_RAW)`.
pub mod param_id {
    use crate::plugin::parameter_id::ParameterId;

    pub const OSCILLATOR_DETUNE_RAW: u32 = 0;
    pub const OSCILLATOR_PULSE_WIDTH_RAW: u32 = 1;
    pub const FILTER_CUTOFF_RAW: u32 = 2;
    pub const FILTER_RESONANCE_RAW: u32 = 3;
    pub const AMP_ATTACK_RAW: u32 = 4;
    pub const AMP_DECAY_RAW: u32 = 5;
    pub const AMP_SUSTAIN_RAW: u32 = 6;
    pub const AMP_RELEASE_RAW: u32 = 7;
    pub const MASTER_VOLUME_RAW: u32 = 8;

    #[inline]
    pub fn oscillator_detune() -> ParameterId {
        ParameterId::new(OSCILLATOR_DETUNE_RAW)
    }
    #[inline]
    pub fn oscillator_pulse_width() -> ParameterId {
        ParameterId::new(OSCILLATOR_PULSE_WIDTH_RAW)
    }
    #[inline]
    pub fn filter_cutoff() -> ParameterId {
        ParameterId::new(FILTER_CUTOFF_RAW)
    }
    #[inline]
    pub fn filter_resonance() -> ParameterId {
        ParameterId::new(FILTER_RESONANCE_RAW)
    }
    #[inline]
    pub fn amp_attack() -> ParameterId {
        ParameterId::new(AMP_ATTACK_RAW)
    }
    #[inline]
    pub fn amp_decay() -> ParameterId {
        ParameterId::new(AMP_DECAY_RAW)
    }
    #[inline]
    pub fn amp_sustain() -> ParameterId {
        ParameterId::new(AMP_SUSTAIN_RAW)
    }
    #[inline]
    pub fn amp_release() -> ParameterId {
        ParameterId::new(AMP_RELEASE_RAW)
    }
    #[inline]
    pub fn master_volume() -> ParameterId {
        ParameterId::new(MASTER_VOLUME_RAW)
    }
}

/// Build the canonical parameter list that maps 1:1 to engine parameters.
///
/// Parameter IDs are stable across plugin versions.
pub fn default_parameter_list() -> Vec<PluginParameter> {
    vec![
        PluginParameter::new(
            param_id::oscillator_detune(),
            "Osc Detune".to_string(),
            "oscillator.detune".to_string(),
            ParameterRange::try_new(-100.0, 100.0, 0.0, None).unwrap(),
        ),
        PluginParameter::new(
            param_id::oscillator_pulse_width(),
            "Osc Pulse Width".to_string(),
            "oscillator.pulse_width".to_string(),
            ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap(),
        ),
        PluginParameter::new(
            param_id::filter_cutoff(),
            "Filter Cutoff".to_string(),
            "filter.cutoff_hz".to_string(),
            ParameterRange::try_new(20.0, 20_000.0, 20_000.0, None).unwrap(),
        ),
        PluginParameter::new(
            param_id::filter_resonance(),
            "Filter Resonance".to_string(),
            "filter.resonance".to_string(),
            ParameterRange::try_new(0.0, 1.0, 0.0, None).unwrap(),
        ),
        PluginParameter::new(
            param_id::amp_attack(),
            "Amp Attack".to_string(),
            "amp_envelope.attack".to_string(),
            ParameterRange::try_new(0.001, 10.0, 0.01, None).unwrap(),
        ),
        PluginParameter::new(
            param_id::amp_decay(),
            "Amp Decay".to_string(),
            "amp_envelope.decay".to_string(),
            ParameterRange::try_new(0.001, 10.0, 0.1, None).unwrap(),
        ),
        PluginParameter::new(
            param_id::amp_sustain(),
            "Amp Sustain".to_string(),
            "amp_envelope.sustain".to_string(),
            ParameterRange::try_new(0.0, 1.0, 0.8, None).unwrap(),
        ),
        PluginParameter::new(
            param_id::amp_release(),
            "Amp Release".to_string(),
            "amp_envelope.release".to_string(),
            ParameterRange::try_new(0.001, 10.0, 0.3, None).unwrap(),
        ),
        PluginParameter::new(
            param_id::master_volume(),
            "Master Volume".to_string(),
            "master.volume".to_string(),
            ParameterRange::try_new(0.0, 1.0, 1.0, None).unwrap(),
        ),
    ]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_event_kind::MidiEventKind;
    use crate::kernel::sample_rate::SampleRate;
    use crate::plugin::plugin_format::PluginFormat;
    use crate::presets::preset::Preset;
    use crate::presets::setup::Setup;
    use crate::shell::midi_normalizer::RawMidiMessage;

    fn sample_rate_44100() -> SampleRate {
        SampleRate::try_new(44100).unwrap()
    }

    fn make_instance() -> PluginInstance {
        PluginInstance::with_defaults(
            PluginFormat::Clap,
            default_parameter_list(),
            4,
            sample_rate_44100(),
        )
    }

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn format_field_matches_construction() {
        let inst =
            PluginInstance::with_defaults(PluginFormat::Vst3, vec![], 2, sample_rate_44100());
        assert_eq!(inst.format, PluginFormat::Vst3);
    }

    #[test]
    fn patch_count_stored_correctly() {
        let inst = make_instance();
        assert_eq!(inst.patch_count, 4);
    }

    #[test]
    fn sample_rate_stored_correctly() {
        let inst = make_instance();
        assert_eq!(inst.sample_rate, sample_rate_44100());
    }

    // ── Initialize command ────────────────────────────────────────────────────

    #[test]
    fn initialize_emits_plugin_initialized_event() {
        let mut inst = make_instance();
        let sr = SampleRate::try_new(48000).unwrap();
        let events = inst.handle(PluginCommand::Initialize {
            max_block_size: 512,
            sample_rate: sr,
        });
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            PluginEvent::PluginInitialized { sample_rate: sr }
        );
    }

    #[test]
    fn initialize_updates_sample_rate() {
        let mut inst = make_instance();
        let sr = SampleRate::try_new(96000).unwrap();
        inst.handle(PluginCommand::Initialize {
            max_block_size: 256,
            sample_rate: sr,
        });
        assert_eq!(inst.sample_rate, sr);
    }

    // ── Reset command ─────────────────────────────────────────────────────────

    #[test]
    fn reset_emits_plugin_reset_event() {
        let mut inst = make_instance();
        let events = inst.handle(PluginCommand::Reset);
        assert_eq!(events, vec![PluginEvent::PluginReset]);
    }

    #[test]
    fn reset_restores_all_parameters_to_defaults() {
        let mut inst = make_instance();
        // Dirty a parameter first.
        inst.handle(PluginCommand::SetParameter {
            id: param_id::filter_cutoff(),
            value: 1_000.0,
        });
        // Confirm it changed.
        let dirty = inst.parameter_value(param_id::filter_cutoff()).unwrap();
        assert!((dirty - 1_000.0).abs() < 1.0);

        inst.handle(PluginCommand::Reset);

        let default_val = inst
            .parameter(param_id::filter_cutoff())
            .unwrap()
            .range()
            .default_value;
        let actual = inst.parameter_value(param_id::filter_cutoff()).unwrap();
        assert!((actual - default_val).abs() < f64::EPSILON);
    }

    // ── SetParameter command ──────────────────────────────────────────────────

    #[test]
    fn set_parameter_emits_changed_event() {
        let mut inst = make_instance();
        let events = inst.handle(PluginCommand::SetParameter {
            id: param_id::filter_cutoff(),
            value: 5_000.0,
        });
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0], PluginEvent::ParameterChanged { id, value: _ } if *id == param_id::filter_cutoff())
        );
    }

    #[test]
    fn set_parameter_value_is_clamped() {
        let mut inst = make_instance();
        // Filter cutoff max is 20_000 Hz; try to set above.
        inst.handle(PluginCommand::SetParameter {
            id: param_id::filter_cutoff(),
            value: 999_999.0,
        });
        let val = inst.parameter_value(param_id::filter_cutoff()).unwrap();
        assert!(val <= 20_000.0);
    }

    #[test]
    fn set_parameter_unknown_id_returns_empty() {
        let mut inst = make_instance();
        let events = inst.handle(PluginCommand::SetParameter {
            id: ParameterId::new(9999),
            value: 1.0,
        });
        assert!(events.is_empty());
    }

    #[test]
    fn set_parameter_same_value_returns_empty() {
        let mut inst = make_instance();
        // Filter cutoff default is 20_000.0; setting it again should not emit.
        let events = inst.handle(PluginCommand::SetParameter {
            id: param_id::filter_cutoff(),
            value: 20_000.0,
        });
        assert!(events.is_empty());
    }

    // ── Parameters: 1:1 mapping invariant ────────────────────────────────────

    #[test]
    fn default_parameters_have_unique_ids() {
        let params = default_parameter_list();
        let mut ids: Vec<u32> = params.iter().map(|p| p.id().get()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), params.len(), "parameter IDs must be unique");
    }

    #[test]
    fn each_parameter_has_engine_mapping() {
        for param in default_parameter_list() {
            assert!(
                !param.engine_mapping().is_empty(),
                "parameter {} has empty engine mapping",
                param.id().get()
            );
        }
    }

    #[test]
    fn parameter_ids_are_stable_across_calls() {
        let first: Vec<u32> = default_parameter_list()
            .iter()
            .map(|p| p.id().get())
            .collect();
        let second: Vec<u32> = default_parameter_list()
            .iter()
            .map(|p| p.id().get())
            .collect();
        assert_eq!(first, second, "parameter IDs must be stable");
    }

    #[test]
    fn parameters_count_matches_engine_params() {
        let inst = make_instance();
        // Every parameter has a unique engine mapping — confirming 1:1 coverage.
        let mappings: std::collections::HashSet<&str> =
            inst.parameters.iter().map(|p| p.engine_mapping()).collect();
        assert_eq!(mappings.len(), inst.parameters.len());
    }

    // ── MIDI normalization (same normalizer as standalone) ────────────────────

    #[test]
    fn normalize_midi_note_on_returns_event() {
        let inst = make_instance();
        let raw = RawMidiMessage::new(vec![0x90, 60, 100], 0);
        let event = inst.normalize_midi(&raw);
        assert!(event.is_some());
        assert_eq!(event.unwrap().kind, MidiEventKind::NoteOn);
    }

    #[test]
    fn normalize_midi_note_off_returns_event() {
        let inst = make_instance();
        let raw = RawMidiMessage::new(vec![0x80, 60, 64], 0);
        let event = inst.normalize_midi(&raw);
        assert!(event.is_some());
        assert_eq!(event.unwrap().kind, MidiEventKind::NoteOff);
    }

    #[test]
    fn normalize_midi_sysex_returns_none() {
        let inst = make_instance();
        let raw = RawMidiMessage::new(vec![0xF0, 0x7E, 0xF7], 0);
        assert!(inst.normalize_midi(&raw).is_none());
    }

    #[test]
    fn normalize_midi_empty_returns_none() {
        let inst = make_instance();
        let raw = RawMidiMessage::new(vec![], 0);
        assert!(inst.normalize_midi(&raw).is_none());
    }

    // ── State persistence (same PresetCodec as standalone) ───────────────────

    #[test]
    fn save_and_restore_preset_round_trip() {
        let inst = make_instance();
        let preset = Preset::default_for("test-preset", "Test Preset");
        let bytes = inst.save_preset(preset.clone());
        let restored = inst.restore_preset(bytes).unwrap();
        assert_eq!(preset, restored);
    }

    #[test]
    fn save_and_restore_setup_round_trip() {
        let inst = make_instance();
        let setup = Setup::new("My Plugin Setup");
        let bytes = inst.save_setup(setup);
        let restored = inst.restore_setup(bytes).unwrap();
        assert_eq!(restored.name, "My Plugin Setup");
    }

    #[test]
    fn restore_preset_invalid_bytes_returns_error() {
        let inst = make_instance();
        let result = inst.restore_preset(b"not-json".to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn restore_setup_invalid_bytes_returns_error() {
        let inst = make_instance();
        let result = inst.restore_setup(b"not-json".to_vec());
        assert!(result.is_err());
    }

    // ── parameter() / parameter_value() queries ───────────────────────────────

    #[test]
    fn parameter_lookup_by_id() {
        let inst = make_instance();
        let param = inst.parameter(param_id::filter_cutoff()).unwrap();
        assert_eq!(param.id(), param_id::filter_cutoff());
    }

    #[test]
    fn parameter_lookup_unknown_id_returns_none() {
        let inst = make_instance();
        assert!(inst.parameter(ParameterId::new(9999)).is_none());
    }

    #[test]
    fn parameter_value_returns_current_value() {
        let mut inst = make_instance();
        inst.handle(PluginCommand::SetParameter {
            id: param_id::filter_resonance(),
            value: 0.75,
        });
        let v = inst.parameter_value(param_id::filter_resonance()).unwrap();
        assert!((v - 0.75).abs() < f64::EPSILON);
    }
}
