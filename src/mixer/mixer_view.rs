// path: src/mixer/mixer_view.rs

use crate::mixer::channel_mixer::ChannelMixer;
use crate::mixer::mixer_param::MixerParam;
use crate::mixer::mixer_view_event::MixerViewEvent;

/// Total number of channels in the mixer.
pub const CHANNEL_COUNT: usize = 16;

/// Number of channels visible in the mixer viewport at one time.
pub const VISIBLE_CHANNELS: usize = 6;

/// Fine adjustment step for continuous parameters (one unit on a 0–100 readout).
const FINE_STEP: f32 = 0.01;

/// Coarse adjustment step (10x fine).
const COARSE_STEP: f32 = 0.10;

/// Maximum value of `viewport_offset` (so the window [offset, offset+5] always
/// contains 16 channels without going past the last).
pub const MAX_VIEWPORT_OFFSET: usize = CHANNEL_COUNT - VISIBLE_CHANNELS; // 10

/// Flux-style store for the entire mixer view.
///
/// `apply` is the **sole** mutation entry point — there are no setters and no
/// other public path that changes any field.  The store is pure and
/// allocation-free: no I/O, no rendering, no audio.
///
/// ## State
///
/// | Field             | Range         | Meaning                                    |
/// |-------------------|---------------|--------------------------------------------||
/// | `cursor_channel`  | 0..=15        | Absolute channel index under the cursor    |
/// | `cursor_param`    | `MixerParam`  | Parameter row under the cursor             |
/// | `viewport_offset` | 0..=10        | Index of the first visible channel         |
/// | `edit_mode`       | bool          | `true` → directional input edits values    |
/// | `mixer`           | `ChannelMixer`| All 16 channels' state                     |
///
/// The visible window is always `[viewport_offset, viewport_offset + 5]` (6
/// channels).  The cursor is **always** inside this window.
///
/// ## Navigation (edit_mode = false)
///
/// * `NavUp` / `NavDown` — move `cursor_param` one row (saturating).
/// * `NavRight` — advance one channel with edge-scrolling:
///   - cursor is **not** at the right edge → move cursor one step right.
///   - cursor **is** at the right edge and window is **not** at the end →
///     scroll the viewport one step right (cursor stays on the same absolute
///     channel, now one position inward).
///   - already at channel 16 (index 15) → no-op.
/// * `NavLeft` — mirror of `NavRight`.
///
/// ## Edit mode (edit_mode = true)
///
/// Directional input on a **continuous** param adjusts the focused channel:
///
/// | Event     | Effect                      |
/// |-----------|-----------------------------||
/// | `NavLeft` | value −= fine (−0.01)       |
/// | `NavRight`| value += fine (+0.01)       |
/// | `NavDown` | value −= coarse (−0.10)     |
/// | `NavUp`   | value += coarse (+0.10)     |
///
/// On a **toggle** param (`Mute` / `Solo`) directional input is a **no-op**.
///
/// ## Toggling
///
/// `ToggleFocusedParam` (double-tap Edit) toggles `Mute` or `Solo` on the
/// focused channel regardless of mode.  On a continuous param it is a no-op.
///
/// ## Examples
///
/// ```
/// use crest_synth::mixer::mixer_view::MixerView;
/// use crest_synth::mixer::mixer_view_event::MixerViewEvent;
/// use crest_synth::mixer::mixer_param::MixerParam;
///
/// let mut view = MixerView::new();
/// assert_eq!(view.cursor_channel(), 0);
/// assert_eq!(view.cursor_param(), MixerParam::Volume);
/// assert!(!view.edit_mode());
///
/// // Navigate right
/// view.apply(MixerViewEvent::NavRight);
/// assert_eq!(view.cursor_channel(), 1);
///
/// // Enter edit mode and adjust volume fine
/// view.apply(MixerViewEvent::EnterEditMode);
/// assert!(view.edit_mode());
/// view.apply(MixerViewEvent::NavRight);
/// let vol = view.mixer().channel(1).volume;
/// assert!((vol - 0.76).abs() < 1e-5);
/// ```
#[derive(Debug, Clone)]
pub struct MixerView {
    cursor_channel: usize,
    cursor_param: MixerParam,
    edit_mode: bool,
    mixer: ChannelMixer,
    viewport_offset: usize,
}

