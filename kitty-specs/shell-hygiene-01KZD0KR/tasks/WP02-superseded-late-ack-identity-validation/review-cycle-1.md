---
wp_id: WP02
reviewer_agent: reviewer-renata
cycle_number: 1
verdict: approved
mission_slug: shell-hygiene-01KZD0KR
reviewed_at: "2026-08-07T04:35:00Z"
affected_files:
  - path: src/shell/webview/projection_channel.rs
---

# WP02 review — cycle 1: APPROVED

Reviewed commit `a0e351d` on `kitty/mission-shell-hygiene-01KZD0KR-lane-b`
(lane HEAD `8509558`, a merge of the mission branch carrying `kitty-specs/`
only). `git diff kitty/mission-shell-hygiene-01KZD0KR..HEAD -- src tests`
touches exactly one file: `src/shell/webview/projection_channel.rs`, +356/-10 —
identical to `owned_files`. No `window.rs` (WP01), no `frame_stream.rs`/`mod.rs`
(WP03), no `tests/webview_projection_shell.rs` (WP04).

Authorities checked: spec.md FR-003 / NFR-001 / NFR-002 / C-001 and User Story 2's
three acceptance scenarios; research.md D2 (decision and all three rejected
alternatives); the WP prompt's Definition of Done.

Every finding below was gathered mechanically in the lane worktree. Nothing in
the implementer's report was taken on trust.

## Claim-by-claim verification

### 1. Bounded store, identities only

`retired_identities: VecDeque<RetiredIdentity>` on `ProjectionChannel`;
`RetiredIdentity { generation: u64, identity: [Value; ACK_IDENTITY_FIELDS.len()] }`.
No `semantic` field — the view model is not retained, so a retired generation
cannot construct a `ShellFrameObservation` even in principle.
`const RETAINED_RETIRED_IDENTITIES: usize = MAX_IN_FLIGHT_DOCUMENTS;` (= 8),
defined in terms of the in-flight bound as D2 requires. `new()` stays `const`.

**Bound proof.** In `retire()`, `len()` is compared against the bound *before*
the single `push_back`, and the only other mutation (`remove(stale)`) shrinks.
`len` therefore never exceeds `RETAINED_RETIRED_IDENTITIES`. Confirmed
empirically, not only by inspection — see the live instrumented run below,
where 216 real retirements over 12 s left the store pinned at exactly 8.
D2's rejected "retain every identity forever" alternative is not what was built.

### 2. Both retirement paths feed the store

- **Path 1 (capacity eviction, `push`)**: `if let Some(evicted) = self.in_flight.pop_front() { self.retire(evicted); }`.
- **Path 2 (ack consumption, `forward_ack`)**: `drain(..=index)` replaced by
  `for _ in 0..=index { let retired = self.in_flight.pop_front().expect(..); self.retire(retired); }`.

**Drain→loop equivalence (no off-by-one).** `drain(..=index)` removes the
`index + 1` front-most elements in front-to-back order. The loop iterates
`index + 1` times popping the front each time: same set, same order, oldest
first. `index` comes from `in_flight.iter().position(..)` and nothing between
that lookup and the loop mutates `in_flight` (the intervening code holds
`&self.in_flight[index]` immutably and builds the observation), so the `expect`
is unreachable — and it is on the window event thread, where the pre-existing
`self.in_flight[index]` already panics on the same impossible condition. No new
panic class.

**No double-retire.** Each `InFlightDocument` is removed from `in_flight` by
exactly one of the two paths and consumed by value, so it reaches `retire()`
once. Re-pushed generations are handled by de-duplication (below).

### 3. One shared `verify_ack_identity`, in-flight path unchanged

The in-flight loop was moved verbatim into a free function
`verify_ack_identity(&Value, &[Value; N], u64) -> Result<(), PaintedAckError>`
(free, not a method, which is what lets the superseded-late path call it while
holding an immutable borrow of `retired_identities`). Diff shows the six removed
lines are exactly the old loop and the added helper body is byte-for-byte the
same comparison, same `unwrap_or(&Value::Null)`, same
`IdentityMismatch { generation, field }`. Both windows call the one
implementation; there is no second copy to drift.

### 4. NFR-002 — nothing loosened

