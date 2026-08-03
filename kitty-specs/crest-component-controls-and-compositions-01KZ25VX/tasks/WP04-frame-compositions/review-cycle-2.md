---
affected_files: []
cycle_number: 2
mission_slug: crest-component-controls-and-compositions-01KZ25VX
reproduction_command:
reviewed_at: '2026-08-03T01:25:01Z'
reviewer_agent: unknown
verdict: rejected
wp_id: WP04
---

# WP04 review — cycle 1

Reviewer: paula-patterns (architecture-scout). Five lenses over one shared surface:
adapter-destination, footer/test-shaping, literal discipline, ownership/scope, viewport safety.

## What passed, verified independently

- **Gates green.** `cargo fmt --all -- --check` clean, `cargo clippy --all-targets -- -D warnings`
  clean, `cargo test` **725 passed / 0 failed** across 28 suites. The `baseline-tests.json`
  "1 pre-existing failure" is confirmed as `<declared-command>` / `"For more information, try '--help'."`
  — a malformed CLI invocation in the capture harness, not a Rust test.
- **Scope is exact.** Commit `83a7d4b` touches the four owned files plus `compositions/mod.rs`
  in exactly two hunks (four `pub mod` lines; four renderer arms). WP05's `Section`,
  `PatchStripRow`, and `UtilityInspectorPanel` arms are untouched and nothing is reordered.
  `git show 83a7d4b --name-only -- tests/` is empty — NFR-005 satisfied. `density.rs` and
  `token.rs` are untouched.
- **FR-004 holds.** Zero declared visual values in all four files. Every nonzero quantity
  resolves from `ViewportDensityPolicy::{bands, split, rhythm}` or from `SemanticColor` /
  `TypeStyle` / `KEYLINE_RESTING_PX` / `MIN_INTERACTIVE_TARGET_PX`. The only literals are
  `0.0` (no-rounding / no-width-constraint / degenerate guard), `5` (band count), `0` (index).
- **FR-010 holds.** No resolution-specific constant, no branch on a raw viewport size.
  `assert_ne!(frame_plan(Desktop), frame_plan(SteamDeck))` (`application_shell.rs:1036`)
  is a real anti-hardcoding guard, not a restatement.
- **FR-009 / C-001 / C-002 hold.** No cached Patch, focus, navigation, reducer, or audio state;
  no `static mut` / `RefCell` / `OnceCell` / `ui.memory` write; no `AppState`; no `SemanticAction`
  named in any composition source.
- **C-003 holds.** The identity band's third design-file text node has no projection field
  (`ShellIdentityHeader` carries exactly `primary_label` + `secondary_label`) and is genuinely
  omitted — no blank node, no reserved space — recorded at `identity_header.rs:18-23` and pinned
  by `identity_header.rs:154` (`text.len() == 2`). The empty footer is proven by
  `footer.rs:229-247`, which checks both no default text *and* no default target frame.
- **The three Figma gaps are genuinely absent and recorded, not approximated.** Each sits under
  an explicit "Raised, not resolved" heading naming the conservative behavior chosen.
- **`frame_plan` is the right shape.** Band order, extent, surface, panel id, composition binding,
  and `observed_label` are all off the adapter. Roughly 40 of the 51 decisions in `paint_shell`
  and its region painters land cleanly in the plan, the four compositions, WP05's stubs, Phase 4a
  primitives, or the density policy. `arrange_band`'s `activated_hint` leaves the adapter a
  four-line mechanical lookup with no choice in it. This is real work, done well.

The rejection is not about the shape of the handover. It is about five specific things WP06
cannot do through it, two of which are only fixable in WP04-owned files.

---

## Issue 1 — BLOCKER. `context_switch` breaks `assert_frame` the instant WP06 wires it in, and NFR-005 forbids the fix there

`src/shell/visual/compositions/application_shell.rs:158` reports the context-line region under
`projection.context_line().context_label()`, which is exactly `"PATCH"` or `"MIXER"`
(`state_projector.rs:534`). That matches the adapter's current `:251` — correct to preserve.

But `context_switch` no longer paints that string. `entry_label`
(`src/shell/visual/compositions/context_switch.rs:68-75`) returns:

```rust
format!("{mark} {}", context.label())
```

producing `"* PATCH"` and `"  MIXER"` — never the bare label. WP04's own permitted-text
assertion confirms it: `application_shell.rs:1193-1212` lists `product_label`, `status_label`,
both identity labels, `path_label`, `mode.label()`, both `entry_label` forms, and the action
hints. `context_label()` is absent from that list.