impl MixerView {
    /// Create a new `MixerView` with default state.
    ///
    /// Cursor starts at channel 0, `Volume` row, navigate mode, viewport at 0.
    pub fn new() -> Self {
        Self {
            cursor_channel: 0,
            cursor_param: MixerParam::Volume,
            edit_mode: false,
            mixer: ChannelMixer::new(),
            viewport_offset: 0,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    /// The absolute channel index (0..=15) currently under the cursor.
    pub fn cursor_channel(&self) -> usize {
        self.cursor_channel
    }

    /// The parameter row currently under the cursor.
    pub fn cursor_param(&self) -> MixerParam {
        self.cursor_param
    }

    /// `true` when the view is in edit mode.
    pub fn edit_mode(&self) -> bool {
        self.edit_mode
    }

    /// The index of the first channel visible in the viewport (0..=10).
    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    /// Read-only access to the underlying `ChannelMixer`.
    pub fn mixer(&self) -> &ChannelMixer {
        &self.mixer
    }

    // ── Single mutation entry point ───────────────────────────────────────────

    /// Apply a `MixerViewEvent`, mutating the store in place.
    ///
    /// This is the **only** public mutation method.  It is pure and
    /// allocation-free.
    pub fn apply(&mut self, event: MixerViewEvent) {
        match event {
            MixerViewEvent::EnterEditMode => {
                self.edit_mode = true;
            }
            MixerViewEvent::ExitEditMode => {
                self.edit_mode = false;
            }
            MixerViewEvent::ToggleFocusedParam => {
                self.apply_toggle();
            }
            MixerViewEvent::NavUp
            | MixerViewEvent::NavDown
            | MixerViewEvent::NavLeft
            | MixerViewEvent::NavRight => {
                if self.edit_mode {
                    self.apply_edit(event);
                } else {
                    self.apply_navigate(event);
                }
            }
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Handle navigation events (navigate mode).
    fn apply_navigate(&mut self, event: MixerViewEvent) {
        match event {
            MixerViewEvent::NavUp => {
                self.cursor_param = self.cursor_param.prev();
            }
            MixerViewEvent::NavDown => {
                self.cursor_param = self.cursor_param.next();
            }
            MixerViewEvent::NavRight => {
                self.navigate_right();
            }
            MixerViewEvent::NavLeft => {
                self.navigate_left();
            }
            _ => {}
        }
    }

    /// Move the cursor (or viewport) one step to the right.
    fn navigate_right(&mut self) {
        let right_edge = self.viewport_offset + (VISIBLE_CHANNELS - 1);
        if self.cursor_channel < right_edge {
            // Cursor is NOT at the visible right edge — move it.
            self.cursor_channel += 1;
        } else if right_edge < CHANNEL_COUNT - 1 {
            // Cursor IS at the right edge and the viewport can scroll right.
            // Scroll the viewport; cursor stays on the same absolute channel
            // (now one position inward from the new right edge).
            self.viewport_offset += 1;
            // cursor_channel is unchanged; it is now one position inward.
        }
        // else: already at channel 16 (index 15) — no-op.
    }

    /// Move the cursor (or viewport) one step to the left.
    fn navigate_left(&mut self) {
        if self.cursor_channel > self.viewport_offset {
            // Cursor is NOT at the visible left edge — move it.
            self.cursor_channel -= 1;
        } else if self.viewport_offset > 0 {
            // Cursor IS at the left edge and the viewport can scroll left.
            self.viewport_offset -= 1;
            // cursor_channel unchanged (one position inward from new left edge).
        }
        // else: already at channel 1 (index 0) — no-op.
    }

    /// Handle directional events in edit mode.
    fn apply_edit(&mut self, event: MixerViewEvent) {
        if self.cursor_param.is_toggle() {
            // Toggle params are not adjustable by directional input.
            return;
        }
        let delta = match event {
            MixerViewEvent::NavLeft => -FINE_STEP,
            MixerViewEvent::NavRight => FINE_STEP,
            MixerViewEvent::NavUp => COARSE_STEP,
            MixerViewEvent::NavDown => -COARSE_STEP,
            _ => return,
        };
        self.mixer
            .adjust(self.cursor_channel, self.cursor_param, delta);
    }

    /// Handle `ToggleFocusedParam` — toggles Mute or Solo, no-op otherwise.
    fn apply_toggle(&mut self) {
        match self.cursor_param {
            MixerParam::Mute => {
                self.mixer.toggle_mute(self.cursor_channel);
            }
            MixerParam::Solo => {
                self.mixer.toggle_solo(self.cursor_channel);
            }
            _ => {
                // Continuous params are not toggled by this event.
            }
        }
    }
}

impl Default for MixerView {
    fn default() -> Self {
        Self::new()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod mixer_view_tests {
    use super::*;
    use crate::mixer::mixer_view_event::MixerViewEvent;

    // ── Initial state ────────────────────────────────────────────────────────

    #[test]
    fn mixer_view_initial_state() {
        let v = MixerView::new();
        assert_eq!(v.cursor_channel(), 0);
        assert_eq!(v.cursor_param(), MixerParam::Volume);
        assert!(!v.edit_mode());
        assert_eq!(v.viewport_offset(), 0);
    }

    // ── Mode toggling ────────────────────────────────────────────────────────

    #[test]
    fn mixer_view_enter_edit_mode() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::EnterEditMode);
        assert!(v.edit_mode());
    }

    #[test]
    fn mixer_view_exit_edit_mode() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::EnterEditMode);
        v.apply(MixerViewEvent::ExitEditMode);
        assert!(!v.edit_mode());
    }

    #[test]
    fn mixer_view_enter_edit_mode_no_param_change() {
        let mut v = MixerView::new();
        let vol_before = v.mixer().channel(0).volume;
        v.apply(MixerViewEvent::EnterEditMode);
        assert_eq!(v.mixer().channel(0).volume, vol_before);
        assert_eq!(v.cursor_channel(), 0);
        assert_eq!(v.cursor_param(), MixerParam::Volume);
    }

    // ── Navigate mode: param rows (NavUp / NavDown) ──────────────────────────

    #[test]
    fn mixer_view_nav_down_moves_param_row() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::NavDown);
        assert_eq!(v.cursor_param(), MixerParam::ReverbSend);
    }

