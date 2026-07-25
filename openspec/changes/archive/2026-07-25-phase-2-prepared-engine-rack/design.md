## Context

Phase 2 increment 1 made `Patch` state, capability metadata, validation, and projection generic, but the running audio path still depends on `SoundFontEngine`: one object owns every SoundFont lane, `AudioRenderer` is generic over that concrete-shaped port, and `LockFreeAudioBoundary` combines command/scalar delivery with a transitional `basedrop` collector. That shape cannot host different engine implementations on different Patches and does not implement the structural ownership protocol already required by `DESIGN.md`.

The evaluated CUE architecture makes this the next bounded increment. Canonical resources added or changed by the architecture are `port.Synth.PreparedInstrument`, `port.Synth.InstrumentPreparer`, `applicationService.Synth.PreparedEngineRackBuilder`, `aggregate.RealTime.PreparedEngineRack`, `aggregate.RealTime.PreparedGraph`, `valueObject.RealTime.GraphRevision`, `port.RealTime.StructuralGraphBoundary`, `applicationService.RealTime.PreparedGraphBuilder`, `applicationService.RealTime.StructuralGraphCoordinator`, `applicationService.RealTime.AudioRenderer`, `adapter.HiDefSoundFontEngine`, and `adapter.LockFreeStructuralGraphBoundary`.

The hard real-time callback remains the primary constraint: bounded preallocated work only, with no allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwind, or owned-state destruction. Discrete commands, latest scalar snapshots, structural graph ownership, and callback observations must remain separate transports. The production application still installs only `instrument.soundfont.hidef`; heterogeneous engines are test fixtures in this increment, not product choices.

## Goals / Non-Goals

**Goals:**

- Make the callback runtime capability-neutral and able to own different prepared instrument implementations in different bounded Patch slots.
- Build a complete graph off the callback and transfer it as one ownership unit, never as a partially updated engine/effect topology.
- Swap a prepared graph only at a block boundary and guarantee that every replaced graph is returned for off-callback destruction, including under return-queue pressure.
- Correlate scalar parameter snapshots with graph structure so a snapshot is consumed only by its intended Patch ordering and capacities.
- Preserve exact Patch-targeted MIDI routing, independent stems, the existing global mixer/reverb/delay path, physical audio, deterministic demos, and the reducer-first control path.
- Remove the superseded SoundFont-shaped runtime port and `basedrop` path in the same implementation change.
- Establish measured, falsifiable proof before introducing Braids.

**Non-Goals:**

- Adding Mutable Instruments Braids source, a C++ toolchain, FFI, a second production capability, or any placeholder engine.
- Adding engine selection, the PATCH page, editable structural capability parameters, layering, modulation, arbitrary routing, persistence, or user-triggered graph edits.
- Adding per-Patch effect inserts or changing the one shared reverb and one shared delay.
- Expanding the active Patch bound or changing the authored mixer topology.
- Generalizing asset loading beyond the current HiDef SoundFont preparation needed to establish the port.

## Decisions

### 1. Split preparation from prepared rendering

`InstrumentPreparer` is a control/worker-side port. It advertises one stable capability identity and may validate configs, resolve/read assets, allocate voices and scratch, and return a typed preparation error. Its output is one owned `Box<dyn PreparedInstrument>` for one `Patch`.

`PreparedInstrument` is the callback-side, object-safe port. It exposes its immutable `PatchId`, bounded MIDI dispatch, bounded all-notes-off, and one render call that fills only caller-owned stereo storage for that Patch. A rack may dynamically dispatch once per targeted command and once per Patch per block; an implementation must not dynamically dispatch in its inner sample loop.

This split keeps asset and construction work out of the callback and prevents SoundFont concepts from becoming the universal runtime API. Keeping a single port with both preparation and rendering was rejected because it would allow file/loading behavior to leak into callback-owned objects. An enum of known engine implementations was rejected because each new engine would require modifying the central runtime and would not prove polymorphism.

### 2. Build the rack atomically from exact capability matches

`PreparedEngineRackBuilder` accepts the accepted ordered Patch slice, sample rate, frame capacity, and installed preparers. For each Patch, it requires exactly one preparer whose capability identity equals the Patch config identity, asks it to prepare one instrument, and verifies that the returned instrument reports the same `PatchId`. It rejects zero/duplicate matches, duplicate Patch identities, mismatched returned identities, invalid capacities, and more than `MAX_PATCHES` before publication. A failure may destroy partially prepared temporary values on the control/worker side, but it returns no rack and changes no active graph.

