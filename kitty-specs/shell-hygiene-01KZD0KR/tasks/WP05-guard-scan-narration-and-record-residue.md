---
work_package_id: WP05
title: Guard-scan coverage, gallery narration, and record residue
dependencies:
- WP03
requirement_refs:
- FR-006
- FR-007
- FR-008
- NFR-002
planning_base_branch: feat/shell-hygiene
merge_target_branch: feat/shell-hygiene
branch_strategy: Planning artifacts for this mission were generated on feat/shell-hygiene. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/shell-hygiene unless the human explicitly redirects the landing branch.
subtasks:
- T017
- T018
- T019
- T020
- T021
history:
- '2026-08-06: authored from plan IC-04/IC-05/IC-06, research D5/D6, crest-spec assets TestingContextModules and ComponentCompositionAcceptanceTests, mission-review OBS-1 + SMELL-1 residue + DRIFT-3'
agent_profile: implementer-ivan
authoritative_surface: tests/
create_intent: []
execution_mode: code_change
owned_files:
- tests/component_composition.rs
- src/testing/component_gallery_scene.rs
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

## ⛔ C-003 — THE GALLERY IS RETAINED

**The component gallery scene, its page assets (`webview-page/gallery.js`,
`webview-page/gallery.css`), its scene source
(`src/testing/component_gallery_scene.rs`), its CLI option, and its
`demo-live-component-library` make target are NOT deleted, NOT converted, and
NOT reduced. Retirement was drafted, surfaced, and deliberately reversed by the
operator on 2026-08-06. Deleting, converting, or reducing ANY gallery artifact
FAILS THIS WORK PACKAGE.**

T019 is **narration only** — a comment. Full CSP parity was considered and
rejected (`research.md` D5): it would require converting the gallery's inline-
style painting and re-homing 48 gallery-borne proof references, for no product
gain. Do not implement it. Do not add a CSP to the gallery handler.

## Objective

Three small, independent corrections that all say the same thing: make the
record match reality.

- **FR-007 (SMELL-1 residue)** — the purity-needle scan binds `page.js` alone
  while the adjacent key-handler scan already enumerates three sources, and
  `gallery.js`'s own header claims the properties the purity loop enforces.
  Extend the loop to every page source it means to bind.
- **FR-006 (OBS-1)** — narrate the gallery scene's policy-free protocol handler
  as a deliberate, declared exemption, so nobody later mistakes it for the drift
  that hid a shipped paint defect and nobody "fixes" it by accident.
- **FR-008 (DRIFT-3)** — discharge documentation residue on two completed
  missions' records.

**Authorities** (cited, not restated — read them):

- `spec.md` FR-006, FR-007, FR-008, NFR-002, C-003; User Stories 4, 5, 6 and
  their acceptance scenarios.
- `plan.md` IC-04, IC-05, IC-06.
- `research.md` **D5** (the gallery is retained; what the narration must name
  and why CSP parity was reversed), **D6** (which scan, which sources, and that
  a legitimate construct is fixed at the source or gets a declared narrated
  exemption — never a silent carve-out), **D4** (why the assertion message at
  l.1742 is wrong but the assertion itself is fine).
- Crest-spec assets `TestingContextModules`, `ComponentCompositionAcceptanceTests`;
  `capability.component_vocabulary` and its gallery acceptance. Read via
  `spec-kitty crest-spec context`.

### Owned files vs documentation surface

The frontmatter's `owned_files` lists **two** files:
`tests/component_composition.rs` and `src/testing/component_gallery_scene.rs`.
Those are what your lane branch commits.

T020's target — the two completed missions' planning trees at
`kitty-specs/webview-shell-cutover-01KZAC7Q/**` and
`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/**` — is your
**documentation surface**: yours to edit and report, never yours to commit from
a lane branch. Spec Kitty rejects `kitty-specs/` paths in `owned_files` outright
(`INVALID_WP_OWNED_FILES_KITTY_SPECS`), which is the same rule the move-task
gate enforces on lane commits. See T020's commit-handling note.

