---
affected_files:
- src/shell/visual/compositions/application_shell.rs
- src/shell/visual/compositions/context_switch.rs
- src/shell/visual/compositions/footer.rs
cycle_number: 2
mission_slug: crest-component-controls-and-compositions-01KZ25VX
reproduction_command: cargo test
reviewed_at: '2026-08-03T04:10:00Z'
reviewer_agent: paula-patterns
review_status: "pending"
verdict: rejected
wp_id: WP04
---

# WP04 review — cycle 2

Reviewer: paula-patterns (architecture-scout). Delta review against my cycle-1 rejection.
Four lenses: index contract, band-precedence/overflow, plan-order/rationale, mixer+meter ownership.

**This is a strong cycle.** Most of what I asked for is genuinely done, two escalations came back
with better evidence than I had, the footer rationale correction is exactly right, and cycle 2
found and fixed a real defect it had introduced in cycle 1. Gates are green and scope is clean.

But **the blocker is not closed — it moved**, and I only found that by checking the second
consumer. Details in Issue 1.

---

## Verified closed

**Cycle-1 Issues 3 and 4 — CLOSED.** `the_plan_claims_space_in_the_authored_order`
(`application_shell.rs:1539-1586`) pins the sequence index by index on region *and* placement
variant at both policies, with extents from the policy. `observed_bands` (`:279-282`) derives from
`ShellRegionId::ALL` and its test holds it against `surface_descriptor()` itself with an
`assert_ne!` vacuity guard — no fourth hardcoded copy. `band_rect` was correctly *not* rewritten to
derive from the plan; it stays an independent oracle with a separate cross-check test (`:1590`).
The corrected module-doc example traces clean through `try_new_semantic`.

**Cycle-1 Issue 6 — LARGELY CLOSED, and it caught a real defect.** Cycle 1 had dropped the shipped
`ScrollArea` (confirmed: `git show 83a7d4b:...footer.rs | grep ScrollArea` is empty). Cycle 2
restored it and proved the consequence: removing it again fails
`a_hint_the_over_full_band_could_not_show_is_still_reachable` with *"14 hint(s) are unreachable"* —
the reported number reproduces exactly — while every containment assertion and all of
`component_vocabulary` stay green. The gesture is real (`egui::Event::MouseWheel` through three
passes). Forcing fixtures now exist for the context line (`context_switch.rs:339`) and identity
(`identity_header.rs:200`), each self-guarding with `assert!(unbounded > band.width())`.

**Cycle-1 Issue 7 — CLOSED, correctly.** The `clicks >= 8` fabrication is gone. Every claim in the
replacement (`footer.rs:19-36`) verifies: clickability predates foundations (`db635196` used
`ui.small_button().clicked()`), the stroke and `MIN_INTERACTIVE_TARGET_PX` floor came from
`589fa01c` (crest-component-foundations), `DESIGN.md:450` requires minimum targets. The stale
`identity_header.rs` "both labels are painted through `fitted_text`" claim is fixed in prose *and*
in code (the closures were swapped to match). Good.

**Gates, re-run by me:** fmt clean, `clippy --all-targets -- -D warnings` clean, `make lint` clean,
**814 passed / 0 failed**. Scope is exactly the four owned files; `compositions/mod.rs` is untouched
this cycle, so WP05's arms are untouched entirely. No new visual literal.

**Both judgement calls — I agree, with independent evidence.**
*Mixer grid: UNOWNED.* WP05's building subtasks are `ListedRow`-role row stacks plus the 420 px
side panel; `ListedRow` cannot reach `Fader` or `Meter` — only `VerticalStrip` can — so `Section`
does not implicitly cover it; the role vocabulary makes that impossible, not merely unstated. The
crest-spec closes the family at seven (`shell.yaml:217-227`) and puts the sixteen-meter obligation
on the **adapter port** (`:388`). The only instruction pointing at the move is WP06 T029 step 4,
issued to a WP owning one file that may make no layout decision. **Decide before WP05 finishes.**
*Meter: unreachable in production — a scoping defect, not a WP03/WP04 code defect.* `track_control`
(`semantic_graphical_view_model.rs:1388-1391`) emits only `Toggle`/`Continuous`; `Identity` reaches
only `PatchMain`. T041 asserts selector-reachability, which holds. The collision is that the shipped
app paints a live meter pinned under NFR-005 while C-002 forbids the view-model field a composition
would need. Needs one narrowly-scoped ruling from the spec authority.

