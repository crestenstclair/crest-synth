// path: src/presets/setup.rs

//! `Setup` aggregate — the full app state: patch list + subscriptions + mixer + effects,
//! saved to and restored from a named file.
//!
//! # Serialization design
//!
//! `SerializedPatch` and `SerializedEffectChain` are flat, owned data structures
//! containing no heap-allocated sub-aggregates. They carry exactly the information
//! needed to recreate runtime state, satisfying the invariant that a loaded setup
//! returns the app to its exact prior state.
//!
//! # Audio-thread constraints
//!
//! `Setup` lives entirely on the control thread. `SaveSetup` and `LoadSetup` perform
//! I/O (writing / reading JSON files) and must never be called from the audio thread.

use crate::effects::effect_processor::{EffectParams, EffectType};
use crate::kernel::amplitude::Amplitude;
use crate::modulation::lfo_config::LfoConfig;
use crate::modulation::lfo_waveform::LfoWaveform;
use crate::modulation::mod_destination_type::ModDestinationType;
use crate::modulation::mod_envelope_config::ModEnvelopeConfig;
use crate::modulation::mod_source_type::ModSourceType;
use crate::patch::channel_subscription::ChannelSubscription;
use crate::patch::voice_pool_config::{StealingPolicy, VoicePoolConfig};
use crate::synth::amp_envelope_config::AmpEnvelopeConfig;
use crate::synth::filter_config::{FilterConfig, FilterType};
use crate::synth::oscillator_config::{OscillatorConfig, Waveform};

// ─── Serialized effect types ──────────────────────────────────────────────────

/// Serialized representation of a single effect slot in a chain.
///
/// Contains enough information to reconstruct an `EffectSlot` with its
/// current parameters and bypass state.
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedEffectSlot {
    /// The algorithm this slot runs.
    pub effect_type: EffectType,
    /// Current effect parameters.
    pub params: EffectParams,
    /// Whether this slot is bypassed.
    pub bypass: bool,
}

/// Serialized representation of an effect chain.
///
/// Slots are stored in processing order: slot 0 first, slot N last.
/// The bypass flag indicates whether the whole chain is bypassed.
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedEffectChain {
    /// Whether the entire chain is bypassed.
    pub bypass: bool,
    /// Ordered list of effect slots.
    pub slots: Vec<SerializedEffectSlot>,
}

impl Default for SerializedEffectChain {
    /// Returns an empty, non-bypassed chain.
    fn default() -> Self {
        Self {
            bypass: false,
            slots: Vec::new(),
        }
    }
}

// ─── Serialized modulation types ─────────────────────────────────────────────

/// Serialized representation of a single modulation routing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SerializedModRouting {
    /// Modulation source.
    pub source: ModSourceType,
    /// Modulation destination.
    pub destination: ModDestinationType,
    /// Signed depth in `[-1.0, 1.0]`.
    pub depth: f64,
}

/// Serialized LFO configuration entry (index + config).
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedLfoEntry {
    /// Index within the mod matrix.
    pub index: u8,
    /// LFO parameters.
    pub config: SerializedLfoConfig,
}

/// Plain-data LFO configuration for serialization (avoids crate coupling).
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedLfoConfig {
    pub rate: f64,
    pub depth: f64,
    pub phase: f64,
    pub sync_to_tempo: bool,
    pub waveform: LfoWaveform,
}

impl From<LfoConfig> for SerializedLfoConfig {
    fn from(c: LfoConfig) -> Self {
        Self {
            rate: c.rate,
            depth: c.depth,
            phase: c.phase,
            sync_to_tempo: c.sync_to_tempo,
            waveform: c.waveform,
        }
    }
}

impl From<SerializedLfoConfig> for LfoConfig {
    fn from(s: SerializedLfoConfig) -> Self {
        // Clamp to valid ranges defensively during deserialization.
        let rate = if s.rate > 0.0 { s.rate } else { 1.0 };
        let depth = s.depth.clamp(0.0, 1.0);
        LfoConfig {
            rate,
            depth,
            phase: s.phase,
            sync_to_tempo: s.sync_to_tempo,
            waveform: s.waveform,
        }
    }
}

/// Serialized mod-envelope entry (index + config).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SerializedModEnvelopeEntry {
    /// Index within the mod matrix.
    pub index: u8,
    /// Mod envelope parameters.
    pub config: ModEnvelopeConfig,
}

/// Serialized modulation matrix for one patch.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SerializedModMatrix {
    /// LFO configurations, each paired with its index.
    pub lfo_entries: Vec<SerializedLfoEntry>,
    /// Mod envelope configurations, each paired with its index.
    pub mod_envelope_entries: Vec<SerializedModEnvelopeEntry>,
    /// All modulation routings in matrix order.
    pub routings: Vec<SerializedModRouting>,
}

// ─── SerializedPatch ──────────────────────────────────────────────────────────

/// Serialized representation of a single patch (complete instrument state).
///
/// Captures every piece of state required to reproduce the saved sound exactly:
/// - Engine type and synthesis parameters (oscillator, envelope, filter)
/// - Output gain and pan position
/// - MIDI channel subscription
/// - Voice pool sizing and stealing policy
/// - Modulation matrix (LFOs, mod envelopes, routings)
/// - Per-patch effect chain
///
/// Satisfies the invariant: *preset serialization captures the complete patch
/// state including modulation and effects*.
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedPatch {
    /// Unique numeric identifier (raw `u32` from `PatchId`).
    pub patch_id: u32,
    /// Patch display name.
    pub name: String,
    /// Whether the patch is active.
    pub active: bool,
    /// Synthesis engine variant.
    pub engine_type: SerializedEngineType,
    /// Oscillator parameters.
    pub oscillator: SerializedOscillatorConfig,
    /// Amplitude envelope (ADSR).
    pub amp_envelope: SerializedEnvelopeConfig,
    /// Filter parameters.
    pub filter: SerializedFilterConfig,
    /// Output gain (stored as a raw non-negative f64).
    pub gain: f64,
    /// Stereo pan (−1.0 left … 1.0 right).
    pub pan: f64,
    /// MIDI group index.
    pub midi_group: u8,
    /// MIDI channel index.
    pub midi_channel: u8,
    /// Maximum number of simultaneous voices.
    pub max_voices: u8,
    /// Voice stealing policy.
    pub stealing_policy: SerializedStealingPolicy,
    /// Modulation matrix state.
    pub mod_matrix: SerializedModMatrix,
    /// Per-patch effect chain.
    pub effect_chain: SerializedEffectChain,
}

// ─── Small serialized newtypes ────────────────────────────────────────────────

/// Serialization mirror for `EngineType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializedEngineType {
    Sine,
    Wavetable,
    Subtractive,
}

