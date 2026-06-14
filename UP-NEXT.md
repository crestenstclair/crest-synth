# Up Next

## Mixer view — rendering fix + demo scene

**Design doc:** [`docs/superpowers/specs/2026-06-14-mixer-view-render-and-demo-design.md`](docs/superpowers/specs/2026-06-14-mixer-view-render-and-demo-design.md)

The Mixer view's domain (`Patch.ChannelMixer`) and UI store (`Mixer.MixerView`)
are generated and proven headless. The remaining work is the **rendering** (the
generated `synth_ui` drew a single text column instead of the channel strips)
and a **demo scene** to verify functionality live. Aesthetic locked to
real-egui-bars / flat default theme (refine visuals later).

### Tasks (author the CUE spec, then run the generation loop)

- [ ] **`spec/editor.cue`** — enrich `asset.StandaloneUiMain` (`synth_ui`)
      prompts: render the 6-strip viewport (`viewportOffset` window) with flat
      green volume bars + peak-cap line, send bars, pan, mute/solo, cyan/amber
      cursor + edit-mode full-box highlight, header scroll indicator.
- [ ] **`spec/editor.cue`** — demo: `--tour` flag + runtime `T` toggle
      (captioned auto-tour over every feature), live on-screen log panel
      mirrored to stdout, loop the `--play` MIDI.
- [ ] **`spec/mixer.cue`** — add `MixerView::visible_channels()` (exactly 6) and
      a shared tour-script asserted headlessly in `asset.MixerDemoMain`
      (`tour steps ok: N`) so a single-column regression is caught.
- [ ] **`spec/manifest.cue`** — fix `ui` target MIDI path to
      `midi/Corridors of Time - Chrono Trigger.mid`; document `--tour`.
- [ ] Run `spec-authoring` → `spec-generate`; commit spec edits + regenerated
      code together.

### Success criteria

1. `synth_ui` shows 6 channel strips with green volume/meter bars moving with
   playback; cursor navigates; edit mode = amber + box highlight; sends/pan/
   mute/solo render.
2. `make ui` launches, loops the MIDI, runs the auto-tour, logs to stdout,
   accepts manual control.
3. `make demo-mixer` passes existing tokens + `tour steps ok:` + the
   6-visible-channels assertion; `ui-smoke` stays green.

### Deferred / separate

- Visual polish & theming.
- Multi-view navigation (synth_ui hosts only the Mixer view for now).
- Pre-existing crate-wide clippy debt in unrelated files (`gui_renderer.rs`,
  `serde_preset_codec.rs`, `plugin_host.rs`, `controller_glyph.rs`) — surfaced
  by the whole-tree gate, not caused by this work.