**T044 blind spot — I agree, and it is worse than stated.** Verified by measurement: with the
ScrollArea removed, 14 of 20 hints are annihilated and `overflow_report` reports **zero** defects.
A shape-stream metric has no denominator; it cannot count what was never emitted. Worse,
`check_no_text_clips_or_overlaps` (`tests/component_vocabulary.rs:1652-1653`) lists only
`ContextLine` and `IdentityHeader` as fixed bands, so a footer run that escapes its container is
not faulted either. **Relay to whoever picks up T044**: it needs a reachability companion that
compares the projection's supplied count against the union of a scrolled sweep, and its
fixed/scrolling exemption should attach to scroll-region clip rects rather than whole bands.
Not WP04's to fix.

---

## Issue 1 — BLOCKER. The label mismatch was relocated, not eliminated: `live_demo_runner.rs` now breaks instead

The `observed_label` fix is correct as far as `tests/graphical_application_shell.rs` is concerned.
I verified it two ways: `band_runs` reads `text.galley.job.text` (`application_shell.rs:1037`) and
the frozen assertion reads the same field with the same exact equality
(`tests/graphical_application_shell.rs:135`), so the new test is genuinely equivalent; and I probed
a `primary_label` long enough to truncate, confirming `galley.job.text` keeps the full original
string at both policies, so equality survives truncation on the identity band too.

**But there is a second consumer of that label, and cycle 2 broke it.**

`src/testing/live_demo_runner.rs:2258-2273`:

```rust
let expected = [
    (
        ShellRegionId::ContextLine,
        shell.context_line().context_label(),      // "PATCH"
    ),
    ...
];
if frame.regions().iter().zip(expected)
    .any(|(actual, (id, label))| actual.id() != id || actual.visible_label() != label)
{
    return Err(LiveDemoError::ShellFrameMismatch);
}
```

It passes today only because the adapter still constructs the observation with
`projection.context_line().context_label()` (`eframe_graphical_window.rs:251`). The moment WP06
wires `band.observed_label(projection)` through — which is exactly what this module's own doc
example instructs — `ContextLine` reports `"* PATCH"`, the comparison fails, and
`tests/live_demo_scene.rs` fails with `ShellFrameMismatch`.

**No work package owns `src/testing/live_demo_runner.rs`.** WP07's `owned_files` are
`component_gallery_scene.rs` and `window_input.rs`; WP06's is the adapter alone; WP08's are
`component_composition.rs`, `component_vocabulary.rs`, `DESIGN.md`, `ROADMAP.md`. So this is the
cycle-1 blocker in a new location, with the same property that made it a blocker: WP06 cannot fix
it from the one file it owns.

Two consumers now hold incompatible expectations, both protected:
- `graphical_application_shell.rs:240-242` — the label must be **painted**, exact galley.
- `live_demo_runner.rs:2266` — the label must **equal `context_label()`**, exactly.

Cycle 1 offered two remedies and cycle 2 chose the one that satisfies only the first. The remedy
that satisfies **both** is to make the band paint the bare label.

**Required change** — in `context_switch.rs`, paint the mark and the label as **two separate
runs** rather than one concatenated string, and revert `observed_label`'s `ContextLine` arm to
`projection.context_line().context_label().to_owned()`:

```rust
// entry_label's concatenation is what breaks whole-galley equality; split it.
for context in TopLevelContext::ALL.into_iter().rev() {
    let selected = context == active;
    text::paint_text(ui, context.label(), TypeStyle::LabelControl,
                     entry_color(selected), ComponentState::Resting);
    text::paint_text(ui, entry_mark(selected), TypeStyle::LabelControl,
                     entry_color(selected), ComponentState::Resting);
}
```

That satisfies every constraint at once: `graphical_application_shell.rs` finds a galley equal to
`"PATCH"`; `live_demo_runner.rs` finds `visible_label() == context_label()`;
`every_band_reports_a_label_it_actually_paints` still finds the reported label among painted runs;
and active/inactive stay distinguishable without color, because the mark is still painted — T019
is satisfied by the mark existing, not by it sharing a galley with the label.

Keep `every_band_reports_a_label_it_actually_paints`. It is a good test and it is what will hold
this closed. Add one assertion to it — or a sibling — that the reported label also equals the value
`live_demo_runner` derives, so the two consumers can never diverge again silently.

