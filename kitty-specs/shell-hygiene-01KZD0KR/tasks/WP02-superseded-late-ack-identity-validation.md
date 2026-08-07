---
work_package_id: WP02
title: Superseded-late ack identity validation
dependencies: []
requirement_refs:
- FR-003
- NFR-001
- NFR-002
planning_base_branch: feat/shell-hygiene
merge_target_branch: feat/shell-hygiene
branch_strategy: Planning artifacts for this mission were generated on feat/shell-hygiene. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/shell-hygiene unless the human explicitly redirects the landing branch.
base_branch: kitty/mission-shell-hygiene-01KZD0KR
base_commit: 7e63daec1477f83fd2536c9ff3dba01bf088ed28
created_at: '2026-08-07T03:02:17.946936+00:00'
subtasks:
- T004
- T005
- T006
- T007
history:
- '2026-08-06: authored from plan IC-02, research D2, crest-spec asset WebviewShellModules, mission-review RISK-4'
agent_profile: implementer-ivan
authoritative_surface: src/shell/webview/
create_intent: []
execution_mode: code_change
owned_files:
- src/shell/webview/projection_channel.rs
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

Make the documented paint-acknowledgement rule — *the ack must copy its
document's identity exactly, or be typed-rejected* — true in **every** window,
including the superseded-late window where it is currently unenforced.

RISK-4 is latent, not live: a superseded-late ack can never construct a
`ShellFrameObservation`, so nothing is fabricated today. It matters because the
documented MUST is wider than the enforcement, and a future consumer of that
window inherits an unchecked path.

**Authorities** (cited, not restated — read them):

- `spec.md` FR-003, NFR-001, NFR-002, C-001; User Story 2 and its three
  acceptance scenarios.
- `plan.md` IC-02.
- `research.md` **D2** — the finding (why the branch is unvalidated: the
  document was already dropped), the decision (bounded retired-identity
  companion store, same eviction discipline, identical `ACK_IDENTITY_FIELDS`
  comparison), and the three rejected alternatives (narrow the documentation;
  retain every identity forever; validate against the newest document). D2 is
  settled. Do not re-open it; do not implement a rejected alternative.
- `requirement.serialized_projection_transport` in the crest-spec — the ack
  identity rule this WP brings the code up to. Read it via
  `spec-kitty crest-spec context`.
- Asset `WebviewShellModules`.

**Hard boundaries**:

- The retained store is **shell-side only**. It must never appear anywhere near
  the real-time callback (C-001). `ProjectionChannel` lives on the event thread;
  keep it there.
- No projection-schema change. The ack payload, the document, and
  `ACK_IDENTITY_FIELDS` keep their exact shapes.
- Do **not** edit `src/shell/webview/window.rs` (WP01),
  `src/shell/webview/frame_stream.rs` or `mod.rs` (WP03), or
  `tests/webview_projection_shell.rs` (WP04).

## Context: what exists today

Read all of this before editing. Line numbers are as of the mission's planning
commit and may drift by a line or two.

**The bound and the fields** (`src/shell/webview/projection_channel.rs`):

- `l.103` — `pub const MAX_IN_FLIGHT_DOCUMENTS: usize = 8;`
- `l.109` — `const ACK_IDENTITY_FIELDS: [&str; 6]` — generation, stateHash,
  context, activeSurface, focusPath, interactionMode.
- `l.284-290` — `struct InFlightDocument { generation, identity:
  [Value; ACK_IDENTITY_FIELDS.len()], semantic }`.
- `l.295-302` — `pub struct ProjectionChannel { last_emitted_generation:
  Option<u64>, in_flight: VecDeque<InFlightDocument> }`.
- `l.463-465` — `fn ack_identity_of(document: &Value) -> [Value; 6]`, the
  single place a document's identity is copied.

**Retirement path 1 — capacity eviction** (`push`, l.343-350):

```rust
// Track the emitted document so its painted ack can correlate. The
// tracker is bounded: when full, the oldest unacked document is a
// lost frame — dropped, degrading observation only.
if self.in_flight.len() == MAX_IN_FLIGHT_DOCUMENTS {
    self.in_flight.pop_front();
}
self.in_flight.push_back(InFlightDocument { generation, identity, semantic });
```

**Retirement path 2 — ack consumption** (`forward_ack`, l.454-457):

```rust
// The ack consumed its document. Everything older was superseded
// without ever acking — dropped, degrading observation only.
self.in_flight.drain(..=index);
Ok(ForwardedAck::Observation(observation))
```

