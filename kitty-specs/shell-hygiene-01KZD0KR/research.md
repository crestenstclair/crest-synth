# Research: Shell Hygiene Sweep

**Mission**: `shell-hygiene-01KZD0KR`
**Phase**: 0 — decisions with rationale, resolved before design.

Every decision below was resolved by reading the production code at HEAD, not
by preference. Locations are the authoritative defect sites named by the two
mission-review reports.

---

## D1 — RISK-3: what "the loop early-returns forever" actually is

**Finding.** `close_window_once_with_retry` (`src/shell/webview/window.rs`)
resolves the window, tries `close()`, retries once, and on a second failure
records `WebviewShellError::WindowClose` on the first-error slot via
`get_or_insert` — then returns normally. Its callers (notably the
`PageSignal::RenderError` arm) then set `close_requested = true` and `return`
from the event-loop callback. Nothing else drives termination: the recorded
error is surfaced only *after* `run_return` yields, and `run_return` yields
only when the window actually closes. Both closes failing therefore leaves a
correctly recorded fatal error that never reaches the operator.

**Decision.** When the close path exhausts its retry, the shell must reach
termination by a route that does not depend on the window closing — the
recorded first error is then surfaced by the existing post-`run_return` path,
unchanged. The retry-once behavior, the typed `WindowClose` error, and the
first-error-wins latch all stay exactly as they are; only the
never-terminates hole is closed.

**Rationale.** The bug is a missing exit edge, not a wrong error. Preserving
`get_or_insert` keeps a prior `PageRenderFailed` winning over the subsequent
close failure, which is the behavior the previous mission proved.

**Alternatives considered.** *Panic on double close failure* — rejected: a
panic in the shell adapter is exactly the silent-fallback-class failure this
program forbids, and it would bypass the typed error. *Surface the error
inline from the callback* — rejected: it duplicates the post-`run_return`
surfacing path and creates two places a fatal error can be reported.

---

## D2 — RISK-4: why the superseded-late window is unvalidated, and what closes it

**Finding.** `forward_painted_ack` (`src/shell/webview/projection_channel.rs`)
looks the acked generation up in `in_flight`. On a miss it returns
`ForwardedAck::SupersededLate { generation }` when the generation is at or
below the newest emission, and `PaintedAckError::UnknownDocument` otherwise.
The identity comparison against `ACK_IDENTITY_FIELDS` happens only on the hit
path. The superseded-late branch is unvalidated **because the document it
would validate against has already been dropped** — `in_flight` is a
`VecDeque` bounded at `MAX_IN_FLIGHT_DOCUMENTS = 8`, and both the
capacity-eviction (`pop_front`) and the ack-consumption (`drain(..=index)`)
discard the identity along with the document.

**Decision.** Retain the identities of retired documents in a bounded
companion store — same `MAX_IN_FLIGHT_DOCUMENTS` order of magnitude, same
`VecDeque` eviction discipline — and validate superseded-late acks against
it with the identical `ACK_IDENTITY_FIELDS` comparison the in-flight path
uses. A superseded-late ack whose identity does not match the retired
document it names is typed-rejected with the same error class an in-flight
mismatch produces. An ack naming a generation older than the retained window
keeps today's behavior: it is a lost frame, not a fabrication.

**Rationale.** This makes the documented rule — "verbatim or typed-rejected"
— true as written, in every window, rather than true wherever the channel
happened to still hold the document. The store is bounded, allocated once,
and lives on the shell side of the boundary (never the real-time callback),
so it introduces no new resource behavior.

**Alternatives considered.** *Narrow the documentation to match the code* —
rejected: the guarantee is worth more than the sentence, and a future
consumer of the superseded-late window would inherit an unchecked path.
*Retain every identity forever* — rejected: unbounded growth over a long
session, for no additional guarantee beyond the retained window.
*Validate against the newest document instead* — rejected: it would compare
an ack to a document it never claimed, producing false rejections.

---

## D3 — RISK-5: what is actually dead, verified by caller search

Verified at HEAD by searching `src/` and `tests/` for external callers:

| Item | Location | External callers |
|---|---|---|
| `QualifyingFrameStream::await_qualifying`, `FrameAwaitError` | `src/shell/webview/frame_stream.rs` | none (one doc-comment mention in `mod.rs`) |
| `LiveDemoRunner::step_index()` *(the accessor)* | `src/testing/live_demo_runner.rs:1753` | none |
| `ControlIntent`, `ControlRequest`, `CompositionIntent` | `src/shell/component_vocabulary.rs:348-585` | none — every reference is internal to the module or its own unit tests |
| `CURSOR_GLYPH` | `src/shell/component_vocabulary.rs:812` | none beyond its own unit test |