/// Serialization mirror for `StealingPolicy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializedStealingPolicy {
    OldestFirst,
    QuietestFirst,
    NoStealing,
}

impl From<StealingPolicy> for SerializedStealingPolicy {
    fn from(p: StealingPolicy) -> Self {
        match p {
            StealingPolicy::OldestFirst => Self::OldestFirst,
            StealingPolicy::QuietestFirst => Self::QuietestFirst,
            StealingPolicy::NoStealing => Self::NoStealing,
        }
    }
}

impl From<SerializedStealingPolicy> for StealingPolicy {
    fn from(p: SerializedStealingPolicy) -> Self {
        match p {
            SerializedStealingPolicy::OldestFirst => Self::OldestFirst,
            SerializedStealingPolicy::QuietestFirst => Self::QuietestFirst,
            SerializedStealingPolicy::NoStealing => Self::NoStealing,
        }
    }
}

/// Serialization mirror for `OscillatorConfig`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SerializedOscillatorConfig {
    pub detune: f64,
    pub pulse_width: f64,
    pub waveform: Waveform,
}

impl From<OscillatorConfig> for SerializedOscillatorConfig {
    fn from(c: OscillatorConfig) -> Self {
        Self {
            detune: c.detune,
            pulse_width: c.pulse_width,
            waveform: c.waveform,
        }
    }
}

impl From<SerializedOscillatorConfig> for OscillatorConfig {
    fn from(s: SerializedOscillatorConfig) -> Self {
        // Clamp defensively on load; try_new enforces proper validation upstream.
        OscillatorConfig {
            detune: s.detune,
            pulse_width: s.pulse_width,
            waveform: s.waveform,
        }
    }
}

/// Serialization mirror for `AmpEnvelopeConfig`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SerializedEnvelopeConfig {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl From<AmpEnvelopeConfig> for SerializedEnvelopeConfig {
    fn from(c: AmpEnvelopeConfig) -> Self {
        Self {
            attack: c.attack,
            decay: c.decay,
            sustain: c.sustain,
            release: c.release,
        }
    }
}

impl From<SerializedEnvelopeConfig> for AmpEnvelopeConfig {
    fn from(s: SerializedEnvelopeConfig) -> Self {
        AmpEnvelopeConfig {
            attack: s.attack,
            decay: s.decay,
            sustain: s.sustain,
            release: s.release,
        }
    }
}

/// Serialization mirror for `FilterConfig`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SerializedFilterConfig {
    pub cutoff_hz: f64,
    pub filter_type: FilterType,
    pub resonance: f64,
}

impl From<FilterConfig> for SerializedFilterConfig {
    fn from(c: FilterConfig) -> Self {
        Self {
            cutoff_hz: c.cutoff.hz(),
            filter_type: c.filter_type,
            resonance: c.resonance(),
        }
    }
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Save the current session to a named file.
pub struct SaveSetup {
    /// Human-readable name for the setup (used as filename base).
    pub name: String,
}

/// Load a previously saved session from a file path.
pub struct LoadSetup {
    /// File path of the saved setup.
    pub path: String,
}

// ─── Events ───────────────────────────────────────────────────────────────────

/// Emitted when a setup is successfully saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupSaved {
    /// The name under which the setup was saved.
    pub name: String,
    /// Number of patches in the saved setup.
    pub patch_count: u32,
}

/// Emitted when a setup is successfully loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupLoaded {
    /// The name of the loaded setup.
    pub name: String,
    /// Number of patches in the loaded setup.
    pub patch_count: u32,
}

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors that can arise when applying commands to a `Setup`.
#[derive(Debug, Clone, PartialEq)]
pub enum SetupError {
    /// The setup name is empty.
    EmptyName,
    /// The file path is empty.
    EmptyPath,
    /// A serialization or deserialization error occurred.
    SerializationError(String),
    /// I/O error (e.g. file not found, permission denied).
    IoError(String),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::EmptyName => write!(f, "setup name must not be empty"),
            SetupError::EmptyPath => write!(f, "file path must not be empty"),
            SetupError::SerializationError(msg) => write!(f, "serialization error: {msg}"),
            SetupError::IoError(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}

impl std::error::Error for SetupError {}

// ─── Aggregate ────────────────────────────────────────────────────────────────

/// The `Setup` aggregate — the full application session state.
///
/// A `Setup` holds:
/// - The session name
/// - An ordered list of serialized patches (one per instrument)
/// - The master effect chain (applied after the mix bus)
/// - The master output gain
///
/// # Serialization guarantee
///
/// `setup_save_setup` serialises all patches, subscriptions, mixer, and effect
/// chains.  `setup_load_setup` reconstructs exactly that state, satisfying the
/// invariant *"restoring a setup must return the app to its exact prior state"*.
///
/// # Audio-thread constraints
///
/// `Setup` is a control-thread value.  `handle_save` and `handle_load` perform
/// blocking file I/O and must never be called from the audio thread.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    /// Human-readable session name.
    pub name: String,
    /// Ordered list of patches in this session.
    pub patches: Vec<SerializedPatch>,
    /// Master effect chain (runs after the mix bus).
    pub master_effect_chain: SerializedEffectChain,
    /// Master output gain (raw non-negative f64).
    pub master_gain: f64,
}

