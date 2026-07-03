// path: src/plugin/plugin_instance.rs

//! `PluginInstance` — the plugin aggregate root.
//!
//! Wraps the host-agnostic engine library as a plugin: it owns the mapping
//! between the plugin's exposed parameters and the engine's own parameters,
//! persists its state through the same `PresetCodec` the standalone app
//! uses (so presets are interchangeable between the two shells), and routes
//! MIDI arriving from the host through the same `MidiNormalizer` the
//! standalone's MIDI input adapter uses, so both shells produce identical
//! `MidiEvent`s from identical bytes.
//!
//! Like the engine library itself, this aggregate has no audio driver,
//! window, or controller code — it is pure plugin-shell bookkeeping
//! (parameters, format, sample rate) plus the command/event reducer a host
//! wrapper (VST3/CLAP/AU) drives.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::kernel::midi_event::MidiEvent;
use crate::kernel::sample_rate::SampleRate;
use crate::preset::preset_codec::{CodecError, Preset, PresetCodec};
use crate::shell::midi_normalizer::{MidiNormalizer, MidiNormalizerError};

/// Stable numeric identifier for a plugin parameter.
///
/// Parameter IDs must remain stable across plugin versions: a host records
/// automation lanes keyed by this ID, and changing it silently orphans a
/// user's saved automation in every DAW project that referenced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ParameterId(u32);

impl ParameterId {
    /// Constructs a `ParameterId` from its underlying stable numeric value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the underlying stable numeric value.
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for ParameterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for ParameterId {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

/// The inclusive value range a `PluginParameter` may take.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParameterRange {
    min: f64,
    max: f64,
}

/// Errors constructing a [`ParameterRange`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterRangeError {
    /// `min` and/or `max` were NaN, which cannot bound anything.
    NotFinite { min: f64, max: f64 },
    /// `min` was greater than `max`.
    MinExceedsMax { min: f64, max: f64 },
}

impl fmt::Display for ParameterRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterRangeError::NotFinite { min, max } => {
                write!(
                    f,
                    "parameter range bounds must be finite, got [{min}, {max}]"
                )
            }
            ParameterRangeError::MinExceedsMax { min, max } => {
                write!(f, "parameter range min {min} exceeds max {max}")
            }
        }
    }
}

impl Error for ParameterRangeError {}

impl ParameterRange {
    /// Constructs a `ParameterRange`, rejecting non-finite bounds or a `min`
    /// greater than `max`.
    pub fn try_new(min: f64, max: f64) -> Result<Self, ParameterRangeError> {
        if !min.is_finite() || !max.is_finite() {
            return Err(ParameterRangeError::NotFinite { min, max });
        }
        if min > max {
            return Err(ParameterRangeError::MinExceedsMax { min, max });
        }
        Ok(Self { min, max })
    }

    pub fn min(&self) -> f64 {
        self.min
    }

    pub fn max(&self) -> f64 {
        self.max
    }

    /// Whether `value` falls within this range (inclusive).
    pub fn contains(&self, value: f64) -> bool {
        !value.is_nan() && (self.min..=self.max).contains(&value)
    }

    /// Clamps `value` into this range.
    pub fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }
}

/// The plugin format(s) `PluginInstance` can be hosted as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginFormat {
    Vst3,
    Clap,
    AudioUnit,
}

/// Errors constructing a [`PluginParameter`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PluginParameterError {
    /// The requested initial value was NaN, which no range can contain.
    NonFiniteInitialValue(f64),
}

impl fmt::Display for PluginParameterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginParameterError::NonFiniteInitialValue(value) => {
                write!(f, "parameter initial value must be finite, got {value}")
            }
        }
    }
}

impl Error for PluginParameterError {}

/// One parameter exposed by the plugin, mapped 1:1 to an engine parameter.
///
/// `engine_mapping` names the engine-side parameter this plugin parameter
/// drives; `id` is the stable numeric ID the host uses for automation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginParameter {
    id: ParameterId,
    name: String,
    engine_mapping: String,
    range: ParameterRange,
    current_value: f64,
}

