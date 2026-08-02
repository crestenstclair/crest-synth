---
schema_version: 1
artifact_type: spec-kitty.analysis-report
command: /spec-kitty.analyze
mission_slug: crest-component-foundations-01KZ02H2
mission_id: 01KZ02H2FPKTV50BJP82MB4T5G
generated_at: '2026-08-02T03:02:27.610098+00:00'
analyzer_agent: unknown
input_artifacts:
  spec.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/spec.md
    sha256: dc339b523f792752dca63e774acb2180d990272f28fe74554efe31fcfd9f8cfb
  plan.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/plan.md
    sha256: db3a1d020484cfa4024339d288311dfbb0ad9fb04e6b22d630c6793fe5b792f8
  tasks.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/tasks.md
    sha256: 478ef0f11ce590d9b3fb2c5220882202c28c2809f33e7925bbadc51d08c90636
  charter:
    path: /Users/crestenstclair/workspace/crest-synth/.kittify/charter/charter.md
    sha256: 0b21a43cf5772d1308561d843239947e53247cc7d071c98c920023d23024672b
verdict: blocked
issue_counts:
  critical: 0
  high: 3
  medium: 7
  low: 5
  info: 0
findings:
- id: A3
  severity: high
  category: coverage
  summary: Adjusting and Disabled are specified color-only, contradicting FR-005, US1 AS-3, SC-005 and crest-spec requirement.explicit_state_rendering; Disabled has no gallery specimen at all.
- id: A4
  severity: high
  category: underspecification
  summary: US2 AS-2, its edge case, quickstart, and witness predicate unbound_digit_retained_page all rest on an unreachable branch — all eight normalized digits are bound to the eight declared pages.
- id: A15
  severity: high
  category: conflict
  summary: witness.component_gallery declares an automated 180 s command with predicates pages_painted=8, states_painted=9 and window_closed=true, while WP05 removes all timeouts and emits the observation after a single paint; no subtask defines the emission point, accumulation, or operator protocol.
- id: A5
  severity: medium
  category: inconsistency
  summary: plan.md Project Structure names flat modules (visual_token.rs, density_policy.rs, component_state.rs) that no WP creates; every WP and T001 use the src/shell/visual/ tree.
- id: A6
  severity: medium
  category: terminology
  summary: '"Witness" names two opposite input contracts — plan.md calls the gallery target a witness and declares witness.component_gallery, while C-005, WP05 and asset.BuildMakefile insist it is explicitly not one.'
- id: A7
  severity: medium
  category: inconsistency
  summary: spec.md's assumption still says 11 shared colors verified against the design file; research.md's corrected 13-published / 16-in-DESIGN.md tally makes the shared set 12 and the union 17.
- id: A8
  severity: medium
  category: coverage
  summary: NFR-004 and SC-007 have no subtask naming tests/control_dispatch_performance.rs or recording its measured duration against the 50 ms ceiling; WP06, the measured-proof package, omits NFR-004 entirely.
- id: A9
  severity: medium
  category: coverage
  summary: The requirement-coverage table maps only FR and NFR items; none of the seven constraints C-001..C-007 appear, including the load-bearing C-004 and C-006.
- id: A10
  severity: medium
  category: inconsistency
  summary: DESIGN.md requires non-color treatment for six states; FR-005 and the crest-spec require eight, and this widening is not among the three durable decisions T005 records.
- id: A16
  severity: medium
  category: traceability
  summary: WP06 frontmatter requirement_refs lists only NFR-001..003 while its subtasks explicitly prove NFR-005, FR-005, FR-010 and C-006, and tasks.md assigns those to other packages.
- id: A11
  severity: low
  category: ambiguity
  summary: The descriptor transition reads 17 -> 33 in plan.md and 21 -> 33 in T023 without either stating whether it means the declared or the constructed count.