impl Setup {
    /// Construct an empty `Setup` with the given name and unity master gain.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            patches: Vec::new(),
            master_effect_chain: SerializedEffectChain::default(),
            master_gain: 1.0,
        }
    }

    /// Return the number of patches in this setup.
    pub fn patch_count(&self) -> u32 {
        self.patches.len() as u32
    }

    // ── SaveSetup ─────────────────────────────────────────────────────────────

    /// Handle a `SaveSetup` command by writing the setup as a JSON file.
    ///
    /// The file is created at `{cmd.name}.setup.json` relative to the current
    /// working directory. Existing files are overwritten.
    ///
    /// # Errors
    ///
    /// Returns `Err(SetupError::EmptyName)` if `cmd.name` is empty.
    /// Returns `Err(SetupError::IoError)` if writing to disk fails.
    ///
    /// # Blocking
    ///
    /// This method performs blocking I/O and must only be called from the
    /// control thread.
    pub fn handle_save(&mut self, cmd: SaveSetup) -> Result<SetupSaved, SetupError> {
        if cmd.name.is_empty() {
            return Err(SetupError::EmptyName);
        }
        self.name = cmd.name.clone();
        let path = format!("{}.setup.json", cmd.name);
        let json = self.serialize_to_json();
        std::fs::write(&path, json).map_err(|e| SetupError::IoError(e.to_string()))?;
        Ok(SetupSaved {
            name: cmd.name,
            patch_count: self.patch_count(),
        })
    }

    /// Handle a `LoadSetup` command by reading and deserializing a JSON file.
    ///
    /// On success, replaces the aggregate's state with the loaded content and
    /// returns `SetupLoaded`.
    ///
    /// # Errors
    ///
    /// Returns `Err(SetupError::EmptyPath)` if `cmd.path` is empty.
    /// Returns `Err(SetupError::IoError)` if reading from disk fails.
    /// Returns `Err(SetupError::SerializationError)` if the JSON is malformed.
    ///
    /// # Blocking
    ///
    /// This method performs blocking I/O and must only be called from the
    /// control thread.
    pub fn handle_load(&mut self, cmd: LoadSetup) -> Result<SetupLoaded, SetupError> {
        if cmd.path.is_empty() {
            return Err(SetupError::EmptyPath);
        }
        let json =
            std::fs::read_to_string(&cmd.path).map_err(|e| SetupError::IoError(e.to_string()))?;
        let loaded = Self::deserialize_from_json(&json)?;
        let patch_count = loaded.patch_count();
        let name = loaded.name.clone();
        *self = loaded;
        Ok(SetupLoaded { name, patch_count })
    }

    // ── (De)serialization helpers ─────────────────────────────────────────────

    /// Serialize the setup to a minimal JSON string.
    ///
    /// This hand-rolled serializer avoids an external `serde` dependency while
    /// keeping the format human-readable and round-trippable.
    pub fn serialize_to_json(&self) -> String {
        let patches_json: Vec<String> = self.patches.iter().map(serialize_patch).collect();
        let master_chain_json = serialize_effect_chain(&self.master_effect_chain);
        format!(
            r#"{{"name":{name},"master_gain":{gain},"master_effect_chain":{chain},"patches":[{patches}]}}"#,
            name = json_string(&self.name),
            gain = self.master_gain,
            chain = master_chain_json,
            patches = patches_json.join(","),
        )
    }

    /// Deserialize from a JSON string produced by `serialize_to_json`.
    pub fn deserialize_from_json(json: &str) -> Result<Self, SetupError> {
        // Minimal parser for the exact JSON shape we emit.
        parse_setup_json(json).ok_or_else(|| {
            SetupError::SerializationError("malformed or unsupported setup JSON".to_owned())
        })
    }
}

impl Default for Setup {
    fn default() -> Self {
        Self::new("Default")
    }
}

// ─── JSON serialization helpers ───────────────────────────────────────────────

fn json_string(s: &str) -> String {
    // Simple JSON string escaping for printable ASCII (no special Unicode).
    let escaped = s
        .replace('\\', r"\\")
        .replace('"', r#"\""#)
        .replace('\n', r"\n")
        .replace('\r', r"\r")
        .replace('\t', r"\t");
    format!("\"{escaped}\"")
}

fn json_bool(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

fn serialize_effect_type(t: EffectType) -> &'static str {
    match t {
        EffectType::Bypass => "Bypass",
        EffectType::Gain => "Gain",
        EffectType::LowPassFilter => "LowPassFilter",
        EffectType::HighPassFilter => "HighPassFilter",
        EffectType::Delay => "Delay",
    }
}

fn serialize_effect_slot(slot: &SerializedEffectSlot) -> String {
    let p = &slot.params;
    format!(
        r#"{{"effect_type":{et},"bypass":{bypass},"gain":{gain},"cutoff_hz":{cutoff_hz},"resonance":{resonance},"delay_secs":{delay_secs},"feedback":{feedback},"wet_mix":{wet_mix},"sample_rate":{sample_rate}}}"#,
        et = json_string(serialize_effect_type(slot.effect_type)),
        bypass = json_bool(slot.bypass),
        gain = p.gain,
        cutoff_hz = p.cutoff_hz,
        resonance = p.resonance,
        delay_secs = p.delay_secs,
        feedback = p.feedback,
        wet_mix = p.wet_mix,
        sample_rate = p.sample_rate,
    )
}

fn serialize_effect_chain(chain: &SerializedEffectChain) -> String {
    let slots_json: Vec<String> = chain.slots.iter().map(serialize_effect_slot).collect();
    format!(
        r#"{{"bypass":{bypass},"slots":[{slots}]}}"#,
        bypass = json_bool(chain.bypass),
        slots = slots_json.join(","),
    )
}

fn serialize_mod_source(s: ModSourceType) -> &'static str {
    match s {
        ModSourceType::Envelope => "Envelope",
        ModSourceType::Lfo => "Lfo",
        ModSourceType::Random => "Random",
        ModSourceType::Macro => "Macro",
        ModSourceType::Velocity => "Velocity",
        ModSourceType::KeyTrack => "KeyTrack",
        ModSourceType::PerNoteBendX => "PerNoteBendX",
        ModSourceType::PerNoteTimbreY => "PerNoteTimbreY",
        ModSourceType::PerNotePressureZ => "PerNotePressureZ",
    }
}

fn serialize_mod_dest(d: ModDestinationType) -> String {
    match d {
        ModDestinationType::OscillatorPitch => "OscillatorPitch".to_owned(),
        ModDestinationType::OscillatorFineTune => "OscillatorFineTune".to_owned(),
        ModDestinationType::OscillatorShape => "OscillatorShape".to_owned(),
        ModDestinationType::FilterCutoff => "FilterCutoff".to_owned(),
        ModDestinationType::FilterResonance => "FilterResonance".to_owned(),
        ModDestinationType::Amplitude => "Amplitude".to_owned(),
        ModDestinationType::Pan => "Pan".to_owned(),
        ModDestinationType::EnvelopeAttack => "EnvelopeAttack".to_owned(),
        ModDestinationType::EnvelopeDecay => "EnvelopeDecay".to_owned(),
        ModDestinationType::EnvelopeSustain => "EnvelopeSustain".to_owned(),
        ModDestinationType::EnvelopeRelease => "EnvelopeRelease".to_owned(),
        ModDestinationType::LfoRate(i) => format!("LfoRate:{i}"),
        ModDestinationType::LfoDepth(i) => format!("LfoDepth:{i}"),
        ModDestinationType::EffectsSend(i) => format!("EffectsSend:{i}"),
    }
}

fn serialize_lfo_waveform(w: LfoWaveform) -> &'static str {
    match w {
        LfoWaveform::Sine => "Sine",
        LfoWaveform::Triangle => "Triangle",
        LfoWaveform::Square => "Square",
        LfoWaveform::Sawtooth => "Sawtooth",
        LfoWaveform::ReverseSawtooth => "ReverseSawtooth",
        LfoWaveform::SampleAndHold => "SampleAndHold",
    }
}

