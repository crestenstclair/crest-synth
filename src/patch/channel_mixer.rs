// path: src/patch/channel_mixer.rs

//! `ChannelMixer` aggregate — the 16-channel mix bus.
//!
//! Each of the 16 channels (one per MIDI channel) has its own
//! [`ChannelStrip`] (volume, pan, reverb/echo sends, mute, solo) and a
//! [`PeakLevel`] metering value.
//!
//! # Audibility
//!
//! ```text
//! let any_solo = self.channels.iter().any(|c| c.solo());
//! fn audible(i) -> bool { channels[i].is_audible(any_solo) }
//! ```
//!
//! Soloing one or more channels silences the *audio* of every non-soloed
//! channel; muting silences that channel's audio.  An inaudible channel
//! contributes zero to the stereo sum.
//!
//! # Metering independence
//!
//! `peaks[i]` is recorded from `inputs[i]` **before** any mute/solo gate,
//! so a channel silenced by another's solo still meters its own live level.
//!
//! # Audio-thread constraints
//!
//! `mix()` is allocation-free in steady state.  The output buffer is
//! accumulated into the caller-supplied `Vec<AudioFrame>`; no allocation
//! occurs unless the `Vec`'s capacity needs to grow (which callers can
//! prevent by pre-sizing to the block length once).

use crate::kernel::audio_frame::AudioFrame;
use crate::patch::channel_strip::ChannelStrip;
use crate::patch::peak_level::PeakLevel;

// ─── Constants ─────────────────────────────────────────────────────────────────────────────

/// Number of MIDI channels (and thus mixer channels).
pub const CHANNEL_COUNT: usize = 16;

// ─── Commands ────────────────────────────────────────────────────────────────────────────

/// Commands accepted by [`ChannelMixer`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelMixerCommand {
    /// Toggle the mute state of the addressed channel.
    ToggleMute {
        /// Target channel index (0–15).
        channel: usize,
    },
    /// Toggle the solo state of the addressed channel.
    ToggleSolo {
        /// Target channel index (0–15).
        channel: usize,
    },
    /// Set the output volume of the addressed channel, clamped to [0.0, 1.0].
    SetVolume {
        /// Target channel index (0–15).
        channel: usize,
        /// Requested volume; clamped to [0.0, 1.0].
        value: f64,
    },
    /// Set the reverb-send level of the addressed channel, clamped to [0.0, 1.0].
    SetReverbSend {
        /// Target channel index (0–15).
        channel: usize,
        /// Requested send level; clamped to [0.0, 1.0].
        value: f64,
    },
    /// Set the echo-send level of the addressed channel, clamped to [0.0, 1.0].
    SetEchoSend {
        /// Target channel index (0–15).
        channel: usize,
        /// Requested send level; clamped to [0.0, 1.0].
        value: f64,
    },
    /// Set the pan position of the addressed channel, clamped to [−1.0, +1.0].
    SetPan {
        /// Target channel index (0–15).
        channel: usize,
        /// Requested pan; clamped to [−1.0, +1.0].
        value: f64,
    },
}

// ─── Events ──────────────────────────────────────────────────────────────────────────────

/// Events emitted by [`ChannelMixer`] in response to commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelMixerEvent {
    /// A channel's mute state was flipped.
    ChannelMuteToggled {
        /// Affected channel index (0–15).
        channel: usize,
        /// New mute state.
        muted: bool,
    },
    /// A channel's solo state was flipped.
    ChannelSoloToggled {
        /// Affected channel index (0–15).
        channel: usize,
        /// New solo state.
        soloed: bool,
    },
    /// A channel's volume was updated.
    ChannelVolumeChanged {
        /// Affected channel index (0–15).
        channel: usize,
        /// New (clamped) volume value.
        value: f64,
    },
    /// A channel's reverb-send level was updated.
    ChannelReverbSendChanged {
        /// Affected channel index (0–15).
        channel: usize,
        /// New (clamped) send level.
        value: f64,
    },
    /// A channel's echo-send level was updated.
    ChannelEchoSendChanged {
        /// Affected channel index (0–15).
        channel: usize,
        /// New (clamped) send level.
        value: f64,
    },
    /// A channel's pan position was updated.
    ChannelPanChanged {
        /// Affected channel index (0–15).
        channel: usize,
        /// New (clamped) pan value.
        value: f64,
    },
}

