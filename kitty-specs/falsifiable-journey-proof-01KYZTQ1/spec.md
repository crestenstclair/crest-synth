# Mission Specification: Falsifiable Journey Proof

**Mission Branch**: `feat/expandable-effects-and-bus-topology`
**Created**: 2026-08-01
**Status**: Draft
**Input**: Post-merge review of `demo-journey-fidelity-and-hygiene-01KYWVYG` (verdict FAIL on
proof adequacy), findings DRIFT-1 and DRIFT-2, plus RISK-1 and the repeated acceptance-matrix
coverage omission.

## Why This Mission Exists

The previous mission reworked the live effects-and-buses demo so every effect-slot occupancy
change travels the player's on-screen PATCH journey and every return occupancy change travels
the MIXER return rows, and it demonstrated that on physical hardware. **That behavior is
correct today.** This mission changes no demonstrated behavior.

What is missing is the ability to notice its loss. The guards that attest the journey assert
over what the scene *declared*, never over what the application *dispatched*. This was proven
by execution, not inspection: replacing the runner's dispatch selection with unconditional
direct injection — reintroducing the exact defect the previous mission was chartered to close —
left the suite at exit 0, 1 passed, 0 failed, and would leave every checkpoint identity, the
live report, and the recorded hardware evidence byte-identical.

A proof whose failure has never been observed is not proof. This mission makes these guards
falsifiable and, as a standing rule of the mission, accepts no new guard until its failure has
been observed and recorded.

## Crest-Spec Grounding

This mission strengthens the proof attached to declarations the crest-spec already carries; it
references them rather than restating them.

| Canonical ID | Relationship |
|---|---|
| `requirement.expandable_effects_behavioral_proof` | The requirement whose journey clause is currently unfalsifiable. Its text is correct; its proof is not. |
| `capability.expandable_effects_and_bus_topology` | The capability whose acceptance the strengthened proof serves. |
| `goal.expand_effects_and_buses` | The goal the capability contributes to. |
| `valueObject.Testing.LiveDemoCheckpoint` | Already declares `input: SemanticAction \| AppEvent` — the concept this mission extends to the topology-specific checkpoint, which omits it. |
| `valueObject.Testing.LiveDemoReport` | Carries the checkpoint sequence and measurement fields the refreshed evidence must populate. |
| `aggregate.Synth.Patch` | Owns the ordered per-position slot view whose occupancy identity RISK-1 concerns. |

Structure this mission needs that the crest-spec **does not yet declare**, and which the
`/spec-kitty.crest-spec` phase must author before planning:

- a dispatched-input-kind observation on the topology checkpoint, and the invariant binding it
  to the event the production reducer actually received;
- an occupant-scalar before/after observation on the parameter-edit checkpoint, and the
  invariant that its criterion is satisfied only by a change;
- the position-derived slot identity invariant on `aggregate.Synth.Patch`;
- a project validation asserting acceptance-matrix coverage across all requirement kinds.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A lost journey fails the build (Priority: P1)

An engineer changes how the live demo runner turns a planned transition into an application
event — refactoring it, simplifying it, or reverting it — in a way that stops the occupancy
change from travelling the player's on-screen journey and back-injects it directly instead.

**Why this priority**: This is the defect the previous mission was chartered to close, and it
is currently reintroducible with zero test signal. Everything else in this mission is smaller.

**Independent Test**: Perform the mutation (replace the runner's conditional dispatch with
unconditional direct injection), run the declared checks, and observe a failure. Restore and
observe a pass. Delivers the whole value of the mission on its own.

**Acceptance Scenarios**:

1. **Given** the demo drives occupancy through the focused row's adjacent-choice gesture,
   **When** the run completes, **Then** each occupancy checkpoint records that the dispatched
   input was the adjacent-choice gesture, not a direct occupancy command.
2. **Given** the runner is mutated so every occupancy change is dispatched as a direct
   occupancy command, **When** the declared checks run, **Then** at least one check fails and
   names the transition whose dispatched input was not the gesture.
