# Implementation Plan: Functional Patch Editor Blockout

**Branch**: `feat/functional-patch-editor` | **Date**: 2026-08-07 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `kitty-specs/functional-patch-editor-01KZERV9/spec.md`

## Summary

Make every installed Patch reachable and editable from the controller, and prove
it on hardware. The reducer already has the patch-selection gesture; nothing on
screen has ever shown it, and the PATCH surface is a flat descriptor-ordered row
list with four designed structures marked unavailable for want of data.

The approach, in dependency order: give the domain the missing canonical values
(a per-Patch voice limit, a PATCH-side path to master gain, an editable MIDI
channel), widen the PATCH control surface and the projection to carry them plus
per-row intent and a subordinate detail surface, recompose the webview page from
a flat row run into the authored grouped strip, then close with one deterministic
acceptance target and one live scene whose controlled negative defeats the patch
switch.

## Technical Context

**Language/Version**: Rust 2021 edition (workspace binary `crest-synth`), plus the
webview projection page as vanilla HTML/CSS/JS under `webview-page/`
**Primary Dependencies**: tauri (webview shell), cpal (physical audio), rustysynth
(SoundFont), rtrb + triple_buffer (RT transports), serde/serde_json (projection
transport), midly (fixture parsing). No new dependency is introduced.
**Storage**: N/A — canonical state is in-process; no persistence layer is touched.
**Testing**: `cargo test --all-targets` plus the declared crest-spec project
validations; this mission adds `tests/functional_patch_editor.rs` and the
`make demo-live-patch-editor` witness with a controlled negative.
**Target Platform**: macOS desktop (development rig), 1920×1080 and 1280×800
authored viewports.
**Project Type**: single Rust project with an embedded webview asset directory.
**Performance Goals**: the audio callback keeps its hard-real-time contract —
zero allocations, zero destructions, no locking, blocking, I/O, or logging — with
voice limiting active. Projection throughput holds the existing bar: monotonic
generations, no generation gap, > 0 qualifying webview frames.
**Constraints**: every edit travels physical input → semantic action/event →
`AppState::apply` → projection. No UI-local state, no second value owner, no
placeholder value in the production shell. The gated live suite requires a
display seating 1920×1080 and refuses rather than degrading.
**Scale/Scope**: the fifteen-Patch production fixture (8 SoundFont, 7 Braids);
three ordered effect slots per Patch; five PATCH Utility rows; one subordinate
detail surface serving two subject kinds.

## Engineering Alignment

Confirmed at spec sign-off (2026-08-07):

- **Voice limit is in scope, refuse-not-steal.** A note-on arriving while the
  Patch already sounds its limit is not started; no latched voice is truncated.
  This is the falsifiable policy — a defeated limit shows as a note that
  sounded. It is also the first production producer of the `Stepped` control
  kind, which the component vocabulary declared and nothing drove.
- **Detail shell now, detail content in Phase 7.** This mission adds the
  `PatchDetail` surface identity, entry from a resolving row, capability-supplied
  sections, and exact-return focus. Sample detail, waveform landmarks, and the
  trapped-focus choice modals stay in Phase 7.
- **MIXER-side carry-forwards stay deferred.** The meter, `MixerControlId::Track`
  inspector reachability, the M/S labels, the multi-select help block, and the
  MIXER Inspector's unbounded row set belong to the phase that owns that surface.

## Charter Check

*GATE: passed before Phase 0; re-checked after design.*

Charter present at `.kittify/charter/charter.md`; context loaded in `compact`
mode. The directives that bind this mission and how the design satisfies them:

| Directive | Bearing on this mission | Status |
|---|---|---|
| DIRECTIVE_001 Architectural Integrity | The mission widens existing boundaries rather than crossing them: Control owns focus and projection, Synth owns the Patch value, RealTime owns enforcement, the page owns paint. `PatchDetail` is one surface identity, not one per subject. | Pass |
| DIRECTIVE_003 Decision Documentation | The voice-limit policy, the five-row Utility bound, the single master-gain owner, and the Phase 5/7 detail split are recorded in `DESIGN.md` durable decisions and declared in the crest-spec. | Pass |
| DIRECTIVE_010 Specification Fidelity | The crest-spec was authored before this plan; every implementation concern below cites the canonical resources it realizes. | Pass |
| DIRECTIVE_024 Locality of Change | Each concern is scoped to one context's modules; the page concerns touch `webview-page/` and the domain concerns do not. | Pass |
| DIRECTIVE_025 Boy Scout Rule | One LOW follow-up (`retire()`'s unproven de-duplication guard) is folded in because this mission drives far more churn through that channel. Four others are explicitly left filed with reasons, in `spec.md`. | Pass |
| DIRECTIVE_035 Bulk Edit | Not a bulk edit — this mission adds new identifiers rather than renaming existing ones across files. No `occurrence_map.yaml`. | N/A |

No violations to justify; Complexity Tracking is omitted.

## Crest-Spec Derivation

The crest-spec was authored at `c6e8290`, before this plan. `spec-kitty crest-spec
doctor` reports the model closed (7 contexts / 135 resources, 15 goals, 21
capabilities, 115 requirements, 33 project validations, 20 witnesses).

**Resources added**

| Canonical ID | Why |
|---|---|
| `valueObject.Control.PatchDetailSubject` | Names which capability fills the polymorphic detail surface; derived from the originating focus, never chosen by a host |
| `valueObject.Synth.VoiceLimit` | The canonical per-Patch ceiling, its descriptor, and its refuse-not-steal enforcement contract |
| `valueObject.Testing.FunctionalPatchEditorObservation` | The structured physical acceptance result for the live scene |

**Resources changed**

| Canonical ID | Change |
|---|---|
| `valueObject.Control.SurfaceId` | Gains `PatchDetail`; declared subordinate rather than persistent |
| `valueObject.Control.PatchControlId` | Gains `Global(GlobalParameter)`, `MidiInput`, `VoiceLimit`; PatchUtility's resolver declared as exactly five ordered rows |
| `valueObject.Control.ReturnPath` | `enteredSurface` gains `PatchDetail`; subordinate surfaces declared non-nesting |
| `valueObject.Control.SemanticControlViewModel` | Gains `requestedValue` and per-row `validActions`; authored-label-not-serialization-key invariant |
| `valueObject.Control.SemanticSurfaceViewModel` | Role gains `Detail`; detail content declared descriptor-resolved |
| `aggregate.Control.InteractionState` | Gains `detailSubject`; detail entry/exit and cross-switch behavior declared |
| `aggregate.Synth.Patch` | Gains `voiceLimit`; `channel` declared editable through the reducer |
| `valueObject.RealTime.ParameterSnapshot` | `RtPatchParameters` carries `VoiceLimit` |
| `valueObject.RealTime.AudioObservationSnapshot` | Gains `voiceLimitRefusals` |
| `service.RealTime.AudioRenderer` | Note-on refusal rule, refusal counting, note-off exemption |
| `valueObject.Shell.ShellComposition` | Gains `PatchStrip` and `CapabilityDetailShell`; Utility seating rule |
| `valueObject.Testing.LiveDemoScene` | The patch-editor plan is not pinned to `patches.first()`; declares the second-instrument journey and its controlled negative |

**Resources retired**: none.

**Intent added**: `goal.edit_every_installed_patch`;
`capability.functional_patch_editor` with three acceptance journeys
(`reach_the_second_instrument`, `drive_the_utility_surface`,
`one_detail_shell_two_subjects`); requirements `composed_patch_strip`,
`on_screen_patch_selection`, `bounded_patch_utility_surface`,
`canonical_voice_limit`, `projected_row_intent`,
`polymorphic_capability_detail_shell`, `authored_control_presentation`,
`functional_patch_editor_behavioral_proof`. Both are added to
`completion.requiredGoals` / `completion.projectChecks`.

**Assets that produce this mission's files**

| Asset | Files it produces here |
|---|---|
| `asset.ControlContextModules` | `src/control/` — control ids, focus, resolver, projections, reducer |
| `asset.SynthContextModules` | `src/synth/` — `VoiceLimit`, Patch state |
| `asset.RealTimeContextModules` | `src/real_time/` — snapshot field, note-on enforcement, refusal counter |
| `asset.ShellContextModules` | `src/shell/` — composition vocabulary |
| `asset.WebviewProjectionPage` | `webview-page/index.html`, `page.css`, `page.js` — the composed strip, Utility, detail shell |
| `asset.TestingContextModules` | `src/testing/` — the live scene and its observation |
| `asset.CrestSynthMain` | `src/bin/crest_synth.rs` — the `--demo-live-patch-editor` mode and its controlled negative flag |
| `asset.BuildMakefile` | `Makefile` — the `demo-live-patch-editor` target |
| `asset.FunctionalPatchEditorAcceptanceTests` | `tests/functional_patch_editor.rs` |
| `asset.ProductDesignAuthority` | `DESIGN.md` durable decisions (already landed at `c6e8290`) |
| `asset.DeliveryRoadmap` | `ROADMAP.md` Phase 5 completion note |

**Declared proof covering the change**

- `validation.functional_patch_editor` — project check, `cargo test --test
  functional_patch_editor`, emitting `CREST_ACCEPTANCE functional_patch_editor
  passed`.
- `validation.asset.make_demo_live_patch_editor` — the target exists.
- `witness.functional_patch_editor` — `make demo-live-patch-editor` with a
  44-field observation and 44 predicates; controlled negative
  `--defeat-patch-selection` expected to exit 1.
- `evidence.functional_patch_editor_contract` — binds the above to
  `validation.semantic_graphical_view_model`, `component_composition`,
  `live_demo`, and `test`.

## Project Structure

### Documentation (this mission)

```
kitty-specs/functional-patch-editor-01KZERV9/
├── spec.md              # /spec-kitty.specify output (committed d9bf2fe)
├── plan.md              # This file
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # /spec-kitty.tasks output — NOT created here
```

No `data-model.md` and no `contracts/`. A crest-spec exists; those artifacts fork
`valueObjects`, `aggregates`, and `ports[].contract` and are rejected at
acceptance. Canonical resources are cited above by ID.

No `research.md`. Phase 0 had no open unknowns: the spec carries no
`[NEEDS CLARIFICATION]` marker, the two decisions that could have gone either way
were settled at sign-off, and the one open question at spec time (whether
carry-forward item 10 had retired with the egui layer) was answered during
crest-spec authoring by inspection — both constants are gone from `src/`.

### Source Code (repository root)

```
src/
├── control/                     # IC-01, IC-03 — reducer, focus, projections
│   ├── patch_control_id.rs      #   + Global / MidiInput / VoiceLimit variants
│   ├── semantic_focus.rs        #   + SurfaceId::PatchDetail, ReturnPath, PatchDetailSubject
│   ├── semantic_resolver.rs     #   + five-row Utility order, detail-subject resolution
│   ├── interaction_state.rs     #   + detailSubject, entry/exit, cross-switch
│   ├── app_state.rs             #   + reducer arms for the new controls and detail entry
│   ├── semantic_graphical_view_model.rs  # + requestedValue, per-row validActions, Detail role
│   └── patch_page_projection.rs #   + the widened PATCH page projection
├── synth/                       # IC-02 — VoiceLimit, Patch state, seeding
├── real_time/                   # IC-02 — snapshot field, note-on refusal, refusal counter
├── shell/                       # IC-04, IC-05 — composition vocabulary
│   └── webview/projection_channel.rs   # IC-08 — retire() guard proof
├── testing/                     # IC-07 — live scene, observation, checkpoints
└── bin/crest_synth.rs           # IC-07 — --demo-live-patch-editor, --defeat-patch-selection

webview-page/                    # IC-04, IC-05 — the composed strip, Utility, detail shell
├── index.html
├── page.css
└── page.js

tests/
└── functional_patch_editor.rs   # IC-06 — deterministic acceptance

Makefile                         # IC-07 — demo-live-patch-editor target
```

**Structure Decision**: the existing single-project layout is unchanged. Every
new file lands in a directory an existing asset already covers, except
`tests/functional_patch_editor.rs`, which the new
`asset.FunctionalPatchEditorAcceptanceTests` covers.

## Implementation Concern Map

> Implementation concerns are not work packages. `/spec-kitty.tasks` translates
> these into executable WPs.

### IC-01 — Widened PATCH control surface

- **Purpose**: give PATCH the canonical focus identities and reducer arms for the
  three Utility controls that had no path at all, and declare the Utility resolver
  as exactly five ordered rows.
- **Relevant requirements**: FR-006, FR-007, FR-008, FR-014; crest-spec
  `requirement.bounded_patch_utility_surface`.
- **Canonical resources**: `valueObject.Control.PatchControlId`,
  `service.Control.SemanticResolver`, `aggregate.Control.AppState`.
- **Affected surfaces**: `src/control/patch_control_id.rs`,
  `src/control/semantic_resolver.rs`, `src/control/app_state.rs`,
  `src/control/serialized_state.rs`.
- **Sequencing/depends-on**: IC-02 (the `VoiceLimit` value must exist before a
  control can address it).
- **Risks**: `Global(GlobalParameter)` on PATCH must resolve the *same* canonical
  `GlobalParameters` value the MIXER Inspector edits. The failure mode is a
  plausible-looking second owner that drifts; the acceptance target asserts single
  ownership explicitly rather than trusting the wiring.

### IC-02 — Canonical voice limit and its real-time enforcement

- **Purpose**: create the per-Patch limit, seed it from the engine ceiling, carry
  it to the callback, and enforce it by refusal — the one carry-forward item with
  no state, no descriptor, and no path anywhere.
- **Relevant requirements**: FR-009, NFR-001; crest-spec
  `requirement.canonical_voice_limit`.
- **Canonical resources**: `valueObject.Synth.VoiceLimit`,
  `aggregate.Synth.Patch`, `valueObject.RealTime.ParameterSnapshot`,
  `valueObject.RealTime.AudioObservationSnapshot`,
  `service.RealTime.AudioRenderer`.
- **Affected surfaces**: `src/synth/`, `src/real_time/parameter_snapshot.rs`,
  `src/real_time/audio_renderer.rs`, `src/real_time/audio_observation*.rs`.
- **Sequencing/depends-on**: none. This concern is the mission's critical path and
  should start first.
- **Risks**: the enforcement point is inside the hard real-time callback. It must
  be a fixed integer comparison against the active-note state the observer already
  maintains — not a new scan, not a new allocation, not a voice destruction. A
  note-off must never be refused; refusing one would strand a latched voice. Both
  are asserted, and the refusal counter exists so a limit that never bit fails a
  predicate rather than passing unnoticed.

### IC-03 — Projection enrichment and the detail surface

- **Purpose**: carry per-row valid actions and the mid-edit requested value, and
  add the subordinate detail surface with its subject, entry rules, and exact
  return.
- **Relevant requirements**: FR-010, FR-011, FR-012, NFR-005; crest-spec
  `requirement.projected_row_intent`,
  `requirement.polymorphic_capability_detail_shell`,
  `requirement.on_screen_patch_selection`.
- **Canonical resources**: `valueObject.Control.PatchDetailSubject`,
  `valueObject.Control.SurfaceId`, `valueObject.Control.ReturnPath`,
  `valueObject.Control.SemanticControlViewModel`,
  `valueObject.Control.SemanticSurfaceViewModel`,
  `aggregate.Control.InteractionState`.
- **Affected surfaces**: `src/control/semantic_focus.rs`,
  `src/control/interaction_state.rs`,
  `src/control/semantic_graphical_view_model.rs`,
  `src/control/patch_page_projection.rs`, `src/control/app_state.rs`.
- **Sequencing/depends-on**: IC-01.
- **Risks**: per-row `validActions` must come from the *same* pure resolver that
  computes the model-level list, evaluated for a counterfactual focus — a second
  hand-maintained list is exactly what the declaration forbids, and it would look
  correct until the two drifted. `requestedValue` must be present only while a
  correlated edit is in flight; an optimistic value written ahead of the reducer
  would break the one-way loop while appearing to work.

### IC-04 — Composed PATCH strip and Utility panel

- **Purpose**: replace the flat row run with the authored grouped strip, and paint
  the five Utility rows, the hint line, per-row hints, ranges, units, and requested
  values.
- **Relevant requirements**: FR-001, FR-003, FR-004, FR-005, FR-013, FR-015,
  NFR-003; crest-spec `requirement.composed_patch_strip`,
  `requirement.authored_control_presentation`.
- **Canonical resources**: `valueObject.Shell.ShellComposition` (`PatchStrip`),
  `asset.WebviewProjectionPage`.
- **Affected surfaces**: `webview-page/page.js`, `webview-page/page.css`,
  `webview-page/index.html`, `src/shell/component_vocabulary.rs`.
- **Sequencing/depends-on**: IC-01, IC-03.
- **Risks**: the grouping must survive both authored viewports without clipping,
  overlap, or a hidden band, and a group with no view data must mark itself
  unavailable rather than vanish. The page must derive no label of its own — the
  `masterGainDb` defect this mission closes was exactly a projected key reaching
  the screen, and re-deriving labels page-side would reopen it in a new place.

### IC-05 — Polymorphic capability detail shell

- **Purpose**: paint the subordinate detail surface as one composition serving
  instrument and effect subjects alike.
- **Relevant requirements**: FR-012; crest-spec
  `requirement.polymorphic_capability_detail_shell`.
- **Canonical resources**: `valueObject.Shell.ShellComposition`
  (`CapabilityDetailShell`), `asset.WebviewProjectionPage`.
- **Affected surfaces**: `webview-page/page.js`, `webview-page/page.css`.
- **Sequencing/depends-on**: IC-03, IC-04.
- **Risks**: the temptation is a branch on subject kind, which reintroduces the
  per-engine page the vocabulary exists to prevent. The acceptance target asserts
  one surface identity serving two subjects rather than merely that both render.

### IC-06 — Deterministic acceptance

- **Purpose**: the declared project validation — non-vacuous, production-path
  proof of every claim above, including its own falsification.
- **Relevant requirements**: all FRs; crest-spec `validation.functional_patch_editor`.
- **Canonical resources**: `asset.FunctionalPatchEditorAcceptanceTests`.
- **Affected surfaces**: `tests/functional_patch_editor.rs`.
- **Sequencing/depends-on**: IC-01 through IC-05.
- **Risks**: vacuity. A fixture that switches once between two Patches of the same
  capability proves almost nothing; the asset's prompts require more than two
  Patches across both engines. Each guard must be shown to fail when its subject
  is defeated, not merely to pass.

### IC-07 — The live scene and its target

- **Purpose**: close LIMIT-1 — navigate Patch-to-Patch on screen, then run the
  full effect-slot journey and an audible edit on the *second* instrument, with
  checkpoints correlating switch, focus, and audible consequence.
- **Relevant requirements**: FR-016, NFR-002, NFR-004; crest-spec
  `requirement.functional_patch_editor_behavioral_proof`,
  `witness.functional_patch_editor`.
- **Canonical resources**: `valueObject.Testing.LiveDemoScene`,
  `valueObject.Testing.FunctionalPatchEditorObservation`,
  `asset.CrestSynthMain`, `asset.BuildMakefile`.
- **Affected surfaces**: `src/testing/`, `src/bin/crest_synth.rs`, `Makefile`.
- **Sequencing/depends-on**: IC-01 through IC-05.
- **Risks**: this is the mission's exit gate and it runs on hardware — it needs
  the external display awake, and the harness refuses rather than degrading. The
  scene must measure the audible delta on each Patch's *own* output; an edit that
  moved both signals proves nothing about reach. The `--defeat-patch-selection`
  negative must actually fail, or the whole demonstration is unfalsifiable.

### IC-08 — Projection channel retirement guard proof

- **Purpose**: prove the load-bearing de-duplication guard in `retire()`, folded
  in from the previous mission's LOW follow-ups because this mission drives a
  whole-surface reprojection per patch switch plus a detail surface appearing and
  disappearing through the same channel.
- **Relevant requirements**: none directly — DIRECTIVE_025.
- **Affected surfaces**: `src/shell/webview/projection_channel.rs`.
- **Sequencing/depends-on**: none.
- **Risks**: small and self-contained. The reviewer left the proving test ready to
  paste; the only risk is landing it without confirming it fails when the guard is
  removed.
