---
wp_id: WP05
reviewer_agent: reviewer-renata
cycle_number: 1
verdict: approved
mission_slug: shell-hygiene-01KZD0KR
reviewed_at: "2026-08-07T04:58:00Z"
affected_files:
  - path: tests/component_composition.rs
  - path: src/testing/component_gallery_scene.rs
  - path: kitty-specs/webview-shell-cutover-01KZAC7Q/spec.md
  - path: kitty-specs/webview-shell-cutover-01KZAC7Q/plan.md
  - path: kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/spec.md
---

# WP05 review — cycle 1: APPROVED

Reviewed lane commit `0aafd1d` on `kitty/mission-shell-hygiene-01KZD0KR-lane-e`
(lane HEAD `b701129`, a merge of the mission branch) and doc commit `f34b5be`
on `feat/shell-hygiene`.

`git show 0aafd1d --name-only` returns exactly two files —
`src/testing/component_gallery_scene.rs` and `tests/component_composition.rs`,
+94/−16 — matching `owned_files` exactly. `git diff
kitty/mission-shell-hygiene-01KZD0KR..HEAD --name-only | grep kitty-specs`
returns nothing: no `kitty-specs/` path on the lane branch, so the move-task
gate has nothing to refuse. The lane's `component_vocabulary.rs`,
`frame_stream.rs`, `mod.rs`, and `live_demo_runner.rs` deltas arrived through
the merged lane-c (WP03, already approved at `b0acb24`) and are not WP05's.

Every finding below was gathered mechanically in the lane worktree. Nothing is
taken from the implementer's report; every probe was planted, run, and reverted
by the reviewer.

## 1. C-003 — the gallery survives intact: **PASS** (blocking constraint)

All five artifacts present and unreduced:

```
webview-page/gallery.css                 9927 bytes
webview-page/gallery.js                 22779 bytes
src/testing/component_gallery_scene.rs 170626 bytes
src/bin/crest_synth.rs:325,342          --demo-live-component-library
Makefile:67-68                          demo-live-component-library target
```

`git diff kitty/mission-shell-hygiene-01KZD0KR..HEAD --stat -- webview-page/
src/bin/crest_synth.rs Makefile` is **empty** — the gallery's page assets, CLI
option, and make target are byte-untouched.

The `component_gallery_scene.rs` diff is genuinely comment-only. Filtering the
commit's added lines for anything that is not a `//` comment returns zero
lines; the diff is +39/−0 with no deletions at all, so no code could have been
displaced.

**Ran it myself.** `./target/release/crest-synth --demo-live-component-library`
launched a live window. Screenshot confirms: title "crest-synth — component
gallery", `PAGE 1 / 15`, the full fifteen-page nav (COLORS, TYPE, SPACING,
STATES, TEXT, VALUES, HINTS, BANDS, ROWS, TOGGLES, FADERS, BROWSER, SHELL,
HEADERS, STRIP), both authored densities side by side (DESKTOP 1920×1080 and
STEAMDECK 1280×800), and all 17 declared colors painting their swatches. The
window closed cleanly on signal.

## 2. T017 — the scan extension is real; `strip_html_comments` is legitimate scoping

**Verdict on the stripper: legitimate scoping, not a loophole.** The decisive
evidence is that it was never load-bearing:

```
needle hits in RAW (unstripped) webview-page/index.html:
Date.now:0  Math.random:0  performance.now:0  setInterval:0  setTimeout:0
localStorage:0  sessionStorage:0  fetch(:0  XMLHttpRequest:0
```

