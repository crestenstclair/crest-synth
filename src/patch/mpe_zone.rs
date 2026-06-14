// path: src/patch/mpe_zone.rs

use crate::kernel::midi_channel::MidiChannel;

/// Error returned when an [`MpeZone`] cannot be constructed because the
/// supplied channel values violate MPE zone invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MpeZoneError {
    /// `member_channel_start` must be strictly less than `member_channel_end`.
    MemberChannelOrderInvalid {
        start: MidiChannel,
        end: MidiChannel,
    },
    /// The manager channel must not be the same as any member channel.
    ManagerChannelOverlapsMemberChannels {
        manager: MidiChannel,
        start: MidiChannel,
        end: MidiChannel,
    },
}

impl std::fmt::Display for MpeZoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MpeZoneError::MemberChannelOrderInvalid { start, end } => write!(
                f,
                "member_channel_start ({}) must be less than member_channel_end ({})",
                start.value(),
                end.value()
            ),
            MpeZoneError::ManagerChannelOverlapsMemberChannels {
                manager,
                start,
                end,
            } => write!(
                f,
                "manager channel ({}) must not overlap member channels ({}-{})",
                manager.value(),
                start.value(),
                end.value()
            ),
        }
    }
}

impl std::error::Error for MpeZoneError {}

/// MPE zone configuration.
///
/// An MPE zone consists of:
/// - A **manager channel** that carries global per-zone parameters (pitch
///   bend range, pressure, etc.).
/// - A contiguous range of **member channels** `[member_channel_start,
///   member_channel_end]` that carry per-note parameters.
///
/// # Invariants
///
/// 1. `member_channel_start < member_channel_end`
/// 2. `manager_channel` does not fall within `[member_channel_start,
///    member_channel_end]`
///
/// # Examples
///
/// ```
/// use crest_synth::kernel::midi_channel::MidiChannel;
/// use crest_synth::patch::mpe_zone::MpeZone;
///
/// let manager = MidiChannel::try_new(0).unwrap();
/// let start   = MidiChannel::try_new(1).unwrap();
/// let end     = MidiChannel::try_new(8).unwrap();
/// let zone = MpeZone::try_new(manager, start, end).unwrap();
/// assert_eq!(zone.manager_channel().value(), 0);
/// assert_eq!(zone.member_channel_start().value(), 1);
/// assert_eq!(zone.member_channel_end().value(), 8);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpeZone {
    manager_channel: MidiChannel,
    member_channel_start: MidiChannel,
    member_channel_end: MidiChannel,
}

impl MpeZone {
    /// Construct a new `MpeZone`, validating all MPE invariants.
    ///
    /// # Errors
    ///
    /// Returns [`MpeZoneError::MemberChannelOrderInvalid`] if
    /// `member_channel_start >= member_channel_end`.
    ///
    /// Returns [`MpeZoneError::ManagerChannelOverlapsMemberChannels`] if
    /// `manager_channel` falls within `[member_channel_start,
    /// member_channel_end]`.
    pub fn try_new(
        manager_channel: MidiChannel,
        member_channel_start: MidiChannel,
        member_channel_end: MidiChannel,
    ) -> Result<Self, MpeZoneError> {
        // Invariant 1: member_channel_start < member_channel_end
        if member_channel_start >= member_channel_end {
            return Err(MpeZoneError::MemberChannelOrderInvalid {
                start: member_channel_start,
                end: member_channel_end,
            });
        }

        // Invariant 2: manager channel must not overlap member channels
        if (member_channel_start..=member_channel_end).contains(&manager_channel) {
            return Err(MpeZoneError::ManagerChannelOverlapsMemberChannels {
                manager: manager_channel,
                start: member_channel_start,
                end: member_channel_end,
            });
        }

        Ok(Self {
            manager_channel,
            member_channel_start,
            member_channel_end,
        })
    }

    /// Return the manager channel for this MPE zone.
    #[inline]
    pub fn manager_channel(self) -> MidiChannel {
        self.manager_channel
    }

    /// Return the first (lowest) member channel.
    #[inline]
    pub fn member_channel_start(self) -> MidiChannel {
        self.member_channel_start
    }

