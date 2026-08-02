---
schema_version: 1
artifact_type: spec-kitty.analysis-report
command: /spec-kitty.analyze
mission_slug: falsifiable-journey-proof-01KYZTQ1
mission_id: 01KYZTQ118MXZGD4MBCR99A978
generated_at: '2026-08-02T00:22:57.475015+00:00'
analyzer_agent: unknown
input_artifacts:
  spec.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/falsifiable-journey-proof-01KYZTQ1/spec.md
    sha256: 0ec3346534a62a38d24a0832f1eda7b69a7a213a2ab72c827ed29cf7fae17270
  plan.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/falsifiable-journey-proof-01KYZTQ1/plan.md
    sha256: bac0fb7547e42a1b14bd82d37b693298e318c1943c12185ac11a2f05b1f8240c
  tasks.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/falsifiable-journey-proof-01KYZTQ1/tasks.md
    sha256: f576c2d6f703331791d6c5d1a8f9b649c84e46180a3e1529e5f50b07bed65d93
  charter:
    path: /Users/crestenstclair/workspace/crest-synth/.kittify/charter/charter.md
    sha256: 0b21a43cf5772d1308561d843239947e53247cc7d071c98c920023d23024672b
verdict: blocked
issue_counts:
  critical: 1
  medium: 3
  high: 1
  low: 2
  info: 0
findings:
- id: C1
  severity: critical
  category: charter-alignment
  summary: The mission's governing rule (C-005 falsification discipline, NFR-006) is enforced only by prose and reviewer diligence, with no proof-enforced invariant — the charter requires the opposite for a constraint whose class has already failed once.
- id: G1
  severity: high
  category: coverage
  summary: Work packages map only functional requirements; all 6 NFRs and all 7 constraints have zero requirement_refs, though the tooling accepts them — the same omit-a-whole-kind defect this mission builds a gate against.
- id: G2
  severity: medium
  category: coverage
  summary: All 7 success criteria are declared in spec.md and referenced by no task file.
- id: S1
  severity: medium
  category: sequencing
  summary: WP04's coverage gate becomes a project completion check the moment it lands, and fails until WP06 authors this mission's acceptance matrix.
- id: S2
  severity: medium
  category: ownership
  summary: Three falsification subtasks temporarily mutate files owned by other work packages; restoration is enforced only by prose instruction.
- id: T1
  severity: low
  category: inconsistency
  summary: The dispatched-input concept is named four different ways across the crest-spec, spec, plan, and task prompts.
- id: P1
  severity: low
  category: process
  summary: This analysis pass briefly mutated WP01's requirement_refs while probing tool behavior; reverted, but two extra commits remain in history.
---

## Specification Analysis Report

**Mission**: `falsifiable-journey-proof-01KYZTQ1`
**Artifacts**: `spec.md`, `plan.md`, `tasks.md` + 6 WP prompts, crest-spec commit `ad9960b`
**Analyzed**: 2026-08-02

All three artifacts were authored in this session by the same agent that is now analyzing them.
That is a conflict of interest, and the findings below are weighted accordingly: the two most
severe are defects in *this analyst's own work*, and both are instances of the exact class the
mission was chartered to close.

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| C1 | Charter Alignment | CRITICAL | spec.md C-005, NFR-006; WP02 T010-T011, WP03 T015, WP04 T018; WP06 T023 | The mission's governing rule — no guard accepted until its failure is observed — is enforced by prose instruction and a reviewer spot-check. Nothing validates that a falsification record exists, is well-formed, or reports a non-zero exit. NFR-006 states a measurable threshold ("guards with only one recorded outcome: 0") with no measuring instrument. | Add a declared validation that scans `evidence/falsification/` for one well-formed record per new guard, each showing an observed non-zero exit and a subsequent zero. Wire it into `completion.projectChecks` beside the coverage gate. This is a crest-spec change, so it returns to `/spec-kitty.crest-spec` before implementation. |
| G1 | Coverage | HIGH | tasks.md Subtask Index; all 6 WP frontmatter `requirement_refs` | 10 FRs are mapped; 6 NFRs and 7 constraints are mapped nowhere. `map-requirements` accepts NFR refs (verified empirically this pass), so this is an omission, not a tool limitation. NFR/C compliance currently lives only in per-WP "Definition of Done" prose. | Extend `requirement_refs` to the NFRs and constraints each WP actually bears — e.g. NFR-001/C-002 to WP01, NFR-002/C-001 to WP01+WP06, NFR-006/C-005 to WP02+WP03+WP04, NFR-004/C-006 to WP06, C-003 to WP03. |
| G2 | Coverage | MEDIUM | spec.md SC-001..SC-007; tasks.md | Seven success criteria, zero references in any task file. SC-001 and SC-002 in particular *are* WP02's falsification subtasks, but nothing states the correspondence, so a reviewer cannot confirm SC coverage from the task artifacts. | Cite the SC each subtask satisfies in the WP prompts, or add an SC column to the Subtask Index. Cheapest fix in this report. |
| S1 | Sequencing | MEDIUM | WP04 T016; `.kittify/crest-spec/project.yaml:129` | `acceptance_matrix_covers_all_requirement_kinds` is already wired into `completion.projectChecks` (29/29). Once WP04 lands, the check fails for this mission until WP06 T026 authors the matrix. WP04's own prompt calls this "correct behavior" — accurate for end-of-mission acceptance, but it means the repo carries a failing completion check for the whole implementation window. | Confirm project checks run only at `spec-kitty accept`, not per-WP. If any per-WP gate evaluates them, WP05 and WP06 become unstartable and WP04 must be resequenced after WP06 — which would invert the dependency graph. Verify before implementation starts. |
| S2 | Ownership | MEDIUM | WP02 T010-T011, WP03 T015, WP04 T018 | The falsification subtasks deliberately mutate files owned by other WPs (`live_demo_runner.rs`, `live_effects_and_buses_scene.rs`, `patch.rs`) and restore them. This is correct in intent — a falsification that touches nothing proves nothing — but an interrupted or abandoned WP leaves a mutation in a lane, and only a prose "confirm `git status` clean" prevents it landing. | Have each falsification subtask assert a clean tree as its final step and fail if not. Better: fold this into C1's validation, which can check the same property mechanically. |
| T1 | Inconsistency | LOW | crest-spec `dispatchedInputKind`; spec.md "dispatched input kind"; plan.md "dispatched input kind"; WP prompts "recorded dispatched kind" / "recorded kind" | Four surface forms for one concept. Harmless today because the crest-spec name is canonical and typed, but the WP prompts' "recorded kind" shorthand could read as a distinct field to an implementer who has not read the crest-spec. | Use "dispatched input kind" verbatim in prompts, reserving "recorded" as an adjective. No artifact change required before implementation. |
| P1 | Process | LOW | WP01 frontmatter; commits `f259ef6`, `41733bd` | To establish whether G1 was an omission or a tool limitation, this pass ran `map-requirements --wp WP01 --refs NFR-005`, which committed. It was reverted with `--replace` and WP01's refs are back to `FR-001, FR-004`, but two commits remain in history. `/spec-kitty.analyze` is declared NON-REMEDIATING and should not have mutated a WP file. | No action needed on content — state is correct. Recorded because the constraint was breached, not because the outcome is wrong. A read-only means of testing ref acceptance would have avoided it. |

