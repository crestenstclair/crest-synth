## 1. Canonical Session Foundations

- [x] 1.1 Add the injectable default-session blueprint and production `INIT` blueprint, construct it through canonical reducer state, and verify exact Patch/Mixer/master/return capture plus reordered/augmented registry invariance and typed missing-default failures.
- [x] 1.2 Extend the saved-session boundary to yield a one-shot private persisted-content replacement payload alongside its prepared graph without adding a second public Session model, and verify v1 migration, v2 round trip, and every existing excluded runtime/device field test still pass.
- [x] 1.3 Add a reducer-owned indication for accepted changes to fields represented by `SavedSession::capture`, and verify every saved field reports change while focus, navigation, MIDI, device lifecycle, observations, resize, and accepted no-op events do not.
- [x] 1.4 Add content-based document baseline comparison around typed `SavedSession` equality, and verify persisted edits become dirty, exact value restoration becomes clean, and non-session activity preserves the prior dirty state.

## 2. Prepared Whole-Session Replacement

- [x] 2.1 Centralize structural graph coordination and revision allocation so existing engine/effect changes and session replacement share one in-flight owner, and verify current engine, topology, activation, stale-result, backpressure, and off-thread retirement tests remain green.
- [x] 2.2 Add one whole-session `AppEvent` transition that atomically replaces persisted Patch/Mixer/master/return content through `AppState::apply`, resets to PATCH Engine focus, preserves device/application runtime, and verify rejection leaves generation and all state unchanged.
- [x] 2.3 Implement the bounded session-candidate worker for default and opened bytes using decode, migration, validation, projection, and complete graph preparation, and verify work runs off callback with distinct decode, version, shape, capability, asset, projection, and preparation failures.
- [x] 2.4 Implement replacement preflight against a cloned state and prove its projected parameters equal the prepared graph snapshot before staging; verify mismatched content, revision, or projection never reaches structural publication.
- [x] 2.5 Implement MIDI admission gating, bounded all-notes-off recovery, shared-coordinator staging, block-boundary acknowledgement, reducer commit, and resume behavior; verify a successful handoff exposes one correlated state/projection/parameter/graph revision with no old-target command reaching the new graph.
- [x] 2.6 Implement failed/busy/stale activation rollback and candidate retirement, and verify the prior saved capture, state hash, graph revision, render output, and input usability remain intact while candidate destruction occurs off callback.

## 3. Document, File, and Worker Boundaries

- [x] 3.1 Add shell-owned document identity, clean baseline, operation, pending continuation, busy state, and typed lifecycle-error models, and verify none serialize into `AppState`, `SavedSession`, device preference, or canonical state-tree documents.
- [x] 3.2 Add host-neutral Open/Save path selection and Save/Discard/Cancel prompt ports with deterministic fakes, and verify cancellation is a typed non-error result and duplicate outstanding requests are rejected without a second dialog.
- [x] 3.3 Implement the filesystem read and same-directory temporary-write/flush/sync/atomic-replace adapter, and verify controlled failures at create, write, flush, sync, and replace preserve an existing destination and clean temporary artifacts where possible.
- [x] 3.4 Add bounded preparation and save worker ownership/shutdown, carrying immutable captures and content tokens rather than paths into canonical state, and verify busy, completion, failure, and shutdown paths retire all owned values off callback.
- [x] 3.5 Compose immutable document name, dirty marker, operation, and typed failure presentation with the shell projection while retaining the full path only in shell state, and verify text/shape status survives reprojection without changing semantic focus or saved capture.

## 4. Lifecycle Workflows

- [x] 4.1 Implement New through the shared default candidate and prepared replacement path, and verify success produces clean Untitled exact defaults while every preparation/stage/activation failure preserves the prior identified or dirty session and graph.
- [x] 4.2 Implement Open dialog, file read, worker submission, transactional activation, and success identity/baseline update, and verify dialog cancellation plus read/decode/migration/validation/capability/asset/preparation/handoff failures leave the prior document and audio usable.
- [x] 4.3 Implement Save to an existing identity and Save-on-Untitled delegation to Save As, and verify successful bytes decode to the exact captured current-version session without changing active canonical state or graph.
- [x] 4.4 Implement Save As tentative destination handling and completion correlation, and verify identity changes only after atomic success, cancellation/failure preserves the prior identity, and an edit accepted during the write leaves the newly identified document dirty against the written baseline.
- [x] 4.5 Implement the resumable dirty-document guard for New, Open, and application close, and verify Save resumes exactly once only after success, Discard continues without I/O, and Cancel/cancelled Save As/save failure aborts the pending action with dirty work intact.
- [x] 4.6 Map every lifecycle stage and nested cause to explicit shell status/error output without fallback, and verify cancellations never fabricate an error while failures retain the exact operation and stage until dismissed or superseded.

