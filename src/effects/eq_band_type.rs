// path: src/effects/eq_band_type.rs

//! `EqBandType` — the response shape of a single EQ band.
//!
//! This is a pure value object: an immutable, `Copy` enumeration with no
//! dependencies on the audio callback, no allocation, and no interior
//! mutability. It exists so that EQ band configuration (wherever it is
//! stored — a patch, a preset, a UI control) can name a filter response by
//! a small closed set of variants rather than a raw integer or string.

use std::fmt;

/// The response shape of a single EQ band.
///
/// Each variant corresponds to a standard filter topology found in a
/// parametric or graphic equalizer. This type carries no coefficients or
/// runtime state — it is purely a classification used by higher-level
/// EQ band configuration to select which filter design to instantiate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EqBandType {
    /// Passes frequencies below the cutoff, attenuates above it.
    LowPass,
    /// Passes frequencies above the cutoff, attenuates below it.
    HighPass,
    /// Passes a range of frequencies around the center, attenuates outside it.
    BandPass,
    /// Attenuates a narrow range of frequencies around the center.
    Notch,
    /// Boosts or cuts frequencies below the corner, leaves the rest unaffected.
    LowShelf,
    /// Boosts or cuts frequencies above the corner, leaves the rest unaffected.
    HighShelf,
    /// Boosts or cuts a bell-shaped range of frequencies around the center.
    Peak,
}

impl EqBandType {
    /// All variants, in a stable, deliberate order — useful for populating
    /// UI pickers (e.g. a gamepad-navigable list) without allocating.
    pub const ALL: [EqBandType; 7] = [
        EqBandType::LowPass,
        EqBandType::HighPass,
        EqBandType::BandPass,
        EqBandType::Notch,
        EqBandType::LowShelf,
        EqBandType::HighShelf,
        EqBandType::Peak,
    ];

    /// True for band types whose response is centered on a single
    /// frequency rather than a cutoff (band-pass, notch, and peak).
    pub const fn is_centered(self) -> bool {
        matches!(
            self,
            EqBandType::BandPass | EqBandType::Notch | EqBandType::Peak
        )
    }

    /// True for shelf types, which affect a whole half of the spectrum
    /// rather than a localized region.
    pub const fn is_shelf(self) -> bool {
        matches!(self, EqBandType::LowShelf | EqBandType::HighShelf)
    }

    /// Whether this band type has a meaningful resonance/quality (`Q`)
    /// parameter, as opposed to shelves which are typically controlled by
    /// slope rather than `Q`.
    pub const fn uses_resonance(self) -> bool {
        matches!(
            self,
            EqBandType::LowPass
                | EqBandType::HighPass
                | EqBandType::BandPass
                | EqBandType::Notch
                | EqBandType::Peak
        )
    }

    /// Whether this band type has a gain parameter (shelves and the peak
    /// band boost or cut; the other types only shape frequency response).
    pub const fn uses_gain(self) -> bool {
        matches!(
            self,
            EqBandType::LowShelf | EqBandType::HighShelf | EqBandType::Peak
        )
    }

    /// A short, human-readable label suitable for UI display.
    pub const fn label(self) -> &'static str {
        match self {
            EqBandType::LowPass => "Low Pass",
            EqBandType::HighPass => "High Pass",
            EqBandType::BandPass => "Band Pass",
            EqBandType::Notch => "Notch",
            EqBandType::LowShelf => "Low Shelf",
            EqBandType::HighShelf => "High Shelf",
            EqBandType::Peak => "Peak",
        }
    }
}

impl fmt::Display for EqBandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl Default for EqBandType {
    /// `Peak` is the most common general-purpose band type in a
    /// parametric EQ, so it is the sensible default for a freshly
    /// created band.
    fn default() -> Self {
        EqBandType::Peak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_peak() {
        assert_eq!(EqBandType::default(), EqBandType::Peak);
    }

    #[test]
    fn all_contains_every_variant_exactly_once() {
        let mut seen = std::collections::HashSet::new();
        for band in EqBandType::ALL {
            assert!(seen.insert(band), "duplicate variant in ALL: {band:?}");
        }
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn centered_types_are_band_pass_notch_and_peak() {
        assert!(EqBandType::BandPass.is_centered());
        assert!(EqBandType::Notch.is_centered());
        assert!(EqBandType::Peak.is_centered());

        assert!(!EqBandType::LowPass.is_centered());
        assert!(!EqBandType::HighPass.is_centered());
        assert!(!EqBandType::LowShelf.is_centered());
        assert!(!EqBandType::HighShelf.is_centered());
    }

    #[test]
    fn shelf_types_are_low_shelf_and_high_shelf() {
        assert!(EqBandType::LowShelf.is_shelf());
        assert!(EqBandType::HighShelf.is_shelf());

        assert!(!EqBandType::LowPass.is_shelf());
        assert!(!EqBandType::HighPass.is_shelf());
        assert!(!EqBandType::BandPass.is_shelf());
        assert!(!EqBandType::Notch.is_shelf());
        assert!(!EqBandType::Peak.is_shelf());
    }

    #[test]
    fn resonance_and_gain_usage_partitions_variants_as_expected() {
        assert!(EqBandType::LowPass.uses_resonance());
        assert!(!EqBandType::LowPass.uses_gain());

        assert!(EqBandType::HighPass.uses_resonance());
        assert!(!EqBandType::HighPass.uses_gain());

        assert!(EqBandType::BandPass.uses_resonance());
        assert!(!EqBandType::BandPass.uses_gain());

        assert!(EqBandType::Notch.uses_resonance());
        assert!(!EqBandType::Notch.uses_gain());

        assert!(!EqBandType::LowShelf.uses_resonance());
        assert!(EqBandType::LowShelf.uses_gain());

        assert!(!EqBandType::HighShelf.uses_resonance());
        assert!(EqBandType::HighShelf.uses_gain());

        assert!(EqBandType::Peak.uses_resonance());
        assert!(EqBandType::Peak.uses_gain());
    }

    #[test]
    fn display_matches_label() {
        for band in EqBandType::ALL {
            assert_eq!(band.to_string(), band.label());
        }
    }

    #[test]
    fn is_copy_and_eq() {
        let a = EqBandType::LowShelf;
        let b = a;
        assert_eq!(a, b);
    }
}
