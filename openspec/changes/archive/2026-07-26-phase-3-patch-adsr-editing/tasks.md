## 1. Master design and executable architecture (planning complete)

- [x] 1.1 Reconcile `DESIGN.md` with the durable Engine → Attack → Decay → Sustain → Release focus order, canonical fine/coarse ADSR editing, scalar-only publication, and engine-lifecycle revision behavior.
- [x] 1.2 Add `goal.edit_patch_envelope` and its focus, adjustment, scalar-publication, lifecycle-coexistence, and behavioral-proof requirements to the executable CUE architecture without introducing a second capability or envelope concept.
- [x] 1.3 Reconcile the Control, Synth, Shell, Testing, validation, asset, evidence, and witness CUE resources to the editable PATCH ADSR surface and stricter existing acceptance targets.
- [x] 1.4 Evaluate `openspec context --json` successfully and confirm the rendered architecture and relationship index contain the new goal, requirements, resources, validations, and witness predicates with no remaining read-only-ADSR contradiction.

## 2. Canonical PATCH focus and reducer behavior

- [x] 2.1 Extend `PatchControlId` to represent Engine or `Envelope(VoiceEnvelopeParameter)`, derive the five-entry order from the canonical envelope descriptor, implement stable `patch.engine` / `patch.envelope.*` serialization, and add exhaustive identity/order/round-trip tests.
- [x] 2.2 Extend `InteractionState` with reducer-only nonwrapping Up/Down movement across the five controls while preserving initialization, context round trips, Patch focus, MIXER selection, and engine-lifecycle focus.
- [x] 2.3 Extract the current envelope adjustment branch into one canonical reducer operation shared by `PatchEditableTarget::Envelope` in MIXER and focused envelope controls in PATCH; preserve descriptor bounds, fine/coarse steps, transactional update, and `ParameterAtBoundary` behavior.
- [x] 2.4 Route PATCH `Navigate` and `Adjust` in `AppState::apply`: accept vertical focus movement, reject horizontal/end-point navigation, retain Engine Left/Right structural requests and vertical rejection, and resolve all four directions on ADSR rows as scalar edits.
- [x] 2.5 Add reducer tests for every focus edge and direction, all four ADSR fields, fine/coarse steps, lower/upper bounds, target-only Patch mutation, unavailable capability rows, later-event recovery, and identical MIXER/PATCH results from identical state.
- [x] 2.6 Prove accepted focus movement and ADSR edits each advance generation exactly once, while rejected navigation/adjustment leaves state and generation exact and never emits an `AudioCommand` or unintended `EngineSelectionEffect`.

## 3. PATCH projection, text rendering, and schema migration

- [x] 3.1 Add `focusedControlId` to `PatchPageProjection` and add canonical `controlId` plus editable status to every envelope row while continuing to derive ids, labels, values, bounds, steps, units, engine choices, and capability rows generically.
- [x] 3.2 Separate Engine action availability from selection: preserve `engine.editable` for Ready/Failed request availability, keep the focused Engine identity through Preparing/Activating, and ensure exactly one Engine-or-ADSR row is selected.
- [x] 3.3 Update PATCH text rendering so the marker and `selectedLine` follow `focusedControlId`, every ADSR row displays its exact canonical descriptor/value data, capability rows remain read-only, and a disabled Engine stays visibly focused.
- [x] 3.4 Advance the applicable StateTree/observation schema versions and the Patch-page production-owned serialized-leaf descriptor for `focusedControlId` and envelope `controlId`; update eager/deferred equality, hashes, round trips, and missing/unexpected schema cases atomically.
- [x] 3.5 Extend projector and text unit tests across SoundFont and Braids, every control focus, Ready/Preparing/Activating/Failed, context round trips, exact selection markers/lines, and absence of engine-specific field matching.
- [x] 3.6 Extend the headless egui-context fixture to dispatch real W/S and K+A/D/W/S events, render the next canonical frame, scroll to each selected line, and prove the window owns no focus, bounds, or mutation logic.

## 4. Scalar publication during engine lifecycle

- [x] 4.1 Add AppLoop boundary assertions showing focus-only navigation republishes a generation-coherent same-revision snapshot with identical values and produces sample-identical output with no discrete or structural effect.
- [x] 4.2 Add Ready and recoverable-Failed cases showing a PATCH ADSR edit publishes one complete active/source-revision snapshot and is consumed through the existing per-voice renderer path without graph preparation.
- [x] 4.3 Add a Preparing case that edits ADSR after request submission, proves the audible source consumes the source-revision value, then proves candidate commit refreshes its initial parameters from the latest canonical value before graph publication.
- [x] 4.4 Add an Activating/queue-pressure case that edits ADSR after candidate commit, proves the new snapshot targets the replacement revision, the source remains finite under its last compatible snapshot, staged retry retains ownership, and the activated graph starts with the latest value.
- [x] 4.5 Prove engine prepared/failure/activation events preserve PATCH control focus and envelope, another Engine edit remains `StructuralEditBusy`, valid ADSR edits remain accepted, and no callback allocation, destruction, blocking, locking, I/O, logging, or panic path is introduced.

## 5. Focused envelope and Patch-page acceptance

- [x] 5.1 Expand `tests/patch_page_projection.rs` to exercise the five-control order through production keyboard translation and AppLoop, exact page/text/tree/snapshot values, nonwrapping rejection recovery, fine/coarse ADSR edits, boundaries, lifecycle visibility, and empty audio/structural effects before its existing acceptance marker.
- [x] 5.2 Expand `tests/per_voice_envelope.rs` and `CREST_ENVELOPE_OBSERVATION` with four PATCH control cases, exact focus order, shared MIXER/PATCH mutation, scalar-only publication, target isolation, and measured all-field SoundFont/Braids per-voice consequences.
- [x] 5.3 Expand schema-surface tests for every serialized `PatchControlId`, new Patch-page leaves, version bump, exact descriptor/discovered equality, and controlled missing/unexpected leaf failures.
- [x] 5.4 Re-run engine-selection workflow acceptance with envelope focus preserved, Engine refocused before requests, scalar edits accepted during pending lifecycle, descriptor-default reverse selection, target-only audio, and no fallback regression.

