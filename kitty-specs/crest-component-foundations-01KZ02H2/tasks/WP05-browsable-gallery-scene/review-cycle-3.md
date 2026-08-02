---
affected_files: []
cycle_number: 3
mission_slug: crest-component-foundations-01KZ02H2
reproduction_command: make demo-live-component-library
reviewed_at: '2026-08-02T17:55:00Z'
reviewer_agent: claude
verdict: approved
wp_id: WP05
---

# WP05 – Browsable gallery scene — review cycle 2

Reviewer: claude · lane-e · `kitty/mission-crest-component-foundations-01KZ02H2-lane-e` @ `9438e12`
(WP05's own work is `a3f3c81` and the fix `9d00b57`; everything else in the lane is merged dependency work.)

**Verdict: approved.** All three review-cycle-1 issues are fixed, and each fix is measured rather than
asserted. I verified by running the window, not by reading the diff.

## Issue 1 — window off the screen edge · fixed

The minimum is now the authored compact width by a measured `856` px height
(`component_gallery_scene.rs:2241`, `:2266-2273`), bounded at both ends:

- **Upper bound**, the bug that was reported: a `const` assertion at `:2247-2251` holds
  `MINIMUM_GALLERY_HEIGHT_PX + GALLERY_WINDOW_CHROME_PX <= Desktop.authored_viewport().height_px`, with
  the chrome allowance declared at 96 px against the 62 px measured on macOS.
  `the_gallery_window_fits_on_the_authored_desktop_display_with_its_chrome` holds the same bound in a
  test. **I falsified the mechanism**: raising `MINIMUM_GALLERY_HEIGHT_PX` to `1000.0` fails compilation
  with `error[E0080]: evaluation panicked: the gallery window is taller than the authored desktop display
  it is reviewed on`. Restored afterwards; the tree is clean.
- **Lower bound**: `the_declared_minimum_window_is_the_size_the_gallery_composes_at` paints all eight
  pages at 856 (no defects) and at 848 (defects reported), so the constant cannot drift above what the
  layout needs.

Measured live on the same 1920 × 1080 logical display where the original defect was found:

```
window pos (320, 71) · size 1280 × 888 · bottom edge 959 · right edge 1600
```

Wholly on screen. The footer band — `1-8 PAGE · 9 UNBOUND DIGIT · CLOSE WINDOW FINISH` — is visible in
every screenshot I took, which is the affordance that was cut before.

## Issue 2 — NFR-005's "at both authored sizes" · fixed

`states_painted` is now `[[Option<PaintedState>; COMPONENT_STATE_COUNT]; policies]` (`:360`), written at
`:1093` from `specimen.policy`, which travels on the `Specimen` the composition constructs. A state
counts only when *every* declared policy painted it (`painted_state_count`, `:457-466`), colorless
distinctness is judged within each composition rather than across them (`:475-488`), and
`states_rendered` names the compositions each state reached (`:528-554`).

The count is capable of reporting a shortfall — the property that was missing:
`a_state_painted_in_only_one_composition_is_not_counted_as_covered` removes one composition's record and
the observation drops to 8. My live observation reported all nine states with
`"viewports":["Desktop","SteamDeck"]`.

## Issue 3 — page 6 specimens unlabelled · fixed

`paint_values_and_status_page` (`:1927-1945`) leads each row with the state name and keeps the realistic
content: `Loading · ENGINE`, `Error · ENGINE`, `Muted · TRACK 03`. Confirmed on screen, both columns.
`every_painted_state_is_distinguishable_without_color_in_both_compositions` now also asserts that the
label a reader sees contains the state's canonical name, so a future row cannot quietly drop it.

## What I ran

```
window walk           digits 1-8, then the unbound 9, then the window's own close button
                      → exit 0, observation emitted after painting
```

All fifteen `witness.component_gallery` predicates in `.kittify/crest-spec/proof/witnesses.yaml`
satisfied:

```
pages_declared 8 · pages_painted 8 · pages_reachable_by_digit 8 · unbound_digit_retained_page true
states_declared 9 · states_painted 9 · states_distinguishable_without_color true
desktop_viewport_painted true · steam_deck_viewport_painted true · bands_retained_both_viewports true
clipped_or_overlapping_text 0 · token_source_exact true · typeface_resolved true
app_state_generation_delta 0 · window_closed true · text_runs_painted 2615
```

Retention is measured rather than taken on trust: the screenshot taken after pressing `9` is
byte-identical to the one taken on page 8 (`md5 567da1b1…`), while page 7's differs. `make test` exit 0
(587 lib tests plus every integration target, 0 failures — no regression against the recorded baseline),
24 gallery unit tests, `cargo clippy --all-targets --all-features` 0 warnings, `cargo fmt --check` clean.

`make demo-live` still runs the autonomous effects-and-buses witness: exit 0, 144 checkpoints, 105/105
editable parameters, 3/3 engine transitions, 0 dropped records, `cleanup=true`, `callbackAllocations=0`.
C-005 holds in the direction that matters — the gallery took nothing from the witness contract and gave
nothing to it.

## Notes, not blocking

- The doc comment on `paint_composition` (`:1462-1463`) says the page is recorded painted "only after
  both compositions did", but `record_painted_page` fires inside each composition (`:1513`), so the first
  one to emit runs marks it. The counter is honest; the comment overstates it. Worth a one-line fix
  whenever this file is next open.
- `a_complete_session_satisfies_the_declared_witness_predicates` still injects `record_digit_request` /
  `record_painted_page` directly, so its `pages_reachable_by_digit == 8` half remains a fixture. Raised
  as non-blocking last cycle and it stays that way: the binding tests and my live run both cover it.

## Coordination

WP06 depends on WP05 and consumes `ALL_GALLERY_PAGES` / `GALLERY_PAGE_COUNT`. Two things changed this
cycle that WP06 must rebase onto: `minimum_gallery_viewport()` is no longer the desktop authored viewport
(it is 1280 × 856), and `states_painted` now means "painted at every declared policy" (the value is still
9). No other WP declares any of WP05's four `owned_files`.

## Anti-pattern checklist

| # | Item | Result |
|---|---|---|
| 1 | Dead code | PASS — every new item has an in-module production caller; the observation's `pub const fn` accessors match `ShellFrameObservation`'s existing style, verified in `src/shell/shell_frame_observation.rs` |
| 2 | Synthetic-fixture test | PASS — the coverage tests run the real `paint_gallery` through a real `egui::Context` and tessellate the output; deleting the paint path fails them. One partial fixture noted above |
| 3 | Silent empty return | PASS — every early return is documented (`record_painted_page` at `:415`, `states_rendered` at `:539`, `emit_text` at `:869`, `hint_line_at` at `:940`) |
| 4 | FR coverage | PASS — FR-007 and FR-008 verified live; NFR-005's "at both authored sizes" clause is now measured per policy, which is what failed last cycle |
| 5 | Frozen surface | PASS — the WP's own commits touch only the four declared `owned_files`; `DESIGN.md:634-644` untouched |
| 6 | Locked decision | PASS — C-004 holds (generation delta 0 measured across a live page walk, translator untouched); C-005 holds (`make demo-live` still the autonomous witness) |
| 7 | Shared-file ownership | PASS — no other WP declares any of these four files |
| 8 | Production fragility | PASS — typed errors throughout, no new panic path in production code; the one `unwrap_or` at `:2340` saturates rather than panics |