The rack stores a fixed-capacity ordered slot array. Slot order equals parameter-snapshot and stem order. Patch-targeted dispatch performs a bounded lookup and calls only the matching instrument; an unknown Patch returns a fixed-size status and never broadcasts or selects a fallback. Rendering clears the caller-owned stems and calls each active slot exactly once into its matching stem.

A hash map was rejected because lookup storage and growth are unnecessary at a sixteen-Patch bound. Sorting by capability or Patch identity was rejected because it could silently break the canonical accepted Patch order used by projections and mixing.

### 3. Treat the entire callback topology as one prepared graph

`PreparedGraphBuilder` first builds the rack, then prepares `PatchAudioBlock`, `MixEngine<GlobalReverbDelay>`, shared effect memory, routing, and all scratch for the declared sample rate and maximum frame count. It creates a nonzero monotonic `GraphRevision` and an initial fixed `ParameterSnapshot` carrying that same revision and the exact ordered Patch identities. Only the resulting complete `PreparedGraph` may reach the callback.

`AudioRenderer` is constructed from an initial prepared graph before audio starts; it no longer has a callback-side `prepare` operation and no longer separately owns an engine, mixer, or optional Patch block. Replacement graphs in this increment must have the same PatchId set as the active graph. This deliberately avoids inventing a user-facing structural `AppEvent` or partial state/graph transaction before the PATCH workflow is designed.

Publishing separate engine, mixer, effect, and routing objects was rejected because the callback could observe incompatible intermediate topology. Mutating an active graph in place was rejected because preparation, failure rollback, and destruction could then occur on the callback.

### 4. Use a dedicated two-queue structural boundary

`LockFreeStructuralGraphBoundary` owns two preallocated SPSC queues with opposite ownership directions:

- control/worker producer → audio consumer for complete prepared graphs;
- audio producer → control/worker consumer for retired graphs.

It also exposes a fixed-size, coherently readable handoff status containing active and retired revisions, applied swap count, and retirement retry count. Status publication cannot backpressure the callback. It does not reuse the `AudioCommand` ring, the `ParameterSnapshot` triple buffer, or the audio-observation transport.

Moving a graph or a box through these queues moves ownership without allocating or destroying its contents. `rtrb` remains the queue primitive because its full result returns the owned value to the caller. A shared mutex, reference-counted active graph swap, or command carrying a graph pointer was rejected because those designs either permit blocking/destruction on the callback or conflate distinct delivery semantics.

### 5. Apply a block-boundary, no-drop swap protocol

At the start of each render block, the callback first retries any graph held in its single preallocated retirement slot. If the retired queue is still full, it retains that graph, increments bounded status, and does not accept another prepared graph. If the slot is clear, it may pop at most one prepared graph, validate its fixed invariants, replace the active graph, and try to return the old graph. A full return queue puts the old graph into the now-empty retirement slot. No callback path drops either graph.

`StructuralGraphCoordinator` permits only one unacknowledged replacement publication. It does not publish another until status and the retired return establish completion, and it drains/destroys returned graphs on control/worker ownership. Queue pressure is therefore observable and bounded rather than silently losing topology.

Accepting all queued replacements in one block was rejected because it increases callback work and can require multiple retirement slots. Dropping the oldest prepared or retired graph was rejected because either action can run destructors on the callback and hides a failed ownership transfer.

### 6. Make scalar snapshots graph-compatible

`ParameterSnapshot` gains a copyable `GraphRevision`. `StateProjector` and `AppLoop` receive the target revision as runtime projection metadata and include it in the parameter snapshot and the StateTree parameters branch; it is not a second mutable synth state or graph owner. The production-owned leaf descriptor and schema coverage include the new field, with the affected serialized schema version advanced deliberately.

Each active graph retains its last compatible fixed snapshot, initialized from the snapshot embedded in the graph. At each block, the renderer accepts the latest scalar snapshot only when its revision, Patch count, and ordered PatchIds match the active graph; otherwise it keeps the last compatible snapshot and records bounded diagnostic status. A graph activation immediately has compatible initial parameters even if scalar publication is early, late, or skipped.

Trusting arrival order alone was rejected because the latest-value scalar buffer may advance before or after structural ownership transfer. Putting structural configs into the copyable scalar snapshot was rejected because decoded assets and engine/effect ownership are neither scalar nor safe to copy through the callback.

### 7. Adapt HiDef as one preparer with per-Patch prepared instruments

The HiDef adapter parses `./sf2/HiDef.sf2` once outside the callback and retains one shared immutable `Arc<SoundFont>`. For each accepted `instrument.soundfont.hidef` Patch, it creates a private prepared value containing its Patch identity, bank/program/percussion mapping, bounded rustysynth synthesizer, and preallocated render scratch. That private value implements `PreparedInstrument`; the preparer remains outside the graph.