Complete list of removed lines in the diff (mechanically enumerated): two
comment lines, `self.in_flight.pop_front();`, the six-line in-flight identity
loop, and `self.in_flight.drain(..=index);`. **Zero removals in the test
module.** The only test-module hunks are a one-line `use super::{..}` addition
and a pure 197-line insertion.

Independently confirmed by hashing every `#[test]` body in the base and head
versions of the file: **all 13 pre-existing tests byte-identical**, 6 new tests
added (the WP claimed three behaviors; six were delivered). No frozen baseline,
threshold, skip list, or assertion touched.

### 5. C-001 — blast radius / RT boundary, verified by ownership

- `grep` over `src/real_time/` for `webview|projection_channel|ProjectionChannel`: **no hits**.
- `grep` over `projection_channel.rs` for `real_time`: **no hits**.
- `grep` over `projection_channel.rs` for `Arc<|Mutex|RwLock|unsafe impl|std::thread|spawn`: **no hits**. No new cross-thread handle of any kind.
- Ownership: the sole production construction is `let mut projection_channel = ProjectionChannel::new();` at `src/shell/webview/window.rs:452`, a local moved into the `app.run_return(move |handle, event| ..)` event-loop closure. The store is unreachable from anywhere else; `forward_ack` has exactly one production caller (`window.rs:484`) and its signature is unchanged.

No reducer, no projection-schema change: `ACK_IDENTITY_FIELDS`, the ack payload
shape, and `InFlightDocument` are untouched.

### 6. A superseded-late ack still constructs no observation

The new code sits inside the `let ... else` early-return block. Its only two
exits are `Err(..)` via `?` and `Ok(ForwardedAck::SupersededLate { generation })`.
`ShellFrameObservation::try_new_semantic` is reached only after the in-flight
`position()` hit, and it requires `document.semantic` — which `RetiredIdentity`
deliberately does not carry. The invariant that keeps RISK-4 latent survives, and
is proved by `a_superseded_late_ack_never_constructs_an_observation`.

### 7. False rejection risk — none

Analytic: the retained identity is the byte-for-byte `ack_identity_of(&document)`
of that exact generation. A healthy page copies its document's identity verbatim,
so the comparison is an equality against the same bytes. The only way to differ
is the same generation emitted twice with different documents, which a monotone
generation counter cannot produce — and which the de-duplication in `retire()`
handles anyway (see finding OBS-A). Beyond-window acks return `SupersededLate`
unchanged, so no ack that used to pass can now fail.

Empirical: measured, not assumed — see the instrumented live run.

## Independent verification runs (lane worktree, all re-run by the reviewer)

| Run | Result |
|---|---|
| `cargo test --lib` | **ok. 637 passed; 0 failed; 1 ignored** (19 in `projection_channel`) |
| `cargo test` (full suite, every integration target) | **all 30 targets ok; 0 failed** — covers the 5 other test files that use `ProjectionChannel` (`graphical_application_shell`, `shell_event_dispatch`, `semantic_graphical_view_model`, `component_composition`, `webview_projection_shell`) |
| `cargo test --test webview_projection_shell --test component_vocabulary --test component_composition` | 15 / 11 / acceptance ok; `CREST_ACCEPTANCE webview_projection_shell passed` |
| `cargo clippy --all-targets` | **clean**, no warnings |
| `CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell -- --nocapture` | **`CREST_ACCEPTANCE webview_projection_shell passed (skipped: none)`**, exit 0. `CREST_WEBVIEW_NFR001 projection-to-paint over 150 paced reducer edits: p50=7.8ms p95=14.0ms max=18.4ms (threshold p95 <= 50ms)`. NFR-002 soak `29.46 Hz, lost 0`. T012 negative control PASS (zero `crest://render-error`). T026 shutdown parity PASS on the shipped binary. |
| `cargo fmt --check` | 3 diffs, **all pre-existing and outside WP02's ownership** — see OBS-C |

### Reviewer's instrumented live run (the negative control that matters)

`forward_ack` was temporarily wrapped to log every outcome, `cargo build`, then
the **shipped binary** was run with a real window for 12 s and closed through
the native close button (the same path T026 drives), exit 0:

```
216 outcome=observation
  0 outcome=superseded_late
  0 outcome=identity_mismatch
  0 outcome=unknown_document
max retired_len=8   max in_flight=1   total forward_ack calls=216
```

