---
schema_version: 1
artifact_type: spec-kitty.analysis-report
command: /spec-kitty.analyze
mission_slug: webview-shell-foundation-01KZ9DN7
mission_id: 01KZ9DN7SDDFNYYYVC74XT21QG
generated_at: '2026-08-05T17:39:58.174215+00:00'
analyzer_agent: unknown
input_artifacts:
  spec.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/webview-shell-foundation-01KZ9DN7/spec.md
    sha256: 76dae780543e9bf1a2f56415b1df6f23ce24dcc753be4315b16125854858357b
  plan.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/webview-shell-foundation-01KZ9DN7/plan.md
    sha256: d99003ef91d4404d3dded93d13d9f039a8c82fec9b0ef29ebc9cd72d85118163
  tasks.md:
    path: /Users/crestenstclair/workspace/crest-synth/kitty-specs/webview-shell-foundation-01KZ9DN7/tasks.md
    sha256: 28c45ddaf06aa98794251f9be412e975e82ac7147e1a128b7f3ae1fed6f9a2a4
  charter:
    path: /Users/crestenstclair/workspace/crest-synth/.kittify/charter/charter.md
    sha256: 0b21a43cf5772d1308561d843239947e53247cc7d071c98c920023d23024672b
verdict: ready
issue_counts:
  high: 0
  critical: 0
  medium: 2
  low: 3
  info: 0
findings:
- id: C1
  severity: medium
  category: coverage
  summary: Mixer-column geometry (82/86/floor) lives in ViewportDensityPolicy, not the token vocabulary; WP04's generator only exports token.rs, so WP05 would hand-declare geometry — the drift class the mission bans for tokens.
- id: C2
  severity: medium
  category: coverage
  summary: FR-003's 'full existing MIXER key vocabulary works unchanged' has no automated proof; WP02 verifies manually and WP06's sections prove render/demo paths, not physical-key-to-translator fidelity.
- id: I1
  severity: low
  category: inconsistency
  summary: NFR-001's crest://painted echo is implemented in page.js (WP05-owned) but specified only in WP06's risks as an out-of-map addition.
- id: A1
  severity: low
  category: ambiguity
  summary: WP06 T024 asserts 'Inspector >= 320 px equivalent in the observation' but renderObservation is structural, not pixel-measuring.
- id: T1
  severity: low
  category: terminology
  summary: Spec Domain Language bans 'fallback' for shell selection; WP01/WP02 prompts use 'fallback' for the NSEvent input path (different sense, same word).
---

## Specification Analysis Report

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| C1 | Coverage | MEDIUM | tasks/WP04-token-generation.md T014; tasks/WP05-mixer-projection-page.md T019 | WP04 exports only `src/shell/visual/token.rs` vocabularies; the mixer-column geometry (82 px column, 86 px pitch, narrow floor) is authored in `ViewportDensityPolicy` (`src/shell/visual/density.rs`) and WP05 T019 permits page-side declaration of "anything the Rust vocabulary does not name" — reintroducing hand-copied authored values, the defect class FR-002 exists to prevent. | During WP04, extend `tokens_css()` to also emit the mixer-column geometry read from `ViewportDensityPolicy`; WP05 then consumes only generated properties. One-line scope note, no re-planning needed. |
| C2 | Coverage | MEDIUM | spec.md FR-003; WP02 DoD; WP06 T024/T026 | FR-003 requires the full MIXER key vocabulary to work unchanged, but its proof is manual (WP02 DoD) plus WP01 probe evidence; WP06's automated sections prove rendering and the demo path, not physical key-to-translator fidelity in the webview shell. | Acceptable for this mission given the WP01 probe log is committed as evidence; note for the successor mission to add a key-injection witness if the platform allows. |
| I1 | Inconsistency | LOW | WP05 T018; WP06 T026 risks | The `crest://painted` ack that NFR-001 measurement needs is a page.js concern owned by WP05 but only specified in WP06's risk note as a future out-of-map edit. | WP05 implementer should add the ack listener while authoring page.js (it is presentation-only); WP06 then measures without out-of-map edits. |
| A1 | Ambiguity | LOW | WP06 T024 | "Inspector ≥ 320 px equivalent in the observation" mixes a pixel threshold into a structural observation contract. | Define the observation field as the Inspector's computed width (an integer the page reports), asserted ≥ 320 at the compact viewport. |
| T1 | Terminology | LOW | spec.md Domain Language; WP01 T003, WP02/plan R-02 | "Fallback" is banned vocabulary for shell selection but used for the NSEvent input path. The senses differ (input mechanism contingency vs. silent shell substitution); the word collision could confuse a reviewer applying the ban literally. | Reviewers read "fallback" in WP01/WP02 as "designed secondary input path"; the shell-selection ban is untouched. No edit required. |

**Coverage Summary Table:**

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|
| FR-001 webview-launch | Yes | T006-T008, T017-T021, T022+ | WP02/WP05/WP06 |
| FR-002 authored-tokens | Yes | T014-T016, T019, T023 | see C1 for geometry gap |
| FR-003 rust-side-input | Yes | T001-T005, T007 | see C2 for proof depth |
| FR-004 one-schema | Yes | T011, T022 | |
| FR-005 meters | Yes | T012, T026 | |
| FR-006 owned-shutdown | Yes | T009, T026 | |
| FR-007 typed-failure | Yes | T010, T025 | |
| NFR-001 50ms-p95 | Yes | T026 | see I1 |
| NFR-002 30hz-meters | Yes | T012, T013, T026 | |
| NFR-003 rt-unchanged | Yes | T004, T013, T026 | |
| NFR-004 both-viewports | Yes | T021, T024 | see A1 |

**Charter Alignment Issues:** none — the charter (59 lines) carries no MUST/SHALL normative statements; governance directives (DIRECTIVE_035/043/044) are honored structurally in the plan.

**Unmapped Tasks:** none — all 26 subtasks map to WPs; all WPs carry requirement_refs.

**Metrics:**

- Total Requirements: 7 FR + 4 NFR + 5 C
- Total Tasks: 26 subtasks / 6 WPs
- Coverage: 100% (every FR/NFR has ≥1 task)
- Ambiguity Count: 1
- Duplication Count: 0
- Critical Issues Count: 0

**Next Actions:** No CRITICAL/HIGH findings — implementation may proceed. C1 and I1 resolve inside WP04/WP05 with the one-line scope notes above; C2 and A1 are reviewer guidance; T1 needs nothing.
