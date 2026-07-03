// path: src/engine/voice_config.rs

//! Per-patch voice configuration: oscillator, envelopes, filter, engine
//! type, and polyphony/voice-stealing behavior for a single patch.
//!
//! `VoiceConfig` is a pure value object -- validated on construction,
//! immutable thereafter. It does not touch the audio thread directly;
//! runtime engines consume it via ParameterBridge snapshots.
//!
//! `VoiceConfig` composes the Engine context's canonical shared value
//! types (`EngineType`, `EnvelopeConfig`, `FilterConfig`,
//! `OscillatorConfig`, `StealPolicy`) rather than redefining them --
//! those types own their own construction-time validation, so
//! `VoiceConfig::try_new` only enforces the one invariant that is its
//! own concern: `maxPolyphony` must be positive.

use std::fmt;

use crate::engine::engine_type::EngineType;
use crate::engine::envelope_config::EnvelopeConfig;
use crate::engine::filter_config::FilterConfig;
use crate::engine::oscillator_config::OscillatorConfig;
use crate::engine::steal_policy::StealPolicy;

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

/// Errors constructing a `VoiceConfig`.
///
/// Every constituent value type (`EnvelopeConfig`, `EngineType`,
/// `FilterConfig`, `OscillatorConfig`, `StealPolicy`) validates its own
/// fields at its own construction site; `VoiceConfig` only adds the
/// `maxPolyphony` check that is its own concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceConfigError {
    NonPositiveMaxPolyphony,
}

impl fmt::Display for VoiceConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveMaxPolyphony => write!(f, "maxPolyphony must be positive"),
        }
    }
}

impl std::error::Error for VoiceConfigError {}

/// Complete per-patch voice configuration: oscillator, envelopes, filter,
/// engine type, and polyphony/voice-stealing behavior.
///
/// A `VoiceConfig` is validated on construction and immutable thereafter;
/// there is no in-place mutation path, so every field of every constituent
/// value type must already be valid before the whole config exists.
#[derive(Debug, Clone, PartialEq)]
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
    /// enforces its own invariants at its own construction site, and
    /// every one of `EngineType`'s four documented variants is accepted
    /// and stored unchanged.
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
    use crate::engine::filter_config::FilterType;
    use crate::engine::oscillator_config::Waveform;

    fn sample_envelope() -> EnvelopeConfig {
        // (attack, decay, release, sustain)
        EnvelopeConfig::try_new(0.01, 0.1, 0.2, 0.7).expect("valid envelope")
    }

    fn sample_filter() -> FilterConfig {
        // (cutoffHz, drive, envelopeAmount, filterType, keyTracking, resonance)
        FilterConfig::try_new(1_000.0, 0.0, 0.0, FilterType::LowPass, 0.0, 0.2)
            .expect("valid filter")
    }

    fn sample_oscillator() -> OscillatorConfig {
        // (detuneCents, pulseWidth, unisonSpread, unisonVoices, waveform)
        OscillatorConfig::new(0.0, 0.5, 0.0, 1, Waveform::Sine).expect("valid oscillator")
    }

    #[test]
    fn accepts_positive_max_polyphony() {
        let config = VoiceConfig::try_new(
            sample_envelope(),
            EngineType::VirtualAnalog,
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
    fn preserves_all_engine_type_variants_unchanged() {
        let variants = [
            EngineType::VirtualAnalog,
            EngineType::Wavetable,
            EngineType::SamplePlayback,
            EngineType::Fm,
        ];
        for variant in variants {
            let config = VoiceConfig::try_new(
                sample_envelope(),
                variant,
                sample_filter(),
                sample_envelope(),
                16,
                sample_oscillator(),
                sample_envelope(),
                StealPolicy::Oldest,
            )
            .expect("valid config");
            assert_eq!(config.engine_type(), variant);
        }
    }
}
