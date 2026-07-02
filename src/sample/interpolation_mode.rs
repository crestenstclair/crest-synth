//! Resampling quality mode for sample playback.
//!
//! `InterpolationMode` selects the algorithm used to reconstruct sample
//! values between the recorded sample points when a sample is played back
//! at a rate other than its native rate. It is a plain value type: no
//! allocation, no I/O, no locking — safe to read on the audio thread.

/// Resampling quality: none (nearest neighbor), linear, cubic, or sinc.
///
/// Higher-quality modes (cubic, sinc) cost more CPU per sample but reduce
/// aliasing and interpolation artifacts. `None` is the cheapest and
/// noisiest; `Sinc` is the most expensive and cleanest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpolationMode {
    /// Nearest-neighbor: no interpolation, just picks the closest sample.
    None,
    /// Linear interpolation between the two nearest samples.
    Linear,
    /// Cubic interpolation using four neighboring samples.
    Cubic,
    /// Windowed-sinc interpolation for the highest reconstruction quality.
    Sinc,
}

impl InterpolationMode {
    /// All variants, in ascending order of computational cost.
    pub const ALL: [InterpolationMode; 4] = [
        InterpolationMode::None,
        InterpolationMode::Linear,
        InterpolationMode::Cubic,
        InterpolationMode::Sinc,
    ];

    /// The number of neighboring samples this mode reads to produce one
    /// interpolated output sample.
    pub const fn taps(self) -> usize {
        match self {
            InterpolationMode::None => 1,
            InterpolationMode::Linear => 2,
            InterpolationMode::Cubic => 4,
            InterpolationMode::Sinc => 8,
        }
    }

    /// A short human-readable label for UI display.
    pub const fn label(self) -> &'static str {
        match self {
            InterpolationMode::None => "None",
            InterpolationMode::Linear => "Linear",
            InterpolationMode::Cubic => "Cubic",
            InterpolationMode::Sinc => "Sinc",
        }
    }
}

impl Default for InterpolationMode {
    /// Linear is the default: a reasonable quality/cost tradeoff for
    /// real-time playback.
    fn default() -> Self {
        InterpolationMode::Linear
    }
}

impl std::fmt::Display for InterpolationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_linear() {
        assert_eq!(InterpolationMode::default(), InterpolationMode::Linear);
    }

    #[test]
    fn taps_increase_with_quality() {
        assert_eq!(InterpolationMode::None.taps(), 1);
        assert_eq!(InterpolationMode::Linear.taps(), 2);
        assert_eq!(InterpolationMode::Cubic.taps(), 4);
        assert_eq!(InterpolationMode::Sinc.taps(), 8);
    }

    #[test]
    fn all_contains_every_variant_once() {
        assert_eq!(InterpolationMode::ALL.len(), 4);
        let mut seen = std::collections::HashSet::new();
        for mode in InterpolationMode::ALL {
            assert!(seen.insert(mode), "duplicate variant in ALL: {mode:?}");
        }
    }

    #[test]
    fn label_matches_display() {
        for mode in InterpolationMode::ALL {
            assert_eq!(mode.label(), mode.to_string());
        }
    }

    #[test]
    fn variants_are_distinct() {
        assert_ne!(InterpolationMode::None, InterpolationMode::Linear);
        assert_ne!(InterpolationMode::Linear, InterpolationMode::Cubic);
        assert_ne!(InterpolationMode::Cubic, InterpolationMode::Sinc);
        assert_ne!(InterpolationMode::Sinc, InterpolationMode::None);
    }
}