impl PluginParameter {
    /// Constructs a `PluginParameter`. `initial_value` is clamped into
    /// `range` rather than rejected, since hosts commonly restore automation
    /// from a slightly different range than the current build declares;
    /// only a non-finite value is rejected outright.
    pub fn try_new(
        id: ParameterId,
        name: impl Into<String>,
        engine_mapping: impl Into<String>,
        range: ParameterRange,
        initial_value: f64,
    ) -> Result<Self, PluginParameterError> {
        if !initial_value.is_finite() {
            return Err(PluginParameterError::NonFiniteInitialValue(initial_value));
        }
        Ok(Self {
            id,
            name: name.into(),
            engine_mapping: engine_mapping.into(),
            current_value: range.clamp(initial_value),
            range,
        })
    }

    pub fn id(&self) -> ParameterId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn engine_mapping(&self) -> &str {
        &self.engine_mapping
    }

    pub fn range(&self) -> ParameterRange {
        self.range
    }

    pub fn current_value(&self) -> f64 {
        self.current_value
    }

    /// Sets the current value, clamping it into `range`. Returns the value
    /// actually applied (post-clamp) so the caller can build an accurate
    /// `ParameterChanged` event payload.
    fn set_value(&mut self, value: f64) -> f64 {
        let clamped = self.range.clamp(value);
        self.current_value = clamped;
        clamped
    }
}

/// Commands `PluginInstance` accepts from its host wrapper.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginCommand {
    Initialize {
        max_block_size: u32,
        sample_rate: SampleRate,
    },
    Reset,
    SetParameter {
        id: ParameterId,
        value: f64,
    },
}

/// Events `PluginInstance` emits in response to a `PluginCommand`.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginEvent {
    PluginInitialized { sample_rate: SampleRate },
    PluginReset,
    ParameterChanged { id: ParameterId, value: f64 },
}

/// Errors applying a `PluginCommand` or persisting/restoring state.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginInstanceError {
    /// `SetParameter` referenced an ID with no matching parameter.
    UnknownParameter(ParameterId),
    /// Two parameters were constructed with the same stable ID.
    DuplicateParameterId(ParameterId),
    /// `Initialize` was called with a zero block size.
    InvalidMaxBlockSize,
    /// Encoding/decoding state through a `PresetCodec` failed.
    Codec(CodecError),
}

impl fmt::Display for PluginInstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginInstanceError::UnknownParameter(id) => {
                write!(f, "no plugin parameter with id {id}")
            }
            PluginInstanceError::DuplicateParameterId(id) => {
                write!(f, "duplicate plugin parameter id {id}")
            }
            PluginInstanceError::InvalidMaxBlockSize => {
                write!(f, "max block size must be non-zero")
            }
            PluginInstanceError::Codec(err) => write!(f, "{err}"),
        }
    }
}

impl Error for PluginInstanceError {}

impl From<CodecError> for PluginInstanceError {
    fn from(err: CodecError) -> Self {
        PluginInstanceError::Codec(err)
    }
}

/// The name stamped on the `Preset` a `PluginInstance` saves through the
/// shared `PresetCodec`. Kept constant so a standalone-authored preset and
/// a plugin-authored one round-trip through the identical codec contract.
const PLUGIN_STATE_PRESET_NAME: &str = "plugin-instance-state";

/// The plugin aggregate root: wraps the engine library as a plugin.
///
/// `midi_normalizer` is injected rather than constructed internally so
/// tests can observe/replace it and so the instance never silently drifts
/// from the standalone's normalization behavior.
pub struct PluginInstance {
    format: PluginFormat,
    parameters: Vec<PluginParameter>,
    patch_count: u8,
    sample_rate: SampleRate,
    midi_normalizer: MidiNormalizer,
}