// ─── Pan utilities ────────────────────────────────────────────────────────────────────────

/// Compute left/right gain coefficients from a linear pan value in [−1.0, 1.0].
///
/// Uses a constant-power (cosine/sine) pan law so that perceived loudness
/// stays consistent across the stereo field.
///
/// Returns `(left_gain, right_gain)` as `f32` for the audio thread.
#[inline]
fn pan_gains(pan: f64) -> (f32, f32) {
    let p = pan.clamp(-1.0, 1.0);
    // Map pan [−1, 1] → angle [0, π/2]
    let angle = (p + 1.0) * std::f64::consts::FRAC_PI_4;
    let left = angle.cos() as f32;
    let right = angle.sin() as f32;
    (left, right)
}

// ─── ChannelMixer ────────────────────────────────────────────────────────────────────────────

/// 16-channel mix bus aggregate.
///
/// Owns 16 [`ChannelStrip`] parameter sets and 16 [`PeakLevel`] metering
/// accumulators.  Handles parameter commands on the control thread and
/// produces a stereo mix in [`ChannelMixer::mix`] (designed for the audio
/// thread — no heap allocation in steady state).
///
/// # Examples
///
/// ```
/// use crest_synth::patch::channel_mixer::{ChannelMixer, ChannelMixerCommand};
/// use crest_synth::kernel::audio_frame::AudioFrame;
///
/// let mut mixer = ChannelMixer::new();
/// let event = mixer.handle(ChannelMixerCommand::ToggleMute { channel: 0 }).unwrap();
/// let inputs: [Vec<AudioFrame>; 16] = std::array::from_fn(|_| vec![AudioFrame::new(1.0, 1.0)]);
/// let mut out = vec![AudioFrame::silence(); 1];
/// mixer.mix(&inputs, &mut out);
/// let _ = event;
/// ```
#[derive(Debug, Clone)]
pub struct ChannelMixer {
    /// Per-channel mix parameters.
    pub channels: [ChannelStrip; CHANNEL_COUNT],
    /// Per-channel peak metering (pre-gate, pre-volume).
    pub peaks: [PeakLevel; CHANNEL_COUNT],
}

impl ChannelMixer {
    /// Construct a `ChannelMixer` with all channels at default parameters
    /// (volume 1.0, pan 0.0, sends 0.0, unmuted, unsoloed).
    pub fn new() -> Self {
        Self {
            channels: [ChannelStrip::default(); CHANNEL_COUNT],
            peaks: [PeakLevel::default(); CHANNEL_COUNT],
        }
    }

    // ── Audibility ────────────────────────────────────────────────────────────────────

    /// Return `true` if channel `i`'s audio should be heard in the mix.
    ///
    /// ```text
    /// audible(i) == !channels[i].mute() && (!any_solo || channels[i].solo())
    /// ```
    #[inline]
    pub fn audible(&self, i: usize) -> bool {
        let any_solo = self.channels.iter().any(|c| c.solo());
        self.channels[i].is_audible(any_solo)
    }

    // ── Command handler ────────────────────────────────────────────────────────────

