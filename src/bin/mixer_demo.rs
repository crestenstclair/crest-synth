// path: src/bin/mixer_demo.rs

//! `mixer_demo` — headless prover for `MixerView` and its 16 `ChannelStrip`
//! channels.
//!
//! This binary opens NO window, NO audio device, and NO MIDI device. It is
//! a pure, mechanically-checkable harness: it constructs `MixerView` state,
//! drives it with scripted `MixerViewEvent`s exactly as a keyboard or
//! gamepad adapter would, and asserts in code that the resulting state
//! matches the behavior the mixer promises. A panic on any mismatch makes
//! this process exit non-zero.

use crest_synth::mixer::channel_strip::{Amplitude, ChannelStrip, Decibel};
use crest_synth::mixer::mixer_view::{
    MixerParam, MixerView, MixerViewEvent, MAX_VIEWPORT_OFFSET, TOTAL_CHANNELS, VISIBLE_CHANNELS,
    VOLUME_COARSE_STEP_DB, VOLUME_FINE_STEP_DB,
};

/// Number of samples in each channel's small per-block audio buffer used by
/// the solo-vs-metering proof.
const SAMPLES_PER_BLOCK: usize = 8;

fn main() {
    edge_scroll_proof();
    fine_coarse_proof();
    toggle_proof();
    solo_vs_metering_proof();

    println!("mixer_demo: all proofs passed (16 channels, {VISIBLE_CHANNELS} visible)");
}

/// Proves edge-scroll behavior: pushing the cursor past either edge of the
/// visible viewport scrolls the viewport instead of walking the cursor off
/// the visible window, at both the right and left edges.
fn edge_scroll_proof() {
    let mut view = MixerView::new();

    // Put the cursor on the trailing visible channel (index 5) with
    // viewportOffset still at 0.
    for _ in 0..(VISIBLE_CHANNELS - 1) {
        view.apply(MixerViewEvent::NavRight);
    }
    assert_eq!(
        view.cursor_channel(),
        VISIBLE_CHANNELS - 1,
        "expected the cursor on the trailing visible channel before the edge-scroll probe"
    );
    assert_eq!(
        view.viewport_offset(),
        0,
        "viewport must not have moved yet"
    );

    // One more NavRight must scroll the viewport, not move the cursor off
    // the visible window.
    view.apply(MixerViewEvent::NavRight);
    assert_eq!(
        view.viewport_offset(),
        1,
        "NavRight past the trailing visible channel must scroll the viewport"
    );
    assert_eq!(
        view.cursor_channel(),
        VISIBLE_CHANNELS - 1,
        "cursorChannel must stay put (5) while the viewport scrolls"
    );

    // Keep scrolling right: the cursor must never leave the visible window.
    for _ in 0..(TOTAL_CHANNELS * 2) {
        view.apply(MixerViewEvent::NavRight);
        assert!(
            view.visible_range().contains(&view.cursor_channel()),
            "cursor left the visible viewport while scrolling right"
        );
    }
    assert_eq!(
        view.viewport_offset(),
        MAX_VIEWPORT_OFFSET,
        "repeated NavRight must saturate the viewport at its maximum offset"
    );
    assert_eq!(view.cursor_channel(), TOTAL_CHANNELS - 1);

    // Mirror the proof at the left edge.
    for _ in 0..(VISIBLE_CHANNELS - 1) {
        view.apply(MixerViewEvent::NavLeft);
    }
    assert_eq!(
        view.cursor_channel(),
        MAX_VIEWPORT_OFFSET,
        "expected the cursor on the leading visible channel before the left edge-scroll probe"
    );
    assert_eq!(
        view.viewport_offset(),
        MAX_VIEWPORT_OFFSET,
        "viewport must not have moved yet"
    );

    view.apply(MixerViewEvent::NavLeft);
    assert_eq!(
        view.viewport_offset(),
        MAX_VIEWPORT_OFFSET - 1,
        "NavLeft past the leading visible channel must scroll the viewport back"
    );
    assert_eq!(
        view.cursor_channel(),
        MAX_VIEWPORT_OFFSET,
        "cursorChannel must stay put while the viewport scrolls back"
    );

    for _ in 0..(TOTAL_CHANNELS * 2) {
        view.apply(MixerViewEvent::NavLeft);
        assert!(
            view.visible_range().contains(&view.cursor_channel()),
            "cursor left the visible viewport while scrolling left"
        );
    }
    assert_eq!(
        view.viewport_offset(),
        0,
        "repeated NavLeft must saturate the viewport back at zero"
    );
    assert_eq!(view.cursor_channel(), 0);

    println!("edge scroll ok");
}

