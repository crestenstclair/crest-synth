---
schema_version: 1
artifact_type: spec-kitty.analysis-report
command: /spec-kitty.analyze
mission_slug: crest-component-foundations-01KZ02H2
mission_id: 01KZ02H2FPKTV50BJP82MB4T5G
generated_at: '2026-08-02T16:50:55.603888+00:00'
analyzer_agent: unknown
input_artifacts:
  spec.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/spec.md
    sha256: 0f77a2e3d3982533100e43d22aabfde6510506162ffe9d417ed81ba91ede0047
  plan.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/plan.md
    sha256: 7edfa99acf5f373f61e9d59bfd937b83b2de00d6838b2f21819ad465caf94d42
  tasks.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/tasks.md
    sha256: 65a832ac84827eeab921d9a8d6127535f2e22e1a62455316d8606ea2979e664b
  charter:
    path: /Users/crestenstclair/workspace/crest-synth/.kittify/charter/charter.md
    sha256: 0b21a43cf5772d1308561d843239947e53247cc7d071c98c920023d23024672b
verdict: ready
issue_counts:
  high: 0
  critical: 0
  medium: 9
  low: 6
  info: 0
findings:
- id: A5
  severity: medium
  category: inconsistency
  summary: plan.md's Project Structure names flat modules (visual_token.rs, authored_typeface.rs, density_policy.rs, component_state.rs, src/shell/primitives/) that no work package creates and that do not exist; every WP and the shipped tree use src/shell/visual/.
- id: A6
  severity: medium
  category: terminology
  summary: '"Witness" names two opposite input contracts — plan.md calls the gallery target a witness and declares witness.component_gallery, while C-005 and WP05 insist it is explicitly not one.'
- id: A7
  severity: medium
  category: inconsistency
  summary: spec.md's assumption still says 11 shared colors were verified against the design file, while NFR-001 and the shipped vocabulary both carry 17 declared colors.
- id: A8
  severity: medium
  category: coverage
  summary: NFR-004 and SC-007 name a 512-event control-path fixture and a 50 ms ceiling, but no subtask names that fixture or records a measured duration against the ceiling.
- id: A10
  severity: medium
  category: inconsistency
  summary: DESIGN.md:575 requires non-color treatment for six states; FR-005 and the crest-spec require eight, and the shipped gallery renders all nine with non-color evidence. The widening is never recorded in the product authority.
- id: A15
  severity: medium
  category: conflict
  summary: witness.component_gallery caps make demo-live-component-library at 180 s, but the scene deliberately has no timeout and waits for a human page walk; no artifact states that the operator protocol must complete inside that budget.
- id: A16
  severity: medium
  category: traceability
  summary: WP06's requirement_refs list NFR-001, NFR-002 and NFR-003 only, while its own subtask T037 proves state exhaustiveness, non-color legibility and page totality — which is NFR-005.
- id: A18
  severity: medium
  category: coverage
  summary: "NFR-005's \"at both authored sizes\" clause is unmeasured: no subtask in WP05 or WP06, and no witness predicate, distinguishes a state painted in one viewport composition from a state painted in both."
- id: A19
  severity: medium
  category: underspecification
  summary: No artifact declares the gallery window's own minimum size. T030 requires both authored compositions inside one window without bounding that window, and the shipped scene's 1920x1080 floor puts its footer band off-screen on a 1920x1080 display.
- id: A9
  severity: low
  category: coverage
  summary: The Requirement coverage table maps FR-001..FR-010 and NFR-001..NFR-006 and omits all seven constraints, including C-004 and C-005, which WP05 is the only package positioned to honour.
- id: A11
  severity: low
  category: ambiguity
  summary: The WindowInput descriptor transition reads "17 -> 33" in plan.md and "21 -> 33" in T023; both are correct against different baselines and neither says which.
- id: A12
  severity: low
  category: inconsistency
  summary: tasks.md asserts every owned_files entry traces to a declared asset file pattern, but the asset patterns are directory globs that cannot discriminate between the packages sharing them.
- id: A13
  severity: low
  category: ambiguity
  summary: 'spec.md still reads Status: Draft after plan, tasks, work packages, lanes, and five implemented work packages were derived from it.'
- id: A14
  severity: low
  category: inconsistency
  summary: The ASCII dependency graph draws one linear chain with an unlabeled return edge; the prose beneath it states the real fan-in correctly.
- id: A17
  severity: low
  category: inconsistency
  summary: plan.md cites "6 new entries in proof/invariants.yaml" with no root, which resolves to no path at the repository root; the entries live under .kittify/crest-spec/proof/.
