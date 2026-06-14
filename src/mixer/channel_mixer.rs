// path: src/mixer/channel_mixer.rs

use crate::mixer::mixer_param::MixerParam;

/// Number of mixer channels.
pub const CHANNEL_COUNT: usize = 16;

/// Per-channel state held by the `ChannelMixer`.
///
/// All continuous values are stored in the normalised 0.0–1.0 range **except
/// `pan`**, which is −1.0 (full left) to +1.0 (full right).
#[derive(Debug, Clone)]
pub struct ChannelState {
    /// Output level, clamped to 0.0–1.0.
    pub volume: f32,
    /// Reverb send level, clamped to 0.0–1.0.
    pub reverb_send: f32,
    /// Echo/delay send level, clamped to 0.0–1.0.
    pub echo_send: f32,
    /// Stereo pan, clamped to −1.0–+1.0.
    pub pan: f32,
    /// `true` when the channel is muted.
    pub mute: bool,
    /// `true` when the channel is soloed.
    pub solo: bool,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            volume: 0.75,
            reverb_send: 0.0,
            echo_send: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
        }
    }
}

/// Holds the state of all [`CHANNEL_COUNT`] mixer channels and exposes a small
/// command API that the `MixerView` reducer calls when editing parameters.
///
/// All writes are performed through named methods; there are no public fields.
/// Clamping is applied by the methods, so callers never need to bounds-check.
///
/// # Examples
///
/// ```
/// use crest_synth::mixer::channel_mixer::{ChannelMixer, CHANNEL_COUNT};
/// use crest_synth::mixer::mixer_param::MixerParam;
///
/// let mut cm = ChannelMixer::new();
/// cm.adjust(0, MixerParam::Volume, 0.10);
/// assert!((cm.channel(0).volume - 0.85).abs() < 1e-6);
/// ```
#[derive(Debug, Clone)]
pub struct ChannelMixer {
    channels: [ChannelState; CHANNEL_COUNT],
}

impl ChannelMixer {
    /// Create a new `ChannelMixer` with all channels at their default values.
    pub fn new() -> Self {
        Self {
            channels: std::array::from_fn(|_| ChannelState::default()),
        }
    }

    /// Read-only access to a single channel's state.
    ///
    /// # Panics
    ///
    /// Panics if `channel >= CHANNEL_COUNT`.
    pub fn channel(&self, channel: usize) -> &ChannelState {
        &self.channels[channel]
    }

    /// Read-only access to all channel states.
    pub fn channels(&self) -> &[ChannelState; CHANNEL_COUNT] {
        &self.channels
    }

    /// Adjust a **continuous** parameter on `channel` by `delta` (positive or
    /// negative). The result is clamped to the parameter's valid range.
    ///
    /// Calling this with a toggle parameter (`Mute` / `Solo`) is a **no-op**.
    ///
    /// # Panics
    ///
    /// Panics if `channel >= CHANNEL_COUNT`.
    pub fn adjust(&mut self, channel: usize, param: MixerParam, delta: f32) {
        let ch = &mut self.channels[channel];
        match param {
            MixerParam::Volume => {
                ch.volume = (ch.volume + delta).clamp(0.0, 1.0);
            }
            MixerParam::ReverbSend => {
                ch.reverb_send = (ch.reverb_send + delta).clamp(0.0, 1.0);
            }
            MixerParam::EchoSend => {
                ch.echo_send = (ch.echo_send + delta).clamp(0.0, 1.0);
            }
            MixerParam::Pan => {
                ch.pan = (ch.pan + delta).clamp(-1.0, 1.0);
            }
            // Toggles are not adjusted by delta.
            MixerParam::Mute | MixerParam::Solo => {}
        }
    }

    /// Toggle the mute state of `channel`.
    ///
    /// # Panics
    ///
    /// Panics if `channel >= CHANNEL_COUNT`.
    pub fn toggle_mute(&mut self, channel: usize) {
        self.channels[channel].mute = !self.channels[channel].mute;
    }

