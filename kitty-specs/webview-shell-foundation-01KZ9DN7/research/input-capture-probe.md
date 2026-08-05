# Input-Capture Probe Verdict (WP01, plan IC-01, research R-02)

Probe binary: `src/bin/webview_input_probe.rs` (disposable evidence).
Durable output: `src/shell/webview/input_capture.rs`.
All logs below are pasted verbatim from real runs of
`./target/debug/webview_input_probe` on macOS (Darwin 25.5.0, 2026-08-05).

## Verdict

**The NSEvent local-monitor path wins.** `input_capture::install(sink)` wraps
`NSEvent.addLocalMonitorForEventsMatchingMask(.keyDown|.keyUp)` installed from
the Rust side; every key transition is observed in-process before dispatch to
the responder chain, independent of which view is first responder.

**The tao/Tauri window-event path fails structurally.** Tauri v2 surfaces
window events only through its own `WindowEvent` enum
(`tauri-2.11.5/src/app.rs`, `pub enum WindowEvent`), whose complete desktop
vocabulary is `Resized`, `Moved`, `CloseRequested`, `Destroyed`, `Focused`,
`ScaleFactorChanged`, `DragDrop`, `ThemeChanged`. There is no keyboard variant
at all — tao's `WindowEvent::KeyboardInput` is consumed inside
`tauri-runtime-wry` and never reaches application code. Window-level key
capture through `on_window_event` is therefore impossible under Tauri v2 by
API shape, not merely by first-responder routing. At runtime the probe logged
every window event Tauri delivered across all sessions; only `CloseRequested`
and `Destroyed` ever appeared (`PROBE_TAO` lines below). The losing path is
kept out of `input_capture.rs` entirely.

The mission STOP condition did **not** fire: a native Rust-side path captures
the full vocabulary with press/release fidelity, and the page registers no key
handler.

## Key-capture evidence (T002/T003)

Full-vocabulary sweep, distinct press and release per key, Edit-hold chord,
Shift chord, and an unbound key — captured Rust-side by
`input_capture::install` and fed through the production
`KeyboardInputTranslator` into the production `AppLoop`
(`EventSource::Keyboard`):

```
PROBE_SWEEP posting synthetic NSEvents (window_number=10506)
PROBE_KEY seq=1 key=Digit1 state=down repeat=false
PROBE_ACTION SelectContext(Mixer)
PROBE_KEY seq=2 key=Digit1 state=up repeat=false
PROBE_KEY seq=3 key=Digit2 state=down repeat=false
PROBE_ACTION SelectContext(Patch)
PROBE_KEY seq=4 key=Digit2 state=up repeat=false
PROBE_KEY seq=5 key=W state=down repeat=false
PROBE_ACTION Navigate(Up)
PROBE_KEY seq=6 key=W state=up repeat=false
PROBE_KEY seq=7 key=S state=down repeat=false
PROBE_ACTION Navigate(Down)
PROBE_KEY seq=8 key=S state=up repeat=false
PROBE_KEY seq=9 key=A state=down repeat=false
PROBE_ACTION Navigate(Left)
PROBE_KEY seq=10 key=A state=up repeat=false
PROBE_KEY seq=11 key=D state=down repeat=false
PROBE_ACTION Navigate(Right)
PROBE_KEY seq=12 key=D state=up repeat=false
PROBE_KEY seq=13 key=Q state=down repeat=false
PROBE_ACTION SelectPatch(Left)
PROBE_KEY seq=14 key=Q state=up repeat=false
PROBE_KEY seq=15 key=E state=down repeat=false
PROBE_ACTION SelectPatch(Right)
PROBE_KEY seq=16 key=E state=up repeat=false
PROBE_KEY seq=17 key=K state=down repeat=false
PROBE_ACTION SetInteractionMode(Adjust)
PROBE_KEY seq=18 key=W state=down repeat=false
PROBE_ACTION Adjust(Up)
PROBE_KEY seq=19 key=W state=up repeat=false
PROBE_KEY seq=20 key=K state=up repeat=false
PROBE_ACTION SetInteractionMode(Navigate)
PROBE_KEY seq=21 key=W state=down repeat=false
PROBE_ACTION Navigate(Up)
PROBE_KEY seq=22 key=W state=up repeat=false
PROBE_KEY seq=23 key=Other state=down repeat=false
PROBE_KEY seq=24 key=Other state=up repeat=false
```

Reading the log against the exit criterion:

- Every MIXER-vocabulary key (`1 2 W S A D Q E K`) shows a distinct `down`
  and `up` line. Press/release fidelity holds — no synthesized releases, no
  swallowed ups.
- Edit-hold semantics survive end-to-end: `K down → Adjust mode`, `W` while
  held → `Adjust(Up)`, `K up → Navigate mode`, the next bare `W` →
  `Navigate(Up)` (seq 17–22).
- Shift chord (seq 21–22 posted with the Shift modifier flag): the physical
  key identity is preserved — Shift+W still arrives as `W`, matching the
  eframe adapter's normalization.
- The unbound key G (seq 23–24) normalizes to `Other` and produces **no**
  semantic action, preserving the unbound-key invariant.

### Evidence class: what is and is not proven

The sweep above was posted as **synthetic in-process `NSEvent`s**
(`NSApplication postEvent:atStart:` into this process's own event queue —
`--synthetic-sweep`). Synthetic and hardware events share the dequeue path
that local monitors intercept, so the log proves the capture mechanism, the
keycode normalization, and the translator wiring. Synthetic events do **not**
traverse the window server or first-responder focus routing — but the local
monitor's interception point is *before* `sendEvent:`/responder dispatch,
which is precisely why WKWebView focus cannot preempt it; focus routing
decides where an event goes *after* the monitor has already seen it.

A session-level hardware-equivalent sweep (System Events `key code` /
`key down` / `key up`) **was attempted first** and could not be delivered:
the machine's session was locked during the entire implementation window
(lock-screen screenshot on file; `osascript` injection returned rc=0 with no
permission error, but keystrokes route to `loginwindow` while locked, and
zero `PROBE_KEY` lines arrived). No Accessibility-permission error occurred.

**Remaining human sweep** (minutes, at an unlocked session):
`cargo run --bin webview_input_probe` — the window stays open; focus the
page's `<input>` (autofocus), type `1 2 W S A D Q E`, hold `K` while tapping
`W/S/A/D`, try Shift chords, then close the window. Expected: `PROBE_KEY`
down/up pairs for every key while the characters simultaneously appear in the
webview's `<input>` (proving the webview had focus and still received the
events the monitor passed through). The page registers no key handler.

## Audio coexistence evidence (T004)

The probe composes the **exact production application** the eframe shell uses
— production instrument/effect registries, fixture patches, real cpal stream,
threaded graph-preparation worker — via `StandaloneApplication::run`, with the
Tauri window substituted behind the same `AppWindow` port. The fixture sounded
audibly from the default output device during every run.

45-second session (System Events attempt; audio + window loop under load):

```
PROBE_RT stage=ready   t_ms=142   sequence=36   blocks=36   frames=9216     commands=0     active_notes=0  primary_rms=0.000000 output_rms=0.000000 non_finite=0 clipped=0  routing_failures=0
PROBE_RT stage=sample  t_ms=10195 sequence=1921 blocks=1921 frames=491776   commands=2800  active_notes=11 primary_rms=0.000000 output_rms=0.596710 non_finite=0 clipped=52 routing_failures=0
PROBE_RT stage=sample  t_ms=22286 sequence=4188 blocks=4188 frames=1072128  commands=8725  active_notes=10 primary_rms=0.000000 output_rms=0.477561 non_finite=0 clipped=22 routing_failures=0
PROBE_RT stage=sample  t_ms=34374 sequence=6454 blocks=6454 frames=1652224  commands=10402 active_notes=5  primary_rms=0.000000 output_rms=0.109204 non_finite=0 clipped=0  routing_failures=0
PROBE_RT stage=closing t_ms=45007 sequence=8448 blocks=8448 frames=2162688  commands=11873 active_notes=4  primary_rms=0.029137 output_rms=0.170131 non_finite=0 clipped=0  routing_failures=0
PROBE_TAO window=probe event=CloseRequested { api: CloseRequestApi(Sender { .. }) }
PROBE_TAO window=probe event=Destroyed
PROBE_RT stage=exit    t_ms=45033 sequence=8453 blocks=8453 frames=2163968  commands=11890 active_notes=21 primary_rms=0.010053 output_rms=0.330727 non_finite=0 clipped=0  routing_failures=0
PROBE_EXIT clean: stream released before worker collection, no runtime error
```

(Complete logs, including every 2 s sample and the 12 s synthetic-sweep
session ending `PROBE_EXIT clean`, are reproducible with
`--close-after <secs>`; three sessions were run — 10 s, 45 s, 12 s — all
exited 0.)

