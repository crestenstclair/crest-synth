---
affected_files: []
cycle_number: 2
mission_slug: functional-patch-editor-01KZERV9
reproduction_command:
reviewed_at: '2026-08-09T13:03:12Z'
reviewer_agent: unknown
verdict: rejected
wp_id: WP06
---

# WP06 review cycle 1 — REJECTED for one required change

The package is otherwise sound and its self-reported limits are honest. Every
substantive claim it makes was reproduced by execution rather than read. It is
rejected for exactly one thing: the F-57 witness amendment has not landed in the
observation, and without it the emitted report cannot satisfy the amended witness.

---

## Required (blocking)

### R1 — Expose `audible_edit_isolated_to_second_patch`

`witness.functional_patch_editor` on `feat/functional-patch-editor` now declares a
**42**-field schema. `first_patch_audible_edit_delta` remains in the schema as a
reported number but **no longer carries a predicate**; it is replaced by:

```yaml
        audible_edit_isolated_to_second_patch: bool
...
    - field: audible_edit_isolated_to_second_patch
      op: eq
      value: true
```

`.kittify/crest-spec/contexts/testing.yaml` declares the same field as
`audibleEditIsolatedToSecondPatch: bool`.

`FunctionalPatchEditorObservation` carries 41 fields and no such field anywhere in
the lane (`grep audible_edit_isolated_to_second_patch src/ tests/` → no match). The
emitted JSON would therefore have nothing for the predicate to read, and acceptance
fails on a missing field rather than on the claim.

The verdict is already computed — it is inline in `shortfalls()` at
`src/testing/functional_patch_editor_observation.rs:199-205`:

```rust
        require(
            self.second_patch_audible_edit_delta - self.first_patch_audible_edit_delta
                >= AUDIBLE_EDIT_DELTA_MARGIN,
            "first_patch_audible_edit_delta",
        );
```

That is the right comparison reported under the wrong name. Hoist it:

1. Add `audible_edit_isolated_to_second_patch: bool` to the struct, positioned after
   `first_patch_audible_edit_delta` to match the declared schema order.
2. Populate it in `resolve()` from
   `delta_on(second) - delta_on(first) >= AUDIBLE_EDIT_DELTA_MARGIN`, so the field
   carries the verdict and `shortfalls()` merely asserts it.
3. Change the `require(...)` above to assert the new field and name the failure
   `"audible_edit_isolated_to_second_patch"`. Drop the
   `first_patch_audible_edit_delta` requirement entirely — the amended witness has no
   predicate on it, and the negative's recorded failing set must name predicates that
   actually exist.
4. Keep both raw deltas reported. The amendment did not remove them.

### R2 — Update the schema pin to 42 fields

`WITNESS_SCHEMA_FIELDS` is `[&str; 41]` and the pin test asserts exhaustiveness in
both directions — every declared field present, and **no key beyond
`schema_version`**. As written it would reject R1's new field. Update the array to
the amended 42-field list in declared order.

The pin is good and it discriminates (verified below); it is now simply pinned to a
stale declaration.

### R3 — The amended witness is not in this lane

`audible_edit_isolated_to_second_patch` exists only on `feat/functional-patch-editor`.
`kitty/mission-functional-patch-editor-01KZERV9` and both `lane-e` and `lane-f` still
carry the pre-amendment witness. Bring the amended crest-spec into the lane so R2's
pin is checked against the live declaration rather than against a copy of the old one.

### R4 — The ROADMAP note's last paragraph is now stale

The note ends with "**One witness predicate disagrees with its own ruling.** …The
witness YAML needs the same amendment its finding already made." That amendment has
since landed. Replace the paragraph with the fact that the witness now asserts the
bounded verdict directly and the observation carries it.

---

## Verified by execution — no action needed

Recorded so the re-review does not repeat it.

**The defeated journey genuinely discriminates.** Falsified, not read. Mutating
`resolve()`'s second-Patch keying to count work regardless of where it landed —

```rust
let second_slots = self.slots_visited.values().map(BTreeSet::len).max().unwrap_or(0) as u32;
```

— makes `work_done_on_the_first_patch_credits_nothing_to_the_second` fail with
`left: 3, right: 0`. That is precisely the defect F-47 exists to prevent, and the
shipped keying catches it. The keying chain is sound end to end: the runner records
`patch_id` from `app_loop.state().interaction().focus_path().patch_id()`
(`live_demo_runner.rs:2132-2133`) — the production reducer's live focus, not the
plan's declared target — and `resolve()` keys "second" from the final installed order
passed in at `standalone_application.rs:235`. A defeated run therefore records its
three slots and its audible edit against Patch 1 and credits zero.

One caveat, not blocking: the two halves are proven separately — the plan-level test
proves the defeated scene targets Patch 1, the unit test proves such a measurement
credits zero — and the join is only exercised live. The code path is correct by
inspection; there is no headless test that drives the defeated plan through the
runner.

**The schema pin discriminates.** Adding `#[serde(rename = "desktopViewportPainted")]`
to one field — exactly what T035's camelCase instruction would have produced — fails
the pin: *"the emitted observation must carry the declared witness field
desktop_viewport_painted"*. Following the witness over T035 was the right call and the
pin is real.