Binding `index.html` raw would have passed today. The stripper therefore was
**not** added to make a failing scan pass — the shape a silent carve-out always
takes. It is the consistency option the WP explicitly sanctioned ("Either strip
it consistently or state why raw is correct"): `page_js` and `gallery_js` are
already comment-stripped by `strip(line, true)`, which only understands `//`
line comments and cannot see HTML comment syntax. The implementer's commit
message states plainly that no needle fired and no exemption was needed —
consistent with what I measured.

Scope of the stripper, probed in isolation against the exact function body:

| Probe | Construct | Result |
|---|---|---|
| H | `<?php ?>`, `<% %>`, `/* */` | all **preserved** — only `<!-- -->` removed |
| I | document with no comments | output **byte-identical** to input |

It cannot hide a real violation in the shapes that matter. Planted-violation
probes on the real files, run against
`no_component_owns_caches_or_dispatches_application_state`, reverted after each:

| # | Source | Planted | Result |
|---|---|---|---|
| P1 | `webview-page/page.js` | `const __probe = Date.now();` | **FAILED** — ``page.js owns `Date.now`, which a pure render cannot`` (line 1839) |
| P2 | `webview-page/gallery.js` | `const __probe = Math.random();` | **FAILED** — ``gallery.js owns `Math.random`, which a pure render cannot`` |
| P3 | `webview-page/index.html` | `<script>const p = setTimeout(f,1);</script>` | **FAILED** — ``index.html owns `setTimeout`, which a pure render cannot`` |
| P4 | `webview-page/index.html` | `<div onclick="localStorage.getItem(1)"></div>` | **FAILED** — ``index.html owns `localStorage`, which a pure render cannot`` |
| P5 | `webview-page/index.html` | needle inside `<!-- ... -->` | **PASSED** (negative control — the intended exemption) |

Each of the three bound sources fires by name, each failure naming its source
and its needle. P4 confirms an `on*` attribute still fires. P5 confirms the
negative control works without also letting real code through — P3 and P4 are
the proof that it does not.

Two further isolation probes confirm the boundary is tight: code *between* two
comments fires (`<!-- a --><script>Math.random()</script><!-- b -->` →
`Math.random`), and a `-->` followed by real code fires (`localStorage`).

The nine needles are byte-identical before and after — verified by extracting
and sorting both lists:

```
"Date.now", "fetch(", "localStorage", "Math.random", "performance.now",
"sessionStorage", "setInterval", "setTimeout", "XMLHttpRequest"
```

Identical sets, indentation-only change. `"fetch("` retains its paren.

The comment above the loop states which sources are bound and why, and — as the
WP required — states the reason for the one deliberate omission (stylesheets:
"every needle below is a JavaScript host API that CSS cannot name"). That is a
stated omission, not a silent one.

### Non-blocking observation — one narrow blind spot

`strip_html_comments` is silent on the JavaScript Annex B "HTML-like comment"
construct, which browsers **do** execute:

```html
<script>
<!--
const p = Date.now();
-->
</script>
```

Probe P6 planted exactly this in `index.html`: the suite **passed**. Inside a
`<script>` element `<!--` acts as a JS line comment rather than markup, so the
code runs while the stripper deletes it. The same applies to an unterminated
`<!--`, which swallows any real script that follows (the code comment claims
this "matches what ships", which holds for markup content but not for script
content).

This is **not** a rejection, for three reasons: it is a pre-existing property of
the chosen approach rather than something traded away for a pass; the scan would
have this shape under any comment-aware stripper; and production `PAGE_CSP` is
`script-src 'self'` with no `'unsafe-inline'`, so no inline script — legacy-
wrapped or not — executes in the shipped window at all. Worth recording as a
known limit of the guard, not worth blocking on.

## 3. NFR-002 — no assertion weakened: **PASS**

The 16 deleted lines account for exactly, and only, what was claimed:

- 2 lines — the old assertion **message** naming `ControlIntent`;
- 14 lines — the old flat loop (`for needle in [`, the nine needle strings,
  `assert!(`, `!page_js.contains(needle)`, its message, `);`).

Nothing else was deleted. The purity assertion was not relaxed — it was
**broadened** from one source to three. The passivity assertion
`!code.contains(&forbidden_action)` appears as a diff **context** line, not a
changed one. No threshold, skip list, or frozen baseline appears anywhere in
the diff.

## 4. T018 — the assertion message: **PASS**

The assertion itself is untouched; only the message changed:

```rust
assert!(
    !code.contains(&forbidden_action),
    "{path} names {forbidden_action}; a component presents the immutable view \
     data it is handed and never converts an input into an action"
);
```

The message now describes what is actually enforced — passivity, the surviving
D4 rule — and names no retired type.

```
grep -rn "ControlIntent\|ControlRequest\|CompositionIntent" src/ tests/
→ zero hits (exit 1)
```

## 5. T019 — the gallery narration is accurate: **PASS**

I verified the load-bearing claim against the actual function rather than
accepting it. `page_asset` in `src/shell/webview/window.rs:135`:

```rust
"/" | "/index.html" => ...   "/tokens.css" => ...   "/page.css" => ...
"/page.js" => ...            four "/fonts/AzeretMono-*.ttf" arms
_ => None,
```

The narration's enumeration is **exact**: the index document, `tokens.css`,
`page.css`, `page.js`, the four Azeret faces, and **no gallery entry**. The
"every other path is a 404 with no fallback" claim is also correct —
`protocol_response` maps `None` to `status(404)` with an empty body and no
fallback branch. The claim licenses a true belief, and a future reader can check
it in one read, exactly as the WP required.

The defect class is named correctly against the source record: DRIFT-1
(`mission-review.md:54`, "the acceptance harness serves the page without the
production CSP", severity HIGH "as the enabler of RISK-1") and RISK-1
(`mission-review.md:73`, the CSP blocking inline style attributes so fader fills
render empty). The narration states the trigger that would invalidate the
exemption — "the moment `page_asset` gains a gallery entry ... this handler must
serve through `crate::shell::webview::protocol_response`" — and both named
symbols exist and are exported (`src/shell/webview/mod.rs:66`). It cites
FR-006/OBS-1 and research D5. No CSP header, no new branch, no `gallery_asset`
refactor, no 404 change.

## 6. T020 — the documentation edits (`f34b5be`): **PASS** on all six

`f34b5be` touches exactly three files. **No** `mission-review.md`,
`analysis-report.md`, `acceptance-matrix.json`, `deterministic-acceptance.json`,
`evidence/`, `retrospective.yaml`, or `status.*` file is touched by this commit.
No finding, verdict, evidence figure, or acceptance record could have been
rewritten, and none was.

**(a) NFR-002 leak bound — PASS.** The disclaimer is present and unavoidable.
It sits inline, immediately after the words "Leak bound", *before* any number:

> Leak bound (quantified after acceptance, discharging analysis finding A1 — it
> was not a pre-declared bar at the accept gate)

A reader cannot reach the figures without passing it. Every figure traces to
committed evidence, none invented:

| Figure | Source |
|---|---|
| 29.43 Hz sustained | `evidence/soak-300s.log:17` "8830 emits = 29.43 Hz", "first/last-third 29.43/29.43 Hz" |
| max gap 44.3 ms | same line, "max gap 44.3ms" |
| 0 lost records | same line, "lost 0" |
| page-side 8830/8830 | same line, "page-side received 8830 frames" |
| 107728 → 103904 KiB | `evidence/soak-300s-rss.samples.log` lines 16 and 30–34 |

The characterization is **conservative, not flattering**: the series actually
ends at 93568/94160 KiB, below the 103904 plateau the document quotes, so the
recorded decline is understated. The "no monotonic growth across sampling
windows" bound holds against the full series (one startup rise, then a long
decline). The evidence path is cited so the claim is checkable.

**(b) Cutover Status line — PASS.** Every element verified:

- "accepted 2026-08-06 (`a38abf7`)" → `a38abf7` = "Accept
  webview-shell-cutover-01KZAC7Q", Thu Aug 6 2026.
- "25/25 pass" → `acceptance-matrix.json`: 25 criteria, all `pass`,
  `overall_verdict: pass`.
- "mission review returned **FAIL** on RISK-1 and RISK-2" →
  `mission-review.md:155` "## **FAIL**", rationale at :159 naming exactly
  RISK-1 and RISK-2 as the two blocking HIGH findings.
- "DRIFT-1/DRIFT-2 as enablers" → DRIFT-1 is labelled "as the enabler of
  RISK-1" at :55.
- "the entire scope of the follow-on `webview-render-fidelity-hardening-01KZCEF8`"
  → that mission's `spec.md:6`: "This mission fixes exactly those four items
  and nothing else."
- "which is merged" → `4649857` squash merge, plus its retrospective and
  post-merge review commits.

The line **adds** the FAIL verdict to a document that previously said only
"Draft". Nothing was softened; the record now reads worse, and truthfully so.

**(c) Cutover 19 rows Open→Accepted — PASS.** 19 rows changed (8 FR + 4 NFR +
7 C), count confirmed by diff. All 19 IDs appear in `acceptance-matrix.json`
with `pass_fail: pass`; none missing, none non-pass.

**(d) Expandable Status line — PASS.** "accepted 2026-07-31 (`a68c5b0`)" →
`a68c5b0` = "Accept expandable-effects-and-bus-topology-01KYNGX8", Fri Jul 31.
"**PASS WITH NOTES**" → `mission-review.md:134`, verbatim. "including its two
addenda" → Addendum at :157 and Addendum 2 at :187.

**(e) Expandable 38 rows Open→Accepted — PASS.** 38 rows changed (19 FR + 8 NFR
+ 11 C), count confirmed. The 27 FR/NFR rows all appear in that mission's
acceptance matrix with `pass`. The 11 constraint rows do **not** appear as
matrix rows — I checked this specifically — but they are supported by the
mission review's own record: `mission-review.md:51` "Full per-ID trace was
performed for FR-001..019, NFR-001..008, **C-001..011**, SC-001..008" and
":52" "**Every FR, NFR, and constraint is ADEQUATE**", with :69 explaining the
gap directly: "constraints C-001..C-011 are enforced by the 28 deterministic
checks and reviews but are not enumerated as acceptance-matrix rows." Supported.

**(f) `Accepted` vs `Met` — `Accepted` is the correct and more honest word.**
This matters most on the cutover, whose review FAILED on RISK-1: fader fills
rendered empty in the shipped app, and the FR coverage matrix at :46 records
"NFR-003 CSP + release gating | ADEQUATE as specified — **but the CSP itself
breaks fader rendering: RISK-1**". "Met" would assert substantive satisfaction
that RISK-1 contradicts. "Accepted" records what the gate actually did, and the
Status line directly above the tables carries the FAIL verdict, so the two read
together. This is the honest pairing.

**(g) Terminology — PASS.** All four occurrences reworded, matching finding I1's
own recommended wording verbatim ("re-hosted scenes" / "Retained scene
re-hosting" / "4 retained live scenes re-hosted", `analysis-report.md:80`):
`spec.md:169` NFR-001, `plan.md:25` Constraints, `plan.md:26` Scale/Scope,
`plan.md:119` IC-03 title.

Both WP-declared validation greps are clean:

```
grep -rn "migrat" .../spec.md .../plan.md
→ spec.md:148 only — the Domain Language rule itself, intact:
  - Avoid "migration" for the demo scenes: scenes are not rewritten; the
    shell under them changes.

grep -rn "^\*\*Status\*\*: Draft" (both specs) → empty
```

The rule that forbids the term still names the term it forbids, as required.

### Non-blocking observations on T020

- "deterministic acceptance 25/25 pass" mildly conflates two artifacts: the 25
  is `acceptance-matrix.json`'s criteria count, while
  `deterministic-acceptance.json` records 31 project checks (`resolved: 31,
  executed: 31, status: passed`). Nothing is inflated — both layers passed —
  but the label names the wrong file.
- `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/spec.md:5` still reads
  `**Status**: Draft` though that mission is merged. Same residue class, but
  correctly **out of scope**: the WP restricted the documentation surface to two
  named planning trees. Flagging for a future sweep, not against WP05.

## 7. Independent test results

All run by the reviewer in the lane worktree.

| Command | Result |
|---|---|
| `cargo test --lib` | **629 passed, 0 failed**, 1 ignored |
| `cargo test --test component_composition` | **15 passed, 0 failed** |
| `cargo test --test component_vocabulary` | **11 passed, 0 failed** |
| `cargo test --test webview_projection_shell` | **PASS** — `CREST_ACCEPTANCE webview_projection_shell passed`; T022/T023/T010/T025 all PASS; live layer skipped on `CREST_WEBVIEW_TESTS=1` absent (pre-existing gate, decided before any window attempt) |
| `cargo clippy --all-targets` | **clean**, exit 0, no warnings |

Net delta, `src/` + `tests/`: **+94/−16 = +78** across two files
(`component_gallery_scene.rs` +39/−0 comment-only;
`component_composition.rs` +55/−16 net +39). NFR-003 satisfied — the addition
is narration and coverage, not mechanism.

`git status` is clean of every planted probe.

## Anti-pattern checklist

| Item | Result |
|---|---|
| Scope creep beyond `owned_files` | **PASS** — exactly two files |
| Weakened/deleted assertion | **PASS** — none; coverage tripled |
| Silent carve-out (narrowed needle, omitted source, `#[allow]`) | **PASS** — none; the one omission (stylesheets) is stated with its reason |
| Behavior change smuggled as narration | **PASS** — `+39/−0`, all `//` |
| Closed gate rewritten to read better | **PASS** — no finding/verdict/evidence file touched; the Status line makes the record read *worse* and truer |
| Invented evidence figure | **PASS** — every figure traced to a committed evidence file |
| Test theatre (guard that cannot fail) | **PASS** — 5 planted probes, 4 fired by name, 1 negative control held |
| `kitty-specs/` on a lane branch | **PASS** — none |

## Verdict

**APPROVED.**

The blocking constraint holds: the gallery is intact and I ran it. The scan
extension is real and falsifiable on all three bound sources. `strip_html_comments`
is legitimate scoping — demonstrably not load-bearing for the pass, tightly
scoped to `<!-- -->`, and unable to hide a violation in any inline `<script>` or
`on*` attribute — with one narrow legacy-construct blind spot recorded above and
mitigated by the production CSP. No assertion was weakened. The narration's
central claim about `page_asset` is exactly true. And the documentation edits
make two completed missions' records read *more* honestly, not less: the cutover
spec now carries the FAIL verdict it previously omitted, and the post-hoc leak
bound is explicitly marked as post-hoc before any number appears.
