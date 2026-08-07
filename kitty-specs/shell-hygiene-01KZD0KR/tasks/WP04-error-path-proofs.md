---
work_package_id: WP04
title: Error-path proofs under the production path
dependencies:
- WP01
- WP02
requirement_refs:
- FR-001
- FR-003
- NFR-001
- NFR-002
planning_base_branch: feat/shell-hygiene
merge_target_branch: feat/shell-hygiene
branch_strategy: Planning artifacts for this mission were generated on feat/shell-hygiene. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/shell-hygiene unless the human explicitly redirects the landing branch.
subtasks:
- T013
- T014
- T015
- T016
history:
- '2026-08-06: authored from plan IC-01/IC-02 proof halves, research D1/D2, crest-spec asset WebviewProjectionShellAcceptanceTests and validation.webview_projection_shell'
agent_profile: implementer-ivan
authoritative_surface: tests/
create_intent: []
execution_mode: code_change
owned_files:
- tests/webview_projection_shell.rs
role: implementer
tags: []
tracker_refs: []
---

## ⚡ Do This First: Load Agent Profile

Before reading anything else in this prompt, load your assigned profile:

```
/ad-hoc-profile-load implementer-ivan
```

Adopt its identity, boundaries, and governance scope for the duration of this
work package. Do not begin reading source or planning edits until the profile
is loaded.

## Objective

Prove WP01's and WP02's fixes **falsifiably, under the production path**, in
`tests/webview_projection_shell.rs`:

- **FR-001** — with a page render failure already recorded and *both* window
  closes forced to fail, the process **ends** carrying the recorded typed error.
  It does not hang, and the error is not swallowed.
- **FR-003** — a superseded-late ack carrying a corrupted identity is
  typed-rejected; a well-formed one is consumed exactly as before; and a healthy
  live run records **zero** ack rejections.

The crest-spec deepened `validation.webview_projection_shell` precisely to name
these two proofs (see `plan.md` Crest-Spec Derivation). This WP is where that
declaration becomes executable.

**Authorities** (cited, not restated — read them):

- `spec.md` FR-001, FR-003, NFR-001, NFR-002; US1 acceptance scenarios 1-2, US2
  acceptance scenarios 1-3; SC-001, SC-002, SC-005.
- `plan.md` IC-01 and IC-02 (the proof halves).
- `research.md` **D1** (what the RISK-3 hole is and what closes it) and **D2**
  (the retired-identity store and the identical comparison).
- `quickstart.md`, section **"Falsifying the two new proofs"** — it names
  exactly which probe each section must survive. That section is your
  acceptance bar for T016.
- Crest-spec asset `WebviewProjectionShellAcceptanceTests`,
  `validation.webview_projection_shell`. Read via `spec-kitty crest-spec context`.

**Hard boundaries**:

- Do **not** edit `src/`. If a proof needs a seam that does not exist, that is a
  WP01 or WP02 gap — report it rather than adding production code here. WP01
  T003 was specifically asked to leave you a `cfg(debug_assertions)` forced-
  close-failure seam.
- Do **not** loosen any frozen baseline, threshold, skip-list entry, or
  assertion (NFR-002). This suite carries the 50 ms p95 latency bar, the
  determinism proofs, and the screenshot fidelity proofs. They stay.
- Do **not** widen an existing honest-skip entry. New live sections get **new**
  entries.

## Context: what exists today

`tests/webview_projection_shell.rs` is a `harness = false` binary (~3400 lines).
Read its module header (l.35-80) before anything else; it states the suite's own
rules, including that a failure inside a running live section is a failure and
never a skip.

**The gate and the skip list** (`fn main`, l.150-207):

```rust
let live = std::env::var("CREST_WEBVIEW_TESTS").as_deref() == Ok("1");
...
let skips: Vec<&str> = if live {
    Vec::new()
} else {
    vec![
        "T024 page render determinism (...)",
        "T024/WP01 paint-acknowledgment identity (...)",
        "T011 painted-geometry fidelity (...)",
        "T012 forced render failure (...)",
        "T026 live layer (...)",
    ]
};
if live { run_live_sections(&fidelity); } else { /* print each skip */ }
...
if skips.is_empty() {
    println!("CREST_ACCEPTANCE webview_projection_shell passed (skipped: none)");
}
```

Sections that need no window run unconditionally, **above** the gate:
`prove_serialized_schema_fidelity`, `prove_token_table_freshness`,
`prove_protocol_policy_parity`, `prove_typed_startup_failure`.

