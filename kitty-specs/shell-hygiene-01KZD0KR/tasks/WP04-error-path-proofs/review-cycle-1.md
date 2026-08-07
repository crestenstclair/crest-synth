---
wp_id: WP04
reviewer_agent: reviewer-renata
cycle_number: 1
mission_slug: shell-hygiene-01KZD0KR
verdict: approved
reviewed_at: "2026-08-07T04:47:01Z"
affected_files:
  - path: tests/webview_projection_shell.rs
---

# WP04 — Error-path proofs under the production path — Review Cycle 1

**Verdict: APPROVED.**

Lane commit `e6689df` on `kitty/mission-shell-hygiene-01KZD0KR-lane-d` (lane HEAD
`bbfa120`, which adds only `kitty-specs/` status files on top). Sole source change:
`tests/webview_projection_shell.rs`, +856/−7.

Every claim was independently reproduced. All three proofs were made to fail on
demand and each failure took the suite down with a nonzero exit — that is the
strongest available answer to "is this proof wired in", and it holds for T013,
T014 and T015 alike.

## 1. Probe 1 (FR-001 / RISK-3) — T013 genuinely dies without the exit edge

Reverted `handle.exit(REQUESTED_EXIT_CODE)` on the `CloseOutcome::RetriesExhausted`
path in `src/shell/webview/window.rs:444-446` to the pre-WP01 shape (record the
typed error, return normally, no exit edge).

**Unit layer first — this is the point of the whole WP:**

```
test result: ok. 640 passed; 0 failed; 1 ignored
```

The unit suite is *fully green with the exit edge deleted*. This independently
reproduces WP01's reviewer's finding. Nothing below the acceptance layer guards
RISK-3.

**T013 under the same probe** — exit `101`, signature verbatim as claimed:

```
thread 'main' panicked at tests/webview_projection_shell.rs:1029:13:
T013 forced double-close failure with a recorded render failure: the process did
not exit within 120s — a shell kept a window running instead of failing typed
```

Blast radius was surgical — in the same probe run `T012 negative control`,
`T012 forced first-render throw` and both `T026 shutdown parity` sections still
PASSED. Only T013 died. T013 is specifically and uniquely sensitive to the exit
edge, and it is the sole regression guard on the mission's headline fix.

`src/shell/webview/window.rs` restored and verified byte-identical (sha256), index
and worktree both clean.

## 2. Probe 2 (FR-003) — the negative controls are genuinely insensitive

This was the subtlest claim in the mission. It holds.

**2a — validation bypassed, guards active (expect death):** commented out
`verify_ack_identity(&ack, &retired.identity, generation)?` in the superseded-late
branch of `ProjectionChannel::forward_ack`. T014 fails at the first mismatch case:

```
T014 evicted/stateHash: a superseded-late ack whose stateHash is not a verbatim
copy must be typed-rejected as PaintedAckError::IdentityMismatch;
got Ok(SupersededLate { generation: 3 })
```

**2b — same bypass, mismatch guards neutralized (expect controls to survive):**
neutralized all five `expect_identity_mismatch` call sites plus the
`inside_window > 0` window-entry guard. T014 **PASSES**:

```
T014 superseded-late ack identity: PASS (... past the retained window
(0 further retirements) ...)
```

**Verdict on the controls: genuinely independent of the mechanism.** What still
passed under the bypass is the whole non-mismatch substance of T014 — the
capacity-eviction and drain structural assertions (`in_flight_documents()`
transitions), the in-flight `Observation` correlation, the two well-formed
`SupersededLate` negative controls on *both* retirement paths, and both
beyond-window lost-frame cases. None of them is coupled to the comparison they
are supposed to be independent of.

Worth recording: the `(0 further retirements)` vs. the healthy `(13 further
retirements)` is itself a tell. T014 carries a *third* guard beyond the mismatch
assertions — `assert!(inside_window > 0, "…or the mismatch cases above proved
nothing")` — which independently detects a bypassed comparison. I had to
neutralize it explicitly to get 2b to pass. That is better than the WP claimed.

