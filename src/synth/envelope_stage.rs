// path: src/synth/envelope_stage.rs

/// ADSR envelope phase.
///
/// Voices transition through these stages during the lifetime of a note:
/// `Idle` → `Attack` → `Decay` → `Sustain` → `Release` → `Idle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvelopeStage {
    /// No active note; the voice is silent and free to be reused.
    #[default]
    Idle,
    /// Amplitude is rising from zero to peak level.
    Attack,
    /// Amplitude is falling from peak level toward the sustain level.
    Decay,
    /// Amplitude is held at the sustain level while the key is pressed.
    Sustain,
    /// Note-off received; amplitude is falling toward zero.
    Release,
}

impl EnvelopeStage {
    /// Returns `true` if the voice is inactive and can be stolen.
    pub fn is_idle(self) -> bool {
        self == EnvelopeStage::Idle
    }

    /// Returns `true` while the note is sounding (any active stage).
    pub fn is_active(self) -> bool {
        !self.is_idle()
    }
}

#[cfg(test)]
mod tests {
    use super::EnvelopeStage;

    #[test]
    fn default_is_idle() {
        assert_eq!(EnvelopeStage::default(), EnvelopeStage::Idle);
    }

    #[test]
    fn idle_predicates() {
        assert!(EnvelopeStage::Idle.is_idle());
        assert!(!EnvelopeStage::Idle.is_active());
    }

    #[test]
    fn active_stages_are_not_idle() {
        for stage in [
            EnvelopeStage::Attack,
            EnvelopeStage::Decay,
            EnvelopeStage::Sustain,
            EnvelopeStage::Release,
        ] {
            assert!(!stage.is_idle(), "{stage:?} should not be idle");
            assert!(stage.is_active(), "{stage:?} should be active");
        }
    }

    #[test]
    fn copy_and_clone() {
        let s = EnvelopeStage::Attack;
        let s2 = s;
        assert_eq!(s, s2);
        assert_eq!(s.clone(), EnvelopeStage::Attack);
    }

    #[test]
    fn debug_repr_is_not_empty() {
        assert!(!format!("{:?}", EnvelopeStage::Sustain).is_empty());
    }
}