### Coverage Summary

| Requirement Key | Has Task? | Task IDs | Notes |
|---|---|---|---|
| FR-001 record dispatched input kind | Yes | T001, T002, T019 | WP01, WP05 |
| FR-002 assert journey over record | Yes | T007 | WP02 |
| FR-003 identify permitted injection by record | Yes | T008 | WP02 |
| FR-004 record occupant scalar before/after | Yes | T003, T004, T019 | WP01, WP05 |
| FR-005 require the scalar to have changed | Yes | T009 | WP02 |
| FR-006 falsification-test every new guard | Yes | T010, T011, T015, T018 | See C1 — tasks exist, enforcement does not |
| FR-007 derive slot identity from position | Yes | T012, T013, T014 | WP03 |
| FR-008 gate acceptance-matrix coverage | Yes | T016, T017 | WP04 |
| FR-009 refresh physical evidence | Yes | T023, T024 | WP06 |
| FR-010 preserve checkpoint identity | Yes | T025 | WP06 |
| NFR-001..006 | Prose only | — | G1: addressed in WP Definitions of Done, mapped in no `requirement_refs` |
| C-001..007 | Prose only | — | G1: same |
| SC-001..007 | No | — | G2: referenced nowhere |

### Charter Alignment Issues

One, and it is the report's most severe finding.

`.kittify/charter/charter.md:34-36`:

> "Prose constraints that failed once are replaced by proof-enforced invariants (see the
> no-name-enumeration project check)."

C-005 is precisely a prose constraint whose class has already failed once — accepting guards
whose failure was never observed is why this mission exists. The mission's answer to that failure
is another prose constraint plus a reviewer instruction to "spot-check at least one." The
charter's own remedy pattern, named in the same sentence, is a project check.

The crest-spec states the same rule from the other direction: *"Do not replace measured proof
with self-reported success text."* Four falsification records written by the agent that performed
the mutations, validated by nothing, are self-reported success text.

The operator chose the committed-artifact form over WP-notes precisely so the evidence would be
"independently checkable." Nothing was tasked to check it. That gap is what C1 records.

### Unmapped Tasks

None. All 27 subtasks map to at least one requirement.

### Metrics

- Total requirements: **30** (10 FR, 6 NFR, 7 C, 7 SC)
- Total tasks: **27** subtasks across **6** work packages
- Functional coverage: **100%** (10/10 FR mapped)
- Full-kind coverage: **33%** (10/30 — NFR, C, and SC unmapped)
- Ambiguity count: **0** (no TODO/TKTK/placeholder markers; all NFRs carry numeric thresholds)
- Duplication count: **0**
- Critical issues: **1**

### Next Actions

**Verdict: blocked.** One CRITICAL and one HIGH finding.

1. **C1 — required before implementation.** Declare a falsification-record validation in the
   crest-spec and wire it into `completion.projectChecks`, then add its implementation to WP04
   (which already owns `scripts/` and the guard-script pattern). Returns to
   `/spec-kitty.crest-spec`, then re-run `/spec-kitty.tasks` for WP04 and re-run this analysis.
   Estimated: 30-40 minutes.
2. **G1 — fix with the CLI, no re-planning needed.** One `map-requirements --batch` call adding
   NFR and constraint refs to each WP. Estimated: 5 minutes.
3. **G2 — cite success criteria in the WP prompts.** Estimated: 10 minutes.
4. **S1 — verify before starting.** Confirm project completion checks run at `accept`, not
   per-WP. If they run per-WP, the dependency graph needs inverting. Estimated: 5 minutes.
5. **S2 and T1 — fold into the C1 work.** No separate pass needed.

Proceeding to `/spec-kitty.implement` without C1 would mean implementing a mission about proof
adequacy whose own central proof is unenforced — the third occurrence of this class in this
mission line, and the first one that was visible in advance.
