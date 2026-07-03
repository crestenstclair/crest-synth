// path: src/modulation/lfo_shape.rs

//! `LfoShape` — the waveform an LFO (low-frequency oscillator) generates.
//!
//! This is a plain value object: a closed set of waveform kinds with no
//! behavior beyond identity, display, and enumeration. It carries no
//! allocation, no locks, and no I/O, so it is safe to read and copy on the
//! audio thread. Any change to the *active* shape for a running LFO must
//! still cross the thread boundary via the `ParameterBridge` or the
//! `EventRing`, per the project's real-time invariants — this type only
//! describes the value being carried, not how it gets there.

use std::fmt;

/// The waveform shape produced by an LFO.
///
/// `Copy` because it is a small, plain enum with no owned resources —
/// exactly the kind of value that can be snapshotted across the RT boundary
/// without allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LfoShape {
    /// Smooth sinusoidal oscillation.
    #[default]
    Sine,
    /// Linear ramp up and down.
    Triangle,
    /// Linear ramp up, hard reset down (or vice versa depending on polarity).
    Saw,
    /// Hard alternation between two levels.
    Square,
    /// Holds a randomly chosen value at each cycle boundary ("sample and hold").
    SampleAndHold,
    /// Continuously varying random value (noise-driven modulation).
    Random,
}

impl LfoShape {
    /// All shapes, in a stable declaration order — useful for UI pickers
    /// (e.g. cycling through shapes with a gamepad d-pad) and for tests that
    /// need to exercise every variant.
    pub const ALL: [LfoShape; 6] = [
        LfoShape::Sine,
        LfoShape::Triangle,
        LfoShape::Saw,
        LfoShape::Square,
        LfoShape::SampleAndHold,
        LfoShape::Random,
    ];

    /// A short, human-readable label for display in the UI.
    pub fn label(self) -> &'static str {
        match self {
            LfoShape::Sine => "Sine",
            LfoShape::Triangle => "Triangle",
            LfoShape::Saw => "Saw",
            LfoShape::Square => "Square",
            LfoShape::SampleAndHold => "Sample & Hold",
            LfoShape::Random => "Random",
        }
    }

    /// True if the shape produces a continuous curve (no discontinuities
    /// beyond cycle wrap), as opposed to a stepped/held value.
    pub fn is_continuous(self) -> bool {
        !matches!(self, LfoShape::SampleAndHold)
    }

    /// The next shape in `ALL`, wrapping around — suitable for a single
    /// gamepad button (e.g. right bumper) cycling through shapes.
    pub fn next(self) -> LfoShape {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// The previous shape in `ALL`, wrapping around — suitable for a single
    /// gamepad button (e.g. left bumper) cycling through shapes.
    pub fn previous(self) -> LfoShape {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        let len = Self::ALL.len();
        Self::ALL[(idx + len - 1) % len]
    }
}

impl fmt::Display for LfoShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_sine() {
        assert_eq!(LfoShape::default(), LfoShape::Sine);
    }

    #[test]
    fn all_contains_every_variant_once() {
        assert_eq!(LfoShape::ALL.len(), 6);
        let mut seen = std::collections::HashSet::new();
        for shape in LfoShape::ALL {
            assert!(seen.insert(shape), "duplicate shape in ALL: {shape:?}");
        }
    }

    #[test]
    fn labels_are_non_empty_and_distinct() {
        let mut labels = std::collections::HashSet::new();
        for shape in LfoShape::ALL {
            let label = shape.label();
            assert!(!label.is_empty());
            assert!(labels.insert(label), "duplicate label: {label}");
        }
    }

    #[test]
    fn display_matches_label() {
        for shape in LfoShape::ALL {
            assert_eq!(shape.to_string(), shape.label());
        }
    }

    #[test]
    fn sample_and_hold_is_not_continuous() {
        assert!(!LfoShape::SampleAndHold.is_continuous());
    }

    #[test]
    fn continuous_shapes_are_continuous() {
        for shape in [
            LfoShape::Sine,
            LfoShape::Triangle,
            LfoShape::Saw,
            LfoShape::Square,
            LfoShape::Random,
        ] {
            assert!(shape.is_continuous(), "{shape:?} should be continuous");
        }
    }

    #[test]
    fn next_cycles_through_all_and_wraps() {
        let start = LfoShape::Sine;
        let mut current = start;
        for _ in 0..LfoShape::ALL.len() {
            current = current.next();
        }
        assert_eq!(current, start, "cycling forward through all shapes should return to start");
    }

    #[test]
    fn previous_cycles_through_all_and_wraps() {
        let start = LfoShape::Sine;
        let mut current = start;
        for _ in 0..LfoShape::ALL.len() {
            current = current.previous();
        }
        assert_eq!(current, start, "cycling backward through all shapes should return to start");
    }

    #[test]
    fn next_and_previous_are_inverses() {
        for shape in LfoShape::ALL {
            assert_eq!(shape.next().previous(), shape);
            assert_eq!(shape.previous().next(), shape);
        }
    }

    #[test]
    fn next_visits_distinct_shape() {
        for shape in LfoShape::ALL {
            assert_ne!(shape.next(), shape);
        }
    }

    #[test]
    fn copy_semantics_do_not_move() {
        let a = LfoShape::Square;
        let b = a;
        // `a` remains usable because LfoShape is Copy.
        assert_eq!(a, b);
    }
}
