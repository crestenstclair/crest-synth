---
schema_version: 1
artifact_type: spec-kitty.analysis-report
command: /spec-kitty.analyze
mission_slug: crest-component-foundations-01KZ02H2
mission_id: 01KZ02H2FPKTV50BJP82MB4T5G
generated_at: '2026-08-02T02:45:10.123564+00:00'
analyzer_agent: unknown
input_artifacts:
  spec.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/spec.md
    sha256: f55ddfa06a22ca9d7ea84a9d100d05f4687525fb8561b08879d4a53f4a7e84c4
  plan.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/plan.md
    sha256: 26eceae2ad46d1a8b0758b79353726827f23a8d20da98d6e85d2aaadd019d4ea
  tasks.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/crest-component-foundations-01KZ02H2/tasks.md
    sha256: 64d48b84dd9e4f752ee53086acf88b48e66cc0f1f83cd411532a1e881ff7e951
  charter:
    path: /Users/crestenstclair/workspace/crest-synth/.kittify/charter/charter.md
    sha256: 0b21a43cf5772d1308561d843239947e53247cc7d071c98c920023d23024672b
verdict: blocked
issue_counts:
  low: 4
  medium: 6
  critical: 2
  high: 2
  info: 0
findings:
- id: A1
  severity: critical
  category: inconsistency
  summary: Authored color count is declared as 13 in spec/plan/quickstart/research/crest-spec but WP01 enumerates and asserts 17; NFR-001 makes the count test-asserted, so the two cannot both hold.
- id: A2
  severity: critical
  category: conflict
  summary: C-002 declares `make demo-live-component-library` out of scope while FR-007, the plan, WP05/T032 and crest-spec asset.BuildMakefile all deliver it.
- id: A3
  severity: high
  category: coverage
  summary: Adjusting and Disabled have no non-color indication in any subtask, contradicting FR-005, SC-005 and crest-spec requirement.explicit_state_rendering.
- id: A4
  severity: high
  category: underspecification
  summary: US2 acceptance scenario 2 and its edge case (unbound number key) are unreachable — all eight normalized digits are bound to the eight declared pages.
- id: A5
  severity: medium
  category: inconsistency
  summary: plan.md Project Structure names flat modules (src/shell/visual_token.rs, density_policy.rs, component_state.rs) while tasks and lanes.json use src/shell/visual/{token,typeface,density,state}.rs.
- id: A6
  severity: medium
  category: terminology
  summary: The gallery target is called a witness in plan.md and declared as witness.component_gallery, while C-005, research R-08 and the asset prompt insist it is explicitly not a witness.
- id: A7
  severity: medium
  category: inconsistency
  summary: 'Shared-color arithmetic disagrees across artifacts: spec says 11 shared colors, research says the design file publishes 13 and DESIGN.md lists 15, DESIGN.md actually lists 16.'
- id: A8
  severity: medium
  category: coverage
  summary: NFR-004 and SC-007 have no verification subtask naming the 512-event control-path fixture or its 50 ms ceiling; T025 relies on `make test` plus an audio check by ear.
- id: A9
  severity: medium
  category: coverage
  summary: The requirement-coverage table maps only FR and NFR items; none of the seven constraints C-001..C-007 appear.
- id: A10
  severity: medium
  category: inconsistency
  summary: DESIGN.md requires non-color treatment for six states; spec FR-005 and the crest-spec require eight, and the widening is not among the three decisions T005 records.
- id: A11
  severity: low
  category: ambiguity
  summary: Descriptor count transition is written as 17 -> 33 in plan.md and 21 -> 33 in T023 without stating which frame each uses.
