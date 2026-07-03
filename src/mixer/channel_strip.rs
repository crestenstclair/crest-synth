//! Channel strip aggregate: one patch's channel — input gain, insert chain,
//! volume, pan, mute/solo, and up to eight send taps.
//!
//! This module models pure domain state and command/event handling. It does
//! not touch the audio thread directly: any real-time consumer reads a
//! snapshot of this aggregate across the ParameterBridge, never the
//! aggregate itself.

use std::f32::consts::FRAC_PI_2;

/// Maximum number of send taps a channel strip may hold.
pub const MAX_SENDS: usize = 8;

/// Error returned when a value is outside the domain-valid range for its type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutOfRangeError {
    pub type_name: &'static str,
    pub value: f32,
    pub min: f32,
    pub max: f32,
}

impl std::fmt::Display for OutOfRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} value {} out of range [{}, {}]",
            self.type_name, self.value, self.min, self.max
        )
    }
}

impl std::error::Error for OutOfRangeError {}

/// A linear amplitude multiplier. 0.0 is silence; 1.0 is unity gain.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Amplitude(f32);

impl Amplitude {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 4.0;

    pub fn try_new(value: f32) -> Result<Self, OutOfRangeError> {
        if value.is_nan() || !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(OutOfRangeError {
                type_name: "Amplitude",
                value,
                min: Self::MIN,
                max: Self::MAX,
            });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub const fn unity() -> Self {
        Self(1.0)
    }
}

impl Default for Amplitude {
    fn default() -> Self {
        Self::unity()
    }
}

/// Gain expressed in decibels, valid over the channel strip's fader range.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Decibel(f32);

impl Decibel {
    pub const MIN: f32 = -96.0;
    pub const MAX: f32 = 12.0;

    pub fn try_new(value: f32) -> Result<Self, OutOfRangeError> {
        if value.is_nan() || !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(OutOfRangeError {
                type_name: "Decibel",
                value,
                min: Self::MIN,
                max: Self::MAX,
            });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    /// Converts this decibel value to a linear amplitude multiplier.
    pub fn to_linear(&self) -> f32 {
        10f32.powf(self.0 / 20.0)
    }

    pub const fn unity() -> Self {
        Self(0.0)
    }
}

impl Default for Decibel {
    fn default() -> Self {
        Self::unity()
    }
}

/// Stereo pan position: -1.0 is full left, 0.0 is center, 1.0 is full right.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Pan(f32);

impl Pan {
    pub const MIN: f32 = -1.0;
    pub const MAX: f32 = 1.0;

    pub fn try_new(value: f32) -> Result<Self, OutOfRangeError> {
        if value.is_nan() || !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(OutOfRangeError {
                type_name: "Pan",
                value,
                min: Self::MIN,
                max: Self::MAX,
            });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub const fn center() -> Self {
        Self(0.0)
    }

    /// Equal-power left/right gain coefficients for this pan position.
    pub fn equal_power_gains(&self) -> (f32, f32) {
        let angle = (self.0 + 1.0) * (FRAC_PI_2 / 2.0);
        (angle.cos(), angle.sin())
    }
}

impl Default for Pan {
    fn default() -> Self {
        Self::center()
    }
}

/// Normalized post-fader peak level, 0.0 (silence) to 1.0 (full scale).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PeakLevel(f32);

impl PeakLevel {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 1.0;

    pub fn try_new(value: f32) -> Result<Self, OutOfRangeError> {
        if value.is_nan() || !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(OutOfRangeError {
                type_name: "PeakLevel",
                value,
                min: Self::MIN,
                max: Self::MAX,
            });
        }
        Ok(Self(value))
    }

