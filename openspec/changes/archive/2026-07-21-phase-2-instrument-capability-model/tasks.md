## 1. Canonical capability domain model

- [x] 1.1 Add Synth-owned modules and public types for `CapabilityId`, `ParameterId`, `AssetReference`, `ParameterValue`, and `ParameterAssignment`, including stable serde forms, identifier validation, equality, and focused unit tests.
- [x] 1.2 Add `ParameterSpec` and `CapabilityDescriptor` with ordered sections, kinds, defaults, Scalar/Structural update classification, bounds/choices/steps, formatter/dependency metadata, asset requirements, voice capacity, and MIDI semantics; reject every invalid or duplicate descriptor case in unit tests.
- [x] 1.3 Add `InstrumentConfig`, `CapabilityRegistry`, and typed capability/config errors; validate exact assignments/assets by semantic id while preserving descriptor order and prove unknown, duplicate, missing, undeclared, wrong-kind, dependency-invalid, and out-of-range cases fail without fallback.
- [x] 1.4 Export each canonical type from the existing Synth/library boundaries without adding an engine enum, dynamic value escape hatch, renderer factory, or new dependency.

## 2. SoundFont capability provider and composition

- [x] 2.1 Add the generic `InstrumentCapabilityProvider` port with only descriptor and generic assignment/asset config-construction operations; verify the public contract contains no SoundFont-specific input, rendering, device, UI, or file-loading operation.
- [x] 2.2 Implement `HiDefSoundFontCapability` with the exact `instrument.soundfont.hidef` descriptor: ordered structural bank, program, percussion, and required `./sf2/HiDef.sf2` asset specs plus current bounded voice and MIDI metadata.
- [x] 2.3 Update production, smoke, live-demo, headless-demo, witness, and test composition roots to construct the provider, build the exact one-entry immutable registry, and inject both through existing services; fail startup on missing, duplicate, invalid, or mismatched registration without fallback.

## 3. Generic Patch installation through the reducer

- [x] 3.1 Replace `Patch`'s `SoundFontInstrument` field with immutable `InstrumentConfig`, update its constructor/accessors/tests, and keep only `ChannelParameters` mutable after installation.
- [x] 3.2 Update `AutomaticMidiTest` to translate each fixture `SoundFontInstrument` into the provider descriptor's stable bank/program/percussion/file assignments, create the config through the generic provider, and propagate conversion or registry disagreement before engine configuration or state installation.
- [x] 3.3 Add the immutable `CapabilityRegistry` to `AppState`, require it at construction, and validate all candidate Patch configs atomically inside `AppState::apply(InstallPatches)` with the externally observable `InvalidInstrumentConfig` rejection.
- [x] 3.4 Migrate every production/test Patch fixture and event-rejection descriptor to the canonical config and registry path; prove a rejected installation leaves registry, Patches, selection, generation, projections, and emitted effects unchanged and later valid input remains processable.

## 4. Descriptor-driven canonical projections

- [x] 4.1 Replace flat SoundFont fields in installation `EventRecord` payloads and canonical state snapshots with the complete generic capability id, ordered assignments, and asset references.
- [x] 4.2 Add the ordered installed descriptors and generic Patch configs to `StateTree`, increment its schema version, update deterministic serde round-trip/equality checks, and update production-owned serialized-leaf descriptors for every nested field and tagged value form.
- [x] 4.3 Change text projection to resolve each config through the registry and render capability label/id plus every value/asset in descriptor order using declared formatting metadata, with no SoundFont/Braids id branch or engine-specific duplicate field list.
- [x] 4.4 Preserve current selection/edit behavior: only mixer `ChannelParameters` and `GlobalParameters` are editable, instrument config remains immutable, and `ParameterSnapshot` plus the real-time transports remain unchanged.

## 5. Current SoundFont renderer adaptation

- [x] 5.1 Update `HiDefSoundFontEngine::configure_patch` to require `instrument.soundfont.hidef`, extract bank/program/percussion/file by stable parameter id during control-side preparation, and return typed errors for every other or malformed config without selecting a default preset, asset, or renderer.
- [x] 5.2 Preserve the single parsed bank, bounded per-channel lanes, PatchId/channel/stem routing, fixed `PatchAudioBlock`, MIDI dispatch, disabled built-in effects, and no-allocation/lock/block/I/O/log/destruction callback behavior; extend existing engine tests to show audio/routing behavior is unchanged after the config migration.

## 6. Executable Phase 2 increment proof

- [x] 6.1 Add `tests/capability_schema.rs` as a named integration target that uses the production provider, registry, reducer, Patch, serializer, and projector to prove the exact descriptor/config values for multiple instruments and every declared invalid/no-fallback case before printing `CREST_ACCEPTANCE capability_schema passed`.
- [x] 6.2 Expand `schema_surface`, `exhaustive_demo_scene`, live-demo fixtures, GUI-context fixtures, mutation fixtures, and shared test support to derive the new capability/config universe from production descriptors, assert exact nested values in both directions, and preserve deterministic two-run equality and existing causal audio predicates.
- [x] 6.3 Run focused validation: `cargo test capability_registry`, `cargo test hidef_soundfont_capability`, `cargo test app_state`, `cargo test state_projector_exact_projection_values`, `cargo test hidef_soundfont_engine`, and `cargo test --test capability_schema -- --nocapture`.
- [x] 6.4 Run the complete gates: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, `cargo run --bin crest-synth -- --smoke`, and every named schema, exhaustive-demo, GUI-context, mutation, and live-demo integration validation; do not mark the change complete if any prior behavior or the new capability proof fails.
- [x] 6.5 Reproduce the physical `make demo-live` fixed-batch overflow, make delayed MIDI polling retain and drain overdue events through bounded batches in exact order, add a production-fixture regression, and rerun the live target plus complete project gates.
- [x] 6.6 Profile the physical fifteen-Patch live control loop, remove per-MIDI immutable-state cloning and self-JSON deserialization, add exact lazy/eager projection equivalence, and require 512 production-path MIDI dispatches within 50 ms with no dropped records.
- [x] 6.7 Replace the completed live terminal's full EventLog dump with a compact lossless count/chain-endpoint summary while retaining the complete typed journal in `LiveDemoReport`, schedule idle eframe redraws at 16 ms, run `make demo-live` through the optimized release profile, and then rerun focused, physical-live, and complete project gates.
