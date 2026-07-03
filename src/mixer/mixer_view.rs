// path: src/mixer/mixer_view.rs

//! `MixerView` — the single mixer-view store.
//!
//! Owns cursor (channel + parameter), viewport offset, edit-mode, and the
//! 16 [`ChannelStrip`] channels it wraps. This is the one entry point that
//! reacts to [`MixerViewEvent`]s: `apply` is the ONLY way to mutate the
//! view. Draw code (egui or otherwise) reads channel values and per-channel
//! peak levels from the wrapped strips but never mutates them directly —
//! all mutation flows through an event applied here, keeping the control
//! plane hermetically testable exactly like the Editor view.
//!
//! Timing (double-tap detection, Edit-hold duration) lives entirely in the
//! input adapter that produces these events; this reducer is timing-free.
//! Keyboard and gamepad adapters emit the identical `MixerViewEvent`
//! variants, so the two input paths are interchangeable here.

use crate::mixer::channel_strip::{ChannelStrip, ChannelStripCommand, Decibel, Pan};

/// Total number of channel strips a [`MixerView`] wraps.
pub const TOTAL_CHANNELS: usize = 16;

/// Number of channels visible in the viewport at once.
pub const VISIBLE_CHANNELS: usize = 6;

/// The highest legal `viewportOffset` (so that `offset + VISIBLE_CHANNELS`
/// never exceeds `TOTAL_CHANNELS`).
pub const MAX_VIEWPORT_OFFSET: usize = TOTAL_CHANNELS - VISIBLE_CHANNELS;

/// Fine adjustment step for volume, in decibels.
pub const VOLUME_FINE_STEP_DB: f32 = 0.5;

/// Coarse adjustment step for volume: 10x the fine step.
pub const VOLUME_COARSE_STEP_DB: f32 = VOLUME_FINE_STEP_DB * 10.0;

/// Fine adjustment step for pan.
pub const PAN_FINE_STEP: f32 = 0.02;

/// Coarse adjustment step for pan: 10x the fine step.
pub const PAN_COARSE_STEP: f32 = PAN_FINE_STEP * 10.0;

/// One row the mixer cursor can rest on within a channel strip.
///
/// `Volume` and `Pan` are continuous parameters: in edit mode, directional
/// input adjusts their value. `Mute` and `Solo` are toggle parameters:
/// their value changes ONLY via [`MixerViewEvent::ToggleFocusedParam`],
/// never via directional input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerParam {
    Volume,
    Pan,
    Mute,
    Solo,
}

impl MixerParam {
    /// Fixed top-to-bottom row order used for NavUp/NavDown traversal.
    const ORDER: [MixerParam; 4] = [
        MixerParam::Volume,
        MixerParam::Pan,
        MixerParam::Mute,
        MixerParam::Solo,
    ];

    fn row_index(self) -> usize {
        Self::ORDER
            .iter()
            .position(|p| *p == self)
            .expect("MixerParam::ORDER covers every variant")
    }

    /// Moves one row up, saturating at the first row.
    fn saturating_prev(self) -> Self {
        let idx = self.row_index();
        Self::ORDER[idx.saturating_sub(1)]
    }

    /// Moves one row down, saturating at the last row.
    fn saturating_next(self) -> Self {
        let idx = self.row_index();
        Self::ORDER[(idx + 1).min(Self::ORDER.len() - 1)]
    }

    /// True for parameters whose value is a continuous quantity adjustable
    /// by directional input while in edit mode (Volume, Pan). False for
    /// toggle parameters (Mute, Solo), which only change via
    /// `ToggleFocusedParam`.
    fn is_continuous(self) -> bool {
        matches!(self, MixerParam::Volume | MixerParam::Pan)
    }
}

/// Semantic events accepted by [`MixerView::apply`].
///
/// These are emitted identically by the keyboard and gamepad input
/// adapters; double-tap detection and Edit-hold timing are resolved by the
/// adapter before a `ToggleFocusedParam` / `EnterEditMode` / `ExitEditMode`
/// event ever reaches this reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerViewEvent {
    /// In navigate mode: move the parameter row up (saturating).
    /// In edit mode on a continuous param: increase by the coarse step.
    NavUp,
    /// In navigate mode: move the parameter row down (saturating).
    /// In edit mode on a continuous param: decrease by the coarse step.
    NavDown,
    /// In navigate mode: move to the previous channel (with edge-scroll).
    /// In edit mode on a continuous param: decrease by the fine step.
    NavLeft,
    /// In navigate mode: move to the next channel (with edge-scroll).
    /// In edit mode on a continuous param: increase by the fine step.
    NavRight,
    /// Enters edit mode. Alone, this changes no parameter value — it is a
    /// no-op until directional input arrives.
    EnterEditMode,
    /// Leaves edit mode, returning to navigation.
    ExitEditMode,
    /// Toggles the focused parameter if it is a toggle parameter (Mute or
    /// Solo). The input adapter raises this on an Edit double-tap. A no-op
    /// when the focused parameter is continuous (Volume, Pan).
    ToggleFocusedParam,
}

