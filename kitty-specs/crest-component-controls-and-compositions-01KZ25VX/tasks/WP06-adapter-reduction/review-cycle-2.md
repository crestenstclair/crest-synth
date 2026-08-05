---
affected_files: []
cycle_number: 2
mission_slug: crest-component-controls-and-compositions-01KZ25VX
reproduction_command:
reviewed_at: '2026-08-03T19:35:57Z'
reviewer_agent: unknown
verdict: approved
wp_id: WP06
review_status: "acknowledged"
---

> **This file is not a review. It is a duplicate, corrected before merge.**
>
> Spec Kitty numbers an acknowledgement artifact one ahead of the review cycle it
> acknowledges, and the copy inherits `verdict: rejected` from the file it copied.
> The body below is byte-identical to `review-cycle-1.md` — the *previous* cycle's rejection,
> which was acknowledged and fixed. **No review with this cycle number was ever
> issued.**
>
> The inherited `verdict: rejected` blocked six separate approvals in this mission
> (waiver W-01) and would have blocked the merge gate for the same wrong reason.
> The outer frontmatter is corrected to `WP06`'s real final outcome: **approved**.
> Nothing in the body is altered — the rejection it records genuinely happened, one
> cycle earlier, in `review-cycle-1.md`.


# WP06 review — cycle 1

**Verdict: changes requested.** Two issues, both small and localized. **The reduction itself is correct
and must not be redone.** Everything in "What is right" below was verified first-hand and should be
left exactly as it is.

---

## What is right (do not touch this on the next cycle)

Measured in `.worktrees/crest-component-controls-and-compositions-01KZ25VX-lane-f`, mission branch
already merged in (`6d33752`).

- **The move is real.** 1,282 → 698. `paint_context_line`, `paint_identity_header`,
  `paint_main_workspace`, `paint_patch_workspace`, `paint_mixer_workspace`, `paint_side_region`,
  `paint_surface_summary`, `paint_footer`, `paint_diagnostic`, `paint_semantic_control`,
  `control_state`, `semantic_value_label`, `chrome_text`, `padded_text`, `trailing_text`,
  `hairline_separator`, `margin`, `shell_frame`, `MIXER_TRACK_MIN_WIDTH_PX`,
  `WORKSPACE_TITLE_ROW_PX` — `grep -rn` over `src/` returns **zero** hits for every one. No
  forwarding shims. No `ScrollArea` anywhere in the adapter.
- **NFR-005 holds.** `git diff --name-only <mission>..HEAD` touches five source files and **no test
  file**. The adapter's own in-file `#[cfg(test)]` module is **byte-identical** to base (base
  1040–1282, now 456–698; `diff` reports no change).
- **Suite parity, independently measured.** Lane: **862 passed / 0 failed / 1 ignored**, exit 0.
  Baseline measured myself in a throwaway worktree at the mission branch: **862 / 0 / 1**, exit 0.
  Identical set. `cargo clippy --all-targets -- -D warnings` exit 0; `cargo fmt --check` exit 0.
- **`ShellFrameObservation` survived intact (R-03).** `observed_bands` iterates
  `ShellRegionId::ALL` = `[ContextLine, IdentityHeader, MainWorkspace, PersistentSideRegion,
  Footer]`, the base's literal order exactly; `observed_label` maps to the same five projection
  accessors the base used; rectangles still come from each panel's `.response.rect`; the emit is
  still after painting. `every_band_reports_the_label_the_live_demo_runner_expects` passes, and
  `tests/live_demo_scene.rs` (both tests) passes — the `live_demo_runner.rs:2258-2273` consumer
  expecting bare `"PATCH"` from `context_label()` is green.
- **All four out-of-`owned_files` changes are genuinely forced.** Each mutation-verified:

  | Change | Mutation | Named failure |
  |---|---|---|
  | `MixerStripBank` wiring (`main_workspace_composition`) | route MIXER → `Section` | `production_update_renders_both_contexts_at_both_reference_viewports`, `graphical_application_shell.rs:326` |
  | `SELECTED <track>` caption | remove caption | same test, `:341`, `assert!(output_contains_text(&output, "SELECTED T00"))` |
  | `section.rs` helper widening | revert to base predicate | `utility_inspector_panel::tests::the_panel_paints_no_text_that_did_not_arrive_in_the_projection` |
  | `toggle.rs` hairline mark | revert to `rect_stroke` | `no_interactive_target_is_below_the_authored_minimum` + `component_vocabulary_acceptance`: *"a framed target is 12 px tall, below the authored 48 px minimum"* |

  I also confirmed the base adapter painted `SELECTED {focused_track}` (`:696`),
  `PATCH {:02} · {}` (`:712`) and `METER {:.3}` (`:566`) itself, so those three spellings really were
  shipped adapter behaviour that no composition carried. The toggle reasoning is exactly right and the
  fix is the right one.
