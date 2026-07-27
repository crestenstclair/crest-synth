## 1. Effect Domain and Capability Contracts

- [x] 1.1 Add canonical `EffectCapabilityId`, `EffectSlotId`, `EffectCapabilityDescriptor`, `PostEffectConfig`, and `EffectCapabilityRegistry` modules and exports while reusing the existing parameter/value/asset types.
- [x] 1.2 Implement descriptor and zero-or-one Patch effect-config validation for unique stable identities, exact ordered assignments, kinds, bounds, dependencies, and typed no-fallback failures.
- [x] 1.3 Add `EffectCapabilityProvider`, `EffectPreparer`, and object-safe `PreparedPostEffect` ports with control/worker/callback ownership separated from instrument ports.
- [x] 1.4 Extend canonical `Patch` and its serialization/equality/hash behavior with ordered `post_effects` while adding no Chorus-specific, bypass, prepared, or UI fields.
- [x] 1.5 Extend `AppState::InstallPatches` and production-constructor registration checks for effect registries/configs/providers/preparers, including `InvalidEffectConfig` and all duplicate/missing/mismatch cases.
- [x] 1.6 Add focused unit tests for effect identity/config/registry/provider validation, zero/one capacity, stable slots, separate instrument/effect semantics, and atomic unchanged rejection.

## 2. Canonical PATCH Control and Projection

- [x] 2.1 Extend `PatchControlId` with stable effect slot/parameter targeting and update its production-owned descriptor and serialization.
- [x] 2.2 Extend the one canonical PATCH resolver to order Engine, ADSR, visible instrument structural choices, then configured effect identity and `ScalarEdit` rows without processor-specific matching.
- [x] 2.3 Implement generic effect sections/identity/parameter rows in `PatchPageProjection`, `TextProjection`, StateTree, and the basic eframe window; render no placeholder for unconfigured Patches.
- [x] 2.4 Route effect fine/coarse adjustments through `AppState::apply`, changing exactly one assignment while preserving structural lifecycle, graph revision, commands, and unrelated state.
- [x] 2.5 Add a separate fixed zero-or-one effect section to `ParameterSnapshot` and project stable slot id plus at most eight descriptor-ordered finite scalars without destructors.
- [x] 2.6 Advance every affected state/page/text/parameter schema version and update exact serialized-leaf descriptors and bidirectional schema-equality expectations.
- [x] 2.7 Extend PATCH/reducer/projection tests for configured and unconfigured Patches, nonwrapping focus, read-only identity, fine/coarse edits, boundaries, stale targets, pending structural work, and scalar-only effects.

## 3. Prepared Effect Rack and Renderer Order

- [x] 3.1 Implement fixed-size `RtPostEffectParameters`, `PatchEffectObservation`, and `PreparedPostEffectRack` with Patch/slot/layout checks and independent instance ownership.
- [x] 3.2 Implement `PreparedPostEffectRackBuilder` with exact registry/preparer matching, zero/one slot capacity, atomic cleanup, negotiated rate/frame capacity, and returned Patch/slot validation.
- [x] 3.3 Extend `PreparedGraph` and `PreparedGraphBuilder` to own aligned engine/effect racks and a compatible initial effect scalar layout before publication.
- [x] 3.4 Change `AudioRenderer` to render each engine stem, measure/process the matching effect rack in place, and only then call `MixEngine`.
- [x] 3.5 Extend `AudioObservationSnapshot` and its atomic transport with coherent fixed pre/post/difference/side effect measurements and bounded routing/layout failure status.
- [x] 3.6 Keep `MixEngine` ownership limited to Patch gain/pan/sends/mute/solo plus the one shared reverb and delay, and verify every send/global effect consumes post-effect stems.
- [x] 3.7 Extend structural transport/retirement instrumentation so complete graphs return and destroy every prepared effect only on control/worker ownership under normal and full-return-queue cases.
- [x] 3.8 Add deterministic prepared-rack/renderer tests for zero-slot pass-through, exact order, layout rejection, target isolation, two independent stateful instances/tails, finite output, and zero callback allocation/deallocation/destruction.

## 4. Pinned Chorus Adapter

- [x] 4.1 Vendor only the required Mutable Instruments Rings Chorus/FxEngine/resource/stmlib source subset at eurorack `08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4` and stmlib `e3bd7c9cc00e4364166f9905c0509b6ffd0535ec`.
- [x] 4.2 Add upstream MIT notices, `vendor/chorus/PROVENANCE.md`, and a complete SHA-256 manifest; add a test that rejects a missing, changed, or unrelated vendored file.
- [x] 4.3 Add an opaque exception-free/RTTI-free Chorus C ABI and extend `build.rs` to compile only its audited source list alongside the existing Braids library.
- [x] 4.4 Implement `ChorusCapability` with product label `Chorus` and exact Amount/Depth order, normalized bounds, defaults, fine/coarse steps, and `ScalarEdit` classifications.
- [x] 4.5 Implement `ChorusPreparer` and RAII ownership with one distinct initialized processor, 2,048-sample 16-bit delay buffer, and LFO/tail state per slot; accept exactly 48 kHz and fail every other rate without bypass.
- [x] 4.6 Add adapter tests for exact schema, pins/hashes/license, FFI lifecycle, scalar response, finite stereo difference/side energy, independent buffers/tails, malformed layout, unsupported rate, and bounded render timing.

