---
affected_files: []
cycle_number: 1
mission_slug: crest-component-foundations-01KZ02H2
reproduction_command: cargo test --test component_vocabulary -- --nocapture
reviewed_at: '2026-08-02T18:55:00Z'
reviewer_agent: claude
verdict: approved
wp_id: WP06
---

# WP06 – Measured proof — review cycle 1

Reviewer: claude · lane-f · `kitty/mission-crest-component-foundations-01KZ02H2-lane-f` @ `59fe6c1`
(WP06's own work is `30d4ad1`; everything else in the lane is merged dependency work.)

**Verdict: approved.** The one question that decides whether this WP is worth anything — *are the
expected values written independently, or derived from the thing under test?* — I answered by
checking the transcription against `DESIGN.md` line by line and then by breaking the code six
different ways and watching the right test go red each time. Every claim in the Activity Log
reproduced.

## Declared validation

```
cargo test --test component_vocabulary -- --nocapture > /tmp/wp06-review.log 2>&1   → exit=0
CREST_COMPONENT_VOCABULARY_OBSERVATION colors=17 type_styles=8 spacing_steps=6 radii=3 states=9
  pages=8 density_policies=2 frames=4 glyph_runs=334 runs_scrolled_out_of_view=14
  sources_scanned=59 lines_scanned=40990
CREST_ACCEPTANCE component_vocabulary passed
11 tests, 11 passed.

make test       → exit=0 (27/27 targets ok)
make lint       → exit=0
make fmt-check  → exit=0
```

Recorded by redirect, never through a pipe.

## The independence claim is true, not asserted

`AUTHORED_COLORS`, `AUTHORED_TYPE_STYLES`, `AUTHORED_SPACING`, and `AUTHORED_RADII` claim to be
transcribed from `DESIGN.md` rather than read back from `token.rs`. I checked every row:

- All 17 colors match `DESIGN.md:536-552` exactly, including the two the mission changed
  (`focus #65e5ff`, `canvas #0c1015`).
- All 8 type styles match `DESIGN.md:559-566` — size, line, weight, tracking.
- Spacing / radius / min target / keylines / halo match `DESIGN.md:569-574`.

The colors are written as `#rrggbb` strings and parsed by `authored_rgb`, which shares no derivation
with the vocabulary's `Color32::from_rgb(0x.., 0x.., 0x..)`. Counts are asserted in both directions,
so a dropped token fails rather than passing by absence.

## Six falsifications — every guard is capable of failing

Each mutation applied to the lane worktree, run, then restored; `git status` clean after each.

| # | Mutation | Result |
|---|---|---|
| A | `Color32::from_rgb(0x65, 0xe5, 0xff)` planted in `src/adapter/production_effects.rs` | `the_literal_guard_reads_the_delivered_tree` FAILED, reporting `production_effects.rs:889: color literal outside the vocabulary` **and** `…:889: palette literal … color/accent/focus (#65e5ff) is spelled here` |
| B | `Self::AccentFocus` drifted to the retired green in `token.rs` | `component_vocabulary_acceptance` FAILED at `component_vocabulary.rs:1170`, `left: [110, 205, 174] right: [101, 229, 255]` |
| C | Adapter reverted to its pre-WP06 state | `no_interactive_target_is_below_the_authored_minimum` FAILED: *a framed target is 18 px tall, below the authored 48 px minimum* |
| D | same | `both_authored_viewports_render_intact` FAILED with the 4 mixer collisions at 1280×800 — `"T00 Level"` over `"0.000"`, and the same for Pan / Mute / Solo |
| E | same | `the_production_render_path_paints_only_authored_values` FAILED naming the three unauthored grays `#0A0A0A`, `#3C3C3C`, `#B4B4B4` |
| F | `MIN_INTERACTIVE_TARGET_PX` restored, planted literal at depth in `eframe_graphical_window.rs`, `cargo fmt` applied | guard FAILED at the rustfmt-normalized line 851 |

D and E are the important pair: they are the exact defects the Activity Log claims measurement found,
and they reappear the moment the fix is removed. This is a live guard, not a decoration.

## The marker, the negative typeface path, the render path

- **Marker discipline (T033).** `ACCEPTANCE_MARKER` is printed once, at
  `component_vocabulary.rs:2304`, as the last statement of `component_vocabulary_acceptance`, after
  all twelve check calls. A failing check panics first — falsification B produced a red run with no
  marker in stdout. Byte-for-byte match with `validation.component_vocabulary`.
- **Typeface negative path (T038).** `check_missing_typeface_is_a_typed_failure` is the primary
  assertion: `expect_err` on a nonexistent directory, typed `FaceUnavailable` naming the weight and
  the file, the visible message required to contain neither "fallback" nor "substitut", plus the
  empty-file `FaceUnreadable` case. It points at a path that does not exist rather than deleting the
  vendored faces; the repository is not mutated.
- **The render path is the production one (T034, T036).** `paint_production_frames` drives the real
  `EframeGraphicalApplication` through a real `egui::Context` with `install_authored_typeface`, at
  both authored viewports in both top-level contexts, and reads the emitted `epaint` shapes. 334
  glyph runs. No downgrade to a declaration-only check anywhere.

## Recorded limitations — checked, not taken on faith

All six are stated in the module docs and the Activity Log. I verified the one that could have been
an evasion: limitation (e) leaves "every declared state painted a specimen at both authored sizes"
to `src/testing/component_gallery_scene.rs`. That module does assert it, over its real paint pass —
`states_painted() == COMPONENT_STATE_COUNT` at `:2621`, `:2630`, `:2961`, with a
`COMPONENT_STATE_COUNT - 1` case at `:2634` proving the ledger can fail. The work is covered, not
dropped. The remaining five (spacing not in the shape stream, radii compared at declaration, split
target measurement, clipping asserted only where nothing scrolls, overlap only between visible runs)
each say where the measurement stops instead of claiming more than it measured.

## Anti-pattern checklist

1. **Dead code** — PASS. The one new production function, `install_authored_chrome`, is called at
   `eframe_graphical_window.rs:207`; falsification E shows what happens without it.
2. **Synthetic-fixture test** — PASS. Falsifications A–F delete or drift the implementation and the
   corresponding test goes red every time.
3. **Silent empty return** — PASS. The only swallow is `remove_dir_all(&empty).ok()` at `:2006`,
   fixture cleanup. `collect_shape`'s `_ => {}` arm is documented: meshes and beziers carry no flat
   color and nothing is claimed about them.
4. **FR coverage** — PASS. NFR-001 (values through the render path), NFR-002 (guard plus its own
   failure proof), NFR-003 (both viewports, band arithmetic, 48 px bound), and NFR-005 / FR-005 /
   FR-010 each carry assertions, not comments.
5. **Frozen surface** — N/A. No file is marked frozen.
6. **Locked decision** — PASS. The two `must not` clauses (`plan.md:201` desktop geometry,
   `spec.md:140` the input-isolated `demo-live-*` witness contract) are untouched; `make test` is
   27/27.
7. **Shared-file ownership** — PASS with coordination noted below.
8. **Production fragility** — PASS. No new `panic!` or `expect` on a production path.
   `install_authored_chrome` is total and idempotent.

## Coordination note — `src/adapter/eframe_graphical_window.rs`

WP06 owns `tests/component_vocabulary.rs`, but the change also carries 146 lines in
`src/adapter/eframe_graphical_window.rs`, which is **WP04's authoritative surface**. This is
recorded here explicitly and is accepted for three reasons:

- The Activity Log discloses it up front, names each of the six defects, and records that the
  operator was consulted on the first two and directed "fix the adapter, assert the real claim".
- WP04 is already **approved**; lane-f is stacked downstream of lane-e which merged lane-d, so there
  is no concurrent writer on the file and no merge hazard.
- The alternative was to weaken the assertions to fit the defects, which is precisely the failure
  mode this WP exists to prevent.

## Verified by running the program

`cargo run --bin crest-synth` opened the real window; screenshot at 1920×1080 confirms the mixer
renders as stacked track columns with label left and value right and no collision, the focus keyline
on `T00 Level` is the authored cyan with its halo, the footer action buttons are full-height targets,
and the band rules are the authored hairline.

## One observation, not a defect

The literal guard is line-local: a call whose arguments are hand-split across source lines is
invisible to it. I confirmed this is unreachable in practice — `cargo fmt --check`, which `make
fmt-check` gates on, **rejects** the hand-split form (exit 1), and when rustfmt does split a deep
call it splits the outer one and keeps `from_rgb(0x65, 0xe5, 0xff)` on a single line, which the guard
catches (falsification F). No change requested; noted so a future reader knows the shape of the
guard's reach.