/// The single mixer-view store: owns cursor (channel + parameter), viewport
/// offset, edit-mode, and the 16 [`ChannelStrip`] channels.
///
/// `apply(MixerViewEvent)` is the ONLY way to mutate this type. Draw code
/// must never reach into a wrapped `ChannelStrip`'s fields directly.
#[derive(Debug, Clone, PartialEq)]
pub struct MixerView {
    channels: [ChannelStrip; TOTAL_CHANNELS],
    cursor_channel: usize,
    cursor_param: MixerParam,
    edit_mode: bool,
    viewport_offset: usize,
}

impl MixerView {
    /// Creates a view over 16 freshly-defaulted channel strips, cursor at
    /// channel 0 / Volume, navigate mode, viewport at the start.
    pub fn new() -> Self {
        Self::with_channels(std::array::from_fn(|_| ChannelStrip::new()))
    }

    /// Full constructor: builds a view over pre-existing channel strip
    /// state (e.g. when restoring a session). Callers that don't care
    /// about initial channel state should use [`MixerView::new`].
    pub fn with_channels(channels: [ChannelStrip; TOTAL_CHANNELS]) -> Self {
        Self {
            channels,
            cursor_channel: 0,
            cursor_param: MixerParam::Volume,
            edit_mode: false,
            viewport_offset: 0,
        }
    }

    /// The 16 wrapped channel strips, in order.
    pub fn channels(&self) -> &[ChannelStrip; TOTAL_CHANNELS] {
        &self.channels
    }

    /// The channel strip at absolute index `index`, if in range.
    pub fn channel(&self, index: usize) -> Option<&ChannelStrip> {
        self.channels.get(index)
    }

    /// The absolute index (0..=15) of the currently focused channel.
    pub fn cursor_channel(&self) -> usize {
        self.cursor_channel
    }

    /// The currently focused parameter row.
    pub fn cursor_param(&self) -> MixerParam {
        self.cursor_param
    }

    /// Whether the view is currently in edit mode.
    pub fn edit_mode(&self) -> bool {
        self.edit_mode
    }

    /// The first absolute channel index currently visible.
    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    /// The inclusive range of absolute channel indices currently visible.
    /// Always exactly [`VISIBLE_CHANNELS`] wide.
    pub fn visible_range(&self) -> std::ops::RangeInclusive<usize> {
        self.viewport_offset..=(self.viewport_offset + VISIBLE_CHANNELS - 1)
    }

    /// The only entry point that mutates a [`MixerView`]. All other
    /// mutation is forbidden; draw code must treat this type as read-only
    /// except through this method.
    pub fn apply(&mut self, event: MixerViewEvent) {
        match event {
            MixerViewEvent::NavUp => {
                if self.edit_mode {
                    self.adjust_focused(false, true);
                } else {
                    self.cursor_param = self.cursor_param.saturating_prev();
                }
            }
            MixerViewEvent::NavDown => {
                if self.edit_mode {
                    self.adjust_focused(false, false);
                } else {
                    self.cursor_param = self.cursor_param.saturating_next();
                }
            }
            MixerViewEvent::NavLeft => {
                if self.edit_mode {
                    self.adjust_focused(true, false);
                } else {
                    self.nav_channel_left();
                }
            }
            MixerViewEvent::NavRight => {
                if self.edit_mode {
                    self.adjust_focused(true, true);
                } else {
                    self.nav_channel_right();
                }
            }
            MixerViewEvent::EnterEditMode => {
                self.edit_mode = true;
            }
            MixerViewEvent::ExitEditMode => {
                self.edit_mode = false;
            }
            MixerViewEvent::ToggleFocusedParam => {
                self.toggle_focused_param();
            }
        }
    }

