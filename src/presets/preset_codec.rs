// path: src/presets/preset_codec.rs

//! `PresetCodec` — serialise/deserialise [`Preset`] and [`Setup`] values.
//!
//! # Format
//!
//! Uses a hand-rolled JSON serializer (human-readable, UTF-8 text) that is
//! fully round-trippable: `deserialize(serialize(p)) == p`.
//!
//! `Setup` delegates to the serialization methods on the [`Setup`] aggregate
//! itself. `Preset` serialization mirrors the same hand-rolled pattern used in
//! `setup.rs`.
//!
//! # Audio-thread safety
//!
//! `PresetCodec`, `Preset`, and `Setup` **never** run on the audio thread.
//! Serialisation/deserialisation happen only on the control/UI thread.

use crate::effects::effect_processor::{EffectParams, EffectType};
use crate::modulation::lfo_waveform::LfoWaveform;
use crate::modulation::mod_destination_type::ModDestinationType;
use crate::modulation::mod_source_type::ModSourceType;
use crate::presets::preset::{
    EngineType, Preset, SerializedEffectChain, SerializedEffectSlot, SerializedModMatrix,
    SerializedModRouting,
};
use crate::presets::preset_id::PresetId;
use crate::presets::preset_metadata::PresetMetadata;
use crate::presets::setup::{Setup, SetupError};
use crate::synth::amp_envelope_config::AmpEnvelopeConfig;
use crate::synth::filter_config::{FilterConfig, FilterType};
use crate::synth::oscillator_config::{OscillatorConfig, Waveform};
use crate::synth::sample_player_config::{InterpolationMode, LoopMode, SamplePlayerConfig};

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Error returned by [`PresetCodec`] when serialisation or deserialisation fails.
#[derive(Debug, Clone, PartialEq)]
pub enum CodecError {
    /// The bytes could not be decoded as valid JSON.
    InvalidJson(String),
    /// The decoded value is structurally invalid (e.g. a field value is out of
    /// range, a required field is missing, or a type tag is unrecognised).
    InvalidData(String),
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::InvalidJson(msg) => write!(f, "codec: invalid JSON: {msg}"),
            CodecError::InvalidData(msg) => write!(f, "codec: invalid data: {msg}"),
        }
    }
}

impl std::error::Error for CodecError {}

