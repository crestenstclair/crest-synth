// path: src/patch/mpe_zone.rs

//! `MpeZone` — an MPE (MIDI Polyphonic Expression) zone.
//!
//! An MPE zone reserves one channel as the "manager" channel, which carries
//! global CCs and zone-wide messages, plus a block of contiguous "member"
//! channels, each of which carries per-note expression (pitch bend,
//! pressure, timbre) for a single active note.
//!
//! This type only enforces the invariants that are local to a single zone
//! (member-channel contiguity and the 15-member cap that follows from a
//! single 16-channel MIDI cable). Enforcing that zones never overlap across
//! patches is the responsibility of whatever aggregate owns the collection
//! of zones (it needs visibility across zones that a single value object
//! cannot have) — `overlaps` is provided here as the primitive that
//! aggregate needs.

use crate::kernel::midi_channel::MidiChannel;
use std::fmt;

/// An MPE zone: a manager channel for global CCs plus contiguous member
/// channels for per-note expression.
///
/// Invariants:
/// - `member_channels` are contiguous (when sorted, each value is exactly
///   one more than the previous).
/// - `member_channels` number at most 15 (a zone plus its manager channel
///   can never exceed the 16 channels available on a single MIDI cable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpeZone {
    manager_channel: MidiChannel,
    member_channels: Vec<MidiChannel>,
}

/// Error returned when constructing an `MpeZone` from invalid state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MpeZoneError {
    /// `member_channels` were not contiguous when sorted.
    NotContiguous,
    /// `member_channels` exceeded the maximum of 15.
    TooManyMembers(usize),
}

impl fmt::Display for MpeZoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MpeZoneError::NotContiguous => {
                write!(f, "mpe zone member channels must be contiguous")
            }
            MpeZoneError::TooManyMembers(count) => {
                write!(f, "mpe zone member channels must number at most 15 (got {count})")
            }
        }
    }
}

impl std::error::Error for MpeZoneError {}

impl MpeZone {
    /// Attempts to construct an `MpeZone` from a manager channel and a set
    /// of member channels.
    ///
    /// Returns `Err(MpeZoneError::TooManyMembers)` if `member_channels` has
    /// more than 15 entries, or `Err(MpeZoneError::NotContiguous)` if the
    /// member channels are not contiguous once sorted.
    ///
    /// ```
    /// use crest_synth::kernel::midi_channel::MidiChannel;
    /// use crest_synth::patch::mpe_zone::MpeZone;
    ///
    /// let manager = MidiChannel::try_new(0).expect("0 is in range");
    /// let members = vec![
    ///     MidiChannel::try_new(1).expect("1 is in range"),
    ///     MidiChannel::try_new(2).expect("2 is in range"),
    ///     MidiChannel::try_new(3).expect("3 is in range"),
    /// ];
    /// assert!(MpeZone::try_new(manager, members).is_ok());
    /// ```
    pub fn try_new(
        manager_channel: MidiChannel,
        member_channels: Vec<MidiChannel>,
    ) -> Result<Self, MpeZoneError> {
        if member_channels.len() > 15 {
            return Err(MpeZoneError::TooManyMembers(member_channels.len()));
        }

        let mut sorted = member_channels.clone();
        sorted.sort();
        for pair in sorted.windows(2) {
            let previous = pair[0].value();
            let next = pair[1].value();
            if next != previous + 1 {
                return Err(MpeZoneError::NotContiguous);
            }
        }

        Ok(Self {
            manager_channel,
            member_channels,
        })
    }

    /// The manager channel, which carries global CCs for the zone.
    pub fn manager_channel(&self) -> MidiChannel {
        self.manager_channel
    }

    /// The contiguous member channels, each carrying per-note expression.
    pub fn member_channels(&self) -> &[MidiChannel] {
        &self.member_channels
    }

    /// Returns `true` if `channel` is either this zone's manager channel or
    /// one of its member channels.
    pub fn contains_channel(&self, channel: MidiChannel) -> bool {
        self.manager_channel == channel || self.member_channels.contains(&channel)
    }

