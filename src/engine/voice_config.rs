// path: src/engine/voice_config.rs

//! Per-patch voice configuration: oscillator, envelopes, filter, and
//! polyphony/voice-stealing behavior for a single patch.
//!
//! `VoiceConfig` is a pure value object -- validated on construction,
//! immutable thereafter. It does not touch the audio thread directly;
//! runtime engines consume it via ParameterBridge snapshots.

use std::fmt;

/// Number of concurrently sounding voices a patch may use.
///
/// Must be positive; conventionally between 8 and 64, though the type
/// itself only enforces `> 0` (the invariant is silent on an upper bound).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaxPolyphony(u8);

impl MaxPolyphony {
    pub fn try_new(value: u8) -> Result<Self, VoiceConfigError> {
        if value == 0 {
            return Err(VoiceConfigError::NonPositiveMaxPolyphony);
        }
        Ok(Self(value))
    }

    pub fn get(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for MaxPolyphony {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Sound-generation strategy a patch's voices use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineType {
    Subtractive,
    Fm,
    Wavetable,
    Sample,
}

/// Behavior applied when a note-on arrives and every voice slot is busy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StealPolicy {
    /// Steal the voice that has been sounding the longest.
    Oldest,
    /// Steal the voice with the lowest pitch.
    Lowest,
    /// Steal the voice with the highest pitch.
    Highest,
    /// Steal the voice with the smallest current amplitude.
    Quietest,
}

/// Filter type applied to the oscillator/sample signal before envelopes
/// shape it and it is summed into the channel strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterKind {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// Normalized cutoff frequency, 0.0 (fully closed) to 1.0 (fully open),
/// typically mapped to a Hz range by the runtime filter implementation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cutoff(f32);

impl Cutoff {
    pub fn try_new(value: f32) -> Result<Self, VoiceConfigError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(VoiceConfigError::CutoffOutOfRange(value));
        }
        Ok(Self(value))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

/// Normalized resonance amount, 0.0 (no emphasis) to 1.0 (near self-oscillation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resonance(f32);

impl Resonance {
    pub fn try_new(value: f32) -> Result<Self, VoiceConfigError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(VoiceConfigError::ResonanceOutOfRange(value));
        }
        Ok(Self(value))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

/// Filter stage configuration for a patch's voices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterConfig {
    kind: FilterKind,
    cutoff: Cutoff,
    resonance: Resonance,
}

impl FilterConfig {
    pub fn new(kind: FilterKind, cutoff: Cutoff, resonance: Resonance) -> Self {
        Self {
            kind,
            cutoff,
            resonance,
        }
    }

    pub fn kind(&self) -> FilterKind {
        self.kind
    }

    pub fn cutoff(&self) -> Cutoff {
        self.cutoff
    }

    pub fn resonance(&self) -> Resonance {
        self.resonance
    }
}

/// Oscillator waveform shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

/// Oscillator detune, in semitones, applied before mixing into the voice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detune(f32);

impl Detune {
    pub fn try_new(value: f32) -> Result<Self, VoiceConfigError> {
        if value.is_nan() || !(-24.0..=24.0).contains(&value) {
            return Err(VoiceConfigError::DetuneOutOfRange(value));
        }
        Ok(Self(value))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

/// Oscillator output level, 0.0 (silent) to 1.0 (unity).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscillatorLevel(f32);

impl OscillatorLevel {
    pub fn try_new(value: f32) -> Result<Self, VoiceConfigError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(VoiceConfigError::OscillatorLevelOutOfRange(value));
        }
        Ok(Self(value))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

/// Oscillator stage configuration for a patch's voices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscillatorConfig {
    waveform: Waveform,
    detune: Detune,
    level: OscillatorLevel,
}

impl OscillatorConfig {
    pub fn new(waveform: Waveform, detune: Detune, level: OscillatorLevel) -> Self {
        Self {
            waveform,
            detune,
            level,
        }
    }

    pub fn waveform(&self) -> Waveform {
        self.waveform
    }

    pub fn detune(&self) -> Detune {
        self.detune
    }