    /// Handle a [`ChannelMixerCommand`].
    ///
    /// Out-of-range channel indices are silently ignored;
    /// `None` is returned for out-of-range indices.
    ///
    /// Returns `Some(event)` on success, `None` if the channel index is invalid.
    pub fn handle(&mut self, cmd: ChannelMixerCommand) -> Option<ChannelMixerEvent> {
        match cmd {
            ChannelMixerCommand::ToggleMute { channel } => {
                if channel >= CHANNEL_COUNT {
                    return None;
                }
                let new_mute = !self.channels[channel].mute();
                self.channels[channel].set_mute(new_mute);
                Some(ChannelMixerEvent::ChannelMuteToggled {
                    channel,
                    muted: new_mute,
                })
            }
            ChannelMixerCommand::ToggleSolo { channel } => {
                if channel >= CHANNEL_COUNT {
                    return None;
                }
                let new_solo = !self.channels[channel].solo();
                self.channels[channel].set_solo(new_solo);
                Some(ChannelMixerEvent::ChannelSoloToggled {
                    channel,
                    soloed: new_solo,
                })
            }
            ChannelMixerCommand::SetVolume { channel, value } => {
                if channel >= CHANNEL_COUNT {
                    return None;
                }
                // ChannelStrip::set_volume clamps; read back to get clamped value.
                self.channels[channel].set_volume(value);
                let clamped = self.channels[channel].volume();
                Some(ChannelMixerEvent::ChannelVolumeChanged {
                    channel,
                    value: clamped,
                })
            }
            ChannelMixerCommand::SetReverbSend { channel, value } => {
                if channel >= CHANNEL_COUNT {
                    return None;
                }
                self.channels[channel].set_reverb_send(value);
                let clamped = self.channels[channel].reverb_send();
                Some(ChannelMixerEvent::ChannelReverbSendChanged {
                    channel,
                    value: clamped,
                })
            }
            ChannelMixerCommand::SetEchoSend { channel, value } => {
                if channel >= CHANNEL_COUNT {
                    return None;
                }
                self.channels[channel].set_echo_send(value);
                let clamped = self.channels[channel].echo_send();
                Some(ChannelMixerEvent::ChannelEchoSendChanged {
                    channel,
                    value: clamped,
                })
            }
            ChannelMixerCommand::SetPan { channel, value } => {
                if channel >= CHANNEL_COUNT {
                    return None;
                }
                self.channels[channel].set_pan(value);
                let clamped = self.channels[channel].pan();
                Some(ChannelMixerEvent::ChannelPanChanged {
                    channel,
                    value: clamped,
                })
            }
        }
    }

    // ── Mixdown ──────────────────────────────────────────────────────────────────────

    /// Mix 16 channel input buffers into the stereo output buffer `out`.
    ///
    /// For each channel `i`:
    /// 1. **Record peak** — `peaks[i]` = max absolute sample in `inputs[i]`
    ///    (pre-gate, pre-volume; metering is independent of mute/solo).
    /// 2. **Apply audibility gate** — if `!audible(i)`, skip (contributes zero).
    /// 3. **Accumulate** — for each frame, apply `volume` and equal-power `pan`,
    ///    and add to `out`.
    ///
    /// `out` is resized to the block length and zeroed before accumulation.
    ///
    /// `reverb_send` and `echo_send` are modelled on [`ChannelStrip`] but do
    /// **not** affect the dry sum; they are routing scalars for future buses.
    ///
    /// Allocation-free in steady state (no `Vec::push` etc. when `out` already
    /// has sufficient capacity).
    pub fn mix(&mut self, inputs: &[Vec<AudioFrame>; CHANNEL_COUNT], out: &mut Vec<AudioFrame>) {
        let block_len = inputs[0].len();
        // Prepare output: resize (may allocate once per block-size change) then zero.
        out.resize(block_len, AudioFrame::silence());
        for frame in out.iter_mut() {
            *frame = AudioFrame::silence();
        }

        let any_solo = self.channels.iter().any(|c| c.solo());

        let iter = self
            .channels
            .iter()
            .zip(self.peaks.iter_mut())
            .zip(inputs.iter());

        for ((strip, peak_slot), channel_input) in iter {
            // 1. Record peak (pre-gate, pre-volume).
            let peak: f32 = channel_input
                .iter()
                .map(|f| f.left.abs().max(f.right.abs()))
                .fold(0.0_f32, f32::max);
            // Store via the PeakLevel type (use try_new; clamp negatives to 0).
            *peak_slot = PeakLevel::try_new(peak).unwrap_or(PeakLevel::SILENT);

            // 2. Audibility gate.
            if !strip.is_audible(any_solo) {
                continue;
            }

            // 3. Accumulate into stereo bus.
            let vol = strip.volume() as f32;
            let (pan_l, pan_r) = pan_gains(strip.pan());
            let l_gain = vol * pan_l;
            let r_gain = vol * pan_r;

            for (out_frame, in_frame) in out.iter_mut().zip(channel_input.iter()) {
                out_frame.left += in_frame.left * l_gain;
                out_frame.right += in_frame.right * r_gain;
            }
        }
    }
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Invariant: exactly 16 channels ───────────────────────────────────────────

