# Mission Specification: Phase 3 Demo Journey Fidelity and Hygiene

**Mission Branch**: `feat/expandable-effects-and-bus-topology` (mission `demo-journey-fidelity-and-hygiene-01KYWVYG`)
**Created**: 2026-07-31
**Status**: Draft
**Input**: User description: ROADMAP.md "Current corrective gate — Phase 3 demo journey fidelity and mission hygiene" (ROADMAP.md:73-84), invoked verbatim. Source findings: parent mission review `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/mission-review.md` — DRIFT-6 addendum (HIGH) plus open items 1–7.

## Why (crest-spec grounding — cited, not restated)

This mission adds no new product capability. It restores evidence fidelity and
retires transitional debt for the already-declared structure:

- `capability.expandable_effects_and_bus_topology` — its acceptance journeys
  (`ordered_patch_effect_chain`, `open_effect_registry`, `bounded_bus_routing`)
  are already proven at reducer/render level; what is missing is the on-screen
  player journey in the retained live scene.
- `capability.live_observable_demo` (`acceptance.live_scene`,
  `acceptance.coherent_live_trace`) and `evidence.exhaustive_demo_scene` /
  `evidence.expandable_effects_and_bus_topology_contract` — the retained Phase 3
  scene is the live-evidence surface this mission reworks.
- `actor.player` walks the on-screen journey; `actor.maintainer` consumes the
  refreshed evidence and the amended acceptance artifacts.

Structure this mission retires that the crest-spec still reflects: the
transitional compacted post-effects view of the Patch effect chain (see
`requirement.fixed_patch_effect_topology` / `requirement.canonical_patch_effect_control`
lineage). The retirement is authored in the crest-spec FIRST during the
`/spec-kitty.crest-spec` phase, before planning; this spec only records the
intent.

**Bulk-edit declaration**: This mission retires the transitional compacted
chain view `post_effects()` (old surface) in favor of the canonical
`effect_slots()` (never-compacted surface) across all callers, and removes the
leftover `reverbSend` test-fixture literals, with per-category rules captured
in `occurrence_map.yaml` (`change_mode: bulk_edit` is set in `meta.json`).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch the slot and return journey happen on screen (Priority: P1)

A player watches the retained Phase 3 live scene (`make demo-live-effects-and-buses`)
on a physical device. Every effect-slot occupancy change now happens the way a
player would perform it: focus visibly moves to the effect-slot row on the
PATCH page, occupancy cycles using the same adjacent-choice gesture that
changes an engine, and at least one parameter of an installed occupant is
edited from the PATCH page with an audible result. Return occupancy changes
likewise travel the MIXER return rows on screen. The single controlled
rejection remains a direct injection — the UI cannot request an unknown entry
by design — and the scene documents that exception where it occurs.

**Why this priority**: This is the HIGH finding (DRIFT-6) that superseded the
parent mission's retained-live-demo requirement and live-demo-gate grading
(its requirement 19 and constraint 10) and is the declared gate for all
later phases. Without it the phase's headline UI functionality remains
undemonstrated in live evidence.

**Independent Test**: Run the retained scene on a physical device and inspect
the refreshed recorded evidence: each slot occupancy change is preceded by
on-screen focus on that slot's row and an adjacent-choice cycle; each return
occupancy change is preceded by focus on that return's MIXER row; one occupant
parameter edit from the PATCH page is audible in the recording.

**Acceptance Scenarios**:

1. **Given** the retained scene running with a real window, physical audio, and
   the real MIDI fixture, **When** the scene changes any effect slot's
   occupancy, **Then** focus is visibly on that slot's PATCH row first and the
   change is performed by the adjacent-choice gesture — not by backstage
   injection.
2. **Given** an installed occupant on the PATCH page, **When** the scene edits
   one of its descriptor-driven parameters from that page, **Then** the edit is
   visible on screen and audibly changes the rendered output.
3. **Given** the MIXER page's return rows, **When** the scene changes a
   return's occupancy, **Then** focus is visibly on that return row first and
   the change is performed through the same on-screen vocabulary.
4. **Given** the scene's controlled rejection of an unknown entry, **When** it
   executes via direct injection, **Then** the scene source documents at that
   point why injection is the only possible path (the UI cannot express an
   unknown entry), and the rejection still shows its visible reason.
5. **Given** the evidence recorded before this mission, **When** the refreshed
   evidence is compared against it, **Then** every pre-existing checkpoint
   identity is byte-identical and all changes are additions.

---

### User Story 2 - Trust the merged phase's hygiene (Priority: P2)