    #[test]
    fn mixer_view_nav_up_moves_param_row() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::NavDown);
        v.apply(MixerViewEvent::NavUp);
        assert_eq!(v.cursor_param(), MixerParam::Volume);
    }

    #[test]
    fn mixer_view_nav_up_saturates_at_volume() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::NavUp);
        assert_eq!(v.cursor_param(), MixerParam::Volume);
    }

    #[test]
    fn mixer_view_nav_down_saturates_at_solo() {
        let mut v = MixerView::new();
        for _ in 0..10 {
            v.apply(MixerViewEvent::NavDown);
        }
        assert_eq!(v.cursor_param(), MixerParam::Solo);
    }

    #[test]
    fn mixer_view_nav_through_all_rows() {
        let mut v = MixerView::new();
        let rows = [
            MixerParam::Volume,
            MixerParam::ReverbSend,
            MixerParam::EchoSend,
            MixerParam::Pan,
            MixerParam::Mute,
            MixerParam::Solo,
        ];
        for expected in &rows {
            assert_eq!(v.cursor_param(), *expected);
            v.apply(MixerViewEvent::NavDown);
        }
        assert_eq!(v.cursor_param(), MixerParam::Solo);
    }

    // ── Navigate mode: channel columns (NavLeft / NavRight, no edge scroll) ──

    #[test]
    fn mixer_view_nav_right_moves_cursor_channel() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::NavRight);
        assert_eq!(v.cursor_channel(), 1);
        assert_eq!(v.viewport_offset(), 0);
    }

    #[test]
    fn mixer_view_nav_left_saturates_at_channel_zero() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::NavLeft);
        assert_eq!(v.cursor_channel(), 0);
        assert_eq!(v.viewport_offset(), 0);
    }

    #[test]
    fn mixer_view_nav_right_saturates_at_channel_15() {
        let mut v = MixerView::new();
        // To reach channel 15: first 5 presses move cursor from 0→5 (no scroll),
        // then each subsequent 2 presses = 1 scroll + 1 channel advance.
        // 5 + 10*2 = 25 total presses to reach channel 15, viewport 10.
        for _ in 0..25 {
            v.apply(MixerViewEvent::NavRight);
        }
        assert_eq!(v.cursor_channel(), 15);
        assert_eq!(v.viewport_offset(), 10);
        // Additional NavRight is a no-op (already at channel 16 = index 15)
        v.apply(MixerViewEvent::NavRight);
        assert_eq!(v.cursor_channel(), 15);
        assert_eq!(v.viewport_offset(), 10);
    }

    // ── Edge scrolling ───────────────────────────────────────────────────────

    #[test]
    fn mixer_view_right_edge_scroll() {
        // Start with cursor at the right edge of the visible window.
        // Window [0..5], cursor at channel 5 (right edge).
        let mut v = MixerView::new();
        for _ in 0..5 {
            v.apply(MixerViewEvent::NavRight);
        }
        assert_eq!(v.cursor_channel(), 5);
        assert_eq!(v.viewport_offset(), 0);

        // NavRight when at right edge: scroll viewport, cursor stays.
        v.apply(MixerViewEvent::NavRight);
        assert_eq!(
            v.cursor_channel(),
            5,
            "cursor stays on same absolute channel"
        );
        assert_eq!(v.viewport_offset(), 1, "viewport scrolled right by 1");
    }

    #[test]
    fn mixer_view_left_edge_scroll() {
        // Set up: move to channel 5, then scroll so that the viewport is at 1
        // and cursor is at channel 5 (one inward from right edge).
        // Then move left to the left edge and scroll.
        let mut v = MixerView::new();
        for _ in 0..5 {
            v.apply(MixerViewEvent::NavRight);
        }
        v.apply(MixerViewEvent::NavRight); // scrolls viewport to 1, cursor stays 5
        assert_eq!(v.viewport_offset(), 1);
        assert_eq!(v.cursor_channel(), 5);

        // Now move left from channel 5 to reach the left edge (channel 1 = index 1 = viewport_offset).
        for _ in 0..4 {
            v.apply(MixerViewEvent::NavLeft);
        }
        assert_eq!(v.cursor_channel(), 1);
        assert_eq!(v.viewport_offset(), 1);

        // At left edge — NavLeft scrolls viewport left, cursor stays.
        v.apply(MixerViewEvent::NavLeft);
        assert_eq!(
            v.cursor_channel(),
            1,
            "cursor stays on same absolute channel"
        );
        assert_eq!(v.viewport_offset(), 0, "viewport scrolled left by 1");
    }

    #[test]
    fn mixer_view_example_from_spec() {
        // "window 0..5 showing channels 1–6, cursor on channel 6 = index 5"
        // NavRight → viewportOffset becomes 1, cursorChannel stays 5.
        let mut v = MixerView::new();
        // Move cursor to index 5 (right edge of window [0..5])
        for _ in 0..5 {
            v.apply(MixerViewEvent::NavRight);
        }
        assert_eq!(v.cursor_channel(), 5);
        assert_eq!(v.viewport_offset(), 0);

        v.apply(MixerViewEvent::NavRight);
        assert_eq!(v.viewport_offset(), 1);
        assert_eq!(v.cursor_channel(), 5);
    }

    #[test]
    fn mixer_view_cursor_always_in_visible_window() {
        // Walk across all channels and verify the invariant holds.
        let mut v = MixerView::new();
        for _ in 0..20 {
            v.apply(MixerViewEvent::NavRight);
            let lo = v.viewport_offset();
            let hi = lo + VISIBLE_CHANNELS - 1;
            assert!(
                (lo..=hi).contains(&v.cursor_channel()),
                "cursor {} not in [{}, {}]",
                v.cursor_channel(),
                lo,
                hi
            );
        }
        for _ in 0..20 {
            v.apply(MixerViewEvent::NavLeft);
            let lo = v.viewport_offset();
            let hi = lo + VISIBLE_CHANNELS - 1;
            assert!(
                (lo..=hi).contains(&v.cursor_channel()),
                "cursor {} not in [{}, {}]",
                v.cursor_channel(),
                lo,
                hi
            );
        }
    }

    #[test]
    fn mixer_view_viewport_stays_in_range() {
        let mut v = MixerView::new();
        for _ in 0..30 {
            v.apply(MixerViewEvent::NavRight);
            assert!(v.viewport_offset() <= MAX_VIEWPORT_OFFSET);
        }
        for _ in 0..30 {
            v.apply(MixerViewEvent::NavLeft);
            assert!(v.viewport_offset() <= MAX_VIEWPORT_OFFSET);
        }
    }

    // ── Edit mode: continuous params ─────────────────────────────────────────

    #[test]
    fn mixer_view_edit_nav_right_fine_increase() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::EnterEditMode);
        let before = v.mixer().channel(0).volume;
        v.apply(MixerViewEvent::NavRight);
        let after = v.mixer().channel(0).volume;
        assert!((after - (before + FINE_STEP)).abs() < 1e-6);
    }

    #[test]
    fn mixer_view_edit_nav_left_fine_decrease() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::EnterEditMode);
        let before = v.mixer().channel(0).volume;
        v.apply(MixerViewEvent::NavLeft);
        let after = v.mixer().channel(0).volume;
        assert!((after - (before - FINE_STEP)).abs() < 1e-6);
    }

    #[test]
    fn mixer_view_edit_nav_up_coarse_increase() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::EnterEditMode);
        let before = v.mixer().channel(0).volume;
        v.apply(MixerViewEvent::NavUp);
        let after = v.mixer().channel(0).volume;
        assert!((after - (before + COARSE_STEP)).abs() < 1e-6);
    }

    #[test]
    fn mixer_view_edit_nav_down_coarse_decrease() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::EnterEditMode);
        let before = v.mixer().channel(0).volume;
        v.apply(MixerViewEvent::NavDown);
        let after = v.mixer().channel(0).volume;
        assert!((after - (before - COARSE_STEP)).abs() < 1e-6);
    }

    #[test]
    fn mixer_view_coarse_is_10x_fine() {
        // Use a reasonable float tolerance for 0.10 vs 10.0 * 0.01
        assert!((COARSE_STEP - 10.0 * FINE_STEP).abs() < 1e-6);
    }

    #[test]
    fn mixer_view_edit_adjusts_focused_channel_only() {
        let mut v = MixerView::new();
        // Move to channel 3
        for _ in 0..3 {
            v.apply(MixerViewEvent::NavRight);
        }
        v.apply(MixerViewEvent::EnterEditMode);
        let vol0_before = v.mixer().channel(0).volume;
        v.apply(MixerViewEvent::NavRight);
        // Channel 0 unchanged, channel 3 changed.
        assert_eq!(v.mixer().channel(0).volume, vol0_before);
        let vol3 = v.mixer().channel(3).volume;
        assert!((vol3 - (0.75 + FINE_STEP)).abs() < 1e-6);
    }

    #[test]
    fn mixer_view_edit_volume_clamps_at_max() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::EnterEditMode);
        for _ in 0..200 {
            v.apply(MixerViewEvent::NavRight);
        }
        assert!((v.mixer().channel(0).volume - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mixer_view_edit_volume_clamps_at_min() {
        let mut v = MixerView::new();
        v.apply(MixerViewEvent::EnterEditMode);
        for _ in 0..200 {
            v.apply(MixerViewEvent::NavLeft);
        }
        assert!((v.mixer().channel(0).volume - 0.0).abs() < 1e-6);
    }

    // ── Edit mode: toggle params are a no-op for directional input ───────────

    #[test]
    fn mixer_view_edit_mute_directional_is_noop() {
        let mut v = MixerView::new();
        // Navigate to Mute row
        for _ in 0..4 {
            v.apply(MixerViewEvent::NavDown);
        }
        assert_eq!(v.cursor_param(), MixerParam::Mute);
        v.apply(MixerViewEvent::EnterEditMode);
        let mute_before = v.mixer().channel(0).mute;
        v.apply(MixerViewEvent::NavLeft);
        v.apply(MixerViewEvent::NavRight);
        v.apply(MixerViewEvent::NavUp);
        v.apply(MixerViewEvent::NavDown);
        assert_eq!(v.mixer().channel(0).mute, mute_before);
    }

    #[test]
    fn mixer_view_edit_solo_directional_is_noop() {
        let mut v = MixerView::new();
        // Navigate to Solo row
        for _ in 0..5 {
            v.apply(MixerViewEvent::NavDown);
        }
        assert_eq!(v.cursor_param(), MixerParam::Solo);
        v.apply(MixerViewEvent::EnterEditMode);
        let solo_before = v.mixer().channel(0).solo;
        v.apply(MixerViewEvent::NavLeft);
        v.apply(MixerViewEvent::NavRight);
        assert_eq!(v.mixer().channel(0).solo, solo_before);
    }

    // ── ToggleFocusedParam ───────────────────────────────────────────────────

    #[test]
    fn mixer_view_toggle_mute() {
        let mut v = MixerView::new();
        // Navigate to Mute row
        for _ in 0..4 {
            v.apply(MixerViewEvent::NavDown);
        }
        assert_eq!(v.cursor_param(), MixerParam::Mute);
        assert!(!v.mixer().channel(0).mute);
        v.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(v.mixer().channel(0).mute);
        v.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(!v.mixer().channel(0).mute);
    }

    #[test]
    fn mixer_view_toggle_solo() {
        let mut v = MixerView::new();
        for _ in 0..5 {
            v.apply(MixerViewEvent::NavDown);
        }
        assert_eq!(v.cursor_param(), MixerParam::Solo);
        assert!(!v.mixer().channel(0).solo);
        v.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(v.mixer().channel(0).solo);
        v.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(!v.mixer().channel(0).solo);
    }

    #[test]
    fn mixer_view_toggle_continuous_is_noop() {
        let mut v = MixerView::new();
        assert_eq!(v.cursor_param(), MixerParam::Volume);
        let vol_before = v.mixer().channel(0).volume;
        v.apply(MixerViewEvent::ToggleFocusedParam);
        assert_eq!(v.mixer().channel(0).volume, vol_before);
    }

    #[test]
    fn mixer_view_toggle_in_navigate_mode_works() {
        let mut v = MixerView::new();
        for _ in 0..4 {
            v.apply(MixerViewEvent::NavDown);
        }
        // Not in edit mode
        assert!(!v.edit_mode());
        v.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(v.mixer().channel(0).mute);
    }

    #[test]
    fn mixer_view_toggle_in_edit_mode_works() {
        let mut v = MixerView::new();
        for _ in 0..4 {
            v.apply(MixerViewEvent::NavDown);
        }
        v.apply(MixerViewEvent::EnterEditMode);
        v.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(v.mixer().channel(0).mute);
    }

    #[test]
    fn mixer_view_toggle_targets_focused_channel() {
        let mut v = MixerView::new();
        // Move to channel 2
        v.apply(MixerViewEvent::NavRight);
        v.apply(MixerViewEvent::NavRight);
        assert_eq!(v.cursor_channel(), 2);
        // Navigate to Mute row
        for _ in 0..4 {
            v.apply(MixerViewEvent::NavDown);
        }
        v.apply(MixerViewEvent::ToggleFocusedParam);
        assert!(!v.mixer().channel(0).mute);
        assert!(v.mixer().channel(2).mute);
    }

    // ── Invariant: cursor channel stays in 0..=15 ────────────────────────────

    #[test]
    fn mixer_view_cursor_channel_stays_in_range() {
        let mut v = MixerView::new();
        for _ in 0..50 {
            v.apply(MixerViewEvent::NavRight);
        }
        assert!(v.cursor_channel() <= 15);
        for _ in 0..50 {
            v.apply(MixerViewEvent::NavLeft);
        }
        assert!(v.cursor_channel() <= 15);
    }

    // ── Default impl ─────────────────────────────────────────────────────────

    #[test]
    fn mixer_view_default_equals_new() {
        let a = MixerView::new();
        let b = MixerView::default();
        assert_eq!(a.cursor_channel(), b.cursor_channel());
        assert_eq!(a.cursor_param(), b.cursor_param());
        assert_eq!(a.edit_mode(), b.edit_mode());
        assert_eq!(a.viewport_offset(), b.viewport_offset());
    }
}