`tests/graphical_application_shell.rs:240-242` asserts, for **every** region in the observation:

```rust
for region in observation.regions() {
    assert!(output_contains_text(output, region.visible_label()));
}
```

and `shape_contains_text` (`:134-136`) compares `text.galley.job.text == expected` — exact
equality. Nothing else in either context paints exactly `"PATCH"` or `"MIXER"`: `main_label` is
`"PATCH WORKSPACE · {name}"`, `side_label` is `"UTILITY"` / `"INSPECTOR"`, `path_label` is
`"MIXER / T00 / levelDb"`, identity is `"PATCH 01 · …"`.

So the moment WP06 swaps `paint_context_line` for `context_switch`, this assertion fails for all
four viewport × context cases. NFR-005 forbids editing that test, and WP06 owns only
`src/adapter/eframe_graphical_window.rs` — it cannot reach either file that could fix this.

**Required change** — in a WP04-owned file, either:
- have `context_switch` also paint the bare active-context label (a separate run, so the exact
  galley match succeeds), or
- change `FrameBand::observed_label` for `ShellRegionId::ContextLine` to report the string the
  band actually paints, and add an assertion tying every band's `observed_label` to a run the
  band emits.

The second is the better fix and closes the class: an observation that reports a label nothing
paints is exactly what `compositions/mod.rs:18-22` says the region binding exists to prevent.
Add a test that fails if a band's `observed_label` is not among its painted runs.

## Issue 2 — BLOCKER. `AudioObservationSnapshot` is structurally stranded, and this is the decision to make now

You asked whether the meter's view-data-only path makes the snapshot unnecessary at the
composition boundary. It does not. Verified:

- `GraphicalShellProjection` exposes `generation`, `state_hash`, `context`, `semantic_model`,
  `context_line`, `identity_header`, `workspace`, `footer`. **No level, no meter, no RMS.**
- `SemanticControlValue` (`semantic_graphical_view_model.rs:47-53`) is
  `Scalar | Parameter | Asset | Identity | Summary`. No meter variant.
- `MixerTrackParameter::MAIN` is `[Level, Pan, Mute, Solo]` (`mixer/mixer_track_parameters.rs:23`).
  **No per-track meter control exists in the projection.** The selector maps
  `(Kind::Identity, Role::VerticalStrip) → Meter` (`controls/mod.rs:301`), but no `Identity`-kind
  control appears on `MixerMain`, so that arm is never reached.
- The adapter reads exactly four snapshot values, all in `paint_mixer_workspace`:
  `parameter_generation()` and `active_graph_revision()` for the staleness gate (`:494-495`),
  `audio.track(id)` (`:544`), and `meter.rms()` (`:550`, `:566`).
- The live meter is pinned under NFR-005: `tests/graphical_application_shell.rs:340`
  (`assert!(output_contains_text(&output, "METER 0.000"))`) and `:618-643`
  (`observation_reads.get() == 1` plus `format!("METER {:.3}", mix.track(track).rms())`).

That leaves WP06 three routes, all closed: keep it (violates FR-006, ~60-80 lines against a
budget already over), widen `CompositionRenderFn` / `arrange_band` (requires editing
`compositions/mod.rs` and `application_shell.rs`, neither in WP06's `owned_files`, and it
contradicts the crest-spec `Meter` invariant at `.kittify/crest-spec/contexts/shell.yaml:210-211`),
or drop the meter (breaks two NFR-005-protected assertions).

C-001 puts audio out of scope for this mission, and WP03's meter renders from view data alone —
both true, and neither helps, because the *value* the meter renders has no route into the
projection. This is the mission's critical path and it needs a decision above WP06. The cheapest
place to open the slot is here, while the boundary is being defined.

**Required change**: do not silently leave this for WP06. Either
(a) carry the level into the projection so `Meter` really does render from view data (a projector
change outside WP04 — escalate rather than implement), or
(b) declare an explicit, named seam in the WP04 boundary that WP06 can fill without editing
WP04-owned files, and record it,
and in either case record the decision and its owner in the WP notes so WP05/WP06 inherit it
rather than rediscovering it.

## Issue 3 — The contract example WP06 is meant to follow does not work as written

`src/shell/visual/compositions/application_shell.rs:27-38` is the example loop that defines
WP06's job. It pushes region observations in `frame_plan` order:

```
frame_plan order:        [ContextLine, IdentityHeader, Footer, PersistentSideRegion, MainWorkspace]
ShellRegionId::ALL:      [ContextLine, IdentityHeader, MainWorkspace, PersistentSideRegion, Footer]
```

