# Crest Synth Foundation Roadmap — Archived

Status: **Archived and complete** — 2026-07-27

This is the historical record of the four foundation phases delivered between 2026-07-20 and 2026-07-27. It is no longer a living plan and must not be extended with another numbered phase. It is not an implementation specification or a replacement for `DESIGN.md`.

The controller-first graphical interface is a separate successor program, not Phase 5. Its scope is larger than these foundation phases combined. `DESIGN.md` remains the single product and technical authority, its linked Figma file remains the visual and interaction reference, and future implementation work must be decomposed into bounded CUE and OpenSpec changes without creating a competing roadmap.

## Historical working method

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

Status: **Complete** — 2026-07-25

Delivered and accepted through the Phase 2 capability-model, prepared-rack, Braids-engine, polymorphic-audio, common-envelope, and control/demo OpenSpec changes, followed by `repair-production-runtime-contracts`. The production fixture and both demos alternate SoundFont and Braids Patches through the capability-neutral reducer, prepared rack, per-voice ADSR, mixer, and hard-real-time render path; all 19 declared project checks passed at the archive checkpoint.

Prove the engine-capability architecture with two concrete engines before building a user-facing Patch page. The existing SoundFont implementation is the first engine. **Braids** is the second engine and replaces the previously planned Plaits milestone.

Patch and application state must describe instruments generically:

