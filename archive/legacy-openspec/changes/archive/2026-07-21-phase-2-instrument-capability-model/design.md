## Context

The current Rust model makes `SoundFontInstrument { bank, program, percussion }` a direct field of `Patch`. The same shape is repeated in startup installation, event records, state serialization, `StateTree`, text rendering, tests, and the SoundFont adapter. A PATCH page or Braids integration built on that model would either add an engine enum with fields for every implementation or scatter capability-id branches across the reducer and views.

`DESIGN.md` now defines descriptors and generic configs as the durable model, and `ROADMAP.md` makes this capability foundation the first checkpoint of Phase 2. The evaluated CUE architecture is authoritative. It preserves:

- physical input → semantic event → `AppState::apply` → projections;
- the existing hard-real-time callback and its three distinct transport categories;
- capability ports with explicit errors and no silent fallback;
- one canonical type per concept and thin adapters;
- PATCH and MIXER as the only eventual top-level UI contexts;
- measured proof through the production reducer and projector.

This is a control/domain migration. SoundFont remains the only prepared renderer, and immutable instrument configs are still installed before audio starts. No new data crosses the audio boundary in this increment.

## Goals / Non-Goals

**Goals:**

- Make `Patch` independent of any engine-specific field shape.
- Define one strongly typed, serializable descriptor/config model that SoundFont uses now and later engines can use without changing `Patch`.
- Make installed capability metadata immutable canonical application state and the source for validation, serialization, projection, and schema coverage.
- Keep the provider port itself polymorphic: it accepts generic assignments/assets and has no SoundFont-specific operation.
- Preserve current SoundFont playback, fixture behavior, mixer parameters, text controls, callback safety, and no-fallback startup semantics.
- Add falsifiable acceptance for exact descriptors/configs and malformed configuration rejection.

**Non-Goals:**

- A prepared engine rack or a generic real-time rendering port.
- Braids source import, C++ compilation, FFI, voice management, rendering, or parameter schema.
- Simultaneous mixed-engine playback, engine selection, layering, or fallback.
- Editing instrument config after installation, including Scalar publication or Structural graph replacement.
- The PATCH page, preset browser, ADSR, modulation, per-Patch effects, or a visual redesign.
- Renaming the fixture-facing `SoundFontInstrument`; it remains a narrow source identity until later fixture work requires a broader model.

## Decisions

### 1. Use closed typed values plus stable semantic identifiers

The Synth context will own `CapabilityId`, `ParameterId`, `AssetReference`, `ParameterValue`, `ParameterAssignment`, `ParameterSpec`, `CapabilityDescriptor`, `InstrumentConfig`, and `CapabilityRegistry`. Rust enums will represent parameter/asset kinds and values; there will be no `Any`, stringly typed value payload, callback, widget, or engine object in the schema.

Identifiers are namespaced stable values such as `instrument.soundfont.hidef` and `soundfont.program`. Labels and semantic accents are presentation metadata and never identity. Descriptor section and parameter order is canonical for serialization and presentation, while behavior looks up values by `ParameterId`.

`ParameterSpec.update` records `Scalar` versus `Structural` now, even though both are immutable after installation in this increment. This avoids changing the descriptor contract when later work adds latest-value scalar publication and prepared structural graph handoff.

Alternative considered: an `EngineType` enum with SoundFont and Braids payload variants. Rejected because every new engine would modify the central Patch type, reducer, serializer, and views, defeating the required polymorphism.

Alternative considered: a `HashMap<String, serde_json::Value>`. Rejected because it loses type, order, bounds, dependency, and exhaustive-validation guarantees and invites UI/engine-specific runtime branching.

### 2. Keep one immutable descriptor registry in canonical `AppState`

The composition root constructs `HiDefSoundFontCapability`, obtains its descriptor, creates a nonempty `CapabilityRegistry`, and passes that registry into `AppState` construction. Registry construction rejects duplicate capability, section, parameter, choice, asset, or MIDI-kind identities and invalid defaults/dependencies/capacity.

