# Mission Review Report: crest-component-controls-and-compositions-01KZ25VX

**Reviewer**: Claude (post-merge mission review, `spec-kitty-mission-review`)
**Date**: 2026-08-03
**Mission**: `crest-component-controls-and-compositions-01KZ25VX` — Crest Component Controls and Compositions
**Baseline commit**: `438d25630d2b0f49e28bf4aabfe5ba64e5678ff5`
**HEAD at review**: `78be821`
**Merge target**: `feat/crest-component-controls-and-compositions` (52 commits ahead of `main`; PR to `main` not yet open)
**WPs reviewed**: WP01–WP09 (all `done`)

---

## Gate Results

The four gates this skill declares (contract tests, architectural tests, cross-repo
e2e, issue matrix) are `spec-kitty`-repository gates. `crest-synth` declares its own
deterministic validations through the crest-spec, so the equivalent gates are run
below and the two that have no analogue here are recorded N/A with the reason.

### Gate 1 — Deterministic acceptance (crest-spec + declared validations)
- Artifact: `kitty-specs/<slug>/deterministic-acceptance.json` (schema v3.0)
- crest-spec gate: `passed`, 18 spec files, 0 diagnostics
- Declared project checks: 31 declared, 31 resolved, 31 executed, 0 diagnostics
- Result: **PASS**
- Note: the artifact's own `semanticReview` block records `not_evaluated` — deterministic
  acceptance does not establish semantic conformance. That is what this review supplies.

### Gate 2 — Full test suite
- Command: `cargo test --all-targets`
- Exit code: 0
- Lib target: 799 passed, 0 failed, 1 ignored (deliberate forward test)
- Every integration target green, including `component_composition` (13 tests) and
  `component_vocabulary` (11 tests)
- Result: **PASS**

### Gate 3 — Cross-repo E2E
- Result: **N/A**. There is no `crest-synth-end-to-end-testing` repository and the
  mission declares no cross-repo behavior. Recorded rather than waived — no
  `mission-exception.md` is needed because no gate applies.

### Gate 4 — Issue matrix
- File: `kitty-specs/<slug>/issue-matrix.md` — absent
- Result: **N/A**. `spec.md` references no GitHub issues, so `finalize-tasks`
  correctly scaffolded no matrix. Verified by inspection of `spec.md`.

No hard gate fails.

---

## FR Coverage Matrix

