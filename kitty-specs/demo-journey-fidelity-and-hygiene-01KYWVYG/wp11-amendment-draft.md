# WP11 staging draft — parent-artifact amendments pending hardware evidence

**Status: DRAFT. Not applied.** The parent mission's `acceptance-matrix.json`
and `mission-review.md` are deliberately left **byte-untouched** by WP11 so far.
They are the parent mission's acceptance record; writing TODO placeholders into
them would corrupt a machine-read acceptance authority and would look, to any
later reader, like a graded amendment. This file stages the amendments instead.
Apply them — with every TODO replaced by a measured value — only after the
physical run in T044 has actually happened.

Out-of-map rationale for the eventual edits (record with them):
"FR-006 amendment of parent acceptance record; kitty-specs paths are
non-declarable by rule."

---

## 1. Deterministic preflight (T044 step 1) — COMPLETE, on the merged lane-k tree

Run 2026-07-31 on the fully merged lane (WP01–WP10):

| Gate | Result |
| --- | --- |
| `cargo build --tests` | exit 0 (after the cross-lane fix below) |
| `cargo test --all-targets` | exit 0 — **533 passed / 0 failed** across 26 targets |
| `cargo test --release --test expandable_effects_and_bus_topology` | exit 0, `CREST_ACCEPTANCE expandable_effects_and_bus_topology passed` |
| `crest-synth-witness --case refused-topology --mutant none` | exit 0 |
| `crest-synth-witness --case refused-topology --mutant refused-topology-published` | exit 1 (mutant caught) |
| `scripts/check_no_name_enumerated_identity.sh` | exit 0, `CREST_STATIC_VALIDATION no_name_enumerated_identity passed` |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |

Release observation (verbatim):

```json
{"schemaVersion":2,"orderedSlotCasesExercised":3,"slotOrderExchangeDistinct":true,
"sameEntryInstancesIndependent":true,"clearedSlotPreservedHeldNotes":true,
"clearedSlotFocusRecovered":true,"addressableReturns":8,"defaultReturnOccupancyExact":true,
"maxOffTargetBusDbfs":-200.0,"mutedOrSoloExcludedWetContribution":0.0,
"unoccupiedReturnSilent":true,"returnContentChangeDryUninterrupted":true,
"topologyRejectionsExercised":4,"rejectionPreservedActiveGraph":true,
"rejectionReasonAttributable":true,"postRejectionValidChangeAccepted":true,
"partiallyAppliedTopologyBlocks":0,"registryEntryAdditionStructuralChanges":0,
"fourthEntryEndToEndExercised":true,"carryOverWrongEngineIdentityRefused":true,
"callbackAllocations":0,"callbackDeallocations":0,"callbackDestructions":0,
"retiredGraphsCollectedOffCallback":15,"activeNotesAtExit":0,"twoRunTraceEqual":true}
```

T043 hygiene greps re-confirmed on the merged tree:

- `grep -rn "WP0[0-9]\|WP10" src/ --include="*.rs"` → **0**
- `grep -rn "reverbSend" src/ tests/` → **1**, the guard fixture only
  (`tests/no_name_enumeration_guard.rs:236`), as T043 specifies
- `grep -in "aux bus" DESIGN.md` → **0**
- `grep -rn "post_effects()\|with_post_effects(" src/ tests/` → **0**

---

## 2. T045 — checkpoint-identity comparison method (BASELINE ESTABLISHED, comparison PENDING)

### Where the parent baseline actually lives

The parent acceptance matrix names two physical logs, `t052-run.log` and
`wp10-t059-live-run.log`. **Neither is committed to this repository, in any
branch, and neither exists on this host** (verified: working tree, `git log
--all --diff-filter=A`, and `git ls-files | grep '\.log$'` all empty). They were
cited by filename but never stored. A byte-level diff against those files is
therefore not possible and must not be claimed.

The durable, auditable parent baseline is instead frozen **in code** by WP01:

- `tests/effects_and_buses.rs:59` — `FROZEN_TOPOLOGY_IDENTITY_BASELINE: [&str; 17]`,
  documented as "the exact topology checkpoint identity sequence the phase gate
  emitted" at the pre-journey claim state.

Corroboration that the frozen 17 really is the parent physical evidence: the
parent matrix's SC-007 row records
`CREST_EFFECTS_AND_BUSES_LIVE_OBSERVATION ... topology_checkpoints=17` for
**both** 2026-07-31 physical runs. The array length and the recorded physical
count agree, so the frozen array faithfully stands in for the lost logs.

### The 17 baseline identities (must survive byte-identical, in this order)