fn serialize_mod_matrix(matrix: &SerializedModMatrix) -> String {
    let lfos: Vec<String> = matrix
        .lfo_entries
        .iter()
        .map(|e| {
            let c = &e.config;
            format!(
                r#"{{"index":{idx},"rate":{rate},"depth":{depth},"phase":{phase},"sync_to_tempo":{sync},"waveform":{wf}}}"#,
                idx = e.index,
                rate = c.rate,
                depth = c.depth,
                phase = c.phase,
                sync = json_bool(c.sync_to_tempo),
                wf = json_string(serialize_lfo_waveform(c.waveform)),
            )
        })
        .collect();
    let envs: Vec<String> = matrix
        .mod_envelope_entries
        .iter()
        .map(|e| {
            let c = &e.config;
            format!(
                r#"{{"index":{idx},"attack":{a},"decay":{d},"sustain":{s},"release":{r}}}"#,
                idx = e.index,
                a = c.attack,
                d = c.decay,
                s = c.sustain,
                r = c.release,
            )
        })
        .collect();
    let routings: Vec<String> = matrix
        .routings
        .iter()
        .map(|r| {
            format!(
                r#"{{"source":{src},"destination":{dst},"depth":{depth}}}"#,
                src = json_string(serialize_mod_source(r.source)),
                dst = json_string(&serialize_mod_dest(r.destination)),
                depth = r.depth,
            )
        })
        .collect();
    format!(
        r#"{{"lfo_entries":[{lfos}],"mod_envelope_entries":[{envs}],"routings":[{routings}]}}"#,
        lfos = lfos.join(","),
        envs = envs.join(","),
        routings = routings.join(","),
    )
}

fn serialize_waveform(w: Waveform) -> &'static str {
    match w {
        Waveform::Sine => "Sine",
        Waveform::Square => "Square",
        Waveform::Saw => "Saw",
        Waveform::Triangle => "Triangle",
        Waveform::Pulse => "Pulse",
    }
}

fn serialize_filter_type(t: FilterType) -> &'static str {
    match t {
        FilterType::LowPass => "LowPass",
        FilterType::HighPass => "HighPass",
        FilterType::BandPass => "BandPass",
    }
}

fn serialize_engine_type(e: SerializedEngineType) -> &'static str {
    match e {
        SerializedEngineType::Sine => "Sine",
        SerializedEngineType::Wavetable => "Wavetable",
        SerializedEngineType::Subtractive => "Subtractive",
    }
}

fn serialize_stealing_policy(p: SerializedStealingPolicy) -> &'static str {
    match p {
        SerializedStealingPolicy::OldestFirst => "OldestFirst",
        SerializedStealingPolicy::QuietestFirst => "QuietestFirst",
        SerializedStealingPolicy::NoStealing => "NoStealing",
    }
}

fn serialize_patch(patch: &SerializedPatch) -> String {
    let osc = &patch.oscillator;
    let env = &patch.amp_envelope;
    let flt = &patch.filter;
    let matrix_json = serialize_mod_matrix(&patch.mod_matrix);
    let chain_json = serialize_effect_chain(&patch.effect_chain);
    let osc_json = format!(
        r#"{{"detune":{d},"pulse_width":{pw},"waveform":{wf}}}"#,
        d = osc.detune,
        pw = osc.pulse_width,
        wf = json_string(serialize_waveform(osc.waveform)),
    );
    let env_json = format!(
        r#"{{"attack":{a},"decay":{d},"sustain":{s},"release":{r}}}"#,
        a = env.attack,
        d = env.decay,
        s = env.sustain,
        r = env.release,
    );
    let flt_json = format!(
        r#"{{"cutoff_hz":{c},"filter_type":{ft},"resonance":{res}}}"#,
        c = flt.cutoff_hz,
        ft = json_string(serialize_filter_type(flt.filter_type)),
        res = flt.resonance,
    );
    format!(
        r#"{{"patch_id":{pid},"name":{name},"active":{active},"engine_type":{et},"oscillator":{osc},"amp_envelope":{env},"filter":{flt},"gain":{gain},"pan":{pan},"midi_group":{mg},"midi_channel":{mc},"max_voices":{mv},"stealing_policy":{sp},"mod_matrix":{matrix},"effect_chain":{chain}}}"#,
        pid = patch.patch_id,
        name = json_string(&patch.name),
        active = json_bool(patch.active),
        et = json_string(serialize_engine_type(patch.engine_type)),
        osc = osc_json,
        env = env_json,
        flt = flt_json,
        gain = patch.gain,
        pan = patch.pan,
        mg = patch.midi_group,
        mc = patch.midi_channel,
        mv = patch.max_voices,
        sp = json_string(serialize_stealing_policy(patch.stealing_policy)),
        matrix = matrix_json,
        chain = chain_json,
    )
}

// ─── JSON deserialization (minimal hand-rolled parser) ────────────────────────

/// Parse a `"key":value` pair from a JSON object string.
///
/// Returns the raw value string (not unquoted) for the first occurrence of `key`.
fn json_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\":", key);
    let start = json.find(needle.as_str())? + needle.len();
    let rest = json[start..].trim_start();
    // Determine end of value by scanning past the value.
    let end = find_json_value_end(rest)?;
    Some(&rest[..end])
}

/// Find the end index of a JSON value starting at position 0 of `s`.
fn find_json_value_end(s: &str) -> Option<usize> {
    let s = s.trim_start();
    let first = s.chars().next()?;
    match first {
        '"' => {
            // String: scan for closing unescaped quote.
            let mut i = 1;
            let bytes = s.as_bytes();
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                } else if bytes[i] == b'"' {
                    return Some(i + 1);
                } else {
                    i += 1;
                }
            }
            None
        }
        '{' | '[' => {
            // Object / array: scan matching bracket.
            let open = first;
            let close = if open == '{' { '}' } else { ']' };
            let mut depth: i32 = 0;
            let mut in_str = false;
            let mut escape = false;
            for (i, ch) in s.char_indices() {
                if escape {
                    escape = false;
                    continue;
                }
                if ch == '\\' && in_str {
                    escape = true;
                    continue;
                }
                if ch == '"' {
                    in_str = !in_str;
                    continue;
                }
                if in_str {
                    continue;
                }
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + ch.len_utf8());
                    }
                }
            }
            None
        }
        _ => {
            // Number, bool, null: scan until delimiter.
            let end = s.find([',', '}', ']']).unwrap_or(s.len());
            Some(end)
        }
    }
}