| ID | WP | Test / evidence | Adequacy | Finding |
|----|----|-----------------|----------|---------|
| FR-001 Control family | WP01 | `component_composition.rs::selection_is_total_over_kind_and_role_with_every_control_reachable` — `control_for` is one exhaustive tuple match, no `_` arm; 28 pairs | ADEQUATE | — |
| FR-002 Product shapes | WP02/03 | `::every_drivable_pair_paints_through_the_production_render_path`; 8 separate control modules, all reached via `control_for` | ADEQUATE | F-01: toggle, choice adjacency, meter have no Figma specimen (product decision, disclosed) |
| FR-003 Nine-state rendering | WP02/03 | `::every_control_paints_every_applicable_state_without_relying_on_color` + per-control pairwise non-color assertions | ADEQUATE | — |
| FR-004 Composition family | WP04/05/09 | `::every_shipped_region_is_produced_by_a_declared_composition`; `ShellComposition` closed at 8, exhaustive `renderer()` at `compositions/mod.rs:288` | ADEQUATE | — |
| FR-005 Shell composes from library | WP06 | Same test, driving real `EframeGraphicalApplication::update` | ADEQUATE | See RISK-2 (one adapter paint remains) |
| FR-006 Adapter holds no visual decisions | WP06 | `::no_visual_decision_survives_outside_the_visual_module` + 3 supporting guard tests, mutation-verified | PARTIAL | RISK-2, RISK-4 |
| FR-007 Gallery coverage | WP07 | `component_vocabulary.rs` set equality over `ALL_COMPONENT_CONTROLS` / `ALL_SHELL_COMPOSITIONS`; 15 pages | ADEQUATE | — |
| FR-008 Coverage assertion | WP07/08 | Generic over declared families, not a hardcoded count | ADEQUATE | — |
| FR-009 Components own no app state | WP01–09 | `::no_component_owns_caches_or_dispatches_application_state` — source scan over ≥20 visual sources for `SemanticAction`, interior mutability, `AppState`/`AppLoop`/`AppEvent`, `screen_rect` | PARTIAL | RISK-3 |
| FR-010 Both viewports from policy | WP09 | `::both_authored_viewports_survive_recomposition`; `ViewportDensityPolicy::mixer_column()` replaced the invented `MIXER_TRACK_MIN_WIDTH_PX` | ADEQUATE | — |
| FR-011 Figma-authored appearance | all | Manual review; the 176px→82px correction retired the horizontal `ScrollArea` and made all 16 faders visible per `DESIGN.md:462` | ADEQUATE | F-11 residual band shrink, recorded |
| FR-012 Additive page vocabulary | WP07 | Eight `(page, digit)` bindings frozen as data at `component_gallery_scene.rs:185-207`; descriptor count 33→41 asserted | ADEQUATE | — |
| FR-013 DESIGN.md corrected | WP08 | `DESIGN.md` diff present in `438d256..HEAD` | ADEQUATE | — |
| FR-014 ROADMAP amended | WP08 | `ROADMAP.md` +27/−? in diff | ADEQUATE | — |
| NFR-001/002 Gallery latency | — | Operator-judged by design, rationale recorded in `spec.md:156` | ADEQUATE (by declaration) | — |
| NFR-003 Adapter size | WP06 | `check_the_render_adapter_is_at_or_below_its_declared_size`, ceiling 497 production lines | **MISSING vs stated number** | DRIFT-2 |
| NFR-004 No visual literals outside module | WP06/08 | Repo-wide guard, 130 sources / 82,372 lines | PARTIAL | RISK-4 |
| NFR-005 Suite unbroken | WP06 | Suite green; `component_vocabulary.rs` edit proved forced | ADEQUATE | DRIFT-3 (record undercounts modified files) |
| NFR-006 / C-001 Demo plays nothing | WP07 | No audio device, no MIDI source constructed | ADEQUATE | DRIFT-1 (the retracted *proof* layer shipped) |
| C-002…C-006 | all | Automated, per acceptance matrix; independently spot-checked | ADEQUATE | — |
| C-007 No proof-about-proof work | WP08 | Acceptance matrix records "retracted before WP08 was dispatched, and no proof-about-proof layer shipped" | **FALSE against the merged tree** | DRIFT-1 |

---

## Drift Findings

### DRIFT-1: The retracted silence-proof layer is in the merged tree

**Type**: LOCKED-CONSTRAINT VIOLATION (C-007) / RECORD INACCURACY
**Severity**: MEDIUM
**Spec reference**: `spec.md:166-168`, C-007, `acceptance-matrix.json` C-007 note

**Evidence** — `src/testing/component_gallery_scene.rs`:
- `:1400` `const GALLERY_SCENE_SOURCE: &str = include_str!("component_gallery_scene.rs");`
- `:1420` `fn audio_or_midi_constructed() -> bool`
- `:1430` `fn production_source(source: &str) -> String` (strips comments and the test module)
- `:1460` `fn source_constructs_audio_or_midi(source: &str) -> bool` with seven runtime-assembled needles
- `:1178` / `:1222` / `:1281` — `audio_or_midi_constructed: bool` field on the observation and its accessor
- Section heading at `:1392`: *"The silence is derived, not declared"*

`spec.md:166` retracts precisely this: *"That became … a formal measured-and-provable
silence property: a derived `audioOrMidiConstructed` field, witness predicates,
source-scan derivations … That escalation was not asked for and is exactly the
proof-about-proof layer C-007 forbids."* `acceptance-matrix.json` records C-007 as
`pass` with the note *"retracted before WP08 was dispatched, and no proof-about-proof
layer shipped."*