3. **Given** the single documented rejection — whose unknown registry entry the gesture cannot
   express — **When** the declared checks run, **Then** it is recognised as the one permitted
   direct injection by its *recorded* dispatched input, and a second direct injection anywhere
   else fails the check.

---

### User Story 2 - A lost parameter edit fails the build (Priority: P1)

An engineer removes or breaks the audible occupant-parameter edit performed from the PATCH
page, leaving the surrounding chain still sounding.

**Why this priority**: The current criterion is satisfied by the ambient probe note on the
already-sounding chain whether or not the edit dispatches at all. It cannot fail, so it grades
nothing.

**Independent Test**: Remove the parameter edit from the scene, run the declared checks, and
observe a failure. Restore and observe a pass.

**Acceptance Scenarios**:

1. **Given** an occupant scalar is edited from the PATCH page, **When** the checkpoint is
   recorded, **Then** it carries the scalar's value before and after the edit, and they differ.
2. **Given** the edit is removed while the chain keeps sounding, **When** the declared checks
   run, **Then** at least one check fails — acceptance, audibility, and a nonzero active-note
   count are no longer sufficient to pass.
3. **Given** the edit is dispatched but rejected, **When** the checkpoint is recorded, **Then**
   the before and after values are equal and the check fails rather than passing on audibility.

---

### User Story 3 - An effect cannot be recorded at a position it does not belong to (Priority: P2)

A recorded or reconstructed patch carries an effect whose slot identity is valid on its own but
belongs to a different position.

**Why this priority**: Real but latent — no current path produces it. Today the occupancy gate
checks only that identities are unique, so a mismatched-but-valid identity would silently
relocate an effect. Closing it by construction costs little and removes the class.

**Independent Test**: Attempt to install an occupant carrying another position's identity and
observe that the stored result carries the correct position identity regardless.

**Acceptance Scenarios**:

1. **Given** an occupant carrying a slot identity belonging to another position, **When** it is
   installed at a position, **Then** the stored occupant carries that position's identity.
2. **Given** any sequence of occupancy changes, **When** the patch is inspected, **Then** every
   occupied position holds the identity derived from that position, and uniqueness still holds.
3. **Given** a recorded patch is round-tripped through the production composition root,
   **When** it is reloaded, **Then** each effect returns to the position its identity denotes.

---

### User Story 4 - An acceptance matrix cannot omit whole requirement kinds (Priority: P3)

A mission author produces an acceptance matrix that grades functional requirements and silently
omits non-functional requirements and constraints.

**Why this priority**: It has now happened in two consecutive missions by different authors,
which indicts the artifact rather than the author. It is process hygiene, not product behavior,
so it ranks below the three above.

**Independent Test**: Present an acceptance matrix lacking non-functional and constraint rows to
the declared validation and observe that it fails.

**Acceptance Scenarios**:

1. **Given** an acceptance matrix grading every requirement kind declared in its mission spec,
   **When** the validation runs, **Then** it passes.
2. **Given** a matrix omitting all non-functional rows, **When** the validation runs, **Then**
   it fails and names the omitted kind.
3. **Given** the validation's tool dependencies are unavailable, **When** it runs, **Then** it
   fails loudly with a distinct status rather than reporting success.

### Edge Cases

- **A transition declares a gesture but the runner dispatches something else.** The recorded
  dispatched input disagrees with the declaration; the check must fail rather than trusting
  either source alone.
- **A rejection is dispatched by gesture.** The documented direct injection is permitted only
  for the rejection; the rejection is not *required* to be injected. If a future scene expresses
  it by gesture, the guard must accept zero direct injections, not demand exactly one.
- **An occupant scalar's edit lands on its existing value.** A no-op edit produces equal before
  and after values and must fail, not pass — otherwise the criterion returns to unfalsifiable.
- **A measurement field has no evidence.** Absent must remain distinguishable from a measured
  zero; a defaulted zero is a regression of the previous mission's contract.
