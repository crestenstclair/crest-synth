//! Oscillator waveform selection.
//!
//! `Waveform` is a pure value object: a small, `Copy`-able enum naming the
//! shape an oscillator produces. It carries no audio state and performs no
//! I/O, allocation, or locking, so it is safe to read and pass by value on
//! the real-time audio thread.

use std::fmt;

/// The shape of signal an oscillator produces.
///
/// This is a plain value — constructing, comparing, or copying a `Waveform`
/// never allocates, locks, or blocks, so it may be read freely from the
/// audio callback. Changing which waveform a running voice uses is a
/// parameter change and must cross the thread boundary via the
/// `ParameterBridge` or the `EventRing`, not by handing a `&mut Waveform`
/// across threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Waveform {
    /// Pure sine tone.
    Sine,
    /// Sawtooth wave (bright, buzzy).
    Saw,
    /// Square wave (hollow, odd-harmonic-only).
    Square,
    /// Triangle wave (soft, few harmonics).
    Triangle,
    /// White noise.
    Noise,
    /// A user- or factory-supplied wavetable, addressed by slot index into
    /// the wavetable bank. The index itself carries no allocation: it is a
    /// plain `u16` naming a table that lives in non-realtime-owned storage.
    Wavetable(u16),
}

impl Waveform {
    /// All unit (non-parameterized) waveform variants, in a stable,
    /// UI-friendly order. Does not include `Wavetable`, which requires an
    /// index and is not a single fixed value.
    pub const UNIT_VARIANTS: [Waveform; 5] = [
        Waveform::Sine,
        Waveform::Saw,
        Waveform::Square,
        Waveform::Triangle,
        Waveform::Noise,
    ];

    /// A short, stable, human-readable name for the waveform. Suitable for
    /// UI labels and log lines; `Wavetable` includes its slot index.
    pub fn name(&self) -> &'static str {
        match self {
            Waveform::Sine => "Sine",
            Waveform::Saw => "Saw",
            Waveform::Square => "Square",
            Waveform::Triangle => "Triangle",
            Waveform::Noise => "Noise",
            Waveform::Wavetable(_) => "Wavetable",
        }
    }

    /// Returns the wavetable bank slot index if this is a `Wavetable`
    /// variant, or `None` for the built-in analog-style waveforms.
    pub fn wavetable_slot(&self) -> Option<u16> {
        match self {
            Waveform::Wavetable(slot) => Some(*slot),
            _ => None,
        }
    }

    /// Whether this waveform is one of the fixed, non-parameterized shapes
    /// (i.e. not `Wavetable`).
    pub fn is_fixed_shape(&self) -> bool {
        !matches!(self, Waveform::Wavetable(_))
    }
}

impl Default for Waveform {
    /// Sine is the conventional, harmonically simplest default oscillator
    /// shape.
    fn default() -> Self {
        Waveform::Sine
    }
}

impl fmt::Display for Waveform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Waveform::Wavetable(slot) => write!(f, "Wavetable[{slot}]"),
            other => write!(f, "{}", other.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_sine() {
        assert_eq!(Waveform::default(), Waveform::Sine);
    }

    #[test]
    fn name_matches_variant() {
        assert_eq!(Waveform::Sine.name(), "Sine");
        assert_eq!(Waveform::Saw.name(), "Saw");
        assert_eq!(Waveform::Square.name(), "Square");
        assert_eq!(Waveform::Triangle.name(), "Triangle");
        assert_eq!(Waveform::Noise.name(), "Noise");
        assert_eq!(Waveform::Wavetable(3).name(), "Wavetable");
    }

    #[test]
    fn display_formats_fixed_shapes_by_name() {
        assert_eq!(Waveform::Sine.to_string(), "Sine");
        assert_eq!(Waveform::Saw.to_string(), "Saw");
        assert_eq!(Waveform::Square.to_string(), "Square");
        assert_eq!(Waveform::Triangle.to_string(), "Triangle");
        assert_eq!(Waveform::Noise.to_string(), "Noise");
    }

    #[test]
    fn display_includes_wavetable_slot() {
        assert_eq!(Waveform::Wavetable(7).to_string(), "Wavetable[7]");
    }

    #[test]
    fn wavetable_slot_extracts_index_only_for_wavetable_variant() {
        assert_eq!(Waveform::Wavetable(5).wavetable_slot(), Some(5));
        assert_eq!(Waveform::Sine.wavetable_slot(), None);
        assert_eq!(Waveform::Noise.wavetable_slot(), None);
    }

    #[test]
    fn is_fixed_shape_distinguishes_wavetable() {
        assert!(Waveform::Sine.is_fixed_shape());
        assert!(Waveform::Saw.is_fixed_shape());
        assert!(Waveform::Square.is_fixed_shape());
        assert!(Waveform::Triangle.is_fixed_shape());
        assert!(Waveform::Noise.is_fixed_shape());
        assert!(!Waveform::Wavetable(0).is_fixed_shape());
    }

    #[test]
    fn unit_variants_covers_all_fixed_shapes() {
        assert_eq!(Waveform::UNIT_VARIANTS.len(), 5);
        for w in Waveform::UNIT_VARIANTS {
            assert!(w.is_fixed_shape());
        }
    }

    #[test]
    fn waveform_is_copy_and_cheap_to_pass_by_value() {
        // A Waveform must be safe to read on the audio thread without
        // allocation, locking, or indirection: enforce this at compile
        // time by relying on Copy semantics here.
        let w = Waveform::Wavetable(2);
        let copied = w;
        assert_eq!(w, copied);
    }

    #[test]
    fn equality_distinguishes_wavetable_slots() {
        assert_ne!(Waveform::Wavetable(1), Waveform::Wavetable(2));
        assert_eq!(Waveform::Wavetable(1), Waveform::Wavetable(1));
    }
}