**Analysis**: the retraction landed in the specification and in the acceptance record
but not in the code. WP07 had already implemented T038/T039 to the pre-retraction
reading, and no work package was asked to remove it. The shipped consequence is small —
the derivation is correct, self-tested, and harmless — but the acceptance record states
as fact something the tree contradicts, and a module that embeds its own ~5,500-line
source into every binary to assert a property about itself is the exact shape C-007
forbids. Fix is deletion, not repair.

---

### DRIFT-2: NFR-003's stated number is not met, and the requirement text was never amended

**Type**: NFR-MISS
**Severity**: LOW
**Spec reference**: NFR-003 (`spec.md:157`), status still `Open`

**Evidence**:
- `wc -l src/adapter/eframe_graphical_window.rs` → **740**. NFR-003 requires ≤ 40% of 1,282 = **512**.
- Production span (lines before `#[cfg(test)]` at `:499`) → **497**.
- `tests/component_composition.rs:1755` — `const ADAPTER_PRODUCTION_LINE_CEILING: usize = 497;`

**Analysis**: the mission substituted a production-only measure for the whole-file
measure NFR-003 states, on the reasoned ground that 243 of the original 1,282 lines are
the adapter's own test module which NFR-005 forbids touching. The substitution is
defensible and the delivered production span (497) actually satisfies NFR-003's *number*
under that measure. The finding is that NFR-003's text was never amended to say so, and
`acceptance-matrix.json` grades it `pass` while its own note reads "THE STATED NUMBER IS
NOT MET AND CANNOT BE." A reader of `spec.md` alone still sees an unmet requirement.

---

### DRIFT-3: The acceptance record undercounts modified pre-existing tests

**Type**: RECORD INACCURACY (NFR-005)
**Severity**: LOW
**Spec reference**: NFR-005, `acceptance-matrix.json` NFR-005 note

**Evidence**: `git diff 438d256..HEAD --stat -- tests/` reports three files:
`component_composition.rs` (new), `component_vocabulary.rs` (+165/−37), and
**`exhaustive_demo_scene.rs` (+11/−?)** — the descriptor-count assertion moved 33 → 41.
The NFR-005 note names `component_vocabulary.rs` as *"the one existing test file this
mission may modify."*

**Analysis**: the `exhaustive_demo_scene.rs` edit is legitimate and declared — T034
requires the `WindowInput` descriptor count to move with the four new keys, and
`tasks.md:223` states the 33 → 41 invariant explicitly. The defect is only that the
acceptance record says "one" where the tree says two. Nothing to fix in code; the record
should be corrected so a future reader does not treat the second edit as unsanctioned.

---

## Risk Findings

### RISK-1: The Utility/Inspector lost its scroll region — roughly half the mixer Inspector is unreachable

> **CLOSED 2026-08-03, after this review, at the operator's direction.** See
> *Resolution* at the end of this finding. The verdict below was **FAIL** when the
> review was issued; it is **PASS WITH NOTES** now that this is fixed.

**Type**: CROSS-WP INTEGRATION / BEHAVIOR REGRESSION
**Severity**: HIGH
**Location**: `src/shell/visual/compositions/utility_inspector_panel.rs` (no scroll region);
regression introduced by the WP06 deletion of `eframe_graphical_window.rs:643` at mission base
**Trigger condition**: MIXER context at either authored viewport — no gesture required

**Evidence, verified in the merged tree**:
- `grep -rn ScrollArea --include="*.rs" src/` returns **exactly one hit**:
  `src/shell/visual/compositions/footer.rs:231`, and it is `ScrollArea::horizontal()`.
  No composition provides vertical scrolling.
- At mission base the adapter wrapped the side region in
  `egui::ScrollArea::vertical().id_salt("crest-synth-side-controls")`. WP06 correctly
  deleted the adapter's paint; nothing received the scroll.