    /// Moves the cursor one channel to the left. At the left edge of the
    /// visible window, scrolls the viewport by one instead, keeping the
    /// cursor on the same absolute channel (now one position inward).
    /// No-op at channel 0 (the first channel).
    fn nav_channel_left(&mut self) {
        if self.cursor_channel == 0 {
            return;
        }
        if self.cursor_channel == self.viewport_offset && self.viewport_offset > 0 {
            self.viewport_offset -= 1;
        } else {
            self.cursor_channel -= 1;
        }
    }

    /// Moves the cursor one channel to the right. At the right edge of the
    /// visible window, scrolls the viewport by one instead, keeping the
    /// cursor on the same absolute channel (now one position inward).
    /// No-op at channel 15 (the last channel).
    fn nav_channel_right(&mut self) {
        if self.cursor_channel == TOTAL_CHANNELS - 1 {
            return;
        }
        let right_edge = self.viewport_offset + VISIBLE_CHANNELS - 1;
        if self.cursor_channel == right_edge && self.viewport_offset < MAX_VIEWPORT_OFFSET {
            self.viewport_offset += 1;
        } else {
            self.cursor_channel += 1;
        }
    }

    /// Adjusts the focused continuous parameter (Volume or Pan) on the
    /// focused channel strip by a fine or coarse step, in the given
    /// direction, clamped to that parameter's valid domain range. A no-op
    /// if the focused parameter is a toggle parameter (Mute, Solo), if the
    /// cursor addresses an out-of-range channel, or if the clamped value
    /// somehow fails validation.
    fn adjust_focused(&mut self, fine: bool, positive: bool) {
        if !self.cursor_param.is_continuous() {
            return;
        }
        let param = self.cursor_param;
        let Some(strip) = self.channels.get_mut(self.cursor_channel) else {
            return;
        };
        match param {
            MixerParam::Volume => {
                let step = if fine {
                    VOLUME_FINE_STEP_DB
                } else {
                    VOLUME_COARSE_STEP_DB
                };
                let delta = if positive { step } else { -step };
                let raw = strip.volume_db().value() + delta;
                let clamped = raw.clamp(Decibel::MIN, Decibel::MAX);
                if let Ok(volume_db) = Decibel::try_new(clamped) {
                    let _ = strip.handle(ChannelStripCommand::SetVolume { volume_db });
                }
            }
            MixerParam::Pan => {
                let step = if fine { PAN_FINE_STEP } else { PAN_COARSE_STEP };
                let delta = if positive { step } else { -step };
                let raw = strip.pan().value() + delta;
                let clamped = raw.clamp(Pan::MIN, Pan::MAX);
                if let Ok(pan) = Pan::try_new(clamped) {
                    let _ = strip.handle(ChannelStripCommand::SetPan { pan });
                }
            }
            MixerParam::Mute | MixerParam::Solo => {}
        }
    }

    /// Toggles the focused toggle parameter (Mute or Solo) on the focused
    /// channel strip. A no-op if the focused parameter is continuous
    /// (Volume, Pan) or if the cursor addresses an out-of-range channel.
    fn toggle_focused_param(&mut self) {
        let param = self.cursor_param;
        let Some(strip) = self.channels.get_mut(self.cursor_channel) else {
            return;
        };
        match param {
            MixerParam::Mute => {
                let mute = !strip.mute();
                let _ = strip.handle(ChannelStripCommand::SetMute { mute });
            }
            MixerParam::Solo => {
                let solo = !strip.solo();
                let _ = strip.handle(ChannelStripCommand::SetSolo { solo });
            }
            MixerParam::Volume | MixerParam::Pan => {}
        }
    }
}

