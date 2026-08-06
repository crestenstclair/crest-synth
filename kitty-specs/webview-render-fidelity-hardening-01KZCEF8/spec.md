# Mission Specification: Webview Render Fidelity and Error-Path Hardening

**Mission Branch**: `feat/webview-shell-cutover`
**Created**: 2026-08-06
**Status**: Draft
**Input**: Fix mission derived from the post-merge mission review of `webview-shell-cutover-01KZAC7Q` (`kitty-specs/webview-shell-cutover-01KZAC7Q/mission-review.md`), which returned **FAIL** on two HIGH findings (RISK-1, RISK-2) enabled by two proof gaps (DRIFT-1, DRIFT-2). This mission fixes exactly those four items and nothing else.

## Crest-Spec Grounding

This mission adds no new system structure; it restores conformance to structure the crest-spec already declares and hardens its proof. Canonical references (cited, not restated):

- `capability.graphical_application_shell` — `acceptance.graphical_application_shell.production_shell` requires the normal application to render every shell region from the immutable graphical projection. RISK-1 violates this in the shipped window: fader fills and position indicators paint empty.
- `requirement.webview_projection_shell` — the webview renders every shipped surface at authored-design parity, and failure paths are typed with no silent fallback. RISK-1 breaks parity; RISK-2 leaves a declared-typed failure path silently unreachable.
- `requirement.serialized_projection_transport` — reducer state change visible within 50 ms p95 under the paced live demo workload. Its recorded evidence was measured without the production security policy (DRIFT-1) and must be re-collected under it.
- `requirement.graphical_shell_behavioral_proof` — the headless acceptance target proves rendering "through the production webview projection path". DRIFT-1 shows the served security policy was not part of that path.
- `evidence.graphical_application_shell_contract`, `evidence.component_vocabulary_contract` — the affected declared evidence.

**Flag for the `/spec-kitty.crest-spec` phase** (declarations that may need authoring before plan):

1. `requirement.webview_projection_shell` declares webview **initialization** failure as a typed fatal error, but does not declare render-time page failure. The typed page-render-failure guarantee (FR-005/FR-006) likely needs an explicit declaration.
2. `requirement.graphical_shell_behavioral_proof` may need tightening so "production webview projection path" explicitly includes serving the page under the production content-security policy — the exact gap DRIFT-1 exploited.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Fader visuals render truthfully in the shipped window (Priority: P1)

A person launches the shipped application (`make run`), plays or adjusts levels, and sees every fader fill and position indicator painted to match the value its readout text shows — at all sixteen mixer tracks and in PATCH context controls — under the exact security policy the production window enforces.

**Why this priority**: This is RISK-1, the confirmed-live HIGH finding: today every fill paints empty while the readout shows the true value. It is the visible product defect the review failed the mission on.

**Independent Test**: Launch the release binary, set a known level (e.g. hex 73 ≈ 90%), and visually/programmatically compare painted fill geometry against readout text.

**Acceptance Scenarios**:

1. **Given** the shipped window serving its production security policy, **When** any track level or control position is set to value V, **Then** the painted fill/indicator geometry reflects V and the readout text agrees.
2. **Given** boundary values (minimum and maximum), **When** rendered, **Then** the fill paints fully empty / fully full respectively, with no clamping mismatch against the readout.
3. **Given** the fix, **When** the served page and policy are inspected, **Then** no JS-built inline `style` attribute carries the geometry (custom properties are set via CSSOM or data-attribute + stylesheet binding) and the policy contains no `unsafe-inline` in any directive.

---

### User Story 2 - Acceptance evidence measures what production ships (Priority: P1)

A maintainer running the shell acceptance proofs gets fidelity, determinism, and latency evidence collected under the identical security policy the production window serves — so a policy-induced paint failure can never again pass acceptance invisibly.

**Why this priority**: This is DRIFT-1, the enabler. Without it, User Story 1's fix cannot be honestly proven, and the same class of defect ships again. It is P1 because it gates the proof of P1.

**Independent Test**: Point the harness at the production response path, assert the served policy equals the production policy byte-for-byte, then re-run the affected proofs.