/// Unquote a JSON string value (strip surrounding quotes and unescape).
fn unquote(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        Some(
            inner
                .replace(r"\\\\", "\\")
                .replace("\\\"", "\"")
                .replace(r"\n", "\n")
                .replace(r"\r", "\r")
                .replace(r"\t", "\t"),
        )
    } else {
        None
    }
}

fn parse_f64(s: &str) -> Option<f64> {
    s.trim().parse().ok()
}
fn parse_f32(s: &str) -> Option<f32> {
    s.trim().parse().ok()
}
fn parse_u8(s: &str) -> Option<u8> {
    s.trim().parse().ok()
}
fn parse_u32(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}
fn parse_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_effect_type(s: &str) -> Option<EffectType> {
    match s {
        "Bypass" => Some(EffectType::Bypass),
        "Gain" => Some(EffectType::Gain),
        "LowPassFilter" => Some(EffectType::LowPassFilter),
        "HighPassFilter" => Some(EffectType::HighPassFilter),
        "Delay" => Some(EffectType::Delay),
        _ => None,
    }
}

fn parse_effect_slot(json: &str) -> Option<SerializedEffectSlot> {
    let et_raw = json_field(json, "effect_type")?;
    let et = parse_effect_type(&unquote(et_raw)?)?;
    let bypass = parse_bool(json_field(json, "bypass")?)?;
    let gain = parse_f32(json_field(json, "gain")?)?;
    let cutoff_hz = parse_f32(json_field(json, "cutoff_hz")?)?;
    let resonance = parse_f32(json_field(json, "resonance")?)?;
    let delay_secs = parse_f32(json_field(json, "delay_secs")?)?;
    let feedback = parse_f32(json_field(json, "feedback")?)?;
    let wet_mix = parse_f32(json_field(json, "wet_mix")?)?;
    let sample_rate = parse_f32(json_field(json, "sample_rate")?)?;
    let params = EffectParams {
        effect_type: et,
        gain,
        cutoff_hz,
        resonance,
        delay_secs,
        feedback,
        wet_mix,
        sample_rate,
    };
    Some(SerializedEffectSlot {
        effect_type: et,
        bypass,
        params,
    })
}

fn parse_effect_chain(json: &str) -> Option<SerializedEffectChain> {
    let bypass = parse_bool(json_field(json, "bypass")?)?;
    let slots_raw = json_field(json, "slots")?;
    let slots = parse_json_array(slots_raw, parse_effect_slot);
    Some(SerializedEffectChain { bypass, slots })
}

/// Parse a JSON array `[...]` by splitting on top-level commas.
fn parse_json_array<T, F: Fn(&str) -> Option<T>>(json: &str, parse_item: F) -> Vec<T> {
    let json = json.trim();
    if !json.starts_with('[') || !json.ends_with(']') {
        return Vec::new();
    }
    let inner = &json[1..json.len() - 1].trim();
    if inner.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();
    let mut rest = *inner;
    while !rest.is_empty() {
        let rest_trimmed = rest.trim_start();
        if rest_trimmed.is_empty() {
            break;
        }
        match find_json_value_end(rest_trimmed) {
            None => break,
            Some(end) => {
                let item_str = &rest_trimmed[..end];
                if let Some(item) = parse_item(item_str) {
                    results.push(item);
                }
                let after = &rest_trimmed[end..].trim_start();
                if let Some(stripped) = after.strip_prefix(',') {
                    rest = stripped;
                } else {
                    break;
                }
            }
        }
    }
    results
}

fn parse_mod_source(s: &str) -> Option<ModSourceType> {
    match s {
        "Envelope" => Some(ModSourceType::Envelope),
        "Lfo" => Some(ModSourceType::Lfo),
        "Random" => Some(ModSourceType::Random),
        "Macro" => Some(ModSourceType::Macro),
        "Velocity" => Some(ModSourceType::Velocity),
        "KeyTrack" => Some(ModSourceType::KeyTrack),
        "PerNoteBendX" => Some(ModSourceType::PerNoteBendX),
        "PerNoteTimbreY" => Some(ModSourceType::PerNoteTimbreY),
        "PerNotePressureZ" => Some(ModSourceType::PerNotePressureZ),
        _ => None,
    }
}

fn parse_mod_dest(s: &str) -> Option<ModDestinationType> {
    match s {
        "OscillatorPitch" => Some(ModDestinationType::OscillatorPitch),
        "OscillatorFineTune" => Some(ModDestinationType::OscillatorFineTune),
        "OscillatorShape" => Some(ModDestinationType::OscillatorShape),
        "FilterCutoff" => Some(ModDestinationType::FilterCutoff),
        "FilterResonance" => Some(ModDestinationType::FilterResonance),
        "Amplitude" => Some(ModDestinationType::Amplitude),
        "Pan" => Some(ModDestinationType::Pan),
        "EnvelopeAttack" => Some(ModDestinationType::EnvelopeAttack),
        "EnvelopeDecay" => Some(ModDestinationType::EnvelopeDecay),
        "EnvelopeSustain" => Some(ModDestinationType::EnvelopeSustain),
        "EnvelopeRelease" => Some(ModDestinationType::EnvelopeRelease),
        s if s.starts_with("LfoRate:") => s[8..].parse().ok().map(ModDestinationType::LfoRate),
        s if s.starts_with("LfoDepth:") => s[9..].parse().ok().map(ModDestinationType::LfoDepth),
        s if s.starts_with("EffectsSend:") => {
            s[12..].parse().ok().map(ModDestinationType::EffectsSend)
        }
        _ => None,
    }
}

fn parse_lfo_waveform(s: &str) -> Option<LfoWaveform> {
    match s {
        "Sine" => Some(LfoWaveform::Sine),
        "Triangle" => Some(LfoWaveform::Triangle),
        "Square" => Some(LfoWaveform::Square),
        "Sawtooth" => Some(LfoWaveform::Sawtooth),
        "ReverseSawtooth" => Some(LfoWaveform::ReverseSawtooth),
        "SampleAndHold" => Some(LfoWaveform::SampleAndHold),
        _ => None,
    }
}

fn parse_mod_routing(json: &str) -> Option<SerializedModRouting> {
    let src = parse_mod_source(&unquote(json_field(json, "source")?)?)?;
    let dst = parse_mod_dest(&unquote(json_field(json, "destination")?)?)?;
    let depth = parse_f64(json_field(json, "depth")?)?;
    Some(SerializedModRouting {
        source: src,
        destination: dst,
        depth,
    })
}