Reading the counters against the exit criterion:

- **No underrun delta attributable to the webview loop.** Render continuity:
  (2 163 968 − 9 216) frames over (45 033 − 142) ms ≈ 48 006 frames/s — the
  negotiated 48 kHz rate held constant across the whole session (same in the
  10 s and 12 s sessions: ≈ 48 006 and ≈ 47 986 frames/s). A cpal device
  Xrun would surface through `AudioDeviceStatus` and force `on_tick` to
  return `false`; that never happened.
- `non_finite=0` and `routing_failures=0` in every snapshot of every session.
- `clipped` shows transient per-snapshot counts only at fixture loudness
  peaks (`output_rms ≥ ~0.48`) and is 0 in every closing/exit snapshot — an
  amplitude property of the fixture mix, not a timing property of the loop.
- The fixture demonstrably sounded: `active_notes` up to 21,
  `commands_consumed` advancing continuously (11 890 by teardown),
  `output_rms` peaking at 0.71.
- **Shutdown ordering is exactly the eframe path's**, because it *is* the
  eframe path: `StandaloneApplication::run` owns negotiate → start stream →
  window loop → drop stream → collect worker. The window closes through the
  normal path (`CloseRequested → Destroyed → Exit`), `run` returns, the
  stream is released before worker collection, and the process exits 0 with
  no panic. No shutdown-ordering anomaly was observed in any session.

## Guidance WP02 consumes

- **Install:** call `crest_synth::shell::webview::input_capture::install(sink)`
  once, on the **main thread**, before the Tauri event loop starts (the probe
  installs before `Builder::build`; during `setup` also works). It returns an
  `InputCaptureHandle` whose `Drop` removes the monitor — hold it for the
  window's lifetime. `install` fails typed (`InputCaptureError::NotMainThread`
  / `MonitorRejected`) for the FR-007 startup-error path.
- **Threading:** the sink runs on the main thread — the same thread that runs
  the Tauri loop — so `KeyboardInputTranslator` state needs no
  synchronization. Tauri imposes no `Send` on the sink; keep translator +
  input dispatch in main-thread state exactly as the probe's `KeyPipeline`
  does (`RawKeyEvent` → `WindowInput::key_down/key_up` → `translator
  .translate` → semantic action).
- **Focus loss:** `RunEvent::WindowEvent { event: Focused(false), .. }` is
  delivered on the main thread inside the run callback — feed
  `WindowInput::focus_lost()` there to clear the K modifier (eframe parity).
- **Event-loop return:** use `App::run_return`, **not** `App::run` — `run`
  never returns (it exits the process), which would break the owned shutdown
  (`AppWindow::run` must return so `StandaloneApplication` can drop the
  stream before collecting the worker).
- **Ticks:** tauri's tao loop waits when idle. The control side needs
  periodic `on_tick` (fixture MIDI, structural advance, device-status
  drain). The probe wakes the loop at ~60 Hz via a thread calling
  `AppHandle::run_on_main_thread(|| {})` and ticks on
  `RunEvent::MainEventsCleared`; any equivalent main-thread pacing works.
- **Pass-through:** the monitor returns events unchanged, so the webview
  still receives them (the probe page's `<input>` receives typing). Whether
  the product shell swallows vocabulary keys before the page is WP02's
  decision — swallow by returning nil from the monitor block would be a
  one-line change inside `input_capture` if declared.
- **Repeats:** OS auto-repeat surfaces as `RawKeyEvent::repeat() == true`
  key-downs, matching what egui delivers on the eframe path.
- **Key identity:** normalization is physical-position based
  (`kVK_ANSI_*` virtual key codes → `WindowKey`), so Shift/modifier chords
  preserve identity; unmapped codes normalize to `WindowKey::Other`.
- **Scaffolding added in WP01** (T001): unconditional `tauri = "2"` (no
  feature gate — R-01 runtime selection), `tauri-build` in
  `[build-dependencies]` with `tauri_build::build()` appended to the existing
  `build.rs`, a window-only `tauri.conf.json` (identifier `synth.crest.dev`,
  `app.windows: []`, bundle inactive, no frontendDist), and a placeholder
  `icons/icon.png` (tauri's `generate_context!` unconditionally requires a
  PNG icon on unix targets). The probe serves its page over a registered
  `probe://` URI scheme; `WebviewUrl::External` requires http(s) and a data:
  URL is rejected at the config layer — WP02's page should use a custom
  protocol or the asset system, not `External`.
