// path: src/mixer/mixer_param.rs

/// The six parameter rows displayed in the mixer view, in top-to-bottom order.
///
/// The row order defines navigation: `NavUp` moves toward `Volume` (the first
/// row) and `NavDown` moves toward `Solo` (the last row). Navigation saturates
/// at both ends — there is no wrap-around.
///
/// `Volume`, `ReverbSend`, `EchoSend`, and `Pan` are **continuous** parameters
/// (adjusted by fine/coarse steps in edit mode).  `Mute` and `Solo` are
/// **toggle** parameters (changed only via `ToggleFocusedParam`).
///
/// # Examples
///
/// ```
/// use crest_synth::mixer::mixer_param::MixerParam;
///
/// let p = MixerParam::Volume;
/// assert!(p.is_continuous());
/// assert!(!p.is_toggle());
///
/// let m = MixerParam::Mute;
/// assert!(m.is_toggle());
/// assert!(!m.is_continuous());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixerParam {
    /// Channel output level (0.0–1.0).
    Volume,
    /// Reverb send level (0.0–1.0).
    ReverbSend,
    /// Echo/delay send level (0.0–1.0).
    EchoSend,
    /// Stereo pan position (−1.0 = full left, +1.0 = full right).
    Pan,
    /// Mute toggle — channel is silenced when true.
    Mute,
    /// Solo toggle — only soloed channels are audible when at least one is soloed.
    Solo,
}

impl MixerParam {
    /// All six rows in top-to-bottom display order.
    pub const ALL: [MixerParam; 6] = [
        MixerParam::Volume,
        MixerParam::ReverbSend,
        MixerParam::EchoSend,
        MixerParam::Pan,
        MixerParam::Mute,
        MixerParam::Solo,
    ];

    /// Returns `true` if this parameter is adjusted by fine/coarse steps.
    pub fn is_continuous(self) -> bool {
        matches!(
            self,
            MixerParam::Volume | MixerParam::ReverbSend | MixerParam::EchoSend | MixerParam::Pan
        )
    }

    /// Returns `true` if this parameter is a boolean toggle.
    pub fn is_toggle(self) -> bool {
        matches!(self, MixerParam::Mute | MixerParam::Solo)
    }

    /// Returns the row index (0 = Volume, 5 = Solo) of this parameter.
    pub fn row_index(self) -> usize {
        match self {
            MixerParam::Volume => 0,
            MixerParam::ReverbSend => 1,
            MixerParam::EchoSend => 2,
            MixerParam::Pan => 3,
            MixerParam::Mute => 4,
            MixerParam::Solo => 5,
        }
    }

    /// Return the param one step toward `Volume`, saturating.
    pub fn prev(self) -> MixerParam {
        let idx = self.row_index();
        if idx == 0 {
            self
        } else {
            MixerParam::ALL[idx - 1]
        }
    }

    /// Return the param one step toward `Solo`, saturating.
    pub fn next(self) -> MixerParam {
        let idx = self.row_index();
        let last = MixerParam::ALL.len() - 1;
        if idx >= last {
            self
        } else {
            MixerParam::ALL[idx + 1]
        }
    }
}

#[cfg(test)]
mod mixer_param_tests {
    use super::*;

    #[test]
    fn mixer_param_continuous_rows() {
        assert!(MixerParam::Volume.is_continuous());
        assert!(MixerParam::ReverbSend.is_continuous());
        assert!(MixerParam::EchoSend.is_continuous());
        assert!(MixerParam::Pan.is_continuous());
    }

    #[test]
    fn mixer_param_toggle_rows() {
        assert!(MixerParam::Mute.is_toggle());
        assert!(MixerParam::Solo.is_toggle());
    }

    #[test]
    fn mixer_param_not_both() {
        for p in &MixerParam::ALL {
            assert_ne!(p.is_continuous(), p.is_toggle());
        }
    }

    #[test]
    fn mixer_param_prev_saturates_at_volume() {
        assert_eq!(MixerParam::Volume.prev(), MixerParam::Volume);
    }

    #[test]
    fn mixer_param_next_saturates_at_solo() {
        assert_eq!(MixerParam::Solo.next(), MixerParam::Solo);
    }

    #[test]
    fn mixer_param_prev_steps_correctly() {
        assert_eq!(MixerParam::ReverbSend.prev(), MixerParam::Volume);
        assert_eq!(MixerParam::Solo.prev(), MixerParam::Mute);
    }

    #[test]
    fn mixer_param_next_steps_correctly() {
        assert_eq!(MixerParam::Volume.next(), MixerParam::ReverbSend);
        assert_eq!(MixerParam::Mute.next(), MixerParam::Solo);
    }

    #[test]
    fn mixer_param_row_index_order() {
        for (i, p) in MixerParam::ALL.iter().enumerate() {
            assert_eq!(p.row_index(), i);
        }
    }
}