```
Slot.startupOccupantCleared   SlotFill.first                SlotFill.second
SlotFill.third                SlotOrder.exchangeFirst       SlotOrder.exchangeSecond
SlotTwin.sameEntry            Send.towardDestinations       Return.contentChanged
Return.emptyOccupied          Topology.refused              Topology.recoveredAfterRefusal
Reroute.chainFollows          Slot.startupOccupantRestored  Slot.secondCleared
Slot.thirdCleared             Return.emptyRestored
```

### The 13 expected additions (declared, from `src/testing/live_effects_and_buses_scene.rs`)

`SlotOccupant.scalarEdited`, `SlotFill.secondCycle1`, `SlotFill.thirdCycle1`,
`SlotFill.thirdCycle2`, `Return.contentChangedCycle1`,
`Return.emptyOccupiedCycle1`, `Return.emptyOccupiedCycle2`,
`Topology.recoveredAfterRefusalCycle1`, `Slot.startupOccupantRestoredCycle1`,
`Slot.thirdClearedCycle1`, `Slot.thirdClearedCycle2`,
`Return.emptyRestoredCycle1`, `Return.emptyRestoredCycle2`

17 retained + 13 added = 30, matching the assertion
`scene.expected_topology_transitions().len() == 30` (`tests/effects_and_buses.rs:152`).

### How the physical comparison will be run (once the log exists)

```sh
LOG=<operator's captured run log>

# The frozen baseline, in order.
cat > /tmp/baseline-17.txt <<'EOF'
Slot.startupOccupantCleared
SlotFill.first
SlotFill.second
SlotFill.third
SlotOrder.exchangeFirst
SlotOrder.exchangeSecond
SlotTwin.sameEntry
Send.towardDestinations
Return.contentChanged
Return.emptyOccupied
Topology.refused
Topology.recoveredAfterRefusal
Reroute.chainFollows
Slot.startupOccupantRestored
Slot.secondCleared
Slot.thirdCleared
Return.emptyRestored
EOF

# 1. Emitted topology identity sequence, in emission order.
grep '^CREST_LIVE_CHECKPOINT ' "$LOG" \
  | sed 's/^CREST_LIVE_CHECKPOINT //' \
  | jq -r 'select(.kind=="topology") | .checkpoint.transition' \
  > /tmp/refreshed-identities.txt

# 2. Retained members, byte-for-byte and in order. Empty diff == 0 modified, 0 removed.
grep -Fxf /tmp/baseline-17.txt /tmp/refreshed-identities.txt > /tmp/retained.txt
diff /tmp/baseline-17.txt /tmp/retained.txt && echo "RETAINED OK: 0 modified, 0 removed"

# 3. Additions.
grep -Fxv -f /tmp/baseline-17.txt /tmp/refreshed-identities.txt > /tmp/additions.txt
wc -l < /tmp/additions.txt   # expect 13
```

The checkpoint JSON shape this relies on is `{"kind":"topology","checkpoint":
{"transition":"...", ...}}` (`src/testing/live_demo_checkpoint.rs:268,336`,
`#[serde(tag = "kind", rename_all = "camelCase")]`), emitted one per line by
`src/bin/crest_synth.rs:162`.

This is the physical twin of the deterministic assertions at
`tests/effects_and_buses.rs:161-171` (declared surface) and `:341-347` (emitted
surface): the identical filter-and-compare, run against the hardware log instead
of the in-process report.

- **TODO(physical): record `retained` diff result — expected 0 modified, 0 removed.**
- **TODO(physical): record the additions count and list — expected the 13 above.**
- **TODO(physical): record total emitted topology checkpoints — expected 30.**

A single modified or removed identity is a hard fail routed back to WP01.

---

## 3. T046 — DRAFT amendment to parent `acceptance-matrix.json`

Append-only. Add these rows **after** the existing 35 `criteria` entries; change
no existing row, grade, or `verified_at`. Suggested `criterion_id` suffix
`-AMEND-01KYWVYG` keeps them unmistakably distinct from the parent's rows.

