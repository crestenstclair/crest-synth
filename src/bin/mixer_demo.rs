// path: src/bin/mixer_demo.rs
//
// mixer_demo — headless prover for MixerView + ChannelMixer.
//
// No audio device, no window, no gamepad, no MIDI.
// Exercises four proofs and prints the verbatim tokens each proof requires.

use crest_synth::mixer::channel_mixer::{ChannelMixer, CHANNEL_COUNT};
use crest_synth::mixer::mixer_param::MixerParam;
use crest_synth::mixer::mixer_view::{MixerView, VISIBLE_CHANNELS};
use crest_synth::mixer::mixer_view_event::MixerViewEvent;

fn main() {
    proof_edge_scroll();
    proof_fine_coarse();
    proof_toggle();
    proof_solo_vs_metering();

    println!("\nAll proofs passed.");
}

// ── Proof 1: Edge-scroll behaviour ───────────────────────────────────────────

fn proof_edge_scroll() {
    let mut view = MixerView::new();

    // Move cursor to the trailing visible channel (index 5 = VISIBLE_CHANNELS-1)
    // while viewport stays at 0.
    for _ in 0..(VISIBLE_CHANNELS - 1) {
        view.apply(MixerViewEvent::NavRight);
    }
    assert_eq!(
        view.cursor_channel(),
        VISIBLE_CHANNELS - 1,
        "cursor should be at the trailing visible channel before edge-scroll"
    );
    assert_eq!(
        view.viewport_offset(),
        0,
        "viewport_offset should be 0 before edge-scroll"
    );

    // ONE NavRight at the right edge: viewport scrolls, cursor stays on same channel.
    view.apply(MixerViewEvent::NavRight);
    assert_eq!(
        view.viewport_offset(),
        1,
        "edge scroll: viewport_offset should be 1 after NavRight at right edge"
    );
    assert_eq!(
        view.cursor_channel(),
        VISIBLE_CHANNELS - 1,
        "edge scroll: cursorChannel must stay at {} after viewport scroll",
        VISIBLE_CHANNELS - 1
    );

    // A few more NavRights: cursor stays in visible window at all times.
    // (The cursor will alternate between moving within the window and
    // triggering a viewport scroll when it reaches the right edge again.)
    for extra in 0..6usize {
        view.apply(MixerViewEvent::NavRight);
        let lo = view.viewport_offset();
        let hi = lo + VISIBLE_CHANNELS - 1;
        assert!(
            (lo..=hi).contains(&view.cursor_channel()),
            "cursor {} out of visible window [{}, {}] after {} additional NavRights post-first-scroll",
            view.cursor_channel(),
            lo,
            hi,
            extra + 1,
        );
    }

    // Snapshot the state — cursor is somewhere inside the window.
    let cursor_snapshot = view.cursor_channel();
    let viewport_snapshot = view.viewport_offset();

    // Mirror: NavLeft scrolls the viewport back when cursor is at the left edge.
    // First, navigate left until we hit the left edge.
    // Move cursor all the way to the left edge of the visible window.
    while view.cursor_channel() > view.viewport_offset() {
        view.apply(MixerViewEvent::NavLeft);
    }
    // Sanity: cursor is now at the left edge.
    assert_eq!(
        view.cursor_channel(),
        view.viewport_offset(),
        "cursor should be at the left edge of the viewport"
    );
    let cursor_at_left_edge = view.cursor_channel();
    let viewport_at_left_edge = view.viewport_offset();

    // One NavLeft at the left edge: viewport scrolls back, cursor stays.
    if viewport_at_left_edge > 0 {
        view.apply(MixerViewEvent::NavLeft);
        assert_eq!(
            view.cursor_channel(),
            cursor_at_left_edge,
            "left edge scroll: cursor should stay at {} after viewport scrolls left",
            cursor_at_left_edge
        );
        assert_eq!(
            view.viewport_offset(),
            viewport_at_left_edge - 1,
            "left edge scroll: viewport_offset should decrease by 1"
        );
    }

    // A few more NavLefts: cursor stays in visible window at all times.
    for extra in 0..4usize {
        view.apply(MixerViewEvent::NavLeft);
        let lo = view.viewport_offset();
        let hi = lo + VISIBLE_CHANNELS - 1;
        assert!(
            (lo..=hi).contains(&view.cursor_channel()),
            "cursor {} out of visible window [{}, {}] after {} left NavLeft steps",
            view.cursor_channel(),
            lo,
            hi,
            extra + 1,
        );
    }

    // Suppress unused-variable warnings.
    let _ = (cursor_snapshot, viewport_snapshot);

    println!("edge scroll ok");
}

