# Mission Review Report: shell-hygiene-01KZD0KR

**Reviewer**: Reviewer Renata (post-merge mission review)
**Date**: 2026-08-06
**Mission**: `shell-hygiene-01KZD0KR` — Shell Hygiene Sweep
**Baseline commit**: `32cb838` (the commit before `feat/shell-hygiene` branched)
**HEAD at review**: `2f428eeeb17f06ada7eef6ddc81b9cab4f02dc3a`
**Accepted**: `3bdd299` · **Merged**: `9698173`
**WPs reviewed**: WP01..WP05 (all `done`)

This mission discharged six deferred findings from two prior post-merge reviews —
RISK-3, RISK-4, RISK-5, OBS-1, SMELL-1 residue, DRIFT-3 — under a no-product-behavior-change
constraint. This review is the first independent read of the merged result.

**Process signal**: the event log records 5 WPs through
`genesis → planned → claimed → in_progress → for_review → in_review → approved → done`
with **zero rejection cycles, zero forced transitions, and no `ReviewerSelfApproval`**.
Zero rejections across five WPs would ordinarily invite suspicion of lenient review;
here each WP carries a 213–417 line review-cycle record and the reviewers surfaced six
non-blocking findings they chose to record rather than absorb, which is the behavior of
real review reaching a clean verdict rather than review that did not happen.

---

## Gate Results

### Gate 1 — Contract tests
- **Result: N/A.** This repository has no `tests/contract/` suite. Per the operator's
  scoping, the declared crest-spec validations are this project's acceptance layer.