Both files restored byte-identical.

## 3. Probe 3 — T015 is not inert, and is sharper than reported

Rewrote the `stateHash` of **exactly one** healthy ack (the first the audit ever
forwards). T015 fires:

```
live section FAILED: T015 negative control failed: the production
ProjectionChannel rejected 1 painted ack(s) that healthy live sections produced —
WP02's validation rejected honest post-paint evidence (NFR-001 product behavior
change): T026 NFR-001 paced reducer edits: generation 2: painted ack for
generation 2 does not copy stateHash verbatim from its document
```

Exit 101. `T012 negative control` still PASSED in the same run — the two controls
are independent.

Note the count: the WP reported `rejected 153 painted ack(s)`, which corresponds to
corrupting the field on *every* forwarded ack. Corrupting a single ack yields
`rejected 1` and still fires. That is a strictly stronger result than reported, not
a discrepancy against the WP.

**Acks are genuinely real, not synthesized.** `audit_arrived_acks` reads the
`PaintedAcks` log the real page fills over `crest://painted`, re-serializes, and
feeds it to the production `ProjectionChannel::forward_ack` on the *same channel
that pushed the document*, interleaved inside the paced loop rather than replayed
in a batch afterwards. Exclusion of non-healthy acks is by construction (a
per-section `pushed: HashSet<u64>` plus a forward-only cursor), never by
subtraction — the discipline the task required.

**Inertness guard present** at `force_page_failures`:
`if ack_audit.forwarded == 0 || ack_audit.observations == 0 { return Err(...) }`.

Observed across three healthy live runs, identically: **153 forwarded, 153
observations, 0 superseded-late, 0 rejections, in-flight peak 2 of 8** (150 paced +
3 PATCH).

## 4. Wiring — every proof is on the run path

Proven empirically rather than by inspection: each of the three was independently
made to fail, and each failure produced a nonzero exit from the suite.

| Proof | Entry point | Failure mode observed |
|---|---|---|
| T014 | `main()` → `prove_superseded_late_ack_identity()` (unconditional, above the gate) | panic, exit 101 (probe 2a) |
| T013 | `main()` → `run_live_sections()` → `prove_forced_double_close_failure_on_the_shipped_binary()` | panic, exit 101 (probe 1) |
| T015 | `drive_live_window()` → 3× `audit_arrived_acks` → `force_page_failures(&…, &ack_audit)` → `Err` | `live section FAILED`, exit 101 (probe 3) |

## 5. NFR-002 — nothing loosened

**Skip list:** exactly one `+` line (the T013 entry), **zero `-` lines**. All five
pre-existing entries are diff context lines, byte-identical. Confirmed against the
hunk directly. T014 correctly adds no entry — it is headless by construction (the
channel takes its emit as a closure, so no window is involved), and it is placed
with the unconditional sections above the gate, so it runs in *both* modes.

**All 7 removed lines** accounted for, none an assertion:

1. the `projection_channel` import line (widened to 2 lines for the new symbols);
2-3. two comment lines, *reworded to be stronger* — "zero render-errors" became
     "zero render-errors AND zero ack rejections";
4. the `force_page_failures(...)` call site, gaining the `&ack_audit` argument;
5-7. three lines absorbed into the extracted `forced_throw_page_variant` helper.

`assert_ne!(variant, committed, "the variant must differ from the page")` survives
verbatim as a context line. No baseline, threshold, or assertion was touched.
Additions to existing live sections are observations only (`HashSet` inserts,
cursor tracking, audit calls); no existing section's assertions changed.

## 6. The two flagged refactors

**(b) `forced_throw_page_variant` — clean.** Correct. The two callers differ *only*
in the evidence filename (`wp03-t012-…` / `wp04-t013-…`); the variant content is
produced by one deterministic body from the one committed `index.html`, so T012 and
T013 cannot drift onto different pages — which is exactly why the pair is a
controlled comparison. T012's own behavior is unchanged (same filename it always
wrote, same assertion) and it passed in all five live runs.