This preserves one file parse and immutable bank while isolating voices and render state per Patch. Built-in rustysynth reverb and chorus remain disabled so the graph's one shared reverb and delay are the only effects. The adapter returns typed errors for a missing/invalid file, unsupported capability, invalid config/preset, or failed preparation and never substitutes another asset or engine.

Keeping all Patch lanes in one SoundFont-shaped runtime object was rejected because it cannot coexist cleanly with a Braids slot. Parsing or cloning the whole bank for every Patch was rejected because it wastes startup time and memory without improving isolation.

### 8. Install state, prepare the graph, then start producers

Automatic MIDI initialization prepares the fixed file and constructs generic Patch configs, validates the provider/registry match, and installs all Patches through `AppEvent::InstallPatches`. It no longer configures a sound engine or starts the source. The composition root then projects the accepted state with the initial graph revision, builds the complete graph, constructs the renderer and boundaries, and only after success starts the MIDI source and physical audio/window owners.

All startup failures are atomic with respect to audio: no MIDI event or device callback can target an absent or partially prepared graph. Normal, smoke, headless-demo, and live-demo composition share this sequence.

Starting the fixture during Patch discovery was rejected because due MIDI could race graph preparation. Allowing the fixture service to depend on a renderer/preparer was rejected because input discovery should not own sound generation.

### 9. Prove polymorphism and ownership at production seams

A named `prepared_engine_rack` integration target uses two behaviorally distinct deterministic test `PreparedInstrument` implementations in one rack. It proves exact command targeting, all-notes-off scope, distinct bounded stems, and that no central engine-identity branch is required. Separate cases exercise real HiDef preparation, missing/duplicate/mismatched preparers, over-capacity/partial failure, block-boundary swap and acknowledgement, one-in-flight throttling, a full retired queue, retained-slot retry, parameter-revision mismatch, and off-callback destruction.

Callback allocator instrumentation and drop sentinels surround the production renderer path; the acceptance marker is emitted only after structured observations show zero callback allocations and zero callback-owned destruction. Existing exhaustive, schema, GUI, mutation, real-time, smoke, performance, live, format, lint, and all-target gates remain required. A construction-only unit test was rejected because it would not falsify Patch misrouting, graph loss, or callback destruction.

## Risks / Trade-offs

- [Trait-object calls add dispatch overhead] → Permit dispatch only once per targeted event and once per active Patch per block, keep inner sample loops concrete, and measure the production render path.
- [Per-Patch rustysynth scratch and synth state increase bounded memory] → Share only the immutable parsed bank, retain explicit `MAX_PATCHES`, voice, and frame capacities, and fail preparation before publication when capacity cannot be honored.
- [Return-queue pressure could stall subsequent structural changes] → Retain exactly one old graph, retry every block, expose retry/acknowledgement status, and enforce one in-flight replacement; audio rendering continues on the active graph.
- [Scalar publication can race a graph swap] → Embed initial parameters in each graph and accept only revision- and Patch-order-compatible latest snapshots.
- [Adding `graphRevision` changes serialized evidence] → Advance the affected schema version and derive expected leaf coverage from production descriptors so stale fixtures fail explicitly.
- [Large cross-cutting migration can leave two runtime paths] → Remove `SoundFontEngine`, generic engine parameters on `AudioRenderer`, and `basedrop` only after the replacement compiles, then use repository-wide searches and all existing gates to prove no superseded path remains.

## Migration Plan

1. Add canonical revisions, prepared ports, rack/graph types, builders, and typed errors while the old renderer still compiles; prove builder behavior with deterministic prepared test instruments.
2. Split HiDef file parsing/preparation from its private prepared instrument and prove one shared bank plus independent Patch state.
3. Add the structural boundary, coordinator, status, return-pressure tests, and off-callback drop sentinels.
4. Convert `AudioRenderer` to own a complete prepared graph and implement compatible snapshot selection and the block-boundary swap protocol.
5. Recompose automatic fixture, shell, smoke, headless, and live startup around accepted Patch installation followed by initial graph preparation and producer start.
6. Add graph revision to canonical parameter/tree projection and production-derived schema evidence, advancing affected schema versions.
7. Remove the superseded `SoundFontEngine` port, transitional `basedrop` dependency and APIs, and any alternate construction path.
8. Run the new named acceptance target and every existing project gate. If migration fails before the final removal, keep the existing committed Phase 2 increment active; no persisted-user-data migration or dual runtime path is required.
