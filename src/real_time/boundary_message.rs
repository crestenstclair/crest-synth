// path: src/real_time/boundary_message.rs

//! `BoundaryMessage` — a discrete message crossing the real-time boundary.
//!
//! `BoundaryMessage` is the payload carried across the RT/non-RT seam by the
//! `EventRing` (see `real_time::event_ring`): a note action, a controller
//! change, a patch or preset switch, or a single parameter update. It is
//! `Copy` and holds no heap-allocated fields, so producing, enqueueing, and
//! consuming one never touches the allocator, a lock, or blocking I/O.
//!
//! Every field is a small, independently-validated newtype rather than a
//! raw primitive, so an out-of-range channel, note, or velocity cannot be
//! smuggled across the boundary silently.

use std::fmt;

/// Zero-indexed MIDI channel address (0-15).
///
/// Channel 1 in MIDI parlance is `ChannelAddress::try_new(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelAddress(u8);

impl ChannelAddress {
    /// Maximum valid zero-indexed channel number.
    pub const MAX: u8 = 15;

    /// Constructs a `ChannelAddress`, returning `None` if `channel` exceeds
    /// [`Self::MAX`].
    pub fn try_new(channel: u8) -> Option<Self> {
        if !(0..=Self::MAX).contains(&channel) {
            None
        } else {
            Some(Self(channel))
        }
    }

    /// Returns the zero-indexed channel number (0-15).
    pub fn get(self) -> u8 {
        self.0
    }
}

impl fmt::Display for ChannelAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ch{}", self.0 + 1)
    }
}

/// A MIDI note number in the standard `0..=127` range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteNumber(u8);

impl NoteNumber {
    /// Maximum valid MIDI note number.
    pub const MAX: u8 = 127;

