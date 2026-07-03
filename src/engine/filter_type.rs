// path: src/engine/filter_type.rs

//! `FilterType` — the response shape of a filter: low-pass, high-pass,
//! band-pass, notch, low shelf, high shelf, or peak.
//!
//! This is a pure value object: it carries no audio state and performs no
//! processing itself. It exists so that filter configuration (e.g. an
//! `engine::filter::Filter`) can select a response shape by value, and so
//! that shape can cross the real-time boundary as plain, `Copy` data via the
//! `ParameterBridge` or the `EventRing`.

/// The response shape of a filter.
///
/// `FilterType` is `Copy` and allocation-free, so it is safe to read and
/// write from the audio thread and to send across the parameter/event
/// boundary without heap allocation or locking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterType {
    /// Passes frequencies below the cutoff, attenuates above it.
    LowPass,
    /// Passes frequencies above the cutoff, attenuates below it.
    HighPass,
    /// Passes a band of frequencies around the center, attenuates outside it.
    BandPass,
    /// Attenuates a band of frequencies around the center, passes outside it.
    Notch,
    /// Boosts or cuts frequencies below a corner frequency, flat above it.
    LowShelf,
    /// Boosts or cuts frequencies above a corner frequency, flat below it.
    HighShelf,
    /// Boosts or cuts a band of frequencies around a center frequency.
    Peak,
}

impl FilterType {
    /// All filter types, in a stable, canonical order — useful for UI
    /// pickers (e.g. cycling through choices with a gamepad).
    pub const ALL: [FilterType; 7] = [
        FilterType::LowPass,
        FilterType::HighPass,
        FilterType::BandPass,
        FilterType::Notch,
        FilterType::LowShelf,
        FilterType::HighShelf,
        FilterType::Peak,
    ];

    /// A short, human-readable name for display in the UI.
    pub fn label(self) -> &'static str {
        match self {
            FilterType::LowPass => "Low Pass",
            FilterType::HighPass => "High Pass",
            FilterType::BandPass => "Band Pass",
            FilterType::Notch => "Notch",
            FilterType::LowShelf => "Low Shelf",
            FilterType::HighShelf => "High Shelf",
            FilterType::Peak => "Peak",
        }
    }

    /// True for shelving and peak types, which have a gain parameter in
    /// addition to a frequency; false for the purely subtractive/band types.
    pub fn has_gain(self) -> bool {
        matches!(
            self,
            FilterType::LowShelf | FilterType::HighShelf | FilterType::Peak
        )
    }

    /// Returns the next filter type in `ALL`, wrapping around — used for
    /// gamepad-driven cycling through the available responses.
    pub fn next(self) -> FilterType {
        let idx = Self::ALL.iter().position(|&t| t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// Returns the previous filter type in `ALL`, wrapping around — used for
    /// gamepad-driven cycling through the available responses.
    pub fn previous(self) -> FilterType {
        let idx = Self::ALL.iter().position(|&t| t == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

impl Default for FilterType {
    /// Low-pass is the most common starting point for a subtractive filter.
    fn default() -> Self {
        FilterType::LowPass
    }
}

#[cfg(test)]
mod tests {
    use super::FilterType;

    #[test]
    fn default_is_low_pass() {
        assert_eq!(FilterType::default(), FilterType::LowPass);
    }

    #[test]
    fn all_contains_seven_distinct_types() {
        let mut seen = FilterType::ALL.to_vec();
        seen.sort_by_key(|t| t.label());
        seen.dedup();
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn labels_are_non_empty_and_distinct() {
        let labels: Vec<&str> = FilterType::ALL.iter().map(|t| t.label()).collect();
        for label in &labels {
            assert!(!label.is_empty());
        }
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), labels.len());
    }

    #[test]
    fn has_gain_is_true_only_for_shelves_and_peak() {
        assert!(!FilterType::LowPass.has_gain());
        assert!(!FilterType::HighPass.has_gain());
        assert!(!FilterType::BandPass.has_gain());
        assert!(!FilterType::Notch.has_gain());
        assert!(FilterType::LowShelf.has_gain());
        assert!(FilterType::HighShelf.has_gain());
        assert!(FilterType::Peak.has_gain());
    }

    #[test]
    fn next_cycles_through_all_and_wraps() {
        let mut current = FilterType::LowPass;
        for _ in 0..FilterType::ALL.len() {
            current = current.next();
        }
        assert_eq!(current, FilterType::LowPass);
    }

    #[test]
    fn previous_is_inverse_of_next() {
        for &t in FilterType::ALL.iter() {
            assert_eq!(t.next().previous(), t);
            assert_eq!(t.previous().next(), t);
        }
    }

    #[test]
    fn next_and_previous_wrap_at_boundaries() {
        assert_eq!(FilterType::Peak.next(), FilterType::LowPass);
        assert_eq!(FilterType::LowPass.previous(), FilterType::Peak);
    }
}
