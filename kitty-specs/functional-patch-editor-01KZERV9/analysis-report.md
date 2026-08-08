---
schema_version: 1
artifact_type: spec-kitty.analysis-report
command: /spec-kitty.analyze
mission_slug: functional-patch-editor-01KZERV9
mission_id: 01KZERV946GWEEMVZZM43EJ437
generated_at: '2026-08-08T23:10:01.521018+00:00'
analyzer_agent: unknown
input_artifacts:
  spec.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/functional-patch-editor-01KZERV9/spec.md
    sha256: e702e3a063629f5a4fbaf3fe8c5c9fb2047f8da2c0d0bfa65d6f7d71ac07d7fc
  plan.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/functional-patch-editor-01KZERV9/plan.md
    sha256: b0638641e101b4c007ac5575ddc0fbfc9f8a1789896a221a23dea7a6be4a160e
  tasks.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/functional-patch-editor-01KZERV9/tasks.md
    sha256: c5660700b90b827484dad6d1807f6776f29aaaac9daef9dd3e03d79b931776ef
  charter:
    path: /Users/crestenstclair/workspace/crest-synth/.kittify/charter/charter.md
    sha256: 0b21a43cf5772d1308561d843239947e53247cc7d071c98c920023d23024672b
verdict: ready
issue_counts:
  high: 0
  low: 4
  critical: 0
  medium: 3
  info: 0
findings:
- id: C1
  severity: medium
  category: coverage
  summary: FR-012's reducer and projection halves are built in WP02 and WP03, but neither lists FR-012 in requirement_refs; only WP04 and WP05 claim it.
- id: C2
  severity: medium
  category: coverage
  summary: SC-003 claims zero rows on the shipped PATCH surface are marked unavailable, but only the five Utility rows are asserted; no task checks the main strip.
- id: C3
  severity: medium
  category: coverage
  summary: The spec's Scope Decisions table claims FR-012 closes the read-only surface-summary control kind, but no subtask asserts a production path produces one.
- id: I1
  severity: low
  category: inconsistency
  summary: plan.md IC-07 omits ROADMAP.md from its affected surfaces, but WP06 owns and edits it (T041).
- id: I2
  severity: low
  category: inconsistency
  summary: plan.md IC-08 declares no dependencies; tasks.md folds it into WP03, which depends on WP02, so its independence is lost without the plan saying so.
- id: S1
  severity: low
  category: sizing
  summary: WP04 carries 9 subtasks against the 3-7 target; the coupling rationale is documented but the package is at the upper bound.
- id: V1
  severity: low
  category: coverage
  summary: C-002 and C-003 are scope-negative constraints with no verification subtask; nothing fails if a modal or a MIXER-side change is introduced.
---

## Specification Analysis Report

**Mission**: functional-patch-editor-01KZERV9
**Artifacts**: spec.md (`d9bf2fe`), plan.md (`03a553d`), tasks.md + 6 WPs (`670cd3c`)
**Crest-spec**: authored at `c6e8290`; `doctor` reports the model closed.

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| C1 | Coverage | MEDIUM | tasks/WP02 (T010–T011), tasks/WP03 (T015), spec.md FR-012 | The detail shell's reducer half (`PatchDetailSubject`, `SurfaceId::PatchDetail`, `InteractionState.detailSubject`) is built in WP02 and its projection half in WP03, but neither WP lists FR-012 in `requirement_refs`. A reviewer checking WP02 against its stated requirements would not check the detail surface at all. | Add FR-012 to WP02 and WP03 `requirement_refs` via `spec-kitty agent tasks map-requirements`. No prompt text needs to change — the work is already described. |
| C2 | Coverage | MEDIUM | spec.md SC-003, tasks/WP05 (T031) | SC-003 states "Zero rows on the shipped PATCH surface are marked unavailable for want of data." T031 asserts none of the five Utility rows is unavailable, but nothing asserts it across the main strip — where the header, selector, envelope group, and slot groups also render. The success criterion is broader than its proof. | Extend WP05 T030 or T031 with a whole-surface assertion: project the fixture, walk every PATCH surface, and assert the unavailable-mark count is zero except where the Scope Decisions table deliberately defers. |
| C3 | Coverage | MEDIUM | spec.md Scope Decisions table, tasks/WP05 (T033) | The Scope Decisions table lists "Control kind: read-only surface summary has no production path" as supplied by FR-012 via capability-declared read-only detail sections. WP01 T001 explicitly asserts the `Stepped` kind now has a producer; the read-only surface summary has no equivalent assertion anywhere. One of the two control-kind closures is claimed but unproven. | Add a validation bullet to WP05 T033 asserting a production projection path produces a read-only surface summary, mirroring T001's `Stepped` assertion. Or withdraw the claim from the Scope Decisions table and defer it. |
| I1 | Inconsistency | LOW | plan.md IC-07, tasks/WP06 frontmatter | IC-07's "Affected surfaces" lists `src/testing/`, `src/bin/crest_synth.rs`, and `Makefile`. WP06's `owned_files` also includes `ROADMAP.md`, and T041 edits it. plan.md's asset table does name `asset.DeliveryRoadmap`, so the intent is recorded — just not in the concern that carries it. | Note the addition when WP06 completes, or amend IC-07's surfaces list. Not blocking: ownership is unambiguous and non-overlapping. |
| I2 | Inconsistency | LOW | plan.md IC-08, tasks/WP03 (T018–T019) | IC-08 declares "Sequencing/depends-on: none" — the `retire()` guard proof is genuinely independent. Packaging it into WP03 makes it wait on WP02. The choice is defensible (same projection domain, one lane instead of a two-subtask package below the minimum size), but plan.md still reads as if it could start immediately. | Accept the packaging; the alternative is a 2-subtask WP below the stated minimum. Worth one line in the WP03 completion note so the sequencing change is visible. |
| S1 | Sizing | LOW | tasks/WP04 | Nine subtasks against the stated 3–7 target and 10 maximum. The rationale is recorded in the WP itself: `page.js` and `page.css` are one coupled surface, and splitting them across packages produces a merge conflict rather than parallelism. | No action. Within the hard limit, with the reason stated where an implementer will read it. |
| V1 | Coverage | LOW | spec.md C-002, C-003 | Both are scope-negative constraints — "no choice modals, no asset surfaces" and "MIXER-side carry-forwards stay deferred". Nothing in the task set fails if a modal or a MIXER Inspector change is introduced. | Constraints of this kind are normally enforced at review rather than by a test. The pre-merge review and the mission review both read scope; leave as is unless a WP starts drifting. |

