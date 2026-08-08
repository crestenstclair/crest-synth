---
description: "Work packages for the Functional Patch Editor Blockout"
---

# Tasks: Functional Patch Editor Blockout

**Input**: Design documents from `kitty-specs/functional-patch-editor-01KZERV9/`
**Prerequisites**: plan.md (required), spec.md (required)

**Tests**: Required. This mission declares one project validation
(`validation.functional_patch_editor`) and one falsifiable witness
(`witness.functional_patch_editor`) in the crest-spec; both are gates.

**Organization**: Work packages (WPxx) group subtasks (Txxx). Each WP is
independently implementable and derives from the crest-spec assets named in
`plan.md`'s Crest-Spec Derivation section.

## Format: `Txxx [P?] [WPxx] Description`

- **[P]**: parallel-safe (different files, no dependency)
- Paths are repo-root relative.

## Path Conventions

Single Rust project: `src/`, `tests/` at repository root, plus `webview-page/`
for the projection page assets.

## Subtask Index

| ID | Description | WP | Parallel |
| --- | --- | --- | --- |
| T001 | Declare the `VoiceLimit` value object and its surface descriptor | WP01 | |
| T002 | Add `voiceLimit` to the Patch aggregate and seed it from the engine ceiling | WP01 | |
| T003 | Carry the limit on `RtPatchParameters` in the parameter snapshot | WP01 | |
| T004 | Refuse note-on beyond the limit in the audio renderer | WP01 | |
| T005 | Count refusals in `AudioObservationSnapshot` | WP01 | |
| T006 | Prove the callback contract holds and the limit is falsifiable | WP01 | |
| T007 | Add `Global`, `MidiInput`, `VoiceLimit` to `PatchControlId` with serialization | WP02 | |
| T008 | Declare the five-row PATCH Utility resolver order | WP02 | |
| T009 | Add reducer arms for master gain from PATCH, MIDI input, and voice limit | WP02 | |
| T010 | Add `PatchDetailSubject`, `SurfaceId::PatchDetail`, and the widened `ReturnPath` | WP02 | |
| T011 | Add `detailSubject` to `InteractionState` with entry, exit, and refusal rules | WP02 | |
| T012 | Clear subordinate surfaces and recover focus across a patch switch | WP02 | |
| T013 | Project per-row `validActions` from the model-level resolver | WP03 | |
| T014 | Project `requestedValue` for exactly the in-flight correlated row | WP03 | |
| T015 | Project the `Detail` surface role and its capability-resolved content | WP03 | |
| T016 | Guarantee authored labels reach the projection, never serialization keys | WP03 | |
| T017 | Make the one-generation patch switch observable in the projection | WP03 | |
| T018 | Prove `retire()`'s de-duplication guard in the projection channel | WP03 | |
| T019 | Confirm the guard's proving test fails when the guard is removed | WP03 | |
| T020 | Render the PATCH main workspace as the grouped `PatchStrip` composition | WP04 | |
| T021 | Render the Patch identity and routing header | WP04 | |
| T022 | Render the envelope group as one titled group | WP04 | |
| T023 | Render one titled group per ordered effect slot with nested occupant rows | WP04 | |
| T024 | Paint each numeric row's range and unit | WP04 | |
| T025 | Paint each row's own valid actions as its right-hand hint | WP04 | |
| T026 | Paint the active and requested values on a row mid-structural-edit | WP04 | |
| T027 | Render the five-row Utility panel and its authored hint line | WP04 | |
| T028 | Render the `CapabilityDetailShell` for both subject kinds | WP04 | |
| T029 | Prove on-screen patch selection, refusal at the ends, and focus recovery | WP05 | |
| T030 | Prove the strip is grouped structure and not a flat control run | WP05 | |
| T031 | Prove the five Utility rows, the single master-gain owner, and MIDI rechannelling | WP05 | |
| T032 | Prove per-row actions, requested values, and painted ranges and units | WP05 | |
| T033 | Prove one detail identity serves two subjects and returns to the exact origin | WP05 | |
| T034 | Prove the voice limit is bounded, seeded, enforced, and falsifiable | WP05 | |
| T035 | Declare `FunctionalPatchEditorObservation` and its measurement rules | WP06 | |
| T036 | Build the live scene plan whose subject is the second Patch | WP06 | |
| T037 | Emit checkpoints correlating the switch, the focus, and the audible edit | WP06 | |
| T038 | Add the `--demo-live-patch-editor` mode to the binary | WP06 | |
| T039 | Add the `--defeat-patch-selection` controlled negative | WP06 | |
| T040 | Add the `make demo-live-patch-editor` target | WP06 | |
| T041 | Record the Phase 5 completion note and LIMIT-1 closure in ROADMAP.md | WP06 | |

## Work Packages

### WP01 - Canonical voice limit and its real-time enforcement

**Goal**: Create the per-Patch voice limit — the one carry-forward item with no
state, no descriptor, and no path anywhere — and enforce it inside the audio
callback by refusal rather than stealing.
**Priority**: P1
**Independent Test**: With the fixture playing, a limit set below the sounding
count stops new notes from starting, leaves latched voices alone, counts the
refusals in the bounded observation, and keeps callback allocations at zero.
**Prompt**: `tasks/WP01-canonical-voice-limit.md`

