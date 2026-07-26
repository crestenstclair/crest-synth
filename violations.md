# Current Architecture Violations

Review baseline: `9685d0a` (`complete phase 2 prepared engine rack`), reviewed 2026-07-25 in a clean detached worktree.

## Scope and authority

This report compares the production implementation with `DESIGN.md`, `ROADMAP.md`, evaluated CUE under `spec/`, and the canonical OpenSpec specifications.

Only implemented scope is evaluated:

- Phase 1 — Live observable demo;
- Phase 2 increment 1 — canonical capability descriptors and generic instrument configuration;
- Phase 2 increment 2 — prepared engine rack and structural graph handoff;
- evergreen architecture and real-time invariants that apply to those paths.

The absent Braids adapter, mixed-engine proof, Patch page, per-Patch effects, modulation, redesigned UI, and other later roadmap work are intentionally deferred and are not violations.

## Summary

| ID | Classification | Severity | Violation |
| --- | --- | --- | --- |
| V-001 | `BLOCKING_CURRENT_VIOLATION` | High | Production composition bypasses declared provider and boundary injection ports. |
| V-002 | `BLOCKING_CURRENT_VIOLATION` | High | The prepared graph is built before the actual device configuration is negotiated. |
| V-003 | `BLOCKING_EVERGREEN_VIOLATION` | High | Runtime device errors are discarded instead of becoming typed visible failures. |
| V-004 | `BLOCKING_CURRENT_VIOLATION` | Medium | Unknown-Patch routing failure is erased by the production renderer. |
| V-005 | `BLOCKING_REQUIRED_PROOF_GAP` | Medium | Three declared CUE test validations succeed while executing zero tests. |

## V-001 — Production composition bypasses declared injection ports

### Authority and applicability

`spec/manifest.cue:242` assigns construction of `HiDefSoundFontCapability`, its separate preparer, graph builders, and distinct audio/structural boundaries to the thin `crest-synth` composition root, which must inject them into `StandaloneApplication`.

`spec/shell.cue:77-99` declares that `StandaloneApplication` consumes `InstrumentCapabilityProvider`, `InstrumentPreparer`, `StructuralGraphBoundary`, `AudioObservation`, `AppWindow`, and `AudioOutput` through their application ports.

This is current Phase 2 increment 1 and 2 scope, not a future multi-engine requirement. Production may still install only HiDef SoundFont; the violation is where the concrete implementation is selected and owned.

### Implementation evidence

- `src/bin/crest_synth.rs:35-50` constructs and injects the ordinary audio boundary, HiDef preparer, MIDI source, window, and audio output, but no capability provider, structural boundary, or observation boundary.
- `src/shell/standalone_application.rs:1-6` imports concrete infrastructure adapters directly.
- `src/shell/standalone_application.rs:333-361` has no fields for the declared provider or structural/observation ports.
- `src/shell/standalone_application.rs:394-395` constructs `HiDefSoundFontCapability` inside the application service.
- `src/shell/standalone_application.rs:415-419` constructs the concrete lock-free structural boundary inside the application service.
- The live path similarly constructs its concrete atomic audio-observation adapter internally.

### Contradiction and consequence

The production application service bypasses the declared composition ownership and depends directly on concrete infrastructure. The provider and structural ports exist as abstractions, but the real composition cannot supply alternatives or deliberate mismatches through them.

Consequently, acceptance cannot prove through the production composition that duplicate, missing, unknown, or mismatched provider/preparer registrations fail as declared. It can only test isolated components or specially constructed lower-level services.

### Required repair and witness

- Move concrete provider, structural-boundary, and observation-boundary selection to the declared composition root.
- Inject the declared ports or factories into `StandaloneApplication` without adding another product engine.
- Keep the current production registry restricted to HiDef SoundFont.
- Add a production-composition witness that supplies a matching provider/preparer and then deliberate missing, duplicate, unknown, and mismatched combinations, proving typed rejection before graph publication.
- Add a replaceable structural-boundary fixture through the same production constructor so the declared port cannot be silently bypassed again.

