---
schema_version: 1
artifact_type: spec-kitty.analysis-report
command: /spec-kitty.analyze
mission_slug: webview-shell-cutover-01KZAC7Q
mission_id: 01KZAC7QD98CQ1SGFN71KF4RX7
generated_at: '2026-08-06T03:42:59.963375+00:00'
analyzer_agent: unknown
input_artifacts:
  spec.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/webview-shell-cutover-01KZAC7Q/spec.md
    sha256: 4726dd7b04cbfcdd3299d966a64aa69206454abb1c947c782f3b47c7789e09f2
  plan.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/webview-shell-cutover-01KZAC7Q/plan.md
    sha256: 07ae252627a3e4eb552704e955f9d4abe189cb989533395e5dc025ec179f06fe
  tasks.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/webview-shell-cutover-01KZAC7Q/tasks.md
    sha256: 82c56f9b4c4d325ea09527e9e7d2a2d0449772197661e60495b211e8da54a693
  charter:
    path: /Users/crestenstclair/workspace/crest-synth/.kittify/charter/charter.md
    sha256: 0b21a43cf5772d1308561d843239947e53247cc7d071c98c920023d23024672b
verdict: ready
issue_counts:
  medium: 4
  high: 0
  low: 5
  critical: 0
  info: 0
findings:
- id: A1
  severity: medium
  category: ambiguity
  summary: NFR-002 acceptance term 'no leak growth trend' has no named metric or quantitative bound in spec or WP06 T024.
- id: A2
  severity: low
  category: ambiguity
  summary: C-002's absolute wording ('no webview-specific variant of the model exists') can be read to forbid WP04's separate gallery-scene document; scope needs a one-line clarification.
- id: C1
  severity: medium
  category: coverage
  summary: Plan performance goal 'reducer state change visible in the webview <=50 ms p95' has no task that measures or asserts it; WP06 T022 collects max-bound fields with no p95/50 ms bar.
- id: C2
  severity: low
  category: coverage
  summary: Subtasks T017-T020 trace only to SC-005/crest-spec validations (no FR), and constraints C-001/C-002/C-003 appear in no WP requirement_refs frontmatter (enforced only in WP prose/DoD).
- id: I1
  severity: medium
  category: inconsistency
  summary: Spec's own Domain Language forbids 'migration' for demo scenes, yet spec NFR-001 says 'migrated scenes' and plan IC-03 is titled 'Retained scene migration' ('4 retained live scenes migrated' in Scale/Scope).
- id: I2
  severity: medium
  category: inconsistency
  summary: SC-004 defines 'net shell code reduction >=10,000 lines' but WP07 T031 measures the whole-mission git diff --stat, which counts committed evidence logs and kitty-specs planning docs, distorting the code-reduction number.
- id: U1
  severity: low
  category: underspecification
  summary: WP07 T028 directs edits to src/shell/standalone_application.rs, which is not in WP07's owned_files and, unlike every comparable case, carries no out-of-map recording instruction.
- id: U2
  severity: low
  category: underspecification
  summary: WP06's created evidence artifacts (evidence/*.log, rt-ab-comparison.md, README.md) are not declared in create_intent/owned_files; only scripts/rt_ab_measurement.sh is.
- id: I3
  severity: low
  category: inconsistency
  summary: WP02 cites the stale foundation-WP06 comment at src/shell/webview/window.rs:183; the reference actually sits at line 191.
---

## Specification Analysis Report

**Mission**: `webview-shell-cutover-01KZAC7Q` — cross-artifact analysis of `spec.md`, `plan.md`, `tasks.md`, `tasks/WP01..WP07`, against `.kittify/charter/charter.md`.