### Gate 2 — Architectural tests
- **Result: N/A.** No `tests/architectural/` suite exists here. The equivalent
  boundary enforcement is executable and lives in the declared validations
  (`component_composition`'s passivity and purity scans, the RT-boundary invariants).

### Gate 3 — Cross-repo E2E
- **Result: N/A.** Single-repository project; no `spec-kitty-end-to-end-testing`
  companion repo exists.

### Gate 4 — Issue matrix
- **Result: N/A.** `spec.md` references prior *mission-review findings*
  (RISK-3/4/5, OBS-1, SMELL-1, DRIFT-3), not GitHub issues, so `finalize-tasks`
  scaffolds no `issue-matrix.md` and none is required. Traceability is carried by the
  FR table and the acceptance matrix instead, and is verified in the FR matrix below.

### Gate 5 — Declared crest-spec validations (this project's acceptance layer)

| Command | Result | Evidence |
|---|---|---|
| `spec-kitty crest-spec doctor` | **PASS** | OK — 7 contexts / 132 resources, 107 requirements, 32 project validations, 19 witnesses |
| `cargo test --lib` | **PASS** | 638 passed, 0 failed, 1 ignored |
| `cargo test --test webview_projection_shell` (headless) | **PASS** | `CREST_ACCEPTANCE webview_projection_shell passed`; six live sections honestly skip-narrated |
| `cargo test --test component_composition` | **PASS** | `CREST_ACCEPTANCE component_composition passed`; 15 passed; 136 sources / 84,872 lines scanned |
| `cargo test --test component_vocabulary` | **PASS** | `CREST_ACCEPTANCE component_vocabulary passed`; 11 passed |
| `cargo clippy --all-targets` | **PASS** | clean, no warnings |
| `cargo fmt --all -- --check` | **PASS** | clean |

### Gate 6 — Full gated live run — **BLOCKED (environmental)**

- Command: `CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell -- --nocapture`
- Result: **did not reach `skipped: none`.** Two bounded attempts both failed at
  `live section FAILED: no attached display seats the authored 1920x1080 viewport`.
- **Cause is hardware, not code.** The harness requires a display whose *logical* size
  is ≥ 1920x1080 (`window.rs` seats the window deliberately, because macOS silently
  clamps a window to a smaller screen and would otherwise measure a narrower viewport).
  The rig's external `LS28AG700N` (3840x2160, "UI Looks like 1920x1080") was asleep on
  the first attempt and disconnected by the second; the remaining built-in Retina panel
  is 3024x1964 physical ≈ 1512x982 logical and cannot seat the authored viewport. I
  woke the displays with `caffeinate` and re-ran; the external panel did not return.
- **The suite's behavior here is correct and is itself a proof of NFR-002**: it treats a
  missing display as a live-section *failure*, never a skip.
- **Mitigation — the two sections this review most needed were obtained by other means:**
  - **T014 ran and passed**: it is headless, executes before the live gate, and reported
    PASS on the merged tree in both attempts.
  - **T013 was replicated directly against the shipped binary** by the reviewer, and
    both its positive and its falsifying arm were executed. See Item 2 below.
- **Not recorded as an operator exception** (`mission-exception.md` is a Gate 3 device
  and Gate 3 is N/A here). It is recorded as an open verification gap: the mission's
  own SC-005 claim of a `skipped: none` live run **stands on the mission's evidence, not
  on this review's**. A reviewer with the external panel attached should re-run it.

---

## FR Coverage Matrix

| FR | Description (brief) | WP | Test / evidence | Adequacy | Finding |
|----|---|---|---|---|---|
| FR-001 | Double close-failure still surfaces the typed error | WP01 | T013 (live) + `close_with_retry` in-module scenarios | **ADEQUATE** — reviewer-verified falsifiable: removing `handle.exit` reproduces the hang | — |
| FR-002 | Close path otherwise unchanged (retry, typed error, first-error latch) | WP01 | `window.rs` in-module tests, 13/13 | ADEQUATE | — |
| FR-003 | Superseded-late acks identity-validated | WP02 | T014 (headless) — 5 of 6 identity fields corrupted/omitted, each asserting the exact field name | **ADEQUATE** — reviewer-verified falsifiable: bypassing the check fails T014 by name | — |
| FR-004 | Control-intent declarations retired *before* deletion | WP03 | crest-spec `7c7f1cf` (2026-08-06 19:43) precedes every `src/`,`tests/` commit in range (earliest 21:50) | ADEQUATE — C-002 satisfied by construction | — |
| FR-005 | Dead code removed, no dangling reference; `CURSOR_GLYPH` claim resolved | WP03 | `rg` over `src/`+`tests/`: zero hits for all four retired symbols | **PARTIAL** — symbols gone, but three doc/code residues survive | **DRIFT-1** |
| FR-006 | Gallery serving path narrated as a deliberate exemption | WP05 | 39-line narration in `component_gallery_scene.rs`, +39/−0 | ADEQUATE | — |
| FR-007 | Purity scan binds every page source; planted violation fails by name | WP05 | Reviewer planted violations in **both** newly covered sources | **ADEQUATE** — see Item 6 evidence below | — |
| FR-008 | Documentation residue discharged | WP05 | T020 amendments to two completed missions | **PARTIAL** — one amendment is selectively windowed; one obligation left stale | **RISK-1**, **DRIFT-3** |

| NFR / C | Result |
|---|---|
| NFR-001 no product behavior change | **PASS on available evidence** — all headless suites byte-identical in outcome; live latency claim not re-verifiable here (Gate 6) |
| NFR-002 no proof weakened | **PASS** — verified independently: the one narrowed test is still falsifiable (Item 1), and no threshold, baseline, or skip entry was loosened |
| ~~NFR-003~~ net code reduction | **WITHDRAWN by operator** — record verified honest (Item 4) |
| C-001 blast radius | **PASS** — no reducer, RT, or projection-schema file appears in the diff |
| C-002 declaration before deletion | **PASS** — verified by commit timestamps |
| C-003 gallery retained | **PASS** — verified exhaustively (Item 3) |
| C-004 scope boundary | **PASS** — no feature added, no control-intent re-declared |

**Legend**: ADEQUATE = the test constrains the required behavior and was demonstrated to
fail when the implementation is disabled; PARTIAL = requirement substantially met with a
documented residue.

---

## The Six Targeted Verifications

### Item 1 — Did any deletion take a live guarantee with it? **PASS**

WP03 deleted four items, retired 2 tests and narrowed 3. The highest-risk narrowing was
`a_frame_recorded_from_another_thread_wakes_the_await` → `a_frame_recorded_through_one_clone_is_visible_through_another`,
which incidentally proved that `QualifyingFrameStream` clones share one
`Arc<StreamShared>` — a live doc claim on the **surviving** type.

I did not take the relocation on trust. I replaced the derived `Clone` with a
deep-copying hand-written impl on the merged tree and ran the suite:

```
shell::webview::frame_stream::tests::a_frame_recorded_through_one_clone_is_visible_through_another ... FAILED
  panicked at src/shell/webview/frame_stream.rs:301: the clone's recording is visible to the original handle
shell::webview::window::tests::clones_of_one_window_share_one_qualifying_frame_stream ... FAILED
  panicked at src/shell/webview/window.rs:758: assertion failed: control_side.poll(&expectation).is_some()
```

**The guarantee is enforced, and doubly so.** Both the relocated proof and an
independent window-level proof die under a deep-copying `Clone`. Tree restored; no
deletion took a live guarantee with it.

One defect in the *record*, not the code: `semantic-acceptance.md` cites the redundant
proof as "`window.rs:596`". Line 596 is a **comment**, not a proof — the actual
independent proof is the test ending at `window.rs:758`. See **DRIFT-2**.

### Item 2 — Are the two new error-path proofs wired and falsifiable on main? **PASS (with one note)**

**T013 (sole regression guard on RISK-3).** The full gated run could not execute here
(Gate 6), so I replicated T013's mechanism directly against the shipped debug binary.

| Run | Outcome |
|---|---|
| Seam armed (`CREST_WEBVIEW_FORCE_CLOSE_FAILURE=1`), forced-throw page | `EXIT=1` in **1.5 s**, stderr carries the typed `PageRenderFailed` with the failing document's identity (`generation: 20`, `stateHash`, `name: TypeError`) |
| **Exit edge deleted** (`handle.exit(REQUESTED_EXIT_CODE)` removed), seam armed | **HUNG** — bounded timeout fired at **90.1 s**. RISK-3 reproduced exactly as described |
| Unit suite with the exit edge still deleted | `cargo test --lib` → **638 passed, 0 failed**; `window.rs` in-module → **13 passed** |

The mission's claim is confirmed precisely: **the unit suite stays green 13/13 with the
exit edge deleted, so T013 is the sole regression guard on RISK-3**, and T013's
mechanism is genuinely load-bearing.

*Note (**RISK-2**)*: T013's **run 1** is vacuous with respect to the seam. I ran the same
page variant with the seam **disarmed** and got an identical `EXIT=1` and an identical
`PageRenderFailed` payload — every run-1 assertion passes with the mechanism off. The
*section* remains falsifiable because run 2 asserts the verbatim forced cause and a
nonzero exit that a clean close cannot produce. Not blocking; worth closing.

**T014 (only acceptance-layer coverage of `forward_ack`).** Ran and passed headless on
the merged tree. I then bypassed WP02's mechanism at the superseded-late branch:

```
thread 'main' panicked at tests/webview_projection_shell.rs:1347:
T014 evicted/stateHash: a superseded-late ack whose stateHash is not a verbatim copy
must be typed-rejected as PaintedAckError::IdentityMismatch; got Ok(SupersededLate { generation: 3 })
```

Dies immediately and **by name**. Genuinely falsifiable. Tree restored.

### Item 3 — C-003 gallery retention **PASS**

The mission deleted 256 lines elsewhere; nothing gallery-shaped went with them.

| Artifact | State on main |
|---|---|
| `src/testing/component_gallery_scene.rs` | present; diff is **+39 / −0** (narration only) |
| `webview-page/gallery.js`, `webview-page/gallery.css` | present, untouched |
| CLI option `--demo-live-component-library` | present (`src/bin/crest_synth.rs:325`), with its guard test that it is never a live-demo alias |
| Make target `demo-live-component-library` | present (`Makefile:67`) |

**The scene still runs.** I launched `./target/release/crest-synth --demo-live-component-library`
bounded at 25 s; it started cleanly and was still alive at the bound — which is the
correct behavior for a hand-browsable scene that waits for the operator. Its 15 pages
and vocabulary are additionally proven green by `component_vocabulary` (`pages=15`) and
`component_composition` (which scans `gallery.js`). C-003 fully satisfied.

### Item 4 — The withdrawn NFR-003 **PASS — the record is honest**

The bar is whether a reader can reconstruct that the requirement existed, was measured
unfavourably, and was **withdrawn rather than reinterpreted**. They can, from three
independent places:

- `spec.md` keeps the row with a strikethrough, `Status: Withdrawn`, and a prose note
  carrying the measurement (−256 lines removed, ~+1,300 net), the date, and the operator's
  verbatim words.
- `acceptance-matrix.json` → `withdrawn_requirements[0]` carries `requirement_id`,
  `title`, `withdrawn_by`, `withdrawn_at`, `measured_outcome`, and a rationale ending
  *"Recorded as WITHDRAWN, not regraded — the measurement stands as taken and no result
  was reinterpreted to pass."*
- `semantic-acceptance.md` goes further and names the two moves that were available and
  refused: the row was **not** flipped `fail`→`pass`, and the requirement was **not**
  rescoped to production code only — where, it discloses, it *would have passed* at −256.

Disclosing the reinterpretation that would have made it pass, and then declining it, is
above the bar. **This does not read as concealment.** It reads as a requirement that was
badly authored, honestly measured, and retired by a named decision-maker.

### Item 5 — The T020 documentation edits **PASS WITH NOTES — one edit reads better than its evidence**

Three classes of amendment to two completed missions. Two are clean; one is not.

**Clean — status rows `Open` → `Accepted`.** Defensible: the mission was accepted with
25/25 deterministic acceptance, and the `Status:` header amendment *volunteers the bad
news* rather than burying it: "The post-merge mission review returned **FAIL** on RISK-1
and RISK-2 ... those four findings were the entire scope of the follow-on mission." A
record that states its own FAIL verdict in the same line that marks it complete is not
being dressed up.

**Clean — terminology "migration"/"migrated" → "re-hosting"/"re-hosted".** Neutral;
changes no claim's truth value.

**Not clean — the NFR-002 post-hoc quantified leak bound (**RISK-1**).** The amendment
declares the bound as *"no monotonic growth across sampling windows"* and reports:
*"The recorded run declined 107728 → 103904 KiB to a plateau."*

I read the cited evidence file. All figures it quotes are genuinely present — but the
reported window is **selected without disclosure**:

```
07:13:09 rss_kb=507840     <- sample 1
07:13:19 rss_kb=607392     <- sample 2 (peak; +99,552 KiB growth)
...      (607k plateau for ~60 s)
07:15:09 rss_kb=107728     <- sample 13: where the amendment starts reporting
...
07:18:10 rss_kb=103904     <- sample 31: where the amendment stops reporting
07:18:20 rss_kb=93568      <- sample 32 (actual run)
07:18:30 rss_kb=94160      <- sample 33 (actual end)
```

The quoted range begins at sample 13 of 33 and ends two samples before the run does. A
reader of the amended NFR-002 would conclude the process sat near 103–107 MiB throughout;
it actually spent its first ~70 seconds at 500–607 MiB, roughly **6× the reported
figure**, and the only monotonic growth in the series — the very thing the declared bound
forbids — occurs in the omitted prefix.

The underlying engineering conclusion is defensible: the prefix is startup/warm-up
allocation and the post-warm-up trend is flat-to-declining, which is what a leak metric
should measure. **The defect is disclosure, not fabrication.** The amendment does not say
it is reporting a post-warm-up window, does not state where that window starts, and does
not mention the peak. It is also self-labelled *"quantified after acceptance ... it was
not a pre-declared bar at the accept gate"*, which is a genuinely honest disclosure and
the reason this is MEDIUM rather than higher.

Verdict on the bar as stated: *history may be amended honestly but never made to read
better than its evidence.* This one edit reads better than its evidence. It should be
amended again to state the warm-up exclusion and the peak explicitly.

### Item 6 — The six recorded follow-ups: **none is a defect that should have blocked release**

| # | Follow-up | Verdict |
|---|---|---|
| 1 | WP02 OBS-A — `retire()`'s dedup guard is load-bearing but unproven | **Correctly recorded; not a blocker.** Independently verified: the guard is genuinely load-bearing (without it a *faithful* ack draws a false `IdentityMismatch`), it is genuinely untested, and the reviewer left a ready-to-paste proving test. A missing test on a correct guard — a gap, not a defect. Worth noting an adversarial hunter proposed *deleting* it; that would have been wrong, and the mission's own record has it right. |
| 2 | WP01 F4 — `std::env::set_var` races `var_os` in one test binary | **Real, correctly characterized as latent.** Confirmed: the base file had **zero** env mutations, the mission introduced four, and `var_os` is read by production paths other tests exercise. Edition is 2021 so the compiler does not force `unsafe`; `setenv`/`getenv` are not thread-safe against each other at libc level regardless of key. Test-only, has not flaked. Not a blocker. |
| 3 | WP04 — `PAINTED_ACK_IDENTITY_FIELDS` doc overstatement | **Accurate as recorded, and mildly understated.** The doc sentence claiming one list is *literally false* — the WP01 section it names still carries its own inline literal, so there are three lists, not two. But the recorded conclusion ("no proof is weakened — the comment is false, not the code") is **correct** and independently confirmed: T014 corrupts or omits 5 of the 6 fields and asserts each by name, so narrowing production fails loudly and widening breaks the well-formed controls. Not a blocker. |
| 4 | WP03 — two unconsumed residues (`Condvar`, `Display for FrameExpectation`) | **Real, and the strongest finding the adversarial pipeline produced** (skeptic-CONFIRMED 9/10 and 7/10, 3-hunter agreement on the first). Inert at runtime, so not a blocker — but they are exactly what FR-005 promised to remove, so this review escalates them from "recorded residue" to a drift finding. See **DRIFT-1**. |
| 5 | Pre-existing soak flake (`receive_phase("meters")`, ~1 run in 5) | **Correctly attributed.** Established as pre-existing on mechanism (a section WP04 never touches) and on measurement (WP04's max gap is *lower* than base). Not this mission's defect; not a blocker. |
| 6 | Inherited red gate — `validation.format` failing since the prior mission | **Correctly handled.** Fixed on the consolidated tree at `cf75033`; no lane could have fixed it without conflicting. `cargo fmt --check` is clean on main. Closed, not deferred. |

**FR-007 evidence** (gathered while assessing this item): I planted a purity violation in
each newly covered source and each failed **by name**, as US5 scenario 2 requires:

```
gallery.js owns `Date.now`, which a pure render cannot
index.html owns `localStorage`, which a pure render cannot
```

---

## Drift Findings

### DRIFT-1: FR-005's "no dangling reference" is incompletely discharged

**Type**: PUNTED-FR (partial) · **Severity**: **MEDIUM** · **Spec reference**: FR-005, US3 scenario 1

The four retired *symbols* are gone — verified by `rg` over `src/` and `tests/`, zero
hits. Three references describing them survive:

1. `src/shell/webview/frame_stream.rs:117-121,138,152-154` — `StreamShared.arrived`, a
   `Condvar` whose only waiter (`await_qualifying`) this mission deleted. The notifier,
   the lock-ordering `drop`, and the import remain, and `record`'s doc still promises it
   *"wakes every waiting consumer."* Skeptic-CONFIRMED 9/10, 3-hunter agreement.
2. `src/shell/webview/frame_stream.rs:107-115` — `impl Display for FrameExpectation`,
   whose sole consumer was the deleted `FrameAwaitError`. Skeptic-CONFIRMED 7/10.
3. `src/shell/webview/window.rs:596-598` — **not recorded anywhere by the mission**:
   ```rust
   // qualifying-frame stream lets scenes
   // block on "generation N painted"
   // without sleeping (T006).
   ```
   This describes the blocking `await_qualifying` that WP03 deleted. WP03's T008
   corrected the `frame_stream.rs` module doc and the `mod.rs` reference but missed this
   one, inside a file another WP owned. A near-identical residue sits at
   `tests/graphical_application_shell.rs:624` ("the seam live scenes block on instead of
   sleeping"), outside the mission's diff entirely.

Items 1 and 2 were recorded as follow-ups (semantic-acceptance §Deletions); item 3 was
not. All are inert at runtime, which is why this is MEDIUM rather than HIGH — but a
`Condvar` advertising a wakeup contract it cannot honor is precisely the "declared and
built mean the same thing again" property US3 was written to restore.

### DRIFT-2: The semantic-acceptance layer cites a comment as a proof

**Type**: RECORD-ACCURACY · **Severity**: **LOW** · **Reference**: `semantic-acceptance.md` line 53

The claim that the relocated clone-sharing proof "carries independent redundant proof at
`window.rs:596`" points at a comment line — and, per DRIFT-1, a *stale* one. The
substance of the claim is nonetheless **true**: an independent proof does exist, as
`clones_of_one_window_share_one_qualifying_frame_stream` (ending `window.rs:758`), and I
confirmed it dies under a deep-copying `Clone`. A wrong line number on a true claim, in
the one document whose job is to be checkable.

### DRIFT-3: `tasks.md` still carries NFR-003 as a live obligation

**Type**: RECORD-RESIDUE · **Severity**: **LOW** · **Reference**: `tasks.md:242`

`spec.md` and `acceptance-matrix.json` both record NFR-003 as withdrawn, but
`tasks.md`'s "Cross-cutting obligations" still reads
*"**NFR-003** — net code reduction across `src/` and `tests/`"* with no withdrawal
marker. A reader of `tasks.md` alone would believe it binding. Ironic given FR-008 is
the "no unfinished status field" requirement.

---

## Risk Findings

### RISK-1: An amended NFR reports a selected sub-window as "the recorded run"

**Type**: RECORD-HONESTY · **Severity**: **MEDIUM**
**Location**: `kitty-specs/webview-shell-cutover-01KZAC7Q/spec.md`, NFR-002 row (added by T020)
**Trigger**: any reader auditing the leak bound against its evidence file

Full analysis in Item 5. The declared bound is "no monotonic growth across sampling
windows"; the sole monotonic growth in the evidence (507,840 → 607,392 KiB) sits in the
undisclosed omitted prefix, and the reported window starts at sample 13 of 33. The
conclusion is defensible; the presentation is not. **Recommended remediation**: amend to
state the warm-up exclusion, the sample index where measurement begins, and the observed
peak. This is the one finding that most directly answers the question this review was
asked to put to the T020 edits.

### RISK-2: T013's run 1 passes with its own mechanism disarmed

**Type**: PROOF-INTEGRITY · **Severity**: **LOW-MEDIUM**
**Location**: `tests/webview_projection_shell.rs:4077-4137`
**Trigger**: any rename of `CLOSE_FAILURE_OVERRIDE_ENV`, or any future edit that disarms the seam

Executably confirmed (Item 2): with the seam disarmed the run produces the same exit code
and the same typed payload, so all of run 1's assertions hold. Run 1 then prints
`"PASS ... with every close forced to fail"` — a claim it never checked. The section as a
whole survives on run 2, so this cannot produce a silently green T013; it produces a
misleading PASS line seconds before the section dies. The env-var name exists as three
hand-written copies (`window.rs:97`, `window.rs:1107`, `tests:3956`) because the
production constant is private. Notably this same file argues the opposite doctrine for
`PAGE_CSP` and `protocol_response`, which were deliberately exported so *"the harness
[cannot] re-implement this privately."* **Recommended**: export the seam name and add one
arm-detection assertion to run 1.

### RISK-3: Two adjacent guard loops disagree about their input, producing a false-failure trip hazard

**Type**: BOUNDARY-CONDITION · **Severity**: **LOW** (fails closed)
**Location**: `tests/component_composition.rs:1823-1858`

WP05 built `strip_html_comments` and applied it to the purity loop it was fixing
(`index_html_markup`), but the structurally identical key-handler loop 24 lines below
still reads raw `index_html`. The stripper exists precisely because *"a document whose
header narrates the rule ... would read as owning what it forbids"* — and that rationale
applies verbatim to the key-handler needles. I demonstrated it:

```
$ # add to index.html: <!-- This page registers no keydown handler; keys are captured Rust-side. -->
index.html registers a key handler (`keydown`); keys are captured Rust-side
```

A comment *documenting* the input rule breaks the suite. It fails closed (a false
failure, never a false pass), so it is a maintenance hazard rather than a proof hole.

### RISK-4: No declared validation ever builds the release-only guard tests

**Type**: DEAD-PROOF · **Severity**: **LOW**
**Location**: `src/shell/webview/window.rs:1097-1110` and every `#[cfg(not(debug_assertions))]` test

`make test` and the configured `test_command` both run debug profiles, so every
release-only test in this crate is compiled out of every run. The
`release_builds_compile_the_forced_close_failure_seam_out` guard therefore never
executes, and it hardcodes the env-var literal rather than referencing the constant, so
a rename would silently void it.

**This one is well mitigated and I verified the mitigation.** `semantic-acceptance.md`
§9 rests the security claim on `strings` over the release binary rather than on the
test. I built `--release` and confirmed it:

```
CREST_WEBVIEW_FORCE_CLOSE_FAILURE : 0 occurrences
CREST_WEBVIEW_PAGE                : 0 occurrences
```

Both debug seams genuinely compile out of the shipped binary. The security property
holds; only its *test* is inert.

### RISK-5: Test-only environment mutation is unsound under parallel test execution

**Type**: ERROR-PATH · **Severity**: **LOW**
**Location**: `src/shell/webview/window.rs:1087,1093,1107,1109`

Confirmed as recorded (Item 6 #2). The base file had zero env mutations by deliberate
choice; this mission introduced four in a 638-test single-process binary where other
tests read `var_os` concurrently. Latent, has not flaked, test-only.

---

## Silent Failure Candidates

| Location | Condition | Result | Assessment |
|---|---|---|---|
| `projection_channel.rs:484-498` | Late ack whose generation is outside the retained window | `Ok(SupersededLate)` — no error | **Correct and deliberate.** Narrated at length: the channel holds nothing to compare against, so tightening this into a rejection would fail honest acks delayed past the window. Not a silent fallback — it constructs no observation either way. |
| `window.rs:431-434` | Window already gone when close is requested | early `return` | **Correct** — nothing left to close; predates this mission. |
| `frame_stream.rs` `poll` | No qualifying observation retained | `None` | **Correct** — the caller re-asks next tick; this is the documented contract. |

No `except`-and-swallow, no empty-string-on-error, no `unwrap_or_default` masking a
failure was found in the mission diff. The mission's substance is the *opposite* of
silent fallback: both RISK-3 and RISK-4 were unreported conditions that now terminate or
type-reject.

---

## Security Notes

| Finding | Location | Risk class | Assessment |
|---|---|---|---|
| Debug-only forced-close seam added to production code | `window.rs:85-97,365-373` | DEBUG-SEAM-IN-PROD | **Mitigated and verified.** `cfg(debug_assertions)`-gated exactly as the `CREST_WEBVIEW_PAGE` precedent; `strings` on a release build confirms **0** occurrences of either seam name. T013 additionally panics rather than passing quietly if ever run in release. |
| Gallery serves without the production CSP | `component_gallery_scene.rs:3083-3121` | POLICY-EXEMPTION | **Accepted, narrated, verified.** The narration names `page_asset` as the structural reason the shipped window cannot reach gallery sources; independently confirmed — `page_asset` has no gallery entry and returns 404 with no fallback. `T010` proves the production seam still serves the real CSP from a single source. |
| No CSP change anywhere in the mission | — | — | Confirmed: `PAGE_CSP` untouched; `T010` passes. |
| No new subprocess/path/network/credential surface | — | — | The only new subprocess spawns are test-side, invoking `CARGO_BIN_EXE_crest-synth` with list arguments and no shell. |

Nothing here is release-gating.

---

## Structural Quality (adversarial-review)

**Pipeline report**: `kitty-specs/shell-hygiene-01KZD0KR/adversarial-review.md`
**Files reviewed**: 9 · **Raised**: 23 · **Merged to verification**: 13 · **Survived skeptic**: 3 · **Survived at ≥7/10**: 2 · **Minor (not itemized)**: 1

Six hunters ran blind and in parallel in a single message; all 13 merged findings went to
`finding-skeptic` under its kill mandate. The skeptic downgraded or refuted 10 of 13,
several times by disproving a harm narrative whose structural observation was correct —
which is the pipeline working as designed.

Both reportable survivors are **Dead Code**, which per this skill's scope discipline is
already Step 7's `DEAD-CODE` class. They are therefore **not** duplicated as `SMELL-N`
entries; they are folded into **DRIFT-1** above, where they carry the skeptic's
confirmation as corroborating evidence:

- Orphaned `Condvar` — CONFIRMED 9/10, 3-hunter agreement
- Dead `impl Display for FrameExpectation` — CONFIRMED 7/10

No structural finding demonstrates a behavior defect, so none feeds the FAIL threshold.
Two skeptic verdicts rested on prediction rather than observation; I executed both, which
confirmed RISK-2 and upgraded RISK-3 from the skeptic's "dormant" to demonstrated.

---

## Final Verdict

## **PASS WITH NOTES**

### Verdict rationale

All eight FRs are delivered and all four constraints hold. The two requirements that
carried real risk — FR-001 and FR-003, the mission's whole reason for existing — are not
merely tested but **provably falsifiable**, which I established by disabling each
mechanism on the merged tree and observing the proof die: removing WP01's exit edge
reproduces the RISK-3 hang at 90 s while the entire unit suite stays green (confirming
T013 is the sole guard), and bypassing WP02's identity check fails T014 by name. The
deletion that most risked taking a guarantee with it did not: the clone-sharing property
survives with two independent proofs, both of which die under a deep-copying `Clone`.
C-003 is fully honored — every gallery artifact is intact, the scene diff is +39/−0, and
the scene runs. The NFR-003 withdrawal is honestly recorded, and notably discloses the
reinterpretation that would have made it pass and declines it.

No CRITICAL or HIGH finding exists, so the verdict is not FAIL. The notes are: one
amended record (**RISK-1**, MEDIUM) presents a selected sub-window of soak evidence as
"the recorded run" without disclosing the exclusion or the 6×-higher peak — the single
place in this mission where the record reads better than its evidence; FR-005's
"no dangling reference" is incompletely discharged (**DRIFT-1**, MEDIUM), with one
residue the mission never recorded; and the full gated live run could not be reproduced
in this environment for lack of an attached display that seats the authored viewport, so
SC-005's `skipped: none` claim rests on the mission's evidence rather than this review's.

### Open items (non-blocking)

1. **RISK-1 (MEDIUM)** — re-amend `webview-shell-cutover` NFR-002 to state the warm-up
   exclusion, the sample index where measurement begins, and the 607,392 KiB peak.
2. **DRIFT-1 (MEDIUM)** — remove the orphaned `Condvar` and `Display for FrameExpectation`;
   correct the stale comments at `window.rs:596-598` and
   `tests/graphical_application_shell.rs:624`.
3. **RISK-2 (LOW-MEDIUM)** — export the close-failure seam name and give T013's run 1 an
   arm-detection assertion.
4. **RISK-3 (LOW)** — feed both `component_composition` scan loops the same
   comment-stripped sources.
5. **RISK-4 / RISK-5 (LOW)** — decide whether release-only guard tests should run in any
   declared validation; replace test env mutation with a non-global seam.
6. **DRIFT-2, DRIFT-3 (LOW)** — fix the `window.rs:596` citation in
   `semantic-acceptance.md`; mark NFR-003 withdrawn in `tasks.md:242`.
7. **Verification gap** — re-run
   `CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell -- --nocapture`
   on the rig with the external display attached and confirm `skipped: none`.
8. **Carried forward as already recorded** — WP02 OBS-A's proving test (written, ready to
   paste) and the pre-existing `receive_phase("meters")` soak flake.

---

## Retrospective Reminder

The canonical post-merge sequence is: **mission review → author or verify retrospective
(`retrospect create`) → surface findings (`summary` aggregates; `synthesize` reviews
proposals)**.

The retrospective for this mission **exists** at
`kitty-specs/shell-hygiene-01KZD0KR/retrospective.yaml` (authored at `bc1763e`). Note
that the default 3.2.0 location `.kittify/missions/01KZD0KR4G2BZG3GHVA1Y2SZPT/retrospective.yaml`
does **not** exist — the record lives in the mission's `kitty-specs/` tree instead. No
`RetrospectiveCaptureFailed` event appears in `status.events.jsonl`, so this is a
location convention rather than a capture failure; no escalation is warranted.

Next steps for the operator:

- `spec-kitty retrospect summary` — cross-mission aggregation (read-only)
- `spec-kitty agent retrospect synthesize --mission shell-hygiene-01KZD0KR` — inspect
  proposals (dry-run by default)
- `spec-kitty agent retrospect synthesize --mission shell-hygiene-01KZD0KR --apply` —
  apply proposals (mutates)

One process observation worth carrying into the retrospective: this mission's reviewers
recorded six findings rather than absorbing them, and five of the six proved accurate
under independent adversarial verification. That is the behavior that made this review
cheap. The one that was mildly understated (#3) was understated in the safe direction.
