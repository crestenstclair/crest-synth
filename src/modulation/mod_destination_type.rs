// path: src/modulation/mod_destination_type.rs

/// Identifies the destination of a modulation signal.
///
/// Destinations are split into per-voice (applied independently to each voice)
/// and global (patch-level) categories. Per-voice destinations are required so
/// that per-note MPE expression dimensions can be routed to individual voices
/// without aggregation at the patch level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModDestinationType {
    // ── Per-voice destinations ─────────────────────────────────────────────────────
    /// Voice oscillator pitch offset in semitones (per-voice).
    OscillatorPitch,

    /// Voice oscillator fine-tune in cents (per-voice).
    OscillatorFineTune,

    /// Voice oscillator pulse-width / shape (per-voice).
    OscillatorShape,

    /// Voice filter cutoff frequency (per-voice).
    #[default]
    FilterCutoff,

    /// Voice filter resonance (per-voice).
    FilterResonance,

    /// Voice output amplitude / gain (per-voice).
    Amplitude,

    /// Voice panning position -1.0 (left) to +1.0 (right) (per-voice).
    Pan,

    /// Voice envelope attack time (per-voice).
    EnvelopeAttack,

    /// Voice envelope decay time (per-voice).
    EnvelopeDecay,

    /// Voice envelope sustain level (per-voice).
    EnvelopeSustain,

    /// Voice envelope release time (per-voice).
    EnvelopeRelease,

    // ── Global / patch-level destinations ──────────────────────────────────────
    /// LFO rate for LFO at the given index (patch-level).
    LfoRate(u8),

    /// LFO depth for LFO at the given index (patch-level).
    LfoDepth(u8),

    /// Send level to an effects bus by index (patch-level).
    EffectsSend(u8),
}

impl ModDestinationType {
    /// Returns `true` if this destination applies independently to each voice
    /// rather than being aggregated at the patch level.
    ///
    /// Per-voice destinations are required so that per-note expression (MPE)
    /// reaches voices directly.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::modulation::mod_destination_type::ModDestinationType;
    ///
    /// assert!(ModDestinationType::FilterCutoff.is_per_voice());
    /// assert!(ModDestinationType::Amplitude.is_per_voice());
    /// assert!(!ModDestinationType::LfoRate(0).is_per_voice());
    /// ```
    #[inline]
    pub fn is_per_voice(self) -> bool {
        matches!(
            self,
            ModDestinationType::OscillatorPitch
                | ModDestinationType::OscillatorFineTune
                | ModDestinationType::OscillatorShape
                | ModDestinationType::FilterCutoff
                | ModDestinationType::FilterResonance
                | ModDestinationType::Amplitude
                | ModDestinationType::Pan
                | ModDestinationType::EnvelopeAttack
                | ModDestinationType::EnvelopeDecay
                | ModDestinationType::EnvelopeSustain
                | ModDestinationType::EnvelopeRelease
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_voice_destinations_are_identified() {
        let per_voice = [
            ModDestinationType::OscillatorPitch,
            ModDestinationType::OscillatorFineTune,
            ModDestinationType::OscillatorShape,
            ModDestinationType::FilterCutoff,
            ModDestinationType::FilterResonance,
            ModDestinationType::Amplitude,
            ModDestinationType::Pan,
            ModDestinationType::EnvelopeAttack,
            ModDestinationType::EnvelopeDecay,
            ModDestinationType::EnvelopeSustain,
            ModDestinationType::EnvelopeRelease,
        ];
        for dest in per_voice {
            assert!(dest.is_per_voice(), "{dest:?} should be per-voice");
        }
    }

    #[test]
    fn global_destinations_are_not_per_voice() {
        assert!(!ModDestinationType::LfoRate(0).is_per_voice());
        assert!(!ModDestinationType::LfoDepth(1).is_per_voice());
        assert!(!ModDestinationType::EffectsSend(0).is_per_voice());
    }

    #[test]
    fn default_is_filter_cutoff() {
        assert_eq!(
            ModDestinationType::default(),
            ModDestinationType::FilterCutoff
        );
    }

    #[test]
    fn copy_semantics() {
        let a = ModDestinationType::OscillatorPitch;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn indexed_variants_are_distinct() {
        assert_ne!(
            ModDestinationType::LfoRate(0),
            ModDestinationType::LfoRate(1)
        );
        assert_ne!(
            ModDestinationType::EffectsSend(0),
            ModDestinationType::EffectsSend(1)
        );
    }

    #[test]
    fn hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ModDestinationType::FilterCutoff);
        set.insert(ModDestinationType::Amplitude);
        set.insert(ModDestinationType::LfoRate(0));
        assert_eq!(set.len(), 3);
    }
}