Note this drains the acked document **and every older one** — so one successful
ack can retire up to `MAX_IN_FLIGHT_DOCUMENTS` identities at once.

**The unvalidated branch** (`forward_ack`, l.388-406):

```rust
let Some(index) = self
    .in_flight
    .iter()
    .position(|document| document.generation == generation)
else {
    // Generations are monotone, so an ack at or below the newest
    // emitted generation names a document this channel pushed and
    // has since dropped (superseded before its ack arrived): a lost
    // frame, not fabricated evidence. Anything newer than every
    // emission cannot have been painted from a pushed document.
    return if self
        .last_emitted_generation
        .is_some_and(|newest| generation <= newest)
    {
        Ok(ForwardedAck::SupersededLate { generation })
    } else {
        Err(PaintedAckError::UnknownDocument { generation })
    };
};
```

**The comparison to reuse verbatim** (the in-flight hit path, l.409-414):

```rust
let document = &self.in_flight[index];
for (field, expected) in ACK_IDENTITY_FIELDS.iter().zip(&document.identity) {
    let supplied = ack.get(*field).unwrap_or(&Value::Null);
    if supplied != expected {
        return Err(PaintedAckError::IdentityMismatch { generation, field });
    }
}
```

**Existing unit tests**: `l.505-947` in the same file. `l.854-864` already
exercises the capacity bound (`MAX_IN_FLIGHT_DOCUMENTS as u64 + 3` pushes,
asserting `in_flight_documents() == MAX_IN_FLIGHT_DOCUMENTS`). `l.836-846`
exercises the supersede-by-newer-ack ordering. Both are the shape to extend.

`ProjectionChannel::forward_ack` has exactly one production caller:
`src/shell/webview/window.rs:484`. Do not change its signature.

## Subtasks

### T004 — Bounded retired-identity store, fed by BOTH retirement paths

**Purpose**: The superseded-late branch cannot validate anything because the
identity it would validate against was thrown away. Keep it, bounded.

**Steps**:

1. Add a companion field to `ProjectionChannel` beside `in_flight` — a
   `VecDeque` of retired `(generation, identity)` pairs. Reuse the existing
   identity type (`[Value; ACK_IDENTITY_FIELDS.len()]`); do not invent a second
   identity representation. If you introduce a small struct, mirror
   `InFlightDocument`'s shape and keep it private.
2. Bound it at the **same order of magnitude** with the **same eviction
   discipline** as `in_flight`: a `pop_front` when full, no growth, no
   reallocation churn. D2 says "same `MAX_IN_FLIGHT_DOCUMENTS` order of
   magnitude" — reuse the constant, or introduce one named for the retained
   window and defined in terms of it, with a doc comment saying why.
3. Feed it from **both** retirement paths:
   - the capacity eviction at l.343-345 — the `pop_front`ed document's identity
     is retired, not discarded;
   - the ack consumption at l.456 — `drain(..=index)` retires **every** drained
     document's identity, in order, not just the acked one.
   D2 is explicit that feeding only one path makes the validation silently
   partial. A reviewer will check both.
4. Keep `ProjectionChannel::new()` `const` if it is `const` today (it is,
   l.307). `VecDeque::new()` is const-constructible; do not break that.
5. Doc-comment the field: what it holds, why it is bounded, why it exists
   (FR-003 / RISK-4), and that it lives on the shell side of the boundary and
   never near the real-time callback.

**Files**: `src/shell/webview/projection_channel.rs`.

**Validation**: `cargo build`; `cargo clippy --all-targets` clean;
`cargo test --lib` green (existing tests must be unaffected at this point).

**Edge cases**:

- A document that is pushed and then evicted by capacity and *later* would also
  be drained cannot happen (it is gone from `in_flight`), but make sure your
  code cannot double-retire the same generation. Duplicate entries waste the
  window and could make an older identity shadow a newer one.
- Ordering matters: the store must evict oldest-first so the retained window is
  always the most-recently-retired N.

### T005 — Validate superseded-late acks against the store

**Purpose**: FR-003. Make the documented rule true as written.

**Steps**:

1. In the superseded-late branch (l.398-402), before returning
   `ForwardedAck::SupersededLate { generation }`, look the acked generation up
   in the retired store.
2. **On a hit**: run the **identical** `ACK_IDENTITY_FIELDS` comparison the
   in-flight path runs at l.409-414 — same iteration, same `unwrap_or(&Value::Null)`
   treatment of a missing field, same `PaintedAckError::IdentityMismatch
   { generation, field }` on mismatch. Factor the loop into one private helper
   used by both call sites rather than copying it; two copies of an identity
   rule is exactly the drift class this mission exists to close. If you factor
   it, the in-flight path's behavior must be byte-identical afterwards.