**Hard boundaries**:

- Do **not** edit `src/shell/webview/*` (WP01/WP02/WP03),
  `src/shell/component_vocabulary.rs` or `src/testing/live_demo_runner.rs`
  (WP03), or `tests/webview_projection_shell.rs` (WP04).
- Do **not** delete or weaken any existing assertion in
  `tests/component_composition.rs` (NFR-002). You are adding coverage, not
  trading it.
- Do **not** rewrite a closed gate's evidence or verdict to read better in
  hindsight (IC-06). T020 amends status fields and terminology only.

## Context: what exists today

### The purity-needle loop (`tests/component_composition.rs:1780-1806`)

```rust
let page_js: String = page_source("page.js")
    .lines().map(|line| strip(line, true)).collect::<Vec<_>>().join("\n");
let index_html = page_source("index.html");
let gallery_js: String = page_source("gallery.js")
    .lines().map(|line| strip(line, true)).collect::<Vec<_>>().join("\n");
for needle in [
    "Date.now", "Math.random", "performance.now", "setInterval",
    "setTimeout", "localStorage", "sessionStorage", "fetch(", "XMLHttpRequest",
] {
    assert!(
        !page_js.contains(needle),
        "the render script owns `{needle}`, which a pure render cannot"
    );
}
```

`gallery_js` and `index_html` are already built and comment-stripped — they are
simply never fed to the purity needles. The failure message names no source
because there is only one.

### The enumeration style already established (l.1806-1819)

```rust
// The gallery script is held to the same input rule as the page: its
// digit keys are bound Rust-side by the testing scene, never page-side.
for (name, source) in [
    ("page.js", &page_js),
    ("index.html", &index_html),
    ("gallery.js", &gallery_js),
] {
    for needle in ["keydown", "keyup", "keypress"] {
        assert!(
            !source.contains(needle),
            "{name} registers a key handler (`{needle}`); keys are captured Rust-side"
        );
    }
}
```

This is the style D6 says to match: `(name, source)` pairs so every failure
names its source.

### The stale assertion message (l.1740-1745)

```rust
assert!(
    !code.contains(&forbidden_action),
    "{path} names {forbidden_action}; a component returns ControlIntent and \
     converts nothing"
);
```

Per D4: the assertion tests **passivity** and is unaffected by WP03's deletion.
Only the message names a type that no longer exists.

### The gallery protocol handler (`src/testing/component_gallery_scene.rs:3083-3095`)

```rust
let app = tauri::Builder::default()
    .register_uri_scheme_protocol("crest", move |_context, request| {
        match gallery_asset(request.uri().path()) {
            Some((content_type, body)) => tauri::http::Response::builder()
                .header("Content-Type", content_type)
                .body(body)
                .expect("the embedded asset response is well-formed"),
            None => tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .expect("the empty not-found response is well-formed"),
        }
    })
```

Only `Content-Type` — no CSP. Unreachable from the shipped window because
`page_asset` (the production asset table in `src/shell/webview/window.rs`) has
no gallery entry.

### The DRIFT-3 residue

`kitty-specs/webview-shell-cutover-01KZAC7Q/mission-review.md:167` names the
three items:

> Process: spec.md status flip; quantify the NFR-002 leak bound; reword residual
> "migration" terminology in spec/plan.

Concretely:

- **NFR-002 leak bound** — `webview-shell-cutover-01KZAC7Q/spec.md:170` says
  "no leak growth trend" with no metric. `analysis-report.md:33` and finding A1
  (l.76) flag it. `mission-review.md:45` records the discharge: *"29.43 Hz
  sustained, 0 lost, RSS declining"*, and l.65 calls it *"discharged honestly via
  committed declining RSS series. LOW."*