`ShellFrameObservation::try_new_semantic` (`src/shell/shell_frame_observation.rs:217-223`)
compares the pushed ids against `ShellRegionId::surface_descriptor()` and returns
`ShellFrameObservationError::RegionOrder` on any mismatch. The documented loop therefore fails at
runtime, and the adapter must reorder and re-find the band for each id — 12-15 lines of exactly
the mapping `FrameBand::observed_region_id` was introduced to eliminate ("so wiring the
observation is a move rather than a mapping the adapter has to get right", `:123-126`).

**Required change**: make the ordering requirement part of the contract rather than the adapter's
problem — e.g. expose the plan in canonical observation order alongside claim order, or a helper
that yields bands in `ShellRegionId::ALL` order — and correct the example. Add an assertion tying
the two orders so the doc cannot drift from `surface_descriptor()`.

## Issue 4 — `frame_plan`'s order is declared load-bearing and nothing pins it

`:170-177` says: *"The order is load-bearing and is the adapter's current order preserved
exactly … reordering this changes the layout even though every extent stays the same."*

Every test iterates the plan as a set (`HashSet` at `:955`, `:962`, `:1329`) or looks bands up by
region (`extent(ShellRegion::…)` at `:1013-1035`). **No test asserts the array's sequence.**
Meanwhile `arrange` (`:338-360`) writes its own `StripBuilder` order and `band_rect` (`:807-835`)
writes a third geometric expectation. All three agree today; nothing holds them together.

The module doc at `:44-47` claims the two paths "cannot drift" because both read the same plan.
That is true for extent and surface and false for order — the one dimension the file itself calls
load-bearing, and the one WP06's loop consumes directly.

**Required change**: assert the plan's placement sequence explicitly (five `assert_eq!`s on
`plan[i].region()` and `plan[i].placement()` discriminant), or derive `arrange`'s strip order from
the plan instead of restating it.

## Issue 5 — The footer's contract test cannot exercise the contract it documents

`src/shell/visual/compositions/footer.rs:342-346` says `addressing_a_hint_reports_that_hint_index`
is *"the contract the render adapter resolves against `action_hints()` and the parallel
`valid_actions()`."*

`SemanticGraphicalViewModel::fixture` sets `valid_actions: Vec::new()`
(`src/control/semantic_graphical_view_model.rs:673`), while `fixture_footer`
(`application_shell.rs:593-603`) supplies four hints. So on every fixture this test runs against,
`action_hints().len() == 4` and `valid_actions().len() == 0` — **every index it asserts is out of
range for the collection the adapter is meant to resolve it against**, and nothing notices.

The test genuinely proves the index round-trips through the composition under real pointer events,
at both policies, with a negative case. That part is good and should stay. What it does not prove
is the half its own doc names.

This matters more than a fixture nit because WP04 *introduces* the coupling. The shipped
`paint_footer` (`eframe_graphical_window.rs:755-779`) iterates `valid_actions()` directly and
emits `valid.action().clone()` from the same element it drew — no index, no second collection, a
mismatch is impossible. WP04 replaces that with two collections joined only by convention and a
comment. Production is safe by construction today (`state_projector.rs:535-542` maps over the same
`semantic`), but nothing enforces it: `ShellFooter::new` accepts any `Vec<String>`, and
`GraphicalShellProjection::new` / `from_data` validate seven coherence conditions, none of which
looks at the footer. The only guard anywhere is a single `len()` equality at one state in
`tests/semantic_graphical_view_model.rs:345-348` — no ordering check, no construction-site
invariant, no type-level guarantee.

**Required change** (in scope): add
`assert_eq!(projection.footer().action_hints().len(), projection.semantic_model().valid_actions().len())`
to this test and give the fixture `valid_actions` that pair with its hints. It will fail today,
which is the signal. **Recommended additionally** (may need a scope amendment): replace
`activated_hint: Option<usize>` with a newtype constructible only from `ShellFooter`, so an index
cannot be minted against an unrelated count. Carrying the pairing in one collection is the real
fix, but `ShellFooter` is outside `owned_files` — raise it rather than reach for it.

## Issue 6 — `BandPrecedence` is documented-only, and the rule is unfalsifiable on two of three bands

The rule at `application_shell.rs:413-431` is correct and correctly applied at all three call
sites — `context_switch.rs:122` (`Trailing`), `identity_header.rs:79` (`Trailing`),
`footer.rs:113` (`Leading`), each with a reasoned inline comment, and in each case the
first-laid-out half uses `text::paint_text` while only the second uses `fitted_text`.