---

## Specification Analysis Report

Re-run after WP01–WP05 were implemented and WP05 was reviewed. The three HIGH findings from the
2026-08-02T03:02 report were re-tested against current artifacts and against a live run of
`make demo-live-component-library`; all three are discharged (see *Discharged findings* below).
No CRITICAL or HIGH finding remains, so implementation may proceed.

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| A5 | Inconsistency | MEDIUM | plan.md:141-147 vs tasks.md:28 (T001), WP01–WP05 prompts | plan.md's Project Structure declares `src/shell/visual_token.rs`, `authored_typeface.rs`, `density_policy.rs`, `component_state.rs`, and `src/shell/primitives/`. T001 creates `src/shell/visual/`, and the shipped tree is `src/shell/visual/{token,typeface,density,state}.rs` with `src/shell/visual/primitives/`. A reader trusting the plan looks for five files that were never created. | Update plan.md's Project Structure block to the `src/shell/visual/` tree the tasks and code actually use. Documentation-only; no code moves. |
| A6 | Terminology | MEDIUM | plan.md:25,113 · `witnesses.yaml:3130` vs spec.md:64, spec.md:140 (C-005), WP05 | "Witness" carries two opposite contracts. plan.md:25 calls `make demo-live-component-library` "the browsable live gallery witness" and plan.md:113 cites `witness.component_gallery`, while spec.md:64 and C-005 define the gallery as the thing that is *not* a witness because it accepts input and asserts no generation. The word is doing contradictory work in one mission. | Keep `witness.component_gallery` as the crest-spec proof ID, and drop "witness" from prose that describes the *scene*. spec.md's own phrasing — browsable, operator-driven — is the term to standardize on. |
| A7 | Inconsistency | MEDIUM | spec.md:154 vs spec.md:125 (NFR-001), research.md | The assumption block still says "all 11 shared colors ... match exactly", while NFR-001 asserts 17 declared colors and the shipped vocabulary carries 17 roles (I confirmed all 17 reach the screen in the live gallery). The 11 is a pre-union count that survived the union decision recorded two lines below it. | Correct the count in spec.md:154 to the verified shared set and leave the union arithmetic in the assumption immediately following. |
| A8 | Coverage | MEDIUM | spec.md:128 (NFR-004), spec.md:176 (SC-007) · tasks.md:240 · WP04 T025 | NFR-004 names a specific, falsifiable ceiling — the 512-event control-path fixture within 50 ms — and SC-007 promises no audible change. tasks.md assigns NFR-004 to WP04, whose only related subtask is T025 "Confirm `make run` changed and existing shell tests still pass". No subtask names the fixture or records a measured duration, so the ceiling is asserted rather than measured. This is the one requirement in the mission with a number attached and no measurement behind it. | Add a subtask (WP04 or WP06) that runs the control-path fixture and records the measured duration against the 50 ms ceiling, the way `witness.component_gallery` records its predicates. |
| A10 | Inconsistency | MEDIUM | DESIGN.md:575 vs spec.md:114 (FR-005) · `requirements.yaml` `explicit_state_rendering` · WP01 T005 | DESIGN.md:575 reads "Focus, mute, solo, loading, error, and selection always have text or shape in addition to color" — six states. FR-005 and the crest-spec require eight, adding adjustment and disabled, and the shipped gallery renders all nine with non-color evidence (measured: `Adjusting` → keyline 3 px + cursor, `Disabled` → keyline 1 px + `Locked` mark). DESIGN.md is the product authority and T005 recorded three durable decisions; this widening is not one of them. | Record the widened state list as a fourth durable decision and update DESIGN.md:575. Charter posture makes code/DESIGN.md fidelity the top gate, so this should not wait for the follow-on mission. |
| A15 | Conflict | MEDIUM | `witnesses.yaml:3130` (`timeout: 180s`) vs WP05 T032 step 3 · plan.md:113 | `witness.component_gallery` runs `make demo-live-component-library` under a 180 s cap. The scene deliberately has no milestone and no total timeout because it waits for the operator. Those coexist only if a human completes an eight-page walk, an unbound-digit press, and a window close inside three minutes — which no artifact states. My own run satisfied all fifteen predicates, so the budget is achievable, but it is undeclared. *(Downgraded from HIGH: T031 and T032 now define the emission point, the accumulation across the session, and the operator protocol, which is what the prior finding said was missing.)* | State the operator budget in WP05's review guidance or in quickstart.md, or raise the witness timeout to a value that matches an unhurried human page walk. |
| A16 | Traceability | MEDIUM | WP06 frontmatter `requirement_refs` vs tasks.md:64 (T037), tasks.md:242 | WP06's `requirement_refs` are NFR-001, NFR-002, NFR-003. Its own subtask T037 is "Prove state exhaustiveness, non-color legibility, page totality" — that is NFR-005 verbatim, and the coverage table credits NFR-005 to WP05 alone. WP06 therefore proves a requirement it does not claim, and the acceptance matrix cannot see it. | Add NFR-005 to WP06's `requirement_refs` and to its coverage row. Resolves alongside A18. |
| A18 | Coverage | MEDIUM | spec.md:129 (NFR-005), spec.md:172 (SC-003) · WP05 T029/T030 · `witnesses.yaml` predicates | NFR-005 requires every state to have a specimen **at both authored sizes**, and SC-003 repeats it. Nothing measures the conjunction. The witness carries `states_painted` (a single count), `desktop_viewport_painted` and `steam_deck_viewport_painted` (each true if that column emitted any text at all) — so nine states painted in one column and none in the other satisfies every declared predicate. The shipped ledger mirrors this: `bands_painted` is indexed per policy, `states_painted` is flat. Behaviour is correct today (I confirmed both columns paint all nine labelled states), but the clause is asserted, not measured — which C-006 forbids. | Index state coverage per density policy, the way band coverage already is, and let the observation's `states_painted` mean "painted at every declared policy". Carried in the WP05 review feedback as issue 2. |
| A19 | Underspecification | MEDIUM | WP05 T030 · spec.md:97 (edge case) · plan.md IC-07 | spec.md's window-size edge case bounds the *product* shell at the compact viewport. Nothing bounds the *gallery* window, yet T030 requires both authored compositions inside one window — which forces a minimum larger than either. The shipped scene resolves this by pinning its floor at 1920×1080; on a 1920×1080 display that pushes the 64 px footer band, which carries the only on-screen browsing affordance, below the screen edge, and the pinned minimum removes the operator's remedy. The paint pass reports zero clipped text because it measures against the egui surface, not the display. | Add an explicit minimum-window statement for the gallery that accounts for window chrome and available work area, and a measurement that would fail when the composed frame cannot fit. Carried in the WP05 review feedback as issue 1. |
| A9 | Coverage | LOW | tasks.md:235-243 | The Requirement coverage table maps all ten FRs and all six NFRs and omits all seven constraints. C-004 (two top-level contexts, scene-local paging) and C-005 (one-way input isolation) are the two the gallery could most easily violate, and no package formally owns them — they survive on prose in the WP05 prompt. | Add a constraint column or a second table so C-001..C-007 name an owning package. |
| A11 | Ambiguity | LOW | plan.md:83 vs tasks.md:50 (T023) · spec.md:22 | The descriptor transition reads "17 → 33" in plan.md and "21 → 33" in T023. Both are right — 17 is the declared count the crest-spec carried, 21 is what the code held — but neither says which baseline it uses. | State the baseline once: "declared 17, actual 21, corrected to 33." |
| A12 | Inconsistency | LOW | tasks.md:7-8, 12-20 | tasks.md asserts "Every `owned_files` entry traces to a declared asset file pattern", but three of the listed patterns are the same `src/*/*` glob shared by WP01–WP05, so the trace does not discriminate. The claim is stronger than the table supports. | Soften the claim, or narrow the asset patterns so each package's ownership is derivable from them. |
| A13 | Ambiguity | LOW | spec.md:5 | The header still reads `**Status**: Draft` after plan, tasks, six work packages, lanes, and five implemented packages were derived from it. | Set it to the project's post-tasks status value. |
| A14 | Inconsistency | LOW | tasks.md:215-220 | The ASCII graph draws one linear chain plus an unlabeled return edge; the prose immediately beneath states the real fan-in (WP04 on WP01+WP02+WP03, WP05 on WP03+WP04) correctly. The picture is the part that is wrong. | Redraw as a DAG or delete it and keep the prose. |
| A17 | Inconsistency | LOW | plan.md:116 | The proof table cites "6 new entries in `proof/invariants.yaml`". No `proof/` exists at the repository root; the entries live under `.kittify/crest-spec/proof/`. | Qualify the path. |