- **"migration" terminology** — `webview-shell-cutover-01KZAC7Q/plan.md:25`
  ("migrated scenes"), `plan.md:26` ("4 retained live scenes migrated"),
  `plan.md:119` ("IC-03 — Retained scene migration"), and `spec.md:169`
  ("migrated scenes"). The cutover spec's own Domain Language at l.148 already
  says: *"Avoid 'migration' for the demo scenes: scenes are not rewritten; the
  shell under them changes."* The documents violate their own canonical term.
- **Status fields** — `webview-shell-cutover-01KZAC7Q/spec.md:5` reads
  `**Status**: Draft`, and its FR/NFR/constraint tables all read `Open` in the
  Status column. `expandable-effects-and-bus-topology-01KYNGX8/spec.md:5` is the
  same. Both missions are complete.

## Subtasks

### T017 — Extend the purity-needle loop to every page source

**Purpose**: FR-007 / SC-004. A source that claims a property the guard does not
check is a gap between the record and the proof.

**Steps**:

1. Restructure the purity loop (l.1789-1806) over the same `(name, source)`
   enumeration style the adjacent key-handler loop uses, so each failure names
   its source and its needle:
   ```rust
   for (name, source) in [
       ("page.js", &page_js),
       ("index.html", &index_html),
       ("gallery.js", &gallery_js),
   ] { for needle in [ ...the nine needles... ] { assert!(!source.contains(needle), "{name} owns `{needle}`, which a pure render cannot"); } }
   ```
   Decide deliberately which sources the loop *means* to bind and say so in the
   comment above it. `page.js`, `index.html`, and `gallery.js` are the three
   already read in this function; if you conclude one of them is not meant to be
   bound (e.g. `index.html` is markup, not script), state the reason in the
   comment rather than silently omitting it — an unstated omission is the exact
   gap this subtask closes.
2. Keep the nine needles byte-identical. Do not remove one to make a source
   pass (NFR-002).
3. Run the suite. **If a legitimate gallery construct fires a needle**, D6 gives
   exactly two acceptable fixes:
   - fix it at the source (change the gallery JS), **provided** the change does
     not delete, convert, or reduce any gallery capability (C-003) — a
     `setTimeout` replaced by a synchronous call is fine, a removed gallery page
     is not; or
   - add a **declared, narrated exemption** mirroring the existing precedent in
     this file (the fader-geometry exemption block from the previous mission is
     the model): a named construct, at a named location, with the reason and its
     blast radius stated.
   A silent carve-out — quietly excluding the source, or the needle, or
   narrowing the string match — is forbidden.
4. Keep the existing key-handler loop exactly as it is. It already works.

**Files**: `tests/component_composition.rs`.

**Validation**: `cargo test --test component_composition`; plus T021's planted
violations.

**Edge cases**:

- `gallery_js` and `page_js` are comment-stripped; `index_html` is **not**
  (l.1783). If you bind `index.html`, a needle appearing in an HTML comment
  would fire falsely. Either strip it consistently or state why raw is correct.
- `"fetch("` with the paren is deliberate (it avoids matching `prefetch`).
  Preserve the exact needle strings.

### T018 — Correct the assertion message naming the deleted `ControlIntent`

**Purpose**: FR-005's tail and D4's recorded consequence. The message must not
name a type that no longer exists.

**Steps**:

1. Confirm WP03 has landed and `ControlIntent` is gone from `src/`:
   `grep -rn "ControlIntent" src/` → zero hits.
2. Rewrite the message at l.1742-1744 to state what the assertion actually
   tests: a component source may not name the forbidden action-conversion
   construct, because components present and never act. Say the surviving rule
   (D4: no ownership, no caching, no reach into `AppState`, no input-to-action
   conversion), not the retired mechanism.
3. **Do not touch the assertion itself.** `!code.contains(&forbidden_action)`
   is the passivity test and is unaffected by the retirement.
4. Grep the whole file for other references to the retired family; there may be
   comments as well as this one message. Correct each the same way.

