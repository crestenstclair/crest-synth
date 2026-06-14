// path: src/synth/sample_player_config.rs

/// Opaque identifier for a loaded sample set.
///
/// A sample set groups all the samples (one per pitch/velocity zone) that a
/// sample player voice can draw from.  The identifier is cheap to copy and
/// compare on the audio thread; the actual audio data lives behind an `Arc`
/// managed by `DeferredDeallocator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SampleSetId(u32);

/// Error returned when a `SampleSetId` is constructed with an invalid value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleSetIdError;

impl std::fmt::Display for SampleSetIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SampleSetId value 0 is reserved; use a non-zero id")
    }
}

impl std::error::Error for SampleSetIdError {}

impl SampleSetId {
    /// Construct a `SampleSetId` from a raw `u32`.
    ///
    /// Returns `Err` if `id` is `0` (reserved value).
    ///
    /// ```
    /// use crest_synth::synth::sample_player_config::SampleSetId;
    /// assert!(SampleSetId::try_new(1).is_ok());
    /// assert!(SampleSetId::try_new(0).is_err());
    /// ```
    pub fn try_new(id: u32) -> Result<Self, SampleSetIdError> {
        if id == 0 {
            return Err(SampleSetIdError);
        }
        Ok(Self(id))
    }

    /// Return the raw `u32` identifier.
    #[inline]
    pub fn get(self) -> u32 {
        self.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Interpolation algorithm used when the playback pitch does not align with
/// the sample's natural pitch.
///
/// Higher quality costs more CPU; choose based on voice budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InterpolationMode {
    /// Nearest-neighbour — fastest, lowest quality.
    NearestNeighbour,
    /// Linear interpolation — good balance for most voices.
    #[default]
    Linear,
    /// Cubic (Hermite) interpolation — highest quality, most CPU.
    Cubic,
}

// ─────────────────────────────────────────────────────────────────────────────

/// Looping behaviour for a sample voice after the sustain phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LoopMode {
    /// Do not loop — play the sample once and stop.
    #[default]
    None,
    /// Loop the sustain region continuously while the note is held.
    Sustain,
    /// Loop the sustain region, then release and play to the end on note-off.
    SustainRelease,
    /// Ping-pong loop: alternate forward and backward through the loop region.
    PingPong,
}

// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for a sample-player engine: which sample set to use,
/// interpolation quality, and looping behaviour.
///
/// All fields are `Copy`-able value types — this struct is cheap to clone and
/// safe to snapshot for the audio thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplePlayerConfig {
    /// Which sample set the player should draw from.
    pub sample_set_id: SampleSetId,
    /// Interpolation algorithm applied when resampling to the target pitch.
    pub interpolation: InterpolationMode,
    /// Looping behaviour during playback.
    pub loop_mode: LoopMode,
}