### Discharged findings

| Prior ID | Prior severity | Disposition |
|---|---|---|
| A3 | HIGH | **Resolved.** spec.md:53-54 now require a non-color indication for focus *and* adjustment, FR-005 names all eight, and Disabled has gallery specimens on pages 5 and 6. Measured live: `states_distinguishable_without_color: true`, with `Adjusting` → `keyline 3 px · cursor >` and `Disabled` → `keyline 1 px · mark Locked`. The residual DESIGN.md half of this finding is carried forward as A10. |
| A4 | HIGH | **Resolved.** WP04's T024 left `Digit3`–`Digit8` unbound in the translator, and the gallery normalizes `Num9`/`Num0` to `WindowKey::Other` while tracking digit-ness scene-locally, so US2 AS-2 and the `unbound_digit_retained_page` predicate are reachable. Measured live: `unbound_key_presses: 1`, `unbound_digit_retained_page: true`. |
| A15 | HIGH | **Downgraded to MEDIUM**, retained above. T031 steps 1-4 and T032 now define the emission point, the accumulation across the session, and the operator protocol — the three things the prior finding said no subtask defined. Only the undeclared 180 s operator budget remains. |

**Coverage Summary Table:**

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|
| FR-001 single-semantic-visual-vocabulary | Yes | T001–T003, T006 | |
| FR-002 azeret-mono-installed-and-mapped | Yes | T004, T021 | |
| FR-003 declared-viewport-density-policies | Yes | T007–T009, T022 | |
| FR-004 reusable-primitives | Yes | T013–T018 | |
| FR-005 explicit-state-rendering | Yes | T010, T011, T017 | Widening beyond DESIGN.md unrecorded — A10 |
| FR-006 production-shell-renders-through-vocabulary | Yes | T020–T022, T025 | |
| FR-007 live-gallery-demo-scene-and-launch-target | Yes | T026, T028–T032 | |
| FR-008 number-key-page-selection | Yes | T027, T023, T024 | |
| FR-009 components-own-no-application-state | Yes | T019 | |
| FR-010 typed-failure-when-typeface-unavailable | Yes | T004, T038 | |
| NFR-001 exact-authored-value-fidelity | Yes | T006, T034 | |
| NFR-002 no-visual-literals-outside-vocabulary | Yes | T035 | |
| NFR-003 both-authored-viewports-render-intact | Yes | T012, T036 | |
| NFR-004 no-real-time-or-control-path-regression | Partial | T025 | No subtask names the 512-event fixture or the 50 ms ceiling — A8 |
| NFR-005 complete-gallery-state-coverage | Partial | T029, T030, T037 | "At both authored sizes" unmeasured — A18; T037 uncredited — A16 |
| NFR-006 vendored-typeface-provenance | Yes | T004 | |
| C-001 … C-007 | Unmapped | — | No owning package in the coverage table — A9 |