**Acceptance Scenarios**:

1. **Given** the acceptance harness, **When** it serves the page, **Then** the response carries the production content-security policy identically to the shipped window (same source of truth, not a copy).
2. **Given** the harness under the production policy, **When** the determinism (T024), latency (50 ms p95 per `requirement.serialized_projection_transport`), and screenshot proofs re-run, **Then** all pass and their committed evidence is refreshed.
3. **Given** a deliberate regression that stops painted fader geometry from reflecting the level variable under the shipped policy, **When** the proof suite runs, **Then** a named proof fails.

---

### User Story 3 - A page render failure is loud, typed, and fatal (Priority: P2)

When the page's render function throws (or an unhandled promise rejection occurs in the page), the application ends with a typed nonzero exit identifying the page render failure — instead of silently keeping a stale display with no acknowledgement and no error.

**Why this priority**: This is RISK-2. The error channel and typed exit path already exist shell-side; only the page-side emitter is missing. Silent staleness in an instrument UI is a trust-destroying failure, but it is P2 because it needs a fault to occur, whereas US1 is wrong on every frame today.

**Independent Test**: Force a render throw in a test build and assert the shell process exits nonzero with the typed page-render-failure error.

**Acceptance Scenarios**:

1. **Given** a running shell, **When** the page's render throws, **Then** the page emits the render-error message with a typed payload, the shell surfaces the typed failure, and the process exits nonzero.
2. **Given** a running shell, **When** an unhandled promise rejection occurs in the page, **Then** the same typed path fires.
3. **Given** the production security policy, **When** the error emit fires, **Then** the emission channel itself is permitted by that policy (the fix must not be dead-on-arrival the way the fills were).
4. **Given** the fix, **When** the page source is read, **Then** the comment falsely claiming render throws already surface is corrected.

---

### User Story 4 - Gallery sources are guard-scanned (Priority: P3)

The executable guard scans that keep page sources honest — the no-input-handler scan and the style-literal scan — also cover the gallery sources, so the currently-clean gallery cannot silently drift out of the rules.

**Why this priority**: This is DRIFT-2 — cheap, adjacent, and preventive. Gallery sources are clean today; this locks that in.

**Independent Test**: Add a forbidden input handler or style literal to a gallery source in a scratch tree and confirm the scans fail.

**Acceptance Scenarios**:

1. **Given** the guard scans, **When** they run, **Then** `gallery.js` and `gallery.css` are inside the scanned set of both the no-input-handler scan and the style-literal scan, and both pass on the current sources.

---

### Edge Cases

- Render throw during the very first render (before any successful paint) vs. during an update render — both must reach the typed exit.
- Repeated render errors before shutdown completes: the first typed error wins; later ones must not corrupt or replace the recorded failure, and must not deadlock teardown.
- The error boundary itself must be minimal enough that it cannot throw before emitting (no rendering, no allocation-heavy formatting in the boundary).
- The render-error emit must succeed under the production policy — verify the channel is not blocked the same way the style attributes were.
- Fader values at exact 0 and maximum: painted geometry must distinguish "empty because value is zero" from "empty because the variable never applied" (the proof in US2 scenario 3 must catch the latter).
- Screenshot/baseline refresh: re-collected evidence under the production policy replaces prior evidence without loosening any frozen baseline comparison rule.

## Domain Language *(canonical terms)*