```jsonc
{
  "criterion_id": "FR-019-AMEND-01KYWVYG",
  "description": "Retained live scene demonstrates the PATCH/MIXER player journey on hardware (supersession of the DRIFT-6 'inadequate for the player journey' note)",
  "proof_type": "manual_qa",
  "evidence": "TODO(physical): refreshed physical run of 'make demo-live-effects-and-buses' — capture path/filename, run date, checkpoint total, CREST_LIVE_SUMMARY line verbatim, and the CREST_EFFECTS_AND_BUSES_LIVE_OBSERVATION line verbatim. Journey observed on screen: focus landing on each PATCH slot row before its occupancy cycles, the audible occupant edit from PATCH, the MIXER return-row walks, and the documented rejection's visible reason. Exactly three product effects visible (the fourth registry entry is test-only).",
  "pass_fail": "TODO(physical)",
  "verified_by": "TODO(physical): operator who ran the rig",
  "verified_at": "TODO(physical): ISO-8601 timestamp of the run",
  "notes": "Amends, does not replace, the parent FR-019 row. The parent row's cited logs (t052-run.log, wp10-t059-live-run.log) are not stored in the repository; this amendment stores its evidence per the same convention and records that gap."
},
{
  "criterion_id": "SC-003-AMEND-01KYWVYG",
  "description": "Add-only checkpoint-identity contract proven against the frozen pre-journey baseline",
  "proof_type": "manual_qa",
  "evidence": "Baseline: FROZEN_TOPOLOGY_IDENTITY_BASELINE (17 identities, tests/effects_and_buses.rs:59), corroborated by the parent's recorded topology_checkpoints=17. Comparison method: WP11 staging draft section 2. TODO(physical): record result — expected 0 modified, 0 removed, 13 added.",
  "pass_fail": "TODO(physical)",
  "verified_by": "TODO(physical)",
  "verified_at": "TODO(physical)",
  "notes": "Deterministic twin asserted at tests/effects_and_buses.rs:161-171 and :341-347, green on the merged tree."
},
{
  "criterion_id": "NFR-004-AMEND-01KYWVYG",
  "description": "Zero dropouts/underruns/discontinuities across the reworked live scene",
  "proof_type": "manual_qa",
  "evidence": "TODO(physical): droppedRecords from CREST_LIVE_EVENT_LOG_SUMMARY (expect 0); count of checkpoints with audio_uninterrupted=false (expect 0); checkpoint completeness ratio.",
  "pass_fail": "TODO(physical)",
  "verified_by": "TODO(physical)",
  "verified_at": "TODO(physical)",
  "notes": null
},
{
  "criterion_id": "NFR-006-AMEND-01KYWVYG",
  "description": "Clean teardown at scene end after the journey rework",
  "proof_type": "manual_qa",
  "evidence": "TODO(physical): cleanup, activeNotes, window_closed, stream_released, owned_graphs_remaining, active_notes_after_cleanup, exit status. Deterministic half re-measured on the merged tree: activeNotesAtExit=0 and retiredGraphsCollectedOffCallback=15.",
  "pass_fail": "TODO(physical)",
  "verified_by": "TODO(physical)",
  "verified_at": "TODO(physical)",
  "notes": "The parent NFR-006 row cites retiredGraphsCollectedOffCallback=8. That value moved to 15 under WP09 (seven further structural changes); the predicate is 'gt 0', so it still passes. This is the ONLY pre-existing numeric that changed."
},
{
  "criterion_id": "SC-007-AMEND-01KYWVYG",
  "description": "Reworked live scene completes with every declared behavior demonstrated, 0 active notes, released resources, normal exit",
  "proof_type": "manual_qa",
  "evidence": "TODO(physical): CREST_LIVE_SUMMARY verbatim and confirmation that CREST_EFFECTS_AND_BUSES_LIVE_OBSERVATION carries zero false keys; expected topology_checkpoints=30 (parent recorded 17).",
  "pass_fail": "TODO(physical)",
  "verified_by": "TODO(physical)",
  "verified_at": "TODO(physical)",
  "notes": null
},
{
  "criterion_id": "SC-008-AMEND-01KYWVYG",
  "description": "Adding a further registry effect is graded on demonstration, not structural inference",
  "proof_type": "automated_test",
  "evidence": "Merged-tree release run: fourthEntryEndToEndExercised=true and registryEntryAdditionStructuralChanges=0 with schemaVersion 2, satisfying the crest-spec open_effect_registry step-1 'observes' clause verbatim. The fourth entry is test-only (tests/expandable_effects_and_bus_topology.rs); the production composition root still builds the three-entry registry, so the diff to production is zero.",
  "pass_fail": "pass",
  "verified_by": "implementer-ivan (WP11 preflight, merged lane-k tree)",
  "verified_at": "2026-07-31",
  "notes": "Closes parent open item 4. Does not depend on the physical run; the physical run must still show exactly three product effects on screen — a visible fourth is a defect."
}
```

`overall_verdict` stays `"pass"`. Do not touch it.

---

## 4. T047 — DRAFT amendment to parent `mission-review.md`

Append at end of file (after "Retrospective Reminder"), rewriting nothing above.