**Coverage Summary Table:**

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|
| FR-001 identity and routing header | yes | T021, T030 | |
| FR-002 on-screen patch navigation | yes | T012, T029, T036 | |
| FR-003 instrument selection from the strip | yes | T020, T026 | |
| FR-004 ordered effect slots | yes | T023, T036 | |
| FR-005 visible ADSR group | yes | T022 | |
| FR-006 persistent Utility panel | yes | T008, T027, T031 | |
| FR-007 master volume on PATCH Utility | yes | T009, T031 | single-owner assertion is structural, not just two reads |
| FR-008 MIDI input on PATCH Utility | yes | T009, T031 | |
| FR-009 voice limit | yes | T001–T006, T034 | falsification required at T006 and T034 |
| FR-010 requested value mid-edit | yes | T014, T026, T032 | |
| FR-011 per-row action hints | yes | T013, T025, T032 | |
| FR-012 polymorphic detail shell | yes | T010, T011, T015, T028, T033 | **C1** — WP02/WP03 do not reference it |
| FR-013 painted ranges and units | yes | T024, T032 | |
| FR-014 authored labels, not keys | yes | T016, T031 | guard is a key-set comparison, not a single-string check |
| FR-015 Utility hint line | yes | T027 | |
| FR-016 `make demo-live-patch-editor` | yes | T035–T040 | mission exit gate |
| NFR-001 callback contract under limiting | yes | T004, T006, T034 | |
| NFR-002 live scene completion bar | yes | T038, T040 | |
| NFR-003 both authored viewports | yes | T027, T028 | |
| NFR-004 projection throughput | yes | T035, T037 | |
| NFR-005 patch switch is one generation | yes | T012, T017, T029 | delta asserted as exactly 1 |
| C-001 one-way loop only | partial | T009, T014, T036 | asserted where it bites; no global guard |
| C-002 no modals, no asset surfaces | no | — | **V1** — scope-negative, review-enforced |
| C-003 MIXER carry-forwards deferred | no | — | **V1** — scope-negative, review-enforced |
| C-004 crest-spec first | yes | — | satisfied at `c6e8290`, before plan.md |
| C-005 gated live suite needs the display | yes | T040 risk table | harness refuses; correct behaviour |
| C-006 no placeholder values | yes | T020, T027, T030 | **C2** — Utility asserted, main strip not |

**Charter Alignment Issues:** none. The six binding directives are addressed in plan.md's Charter Check, and DIRECTIVE_025's fold-in decision is recorded with reasons for the four follow-ups left filed.

**Unmapped Tasks:** none. All 41 subtasks belong to exactly one work package, and every work package traces to declared crest-spec assets.

**Metrics:**

- Total requirements: 27 (16 FR, 5 NFR, 6 C)
- Total subtasks: 41 across 6 work packages
- Coverage: 25/27 with ≥1 subtask (93%). The two uncovered are scope-negative constraints (V1).
- Functional-requirement coverage: 16/16 (100%)
- Ambiguity count: 0 — no `[NEEDS CLARIFICATION]` marker, no unresolved placeholder in any artifact
- Duplication count: 0
- Critical issues: 0
- High issues: 0

**Work package sizing:** WP01 6, WP02 6, WP03 7, WP04 9, WP05 6, WP06 7. One package (WP04) is above the 3–7 target and below the 10 maximum, with its rationale recorded in the package.

**Falsification discipline:** four subtasks require an observed failure rather than a described one — T006 (voice limit defeated), T019 (`retire()` guard removed), T030 (flat-run negative), T034 (limit defeated in acceptance) — plus the mission-level controlled negative at T039. This is the strongest signal in the task set and reviewers should check the recorded failure text, not the checkbox.

## Next Actions

Verdict is **ready**: no critical or high findings, so the implement gate is satisfied.

Three MEDIUM findings are worth closing first because each is a few minutes of
bookkeeping and each removes a real blind spot at review time:

1. `spec-kitty agent tasks map-requirements --wp WP02 --refs FR-012` and the same
   for WP03 — closes C1.
2. Add the whole-surface unavailable-count assertion to WP05 T030/T031 — closes C2.
3. Add the read-only surface-summary assertion to WP05 T033, or withdraw that
   claim from the spec's Scope Decisions table — closes C3.

The four LOW findings need no action before implementation. I1 and I2 are
plan-versus-tasks drift that the mission review will see anyway; S1 is a
documented deliberate choice; V1 is the normal shape of a scope constraint.
