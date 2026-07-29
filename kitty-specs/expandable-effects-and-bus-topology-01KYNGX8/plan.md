# Implementation Plan: Expandable Effects and Bus Topology

**Branch**: `feat/expandable-effects-and-bus-topology` | **Date**: 2026-07-28 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/spec.md`

## Summary

Replace name-enumerated effect and routing concepts with descriptor-driven,
index-addressed generic ones, then use that generality to deliver three ordered
effect slots per Patch and eight bus returns.

The governing decision of this plan is **open-closed by construction**. Every
place the current code names a specific effect — `MixerTrackParameter::ReverbSend`,
`GlobalEffectsProcessor::process(reverb_input, delay_input, ...)`,
`GlobalParameter::ReverbRoomSize`, one `effect` field per Patch — is replaced by a
fixed-capacity array of a generic type addressed by index. Adding the twelve
planned roster effects afterwards must require zero changes to slot, routing,
snapshot, preparation, projection, or render structure. That is `SC-008`, and it
is the acceptance test for this plan's design, not merely for its code.

## Technical Context

**Language/Version**: Rust 2021 edition, toolchain 1.96.0
**Primary Dependencies**: cpal 0.18 (audio device), eframe/egui 0.32 + egui_extras (shell), rustysynth 1.3 (SoundFont), midly 0.5 (fixture MIDI), rtrb 0.3 (event ring), triple_buffer 9.0 (latest-value snapshot transport), serde/serde_json, thiserror/anyhow; pinned Mutable Instruments C++ DSP via `cc` build script
**Storage**: None. No preset or session persistence exists or is added; serialized state is an in-process diagnostic projection only
**Testing**: `cargo test --all-targets`, plus `cargo-nextest` JUnit output in the pre-review gate; deterministic scene proofs, a behavioral mutation harness, real-time contract validations, and a physical-device live demo scene
**Target Platform**: macOS and Linux desktop (authored desktop viewport) and Steam Deck viewport class
**Project Type**: Single Rust binary + library, hexagonal, seven bounded contexts under `src/`
**Performance Goals**: Bounded stereo render at 48 kHz with zero underruns across the live scene; topology activation at a block boundary with no partially applied state observable
**Constraints**: Hard real-time callback — no allocation, locking, blocking, I/O, logging, panic, or destruction; fixed preallocated capacity for 16 patches x 3 slots x 8 scalars, 16 tracks x 8 sends, and 8 returns x 8 scalars; product ceiling of three slots and eight returns must not be exceeded
**Scale/Scope**: ~25 source files touched across Synth, Mixer, RealTime, Control, Shell, and Testing contexts; ~330 occurrences of the four retired name-enumerated concepts; one new retained live demo scene

## Charter Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Skipped — no charter exists.** `spec-kitty charter context --action plan` returns
`mode: missing`; `.kittify/charter/charter.md` is absent. Built-in directives still
apply and the ones that bind this plan are:

| Directive | Application here |
|---|---|
| DIRECTIVE_001 Architectural Integrity | The generic-array redesign is the whole point; boundaries between Synth (effects), Mixer (routing), and RealTime (bounded transport) must not blur under it |
| DIRECTIVE_003 Decision Documentation | Four plan decisions recorded via decision moments; the open-closed rationale is captured in `research.md` |
| DIRECTIVE_010 Specification Fidelity | C-009 forbids deferring architecture-spec edits past planning |
| DIRECTIVE_024 Locality of Change | Tension acknowledged: this change is deliberately non-local. The blast radius is justified in Complexity Tracking below |
| DIRECTIVE_025 Boy Scout Rule | Touched files with enumerated-effect coupling get generalized rather than extended |

Re-checked after Phase 1: no new violations. Complexity Tracking records the one
standing tension.

## Architecture Reconciliation

`CLAUDE.md` requires this section, and here it is load-bearing rather than
ceremonial: the architecture spec currently declares this mission a **non-goal**.
Per the spec's own rule — *"Never silently plan around a conflict with the
architecture spec. If the requested change changes architecture, update the listed
source files during planning and reload them"* — the edits below are part of
planning, not a downstream task. C-009 restates this as a mission constraint.

### Declarations to narrow

| Source | Current declaration | Required change |
|---|---|---|
| `.kittify/architecture/project.yaml:17-19` | `nonGoals.additional_effects` — "does not provide another insert type, more than one slot per Patch, bypass, selection, reordering, effect chains" | Narrow to: the roster of additional effect *types* remains out of scope; slots, selection, and ordering are now in scope up to the declared ceiling |
| `.kittify/architecture/project.yaml:32-35` | `nonGoals.later_roadmap_phases` — excludes "Phase 3 expansion beyond at most three effect slots per Patch and eight bus returns" | Remove the Phase 3 clause; keep Phases 4-9 excluded |
| `.kittify/architecture/project.yaml:47-52` | `meta.avoid` — "effect selection, bypass, multiple slots, reordering, or arbitrary routing"; "arbitrary buses" | Replace with the sharper rule this mission actually needs: avoid *name-enumerated* effect and routing identities; buses remain bounded and validated, never arbitrary |
| `.kittify/architecture/project.yaml:2-10` | `mission` — describes "one configured first-Patch Chorus" and "one fixed sixteen-track mixer" | Restate the executable slice to include ordered slots and bounded bus returns |
| `.kittify/architecture/project.yaml:104-117` | `completion.requiredGoals` | Add the new goal |
| `.kittify/architecture/project.yaml:118-145` | `completion.projectChecks` | Add the new project check |
| `DESIGN.md:689` | "Chorus is the first concrete Patch effect" | Reverb and delay join the same registry as peers |
| `DESIGN.md:691` | "statically bounded to zero or one post effect per Patch… no selector, bypass, reorder, placeholder, or dynamic graph editing yet" | Superseded; restate at three ordered slots with selection |
| `DESIGN.md:309` | "at most eight values for the current zero-or-one effect slot"; "MIXER contains only the sixteen track-owned controls and distinct globals" | Restate for three slots and for sends addressed by bus |
| `DESIGN.md:418` | "current production bound is zero or one effect per Patch" | Restate at the new bound; the sentence already anticipates this ("Phase 3 may expand this only up to the product maximum") |

`DESIGN.md:690` and `:418` already authorize three slots and eight returns as the
product maximum, so this raises the executable slice to a ceiling the master design
sanctions. It is not a product redefinition, and the Figma fixtures are unaffected.

### Canonical resources

**Retired**

- `port.Mixer.GlobalEffectsProcessor` — a two-input signature that cannot express N returns
- `adapter.GlobalReverbDelay` as a port implementation — its DSP survives, its role changes

**Added**

- `valueObject.Mixer.BusId`, `valueObject.Mixer.BusSendLevel`, `aggregate.Mixer.BusReturn`
- `valueObject.Synth.EffectSlotIndex` (peer of the existing `EffectSlotId`)
- `adapter.ReverbCapability`, `adapter.ReverbPreparer`, `adapter.DelayCapability`, `adapter.DelayPreparer` — peers of `adapter.ChorusCapability` / `adapter.ChorusPreparer`, implementing `port.Synth.EffectCapabilityProvider` and `port.Synth.EffectPreparer`
- `capability.expandable_effects_and_bus_topology` with acceptance scenarios drawn from spec User Stories 1-3
- `goal.expand_effects_and_buses` added to `completion.requiredGoals`
- `evidence.expandable_effects_and_bus_topology_contract`, a matching validation, and a witness with positive and controlled-negative commands
- Project check `expandable_effects_and_bus_topology`

**Modified**

- `context.Synth` — the effect registry serves both slot and return roles; ubiquitous language gains slot index and return
- `context.Mixer` — invariants gain bounded bus identity, indexed sends, and post-gate send position
- `context.RealTime` — snapshot capacity invariants restated for the widened fixed layout
- `context.Control` — focus, valid actions, status, and errors extend to slot and return rows
- `context.Testing` — the new retained scene and its measurements

### New architecture invariant

The user's objection is that the closed design should never have shipped, given
the expansion was declared from the start. Prose will not prevent a recurrence, so
this plan adds a **proof-enforced invariant** rather than a note:

> **No name-enumerated effect or routing identity.** No type in Synth, Mixer,
> RealTime, or Control may enumerate a variant, field, or descriptor entry named
> after a specific effect or bus. Effects, slots, sends, and returns are addressed
> by index into descriptor-driven arrays. Adding a registry entry must require no
> change to any of these types.

Its validation is a static check over the source tree that fails on effect-specific
identifiers in the named contexts, seeded with the concrete strings this mission
retires. It runs as a project check, so a future closed shortcut fails the build
rather than the review.

### Bulk Edit Classification

`change_mode: bulk_edit` is set; `occurrence_map.yaml` is present and schema-valid.
Four structural renames, ~330 occurrences, ~25 files:

| Retired name | Replacement | Approx. occurrences |
|---|---|---|
| `MixerTrackParameter::ReverbSend` / `::DelaySend` and their fields | indexed `sends` array addressed by `BusId` | 93 across 14 files |
| `GlobalParameter` reverb and delay fields | descriptor scalars of registry entries + per-return level | 215 across 13 files |
| `GlobalEffectsProcessor` | bounded bus return rack over the generic prepared-effect boundary | 18 across 6 files |
| `GlobalReverbDelay` | reverb and delay capability/preparer adapter pairs | 22 across 5 files |

One category departs from its default: `serialized_keys` is `rename`, not
`do_not_change`. Justification is recorded in the map — there is no persistence and
no external consumer, so these camelCase names exist only in the diagnostic
projection and the snapshot leaf descriptor. Keeping `reverbSend` on a generic model
would reintroduce the coupling the mission exists to remove. `cli_commands` and
`logs_telemetry` stay `do_not_change` so every earlier retained phase scene remains
runnable and comparable.

## Project Structure

### Documentation (this mission)

```
kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/
├── plan.md               # This file
├── spec.md               # Mission specification
├── research.md           # Phase 0 output
├── data-model.md         # Phase 1 output
├── quickstart.md         # Phase 1 output
├── occurrence_map.yaml   # Bulk-edit classification (required before implement)
├── contracts/            # Phase 1 output
│   ├── effect-registry.md
│   ├── bus-routing.md
│   └── realtime-snapshot.md
├── checklists/
│   └── requirements.md
└── tasks/                # /spec-kitty.tasks output — NOT created here
```

### Source Code (repository root)

```
src/
├── kernel/                        # shared ids
├── synth/                         # effect registry, descriptors, slots
│   ├── patch.rs                   # post_effects -> bounded ordered slots
│   ├── effect_capability.rs       # registry serving slot AND return roles
│   └── prepared_engine_rack_builder.rs
├── mixer/                         # routing
│   ├── bus_return.rs              # NEW - replaces global_effects_processor.rs
│   ├── bus_id.rs                  # NEW - bounded bus identity
│   ├── mixer_track_parameters.rs  # indexed sends replace named variants
│   ├── global_parameters.rs       # reduced to master gain
│   └── mix_engine.rs              # indexed send accumulation, return summing
├── real_time/                     # bounded transports
│   ├── parameter_snapshot.rs      # widened fixed layout
│   ├── prepared_post_effect_rack.rs  # 1 slot/Patch -> 3
│   ├── prepared_bus_return_rack.rs   # NEW
│   ├── prepared_graph.rs / prepared_graph_builder.rs
│   └── structural_graph_coordinator.rs
├── control/                       # reducer, focus, projections
│   ├── state_tree.rs / serialized_state.rs / state_projector.rs
│   ├── semantic_focus.rs          # slot and return rows
│   └── patch_page_projection.rs
├── adapter/                       # capability + preparer implementations
│   ├── reverb_capability.rs       # NEW ─┐ from global_reverb_delay.rs
│   ├── reverb_preparer.rs         # NEW  │
│   ├── delay_capability.rs        # NEW  │
│   └── delay_preparer.rs          # NEW ─┘
├── shell/                         # egui rendering only
└── testing/                       # scenes, harness, measurements
    └── live_effects_and_buses_scene.rs   # NEW retained scene