```markdown
## Addendum 2 (TODO(physical): date) — DRIFT-6 resolution and open-item disposition

Mission `demo-journey-fidelity-and-hygiene-01KYWVYG` reworked the retained scene
and swept this review's open items. WP01–WP10 are approved and merged; WP11
carries the evidence gate.

### Scene rework

Every slot and return occupancy change now dispatches the adjacent-choice
gesture behind a focus-verified journey to the exact row; the scene's declared
topology transitions grew 17 → 30. The single surviving direct injection is the
documented rejection (`Topology.refused`) — the UI cannot request an unknown
registry entry by design — asserted as the only one in
`tests/effects_and_buses.rs`.

### Refreshed physical evidence

- TODO(physical): run date, host, audio device, MIDI fixture identifiers.
- TODO(physical): checkpoint completeness (expect 100%), droppedRecords (expect 0),
  false observation keys (expect 0), teardown, exit status.
- TODO(physical): confirmation that exactly three product effects were visible.
- TODO(physical): identity comparison — expect 0 modified, 0 removed, 13 added
  (method: WP11 staging draft section 2).

### FR-019 / C-010 grading

TODO(physical): once the figures above are recorded, state that the DRIFT-6
"superseded: inadequate for the player journey" note is resolved and the
FR-019/C-010 RECORDED-MANUAL grading is restored to adequate. **Do not write this
sentence before the run.**

### Open items 1–7 — disposition

| # | Item | Disposition | Closing WP | Proof pointer |
| --- | --- | --- | --- | --- |
| 1 | DRIFT-1 compact view | CLOSED | WP02–WP05 | `grep -rn "post_effects()\|with_post_effects(" src/ tests/` → 0 hits. `Patch::with_effect_slot` (`src/synth/patch.rs:180`) is the position-explicit replacement; `PatchInput::post_effects()` (`src/control/event_record.rs:190`) survives only as frozen serialized vocabulary, as T043 sanctions. |
| 2 | DRIFT-2 startup fallback | CLOSED | WP04 | `production_startup_bus_returns` (`src/adapter/production_effects.rs:93`) returns `Result`; the production root consumes it at `src/shell/standalone_application.rs:737` and propagates `ApplicationError::DefaultBusReturns`. The `unwrap_or_default` survives only in the documented TEST-only `startup_bus_returns`, which no production path calls. |
| 3 | RISK-2 twin test | CLOSED | WP07 | `tests/topology_change_lifecycle.rs:1205` `return_clear_held_note_continuity_preserves_held_voices_sample_exactly`. |
| 4 | SC-008 fixture (optional) | CLOSED — delivered, not deferred | WP09 | Release observation `fourthEntryEndToEndExercised: true` (schemaVersion 2); fourth entry is test-only, production registry unchanged. |
| 5 | DRIFT-3/4/5 cleanups | CLOSED | WP06, WP10 | Four T043 greps: WP-comments 0; `reverbSend` only the guard fixture (`tests/no_name_enumeration_guard.rs:236`); "aux bus" in `DESIGN.md` 0; compact-view symbols 0. DRIFT-4: `src/testing/live_demo_report.rs` measurements are `Option`-typed and distinguish absent from measured (`:794`, `:811`, `:845`); no `unwrap_or(0)` remains. |
| 6 | RISK-1 layout hardening (optional) | CLOSED — delivered, not deferred | WP08 | `PreparedGraphLayout::effect_capability_identity` (`src/real_time/prepared_graph.rs:435`); exercised by `measure_carry_over_identity_refusal`, surfaced as `carryOverWrongEngineIdentityRefused: true`. |
| 7 | Guard tool gating | CLOSED | WP10 | `require_tools` (`scripts/check_no_name_enumerated_identity.sh:75-89`) exits **3** naming each missing tool, with an explicit "a missing tool is a failure, never a pass" message; covered by `tests/no_name_enumeration_guard.rs`. |

All seven closed; none deferred. SC-007's deferral allowance (items 4 and 6) was
not needed — both optional items were delivered.

### Deterministic re-verification on the merged tree

533 passed / 0 failed across 26 targets; clippy and fmt clean; witness binary
exits 0 clean and 1 under the `refused-topology-published` mutant; guard script
exits 0 with its marker. Observation `schemaVersion: 2`,
`fourthEntryEndToEndExercised: true`, `carryOverWrongEngineIdentityRefused: true`,
`twoRunTraceEqual: true`, `retiredGraphsCollectedOffCallback: 15` (was 8; only
pre-existing numeric that moved; predicate `gt 0` still holds).
```

---

## 5. Evidence-storage gap to settle at T044 time

The parent stored no run logs, only filenames. To make this amendment auditable,
the refreshed log should be **committed**, not merely cited. Proposed path,
following the parent's naming convention:
`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/evidence/wp11-t044-live-run.log`.
Confirm with the coordinator before committing, since `.gitignore` excludes
`.kittify/logs/` but not this path.