- **A new checkpoint identity collides with a frozen one.** Additions must be new identities;
  reusing a frozen identity for new meaning breaks the add-only contract.
- **The physical rig is unavailable.** The mission stops and reports; headless output is never
  substituted for physical evidence.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | User Story | Priority | Status |
|----|-------|------------|----------|--------|
| FR-001 | Record the dispatched input kind | As a reviewer, I want each occupancy checkpoint to record which kind of input the production reducer actually received, so that the record shows what happened rather than what was planned. | High | Open |
| FR-002 | Assert the journey over the recorded dispatch | As a reviewer, I want the journey check to read the recorded dispatched input rather than the scene's declaration, so that mutating the dispatch path fails the check. | High | Open |
| FR-003 | Identify the permitted injection by its record | As a reviewer, I want the single documented direct injection recognised by its recorded dispatched input, so that an added injection elsewhere is detected. | High | Open |
| FR-004 | Record the occupant scalar before and after | As a reviewer, I want the parameter-edit checkpoint to carry the edited scalar's value before and after dispatch, so that the edit's effect is visible in the record. | High | Open |
| FR-005 | Require the scalar to have changed | As a reviewer, I want the edit criterion satisfied only when the recorded before and after values differ, so that ambient audibility cannot pass it. | High | Open |
| FR-006 | Falsification-test every new guard | As a reviewer, I want each new guard exercised by a recorded mutation that makes it fail and a restoration that makes it pass, so that no guard is accepted on assertion alone. | High | Open |
| FR-007 | Derive slot identity from position | As a maintainer, I want an installed occupant's slot identity derived from the position it occupies rather than accepted from the caller, so that a mismatched identity is inexpressible. | Medium | Open |
| FR-008 | Gate acceptance-matrix coverage | As a mission author, I want acceptance to fail when a matrix omits a requirement kind its spec declares, so that the omission cannot recur. | Medium | Open |
| FR-009 | Refresh the physical evidence | As a reviewer, I want the reworked scene re-run on the physical rig with the new fields populated by measured values, so that the strengthened record is demonstrated rather than asserted. | High | Open |
| FR-010 | Preserve every existing checkpoint identity | As a reviewer, I want the frozen identity baseline reproduced byte-identically, so that the new evidence remains comparable to the recorded evidence it supersedes. | High | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Real-time discipline preserved | The audio callback performs zero allocations and zero destructions across the whole live run, as measured by the existing per-checkpoint counters. | Reliability | High | Open |
| NFR-002 | Checkpoint identity stability | Comparison against the frozen baseline yields exactly 0 modified and 0 removed identities; every difference is an addition, and the count of additions is recorded. | Compatibility | High | Open |
| NFR-003 | Whole-suite health | All test targets pass with 0 failures; formatting and lint checks exit 0 with warnings denied. | Quality | High | Open |
| NFR-004 | Live-run completeness | The physical run reaches 100% of its declared checkpoints with 0 dropped records, zero false observation keys, clean teardown, and normal parent-process exit. | Reliability | High | Open |
| NFR-005 | Absent distinguished from zero | Every measurement field on the refreshed report shows either a measured value or an explicit absent marker; the count of defaulted zeros is 0. | Observability | High | Open |
| NFR-006 | Falsification evidence completeness | For each new guard, both outcomes are recorded: the mutation with its observed failure, and the restoration with its observed pass. Guards with only one recorded outcome: 0. | Quality | High | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | Add-only checkpoint identity | Existing checkpoint identities stay byte-identical and in order; new identities and fields are pure additions. Inherited from the parent mission's frozen baseline. | Technical | High | Open |
| C-002 | Production path only | All proof runs through the production reducer, projections, and render path; no test-only reducer or shadow projection may satisfy a criterion. | Technical | High | Open |
| C-003 | Serialized vocabulary unchanged | The serialized slot-identity vocabulary and observation key names stay as recorded; the identity derivation changes how a value is produced, never what it is called. | Technical | High | Open |
| C-004 | No new direct injection | The documented rejection remains the only permitted direct semantic-action injection, and it stays documented inline in the scene. | Technical | High | Open |
| C-005 | Observed failure precedes acceptance | No new guard is accepted until its failure has been observed under a deliberate mutation and recorded. This is the mission's governing rule, not a nicety. | Process | High | Open |
| C-006 | Physical evidence is never substituted | Headless output may not stand in for a physical run; if the rig cannot be driven, the mission stops and reports. | Process | High | Open |
| C-007 | No scope growth | Patch switching and any patch-selection gesture are excluded — they are a declared Phase 5 entry condition. This mission adds no product behavior. | Business | High | Open |

