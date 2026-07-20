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

## Phase 2 — Basic Patch page

Status: **Next**

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
- engine-specific parameters.

For this phase, SoundFont is the only installed engine. The engine field exists but has one valid choice. The SoundFont file field is visible and locked to `./sf2/HiDef.sf2`. The SoundFont instrument field displays the preset name; holding Edit and pressing Left or Right cycles through valid instruments.

ADSR is common Patch behavior and must change audible playback. A value changing only in the UI or serialized state does not complete the increment.

Implement this milestone as separate small OpenSpec changes:

1. Page selection and Patch-page projection.
2. Editable Patch identity/configuration and working ADSR.
3. SoundFont preset discovery, display by name, and Edit+Left/Right selection.

## Phase 3 — Plaits engine

Status: **Queued**

Add a second synthesis engine named **Plaits**.

Use and wrap the Mutable Instruments Plaits C++ implementation rather than reimplementing its synthesis algorithms. The wrapper must satisfy Crest Synth's engine capability and hard real-time contracts, including bounded preparation, rendering, event handling, and destruction.

Implement this milestone in two increments:

1. Build, wrap, and prove the Plaits renderer behind the engine boundary.
2. Expose Plaits through the Patch page with its engine-specific parameter schema and built-in envelope behavior.

This phase does not add modulation routing or a modulation matrix.

## Phase 4 — Static per-Patch effects

Status: **Queued**

Introduce the first per-Patch post-effects path using selected Mutable Instruments open-source effect implementations.

The first topology is fixed and prebuilt:

```text
Patch engine → ordered Patch effects → Patch mix/routing
```

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

This phase begins only after the SoundFont, Plaits, envelope, and static effect capabilities are stable enough for the UI to describe real installed functionality rather than placeholders.

## Explicitly deferred

- modulation sources and modulation routing;
- modulation matrix UI;
- dynamically configurable effect graphs;
- arbitrary effect reordering and routing;
- plugin hosting;
- additional engines beyond SoundFont and Plaits;
- broad preset/session/library management.