3. **On a hit with a matching identity**: return `ForwardedAck::SupersededLate
   { generation }` exactly as today. The mission narrows nothing that already
   worked (spec US2 acceptance scenario 2).
4. **On a miss**: see T006.
5. Update `forward_ack`'s doc comment (l.359-376) and the module header's
   description of the ack rule (l.38-40) so both state the rule the code now
   actually enforces, in every window. Name FR-003. Do not leave a doc sentence
   that is now narrower or wider than the code.

**Files**: `src/shell/webview/projection_channel.rs`.

**Validation**: the tests you add in T007.

**Edge cases**:

- The `generation` field is itself one of `ACK_IDENTITY_FIELDS`. It was already
  matched to select the document, so it will compare equal — that is fine and
  is exactly how the in-flight path behaves. Do not special-case it.
- `PaintedAckError::UnknownDocument` is for a generation **newer** than every
  emission. That branch is unchanged; do not fold it into the new logic.

### T006 — Beyond-window acks stay lost frames; healthy acks stay accepted

**Purpose**: The change must reject only what it should. A false rejection is a
product behavior change (NFR-001) and would break the live suite.

**Steps**:

1. **Miss in the retired store** (the generation is older than the retained
   window): keep **today's behavior** — `ForwardedAck::SupersededLate
   { generation }`. D2: "An ack naming a generation older than the retained
   window keeps today's behavior: it is a lost frame, not a fabrication." It is
   **not** a rejection, and it is **not** `UnknownDocument`.
2. Narrate this in a comment at the miss branch: the retained window is finite
   by design, and an ack that outlived it is a lost frame the channel cannot
   speak to either way. State the trade-off explicitly so a future reader does
   not "tighten" it into a false rejection.
3. **Well-formed superseded-late acks** must be consumed exactly as before. Walk
   the existing tests at l.836-870 and confirm none of them change behavior.
4. Confirm nothing on this path can now allocate unboundedly or construct an
   observation. A superseded-late ack must still never produce a
   `ShellFrameObservation` — that invariant is what makes RISK-4 latent rather
   than live, and it must survive.

**Files**: `src/shell/webview/projection_channel.rs`.

**Validation**: the tests you add in T007; plus every pre-existing test in the
module still green **without modification** (NFR-002).

**Edge cases**:

- A channel that has emitted nothing (`last_emitted_generation == None`) still
  takes the `UnknownDocument` branch for any ack. Unchanged.
- An ack for generation 0 when the newest emission is 0 and the document is
  still in flight takes the hit path, not the superseded-late path. Unchanged.

### T007 — Unit tests for all three cases plus a bypass probe

**Purpose**: Prove the three behaviors and prove the proof can fail.

**Steps**:

1. Extend the in-module test block (`l.505-947`). Follow its existing helpers
   (`ack_for`, `advanced`, the push/ack fixtures) rather than inventing new
   scaffolding.
2. **Test A — corrupted superseded-late ack is typed-rejected**: push a
   document, supersede it (either by capacity or by a newer successful ack —
   cover **both retirement paths**, because T004's whole point is that both feed
   the store), then feed an ack for the retired generation with one identity
   field corrupted. Assert `PaintedAckError::IdentityMismatch` naming that
   field. Do this for at least two different corrupted fields.
3. **Test B — well-formed superseded-late ack is accepted unchanged**: same
   setup, uncorrupted ack, assert `ForwardedAck::SupersededLate { generation }`.
4. **Test C — beyond the retained window keeps lost-frame behavior**: retire
   more than the retained window's worth of documents, then ack the oldest.
   Assert `ForwardedAck::SupersededLate`, **not** an error. Feed it a corrupted
   identity too and assert it is still `SupersededLate` — outside the window the
   channel makes no claim, and a false rejection there is the failure mode T006
   guards.
5. **Bypass probe**: comment out the new identity comparison in the
   superseded-late branch, run the suite, confirm Test A fails and Tests B and C
   still pass, restore, confirm all green. Record both outcomes verbatim in the
   WP report. This is the same probe WP04 T016 will run at the acceptance layer;
   yours proves it at the unit layer.
6. Do not modify or weaken any existing assertion in the module to accommodate
   the new field (NFR-002). If an existing test constructs `ProjectionChannel`
   by literal struct initialization and your new field breaks it, add the field
   to the literal — do not relax the test.

**Files**: `src/shell/webview/projection_channel.rs` (test module).

**Validation**: `cargo test --lib` green; the three headless suites green; the
probe recorded.

**Edge cases**:

- Test C must retire strictly more than the retained window, not exactly the
  window size — an off-by-one there would make the test pass for the wrong
  reason.
- If you reused `MAX_IN_FLIGHT_DOCUMENTS` as the retained bound, write the tests
  in terms of the constant, never the literal `8`.

## Branch Strategy

- **Planning base branch**: `feat/shell-hygiene`
- **Merge target branch**: `feat/shell-hygiene`

Planning artifacts for this mission were generated on `feat/shell-hygiene`.
During implementation this WP works on its own lane branch and merges back into
`feat/shell-hygiene` unless the human explicitly redirects the landing branch.

### Gate context — read this, it prevents three known failures

1. **Commit ONLY your owned production/test files on the lane branch.** Your
   owned file is `src/shell/webview/projection_channel.rs`. The move-task gate
   **REFUSES** commits touching `kitty-specs/` on a lane branch. Review
   artifacts, evidence, and status files go on `feat/shell-hygiene` from the
   primary checkout — not from your lane. If you find yourself wanting to
   `git add kitty-specs/...`, stop: that is the gate you are about to trip.
2. **Do not park waiting for a background notification.** If you launch anything
   in the background, use bounded foreground waits and check the run state
   yourself. Never end a turn with "waiting for the build to finish".
3. **Run `cargo test --lib` and the headless suites before requesting review.**
   The headless set is
   `cargo test --test webview_projection_shell --test component_vocabulary
   --test component_composition`. Paste the results.
4. **NFR-001 forbids product behavior change; NFR-002 forbids weakening any
   proof.** No frozen baseline, threshold, skip list, or assertion may be
   loosened. If a proof fails, the fix is in your code, never in the proof.

## Definition of Done

- [ ] A bounded retired-identity store exists on `ProjectionChannel`, doc-
      commented, with the same eviction discipline as `in_flight`.
- [ ] **Both** retirement paths feed it: the capacity `pop_front` and the
      ack-consumption `drain(..=index)` (every drained document, not just the
      acked one).
- [ ] The superseded-late branch validates a store hit with the identical
      `ACK_IDENTITY_FIELDS` comparison, factored into one helper shared with the
      in-flight path, returning `PaintedAckError::IdentityMismatch` on mismatch.
- [ ] A store miss keeps today's `SupersededLate` lost-frame behavior, narrated.
- [ ] Well-formed superseded-late acks are consumed exactly as before; no
      pre-existing test changed.
- [ ] A superseded-late ack still can never construct a `ShellFrameObservation`.
- [ ] Unit tests cover corrupted (both retirement paths), well-formed, and
      beyond-window cases; the bypass probe is recorded with both outcomes.
- [ ] `forward_ack`'s doc comment and the module header state the rule the code
      now enforces, in every window.
- [ ] `cargo test --lib` and the three headless suites green;
      `cargo clippy --all-targets` clean.
- [ ] No file outside `src/shell/webview/projection_channel.rs` is modified;
      `forward_ack`'s signature is unchanged.

## Risks / Reviewer Guidance

- **The highest-value check is that both retirement paths feed the store.** Read
  `push`'s eviction and `forward_ack`'s `drain` and confirm each one retires
  identities. A store fed only by capacity eviction validates almost nothing in
  practice, because ack consumption is the common retirement path.
- **`drain(..=index)` retires a range.** If the implementation retires only
  `self.in_flight[index]`, every superseded older document silently escapes the
  store. Check the loop covers the whole drained range, in order.
- **Two copies of the identity comparison is a regression, not a fix.** The
  comparison must be one helper. If it was duplicated, the in-flight path and
  the superseded-late path can drift, which is the exact class of defect this
  mission closes.
- **False rejections are the dangerous failure.** WP04's live negative control
  (zero ack rejections across a healthy run) is the backstop, but a reviewer
  should reason about it here: which healthy scenario could now produce
  `IdentityMismatch` that did not before? The answer must be "none".
- **Bound discipline.** Confirm the store cannot grow without bound over a long
  session — D2 rejected "retain every identity forever" explicitly.
- **RT boundary.** Confirm nothing new is reachable from the real-time callback.
  `ProjectionChannel` is event-thread-only; a reviewer should verify no new
  `Arc`, `Mutex`, or cross-thread handle was introduced.