But `band_row` (`:450-458`) hands both closures the same `&mut Ui`, and `fitted_text` (`:514`)
accepts any `&mut Ui`. `precedence` is a runtime value with no link to closure content. A future
call site can pass `BandPrecedence::Leading` and call `fitted_text` in the leading closure; it
compiles, and it silently reintroduces the overlap the doc exists to prevent.

Only `identity_header.rs:190-238` has a fixture long enough to force truncation (with a good
self-check at `:213-218` that refuses to pass against a fixture that fits). The context line and
the footer run against `projection_fixture`, whose content is short. **Inverting
`context_switch.rs:122` or `footer.rs:113` today leaves all 43 tests green.**

Related and more urgent: the footer's recorded overflow behavior (`footer.rs:53-55`) is *"hints
are cut off at that edge"* — that is clipping, which is precisely what WP08's T044 counts as a
defect (`clipped_or_overlapping_text == 0`). No test supplies more hints than fit, so whether the
chosen behavior satisfies T044 is currently unmeasured.

**Required change**: add forcing fixtures for the context line (a `product_label` longer than the
band) and the footer (~20 hints), asserting `containment_defects` is empty at both policies. That
makes the precedence rule falsifiable on all three bands and measures the hint-overflow claim
against the same defect T044 will count. Consider `NaturalHalf` / `TruncatingHalf` newtypes on
`band_row` to make the rule enforced-by-construction — `fitted_text` is already `pub(super)` with
two call sites, so the blast radius is small.

## Issue 7 — Correct the recorded rationale for the footer decision

**The decision is right. The reason recorded for it is false, and it will be inherited.**

Keeping the footer's hints clickable, stroked, and at the authored minimum height preserves real
shipped behavior. `git blame -L 755,780 src/adapter/eframe_graphical_window.rs` shows `db63519`
introduced `ui.add(button).clicked() → emitted = Some(valid.action().clone())` two missions ago,
and `589fa01` (the `crest-component-foundations` mission) added `.stroke(KEYLINE_RESTING_PX, …)`
and `.min_size(vec2(0.0, MIN_INTERACTIVE_TARGET_PX))`. `footer.rs:158-168` copies that shape
line for line. The divergence from the Figma plain-hint-text `Controls` frame is inherited, not
introduced — and dropping touch addressability on a Steam Deck target to match a desktop-only
frame would have been the wrong call.

But the stated justification — that replacing them with plain text would drop `clicks` below 8 and
fail a test NFR-005 freezes — does not survive contact with the code:

- `tests/component_vocabulary.rs` sweeps `EframeGraphicalApplication::update`
  (`paint_production_frames`, `:598-683`). It contains zero references to `composition`,
  `ShellComposition`, or `controls::`.
- WP04 touched no adapter file, and nothing calls `arrange_band` outside `compositions/`.
- **`footer.rs` contributes exactly 0 to both `framed` and `clicks`.** Making its hints plain text
  could not have failed that test.
- The claim that the footer dominates *both* counts is half-wrong: `clicks >= 8` (`:1628-1632`)
  genuinely does depend on the *adapter's* footer buttons (~4 without them, from the lone
  `CollapsingHeader`), but `framed >= 8` (`:1609-1613`) is carried comfortably by the 16
  mixer-track frames (`:519-524`) and the control rows.
- What *would* fail on plain text are three tests written in this WP —
  `footer.rs:281`, `:347`, `:229`. NFR-005 does not protect those. "These are my own tests" is the
  honest framing.

An existing test asserting a click count is not authority over what the design says the footer is.
Here it did not need to be, because the shipped adapter is. **Required change**: rewrite the
justification in `footer.rs` (and the WP record) to rest on preserved shipped behavior and the
touch-target requirement, and keep the Figma divergence recorded as raised-not-resolved. Do not
leave a false constraint for WP06 and WP08 to inherit.

## Issue 8 — Two constants and a policy file no work package owns

`WORKSPACE_TITLE_ROW_PX = 42.0` (`eframe_graphical_window.rs:34`, used at `:446` and `:629`) and
`MIXER_TRACK_MIN_WIDTH_PX = 176.0` (`:37`, used at `:536`) must land as `ViewportDensityPolicy`
accessors in `src/shell/visual/density.rs` — nothing in `bands()` (48/72/896/64, 40/60/644/56),
`rhythm()` (52/66, 48/56), `split()` (420/320), or `utility_control()` (380/280) sizes a sub-band
title row or a mixer column. The adapter's own doc at `:26-33` already records the gap and defers
it to "the follow-on mission" — this one.

