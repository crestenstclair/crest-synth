---
work_package_id: WP06
title: 'Hardware evidence: scenes, RT A/B, soak'
dependencies:
- WP03
requirement_refs:
- C-007
- FR-003
- NFR-001
- NFR-002
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
subtasks:
- T022
- T023
- T024
- T025
history:
- '2026-08-06: authored from plan IC-03 (evidence half); C-007 evidence wall'
agent_profile: implementer-ivan
authoritative_surface: kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/
create_intent:
- scripts/rt_ab_measurement.sh
- kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/README.md
execution_mode: code_change
owned_files:
- kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/**
- scripts/rt_ab_measurement.sh
role: implementer
tags: []
tracker_refs: []
---

## ⚡ Do This First: Load Agent Profile

Before reading anything else in this prompt, load your assigned profile:

```
/ad-hoc-profile-load implementer-ivan
```

Adopt its identity, boundaries, and governance scope for the duration of this
work package.

## Objective

Build the C-007 evidence wall: with both shells still in the tree, take the
same-workload RT A/B measurement, run all four retained live scenes through
the webview shell on the physical rig, run the 300 s soak, and commit every
log under `kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/`. WP07's
deletion is forbidden until these commits exist. This machine IS the rig —
run the targets yourself with the real window and physical audio output; a
headless, silent, or mocked substitute satisfies nothing (ROADMAP live-demo
requirement). Never fabricate or trim a log: a failed run is committed as a
failed run and reported.

Authorities: spec FR-003/NFR-001/NFR-002/C-004/C-007, requirement
`serialized_projection_transport` (RT bounds unchanged from the recorded
pre-cutover baseline; 30 Hz meters over five minutes), ROADMAP gate terms,
the foundation review's RISK-3 (literal same-workload A/B) and RISK-4 (soak).

## Context

- Prerequisite: WP03 merged — all four `make demo-live-<scene>` targets run
  webview-hosted and its local smoke pass succeeded. Do NOT start hardware
  runs until the deterministic twins are green at your lane's HEAD.
- Evidence precedent: `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/evidence/wp11-t044-live-run.log`
  (committed complete log, not a citation) and its identity-comparison
  addendum. Match that bar.
- The RT measurement fields already exist in live reports (e.g.,
  `frames_to_projection_max`, `render_blocks_to_audible_max`), measured, not
  defaulted — distinguishing absent from zero is established behavior.

## Subtasks

### T022 — RT A/B same-workload measurement

**Purpose**: the webview shell adds no real-time callback work (NFR-001);
this is the last moment both shells exist to compare.

**Steps**:
1. Write `scripts/rt_ab_measurement.sh`: run the same live scene workload
   (`--demo-live-sixteen-track-mixer-routing` is the steadiest sustained
   load) once under the egui shell and once under the webview shell,
   collecting the RT-relevant measured fields from each run's structured
   output (callback timing bounds, `audio_uninterrupted` counts,
   render-blocks-to-audible) plus process-level CPU of the audio thread if
   the existing observation carries it. No new RT instrumentation — read
   what the production reports already measure.
2. Run it on this machine, physical audio device, no other heavy load.
3. Record both raw logs plus a comparison summary
   (`evidence/rt-ab-comparison.md`): field-by-field, with the conclusion
   stated as measured numbers, not adjectives. The acceptance bar: webview
   bounds within the egui baseline's envelope; zero
   `audio_uninterrupted=false` in both.

### T023 — Four scene hardware runs

**Purpose**: retained evidence survives the cutover (FR-003, gate term).

**Steps**:
1. In order, run each target with the real window and physical audio, piping
   the complete structured output to a log:
   `make demo-live-graphical-shell`, `make demo-live-semantic-view-model`,
   `make demo-live-sixteen-track-mixer-routing`,
   `make demo-live-effects-and-buses`.
2. Each run must show: process exit 0; its scene's declared checkpoint
   completeness; zero checkpoints with `audio_uninterrupted=false`; clean
   teardown fields (`cleanup=true`, `activeNotes=0`, `window_closed=true`,
   `stream_released=true`, `owned_graphs_remaining=0` or the scene's
   equivalents); qualifying webview frame counts from the WP02 forwarding
   (nonzero, correlated with checkpoints).
3. Commit each complete log as
   `evidence/<scene>-live-run.log`. A failed run: commit the log, diagnose,
   fix (in the owning WP's territory via lane notes if not yours), re-run,
   commit the passing log alongside — never overwrite a failure silently.

### T024 — 300 s soak

**Purpose**: NFR-002 / foundation RISK-4.

**Steps**:
1. `CREST_WEBVIEW_FULL_SOAK=1` with the existing soak entry point (see
   foundation NFR-002 notes; the 60 s structural-bound run exists — the full
   soak flag was left for this successor).
2. Record: no leak growth trend across the window (the soak's own measured
   fields), `droppedRecords=0`/lossless where reported, meter cadence
   sustained, clean teardown, exit 0.
3. Commit `evidence/soak-300s.log` plus two lines of summary in the evidence
   README.

### T025 — Evidence summary and identity comparison

**Purpose**: one page a reviewer reads to trust the wall; the add-only
contract proven on hardware, not only in twins.

**Steps**:
1. `evidence/README.md`: table of runs (scene, date, exit code, checkpoint
   completeness, audio-uninterrupted count, teardown verdict, log path,
   commit).
2. Identity comparison on hardware logs: for each scene with a frozen
   baseline (notably `FROZEN_TOPOLOGY_IDENTITY_BASELINE`), compare emitted
   checkpoint identities: N/N baseline preserved byte-identically and in
   order, 0 modified, 0 removed, additions listed as pure insertions.
   Record the counts per scene in the README.
3. State explicitly in the README that these commits precede the egui
   deletion (C-007) with the evidence commit hashes — WP07 links to this.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-cutover`.
Execution worktrees are allocated per computed lane from `lanes.json`; enter
the lane workspace `spec-kitty agent action implement WP06 --agent claude`
gives you.

## Definition of Done

- Six committed artifacts minimum: four scene logs, RT A/B logs+comparison,
  soak log, plus `evidence/README.md`.
- Every passing run: exit 0, zero audio interruptions, clean teardown,
  nonzero qualifying webview frames.
- Identity comparison recorded per scene: 0 modified, 0 removed.
- Failures (if any) committed and narrated, never replaced silently.

## Reviewer Guidance

- Open the raw logs, not just the README — spot-check teardown fields and
  one checkpoint sequence against the scene's declared protocol.
- Verify the A/B ran the SAME workload both sides (scene name, fixture,
  duration in the logs).
- Check log timestamps against commit order — evidence must predate the
  WP07 deletion commit.
- Absent-vs-zero: any measurement field that reads 0 should be provably
  measured (the live-report schema distinguishes these — confirm).
