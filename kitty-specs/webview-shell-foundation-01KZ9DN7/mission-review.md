# Mission Review Report: webview-shell-foundation-01KZ9DN7

**Reviewer**: Claude (post-merge mission review, `spec-kitty-mission-review`)
**Date**: 2026-08-05
**Mission**: `webview-shell-foundation-01KZ9DN7` — Webview Shell Foundation
**Baseline commit**: `aedcafde732a96aa756a71b8fb8d12aa61497255` (meta.json `baseline_merge_commit`)
**HEAD at review**: `2711988`
**Merge target**: `feat/webview-shell-foundation` (mission is `pr_bound`; PR to `main` not yet open)
**WPs reviewed**: WP01–WP06 (all `done`)

Process record verified from `status.events.jsonl` before anything else: all six
WPs moved `planned → claimed → in_progress → for_review → in_review → approved →
done` in exactly **one review cycle each**, with **zero `force: true` events, zero
rejections**, implementer actors (`implementer-ivan`, `frontend-freddy`) distinct
from the reviewer (`reviewer-renata` profile, human verdict `reviewer: cresten`)
on every approval. The approval evidence is unusually deep — the WP01 reviewer
independently confirmed the tauri API-shape claim in vendored source, and the WP06
reviewer re-ran the declared validation both headless and live and re-measured
both NFRs. No self-approval, no gate bypass.

---

## Gate Results

Gates 1–4 of the review skill are `spec-kitty`-repository gates. This project has
no analogue for any of them; the project's own hard gate is the crest-spec
declared deterministic acceptance plus the full suite, run independently below.

### Gate 1 — Contract tests: **N/A**
No `tests/contract/` exists; this project's canonical resources live in the
crest-spec (`C-005` forbids `contracts/` artifacts — their absence is correct,
not a gap).

### Gate 2 — Architectural tests: **N/A**
No `tests/architectural/` exists; the equivalent guarantees are the crest-spec
declared validations, run below.

### Gate 3 — Cross-repo E2E: **N/A**
No cross-repo e2e harness exists and the mission declares no cross-repo behavior.

### Gate 4 — Issue matrix: **N/A**
`spec.md` references no GitHub issues; no matrix was scaffolded, correctly.

### Project hard gate A — Declared deterministic acceptance
- `spec-kitty accept` recorded at commit `fa61229` ("Accept
  webview-shell-foundation-01KZ9DN7", `accepted_by: crestenstclair`,
  2026-08-05T21:30:33Z); artifact `deterministic-acceptance.json` v3.0,
  `passed: true`, 31 checks, crest_spec gate `passed`.
- **This review re-ran the mission's declared validation at HEAD**:
  `cargo test --test webview_projection_shell -- --nocapture` → exit 0,
  marker `CREST_ACCEPTANCE webview_projection_shell passed (skipped: T024 page
  render determinism (DOM layer at 1920x1080 and 1280x800); T026 live layer
  (real-window shutdown parity, NFR-001 projection-to-paint, NFR-002 meter
  soak))` — headless run, T022/T023/T025 executed, live sections skip-listed
  exactly as designed.
- **However, the committed record has a provenance defect and two declared
  validations are red at HEAD** — see DRIFT-1, DRIFT-2, DRIFT-3. Result as an
  evidence artifact: **FAIL** (record does not cover the merged tree; re-running
  the gate at HEAD today would not go green). Result for the code under it:
  independently verified PASS by this review's own runs.