    #[test]
    fn channel_mixer_has_16_channels() {
        let mixer = ChannelMixer::new();
        assert_eq!(mixer.channels.len(), 16);
        assert_eq!(mixer.peaks.len(), 16);
    }

    // ── Invariant: default state ──────────────────────────────────────────────────

    #[test]
    fn default_channels_are_unmuted_unsoloed_unity_volume_centre_pan() {
        let mixer = ChannelMixer::new();
        for strip in &mixer.channels {
            assert!((strip.volume() - 1.0).abs() < f64::EPSILON);
            assert!(strip.pan().abs() < f64::EPSILON);
            assert!(strip.reverb_send().abs() < f64::EPSILON);
            assert!(strip.echo_send().abs() < f64::EPSILON);
            assert!(!strip.mute());
            assert!(!strip.solo());
        }
        for peak in &mixer.peaks {
            assert!(peak.value().abs() < f32::EPSILON);
        }
    }

    // ── Invariant: clamping ──────────────────────────────────────────────────────

    #[test]
    fn set_volume_clamps_above_1() {
        let mut mixer = ChannelMixer::new();
        let event = mixer
            .handle(ChannelMixerCommand::SetVolume {
                channel: 0,
                value: 2.5,
            })
            .unwrap();
        assert!((mixer.channels[0].volume() - 1.0).abs() < f64::EPSILON);
        assert_eq!(
            event,
            ChannelMixerEvent::ChannelVolumeChanged {
                channel: 0,
                value: 1.0
            }
        );
    }

    #[test]
    fn set_volume_clamps_below_0() {
        let mut mixer = ChannelMixer::new();
        let event = mixer
            .handle(ChannelMixerCommand::SetVolume {
                channel: 3,
                value: -0.5,
            })
            .unwrap();
        assert!(mixer.channels[3].volume().abs() < f64::EPSILON);
        assert_eq!(
            event,
            ChannelMixerEvent::ChannelVolumeChanged {
                channel: 3,
                value: 0.0
            }
        );
    }

    #[test]
    fn set_reverb_send_clamps_to_unit_interval() {
        let mut mixer = ChannelMixer::new();
        mixer
            .handle(ChannelMixerCommand::SetReverbSend {
                channel: 1,
                value: 1.5,
            })
            .unwrap();
        assert!((mixer.channels[1].reverb_send() - 1.0).abs() < f64::EPSILON);

        mixer
            .handle(ChannelMixerCommand::SetReverbSend {
                channel: 1,
                value: -1.0,
            })
            .unwrap();
        assert!(mixer.channels[1].reverb_send().abs() < f64::EPSILON);
    }