**(a) `PAINTED_ACK_IDENTITY_FIELDS` — one inaccurate doc comment (non-blocking).**
The constant is defined at `tests/webview_projection_shell.rs:185` and used in
**exactly one place**, line 1283 inside `painted_ack_for` (T014's helper). It is
**not** used by the pre-existing WP01 ack-identity section, which still carries its
own inline six-field array at lines ~2802-2809. The constant's doc comment claims
otherwise:

> "One list in this file: the WP01 ack-identity section and the T014 corrupted-ack
> section must not be able to disagree about which fields are identity."

That is not true as written — there are two lists in this file and they *can*
disagree.

Mitigating, and why this does not block:

- All **three** lists agree today, six fields in identical order: the production
  `ACK_IDENTITY_FIELDS` (`projection_channel.rs:139`), the new test constant, and
  the WP01 inline array — `generation, stateHash, context, activeSurface,
  focusPath, interactionMode`.
- The WP01 section's assertion is **unchanged** (no removed line anywhere in that
  region), so no proof was weakened. NFR-002 is intact.
- The defect is a comment overstating what the code does, not a behavioral gap.

**Recommendation for the mission owner (post-merge, not a WP04 gate):** either
point the WP01 section's loop at `PAINTED_ACK_IDENTITY_FIELDS` — a one-line change
that makes the comment true — or soften the comment. Rejecting a fully-falsified WP
over one sentence would be disproportionate; leaving it unrecorded would not.

## 7. Release guard — verified

`prove_forced_double_close_failure_on_the_shipped_binary` opens with:

```rust
#[cfg(not(debug_assertions))]
panic!(
    "T013 needs the debug-only {CLOSE_FAILURE_SEAM_ENV} seam, which a release build \
     compiles out of the shipped binary: this section cannot run here and must not \
     report a pass"
);
```

Correct placement (first statement) and correct polarity: the `test` profile
inherits `debug_assertions` from `dev`, so the guard is inert under `cargo test`
and fires under `cargo test --release`, where WP01's seam does not exist. The task
permitted "skip loudly or fail to build" — this panics loudly. Verified by
inspection; not compiled in release (a release build of the Tauri dep graph was not
worth the cycles, and either outcome — panic or build failure — satisfies the
requirement).

## 8. The transient soak failure — assessed, not WP04's

I reproduced it on my **first** live run:

```
CREST_WEBVIEW_NFR002 soak configuration: running 60s
live section FAILED: timed out waiting for harness phase "meters"
```

I did **not** take this on the implementer's word. My tally:

| Build | Live runs | Soak result |
|---|---|---|
| WP04 (lane HEAD) | 5 | 4 pass, 1 fail (the first run, immediately after a clippy compile) |
| Base test file, same src / machine / target dir | 2 | 2 pass |

**Assessment: genuinely unrelated to WP04.** Grounds, in order of weight:

1. **Mechanism.** The failure is the post-soak `window.eval` → `receive_phase("meters", 10s)`
   round trip at `tests/webview_projection_shell.rs:2931`, in a section
   (lines 2846-2945) that WP04's diff does not touch at all. WP04 adds **zero**
   page-side work and **zero** additional IPC traffic. Its three
   `audit_arrived_acks` insertions are Rust-side only (clone, serialize,
   `forward_ack`), complete before the soak begins, and queue nothing for the page.
   There is no path by which they starve the WKWebView IPC listener 60 seconds
   later.
2. **Measurement points the other way.** The base run's soak `max gap` was
   **42.0ms**; every WP04 run measured **34.8-35.0ms**. If WP04 had added timing
   pressure this is where it would show, and it does not.
3. **The soak passed on three consecutive WP04-code runs across three different
   builds** (unmodified, probe-1 src, probe-3 tests).