Overall: the artifact set is unusually coherent. All 8 FRs and 4 NFRs are mapped to work packages via `requirement_refs`; the dependency graph, wave ordering, and the C-007 evidence-before-deletion gate are stated identically in spec, plan, and tasks; file-ownership boundaries between parallel WPs are explicit and almost always carry out-of-map escape hatches. Fact-checks against the repo confirmed the artifacts' claims (fifteen gallery pages, eight-entry `FROZEN_DIGIT_BINDING_BASELINE`, four `demo-live-*` Makefile targets, `FROZEN_TOPOLOGY_IDENTITY_BASELINE` at `tests/effects_and_buses.rs:59`, WP03's owned files covering all four scene hosts). No charter MUST violation, no duplicate or conflicting requirement, no zero-coverage blocking requirement was found. Findings below are refinements, not blockers.

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| A1 | ambiguity | medium | spec.md NFR-002; tasks/WP06-hardware-evidence.md T024 | NFR-002 requires the 300 s soak to show "no leak growth trend"; neither the spec nor T024 names the measured field(s) or a quantitative bound — T024 defers to "the soak's own measured fields" without saying which or what passes. | In WP06 T024, name the leak-relevant measured fields from the existing soak report and state the acceptance bound (e.g. final RSS within N% of the post-warmup value, or zero monotonic growth across sampling windows), so the evidence reviewer applies a bar, not a judgment call. |
| A2 | ambiguity | low | spec.md C-002; tasks/WP04-gallery-webview.md T014 | C-002 says "no webview-specific variant of the model exists", while WP04 T014 deliberately introduces a separate gallery-scene document "beside" `SemanticGraphicalViewModel`. WP04 pre-empts the conflict in prose, but a reviewer holding C-002 literally could reject WP04. | Add one clarifying clause to C-002 (or the WP04 risk note): the one-schema constraint governs production shell documents; testing-context gallery documents are out of its scope and make no claim on the production schema. |
| C1 | coverage | medium | plan.md Technical Context (Performance Goals); tasks/WP06-hardware-evidence.md T022/T025 | The plan commits to "reducer state change visible in the webview ≤50 ms p95 under the paced live workload", but no subtask asserts it: T022 collects `frames_to_projection_max` / `render_blocks_to_audible_max` (max bounds) with no p95/50 ms acceptance bar, and no spec NFR carries the number. | Either add the ≤50 ms p95 bar to WP06 T022/T025's evidence criteria (the fields exist in live reports) or explicitly record in plan.md that the obligation is discharged by the crest-spec `requirement.serialized_projection_transport` validation, naming where it is checked. |
| C2 | coverage | low | tasks/WP05-test-retargeting.md frontmatter; tasks.md; spec.md C-001/C-002/C-003 | T017–T020 (five retargeted acceptance contracts) map to no FR — WP05's `requirement_refs` lists only FR-008; their authority is SC-005 plus crest-spec validations. Likewise cross-cutting constraints C-001/C-002/C-003 appear in no WP's `requirement_refs`, though each is enforced in WP prose, DoD, and existing tests. | Traceability only: add the relevant constraint IDs (C-001..C-003) and an SC-005 note to WP05's frontmatter so the mapping is machine-visible, or accept prose-level enforcement as sufficient and record that decision. |
| I1 | inconsistency | medium | spec.md Domain Language vs spec.md NFR-001; plan.md IC-03 title and Scale/Scope | The spec's Domain Language section rules: "Avoid 'migration' for the demo scenes: scenes are not rewritten; the shell under them changes." Yet spec NFR-001 says "migrated scenes record zero `audio_uninterrupted=false` checkpoints", plan IC-03 is titled "Retained scene migration + hardware evidence", and plan Scale/Scope says "4 retained live scenes migrated". WP03 itself uses the correct framing ("the shell under them changes"). | Reword the three occurrences ("re-hosted scenes" / "Retained scene re-hosting" / "4 retained live scenes re-hosted") so the artifacts obey the spec's own terminology rule; this is exactly the drift the rule exists to prevent. |
| I2 | inconsistency | medium | spec.md SC-004 vs tasks/WP07-flip-and-deletion.md T031 | SC-004 promises "Net shell code reduction of at least 10,000 lines against the ~17k-line hand-painted visual layer", but T031 measures "net line delta of the mission's diff (`git diff --stat` against the pre-mission base)" — a whole-repo delta that includes committed hardware evidence logs, `evidence/README.md`, and the ~2,700 lines of kitty-specs planning artifacts, so the measured number is not the shell-code reduction and could spuriously miss the ≥10k bar. | Scope T031's measurement to the code surfaces (`src/`, `tests/`, `webview-page/`, `Cargo.*`) or exclude `kitty-specs/` from the diff, and record both the scoped and raw numbers in the ROADMAP gate note. |
| U1 | underspecification | low | tasks/WP07-flip-and-deletion.md T028 vs WP07 frontmatter owned_files | T028 step 1 directs edits in "`src/shell/standalone_application.rs` composition (the latter is WP03-owned but merged — you edit crest_synth and any residual selection seam)", but `standalone_application.rs` is absent from WP07's `owned_files`, and unlike every comparable boundary crossing in WP03/WP04/WP07 (each says "record as out-of-map with rationale") this one carries no recording instruction. | Add `src/shell/standalone_application.rs` to WP07's owned_files (WP03 is merged by then) or append the standard "record as out-of-map" instruction to T028. |
| U2 | underspecification | low | tasks/WP06-hardware-evidence.md frontmatter vs T023/T024/T025 | WP06 creates `evidence/<scene>-live-run.log` (x4), `evidence/rt-ab-comparison.md`, `evidence/soak-300s.log`, and `evidence/README.md`, but its `create_intent`/`owned_files` declare only `scripts/rt_ab_measurement.sh` — inconsistent with WP04/WP05, which declare every created file, and a potential trip for ownership-gated review. | Add the evidence paths (or an `evidence/**` glob under the mission directory) to WP06's `create_intent`/`owned_files`. |
| I3 | inconsistency | low | tasks/WP02-forwarding-and-hardening.md Context and T005 vs src/shell/webview/window.rs:191 | WP02 says "the stale comment at `src/shell/webview/window.rs:183` still points at 'WP06' of the foundation mission"; the WP06 reference actually sits at line 191 ("acceptance work package that measures it (WP06 T026)"). The substantive claim is correct; the line anchor has drifted. | Cite the comment by its text rather than line number (or update to :191) so T005 step 4 targets the right comment after any rebase. |