## Issue 2 — BLOCKER. `assert_eq!(measured, 3)` breaks the build when WP05 lands, in a file WP05 cannot edit

`application_shell.rs:1475-1482`. The doc above it (`:1450-1452`) claims: *"When they do paint,
they are held to the same rule with no change here."* The label loop does apply automatically —
that half is true. The count assertion does not.

I verified in a scratch copy, twice:

1. Stand-in painting a **wrong** label → fails on the label assertion:
   `PersistentSideRegion at Desktop in PATCH reports "UTILITY", which it never painted; it painted ["PATCH WORKSPACE"]`.
   That is the rule working.
2. Stand-in painting the **correct** labels for both bands — everything right → still fails:
   `assertion left == right failed: Desktop at PATCH measured 5 bands, not the three chrome bands this work package fills / left: 5 / right: 3`

WP05 owns `section.rs`, `patch_strip_row.rs`, `utility_inspector_panel.rs`; WP06 owns the adapter.
Neither can edit `application_shell.rs`. WP05 is implementing now, so this is an imminent lane
block of the same class as Issue 1.

**Required change** — pin non-vacuity by naming the regions that must paint, not by pinning a total:

```rust
let mut measured = Vec::new();
for band in frame_plan(&policy) {
    let region = band.observed_region_id();
    let painted = painted_text(&band_runs(&projection, &policy, region));
    if painted.is_empty() {
        continue;
    }
    measured.push(region);
    let reported = band.observed_label(&projection);
    assert!(
        painted.iter().any(|run| run == &reported),
        "{region:?} at {} in {} reports {reported:?}, which it never painted; it painted {painted:?}",
        policy.canonical_name(), context.label()
    );
}
for region in [
    ShellRegionId::ContextLine,
    ShellRegionId::IdentityHeader,
    ShellRegionId::Footer,
] {
    assert!(
        measured.contains(&region),
        "{} at {} painted nothing into {region:?}, so the label rule was never exercised there",
        policy.canonical_name(), context.label()
    );
}
```

Non-vacuous exactly as before, and it survives WP05 unchanged — which is what the doc already
promises. Keep the doc sentence; it becomes true.

## Issue 3 — A live defect in shipped cycle-2 code: the footer's path label is unbounded

Cycle 2 restored the `ScrollArea` but did **not** restore the path label's width cap. The shipped
`paint_footer` gave the path a hard `Size::relative(FOOTER_PATH_FRACTION)` = 38% cell
(`eframe_graphical_window.rs:44,739`). `band_row` gives the natural half whatever it asks for.

Measured on the delivered tree, with an overlong `path_label` (a deeply nested focus path — the
projection places no bound on it):

```
PROBE Desktop   -> 1 run, 1 containment defect; overflow_report defects = 0
PROBE SteamDeck -> 1 run, 1 containment defect; overflow_report defects = 0
```

The path label runs 2015 px in a 1920 px band, **both hints vanish entirely, and the scroll gesture
does not bring them back** — they are permanently unreachable, not merely off-screen. And
`overflow_report` — T044's own rule — reports zero defects, because the run's clip rect is the
full-screen `CentralPanel`, not the band.

This is precisely the class of defect cycle 2 correctly identified and fixed on the hints side,
re-entering from the other half of the same band. `DESIGN.md:514` says the footer echoes the path
*and* the valid actions; a path that erases every action fails that.

**Required change**: bound the natural half in `band_row`, or bound the path label at its call
site (`footer.rs:144-150`) so the truncating half is guaranteed non-zero width. A fraction is a
visual value, so it must come from `ViewportDensityPolicy` — if no accessor exists, use
`TruncatingHalf`'s minimum and cut the path instead, and record the choice. Add a fixture: an
overlong `path_label` with hints supplied, asserting at both policies that every hint is still
reachable by gesture.

## Issue 4 — `AddressedHint::index()` is `pub`, so the guard is opt-out

`application_shell.rs:327`. The rest of the type is sealed well: fields private, `minted` is
`pub(super)`, no `Default`/`From`/`Deref`/`Deserialize`, not re-exported from `visual/mod.rs`, and
`resolve` genuinely refuses on cardinality mismatch with no panic path. Good design.

