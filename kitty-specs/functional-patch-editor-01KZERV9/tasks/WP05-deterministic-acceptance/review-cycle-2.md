---
affected_files: []
cycle_number: 2
mission_slug: functional-patch-editor-01KZERV9
reproduction_command:
reviewed_at: '2026-08-09T12:44:37Z'
reviewer_agent: unknown
verdict: rejected
wp_id: WP05
---

# WP05 review — cycle 1

**Verdict: reject.** One defect, mechanical, entirely inside
`tests/functional_patch_editor.rs`. Everything else this package claims was
reproduced and holds.

## The defect: the transcription's central rule is not pinned

F-52 let `page_strip_groups` stay on one condition, stated in the finding itself:

> the transcription is acceptable **only while every rule it copies is pinned to
> the committed source, and every pin is falsified by mutating that source.**

It is not. I mutated `webview-page/page.js` six times, each time breaking a rule
the transcription copies, and ran `cargo test --test functional_patch_editor`.
All six passed green:

| # | mutation to `webview-page/page.js` | result |
|---|---|---|
| 1 | `stripGroupKey`: `"patch.envelope."` → `"patch.envelopeXX."` | **MISSED** |
| 2 | `stripGroupKey`: `patch.effect.` arm returns `null` instead of `openSlot` | **MISSED** |
| 3 | `stripGroups`: `if (!control.visible) continue` neutralized | **MISSED** |
| 4 | `groupHeadControlId`: `slot.N` → `patch.engine` | **MISSED** |
| 5 | `stripGroups`: `openSlot = stripGroupKey(id, null)` → `openSlot = null` | **MISSED** |
| 6 | `stripGroups`: unknown identities dropped instead of joining `?group` | **MISSED** |

`page_strip_group_key` (`tests/functional_patch_editor.rs:529`) is a line-for-line
copy of `stripGroupKey` (`webview-page/page.js:989`), and **no entry in
`check_the_transcribed_page_rules_match_the_committed_script` mentions any of its
five prefix rules.** The pinned `DESIGNED_STRIP_GROUPS` table pins the group
*names*; the mapping from control identity to group — the rule T030's entire claim
is about — is unpinned. So is the `visible` skip, the `openSlot` carry that makes
occupant rows nest under their own slot, the `?group` fallback the flat-run
negative depends on, and `groupHeadControlId`, which `projected_screen_strings`
uses to build the group titles it walks.

This matters *now*, not hypothetically: WP06 is editing `page.js` in lane-f today,
and F-44's live `stripGroupsPainted` producer has not landed. Until it does, this
file is the only executed proof of grouping in the mission, and its central rule
is a copy tied to nothing.

Two further copied rules are also unpinned:

| # | mutation | result |
|---|---|---|
| 7 | `rangeEndpointText`: `toFixed(3)` → `toFixed(1)` | **MISSED** |
| 8 | `controlValueText`: toggle `"ON"/"OFF"` → `"TRUE"/"FALSE"` | **MISSED** |

## Sibling blind pin (F-42 / F-53's shape, one entry over from the one you found)

`("the painted range span", "data-role=\"row-range\"")` occurs **twice** in
`page.js`: at `:798` inside `rangeHtml`, the painting site, and at `:1778` inside
`renderObservation`, which *reads* it as a CSS selector. Renaming only the painting
site leaves the pin satisfied:

| mutation | result |
|---|---|
| `page.js:798` `data-role="row-range"` → `"row-rangeZZ"` (endpoints untouched) | **MISSED** |

The set is not blind — the two `rangeEndpointText` pins independently catch a fully
emptied `rangeHtml` (verified, CAUGHT) — but this entry individually claims to guard
the painted span and cannot. It is the same anchor-appears-elsewhere shape as the
`control && control.numericRange` pin you found and fixed.

`("the authored option label read (F-33)", "control.selectedLabel")` also matches
three times, but all three are inside the one `controlValueText` choice arm, so it
is single-site in effect. Reverting that arm was CAUGHT. No change needed.

## What to do

All of this is in `tests/functional_patch_editor.rs`, which you own.

