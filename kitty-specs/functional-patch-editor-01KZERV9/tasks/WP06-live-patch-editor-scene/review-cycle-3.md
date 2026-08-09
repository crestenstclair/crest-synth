---
affected_files:
- src/testing/functional_patch_editor_observation.rs
- src/testing/live_demo_report.rs
- ROADMAP.md
cycle_number: 3
mission_slug: functional-patch-editor-01KZERV9
reproduction_command: 'cargo test --all-targets'
reviewed_at: '2026-08-09T20:40:00Z'
reviewer_agent: claude
verdict: approved
wp_id: WP06
---

# WP06 review cycle 2 — APPROVED

The one required change landed, and it landed the way the rejection asked: the
verdict is hoisted to its own field, the predicate asserts the claim rather than
a proxy for it, and both raw deltas stay reported beside it. Every claim below
was executed. Lane `kitty/mission-functional-patch-editor-01KZERV9-lane-f`,
content commit `b2dce57`.

## R1-R4 verified

**R1.** `audible_edit_isolated_to_second_patch: bool` sits at
`src/testing/functional_patch_editor_observation.rs:96`, immediately after
`first_patch_audible_edit_delta`, matching the declared schema order. It is
computed in `resolve()` (line 729) as
`delta_on(second) - delta_on(first) >= AUDIBLE_EDIT_DELTA_MARGIN`.
`shortfalls()` asserts the field and names it (lines 205-208); the
`first_patch_audible_edit_delta` requirement is gone. `shortfalls()` now emits
41 names, identical in **content and order** to the witness's 41 `- field:`
predicates. The one schema field with no predicate is
`first_patch_audible_edit_delta`, exactly as the amendment intended.

**R2/R3.** `WITNESS_SCHEMA_FIELDS` is `[&str; 42]` and equals
`witness.functional_patch_editor`'s declared schema **order-exactly**, checked
against the live YAML in the lane rather than against the array's own copy. The
struct's field order minus `schema_version` is the same list. The amended
witness is in this lane.

**R4.** The ROADMAP note's closing paragraph is replaced with what is now true.

## Falsified, not read

Both reported falsifications reproduce verbatim, and neither message was
paraphrased:

- `#[serde(rename = "audibleEditIsolatedToSecondPatch")]` on the new field →
  `the emitted observation must carry the declared witness field
  audible_edit_isolated_to_second_patch` (`observation.rs:1126`).
- reverting `WITNESS_SCHEMA_FIELDS` to the 41-entry list →
  `the emitted observation carries audible_edit_isolated_to_second_patch, which
  the witness does not declare` (`observation.rs:1132`).

The emitted JSON was dumped and compared against the live witness YAML directly,
not against the pin array: 43 keys, of which 42 are the declared set exactly
(no extras, none missing) and the 43rd is `schema_version`. The pin's exception
is the only exception.

**The absent-evidence property is asserted, not accidental.** Widening the
isolation comparison so that `0.0 - 0.0` clears it fails
`work_done_on_the_first_patch_credits_nothing_to_the_second` at
`assert!(!observation.audible_edit_isolated_to_second_patch())`
(`observation.rs:959`). A run with no edit on the second Patch resolves `edit`
to `None`, reports both deltas at zero, and reads as "not isolated" — not as
isolation by default.

## F-61's correction is upheld; my cycle-1 finding had the polarity backwards

Both directions were run here.

**Narrowing** the gate to a never-matching literal fails
`tests/effects_and_buses.rs:390` — `the cumulative scene retains
effects-and-buses evidence` — while `live_demo_scene` (2), `live_patch_editor_scene`
(6) and `topology_change_lifecycle` (11) stay green. The suites my cycle-1 sweep
listed do stay green; the suite that owns the phase was not among them, and it
fails. That direction was already covered.

**Widening** it to always-true leaves all four suites green:
`effects_and_buses` 1, `live_demo_scene` 2, `live_patch_editor_scene` 6,
`topology_change_lifecycle` 11. That was the genuinely open direction, and it is
the one that breaks this mission's scene.

Both pins fail on their own direction:
`effects_and_buses_evidence_is_gated_to_the_scene_that_declares_the_phase` fails
at `live_demo_report.rs:2310` under narrowing and at `:2314` under widening. The
companion test `a_non_effects_scenes_topology_checkpoints_would_fail_the_bus_contract_ungated`
shows why the gate is load-bearing rather than tidy.

## ROADMAP note

Claims nothing the evidence does not show. Field count 42;
`audibleEditIsolatedToSecondPatch` is in the unmeasured list;
`secondPatchAudibleEditDelta = 0.0` is named as a synthetic unit-test
measurement rather than a live number; `midiInputRechannelled` is graded as
"the row is Patch-local and re-projects across a switch" with the name recorded
as broader than the measurement. **LIMIT-1's second bullet is unstruck** and
carries `STILL OPEN (2026-08-09) — BUILT AND UNRUN`.

## Execution

`cargo test --all-targets` in the lane: **30 suites, 821 passed, 0 failed**,
lib **718**, real `CARGO_EXIT=0` (no pipe). `cargo clippy --all-targets` clean.
`make -n demo-live-patch-editor` exits 0, `make help` lists it, `demo-live`
still aliases `demo-live-effects-and-buses`.

One run of three hit `tests/input_capture_witness.rs:428` (43 of 46
transitions). That is F-12: the file is untouched by this lane (last changed by
`kitty/mission-webview-shell-cutover-01KZAC7Q`), and a subsequent run reported
`CREST_KEY_WITNESS_PARTIAL … the witness window never held key focus in this
environment`. Not a WP06 regression.

F-62's field-ordering fix is on `feat/functional-patch-editor` and not yet in
this lane; `contexts/testing.yaml` lists the field in a different position there.
`proof/witnesses.yaml` — the declaration the pin is checked against — is
identical on both, so nothing fails either way.

## Carried to the accept gate as stated limits, not passing predicates

The live witness is **built and unrun**. Unexecuted: every predicate needing a
painted frame (`stripGroupsPainted`, `stripFlatControlRun`,
`qualifyingWebviewFrames`, `desktopViewportPainted`, `physicalAudioNonzero`,
`secondPatchAudibleEditDelta`, `firstPatchAudibleEditDelta`,
`audibleEditIsolatedToSecondPatch`, `checkpointsCorrelatingSwitchFocusAudio`,
`voiceLimitRefusals`, `projectionGenerationGaps`, and the teardown quartet
`activeNotesAfterCleanup` / `windowClosed` / `streamReleased` /
`ownedGraphsRemaining`); the observation marker has never been emitted by a
completed run; `AUDIBLE_EDIT_DELTA_MARGIN = 1.0e-3` remains a declaration
(F-63: it now stands behind a witness predicate, so the first completed run
either confirms it or moves a predicate); the controlled negative's exit code
proves nothing under F-40 and its falsifying power rests on the headless keying
tests, the plan→runner→resolve join being live-only; `midiInputRechannelled` is
graded narrower than its name (F-60).