    /// Return the last (highest) member channel.
    #[inline]
    pub fn member_channel_end(self) -> MidiChannel {
        self.member_channel_end
    }

    /// Return `true` if the given channel is within the member channel range.
    #[inline]
    pub fn is_member_channel(self, channel: MidiChannel) -> bool {
        (self.member_channel_start..=self.member_channel_end).contains(&channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(v: u8) -> MidiChannel {
        MidiChannel::try_new(v).unwrap()
    }

    #[test]
    fn valid_lower_zone() {
        // Classic MPE lower zone: manager=0, members=1..=8
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        assert_eq!(zone.manager_channel().value(), 0);
        assert_eq!(zone.member_channel_start().value(), 1);
        assert_eq!(zone.member_channel_end().value(), 8);
    }

    #[test]
    fn valid_upper_zone() {
        // Classic MPE upper zone: manager=15, members=9..=14
        let zone = MpeZone::try_new(ch(15), ch(9), ch(14)).unwrap();
        assert_eq!(zone.manager_channel().value(), 15);
        assert_eq!(zone.member_channel_start().value(), 9);
        assert_eq!(zone.member_channel_end().value(), 14);
    }

    #[test]
    fn member_channel_start_must_be_less_than_end() {
        // start == end is invalid
        let err = MpeZone::try_new(ch(0), ch(5), ch(5)).unwrap_err();
        assert!(matches!(
            err,
            MpeZoneError::MemberChannelOrderInvalid { .. }
        ));
    }

    #[test]
    fn member_channel_start_greater_than_end_is_invalid() {
        let err = MpeZone::try_new(ch(0), ch(8), ch(1)).unwrap_err();
        assert!(matches!(
            err,
            MpeZoneError::MemberChannelOrderInvalid { .. }
        ));
    }

    #[test]
    fn manager_channel_at_start_of_member_range_is_invalid() {
        // manager == member_start overlaps
        let err = MpeZone::try_new(ch(1), ch(1), ch(8)).unwrap_err();
        assert!(matches!(
            err,
            MpeZoneError::ManagerChannelOverlapsMemberChannels { .. }
        ));
    }

    #[test]
    fn manager_channel_at_end_of_member_range_is_invalid() {
        let err = MpeZone::try_new(ch(8), ch(1), ch(8)).unwrap_err();
        assert!(matches!(
            err,
            MpeZoneError::ManagerChannelOverlapsMemberChannels { .. }
        ));
    }

    #[test]
    fn manager_channel_in_middle_of_member_range_is_invalid() {
        let err = MpeZone::try_new(ch(5), ch(1), ch(8)).unwrap_err();
        assert!(matches!(
            err,
            MpeZoneError::ManagerChannelOverlapsMemberChannels { .. }
        ));
    }

    #[test]
    fn is_member_channel_returns_true_for_members() {
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        assert!(zone.is_member_channel(ch(1)));
        assert!(zone.is_member_channel(ch(5)));
        assert!(zone.is_member_channel(ch(8)));
    }

    #[test]
    fn is_member_channel_returns_false_for_non_members() {
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        assert!(!zone.is_member_channel(ch(0)));
        assert!(!zone.is_member_channel(ch(9)));
        assert!(!zone.is_member_channel(ch(15)));
    }

    #[test]
    fn copy_semantics() {
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        let copied = zone;
        assert_eq!(zone, copied);
    }

    #[test]
    fn error_display_member_order() {
        let err = MpeZone::try_new(ch(0), ch(8), ch(1)).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("member_channel_start"));
        assert!(msg.contains("member_channel_end"));
    }

    #[test]
    fn error_display_manager_overlap() {
        let err = MpeZone::try_new(ch(5), ch(1), ch(8)).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("manager"));
    }

    #[test]
    fn minimal_valid_zone_two_members() {
        // smallest valid zone: members span exactly two adjacent channels
        let zone = MpeZone::try_new(ch(0), ch(1), ch(2)).unwrap();
        assert_eq!(zone.member_channel_start().value(), 1);
        assert_eq!(zone.member_channel_end().value(), 2);
    }
}