**Decision.** Delete all four. The `step_index` **field** is heavily used
inside `live_demo_runner.rs` and stays; only the public accessor goes — the
mission-review finding named the accessor, and conflating the two would break
the runner.

`CURSOR_GLYPH` gets the narrower of the two available fixes: its doc claim to
be a single source is false because the gallery hardcodes its own glyph, and
no surviving code consumes the constant, so the constant and its claim go
together rather than the claim being softened in place.

**Rationale.** A public item with no caller is a promise the codebase does not
keep; four of them in one module invite a fifth.

---

## D4 — The control-intent retirement is a boundary re-authoring, not a deletion

**Finding.** The crest-spec never declared `ControlIntent` by name. It declared
the *mechanism* in prose, in three places: `requirement.component_state_ownership_boundary`
("return typed semantic UI intent"), the matching invariant in
`proof/invariants.yaml`, and `requirement.configurable_control_family`
("returning typed semantic intent rather than acting"). After the webview
cutover no Rust component returns anything — the page renders, and the shell
translates physical input into semantic actions.

**Decision.** Retire the *mechanism* clause from all three; keep and sharpen
the *boundary*, which is still live and still proven by
`tests/component_composition.rs` (component sources may not name `AppState`,
may not hold interior mutability, may not convert an input into an action).
Authored in the crest-spec phase (commit `7c7f1cf`) before any code deletion,
per C-002.

**Rationale.** Retiring the whole requirement would delete a guarantee that is
still true and still enforced. The passivity survives its original expression.

**Consequence for implementation.** The assertion *message* at
`tests/component_composition.rs:1742` ("a component returns ControlIntent and
converts nothing") names the retiring type; the assertion itself tests
passivity and is unaffected. The message is corrected with the deletion.

---

## D5 — OBS-1: the gallery is retained, and its serving path is narrated

**Finding.** `component_gallery_scene.rs` registers its own `crest://`
protocol handler attaching only `Content-Type` — no CSP — and `gallery.js`
paints via ten JS-built inline `style=` attributes. This is the same shape as
DRIFT-1, the drift that hid a shipped paint defect. It is unreachable today:
`page_asset` (the production asset table) has no gallery entry, so the shipped
window cannot load gallery sources.

**Decision (operator, 2026-08-06).** Keep the gallery. Record the policy-free
serving as a deliberate, narrated property at the handler, naming the
production asset table as the reason it is unreachable and the defect class it
must not be confused with. Full CSP parity was drafted and reversed: it would
require converting the gallery's inline-style painting and re-homing the 48
gallery-borne proof references in `component_vocabulary`, for no product gain.

**Rationale.** The cheapest honest resolution. The risk was never the missing
policy; it was that a future reader could not tell whether the omission was
deliberate. Narration fixes exactly that.

---

## D6 — SMELL-1 residue: which scan, which sources

**Finding.** The previous mission added `gallery.js` to the no-input-handler
scan in `tests/component_composition.rs` but not to the purity-needle loop
(`Date.now`, `Math.random`, `setTimeout`, `localStorage`, …) at lines
1790-1805, which still binds `page.js` alone. `gallery.js`'s own header
declares the properties that loop enforces.

**Decision.** Extend the purity-needle loop to every page source it is meant
to bind, matching the enumeration style the previous mission established. Any
legitimate construct that fires is fixed at the source or gets a declared,
narrated exemption mirroring the existing precedent — never a silent
carve-out.

**Rationale.** A source that claims a property the guard does not check is a
gap between the record and the proof, which is the class of defect this
mission exists to close.

---

## D7 — Sequencing: what must happen before what

`RISK-3` (window), `RISK-4` (projection channel), `RISK-5` (dead code),
`OBS-1` (gallery narration), `SMELL-1` (scan), and `DRIFT-3` (documents) touch
six disjoint surfaces and share no file. They parallelize completely.

The one ordering constraint is C-002, already satisfied: the crest-spec
retirement (`7c7f1cf`) precedes every deletion.

**Decision.** Slice by surface, not by finding severity, so no two work
packages own the same file.