I am recording this honestly: 2/2 base passes against 1/5 WP04 failures is not
statistically distinguishing on its own, and I lean on the mechanism argument to
close it. This is a **pre-existing, load-sensitive acceptance proof** — roughly a
1-in-5 flake on a loaded machine — and it belongs on the mission's radar as a
follow-up, not on WP04's ledger. Nothing about it was introduced or worsened here.

## 9. Independent test results

All run in the lane worktree at `bbfa120`, every live invocation explicitly bounded.

| Run | Result |
|---|---|
| `cargo test --lib` | **640 passed**, 0 failed, 1 ignored |
| `cargo test --test webview_projection_shell --test component_vocabulary --test component_composition` (headless) | **green** — 15 + 11 + suite; T014 PASS with 13 further retirements; skip list = 5 pre-existing + 1 new T013 |
| `cargo clippy --all-targets` | **clean**, zero warnings |
| Live run 1 | FAIL — pre-existing "meters" soak flake (see §8) |
| Live run 2 | **PASS, `skipped: none`**, p95 **8.7ms** |
| Live run 3 (final, files verified byte-identical) | **PASS, `skipped: none`**, p95 **8.3ms** |

Final live run tail:

```
CREST_WEBVIEW_NFR001 projection-to-paint over 150 paced reducer edits:
  p50=7.7ms p95=8.3ms max=8.9ms (threshold p95 <= 50ms)
T015 ack-rejection negative control: PASS (153 real painted acks ... 153 became
  observations, 0 lost late frames, 0 rejected; in-flight peaked at 2 of 8)
T013 forced double-close failure (prior error recorded): PASS (exit=Some(1) after
  651.1ms ... recorded PageRenderFailed surfaced, second-recorded WindowClose did not)
T013 forced double-close failure (no prior error): PASS (exit=Some(1) after 61.2s;
  scene completed, both closes forced to fail, typed WindowClose surfaced verbatim)
CREST_ACCEPTANCE webview_projection_shell passed (skipped: none)
```

NFR-001 latency bar held with ~6x margin on every run.

## 10. Probe hygiene

All probe edits reverted. sha256 of `src/shell/webview/window.rs`,
`src/shell/webview/projection_channel.rs` and `tests/webview_projection_shell.rs`
verified byte-identical to the reviewed commit; `git diff HEAD` empty; index clean
(one intermediate `git checkout <ref> -- <path>` staged the base file during the
§8 A/B and was fully unwound). Only untracked residue is `.spec-kitty/` tooling
state. No probe committed.

Known and not WP04's: three pre-existing `cargo fmt --check` diffs at
`src/shell/webview/window.rs:331` and `tests/webview_projection_shell.rs:664` and
`:3242`. Confirmed present, confirmed pre-existing, mission owner lands them
post-merge.

## Definition of Done

- [x] Live forced-double-close section asserts bounded termination, nonzero exit,
      and typed `PageRenderFailed` precedence; plus the no-prior-error
      `WindowClose` case
- [x] Same production protocol seam; exactly one new honest-skip entry; no existing
      entry widened or reworded
- [x] Headless corrupted-ack section: 5 rewritten/omitted fields as
      `IdentityMismatch` across **both** retirement paths, well-formed control,
      beyond-window lost-frame case
- [x] Suite-wide zero-ack-rejection control, placed last, mirroring the
      render-error control, with an inertness guard
- [x] `skipped: none`; p95 within 50 ms
- [x] All three probes independently reproduced by the reviewer; no probe committed;
      tree clean
- [x] No file outside `tests/webview_projection_shell.rs` modified

## Non-blocking follow-ups for the mission owner

1. `PAINTED_ACK_IDENTITY_FIELDS` doc comment overstates its reach (§6a) — make it
   true with a one-line change, or soften the sentence.
2. The `"meters"` harness-phase soak is a ~1-in-5 load-sensitive flake (§8),
   pre-existing and mission-wide. Worth a bound review or a retry, separately.
3. Three pre-existing `cargo fmt --check` diffs, already owned post-merge.
