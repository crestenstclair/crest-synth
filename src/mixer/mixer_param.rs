// path: src/mixer/mixer_param.rs

//! Row identifiers for a channel strip in top-to-bottom navigation order.

/// The six parameter rows of a channel strip, top-to-bottom.
///
/// `Volume`, `ReverbSend`, `EchoSend`, `Pan` are continuous parameters.
/// `Mute` and `Solo` are toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixerParam {
    Volume,
    ReverbSend,
    EchoSend,
    Pan,
    Mute,
    Solo,
}

impl MixerParam {
    /// All six rows in canonical top-to-bottom navigation order.
    pub const ROW_ORDER: [MixerParam; 6] = [
        MixerParam::Volume,
        MixerParam::ReverbSend,
        MixerParam::EchoSend,
        MixerParam::Pan,
        MixerParam::Mute,
        MixerParam::Solo,
    ];

    /// True for continuously-valued parameters (Volume, ReverbSend, EchoSend, Pan).
    pub fn is_continuous(self) -> bool {
        !self.is_toggle()
    }

    /// True for two-state toggle parameters (Mute, Solo).
    pub fn is_toggle(self) -> bool {
        matches!(self, MixerParam::Mute | MixerParam::Solo)
    }

    /// The index into `ChannelStrip::sends` this parameter addresses, if any.
    ///
    /// `ReverbSend` addresses `sends[0]`; `EchoSend` addresses `sends[1]`.
    /// All other parameters do not address a send slot.
    pub fn send_index(self) -> Option<usize> {
        match self {
            MixerParam::ReverbSend => Some(0),
            MixerParam::EchoSend => Some(1),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_order_matches_navigation_order() {
        assert_eq!(
            MixerParam::ROW_ORDER,
            [
                MixerParam::Volume,
                MixerParam::ReverbSend,
                MixerParam::EchoSend,
                MixerParam::Pan,
                MixerParam::Mute,
                MixerParam::Solo,
            ]
        );
    }

    #[test]
    fn continuous_params_are_not_toggles() {
        for param in [
            MixerParam::Volume,
            MixerParam::ReverbSend,
            MixerParam::EchoSend,
            MixerParam::Pan,
        ] {
            assert!(param.is_continuous());
            assert!(!param.is_toggle());
        }
    }

    #[test]
    fn toggle_params_are_not_continuous() {
        for param in [MixerParam::Mute, MixerParam::Solo] {
            assert!(param.is_toggle());
            assert!(!param.is_continuous());
        }
    }

    #[test]
    fn send_index_addresses_expected_send_slots() {
        assert_eq!(MixerParam::ReverbSend.send_index(), Some(0));
        assert_eq!(MixerParam::EchoSend.send_index(), Some(1));
        assert_eq!(MixerParam::Volume.send_index(), None);
        assert_eq!(MixerParam::Pan.send_index(), None);
        assert_eq!(MixerParam::Mute.send_index(), None);
        assert_eq!(MixerParam::Solo.send_index(), None);
    }
}