`AppState::apply(InstallPatches)` validates every candidate `InstrumentConfig` against the registry before committing any Patch. A bad config yields the externally reachable `InvalidInstrumentConfig` rejection; because `apply` already reduces against a clone and commits only on success, installation remains atomic. The registry is never mutated by an event.

The registry contains metadata only. It does not contain providers, renderer factories, decoded assets, closures, buffers, devices, or destructor-bearing prepared state.

Alternative considered: a process-global registry. Rejected because it would introduce hidden state, make deterministic fixtures harder to isolate, and bypass the canonical serialized state used by proof.

Alternative considered: copy the descriptor into every Patch. Rejected because copies can drift and inflate state; a Patch carries only the capability id and config values/assets.

### 3. Keep the provider port generic and the fixture translation narrow

`InstrumentCapabilityProvider` exposes:

- `descriptor() -> CapabilityDescriptor`;
- `create_config(values, asset_references) -> Result<InstrumentConfig, CapabilityError>`.

It runs only on the control side. `HiDefSoundFontCapability` declares the one current descriptor and validates generic values/assets against it. The port has no `SoundFontInstrument` method; otherwise every future provider would be forced to implement a SoundFont concern.

`AutomaticMidiTest` remains a fixture-specific adapter service. It translates each discovered `SoundFontInstrument` into assignments for `soundfont.bank`, `soundfont.program`, `soundfont.percussion`, and `soundfont.file`, then asks the provider to create the config. This localized knowledge does not leak into `Patch`, `AppState`, the projector, or the provider contract.

Alternative considered: make the provider accept an open bag of source hints. Rejected because it recreates a dynamically typed escape hatch. Alternative considered: add a source-specific method to the generic provider. Rejected because it makes the future Braids provider depend on SoundFont vocabulary.

### 4. Replace the Patch instrument field and migrate serialization as one breaking slice

`Patch.instrument: SoundFontInstrument` becomes `Patch.instrument: InstrumentConfig`. `SoundFontInstrument` remains owned by the Synth context and used by MIDI discovery/translation, but it is no longer canonical Patch state.

Startup installation records, snapshots, and `StateTree` change from flat `bank`, `program`, and `percussion` fields to:

```text
instrument
├── capabilityId
├── values[] { parameterId, value }
└── assetReferences[] { parameterId, reference }
```

The serialized `StateTree` schema version increments because this is an intentional breaking observation-schema change. The new tree also includes the ordered installed capability descriptors. Decode/encode round-trip checks and exact leaf discovery cover the complete nested values rather than opaque debug strings.

There is no persistence migration because the repository has no stored sessions or patches. All configs are recreated deterministically at startup.

### 5. Adapt SoundFont without pretending the runtime is already polymorphic

`SoundFontEngine` remains the current render port for this increment. `HiDefSoundFontEngine::configure_patch` requires capability id `instrument.soundfont.hidef`, extracts bank/program/percussion/file by stable parameter id after validation, and returns a typed error for every other capability or malformed config. It must not select a default preset or renderer.

The shared parsed bank, per-channel prepared lanes, fixed PatchId/channel/stem routing, MIDI dispatch, `PatchAudioBlock`, `ParameterSnapshot`, `AudioBoundary`, `AudioRenderer`, and mixer remain unchanged. All generic config inspection occurs during control-side preparation; the callback receives no descriptor collections or dynamic config values.

Alternative considered: introduce the multi-engine rack in the same change. Rejected because it would combine the control schema migration with structural RT handoff and engine dispatch, making failures harder to isolate. The rack is the next roadmap checkpoint and will consume this config model.

### 6. Drive serialization and text from descriptors, but keep config read-only