    /// Returns `true` if this zone shares any channel (manager or member)
    /// with `other`.
    ///
    /// Consumers that own a collection of zones (e.g. one per patch) use
    /// this to enforce the cross-zone invariant that MPE zones never
    /// overlap: an overlapping zone would make per-note expression
    /// ambiguous, since a channel could belong to two zones at once.
    pub fn overlaps(&self, other: &MpeZone) -> bool {
        if self.contains_channel(other.manager_channel) {
            return true;
        }
        other
            .member_channels
            .iter()
            .any(|channel| self.contains_channel(*channel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(value: u8) -> MidiChannel {
        MidiChannel::try_new(value).expect("value is in 0..=15")
    }

    #[test]
    fn try_new_accepts_contiguous_members() {
        let zone = MpeZone::try_new(channel(0), vec![channel(1), channel(2), channel(3)])
            .expect("contiguous members should be accepted");
        assert_eq!(zone.manager_channel(), channel(0));
        assert_eq!(zone.member_channels(), &[channel(1), channel(2), channel(3)]);
    }

    #[test]
    fn try_new_accepts_members_supplied_out_of_order() {
        let zone = MpeZone::try_new(channel(0), vec![channel(3), channel(1), channel(2)])
            .expect("contiguous members should be accepted regardless of input order");
        assert_eq!(zone.member_channels().len(), 3);
    }

    #[test]
    fn try_new_accepts_empty_members() {
        let zone = MpeZone::try_new(channel(0), vec![])
            .expect("an empty member list is trivially contiguous");
        assert!(zone.member_channels().is_empty());
    }

    #[test]
    fn try_new_accepts_single_member() {
        let zone = MpeZone::try_new(channel(0), vec![channel(1)])
            .expect("a single member is trivially contiguous");
        assert_eq!(zone.member_channels(), &[channel(1)]);
    }

    #[test]
    fn try_new_accepts_exactly_fifteen_members() {
        let members: Vec<MidiChannel> = (1..=15).map(channel).collect();
        let zone = MpeZone::try_new(channel(0), members)
            .expect("15 members is the allowed maximum");
        assert_eq!(zone.member_channels().len(), 15);
    }

    #[test]
    fn try_new_rejects_non_contiguous_members() {
        let result = MpeZone::try_new(channel(0), vec![channel(1), channel(2), channel(4)]);
        assert_eq!(result, Err(MpeZoneError::NotContiguous));
    }

    #[test]
    fn try_new_rejects_duplicate_members_as_non_contiguous() {
        let result = MpeZone::try_new(channel(0), vec![channel(1), channel(1), channel(2)]);
        assert_eq!(result, Err(MpeZoneError::NotContiguous));
    }

    #[test]
    fn try_new_rejects_more_than_fifteen_members() {
        // 15 channels is the max possible on a 16-channel cable once the
        // manager channel takes one; simulate an over-count directly since
        // MidiChannel itself only has 16 valid values.
        let members: Vec<MidiChannel> = (0..=15).map(channel).chain(std::iter::once(channel(15))).collect();
        let result = MpeZone::try_new(channel(0), members.clone());
        assert_eq!(result, Err(MpeZoneError::TooManyMembers(members.len())));
    }

    #[test]
    fn contains_channel_matches_manager_and_members() {
        let zone = MpeZone::try_new(channel(0), vec![channel(1), channel(2)]).expect("valid zone");
        assert!(zone.contains_channel(channel(0)));
        assert!(zone.contains_channel(channel(1)));
        assert!(zone.contains_channel(channel(2)));
        assert!(!zone.contains_channel(channel(3)));
    }

    #[test]
    fn overlaps_detects_shared_manager_channel() {
        let zone_a = MpeZone::try_new(channel(0), vec![channel(1), channel(2)]).expect("valid zone");
        let zone_b = MpeZone::try_new(channel(0), vec![channel(8), channel(9)]).expect("valid zone");
        assert!(zone_a.overlaps(&zone_b));
        assert!(zone_b.overlaps(&zone_a));
    }

    #[test]
    fn overlaps_detects_shared_member_channel() {
        let zone_a = MpeZone::try_new(channel(0), vec![channel(1), channel(2), channel(3)]).expect("valid zone");
        let zone_b = MpeZone::try_new(channel(8), vec![channel(3), channel(4)]).expect("valid zone");
        assert!(zone_a.overlaps(&zone_b));
        assert!(zone_b.overlaps(&zone_a));
    }

    #[test]
    fn overlaps_is_false_for_disjoint_zones() {
        let zone_a = MpeZone::try_new(channel(0), vec![channel(1), channel(2), channel(3)]).expect("valid zone");
        let zone_b = MpeZone::try_new(channel(8), vec![channel(9), channel(10)]).expect("valid zone");
        assert!(!zone_a.overlaps(&zone_b));
        assert!(!zone_b.overlaps(&zone_a));
    }

    #[test]
    fn error_messages_are_descriptive() {
        assert_eq!(
            MpeZoneError::NotContiguous.to_string(),
            "mpe zone member channels must be contiguous"
        );
        assert_eq!(
            MpeZoneError::TooManyMembers(17).to_string(),
            "mpe zone member channels must number at most 15 (got 17)"
        );
    }
}