**Charter Alignment Issues:** None. The charter's stated priorities are code/crest-spec/DESIGN.md
fidelity and measured, falsifiable proof. A10 (DESIGN.md narrower than shipped behaviour), A18
(an NFR clause asserted rather than measured) and A8 (a numeric ceiling with no measurement) all
press on those priorities, but none violates a MUST principle, and each has a named remedy inside
the current mission.

**Unmapped Tasks:** None. All 38 subtasks map to at least one requirement.

**Metrics:**

- Total Requirements: 16 (10 FR + 6 NFR), plus 7 constraints
- Total Tasks: 38 subtasks across 6 work packages
- Coverage: 16/16 FR+NFR have at least one subtask (100%); 2 of those are partial (NFR-004, NFR-005); 0/7 constraints appear in the coverage table
- Ambiguity Count: 2
- Duplication Count: 0
- Critical Issues Count: 0

## Next Actions

No CRITICAL or HIGH findings — implementation may proceed.

- A18 and A19 are already written up as issues 2 and 1 of the WP05 review feedback at
  `kitty-specs/crest-component-foundations-01KZ02H2/tasks/WP05-browsable-gallery-scene/review-cycle-1.md`,
  so they are fixed in code during the WP05 rework rather than by editing planning artifacts.
- A16 wants one line in WP06's frontmatter (`requirement_refs`) and one cell in tasks.md:242 before
  WP06 starts, so the acceptance matrix credits the proof it actually carries.
- A10 wants a `DESIGN.md` edit recording the widened state list. WP01 owns `DESIGN.md` and is already
  approved, so this needs either a WP01 follow-up or an explicit deferral.
- A5, A6, A7, A9, A11, A12, A13, A14, A17 are documentation corrections in spec.md and plan.md with no
  code consequence; batch them rather than interrupting the WP05 rework.