- **Production CSP / shipped policy**: the exact content-security-policy string the production window attaches to the `crest://` page response (`src/shell/webview/window.rs`). There is one source of truth; the harness must serve it, not restate it.
- **Painted geometry**: the actual laid-out/painted fill or indicator size the user sees — distinct from **readout text**, which already renders correctly.
- **Typed page render failure**: the existing shell-side error variant (`PageRenderFailed`) reached via the `crest://render-error` channel; "typed" excludes string-matching on console output.
- Avoid: "relax the CSP", "temporary inline exception" — weakening the policy is out of bounds by constraint C-004.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | Requirement | User Story | Priority | Status |
|----|-------|-------------|------------|----------|--------|
| FR-001 | CSP-conformant geometry painting | Fader fill and position-indicator geometry is applied without JS-built inline `style` attributes (CSSOM `setProperty` or data-attribute + stylesheet binding), so it paints correctly under the production policy. | US1 | High | Open |
| FR-002 | Harness serves the production policy | The acceptance harness serves the page through the production response path (or attaches the identical policy from the same source of truth), asserted equal to what production ships. | US2 | High | Open |
| FR-003 | Affected proofs re-run under production policy | Determinism (T024), latency (NFR-002 here), and screenshot evidence are re-collected under the production policy and committed. | US2 | High | Open |
| FR-004 | Falsifiable paint-fidelity proof | A named proof fails if painted fader geometry stops reflecting the level variable under the shipped policy (kills both a regressed fix and a regressed harness). | US2 | High | Open |
| FR-005 | Page-side error boundary | The page emits `crest://render-error` with a typed payload on any render throw and on unhandled promise rejection; the false "already surfaces" comment is corrected. | US3 | High | Open |
| FR-006 | Typed nonzero exit on render failure | A forced render throw makes the shell exit nonzero with the typed page-render-failure error, proven by a falsifiable test. | US3 | High | Open |
| FR-007 | Gallery guard coverage | The no-input-handler scan and the style-literal scan include `gallery.js` and `gallery.css` in their scanned sets. | US4 | Medium | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Policy hardening, never weakening | The final policy contains no `unsafe-inline` (or weaker source) in any directive, is at least as restrictive as the current shipped policy, and adds `base-uri 'none'; form-action 'none'`. Verified by an executable check on the single policy source. | Security | High | Open |
| NFR-002 | Latency holds under production policy | Reducer state change visible in the webview within 50 ms p95 under the paced live demo workload (per `requirement.serialized_projection_transport`), measured with the production policy served. | Performance | High | Open |
| NFR-003 | Determinism holds under production policy | The rendered-document determinism proof (T024) produces identical output across repeated runs with the production policy served. | Reliability | High | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | Blast radius | Changes are page-, harness-, and transport-side only: no reducer, real-time, or projection schema changes. | Technical | High | Open |
| C-002 | Existing rules hold | All frozen baselines, the token single-source rule, and the input-boundary rules remain unchanged and passing. | Technical | High | Open |
| C-003 | Scope boundary | The mission review's structural/dead-code open items (RISK-3, RISK-4, RISK-5, DRIFT-3) are explicitly out of scope — they belong to a separate hygiene mission. | Process | High | Open |
| C-004 | CSP never weakened | No directive anywhere gains `unsafe-inline`, `unsafe-eval`, wildcard sources, or any loosening relative to the shipped policy. | Security | High | Open |

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Launching the shipped application shows every fader fill and position indicator matching its readout value, at all sixteen tracks, under the policy the production window enforces (spot-checkable at any value, including 0 and max).
- **SC-002**: A forced page render failure ends the application with a typed nonzero exit in 100% of forced-failure test runs — never a silently stale display.
- **SC-003**: All affected acceptance evidence (determinism, latency, screenshots) exists re-collected under the production policy, and a deliberate paint-fidelity regression fails a named proof.
- **SC-004**: Gallery sources are inside both executable guard scans; introducing a violation there fails the suite.
- **SC-005**: Re-running the mission-review checks for RISK-1, RISK-2, DRIFT-1, and DRIFT-2 finds all four resolved, with no new findings introduced in the touched seams.

## Assumptions

- The mission-review report (`kitty-specs/webview-shell-cutover-01KZAC7Q/mission-review.md`) is accurate as of commit `f782d15`; its file/line references are the authoritative defect locations.
- CSSOM `style.setProperty` is exempt from `style-src` inline-attribute blocking (standard CSP semantics); if a WKWebView deviation surfaces, the data-attribute + stylesheet binding alternative is the fallback within the same requirement.
- The `crest://render-error` emission uses the same page→shell channel class as the existing ack emission; US3 scenario 3 exists precisely to verify rather than assume it survives the production policy.
- "Re-run the affected proofs" means refreshing the committed evidence artifacts in the same locations/conventions the cutover mission established, not inventing a new evidence scheme.
