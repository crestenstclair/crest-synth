# Mission Specification: Shell Hygiene Sweep

**Mission Branch**: `feat/shell-hygiene`
**Created**: 2026-08-06
**Status**: Draft
**Input**: The deferred non-blocking findings from two post-merge mission reviews — `kitty-specs/webview-shell-cutover-01KZAC7Q/mission-review.md` (RISK-3, RISK-4, RISK-5, DRIFT-3, explicitly parked by that mission's C-003) and `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/mission-review.md` (OBS-1, SMELL-1 residue). This mission discharges exactly those items and nothing else.

## Crest-Spec Grounding

This mission adds no system structure; it restores conformance to structure the crest-spec already declares, and retires one small declaration set the webview cutover orphaned. `crest_spec_impact: structural` (one retirement, no additions). Canonical references, cited not restated:

- `requirement.webview_projection_shell` — declares typed failure paths with no silent fallback. RISK-3 leaves a recorded fatal error unreported when teardown itself fails, so the typed path exists but does not always reach the operator.
- `requirement.serialized_projection_transport` — the paint-acknowledgement identity rule RISK-4's superseded-late window enforces more narrowly than it documents.
- `capability.component_vocabulary` and its gallery acceptance — the gallery scene is **retained** (see US4); this mission records what its serving path is, and does not change it.
- The `ControlIntent` / `ControlRequest` / `CompositionIntent` declarations — orphaned by the webview cutover and **retired** here, per operator decision.

**Flag for the `/spec-kitty.crest-spec` phase**: the only declaration change is the control-intent retirement. It is authored there first, before any deletion. If the gallery's policy-free serving needs to become a declared property rather than a code comment, that phase decides where the declaration lives.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A page render failure always surfaces, even when the window will not close (Priority: P1)

When a page render fails and *both* attempts to close the window then fail, the application still ends by surfacing the recorded typed error, instead of looping with the fatal error recorded but never reported.

**Why this priority**: This is RISK-3. The trigger is improbable, but the failure mode is the one class this program treats as unacceptable — a recorded fatal error that never reaches the operator. It is a correctness hole in the exact error path the previous mission hardened.

**Independent Test**: Force both close attempts to fail with a render error already recorded; assert the process ends nonzero carrying that typed error rather than hanging.

**Acceptance Scenarios**:

1. **Given** a recorded typed failure and both close attempts failing, **When** the event loop continues, **Then** the process terminates and the first recorded typed error is surfaced — no unbounded early-return, no swallowed error.
2. **Given** the same double-close failure with no prior error recorded, **Then** the close failure itself surfaces as its typed `WindowClose` error.
3. **Given** the fix, **When** an ordinary single close failure occurs, **Then** the existing retry-once behavior and the first-error-wins latch are unchanged.

---

### User Story 2 - Ack identity enforcement matches what it documents (Priority: P2)

Every painted acknowledgement the shell consumes is validated against the document identity it claims, including the superseded-late window — so the documented "verbatim or typed-rejected" rule is true as written rather than true in most windows.

**Why this priority**: This is RISK-4. No proof breaks today because a superseded-late ack can never construct an observation, so the gap is latent. It matters because the documented MUST is wider than the enforcement, and a future consumer of that window would inherit an unchecked path.

**Independent Test**: Feed a superseded-late ack carrying a corrupted identity; assert it is typed-rejected rather than silently consumed.

**Acceptance Scenarios**:

1. **Given** a superseded-late ack whose identity does not match its claimed document, **When** the channel consumes it, **Then** it is rejected with the same typed error class an in-flight identity mismatch produces.
2. **Given** a well-formed superseded-late ack, **Then** it is consumed exactly as before — the mission narrows nothing that already worked.
3. **Given** the change, **When** a full live run executes, **Then** it records zero ack rejections.

---

### User Story 3 - Dead code is gone and the spec agrees with the code (Priority: P2)

A reader of the shell finds no public API without a caller and no crest-spec declaration without an implementation, so "declared" and "built" mean the same thing again.

**Why this priority**: This is RISK-5. Its most serious item is the `ControlIntent`/`ControlRequest`/`CompositionIntent` family: the crest-spec declares a control-intent vocabulary the cutover orphaned, so spec and code presently disagree. **Operator decision (2026-08-06): retire the declarations** — delete the dead code and the crest-spec declarations together; Phase 5 re-declares control intent where it actually lives when it needs it.

**Independent Test**: For each retired item, grep for callers before deletion and for dangling references after.

**Acceptance Scenarios**:

1. **Given** the retirement, **When** the shell is searched, **Then** `QualifyingFrameStream::await_qualifying`, `FrameAwaitError`, `LiveDemoRunner::step_index`, and the `ControlIntent`/`ControlRequest`/`CompositionIntent` family are absent, along with their crest-spec declarations.
2. **Given** the control-intent retirement, **Then** it is authored in the crest-spec before the code is deleted — never a declaration retired to accommodate a deletion already made.
3. **Given** `CURSOR_GLYPH`'s claim to be a single source, **Then** either it is the single source in fact or the false claim is removed — no constant may document an authority it does not hold.
4. **Given** the deletions, **When** the full suite runs, **Then** it passes with no behavior change and no proof removed to accommodate a deletion.

---

### User Story 4 - The gallery's serving path is recorded rather than assumed (Priority: P3)

A maintainer reading the component gallery scene finds its policy-free serving stated as a deliberate, narrated property of a testing-only surface — so nobody later mistakes it for the drift that hid a shipped defect, and nobody "fixes" it by accident.

**Why this priority**: This is OBS-1. **Operator decision (2026-08-06): keep the gallery and declare the exemption.** Full CSP parity was considered and reversed as not worth its cost — it would require converting the gallery's inline-style painting and re-homing proof coverage for no product gain. The gallery is a testing scene the production asset table can never serve, so the latent shape is real but unreachable.

**Independent Test**: Read the gallery scene's serving path; the narration states why it serves no policy and what would have to change if it ever served production assets.

**Acceptance Scenarios**:

1. **Given** the gallery scene, **When** its protocol handler is read, **Then** the policy-free serving is narrated as deliberate, naming the production asset table as the reason it is unreachable from the shipped window and the paint-fidelity defect class it must not be confused with.
2. **Given** the gallery is retained, **Then** no gallery source, page asset, scene, CLI option, or make target is deleted, and both acceptance suites pass unchanged.

---

### User Story 5 - Guard scans bind every source they claim to (Priority: P3)

The executable guard scans that keep page sources honest cover every page source they are meant to bind, so a source cannot sit silently outside a rule it is supposed to obey.

**Why this priority**: This is SMELL-1's residue. The previous mission added the gallery to the key-handler scan but not to the purity-needle scan, while the gallery's own header claims the properties that scan enforces. Cheap, and it closes a gap between what a source claims and what is checked.

**Independent Test**: Plant a purity violation in each scanned source; each must fail by name.

**Acceptance Scenarios**:

1. **Given** the purity-needle scan, **When** it runs, **Then** every page source it is meant to bind is inside its scanned set, and all pass on the current sources.
2. **Given** a planted violation in any newly covered source, **Then** the suite fails naming that source and the offending needle.

---

### User Story 6 - The record says what actually happened (Priority: P3)

A reader of the completed missions finds no stale terminology, no unfinished status field, and no unquantified claim left implicit.

**Why this priority**: This is DRIFT-3. Documentation-only, but the alternative is a record that reads as unfinished after the work finished.

**Acceptance Scenarios**:

1. **Given** DRIFT-3, **Then** the NFR-002 leak bound is quantified or explicitly recorded as unquantified-by-decision with its rationale, stale "migration" terminology is gone from the affected planning documents, and no completed mission's spec still reads `Draft`/`Open`.

---

### Edge Cases

- The double-close fix must not alter the ordinary single-close-failure path, the retry-once behavior, or the first-error-wins latch the previous mission established.
- The superseded-late validation must not reject acks a healthy run legitimately produces — the negative control is a full live run recording zero rejections.
- Deleting `CURSOR_GLYPH`'s consumer versus deleting its claim are different fixes with different blast radii; pick by whether any surviving code needs the glyph.
- Retiring the control-intent declarations must not disturb the surviving component vocabulary the gallery and the production page both prove.
- A purity needle newly bound to a source may fire on a legitimate construct; the fix is at the source or a declared, narrated exemption mirroring the existing precedent — never a silent carve-out.

## Domain Language *(canonical terms)*

- **Retire (a declaration)**: remove it from the crest-spec deliberately, together with the code that realized it, recorded as a decision. Distinct from *delete*, which is a code action with no declarative meaning.
- **Declared exemption**: a narrated, deliberately recorded deviation whose reason and blast radius are stated at the code that deviates. Distinct from a *silent carve-out*, which is forbidden.
- **Superseded-late ack**: a paint acknowledgement arriving for a generation at or below the newest tracked document, no longer in flight.
- Avoid: "clean up the tests" — every proof change here is a fix or a declared retirement, never a cleanup.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | Requirement | User Story | Priority | Status |
|----|-------|-------------|------------|----------|--------|
| FR-001 | Double close-failure still surfaces the error | When both window-close attempts fail, the loop terminates and the first recorded typed error is surfaced rather than retained unreported. | US1 | High | Open |
| FR-002 | Close-path behavior otherwise unchanged | The single-close-failure retry, the `WindowClose` typed error, and the first-error-wins latch behave exactly as before. | US1 | High | Open |
| FR-003 | Superseded-late acks are identity-validated | An ack in the superseded-late window is validated against its claimed identity and typed-rejected on mismatch, matching the documented rule; well-formed acks are unaffected. | US2 | Medium | Open |
| FR-004 | Control-intent declarations retired first | The `ControlIntent`/`ControlRequest`/`CompositionIntent` declarations are retired in the crest-spec before their code is deleted. | US3 | Medium | Open |
| FR-005 | Dead code removed | `QualifyingFrameStream::await_qualifying`, `FrameAwaitError`, `LiveDemoRunner::step_index`, and the retired control-intent family are absent with no dangling reference; `CURSOR_GLYPH`'s single-source claim is made true or removed. | US3 | Medium | Open |
| FR-006 | Gallery serving path narrated | The gallery scene's policy-free serving is recorded as a deliberate, narrated property naming why it is unreachable from the shipped window and what would change if that stopped being true. | US4 | Low | Open |
| FR-007 | Guard-scan coverage completed | The purity-needle scan binds every page source it is meant to cover, with no source silently outside the scanned set, and a planted violation fails by name. | US5 | Low | Open |
| FR-008 | Documentation residue discharged | The NFR-002 leak bound is quantified or recorded as unquantified-by-decision, stale "migration" terminology is removed, and no completed mission's spec still reads `Draft`/`Open`. | US6 | Low | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | No product behavior change | The shipped application's rendered output and semantic behavior are unchanged: the live acceptance suite passes with `skipped: none` and latency stays within the declared 50 ms p95. | Reliability | High | Open |
| NFR-002 | No proof weakened | No frozen baseline, threshold, skip list, or assertion is loosened, and no declared validation stops executing except where it retires with its declaration. | Reliability | High | Open |
| NFR-003 | Net code reduction | The mission removes more code than it adds, measured on `src/` and `tests/` only. | Maintainability | Low | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | Blast radius | No reducer, real-time, projection-schema, or product-surface change. Shell, testing-scene, test, and document surfaces only. | Technical | High | Open |
| C-002 | Declaration before deletion | Every retirement is authored in the crest-spec first and deleted second. A deletion whose declaration still stands, or a declaration retired to accommodate an already-made deletion, both fail acceptance. | Process | High | Open |
| C-003 | Gallery retained | The component gallery scene, its page assets, its CLI option, and its make target are not deleted, converted, or reduced. Retirement was considered and deliberately reversed. | Process | High | Open |
| C-004 | Scope boundary | Phase 5 product work is out of scope. This mission adds no feature and re-declares no control-intent vocabulary; it fixes, retires, and records. | Process | High | Open |

## Success Criteria *(mandatory)*

- **SC-001**: A forced double close-failure ends the process with the recorded typed error surfaced, in 100% of forced runs.
- **SC-002**: A corrupted superseded-late ack is typed-rejected, and a full live run records zero rejections.
- **SC-003**: No public item in the touched modules lacks a caller, and no surviving crest-spec declaration lacks an implementation.
- **SC-004**: A planted purity violation in any scanned page source fails the suite by name.
- **SC-005**: The full live acceptance run passes with `skipped: none` after every change, proving no product behavior moved.
- **SC-006**: Re-running the two mission reviews' checks for RISK-3, RISK-4, RISK-5, DRIFT-3, OBS-1, and the SMELL-1 residue finds all resolved, with no new findings in the touched seams.

## Assumptions

- The two mission-review reports are accurate as of their recorded commits; their file and line references are the authoritative defect locations.
- The gallery remains a testing-only scene the production asset table cannot serve; FR-006 records that fact rather than assuming it stays true silently.
- No surviving code depends on the control-intent family; the retirement verifies this by caller search rather than by compilation alone.