    /// Builds a peak level from a raw (possibly out-of-range) magnitude by
    /// clamping to the meter's displayable range. Metering is best-effort
    /// visual feedback, not a validated domain input, so clamping — rather
    /// than rejecting — is the correct behavior for over-scale signals.
    pub fn from_clamped(raw: f32) -> Self {
        let clamped = if raw.is_nan() {
            0.0
        } else {
            raw.clamp(Self::MIN, Self::MAX)
        };
        Self(clamped)
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub const fn silence() -> Self {
        Self(0.0)
    }
}

impl Default for PeakLevel {
    fn default() -> Self {
        Self::silence()
    }
}

/// Identifies which bus a send tap routes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendBusId(pub u8);

/// One send: a routed tap to a bus at a given level, pre- or post-fader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SendTap {
    pub bus: SendBusId,
    pub level: Amplitude,
    /// Send taps are post-fader by default; pre-fader is an explicit
    /// opt-in per send.
    pub pre_fader: bool,
}

impl SendTap {
    pub fn new(bus: SendBusId, level: Amplitude) -> Self {
        Self {
            bus,
            level,
            pre_fader: false,
        }
    }

    pub fn pre_fader(bus: SendBusId, level: Amplitude) -> Self {
        Self {
            bus,
            level,
            pre_fader: true,
        }
    }
}

/// Commands accepted by a [`ChannelStrip`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelStripCommand {
    SetMute { mute: bool },
    SetPan { pan: Pan },
    SetSend { index: u8, tap: SendTap },
    SetSolo { solo: bool },
    SetVolume { volume_db: Decibel },
}

/// Domain events emitted by a [`ChannelStrip`] in response to commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelStripEvent {
    SoloChanged { solo: bool },
    LevelChanged { volume_db: Decibel },
}

/// Errors returned when a command cannot be applied to a [`ChannelStrip`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelStripError {
    /// `index` was outside `0..MAX_SENDS`.
    SendIndexOutOfRange { index: u8 },
}

impl std::fmt::Display for ChannelStripError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelStripError::SendIndexOutOfRange { index } => write!(
                f,
                "send index {index} out of range: a strip has at most {MAX_SENDS} send taps"
            ),
        }
    }
}

impl std::error::Error for ChannelStripError {}

/// One patch's channel: input gain, insert chain, volume, pan, mute/solo,
/// and up to eight send taps.
///
/// This is the aggregate root for a single mixer channel. It holds no
/// references to other aggregates or to I/O — it is pure domain state,
/// mutated only through [`ChannelStrip::handle`]. Real-time audio code
/// never touches this type directly; it consumes an immutable snapshot
/// published across the ParameterBridge.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelStrip {
    input_gain: Amplitude,
    mute: bool,
    pan: Pan,
    peak: PeakLevel,
    sends: [Option<SendTap>; MAX_SENDS],
    solo: bool,
    volume_db: Decibel,
}

impl ChannelStrip {
    /// Creates a new channel strip at unity gain, centered pan, silent
    /// meter, no sends, unmuted and unsoloed.
    pub fn new() -> Self {
        Self {
            input_gain: Amplitude::unity(),
            mute: false,
            pan: Pan::center(),
            peak: PeakLevel::silence(),
            sends: [None; MAX_SENDS],
            solo: false,
            volume_db: Decibel::unity(),
        }
    }

    pub fn input_gain(&self) -> Amplitude {
        self.input_gain
    }

    pub fn mute(&self) -> bool {
        self.mute
    }

    pub fn pan(&self) -> Pan {
        self.pan
    }

    pub fn peak(&self) -> PeakLevel {
        self.peak
    }

    pub fn solo(&self) -> bool {
        self.solo
    }

    pub fn volume_db(&self) -> Decibel {
        self.volume_db
    }

    /// Returns the send tap at `index`, if one has been set.
    pub fn send(&self, index: usize) -> Option<&SendTap> {
        self.sends.get(index).and_then(|slot| slot.as_ref())
    }

    /// Returns all occupied send tap slots.
    pub fn sends(&self) -> impl Iterator<Item = &SendTap> {
        self.sends.iter().filter_map(|slot| slot.as_ref())
    }