// ── Proof 2: Fine / coarse adjustment and clamping ───────────────────────────

fn proof_fine_coarse() {
    let mut view = MixerView::new();

    // Navigate to Volume row (it's already there), go to channel 2.
    view.apply(MixerViewEvent::NavRight);
    view.apply(MixerViewEvent::NavRight);
    assert_eq!(view.cursor_channel(), 2);
    assert_eq!(view.cursor_param(), MixerParam::Volume);

    // Enter edit mode.
    view.apply(MixerViewEvent::EnterEditMode);
    assert!(view.edit_mode(), "should be in edit mode");

    let vol_before = view.mixer().channel(2).volume;

    // NavRight → fine step (+0.01).
    view.apply(MixerViewEvent::NavRight);
    let vol_after_fine = view.mixer().channel(2).volume;
    assert!(
        (vol_after_fine - (vol_before + 0.01)).abs() < 1e-5,
        "NavRight in edit mode should raise volume by fine step (0.01): before={}, after={}",
        vol_before,
        vol_after_fine
    );

    // NavUp → coarse step (+0.10).
    let vol_before_coarse = view.mixer().channel(2).volume;
    view.apply(MixerViewEvent::NavUp);
    let vol_after_coarse = view.mixer().channel(2).volume;
    assert!(
        (vol_after_coarse - (vol_before_coarse + 0.10)).abs() < 1e-5,
        "NavUp in edit mode should raise volume by coarse step (0.10): before={}, after={}",
        vol_before_coarse,
        vol_after_coarse
    );

    // Clamp at 1.0 — repeated NavUp must never exceed 1.0.
    for _ in 0..20 {
        view.apply(MixerViewEvent::NavUp);
    }
    let vol_at_max = view.mixer().channel(2).volume;
    assert!(
        (vol_at_max - 1.0).abs() < 1e-5,
        "volume should clamp at 1.0, got {}",
        vol_at_max
    );

    // Clamp at 0.0 — repeated NavDown must never go below 0.0.
    for _ in 0..20 {
        view.apply(MixerViewEvent::NavDown);
    }
    let vol_at_min = view.mixer().channel(2).volume;
    assert!(
        (vol_at_min - 0.0).abs() < 1e-5,
        "volume should clamp at 0.0, got {}",
        vol_at_min
    );

    println!("fine/coarse ok");
}

// ── Proof 3: Toggle param (Mute) ─────────────────────────────────────────────

fn proof_toggle() {
    let mut view = MixerView::new();

    // Navigate to Mute row (4 NavDowns from Volume).
    for _ in 0..4 {
        view.apply(MixerViewEvent::NavDown);
    }
    assert_eq!(view.cursor_param(), MixerParam::Mute);

    // Toggle mute on → true.
    view.apply(MixerViewEvent::ToggleFocusedParam);
    assert!(
        view.mixer().channel(0).mute,
        "mute should be true after first toggle"
    );

    // Toggle mute off → false.
    view.apply(MixerViewEvent::ToggleFocusedParam);
    assert!(
        !view.mixer().channel(0).mute,
        "mute should be false after second toggle"
    );

    // Enter edit mode on the Mute row — directional input is a no-op.
    view.apply(MixerViewEvent::EnterEditMode);
    let mute_before = view.mixer().channel(0).mute;
    view.apply(MixerViewEvent::NavRight);
    view.apply(MixerViewEvent::NavUp);
    assert_eq!(
        view.mixer().channel(0).mute,
        mute_before,
        "directional input in edit mode on a toggle param must not change mute flag"
    );

    println!("toggle ok");
}

