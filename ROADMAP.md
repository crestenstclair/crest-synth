# Crest Synth Roadmap

This is an ordered, living roadmap. It is not an implementation specification or a replacement for `DESIGN.md`.

Work proceeds one small change at a time. Only the current phase should be translated into detailed CUE architecture and an active OpenSpec change. Later phases remain directional until the preceding phase is complete and reviewed.

## Working method

For each increment:

1. Review and narrow the increment with the user.
2. Update only the CUE architecture needed for that increment.
3. Create one OpenSpec proposal from the evaluated CUE context.
4. Implement and verify that proposal without pulling in later roadmap work.
5. Archive and checkpoint the completed change before starting the next one.

Modulation, dynamic effect-graph editing, and other later systems must not be introduced early.

## Phase 1 — Live observable demo

Status: **Complete** — 2026-07-20

Delivered and accepted through OpenSpec change `phase-1-live-observable-demo`. The production live run exercised all 67 current editable parameter instances across 15 Patches, emitted coherent control/audio evidence without dropped records, cleaned up to zero active notes, and preserved the final canonical view until manual close.

Keep the existing deterministic, exhaustive headless demo. Add a separate human-observable demo that runs through the real standalone application.

The live demo must:

- open the real Crest Synth UI;
- open the physical audio output and produce audible SoundFont playback;
- use the existing MIDI fixture;
- run a paced autonomous scene through the normal semantic event, reducer, projection, and audio-publication path;
- modify every currently editable parameter at least once, with enough dwell time for the change to be visible and audible;
- update the visible UI from canonical projected state throughout the scene;
- record accepted and rejected events using the existing event log;
- expose useful structured checkpoints showing the input, expected transition, accepted state generation, projected value, emitted effects, and audio observation;
- emit the final event log, state tree, coverage result, and human-readable summary;
- stop active notes when the scene finishes and leave the final UI state visible until the user closes the window.

The live path must not mutate UI, engine, or audio state directly. Autonomous actions enter through the same application event path as other inputs. Audio-thread observations use bounded counters or snapshots; the callback does not log.

Suggested entry point: `make demo-live`, backed by a dedicated interactive CLI option. The existing `make demo` headless proof remains unchanged.

### Phase 1 completion

- A user can launch one command, see the UI, hear the fixture, and watch the scene operate the synth.
- Every current editable parameter is visibly exercised through the production event path.
- The event records, state tree, parameter projection, and measured audio consequence agree at each declared checkpoint.
- Existing headless demo, schema, mutation, real-time, and project checks continue to pass.

## Phase 2 — Polymorphic engine foundation and Braids

Status: **Next**

Prove the engine-capability architecture with two concrete engines before building a user-facing Patch page. The existing SoundFont implementation is the first engine. **Braids** is the second engine and replaces the previously planned Plaits milestone.

Patch and application state must describe instruments generically:

```text
Patch
├── stable PatchId and MIDI mapping
├── InstrumentConfig { capability_id, values, asset references }
├── ordered PostFx slots
└── MixerRoute
```

Instrument implementations register capability descriptors containing stable IDs, labels, ordered sections, parameter specifications, asset requirements, preparation behavior, and a prepared real-time renderer factory. A parameter specification must also declare whether a change is a latest-value scalar update or a structural change that requires off-thread preparation.

The reducer, state projector, serialization, observation schema, and later UI must address capabilities and parameters by stable semantic IDs. They must not match on SoundFont or Braids to decide which fields exist. SoundFont-specific preset identity remains inside the SoundFont configuration and adapter rather than defining the shape of every Patch.

The real-time side owns a bounded prepared engine rack. Each Patch routes MIDI to exactly one prepared instrument and receives one distinct stereo stem. Polymorphic dispatch may occur once per Patch or render block, but never inside an engine's inner sample loop. Engine construction, asset loading, resampling preparation, graph replacement, and destruction remain off the callback. A failed or unavailable capability produces a typed visible error and never substitutes another engine.

### Braids adapter

Use and wrap the Mutable Instruments Braids C++ `MacroOscillator` implementation rather than reimplementing its synthesis algorithms. Pin the audited upstream source, preserve required copyright and license notices, compile only the required desktop-safe DSP subset, and keep C++ and FFI types behind the adapter.

The first Braids descriptor exposes:

- Model;
- Timbre;
- Color.

Pitch, note lifecycle, velocity, and voice assignment come from Crest Synth's canonical MIDI and Patch contracts. Braids is originally monophonic; the adapter must declare a bounded voice policy. Polyphony uses one fully prepared `MacroOscillator` instance per voice. If measured limits require monophony or a smaller voice count, expose that limit explicitly rather than silently dropping or rerouting notes.

The upstream oscillator's 96 kHz and 24-sample rendering assumptions must be handled inside the prepared adapter with bounded scratch and an explicit sample-rate policy. Unsupported device configurations fail clearly before rendering. The callback performs no allocation, locking, blocking, I/O, logging, panic, unwinding, or destruction across the FFI boundary.