1. Pin every rule `page_strip_group_key`, `page_strip_groups` and
   `page_group_head_control_id` copy — the five prefix arms, the `visible` skip,
   the `openSlot` assignment, the `?group` fallback, and the two
   `groupHeadControlId` arms — to the statements in `page.js` that implement them.
2. Pin `rangeEndpointText`'s `toFixed(3)` and `controlValueText`'s toggle wording,
   or drop those formatting rules from the transcription.
3. Re-anchor `"the painted range span"` to a fragment that occurs only at the
   painting site (the full `'<span class="prow-range type-hint muted"
   data-role="row-range">' +` line is unique).
4. Re-run mutations 1–8 above and record CAUGHT for each. Add a note to
   `check_the_transcribed_page_rules_match_the_committed_script` asserting each new
   anchor is unique in the source, so the next entry cannot be added blind.

Nothing else. Do not touch `page.js` — lane-f is in it.

## What I verified and found sound (do not redo)

- **F-28 — closed.** All seven previously-MISSED sites flipped to CAUGHT under my
  own sweep (`patch_page_projection.rs` :857, :1192, :1336, :1349, :1386, :1395,
  :1451, each reverted `label()` → `id()`), plus three previously-CAUGHT controls
  re-run and still CAUGHT.
- **F-33 — closed, both halves, plus both extra catches.** `selected_label`
  defeated → CAUGHT on the active value; `requested_label` defeated → CAUGHT on the
  in-flight lifecycle band via the `preset swap in flight` fixture; engine-row
  Identity → capability id → CAUGHT; slot-occupancy Identity → effect id → CAUGHT.
  Your report that the finding under-counted by one field is correct.
- **Non-vacuity.** Fixture is 4 Patches / 2 engines with asserted schema divergence;
  generation delta asserted as exactly 1 (mutating `checked_add(1)` → `(2)` →
  CAUGHT); end-of-order refusal by whole-state and whole-document comparison
  (`select_patch` made to wrap → CAUGHT); master-gain single owner asserted by
  naming the exact two serialized leaves (a third leaf added → CAUGHT); SC-003's
  exception named as a zero rather than absorbed. Voice-limit falsification
  reproduced (refusal branch defeated → CAUGHT, "5 voices latched under a limit
  of 3"); engine seeding collapsed → CAUGHT; requested value leaked onto settled
  rows → CAUGHT (19 rows named).
- The `if switched.is_ok()` branch in
  `check_an_in_flight_edit_stays_correlated_and_a_subordinate_surface_is_left`
  takes the `ok` path in practice — verified, its assertion fires under mutation.
  Not a skipped claim.
- Marker emitted from exactly one place, strictly last.
- Full suite green: 701 lib + every integration target. `input_capture_witness` is
  F-12, `webview_projection_shell` live section is F-40. Clippy and fmt clean,
  crest-spec doctor OK.
- **Geometry claim confirmed independently.** With the display woken, T024 PASS and
  T011 PASS, numbers identical to F-41: navigate 12 rows/576px, 753 in 520
  (scroll 233); adjust 12/610, 787 in 520 (scroll 267); braids 11/528, 705 in 520
  (scroll 185); desktop 753 in 770, scroll 0. Live ack section fails on F-40 only.
- **Per-row geometry (`viewport`, `workspaceBody`, `heightPx`/`railPx`/…) is
  genuinely not assertable here.** `renderObservation` is exposed only as
  `window.crest.renderObservation` (`page.js:2120`) and needs a real window;
  `paintedEvidence` carries shell-region rects, not per-row geometry. Your reading
  is right and is not held against the package.
- **No collision with WP06.** lane-f's uncommitted `page.js` work is 16 additive
  lines in `paintedEvidence` and touches none of your hunks; twelve of your
  fourteen pins already hold against it (the two F-33 pins fail only because
  lane-f predates your change, which the merge resolves). Your
  `tests/webview_projection_shell.rs` reflow is byte-identical to lane-f's own
  uncommitted reflow of the same two hunks, so it merges clean.