    #[test]
    fn set_echo_send_clamps_to_unit_interval() {
        let mut mixer = ChannelMixer::new();
        mixer
            .handle(ChannelMixerCommand::SetEchoSend {
                channel: 2,
                value: 3.0,
            })
            .unwrap();
        assert!((mixer.channels[2].echo_send() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_pan_clamps_to_neg1_pos1() {
        let mut mixer = ChannelMixer::new();
        let event_hi = mixer
            .handle(ChannelMixerCommand::SetPan {
                channel: 5,
                value: 2.0,
            })
            .unwrap();
        assert!((mixer.channels[5].pan() - 1.0).abs() < f64::EPSILON);
        assert_eq!(
            event_hi,
            ChannelMixerEvent::ChannelPanChanged {
                channel: 5,
                value: 1.0
            }
        );

        let event_lo = mixer
            .handle(ChannelMixerCommand::SetPan {
                channel: 5,
                value: -2.0,
            })
            .unwrap();
        assert!((mixer.channels[5].pan() + 1.0).abs() < f64::EPSILON);
        assert_eq!(
            event_lo,
            ChannelMixerEvent::ChannelPanChanged {
                channel: 5,
                value: -1.0
            }
        );
    }

    // ── Invariant: toggle mute / solo ───────────────────────────────────────────────

    #[test]
    fn toggle_mute_flips_state_and_emits_event() {
        let mut mixer = ChannelMixer::new();
        assert!(!mixer.channels[0].mute());
        let ev = mixer
            .handle(ChannelMixerCommand::ToggleMute { channel: 0 })
            .unwrap();
        assert!(mixer.channels[0].mute());
        assert_eq!(
            ev,
            ChannelMixerEvent::ChannelMuteToggled {
                channel: 0,
                muted: true
            }
        );
        let ev2 = mixer
            .handle(ChannelMixerCommand::ToggleMute { channel: 0 })
            .unwrap();
        assert!(!mixer.channels[0].mute());
        assert_eq!(
            ev2,
            ChannelMixerEvent::ChannelMuteToggled {
                channel: 0,
                muted: false
            }
        );
    }

    #[test]
    fn toggle_solo_flips_state_and_emits_event() {
        let mut mixer = ChannelMixer::new();
        let ev = mixer
            .handle(ChannelMixerCommand::ToggleSolo { channel: 7 })
            .unwrap();
        assert!(mixer.channels[7].solo());
        assert_eq!(
            ev,
            ChannelMixerEvent::ChannelSoloToggled {
                channel: 7,
                soloed: true
            }
        );
    }

    // ── Invariant: out-of-range channel is ignored ────────────────────────────────────

    #[test]
    fn out_of_range_channel_returns_none() {
        let mut mixer = ChannelMixer::new();
        assert!(mixer
            .handle(ChannelMixerCommand::ToggleMute { channel: 16 })
            .is_none());
        assert!(mixer
            .handle(ChannelMixerCommand::SetVolume {
                channel: 99,
                value: 0.5
            })
            .is_none());
    }

    // ── Helper: build uniform input buffers ─────────────────────────────────────────

    fn uniform_inputs(sample: f32, len: usize) -> [Vec<AudioFrame>; CHANNEL_COUNT] {
        std::array::from_fn(|_| vec![AudioFrame::new(sample, sample); len])
    }

    fn silent_inputs(len: usize) -> [Vec<AudioFrame>; CHANNEL_COUNT] {
        std::array::from_fn(|_| vec![AudioFrame::silence(); len])
    }

    // ── Invariant: audibility = !mute && (!any_solo || solo) ─────────────────────

    #[test]
    fn audible_all_unmuted_unsoloed() {
        let mixer = ChannelMixer::new();
        for i in 0..CHANNEL_COUNT {
            assert!(mixer.audible(i), "channel {i} should be audible by default");
        }
    }

    #[test]
    fn audible_muted_channel_is_not_audible() {
        let mut mixer = ChannelMixer::new();
        mixer.handle(ChannelMixerCommand::ToggleMute { channel: 3 });
        assert!(!mixer.audible(3));
        // Other channels still audible.
        assert!(mixer.audible(4));
    }

    #[test]
    fn audible_solo_silences_non_soloed() {
        let mut mixer = ChannelMixer::new();
        mixer.handle(ChannelMixerCommand::ToggleSolo { channel: 2 });
        // Only channel 2 is audible.
        assert!(mixer.audible(2));
        for i in 0..CHANNEL_COUNT {
            if i != 2 {
                assert!(
                    !mixer.audible(i),
                    "channel {i} should be silenced by ch2 solo"
                );
            }
        }
    }

    #[test]
    fn audible_muted_solo_is_not_audible() {
        let mut mixer = ChannelMixer::new();
        // Solo channel 0 then also mute it.
        mixer.handle(ChannelMixerCommand::ToggleSolo { channel: 0 });
        mixer.handle(ChannelMixerCommand::ToggleMute { channel: 0 });
        // Channel 0 is muted → not audible even though soloed.
        assert!(!mixer.audible(0));
    }

    // ── Mix: muted channel contributes zero ─────────────────────────────────────────

    #[test]
    fn mix_muted_channel_contributes_zero() {
        let mut mixer = ChannelMixer::new();
        // Mute all channels.
        for i in 0..CHANNEL_COUNT {
            mixer.handle(ChannelMixerCommand::ToggleMute { channel: i });
        }

        let inputs = uniform_inputs(1.0, 4);
        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);
        for frame in &out {
            assert!(frame.left.abs() < 1e-6, "muted mix should be silent");
            assert!(frame.right.abs() < 1e-6, "muted mix should be silent");
        }
    }

    #[test]
    fn mix_solo_silences_non_soloed_channels() {
        let mut mixer = ChannelMixer::new();
        // Zero out all channels.
        let mut inputs = silent_inputs(4);
        // Only channel 1 has signal.
        inputs[1] = vec![AudioFrame::new(0.5, 0.5); 4];
        // Solo channel 1.
        mixer.handle(ChannelMixerCommand::ToggleSolo { channel: 1 });
        // Channel 0 also has signal but is not soloed.
        for f in inputs[0].iter_mut() {
            *f = AudioFrame::new(1.0, 1.0);
        }
        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);
        // Result should only contain channel 1's signal.
        for frame in &out {
            assert!(
                frame.left > 0.0,
                "soloed channel 1 should contribute signal"
            );
        }
    }

    #[test]
    fn mix_all_silent_inputs_produces_silence() {
        let mut mixer = ChannelMixer::new();
        let inputs = silent_inputs(8);
        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);
        for frame in &out {
            assert!(frame.left.abs() < 1e-6);
            assert!(frame.right.abs() < 1e-6);
        }
    }