## V-002 — Graph preparation precedes device negotiation

### Authority and applicability

`spec/realtime.cue:64-79` declares that a `PreparedGraph` owns its validated `sampleRate` and `maxFrames`, and that every callback buffer is bounded by that prepared capacity.

`openspec/specs/prepared-engine-rack/spec.md:39-48` requires the complete graph to be prepared for the supported device configuration before the renderer or device callback can use it.

`DESIGN.md:174-194` requires the host to negotiate the supported device configuration and prepare bounded callback work for that configuration. This applies to the existing physical-audio path delivered in Phase 1.

### Implementation evidence

- `ApplicationConfig::default` fixes preparation at 48 kHz and 1,024 frames in `src/shell/standalone_application.rs:94-102`.
- `prepare_startup` builds the complete graph from those fixed values in `src/shell/standalone_application.rs:408-414`.
- Only afterward does `CpalAudioOutput` select the actual device configuration and sample rate in `src/adapter/cpal_audio_output.rs:22-32`.
- Both production render closures explicitly ignore the negotiated sample rate in `src/shell/standalone_application.rs:468-470` and `src/shell/standalone_application.rs:546-548`.
- The native stereo path forwards the complete device buffer directly in `src/adapter/cpal_audio_output.rs:105-118`.
- `AudioRenderer` silently renders only `min(device_frames, graph.max_frames)` in `src/real_time/audio_renderer.rs:110`, leaving the remainder of an oversized native buffer silent after its initial clear.

### Contradiction and consequence

The actual device configuration is learned too late to participate in graph preparation. A device running at a non-48 kHz rate receives instruments and effects prepared for 48 kHz, which changes pitch, timing, and time-based processing. A native callback larger than 1,024 frames is not rejected or fully adapted; it receives a silently zeroed tail.

This is an implemented-path incompatibility, not merely missing performance tuning.

### Required repair and witness

- Split device selection/negotiation from stream start, returning a typed configuration before graph preparation.
- Validate the negotiated rate, channel mapping, sample format, and callback capacity before building the graph.
- Prepare instruments, effects, scratch storage, and the graph from the accepted actual configuration.
- Start the stream only after the compatible graph and renderer exist.
- Bound or chunk every device path, including native stereo, without silently truncating a callback.
- Add production-path witnesses for a supported non-default sample rate, exact capacity, oversized callbacks, and unsupported configurations rejected before rendering.

## V-003 — Runtime device failures are discarded

### Authority and applicability

`spec/shell.cue:116-123` states that runtime audio-device failures remain typed, visible `ApplicationError`s.

`DESIGN.md:41` requires unavailable devices to produce typed visible errors. This is an evergreen rule for the completed physical-audio path.

### Implementation evidence

- Both CPAL stream builders install `ignore_stream_error` as their runtime error callback in `src/adapter/cpal_audio_output.rs:111-119` and `src/adapter/cpal_audio_output.rs:141-154`.
- `src/adapter/cpal_audio_output.rs:190` defines that handler as an empty function.
- `AudioOutput::open` can return setup errors, but its port exposes no post-start error/status path to control ownership.

### Contradiction and consequence

Device disconnection and other failures delivered after successful startup disappear inside the adapter. The application can continue presenting an apparently healthy runtime while audio has failed, directly contradicting the typed-visible-error contract.

### Required repair and witness

- Add a bounded, non-blocking runtime device-status/error path from the adapter callback to control ownership.
- Convert the received status to the declared typed application-visible failure or recovery state outside the real-time callback.
- Do not log, allocate, format, or invoke UI behavior inside the device callback.
- Add a controlled production witness that starts successfully, injects a later device failure, and observes the exact typed control/application outcome.

## V-004 — Unknown-Patch routing failure is erased

### Authority and applicability

`spec/realtime.cue:45-52` requires unknown `PatchId` dispatch to return a fixed-size status without fallback or broadcast.

