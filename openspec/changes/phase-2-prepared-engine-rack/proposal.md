## Why

The current runtime is capability-polymorphic in control state but still hard-wires one SoundFont renderer and one transitional retirement mechanism into the audio path. Before Braids or the PATCH page can be added safely, Crest Synth needs the bounded prepared-engine rack and complete structural graph handoff required by `DESIGN.md`, with ownership transfer and destruction kept off the hard real-time callback.

## What Changes

- Add a fixed-capacity `PreparedEngineRack` that owns one capability-neutral prepared instrument per installed Patch, routes commands by `PatchId`, and renders each Patch into its matching bounded stem.
- Separate control/worker-side instrument preparation from callback-side instrument dispatch and rendering through `InstrumentPreparer` and `PreparedInstrument` ports.
- Prepare one complete `PreparedGraph` off the callback, including the engine rack, Patch stems, mixer, shared reverb/delay state, routing, capacities, and initial parameter snapshot.
- Add a dedicated bounded structural boundary for prepared control-to-audio graph ownership, retired audio-to-control graph ownership, and fixed-size handoff status; graph swaps occur only at block boundaries.
- Tag parameter snapshots and projections with a monotonic graph revision so the renderer never consumes scalar state intended for another graph.
- Adapt HiDef SoundFont to parse its immutable bank once and create private prepared instruments for each Patch without exposing SoundFont-specific behavior to the rack.
- Split automatic fixture installation from playback start so accepted Patches and the initial graph are fully prepared before MIDI or physical audio begins.
- Add falsifiable acceptance for heterogeneous test instruments, exact Patch targeting and stem isolation, atomic preparation failures, graph swap acknowledgement, bounded return-queue pressure, and zero callback allocation or destruction.
- **BREAKING**: Replace the internal `SoundFontEngine` runtime port and transitional `basedrop` retirement path with the generic prepared-instrument and structural-graph ownership boundaries in the same change.
- Keep HiDef SoundFont as the only production capability and preparer. This change does not add Braids, C++/FFI, user engine selection, the PATCH page, modulation, layering, or per-Patch effects.

## Capabilities

### New Capabilities

- `prepared-engine-rack`: Fixed-capacity heterogeneous prepared-instrument ownership, complete off-callback graph preparation, block-boundary activation, acknowledgement, and off-callback retirement.

### Modified Capabilities

- `instrument-capability-model`: Separate descriptor/config providers from capability-matched runtime preparers and advance the Phase 2 boundary to the generic prepared rack while retaining one production capability.
- `realtime-execution`: Separate discrete commands, latest scalar snapshots, structural graph ownership, and observations; require complete graph swaps and bounded off-callback retirement.
- `soundfont-audio`: Replace the shared runtime engine abstraction with one shared parsed bank and independent per-Patch prepared SoundFont instruments behind the generic port.
- `automatic-test-midi`: Require Patch installation and initial graph preparation to complete before fixture emission begins.
- `one-way-parameter-control`: Correlate every parameter publication and projection with its target graph revision while preserving the reducer-first one-way path.
- `observable-demo-scene`: Expand production-derived serialized coverage and required gates to prove graph revision, prepared-rack routing, and structural handoff without weakening existing deterministic evidence.

## Impact

- Affects synth ports and the HiDef adapter, real-time value types and boundaries, `AudioRenderer`, mixer ownership, control projection, shell composition, automatic MIDI startup sequencing, schema/versioned observations, and behavioral tests.
- Adds a lock-free structural graph adapter and a named `prepared_engine_rack` integration target; retains the existing command ring, scalar triple buffer, and latest observation transport as separate mechanisms.
- Removes `basedrop` after the new return queue owns all replaced graph state; no new production engine dependency is introduced.
- Preserves `AppEvent` → `AppState::apply` → projections/audio, current SoundFont playback, global reverb and delay as audio effects, `make demo`, and optimized `make demo-live` behavior.
