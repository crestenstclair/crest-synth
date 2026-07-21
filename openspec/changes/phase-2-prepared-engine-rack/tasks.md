## 1. Canonical Prepared-Instrument Contracts

- [ ] 1.1 Add the copyable nonzero `GraphRevision` value and extend `ParameterSnapshot` construction, compatibility checks, getters, and fixed-storage tests with graph revision and exact ordered Patch identities.
- [ ] 1.2 Replace the SoundFont-shaped runtime trait with object-safe `PreparedInstrument` and control/worker-side `InstrumentPreparer` ports, including fixed-size callback statuses and typed preparation errors with no fallback.
- [ ] 1.3 Implement the fixed-capacity ordered `PreparedEngineRack` with exact `PatchId` lookup, targeted dispatch, Patch-scoped and global all-notes-off, caller-owned stem rendering, and no engine-identity branching.
- [ ] 1.4 Implement `PreparedEngineRackBuilder` so every accepted Patch has exactly one capability-matched preparer and missing, duplicate, mismatched, duplicate-Patch, invalid-capacity, over-capacity, and partial-preparation cases fail atomically off the callback.
- [ ] 1.5 Add focused unit tests using two distinct deterministic prepared instrument types to prove bounded heterogeneous dispatch, exact stem ownership, unknown-Patch behavior, and builder failure cleanup.

## 2. HiDef SoundFont Preparation

- [ ] 2.1 Refactor the HiDef adapter into one `InstrumentPreparer` that validates `instrument.soundfont.hidef`, opens and parses `./sf2/HiDef.sf2` once outside the callback, and retains one shared immutable parsed bank.
- [ ] 2.2 Move each Patch's rustysynth synthesizer, preset/percussion mapping, voice state, and preallocated left/right render scratch into a private `PreparedInstrument` implementation with built-in reverb and chorus disabled.
- [ ] 2.3 Prove one bank parse supplies independent melodic and percussion prepared instruments, exact targeted MIDI and finite bounded stems, invalid configs/presets/assets fail without substitution, and prepared operations allocate and destroy nothing in callback scope.

## 3. Complete Prepared Graph

- [ ] 3.1 Add `PreparedGraph` as the sole owner of one revision, initial compatible parameters, prepared rack, bounded Patch stems, prepared mixer, shared reverb/delay state, routing, sample rate, frame capacity, and scratch needed by the callback.
- [ ] 3.2 Implement `PreparedGraphBuilder` to prepare the complete graph off-callback and validate revision, sample/frame capacities, and exact rack/parameter/stem/mixer Patch order before returning ownership.
- [ ] 3.3 Add graph-builder tests proving successful complete preparation and atomic failure for an invalid instrument, effect, route, revision, parameter layout, sample rate, or frame capacity without changing an existing graph.

## 4. Structural Ownership Transport

- [ ] 4.1 Define split control/worker and audio handles for `StructuralGraphBoundary`, plus a fixed-size coherently readable `GraphHandoffStatus` covering active/retired revisions, applied swaps, retirement retries, and incompatible snapshots.
- [ ] 4.2 Implement `LockFreeStructuralGraphBoundary` with separately preallocated prepared-graph and retired-graph SPSC queues and prove it shares no storage or API with commands, scalar snapshots, or audio observations.
- [ ] 4.3 Implement `StructuralGraphCoordinator` publication, one-unacknowledged-replacement throttling, acknowledgement tracking, returned-graph draining, and control/worker-side destruction.
- [ ] 4.4 Add ownership/drop-sentinel tests proving full queues return the same owned graph, status publication does not backpressure audio, and graph destructors run only when returned graphs are collected outside callback scope.

## 5. Renderer Graph Ownership and Swap Protocol

- [ ] 5.1 Refactor `AudioRenderer` to be constructed from one complete initial `PreparedGraph`; remove its SoundFont engine generic, separate mixer/Patch-block ownership, optional preparation state, and callback-side `prepare` operation.
- [ ] 5.2 At each block start, retry the fixed retirement slot first, refuse another replacement while it is occupied, activate at most one waiting graph, and return or retain the old graph without any callback drop.
- [ ] 5.3 Route commands and all-notes-off through the active rack, render its exact bounded stems through the active graph's mixer, and preserve existing active-note and mixer observation semantics.
- [ ] 5.4 Retain the last graph-compatible scalar snapshot, accept a latest snapshot only when revision and ordered Patch layout match, and use each newly active graph's embedded initial snapshot when scalar publication is early, late, or incompatible.
- [ ] 5.5 Add renderer tests for exact heterogeneous routing, finite output, activation only at block boundaries, at-most-one swap per block, retirement pressure and retry, second-graph throttling, snapshot races, and status/observation counters.
- [ ] 5.6 Instrument the production callback path with allocator and destructor sentinels and prove zero allocation, deallocation, owned-state destruction, panic, logging, or blocking during normal rendering and graph-pressure cases.