## 6. Deterministic and live demo proof

- [x] 6.1 Extend the exhaustive demo plan/report/coverage so the focused Patch's Attack, Decay, Sustain, and Release identifiers are navigated and reversibly edited through normalized PATCH input exactly once, while remaining editable instances keep the canonical MIXER resolver path.
- [x] 6.2 Add deterministic Preparing and Activating ADSR checkpoints with exact state/page/text/tree/snapshot/effect/revision/audio comparisons, candidate refresh, target first-consumption, source continuity, untargeted equality, and baseline restoration.
- [x] 6.3 Extend `CREST_OBSERVATION` with the four PATCH ADSR cases, exact focus projection, scalar-only behavior, and scalar/structural-coexistence predicates; keep 17-input coverage, empty missing/unexpected sets, and byte-identical fresh-run evidence.
- [x] 6.4 Update `LiveDemoScene` so the focused first Patch's four frozen envelope coverage identifiers run through semantic PATCH focus/adjustment steps, remain visibly selected for at least 500 ms, receive matching-generation audio checkpoints, and are not credited again through MIXER.
- [x] 6.5 Return live PATCH focus to Engine semantically before the existing SoundFont → Braids → descriptor-default SoundFont sequence; preserve lifecycle/revision/target-audio proof, mapped-input isolation, exact coverage, semantic cleanup, one report/close, and control-owned teardown.
- [x] 6.6 Extend deterministic live-contract assertions for exact PATCH control/value/effect checkpoints and separately verify physical composition still uses the production threaded worker and bounded physical audio observation path.

## 7. Targeted and regression verification

- [x] 7.1 Run focused unit suites for `PatchControlId`, `InteractionState`, `AppState`, `PatchPageProjection`, `StateProjector`, `TextProjection`, AppLoop revision handling, keyboard translation, and live/exhaustive plan construction.
- [x] 7.2 Run `cargo test --test patch_page_projection -- --nocapture`, `cargo test --test per_voice_envelope -- --nocapture`, `cargo test --test engine_selection_workflow -- --nocapture`, `cargo test --test exhaustive_demo_scene -- --nocapture`, `cargo test --test live_demo_scene -- --nocapture`, `cargo test --test schema_surface -- --nocapture`, and `cargo test --test eframe_context -- --nocapture` with every declared structured predicate and acceptance marker.
- [x] 7.3 Run prepared-rack, Braids, production-runtime, real-time contract, mutation-harness, smoke, and control-dispatch-performance gates to prove the new control surface did not weaken routing, graph ownership, callback safety, mutation resistance, or the 512-event responsiveness ceiling.
- [x] 7.4 Run `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `make smoke` with no ignored failures or zero-test selectors.
- [x] 7.5 Run two fresh `make demo` executions and require byte-identical logical event logs, state trees, PATCH focus/ADSR and lifecycle checkpoints, coverage, and observation JSON with no excluded declared field.
- [x] 7.6 Run one complete `make demo-live` on a usable 48 kHz physical device and require visible focused ADSR checkpoints, audible finite consequences, both acknowledged engine directions, exact coverage, final Ready default SoundFont state, semantic note cleanup, one close, stream release, and exit zero.
- [x] 7.7 Run every CUE-declared project validation and witness, `openspec context --json`, `openspec doctor --json`, `openspec validate phase-3-patch-adsr-editing --strict`, and `openspec validate --all --strict` before accepting the increment.
- [x] 7.8 After every acceptance gate passes, update `ROADMAP.md` to mark PATCH ADSR editing complete and identify SoundFont preset discovery/selection as the next separate Phase 3 increment without expanding this change.

## 8. Physical live-demo lockup repair

- [x] 8.1 Reconcile `DESIGN.md`, evaluated CUE, proposal/design/spec/summary artifacts, and the live runtime contract with valid-default-first CPAL negotiation plus typed ten-second no-progress and 120-second whole-run deadlines.
- [x] 8.2 Make CPAL accept a valid preferred default before optional range enumeration and retain a valid default when that enumeration fails, with controlled selection tests and no invented device/config fallback.
- [x] 8.3 Add runner milestone/whole-run watchdogs, actionable autonomous startup status, and deterministic standalone tests proving a stalled audio observation closes, cleans up, releases ownership, exits typed, and emits no completed report.
- [x] 8.4 Run focused device/live suites and one complete physical `make demo-live` through final report, close, stream release, parent exit, and no owned descendants before the standard end-of-apply acceptance pass.

## 9. Sparse-fixture checkpoint deadlock repair

- [x] 9.1 Reproduce the post-checkpoint-27 physical failure, trace the frozen fixture/exact-generation wait, and reconcile `DESIGN.md`, evaluated CUE, and OpenSpec with schedule-independent semantic parameter probes.
- [x] 9.2 Bracket every accepted scalar checkpoint with a bounded owning-Patch NoteOn/NoteOff pair through `AppLoop`, preserve one-event-per-tick and scalar-only coverage, and add exact plan/production-render regression assertions.
- [x] 9.3 Run formatting, focused live/standalone suites, strict OpenSpec validation, and the complete deterministic acceptance contract.
- [x] 9.4 Run a fresh physical `make demo-live` past checkpoint 27 through all scalar coverage, both engine transitions, final report, stream release, parent exit, and no owned descendants.