**No work package in this mission owns `density.rs`.** WP05 owns three composition files, WP06
owns only `src/adapter/eframe_graphical_window.rs`, WP08 owns two test files plus `DESIGN.md` and
`ROADMAP.md`. WP05's DoD forbids declaring them locally ("Zero literals"; reviewer guidance #1
makes any bare pixel constant a reject). So today these two constants have no legal home.

This is not WP04's defect — the three fraction constants that *were* WP04's territory
(`CONTEXT_PRODUCT_FRACTION`, `CONTEXT_MODE_FRACTION`, `FOOTER_PATH_FRACTION`) are correctly
dissolved by the `band_row` rhythm. **Required action**: raise it as a scope amendment
(`density.rs` added to WP05's or WP06's `owned_files`) rather than leaving WP06 to discover it.

## Issue 9 — The line budget does not close

For the record, since NFR-003 is the constraint this signature exists to serve:

```
  330   must-stay non-test (window setup, app impl, input translation,
        observation construction, panel construction, band loop)
+  33   install_authored_chrome + shell_frame — needs an egui::Context;
        every WP04 entry point takes &mut Ui, and WP06 cannot add one
+  60   mixer meter residue (Issue 2)
+   8   the two sub-band constants (Issue 8)
+ 199   inline mod tests, already trimmed by 44
─────
  650   projected landing, against ≤512
```

Best case with every blocker solved is ~529 — still over. `install_authored_chrome`
(`:322-348`) is worth calling out separately: it binds six egui `Style` visuals to `SemanticColor`
roles, it is pure visual authority, WP06's own T030 step 6 demands it move, and there is no
`Context`-taking entry point in `application_shell` for it to move into.

**Required action**: at minimum, add a `Context`-taking entry point to `application_shell` so
chrome installation has a destination, and flag the residual budget so WP06 is not set up to fail
NFR-003 by arithmetic.

---

## Smaller items, fix while the files are open

- **`identity_header.rs:33-35`** says *"both labels are painted through `fitted_text`"*. Only the
  primary is (`:81`); the secondary uses `paint_text` (`:92`) — which is what the precedence rule
  requires, and what `:36-40` correctly describes. The prose contradicts the code three lines below
  it and the rule in Issue 6.
- **`context_switch.rs`** carries no record of the "no compact frame exists for this band" gap.
  `identity_header.rs:28-31` and `footer.rs:51-57` both record it; the context line is the third
  band it applies to.
- **`band_for`** (`application_shell.rs:395-400`) falls back to `plan[0]`. Unreachable by
  construction and documented, but a `debug_assert!(band.region == region)` in the `None` arm costs
  nothing and turns a silent double-tiling into a signal.
- **`frame_plan` binds `MainWorkspace` → `ShellComposition::Section`** unconditionally, so the
  Patch-vs-Mixer surface branch (`eframe_graphical_window.rs:458-463`) has to live inside `Section`
  — which WP05's T023 specifies as "a titled group of rows", not a surface dispatcher. Reachable,
  but not what WP05 was told to build, and WP06 cannot reconcile it from its own file. Worth
  aligning now.
- **`compositions/mod.rs:406-416`** `composition_sources()` uses a non-recursive `read_dir`, so a
  future `compositions/section/mod.rs` would escape the `SemanticAction` and wildcard-arm scans.
  Not a WP04 defect; flag it to WP05.
- **`band_runs`** (`application_shell.rs:846`) filters `!content.trim().is_empty()`, so a
  whitespace-only placeholder node would be invisible to the "paints only what the projection
  supplied" assertions. No such node exists today; note it as a regression-detection gap.
- **`both_authored_viewports_retain_every_band_and_the_side_region`** (`:1096-1131`): the
  `band_rect`-based half is tautological — `band_rect` is a pure function of the policy, so
  `side.width() == policy.split().side_px` cannot fail whatever the composition does. The failure
  messages ("dropped", "narrowed or hid the persistent side region") promise more than the
  assertions deliver. The `band_runs` half (`:1140-1151`) is genuinely coupled and is what carries
  the test.

---

## Verdict

The frame handover is well-designed and most of it should survive rework unchanged. **Issues 1, 3,
4, 5, 6, and 7 are fixable inside `owned_files` and should be fixed in this cycle.** **Issues 2, 8,
and 9 need a decision above this work package** — raise them, record them, and do not let WP06
inherit them undeclared.

Issue 1 is the one that makes this a reject rather than a set of notes: it is a concrete NFR-005
break that WP04 causes, that surfaces only when WP06 wires the band in, and that WP06 cannot fix
from the one file it owns.