fn parse_lfo_entry(json: &str) -> Option<SerializedLfoEntry> {
    let index = parse_u8(json_field(json, "index")?)?;
    let rate = parse_f64(json_field(json, "rate")?)?;
    let depth = parse_f64(json_field(json, "depth")?)?;
    let phase = parse_f64(json_field(json, "phase")?)?;
    let sync_to_tempo = parse_bool(json_field(json, "sync_to_tempo")?)?;
    let waveform = parse_lfo_waveform(&unquote(json_field(json, "waveform")?)?)?;
    Some(SerializedLfoEntry {
        index,
        config: SerializedLfoConfig {
            rate,
            depth,
            phase,
            sync_to_tempo,
            waveform,
        },
    })
}

fn parse_mod_envelope_entry(json: &str) -> Option<SerializedModEnvelopeEntry> {
    let index = parse_u8(json_field(json, "index")?)?;
    let attack = parse_f64(json_field(json, "attack")?)?;
    let decay = parse_f64(json_field(json, "decay")?)?;
    let sustain = parse_f64(json_field(json, "sustain")?)?;
    let release = parse_f64(json_field(json, "release")?)?;
    Some(SerializedModEnvelopeEntry {
        index,
        config: ModEnvelopeConfig {
            attack,
            decay,
            sustain,
            release,
        },
    })
}

fn parse_mod_matrix(json: &str) -> Option<SerializedModMatrix> {
    let lfos_raw = json_field(json, "lfo_entries")?;
    let envs_raw = json_field(json, "mod_envelope_entries")?;
    let routings_raw = json_field(json, "routings")?;
    let lfo_entries = parse_json_array(lfos_raw, parse_lfo_entry);
    let mod_envelope_entries = parse_json_array(envs_raw, parse_mod_envelope_entry);
    let routings = parse_json_array(routings_raw, parse_mod_routing);
    Some(SerializedModMatrix {
        lfo_entries,
        mod_envelope_entries,
        routings,
    })
}

fn parse_waveform(s: &str) -> Option<Waveform> {
    match s {
        "Sine" => Some(Waveform::Sine),
        "Square" => Some(Waveform::Square),
        "Saw" => Some(Waveform::Saw),
        "Triangle" => Some(Waveform::Triangle),
        "Pulse" => Some(Waveform::Pulse),
        _ => None,
    }
}

fn parse_filter_type(s: &str) -> Option<FilterType> {
    match s {
        "LowPass" => Some(FilterType::LowPass),
        "HighPass" => Some(FilterType::HighPass),
        "BandPass" => Some(FilterType::BandPass),
        _ => None,
    }
}

fn parse_engine_type(s: &str) -> Option<SerializedEngineType> {
    match s {
        "Sine" => Some(SerializedEngineType::Sine),
        "Wavetable" => Some(SerializedEngineType::Wavetable),
        "Subtractive" => Some(SerializedEngineType::Subtractive),
        _ => None,
    }
}

fn parse_stealing_policy(s: &str) -> Option<SerializedStealingPolicy> {
    match s {
        "OldestFirst" => Some(SerializedStealingPolicy::OldestFirst),
        "QuietestFirst" => Some(SerializedStealingPolicy::QuietestFirst),
        "NoStealing" => Some(SerializedStealingPolicy::NoStealing),
        _ => None,
    }
}

fn parse_oscillator(json: &str) -> Option<SerializedOscillatorConfig> {
    let detune = parse_f64(json_field(json, "detune")?)?;
    let pulse_width = parse_f64(json_field(json, "pulse_width")?)?;
    let waveform = parse_waveform(&unquote(json_field(json, "waveform")?)?)?;
    Some(SerializedOscillatorConfig {
        detune,
        pulse_width,
        waveform,
    })
}

fn parse_amp_envelope(json: &str) -> Option<SerializedEnvelopeConfig> {
    let attack = parse_f64(json_field(json, "attack")?)?;
    let decay = parse_f64(json_field(json, "decay")?)?;
    let sustain = parse_f64(json_field(json, "sustain")?)?;
    let release = parse_f64(json_field(json, "release")?)?;
    Some(SerializedEnvelopeConfig {
        attack,
        decay,
        sustain,
        release,
    })
}

fn parse_filter(json: &str) -> Option<SerializedFilterConfig> {
    let cutoff_hz = parse_f64(json_field(json, "cutoff_hz")?)?;
    let filter_type = parse_filter_type(&unquote(json_field(json, "filter_type")?)?)?;
    let resonance = parse_f64(json_field(json, "resonance")?)?;
    Some(SerializedFilterConfig {
        cutoff_hz,
        filter_type,
        resonance,
    })
}

fn parse_patch(json: &str) -> Option<SerializedPatch> {
    let patch_id = parse_u32(json_field(json, "patch_id")?)?;
    let name = unquote(json_field(json, "name")?)?;
    let active = parse_bool(json_field(json, "active")?)?;
    let engine_type = parse_engine_type(&unquote(json_field(json, "engine_type")?)?)?;
    let oscillator = parse_oscillator(json_field(json, "oscillator")?)?;
    let amp_envelope = parse_amp_envelope(json_field(json, "amp_envelope")?)?;
    let filter = parse_filter(json_field(json, "filter")?)?;
    let gain = parse_f64(json_field(json, "gain")?)?;
    let pan = parse_f64(json_field(json, "pan")?)?;
    let midi_group = parse_u8(json_field(json, "midi_group")?)?;
    let midi_channel = parse_u8(json_field(json, "midi_channel")?)?;
    let max_voices = parse_u8(json_field(json, "max_voices")?)?;
    let stealing_policy = parse_stealing_policy(&unquote(json_field(json, "stealing_policy")?)?)?;
    let mod_matrix = parse_mod_matrix(json_field(json, "mod_matrix")?)?;
    let effect_chain = parse_effect_chain(json_field(json, "effect_chain")?)?;
    Some(SerializedPatch {
        patch_id,
        name,
        active,
        engine_type,
        oscillator,
        amp_envelope,
        filter,
        gain,
        pan,
        midi_group,
        midi_channel,
        max_voices,
        stealing_policy,
        mod_matrix,
        effect_chain,
    })
}

fn parse_setup_json(json: &str) -> Option<Setup> {
    let name = unquote(json_field(json, "name")?)?;
    let master_gain = parse_f64(json_field(json, "master_gain")?)?;
    let master_effect_chain = parse_effect_chain(json_field(json, "master_effect_chain")?)?;
    let patches_raw = json_field(json, "patches")?;
    let patches = parse_json_array(patches_raw, parse_patch);
    Some(Setup {
        name,
        patches,
        master_effect_chain,
        master_gain,
    })
}

// ─── Convenience builders ─────────────────────────────────────────────────────