    pub fn level(&self) -> OscillatorLevel {
        self.level
    }
}

/// A non-negative envelope stage duration, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopeTime(f32);

impl EnvelopeTime {
    pub fn try_new(value: f32) -> Result<Self, VoiceConfigError> {
        if value.is_nan() || value.is_sign_negative() {
            return Err(VoiceConfigError::NegativeEnvelopeTime(value));
        }
        Ok(Self(value))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

/// Sustain level, 0.0 (silent) to 1.0 (full amplitude).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SustainLevel(f32);

impl SustainLevel {
    pub fn try_new(value: f32) -> Result<Self, VoiceConfigError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(VoiceConfigError::SustainOutOfRange(value));
        }
        Ok(Self(value))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

/// A four-stage attack/decay/sustain/release envelope shape shared by the
/// amplitude, filter, and pitch envelopes of a voice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopeConfig {
    attack: EnvelopeTime,
    decay: EnvelopeTime,
    sustain: SustainLevel,
    release: EnvelopeTime,
}

impl EnvelopeConfig {
    pub fn new(
        attack: EnvelopeTime,
        decay: EnvelopeTime,
        sustain: SustainLevel,
        release: EnvelopeTime,
    ) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
        }
    }

    pub fn attack(&self) -> EnvelopeTime {
        self.attack
    }

    pub fn decay(&self) -> EnvelopeTime {
        self.decay
    }

    pub fn sustain(&self) -> SustainLevel {
        self.sustain
    }

    pub fn release(&self) -> EnvelopeTime {
        self.release
    }
}

/// Errors constructing a `VoiceConfig` or any of its constituent value types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoiceConfigError {
    NonPositiveMaxPolyphony,
    CutoffOutOfRange(f32),
    ResonanceOutOfRange(f32),
    DetuneOutOfRange(f32),
    OscillatorLevelOutOfRange(f32),
    NegativeEnvelopeTime(f32),
    SustainOutOfRange(f32),
}

impl fmt::Display for VoiceConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveMaxPolyphony => write!(f, "maxPolyphony must be positive"),
            Self::CutoffOutOfRange(v) => write!(f, "filter cutoff {v} is out of range [0.0, 1.0]"),
            Self::ResonanceOutOfRange(v) => {
                write!(f, "filter resonance {v} is out of range [0.0, 1.0]")
            }
            Self::DetuneOutOfRange(v) => {
                write!(f, "oscillator detune {v} is out of range [-24.0, 24.0]")
            }
            Self::OscillatorLevelOutOfRange(v) => {
                write!(f, "oscillator level {v} is out of range [0.0, 1.0]")
            }
            Self::NegativeEnvelopeTime(v) => write!(f, "envelope time {v} must be non-negative"),
            Self::SustainOutOfRange(v) => {
                write!(f, "envelope sustain {v} is out of range [0.0, 1.0]")
            }
        }
    }
}

impl std::error::Error for VoiceConfigError {}

/// Complete per-patch voice configuration: oscillator, envelopes, filter,
/// and polyphony/voice-stealing behavior.
///
/// A `VoiceConfig` is validated on construction and immutable thereafter;
/// there is no in-place mutation path, so every field of every constituent
/// value type must already be valid before the whole config exists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceConfig {
    amp_envelope: EnvelopeConfig,
    engine_type: EngineType,
    filter: FilterConfig,
    filter_envelope: EnvelopeConfig,
    max_polyphony: MaxPolyphony,
    oscillator: OscillatorConfig,
    pitch_envelope: EnvelopeConfig,
    steal_policy: StealPolicy,
}