- **No dead code.** `frame_plan_for`, `main_workspace_composition`, `install_authored_chrome`,
  `SELECTED_LABEL`, `PATCH_LABEL` all have production callers.

---

## NFR-003 is mis-specified — this is a finding, not a failure, and no line-count work is asked for

Your arithmetic is correct. I reproduced all of it:

- total 698; `#[cfg(test)]` at 456, so the test module is **243 lines**;
- production span 1–455 = 455 lines, of which **73 comment + 30 blank**, leaving **352**;
- floor = 352 + 243 = **595**, which is **83 over 512** at zero comments and zero blank lines.

The test module is byte-identical to base and NFR-005 forbids touching it, so **NFR-003 and NFR-005
are in direct arithmetic conflict at the stated numbers**. NFR-003 was set as 40% of a whole-file
1,282 that silently included a 243-line in-file test module. Nobody accounted for it. The requirement
as written is unreachable by any correct implementation. **Do not compress to chase it.**

Recorded for mission review: the defensible measure is production-only, 1,039 → **455**, a 56%
reduction. Note for issue 2 below: 40% of the 1,039 production baseline is 415, and the meter residue
is ~53 lines (46 for the function, 3 for the call site, ~4 imports used nowhere else). Removing it
lands production at ~402 — **under** that threshold. The residue is the only thing between this file
and a defensible reading of its own requirement.

---

## Issue 1 — the `section.rs` widening is necessary but not correctly bounded, and a tighter form passes

`section.rs` is WP05's approved file and F-14 explicitly reserves this helper. You were right to check
rather than assume, and right that it had to change. The problem is the *shape* of the change.

**What I measured.** `section.rs:1029` now admits a run of the form
`<declared label><space><atom>`, where the remainder goes through `atom` — which includes the
`text.contains(piece)` arm. F-14 names that arm as the reason a fabricated `"0"` passed 10/10 inside
`"T00 Level"`. Your comment says the new path "inherits the containment weakness". It does, and it
carries it into a position that was **closed** before this WP: mutation 2 above proves the base
predicate rejected *every* `label value` run, including the shipped one.

Three runs, each on the whole `cargo test --lib` module:

| Painted into the Inspector | Shipped helper | Result |
|---|---|---|
| `format!("{CURSOR_LABEL} 0")` — fabricated numeral, substring of `"T00 Level"` | admits | **781 passed, 0 failed** — the fabrication ships undetected |
| `format!("{CURSOR_LABEL} 999")` — non-substring | rejects | `the_panel_paints_no_text_that_did_not_arrive_in_the_projection` FAILED |

So the bound has *some* force, but it is exactly F-14's hole re-opened in a new position.

**A strictly tighter bound exists and I ran it green.** Require the remainder to match by *equality*
rather than containment:

```rust
|| piece.split_once(' ').is_some_and(|(label, rest)| {
    let rest = rest.trim();
    labels.contains(&label)
        && !rest.is_empty()
        && (labels.contains(&rest)
            || declared.iter().any(|word| word == rest)
            || projected.iter().any(|text| text == rest))
})
```

Measured with this substituted in and nothing else changed:

- `cargo test --lib` → **781 passed, 0 failed, 1 ignored**. Everything you need still passes.
- with `format!("{CURSOR_LABEL} 0")` also applied → `the_panel_paints_no_text_that_did_not_arrive_in_the_projection`
  **FAILED**. The hole is closed.

It works because `"SELECTED T00"`'s remainder is `"T00"` and **you already pushed
`focused_track.to_string()` into `projected_text` verbatim** — so equality is satisfied without
containment. `"PATCH 01"` never reaches this path at all: it passes through `atom` directly, since
your (correct) addition of `identity_header().primary_label()` to the collector puts the literal
`"PATCH 01 · Graphical Shell"` in the projected set.

**Asked for:** the equality-bounded form above, in place of `atom(rest.trim())`. Keep everything else
in the helper, including the `projected_text` additions — those are correct and are what makes the
tighter bound viable. Leave the `atom` arm itself alone; the standalone-`"0"` hole F-14 records stays
mission review's, not yours. Then re-run `cargo test --lib` and the full suite.

## Issue 2 — the meter residue consumes the composition's band, which is the layout decision FR-006 forbids

**Your resolution of the three-way conflict is right, and I am not asking you to reverse it.** I
confirmed both protected assertions pin a live reading — deleting the call fails
`production_update_renders_both_contexts_at_both_reference_viewports` at `:340` on `"METER 0.000"`
*and* `mixer_frame_reads_one_compatible_immutable_audio_observation` at `:641`. I also confirmed the
second test demands exactly **one** reading for track 0, so one reading satisfies both. Restating
`MixerStripBank`'s inset/pitch/origin to align sixteen would indeed be forbidden layout plus a second
copy that drifts. Keeping one reading in the adapter is the correct call, the fifteen lost readings are
properly recorded, and the compatibility check is preserved unrelaxed.