Three things this establishes that inspection could not:

1. **Zero ack rejections across 216 real painted acks** through the production
   `window.rs:484 → forward_ack` path. FR-003's "well-formed acks unaffected"
   and NFR-001 hold on the live product.
2. **The bound holds under real churn**: 216 retirements in 12 s (~18/s) left
   `retired_identities.len()` pinned at exactly 8 and never above. Extrapolated,
   an 8-hour session performs ~500k retirements with the store still at 8.
3. **`in_flight` never exceeded 1** in the shipped product — each document is
   acked before its successor is pushed. This is precisely why RISK-4 was latent,
   and why the new branch is effectively unreachable in a healthy live run.

The instrumentation was reverted; the lane worktree is byte-identical to
`a0e351d`'s content (`sha256 30774ea2…09f48`, `git diff HEAD` empty).

## Reviewer's own disable-the-mechanism probes

Six probes, each applied to a pristine copy and reverted. `cargo test --lib
shell::webview::projection_channel` (19 tests) after each:

| Probe | Change | Result |
|---|---|---|
| baseline | none | 19 passed |
| **P1** | bypass the superseded-late `verify_ack_identity` call | **3 FAILED**: `a_drained_document_…`, `a_capacity_evicted_document_…`, `the_retained_window_holds_exactly_its_bound_…`; the faithful-ack and beyond-window tests still pass |
| **P2** | disable FEED 1 (capacity-eviction `retire`) | **2 FAILED**: `a_capacity_evicted_document_…`, `the_retained_window_…`; the drained-path test passes |
| **P3** | disable FEED 2 (restore the old `drain(..=index)`) | **1 FAILED**: `a_drained_document_…`; the capacity test passes |
| **P4** | *off-by-one probe:* retire **only** `in_flight[index]`, drop the older drained documents unretired | **1 FAILED**: `a_drained_document_…` |
| **P5** | remove the retained-window bound (unbounded growth) | **1 FAILED**: `an_ack_older_than_the_retained_window_stays_a_lost_frame` |
| **P6** | remove the generation de-duplication in `retire()` | **0 FAILED — 19 passed** ⇦ see OBS-A |

P2 and P3 independently prove both feeds are load-bearing. P4 is the reviewer's
addition and proves the *whole drained range* is retired, not just the acked
document — the exact partial-validation failure D2 warns about. P5 proves the
beyond-window test is not vacuous.

## Findings

### OBS-A (non-blocking, proof gap) — `retire()`'s de-duplication is load-bearing but unproven

Probe P6 removed the de-duplication block and **all 19 tests still passed**.
The guard is not decoration: `forward_ack` looks the retired store up with
`.iter().find(..)`, which returns the **oldest** matching entry, so a duplicate
would make a *stale* identity shadow the current one and produce a **false
rejection** — the failure mode the WP calls the dangerous one.

Verified with a purpose-built probe test (generation 5 emitted twice with
different documents, never both in flight; `push` only short-circuits on
`== last_emitted`, so both are `Emitted`):

- with the de-duplication as implemented → `SupersededLate { generation: 5 }` (correct);
- with it removed → `IdentityMismatch { generation: 5, field: "stateHash" }` on a **faithful** ack.

So the implementation is **correct**; what is missing is a proof that it stays
correct. Not a blocker: the WP's DoD does not require this test, T004 asked only
that the code "cannot double-retire the same generation" (it cannot), and the
triggering sequence needs a non-monotone generation, which production cannot
produce (measured `in_flight` max = 1 live). Recommended as a follow-up — the
test below passes as-is against the current implementation and fails without the
guard:

```rust
#[test]
fn a_regenerated_generation_retires_only_its_newest_identity() {
    let mut channel = ProjectionChannel::new();
    let five_a = projection(5, "state-5-A");
    let six = projection(6, "state-6");
    let five_b = projection(5, "state-5-B");
    let seven = projection(7, "state-7");

    push_ok(&mut channel, &five_a);
    push_ok(&mut channel, &six);
    channel.forward_ack(&ack_for(&six).to_string()).expect("6 acks; 5a and 6 retire");

    push_ok(&mut channel, &five_b);
    push_ok(&mut channel, &seven);
    channel.forward_ack(&ack_for(&seven).to_string()).expect("7 acks; 5b and 7 retire");
    assert_eq!(channel.in_flight_documents(), 0);

    // The retained entry for generation 5 must be 5b, not the stale 5a:
    // otherwise a faithful replay is a false rejection.
    assert_eq!(
        channel.forward_ack(&ack_for(&five_b).to_string())
            .expect("a faithful replay must not be a false rejection"),
        ForwardedAck::SupersededLate { generation: 5 }
    );
}
```