### Project hard gate B — Full test suite
- `cargo test` at HEAD (this review's run): **exit 0**. Lib target: **830 passed
  / 0 failed / 2 ignored** (both ignores deliberate: `write_tokens_css`
  generator, spike dump harness). Every integration target green, including
  `component_vocabulary` (11), `component_composition` (13), all RT-contract
  targets, and the `harness = false` webview target.
- `cargo clippy --all-targets -- -D warnings` at HEAD: **exit 0**.
- `cargo fmt --all -- --check` at HEAD: **exit 1** — one diff,
  `src/shell/webview/window.rs:87` (DRIFT-3). This is declared
  `validation.format`, so the format gate is red on the merged tree.
- `scripts/check_acceptance_matrix_coverage.sh` at HEAD: **FAIL** —
  "webview-shell-foundation-01KZ9DN7: specification declares constraints (C-)
  but the acceptance record grades no C- row" (DRIFT-2). This is declared
  `validation.acceptance_matrix_covers_all_requirement_kinds`.

---

## FR Coverage Matrix

Adequacy legend — **ADEQUATE**: deleting the implementation fails a committed
automated proof (or a recorded live run by an independent reviewer).
**PARTIAL**: real coverage with a disclosed hole. **LIVE-GATED**: the automated
proof runs only under `CREST_WEBVIEW_TESTS=1`; headless runs skip-list it
loudly. **MISSING**: no proof would fail.

| ID | WP (requirement_refs) | Test / evidence | Adequacy | Finding |
|----|----|-----------------|----------|---------|
| FR-001 launch + explicit selection, egui default | WP02, WP05, WP06 | `crest_synth.rs` `the_shell_selection_defaults_to_egui_and_parses_both_explicit_values` (default, both values, dup/invalid/mode-combination rejection); `ShellSelection::default() == Egui` (`mod.rs` test); T024/T026 render the committed page in a real Tauri window (live-gated; executed by implementer and re-run by reviewer, screenshots recorded) | ADEQUATE | — |
| FR-002 authored composition from generated tokens | WP04, WP05, WP06 | T023 byte-freshness of committed `tokens.css` vs `token_export::tokens_css()` (74 properties, drift names its property — mutation-verified both in unit tests and against the committed file); `page.css` verified this review: **0** hex literals, 6 px literals all inside the one documented fader-specimen block the vocabulary does not name; mixer-column geometry and surface split generated from `ViewportDensityPolicy` (analysis C1 resolved, `token_export.rs:197-221`) | ADEQUATE | Analysis C1 resolved as recommended |
| FR-003 input captured Rust-side, full key vocabulary unchanged | WP01, WP02, WP06 | Two-layer by declared posture: (1) committed probe verdict `research/input-capture-probe.md` — NSEvent local monitor, full-vocabulary synthetic sweep through the production `KeyboardInputTranslator`, tao path proven structurally impossible (reviewer independently confirmed `tauri::WindowEvent` has no keyboard variant in vendored source); (2) live reviewer key-drives at WP05/WP06 (System Events, focus T00→T04, level adjust). Automated at HEAD: `input_capture` bijectivity test (every `WindowKey` except `Other` from exactly one keycode — fails if the map is deleted); `window.rs` test asserting index.html and page.js register no `keydown/keyup/keypress` (fails if the page ever takes input) | PARTIAL (by accepted design) | Analysis C2 — no automated key-injection witness — **accepted and recorded** (analysis-report C2 "Acceptable for this mission"; acceptance-matrix FR-003 note "recorded for the successor mission") |
| FR-004 one schema, no fork | WP03, WP06 | T022: production `ProjectionChannel::push` (the identical call site `window.rs:303` uses) byte-identical + canonicalized + structural round-trip vs the projector's own serialization across 3 ascending generations, key surface == `SERIALIZED_PROPERTY_DESCRIPTOR`; runs headless every run. A webview-only struct or trimmed field fails by construction | ADEQUATE | — |
| FR-005 meters, latest-value, loss display-only | WP03, WP06 | `meter_channel` unit tests (single slot, newest-wins, `FrameLost` never fatal, interval derived from named 30 Hz rate); T026 live soak: 29.40 Hz sustained 60 s, thirds equal, 0 lost, page received 100 % (implementer + reviewer runs) | ADEQUATE / LIVE-GATED | RISK-4 (5-minute figure) |
| FR-006 owned shutdown parity | WP02, WP06 | T026 live: real close-button (System Events by unix pid) runs of the shipped binary under **both** shells → self-exit 0, no error output; harness window `run_return == 0` through CloseRequested → Destroyed → Exit. Headless runs have no shutdown proof — disclosed in the skip list | ADEQUATE / LIVE-GATED | — |
| FR-007 typed startup failure, no fallback | WP02, WP06 | T025 headless, every run, on the **shipped binary** as subprocess: `CREST_WEBVIEW_PAGE=/nonexistent` → nonzero self-exit within timeout, typed `PageLoadFailed` display on stderr, zero `eframe/egui/winit` markers on either stream; plus typed-error unit tests in `mod.rs`/`window.rs`. Structural: `EframeGraphicalWindow` is constructed only in the bin's egui/demo arms (grep verified) — no code path reaches it after webview selection or failure | ADEQUATE | — |
| NFR-001 ≤ 50 ms p95 projection-to-paint | WP06 | T026 live: 150 paced reducer edits through production projector + emit path, `crest://painted` generation echo — implementer p95 8.4 ms, reviewer re-measured p95 8.5 ms / max 30.8 ms (both recorded) | ADEQUATE / LIVE-GATED | — |
| NFR-002 30 Hz meters, no queue growth over 5 min | WP03, WP06 | Single-`Option` pending slot bounds queue depth at 1 **by construction** (unit-tested); live soak 60 s at 29.40 Hz, first/last-third rates equal, emit count pace-bounded, 0 lost; 300 s soak exists behind `CREST_WEBVIEW_FULL_SOAK=1` | PARTIAL | RISK-4: the 5-minute duration was never demonstrated on record; 60 s + structural bound accepted with reviewer ruling on the 29.4 Hz quantization face |
| NFR-003 RT callback bounds unchanged | WP01, WP03, WP06 | WP01 `PROBE_RT`: production fixture sounding inside the tauri process, ~48,006 frames/s constant, non_finite=0, no xrun, 3 sessions; transports run on the event thread reading the same accessors the eframe window reads (verified by inspection at review); all RT-contract test targets green at HEAD | PARTIAL | RISK-3: no A/B counter comparison against the egui-shell baseline under the same workload (the literal spec text) was recorded |
| NFR-004 both authored viewports | WP05, WP06 | T024 live: real window seated at 1920×1080 and 1280×800 (viewport width asserted from inside the page), double-render deep-equal, five bands painted, 16 columns × 5 declared structures in order, Inspector `widthPx` ≥ 420/320 floors (analysis A1 resolved: integer computed width) | ADEQUATE / LIVE-GATED | — |
| C-001 boundaries untouched | all | Transports read only the immutable projection / atomic observation snapshot on the event thread; input path is physical → translator → semantic action; suite + clippy green | ADEQUATE | — |
| C-002 page stateless | WP05, WP06 | `render(model)` pure (no Date.now/Math.random, document-order walks — verified by read); T024 double-render determinism; the one carve-out (meter animation) matches the declared exception verbatim and only the meter listener writes it | ADEQUATE | — |
| C-003 egui default + proofs green | WP02, WP06 | Default proven by parse tests; every pre-existing proof target green at HEAD (this review's `cargo test`) | ADEQUATE | — |
| C-004 scope exclusions | all | No PATCH context, no modals, no page→reducer action IPC (production Rust registers **no** event listener; page emits only the painted ack), no egui deletion in the diff | ADEQUATE | — |
| C-005 crest-spec first, no data-model/contracts | — | `adapter.TauriWebviewWindow`, `requirement.webview_projection_shell`, `requirement.serialized_projection_transport`, amended `requirement.selected_egui_stack`, assets and `validation.webview_projection_shell` all declared at `51a329e` (17:03Z), before planning (17:32Z); no `data-model.md`/`contracts/` exist | ADEQUATE | But see DRIFT-2: the acceptance matrix grades no C- rows |

All five recorded analysis findings verified: **C1** resolved in WP04 exactly as
recommended (density-policy geometry generated); **C2** consciously accepted and
recorded twice; **I1** resolved in WP05 (painted ack authored in
`page.js:769-829`, consumed by T026 — no out-of-map edit); **A1** resolved
(`inspector.widthPx` integer, `page.js:712`, asserted ≥ floor in
`webview_projection_shell.rs:960-969`); **T1** needed nothing and got nothing.

---

## Drift Findings

### DRIFT-1: The committed deterministic-acceptance record never exercised the merged mission

**Type**: ACCEPTANCE-EVIDENCE PROVENANCE
**Severity**: HIGH
**Spec reference**: project hard gate (CLAUDE.md: "`spec-kitty accept` runs the
declared deterministic validations and both acceptance layers must pass");
crest-spec `validation.webview_projection_shell`.

**Evidence, all verified in the committed artifacts**:
- `deterministic-acceptance.json` (committed at HEAD `2711988`, "record
  deterministic-acceptance report on the target branch") contains **31** checks;
  the string `webview_projection_shell` appears **zero** times in the 1,861-line
  file. The crest-spec at the accept commit (`fa61229`) declares **32**
  `projectValidations`, `webview_projection_shell` first among them
  (`git show fa61229:.kittify/crest-spec/proof/validations.yaml`).
- The record's `validation.test` (`cargo test --all-targets`, exit 0) stdout
  contains **zero** matches for `token_export|webview::` and no
  `CREST_ACCEPTANCE webview_projection_shell` marker — the tree it compiled did
  not contain the mission implementation. Consistent with `meta.json`
  `accepted_from_commit: 0d31f3f`, a pre-merge target-branch commit at which
  `git ls-tree` shows no `tests/webview_projection_shell.rs` (the code lived on
  the coordination branch, whose own crest-spec — verified in
  `.worktrees/webview-shell-foundation-01KZ9DN7-coord` — contains **zero**
  occurrences of the validation declaration).
- The known accept-before-merge circularity procedure for this repo ends with
  "merge → re-run accept → green record in deterministic-acceptance.json". No
  post-merge re-accept is recorded; the committed record is the pre-merge run
  under a commit title implying it covers the target branch.

**Analysis**: the mission's own declared validation was structurally invisible to
the recorded accept run — the one artifact that exists to prove the declared
validations passed proves them for a tree without this mission in it. The code
itself is fine: the WP06 reviewer ran the declared validation headless **and**
live in the lane-f worktree with recorded NFR measurements (approval event
`01KZ9K...` in `status.events.jsonl`), and this review re-ran it at HEAD (pass,
headless skip list). The defect is confined to the acceptance record — but it is
exactly the failure mode acceptance records exist to prevent, and re-running
accept at HEAD today would **not** go green (DRIFT-2, DRIFT-3). Remediation is
small: fix DRIFT-2/DRIFT-3, re-run `spec-kitty accept` at HEAD, and commit the
32-check green record.

---

### DRIFT-2: The acceptance matrix grades no constraint rows and the declared coverage validation fails at HEAD

**Type**: RECORD OMISSION / RED DECLARED VALIDATION
**Severity**: MEDIUM (subsumed into DRIFT-1's remediation)
**Spec reference**: spec.md Constraints C-001…C-005; crest-spec
`validation.acceptance_matrix_covers_all_requirement_kinds`.

**Evidence**: `acceptance-matrix.json` (identical on root and the coord
worktree, verified by canonical diff) carries exactly 11 criteria — FR-001…007
and NFR-001…004 — and no C- row. Run at HEAD by this review,
`scripts/check_acceptance_matrix_coverage.sh` exits nonzero:
*"webview-shell-foundation-01KZ9DN7: specification declares constraints (C-) but
the acceptance record grades no C- row."*

**Analysis**: this guard was built after "two consecutive missions shipped with
an acceptance matrix that graded functional requirements only" (script header) —
this is the third. It did not fire at accept time because its scope opens only
once `deterministic-acceptance.json` exists, which happened at HEAD, after
accept. The constraint evidence itself exists scattered through the WP reviews
(this review graded C-001…C-005 ADEQUATE above); the work is transcription, not
proof.

---

### DRIFT-3: `validation.format` is red at HEAD — the known rustfmt drift in window.rs was left to an owner who never existed

**Type**: RED DECLARED VALIDATION / SHARED-FILE OWNERSHIP RESIDUE
**Severity**: LOW (one mechanical hunk)
**Spec reference**: crest-spec `validation.format` (`cargo fmt --all -- --check`).

**Evidence**: `cargo fmt --all -- --check` at HEAD exits 1 with exactly one
diff, `src/shell/webview/window.rs:87` (the `"/tokens.css"` match arm exceeding
line width). The WP06 approval note records it verbatim: *"WP05's rustfmt drift
in window.rs untouched, left to its owner"* — WP05 (page assets) had edited
WP02/WP03-owned `window.rs` out-of-map to embed the split assets, WP06's scope
note forbade production edits, and no WP after WP06 exists. The pre-merge accept
record shows `validation.format: passed` because it ran on the tree without the
drift (DRIFT-1).

---

### DRIFT-4: The painted-ack → `ShellFrameObservation` forwarding was deferred down a chain that ended without an owner

**Type**: DOCUMENTED-DEFERRAL TERMINATION / STALE COMMENT
**Severity**: LOW (zero production impact today)

**Evidence**: `window.rs:183-190` accepts and discards the
`FrameObservationCallback` (`let _on_frame = on_frame;`) with a comment saying
forwarding *"belongs to the acceptance work package that measures it (WP06
T026)."* WP06 shipped with *"this WP changes no production code"* (approval
note) and measured NFR-001 through its own harness listener instead. The WP02
reviewer ruled the withholding correct for the placeholder window; the WP05
reviewer recorded *"Rust forwarding honestly left at `_on_frame` for WP06
T026"*; WP06 left it. At HEAD nobody forwards.

**Analysis**: impact today is nil — in the only mode the webview shell can run
(standalone; the bin rejects `--shell webview` with every demo/smoke mode),
`StandaloneApplication` wires `on_frame` to a no-op
(`standalone_application.rs:921`); the sole real consumer is the demo-live path
(`:1062`), which is egui-only by C-004. But the page already emits everything
needed (post-paint region bounds and labels keyed by `ShellRegionId` names), the
comment now points at a completed WP, and the successor mission that lets the
webview shell run the paced live demo **must** land this forwarding first.
Record the owner explicitly instead of leaving a pointer to the past.

---

## Risk Findings

### RISK-1: A page-side render exception silently freezes the projection surface

**Severity**: LOW-MEDIUM · **Trigger**: a defect in `page.js` `render()` throwing
mid-listener (schema surprises are excluded by FR-004's proof; this requires a
page bug).

**Trace**: `page.js:818-829` — the projection listener calls `render(latestModel)`
directly; an exception unwinds into the tauri event dispatch, the painted ack
never fires, and nothing reaches Rust: production `window.rs` installs no
`crest://painted` listener (DRIFT-4) and no page-error channel. The generation
gate advanced on **emit**, not paint (`projection_channel.rs:161`), so the
document is not re-pushed. Consequence: keys, reducer, and audio keep working
while the screen silently shows the last good frame. The egui shell cannot fail
this way (paint failures surface through the frame-observation error path).
Mitigation exists in the design (the painted ack is exactly the liveness signal;
DRIFT-4's forwarding closes this); the page's purity proof (T024) and the
schema-fidelity proof (T022) make the trigger genuinely unlikely. Non-blocking;
fold into DRIFT-4's successor-mission item.

### RISK-2: If `window.close()` fails on the owned-close path, the loop idles forever with ticks stopped

**Severity**: LOW · **Trigger**: `handle.get_webview_window("main")` returning a
window whose `close()` errors (or `None`) on the false-tick (`window.rs:328-334`)
or transport-error (`:309-313`) paths while the OS window survives.

**Trace**: both paths set `close_requested = true` first, so `MainEventsCleared`
returns early forever after; if the close silently fails (`let _ =`), no code
path retries and `run_return` never exits — a hang with a live window and a dead
UI, and on the transport-error path the recorded typed error is never surfaced.
In practice `close()` on a live window does not fail, and the Destroyed handler
covers the race where the window is already gone (close_requested set, tauri's
default ExitRequested ends the loop). Worth a retry-or-panic on close failure
whenever this file is next touched.

### RISK-3: NFR-003's literal A/B measurement was never taken

**Severity**: LOW · The spec asks for "measured bounds … unchanged from the egui
shell baseline under the same workload". Delivered evidence (probe RT health
under tauri+cpal coexistence, event-thread-only transports by inspection, all RT
contract targets green) supports the conclusion structurally, but no
same-workload counter comparison between shells exists on record — partly
because the paced-demo workload cannot run under `--shell webview` at all
(C-004's bin rejection). Honest, disclosed in the matrix's evidence text; the
successor mission that enables the demo through the webview shell should take
the literal measurement.

### RISK-4: NFR-002's five-minute soak exists but was never run on record

**Severity**: LOW · The declared validation's default soak is 60 s; the 300 s
soak is one env var away (`CREST_WEBVIEW_FULL_SOAK=1`) and is printed in every
run's configuration line, but no 300 s execution is recorded anywhere. The
structural bound (single `Option` slot) plus 60 s sustainment with equal thirds
makes queue growth implausible; still, the spec says five minutes and the record
shows one.

### RISK-5: T024/T026 live-gating — judged honest

**Severity**: INFORMATIONAL (recorded as a judgement, not a defect) · The gate is
decided exactly once in `main()` from `CREST_WEBVIEW_TESTS` alone, before any
window attempt; a failure inside an admitted live section panics or
`exit(101)`s **before** the marker prints, so a live failure can never demote to
a skip; headless runs print per-section `CREST_WEBVIEW_SKIP` lines and the
acceptance marker itself carries the skip list. The residual honesty gap is that
the declared assertion (`stdout-contains … passed`) is satisfiable by the
headless subset — the marker does not require `(skipped: none)` — so the
deterministic gate alone never proves the live layer; the live proof lives in
the WP06 implementer and reviewer runs (both recorded with NFR numbers). That
split is exactly what the spec's two-layer testing posture declares. Verdict:
the design keeps the gap loud rather than hiding it.

---

## Silent Failure Candidates

Scanned all of `src/shell/webview/` and `webview-page/page.js` for
swallowed-error patterns (`unwrap_or_default`, `.ok()`, `let _ =`, bare `catch`).

| Location | Pattern | Assessment |
|----------|---------|------------|
| `window.rs:319` | `let _ = meter_channel.emit_if_due(...)` — `FrameLost` dropped | **Deliberate and declared** (loss degrades display only, crest-spec rule; unit-tested loss semantics). Note: production keeps no loss counter, so sustained meter loss is invisible outside the live harness — acceptable for render evidence |
| `window.rs:311, 332` | `let _ = window.close()` | See RISK-2 — a failed close hangs rather than corrupts; LOW |
| `window.rs:240` | `let _ = window.set_focus()` | Cosmetic; capture is monitor-based, not focus-based |
| `window.rs:340` | `let _ = waker.join()` | Waker thread body cannot panic meaningfully; benign |
| `projection_channel.rs:155-161` | gate advances only on successful emit | The **opposite** of a silent drop: a failed emit retries next tick (unit-tested `a_failed_emit_is_typed_and_leaves_the_gate_unmoved`) |
| `page.js:814-816` | `attachTransports` returns silently without `window.__TAURI__` | Declared headless-harness mode; in production the window always injects the global (`withGlobalTauri`) |
| `page.js` | no `catch` anywhere | No swallowed exceptions; the flip side is RISK-1 |

No `catch`-and-return-empty, no `unwrap_or_default`, no `.ok()` discards in any
production file this mission added. Every error type is a typed enum carrying
its cause (`WebviewShellError`, `ProjectionChannelError`, `InputCaptureError`,
`TokenDrift`).

---

## Security Notes

| Area | Finding | Risk class | Assessment |
|------|---------|-----------|------------|
| `crest://` protocol handler | `page_asset()` (`window.rs:87-102`) is a closed match over 9 fixed paths; everything else is 404 with an empty body. No filesystem access at request time (all assets embedded at compile time). Traversal probed by committed test: `page_asset("/../Cargo.toml") == None` (`window.rs:421`) plus full-closure test over every declared asset | PATH-TRAVERSAL | **Constrained by construction and tested.** The claim "traversal-tested" is true |
| `CREST_WEBVIEW_PAGE` | Production-reachable env override serving an arbitrary local file as the index document (`window.rs:135-145,174-179`); read **once** before any window exists — no TOCTOU (the read is the use); supporting assets stay embedded | LOCAL OVERRIDE | Documented as an internal test seam (`mod.rs:21-28`), consumed by T025 against the shipped binary. Anyone who can set the env already controls the process; not an escalation. Successor could gate it under `cfg(debug_assertions)` for hygiene |
| `tauri.conf.json` ACL | One capability, `windows: ["main"]`, permissions `["core:event:default"]` only — exactly the listen/emit surface the page uses (`crest://projection`, `crest://meters` in; `crest://painted` out). No fs/shell/http/dialog plugin permissions exist to grant. `withGlobalTauri: true` is required (no bundler). Production Rust registers **no** event listener, so the page→Rust surface is nil | ACL SCOPE | **Minimal grant, verified against actual page usage** |
| CSP | `"csp": null` | HARDENING (LOW) | All content resolves over `crest://` embedded assets and every document-derived string is `escapeHtml`-ed; there is no remote URL anywhere in the page. A restrictive CSP would be free defense-in-depth; note for the successor mission |
| NSEvent monitor | `addLocalMonitorForEventsMatchingMask(KeyDown\|KeyUp)` (`input_capture.rs:193-198`) — a **local** (in-process) monitor, not the global variant: it observes only events delivered to this process, needs no accessibility grant, passes events through unmodified, and is removed on handle drop | INPUT CAPTURE SCOPE | In-process only, confirmed in code |
| Network | Zero network surface added: no fetch/XHR/WebSocket in any page asset (grepped: 0 hits), no HTTP client crates added, page cannot load remote content (custom protocol + no remote URLs). The tauri dependency's IPC is window-local | NETWORK | **Zero network surface, verified** |
| Subprocess (tests only) | T025/T026 spawn the shipped binary, `kill -9 <pid>`, `screencapture -x`, `osascript -e` — all arg-vector `Command`s, no shell interpolation; the only interpolated value in the osascript source is the numeric child pid. Live-gated where they touch the window server | SUBPROCESS | Test-only, no injection vector |

No blocking security findings.

---

## Final Verdict

**FAIL** — on DRIFT-1 (with DRIFT-2/DRIFT-3 as its concrete red gates), and on
that alone. Remediation is small and entirely in the records layer; no
production code beyond one rustfmt hunk needs to change.

### Verdict rationale

The implementation itself is faithful and unusually well proven. Every FR traces
to evidence that would fail if the implementation were deleted; the no-fallback
invariant holds structurally (the eframe window is constructible only from the
bin's egui/demo arms, and T025 proves the shipped binary self-exits typed with
no eframe markers); the one-schema rule is proven byte-for-byte through the
production emit path; the token vocabulary is generated, injective,
byte-fresh-checked, and mutation-verified; the page is a pure render over the
canonical document with its statelessness and anatomy asserted from the painted
DOM; input stays Rust-side with the page grepped clean of key handlers by a
committed test; and the live layer — real window, real keys, real close button,
both shells, both NFRs — was executed twice, by the implementer and
independently by the reviewer, with numbers recorded in the approval events.
All five analysis findings were resolved or consciously accepted with records.
The process record is clean: six WPs, six single-cycle approvals, zero force
events, reviewer distinct from implementer throughout.

The verdict is nonetheless FAIL because the project's own hard gate is not
honestly satisfied on the merged tree **right now**. The committed
`deterministic-acceptance.json` — the artifact whose entire purpose is to prove
the declared validations passed — was recorded from a pre-merge tree that
contained neither the mission's code nor (in the runner's view) the mission's
own declared validation: 31 of 32 declared checks, zero occurrences of
`webview_projection_shell`, zero mission tests in its `cargo test --all-targets`
output, under a commit titled "record deterministic-acceptance report on the
target branch". And re-running the gate at HEAD today does not pass:
`validation.format` fails on one known, recorded, never-owned rustfmt hunk, and
`validation.acceptance_matrix_covers_all_requirement_kinds` fails because the
matrix grades no constraint rows — the third mission in a row to omit a declared
requirement kind, per the guard's own header. Each item is trivially fixable;
none was surfaced by the implementation team, and a review that certified
"releasable" over a red hard gate would repeat the exact record-over-reality
failure DRIFT-1 documents.

**Path to PASS WITH NOTES** (expected to be under an hour, all records-layer):
1. `cargo fmt` — commit the one hunk in `src/shell/webview/window.rs` (DRIFT-3).
2. Grade C-001…C-005 rows in `acceptance-matrix.json` (root and coord copies) —
   the evidence is already in the WP review notes and this report (DRIFT-2).
3. Re-run `spec-kitty accept` at HEAD and commit the resulting 32-check green
   `deterministic-acceptance.json` (DRIFT-1). This review's own runs predict
   green once 1–2 land: declared webview validation passes headless, full suite
   830/0/2 lib + all integration targets, clippy clean.

### Open items (non-blocking, in priority order)

1. **DRIFT-4 / RISK-1** — give the painted-ack → `ShellFrameObservation`
   forwarding an explicit owner (the successor mission that runs the demo
   through the webview shell needs it first); update the stale `window.rs:183`
   comment that still points at WP06.
2. **RISK-3** — take the literal same-workload RT A/B measurement when the
   paced demo can run under `--shell webview`.
3. **RISK-4** — run the 300 s soak once (`CREST_WEBVIEW_FULL_SOAK=1`) and record
   it, or amend NFR-002's record to the 60 s + structural-bound basis.
4. **RISK-2** — retry-or-surface on `window.close()` failure next time
   `window.rs` is touched.
5. **Security hardening** — set a restrictive CSP; consider
   `cfg(debug_assertions)` on `CREST_WEBVIEW_PAGE`.
6. **Housekeeping** — decide the retention of the WP01 disposable probe binary
   (`src/bin/webview_input_probe.rs`, 484 lines, documented evidence artifact
   with the human hardware-sweep instruction; harmless but permanent as a
   `[[bin]]`); consider migrating the six fader-specimen px values from
   `page.css` into the authored vocabulary so FR-002's "single source" covers
   them too.

---

## Retrospective Reminder

The canonical post-merge sequence is: **mission review → author or verify
retrospective → surface findings**.

The retrospective record **exists**:
`kitty-specs/webview-shell-foundation-01KZ9DN7/retrospective.yaml` (135 lines,
commit `44535ae`, capture event present as the final `status.events.jsonl`
entry, 2026-08-05T21:32:10Z, generator `spec-kitty-generator`). Nothing to
author. Two notes on its content:
- Its gap **g-001 ("data-model.md absent") is a generator false positive** for
  this project: C-005 and CLAUDE.md forbid `data-model.md` as a crest-spec fork;
  its absence is correct. Do not "fix" it.
- The merge output referenced
  `.kittify/missions/01KZ9DN7SDDFNYYYVC74XT21QG/retrospective.yaml`; no such
  path exists — the canonical copy is the kitty-specs one above.

To surface findings: `spec-kitty retrospect summary`, then
`spec-kitty agent retrospect synthesize --mission webview-shell-foundation-01KZ9DN7`
(dry-run; `--apply` mutates).