**The live section runner** is `run_live_sections` at l.1220. Note l.1233-1237:
one protocol registration for every live section, routed through the production
seam — "No section gets a laxer serving path." Honour that.

**The negative-control pattern to copy** is the render-error counter at
l.2958-2983: it is deliberately asserted **last** ("Deliberately last: every
healthy section above must have produced zero…", l.2451), counting
`crest://render-error` events across every healthy live section. T015 is the
same shape for ack rejections.

**The forced-failure subprocess pattern** already exists: the "forced first-render
throw subprocess section" referenced at l.3186. Read it — running the shell in a
subprocess and asserting on its exit is very likely how T013 should assert
"the process ends, not hangs".

**Your dependencies' seams**:

- WP01 leaves a `#[cfg(debug_assertions)]` forced-close-failure seam in
  `src/shell/webview/window.rs`, mirroring `CREST_WEBVIEW_PAGE`. Read WP01's
  delivered code and its report before designing T013.
- WP02 leaves `ProjectionChannel` validating superseded-late acks against a
  bounded retired-identity store, with the same
  `PaintedAckError::IdentityMismatch` error class. Read WP02's delivered code
  and its report before designing T014.

## Subtasks

### T013 — Forced double-close-failure section

**Purpose**: FR-001 / SC-001. The recorded typed error surfaces and the process
ends, in 100% of forced runs.

**Steps**:

1. Read WP01's delivered `window.rs` and its report. Identify the exact
   `cfg(debug_assertions)` seam and how to arm it from a test.
2. Add a section that: arms the seam so **both** close attempts fail; drives a
   page render failure so a `PageRenderFailed` is already recorded on the
   first-error slot; and lets the shell run.
3. Assert **all three** of:
   - the process **terminates** — bounded wait, then a hard failure if it does
     not. A hang must fail the section loudly, not time the whole suite out with
     an ambiguous signature. Give the bound a comment explaining the number.
   - the exit is **nonzero**;
   - the surfaced error is the recorded **`PageRenderFailed`**, not the
     `WindowClose` — the latch's precedence (FR-002) proven end-to-end, matched
     on the typed error's identity, never by fuzzy string search on console
     output.
4. Add the second case from US1 acceptance scenario 2: the same double-close
   failure with **no prior error recorded** surfaces the `WindowClose` error
   itself. This is the one case where `WindowClose` is what the operator sees.
5. This section needs a real window, so it is a **live** section. Register it
   inside `run_live_sections` through the same production protocol seam every
   other live section uses (l.1233-1237). Add a **new** entry to the skip list
   (l.174-181) describing it in the established style, naming the subtask id and
   what it proves. Do **not** extend an existing entry's text to cover it.
6. If the subprocess pattern at l.3186 fits, reuse it rather than inventing a
   second way to observe an exit code.

**Files**: `tests/webview_projection_shell.rs`.

**Validation**: `CREST_WEBVIEW_TESTS=1 cargo test --test
webview_projection_shell -- --nocapture` — the section runs and passes; the
headless run prints the new skip line and still reports its skip list correctly.

**Edge cases**:

- A section that passes because the seam was never actually armed is worthless.
  Assert that the forced condition really occurred (e.g. the section observes
  the close was attempted twice), not just that the process exited.
- Release builds compile the seam out. If the acceptance suite can build in
  release, the section must be gated so it does not silently vanish into a
  false pass — a compiled-out section must skip loudly or fail to build, never
  pass quietly.
- Do not leave a stray window or subprocess alive on failure; the suite's later
  sections and the operator's machine both suffer.

### T014 — Corrupted superseded-late ack section + well-formed negative control

**Purpose**: FR-003 / SC-002 first half. A corrupted superseded-late ack is
typed-rejected.

**Steps**:

1. Read WP02's delivered `projection_channel.rs` and its report. Note which
   retirement path (capacity eviction vs ack consumption) puts an identity in
   the store, and drive **both** — a proof that covers one path would pass while
   the other stayed unvalidated.
2. Add a section that, using the production `ProjectionChannel` and the
   production document/ack construction this suite already has (see the fixture
   helpers around l.267-360 and `page_painted_ack`-style builders):
   - pushes documents until a generation is retired;
   - feeds an ack for that retired generation with **one identity field
     corrupted**; asserts `PaintedAckError::IdentityMismatch` naming that field.
     Cover at least two different fields.
3. Add the **well-formed negative control** in the same section: the identical
   setup with an uncorrupted ack asserts `ForwardedAck::SupersededLate`. This is
   what proves the change rejects only what it should (US2 acceptance scenario 2)
   and it is the control quickstart requires to keep passing when the mechanism
   is bypassed.
4. Add the beyond-the-retained-window case: an ack older than the retained
   window keeps today's lost-frame behavior — `SupersededLate`, **not** a
   rejection, even with a corrupted identity. A false rejection there is the
   NFR-001 failure mode.
5. This section needs **no window**. Put it with the unconditional headless
   sections above the gate (alongside `prove_protocol_policy_parity` and
   friends) so it runs everywhere and adds **no** skip-list entry. Only
   window-bearing sections belong in the skip list.

**Files**: `tests/webview_projection_shell.rs`.

**Validation**: `cargo test --test webview_projection_shell` (headless) — the
section runs and passes with no new skip entry.

**Edge cases**:

- Use the suite's real fixture documents, not hand-rolled JSON that happens to
  satisfy the identity fields. The point is a production-path proof.
- Assert on the typed error variant and its `field`, never on a `Display`
  string.
- If WP02 factored the comparison into a shared helper, do not test the helper
  directly — test through `forward_ack`, which is what production calls.

### T015 — Suite-wide zero-ack-rejection negative control

**Purpose**: FR-003 / SC-002 second half and US2 acceptance scenario 3: a full
live run records zero ack rejections. This is the guard that WP02 did not
introduce a false rejection into healthy operation.

**Steps**:

1. Read the existing render-error negative control at l.2958-2983 and the
   "deliberately last" comment at l.2451. Mirror that structure exactly — do not
   invent a second bookkeeping mechanism.
2. Count `PaintedAckError` outcomes (all variants, not just
   `IdentityMismatch`) across every **healthy** live section. Sections that
   deliberately feed a bad ack must be excluded by construction, not by a
   subtraction — the existing render-error control shows how (the forced-failure
   subprocess is separate from the healthy sections it counts).
3. Assert the count is exactly zero, with a failure message that names what a
   nonzero count means: WP02's validation rejected an ack a healthy run
   legitimately produced, which is a product behavior change (NFR-001).
4. Place the assertion **last** among the live sections, after every healthy
   section has run.

**Files**: `tests/webview_projection_shell.rs`.

**Validation**: `CREST_WEBVIEW_TESTS=1 cargo test --test
webview_projection_shell -- --nocapture` — zero rejections, suite green.

**Edge cases**:

- If the counter is threaded through existing sections, do not alter what those
  sections assert. Adding an observation is fine; changing an assertion is
  NFR-002.
- A control that can only ever read zero (because nothing increments it) is not
  a control. Prove it increments: temporarily corrupt an ack in a healthy
  section, confirm the control fires, revert. Record that in T016.

### T016 — Full gated run and the disable-the-mechanism probes

**Purpose**: NFR-001, NFR-002, SC-005, and the quickstart's falsifiability bar.

**Steps**:

1. Run the full gated suite:
   ```
   CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell -- --nocapture
   ```
   It must print `CREST_ACCEPTANCE webview_projection_shell passed (skipped:
   none)`. Paste the tail of the output including that line and the reported
   projection-to-paint p95 (it must stay within the declared 50 ms — NFR-001).
2. Run the headless form too and paste its skip list, showing your new T013
   entry present and every pre-existing entry **byte-identical**.
3. **Probe 1 (FR-001)** — per quickstart: restore the old
   `close_window_once_with_retry` return locally (record the typed error, return
   normally, no exit edge). Confirm the T013 section **fails**, with a timeout
   or never-surfaced-error signature. Restore WP01's fix; confirm it passes.
   Paste both outcomes.
4. **Probe 2 (FR-003)** — per quickstart: bypass the retired-identity comparison
   in the superseded-late branch locally. Confirm the T014 corrupted-ack case
   **fails** and the well-formed negative control **still passes**. Restore;
   confirm all green. Paste both outcomes.
5. **Probe 3 (T015)** — the increment proof from T015's edge cases. Paste both
   outcomes.
6. Confirm and state in the report: no frozen baseline, threshold, skip entry,
   or assertion was loosened; the only skip-list change is one added entry.
7. Report the net line delta on `tests/` (NFR-003 — this WP adds lines; say how
   many, so the mission-level net can be computed).

**Files**: none modified — verification and reporting only. Local probe edits
must be reverted before the final run; **never commit a probe**.

**Validation**: everything above pasted into the WP report.

**Edge cases**:

- The live run needs a real WKWebView and a session that can open windows. If it
  cannot run in your environment, say so explicitly and do **not** substitute a
  headless run for it — an unrun live proof is an unrun proof, not a skip.
- Probes are local, temporary, and reverted. `git status` must be clean of them
  before you request review.

## Branch Strategy

- **Planning base branch**: `feat/shell-hygiene`
- **Merge target branch**: `feat/shell-hygiene`
- **Depends on**: WP01, WP02 — branch from a base that already carries both.
  Their production seams are your test's subject; starting before they land
  produces a proof of nothing.

Planning artifacts for this mission were generated on `feat/shell-hygiene`.
During implementation this WP works on its own lane branch and merges back into
`feat/shell-hygiene` unless the human explicitly redirects the landing branch.

### Gate context — read this, it prevents three known failures

1. **Commit ONLY your owned production/test files on the lane branch.** Your
   owned file is `tests/webview_projection_shell.rs`. The move-task gate
   **REFUSES** commits touching `kitty-specs/` on a lane branch. Review
   artifacts, evidence, and status files go on `feat/shell-hygiene` from the
   primary checkout — not from your lane. If a proof produces an evidence
   artifact, name it in your report and let the mission owner land it on
   `feat/`.
2. **Do not park waiting for a background notification.** The live suite is
   long-running; use bounded foreground waits and check the run state yourself.
   Never end a turn with "waiting for the live run to finish".
3. **Run `cargo test --lib` and the headless suites before requesting review**,
   in addition to the gated live run. The headless set is
   `cargo test --test webview_projection_shell --test component_vocabulary
   --test component_composition`. Paste the results.
4. **NFR-001 forbids product behavior change; NFR-002 forbids weakening any
   proof.** No frozen baseline, threshold, skip list, or assertion may be
   loosened. If a proof fails, the fix is in the code, never in the proof.

## Definition of Done

- [ ] A live forced-double-close-failure section asserts termination (bounded,
      with a loud failure on hang), a nonzero exit, and that the surfaced error
      is the recorded `PageRenderFailed`; plus the no-prior-error case surfacing
      `WindowClose`.
- [ ] The section serves through the same production protocol seam as every
      other live section, and adds **one new** honest-skip entry; no existing
      entry is widened or reworded.
- [ ] A headless corrupted-superseded-late-ack section asserts
      `PaintedAckError::IdentityMismatch` for at least two fields, across **both**
      retirement paths, with a well-formed negative control and a
      beyond-the-window lost-frame case.
- [ ] A suite-wide control asserts zero ack rejections across every healthy live
      section, placed last, mirroring the render-error control.
- [ ] `CREST_WEBVIEW_TESTS=1` run prints `skipped: none`; p95 within 50 ms;
      output pasted.
- [ ] All three probes recorded with both outcomes; no probe committed;
      `git status` clean.
- [ ] Net `tests/` line delta reported.
- [ ] No file outside `tests/webview_projection_shell.rs` is modified.

## Risks / Reviewer Guidance

- **The first thing to check is falsifiability.** Both probes are quickstart-
  mandated. A reviewer should re-run at least one and confirm the section dies.
  A section that passes with its mechanism disabled is not a proof.
- **Check the skip list diff line by line.** The only acceptable change is one
  added entry. A reworded existing entry is a widened skip and violates NFR-002.
- **Check the negative controls are real.** T014's well-formed case and T015's
  zero-count must both be able to fail. Ask for the increment proof.
- **Check the hang failure mode.** If the double-close section hangs, the suite
  must fail with a signature that says "did not terminate", not sit until an
  outer harness timeout. That is the exact failure this mission exists to make
  visible.
- **Check the latch precedence assertion is typed.** Matching on console text
  would let a `WindowClose` masquerade as a `PageRenderFailed`.
- **Check both retirement paths are exercised in T014.** Covering only capacity
  eviction (or only ack consumption) leaves half of WP02 unproven, which is the
  same partial-enforcement shape RISK-4 was.
- **Check the p95.** NFR-001's latency bar is a product-behavior guard; a
  regression there means something in WP01/WP02 moved more than intended.