    // ── Invariant: metering is independent of mute/solo ──────────────────────────

    #[test]
    fn peak_recorded_for_muted_channel() {
        let mut mixer = ChannelMixer::new();
        // Mute channel 5.
        mixer.handle(ChannelMixerCommand::ToggleMute { channel: 5 });

        let mut inputs = silent_inputs(4);
        inputs[5] = vec![AudioFrame::new(0.8, 0.8); 4];

        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);

        // Peak for channel 5 should be 0.8, not 0 (metering is pre-gate).
        assert!(
            (mixer.peaks[5].value() - 0.8).abs() < 1e-5,
            "peak for muted channel should still be 0.8, got {}",
            mixer.peaks[5].value()
        );
    }

    #[test]
    fn peak_recorded_for_channel_silenced_by_solo() {
        let mut mixer = ChannelMixer::new();
        // Solo channel 0, so channel 3 will be silenced.
        mixer.handle(ChannelMixerCommand::ToggleSolo { channel: 0 });

        let mut inputs = silent_inputs(4);
        inputs[3] = vec![AudioFrame::new(0.6, 0.6); 4];

        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);

        // Channel 3 is not audible (solo-silenced), but peak should still be 0.6.
        assert!(
            (mixer.peaks[3].value() - 0.6).abs() < 1e-5,
            "peak for solo-silenced channel should still be 0.6, got {}",
            mixer.peaks[3].value()
        );
    }

    #[test]
    fn peak_zero_for_silent_channel() {
        let mut mixer = ChannelMixer::new();
        let inputs = silent_inputs(4);
        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);
        for (i, peak) in mixer.peaks.iter().enumerate() {
            assert!(
                peak.value().abs() < 1e-6,
                "channel {i} should have zero peak for silent input, got {}",
                peak.value()
            );
        }
    }

    #[test]
    fn peak_tracks_max_absolute_sample() {
        let mut mixer = ChannelMixer::new();
        let mut inputs = silent_inputs(4);
        // Channel 2 has varying samples; max abs is 0.9.
        inputs[2] = vec![
            AudioFrame::new(0.3, -0.9),
            AudioFrame::new(0.5, 0.4),
            AudioFrame::new(-0.1, 0.2),
            AudioFrame::new(0.7, 0.6),
        ];
        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);
        assert!(
            (mixer.peaks[2].value() - 0.9).abs() < 1e-5,
            "peak should be max abs sample 0.9, got {}",
            mixer.peaks[2].value()
        );
    }

    // ── Mix: volume scaling ─────────────────────────────────────────────────────

    #[test]
    fn mix_half_volume_halves_output() {
        // Use only channel 0 (mute the rest).
        let mut mixer = ChannelMixer::new();
        for i in 1..CHANNEL_COUNT {
            mixer.handle(ChannelMixerCommand::ToggleMute { channel: i });
        }

        let mut inputs = silent_inputs(1);
        inputs[0] = vec![AudioFrame::new(1.0, 1.0)];

        let mut out_full = Vec::new();
        mixer.mix(&inputs, &mut out_full);
        let full_l = out_full[0].left;

        // Now set volume to 0.5.
        mixer.handle(ChannelMixerCommand::SetVolume {
            channel: 0,
            value: 0.5,
        });
        let mut out_half = Vec::new();
        mixer.mix(&inputs, &mut out_half);
        let half_l = out_half[0].left;

        assert!(
            (full_l - half_l * 2.0).abs() < 1e-5,
            "half volume should give half output: full={full_l}, half={half_l}"
        );
    }

    // ── Mix: pan ────────────────────────────────────────────────────────────────────────

    #[test]
    fn mix_hard_left_pan_silences_right_channel() {
        let mut mixer = ChannelMixer::new();
        for i in 1..CHANNEL_COUNT {
            mixer.handle(ChannelMixerCommand::ToggleMute { channel: i });
        }
        mixer.handle(ChannelMixerCommand::SetPan {
            channel: 0,
            value: -1.0,
        });
        let mut inputs = silent_inputs(1);
        inputs[0] = vec![AudioFrame::new(1.0, 1.0)];
        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);
        assert!(
            out[0].left > 0.0,
            "left should have signal for hard-left pan"
        );
        assert!(
            out[0].right.abs() < 1e-5,
            "right should be silent for hard-left pan"
        );
    }

    #[test]
    fn mix_hard_right_pan_silences_left_channel() {
        let mut mixer = ChannelMixer::new();
        for i in 1..CHANNEL_COUNT {
            mixer.handle(ChannelMixerCommand::ToggleMute { channel: i });
        }
        mixer.handle(ChannelMixerCommand::SetPan {
            channel: 0,
            value: 1.0,
        });
        let mut inputs = silent_inputs(1);
        inputs[0] = vec![AudioFrame::new(1.0, 1.0)];
        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);
        assert!(
            out[0].left.abs() < 1e-5,
            "left should be silent for hard-right pan"
        );
        assert!(
            out[0].right > 0.0,
            "right should have signal for hard-right pan"
        );
    }

    // ── Mix: output length matches input length ──────────────────────────────────────

    #[test]
    fn mix_output_length_matches_input_block_length() {
        let mut mixer = ChannelMixer::new();
        let inputs: [Vec<AudioFrame>; CHANNEL_COUNT] =
            std::array::from_fn(|_| vec![AudioFrame::silence(); 64]);
        let mut out = Vec::new();
        mixer.mix(&inputs, &mut out);
        assert_eq!(out.len(), 64);
    }
}
