---
affected_files:
- src/shell/visual/compositions/application_shell.rs
- src/shell/visual/compositions/context_switch.rs
cycle_number: 3
mission_slug: crest-component-controls-and-compositions-01KZ25VX
reproduction_command: cargo test --lib compositions::
reviewed_at: '2026-08-03T05:40:00Z'
reviewer_agent: paula-patterns
review_status: "pending"
verdict: rejected
wp_id: WP04
---

# WP04 review — cycle 3

Reviewer: paula-patterns (architecture-scout).

## Reviewer error, stated up front

**I approved this work package earlier in this cycle and that approval was premature.** I issued it
after verifying the five cycle-2 items myself but before all review lenses had returned. Two of them
came back afterwards with defects I had missed, and I have since reproduced both independently. My
own checks missed them for two specific reasons worth recording, because they are the same reasons a
reviewer would miss them again:

- I ran targeted test filters (`--lib compositions::footer`, single test names) rather than the whole
  `compositions::` module, so I never saw the second lane block fire.
- I audited the reapplied `context_switch.rs` at **item** granularity — every const, function, test
  and doc heading — and it was complete at that granularity. The loss was **inside** a surviving
  test. That is precisely the failure mode I was asked to hunt for, and item-level inventory is
  blind to it.

The approval is withdrawn. Everything I verified as closed in that note remains closed; the two
findings below are additional.

## Confirmed still closed (unchanged from my approval note)

The blocker fix, the `measured == 3` removal, the path-label reserve (I falsified it myself), the
`AddressedHint::index()` removal, the biconditional tripwire and `#[ignore]`d forward test, the
labelled retractions, gates (fmt 0, clippy 0, `make lint` 0, 816/0/1), scope (four owned files,
`compositions/mod.rs` byte-identical — verified by empty `git diff` and `git log`), and no new visual
literal. None of that is reopened.

---

## Issue A — SHIP-BLOCKING. A second cross-WP lane block, same class as the one cycle 3 just fixed

`src/shell/visual/compositions/application_shell.rs:1990-2034`,
`the_frame_paints_only_text_the_projection_supplied`.

It measures the **whole frame** — `painted_text(&painted_frame(&projection, &policy))` at `:2021` —
against a **chrome-only whitelist** built at `:1999-2019`: the two context-line labels, both identity
labels, the footer path, the mode label, both `TopLevelContext` labels, the two entry marks, and the
action hints. `projection.workspace().main_label()` and `side_label()` are absent, as is anything
else WP05 will ever paint.

Reproduced by me on the cycle-3 tree, with `Section` and `UtilityInspectorPanel` pointed at stand-ins
painting the **correct** labels:

```
test ...application_shell::tests::every_band_reports_a_label_it_actually_paints ... ok
test ...application_shell::tests::the_frame_paints_only_text_the_projection_supplied ... FAILED

panicked at src/shell/visual/compositions/application_shell.rs:2026:21:
the frame invented "PATCH WORKSPACE" at Desktop in PATCH

test result: FAILED. 55 passed; 1 failed; 1 ignored
```

This is structurally identical to the cycle-2 `assert_eq!(measured, 3)` finding: WP04-authored (it
traces to `83a7d4b`), lives in a file WP05 cannot edit, and fires the moment WP05 paints. It is
**worse** than the count pin, because adding two labels to the whitelist is not a fix — the whitelist
adjudicates every string in the frame, so WP05's section titles, parameter names, parameter values
and patch-strip row text will each be reported as invented.

**Required change.** Scope the loop to the three bands this work package fills, exactly as this
test's own band-scoped siblings already do (`identity_header.rs:176`, `context_switch.rs:319`):

```rust
for region in [
    ShellRegionId::ContextLine,
    ShellRegionId::IdentityHeader,
    ShellRegionId::Footer,
] {
    for run in painted_text(&band_runs(&projection, &policy, region)) {
        let trimmed = run.trim();
        if trimmed.is_empty() {
            continue;
        }
        assert!(
            permitted.iter().any(|allowed| allowed.trim() == trimmed),
            "the {region:?} band invented {run:?} at {} in {}",
            policy.canonical_name(),
            context.label()
        );
    }
}
```

This preserves exactly what the test measures today — only chrome paints today — and stops WP04
adjudicating regions it does not fill. C-003 for the workspace bands is WP05's to assert in WP05's
own files.

## Issue B — SHIP-BLOCKING. The reapply silently dropped the assertion that pins T019's actual product invariant

Cycle 2 had, inside `both_top_level_contexts_are_painted_and_exactly_one_is_marked` (`82f35c7`
`context_switch.rs:228`):

```rust
assert!(marked[0].contains(active.label()));
```

That line tied the mark to the **active** context. It is gone from cycle 3 and was not replaced. The
cycle-3 successor (`context_switch.rs:258-268`) only *counts* marks:

```rust
let marks = text.iter().filter(|run| run.as_str() == ACTIVE_CONTEXT_MARK).count();
assert_eq!(marks, 1, ...);
```

Reproduced by me. Mutating `context_switch.rs:181` from `let selected = context == active;` to
`let selected = context != active;` — painting the `*` next to the **inactive** context, an
operator-visible inversion of the invariant T019 exists to protect:

```
cargo test --lib                          -> 735 passed; 0 failed; 1 ignored
cargo test --test component_vocabulary    ->  11 passed; 0 failed
cargo test --test graphical_application_shell ->  5 passed; 0 failed
```

**Nothing in the repository catches the mark landing on the wrong entry.** A user who cannot
distinguish the two accent colors would read the switch backwards, and the suite stays green.

This matters more than a missing assertion normally would, because it is the exact risk the
reapply-from-`git checkout` disclosure was supposed to be audited for, and because splitting the mark
into its own run — the correct fix for the blocker — is what made the old string-containment
assertion impossible. The replacement had to be geometric and was not written.

**Required change.** Assert the mark's adjacency to the *active* label geometrically, using the same
`band_runs` measurement the T019 test already performs:

```rust
let mark = runs.iter().find(|r| r.content == ACTIVE_CONTEXT_MARK).expect("the active mark is painted");
let active_label = runs.iter().find(|r| r.content == active.label()).expect("the active label is painted");
let other = TopLevelContext::ALL.into_iter().find(|c| *c != active).expect("two contexts");
let other_label = runs.iter().find(|r| r.content == other.label()).expect("the inactive label is painted");
assert!(
    (active_label.rect.min.x - mark.rect.max.x).abs() < (other_label.rect.min.x - mark.rect.max.x).abs(),
    "{} at {}: the mark is nearer the inactive entry",
    active.label(), policy.canonical_name()
);
```

Verify it by the mutation above: `context != active` must fail it.

While there: `the_active_entry_is_distinguishable_from_the_inactive_one_without_color`
(`context_switch.rs:280`) does not asssert which entry is active — it proves the switch does not
shift and that two helper functions differ. The name should match the body, or the body should match
the name. Dropping the mark entirely leaves that test green; only its sibling catches it.

---

## Not ship-blocking — record and route

1. **The reachability sweep's aim has a silent fallback that reinstates the disclosed bug.**
   `footer.rs:312-314`: `band_rects(...).first().map_or_else(|| band.center(), |t| t.rect.center())`.
   `band_rects` filters targets whose *centre* is inside the band, but the reserved strip is 72/64 px
   while a hint target is 103–303 px wide, so the first target's centre sits outside the band as soon
   as the first hint grows slightly. Measured: changing the first fixture hint from `"1 OPEN MIXER"`
   to `"1 OPEN THE MIXER"` — four characters — makes the shipped test fail with a false negative
   (test reports 1/4 reachable; truth is 4/4). Margin is one two-character token on SteamDeck. Also,
   with zero expected hints the sweep passes vacuously. The failure direction is safe, but the
   obvious repair when it fires is to shorten the fixture, which guts the test.
2. **T044 needs a reachability companion**, and inherits (1)'s fallback, the vacuous-on-empty pass,
   frame-wide unattributed run matching, and a single sweep direction. Route to WP08.
3. **The footer reserve is adequate but crude.** 72/64 px is 3.75%/5.00% of the band against the
   shipped adapter's 38%. The one visible hint shows ~18-21% of its glyphs, hard-cut with no ellipsis.
   Nothing asserts the visible strip stays >= `MIN_INTERACTIVE_TARGET_PX`; it does today by
   arithmetic coincidence. The principled fix is the `ViewportDensityPolicy` band-split accessor the
   doc says does not exist — which is the unowned `density.rs` gap.
4. **`density.rs` and `src/testing/live_demo_runner.rs` are owned by no work package.**
5. **`context_switch.rs:16-19`** claims an exhaustive `match` on `TopLevelContext` "below"; there is
   no such match — the file iterates `TopLevelContext::ALL`. Pre-existing, identical in cycle 2, but
   false as written: adding a variant would error in `top_level_context.rs`, not here.
6. **`entry_mark(false)` in the `permitted` lists** (`context_switch.rs:330`, `application_shell.rs:2017`)
   is unreachable — `band_runs` strips whitespace-only runs at `application_shell.rs:1212`.
7. **Mixer grid unowned; meter production-unreachable.** Unchanged, for the spec authority.

---

## What makes this pass

Issues A and B only. Both are test-side, both are in WP04-owned files, and together they are roughly
25 lines. Change nothing else — the cycle-3 production fixes are correct and I verified them.

Issue A is urgent independently of the rest: WP05 is in review now and will hit it on merge.