A maintainer reading the Phase 3 post-merge review finds every non-blocking
open item closed: the transitional compacted chain view is retired (all callers
migrated to the canonical never-compacted view, and the production composition
root no longer re-compacts a gapped chain on round-trip); a failed default
return composition at production boot surfaces an explicit error instead of
silently playing with empty returns; the RETURN-clear held-note continuity twin
test exists; the live-report measurement fields distinguish "no evidence
recorded" from a measured zero; stale work-package handoff comments, the
`DESIGN.md` "aux buses" wording, and leftover `reverbSend` fixture literals are
gone; and the name-enumeration guard script fails loudly when its tools are
missing instead of passing vacuously.

**Why this priority**: These are the review's recorded open items 1–3, 5, and 7.
Leaving them open past the corrective gate turns documented debt into permanent
drift, and two of them (silent boot fallback, vacuous guard) are silent-failure
classes.

**Independent Test**: Each item is independently verifiable: search for
remaining callers of the retired view (must be zero); boot with a failing
default-return composition (must fail visibly); run the new twin test; feed the
live report a run missing a measurement (must render as absent, not zero);
grep for the stale comments/wording/literals (must be gone); run the guard
script without its tools on PATH (must exit non-zero).

**Acceptance Scenarios**:

1. **Given** a Patch whose chain has an empty slot before an occupied slot,
   **When** it round-trips through the production composition root, **Then**
   the gap is preserved exactly (no re-compaction) and the retired compact view
   has zero remaining callers.
2. **Given** a production boot where default bus-return composition fails,
   **When** the instrument starts, **Then** startup surfaces the composition
   error explicitly instead of proceeding with silent returns.
3. **Given** held notes sounding through an occupied return, **When** that
   return is cleared, **Then** a dedicated twin test proves sample-level
   continuity equivalent to the existing slot-clear proof.
4. **Given** a live run that never populated a measurement field, **When** the
   live report renders, **Then** the field reads as absent evidence, and only a
   genuinely measured zero reads as zero.
5. **Given** the name-enumeration guard script on a machine missing one of its
   required tools, **When** it runs, **Then** it exits non-zero naming the
   missing tool rather than reporting "no candidates".

---

### User Story 3 - Optional hardening while the area is open (Priority: P3)

A maintainer additionally gains, if cheap while the relevant areas are already
open: an end-to-end fixture that registers a fourth effect entry and drives it
through slot, return, preparation, projection, and render — converting the
parent mission's SC-008 structural inference into a demonstration — and
per-position engine-capability identity in the prepared-graph layout
attestation, deepening the carry-over guards' defense-in-depth.