- The shipped acceptance test **measures and permits** the overflow:
  `tests/component_composition.rs:2459-2465` classifies a run that leaves
  `PersistentSideRegion` downward as `Escape::BelowTheBand`, and `:2210`
  `const SIDE_REGION_OVERFLOW_CEILING: usize = 22;` allows up to 22 escaping runs per frame.
- Measurement recorded in `cross-wp-findings.md` F-18: **39 runs escape at 1920×1080, 46 at
  1280**. `Return B0 Level` paints at y=1039 against a panel ending at 1016; `Return B6 Level`
  at y=1795. Everything from `Return B0`'s level row down — the returns' levels, all of
  `Return B1`–`B7`, and master gain — is painted off the panel and reachable by no gesture.

**Analysis**: this is the canonical cross-WP hole. WP06 owned the deletion, WP05 owns the
composition that should have absorbed the scroll, and neither reviewer had scope over both.
No test failed: `check_no_text_clips_or_overlaps` asserts only on `ContextLine` and
`IdentityHeader`, so the whole suite is green with the regression present.

It contradicts two of the mission's own acceptance scenarios:
- US2 scenario 4 — *"the existing shell, projection, and focus tests pass unchanged — this is a
  re-composition, not a behavior change."* It is a behavior change; the tests simply cannot see it.
- US2 scenario 5 — *"the header/footer bands, **the visible Utility/Inspector**, and the minimum
  interactive targets are all retained, as `DESIGN.md:450` requires."*

It is **not** covered by the operator's 2026-08-03 display-fidelity relaxation
(`spec.md:170`), which names *"seat widths, band-height derivations, and text-clipping
details."* F-18's author draws the distinction explicitly: *"it is content the operator
could reach and now cannot."* F-18's recorded owner is *"WP05's composition surface /
mission review"* — it was deferred to this review for a decision, not accepted.

**The decision F-18 asks for**: restoring a vertical scroll region in
`utility_inspector_panel.rs` matches shipped behavior and is the cheap answer, but
`MixerStripBank`'s authored rule is deliberately *"uniform narrowing, never scrolling and
never elision"* — the Inspector is a different surface and may legitimately answer
differently. Whoever takes it should also check the shorter PATCH Utility surface.

#### Resolution

**Scroll restored, not narrowing.** The bank's rule governs a fixed cardinality of sixteen
that the design seats inside the authored content width; the Inspector's extent is vertical
and driven by how many return buses, sends and routed patches the projection carries, so
there is no width budget to reclaim and no authored count to fit.

- `src/shell/visual/compositions/utility_inspector_panel.rs` — `entry_viewport`, a
  `ScrollArea::vertical()` arranged *below* `render_title` so the surface stays named while
  its entries scroll. Mirrors `footer.rs:231`'s existing use.
- `tests/component_composition.rs` — `classify_escape` rewritten. The old rule permitted any
  downward escape from `PersistentSideRegion` up to `SIDE_REGION_OVERFLOW_CEILING = 22`;
  the constant is deleted. The new rule discriminates on **reachability**, read from the
  clip rectangle the shape stream already carries: a run whose container *is the band* has
  no scroll region it could have come from and is a `Defect`; a run clipped by a viewport
  strictly inside the band was composed into a scroll area and is `ScrolledOutOfView`.

**Measured on the production frame, both ways.** Before the fix the escaping runs carried the
band rect itself as their clip — `[[1500,120]-[1920,1016]]` at 1920×1080, the band exactly.
After, they carry `[[1500,154]-[1920,1016]]`, the entry viewport below the pinned title.

**Falsified**: stashing only the composition change and re-running
`both_authored_viewports_survive_recomposition` fails by name, listing each unreachable run
with its rect and the band-sized clip that proves nothing can reveal it —
`"-3.000" at [[1189.9 1820.0]-[1244.0 1840.0]] escapes persistentSideRegion (clip [[960.0 100.0]-[1280.0 744.0]])`.
The guard now catches the class it previously permitted.