impl SamplePlayerConfig {
    /// Construct a `SamplePlayerConfig`.
    ///
    /// Returns `Err` if `sample_set_id` is invalid (see [`SampleSetId::try_new`]).
    ///
    /// ```
    /// use crest_synth::synth::sample_player_config::{
    ///     InterpolationMode, LoopMode, SamplePlayerConfig,
    /// };
    /// let cfg = SamplePlayerConfig::try_new(1, InterpolationMode::Linear, LoopMode::Sustain)
    ///     .unwrap();
    /// assert_eq!(cfg.interpolation, InterpolationMode::Linear);
    /// assert_eq!(cfg.loop_mode, LoopMode::Sustain);
    /// ```
    pub fn try_new(
        sample_set_id: u32,
        interpolation: InterpolationMode,
        loop_mode: LoopMode,
    ) -> Result<Self, SampleSetIdError> {
        Ok(Self {
            sample_set_id: SampleSetId::try_new(sample_set_id)?,
            interpolation,
            loop_mode,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SampleSetId ──────────────────────────────────────────────────────────

    #[test]
    fn sample_set_id_nonzero_accepted() {
        assert!(SampleSetId::try_new(1).is_ok());
        assert!(SampleSetId::try_new(u32::MAX).is_ok());
    }

    #[test]
    fn sample_set_id_zero_rejected() {
        assert!(SampleSetId::try_new(0).is_err());
    }

    #[test]
    fn sample_set_id_get_roundtrips() {
        let id = SampleSetId::try_new(42).unwrap();
        assert_eq!(id.get(), 42);
    }

    #[test]
    fn sample_set_id_copy_semantics() {
        let a = SampleSetId::try_new(7).unwrap();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn sample_set_id_error_display() {
        let err = SampleSetIdError;
        assert!(err.to_string().contains("reserved"));
    }

    // ── InterpolationMode ────────────────────────────────────────────────────

    #[test]
    fn interpolation_mode_default_is_linear() {
        assert_eq!(InterpolationMode::default(), InterpolationMode::Linear);
    }

    #[test]
    fn interpolation_mode_variants_are_copy() {
        let a = InterpolationMode::Cubic;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn interpolation_mode_all_variants_debug() {
        for mode in [
            InterpolationMode::NearestNeighbour,
            InterpolationMode::Linear,
            InterpolationMode::Cubic,
        ] {
            let s = format!("{:?}", mode);
            assert!(!s.is_empty());
        }
    }

    // ── LoopMode ─────────────────────────────────────────────────────────────

    #[test]
    fn loop_mode_default_is_none() {
        assert_eq!(LoopMode::default(), LoopMode::None);
    }

    #[test]
    fn loop_mode_variants_are_copy() {
        let a = LoopMode::PingPong;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn loop_mode_all_variants_debug() {
        for mode in [
            LoopMode::None,
            LoopMode::Sustain,
            LoopMode::SustainRelease,
            LoopMode::PingPong,
        ] {
            let s = format!("{:?}", mode);
            assert!(!s.is_empty());
        }
    }

    // ── SamplePlayerConfig ───────────────────────────────────────────────────

    #[test]
    fn sample_player_config_valid_construction() {
        let cfg =
            SamplePlayerConfig::try_new(1, InterpolationMode::Linear, LoopMode::Sustain).unwrap();
        assert_eq!(cfg.sample_set_id.get(), 1);
        assert_eq!(cfg.interpolation, InterpolationMode::Linear);
        assert_eq!(cfg.loop_mode, LoopMode::Sustain);
    }

    #[test]
    fn sample_player_config_zero_id_rejected() {
        assert!(SamplePlayerConfig::try_new(0, InterpolationMode::Linear, LoopMode::None).is_err());
    }

    #[test]
    fn sample_player_config_all_interpolation_modes() {
        for interp in [
            InterpolationMode::NearestNeighbour,
            InterpolationMode::Linear,
            InterpolationMode::Cubic,
        ] {
            assert!(SamplePlayerConfig::try_new(1, interp, LoopMode::None).is_ok());
        }
    }

    #[test]
    fn sample_player_config_all_loop_modes() {
        for lm in [
            LoopMode::None,
            LoopMode::Sustain,
            LoopMode::SustainRelease,
            LoopMode::PingPong,
        ] {
            assert!(SamplePlayerConfig::try_new(1, InterpolationMode::Linear, lm).is_ok());
        }
    }

    #[test]
    fn sample_player_config_copy_semantics() {
        let a =
            SamplePlayerConfig::try_new(5, InterpolationMode::Cubic, LoopMode::PingPong).unwrap();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn sample_player_config_debug_output() {
        let cfg =
            SamplePlayerConfig::try_new(3, InterpolationMode::Cubic, LoopMode::SustainRelease)
                .unwrap();
        let s = format!("{:?}", cfg);
        assert!(s.contains("SamplePlayerConfig"));
    }

    #[test]
    fn sample_player_config_id_max_u32_accepted() {
        assert!(
            SamplePlayerConfig::try_new(u32::MAX, InterpolationMode::Linear, LoopMode::None)
                .is_ok()
        );
    }
}