/// Build a `SerializedPatch` from its constituent parts.
///
/// Used in tests and higher-level application code to create patch snapshots.
#[allow(clippy::too_many_arguments)]
pub fn build_serialized_patch(
    patch_id: u32,
    name: impl Into<String>,
    active: bool,
    engine_type: SerializedEngineType,
    oscillator: OscillatorConfig,
    amp_envelope: AmpEnvelopeConfig,
    filter: FilterConfig,
    gain: Amplitude,
    pan: f64,
    subscription: ChannelSubscription,
    voice_pool_config: VoicePoolConfig,
    mod_matrix: SerializedModMatrix,
    effect_chain: SerializedEffectChain,
) -> SerializedPatch {
    SerializedPatch {
        patch_id,
        name: name.into(),
        active,
        engine_type,
        oscillator: oscillator.into(),
        amp_envelope: amp_envelope.into(),
        filter: filter.into(),
        gain: gain.value(),
        pan,
        midi_group: subscription.address().group().value(),
        midi_channel: subscription.address().channel().value(),
        max_voices: voice_pool_config.max_voices(),
        stealing_policy: voice_pool_config.stealing_policy().into(),
        mod_matrix,
        effect_chain,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_group::MidiGroup;
    use crate::patch::channel_subscription::ChannelAddress;
    use crate::patch::voice_pool_config::StealingPolicy;

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn make_subscription() -> ChannelSubscription {
        let group = MidiGroup::try_new(0).unwrap();
        let channel = MidiChannel::try_new(0).unwrap();
        let address = ChannelAddress::new(group, channel);
        ChannelSubscription::new(address, None)
    }

    fn make_patch(id: u32, name: &str) -> SerializedPatch {
        build_serialized_patch(
            id,
            name,
            true,
            SerializedEngineType::Sine,
            OscillatorConfig::default(),
            AmpEnvelopeConfig::default(),
            FilterConfig::default(),
            Amplitude::unity(),
            0.0,
            make_subscription(),
            VoicePoolConfig::default(),
            SerializedModMatrix::default(),
            SerializedEffectChain::default(),
        )
    }

    // ── Setup::new ─────────────────────────────────────────────────────────────

    #[test]
    fn setup_new_creates_empty_setup_with_given_name() {
        let s = Setup::new("Live Set");
        assert_eq!(s.name, "Live Set");
        assert!(s.patches.is_empty());
        assert_eq!(s.patch_count(), 0);
        assert!((s.master_gain - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn setup_default_is_named_default() {
        let s = Setup::default();
        assert_eq!(s.name, "Default");
    }

    #[test]
    fn patch_count_reflects_patches_vec() {
        let mut s = Setup::new("Test");
        assert_eq!(s.patch_count(), 0);
        s.patches.push(make_patch(1, "Lead"));
        assert_eq!(s.patch_count(), 1);
        s.patches.push(make_patch(2, "Bass"));
        assert_eq!(s.patch_count(), 2);
    }

    // ── SaveSetup / LoadSetup ─────────────────────────────────────────────────

    #[test]
    fn save_setup_returns_error_for_empty_name() {
        let mut s = Setup::new("Test");
        let result = s.handle_save(SaveSetup {
            name: String::new(),
        });
        assert!(matches!(result, Err(SetupError::EmptyName)));
    }

    #[test]
    fn load_setup_returns_error_for_empty_path() {
        let mut s = Setup::new("Test");
        let result = s.handle_load(LoadSetup {
            path: String::new(),
        });
        assert!(matches!(result, Err(SetupError::EmptyPath)));
    }

    #[test]
    fn save_and_load_roundtrip_preserves_setup_state() {
        let dir = tempfile::tempdir().unwrap();
        let name = "roundtrip_test";
        let path = dir
            .path()
            .join(format!("{name}.setup.json"))
            .to_string_lossy()
            .to_string();

        let mut original = Setup::new(name);
        original.patches.push(make_patch(1, "Lead Synth"));
        original.patches.push(make_patch(2, "Bass"));
        original.master_gain = 0.8;

        // Save using the path trick: write directly to the expected path.
        let json = original.serialize_to_json();
        std::fs::write(&path, &json).unwrap();

        // Load from that path.
        let mut loaded = Setup::new("blank");
        let event = loaded
            .handle_load(LoadSetup { path: path.clone() })
            .unwrap();

        assert_eq!(event.name, name);
        assert_eq!(event.patch_count, 2);
        assert_eq!(loaded.name, name);
        assert_eq!(loaded.patches.len(), 2);
        assert_eq!(loaded.patches[0].name, "Lead Synth");
        assert_eq!(loaded.patches[1].name, "Bass");
        assert!((loaded.master_gain - 0.8).abs() < 1e-9);
    }

    #[test]
    fn save_and_load_roundtrip_with_handle_save() {
        let dir = tempfile::tempdir().unwrap();
        // Change to temp dir so the file lands there.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mut original = Setup::new("SaveHandle");
        original.patches.push(make_patch(42, "Pad"));
        original.master_gain = 0.5;

        let save_event = original
            .handle_save(SaveSetup {
                name: "SaveHandle".to_owned(),
            })
            .unwrap();
        assert_eq!(save_event.name, "SaveHandle");
        assert_eq!(save_event.patch_count, 1);

        // Verify the file was created in the temp dir.
        let json_path = dir.path().join("SaveHandle.setup.json");
        assert!(json_path.exists(), "expected file to be created");

        // Restore working directory.
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn load_setup_returns_error_for_missing_file() {
        let mut s = Setup::new("Test");
        let result = s.handle_load(LoadSetup {
            path: "/tmp/nonexistent_crest_setup_file_xyz.json".to_owned(),
        });
        assert!(matches!(result, Err(SetupError::IoError(_))));
    }

    #[test]
    fn load_setup_returns_error_for_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json").to_string_lossy().to_string();
        std::fs::write(&path, b"not valid json").unwrap();

        let mut s = Setup::new("Test");
        let result = s.handle_load(LoadSetup { path });
        assert!(matches!(result, Err(SetupError::SerializationError(_))));
    }

    // ── Serialization correctness ─────────────────────────────────────────────

    #[test]
    fn setup_serializes_name_and_patch_count() {
        let mut s = Setup::new("My Setup");
        s.patches.push(make_patch(1, "Kick"));
        let json = s.serialize_to_json();
        assert!(json.contains("My Setup"), "name should appear in JSON");
        assert!(json.contains("Kick"), "patch name should appear in JSON");
    }

    #[test]
    fn serialized_effect_chain_default_roundtrips() {
        let chain = SerializedEffectChain::default();
        let json = serialize_effect_chain(&chain);
        let parsed = parse_effect_chain(&json).unwrap();
        assert_eq!(parsed, chain);
    }

    #[test]
    fn serialized_effect_chain_with_slots_roundtrips() {
        let slot = SerializedEffectSlot {
            effect_type: EffectType::Gain,
            bypass: false,
            params: EffectParams {
                effect_type: EffectType::Gain,
                gain: 2.0,
                wet_mix: 1.0,
                ..EffectParams::default()
            },
        };
        let chain = SerializedEffectChain {
            bypass: false,
            slots: vec![slot],
        };
        let json = serialize_effect_chain(&chain);
        let parsed = parse_effect_chain(&json).unwrap();
        assert_eq!(parsed.slots.len(), 1);
        assert_eq!(parsed.slots[0].effect_type, EffectType::Gain);
        assert!((parsed.slots[0].params.gain - 2.0).abs() < 1e-6);
    }

    #[test]
    fn patch_roundtrip_preserves_all_fields() {
        let patch = make_patch(99, "Strings");
        let json = serialize_patch(&patch);
        let parsed = parse_patch(&json).unwrap();
        assert_eq!(parsed.patch_id, 99);
        assert_eq!(parsed.name, "Strings");
        assert!(parsed.active);
        assert_eq!(parsed.engine_type, SerializedEngineType::Sine);
        assert!((parsed.gain - 1.0).abs() < f64::EPSILON);
    }

    // ── Modulation matrix roundtrip ───────────────────────────────────────────

    #[test]
    fn mod_matrix_with_routing_roundtrips() {
        let matrix = SerializedModMatrix {
            lfo_entries: vec![SerializedLfoEntry {
                index: 0,
                config: SerializedLfoConfig {
                    rate: 3.0,
                    depth: 0.5,
                    phase: 0.0,
                    sync_to_tempo: false,
                    waveform: LfoWaveform::Triangle,
                },
            }],
            mod_envelope_entries: vec![SerializedModEnvelopeEntry {
                index: 0,
                config: ModEnvelopeConfig::default(),
            }],
            routings: vec![SerializedModRouting {
                source: ModSourceType::Lfo,
                destination: ModDestinationType::FilterCutoff,
                depth: 0.7,
            }],
        };
        let json = serialize_mod_matrix(&matrix);
        let parsed = parse_mod_matrix(&json).unwrap();
        assert_eq!(parsed.lfo_entries.len(), 1);
        assert!((parsed.lfo_entries[0].config.rate - 3.0).abs() < f64::EPSILON);
        assert_eq!(parsed.routings.len(), 1);
        assert_eq!(parsed.routings[0].source, ModSourceType::Lfo);
        assert_eq!(
            parsed.routings[0].destination,
            ModDestinationType::FilterCutoff
        );
        assert!((parsed.routings[0].depth - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn per_note_expression_routing_roundtrips() {
        let matrix = SerializedModMatrix {
            lfo_entries: Vec::new(),
            mod_envelope_entries: Vec::new(),
            routings: vec![SerializedModRouting {
                source: ModSourceType::PerNoteBendX,
                destination: ModDestinationType::OscillatorPitch,
                depth: 1.0,
            }],
        };
        let json = serialize_mod_matrix(&matrix);
        let parsed = parse_mod_matrix(&json).unwrap();
        assert_eq!(parsed.routings[0].source, ModSourceType::PerNoteBendX);
        assert_eq!(
            parsed.routings[0].destination,
            ModDestinationType::OscillatorPitch
        );
    }

    // ── Setup preserves patches and subscriptions ──────────────────────────────

    #[test]
    fn setup_preserves_all_patches_on_roundtrip() {
        let mut s = Setup::new("Multi");
        for i in 0u32..4 {
            s.patches.push(make_patch(i, &format!("Patch {i}")));
        }
        let json = s.serialize_to_json();
        let loaded = Setup::deserialize_from_json(&json).unwrap();
        assert_eq!(loaded.patches.len(), 4);
        for (i, patch) in loaded.patches.iter().enumerate() {
            assert_eq!(patch.name, format!("Patch {i}"));
        }
    }

    #[test]
    fn setup_preserves_master_gain_on_roundtrip() {
        let mut s = Setup::new("GainTest");
        s.master_gain = 0.42;
        let json = s.serialize_to_json();
        let loaded = Setup::deserialize_from_json(&json).unwrap();
        assert!((loaded.master_gain - 0.42).abs() < 1e-9);
    }

    #[test]
    fn setup_preserves_master_effect_chain_bypass_on_roundtrip() {
        let mut s = Setup::new("FxTest");
        s.master_effect_chain = SerializedEffectChain {
            bypass: true,
            slots: Vec::new(),
        };
        let json = s.serialize_to_json();
        let loaded = Setup::deserialize_from_json(&json).unwrap();
        assert!(loaded.master_effect_chain.bypass);
    }

    // ── Error display ──────────────────────────────────────────────────────────

    #[test]
    fn error_empty_name_display() {
        assert!(SetupError::EmptyName.to_string().contains("empty"));
    }

    #[test]
    fn error_empty_path_display() {
        assert!(SetupError::EmptyPath.to_string().contains("path"));
    }

    #[test]
    fn error_io_display_contains_message() {
        let e = SetupError::IoError("disk full".to_owned());
        assert!(e.to_string().contains("disk full"));
    }

    #[test]
    fn error_serialization_display_contains_message() {
        let e = SetupError::SerializationError("bad JSON".to_owned());
        assert!(e.to_string().contains("bad JSON"));
    }

    // ── StealingPolicy / EngineType conversions ─────────────────────────────────

    #[test]
    fn stealing_policy_roundtrip() {
        let policies = [
            StealingPolicy::OldestFirst,
            StealingPolicy::QuietestFirst,
            StealingPolicy::NoStealing,
        ];
        for policy in policies {
            let serialized: SerializedStealingPolicy = policy.into();
            let restored: StealingPolicy = serialized.into();
            assert_eq!(policy, restored);
        }
    }

    // ── build_serialized_patch subscription fields ───────────────────────────────

    #[test]
    fn build_serialized_patch_stores_subscription_fields() {
        let group = MidiGroup::try_new(3).unwrap();
        let channel = MidiChannel::try_new(7).unwrap();
        let address = ChannelAddress::new(group, channel);
        let sub = ChannelSubscription::new(address, None);
        let patch = build_serialized_patch(
            1,
            "Test",
            false,
            SerializedEngineType::Subtractive,
            OscillatorConfig::default(),
            AmpEnvelopeConfig::default(),
            FilterConfig::default(),
            Amplitude::unity(),
            0.5,
            sub,
            VoicePoolConfig::default(),
            SerializedModMatrix::default(),
            SerializedEffectChain::default(),
        );
        assert_eq!(patch.midi_group, 3);
        assert_eq!(patch.midi_channel, 7);
        assert!((patch.pan - 0.5).abs() < f64::EPSILON);
    }
}