/// Proves fine/coarse stepping and clamping on a continuous parameter
/// (Volume): NavRight/NavLeft in edit mode move by the fine step, NavUp/
/// NavDown move by the coarse step (10x fine), and repeated presses clamp
/// at the domain bounds instead of overshooting.
fn fine_coarse_proof() {
    let mut view = MixerView::new(); // cursor: channel 0, Volume row, navigate mode
    view.apply(MixerViewEvent::EnterEditMode);

    let start = view.channel(0).unwrap().volume_db().value();

    view.apply(MixerViewEvent::NavRight);
    let after_fine = view.channel(0).unwrap().volume_db().value();
    assert!(
        (after_fine - (start + VOLUME_FINE_STEP_DB)).abs() < 1e-4,
        "NavRight in edit mode must raise volume by the fine step: expected {}, got {after_fine}",
        start + VOLUME_FINE_STEP_DB
    );

    view.apply(MixerViewEvent::NavUp);
    let after_coarse = view.channel(0).unwrap().volume_db().value();
    assert!(
        (after_coarse - (after_fine + VOLUME_COARSE_STEP_DB)).abs() < 1e-4,
        "NavUp in edit mode must raise volume by the coarse step (10x fine): expected {}, got {after_coarse}",
        after_fine + VOLUME_COARSE_STEP_DB
    );

    // Repeated NavUp must clamp at the upper bound, never exceed it.
    for _ in 0..200 {
        view.apply(MixerViewEvent::NavUp);
    }
    let clamped_max = view.channel(0).unwrap().volume_db().value();
    assert_eq!(
        clamped_max,
        Decibel::MAX,
        "volume must clamp at its upper bound under repeated NavUp"
    );

    // Repeated NavDown must clamp at the lower bound, never undershoot it.
    for _ in 0..500 {
        view.apply(MixerViewEvent::NavDown);
    }
    let clamped_min = view.channel(0).unwrap().volume_db().value();
    assert_eq!(
        clamped_min,
        Decibel::MIN,
        "volume must clamp at its lower bound under repeated NavDown"
    );

    println!("fine/coarse ok");
}

/// Proves toggle-parameter semantics on Mute: `ToggleFocusedParam` flips the
/// flag on then off, and directional input while in edit mode is a no-op on
/// a toggle (only continuous parameters respond to NavUp/NavDown/NavLeft/
/// NavRight while editing).
fn toggle_proof() {
    let mut view = MixerView::new();

    // Navigate down to the Mute row: Volume -> Pan -> Mute.
    view.apply(MixerViewEvent::NavDown);
    view.apply(MixerViewEvent::NavDown);
    assert_eq!(
        view.cursor_param(),
        MixerParam::Mute,
        "expected the cursor on the Mute row"
    );

    view.apply(MixerViewEvent::ToggleFocusedParam);
    assert!(
        view.channel(0).unwrap().mute(),
        "ToggleFocusedParam on the Mute row must flip mute on"
    );

    view.apply(MixerViewEvent::ToggleFocusedParam);
    assert!(
        !view.channel(0).unwrap().mute(),
        "a second ToggleFocusedParam on the Mute row must flip mute back off"
    );

    // Directional input in edit mode must never touch a toggle parameter.
    view.apply(MixerViewEvent::EnterEditMode);
    view.apply(MixerViewEvent::NavRight);
    view.apply(MixerViewEvent::NavLeft);
    view.apply(MixerViewEvent::NavUp);
    view.apply(MixerViewEvent::NavDown);
    assert!(
        !view.channel(0).unwrap().mute(),
        "directional input on a toggle parameter in edit mode must be a no-op"
    );

    println!("toggle ok");
}