**Verified**: `cargo test --all-targets` green (799/0/1 lib + every integration target),
`cargo clippy --all-targets -- -D warnings` exit 0, `cargo fmt --all -- --check` exit 0,
`./target/debug/crest-synth` runs clean.

Also answered here, which F-18 left open: **the PATCH Utility side surface does not overflow
at either viewport** and needed no change.

---

### RISK-2: The render adapter still paints one run

**Type**: BOUNDARY-CONDITION (FR-006 / SC-003)
**Severity**: LOW
**Location**: `src/adapter/eframe_graphical_window.rs:346` — `fn paint_focused_track_meter`

**Analysis**: SC-003 states *"no region is painted by the render adapter."* One paint
survives. It is disclosed, not concealed — `tests/component_composition.rs:68-73` records
it and its reason: the focused track's level *"has a source, a declared painter, and no
route between them."* It resolves every color, style, and extent through the vocabulary,
so `scan_visual_decisions` passes over it correctly. This is a genuine architectural
residue for Phase 5 to close, sized at one function.

---

### RISK-3: FR-009's runtime half covers controls but not compositions

**Type**: TEST-COVERAGE
**Severity**: LOW
**Location**: `tests/component_composition.rs:2058` `check_no_component_owns_or_dispatches_application_state`

**Analysis**: `acceptance-matrix.json` FR-009 hands this to mission review verbatim
("The runtime half currently covers controls but not compositions"). Verified: the
**static** half is complete — the scan reads all visual sources including
`compositions/`, and rejects `SemanticAction`, `static mut`, `OnceLock`, `OnceCell`,
`RefCell`, `thread_local!`, `lazy_static`, `Mutex<`, `RwLock<`, `AtomicUsize`,
`AtomicBool`, `AppState`, `AppLoop`, `AppEvent`, `AudioObservationSnapshot`, and
`screen_rect` outside `density.rs`. Because ownership in Rust can only be smuggled
through one of those constructs, the static scan is close to sufficient and the
residual risk is small. Worth closing when compositions next change.

---

### RISK-4: The visual-decision guard has no construct coverage for `Vec2`, and the "tree is clean" claim overstates

**Type**: TEST-COVERAGE (NFR-004 / FR-006)
**Severity**: LOW
**Location**: guard needles in `tests/component_composition.rs`