### OBS-B (non-blocking, reporting accuracy) — the live suite's 150-paced-edit section does not exercise `forward_ack`

`tests/webview_projection_shell.rs` contains **no call to `forward_ack`** at all
(grep for `forward_ack|ForwardedAck|PaintedAckError` returns nothing); its
`ProjectionChannel` use is the `push`/emit path, and the WP01 paint-ack identity
section validates identity in the harness/page layer. The live suite reaches
`forward_ack` only through the shipped-binary sections (T026 real-window run,
T012/T025 subprocesses).

The implementer's report frames "150 paced edits at p95 13.3 ms with zero ack
rejections" as the negative control for this change; the latency section and the
ack path are different code paths. SC-002 is still satisfied — the shipped-binary
sections do run `forward_ack` and exited 0 — and the reviewer's instrumented run
above supplies the actual number (216/216 observations, 0 rejections). Recorded
so the mission owner reads the evidence correctly; WP04 owns the live acceptance
layer for this window.

### OBS-C (non-blocking, not WP02's) — 3 pre-existing `cargo fmt --check` diffs

`cargo fmt --check` reports diffs at
`src/shell/webview/window.rs:307` (WP01's file) and
`tests/webview_projection_shell.rs:618` and `:2684` (WP04's file).

Verified genuinely pre-existing: both files are byte-identical between the lane
HEAD, the mission base `kitty/mission-shell-hygiene-01KZD0KR`, and the planning
branch `feat/shell-hygiene` (`git diff --quiet` on each pair). WP02's own file
is fmt-clean. **Not held against WP02** — flagged so the mission owner lands them
in WP01/WP04 or a formatting pass before merge.

### OBS-D (informational) — NFR-003 watch

WP02 is +356/-10 (≈198 test lines, the rest production and doc comment).
NFR-003 ("the mission removes more code than it adds, measured on `src/` and
`tests/`") is a mission-level obligation carried mainly by WP03's deletions;
WP02's `requirement_refs` are FR-003/NFR-001/NFR-002 only, so this is not a WP02
defect. Noted for the accept gate's arithmetic.

## Definition of Done

| DoD item | Status |
|---|---|
| Bounded retired-identity store, doc-commented, same eviction discipline | ✅ |
| Both retirement paths feed it (capacity `pop_front` **and** every drained document) | ✅ probes P2/P3/P4 |
| Superseded-late hit validated by the identical comparison, one shared helper, typed `IdentityMismatch` | ✅ probe P1 |
| Store miss keeps `SupersededLate`, narrated against future "tightening" | ✅ probe P5 |
| Well-formed superseded-late acks consumed exactly as before; no pre-existing test changed | ✅ 13/13 byte-identical |
| A superseded-late ack still constructs no `ShellFrameObservation` | ✅ |
| Unit tests cover corrupted (both paths), well-formed, beyond-window; bypass probe recorded | ✅ 6 new tests |
| `forward_ack` doc comment + module header state the enforced rule, naming FR-003 | ✅ |
| `cargo test --lib`, three headless suites green; `cargo clippy --all-targets` clean | ✅ |
| Only `projection_channel.rs` modified; `forward_ack` signature unchanged | ✅ |

## Verdict

**APPROVED.** D2 is implemented as decided, not as one of its rejected
alternatives. Both retirement paths feed a store that is provably bounded under
measured live churn, the drain→loop conversion is exactly equivalent, the
identity rule has one implementation, no proof was weakened, nothing crosses the
real-time boundary, and the shipped product records zero ack rejections across
216 real acks. OBS-A is a proof gap in correct defensive code with a ready-made
test attached; OBS-B and OBS-C are reporting/hygiene items for the mission owner.
The lane worktree was left clean.