impl From<SetupError> for CodecError {
    fn from(e: SetupError) -> Self {
        match e {
            SetupError::SerializationError(msg) => CodecError::InvalidJson(msg),
            other => CodecError::InvalidData(other.to_string()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PresetCodec
// ─────────────────────────────────────────────────────────────────────────────

/// Codec that serialises and deserialises [`Preset`] and [`Setup`] values.
///
/// The implementation uses a hand-rolled UTF-8 JSON format that is
/// human-readable and fully round-trippable. No external serde dependency is
/// required.
///
/// | method              | contract signature                                 |
/// |---------------------|----------------------------------------------------|\n/// | `serialize`         | `Preset → Vec<u8>`                                 |
/// | `deserialize`       | `Vec<u8> → Result<Preset, CodecError>`             |
/// | `serialize_setup`   | `Setup → Vec<u8>`                                  |
/// | `deserialize_setup` | `Vec<u8> → Result<Setup, CodecError>`              |
///
/// `PresetCodec` has no mutable state — it is safe to construct once and reuse.
///
/// # Audio-thread safety
///
/// `PresetCodec` must **never** be used on the audio thread. JSON parsing
/// allocates heap memory. All codec operations belong on the control / UI thread.
pub struct PresetCodec;

impl PresetCodec {
    /// Create a new `PresetCodec`.
    pub fn new() -> Self {
        Self
    }

    // ── Preset ────────────────────────────────────────────────────────────────────────

    /// Serialise a [`Preset`] to UTF-8 JSON bytes.
    ///
    /// The resulting bytes can be written to disk, sent over a network, or
    /// stored in a database and later restored via [`PresetCodec::deserialize`].
    pub fn serialize(&self, preset: Preset) -> Vec<u8> {
        serialize_preset_to_json(&preset).into_bytes()
    }

    /// Deserialise a [`Preset`] from UTF-8 JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidJson`] when `bytes` cannot be parsed as
    /// valid UTF-8 JSON or when a required field is missing.
    ///
    /// Returns [`CodecError::InvalidData`] when a field value violates an
    /// invariant (e.g. resonance out of range, unknown engine type).
    pub fn deserialize(&self, bytes: Vec<u8>) -> Result<Preset, CodecError> {
        let json =
            std::str::from_utf8(&bytes).map_err(|e| CodecError::InvalidJson(e.to_string()))?;
        parse_preset_json(json)
    }

    // ── Setup ─────────────────────────────────────────────────────────────────────────

    /// Serialise a [`Setup`] to UTF-8 JSON bytes.
    pub fn serialize_setup(&self, setup: Setup) -> Vec<u8> {
        setup.serialize_to_json().into_bytes()
    }

    /// Deserialise a [`Setup`] from UTF-8 JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidJson`] when `bytes` cannot be parsed as
    /// valid UTF-8 JSON or the structure does not match `Setup`.
    pub fn deserialize_setup(&self, bytes: Vec<u8>) -> Result<Setup, CodecError> {
        let json =
            std::str::from_utf8(&bytes).map_err(|e| CodecError::InvalidJson(e.to_string()))?;
        Setup::deserialize_from_json(json).map_err(CodecError::from)
    }
}

impl Default for PresetCodec {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Preset JSON serialization (hand-rolled, mirrors setup.rs pattern)
// ─────────────────────────────────────────────────────────────────────────────

fn json_string(s: &str) -> String {
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

fn serialize_engine_type(e: EngineType) -> &'static str {
    match e {
        EngineType::Oscillator => "Oscillator",
        EngineType::SamplePlayer => "SamplePlayer",
    }
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

fn serialize_effect_type(t: EffectType) -> &'static str {
    match t {
        EffectType::Bypass => "Bypass",
        EffectType::Gain => "Gain",
        EffectType::LowPassFilter => "LowPassFilter",
        EffectType::HighPassFilter => "HighPassFilter",
        EffectType::Delay => "Delay",
    }
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

fn serialize_interpolation_mode(m: InterpolationMode) -> &'static str {
    match m {
        InterpolationMode::NearestNeighbour => "NearestNeighbour",
        InterpolationMode::Linear => "Linear",
        InterpolationMode::Cubic => "Cubic",
    }
}

fn serialize_loop_mode(m: LoopMode) -> &'static str {
    match m {
        LoopMode::None => "None",
        LoopMode::Sustain => "Sustain",
        LoopMode::SustainRelease => "SustainRelease",
        LoopMode::PingPong => "PingPong",
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

fn serialize_mod_routing(r: &SerializedModRouting) -> String {
    format!(
        r#"{{"source":{src},"destination":{dst},"depth":{depth}}}"#,
        src = json_string(serialize_mod_source(r.source)),
        dst = json_string(&serialize_mod_dest(r.destination)),
        depth = r.depth,
    )
}

fn serialize_mod_matrix(matrix: &SerializedModMatrix) -> String {
    let lfos: Vec<String> = matrix
        .lfo_configs
        .iter()
        .enumerate()
        .map(|(i, c)| {
            format!(
                r#"{{"index":{idx},"rate":{rate},"depth":{depth},"phase":{phase},"sync_to_tempo":{sync},"waveform":{wf}}}"#,
                idx = i,
                rate = c.rate,
                depth = c.depth,
                phase = c.phase,
                sync = json_bool(c.sync_to_tempo),
                wf = json_string(serialize_lfo_waveform(c.waveform)),
            )
        })
        .collect();
    let envs: Vec<String> = matrix
        .mod_envelopes
        .iter()
        .enumerate()
        .map(|(i, e)| {
            format!(
                r#"{{"index":{idx},"attack":{a},"decay":{d},"sustain":{s},"release":{r}}}"#,
                idx = i,
                a = e.attack,
                d = e.decay,
                s = e.sustain,
                r = e.release,
            )
        })
        .collect();
    let routings: Vec<String> = matrix.routings.iter().map(serialize_mod_routing).collect();
    format!(
        r#"{{"lfo_configs":[{lfos}],"mod_envelopes":[{envs}],"routings":[{routings}]}}"#,
        lfos = lfos.join(","),
        envs = envs.join(","),
        routings = routings.join(","),
    )
}

fn serialize_tags(tags: &[String]) -> String {
    let items: Vec<String> = tags.iter().map(|t| json_string(t)).collect();
    format!("[{}]", items.join(","))
}

fn serialize_metadata(meta: &PresetMetadata) -> String {
    format!(
        r#"{{"name":{name},"author":{author},"category":{category},"created_at":{created_at},"tags":{tags}}}"#,
        name = json_string(&meta.name),
        author = json_string(&meta.author),
        category = json_string(&meta.category),
        created_at = json_string(&meta.created_at),
        tags = serialize_tags(&meta.tags),
    )
}

fn serialize_sample_player(cfg: &SamplePlayerConfig) -> String {
    format!(
        r#"{{"sample_set_id":{id},"interpolation":{interp},"loop_mode":{lm}}}"#,
        id = cfg.sample_set_id.get(),
        interp = json_string(serialize_interpolation_mode(cfg.interpolation)),
        lm = json_string(serialize_loop_mode(cfg.loop_mode)),
    )
}

fn serialize_preset_to_json(preset: &Preset) -> String {
    let osc = &preset.oscillator;
    let env = &preset.amp_envelope;
    let flt = &preset.filter;
    let matrix_json = serialize_mod_matrix(&preset.mod_matrix);
    let chain_json = serialize_effect_chain(&preset.effect_chain);
    let sample_player_json = match &preset.sample_player {
        Some(sp) => serialize_sample_player(sp),
        None => "null".to_owned(),
    };
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
        c = flt.cutoff.hz(),
        ft = json_string(serialize_filter_type(flt.filter_type)),
        res = flt.resonance(),
    );
    format!(
        r#"{{"id":{id},"metadata":{metadata},"engine_type":{et},"oscillator":{osc},"amp_envelope":{env},"filter":{flt},"sample_player":{sp},"mod_matrix":{matrix},"effect_chain":{chain}}}"#,
        id = json_string(preset.id.as_str()),
        metadata = serialize_metadata(&preset.metadata),
        et = json_string(serialize_engine_type(preset.engine_type)),
        osc = osc_json,
        env = env_json,
        flt = flt_json,
        sp = sample_player_json,
        matrix = matrix_json,
        chain = chain_json,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Preset JSON deserialization (hand-rolled)
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the raw value string for the first occurrence of `key` in a JSON object.
fn json_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":");
    let start = json.find(needle.as_str())? + needle.len();
    let rest = json[start..].trim_start();
    let end = find_json_value_end(rest)?;
    Some(&rest[..end])
}

/// Find the index after the end of the JSON value starting at position 0 of `s`.
fn find_json_value_end(s: &str) -> Option<usize> {
    let s = s.trim_start();
    let first = s.chars().next()?;
    match first {
        '"' => {
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
        'n' if s.starts_with("null") => Some(4),
        _ => {
            // Number, bool: scan until delimiter.
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

fn parse_json_array<T, F: Fn(&str) -> Option<T>>(json: &str, parse_item: F) -> Vec<T> {
    let json = json.trim();
    if !json.starts_with('[') || !json.ends_with(']') {
        return Vec::new();
    }
    let inner = json[1..json.len() - 1].trim();
    if inner.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();
    let mut rest = inner;
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
                let after = rest_trimmed[end..].trim_start();
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

fn parse_engine_type(s: &str) -> Option<EngineType> {
    match s {
        "Oscillator" => Some(EngineType::Oscillator),
        "SamplePlayer" => Some(EngineType::SamplePlayer),
        _ => None,
    }
}

fn parse_interpolation_mode(s: &str) -> Option<InterpolationMode> {
    match s {
        "NearestNeighbour" => Some(InterpolationMode::NearestNeighbour),
        "Linear" => Some(InterpolationMode::Linear),
        "Cubic" => Some(InterpolationMode::Cubic),
        _ => None,
    }
}

fn parse_loop_mode(s: &str) -> Option<LoopMode> {
    match s {
        "None" => Some(LoopMode::None),
        "Sustain" => Some(LoopMode::Sustain),
        "SustainRelease" => Some(LoopMode::SustainRelease),
        "PingPong" => Some(LoopMode::PingPong),
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

fn parse_effect_slot(json: &str) -> Option<SerializedEffectSlot> {
    let et = parse_effect_type(&unquote(json_field(json, "effect_type")?)?)?;
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

fn parse_lfo_config(json: &str) -> Option<(usize, crate::modulation::lfo_config::LfoConfig)> {
    use crate::modulation::lfo_config::LfoConfig;
    let index = json_field(json, "index")?.trim().parse::<usize>().ok()?;
    let rate = parse_f64(json_field(json, "rate")?)?;
    let depth = parse_f64(json_field(json, "depth")?)?;
    let phase = parse_f64(json_field(json, "phase")?)?;
    let sync_to_tempo = parse_bool(json_field(json, "sync_to_tempo")?)?;
    let waveform = parse_lfo_waveform(&unquote(json_field(json, "waveform")?)?)?;
    let cfg = LfoConfig::try_new(rate, depth, phase, sync_to_tempo, waveform).ok()?;
    Some((index, cfg))
}

fn parse_mod_envelope(
    json: &str,
) -> Option<(
    usize,
    crate::modulation::mod_envelope_config::ModEnvelopeConfig,
)> {
    use crate::modulation::mod_envelope_config::ModEnvelopeConfig;
    let index = json_field(json, "index")?.trim().parse::<usize>().ok()?;
    let attack = parse_f64(json_field(json, "attack")?)?;
    let decay = parse_f64(json_field(json, "decay")?)?;
    let sustain = parse_f64(json_field(json, "sustain")?)?;
    let release = parse_f64(json_field(json, "release")?)?;
    let cfg = ModEnvelopeConfig::try_new(attack, decay, sustain, release).ok()?;
    Some((index, cfg))
}

fn parse_mod_matrix(json: &str) -> Option<SerializedModMatrix> {
    let lfos_raw = json_field(json, "lfo_configs")?;
    let envs_raw = json_field(json, "mod_envelopes")?;
    let routings_raw = json_field(json, "routings")?;

    let lfo_pairs = parse_json_array(lfos_raw, parse_lfo_config);
    let env_pairs = parse_json_array(envs_raw, parse_mod_envelope);
    let routings = parse_json_array(routings_raw, parse_mod_routing);

    // Rebuild indexed vecs (fill gaps with defaults if indices are non-contiguous).
    let max_lfo = lfo_pairs
        .iter()
        .map(|(i, _)| *i)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);
    let mut lfo_configs = vec![crate::modulation::lfo_config::LfoConfig::default(); max_lfo];
    for (i, cfg) in lfo_pairs {
        if i < lfo_configs.len() {
            lfo_configs[i] = cfg;
        }
    }

    let max_env = env_pairs
        .iter()
        .map(|(i, _)| *i)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);
    let mut mod_envelopes =
        vec![crate::modulation::mod_envelope_config::ModEnvelopeConfig::default(); max_env];
    for (i, cfg) in env_pairs {
        if i < mod_envelopes.len() {
            mod_envelopes[i] = cfg;
        }
    }

    Some(SerializedModMatrix {
        lfo_configs,
        mod_envelopes,
        routings,
    })
}

fn parse_tags(json: &str) -> Vec<String> {
    parse_json_array(json, unquote)
}

fn parse_metadata(json: &str) -> Option<PresetMetadata> {
    let name = unquote(json_field(json, "name")?)?;
    let author = unquote(json_field(json, "author")?)?;
    let category = unquote(json_field(json, "category")?)?;
    let created_at = unquote(json_field(json, "created_at")?)?;
    let tags_raw = json_field(json, "tags")?;
    let tags = parse_tags(tags_raw);
    Some(PresetMetadata {
        name,
        author,
        category,
        created_at,
        tags,
    })
}

fn parse_oscillator_config(json: &str) -> Option<OscillatorConfig> {
    let detune = parse_f64(json_field(json, "detune")?)?;
    let pulse_width = parse_f64(json_field(json, "pulse_width")?)?;
    let waveform = parse_waveform(&unquote(json_field(json, "waveform")?)?)?;
    OscillatorConfig::try_new(detune, pulse_width, waveform).ok()
}

fn parse_amp_envelope(json: &str) -> Option<AmpEnvelopeConfig> {
    let attack = parse_f64(json_field(json, "attack")?)?;
    let decay = parse_f64(json_field(json, "decay")?)?;
    let sustain = parse_f64(json_field(json, "sustain")?)?;
    let release = parse_f64(json_field(json, "release")?)?;
    AmpEnvelopeConfig::try_new(attack, decay, sustain, release).ok()
}

fn parse_filter_config(json: &str) -> Option<FilterConfig> {
    let cutoff_hz = parse_f64(json_field(json, "cutoff_hz")?)?;
    let filter_type = parse_filter_type(&unquote(json_field(json, "filter_type")?)?)?;
    let resonance = parse_f64(json_field(json, "resonance")?)?;
    FilterConfig::try_new(cutoff_hz, filter_type, resonance).ok()
}

fn parse_sample_player(json: &str) -> Option<SamplePlayerConfig> {
    if json.trim() == "null" {
        return None;
    }
    let id = parse_u32(json_field(json, "sample_set_id")?)?;
    let interp = parse_interpolation_mode(&unquote(json_field(json, "interpolation")?)?)?;
    let lm = parse_loop_mode(&unquote(json_field(json, "loop_mode")?)?)?;
    SamplePlayerConfig::try_new(id, interp, lm).ok()
}

fn parse_preset_json(json: &str) -> Result<Preset, CodecError> {
    let id_raw = json_field(json, "id")
        .and_then(unquote)
        .ok_or_else(|| CodecError::InvalidJson("missing field 'id'".to_owned()))?;
    let metadata_raw = json_field(json, "metadata")
        .ok_or_else(|| CodecError::InvalidJson("missing field 'metadata'".to_owned()))?;
    let metadata = parse_metadata(metadata_raw)
        .ok_or_else(|| CodecError::InvalidJson("malformed 'metadata'".to_owned()))?;
    let et_raw = json_field(json, "engine_type")
        .and_then(unquote)
        .ok_or_else(|| CodecError::InvalidJson("missing field 'engine_type'".to_owned()))?;
    let engine_type = parse_engine_type(&et_raw)
        .ok_or_else(|| CodecError::InvalidData(format!("unknown engine_type: {et_raw}")))?;
    let osc_raw = json_field(json, "oscillator")
        .ok_or_else(|| CodecError::InvalidJson("missing field 'oscillator'".to_owned()))?;
    let oscillator = parse_oscillator_config(osc_raw)
        .ok_or_else(|| CodecError::InvalidData("malformed 'oscillator'".to_owned()))?;
    let env_raw = json_field(json, "amp_envelope")
        .ok_or_else(|| CodecError::InvalidJson("missing field 'amp_envelope'".to_owned()))?;
    let amp_envelope = parse_amp_envelope(env_raw)
        .ok_or_else(|| CodecError::InvalidData("malformed 'amp_envelope'".to_owned()))?;
    let flt_raw = json_field(json, "filter")
        .ok_or_else(|| CodecError::InvalidJson("missing field 'filter'".to_owned()))?;
    let filter = parse_filter_config(flt_raw)
        .ok_or_else(|| CodecError::InvalidData("malformed 'filter'".to_owned()))?;
    let sp_raw = json_field(json, "sample_player")
        .ok_or_else(|| CodecError::InvalidJson("missing field 'sample_player'".to_owned()))?;
    let sample_player = parse_sample_player(sp_raw);
    let matrix_raw = json_field(json, "mod_matrix")
        .ok_or_else(|| CodecError::InvalidJson("missing field 'mod_matrix'".to_owned()))?;
    let mod_matrix = parse_mod_matrix(matrix_raw)
        .ok_or_else(|| CodecError::InvalidJson("malformed 'mod_matrix'".to_owned()))?;
    let chain_raw = json_field(json, "effect_chain")
        .ok_or_else(|| CodecError::InvalidJson("missing field 'effect_chain'".to_owned()))?;
    let effect_chain = parse_effect_chain(chain_raw)
        .ok_or_else(|| CodecError::InvalidJson("malformed 'effect_chain'".to_owned()))?;

    Ok(Preset::new(
        PresetId::new(id_raw),
        metadata,
        engine_type,
        oscillator,
        amp_envelope,
        filter,
        sample_player,
        mod_matrix,
        effect_chain,
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modulation::mod_destination_type::ModDestinationType;
    use crate::modulation::mod_source_type::ModSourceType;
    use crate::presets::preset::{SerializedModMatrix, SerializedModRouting};
    use crate::presets::setup::{
        build_serialized_patch, SerializedEffectChain as SetupChain, SerializedEngineType,
        SerializedModMatrix as SetupMod, SerializedPatch, Setup,
    };
    use crate::synth::filter_config::FilterType;
    use crate::synth::oscillator_config::Waveform;

    // ── Helpers ───────────────────────────────────────────────────────────────────────

    fn default_preset(id: &str) -> Preset {
        Preset::default_for(id, "Test Preset")
    }

    fn make_setup_patch(id: u32, name: &str) -> SerializedPatch {
        use crate::kernel::amplitude::Amplitude;
        use crate::kernel::midi_channel::MidiChannel;
        use crate::kernel::midi_group::MidiGroup;
        use crate::patch::channel_subscription::{ChannelAddress, ChannelSubscription};
        use crate::patch::voice_pool_config::VoicePoolConfig;

        let group = MidiGroup::try_new(0).unwrap();
        let channel = MidiChannel::try_new(0).unwrap();
        let address = ChannelAddress::new(group, channel);
        let sub = ChannelSubscription::new(address, None);
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
            sub,
            VoicePoolConfig::default(),
            SetupMod::default(),
            SetupChain::default(),
        )
    }

    // ── CodecError ───────────────────────────────────────────────────────────────────────

    #[test]
    fn codec_error_invalid_json_display() {
        let e = CodecError::InvalidJson("unexpected eof".to_string());
        assert!(e.to_string().contains("invalid JSON"));
        assert!(e.to_string().contains("unexpected eof"));
    }

    #[test]
    fn codec_error_invalid_data_display() {
        let e = CodecError::InvalidData("gain must be non-negative".to_string());
        assert!(e.to_string().contains("invalid data"));
    }

    // ── Preset round-trip ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn preset_serialize_deserialize_round_trip() {
        let codec = PresetCodec::new();
        let original = default_preset("bright-pad");
        let bytes = codec.serialize(original.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn preset_serialized_bytes_are_valid_utf8() {
        let codec = PresetCodec::new();
        let bytes = codec.serialize(default_preset("test"));
        assert!(
            std::str::from_utf8(&bytes).is_ok(),
            "output must be valid UTF-8 JSON"
        );
    }

    #[test]
    fn preset_json_contains_name_field() {
        let codec = PresetCodec::new();
        let mut preset = default_preset("test");
        preset.metadata.name = "Warm Bass".to_string();
        let bytes = codec.serialize(preset);
        let json = std::str::from_utf8(&bytes).unwrap();
        assert!(
            json.contains("Warm Bass"),
            "serialised JSON must contain the name"
        );
    }

    #[test]
    fn deserialize_invalid_bytes_returns_error() {
        let codec = PresetCodec::new();
        let result = codec.deserialize(b"not-json".to_vec());
        assert!(matches!(result, Err(CodecError::InvalidJson(_))));
    }

    #[test]
    fn deserialize_empty_bytes_returns_error() {
        let codec = PresetCodec::new();
        let result = codec.deserialize(vec![]);
        assert!(matches!(result, Err(CodecError::InvalidJson(_))));
    }

    // ── Preset captures complete patch state ──────────────────────────────────────────────────────

    #[test]
    fn preset_captures_oscillator_config() {
        let codec = PresetCodec::new();
        let mut preset = default_preset("osc-test");
        preset.oscillator = OscillatorConfig::try_new(25.0, 0.3, Waveform::Saw).unwrap();
        let bytes = codec.serialize(preset.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(restored.oscillator, preset.oscillator);
    }

    #[test]
    fn preset_captures_amp_envelope() {
        let codec = PresetCodec::new();
        let mut preset = default_preset("env-test");
        preset.amp_envelope = AmpEnvelopeConfig::try_new(0.5, 0.2, 0.6, 1.0).unwrap();
        let bytes = codec.serialize(preset.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(restored.amp_envelope, preset.amp_envelope);
    }

    #[test]
    fn preset_captures_filter_config() {
        let codec = PresetCodec::new();
        let mut preset = default_preset("filter-test");
        preset.filter = FilterConfig::try_new(3_000.0, FilterType::HighPass, 0.7).unwrap();
        let bytes = codec.serialize(preset.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(restored.filter, preset.filter);
    }

    #[test]
    fn preset_captures_engine_type() {
        let codec = PresetCodec::new();
        let mut preset = default_preset("engine-test");
        preset.engine_type = EngineType::SamplePlayer;
        let bytes = codec.serialize(preset.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(restored.engine_type, EngineType::SamplePlayer);
    }

    #[test]
    fn preset_captures_modulation_matrix() {
        let codec = PresetCodec::new();
        let routing = SerializedModRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.5,
        };
        let mut preset = default_preset("mod-test");
        preset.mod_matrix = SerializedModMatrix {
            lfo_configs: Vec::new(),
            mod_envelopes: Vec::new(),
            routings: vec![routing],
        };
        let bytes = codec.serialize(preset.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(restored.mod_matrix.routings.len(), 1);
        assert!(matches!(
            restored.mod_matrix.routings[0].source,
            ModSourceType::Lfo
        ));
        assert!(matches!(
            restored.mod_matrix.routings[0].destination,
            ModDestinationType::FilterCutoff
        ));
        assert!((restored.mod_matrix.routings[0].depth - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn preset_captures_effect_chain() {
        let codec = PresetCodec::new();
        let slot = SerializedEffectSlot {
            effect_type: EffectType::LowPassFilter,
            params: EffectParams {
                effect_type: EffectType::LowPassFilter,
                cutoff_hz: 2_000.0,
                ..EffectParams::default()
            },
            bypass: false,
        };
        let mut preset = default_preset("fx-test");
        preset.effect_chain = SerializedEffectChain {
            bypass: false,
            slots: vec![slot],
        };
        let bytes = codec.serialize(preset.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(restored.effect_chain.slots.len(), 1);
        assert!(matches!(
            restored.effect_chain.slots[0].effect_type,
            EffectType::LowPassFilter
        ));
        assert!((restored.effect_chain.slots[0].params.cutoff_hz - 2_000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn preset_captures_metadata_tags() {
        let codec = PresetCodec::new();
        let mut preset = default_preset("tag-test");
        preset.metadata = PresetMetadata::new(
            "Warm Pad",
            "Alice",
            "Pad",
            "2025-01-01T00:00:00Z",
            vec!["warm".to_string(), "ambient".to_string()],
        );
        let bytes = codec.serialize(preset.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(restored.metadata.name, "Warm Pad");
        assert_eq!(restored.metadata.author, "Alice");
        assert_eq!(restored.metadata.tags, vec!["warm", "ambient"]);
    }

    #[test]
    fn preset_captures_per_note_expression_sources() {
        // Validates: MPE expression dimensions exist as named per-voice mod sources.
        let codec = PresetCodec::new();
        let routing = SerializedModRouting {
            source: ModSourceType::PerNoteBendX,
            destination: ModDestinationType::OscillatorPitch,
            depth: 0.8,
        };
        let mut preset = default_preset("mpe-test");
        preset.mod_matrix = SerializedModMatrix {
            lfo_configs: Vec::new(),
            mod_envelopes: Vec::new(),
            routings: vec![routing],
        };
        let bytes = codec.serialize(preset.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert!(matches!(
            restored.mod_matrix.routings[0].source,
            ModSourceType::PerNoteBendX
        ));
    }

    // ── Setup round-trip ────────────────────────────────────────────────────────────────────────

    #[test]
    fn setup_serialize_deserialize_round_trip() {
        let codec = PresetCodec::new();
        let mut setup = Setup::new("My Session");
        setup.patches.push(make_setup_patch(1, "Lead"));
        setup.patches.push(make_setup_patch(2, "Pad"));
        setup.master_gain = 0.9;
        let bytes = codec.serialize_setup(setup.clone());
        let restored = codec.deserialize_setup(bytes).unwrap();
        assert_eq!(restored.name, "My Session");
        assert_eq!(restored.patches.len(), 2);
        assert!((restored.master_gain - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn setup_captures_all_patches() {
        let codec = PresetCodec::new();
        let mut setup = Setup::new("Test");
        setup.patches.push(make_setup_patch(1, "Lead"));
        setup.patches.push(make_setup_patch(2, "Bass"));
        let bytes = codec.serialize_setup(setup);
        let restored = codec.deserialize_setup(bytes).unwrap();
        assert_eq!(restored.patches.len(), 2);
        assert_eq!(restored.patches[0].name, "Lead");
        assert_eq!(restored.patches[1].name, "Bass");
    }

    #[test]
    fn setup_captures_master_gain() {
        let codec = PresetCodec::new();
        let mut setup = Setup::new("Test");
        setup.master_gain = 0.75;
        let bytes = codec.serialize_setup(setup);
        let restored = codec.deserialize_setup(bytes).unwrap();
        assert!((restored.master_gain - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn deserialize_setup_invalid_bytes_returns_error() {
        let codec = PresetCodec::new();
        let result = codec.deserialize_setup(b"not-json".to_vec());
        assert!(matches!(result, Err(CodecError::InvalidJson(_))));
    }

    #[test]
    fn setup_preserves_patch_insertion_order() {
        let codec = PresetCodec::new();
        let mut setup = Setup::new("Order Test");
        for i in 0..5u32 {
            setup
                .patches
                .push(make_setup_patch(i, &format!("Patch {i}")));
        }
        let bytes = codec.serialize_setup(setup);
        let restored = codec.deserialize_setup(bytes).unwrap();
        for (i, patch) in restored.patches.iter().enumerate() {
            assert_eq!(patch.name, format!("Patch {i}"));
        }
    }

    // ── Default ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn preset_codec_default_creates_instance() {
        let _codec = PresetCodec;
    }
}