## 6. Control Projection and Startup Composition

- [ ] 6.1 Pass the target `GraphRevision` into `StateProjector` and `AppLoop` as runtime projection metadata, include it in every parameter snapshot and StateTree parameters branch, and preserve the optimized MIDI generation-only projection path.
- [ ] 6.2 Add `graphRevision` to the production-owned leaf descriptor and schema-surface discovery, deliberately advance every affected serialized schema version, and update exact deterministic fixtures rather than excluding the new leaf.
- [ ] 6.3 Split `AutomaticMidiTest` into Patch discovery/installation and explicit source start; remove all sound-engine configuration from the input service and reject tick calls until start completes.
- [ ] 6.4 Recompose normal, smoke, observation, headless-demo, and live-demo startup to install Patches through `AppState::apply`, project the initial revision, build the complete graph, construct audio ownership, then start MIDI and physical audio/window producers.
- [ ] 6.5 Poll structural status and collect returned graphs from the existing control-side tick without adding a user structural `AppEvent`, engine selector, or second state owner.
- [ ] 6.6 Update smoke, live, event-log, StateTree, and witness observations to report the one parsed SoundFont bank, per-Patch prepared instruments, active graph revision, and zero callback destruction while keeping compact live output and 16 ms optimized repaint behavior.

## 7. Remove the Superseded Runtime Path

- [ ] 7.1 Delete the `SoundFontEngine` port and its engine-wide configure/render APIs after every caller uses preparers, prepared instruments, and the rack; retain no compatibility adapter or silent fallback.
- [ ] 7.2 Remove `basedrop`, generic retired-state APIs, collector calls, and the combined retirement responsibility from `LockFreeAudioBoundary`, leaving only the discrete command ring and latest scalar snapshot there.
- [ ] 7.3 Update crate exports, module declarations, Cargo metadata, documentation, and composition types, then use repository-wide searches to prove no stale `SoundFontEngine`, `basedrop`, old renderer preparation, or alternate graph-retirement path remains.

## 8. Behavioral Acceptance and Project Gates

- [ ] 8.1 Add `tests/prepared_engine_rack.rs` using production builders, renderer, structural boundary, coordinator, HiDef preparer, and two distinct deterministic prepared instrument implementations.
- [ ] 8.2 Make the named target prove exact capability matching, targeted commands, isolated stems, HiDef preparation, complete block-boundary swaps, graph-revision acknowledgement, one-in-flight throttling, full-return-queue retention/retry, compatible snapshots, zero callback allocation/destruction, and off-callback drops before printing `CREST_ACCEPTANCE prepared_engine_rack passed`.
- [ ] 8.3 Update existing capability-schema, schema-surface, exhaustive-demo, GUI-context, mutation, live-demo, control-dispatch-performance, and smoke support so their original falsifiable predicates still exercise the production reducer and render path through the prepared graph.
- [ ] 8.4 Run `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`; resolve every failure without weakening an assertion or callback invariant.
- [ ] 8.5 Run `cargo test --test prepared_engine_rack -- --nocapture`, `cargo test --test capability_schema -- --nocapture`, `cargo test --test control_dispatch_performance -- --nocapture`, `cargo test --test exhaustive_demo_scene -- --nocapture`, `cargo test --test schema_surface -- --nocapture`, `cargo test --test eframe_context -- --nocapture`, `cargo test --test behavioral_mutation_harness -- --nocapture`, and `cargo test --test live_demo_scene -- --nocapture`, and verify every declared acceptance marker.
- [ ] 8.6 Run `make smoke`, `make demo`, and the optimized physical `make demo-live` listening/performance check; verify responsive playback, compact live output, one shared reverb and delay as audio effects, no callback drops, and no sustained CPU regression.
- [ ] 8.7 Run `openspec context --json`, `openspec validate phase-2-prepared-engine-rack --strict`, and `openspec validate --all --strict` to prove the implementation and completed task record remain coherent with the evaluated CUE architecture and all permanent specs.