**Why this priority**: Explicitly optional in the corrective gate ("if cheap
while there"; review open items 4 and 6). Deferring either does not fail the
mission, but the deferral must be recorded in the amended review addendum.

**Independent Test**: The fourth-entry fixture passes end-to-end with zero
changes to slot/routing/preparation/projection/render structure; the layout
attestation rejects a same-scalar-count wrong-engine substitution at a
non-selected position.

**Acceptance Scenarios**:

1. **Given** a test registry with a fourth effect entry, **When** the fixture
   drives it through slot and return occupancy, preparation, projection, and
   render, **Then** it behaves as a full citizen with zero structural changes.
2. **Given** a prepared layout carrying per-position engine-capability
   identity, **When** a carry-over candidate presents the right scalar count
   but the wrong engine identity, **Then** the carry-over is refused and the
   fresh instance is kept.

---

### Edge Cases

- A gapped effect chain (empty slot before an occupied slot) round-tripping
  through the composition root — must survive un-compacted (US2 scenario 1).
- Default-return composition failure at boot — must be loud, not silent.
- A live run whose report is missing a measurement field — must read as
  absent, never as an instant/zero pass.
- Guard script executed where `rg`/`perl` are unavailable — must fail naming
  the dependency.
- The controlled rejection: the one occupancy change that legitimately cannot
  travel the UI journey — kept as injection, documented inline in the scene.
- Held notes across a RETURN clear — continuity must be proven by a dedicated
  twin test, not inferred from the slot-clear proof.
- Checkpoint comparison when the refreshed run adds new checkpoints — existing
  identities must remain byte-identical; only additions are permitted.

## Domain Language *(canonical terms)*

- **bus returns** — canonical; do not write "aux buses" (the last such wording,
  `DESIGN.md:204`, is corrected by this mission).
- **effect slot / slot occupancy** — the PATCH-page ordered chain positions;
  occupancy cycles via the **adjacent-choice gesture** (the same vocabulary as
  engine selection — no second vocabulary).
- **canonical chain view** — the never-compacted per-slot view
  (`effect_slots()`); the transitional compacted view (`post_effects()`) is
  retired by this mission and must not be reintroduced.
- **checkpoint identity** — the byte-comparable identity of a retained-scene
  checkpoint; this mission's evidence changes are **add-only**.
- **retained scene** — the phase's named live demo
  (`demo-live-effects-and-buses`); it is retained evidence, never replaced by
  later phases.
- `reverbSend` — forbidden legacy term; remaining test-fixture literals are
  removed by this mission.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | User Story | Priority | Status |
|----|-------|------------|----------|--------|
| FR-001 | On-screen slot occupancy journey | As a player, I want every effect-slot occupancy change in the retained scene performed by moving focus to that slot's PATCH row and cycling with the adjacent-choice gesture, so that the phase's headline UI journey is demonstrated, not simulated. | High | Open |
| FR-002 | Audible occupant parameter edit from PATCH | As a player, I want the scene to edit at least one installed occupant's parameter from the PATCH page with an audible result, so that descriptor-driven parameter rows are demonstrated live. | High | Open |
| FR-003 | On-screen return occupancy journey | As a player, I want every return occupancy change in the retained scene performed through the MIXER return rows on screen, so that return management is demonstrated as a player experiences it. | High | Open |
| FR-004 | Documented rejection exception | As a maintainer, I want the controlled rejection's direct injection kept and documented inline in the scene (the UI cannot request an unknown entry by design), so that the one sanctioned exception is explicit. | Medium | Open |
| FR-005 | Physical re-run with refreshed evidence | As a maintainer, I want the reworked scene re-run on a physical device and the recorded evidence refreshed, so that the live gate grades against the player journey actually shown. | High | Open |
| FR-006 | Amended acceptance artifacts | As a maintainer, I want the parent mission's acceptance matrix and post-merge review addendum amended to reference the refreshed evidence and the disposition of every open item, so that the record is honest and complete. | High | Open |
| FR-007 | Retire the compacted chain view | As a maintainer, I want all callers migrated to the canonical never-compacted chain view and the transitional compacted view deleted, with the composition-root round-trip preserving gapped chains exactly, so that one representation of the chain remains. | High | Open |
| FR-008 | Loud default-return composition failure | As a maintainer, I want a failed default bus-return composition to propagate as an explicit error at the production composition root, so that the instrument never boots silently degraded. | High | Open |
| FR-009 | RETURN-clear continuity twin test | As a maintainer, I want a dedicated sample-level twin test proving held-note continuity across a return clear, so that the return path's continuity contract is proven, not inferred. | Medium | Open |
| FR-010 | Absent-vs-zero live measurements | As a maintainer, I want live-report measurement fields to distinguish absent evidence from a measured zero, so that a regression that stops populating them cannot read as the strongest pass. | Medium | Open |
| FR-011 | Stale handoff-comment cleanup | As a maintainer, I want the stale work-package-numbered handoff comments removed from durable code, so that comments state constraints, not an erased timeline. | Low | Open |
| FR-012 | Canonical bus-return wording | As a maintainer, I want the remaining "aux buses" wording in the design document corrected to the canonical bus-return vocabulary. | Low | Open |
| FR-013 | Legacy fixture-literal cleanup | As a maintainer, I want the leftover `reverbSend` test-fixture literals removed, so that no forbidden-term residue survives in fixtures. | Low | Open |
| FR-014 | Guard script tool gating | As a maintainer, I want the name-enumeration guard script to fail non-zero when a required tool is missing, so that the guard can never pass vacuously. | Medium | Open |
| FR-015 | Fourth-entry end-to-end fixture (optional) | As a maintainer, I want an end-to-end fixture registering a fourth effect entry and driving it through slot, return, preparation, projection, and render, so that registry openness is demonstrated rather than inferred. Deferrable with recorded rationale. | Low | Open |
| FR-016 | Per-position engine identity attestation (optional) | As a maintainer, I want the prepared-graph layout to carry engine-capability identity per position so carry-over guards can refuse a same-scalar-count wrong engine. Deferrable with recorded rationale. | Low | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Evidence completeness of the refreshed run | The refreshed physical run completes 100% of its declared checkpoints with 0 dropped records, 0 false observation keys, and clean teardown — matching or exceeding the parent mission's recorded live-run standard. | Reliability | High | Open |
| NFR-002 | Zero silent-fallback paths in swept areas | After the sweep, 0 code paths remain where a default-return composition failure, an absent live measurement, or a missing guard tool produces a passing/benign result. | Reliability | High | Open |
| NFR-003 | Regression safety | The full test suite and all previously declared deterministic checks pass with 0 failures after every change in this mission. | Quality | High | Open |
| NFR-004 | Journey visibility pacing | Every on-screen journey step in the reworked scene (focus move, occupancy cycle, parameter edit) is visible in the recorded evidence as a distinct step — 0 occupancy changes appear on screen without their preceding focus step. | Usability | Medium | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | Add-only checkpoint identity | Every checkpoint identity present in the parent mission's recorded evidence remains byte-identical; the reworked scene only adds checkpoints. | Technical | High | Open |
| C-002 | Production journey path | All new scene interactions travel physical-input → semantic action → `AppState::apply` → view/audio projections through the production reducer and render path; no scene-only shortcuts or new backdoors. | Technical | High | Open |
| C-003 | Single sanctioned injection | The controlled rejection is the only occupancy change in the scene permitted to use direct semantic-action injection, and it carries inline documentation of why. | Technical | High | Open |
| C-004 | Real-time discipline unchanged | No change weakens the hard real-time callback's bounded, preallocated, allocation/lock/block/IO/log/panic-free contract. | Technical | High | Open |
| C-005 | Crest-spec first | The compacted-view retirement (and any other structural change) is authored in the crest-spec before planning; `data-model.md`/`contracts/` are never produced. | Process | High | Open |
| C-006 | No scope growth | No new effects, engines, buses, UI surfaces, or Phase 4 component work; this gate only heals Phase 3 evidence and hygiene. | Business | High | Open |
| C-007 | Bulk-edit discipline | The `post_effects()` → `effect_slots()` migration and `reverbSend` cleanup follow the approved `occurrence_map.yaml`; serialized keys and external contracts are not renamed. | Process | High | Open |
| C-008 | Amend, do not rewrite | The parent mission's acceptance matrix and review addendum are amended add/append-style; existing recorded history is not altered or deleted. | Process | Medium | Open |

### Key Entities

- **Retained scene checkpoint**: a named, byte-comparable evidence record in
  the Phase 3 live scene; pre-existing identities are immutable this mission.
- **Live demo report measurement**: a per-run measured quantity that must carry
  an explicit absent state distinct from zero.
- **Effect chain (canonical view)**: the ordered, possibly gapped, never
  compacted per-slot occupancy of a Patch.
- **Occurrence map**: the bulk-edit classification artifact governing which
  categories of `post_effects` / `reverbSend` occurrences change and which
  must not.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the refreshed evidence, 100% of effect-slot occupancy changes
  are preceded on screen by focus on that slot's PATCH row and performed by the
  adjacent-choice gesture; 100% of return occupancy changes travel the MIXER
  return rows; the only direct injection remaining is the single documented
  controlled rejection.
- **SC-002**: At least 1 occupant parameter edit performed from the PATCH page
  is audible in the refreshed recording.
- **SC-003**: A byte-level comparison shows 0 modified and 0 removed checkpoint
  identities versus the parent mission's evidence; all evidence changes are
  additions.
- **SC-004**: The refreshed physical run records 100% of checkpoints, 0 dropped
  records, 0 false observation keys, and clean teardown, and both the parent
  mission's acceptance matrix and review addendum reference it.
- **SC-005**: 0 callers of the retired compacted chain view remain, and a
  gapped chain survives the production composition-root round-trip with its
  gaps intact.
- **SC-006**: 0 startup paths remain that degrade a failed default-return
  composition into silent returns; the failure is observable at boot.
- **SC-007**: Each of the review's 7 open items is closed, or (for the two
  optional items only) recorded as deferred with rationale, in the amended
  addendum — 7 of 7 dispositioned.
- **SC-008**: All existing tests and deterministic checks pass after the
  mission; 0 regressions attributable to the sweep.

## Assumptions

- The physical rig used for the 2026-07-31 parent-mission runs remains
  available for the re-run (FR-005).
- The reducer-level UI vocabulary for slot rows, adjacent-choice cycling, and
  descriptor-driven parameter rows already exists and is deterministically
  proven; this mission adds live demonstration, not new UI capability.
- The two optional hardening items (FR-015, FR-016) may be deferred without
  failing the mission if they prove not-cheap; the deferral decision is
  recorded in the amended addendum (SC-007).
- The parent mission's artifacts (`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/`)
  remain the canonical home for the amended acceptance matrix and review
  addendum; this mission does not fork them.