### Coverage Summary Table

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|
| FR-001 | Yes | T001–T004 (WP01) | requirement_refs: WP01 |
| FR-002 | Yes | T028, T031 (WP07) | requirement_refs: WP07 |
| FR-003 | Yes | T010–T013 (WP03), T023 (WP06) | Deliberate code-half/evidence-half split; refs in both WPs |
| FR-004 | Yes | T005, T006, T009 (WP02); page half T003 (WP01) | requirement_refs: WP02 |
| FR-005 | Yes | T014–T016 (WP04) | requirement_refs: WP04 |
| FR-006 | Yes | T029, T031 (WP07) | requirement_refs: WP07 |
| FR-007 | Yes | T030 (WP07) | Crest-spec half already authored at `307873e`; DESIGN.md half is T030 |
| FR-008 | Yes | T021 (WP05) | requirement_refs: WP05 |
| NFR-001 | Yes | T022 (WP06) | requirement_refs: WP06 |
| NFR-002 | Yes | T024 (WP06) | See finding A1 (unquantified "leak growth trend") |
| NFR-003 | Yes | T007, T009 (WP02) | requirement_refs: WP02 |
| NFR-004 | Yes | T027 (WP07); enforced in T001/T014 discipline | requirement_refs: WP07 |
| C-001 | Prose only | Enforced in WP01 DoD, WP04 review guidance, T021 witness | No requirement_refs entry (finding C2) |
| C-002 | Prose only | Enforced in WP01 objective/DoD | No requirement_refs entry (findings C2, A2) |
| C-003 | Prose only | Enforced in WP02 (no RT work), WP03 (teardown), WP07 tripwires | No requirement_refs entry (finding C2) |
| C-004 | Yes | T013 (WP03), T025 (WP06) | requirement_refs: WP03 |
| C-005 | Satisfied | — (planning-phase constraint) | Verified: no `data-model.md`, no `contracts/` in mission dir |
| C-006 | Yes | T014–T016 (WP04) | requirement_refs: WP04 |
| C-007 | Yes | T023/T025 (WP06), WP07 hard gate | requirement_refs: WP06; WP07 opens with a STOP check |

### Charter Alignment Issues

None. Verified against `.kittify/charter/charter.md` (2026-07-31 posture): full mission rigor is followed (crest-spec authored first at `307873e`, before plan); the silent-design-drift priority is honored (ROADMAP gate, crest-spec retirement declaration, DESIGN.md pivot as FR-007/T030); retained live-demo scenes stay phase gates (WP06 evidence wall precedes WP07 deletion — C-007); proofs remain executable invariants rather than prose; exception handling follows the "self-service but never silent" rule via the out-of-map recording pattern throughout the WP files (with the single omission noted as U1). Hardware/proof gates are operator-run, never waived autonomously (WP06 objective states this explicitly).

### Unmapped Tasks

- **T017, T018, T019, T020** (WP05): trace to SC-005 and the crest-spec's re-declared validations (`shell_event_dispatch`, `graphical_application_shell`, `semantic_graphical_view_model`, `component_vocabulary`, `component_composition`) but to no FR/NFR/C in `requirement_refs` (finding C2). The work itself is well-specified and clearly derived; this is a traceability gap only.
- All other subtasks (T001–T016, T021–T031) map to at least one requirement via their WP's `requirement_refs` or the WP body's stated FR/SC anchors.

### Metrics

- **Total Requirements**: 19 (8 FR + 4 NFR + 7 Constraints)
- **Total Tasks**: 31 subtasks across 7 work packages
- **Coverage**: 100% by artifact prose (every FR/NFR/C has an enforcing task, DoD clause, or is already satisfied); 15/19 = 79% by machine-readable `requirement_refs` frontmatter (C-001, C-002, C-003 prose-only; C-005 satisfied at planning)
- **Ambiguity Count**: 2 (A1, A2)
- **Duplication Count**: 0 (FR-003's dual mapping is a deliberate, documented code/evidence split, not duplication)
- **Critical Issues Count**: 0 (high: 0)
