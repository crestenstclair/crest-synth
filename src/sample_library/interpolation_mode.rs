// path: src/sample_library/interpolation_mode.rs

/// Sample interpolation quality used when reading between discrete sample points.
///
/// Higher-quality modes reduce aliasing artifacts at the cost of more CPU work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InterpolationMode {
    /// No interpolation — use the nearest sample.
    ///
    /// Cheapest option; audible aliasing at low pitch ratios.
    #[default]
    Nearest,

    /// Linear interpolation between adjacent samples.
    ///
    /// Good balance between quality and CPU cost for most use cases.
    Linear,

    /// Cubic (Hermite) interpolation across four surrounding samples.
    ///
    /// Smoother than linear with minimal phase distortion.
    Cubic,

    /// Windowed-sinc (band-limited) interpolation.
    ///
    /// Highest quality; eliminates aliasing at the cost of the most CPU time.
    Sinc,
}

impl InterpolationMode {
    /// Returns all variants in increasing quality order.
    pub fn all() -> &'static [InterpolationMode] {
        &[
            InterpolationMode::Nearest,
            InterpolationMode::Linear,
            InterpolationMode::Cubic,
            InterpolationMode::Sinc,
        ]
    }
}

impl std::fmt::Display for InterpolationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpolationMode::Nearest => write!(f, "Nearest"),
            InterpolationMode::Linear => write!(f, "Linear"),
            InterpolationMode::Cubic => write!(f, "Cubic"),
            InterpolationMode::Sinc => write!(f, "Sinc"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_nearest() {
        assert_eq!(InterpolationMode::default(), InterpolationMode::Nearest);
    }

    #[test]
    fn all_returns_four_variants() {
        assert_eq!(InterpolationMode::all().len(), 4);
    }

    #[test]
    fn all_variants_present() {
        let all = InterpolationMode::all();
        assert!(all.contains(&InterpolationMode::Nearest));
        assert!(all.contains(&InterpolationMode::Linear));
        assert!(all.contains(&InterpolationMode::Cubic));
        assert!(all.contains(&InterpolationMode::Sinc));
    }

    #[test]
    fn display_names() {
        assert_eq!(InterpolationMode::Nearest.to_string(), "Nearest");
        assert_eq!(InterpolationMode::Linear.to_string(), "Linear");
        assert_eq!(InterpolationMode::Cubic.to_string(), "Cubic");
        assert_eq!(InterpolationMode::Sinc.to_string(), "Sinc");
    }

    #[test]
    fn clone_and_copy() {
        let mode = InterpolationMode::Cubic;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn equality() {
        assert_eq!(InterpolationMode::Linear, InterpolationMode::Linear);
        assert_ne!(InterpolationMode::Linear, InterpolationMode::Sinc);
    }
}