`StateProjector` serializes the immutable registry and generic configs from accepted `AppState`. Text rendering resolves each Patch config to its descriptor and walks the descriptor's stable section/parameter order. Formatting uses the declared formatter and labels; it contains no match on SoundFont or future Braids capability ids.

The current selectable/editable rows remain only `ChannelParameters` and `GlobalParameters`. Instrument descriptor/config lines are visible but not selectable or editable. This preserves current keyboard behavior while laying the schema foundation for a later PATCH page.

`ParameterSnapshot` remains mixer-only because instrument config is structural and immutable in this increment. Adding descriptor collections to the latest-scalar RT transport would violate the transport separation and provide no runtime benefit.

### 7. Prove exact generic behavior through named production-path tests

A new `tests/capability_schema.rs` target constructs the production `HiDefSoundFontCapability` and registry, translates discriminating fixture identities, installs Patches through `AppState`/`AppLoop`, and inspects canonical state/text. It asserts:

- the exact single descriptor and ordered parameter metadata;
- exact config values/assets for multiple distinct instruments;
- exact reducer installation, serialization round trip, and generic text order;
- typed rejection with no partial commit or fallback for unknown, duplicate, missing, undeclared, wrong-kind, dependency-invalid, and out-of-range cases.

The existing schema-surface and exhaustive-demo tests expand their expected universe from the production descriptors and discovered nested leaves. Existing SoundFont, real-time, mixer, GUI-context, live-demo, mutation, formatting, lint, smoke, and all-target gates remain required.

### 8. Drain delayed fixture input across bounded polls

The descriptor-bearing state tree increases measured control-side projection work. A physical live-demo tick can therefore span a dense interval containing more due fixture events than one `FixedEventBatch` holds. `CorridorsMidiEventSource::poll` will fill only the batch's remaining fixed capacity, advance its cursor only for appended events, and retain the rest as overdue input for later polls. `AutomaticMidiTest` continues to dispatch one bounded batch per tick through `AppLoop`, so event ordering, the canonical reducer/projector path, and the hard-real-time boundary stay unchanged.

Alternative considered: increase `FIXED_EVENT_BATCH_CAPACITY`. Rejected because any fixed larger number can fail after a sufficiently delayed control tick and would hide the missing backlog contract. Alternative considered: drop or coalesce overdue MIDI. Rejected because it changes the deterministic fixture and invalidates causal playback evidence.

### 9. Share generation-only MIDI projections and bound interactive work

Profiling the physical fifteen-Patch live demo shows that the control thread, not synth DSP, is the bottleneck: every MIDI event deep-clones canonical state, serializes it, deserializes it for text, deserializes it again for `StateTree`, serializes the tree, and later clones retained history for repeated reads. This work turns one bounded 256-event catch-up batch into a visible stall even though MIDI changes no Patch, capability, mixer, or selection value.

`AppState::apply` will preserve the canonical reducer boundary but give `Midi` a read-only transaction: validate the Patch target, commit only the next generation, and return one command without replacing immutable registry/Patch storage. `StateProjector` will build one borrowed canonical serialized-state view for eager projection and will never deserialize Crest's own JSON on that production path. `AppLoop` will retain the current snapshot and fixed parameter projection. For an accepted MIDI generation, it will share the prior immutable state suffix, text body, and state-tree template, advance their generation/hash fields, publish parameters and the discrete command in the existing order, and materialize complete JSON only when an observer requests it.

Deferred output is an optimization of representation, not weaker evidence. A unit proof materializes the fast StateSnapshot and StateTree and requires exact equality with an eager `project_with_tree` call from the same accepted `AppState`. A named fifteen-Patch integration target dispatches 512 events through the real reducer, projector, journal, and publication boundary within a 50 ms unoptimized-test ceiling with no dropped records. The complete live EventLog remains in `LiveDemoReport`; the interactive binary emits only a compact lossless count/endpoint summary so terminal rendering cannot become the final bottleneck. The eframe adapter schedules idle frames at 16 ms instead of requesting immediate perpetual repaint, while native events can still wake it sooner, and `make demo-live` runs the optimized release binary.