impl Default for MixerView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_view_starts_at_channel_zero_volume_navigate_mode() {
        let view = MixerView::new();
        assert_eq!(view.cursor_channel(), 0);
        assert_eq!(view.cursor_param(), MixerParam::Volume);
        assert!(!view.edit_mode());
        assert_eq!(view.viewport_offset(), 0);
        assert_eq!(*view.visible_range().start(), 0);
        assert_eq!(*view.visible_range().end(), VISIBLE_CHANNELS - 1);
    }

    #[test]
    fn nav_down_moves_through_rows_and_saturates_at_last_row() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::NavDown);
        assert_eq!(view.cursor_param(), MixerParam::Pan);
        view.apply(MixerViewEvent::NavDown);
        assert_eq!(view.cursor_param(), MixerParam::Mute);
        view.apply(MixerViewEvent::NavDown);
        assert_eq!(view.cursor_param(), MixerParam::Solo);
        // Saturates: one more NavDown stays at Solo.
        view.apply(MixerViewEvent::NavDown);
        assert_eq!(view.cursor_param(), MixerParam::Solo);
    }

    #[test]
    fn nav_up_saturates_at_first_row() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::NavUp);
        assert_eq!(view.cursor_param(), MixerParam::Volume);
    }

    #[test]
    fn nav_right_moves_cursor_within_visible_window() {
        let mut view = MixerView::new();
        for _ in 0..3 {
            view.apply(MixerViewEvent::NavRight);
        }
        assert_eq!(view.cursor_channel(), 3);
        assert_eq!(view.viewport_offset(), 0);
    }

    #[test]
    fn nav_right_at_visible_right_edge_scrolls_viewport_keeping_absolute_channel() {
        let mut view = MixerView::new();
        for _ in 0..(VISIBLE_CHANNELS - 1) {
            view.apply(MixerViewEvent::NavRight);
        }
        assert_eq!(view.cursor_channel(), VISIBLE_CHANNELS - 1);
        assert_eq!(view.viewport_offset(), 0);

        view.apply(MixerViewEvent::NavRight);
        assert_eq!(view.cursor_channel(), VISIBLE_CHANNELS - 1);
        assert_eq!(view.viewport_offset(), 1);
        assert!(view.visible_range().contains(&view.cursor_channel()));
    }

    #[test]
    fn nav_right_is_noop_at_last_channel() {
        let mut view = MixerView::new();
        // Scrolling and cursor movement alternate once the cursor first
        // reaches a visible edge, so reaching the last channel takes more
        // than TOTAL_CHANNELS presses; TOTAL_CHANNELS * 2 is ample and any
        // presses past the last channel are no-ops.
        for _ in 0..(TOTAL_CHANNELS * 2) {
            view.apply(MixerViewEvent::NavRight);
        }
        assert_eq!(view.cursor_channel(), TOTAL_CHANNELS - 1);
        assert_eq!(view.viewport_offset(), MAX_VIEWPORT_OFFSET);

        view.apply(MixerViewEvent::NavRight);
        assert_eq!(view.cursor_channel(), TOTAL_CHANNELS - 1);
        assert_eq!(view.viewport_offset(), MAX_VIEWPORT_OFFSET);
    }

    #[test]
    fn nav_left_is_noop_at_first_channel() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::NavLeft);
        assert_eq!(view.cursor_channel(), 0);
        assert_eq!(view.viewport_offset(), 0);
    }

    #[test]
    fn nav_left_at_visible_left_edge_scrolls_viewport_keeping_absolute_channel() {
        let mut view = MixerView::new();
        for _ in 0..(TOTAL_CHANNELS * 2) {
            view.apply(MixerViewEvent::NavRight);
        }
        assert_eq!(view.cursor_channel(), TOTAL_CHANNELS - 1);
        assert_eq!(view.viewport_offset(), MAX_VIEWPORT_OFFSET);

        // Move cursor back to the left edge of the visible window.
        for _ in 0..(VISIBLE_CHANNELS - 1) {
            view.apply(MixerViewEvent::NavLeft);
        }
        assert_eq!(view.cursor_channel(), MAX_VIEWPORT_OFFSET);
        assert_eq!(view.viewport_offset(), MAX_VIEWPORT_OFFSET);

        view.apply(MixerViewEvent::NavLeft);
        assert_eq!(view.cursor_channel(), MAX_VIEWPORT_OFFSET);
        assert_eq!(view.viewport_offset(), MAX_VIEWPORT_OFFSET - 1);
        assert!(view.visible_range().contains(&view.cursor_channel()));
    }

    #[test]
    fn cursor_always_stays_within_visible_window_during_full_traversal() {
        let mut view = MixerView::new();
        for _ in 0..(TOTAL_CHANNELS * 2) {
            view.apply(MixerViewEvent::NavRight);
            assert!(view.visible_range().contains(&view.cursor_channel()));
        }
        for _ in 0..(TOTAL_CHANNELS * 2) {
            view.apply(MixerViewEvent::NavLeft);
            assert!(view.visible_range().contains(&view.cursor_channel()));
        }
    }

    #[test]
    fn enter_edit_mode_alone_changes_no_value() {
        let mut view = MixerView::new();
        let before = view.channel(0).unwrap().volume_db();
        view.apply(MixerViewEvent::EnterEditMode);
        assert!(view.edit_mode());
        assert_eq!(view.channel(0).unwrap().volume_db(), before);
    }

    #[test]
    fn edit_mode_right_adjusts_volume_by_fine_step() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::EnterEditMode);
        view.apply(MixerViewEvent::NavRight);
        let expected = Decibel::unity().value() + VOLUME_FINE_STEP_DB;
        assert!((view.channel(0).unwrap().volume_db().value() - expected).abs() < 1e-4);
    }

    #[test]
    fn edit_mode_up_adjusts_volume_by_coarse_step() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::EnterEditMode);
        view.apply(MixerViewEvent::NavUp);
        let expected = Decibel::unity().value() + VOLUME_COARSE_STEP_DB;
        assert!((view.channel(0).unwrap().volume_db().value() - expected).abs() < 1e-4);
    }

    #[test]
    fn edit_mode_volume_clamps_at_max() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::EnterEditMode);
        for _ in 0..50 {
            view.apply(MixerViewEvent::NavUp);
        }
        assert_eq!(view.channel(0).unwrap().volume_db().value(), Decibel::MAX);
    }

    #[test]
    fn edit_mode_volume_clamps_at_min() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::EnterEditMode);
        for _ in 0..50 {
            view.apply(MixerViewEvent::NavDown);
        }
        assert_eq!(view.channel(0).unwrap().volume_db().value(), Decibel::MIN);
    }

    #[test]
    fn edit_mode_pan_fine_and_coarse_steps_and_clamping() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::NavDown); // cursor_param -> Pan
        view.apply(MixerViewEvent::EnterEditMode);
        view.apply(MixerViewEvent::NavRight);
        let expected = Pan::center().value() + PAN_FINE_STEP;
        assert!((view.channel(0).unwrap().pan().value() - expected).abs() < 1e-4);

        for _ in 0..50 {
            view.apply(MixerViewEvent::NavUp);
        }
        assert_eq!(view.channel(0).unwrap().pan().value(), Pan::MAX);
    }

    #[test]
    fn edit_mode_directional_input_never_toggles_mute_or_solo() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::NavDown); // Pan
        view.apply(MixerViewEvent::NavDown); // Mute
        view.apply(MixerViewEvent::EnterEditMode);
        view.apply(MixerViewEvent::NavUp);
        view.apply(MixerViewEvent::NavDown);
        view.apply(MixerViewEvent::NavLeft);
        view.apply(MixerViewEvent::NavRight);
        assert!(!view.channel(0).unwrap().mute());
    }

    #[test]
    fn toggle_focused_param_toggles_mute() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::NavDown); // Pan
        view.apply(MixerViewEvent::NavDown); // Mute
        view.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(view.channel(0).unwrap().mute());
        view.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(!view.channel(0).unwrap().mute());
    }

    #[test]
    fn toggle_focused_param_toggles_solo() {
        let mut view = MixerView::new();
        for _ in 0..3 {
            view.apply(MixerViewEvent::NavDown);
        }
        assert_eq!(view.cursor_param(), MixerParam::Solo);
        view.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(view.channel(0).unwrap().solo());
    }

    #[test]
    fn toggle_focused_param_is_noop_on_continuous_param() {
        let mut view = MixerView::new();
        let before = view.channel(0).unwrap().volume_db();
        view.apply(MixerViewEvent::ToggleFocusedParam);
        assert_eq!(view.channel(0).unwrap().volume_db(), before);
    }

    #[test]
    fn exit_edit_mode_returns_to_navigate_mode() {
        let mut view = MixerView::new();
        view.apply(MixerViewEvent::EnterEditMode);
        assert!(view.edit_mode());
        view.apply(MixerViewEvent::ExitEditMode);
        assert!(!view.edit_mode());
    }

    #[test]
    fn wraps_exactly_sixteen_channels() {
        let view = MixerView::new();
        assert_eq!(view.channels().len(), TOTAL_CHANNELS);
        assert!(view.channel(TOTAL_CHANNELS).is_none());
    }

    #[test]
    fn metering_is_independent_of_solo_and_mute() {
        use crate::mixer::channel_strip::Amplitude;

        let mut channels: [ChannelStrip; TOTAL_CHANNELS] =
            std::array::from_fn(|_| ChannelStrip::new());
        channels[0]
            .handle(ChannelStripCommand::SetMute { mute: true })
            .unwrap();
        channels[1]
            .handle(ChannelStripCommand::SetSolo { solo: true })
            .unwrap();
        let mut view = MixerView::with_channels(channels);

        // Meter a muted, non-soloed channel while another channel is
        // soloed: it must still report a real (non-silent) peak.
        let peak = view.channels[0].meter(Amplitude::unity());
        assert!(peak.value() > 0.0);
    }
}
