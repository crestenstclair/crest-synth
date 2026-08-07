# Adversarial Review: shell-hygiene-01KZD0KR mission diff

**Date:** 2026-08-06 · **Mode:** commit range `32cb838..HEAD`, scoped to `src/` and `tests/`
**Files:** 9 · **Raised:** 23 findings (6 blind hunters) · **Merged to verification:** 13 · **Survived skeptic:** 3 · **Survived at ≥7/10 (reportable):** 2

Pipeline run per the `adversarial-review` contract: six hunters spawned blind and in
parallel in a single message; findings merged and deduplicated; every surviving
finding sent to `finding-skeptic` batched by file, prompted with the kill mandate.
Skeptics were constrained to static verification (`rg`, `git show`, Read) because a
live acceptance run held the cargo build lock; every claim they flagged as needing
execution was subsequently executed by the orchestrator and is recorded below.

## Hunter yield

| Hunter | Raised |
|---|---|
| bloater-hunter | 8 |
| oo-abuser-hunter | 0 (explicit abstention) |
| change-preventer-hunter | 3 |
| dispensable-hunter | 5 |
| coupler-hunter | 3 |
| clean-code-reviewer | 4 |

Cross-hunter agreement (the pipeline's strongest true-positive signal) clustered on
three locations: the orphaned `Condvar` in `frame_stream.rs` (3 hunters), the
close-failure seam name triplication (3 hunters), and the identity-field list
duplication (2 hunters).

---

## Critical

None.

---

## Major

### Dead synchronization primitive — `src/shell/webview/frame_stream.rs:39,117-121,138,152-154`

**Skeptic: CONFIRMED (9/10)** — four independent kill attempts failed; `rg` over the
crate returns the import, the field, and the `notify_all()` and **zero** `wait`/
`wait_timeout` call sites.

Evidence:

```rust
struct StreamShared {
    recent: Mutex<VecDeque<ShellFrameObservation>>,
    arrived: Condvar,
}
...
    /// Records one forwarded observation and wakes every waiting consumer.
...
        drop(recent);
        self.shared.arrived.notify_all();
```

WP03 deleted `await_qualifying`, the module's only waiter, and left the notifier, the
field, the lock-ordering `drop(recent)`, and the `Condvar` import standing. The
`record` doc still promises it "wakes every waiting consumer" — a claim no code path
can now honor. The compiler cannot flag it because `notify_all()` counts as a use.

**Harm:** every forwarded observation pays a wake for nobody on the window's event
thread, and a maintainer who needs a blocking consumer reads this as "blocking is
already half-supported here" and writes a `wait()` against a primitive whose parking
contract (the "never call this on the window's event thread" warning) was deleted
along with `await_qualifying`.

**Suggested technique:** Remove Dead Code — drop the field, the `notify_all()`, the
explicit `drop`, and the import; correct the `record` doc. *(Recorded, not applied.)*

**FR exposure:** FR-005 requires the retired items gone "with no dangling reference."
This is the dangling reference. Folded into the mission review as **DRIFT-1**.

### Dead `impl Display for FrameExpectation` — `src/shell/webview/frame_stream.rs:107-115`

**Skeptic: CONFIRMED (7/10)** — at the base commit the impl's sole consumer was the
`{expectation}` interpolation inside `impl Display for FrameAwaitError`, which this
change deleted. `rg` for any formatting of a `FrameExpectation` returns zero hits
repo-wide; all five surviving construction sites pass it to `poll` and never format it.

Confidence held at 7 rather than 9 because a `Display` impl on a `pub` type carries a
weak public-API defense — weak here because the crate is unpublished at `0.1.0` and
its only consumers are in-repo, all enumerated.

**Suggested technique:** Remove Dead Code. *(Recorded, not applied.)*

**FR exposure:** same as above — folded into **DRIFT-1**.

---

## Minor

Reported as a count per the mission-review scope discipline. One finding survived
the skeptic below the 7/10 reporting threshold:

- **Long Parameter List — `tests/webview_projection_shell.rs:1679-1686`** (`audit_arrived_acks`,
  6 parameters). Skeptic: CONFIRMED (6/10). `channel`, `cursor`, and `pushed` are one
  audit session's state with nothing typing them together, and all four candidate
  bindings coexist in one 620-line scope, so a mispairing compiles and silently
  under-counts while the T015 control (which only fails when `forwarded == 0`) still
  reports PASS. Below threshold; recorded here, not escalated.

---

## Downgraded and refuted (appendix)

Nine findings were downgraded and one refuted. The recurring pattern was accurate
structural observation paired with an over-reaching harm claim.

| Finding | Verdict | Why it did not survive as stated |
|---|---|---|
| Unreachable `if let Some(evicted)` vs sibling `.expect` (`projection_channel.rs:402-406`) | DOWNGRADED to trivial | The two sites do not share an invariant — the `.expect` is guaranteed by a `position()` lookup independent of `MAX_IN_FLIGHT_DOCUMENTS`. Neither half of the claimed "RISK-4 reopens" scenario is reachable. |
| `retire()` dedup guard is speculative generality (`projection_channel.rs:564-575`) | DOWNGRADED to low | The countable claim holds (no test exercises it) but the remedy is inverted: the WP02 reviewer's own probe shows that **without** the guard a faithful ack gets a *false* `IdentityMismatch`. The guard is load-bearing; it needs the test, not deletion. Already recorded as WP02 OBS-A. |
| Seam-name triplication makes T013 run 1 vacuous | DOWNGRADED to low | Run 1 vacuity **confirmed** (and later proven executably — see below), but run 2 lives in the same function and fails a disarmed seam at two independent points, so the section can never be silently green. |
| Release compile-out guard is a tautology (`window.rs:1097-1110`) | DOWNGRADED to low | `forced_close_failure()` *does* reference the constant under `cfg`, so the compile-time mechanism is real; the doc merely attributes it to the wrong code. Skeptic surfaced a larger true point instead: no declared validation builds `#[cfg(not(debug_assertions))]` tests at all. |
| Long Method, 122 lines, shadowed subprocess bindings (`webview_projection_shell.rs:4062-4183`) | DOWNGRADED to minor | Counts exact, harm wrong: the two scenarios are asymmetric, so a misplaced assertion panics loudly rather than passing against the wrong subprocess. |
| Long Method, shadowed close-retry fixtures (`window.rs:968-1035`) | REFUTED | The shadowing *is* the isolation mechanism — each scenario deliberately starts from a fresh counter and unlatched `RefCell`. The finding criticized the correctness-preserving choice. |
| Identity-field list triplication (`PAINTED_ACK_IDENTITY_FIELDS`) | DOWNGRADED to low | The doc sentence claiming one list is **literally false** — the WP01 section it names still carries its own inline literal. But the consequence chain dies: T014 corrupts or omits 5 of 6 identity fields and asserts the exact field name, so narrowing production fails loudly and widening breaks the well-formed controls. |
| Two scan loops disagree on stripped vs raw `index.html` (`component_composition.rs:1823-1858`) | DOWNGRADED to low | Skeptic judged the exposure pre-existing and dormant. **Orchestrator then executed the probe and it fires** — see below. |
| 184-line proof with shadowed fixtures (`webview_projection_shell.rs:1449-1632`) | DOWNGRADED to low-medium | Section 4's coupling to section 2 is essential to what it proves; deletion is a compile error, not a silent break. |
| Feature Envy: T015 verdict homed in `force_page_failures` | DOWNGRADED to low | The placement follows a deliberate pre-existing checkpoint pattern, the block runs before any fault is forced, and the "unchecked invariant" is structurally guaranteed by an exhaustive two-variant match. |

## Claims escalated to execution

Two skeptic verdicts rested on predictions rather than observation. The orchestrator
ran both on the merged tree:

1. **T013 run-1 vacuity — CONFIRMED executably.** Running the shipped binary against
   the forced-throw page with the seam **disarmed** produces `EXIT=1` and the same
   `PageRenderFailed` payload (same generation, same `stateHash`) as the armed run.
   Every run-1 assertion is satisfied with the seam off.
2. **Scan-loop asymmetry — CONFIRMED executably, contradicting the skeptic's "dormant".**
   Adding the HTML comment `<!-- This page registers no keydown handler; keys are
   captured Rust-side. -->` to `webview-page/index.html` fails the suite with
   `index.html registers a key handler (`keydown`); keys are captured Rust-side` — a
   false failure caused by the second loop reading raw `index_html` while the loop
   this mission fixed reads comment-stripped `index_html_markup`. It fails closed, so
   it is a maintenance trip hazard rather than a proof hole.

## Summary

A small, disciplined diff. The pipeline found no Critical structural defects and
nothing that forces a FAIL verdict — consistent with the skill's guidance that an
empty Critical section is the expected outcome on a hygiene mission. The two
reportable findings are both **Dead Code left behind by WP03's deletions**: an
orphaned `Condvar` whose doc still advertises a wakeup that cannot happen, and a
`Display` impl whose only consumer was deleted in the same change. Both are inert at
runtime; both are exactly the residue class FR-005 promised to eliminate, which is
why they are folded into the mission review as a drift finding rather than a pure
maintainability note. The skeptic's kill mandate did substantial work here — it
downgraded or refuted 10 of 13 findings, in several cases by disproving a harm
narrative whose structural observation was entirely correct.