Phase completion requires a mixed-engine scene containing at least one SoundFont Patch and one Braids Patch. Both must respond to targeted MIDI, render nonzero finite isolated stems, consume only their own capability parameters, and pass allocation, destruction, timing, routing, and controlled-negative proofs through the production reducer and render path.

Implement this milestone as separate small OpenSpec changes:

1. Introduce canonical capability descriptors, generic instrument configuration, parameter values, and update classifications; adapt the existing SoundFont path without changing its current behavior.
2. Introduce the bounded prepared engine rack and structural graph handoff needed to host different engines on different Patches and retire replaced state off-thread.
3. Build, wrap, and prove the Braids renderer behind the prepared-instrument boundary, including its sample-rate, block-size, voice, FFI, source-pin, and license constraints.
4. Prove simultaneous SoundFont and Braids Patches, schema-derived engine parameters, exact MIDI routing, parameter isolation, audible output, and hard real-time behavior.

This phase does not add the Patch page, per-Patch effects, modulation routing, a modulation matrix, arbitrary graph editing, or plugin hosting.

## Phase 3 — Schema-driven Patch page

Status: **Queued**

Preserve the current basic interface while introducing two directly selectable pages:

- `1` opens the existing view.
- `2` opens the Patch page.

Patch identity is the primary unit; MIDI channel is a Patch property rather than the thing being edited as if it were the instrument itself.

The Patch page includes:

- MIDI channel;
- engine;
- Attack;
- Decay;
- Sustain;
- Release;
- capability-provided engine-specific parameters.

The page renders the active `CapabilityDescriptor`; it does not contain SoundFont or Braids field lists. The engine selector contains the installed registry entries, initially SoundFont and Braids. Selecting an engine requests off-thread preparation. The existing active instrument remains audible while the request is pending, successful preparation commits through the semantic event and prepared-graph path, and failure leaves the active configuration and graph unchanged while showing the typed error.

The SoundFont descriptor exposes the locked `./sf2/HiDef.sf2` asset and the selected preset name. Holding Edit and pressing Left or Right cycles through valid discovered presets. The Braids descriptor exposes Model, Timbre, and Color.

ADSR is common semantic Patch configuration, but it must be applied per voice by every admitted engine. A post-stem envelope is not a conforming implementation because overlapping notes require independent note lifecycles. Do not expose an ADSR field for an engine until changing it produces a measured audible result through that engine's production render path.

Implement this milestone as separate small OpenSpec changes:

1. Add page selection and a Patch-page projection that renders installed capability descriptors and stable parameter IDs.
2. Add asynchronous engine selection through worker preparation, semantic completion events, prepared-graph publication, acknowledgement, and visible failure handling.
3. Add common per-voice ADSR behavior for SoundFont and Braids with audible overlapping-note proof.
4. Add SoundFont preset discovery, display by name, and Edit+Left/Right selection through the SoundFont descriptor.

This phase does not add modulation routing or a modulation matrix.

## Phase 4 — Static per-Patch effects

Status: **Queued**

Introduce the first per-Patch post-effects path using selected Mutable Instruments open-source effect implementations.

The first topology is fixed and prebuilt:

```text
Patch engine → ordered Patch effects → Patch mix/routing
```

Effects register the same kind of capability-owned parameter descriptors and preparation behavior as instruments. The Patch page renders only installed effect capabilities and their real fields; it does not expose placeholder slots or hard-code processor-specific controls.

Although the topology is static, it must use the same engine/effect ownership and preparation boundaries that a later configurable graph can extend. Do not introduce arbitrary routing, dynamic graph editing, modulation, or plugin hosting in this phase.

Choose the exact initial effects and confirm their source and license compatibility when this phase becomes current. Add effects one at a time so each processor has its own audible and real-time proof.

## Phase 5 — Figma-derived interface

Status: **Queued**

Use the existing Crest Synth Figma design and `DESIGN.md` as the visual and interaction references. Derive the additional CUE architecture and OpenSpec requirements needed to replace the basic text interface with the compact controller-first UI.

The target interface retains only the Crest Synth concepts established in the design:

- PATCH and MIXER top-level contexts;
- Patch strip and Patch identity;
- polymorphic instrument detail;
- polymorphic effect detail;
- visible ADSR editing;
- Utility/Inspector behavior;
- functional mixer faders;
- sparse semantic color and minimal paneling.

This phase begins only after the SoundFont, Braids, envelope, and static effect capabilities are stable enough for the UI to describe real installed functionality rather than placeholders.

## Explicitly deferred

- modulation sources and modulation routing;
- modulation matrix UI;
- dynamically configurable effect graphs;
- arbitrary effect reordering and routing;
- plugin hosting;
- additional engines beyond SoundFont and Braids;
- broad preset/session/library management.