    /// Applies a command, mutating state and returning the domain events
    /// raised as a result. Commands that only update state without a
    /// corresponding declared event (SetMute, SetPan, SetSend) return an
    /// empty event list.
    pub fn handle(
        &mut self,
        command: ChannelStripCommand,
    ) -> Result<Vec<ChannelStripEvent>, ChannelStripError> {
        match command {
            ChannelStripCommand::SetMute { mute } => {
                self.mute = mute;
                Ok(Vec::new())
            }
            ChannelStripCommand::SetPan { pan } => {
                self.pan = pan;
                Ok(Vec::new())
            }
            ChannelStripCommand::SetSend { index, tap } => {
                let slot = usize::from(index);
                if slot >= MAX_SENDS {
                    return Err(ChannelStripError::SendIndexOutOfRange { index });
                }
                self.sends[slot] = Some(tap);
                Ok(Vec::new())
            }
            ChannelStripCommand::SetSolo { solo } => {
                self.solo = solo;
                Ok(vec![ChannelStripEvent::SoloChanged { solo }])
            }
            ChannelStripCommand::SetVolume { volume_db } => {
                self.volume_db = volume_db;
                Ok(vec![ChannelStripEvent::LevelChanged { volume_db }])
            }
        }
    }

    /// Computes the post-volume, post-pan peak level for an input sample
    /// magnitude and updates the strip's metered peak.
    ///
    /// Per invariant, peak metering reflects the level *after* volume and
    /// pan are applied — never the raw input.
    pub fn meter(&mut self, input: Amplitude) -> PeakLevel {
        let post_volume = input.value() * self.volume_db.to_linear();
        let (left_gain, right_gain) = self.pan.equal_power_gains();
        let left = post_volume * left_gain;
        let right = post_volume * right_gain;
        let peak = left.abs().max(right.abs());
        self.peak = PeakLevel::from_clamped(peak);
        self.peak
    }
}

impl Default for ChannelStrip {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_channel_strip_has_sane_defaults() {
        let strip = ChannelStrip::new();
        assert_eq!(strip.volume_db(), Decibel::unity());
        assert_eq!(strip.pan(), Pan::center());
        assert!(!strip.mute());
        assert!(!strip.solo());
        assert_eq!(strip.peak(), PeakLevel::silence());
        assert_eq!(strip.sends().count(), 0);
    }

    #[test]
    fn set_volume_mutates_state_and_emits_level_changed() {
        let mut strip = ChannelStrip::new();
        let volume_db = Decibel::try_new(-6.0).unwrap();
        let events = strip
            .handle(ChannelStripCommand::SetVolume { volume_db })
            .unwrap();
        assert_eq!(strip.volume_db(), volume_db);
        assert_eq!(events, vec![ChannelStripEvent::LevelChanged { volume_db }]);
    }

    #[test]
    fn set_solo_mutates_state_and_emits_solo_changed() {
        let mut strip = ChannelStrip::new();
        let events = strip
            .handle(ChannelStripCommand::SetSolo { solo: true })
            .unwrap();
        assert!(strip.solo());
        assert_eq!(events, vec![ChannelStripEvent::SoloChanged { solo: true }]);
    }

    #[test]
    fn set_mute_mutates_state_without_events() {
        let mut strip = ChannelStrip::new();
        let events = strip
            .handle(ChannelStripCommand::SetMute { mute: true })
            .unwrap();
        assert!(strip.mute());
        assert!(events.is_empty());
    }

    #[test]
    fn set_pan_mutates_state_without_events() {
        let mut strip = ChannelStrip::new();
        let pan = Pan::try_new(-0.5).unwrap();
        let events = strip.handle(ChannelStripCommand::SetPan { pan }).unwrap();
        assert_eq!(strip.pan(), pan);
        assert!(events.is_empty());
    }