    /// Constructs a `NoteNumber`, returning `None` if `value` exceeds
    /// [`Self::MAX`].
    pub fn try_new(value: u8) -> Option<Self> {
        if !(0..=Self::MAX).contains(&value) {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Returns the raw MIDI note number (0-127).
    pub fn get(self) -> u8 {
        self.0
    }
}

/// Opaque identity tag correlating a `NoteOn` with its matching `NoteOff`.
///
/// Voice stealing and per-note modulation key off `NoteId` rather than
/// `NoteNumber` to avoid ambiguity when a note number repeats before its
/// previous instance has released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteId(u64);

impl NoteId {
    /// Constructs a `NoteId` from a raw identifier.
    ///
    /// Uniqueness across the lifetime of a session is the caller's
    /// responsibility (typically a monotonically increasing counter on the
    /// non-real-time thread).
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw identifier.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// A high-resolution, normalized velocity (or per-note pressure) value in
/// `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity(f32);

impl Velocity {
    /// Constructs a `Velocity`, returning `None` if `value` is `NaN` or
    /// outside the `0.0..=1.0` range.
    pub fn try_new(value: f32) -> Option<Self> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Returns the normalized velocity value.
    pub fn get(self) -> f32 {
        self.0
    }
}

/// A MIDI continuous-controller number (0-127).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControllerNumber(u8);

impl ControllerNumber {
    /// Maximum valid controller number.
    pub const MAX: u8 = 127;

    /// Constructs a `ControllerNumber`, returning `None` if `value` exceeds
    /// [`Self::MAX`].
    pub fn try_new(value: u8) -> Option<Self> {
        if !(0..=Self::MAX).contains(&value) {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Returns the raw controller number (0-127).
    pub fn get(self) -> u8 {
        self.0
    }
}

/// A MIDI continuous-controller value (0-127).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControllerValue(u8);

impl ControllerValue {
    /// Maximum valid controller value.
    pub const MAX: u8 = 127;

    /// Constructs a `ControllerValue`, returning `None` if `value` exceeds
    /// [`Self::MAX`].
    pub fn try_new(value: u8) -> Option<Self> {
        if !(0..=Self::MAX).contains(&value) {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Returns the raw controller value (0-127).
    pub fn get(self) -> u8 {
        self.0
    }
}

/// Identifies the patch a `PatchChange` message switches a channel to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatchNumber(u32);

impl PatchNumber {
    /// Constructs a `PatchNumber` from a raw value. Construction never
    /// fails: any `u32` is a well-formed patch identifier.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw patch number.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Identifies the preset a `PresetLoad` message asks to load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PresetNumber(u32);

impl PresetNumber {
    /// Constructs a `PresetNumber` from a raw value. Construction never
    /// fails: any `u32` is a well-formed preset identifier.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw preset number.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Identifies a single addressable slot on the `ParameterBridge` that a
/// `ParameterUpdate` message targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterIndex(usize);

impl ParameterIndex {
    /// Constructs a `ParameterIndex` from a raw slot index. Construction
    /// never fails; range-checking against a bridge's capacity is the
    /// receiving `ParameterBridge`'s responsibility.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the raw slot index.
    pub fn get(self) -> usize {
        self.0
    }
}

/// A validated parameter value carried by a `ParameterUpdate` message.
///
/// Only requires finiteness: unlike `Velocity`, parameter values are not
/// constrained to `0.0..=1.0` because different parameters have different
/// native ranges (frequency in Hz, gain in dB, ...).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterValue(f32);

impl ParameterValue {
    /// Constructs a `ParameterValue`, returning `None` if `value` is `NaN`
    /// or infinite.
    pub fn try_new(value: f32) -> Option<Self> {
        if value.is_finite() {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the raw parameter value.
    pub fn get(self) -> f32 {
        self.0
    }
}

/// A discrete message crossing the real-time boundary.
///
/// `BoundaryMessage` is `Copy` and contains no heap-allocated fields, so it
/// is safe to enqueue onto and dequeue from the `EventRing` from either side
/// of the RT/non-RT seam without allocating, locking, or blocking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundaryMessage {
    /// A note has been struck.
    NoteOn {
        channel: ChannelAddress,
        note: NoteNumber,
        note_id: NoteId,
        velocity: Velocity,
    },
    /// A note has been released.
    NoteOff {
        channel: ChannelAddress,
        note: NoteNumber,
        note_id: NoteId,
        velocity: Velocity,
    },
    /// A continuous controller value has changed.
    ControlChange {
        channel: ChannelAddress,
        controller: ControllerNumber,
        value: ControllerValue,
    },
    /// The active patch on a channel has changed.
    PatchChange {
        channel: ChannelAddress,
        patch: PatchNumber,
    },
    /// A preset should be loaded, replacing the current session state.
    PresetLoad { preset: PresetNumber },
    /// A single parameter slot has a new value.
    ParameterUpdate {
        parameter: ParameterIndex,
        value: ParameterValue,
    },
}

impl BoundaryMessage {
    /// The channel this message is addressed to, if any.
    ///
    /// `PresetLoad` and `ParameterUpdate` are not channel-scoped and return
    /// `None`.
    pub fn channel(&self) -> Option<ChannelAddress> {
        match self {
            BoundaryMessage::NoteOn { channel, .. }
            | BoundaryMessage::NoteOff { channel, .. }
            | BoundaryMessage::ControlChange { channel, .. }
            | BoundaryMessage::PatchChange { channel, .. } => Some(*channel),
            BoundaryMessage::PresetLoad { .. } | BoundaryMessage::ParameterUpdate { .. } => None,
        }
    }

    /// The discriminant kind of this message, without its payload.
    pub fn kind(&self) -> BoundaryMessageKind {
        match self {
            BoundaryMessage::NoteOn { .. } => BoundaryMessageKind::NoteOn,
            BoundaryMessage::NoteOff { .. } => BoundaryMessageKind::NoteOff,
            BoundaryMessage::ControlChange { .. } => BoundaryMessageKind::ControlChange,
            BoundaryMessage::PatchChange { .. } => BoundaryMessageKind::PatchChange,
            BoundaryMessage::PresetLoad { .. } => BoundaryMessageKind::PresetLoad,
            BoundaryMessage::ParameterUpdate { .. } => BoundaryMessageKind::ParameterUpdate,
        }
    }
}

/// The kind of a [`BoundaryMessage`], without its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundaryMessageKind {
    /// A note has been struck.
    NoteOn,
    /// A note has been released.
    NoteOff,
    /// A continuous controller value has changed.
    ControlChange,
    /// The active patch on a channel has changed.
    PatchChange,
    /// A preset should be loaded.
    PresetLoad,
    /// A single parameter slot has a new value.
    ParameterUpdate,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(n: u8) -> ChannelAddress {
        ChannelAddress::try_new(n).unwrap()
    }

    fn note(n: u8) -> NoteNumber {
        NoteNumber::try_new(n).unwrap()
    }

    fn velocity(v: f32) -> Velocity {
        Velocity::try_new(v).unwrap()
    }

    #[test]
    fn channel_address_accepts_valid_range() {
        assert_eq!(ChannelAddress::try_new(0).unwrap().get(), 0);
        assert_eq!(ChannelAddress::try_new(15).unwrap().get(), 15);
    }

    #[test]
    fn channel_address_rejects_out_of_range() {
        assert!(ChannelAddress::try_new(16).is_none());
    }

    #[test]
    fn channel_address_displays_one_indexed() {
        assert_eq!(format!("{}", channel(0)), "ch1");
    }

    #[test]
    fn note_number_accepts_valid_range() {
        assert_eq!(NoteNumber::try_new(60).unwrap().get(), 60);
        assert_eq!(NoteNumber::try_new(127).unwrap().get(), 127);
    }

    #[test]
    fn note_number_rejects_out_of_range() {
        assert!(NoteNumber::try_new(128).is_none());
    }

    #[test]
    fn velocity_accepts_valid_range() {
        assert_eq!(Velocity::try_new(0.0).unwrap().get(), 0.0);
        assert_eq!(Velocity::try_new(1.0).unwrap().get(), 1.0);
    }

    #[test]
    fn velocity_rejects_out_of_range_and_nan() {
        assert!(Velocity::try_new(-0.01).is_none());
        assert!(Velocity::try_new(1.01).is_none());
        assert!(Velocity::try_new(f32::NAN).is_none());
    }

    #[test]
    fn controller_number_and_value_accept_valid_range() {
        assert_eq!(ControllerNumber::try_new(127).unwrap().get(), 127);
        assert_eq!(ControllerValue::try_new(0).unwrap().get(), 0);
    }

    #[test]
    fn controller_number_and_value_reject_out_of_range() {
        assert!(ControllerNumber::try_new(128).is_none());
        assert!(ControllerValue::try_new(128).is_none());
    }

    #[test]
    fn patch_and_preset_numbers_round_trip() {
        assert_eq!(PatchNumber::new(7).get(), 7);
        assert_eq!(PresetNumber::new(9).get(), 9);
    }

    #[test]
    fn parameter_index_round_trips() {
        assert_eq!(ParameterIndex::new(3).get(), 3);
    }

    #[test]
    fn parameter_value_accepts_finite_values() {
        assert_eq!(ParameterValue::try_new(-12.5).unwrap().get(), -12.5);
    }

    #[test]
    fn parameter_value_rejects_non_finite_values() {
        assert!(ParameterValue::try_new(f32::NAN).is_none());
        assert!(ParameterValue::try_new(f32::INFINITY).is_none());
        assert!(ParameterValue::try_new(f32::NEG_INFINITY).is_none());
    }

    #[test]
    fn note_on_carries_its_fields_and_kind() {
        let note_id = NoteId::new(42);
        let message = BoundaryMessage::NoteOn {
            channel: channel(3),
            note: note(69),
            note_id,
            velocity: velocity(0.75),
        };

        assert_eq!(message.kind(), BoundaryMessageKind::NoteOn);
        assert_eq!(message.channel(), Some(channel(3)));
    }

    #[test]
    fn note_off_reports_correct_kind() {
        let message = BoundaryMessage::NoteOff {
            channel: channel(0),
            note: note(60),
            note_id: NoteId::new(1),
            velocity: velocity(0.0),
        };
        assert_eq!(message.kind(), BoundaryMessageKind::NoteOff);
    }

    #[test]
    fn control_change_is_channel_scoped() {
        let message = BoundaryMessage::ControlChange {
            channel: channel(5),
            controller: ControllerNumber::try_new(74).unwrap(),
            value: ControllerValue::try_new(100).unwrap(),
        };
        assert_eq!(message.kind(), BoundaryMessageKind::ControlChange);
        assert_eq!(message.channel(), Some(channel(5)));
    }

    #[test]
    fn patch_change_is_channel_scoped() {
        let message = BoundaryMessage::PatchChange {
            channel: channel(2),
            patch: PatchNumber::new(12),
        };
        assert_eq!(message.kind(), BoundaryMessageKind::PatchChange);
        assert_eq!(message.channel(), Some(channel(2)));
    }

    #[test]
    fn preset_load_is_not_channel_scoped() {
        let message = BoundaryMessage::PresetLoad {
            preset: PresetNumber::new(3),
        };
        assert_eq!(message.kind(), BoundaryMessageKind::PresetLoad);
        assert_eq!(message.channel(), None);
    }

    #[test]
    fn parameter_update_is_not_channel_scoped() {
        let message = BoundaryMessage::ParameterUpdate {
            parameter: ParameterIndex::new(0),
            value: ParameterValue::try_new(0.5).unwrap(),
        };
        assert_eq!(message.kind(), BoundaryMessageKind::ParameterUpdate);
        assert_eq!(message.channel(), None);
    }

    #[test]
    fn boundary_message_is_copy() {
        let message = BoundaryMessage::PresetLoad {
            preset: PresetNumber::new(1),
        };
        let copied = message;
        assert_eq!(message, copied);
    }
}
