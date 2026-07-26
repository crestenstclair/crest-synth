## 1. Authoritative lifecycle contract

- [x] 1.1 Update `DESIGN.md` so `make demo-live` is an autonomous bounded witness that emits its final report before normal window/audio teardown and does not accept semantic edits while active.
- [x] 1.2 Reconcile the live-demo goal, Shell, Testing, manifest, validation, and witness CUE declarations with successful auto-exit and active-scene input isolation.
- [x] 1.3 Evaluate the CUE project and strictly validate the OpenSpec change before editing runtime behavior.

## 2. Live application behavior

- [x] 2.1 Replace live-mode keyboard dispatch with a stateless semantic input sink while preserving the normal interactive input callback and native early close.
- [x] 2.2 Make the first complete live report callback request window close in the same control tick, then return success after normal stream teardown without post-completion ticks.

## 3. Deterministic regression proof

- [x] 3.1 Update the deterministic live `AppWindow` witness to inject an adjustment between autonomous dispatch and audio observation and prove no keyboard event, generation, projection, or parameter publication changes.
- [x] 3.2 Assert exactly one final report and successful close request, zero post-completion ticks, matching final canonical projection, and preservation of typed early-close failure.

## 4. Verification

- [x] 4.1 Run Rust formatting, Clippy with warnings denied, and the targeted standalone, eframe, live-demo, CLI, and production-runtime tests.
- [x] 4.2 Run the full Cargo suite, evaluated-CUE checks, strict OpenSpec validation, semantic review, and deterministic OpenSpec acceptance.
- [x] 4.3 Run the physical `make demo-live` witness unattended with a bounded timeout and require one final summary, normal process exit, and no lingering process.
