---
affected_files: []
cycle_number: 2
mission_slug: crest-component-foundations-01KZ02H2
reproduction_command:
reviewed_at: '2026-08-02T16:43:04Z'
reviewer_agent: unknown
verdict: rejected
wp_id: WP05
---

# WP05 – Browsable gallery scene — review cycle 1

Reviewer: claude · lane-e · `kitty/mission-crest-component-foundations-01KZ02H2-lane-e` @ `a3f3c81`

**Verdict: changes requested.** The scene is well built and I verified it by running it, not
by reading it. Three issues below; issue 1 is the one that matters for a scene whose only job
is human review.

## What I verified myself (not taken from the activity log)

I built the release binary, opened the window, drove digits `1`–`8`, pressed the unbound `9`,
paged back, and closed the window. Exit 0. The emitted observation:

```
pages_declared 8 · pages_painted 8 · pages_reachable_by_digit 8 · unbound_digit_retained_page true
states_declared 9 · states_painted 9 · states_distinguishable_without_color true
desktop_viewport_painted true · steam_deck_viewport_painted true · bands_retained_both_viewports true
clipped_or_overlapping_text 0 · token_source_exact true · typeface_resolved true
app_state_generation_delta 0 · window_closed true · text_runs_painted 3561
```

All fifteen `witness.component_gallery` predicates satisfied. `make test` exit 0,
`cargo clippy --all-targets --all-features` zero warnings, `cargo fmt --check` clean,
21 gallery unit tests pass. Ownership is clean: the commit touches exactly the four declared
`owned_files` and no other WP owns any of them. `--demo-live` still resolves to the autonomous
effects-and-buses witness and the gallery sets no `demo_live` flag. `DESIGN.md:634-644` is
untouched, so C-005 holds in the direction that matters.

## Issue 1 — The footer band is off-screen at the gallery's own minimum size, and the operator cannot resize it into view

`run()` sets both `with_inner_size` and `with_min_inner_size` to
`ViewportDensityPolicy::Desktop.authored_viewport()` — 1920 × 1080
(`src/testing/component_gallery_scene.rs:2199-2205`, `:2154-2156`).

Measured on a 1920 × 1080 logical display during my run:

```
window position (0, 30) · window size 1920 × 1112   →   bottom edge at 1142, screen height 1080
```

62 px of the window sits below the screen. That is exactly the desktop footer band (64 px),
so `paint_footer_band` — `1-8 PAGE · 9 UNBOUND DIGIT · CLOSE WINDOW FINISH` plus
`SCENE-LOCAL PAGING · NO APPLICATION STATE CHANGES` — is never visible. Because
`min_inner_size` equals the full authored viewport, the operator cannot shrink the window to
reach it. The paint pass reports `clipped_or_overlapping_text: 0` throughout, correctly: it
measures text against the egui surface, and the surface is the full 1080 px. Nothing in the
observation can see that the display is smaller than the window.

This is the display class the desktop policy is authored for, and it is the only scene in the
mission built for a human to look at. The on-screen affordance that explains how to browse is
the part that gets cut.

The side-by-side composition is the right call and I am not asking you to undo it. Two ways
out that keep it:

- Derive the minimum from what the window can actually occupy — subtract the window chrome
  and clamp to the available work area — so the composed frame always fits on screen; or
- keep 1920 × 1080 as the *initial* size, drop the hard `min_inner_size` floor, and let the
  stage reflow or scale so the footer stays reachable when the operator shrinks the window.
  The paint pass already reports clipping honestly, so a too-small window fails loudly instead
  of silently hiding a band.

Whichever you choose, please add a measurement that would have caught this: the current tests
paint into a synthetic `screen_rect` of exactly the declared minimum, which can never disagree
with the declared minimum.

## Issue 2 — NFR-005's "at both authored sizes" is not measured for states

`GalleryPaintLedger::bands_painted` is `[[bool; regions]; policies]` — per policy, which is
right. `states_painted` right beside it is a flat `[Option<PaintedState>; COMPONENT_STATE_COUNT]`
(`:341-343`). A state recorded by the Desktop column alone still counts, so
`states_painted == 9` does not evidence the clause NFR-005 actually names:

> Every state in the closed state vocabulary has at least one gallery specimen **at both
> authored sizes**

Same for the WP's own success criterion ("Every declared `ComponentState` appears with
representative content, at both authored viewport sizes"). `desktop_viewport_painted` and
`steam_deck_viewport_painted` only prove each column emitted at least one text run, not that
each column emitted all nine specimens. `every_page_paints_both_authored_viewport_compositions`
asserts the same weak thing.

Behaviourally you are fine — I confirmed on page 5 that both columns paint all nine labelled
states — but this WP's whole thesis is that coverage is measured rather than asserted, and this
one is asserted. `bands_painted` two lines above shows the shape the fix takes: carry the active
policy on `SpecimenPainter` (set it in `paint_composition`, which already knows it) and index
`states_painted` by policy, then let `states_painted` in the observation mean "painted at every
declared policy". It should stay 9.

## Issue 3 — Page 6 specimens are not labelled with their state name

T029 step 5: "Every specimen must be **labeled with its state name**. A wall of unlabeled
variants is not judgable."

Page 5 does this correctly — every row reads `Resting`, `Focused`, `Adjusting`, `Disabled`,
`Loading`, `Error`, `Muted`, `Soloed`, `Selected`. Page 6 does not: `status_specimens()`
(`:1872-1940`) labels the rows `MASTER GAIN`, `CUTOFF`, `RESONANCE`, `ASSET SLOT`, `ENGINE`,
`TRACK 03`… The state has to be inferred from the keyline weight or the mark.

Realistic content on this page is the right instinct — don't replace it with a second copy of
page 5. Add the state name alongside it (a caption row, or a left-hand state column beside the
label) so the row says both what it is and which state it is in.

## Notes, not blocking

- `PaintedState` is overwritten each time a state repaints, so the recorded evidence is
  whichever composition painted last (the Steam Deck column). That is fine today; it becomes
  meaningful if issue 2 is fixed by indexing per policy.
- `a_complete_session_satisfies_the_declared_witness_predicates` (`:2710`) takes real painted
  coverage but injects `record_digit_request` / `record_painted_page` / `record_unbound_key`
  directly, so the `pages_reachable_by_digit == 8` half of that test is a fixture. The binding
  tests and my live run both cover it, so I am not asking for a change — but if
  `ComponentGalleryApplication::handle_input` were driven from a headless `RawInput` event
  stream, that last fixture would go away too.
- The observation's `pub const fn` accessors have no non-test caller. That matches
  `ShellFrameObservation`'s existing house style, so I am treating it as convention rather than
  dead code.
- `make demo-live` correctness was confirmed from the parse table and
  `the_component_gallery_is_its_own_option_and_never_a_live_demo_alias`, not from a physical
  audio run.

## Coordination

WP06 depends on WP05 and consumes `ALL_GALLERY_PAGES` / `GALLERY_PAGE_COUNT`. Issue 2's fix
changes what `states_painted` means (it tightens, and the value stays 9); issue 1's fix changes
`minimum_gallery_viewport()`. WP06 should rebase after this cycle lands.

## Anti-pattern checklist

| # | Item | Result |
|---|---|---|
| 1 | Dead code | PASS — every new item has an in-module production caller; accessor style matches `ShellFrameObservation` |
| 2 | Synthetic-fixture test | PASS — deleting the paint path fails the coverage tests; one partial fixture noted above |
| 3 | Silent empty return | PASS — each early return is documented (`record_painted_page`, `emit_text`, `hint_line_at`) |
| 4 | FR coverage | **FAIL** — NFR-005's "at both authored sizes" clause is not asserted for states (issue 2) |
| 5 | Frozen surface | PASS — only the four declared `owned_files` are touched |
| 6 | Locked decision | PASS — C-004 and C-005 hold; generation delta measured at 0 across a live page walk; translator untouched |
| 7 | Shared-file ownership | PASS — no other WP declares any of these four files |
| 8 | Production fragility | PASS — typed errors throughout, no new panic path, no re-entrant `RefCell` borrow |