impl PluginInstance {
    /// Full constructor: accepts every dependency and piece of state.
    /// Rejects duplicate parameter IDs, since the host automation contract
    /// requires each ID to identify exactly one parameter.
    pub fn try_new(
        format: PluginFormat,
        parameters: Vec<PluginParameter>,
        patch_count: u8,
        sample_rate: SampleRate,
        midi_normalizer: MidiNormalizer,
    ) -> Result<Self, PluginInstanceError> {
        let mut seen = std::collections::HashSet::with_capacity(parameters.len());
        for parameter in &parameters {
            if !seen.insert(parameter.id()) {
                return Err(PluginInstanceError::DuplicateParameterId(parameter.id()));
            }
        }

        Ok(Self {
            format,
            parameters,
            patch_count,
            sample_rate,
            midi_normalizer,
        })
    }

    /// Convenience constructor for callers that don't need to inject a
    /// specific `MidiNormalizer` (e.g. production host-wrapper startup).
    pub fn new(
        format: PluginFormat,
        parameters: Vec<PluginParameter>,
        patch_count: u8,
        sample_rate: SampleRate,
    ) -> Result<Self, PluginInstanceError> {
        Self::try_new(
            format,
            parameters,
            patch_count,
            sample_rate,
            MidiNormalizer::new(),
        )
    }

    pub fn format(&self) -> PluginFormat {
        self.format
    }

    pub fn parameters(&self) -> &[PluginParameter] {
        &self.parameters
    }

    pub fn patch_count(&self) -> u8 {
        self.patch_count
    }

    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// The command→event reducer: applies `command`, mutating state, and
    /// returns the event that occurred.
    pub fn apply(&mut self, command: PluginCommand) -> Result<PluginEvent, PluginInstanceError> {
        match command {
            PluginCommand::Initialize {
                max_block_size,
                sample_rate,
            } => {
                if max_block_size == 0 {
                    return Err(PluginInstanceError::InvalidMaxBlockSize);
                }
                self.sample_rate = sample_rate;
                Ok(PluginEvent::PluginInitialized { sample_rate })
            }
            PluginCommand::Reset => Ok(PluginEvent::PluginReset),
            PluginCommand::SetParameter { id, value } => {
                let parameter = self
                    .parameters
                    .iter_mut()
                    .find(|parameter| parameter.id() == id)
                    .ok_or(PluginInstanceError::UnknownParameter(id))?;
                let applied = parameter.set_value(value);
                Ok(PluginEvent::ParameterChanged { id, value: applied })
            }
        }
    }

    /// Normalizes a raw MIDI 1.0 message from the host through the same
    /// `MidiNormalizer` the standalone's MIDI input adapter uses, so both
    /// shells produce identical `MidiEvent`s from identical bytes.
    pub fn normalize_host_midi(&mut self, bytes: &[u8]) -> Result<MidiEvent, MidiNormalizerError> {
        self.midi_normalizer.normalize(bytes)
    }

    /// Serializes this instance's state through `codec` — the same
    /// `PresetCodec` the standalone app uses — so plugin- and
    /// standalone-authored state are interchangeable.
    pub fn save_state(&self, codec: &dyn PresetCodec) -> Result<Vec<u8>, PluginInstanceError> {
        let snapshot = PluginInstanceSnapshot::from(self);
        let payload = serde_json::to_vec(&snapshot)
            .map_err(|err| PluginInstanceError::Codec(CodecError::Malformed(err.to_string())))?;
        let preset = Preset::new(PLUGIN_STATE_PRESET_NAME, payload);
        Ok(codec.encode(preset)?)
    }

    /// Reconstructs a `PluginInstance` from bytes previously produced by
    /// [`save_state`](Self::save_state), through the same `codec`.
    pub fn load_state(
        codec: &dyn PresetCodec,
        data: &[u8],
        midi_normalizer: MidiNormalizer,
    ) -> Result<Self, PluginInstanceError> {
        let preset = codec.decode(data)?;
        let snapshot: PluginInstanceSnapshot = serde_json::from_slice(preset.patch_payload())
            .map_err(|err| PluginInstanceError::Codec(CodecError::Malformed(err.to_string())))?;
        snapshot.into_plugin_instance(midi_normalizer)
    }