- id: A12
  severity: low
  category: inconsistency
  summary: tasks.md claims every owned_files entry traces to a declared asset file pattern, but no asset in assets.yaml carries a files key and three assets share the non-discriminating pattern src/*/*.
- id: A13
  severity: low
  category: ambiguity
  summary: 'spec.md still carries Status: Draft after plan, tasks, work packages and lanes were derived from it.'
- id: A14
  severity: low
  category: inconsistency
  summary: The dependency-graph diagram in tasks.md does not express the WP04 and WP05 fan-in that the prose beneath it states.
- id: A17
  severity: low
  category: inconsistency
  summary: plan.md cites proof/invariants.yaml, but no proof/ directory exists at the repository root; the six entries landed in .kittify/crest-spec/proof/invariants.yaml.
---

## Specification Analysis Report

**Mission**: `crest-component-foundations-01KZ02H2`
**Artifacts**: spec.md (176 L), plan.md (246 L), tasks.md (242 L), 6 WP prompts, research.md, quickstart.md, `.kittify/crest-spec/`, `.kittify/charter/charter.md`

**Resolved since the previous report**: A1 (the 13-vs-17 color count is now 17 at every site including `shell.yaml:84` and `capabilities.yaml:692`) and A2 (C-002 now defers only configurable controls and compositions, keeping `make demo-live-component-library` in scope). Finding IDs are carried forward unchanged for the items that remain, so this report can be diffed against the last one.

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| A3 | Coverage | HIGH | spec.md:53 (US1 AS-3), :114 (FR-005), :174 (SC-005) · `requirements.yaml` `explicit_state_rendering` · WP03:154,159,179,210 · WP05:142,191 · WP06:207-209 | FR-005 and the crest-spec require **adjustment** and **disabled** to render with text or shape in addition to color. Neither gets one. T015 paints a 3 px keyline for both `Focused` and `Adjusting` — the same shape, differing only in color — and supplies the `>` cursor as the non-color **focus** indication alone. T016 gives `Adjusting` `accent/adjust` and `Disabled` `text/muted`, colors only. T017's note states outright that `Resting`, `Focused`, `Adjusting`, and `Disabled` have no status mark. WP05's eight pages never declare a `Disabled` specimen. T037 then enumerates the non-color signals to assert — `>`, `M ON`, `S ON`, `PREPARING`/`ACTIVATING`, typed error text, a selected mark — and omits both states. The witness predicates `states_painted = 9` and `states_distinguishable_without_color = true` cannot pass as the work is written. | Give `Adjusting` and `Disabled` a non-color treatment in WP03 (T015/T016/T017), add a named specimen for each to WP05 T028/T029 and to T037's assertion list — or record a deliberate crest-spec narrowing of `explicit_state_rendering`. Resolves with A10. |
| A4 | Underspecification | HIGH | spec.md:71 (US2 AS-2), :101 · quickstart.md:34 · WP05 T026 step 2, T027 step 2 · `shell.yaml:36-43` · `witnesses.yaml` `unbound_digit_retained_page` | "Presses a number key with no page bound to it" cannot occur. The normalized vocabulary carries exactly `Digit1`–`Digit8`; WP05 binds all eight to the eight declared pages; 9 and 0 normalize to `Other`. The acceptance scenario, the edge case, the quickstart line, and the witness predicate `unbound_digit_retained_page = true` therefore all sit on a branch that can never be taken — the vacuous guard C-006 exists to forbid. T027 step 2 does give a reachable formulation ("any other key, including an unbound digit"), so the behavior is implementable; only the criteria are unfalsifiable. | Restate AS-2, the edge case, the quickstart line, and the witness field in terms of any unbound normalized key including `Other`, and make T027's test drive `WindowKey::Other` explicitly. |
| A15 | Conflict | HIGH | `witnesses.yaml:3130-3220` vs WP05 T031 steps 1-3, T032 step 3 · plan.md:113 | `witness.component_gallery` is declared as an automated witness: `command: make demo-live-component-library`, `timeout: 180s`, `observation.kind: json_stdout`, 15 predicates including `pages_painted = 8`, `states_painted = 9`, `pages_reachable_by_digit = 8`, and `window_closed = true`. WP05 T032 step 3 deliberately removes every timeout because "a browsable scene waits for the operator by design", and T031 step 1 emits the observation "**after** painting" — a single paint pass, with the window still open. A single paint cannot have painted 8 pages, and `window_closed` cannot be true while the window is open. The only reading that closes the gap — accumulate counters across the session and emit on window close, after an operator has visited all eight pages and pressed an unbound key — appears in no subtask, and nothing tells the operator that protocol. As written the witness fails or hangs on its first run. | Specify the emission point (on window close), the counter accumulation semantics, and the operator protocol in T031; state in T032 or the quickstart what the operator must do before closing. Reconcile the declared 180 s timeout with a human-attended scene, or mark the witness operator-attended in the crest-spec. |
| A5 | Inconsistency | MEDIUM | plan.md:141-147 vs WP01:36-41,T001 · WP02:31-33 · WP03:33-35 · WP04:31-34 · WP05:33-37 | plan.md's Project Structure declares `src/shell/visual_token.rs`, `authored_typeface.rs`, `density_policy.rs`, `component_state.rs`, and `primitives/`. Every WP `owned_files` block and T001's module tree use `src/shell/visual/{mod,token,typeface,density,state,primitives}.rs`. plan.md also omits two files the WPs own: `src/shell/keyboard_input_translator.rs` (WP04) and `src/bin/crest_synth.rs` (WP05). Lane write-scope enforcement follows the WP paths, so the plan's tree is already stale. | Update plan.md:141-147 to the `src/shell/visual/` tree and add the two omitted files. |
| A6 | Terminology | MEDIUM | plan.md:25,113 · `witnesses.yaml:3130` vs spec.md:64,140 (C-005) · research.md R-08 · `assets.yaml:183-186` | "Witness" carries two opposite contracts. plan.md:25 calls the target "the browsable live gallery **witness**" and the proof table declares `witness.component_gallery`, while C-005, R-08, WP05 and the `BuildMakefile` prompt all state this scene is explicitly *not* a witness and not a `demo-live` alias. R-08 names this exact conflation as the risk. | Say "measured observation" in plan.md:25 and add one line to the proof table distinguishing `witness.component_gallery` from the input-isolated autonomous `demo-live-*` contract. Resolves with A15. |
| A7 | Inconsistency | MEDIUM | spec.md:154-155 vs research.md:32 vs DESIGN.md:534-551 | research.md is now correct — the design file publishes 13 variables and `DESIGN.md` lists 16 — which makes the shared set 12, plus one design-file-only (`bg/selected`) plus four `DESIGN.md`-only accents, union 17. spec.md's assumption still says "all **11** shared colors" were verified. Every count site now says 17, so nothing downstream breaks; the risk is that someone re-derives 11 + 1 + 4 = 16 and "corrects" the 17 back down. This arithmetic is what produced the previous report's CRITICAL. | Correct spec.md:154-155 to 12 shared / 1 design-file-only / 4 `DESIGN.md`-only / 17 union. |
| A8 | Coverage | MEDIUM | spec.md:128 (NFR-004), :176 (SC-007) · tasks.md:240 · WP04 T025 steps 2-3 · WP06 frontmatter | NFR-004 names a specific ceiling — the 512-event control-path fixture within 50 ms — and SC-007 claims the instrument sounds and responds exactly as before. The only verification is T025's `make test` plus "run `make demo-live` and confirm the audio behavior is unchanged" by ear. `make test` is `cargo test --all-targets`, so `tests/control_dispatch_performance.rs` (DISPATCH_COUNT = 512) does execute — but no subtask names it, no subtask records the measured duration as evidence, and WP06, the package whose entire purpose is measured proof, does not list NFR-004 at all. | Add a step to T025 or a WP06 subtask running `cargo test --test control_dispatch_performance` and recording the measured dispatch duration in the Activity Log. |
| A9 | Coverage | MEDIUM | tasks.md:235-243 | The Requirement coverage table maps FR-001..FR-010 and NFR-001..NFR-006 and omits all seven constraints. C-004 (two top-level contexts, gallery paging never a `SemanticAction`) and C-006 (deterministic proof discipline) are load-bearing and are enforced only implicitly, by T024/T027 and by WP06 respectively. | Add constraint rows: C-004 → T024/T027, C-005 → WP05, C-006 → WP06, C-007 → already landed. |
| A10 | Inconsistency | MEDIUM | DESIGN.md:575 vs spec.md:114 · `requirements.yaml` `explicit_state_rendering` · WP01 T005 | `DESIGN.md:575` requires non-color treatment for six states — focus, mute, solo, loading, error, selection. FR-005 and the crest-spec require eight, adding adjustment and disabled. `DESIGN.md` is the product authority, and T005 records exactly three durable decisions (color union, authored Steam Deck policy, loading/error reuse); this widening is not one of them, and T005's only `DESIGN.md` table edit is adding `bg/selected`. | Add the state-list widening as a fourth durable decision in T005 and update `DESIGN.md:575`, or narrow FR-005 to the six. Resolves with A3. |
| A16 | Traceability | MEDIUM | WP06:6-9 vs WP06:82,203,225 · tasks.md:237-242 | WP06's frontmatter `requirement_refs` lists NFR-001, NFR-002, NFR-003. Its own subtasks state otherwise: T037's purpose is "NFR-005 and FR-005", T038's is FR-010, and the Context block cites NFR-005 and C-006. tasks.md's coverage table assigns NFR-005 to WP05 and FR-010 to WP01 alone. Every requirement does have at least one task, so this is traceability drift rather than a gap — but a reviewer checking WP06 against its declared refs will not check the assertions that matter most. | Add NFR-005, FR-005, FR-010 to WP06's `requirement_refs` and mark the shared ownership in tasks.md's coverage table. |
| A11 | Ambiguity | LOW | plan.md:83 vs tasks.md:50 (T023) · spec.md:22 | The descriptor transition reads "17 → 33" in the plan and "21 → 33" in T023. Both are right in different frames — 17 was the stale *declared* count, 21 is the current *constructed* count in `window_input.rs:42`, 33 is the new total (16 keys × 2 + `FocusLost`) — but neither states its frame. | Write "declared 17 → 33, constructed 21 → 33" once and use it in both places. |
| A12 | Inconsistency | LOW | tasks.md:7-8,12-20 · `assets.yaml` | tasks.md asserts "Every `owned_files` entry traces to a declared asset file pattern", but no asset in `assets.yaml` carries a `files:` key — asset scope is expressed in prose. The table then gives three different assets the same non-discriminating pattern `src/*/*`, which does not match the three-level `src/shell/visual/token.rs` the WPs actually create. | Replace the File pattern column with the asset's declared surface prose, or soften the claim. |
| A13 | Ambiguity | LOW | spec.md:5 | Header still reads `**Status**: Draft` after plan, tasks, work packages, and lanes were derived from it. | Set to the project's post-tasks value. |
| A14 | Inconsistency | LOW | tasks.md:215-220 | The ASCII graph shows one linear chain with an unlabeled return edge; the prose beneath correctly states WP04 depends on WP01+WP02+WP03 and WP05 on WP03+WP04. The diagram does not carry that fan-in. | Redraw with explicit edges, or delete the diagram and keep the prose. |
| A17 | Inconsistency | LOW | plan.md:116 | The proof table cites "6 new entries in `proof/invariants.yaml`". No `proof/` directory exists at the repository root; the six entries are present and correct in `.kittify/crest-spec/proof/invariants.yaml`. | Qualify the path. |

**Coverage Summary Table:**

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|
| FR-001 single-semantic-visual-vocabulary | Yes | T001–T003, T006 | Count now consistently 17 everywhere — A1 resolved |
| FR-002 azeret-mono-installed-and-mapped | Yes | T004, T021 | |
| FR-003 declared-viewport-density-policies | Yes | T007–T009, T012 | |
| FR-004 reusable-primitives | Yes | T013–T018 | |
| FR-005 explicit-state-rendering | Partial | T010, T011, T015, T017, T037 | Adjusting and Disabled color-only; no Disabled specimen — A3 |
| FR-006 production-shell-renders-through-vocabulary | Yes | T020–T022, T025 | |
| FR-007 live-gallery-demo-scene-and-launch-target | Yes | T026–T032 | C-002 now keeps the target in scope — A2 resolved |
| FR-008 number-key-page-selection | Partial | T027 | Unbound-digit branch unreachable — A4 |
| FR-009 components-own-no-application-state | Yes | T019 | |
| FR-010 typed-failure-when-typeface-unavailable | Yes | T004, T021, T038 | Not in WP06's declared refs — A16 |
| NFR-001 exact-authored-value-fidelity | Yes | T006, T034 | Expected values written independently — good |
| NFR-002 no-visual-literals-outside-vocabulary | Yes | T035 | Guard's own failure proof included — good |
| NFR-003 both-authored-viewports-render-intact | Yes | T012, T036 | |
| NFR-004 no-real-time-or-control-path-regression | Weak | T025 | Fixture runs under `make test` but is never named or measured — A8 |
| NFR-005 complete-gallery-state-coverage | Partial | T031, T037 | Fails on Disabled as specified — A3; witness unsatisfiable — A15 |
| NFR-006 vendored-typeface-provenance | Yes | already landed | `vendor/azeret-mono/` present with license, provenance, and hash manifest |
| C-001 eframe/egui stack only | Implicit | — | No task introduces another stack |
| C-002 bounded scope | Yes | T032 | Rewritten since the last report; launch target now explicitly in scope |
| C-003 crest-spec authored before planning | Yes | — | Authored at commit `d02ad6b`; `crest-spec doctor` green |
| C-004 two top-level contexts preserved | Implicit | T024, T027 | Not in the coverage table — A9 |
| C-005 gallery input isolation is one-way | Implicit | T032 | Not in the coverage table — A9; see also A6, A15 |
| C-006 deterministic proof discipline | Implicit | WP06 | Not in the coverage table — A9; A4 and A15 are the live threats to it |
| C-007 typeface licensing | Yes | already landed | OFL 1.1 retained verbatim |

**Charter Alignment Issues:** None. The mission followed the full workflow (specify → crest-spec → plan → analyze → tasks), the crest-spec was authored first at `d02ad6b` and not retrofitted, no `data-model.md` or `contracts/` exists, and `DIRECTIVE_035` correctly stays unset — the adapter constants are deleted and replaced by new identifiers, not renamed. A3, A4, and A15 are proof-quality gaps of exactly the kind `DIRECTIVE_010` and C-006 exist to catch; they are findings for this gate to surface, not charter breaches.

**Unmapped Tasks:** None. All 38 subtasks map to at least one requirement or constraint.

**Metrics:**

- Total Requirements: 23 (10 FR, 6 NFR, 7 C)
- Total Tasks: 38 subtasks across 6 work packages
- Coverage: 16/16 FR+NFR have at least one task (100%); 3 are partial or weak (FR-005, FR-008, NFR-004, NFR-005). 4 of 7 constraints are covered only implicitly and appear in no coverage table.
- Ambiguity Count: 2
- Duplication Count: 0
- Critical Issues Count: 0

## Next Actions

No CRITICAL issues. Three HIGH findings should be resolved before `/spec-kitty.implement` — each one makes a declared proof unfalsifiable, which is the specific failure C-006 and this mission's own WP06 risk table are written to prevent.

1. **A3** — edit `tasks/WP03-reusable-primitives.md` (T015, T016, T017) and `tasks/WP05-browsable-gallery-scene.md` (T028/T029 specimen list) to give `Adjusting` and `Disabled` a non-color treatment and a specimen, and add both to WP06 T037's assertion list. Resolve A10 in the same pass by adding the state-list widening as a fourth durable decision in WP01 T005.
2. **A4** — restate spec.md:71 and :101, quickstart.md:34, and the `unbound_digit_retained_page` field in terms of any unbound normalized key including `Other`, and have WP05 T027's test drive `WindowKey::Other`.
3. **A15** — specify the observation's emission point, accumulation semantics, and operator protocol in WP05 T031/T032, and reconcile the crest-spec's automated 180 s witness declaration with a human-attended browsable scene.

The seven MEDIUM findings are worth a single editing pass but do not block: A5 and A17 are plan.md drift, A6 is a terminology fix, A7 is one sentence of arithmetic, A8 adds one command to T025, A9 adds four rows to a table, A16 adds three entries to WP06's frontmatter. The five LOW findings are cleanup.

Because A15 touches `.kittify/crest-spec/proof/witnesses.yaml`, resolving it is a deliberate crest-spec amendment through `/spec-kitty.crest-spec`, not an edit made to let planned code pass. Re-run `/spec-kitty.analyze` after remediation — the current report's hashes will be stale.