`openspec/specs/prepared-engine-rack/spec.md:24-37` requires that an unknown Patch produce a bounded observable failure while leaving other instruments unchanged. This is explicit Phase 2 increment 2 behavior.

### Implementation evidence

- `PreparedEngineRack::dispatch` correctly returns `RackDispatchError::UnknownPatch` in `src/real_time/prepared_engine_rack.rs:64-75`.
- `AudioRenderer` immediately converts that result to `is_ok()` and discards the actual failure in `src/real_time/audio_renderer.rs:83-90`.
- `AudioObservationSnapshot` has no routing-failure field, and no other callback-to-control status reports it.
- `tests/prepared_engine_rack.rs:331-352` injects an unknown Patch through the production renderer and proves that no instrument was misrouted, but does not assert that the required failure was observable.

### Contradiction and consequence

The rack component honors its local API, but the production caller erases the status. “No incorrect instrument received the command” proves routing isolation; it does not prove observable failure. Production therefore turns a declared error into an indistinguishable no-op.

### Required repair and witness

- Preserve a fixed-size routing-failure observation through an existing appropriate latest-value status or a dedicated bounded observation field.
- Keep all callback-side work allocation-free and non-blocking.
- Extend the production renderer witness to assert both independent obligations: no fallback/mutation and an incremented or otherwise exact observable routing failure.
- Retain the direct rack error test, but do not treat it as coverage for the caller that consumes the result.

## V-005 — Three declared validation selectors execute zero tests

### Authority and applicability

`spec/realtime.cue:314-318` declares three test validations as evidence for the current `AudioRenderer` contract:

- `cargo test audio_renderer_realtime_contract`;
- `cargo test prepared_graph_handoff`;
- `cargo test audio_observation_realtime_contract`.

These are current proof declarations attached to an implemented application service.

### Execution evidence

Each command exits successfully while selecting zero tests. Related tests exist under other names, including renderer allocation, graph swap, retirement pressure, atomic observation, and heterogeneous routing tests, and the broad `cargo test --all-targets` suite executes them. That broad success does not make the three declared targeted commands truthful.

### Contradiction and consequence

The CUE validation records claim to execute specific behavioral proof, but their selectors exercise nothing. Any acceptance flow that checks only process exit can report these resource validations as passing without running the declared witness.

### Required repair and witness

- Rename or add tests so each declared selector executes its intended coherent witness, or update each declaration to an exact existing target that proves the stated behavior.
- Require structured evidence that every test-bearing validation executed at least one test under its own declared selector.
- Do not borrow execution counts from `cargo test --all-targets` or another validation.
- Add a controlled acceptance test proving that a zero-selection command fails even when the broad suite passes.

## Verified aligned boundaries

The review found the following implemented boundaries materially aligned with current CUE/OpenSpec:

- semantic inputs enter `AppLoop` and mutate canonical state only through `AppState::apply`;
- view, serialized state, and audio parameters project from accepted canonical state;
- Patch instrument configuration and capability metadata are generic and reject unknown configurations without fallback;
- discrete commands, latest scalar snapshots, and prepared/retired structural graph ownership use distinct transports;
- the prepared rack and Patch stems preserve bounded ordered Patch identity;
- complete graph replacement occurs at block boundaries;
- retirement pressure retains graph ownership for retry, and replaced graph destruction occurs on control ownership;
- production-path behavioral tests measure nonzero audio, routing isolation, projection agreement, mutation counterexamples, and callback allocation/destruction behavior.

## Verification state

With the required local `sf2/HiDef.sf2` fixture present:

- evaluated CUE context succeeds;
- `openspec validate --all --strict --json` passes all 9 canonical specifications;
- formatting and clippy pass;
- `cargo test --all-targets` passes 244 tests;
- every named project acceptance target emits its required marker;
- the three validation selectors in V-005 still execute zero tests and remain open proof gaps.

Passing broad checks do not supersede the five specific contradictions above.