**Files**: `tests/component_composition.rs`.

**Validation**: `cargo test --test component_composition`;
`grep -rn "ControlIntent\|ControlRequest\|CompositionIntent" src/ tests/` →
zero hits.

**Edge cases**:

- If WP03 has not landed, do this subtask last — a message corrected before the
  type is deleted leaves the file momentarily accurate and then wrong again.

### T019 — Narrate the gallery's policy-free protocol handler

**Purpose**: FR-006 / US4 acceptance scenario 1. **Narration only. C-003.**

**Steps**:

1. Add a comment immediately above `register_uri_scheme_protocol` at
   `src/testing/component_gallery_scene.rs:3084`. It must state, explicitly:
   - **What is deliberate**: this handler attaches `Content-Type` and no content
     security policy, and that is a recorded decision, not an oversight.
   - **Why it is unreachable from the shipped window**: `page_asset`, the
     production asset table in `src/shell/webview/window.rs`, has **no gallery
     entry** — the shipped window cannot load gallery sources. Name the asset
     table by name so a future reader can check whether it is still true.
   - **What defect class it must not be confused with**: DRIFT-1 from the
     webview-shell-cutover mission review — the harness serving the production
     page without the production CSP, which hid a shipped paint-fidelity defect.
     The shapes look identical; the difference is reachability.
   - **What would have to change if it stopped being true**: if the production
     asset table ever gains a gallery entry, this handler must serve through the
     production policy seam (`protocol_response`/`PAGE_CSP`) exactly as the
     acceptance harness does.
   - **Why full parity was not done**: reference `research.md` D5 and the
     operator decision — converting the gallery's inline-style painting and
     re-homing its proof references for no product gain.
2. Cite FR-006 and OBS-1 so the narration is findable from the record.
3. **Change no behavior.** No CSP header, no new response branch, no refactor of
   `gallery_asset`, no change to the 404 branch. This subtask's entire diff is a
   comment.

**Files**: `src/testing/component_gallery_scene.rs`.

**Validation**: `cargo build`; `cargo test --lib`;
`git diff src/testing/component_gallery_scene.rs` shows **only** added comment
lines.

**Edge cases**:

- A narration that says "this is fine" without naming the reachability
  precondition is not narration — it is a shrug. The precondition is what makes
  it checkable later.

### T020 — Discharge the DRIFT-3 documentation residue

**Purpose**: FR-008 / US6. The record says what actually happened.

**Steps**:

1. **NFR-002 leak bound** — in
   `kitty-specs/webview-shell-cutover-01KZAC7Q/spec.md:170`, either:
   - quantify it: state the measured field(s) and the acceptance bound the soak
     actually applied (the mission review's recorded evidence is *29.43 Hz
     sustained, 0 lost records, RSS declining across sampling windows*); or
   - record it as **unquantified-by-decision** with its rationale, referencing
     `analysis-report.md` finding A1 and `mission-review.md:65`'s honest
     discharge.
   Either is acceptable per FR-008; pick one and make the document say which.
   Do **not** invent a number the soak did not measure.
2. **"migration" terminology** — remove it from
   `webview-shell-cutover-01KZAC7Q/plan.md:25, 26, 119` and `spec.md:169`,
   replacing it with language consistent with that mission's own Domain Language
   at `spec.md:148`: *scenes are not rewritten; the shell under them changes*.
   Leave the Domain Language entry itself intact — it is the rule, and it must
   keep naming the term it forbids.
3. **Status fields** — no completed mission's spec may still read `Draft` or
   `Open`:
   - `webview-shell-cutover-01KZAC7Q/spec.md:5` and its FR/NFR/constraint table
     Status columns;
   - `expandable-effects-and-bus-topology-01KYNGX8/spec.md:5` and its tables.
   Set them to the terminal state the mission actually reached. Check each
   mission's `status.json` / `mission-review.md` verdict for the truthful value
   rather than assuming.