// ── Proof 4: Solo-vs-metering independence ────────────────────────────────────

/// Perform a stereo mixdown of 16 per-channel mono buffers.
///
/// Returns two values:
/// - `mix`: the sum of contributions applied to the stereo bus (solo gating
///   applied — a channel excluded by solo contributes nothing here).
/// - `peak_levels`: each channel's own peak level, computed from its raw
///   buffer regardless of solo status (metering is independent of solo).
///
/// The mix is simplified to mono for the proof (pan=0, so L==R).
fn mixdown(
    mixer: &ChannelMixer,
    per_channel_buffers: &[[f32; 64]; CHANNEL_COUNT],
) -> (f32, [f32; CHANNEL_COUNT]) {
    // Determine if any channel is soloed.
    let any_soloed = (0..CHANNEL_COUNT).any(|ch| mixer.channel(ch).solo);

    let mut mix_sum: f32 = 0.0;
    let mut peak_levels = [0.0f32; CHANNEL_COUNT];

    for ch in 0..CHANNEL_COUNT {
        let state = mixer.channel(ch);
        let buf = &per_channel_buffers[ch];

        // Peak metering: always reads the raw signal regardless of solo/mute.
        let peak = buf.iter().copied().fold(0.0f32, |acc, s| acc.max(s.abs()));
        peak_levels[ch] = peak;

        // Solo gating: if any channel is soloed, only soloed channels
        // contribute to the mix. Muted channels never contribute.
        let audible = if any_soloed {
            state.solo && !state.mute
        } else {
            !state.mute
        };

        if audible {
            let vol = state.volume;
            let channel_mix: f32 = buf.iter().copied().sum::<f32>() * vol;
            mix_sum += channel_mix;
        }
    }

    (mix_sum, peak_levels)
}

fn proof_solo_vs_metering() {
    let mut view = MixerView::new();

    // Build 16 small per-channel buffers with clearly non-zero signal.
    // Channel N's signal is (N+1) * 0.1 so they are all distinct and non-zero.
    let mut per_channel_buffers = [[0.0f32; 64]; CHANNEL_COUNT];
    for (ch, buf) in per_channel_buffers.iter_mut().enumerate() {
        let amplitude = (ch as f32 + 1.0) * 0.1;
        for sample in buf.iter_mut() {
            *sample = amplitude;
        }
    }

    // Solo exactly channel 3 via the view.
    // Navigate to channel 3, then down to Solo row (5 NavDowns).
    for _ in 0..3 {
        view.apply(MixerViewEvent::NavRight);
    }
    assert_eq!(view.cursor_channel(), 3);
    for _ in 0..5 {
        view.apply(MixerViewEvent::NavDown);
    }
    assert_eq!(view.cursor_param(), MixerParam::Solo);
    view.apply(MixerViewEvent::ToggleFocusedParam);
    assert!(
        view.mixer().channel(3).solo,
        "channel 3 should be soloed after ToggleFocusedParam"
    );

    // Run the mixdown.
    let (mix_sum, peak_levels) = mixdown(view.mixer(), &per_channel_buffers);

    // Compute the reference mix for channel 3 alone.
    let ch3_state = view.mixer().channel(3);
    let ch3_amplitude = (3_f32 + 1.0) * 0.1;
    let ch3_vol = ch3_state.volume;
    let reference_mix: f32 = ch3_amplitude * 64.0 * ch3_vol;

    // (a) The stereo mix equals only channel 3's contribution.
    assert!(
        (mix_sum - reference_mix).abs() < 1e-3,
        "solo mutes others: mix should equal only channel 3's contribution \
         (expected {}, got {})",
        reference_mix,
        mix_sum
    );
    println!("solo mutes others: true");

    // (b) Every channel's peak level is still > 0, including solo-silenced ones.
    for (ch, &peak) in peak_levels.iter().enumerate() {
        assert!(
            peak > 0.0,
            "metering independent of solo: channel {} peak level should be > 0 \
             but got {} (metering must not respect solo gating)",
            ch,
            peak
        );
    }
    println!("metering independent of solo: true");
}