But `index()` hands out the raw `usize`, so WP06 can write `valid_actions[hint.index()]` and get
the cycle-1 bug back verbatim — including a panic against today's empty `valid_actions`. The only
callers are three assertions inside `compositions/` (`footer.rs:464,468`, `application_shell.rs:1702`).

**Required change**: `pub const fn index` → `pub(super) const fn index`. Zero call-site cost; it
turns "the guard is available" into "the guard is the only way out".

## Issue 5 — the "not fixable in owned files" claim is false, and the tripwire asserts only half the contract

`footer.rs:509-515` says repairing the fixture desync "means giving `SemanticGraphicalViewModel::fixture`
valid actions, and that file is outside this work package's owned files." That names one repair as
though it were the only one. Three crate-reachable alternatives need no edit outside `owned_files`:

- `StateProjector::project_with_shell` — `src/control/state_projector.rs:219`, fully `pub`, returns
  a projection paired **by construction**.
- `SemanticGraphicalViewModel::project` — `semantic_graphical_view_model.rs:690`, `pub(crate)`.
- `StateProjector::graphical_shell_projection` — `state_projector.rs:393`, `pub`.

And the pattern is already established one directory over: `src/shell/visual/controls/compact_slider.rs:552`
builds a real `AppState` with no asset file and calls `SemanticGraphicalViewModel::project`.

The defensible claim is *"not cheaply repairable for the shared fixture without changing what every
other footer assertion measures"* — true, and materially weaker.

The consequence matters more: against a 4-vs-0 fixture, `resolve` returning `None` is
indistinguishable from `resolve` being unimplemented. The **positive** half — what WP06 consumes —
is exercised only against hand-built arrays (`application_shell.rs:1697-1720`), never a real projection.

**Required change**, all in owned files: add `paired_projection_fixture` to `testing_support` via
`StateProjector::project_with_shell` (leave `projection_fixture` alone), and replace the tripwire
with the biconditional run over both:

```rust
let addressed = AddressedHint::minted(0, hints.len());
if hints.len() == actions.len() {
    assert_eq!(addressed.resolve(actions), actions.first());
} else {
    assert!(addressed.resolve(actions).is_none());
}
```

Keep the desync visible with an `#[ignore = "shared fixture desync"]` test asserting the desired
pairing. A suite should encode what must be true, not pin the current absence of a repair.

---

## Smaller items — fix while the files are open

- **`identity_header.rs:48-49`** — "the only arrangement of this band that compiles" overstates it.
  The types forbid truncating in the first-allocated half; they do not pin which half that is.
  Flipping `BandPrecedence` compiles and is caught by a **test**, not the compiler. Same for
  `application_shell.rs:561-563` and `:613-614`. Credit the fixture, which is what actually holds it.
  (`NaturalHalf::ui()` at `:583` is a necessary and acceptable hatch — no change needed, just
  accurate prose.)
- **`footer.rs:32`** — "the three assertions" is two of three;
  `a_footer_with_no_hints_renders_empty_rather_than_defaulted` passes under plain text. The
  rhetorical point stands; the count does not.
- **`a_hint_the_over_full_band_could_not_show_is_still_reachable`** asserts `.any()` over
  previously-hidden hints, so it proves the gesture reveals *something*, not that all 20 are
  reachable. Sweep to the end and union the passes.
- **The two overlong-band tests** (`context_switch.rs:339`, `identity_header.rs:200`) use
  `band_runs` (centre-in-band) rather than `band_row_runs`, which this file's own comment
  (`:1122-1136`) warns is "exactly the wrong one" for measuring overflow — an escaping run's centre
  can fall outside the band and leave the measurement silently. They pass today by geometry, not
  design. Switch them to `band_row_runs`.
- **Retractions are unlabelled.** Both corrections this cycle are silent prose rewrites. A
  retraction that is not marked as one cannot be audited by the next reader. One line each
  ("cycle 1 said X; measured false, see <test>") would make the record self-describing.

---

## What makes this pass

Issues 1-5. Issue 1 is the one that keeps this a reject: the blocker I rejected cycle 1 for is
still live, relocated to `live_demo_runner.rs`, and still unfixable from WP06's owned files. The
fix is small and satisfies both consumers at once.

Change nothing else — the plan-order pins, `observed_bands`, the type split, the scroll
restoration, the reachability harness, and the rewritten rationale are all correct, and re-opening
them risks what now works.