## 5. Production Shell Integration

- [x] 5.1 Split standalone startup into a fixture-free production session path and retained explicit demo/witness paths, and verify a failing spy fixture source is never prepared or started by normal startup while existing demo targets still install and play their fixtures.
- [x] 5.2 Make the normal production path prepare the exact default session before audio/window start, and verify first projection/graph correlation, clean Untitled identity, physical-MIDI playability, silence before input, and typed fatal startup failure without empty or fixture fallback.
- [x] 5.3 Extend the Tauri application adapter with File menu commands, conventional platform shortcuts, native Open/Save dialogs, unsaved-change decisions, and close interception, and verify adapter-level command normalization emits each host-neutral lifecycle command once.
- [x] 5.4 Wire lifecycle advancement, document projection, worker polling, and orderly session-worker teardown into the production window/control loop, and verify the app stays responsive during preparation/write operations and exits with zero live file, candidate, graph, MIDI-device, or audio owners.
- [x] 5.5 Add a bounded native lifecycle handoff that exercises menu/shortcut availability, New cancellation, Save As, Open round trip, dirty close Cancel/Save/Discard, visible controlled failure, and clean teardown; verify its report distinguishes completed, failed, and environmental-skip evidence without claiming broad Figma parity.

## 6. Acceptance, Regression, and As-Built Record

- [x] 6.1 Add a deterministic production-boundary lifecycle matrix for successful and cancelled startup/New/Open/Save/Save As/close journeys, and verify exact capture, identity, baseline, dirty state, focus, projection, graph revision, and audible render assertions use the production reducer/projector/worker/graph seams.
- [x] 6.2 Add a controlled negative matrix for file, decode, migration, validation, capability, asset, projection, preparation, structural-stage, activation, encode, and atomic-write failures, and verify every case proves unchanged prior state hash, capture, identity, dirty state, graph revision, and usable audio.
- [x] 6.3 Add callback instrumentation around concurrent preparation, activation, failure, save, and candidate retirement, and verify zero callback allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwinding, or graph destruction with activation only at block boundaries.
- [x] 6.4 Run formatting, warnings-denied lint, exact-validation self-test, focused lifecycle targets, `cargo test --all-targets`, existing engine/effect/Sample/Mixer/MIDI/webview regressions, and the scoped native lifecycle witness; record every command's truthful pass/fail/skip result before completion.
- [x] 6.5 Update `DESIGN.md` only after production evidence exists, documenting the fixture-free startup, canonical default, shell-owned document boundary, transactional graph commit, atomic save behavior, explicit remaining visual/platform gaps, and measured proof; verify the text does not cite OpenSpec as as-built evidence or introduce a competing roadmap.

## Validation record — 2026-08-28

- `cargo fmt --all -- --check`: passed.
- `cargo check --all-targets`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `bash scripts/run_exact_test_validation.sh --self-test`: passed with `CREST_TEST_VALIDATION zero-selection-rejected passed`.
- `bash scripts/check_no_name_enumerated_identity.sh`: passed with `CREST_STATIC_VALIDATION no_name_enumerated_identity passed`.
- `cargo test session --lib`: passed, 41 tests.
- `cargo test shell::session_lifecycle::tests::threaded_production_boundary_matrix_runs_new_open_save_save_as_and_close --lib -- --exact`: passed, exactly one test.
- `cargo test --test production_runtime_contracts`: passed, 10 tests.
- `cargo test --all-targets --quiet`: final run passed. The library target reported 834 passed and two measurement-only ignores; every binary and integration target passed, including engine, effects, Sample, Mixer, MIDI, no-name guard, and webview regressions.
- `webview_projection_shell`: its headless serialization, policy, startup-failure, and late-ack checks passed. Its environment-gated real-window/DOM groups reported explicit skips because `CREST_WEBVIEW_TESTS=1` was absent; no skip was counted as completion.
- Two earlier broad-suite attempts failed truthfully: the first found the frozen AppEvent surface count had not included whole-session replacement, and the second found a concrete default capability designation inside the generic shell. Both defects were corrected; their focused regressions and the final all-target run passed.
- One partial-name witness invocation selected zero tests. It was not credited; the fully qualified exact invocation above ran and passed one test.