**What is wrong is where it paints.** `paint_focused_track_meter` runs on `band_ui` *before*
`arrange_band`, and `text::paint_text` is `ui.label(...)`, which advances the layout cursor.
`MixerStripBank::render` → `render_bank` → `section::inset_scope`, which anchors on
`ui.available_rect_before_wrap()` (`section.rs:153`). So on MIXER — always, because focus there is
always a mixer track — **the adapter hands the bank a MainWorkspace band shorter than the policy
declared, by one line height.** The sixteen columns are allocated into a rect the adapter shrank.
WP09's geometry proofs call the composition directly with a full band and cannot see this.

The live run confirms it by eye: `METER 0.000` renders as a bare unstyled line at the top-left of the
workspace, above and partly overlapping `MIXER WORKSPACE`, outside any composition's structure.

That is not "one reading kept in the adapter" — it is a reading kept in the adapter **and given a
position, and taking extent from the composition it was just wired to**. Placement and extent are
precisely what the `AppWindow` port invariant (`shell.yaml:377`) and FR-006 deny the window. The
disclosure covered the first half of that; it did not cover the second, and the second is the part
that changes what the composition paints.

**Asked for:** stop the meter perturbing the band. Paint it into a detached child whose rect is
derived from the panel rectangle rather than from the layout cursor — the same
`ui.new_child(UiBuilder::new().max_rect(...))` device `paint_shell` already uses for `band_ui`, so the
bank's `available_rect_before_wrap()` sees the full band — and place it after `arrange_band` so it
cannot displace anything. Somewhere it does not collide with the legend. Then confirm both meter
assertions still pass and `cargo test` is still 862/0.

Two smaller notes on the same residue, neither of them separately blocking:

- `TypeStyle::CodeValue`, `SemanticColor::TextSecondary`, `ComponentState::Resting` and the
  `"METER {:.3}"` spelling are all chosen in the adapter. They are authored *role names*, not
  literals, so NFR-004 is not violated — but they are the last visual choices in the file. If they can
  ride along behind a named helper in the visual module while the reading stays adapter-side, take it.
  If not, leave them and say so; I will not hold the WP for it.
- Removing/relocating this function is also what puts production-only at ~402 (see NFR-003 above).

---

## Not yours — recorded so it reaches the right owner

- **Inspector/PATCH text clips with no ellipsis** (`PATCH 01 · Xylophone 1 (bank 0, program 13, perc`;
  `Locke` for `Locked`), seen live at desktop. The mechanism is `section::caption`
  (`section.rs:321`), which allocates a single line of `InstructionHint.line_height_px` and paints
  through `painter_at(response.rect)` — single-line, hard-clipped. The base adapter's `chrome_text`
  was `ui.label`, which **wraps**. So this is a real change from base, but it is WP05's approved
  composition, not your code; you were told to route the side region through it (T030 step 1). Your
  `PATCH {:02} ` prefix lengthens the run by six characters, and it is required by the protected
  assertion, so it is not removable either. **WP08's T044 asserts against exactly this**, and neither
  WP08 nor WP06 owns `section.rs`. This belongs in `cross-wp-findings.md` for mission review. Do not
  fix it here.
- **Every mixer cell repeats the track id** (`T00` header, then `T00 Level` / `T00 Pan` / `T00 Mute` /
  `T00 Solo`). F-06/F-10 made visible; built in `semantic_graphical_view_model.rs`, unowned.
- **`tests/component_composition.rs` does not exist**, but
  `.kittify/crest-spec/proof/validations.yaml:44` declares `cargo test --test component_composition`
  as a project validation. `spec-kitty accept` will fail on it. WP08's asset, flagged here only
  because I hit it.
- **Confirmed closed:** F-C. After the mission merge, `spec-kitty crest-spec doctor` reports
  *Crest-spec OK — 130 resources, 102 requirements, 31/31 completion checks*.
- **Credit where due:** C-003's mark-unavailable rule is visibly working in the PATCH Utility panel —
  `MASTER VOLUME`, `MIDI INPUT`, `VOICE LIMIT` each render the declared `--` rather than a fabricated
  value. And all sixteen mixer columns seat at 1820 px with no scrollbar and no clipped sixteenth
  column: the `ScrollArea` removal and `MixerStripBank` wiring are correct in the shipped product.

## Scope of the next cycle

Two edits: the equality bound in `section.rs`, and the meter's placement in the adapter. Nothing else.
Re-run the full suite (expect 862/0/1), `cargo clippy --all-targets -- -D warnings`, and `cargo fmt
--check`. Do not chase the 512-line number and do not restructure anything listed under "What is
right".