## 5. Complete Structural Workflow and Production Composition

- [x] 5.1 Extend graph-preparation requests/candidates and validation so preset/engine changes preserve every Patch effect slot, config, parameter identity, order, and scalar layout exactly.
- [x] 5.2 Refresh candidate initial effect scalars from the latest committed generation and prove Amount/Depth edits during Preparing and Activating cannot be reverted at activation.
- [x] 5.3 Reject any candidate effect-config/layout drift and ensure all partial/current/candidate effect ownership is destroyed only off callback across success, worker failure, stale result, publication pressure, and shutdown.
- [x] 5.4 Inject and freeze `ChorusCapability`, `ChorusPreparer`, and `PreparedPostEffectRackBuilder` through `StandaloneApplication` and the binary composition root without constructing hidden concrete adapters in application services.
- [x] 5.5 Update `AutomaticMidiTest`/production Patch construction to configure one stable Chorus slot on the first fixture Patch only and no slots on all remaining Patches.
- [x] 5.6 Update normal, smoke, observation, deterministic-worker, threaded-worker, and physical-device startup paths to build the same complete engine/effect graph before MIDI or stream start.
- [x] 5.7 Extend production-runtime and engine/preset workflow tests for effect registration, negotiated exact-rate preparation, structural preservation, source/candidate audio continuity, teardown, and zero fallback/bypass.

## 6. Deterministic and Live Demo Coverage

- [x] 6.1 Derive effect capability/slot/control/parameter/rejection/serialized-leaf expectations in `DemoScene` from production descriptors/configs/resolvers before dispatch.
- [x] 6.2 Extend `ExhaustiveGuiDemo` and its report with reversible Chorus Amount/Depth edits, exact projections, scalar-only publication, effect-stage order/difference/side energy, unconfigured-Patch isolation, and restored baselines.
- [x] 6.3 Add a focused two-Chorus deterministic graph inside acceptance to prove independent instance/LFO/delay/tail state without changing the one-Chorus production fixture contract.
- [x] 6.4 Preserve and assert the exact Chorus config/layout through the deterministic adjacent-preset and two-direction engine sequence, controlled failures, activation, retirement, and final default-SoundFont state.
- [x] 6.5 Extend `LiveDemoScene`, checkpoints, report, and coverage to navigate/edit Amount and Depth exactly once through PATCH with semantic NoteOn/NoteOff probes and exact-generation physical effect observations.
- [x] 6.6 Require every live structural checkpoint and final StateTree to retain the Chorus config/layout and require final note cleanup, zero active notes, window close, stream release, effect-graph collection, worker shutdown, and inert completion.
- [x] 6.7 Make live no-progress/total-time errors stage-specific for effect observation as well as structural/cleanup stages and guarantee bounded teardown with no completed report on failure.

## 7. Focused Acceptance and Regression Gates

- [x] 7.1 Create `tests/static_patch_effect.rs` using production provider/preparer, reducer/projector, complete graph, renderer, effect rack, mixer, and observation seams; emit structured evidence and the acceptance marker only after every predicate passes.
- [x] 7.2 Extend `capability_schema`, `patch_page_projection`, `schema_surface`, `prepared_engine_rack`, `engine_selection_workflow`, `soundfont_preset_selection`, and production runtime targets with their declared effect contracts.
- [x] 7.3 Extend `exhaustive_demo_scene` and `live_demo_scene` targets for exact effect coverage, schema versions, order, isolation, independent instances, structural preservation, cleanup, and non-vacuous markers.
- [x] 7.4 Run `cargo test --release --test static_patch_effect -- --nocapture` and confirm exact pins/hashes/license, Amount/Depth cases, target/stereo/independence/preservation predicates, zero fallback/callback hazards, and p99 below 2.666 ms.
- [x] 7.5 Run affected named integration targets individually with `--nocapture`, then `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --all -- --check`; fix every failure without weakening assertions.
- [x] 7.6 Run `make smoke` and the actual deterministic `make demo`; verify the complete effect-aware reports have zero missing/unexpected coverage and normal process exit.
- [x] 7.7 Run the actual release-mode physical `make demo-live` yourself (not `make -n`), hear/observe both Chorus edits and all three structural transitions, and require semantic cleanup, final records, window close, stream/worker/graph teardown, and parent exit code 0 before checking this task.