```text
Patch
├── stable PatchId and MIDI mapping
├── InstrumentConfig { capability_id, values, asset references }
├── VoiceEnvelope { attack_ms, decay_ms, sustain, release_ms }
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

Pitch, note lifecycle, velocity, and voice assignment come from Crest Synth's canonical MIDI and Patch contracts. Every Braids Patch owns a distinct bank of exactly sixteen voices, using one fully prepared `MacroOscillator` instance and one independent envelope per voice. For `N` admitted active Braids Patches, capacity is `16 × N`; three Braids Patches own forty-eight voices. There is no Braids-specific Patch-count limit or global Braids voice budget—the only concurrent Patch bound is the engine-agnostic prepared rack capacity required by the hard-real-time contract. Note-on takes an idle slot or deterministically steals the oldest within only the targeted Patch, note-off releases matching slots there, and all-notes-off clears that Patch's bounded bank.

SoundFont uses a different voice policy: each SoundFont Patch owns one synthesizer instance with engine-managed polyphony under a finite prepared real-time safety ceiling. Crest does not impose Braids' sixteen-voice limit on SoundFont and does not create one synthesizer per note. Its common envelope must reach independent native voices through the SoundFont backend; a nonconforming backend must be extended or replaced rather than hidden behind a post-stem envelope.

Attack, Decay, Sustain, and Release are canonical Patch state rather than Braids fields. Both production engines apply that state per note; a post-stem envelope does not conform. The live scalar surface is derived generically as Patch mixer values, common ADSR values, then descriptor-classified engine values. SoundFont's preset and asset fields remain structural and visible but locked in this phase.

The upstream oscillator's 96 kHz and 24-sample rendering assumptions must be handled inside the prepared adapter with bounded scratch and an explicit sample-rate policy. Unsupported device configurations fail clearly before rendering. The callback performs no allocation, locking, blocking, I/O, logging, panic, unwinding, or destruction across the FFI boundary.

Phase completion requires normal, smoke, headless-demo, and live-demo composition to alternate fixture Patches between SoundFont and Braids. Both engines must respond to targeted MIDI, render nonzero finite isolated stems, consume only their own capability parameters, and pass allocation, destruction, timing, routing, overlapping-envelope, deterministic-stealing, and controlled-negative proofs through the production reducer and render path. The headless and live scenes must modify every editable mixer, ADSR, Braids, and global parameter instance using the same schema-derived resolver.

Implement this milestone as separate small OpenSpec changes:

1. Introduce canonical capability descriptors, generic instrument configuration, parameter values, and update classifications; adapt the existing SoundFont path without changing its current behavior.
2. Introduce the bounded prepared engine rack and structural graph handoff needed to host different engines on different Patches and retire replaced state off-thread.
3. Add the canonical per-note ADSR and fixed descriptor-ordered scalar projection, including an engine-native envelope seam for the one synthesizer owned by each SoundFont Patch.
4. Build, wrap, and prove one sixteen-voice Braids bank per Braids Patch behind the prepared-instrument boundary, including its scaling, sample-rate, block-size, FFI, source-pin, and license constraints.
5. Prove alternating SoundFont and Braids Patches, schema-derived engine and envelope parameters, exact MIDI routing, parameter isolation, audible output, and hard real-time behavior in both demos.

This phase does not add the Patch page, per-Patch effects, modulation routing, a modulation matrix, arbitrary graph editing, or plugin hosting.

## Phase 3 — Schema-driven Patch page

Status: **Complete** — 2026-07-26

Completed increments:

1. Page selection and the read-only, descriptor-driven Patch-page projection, accepted through OpenSpec change `phase-3-patch-page-projection`.
2. Asynchronous focused-Patch engine selection, accepted on 2026-07-26 through OpenSpec change `phase-3-asynchronous-engine-selection`. The production path now performs descriptor-default SoundFont ↔ Braids replacement through one correlated capacity-one worker and complete graph handoff while the source remains audible, publishes no fallback, retires graphs off callback, and proves both directions in deterministic and physical live witnesses.
3. Canonical PATCH ADSR focus and editing, completed on 2026-07-26 through OpenSpec change `phase-3-patch-adsr-editing`. PATCH now owns the nonwrapping Engine → Attack → Decay → Sustain → Release focus surface, reuses the existing per-voice envelope mutation and scalar snapshot path, remains coherent through engine preparation and activation, and proves every field through both engines plus the physical live demo.
4. SoundFont preset discovery and semantic selection, completed on 2026-07-26 through OpenSpec change `phase-3-soundfont-preset-selection`. The fixed SF2 is parsed once into an authored-name, numerically ordered control catalog plus a metadata-free numeric render bank; PATCH derives Preset after Release from the descriptor and sends adjacent choices through the shared structural worker/graph lifecycle. Release-mode real-SF2, deterministic two-run, and physical live witnesses prove exact names/order, source-preserving failure and preparation, audible activation, no fallback, and zero callback allocation or destruction.

Phase 3 has no remaining planned increment. Its schema-driven control and projection foundation is the base used by Phase 4's static per-Patch effect controls.

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

ADSR is already common semantic Patch configuration and is already applied per voice by SoundFont and Braids before this page begins. The Patch page renders those canonical fields; it must not create a second UI-owned envelope model.

Implement this milestone as separate small OpenSpec changes:

1. Add page selection and a Patch-page projection that renders installed capability descriptors and stable parameter IDs.
2. Add asynchronous engine selection through worker preparation, semantic completion events, prepared-graph publication, acknowledgement, and visible failure handling.
3. Render and edit the existing common per-voice ADSR values without duplicating their state or DSP behavior.
4. Add SoundFont preset discovery, display by name, and Edit+Left/Right selection through the SoundFont descriptor.

This phase does not add modulation routing or a modulation matrix.

## Phase 4 — Static per-Patch effects

Status: **Complete** — 2026-07-27

Delivered and accepted through OpenSpec change `phase-4-first-static-patch-effect`, archived as `2026-07-27-phase-4-first-static-patch-effect`. The production fixture routes its first Patch through one prepared Chorus, derives Amount and Depth from the effect descriptor in the existing PATCH UI, preserves the effect configuration across structural engine changes, and proves post-engine/pre-mixer ordering, Patch isolation, independent instance state, audible stereo output, and hard-real-time callback safety in the accepted deterministic and physical live witnesses.

Introduce the first per-Patch post-effects path with one MIT-licensed Mutable Instruments Rings Chorus, pinned to the audited eurorack/stmlib revisions recorded in `DESIGN.md` and exposed to the product as `Chorus`.

The first topology is fixed and prebuilt:

```text
Patch engine → ordered Patch effects → Patch mix/routing
```

Effects use a separate capability descriptor/registry/provider/preparer family while sharing canonical parameter types with instruments. The first production fixture configures Chorus only on its first Patch and exposes descriptor-derived Amount and Depth rows. The Patch page renders only configured effect capabilities and their real fields; it does not expose placeholder slots or hard-code processor-specific controls.

Although the topology is static, it must use the same engine/effect ownership and preparation boundaries that a later configurable graph can extend. Do not introduce arbitrary routing, dynamic graph editing, modulation, or plugin hosting in this phase.

The initial adapter admits exactly 48 kHz, owns independent prepared delay/LFO state per instance, and must prove source/license hashes, order, Patch isolation, independent instances, audible stereo output, structural preservation, and callback safety through its named acceptance target plus both demos. Add later effects one at a time with their own source review and proof.

Phase 4 completes this foundation roadmap.

## Successor program — controller-first graphical interface

This work is deliberately outside the archived phase sequence. It replaces the basic text adapter with the product interface defined by `DESIGN.md` and the linked Figma file, while preserving the proven reducer, schema-derived capability controls, audio projections, and real-time boundaries.

The successor program includes the complete, integrated behavior of:

- PATCH and MIXER top-level contexts;
- Patch strip and Patch identity;
- polymorphic instrument detail;
- polymorphic effect detail;
- visible ADSR editing;
- Utility/Inspector behavior;
- functional mixer faders;
- sparse semantic color and minimal paneling.

Those are product requirements, not a pre-sequenced implementation checklist. The program must first be explored against the complete Figma composition and current executable architecture, then divided into reviewable OpenSpec changes with their own behavioral and visual acceptance. No sequence or estimate is declared by this archived roadmap.

## Other work outside this archived roadmap

- modulation sources and modulation routing;
- modulation matrix UI;
- dynamically configurable effect graphs;
- arbitrary effect reordering and routing;
- plugin hosting;
- additional engines beyond SoundFont and Braids;
- broad preset/session/library management.
