//! [`ChorusParams`]: chorus/flanger settings — depth, modulation rate, voice count, and
//! wet/dry mix for a chorus-style effect.

use std::fmt;

/// Error returned when constructing a [`ChorusParams`] with invalid field values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChorusParamsError {
    /// `voiceCount` was zero; a chorus effect needs at least one voice to produce sound.
    VoiceCountNotPositive,
}

impl fmt::Display for ChorusParamsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChorusParamsError::VoiceCountNotPositive => {
                write!(f, "voiceCount must be positive")
            }
        }
    }
}

impl std::error::Error for ChorusParamsError {}

/// Chorus/flanger settings: modulation depth, modulation rate, number of detuned voices,
/// and the wet/dry mix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChorusParams {
    depth: f64,
    rate_hz: f64,
    voice_count: u8,
    wet: f64,
}

impl ChorusParams {
    /// Constructs a `ChorusParams`, validating that `voice_count` is positive.
    ///
    /// # Errors
    ///
    /// Returns [`ChorusParamsError::VoiceCountNotPositive`] if `voice_count` is zero.
    pub fn try_new(
        depth: f64,
        rate_hz: f64,
        voice_count: u8,
        wet: f64,
    ) -> Result<Self, ChorusParamsError> {
        if voice_count == 0 {
            return Err(ChorusParamsError::VoiceCountNotPositive);
        }

        Ok(Self {
            depth,
            rate_hz,
            voice_count,
            wet,
        })
    }

    /// Modulation depth.
    pub fn depth(&self) -> f64 {
        self.depth
    }

    /// Modulation rate, in Hz.
    pub fn rate_hz(&self) -> f64 {
        self.rate_hz
    }

    /// Number of detuned voices. Always positive.
    pub fn voice_count(&self) -> u8 {
        self.voice_count
    }

    /// Wet/dry mix.
    pub fn wet(&self) -> f64 {
        self.wet
    }
}

impl Default for ChorusParams {
    /// A single-voice chorus with no modulation and a fully dry mix.
    fn default() -> Self {
        Self {
            depth: 0.0,
            rate_hz: 0.0,
            voice_count: 1,
            wet: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_positive_voice_count() {
        let params = ChorusParams::try_new(0.5, 1.2, 3, 0.4).unwrap();
        assert_eq!(params.depth(), 0.5);
        assert_eq!(params.rate_hz(), 1.2);
        assert_eq!(params.voice_count(), 3);
        assert_eq!(params.wet(), 0.4);
    }

    #[test]
    fn try_new_rejects_zero_voice_count() {
        let err = ChorusParams::try_new(0.5, 1.2, 0, 0.4).unwrap_err();
        assert_eq!(err, ChorusParamsError::VoiceCountNotPositive);
    }

    #[test]
    fn default_has_single_voice() {
        let params = ChorusParams::default();
        assert_eq!(params.voice_count(), 1);
    }
}