4. **Do not** rewrite any evidence, verdict, finding, or acceptance record.
   Status fields and terminology only. If a change would make a closed gate read
   better than it did, do not make it — report it instead.
5. Your two owned `kitty-specs/**` globs are the only planning trees you may
   touch. Do not amend this mission's own `kitty-specs/shell-hygiene-01KZD0KR/`
   artifacts.

**Files**: `kitty-specs/webview-shell-cutover-01KZAC7Q/**`,
`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/**` — your
documentation surface, deliberately **not** in `owned_files` (see "Owned files
vs documentation surface" above).

**Validation**:

```
grep -rn "migrat" kitty-specs/webview-shell-cutover-01KZAC7Q/spec.md kitty-specs/webview-shell-cutover-01KZAC7Q/plan.md
grep -rn "^\*\*Status\*\*: Draft" kitty-specs/webview-shell-cutover-01KZAC7Q/spec.md kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/spec.md
```
The first must return only the Domain Language line at `spec.md:148`; the second
must be empty.

**⚠️ Commit handling for T020** — the move-task gate **REFUSES** `kitty-specs/`
commits on a lane branch. Make these edits, verify them, and then:

- do **not** `git add` or commit them on your lane branch;
- list every changed file with its exact edit in your WP report, so the mission
  owner lands them on `feat/shell-hygiene` from the primary checkout;
- if your harness gives you the primary checkout on `feat/shell-hygiene`
  directly, apply and commit them there instead, with a conventional-commit
  message and the `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`
  trailer.

Your lane commit carries **only** `tests/component_composition.rs` and
`src/testing/component_gallery_scene.rs`.

**Edge cases**:

- Some Status columns may already be correct. Do not flip a value that is
  already truthful.
- `expandable-effects-and-bus-topology-01KYNGX8` has its **own** NFR-002 (bounded
  topology capacity) — unrelated to the cutover's leak bound. Do not conflate
  them.

### T021 — Run both guard suites; record a planted-violation probe per source

**Purpose**: FR-007 / SC-004 falsifiability. A guard that cannot fail guards
nothing.

**Steps**:

1. Run both guard suites and paste the output:
   ```
   cargo test --test component_composition --test component_vocabulary
   ```
2. For **each newly covered source** (each source T017 added to the purity loop),
   plant one violation in a scratch copy — e.g. a `Date.now()` call in
   `gallery.js` — run `component_composition`, and confirm it fails **naming that
   source and the offending needle**. Revert. Repeat per source. Paste each
   before/after.
3. Confirm the gallery is intact (C-003):
   ```
   ls webview-page/gallery.js webview-page/gallery.css src/testing/component_gallery_scene.rs
   grep -n "demo-live-component-library" Makefile
   ```
   Paste the output. All must still exist.
4. Confirm no existing assertion was weakened: `git diff tests/component_composition.rs`
   should show added coverage and one corrected message, no removed or relaxed
   assertion (NFR-002).
5. Report the net line delta on `src/` and `tests/` (NFR-003).

**Files**: none modified — verification only. Planted violations are local,
temporary, and reverted; **never commit one**.

**Validation**: everything above pasted into the WP report; `git status` clean
of probe edits.

**Edge cases**:

- If a planted violation does **not** fail the suite, the source was not
  actually bound. Fix T017 and re-probe; do not report a probe that did not
  fire.

## Branch Strategy

- **Planning base branch**: `feat/shell-hygiene`
- **Merge target branch**: `feat/shell-hygiene`
- **Depends on**: WP03 — T018 corrects a message naming a type WP03 deletes.
  Branch from a base that already carries WP03.

Planning artifacts for this mission were generated on `feat/shell-hygiene`.
During implementation this WP works on its own lane branch and merges back into
`feat/shell-hygiene` unless the human explicitly redirects the landing branch.

### Gate context — read this, it prevents three known failures

1. **Commit ONLY your owned production/test files on the lane branch.** For this
   WP that means `tests/component_composition.rs` and
   `src/testing/component_gallery_scene.rs` **only**. The move-task gate
   **REFUSES** commits touching `kitty-specs/` on a lane branch — and T020's
   whole output is `kitty-specs/`. Follow T020's commit-handling note: report
   the doc edits for the primary checkout on `feat/shell-hygiene`, do not commit
   them from your lane.
2. **Do not park waiting for a background notification.** If you launch anything
   in the background, use bounded foreground waits and check the run state
   yourself. Never end a turn with "waiting for the suite to finish".
3. **Run `cargo test --lib` and the headless suites before requesting review.**
   The headless set is
   `cargo test --test webview_projection_shell --test component_vocabulary
   --test component_composition`. Paste the results.
4. **NFR-001 forbids product behavior change; NFR-002 forbids weakening any
   proof.** No frozen baseline, threshold, skip list, or assertion may be
   loosened. If a newly bound source fails, the fix is at the source or a
   declared narrated exemption — never a silent carve-out.

## Definition of Done

- [ ] The purity-needle loop enumerates every page source it means to bind, in
      the established `(name, source)` style, with each failure naming its
      source and needle; the comment states which sources are bound and why.
- [ ] All nine needles preserved byte-identical; no existing assertion removed
      or relaxed.
- [ ] Any needle hit was fixed at the source or given a declared, narrated
      exemption mirroring the existing precedent — never a silent carve-out.
- [ ] The assertion message at l.1742 no longer names `ControlIntent`; the
      assertion itself is untouched; zero remaining references to the retired
      family in `src/` and `tests/`.
- [ ] The gallery protocol handler carries the FR-006 narration naming the
      production asset table, the DRIFT-1 defect class, the condition under
      which it stops being true, and the D5 reversal — and **nothing else in
      that file changed**.
- [ ] **C-003 verified**: `gallery.js`, `gallery.css`,
      `component_gallery_scene.rs`, the CLI option, and the
      `demo-live-component-library` make target all still exist and are
      unreduced; the check output is pasted.
- [ ] DRIFT-3 discharged: leak bound quantified or recorded
      unquantified-by-decision with rationale; "migration" gone from the cutover
      spec/plan except the Domain Language rule; no completed mission's spec
      reads `Draft`/`Open`; the two verification greps are clean.
- [ ] T020's `kitty-specs/` edits are reported for the primary checkout, **not**
      committed on the lane branch.
- [ ] Both guard suites green; one planted-violation probe recorded per newly
      covered source, each failing by name; `git status` clean of probes.
- [ ] Net `src/` and `tests/` line deltas reported.

## Risks / Reviewer Guidance

- **C-003 first.** Before anything else, a reviewer should run the gallery-intact
  checks. Any deleted, converted, or reduced gallery artifact fails this WP
  outright, regardless of how good the rest is.
- **T019 must be a comment-only diff.** `git diff
  src/testing/component_gallery_scene.rs` should show added `//` lines and
  nothing else. A CSP header added here is the reversed decision being
  re-implemented.
- **Check the narration is checkable.** It must name `page_asset` as the
  reachability precondition. "This is a testing scene so it's fine" is not
  narration; the point is that a future reader can verify the precondition still
  holds.
- **Check for a silent carve-out.** If a needle hit was handled, look at how: a
  narrowed needle string, an omitted source, or a `#[allow]` is a carve-out. A
  named, located, reasoned exemption block is the sanctioned form (D6).
- **Check the probes fired.** One per newly covered source, each failure naming
  that source. A single probe on `page.js` proves the loop that already worked.
- **Check T020 changed only status and terminology.** Any edit to a finding, a
  verdict, an evidence figure, or an acceptance record is out of bounds — the
  record must stay honest about what happened, including what was imperfect.
- **Check the lane commit's file list.** It must contain exactly two files. A
  `kitty-specs/` path in the lane commit is the move-task gate about to refuse.
