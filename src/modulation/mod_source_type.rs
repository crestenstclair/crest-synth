// path: src/modulation/mod_source_type.rs

/// Identifies the source of a modulation signal.
///
/// Per-note expression dimensions (`PerNoteBendX`, `PerNoteTimbreY`,
/// `PerNotePressureZ`) are modelled as first-class mod sources so that voices
/// can consume them independently from the moment they are wired up, enabling
/// MPE support without a later refactor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModSourceType {
    /// ADSR / multi-stage envelope generator
    Envelope,
    /// Low-frequency oscillator
    Lfo,
    /// Sample-and-hold random value
    Random,
    /// User-assignable macro knob
    Macro,
    /// MIDI velocity of the triggering note-on
    Velocity,
    /// Key tracking (note number relative to a centre pitch)
    KeyTrack,
    /// Per-note pitch-bend expression (MPE dimension X)
    PerNoteBendX,
    /// Per-note timbre / slide expression (MPE dimension Y)
    PerNoteTimbreY,
    /// Per-note pressure / aftertouch expression (MPE dimension Z)
    PerNotePressureZ,
}

impl ModSourceType {
    /// Returns `true` if this source carries per-note expression data that
    /// must reach each voice directly (never aggregated at patch level).
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::modulation::mod_source_type::ModSourceType;
    ///
    /// assert!(ModSourceType::PerNoteBendX.is_per_note_expression());
    /// assert!(ModSourceType::PerNoteTimbreY.is_per_note_expression());
    /// assert!(ModSourceType::PerNotePressureZ.is_per_note_expression());
    /// assert!(!ModSourceType::Velocity.is_per_note_expression());
    /// assert!(!ModSourceType::Envelope.is_per_note_expression());
    /// ```
    #[inline]
    pub fn is_per_note_expression(self) -> bool {
        matches!(
            self,
            ModSourceType::PerNoteBendX
                | ModSourceType::PerNoteTimbreY
                | ModSourceType::PerNotePressureZ
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ModSourceType;

    #[test]
    fn per_note_expression_variants_are_identified() {
        assert!(ModSourceType::PerNoteBendX.is_per_note_expression());
        assert!(ModSourceType::PerNoteTimbreY.is_per_note_expression());
        assert!(ModSourceType::PerNotePressureZ.is_per_note_expression());
    }

    #[test]
    fn non_per_note_variants_are_not_expression() {
        let non_per_note = [
            ModSourceType::Envelope,
            ModSourceType::Lfo,
            ModSourceType::Random,
            ModSourceType::Macro,
            ModSourceType::Velocity,
            ModSourceType::KeyTrack,
        ];
        for src in non_per_note {
            assert!(
                !src.is_per_note_expression(),
                "{src:?} should not be a per-note expression source"
            );
        }
    }

    #[test]
    fn all_nine_variants_are_distinct() {
        use std::collections::HashSet;
        let all = [
            ModSourceType::Envelope,
            ModSourceType::Lfo,
            ModSourceType::Random,
            ModSourceType::Macro,
            ModSourceType::Velocity,
            ModSourceType::KeyTrack,
            ModSourceType::PerNoteBendX,
            ModSourceType::PerNoteTimbreY,
            ModSourceType::PerNotePressureZ,
        ];
        let unique: HashSet<_> = all.iter().collect();
        assert_eq!(unique.len(), all.len(), "duplicate variant detected");
    }

    #[test]
    fn copy_and_clone_work() {
        let original = ModSourceType::PerNoteBendX;
        let copied = original;
        let cloned = original;
        assert_eq!(original, copied);
        assert_eq!(original, cloned);
    }

    #[test]
    fn debug_format_is_non_empty() {
        let formatted = format!("{:?}", ModSourceType::Envelope);
        assert!(!formatted.is_empty());
    }
}