    /// Compares state relevant to persistence (format, parameters, patch
    /// count, sample rate) while ignoring the `MidiNormalizer`, which
    /// carries no persisted state and has no equality of its own. Used to
    /// prove a save/load round trip reconstructs an equivalent instance.
    pub fn has_equivalent_state(&self, other: &Self) -> bool {
        self.format == other.format
            && self.parameters == other.parameters
            && self.patch_count == other.patch_count
            && self.sample_rate == other.sample_rate
    }
}

/// The on-wire shape of `PluginInstance` state, serialized as the
/// `patch_payload` of a `PresetCodec::Preset`. Kept separate from the
/// domain type so the physical layout can evolve independently of the
/// aggregate's in-memory shape.
#[derive(Debug, Serialize, Deserialize)]
struct PluginInstanceSnapshot {
    format: PluginFormat,
    parameters: Vec<PluginParameter>,
    patch_count: u8,
    sample_rate: u32,
}

impl From<&PluginInstance> for PluginInstanceSnapshot {
    fn from(instance: &PluginInstance) -> Self {
        Self {
            format: instance.format,
            parameters: instance.parameters.clone(),
            patch_count: instance.patch_count,
            sample_rate: instance.sample_rate.get(),
        }
    }
}

impl PluginInstanceSnapshot {
    fn into_plugin_instance(
        self,
        midi_normalizer: MidiNormalizer,
    ) -> Result<PluginInstance, PluginInstanceError> {
        let sample_rate = SampleRate::try_new(self.sample_rate)
            .map_err(|err| PluginInstanceError::Codec(CodecError::Malformed(err.to_string())))?;

        PluginInstance::try_new(
            self.format,
            self.parameters,
            self.patch_count,
            sample_rate,
            midi_normalizer,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::preset_codec::{migrate_preset, RawPreset, CURRENT_PRESET_FORMAT_VERSION};

    /// A dependency-free JSON `PresetCodec` used only to exercise the save
    /// state / load state round trip in this module's tests, without
    /// reaching for another resource's adapter. Routes through the shared
    /// `migrate_preset` policy exactly as a production adapter must.
    #[derive(Debug, Serialize, Deserialize)]
    struct TestPresetDocument {
        format_version: u32,
        name: String,
        patch_payload: Vec<u8>,
    }

    struct TestJsonPresetCodec;

    impl PresetCodec for TestJsonPresetCodec {
        fn decode(&self, data: &[u8]) -> Result<Preset, CodecError> {
            let document: TestPresetDocument = serde_json::from_slice(data)
                .map_err(|err| CodecError::Malformed(err.to_string()))?;
            migrate_preset(RawPreset {
                format_version: document.format_version,
                name: document.name,
                patch_payload: document.patch_payload,
            })
        }

        fn encode(&self, preset: Preset) -> Result<Vec<u8>, CodecError> {
            let document = TestPresetDocument {
                format_version: CURRENT_PRESET_FORMAT_VERSION,
                name: preset.name().to_string(),
                patch_payload: preset.patch_payload().to_vec(),
            };
            serde_json::to_vec(&document).map_err(|err| CodecError::Malformed(err.to_string()))
        }
    }

    fn sample_parameter() -> PluginParameter {
        PluginParameter::try_new(
            ParameterId::new(1),
            "Cutoff",
            "engine.filter.cutoff",
            ParameterRange::try_new(0.0, 1.0).expect("valid range"),
            0.5,
        )
        .expect("valid parameter")
    }

    fn sample_instance() -> PluginInstance {
        PluginInstance::try_new(
            PluginFormat::Vst3,
            vec![sample_parameter()],
            1,
            SampleRate::try_new(48_000).expect("valid sample rate"),
            MidiNormalizer::new(),
        )
        .expect("valid instance")
    }

    #[test]
    fn initialize_yields_plugin_initialized_with_matching_sample_rate() {
        let mut instance = sample_instance();
        let sample_rate = SampleRate::try_new(44_100).expect("valid sample rate");

        let event = instance
            .apply(PluginCommand::Initialize {
                max_block_size: 512,
                sample_rate,
            })
            .expect("initialize succeeds");

        assert_eq!(event, PluginEvent::PluginInitialized { sample_rate });
        assert_eq!(instance.sample_rate(), sample_rate);
    }

    #[test]
    fn initialize_rejects_a_zero_max_block_size() {
        let mut instance = sample_instance();
        let sample_rate = SampleRate::try_new(44_100).expect("valid sample rate");

        let result = instance.apply(PluginCommand::Initialize {
            max_block_size: 0,
            sample_rate,
        });

        assert_eq!(result, Err(PluginInstanceError::InvalidMaxBlockSize));
    }

    #[test]
    fn reset_yields_plugin_reset() {
        let mut instance = sample_instance();

        let event = instance
            .apply(PluginCommand::Reset)
            .expect("reset succeeds");

        assert_eq!(event, PluginEvent::PluginReset);
    }

    #[test]
    fn set_parameter_yields_parameter_changed_with_matching_payload() {
        let mut instance = sample_instance();
        let id = ParameterId::new(1);

        let event = instance
            .apply(PluginCommand::SetParameter { id, value: 0.75 })
            .expect("set parameter succeeds");

        assert_eq!(event, PluginEvent::ParameterChanged { id, value: 0.75 });
        assert_eq!(instance.parameters()[0].current_value(), 0.75);
    }

    #[test]
    fn set_parameter_clamps_an_out_of_range_value() {
        let mut instance = sample_instance();
        let id = ParameterId::new(1);

        let event = instance
            .apply(PluginCommand::SetParameter { id, value: 5.0 })
            .expect("set parameter succeeds");

        assert_eq!(event, PluginEvent::ParameterChanged { id, value: 1.0 });
    }

    #[test]
    fn set_parameter_rejects_an_unknown_id() {
        let mut instance = sample_instance();
        let unknown = ParameterId::new(999);

        let result = instance.apply(PluginCommand::SetParameter {
            id: unknown,
            value: 0.1,
        });

        assert_eq!(result, Err(PluginInstanceError::UnknownParameter(unknown)));
    }

    #[test]
    fn constructing_with_duplicate_parameter_ids_is_rejected() {
        let duplicate = sample_parameter();
        let result = PluginInstance::try_new(
            PluginFormat::Vst3,
            vec![sample_parameter(), duplicate],
            1,
            SampleRate::try_new(48_000).expect("valid sample rate"),
            MidiNormalizer::new(),
        );

        match result {
            Err(PluginInstanceError::DuplicateParameterId(id)) => {
                assert_eq!(id, ParameterId::new(1));
            }
            _ => panic!("expected DuplicateParameterId error"),
        }
    }

    #[test]
    fn state_round_trips_through_the_preset_codec() {
        let codec = TestJsonPresetCodec;
        let mut instance = sample_instance();
        instance
            .apply(PluginCommand::SetParameter {
                id: ParameterId::new(1),
                value: 0.9,
            })
            .expect("set parameter succeeds");

        let bytes = instance.save_state(&codec).expect("save succeeds");
        let restored = PluginInstance::load_state(&codec, &bytes, MidiNormalizer::new())
            .expect("load succeeds");

        assert!(instance.has_equivalent_state(&restored));
        assert_eq!(restored.parameters()[0].current_value(), 0.9);
    }

    #[test]
    fn normalize_host_midi_delegates_to_the_injected_normalizer() {
        let mut instance = sample_instance();

        let event = instance
            .normalize_host_midi(&[0x90, 60, 100])
            .expect("normalizes a valid note-on");

        assert_eq!(event.note().value(), 60);
    }
}