tests/                             # integration + contract proofs
```

**Structure Decision**: The existing seven-context hexagonal layout is kept exactly.
No context boundary moves. New concepts land in the context that already owns their
concern — bus identity and returns in `src/mixer/`, effect slots and the registry in
`src/synth/`, bounded transport in `src/real_time/`. The two structural moves are
declared in `occurrence_map.yaml`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Non-local change spanning ~25 files across 6 contexts, in tension with DIRECTIVE_024 Locality of Change | The closed enumerations are themselves spread across those files. Generalizing one axis while leaving the others enumerated would leave the model incoherent — a generic send array feeding a two-input processor, or three slots whose parameters cannot be projected | Extending in place (adding `Send3..Send8` variants, widening `process()` to N inputs) was explicitly rejected by the user as perpetuating the original design fault. It would satisfy the FRs while leaving the next roster addition equally expensive, failing SC-008 |
| A new static project check constraining permissible identifiers | Prose invariants did not prevent the original closed design; the expansion was declared in DESIGN.md from the start and the code closed against it anyway | A review-time convention was rejected: it is exactly what failed here. The check makes regression a build failure |

## Implementation Concern Map

> Implementation concerns are NOT work packages. `/spec-kitty.tasks` translates
> these into executable WPs — one concern may become several WPs, and small
> concerns may merge.

### IC-01 — Generic effect registry serving both roles

- **Purpose**: Make one descriptor-driven effect registry the single source of effect identity, schema, and preparation, usable identically by a Patch slot and a bus return.
- **Relevant requirements**: FR-003, FR-009, FR-010, SC-008
- **Affected surfaces**: `src/synth/effect_capability.rs`, `src/synth/patch.rs`, `src/adapter/{reverb,delay}_{capability,preparer}.rs`, retired `src/adapter/global_reverb_delay.rs`
- **Sequencing/depends-on**: none — this is the foundation every other concern consumes
- **Risks**: Reverb and delay DSP must move behind the generic prepared-effect boundary without changing its sound; the existing `EffectError` preparation vocabulary must absorb their storage needs

### IC-02 — Ordered effect slots on the Patch

- **Purpose**: Replace one optional effect per Patch with three ordered, independently occupied slots carrying their own values and instance state.
- **Relevant requirements**: FR-001, FR-004, FR-005, C-001
- **Affected surfaces**: `src/synth/patch.rs`, `src/real_time/prepared_post_effect_rack.rs`, `src/real_time/prepared_graph_builder.rs`
- **Sequencing/depends-on**: IC-01
- **Risks**: `PreparedPostEffectRack::matches_parameters` currently proves a one-to-one Patch/slot correspondence; the widened proof must stay exact rather than becoming permissive

### IC-03 — Bounded bus identity, indexed sends, and returns

- **Purpose**: Introduce validated bus identities, replace the two named send fields with an indexed send array, and add the bounded return rack that retires the global effects port.
- **Relevant requirements**: FR-006, FR-007, FR-008, FR-011, C-002, C-005, C-006
- **Affected surfaces**: `src/mixer/bus_id.rs`, `src/mixer/bus_return.rs`, `src/mixer/mixer_track_parameters.rs`, `src/mixer/global_parameters.rs`, `src/mixer/mix_engine.rs`, `src/real_time/prepared_bus_return_rack.rs`
- **Sequencing/depends-on**: IC-01
- **Risks**: The post-fader/post-gate send position at `mix_engine.rs:164-167` and the "mute always wins / solo excludes" rule must survive generalization byte-for-byte in behavior; returns must not create a routing cycle

### IC-04 — Widened bounded real-time transport

- **Purpose**: Grow the single fixed latest-value snapshot to carry 16x3 slots, 16x8 sends, and 8 returns while preserving fixed layout, exact matching, and callback safety.
- **Relevant requirements**: FR-012, NFR-001, NFR-002, NFR-003
- **Affected surfaces**: `src/real_time/parameter_snapshot.rs`, `src/real_time/prepared_graph.rs`, `src/real_time/audio_renderer.rs`
- **Sequencing/depends-on**: IC-02, IC-03
- **Risks**: The snapshot roughly triples; the triple-buffer publish cost and the `SERIALIZED_LEAF_DESCRIPTOR` must both be re-proved. This is the highest real-time risk in the mission

### IC-05 — Topology change lifecycle, validation, and rejection

- **Purpose**: Route slot and return occupancy changes through the existing correlated structural-edit lifecycle, with validation, visible outcome, controlled rejection, recovery, and off-callback retirement.
- **Relevant requirements**: FR-002, FR-012, FR-013, FR-014, FR-015, FR-016, FR-018
- **Affected surfaces**: `src/real_time/structural_graph_coordinator.rs`, `src/real_time/graph_preparation_worker.rs`, `src/control/state_tree.rs`, `src/control/state_projector.rs`
- **Sequencing/depends-on**: IC-02, IC-03
- **Risks**: Two changes requested before the first is acknowledged must not reorder or drop acknowledgements; a refused change must leave the active graph untouched

### IC-06 — Semantic focus and projection for slots and returns

- **Purpose**: Extend the reducer-owned focus order and projections to slot and destination rows using the existing adjacent-choice contract, with deterministic focus recovery when rows appear or disappear.
- **Relevant requirements**: FR-002, FR-003, FR-014, FR-017, C-003, C-008
- **Affected surfaces**: `src/control/semantic_focus.rs`, `src/control/patch_page_projection.rs`, `src/control/text_projection.rs`, `src/shell/app_window.rs`
- **Sequencing/depends-on**: IC-05
- **Risks**: Focus must resolve deterministically when a slot is cleared while its parameters hold focus; PATCH and MIXER must remain the only top-level contexts

### IC-07 — Architecture spec and DESIGN.md reconciliation

- **Purpose**: Narrow the three non-goal declarations, restate the superseded DESIGN.md decisions, and add the new capability, goal, requirements, evidence, validation, witness, and the no-name-enumeration invariant.
- **Relevant requirements**: C-009, DIRECTIVE_010
- **Affected surfaces**: `.kittify/architecture/project.yaml`, `capabilities.yaml`, `goals.yaml`, `requirements.yaml`, `contexts/{synth,mixer,realtime,control,testing}.yaml`, `proof/{evidence,validations,witnesses,invariants}.yaml`, `adapters.yaml`, `assets.yaml`, `DESIGN.md`
- **Sequencing/depends-on**: none — must land before or alongside the first implementation WP, never after
- **Risks**: Narrowing a non-goal too far would silently pull the deferred roster into scope; C-011 must survive the edit

### IC-08 — Retained live scene and measured proof

- **Purpose**: Deliver `make demo-live-effects-and-buses` and the deterministic, mutation, and real-time proofs that make every declared behavior falsifiable.
- **Relevant requirements**: FR-019, C-010, NFR-004, NFR-005, NFR-006, NFR-007, SC-001 through SC-007
- **Affected surfaces**: `src/testing/live_effects_and_buses_scene.rs`, `src/testing/behavioral_mutation_harness.rs`, `src/testing/live_mixer_routing_measurement.rs`, `Makefile`, `src/bin/crest_synth.rs`, `tests/`
- **Sequencing/depends-on**: IC-04, IC-05, IC-06
- **Risks**: Existing phase scenes and their checkpoint identities must remain runnable and comparable (`logs_telemetry: do_not_change`); the scene must prove order-sensitivity audibly, not merely structurally