    /// Toggle the solo state of `channel`.
    ///
    /// # Panics
    ///
    /// Panics if `channel >= CHANNEL_COUNT`.
    pub fn toggle_solo(&mut self, channel: usize) {
        self.channels[channel].solo = !self.channels[channel].solo;
    }
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod channel_mixer_tests {
    use super::*;

    #[test]
    fn channel_mixer_default_volume() {
        let cm = ChannelMixer::new();
        assert!((cm.channel(0).volume - 0.75).abs() < 1e-6);
    }

    #[test]
    fn channel_mixer_adjust_volume_positive() {
        let mut cm = ChannelMixer::new();
        cm.adjust(0, MixerParam::Volume, 0.10);
        assert!((cm.channel(0).volume - 0.85).abs() < 1e-6);
    }

    #[test]
    fn channel_mixer_adjust_volume_negative() {
        let mut cm = ChannelMixer::new();
        cm.adjust(0, MixerParam::Volume, -0.10);
        assert!((cm.channel(0).volume - 0.65).abs() < 1e-6);
    }

    #[test]
    fn channel_mixer_adjust_volume_clamps_at_max() {
        let mut cm = ChannelMixer::new();
        cm.adjust(0, MixerParam::Volume, 100.0);
        assert!((cm.channel(0).volume - 1.0).abs() < 1e-6);
    }

    #[test]
    fn channel_mixer_adjust_volume_clamps_at_min() {
        let mut cm = ChannelMixer::new();
        cm.adjust(0, MixerParam::Volume, -100.0);
        assert!((cm.channel(0).volume - 0.0).abs() < 1e-6);
    }

    #[test]
    fn channel_mixer_adjust_pan_clamps() {
        let mut cm = ChannelMixer::new();
        cm.adjust(0, MixerParam::Pan, 5.0);
        assert!((cm.channel(0).pan - 1.0).abs() < 1e-6);
        cm.adjust(0, MixerParam::Pan, -10.0);
        assert!((cm.channel(0).pan - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn channel_mixer_adjust_toggle_is_noop() {
        let mut cm = ChannelMixer::new();
        let before_mute = cm.channel(0).mute;
        cm.adjust(0, MixerParam::Mute, 1.0);
        assert_eq!(cm.channel(0).mute, before_mute);
    }

    #[test]
    fn channel_mixer_toggle_mute() {
        let mut cm = ChannelMixer::new();
        assert!(!cm.channel(0).mute);
        cm.toggle_mute(0);
        assert!(cm.channel(0).mute);
        cm.toggle_mute(0);
        assert!(!cm.channel(0).mute);
    }

    #[test]
    fn channel_mixer_toggle_solo() {
        let mut cm = ChannelMixer::new();
        assert!(!cm.channel(0).solo);
        cm.toggle_solo(0);
        assert!(cm.channel(0).solo);
        cm.toggle_solo(0);
        assert!(!cm.channel(0).solo);
    }

    #[test]
    fn channel_mixer_channels_are_independent() {
        let mut cm = ChannelMixer::new();
        cm.adjust(3, MixerParam::Volume, 0.10);
        for i in 0..CHANNEL_COUNT {
            if i == 3 {
                assert!((cm.channel(i).volume - 0.85).abs() < 1e-6);
            } else {
                assert!((cm.channel(i).volume - 0.75).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn channel_mixer_adjust_reverb_send() {
        let mut cm = ChannelMixer::new();
        cm.adjust(1, MixerParam::ReverbSend, 0.5);
        assert!((cm.channel(1).reverb_send - 0.5).abs() < 1e-6);
    }

    #[test]
    fn channel_mixer_adjust_echo_send() {
        let mut cm = ChannelMixer::new();
        cm.adjust(2, MixerParam::EchoSend, 0.3);
        assert!((cm.channel(2).echo_send - 0.3).abs() < 1e-6);
    }
}