**Evidence**: `acceptance-matrix.json` NFR-004 states *"The tree was grepped and is clean
of those forms today."* `grep -rnE "Vec2::new|vec2\(" --include="*.rs" src/ | grep -v
src/shell/visual/` returns **8 hits**, all in `src/testing/component_gallery_scene.rs`
(`:3398, :3423, :3483, :3900, :4177, :4696, :4706, :5421`).

**Analysis**: I inspected each. Seven resolve their arguments from the vocabulary
(`SpacingStep::S8.resolve()`, `SpacingStep::S32.resolve()`, `KEYLINE_EMPHASIS_PX`,
`viewport.width_px`, `smallest.width_px`); the eighth (`:5421`,
`Vec2::new(1600.0, 900.0)`) is inside a test module. **There is no concealed violation**
— the substance of the claim holds. What does not hold is its phrasing: the forms are
present and the guard cannot see them, so the next `Vec2::new(12.0, 8.0)` written here
would ship unflagged. Add `Vec2::new` / `vec2(` to the needle set with a
vocabulary-resolved allowance, the same shape the guard already uses for `CornerRadius`.

---

### RISK-5: Six approvals bypassed the review-artifact gate under waiver W-01

**Type**: PROCESS / REVIEW INDEPENDENCE
**Severity**: LOW (root cause is a tooling defect, and it is fully documented)

**Evidence** from `status.events.jsonl` lane transitions:
- WP03 cycle 2 → approved: *"Override reason: review-cycle-2.md is this reviewer's own…"*
- WP04: three rejection cycles, then `force: true` — *"ARBITER DECISION after 3 review cycles"*
- WP09 cycle 2 → approved: *"ARTIFACT OVERRIDE, W-01 fifth occurrence"*
- WP06 → approved: *"W-01 WAIVER, SIXTH OCCURRENCE — I inspected the blocking file myself before overr…"*
- WP04 `approved → approved` with `force: true`: *"Metadata repair only, no verdict change"*

**Analysis**: the root cause is documented in `tasks/WP04-frame-compositions/review-cycle-4.md`:
Spec Kitty numbers an acknowledgement artifact one ahead of the cycle it acknowledges, and
the copy inherits `verdict: rejected` from the file it copied — so a stale rejected verdict
blocked six legitimate approvals. `review-cycle-4.md` is explicitly *not a review*; it is a
corrected duplicate of cycle 3.

Review quality itself holds up under inspection: WP04's cycle-3 reviewer opened with
*"I approved this work package earlier in this cycle and that approval was premature"* and
withdrew it, and the blocking issue it raised (a whole-frame paint measured against a
chrome-only whitelist) **is fixed in the merged tree** —
`application_shell.rs:2131-2155` now scopes the adjudication to `ContextLine`,
`IdentityHeader`, and `Footer` only. Recorded as a tooling defect to fix upstream, not as
a review-integrity failure.

---

## Silent Failure Candidates

Scanned every file under `src/shell/visual/controls/` and `src/shell/visual/compositions/`
plus the new gallery code for default-value returns on malfunction.

| Location | Condition | Result | Assessment |
|----------|-----------|--------|------------|
| `controls/mod.rs:252` | `ControlSelection::NotAskableInRole` | `None` | Not a silent failure — a typed refusal in an exhaustive match, asserted by `selection_is_total_over_kind_and_role_with_every_control_reachable` |
| `compositions/mod.rs:103` | `ShellComposition::WholeFrame` | `None` | Not a silent failure — `ApplicationShell` legitimately binds no single region |
| `section.rs:1040`, `utility_inspector_panel.rs:390`, `parameter_row.rs:299`, `toggle.rs:267`, `compact_slider.rs:99,103` | non-applicable variants | `None` | All exhaustive-match arms over closed unions; a new variant is a compile error |
| `component_gallery_scene.rs:1432` | `#[cfg(test)]` marker absent in source | scans the **whole** file | Fail-safe, not fail-silent: the fallback is strictly more conservative and would report `true` rather than a false silence |

**No `catch`-and-return-empty pattern was found in any file this mission added.** The
error type `ComponentGalleryError` (`:1481`) is a typed enum over four real failure modes
with `thiserror`, and each carries its cause.

---

## Security Notes

| Area | Finding | Risk class | Assessment |
|------|---------|-----------|------------|
| Subprocess | None introduced | — | `grep` over the diff for `subprocess`/`Command`/`shell` returns nothing in `src/` |
| Network / HTTP | None introduced | — | No `reqwest`, `hyper`, or socket use added |
| Auth / credentials | None introduced | — | Mission touches no auth path |
| Path handling | `std::fs::read_to_string(repository_root().join(...))` | PATH-TRAVERSAL (N/A) | Test targets only, fixed literal paths, no user input |
| Binary contents | `include_str!("component_gallery_scene.rs")` at `:1400` embeds the module's ~5,500-line source into every binary that links it | SOURCE-DISCLOSURE (informational) | Not a vulnerability in a locally-run desktop synth; noted because it is a side effect of DRIFT-1 and disappears when that code is deleted |

No blocking security findings. The mission is a rendering slice with no I/O boundary of
its own.

---

## Final Verdict

**PASS WITH NOTES** — issued FAIL on one blocking finding; that finding was fixed
immediately after, at the operator's direction, and the verdict updated. See
RISK-1 § Resolution.

### Verdict rationale

The mission delivered what it promised, and the delivery is unusually well evidenced:
all fourteen FRs trace to tests that constrain real behavior through the production
render path, the closed unions are exhaustive by construction rather than by assertion,
the guard tests are mutation-verified, the whole suite is green at 799/0/1 with every
integration target passing, and both acceptance layers pass. The one substantive
architectural discovery mid-mission — that the composition family was incomplete —
was handled correctly: the crest-spec was amended first, then WP09 derived from it.
The proof artifacts state their own limitations rather than papering over them, which
is why this review could confirm most of them cheaply.

The verdict is nonetheless FAIL, on **RISK-1** alone. The mission deleted the side
region's vertical scroll and no composition absorbed it, so at the authored desktop
viewport roughly half the mixer Inspector — the return-bus levels, `Return B1` through
`B7`, and master gain — is painted past the bottom of its panel and is reachable by no
gesture. That is content the operator could reach before this mission and cannot reach
after it. It contradicts US2 acceptance scenario 5 directly, and falsifies US2 scenario
4's claim that this was a re-composition rather than a behavior change. It is not covered
by the operator's display-fidelity relaxation, which names seat widths, band-height
derivations, and text-clipping — F-18's author draws that distinction explicitly. F-18's
recorded owner is "mission review", meaning it was deferred here for a decision rather
than accepted; this review is that decision point, and a shipped unreachability
regression should be closed before the branch reaches `main`.

**That blocker is now closed** — scroll region restored and the guard rewritten to
discriminate reachable from unreachable rather than to tolerate a count. RISK-1 §
Resolution records the change, the before/after clip measurements, and the falsification.
With it closed the mission is releasable, and the verdict is PASS WITH NOTES.

One recommendation from the original report was **not** carried out, and deliberately:
extending `check_no_text_clips_or_overlaps` (`tests/component_vocabulary.rs:1662`) to
assert on `PersistentSideRegion`. That helper treats a run escaping a *fixed* band as a
defect and anything else as scrolled-out-of-view. The side region is no longer a fixed
band — it now has a scroll viewport — so adding it there would flag legitimately scrolled
content as unreadable. The equivalent guarantee lives in `classify_escape` instead, which
reads the clip rather than a band list and therefore stays correct as compositions gain or
lose scroll regions.

Everything else below is non-blocking.

### Open items (non-blocking)

1. **DRIFT-1** — delete the source-scan silence derivation from
   `src/testing/component_gallery_scene.rs:1178, 1222, 1281, 1391-1477`, or correct
   `acceptance-matrix.json`'s C-007 note, which currently asserts it did not ship.
2. **DRIFT-2** — amend NFR-003's text to the production-span measure the mission
   actually enforces (`ADAPTER_PRODUCTION_LINE_CEILING = 497`), so `spec.md` stops
   reading as an unmet requirement.
3. **DRIFT-3** — correct the NFR-005 note to name both modified pre-existing test files.
4. **RISK-2** — route `paint_focused_track_meter` out of the adapter in Phase 5; it is
   the last paint outside the visual module.
5. **RISK-4** — add `Vec2::new` / `vec2(` to the visual-decision guard's needle set with
   a vocabulary-resolved allowance.

Lower priority, recorded not assigned: RISK-3 (FR-009's runtime half over compositions),
RISK-5 (report the W-01 acknowledgement-artifact numbering defect upstream to Spec Kitty),
and the ten designed-but-undriven structures F-10 lists, which are Phase 5's declared input.

---

## Retrospective Reminder

The canonical post-merge sequence is: **mission review → author or verify retrospective →
surface findings**.

The retrospective record **exists** and was authored at the runtime terminus:
`kitty-specs/crest-component-controls-and-compositions-01KZ25VX/retrospective.yaml`
(8,964 bytes, commit `853545d`, `RetrospectiveCaptured` event present in
`status.events.jsonl`). No `RetrospectiveCaptureFailed` event is recorded. Nothing to
author.

To surface findings:

```
spec-kitty retrospect summary
spec-kitty agent retrospect synthesize --mission crest-component-controls-and-compositions-01KZ25VX
spec-kitty agent retrospect synthesize --mission crest-component-controls-and-compositions-01KZ25VX --apply
```

(`summary` aggregates read-only; `synthesize` is dry-run by default and `--apply` mutates.)