**F-44 is satisfied.** The grouping rule exists only in `webview-page/page.js`
(`querySelectorAll(".pgroup")`). Every Rust "group" token in the diff is either
transport (`measured_strip` parsing two leaves), a field name, or a doc comment. There
is no second implementation. `measured_strip` correctly types a present-but-malformed
`strip` as a malformed ack rather than a silent zero, and
`absent_strip_evidence_fails_rather_than_reading_as_grouped` pins that absent evidence
reports the flat run rather than the clean state.

**F-49's gate does not weaken the effects scene.** `complete` uses
`effects_and_buses.as_ref().is_none_or(is_complete)`, and for
`EFFECTS_AND_BUSES_SCENE_NAME` the gate is true, so that scene takes a byte-identical
path. Confirmed by inspection of `live_demo_report.rs:1216-1236`.

**The environmental claim reproduces.** Both runs executed here:

- `./target/release/crest-synth --demo-live-effects-and-buses` (the **shipped**
  control) → exit 1, `no progress while awaiting parameter projection paint
  confirmation at step Some(3) with predicate Some(OutputLevel) for 10.0 seconds`.
- `--demo-live-patch-editor` → exit 1, **identical** step, predicate, and timeout.
- `--demo-live-patch-editor --defeat-patch-selection` → exit 1, same failure.
- `--defeat-patch-selection` alone → exit 1, `requires --demo-live-patch-editor`.
- `make -n demo-live-patch-editor` resolves; `demo-live` still aliases
  `demo-live-effects-and-buses`.

The blocker is the environment, not this scene. That now rests on reproduction rather
than on one agent's word.

**NFR-004 reproduces in release.** MIXER Main 89 rows: **2370 µs** measured here
against ROADMAP's 2382 µs, bar 3.00 ms. PATCH Main 15 rows: **470 µs**, matching
exactly.

**23 new tests** — the count is accurate (18 + 4 across the two commits, plus the
integration file). All green: 6 integration, 10 observation unit, 715 lib total.

**No WP05 collision.** `git merge-tree` of lane-e and lane-f yields exactly one
`page.js` conflict, at the `requestedLabel` / F-33 region that is entirely WP05's.
WP06's `paintedEvidence` addition merges clean; both `groupsPainted` and `pgroup-rows`
survive the merge.

---

## Rulings on the four self-reported limits

1. **snake_case over T035's camelCase — correct, and pinned.** The witness is what
   acceptance reads; T035's instruction was wrong. The pin discriminates (falsified
   above). Keep the decision; only extend the array per R2.

2. **`AUDIBLE_EDIT_DELTA_MARGIN = 1.0e-3` unmeasured — documenting is sufficient.**
   It is declared in code with its rationale, in ROADMAP, and in F-59, and it is
   labelled a declaration rather than a measurement in all three. No bound can be
   derived without a completed live run, and the run is environmentally blocked.
   Deriving a number from an unrun rig would be worse than declaring one. Carry it as
   a stated limit into acceptance; the first completed run confirms or moves it.
   **Note that R1 makes this margin load-bearing on a predicate that now asserts it
   directly** — which is an argument for the amendment, not against the margin.

3. **The negative's exit code proves nothing under F-40 — the headless tests do carry
   it, but only as far as they reach.** The falsification is real: the mutation above
   shows the keying rule is what makes the negative discriminate, and it is executed.
   What the headless tests do *not* cover is the plan→runner→resolve join, which is
   live-only. So the negative's falsifying power today is "the keying rule provably
   discriminates on a measurement of the shape the runner produces", not "the negative
   was observed to fail on reach". That is a narrower claim than a passing negative
   and should be recorded as such at accept. It is enough to approve the design; it is
   not enough to strike LIMIT-1, and WP06 did not strike it.

4. **`midi_input_rechannelled` overpromises — grade it for what it measures.** The
   implementation is `self.midi_input_values.len() > 1` — the projected MIDI-input row
   took more than one distinct value across the run. On a fixture packing 15 Patches
   onto channels 0–14 an adjacent step is a `DuplicateMidiChannel` refusal, so a
   completed edit is not measurable here. The field discriminates for what the scene
   claims (the row is Patch-local and follows the switch, so a defeated run projects
   one value) and FR-008 editability is proven in WP05's target. Grade it as
   *"the MIDI-input row is Patch-local and re-projects across a switch"* and record
   that the name is broader than the measurement. Do not rename the witness field
   mid-mission; record the gap in the acceptance note.

---

## Recommended, non-blocking

**The F-49 scene-name gate is unpinned.** Mutating

```rust
let effects_and_buses = (scene == crate::testing::EFFECTS_AND_BUSES_SCENE_NAME)
```

to a never-matching literal leaves **all 715 lib tests and the
topology/mixer/live-demo/shell integration suites green**, while silently dropping
the entire effects-and-buses evidence block from the emitted report and removing its
`complete` requirement. The gate is correct today, but nothing would catch a
regression in it. One test asserting the effects scene's report carries
`effects_and_buses == Some(..)` and a patch-editor-scene report carries `None` would
close it. Worth doing while R1 is open.