### Key Entities

- **Dispatched input kind**: which category of input the production reducer actually received
  for a transition — an adjacent-choice gesture on a focused row, or a direct semantic action.
  Distinct from the transition's declared expected outcome, which already exists.
- **Occupant scalar reading**: the value of the edited effect parameter as projected before
  dispatch and after acceptance, recorded as a pair so their difference is inspectable.
- **Frozen identity baseline**: the recorded sequence of checkpoint identities from the prior
  mission, against which every run's declared and emitted identities are compared.
- **Falsification record**: for one guard, the mutation applied, the observed failure, the
  restoration, and the observed pass.
- **Position-derived slot identity**: an effect's slot identity as determined by the position it
  occupies, rather than as supplied alongside it.

## Domain Language

| Canonical term | Meaning here | Avoid |
|---|---|---|
| dispatched input | what the production reducer received | "the event", "the action" (ambiguous with the declared expectation) |
| declared transition | what the scene planned before dispatch | "the checkpoint" (that is the record of both) |
| falsifiable | its failure has been observed under a deliberate mutation | "tested", "covered", "verified" |
| journey | focus visibly reaching the row, then the adjacent-choice gesture | "navigation", "the flow" |
| add-only | existing identities byte-identical; differences are insertions | "backwards compatible" |

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Removing the on-screen journey from the demo's occupancy changes causes at least
  one declared check to fail. Measured by performing that removal and observing the failure —
  the same mutation that currently passes.
- **SC-002**: Removing the occupant parameter edit, while the chain keeps sounding, causes at
  least one declared check to fail. Measured by performing that removal and observing the
  failure.
- **SC-003**: Comparison of the refreshed run against the frozen baseline yields 0 modified and
  0 removed checkpoint identities, with every addition enumerated.
- **SC-004**: An effect carrying another position's slot identity cannot be stored at a
  position: after any installation, 100% of occupied positions hold the identity derived from
  that position.
- **SC-005**: An acceptance matrix omitting a requirement kind declared in its spec fails
  acceptance, and the failure names the omitted kind.
- **SC-006**: The physical re-run reaches 100% of declared checkpoints with 0 dropped records
  and normal exit, and 0 measurement fields carry a defaulted zero in place of absent evidence.
- **SC-007**: Every new guard has a recorded falsification: guards accepted without an observed
  failure: 0.

## Assumptions

- The physical rig available for the previous mission's run remains available; the refreshed run
  uses the same real window, physical audio device, and real MIDI fixture.
- Only the live effects-and-buses scene is re-run. Other live scenes are unaffected by these
  changes and re-running them is out of scope under locality of change.
- The registry still composes three product effects in the production composition root; the
  fourth entry stays test-only, so the physical window shows exactly three.
- The parent mission's occurrence-map rulings still hold: serialized keys and command names are
  unchanged by this mission.

## Out of Scope

- Patch switching, and any patch-selection semantic gesture (recorded as LIMIT-1; a declared
  Phase 5 entry condition).
- Multi-instrument live demonstration.
- Any change to demonstrated product behavior. If a guard's introduction requires changing what
  the demo does, that is a finding to raise, not a change to make.
- The upstream `spec-kitty retrospect summary` defect recorded as n-008 in the prior mission's
  retrospective — it is not fixable in this repository.