impl VoiceConfig {
    /// Construct a `VoiceConfig` from already-validated constituent parts.
    ///
    /// `max_polyphony` is a plain `u8` here (not `MaxPolyphony`) so callers
    /// building from raw preset data get the "must be positive" check
    /// enforced at this single seam; every other field type already
    /// enforces its own invariants at its own construction site.
    ///
    /// Takes eight parameters by design -- one per `VoiceConfig` field --
    /// rather than an arbitrary grouping that would only exist to dodge a
    /// lint; the allow below documents that this is a deliberate,
    /// reviewed exception to the argument-count lint.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        amp_envelope: EnvelopeConfig,
        engine_type: EngineType,
        filter: FilterConfig,
        filter_envelope: EnvelopeConfig,
        max_polyphony: u8,
        oscillator: OscillatorConfig,
        pitch_envelope: EnvelopeConfig,
        steal_policy: StealPolicy,
    ) -> Result<Self, VoiceConfigError> {
        let max_polyphony = MaxPolyphony::try_new(max_polyphony)?;
        Ok(Self {
            amp_envelope,
            engine_type,
            filter,
            filter_envelope,
            max_polyphony,
            oscillator,
            pitch_envelope,
            steal_policy,
        })
    }

    pub fn amp_envelope(&self) -> EnvelopeConfig {
        self.amp_envelope
    }

    pub fn engine_type(&self) -> EngineType {
        self.engine_type
    }

    pub fn filter(&self) -> FilterConfig {
        self.filter
    }

    pub fn filter_envelope(&self) -> EnvelopeConfig {
        self.filter_envelope
    }

    pub fn max_polyphony(&self) -> MaxPolyphony {
        self.max_polyphony
    }

    pub fn oscillator(&self) -> OscillatorConfig {
        self.oscillator
    }

    pub fn pitch_envelope(&self) -> EnvelopeConfig {
        self.pitch_envelope
    }

    pub fn steal_policy(&self) -> StealPolicy {
        self.steal_policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_envelope() -> EnvelopeConfig {
        EnvelopeConfig::new(
            EnvelopeTime::try_new(0.01).unwrap(),
            EnvelopeTime::try_new(0.1).unwrap(),
            SustainLevel::try_new(0.7).unwrap(),
            EnvelopeTime::try_new(0.2).unwrap(),
        )
    }

    fn sample_filter() -> FilterConfig {
        FilterConfig::new(
            FilterKind::LowPass,
            Cutoff::try_new(0.5).unwrap(),
            Resonance::try_new(0.2).unwrap(),
        )
    }

    fn sample_oscillator() -> OscillatorConfig {
        OscillatorConfig::new(
            Waveform::Saw,
            Detune::try_new(0.0).unwrap(),
            OscillatorLevel::try_new(1.0).unwrap(),
        )
    }

    #[test]
    fn accepts_positive_max_polyphony() {
        let config = VoiceConfig::try_new(
            sample_envelope(),
            EngineType::Subtractive,
            sample_filter(),
            sample_envelope(),
            16,
            sample_oscillator(),
            sample_envelope(),
            StealPolicy::Oldest,
        );
        assert!(config.is_ok());
        assert_eq!(config.unwrap().max_polyphony().get(), 16);
    }

    #[test]
    fn rejects_zero_max_polyphony() {
        let config = VoiceConfig::try_new(
            sample_envelope(),
            EngineType::Fm,
            sample_filter(),
            sample_envelope(),
            0,
            sample_oscillator(),
            sample_envelope(),
            StealPolicy::Quietest,
        );
        assert_eq!(config, Err(VoiceConfigError::NonPositiveMaxPolyphony));
    }

    #[test]
    fn max_polyphony_type_rejects_zero_directly() {
        assert_eq!(
            MaxPolyphony::try_new(0),
            Err(VoiceConfigError::NonPositiveMaxPolyphony)
        );
    }

    #[test]
    fn max_polyphony_type_accepts_typical_values() {
        assert!(MaxPolyphony::try_new(8).is_ok());
        assert!(MaxPolyphony::try_new(64).is_ok());
    }

    #[test]
    fn cutoff_rejects_out_of_range() {
        assert!(Cutoff::try_new(1.5).is_err());
        assert!(Cutoff::try_new(-0.1).is_err());
        assert!(Cutoff::try_new(f32::NAN).is_err());
    }

    #[test]
    fn envelope_time_rejects_negative() {
        assert!(EnvelopeTime::try_new(-0.01).is_err());
        assert!(EnvelopeTime::try_new(0.0).is_ok());
    }
}
