---
work_package_id: WP10
title: Hygiene sweep & guard gating
dependencies:
- WP05
requirement_refs:
- FR-011
- FR-012
- FR-013
- FR-014
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T039
- T040
- T041
- T042
- T043
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-06/IC-07 (DRIFT-3/5 + guard security note)
agent_profile: implementer-ivan
authoritative_surface: scripts/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- scripts/check_no_name_enumerated_identity.sh
- tests/no_name_enumeration_guard.rs
- DESIGN.md
- src/control/state_tree.rs
- src/control/app_loop.rs
- src/mixer/bus_return.rs
- src/mixer/global_parameters.rs
- src/mixer/mix_engine.rs
- src/mixer/mixer_track_parameters.rs
- src/real_time/audio_renderer.rs
- src/synth/effect_capability.rs
- src/testing/behavioral_mutation_harness.rs
- src/testing/live_demo_checkpoint.rs
- src/testing/live_demo_scene.rs
- src/testing/live_mixer_routing_measurement.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP10 – Hygiene sweep & guard gating

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Close the parent review's small-but-real hygiene items: gate the
name-enumeration guard script on its tool dependencies (vacuous-gate security
note); replace the leftover `reverbSend` fixture literals; correct the
`DESIGN.md:204` "aux buses" wording; sweep the stale WP-numbered handoff
comments in the enumerated remainder files; and run the repo-wide final
verification greps after the migration WPs land.

## Context

- Guard today: `scripts/check_no_name_enumerated_identity.sh` masks missing
  `rg`/`perl` as "no candidates" via `|| true` — a missing tool reads as a
  pass. Crest-spec now declares (validation description +
  `asset.ValidationScripts`): the script verifies its required tools first
  and exits non-zero NAMING the missing tool. The in-process backstop
  (`tests/no_name_enumeration_guard.rs`, incl. `--self-test`) stays.
- Occurrence-map protections you must respect: the `reverbSend` literal in
  `tests/no_name_enumeration_guard.rs:178` is a DELIBERATE positive-detection
  fixture — preserved; parent-mission history under `kitty-specs/…01KYNGX8/`
  is never term-scrubbed; the crest-spec is not edited from here.
- Comment cleanup division of labor: WPs 01–08 clean their own files. This WP
  owns the remainder (the 11 enumerated source files) and the final
  repo-wide verification (hence the WP05 dependency — run greps against the
  fully migrated tree).

## Subtasks

### T039 — Guard script tool gating + guard-test coverage

**Steps**:
1. At the top of `scripts/check_no_name_enumerated_identity.sh`, verify every
   required external tool (`command -v rg`, `command -v perl`, plus anything
   else the script invokes); on absence, print an error NAMING the tool and
   exit non-zero. Remove/replace the `|| true` constructs that convert tool
   failure into "no candidates".
2. Distinguish exit meanings: missing tool ≠ "candidates found" ≠ clean pass
   — keep the declared healthy-path contract byte-compatible (exit 0 +
   `CREST_STATIC_VALIDATION no_name_enumerated_identity passed`).
3. Extend `tests/no_name_enumeration_guard.rs` to cover the gate (e.g., run
   the script with a PATH lacking the tool and assert non-zero exit naming
   it), alongside the existing `--self-test` coverage.

**Validation**: healthy run unchanged; `PATH=/usr/bin:/bin` (or equivalent
tool-less environment) run exits non-zero naming the missing tool; guard
test covers it.

### T040 — Replace reverbSend fixture literals (guard fixture preserved)

**Steps**:
1. Replace the two leftover literals at `src/control/state_tree.rs:1389` and
   `:1593` (`"PATCH Lead\n> reverbSend=0.4\nGLOBAL"`) with canonical
   vocabulary (e.g., an existing indexed-send or masterGainDb-style label the
   projection actually produces today) — the tests' intent (formatting/shape
   assertions) must be preserved.
2. Do NOT touch `tests/no_name_enumeration_guard.rs:178` — that literal is
   the guard's positive-detection fixture (occurrence-map exception).

**Validation**: `grep -rn "reverbSend" src/ tests/` matches only the guard
fixture; state_tree tests green with equally strong assertions.

### T041 — DESIGN.md:204 "aux buses" → bus-return vocabulary

**Steps**:
1. Rewrite the sentence at `DESIGN.md:204` ("…voices, post-FX slots, tracks,
   aux buses, events…") using the canonical bus-return vocabulary; keep the
   sentence's meaning (bounded render complexity) intact.
2. Scan DESIGN.md for any other "aux bus" residue while there.

**Validation**: `grep -in "aux bus" DESIGN.md` → no output.

### T042 — Stale WP-comment sweep in enumerated remainder files

**Steps**:
1. Sweep the 11 owned source files (planning-time counts:
   `src/control/app_loop.rs` 1, `src/mixer/bus_return.rs` 1,
   `global_parameters.rs` 1, `mix_engine.rs` 2, `mixer_track_parameters.rs`
   4, `src/real_time/audio_renderer.rs` 4, `src/synth/effect_capability.rs`
   1, `src/testing/behavioral_mutation_harness.rs` 4,
   `live_demo_checkpoint.rs` 5, `live_demo_scene.rs` 6,
   `live_mixer_routing_measurement.rs` 1).
2. Judgment rule (spec FR-011): delete pure timeline narration ("added in
   WP04", "WP06 will retire this"); rewrite comments that carry a genuine
   constraint in durable, mission-agnostic language. At least two parent
   comments are factually false already (they defer to WPs that shipped) —
   falsity is the reason this sweep exists.

**Validation**: grep for `WP0[0-9]|WP10` in the 11 files → no output; no
constraint knowledge lost (reviewer judges).

### T043 — Repo-wide final hygiene verification greps

**Steps**:
1. After WP05 is merged into your lane's base (dependency), run and record
   in the WP notes:
   - `grep -rn "WP0[0-9]\|WP10" src/ --include="*.rs"` → empty
   - `grep -rn "reverbSend" src/ tests/` → only the guard fixture
   - `grep -in "aux bus" DESIGN.md` → empty
   - `grep -rn "post_effects()\|with_post_effects(" src/ tests/` → empty
2. Any hit outside your ownership: report it against the owning WP (do not
   fix cross-ownership from here).

**Validation**: all four greps clean (or reported); `cargo test
--all-targets` green.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. This WP depends on WP05; the runtime's
computed lane carries the dependency commits per `lanes.json`.

## Test Strategy

`bash scripts/check_no_name_enumerated_identity.sh` (healthy + tool-less
runs), `cargo test --test no_name_enumeration_guard`,
`cargo test --all-targets`, plus the four verification greps.

## Definition of Done

- Guard fails loudly (non-zero, tool named) without its tools; healthy
  output byte-compatible; guard test covers the gate.
- reverbSend only in the guard fixture; "aux buses" gone from DESIGN.md.
- Zero stale WP comments repo-wide; verification greps recorded.

## Reviewer Guidance

- Try the guard yourself in a tool-less PATH — the exit code and message are
  the deliverable.
- Diff T040 carefully: the replacement label must be something the
  projection really emits (no invented vocabulary).
- Spot-check comment deletions against the parent WP files: if a deleted
  comment encoded a real constraint, it must reappear in durable form.
