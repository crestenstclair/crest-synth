# Mixer View — Rendering & Demo Scene Design

**Date:** 2026-06-14
**Status:** Approved (design); pending implementation via crest-spec loop
**Context:** crest-synth's first real GUI view. The Mixer view's domain model
(`Patch.ChannelMixer`) and UI store (`Mixer.MixerView`) already exist and are
proven headless. This design fixes the *rendering* (the generated `synth_ui`
drew a single text column, not the channel strips) and adds a demo experience
to verify all functionality live.

> crest-spec project rule: we author the CUE spec; the generate→validate→retry
> loop writes the Rust. This doc drives enrichment of asset `prompts`, never
> hand-written `.rs`.

## Problem (observed)

Running `synth_ui` showed: a single channel rendered as a vertical text list,
no green volume bar, no meters, no logs, and the `make ui` MIDI path was wrong.
The headless validations (`ui-smoke`, `demo-mixer`) passed because they prove
construction/audio/metering/logic — not visual layout. So the rendering brief
was under-specified, and nothing mechanically caught the single-column render.

## Visual aesthetic (locked)

Real egui widgets (filled bars/meters, real 6-column layout) with egui's
**default flat theme** — no gradients, glows, shadows, or custom polish. Visual
refinement is explicitly deferred. Mockup reference:
`.superpowers/brainstorm/<session>/content/midfi-egui.html`.

## 1. Mixer view rendering (the core fix)

The egui draw code must render the **6 visible channel strips that `MixerView`
exposes via `viewportOffset`**, side by side as 6 columns. It remains a pure
view over `MixerView`/`ChannelMixer` (no state mutation in draw code).

Each strip, top → bottom:

- **Volume** — a **vertical filled bar** (egui rect): fill height = volume
  (0.0–1.0), flat green; plus a distinct **peak-cap line** drawn at the
  channel's live `PeakLevel` read back from the `ChannelMixer`. The bar is both
  the volume control and the meter. Numeric value beneath.
- **Reverb send / Echo send** — small horizontal filled bars + value.
- **Pan** — L / C / R indicator.
- **Mute / Solo** — labeled toggle indicators, lit when on.

Cursor & modes:

- The focused (channel, parameter) cell gets a **frame stroke**: **cyan in
  NAVIGATE, amber in EDIT**.
- In EDIT mode on the Volume row, the **entire strip box** is highlighted
  (matches "holding Edit highlights the full box containing the level strip").

Header: title, current mode (NAVIGATE/EDIT), key hints, and the
`channels N–M of 16` scroll position derived from `viewportOffset`.

**No behavior/logic change** — navigate/edit, fine (←/→, step 0.01) / coarse
(↑/↓, 0.10), edge-scroll, and double-tap-toggle already live correctly in
`MixerView`. This section is purely about *rendering the viewport window and the
bars* instead of a single text column.

## 2. Demo experience (`synth_ui`)

One binary doing three things at once:

- **Audio** — `--play <FILE.mid>` loads and **loops** a MIDI file through the
  live engine so meters move. The Makefile `ui` target passes the corrected
  path `midi/Corridors of Time - Chrono Trigger.mid` (the dir is `crest-synth/midi/`,
  not `../../../midi/`).
- **Manual** — W/A/S/D navigate, hold **J** = edit, double-tap **J** = toggle.
  Always live.
- **Auto-tour** — `--tour` flag, and **T** toggles it at runtime. A scripted,
  captioned sequence that exercises **every** feature: pan across all 16
  channels (demonstrating edge-scroll), enter edit + sweep a volume fine/coarse,
  adjust sends and pan, double-tap mute and solo. A **caption banner** shows
  `▶ Tour step N/total: …` and `any key = manual · T = tour`. Any manual key
  pauses the tour; `T` resumes/restarts it.
- **Live log panel** (bottom of window) — prints each `MixerViewEvent` and the
  resulting state change (e.g. `NavRight fine → CH1 Vol 0.75→0.76`,
  `viewport → channels 2–7 (cursor stays CH6)`, `Toggle → CH3 Solo ON`),
  **mirrored to stdout** (fixes "no logs").

## 3. Provability (keep the whole-tree gate green)

Rendering can't be headless-asserted, but we harden the logic proofs so a
regression like "only one channel" is caught mechanically:

- **`MixerView::visible_channels()`** — expose the current 6-channel window
  (offset + slice). Add a `mixer_demo` assertion that it returns **exactly 6**
  channels and tracks `viewportOffset` across edge-scroll. A single-column model
  then can't pass.
- **Shared tour script** — the scripted tour sequence is defined once and run
  **headlessly** through `MixerView` + `ChannelMixer` in `mixer_demo`, asserting
  each step's expected state and printing `tour steps ok: N`. The visual
  `--tour` consumes the same script, so the on-screen demo and the headless
  proof can't drift.
- `ui-smoke` retains `ui smoke ok` / `render non-silent: true` /
  `channel metered: true`.

## 4. Spec changes (CUE)

- **`editor.cue`** → enrich `asset.StandaloneUiMain` (`synth_ui`) prompts with
  §1 (render the 6-strip viewport, green volume bars + peak cap, send bars, pan,
  toggles, cyan/amber cursor + edit box highlight, header with scroll position)
  and §2 (`--tour`/`T`, caption banner, on-screen log panel mirrored to stdout,
  loop the `--play` MIDI).
- **`mixer.cue`** → add `visible_channels()` to `MixerView`; add the shared
  tour-script + its headless assertions and `tour steps ok:` token to
  `asset.MixerDemoMain` (and its `demo-mixer` validation).
- **`manifest.cue`** → fix the `ui` target MIDI path; document `--tour`.

## 5. Out of scope

- Visual polish / theming (deferred by design).
- Multi-view navigation / view switching (synth_ui hosts only the Mixer view).
- Pre-existing crate-wide clippy debt in unrelated files (`gui_renderer.rs`,
  `serde_preset_codec.rs`, `plugin_host.rs`, `controller_glyph.rs`) — tracked
  separately; surfaced by the whole-tree gate but not caused by this work.

## Success criteria

1. `synth_ui` renders 6 channel strips side by side with green volume/meter bars
   that move with playback; cursor navigates cells; edit mode shows amber + box
   highlight; mute/solo/sends/pan render.
2. `make ui` launches, auto-plays the looping MIDI, runs the auto-tour, prints
   logs to stdout, and accepts manual control.
3. `make demo-mixer` passes with the existing tokens plus `tour steps ok:` and
   the 6-visible-channels assertion; `ui-smoke` still green.