    #[test]
    fn set_send_stores_tap_at_index() {
        let mut strip = ChannelStrip::new();
        let tap = SendTap::new(SendBusId(2), Amplitude::try_new(0.5).unwrap());
        let events = strip
            .handle(ChannelStripCommand::SetSend { index: 3, tap })
            .unwrap();
        assert!(events.is_empty());
        assert_eq!(strip.send(3), Some(&tap));
    }

    #[test]
    fn set_send_rejects_index_at_or_beyond_capacity() {
        let mut strip = ChannelStrip::new();
        let tap = SendTap::new(SendBusId(0), Amplitude::unity());
        let result = strip.handle(ChannelStripCommand::SetSend {
            index: MAX_SENDS as u8,
            tap,
        });
        assert_eq!(
            result,
            Err(ChannelStripError::SendIndexOutOfRange {
                index: MAX_SENDS as u8
            })
        );
    }

    #[test]
    fn strip_never_exceeds_eight_send_taps() {
        let mut strip = ChannelStrip::new();
        for i in 0..MAX_SENDS as u8 {
            let tap = SendTap::new(SendBusId(i), Amplitude::unity());
            strip
                .handle(ChannelStripCommand::SetSend { index: i, tap })
                .unwrap();
        }
        assert_eq!(strip.sends().count(), MAX_SENDS);

        let overflow_tap = SendTap::new(SendBusId(9), Amplitude::unity());
        let result = strip.handle(ChannelStripCommand::SetSend {
            index: MAX_SENDS as u8,
            tap: overflow_tap,
        });
        assert!(result.is_err());
        assert_eq!(strip.sends().count(), MAX_SENDS);
    }

    #[test]
    fn send_taps_are_post_fader_by_default() {
        let tap = SendTap::new(SendBusId(0), Amplitude::unity());
        assert!(!tap.pre_fader);
    }

    #[test]
    fn send_tap_pre_fader_is_explicit_opt_in() {
        let tap = SendTap::pre_fader(SendBusId(0), Amplitude::unity());
        assert!(tap.pre_fader);
    }

    #[test]
    fn meter_reflects_volume_and_pan_applied_to_input() {
        let mut strip = ChannelStrip::new();
        let volume_db = Decibel::try_new(-6.0).unwrap();
        strip
            .handle(ChannelStripCommand::SetVolume { volume_db })
            .unwrap();
        let pan = Pan::try_new(1.0).unwrap(); // full right
        strip.handle(ChannelStripCommand::SetPan { pan }).unwrap();

        let input = Amplitude::unity();
        let peak = strip.meter(input);

        // Full input at unity, attenuated by -6dB volume and panned hard
        // right: right channel carries full post-volume gain, left is
        // silent, so peak should equal the post-volume linear gain.
        let expected = volume_db.to_linear();
        assert!((peak.value() - expected).abs() < 1e-4);
        assert_eq!(strip.peak(), peak);
    }

    #[test]
    fn meter_never_exceeds_displayable_range() {
        let mut strip = ChannelStrip::new();
        let loud_input = Amplitude::try_new(4.0).unwrap();
        let peak = strip.meter(loud_input);
        assert!((PeakLevel::MIN..=PeakLevel::MAX).contains(&peak.value()));
    }

    #[test]
    fn amplitude_rejects_out_of_range_values() {
        assert!(Amplitude::try_new(-0.1).is_err());
        assert!(Amplitude::try_new(f32::NAN).is_err());
        assert!(Amplitude::try_new(Amplitude::MAX + 0.1).is_err());
    }

    #[test]
    fn decibel_rejects_out_of_range_values() {
        assert!(Decibel::try_new(Decibel::MIN - 1.0).is_err());
        assert!(Decibel::try_new(Decibel::MAX + 1.0).is_err());
    }

    #[test]
    fn pan_rejects_out_of_range_values() {
        assert!(Pan::try_new(-1.1).is_err());
        assert!(Pan::try_new(1.1).is_err());
    }
}
