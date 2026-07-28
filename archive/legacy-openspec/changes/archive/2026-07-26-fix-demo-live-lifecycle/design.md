## Context

`make demo-live` is a physical production-path witness, but its current success state is not terminal. `LiveDemoRunner` becomes inert and exposes a complete report, while `StandaloneApplication` continues returning `true` from the window tick forever; the eframe window, CPAL stream, and parent Make process therefore remain resident until a user closes the window. A monitored production run emitted its final summary after 89 seconds and was still resident with no additional output after 319 seconds.

The same window continues to dispatch keyboard `AppEvent`s into the shared `AppLoop` while an autonomous checkpoint is pending. Because scalar parameters use the required latest-complete-snapshot transport, an accepted user edit can replace the checkpoint generation before the audio callback observes it. The runner correctly refuses to fabricate an exact-generation checkpoint, but the application then exits as a failed demo. The deterministic live window does not currently inject this interleaving, so the existing acceptance target misses the production failure.

The master design still requires the canonical reducer/render path, exact generation correlation, a thin window adapter, normal ownership teardown, and no callback-side lifecycle work. This change updates the durable live-demo lifetime rather than weakening any of those constraints.

## Goals / Non-Goals

**Goals:**

- Make a successful `make demo-live` invocation bounded: emit the final report once, request window close, release the physical stream, and return success.
- Prevent mapped window input from changing canonical state or replacing a pending checkpoint snapshot during the autonomous scene.
- Preserve native early close as a typed incomplete result with semantic note cleanup.
- Add a deterministic production-composition witness for both input isolation and one-shot successful shutdown.
- Leave normal interactive input and the headless exhaustive demo unchanged.

**Non-Goals:**

- Relax exact checkpoint generation matching or retain historical scalar snapshots.
- Move state, scene, report, or audio ownership into the window adapter.
- Add a cancel key, progress UI, timeout policy, controller feature, or third top-level context.
- Change fixture contents, parameter dwell, engine preparation, audio transports, or callback behavior.

## Decisions

### Successful report emission terminates the live window tick

`LiveDemoRunner` remains window-agnostic: it completes cleanup, freezes the report, and becomes inert. `StandaloneApplication`, which already owns both the runner and window callbacks, detects the first complete report, invokes `onComplete` synchronously, records that emission, and returns `false` from that same control-side tick. The existing window port interprets `false` as a close request. After `AppWindow::run` returns, application ownership drops the physical stream and returns `Ok(())` because no runtime error was retained.

This orders the observable effects as report emission → window close request → normal stream teardown → process success. It avoids a timer, adapter-specific close API, or callback-side shutdown. Returning success directly from the runner was rejected because the runner does not own the window or stream. Keeping a post-completion grace period was rejected because it recreates an arbitrary resident lifetime and is not needed for the structured final evidence.

### The autonomous live window has a semantic input sink

Normal standalone mode retains its existing input callback and canonical `EventSource::Keyboard` dispatch. In `--demo-live`, the application supplies an explicit no-op semantic input callback while the autonomous witness owns the control sequence. Eframe may still normalize native key events, but the live-mode sink performs no mutation, publication, logging, or alternate state handling. Native window close remains available because viewport ownership is independent of the semantic event callback.

Ignoring edits is preferred to retrying a checkpoint. A retry would need to restore selection and every affected parameter from arbitrary interleaved user state, would change the frozen scene oracle, and could falsely credit a dwell that was not continuously visible. Ignoring only in this dedicated witness keeps the proof deterministic without weakening the product's normal one-way input path.

### Deterministic composition proof injects the production race

The deterministic `AppWindow` witness invokes one accepted-looking adjustment after the first autonomous dispatch but before rendering its pending snapshot. It then proves the input caused no `Keyboard` event, generation advance, parameter publication, or checkpoint failure. On completion it proves `onComplete` ran exactly once, the tick requested close exactly once, no post-completion tick was needed, the final projection/report agree, and `run_live_demo` returned success.

The existing early-close witness remains and must still return `LiveDemoIncomplete` without a success report. The existing live runner integration continues proving exact generation correlation, cleanup, and coverage independently of window lifetime.

### CUE and master design change with the implementation

The current CUE rules explicitly require persistent post-completion rendering and forbid auto-close. Those declarations and the corresponding `DESIGN.md` durable decision are changed before Rust behavior so evaluated architecture and implementation remain aligned. No other capability or top-level context changes.

## Risks / Trade-offs

- [The final window is no longer available for manual inspection] → The complete final StateTree, coverage, event-log summary, and human summary are emitted before shutdown; use normal interactive mode for open-ended inspection.
- [A user may expect keys to control the demo] → `demo-live` is explicitly an autonomous witness, while ordinary `make run` remains fully interactive and native close still cancels the demo.
- [The boolean tick result represents both success and failure closure] → `StandaloneApplication` remains the authority for the retained terminal error; after the window returns it distinguishes `Ok(())`, `LiveDemoIncomplete`, device failure, and other typed failures exactly as before.
- [A regression could close before report output flushes] → `onComplete` executes synchronously before the tick returns the close request, and the deterministic witness asserts the report callback precedes window return.

## Migration Plan

1. Update `DESIGN.md` and the affected goal, Shell, Testing, manifest, validation, and witness declarations in CUE.
2. Update the OpenSpec live-demo contract and strictly validate the change.
3. Change only live-mode input and tick callbacks plus their deterministic fake-window witness.
4. Run targeted live, eframe, CLI, and production-runtime tests, then full OpenSpec acceptance.
5. Run the physical `make demo-live` witness and require it to emit one final summary and return success without user input.

Rollback is the inverse source change; it restores the former persistent-window contract and tests without a data or state migration.