**Subtasks**:

T001 Declare the `VoiceLimit` value object and its surface descriptor in `src/synth/`
T002 Add `voiceLimit` to the Patch aggregate and seed it from the engine ceiling
T003 Carry the limit on `RtPatchParameters` in `src/real_time/parameter_snapshot.rs`
T004 Refuse note-on beyond the limit in `src/real_time/audio_renderer.rs`
T005 Count refusals in `AudioObservationSnapshot`
T006 Prove the callback contract holds and that a defeated limit fails a predicate

**Implementation Sketch**:

1. Value object first, with bounds, Stepped classification, and fine/coarse steps.
2. Patch field plus installation seeding from the active engine's ceiling.
3. Snapshot field — plain bounded integer, Copy, destructor-free.
4. Renderer: compare against the active-note count the observer already tracks.
5. Refusal counter, saturating, in the same path as `routingFailures`.

**Dependencies**: None — this is the critical path and starts first.
**Risks**: The enforcement point is the hard real-time callback. It must be a
fixed integer test, not a new scan; it must never refuse a note-off; it must
never destroy a latched voice.

---

### WP02 - Widened PATCH control surface and reducer

**Goal**: Give PATCH the canonical focus identities and reducer arms for the
three Utility controls that had no path, and add the subordinate detail surface
identity with its entry, exit, and cross-switch rules.
**Priority**: P1
**Independent Test**: PATCH Utility resolves exactly five ordered rows, each
editable through `AppState::apply`; entering detail from a resolving row records
the exact origin and refuses from an empty slot or a Utility row.
**Prompt**: `tasks/WP02-widened-patch-control-surface.md`

**Subtasks**:

T007 Add `Global`, `MidiInput`, `VoiceLimit` to `PatchControlId` with serialization
T008 Declare the five-row PATCH Utility resolver order in `src/control/semantic_resolver.rs`
T009 Add reducer arms for master gain from PATCH, MIDI input, and voice limit
T010 Add `PatchDetailSubject`, `SurfaceId::PatchDetail`, and the widened `ReturnPath`
T011 Add `detailSubject` to `InteractionState` with entry, exit, and refusal rules
T012 Clear subordinate surfaces and recover focus across a patch switch

**Implementation Sketch**:

1. Control identities and their serialization derivations.
2. The Utility resolver's five-row order.
3. Reducer arms — master gain must resolve the *same* canonical value the MIXER
   Inspector edits, through the same descriptor.
4. Surface identity, subject, and return path.
5. Interaction state transitions, moving all three detail fields together.

**Dependencies**: WP01 — a control cannot address `VoiceLimit` before it exists.
**Risks**: The master-gain path is the one that can silently go wrong. A second
owner would look correct until the two values drifted; WP05 asserts single
ownership explicitly rather than trusting the wiring.

---

### WP03 - Projection enrichment and channel integrity

**Goal**: Carry per-row valid actions and the mid-edit requested value into the
projection, project the detail surface, guarantee authored labels, and prove the
projection channel's load-bearing de-duplication guard.
**Priority**: P1
**Independent Test**: Every projected control carries its own action list
agreeing with the model-level list at the focused row, and a `requestedValue`
present on exactly the in-flight row and no other.
**Prompt**: `tasks/WP03-projection-enrichment.md`

**Subtasks**:

T013 Project per-row `validActions` from the same pure resolver as the model-level list
T014 Project `requestedValue` for exactly the in-flight correlated row
T015 Project the `Detail` surface role and its capability-resolved content
T016 Guarantee authored labels reach the projection, never serialization keys
T017 Make the one-generation patch switch observable in the projection
T018 Prove `retire()`'s de-duplication guard in `src/shell/webview/projection_channel.rs`
T019 Confirm that proving test fails when the guard is removed

**Implementation Sketch**:

1. Per-row actions resolved for a counterfactual focus, through the existing
   resolver — never a second list.
2. Requested value sourced from the correlated structural lifecycle only.
3. Detail surface projected exactly while `InteractionState` holds an entry.
4. Label guarantee: the `masterGainDb` defect closes here.
5. The folded-in LOW follow-up, with its falsification.

**Dependencies**: WP02.
**Risks**: A second hand-maintained action list would look correct until it
drifted from the resolver. An optimistic `requestedValue` written ahead of the
reducer would break the one-way loop while appearing to work.

---

### WP04 - The composed PATCH surface in the webview page

**Goal**: Replace the flat row run with the authored grouped strip, paint every
row's range, unit, hint, and requested value, seat the five-row Utility panel,
and render the polymorphic detail shell.
**Priority**: P1
**Independent Test**: PATCH paints an identity header, an instrument selector, a
titled envelope group, and one titled group per effect slot with nested occupant
rows, at both authored viewports, with no clipped or overlapped row.
**Prompt**: `tasks/WP04-composed-patch-surface.md`