- id: A12
  severity: low
  category: inconsistency
  summary: tasks.md claims every owned_files entry traces to a declared asset file pattern, but crest-spec assets declare no files key and three assets share the placeholder pattern src/*/*.
- id: A13
  severity: low
  category: ambiguity
  summary: 'spec.md still carries Status: Draft after plan, tasks and work packages were finalized from it.'
- id: A14
  severity: low
  category: inconsistency
  summary: The dependency-graph diagram in tasks.md does not express the WP04 fan-in that the prose beneath it states.
---

## Specification Analysis Report

**Mission**: `crest-component-foundations-01KZ02H2` · **Artifacts**: spec.md (176 L), plan.md (246 L), tasks.md (242 L), 6 WP prompts, research.md, quickstart.md, `.kittify/crest-spec/`, `.kittify/charter/charter.md`

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| A1 | Inconsistency | CRITICAL | spec.md:125,170 · plan.md:30 · quickstart.md:25 · research.md:41 · `.kittify/crest-spec/contexts/shell.yaml:84` · `.kittify/crest-spec/capabilities.yaml:692` · tasks.md:29 · WP01:77,136,269 | Five artifacts plus the crest-spec say the vocabulary holds **13** authored colors. WP01's own table enumerates **17** (`bg/canvas,surface,panel,elevated,selected`, `border/default,strong`, `text/primary,secondary,muted`, `accent/focus,adjust,positive,warning,instrument,patch,chorus`) and T006 instructs the implementer to *assert the count is 17*. The crest-spec's own prose is self-contradicting: it says "thirteen" and then enumerates 14 named colors plus "the declared identity accents" (3 more). NFR-001 makes this count test-asserted, so an implementer cannot satisfy both. The root error is in research.md R-02, which took the design file's 13 published variables as the union count when the union is 13 + 4 DESIGN.md-only accents = 17. | Fix the count to **17** at every site, starting with `shell.yaml:84` and `capabilities.yaml:692` (a crest-spec amendment, not a silent edit), then spec NFR-001/SC-001, plan.md:30, quickstart.md:25, research.md:41, tasks.md:29 and the WP01 headings. |
| A2 | Conflict | CRITICAL | spec.md:6,137 (C-002) vs spec.md:116 (FR-007) · plan.md:25,86,113 · tasks.md:175 · WP05 T032 · `.kittify/crest-spec/assets.yaml:183` | C-002 states `make demo-live-component-library` is "out of scope and belongs to the follow-on Phase 4 mission" — repeated in the Input line. FR-007 requires "Live gallery demo scene **and launch target**", WP05's independent test *is* `make demo-live-component-library`, and `asset.BuildMakefile` declares the target. A reviewer applying C-002 as written blocks T032; an implementer following FR-007 violates a High-priority constraint. Charter DIRECTIVE_010 (Specification Fidelity) makes spec-vs-crest-spec divergence the costliest failure class. | Rewrite C-002 to defer only configurable controls and reusable compositions, keeping the launch target in scope (which is what the crest-spec, plan, and WP05 already assume). |
| A3 | Coverage | HIGH | spec.md:114 (FR-005), :53 (US1 AS-3), :174 (SC-005) · `.kittify/crest-spec/requirements.yaml` explicit_state_rendering · WP03:117,179,210 · WP05:210 | FR-005 and the crest-spec both require **adjustment** and **disabled** to render with text or shape in addition to color. No subtask provides either. T015 supplies the `>` cursor as "the non-color **focus** indication" only; T016 gives `Disabled` `text/muted` and `Adjusting` `accent/adjust` — colors alone; T017 explicitly declares `Resting`, `Focused`, `Adjusting`, `Disabled` to have **no status mark**; WP05 T029 lists specimens for Loading, Error, Muted, Soloed, Selected and never names Disabled. T037's non-color-legibility assertion would fail against the design as specified. | Add the non-color treatment for `Adjusting` and `Disabled` to WP03 (T015/T016/T017) and a named specimen for each to WP05 T028/T029, or record a deliberate crest-spec narrowing of `explicit_state_rendering`. |
| A4 | Underspecification | HIGH | spec.md:71 (US2 AS-2), :101 (edge case) · WP05 T026 step 2, T027 step 2 · `.kittify/crest-spec/contexts/shell.yaml:36-43` | "Presses a number key with no page bound to it" cannot occur: the normalized key vocabulary carries exactly `Digit1`–`Digit8`, and WP05 binds all eight to the eight declared pages. Digits 9 and 0 normalize to `Other`, so the scenario is only reachable as a non-digit. The acceptance scenario and its edge case can therefore never fail — precisely the vacuous guard C-006 forbids. | Either restate AS-2 and the edge case in terms of any unbound normalized key including `Other`, or make T027's test drive `WindowKey::Other` explicitly and say so in the scenario. |
| A5 | Inconsistency | MEDIUM | plan.md:141-147 vs tasks/WP01:113 · WP02 · WP03:29-35 · lanes.json write_scope | plan.md's Project Structure declares `src/shell/visual_token.rs`, `authored_typeface.rs`, `density_policy.rs`, `component_state.rs`, `primitives/`. Every WP and every lane write-scope uses `src/shell/visual/{token,typeface,density,state,primitives}.rs`. Lane write-scope enforcement follows the WP paths, so the plan's tree is already stale. | Update plan.md:141-147 to the `src/shell/visual/` tree the WPs and lanes actually use. |
| A6 | Terminology | MEDIUM | plan.md:25,113 · `.kittify/crest-spec/proof/witnesses.yaml:3155` vs spec.md:64,140 (C-005) · research.md R-08 · `.kittify/crest-spec/assets.yaml:184` | "Witness" carries two meanings. plan.md:25 calls the target "the browsable live gallery **witness**" and the proof table declares `witness.component_gallery`, while C-005, R-08 and the `BuildMakefile` prompt state the scene is *explicitly not* an autonomous witness and "not a `demo-live` alias". R-08 names this exact conflation as the risk. | Say "measured observation" (or "browsable gallery proof") in plan.md:25 and add one line to the proof table distinguishing `witness.component_gallery` from the input-isolated `demo-live-*` witness contract. |
| A7 | Inconsistency | MEDIUM | spec.md:154-155 vs research.md:31-35 vs DESIGN.md:534-551 | Three incompatible tallies: the spec's assumption says "all **11** shared colors" match; research R-02 says the design file publishes **13** variables and DESIGN.md lists **15**; DESIGN.md's table actually holds **16** rows. The consistent set is 12 shared + 1 design-file-only (`bg/selected`) + 4 DESIGN.md-only = 17. This is the arithmetic that produced A1. | Correct spec.md:154-155 and research.md:31-35 to 12 / 1 / 4 / 17 in one pass with A1. |
| A8 | Coverage | MEDIUM | spec.md:128 (NFR-004), :176 (SC-007) · tasks.md:240 · WP04 T025 | NFR-004 names a specific ceiling — the 512-event control-path fixture within 50 ms — and SC-007 claims audio is unchanged. The only verification is T025's `make test` plus "run `make demo-live` and confirm the audio behavior is unchanged" by ear. `make test` is `cargo test --all-targets`, so `tests/control_dispatch_performance.rs` does run, but no subtask names the fixture or requires its timing be recorded as evidence, and WP06 (the measured-proof WP) does not list NFR-004 at all. | Add a step to T025 (or a WP06 subtask) that runs `cargo test --test control_dispatch_performance` and records the measured dispatch duration in the Activity Log. |
| A9 | Coverage | MEDIUM | tasks.md:235-243 | The Requirement coverage table maps FR-001..FR-010 and NFR-001..NFR-006 but omits all seven constraints. C-004 (two top-level contexts) and C-006 (deterministic proof discipline) are load-bearing and are enforced only implicitly by T024/T027 and WP06. | Add a constraint row set to the coverage table, mapping C-004 → T024/T027, C-005 → WP05, C-006 → WP06, C-007 → already landed. |
| A10 | Inconsistency | MEDIUM | DESIGN.md:575 vs spec.md:114 · WP01 T005 | DESIGN.md requires non-color treatment for six states (focus, mute, solo, loading, error, selection). FR-005 and the crest-spec require eight, adding adjustment and disabled. DESIGN.md is the product authority, and T005 records only three durable decisions (color union, Steam Deck policy, loading/error reuse) — this widening is not one of them. | Add the state-list widening as a fourth durable decision in T005 and update DESIGN.md:575, or narrow FR-005 to DESIGN.md's six. Resolves with A3. |
| A11 | Ambiguity | LOW | plan.md:83 vs tasks.md:50 (T023) · spec.md:22 | The descriptor transition reads "17 → 33" in the plan and "21 → 33" in T023. Both are correct in different frames — 17 was the stale *declared* count, 21 is the current *code* count, 33 is the new total (16 keys × 2 + FocusLost) — but neither states its frame. | Write "declared 17 → 33, constructed 21 → 33" once and use it in both places. |
| A12 | Inconsistency | LOW | tasks.md:7-8,12-20 · `.kittify/crest-spec/assets.yaml` | tasks.md asserts "Every `owned_files` entry traces to a declared asset file pattern", but no asset in `assets.yaml` carries a `files:` key — asset scope is expressed in prose descriptions. The table then gives three different assets the same non-discriminating pattern `src/*/*`. | Either replace the File pattern column with the asset's declared surface prose, or soften the claim in tasks.md:7-8. |
| A13 | Ambiguity | LOW | spec.md:5 | Header still reads `**Status**: Draft` after plan, tasks, WPs, and lanes were derived from it. | Set to Approved (or whatever this project's post-tasks value is). |
| A14 | Inconsistency | LOW | tasks.md:215-220 | The ASCII graph shows one linear chain with an unlabeled return edge; the prose below correctly states WP04 depends on WP01+WP02+WP03 and WP05 on WP03+WP04. The diagram does not carry that fan-in. | Redraw with explicit edges or delete the diagram and keep the prose. |

**Coverage Summary Table:**

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|
| FR-001 single-semantic-visual-vocabulary | Yes | T001–T003, T006 | Color count contested — see A1 |
| FR-002 azeret-mono-installed-and-mapped | Yes | T004, T021 | |
| FR-003 declared-viewport-density-policies | Yes | T007–T009, T012 | |
| FR-004 reusable-primitives | Yes | T013–T018 | |
| FR-005 explicit-state-rendering | Partial | T010, T011, T015, T017, T037 | Adjusting and Disabled have no non-color treatment — A3 |
| FR-006 production-shell-renders-through-vocabulary | Yes | T020–T022, T025 | |
| FR-007 live-gallery-demo-scene-and-launch-target | Yes | T026–T032 | Contradicted by C-002 — A2 |
| FR-008 number-key-page-selection | Partial | T027 | Unbound-digit branch unreachable — A4 |
| FR-009 components-own-no-application-state | Yes | T019 | |
| FR-010 typed-failure-when-typeface-unavailable | Yes | T004, T021, T038 | |
| NFR-001 exact-authored-value-fidelity | Yes | T006, T034 | Asserted count contradicts spec — A1 |
| NFR-002 no-visual-literals-outside-vocabulary | Yes | T035 | Guard's own failure proof included — good |
| NFR-003 both-authored-viewports-render-intact | Yes | T012, T036 | |
| NFR-004 no-real-time-or-control-path-regression | Weak | T025 | No named fixture or recorded timing — A8 |
| NFR-005 complete-gallery-state-coverage | Partial | T031, T037 | Will fail on Disabled as specified — A3 |
| NFR-006 vendored-typeface-provenance | Yes | already landed | `OFL.txt`, `PROVENANCE.md`, `SHA256SUMS` and the `varLib.instancer` derivation are present and verified |
| C-001 eframe/egui stack only | Implicit | — | No task introduces another stack |
| C-002 bounded scope | **Conflicting** | T032 | A2 |
| C-003 crest-spec authored before planning | Satisfied | — | Commit `d02ad6b`; doctor OK |
| C-004 two top-level contexts preserved | Yes | T024, T027 | Not in the coverage table — A9 |
| C-005 gallery input isolation is one-way | Yes | T026, T027 | Terminology drift — A6 |
| C-006 deterministic proof discipline | Yes | T034–T038 | |
| C-007 typeface licensing | Satisfied | — | OFL 1.1 verbatim in `vendor/azeret-mono/OFL.txt` |

**Charter Alignment Issues:**

- **DIRECTIVE_010 Specification Fidelity** — A1 and A2 are both spec↔crest-spec divergences. The charter names silent design drift "the costliest failure" and puts crest-spec/`DESIGN.md` fidelity above speed, polish, and even the real-time contract. A1 additionally requires a **crest-spec amendment** (`shell.yaml:84`, `capabilities.yaml:692`), which the charter forbids doing after the fact to permit already-planned code — it must be an explicit, reasoned correction, not a quiet edit during implementation.
- **DIRECTIVE_003 Decision Documentation** — A10: the FR-005 state-list widening past DESIGN.md's six states is an undocumented decision. plan.md:41 claims all such decisions land in `asset.ProductDesignAuthority`; this one does not.
- No other directive conflicts. DIRECTIVE_001, DIRECTIVE_024, DIRECTIVE_025 and DIRECTIVE_035 evaluations in plan.md:40-45 hold as written; the `WindowInput` 17→33 correction is correctly domain-matched.
- Positive: C-006 and WP06 T035 require the literal-absence guard to be **demonstrated failing**, which directly answers the charter's "prose constraints that failed once are replaced by proof-enforced invariants".

**Unmapped Tasks:** None. All 38 subtasks map to at least one FR, NFR, or constraint.

**Metrics:**

- Total requirements: 23 (10 FR + 6 NFR + 7 constraints)
- Total tasks: 38 subtasks across 6 work packages
- Coverage: 16/16 FR+NFR have ≥1 task (100%); 3 are partial or weak (FR-005, FR-008, NFR-004). Constraints: 5/7 explicitly tasked, 1 satisfied pre-mission, 1 conflicting.
- Ambiguity count: 4 (A4, A11, A13, plus the two readings of "witness" in A6)
- Duplication count: 1 (FR-005 mapped to both WP02 and WP03 — a benign split, no action)
- Critical issues count: 2