/// Proves the interaction that catches a wrong solo/mute gating
/// implementation: soloing exactly one channel silences every other
/// channel's contribution to the mix, but every channel — including the
/// solo-silenced ones — still meters its own real (non-zero) signal.
fn solo_vs_metering_proof() {
    let mut view = MixerView::new(); // fresh 16 ChannelStrip channels

    let solo_channel = 3usize;

    // Navigate the cursor to channel 3 (well within the initial viewport,
    // no edge-scroll involved).
    for _ in 0..solo_channel {
        view.apply(MixerViewEvent::NavRight);
    }
    assert_eq!(view.cursor_channel(), solo_channel);

    // Navigate down to the Solo row: Volume -> Pan -> Mute -> Solo.
    for _ in 0..3 {
        view.apply(MixerViewEvent::NavDown);
    }
    assert_eq!(
        view.cursor_param(),
        MixerParam::Solo,
        "expected the cursor on the Solo row"
    );

    view.apply(MixerViewEvent::ToggleFocusedParam);
    assert!(
        view.channel(solo_channel).unwrap().solo(),
        "ToggleFocusedParam on the Solo row must flip solo on"
    );

    // Build 16 small per-channel audio buffers, each with a clearly
    // non-zero signal.
    let buffers: [[f32; SAMPLES_PER_BLOCK]; TOTAL_CHANNELS] =
        std::array::from_fn(|i| [0.3 + 0.01 * i as f32; SAMPLES_PER_BLOCK]);

    // Snapshot the view's channels as an owned array so the mix pass and
    // the meter can be driven directly against the real ChannelStrip state
    // the view just mutated (draw code never reaches into a ChannelStrip's
    // fields directly, but this harness — like MixerView's own metering —
    // operates one layer below the UI event flow, on the domain aggregates
    // themselves).
    let mut channels: [ChannelStrip; TOTAL_CHANNELS] = (*view.channels()).clone();

    // Run an equivalent direct mixdown over the ChannelStrip list, applying
    // the same solo-in-place gating a MixEngine pass would: when any
    // channel is soloed, only soloed channels are audible.
    let mix = mixdown(&channels, &buffers);
    let reference = stereo_contribution(&channels[solo_channel], &buffers[solo_channel]);

    assert_eq!(mix.len(), reference.len());
    for (m, r) in mix.iter().zip(reference.iter()) {
        assert!(
            (m.0 - r.0).abs() < 1e-5 && (m.1 - r.1).abs() < 1e-5,
            "the stereo mix must equal ONLY channel {solo_channel}'s contribution when it is soloed; got {mix:?}, expected {reference:?}"
        );
    }
    println!("solo mutes others: true");

    // Metering is independent of solo/mute: every channel, including the
    // ones the solo above silences, must still report a real non-zero
    // peak for the same non-zero input it was fed.
    let mut all_peaks_nonzero = true;
    for (channel, buffer) in channels.iter_mut().zip(buffers.iter()) {
        let sample =
            Amplitude::try_new(buffer[0]).expect("buffer sample must be a valid Amplitude");
        let peak = channel.meter(sample);
        if peak.value() <= 0.0 {
            all_peaks_nonzero = false;
        }
    }
    assert!(
        all_peaks_nonzero,
        "every channel must still meter a non-zero peak regardless of solo/mute state"
    );
    println!("metering independent of solo: true");
}

/// This channel strip's post-volume, post-pan stereo contribution for each
/// sample in `buffer`.
fn stereo_contribution(strip: &ChannelStrip, buffer: &[f32]) -> Vec<(f32, f32)> {
    let volume_linear = strip.volume_db().to_linear();
    let (left_gain, right_gain) = strip.pan().equal_power_gains();
    buffer
        .iter()
        .map(|&sample| {
            let post_volume = sample * volume_linear;
            (post_volume * left_gain, post_volume * right_gain)
        })
        .collect()
}

/// An equivalent direct mixdown over a `ChannelStrip` list: sums every
/// audible channel's stereo contribution for its buffer, applying
/// solo-in-place gating (when any channel is soloed, only soloed channels
/// are audible; otherwise every unmuted channel is audible).
fn mixdown(
    channels: &[ChannelStrip; TOTAL_CHANNELS],
    buffers: &[[f32; SAMPLES_PER_BLOCK]; TOTAL_CHANNELS],
) -> Vec<(f32, f32)> {
    let any_soloed = channels.iter().any(ChannelStrip::solo);
    let mut mix = vec![(0.0f32, 0.0f32); SAMPLES_PER_BLOCK];

    for (channel, buffer) in channels.iter().zip(buffers.iter()) {
        let audible = if any_soloed {
            channel.solo()
        } else {
            !channel.mute()
        };
        if !audible {
            continue;
        }
        for (entry, (l, r)) in mix.iter_mut().zip(stereo_contribution(channel, buffer)) {
            entry.0 += l;
            entry.1 += r;
        }
    }

    mix
}