**Subtasks**:

T020 Render the PATCH main workspace as the grouped `PatchStrip` composition
T021 Render the Patch identity and routing header
T022 Render the envelope group as one titled group
T023 Render one titled group per ordered effect slot with nested occupant rows
T024 Paint each numeric row's range and unit
T025 Paint each row's own valid actions as its right-hand hint
T026 Paint the active and requested values on a row mid-structural-edit
T027 Render the five-row Utility panel and its authored hint line
T028 Render the `CapabilityDetailShell` for both subject kinds

**Implementation Sketch**:

1. Group anatomy in `page.css`, group rendering in `page.js`.
2. Header, envelope group, slot groups — each with its own unavailable marking.
3. Row-level painting: range, unit, hint, requested value.
4. Utility panel — bounded at five rows, so no scroll affordance.
5. Detail shell — one composition, no branch on subject kind.

**Dependencies**: WP03.
**Risks**: Nine subtasks, above the usual target. They are kept in one package
because `page.js` and `page.css` are a single coupled surface — splitting them
across packages would produce a merge conflict rather than parallelism. The page
must derive no label of its own; re-deriving labels page-side would reopen the
serialization-key defect in a new place.

---

### WP05 - Deterministic acceptance

**Goal**: The declared project validation — non-vacuous, production-path proof of
every claim, including its own falsification.
**Priority**: P1
**Independent Test**: `cargo test --test functional_patch_editor` emits
`CREST_ACCEPTANCE functional_patch_editor passed` and exits 0.
**Prompt**: `tasks/WP05-deterministic-acceptance.md`

**Subtasks**:

T029 Prove on-screen patch selection, refusal at the ends, and focus recovery
T030 Prove the strip is grouped structure and not a flat control run
T031 Prove the five Utility rows, the single master-gain owner, and MIDI rechannelling
T032 Prove per-row actions, requested values, and painted ranges and units
T033 Prove one detail identity serves two subjects and returns to the exact origin
T034 Prove the voice limit is bounded, seeded, enforced, and falsifiable

**Implementation Sketch**:

1. Fixture with more than two Patches across both engines — a single switch
   between two Patches of the same capability is vacuous.
2. Each guard demonstrated to fail when its subject is defeated.
3. Drive everything through the production reducer and render path.

**Dependencies**: WP04.
**Risks**: Vacuity is the whole risk here. A test that asserts a structure exists
without driving it through production seams passes while proving nothing.

---

### WP06 - The live scene, its target, and the roadmap close

**Goal**: Close LIMIT-1 — navigate Patch-to-Patch on screen, then run the full
effect-slot journey and an audible edit on the *second* instrument, with
checkpoints correlating switch, focus, and audible consequence.
**Priority**: P1
**Independent Test**: `make demo-live-patch-editor` exits 0 with a complete
report on a real window with physical audio; the `--defeat-patch-selection`
negative exits 1.
**Prompt**: `tasks/WP06-live-patch-editor-scene.md`

**Subtasks**:

T035 Declare `FunctionalPatchEditorObservation` and its measurement rules
T036 Build the live scene plan whose subject is the second Patch
T037 Emit checkpoints correlating the switch, the focus, and the audible edit
T038 Add the `--demo-live-patch-editor` mode to `src/bin/crest_synth.rs`
T039 Add the `--defeat-patch-selection` controlled negative
T040 Add the `make demo-live-patch-editor` target
T041 Record the Phase 5 completion note and LIMIT-1 closure in `ROADMAP.md`

**Implementation Sketch**:

1. Observation type mirroring the declared witness schema.
2. Scene plan that selects a second Patch and derives its targets from *that*
   Patch's descriptors.
3. Checkpoints correlating the three facts the gate requires.
4. Binary mode, controlled negative, Makefile target.
5. The roadmap note, written after the evidence exists.

**Dependencies**: WP04. Runs in parallel with WP05.
**Risks**: This is the exit gate and it runs on hardware. It needs the external
display awake; the harness refuses rather than degrading. The scene must measure
the audible delta on each Patch's *own* output — an edit that moved both proves
nothing about reach. If `--defeat-patch-selection` does not actually fail, the
whole demonstration is unfalsifiable.

---

## Dependencies & Execution Order

- **WP01**: no dependencies — starts immediately
- **WP02**: depends on WP01
- **WP03**: depends on WP02
- **WP04**: depends on WP03
- **WP05**: depends on WP04
- **WP06**: depends on WP04 — parallel with WP05

## Parallel Execution Opportunities

WP05 and WP06 run simultaneously once WP04 lands. The chain WP01 → WP02 → WP03 →
WP04 is genuinely sequential: each layer's output is the next layer's input, and
the domain value must exist before a control can address it, before a projection
can carry it, before a page can paint it.

## Implementation Strategy

**Critical path**: WP01 → WP02 → WP03 → WP04, then WP05 and WP06 in parallel.

**MVP**: WP01 through WP04 delivers the functional Patch editor on screen. WP05
and WP06 are what make it provable, and WP06 is the mission's declared exit gate
— the mission does not close without it.