Alternative considered: skip reducer generation or projection for performance MIDI. Rejected because it breaks the one-way state path, EventRecord chain, and generation-correlated audio proof. Alternative considered: batch many MIDI messages into one semantic state event. Rejected because it changes event ordering and evidence identity. Alternative considered: retain eager JSON and only raise the fixed batch capacity. Rejected because it increases the duration of a control-thread stall and leaves work proportional to descriptor/config document size.

## Risks / Trade-offs

- [Generic value types add more domain code before a second renderer exists] → Keep this increment bounded to types that are already required by the durable design and prove them by migrating the complete current SoundFont path.
- [Descriptors and configs can disagree despite sharing identifiers] → Centralize validation in canonical descriptor/registry operations, validate again at reducer installation, and use controlled malformed cases.
- [A generic provider could accidentally regain engine-specific methods] → Fix the port contract to generic assignments/assets and keep fixture translation outside the port.
- [Projection becomes more allocation-heavy on the control thread] → Preserve deterministic ordered vectors and avoid all descriptor/config work in the callback; optimize control-side storage only if measurement warrants it.
- [Projection latency can make more fixture MIDI due than one bounded poll can carry] → Fill one fixed batch per tick and retain the ordered backlog for later polls without loss, duplication, or capacity failure.
- [Lazy projection could diverge from canonical eager serialization] → Share only generation-invariant storage, use the same stable hash semantics, and force both deferred documents to materialize and compare byte-for-byte with eager output in tests.
- [Wall-clock performance assertions can be noisy] → Use a deliberately broad 50 ms debug-profile ceiling around a path that normally completes in a few milliseconds, and retain exact event-count/drop/equivalence assertions so timing alone cannot pass the test.
- [A complete live EventLog can overwhelm an interactive terminal] → Retain it in the typed report and deterministic tests, but emit only one compact lossless summary with chain endpoints from the live CLI.
- [An unconditional repaint loop can consume a core even after control projection is fast] → Schedule the next idle frame at 16 ms, retain event-driven wakeups, and run the physical demo in the release profile.
- [Observation consumers break when flat instrument fields move] → Increment `StateTree` schema version, update exact leaf descriptors/tests in the same change, and document the intentional breaking schema change.
- [The model can be mistaken for completed multi-engine support] → Keep one installed capability invariant and explicit non-goals; do not add an engine enum, rack, Braids stubs, selection, or fallback.
- [Future Braids needs metadata not anticipated here] → The descriptor includes voice capacity, MIDI semantics, assets, typed parameters, dependencies, and Scalar/Structural update class; add engine-owned parameters through descriptor data rather than Patch fields.

## Migration Plan

1. Add and unit-test the canonical identifiers, typed values, descriptors, configs, registry validation, and generic provider port without changing the callback.
2. Add `HiDefSoundFontCapability` and construct the exact one-entry registry at every production and test composition root.
3. Change `Patch` and `AutomaticMidiTest` to create and install generic configs; add atomic `InvalidInstrumentConfig` reducer rejection.
4. Migrate event-record, state-snapshot, `StateTree`, and text projection schemas together; increment the tree schema version and update production-owned leaf descriptors.
5. Adapt `HiDefSoundFontEngine` to read the validated generic config during preparation and retain the existing prepared render path.
6. Add the named capability-schema target and expand exhaustive/schema-derived verification.
7. Run all project checks. If rollback is necessary before merging, revert the slice as a unit; there is no persisted user data to transform and no partial runtime compatibility mode is supported.
8. Profile the physical live path, replace eager per-MIDI clone/parse/serialization with exact generation-only projection sharing, add the measured fifteen-Patch acceptance target, bound final terminal evidence with a compact journal summary, pace idle UI frames, and run the physical demo in release mode.
