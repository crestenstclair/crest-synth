## Context

The production reducer, semantic resolver, and graphical projection already support one subordinate `PatchDetail` surface whose `PatchDetailSubject` is either the active Patch instrument or one exact occupied effect slot. The installed instrument/effect registries own ordered descriptor sections and parameters; `SemanticSurfaceSectionViewModel`, `SemanticControlViewModel`, and `SemanticVisualizationViewModel` serialize those facts to the webview. `page.js` already renders both subjects through `detailShellHtml`, but the present composition is a functional blockout and its production-path observations prove only a narrow subset of the authored Detail hierarchy and responsive behavior.

The live Figma Responsive Front-End Contract (`98:2`), Instrument Detail (`37:7`), FX Detail (`38:60`), Interaction Map (`49:3`), and Patch Overview (`95:202`) were inspected before this design was written. They establish a shared compact-parameter grammar inside the existing shell: Patch identity remains visible, Detail has a clear subject header, descriptor sections and flat parameter rows preserve order, lifecycle and control states remain explicit, Utility persists, and the breadcrumb/footer exposes only valid controller actions. Their wide frame dimensions and example capability names/counts are calibration fixtures, not production data or fixed coordinates.

`DESIGN.md` remains the as-built architecture authority. This change must preserve the physical-input → semantic action/event → `AppState::apply` → semantic view/audio projections path, singular stable focus, reducer-owned subordinate and return sessions, capability ports with typed failure and no substitution, and the hard real-time boundary. OpenSpec records scope and acceptance only; implementation evidence is recorded durably in `DESIGN.md` after it exists.

## Goals / Non-Goals

**Goals:**

- Implement Figma-aligned Instrument Detail and FX Detail through the existing production reducer, semantic projection, serialization, responsive shell, and committed webview renderer.
- Use one shared Detail composition for both canonical subject variants while retaining their meaningful identity differences: instrument versus exact effect-slot position.
- Render ordered descriptor sections, canonical controls, optional projected visualizations, values/units/bounds/position, lifecycle/failure state, and projected valid actions without a JavaScript schema or capability-name switch.
- Preserve exactly one semantic focus, exact origin return, deterministic return repair, persistent Utility, and presentation-only reflow at representative widths and enlarged text.
- Produce falsifiable reducer-to-DOM and native-window evidence, including deterministic repeat rendering, no-action resize proof, Shift modifier regression preservation, and Mixer non-regression.

**Non-Goals:**

- Redesign Sample Detail, Sample Browser, Engine/Post FX option modals, Mixer composition, or native visual polish beyond regressions caused by this slice.
- Define Select/multi-select behavior or change controller semantics.
- Add named SoundFont, Sample, Braids, Chorus, Reverb, Delay, or other capability branches, fixed parameter counts, or Figma fixture values to production code.
- Create a second Detail state model, DOM-owned navigation, coordinate focus, viewport state, or resize-derived product events.
- Change saved-session schema, capability availability, audio rendering, graph preparation, real-time transports, devices, assets, or callback work.
- Claim broader Figma parity from functional, structural, or reducer tests.

## Decisions

### 1. Keep one reducer-owned Detail surface and canonical subject union

`PatchSubordinateSession::Detail`, `SurfaceId::PatchDetail`, `PatchDetailSubject`, `FocusPath`, and `ReturnPath` remain the only authorities for whether Detail is open, which subject it represents, which target is focused, and where close returns. Instrument entry is admitted only from the Engine origin; effect entry is admitted only from an occupied canonical effect-slot origin. The effect subject retains its stable slot identity, while the return origin retains the canonical slot position. Two slots with the same effect capability therefore remain different Detail subjects, and an empty slot cannot manufacture one.

The webview may select instrument-versus-effect presentation roles from the serialized subject variant and exact origin path. It must not identify a subject from its label, capability name, DOM index, or visual order.

Alternatives considered:

- Separate Instrument Detail and FX Detail sessions/renderers were rejected because they duplicate navigation, focus, lifecycle, and return behavior already represented by one canonical union.
- A DOM-local selected slot or subject cache was rejected because it could diverge from reducer state on reprojection or resize.

### 2. Consume the existing generic projection before adding fields

The renderer will consume the existing projected Patch summary, Detail summary, return origin, ordered `SemanticSurfaceSectionViewModel` list, control paths, control values/ranges/units, lifecycle/error/requested fields, declared interactions, valid actions, and visualizations. Instrument parameter sections remain in descriptor order; shared envelope controls appear only through their canonical projected section; effect sections remain in descriptor order. Parameter counts are computed from projected section/control membership, never from a Figma fixture.

The current projection appears sufficient for the required identity and row anatomy: Patch Main supplies the active Patch name and originating Engine/slot label; `PatchDetailSubject` supplies subject kind and stable effect-slot identity; `ReturnPath::origin` supplies canonical effect position; controls carry value, unit, range, requested/active lifecycle, typed error, focus, editability, and valid actions. If implementation proves one visual fact cannot be expressed without adapter inference, extend the existing canonical summary/section/control type with the smallest generic descriptor-driven field and add serialization tests. No parallel `InstrumentDetailViewModel` or `FxDetailViewModel` is permitted.

Alternatives considered:

- Reconstructing section order, slot position, labels, or supported interactions in JavaScript was rejected because it would turn the webview into a second product model.
- Adding capability-specific presentation structs was rejected because differently shaped installed descriptors must share the same path.

### 3. Extend one shared Detail visual vocabulary

`detailShellHtml` remains the single composition and will be decomposed only into shared helpers for Detail identity/header, section heading, parameter row, lifecycle/status area, and optional non-focusable visualization. Instrument and effect treatments reuse the same row/state classes; semantic subject kind selects a generic tone/identity treatment, never a concrete capability name.

Every row presents its projected label, active value, optional unit, optional bounds, normalized position indicator where numeric, and its own projected valid-action hints. Structural lifecycle presents active and requested readings simultaneously and distinguishes loading/preparing/activating, ready, unavailable/disabled, and typed failure. Focus, adjusting, disabled, loading, failure, read-only, and unavailable use a keyline/shape or explicit marker plus text, as well as color. Optional envelope/waveform/status visualizations remain non-focusable and cannot create a second target.

Alternatives considered:

- Screen-specific CSS grammars were rejected because the authored screens intentionally share a compact parameter-row system.
- Copying the generated React/Tailwind reference was rejected because the production webview uses static JavaScript/HTML, repository CSS, and Rust-exported tokens.

### 4. Reuse the responsive shell with one semantic DOM

The existing context line, Patch identity band, Grid/Flex workspace, persistent Utility region, and footer remain intact. Detail owns an independently scrollable content region inside the main track. Wide and Standard modes use available horizontal space for readable label/rail/value/action composition and bounded Utility width. Compact mode stacks main Detail then Utility in document order; rows and header content wrap before clipping. `minmax()`, `clamp()`, intrinsic sizing, bounded token gaps/tracks, and the existing 48px interactive floor remain the layout vocabulary.

CSS/container presentation computes layout mode. JavaScript does not store Wide/Standard/Compact, rebuild a mode-specific product tree, dispatch a semantic event, or call `AppState::apply` in response to viewport changes. Identity-based reveal/scroll may run after paint because scrolling is presentation, not selection.

Alternatives considered:

- Scaling the wide Figma canvas was rejected because it breaks text and target floors.
- Separate compact markup was rejected because duplicate DOMs weaken singular-focus and deterministic-render guarantees.

### 5. Render reducer-resolved interactions, not inferred affordances

Physical keyboard/controller input continues to normalize before product logic. Unmodified arrows move semantic focus; Edit plus Left/Right or Up/Down adjusts according to the reducer's fine/coarse support; Edit confirms/toggles where supported; Shift+Up enters an admitted Detail from Patch Overview; Shift+Down closes it. The row hints and footer are direct presentations of projected `validActions`; the DOM does not infer availability from a label, control type, lifecycle color, or position.

At every accepted Detail state, exactly one projected visible target is focused. Close restores the exact Engine or effect-slot origin when live. If a schema change removes it, the existing deterministic return repair chooses the nearest enabled semantic sibling and reports the repair through projected status.

### 6. Extend measured production-path observations for Detail

The existing `renderObservation`/webview witness will report Detail subject kind, Patch identity, canonical effect position when applicable, section IDs/labels/order, row identity/state/value/unit/range/position/hints, lifecycle readings, visualization focusability, focused visible target, workspace/Utility bounds, target floors, overflow/overlap, independent scroll reachability, and paint acknowledgement. Assertions compare those observations with the exact serialized semantic projection rather than expected fixture strings.

Fixtures cover at least two instrument descriptors with different section/control shapes and at least two effect descriptors with different parameter counts, plus occupied slots at every position, an empty slot, requested/active/in-progress/failed/unavailable/disabled states, long content, and enlarged text. The same accepted projection is rendered at wide desktop, 1440px, 1280×800, an intermediate width, compact width, and 1280×800 enlarged text. Repeated rendering at one viewport must yield the same structural observation; resize-only runs must preserve generation, focus path, return path, subject, surfaces, and controls while recording no semantic action or reducer application.

Native execution remains necessary for final visual/controller evidence. Environment-dependent native exclusions are reported as exclusions, not converted into acceptance.

### 7. Preserve native Shift and unrelated product boundaries

The AppKit input adapter's `FlagsChanged` rule remains covered: modifier transitions are non-repeatable and never query the key-repeat property. Detail changes do not alter the input adapter unless a failing regression requires a narrowly scoped correction. Mixer projection/DOM reachability and semantic identity receive non-regression assertions, but Mixer visual redesign stays deferred. No task enters the audio callback or changes real-time transports.

`DESIGN.md` is updated only after implementation evidence exists, recording Instrument/FX Detail status, any actual generic projection addition, verified responsive behavior, production-path proof, honest native exclusions, and the remaining Sample, Browser, option-state, Mixer, and native-polish gaps.

### 8. Expand the reducer-owned engine lifecycle

The accepted decision in `decision-result.json` expands `EngineSelectionStatusKind` to distinguish Loading, Validating, Preparing, Activating, Ready, Unavailable, and Failed. An accepted structural request enters Loading. Worker-side progression advances the same correlated request through Validating and Preparing before preparation can complete. Unavailable is a typed, non-success terminal state for a requested provider or worker that cannot be supplied; Failed remains the terminal state for preparation, compatibility, capacity, and other typed failures. Neither state selects a substitute capability or value.

Every phase transition is an `AppEvent` applied through `AppState::apply`; a worker or shell adapter cannot mutate lifecycle state directly. The status retains one immutable correlation across Loading, Validating, Preparing, Activating, Unavailable, and Failed. This expands a canonical concept rather than reusing Sample Browser state or adding a DOM lifecycle.

### 9. Commit the prepared candidate only after graph acknowledgement

The accepted active-reading decision defines “active” as the configuration owned by the acknowledged audio graph. Preparing and Activating therefore retain the old canonical Patch/effect/return value and present the requested candidate separately. Complete candidate configuration/topology and candidate scalar parameters remain prepared and owned off callback. `EnginePrepared` records the pending control-side commit and stages the complete prepared graph, but does not replace canonical Patch or return state. `EngineActivationAcknowledged` verifies correlation and graph retirement, then atomically commits the pending candidate through `AppState::apply`, repairs semantic paths against the before/after orders, and advances lifecycle to Ready.

The callback contract is unchanged: it still swaps one fully prepared graph at a block boundary and retires the old graph off-thread. Failure before acknowledgement drops pending control-side candidate state and leaves both acknowledged product state and active graph identity unchanged. No persisted schema change is introduced because pending structural state is transient runtime state.

### 10. Switch between Detail and Utility through the reducer

The accepted navigation decision keeps Detail and Utility as mutually exclusive semantic surfaces while adding an explicit reducer-owned transition between them. Right from Detail enters Utility at its remembered/first enabled control while suspending the exact Detail subject, Detail focus, and original Overview return path. Left from Utility restores that suspended Detail session and focus. Left from Detail still closes to the exact live/repaired Overview origin; Right from Utility remains unavailable. The webview continues to project both visible regions from immutable state and never owns the transition or derives it from coordinates.

This extends `PatchSubordinateSession` with the smallest generic suspended-Detail/Utility state needed to preserve singular focus and exact return identity. It does not stack two active subordinate sessions or create a second focus.

### 11. Make Detail-origin repair reachable and visible

The accepted repair decision adds a reducer-owned schema transition that can disable a canonical Patch Overview origin while Detail or its suspended Utility session is open. The transition captures the old enabled main order, updates the enabled-origin schema, repairs remembered and return origins with `SemanticResolver::recover`, and exposes one generic projected repair status containing the removed origin and selected replacement. Closing Detail returns to that replacement. The notice is cleared by the next accepted semantic navigation/edit action after it has been projected; color is not its only signal.

Origin enablement is semantic schema state, not viewport state or capability-name logic. At least one Overview origin must remain enabled, the active Detail subject is never silently retargeted, and an origin that remains enabled retains its exact identity.

### 12. Keep native acceptance mandatory; use a scaffold only as support

The accepted native decision retains the production WKWebView and physical controller requirements. A standalone HTML scaffold may load committed `page.js`, `page.css`, tokens, and serialized mock fixtures to accelerate layout inspection and deterministic browser measurements, but scaffold evidence is supporting evidence only. It cannot complete the native paint, native input, real-window geometry, teardown, or manual controller tasks. Those tasks remain open until the exact native suite runs on a working interactive macOS host.

## Risks / Trade-offs

- [Risk] The existing Detail summary may not expose a presentation fact without awkward cross-surface lookup. → First use the canonical summary/return/control graph; if lookup would become semantic inference, add one generic field to the existing Rust model and test exact serialization.
- [Risk] Shared envelope placement could accidentally reorder descriptor-owned instrument sections. → Preserve descriptor section iteration verbatim and test section/control path correspondence separately from the canonical shared-envelope section.
- [Risk] Duplicate effect names could collapse slot identity in the renderer or witness. → Assert subject/return identity by stable IDs and canonical slot position while treating labels as display only.
- [Risk] Long action runs and lifecycle text can force rows below the 48px reference rhythm or create horizontal overflow. → Allow bounded wrapping and vertical growth, retain the minimum floor rather than a fixed height, and measure overlap/overflow/scroll reachability at every representative condition.
- [Risk] Structural lifecycle mappings could merge requested and active values or treat an unfamiliar phase as settled. → Render all projected lifecycle kinds explicitly, preserve active and requested readings, and show unknown/typed failures rather than falling back.
- [Risk] Scroll-to-focus can be mistaken for stateful navigation during reflow. → Keep identity unchanged, observe zero emitted action/`AppState::apply` calls, and assert scroll changes only presentation.
- [Risk] Global row/CSS changes could regress Utility, modal, or Mixer blockouts. → Scope shared-class changes carefully and retain Utility/modal/Mixer structural and geometry non-regression checks.
- [Trade-off] Automated structural evidence cannot by itself establish visual fidelity. → Require manual/native comparison to Figma and document any host limitations honestly.

## Migration Plan

1. Add or strengthen failing reducer/projection tests for subject admission, descriptor ordering, singular focus, return/repair, and lifecycle distinctions.
2. Extend the existing canonical projection only if required by the shared visual contract, with exact serialization coverage.
3. Refine the shared Detail renderer and CSS inside the existing responsive shell; keep Utility, footer, and one semantic DOM.
4. Extend headless/native render observations and representative viewport fixtures, then fix measured target, overlap, overflow, reachability, and determinism failures.
5. Run focused and broad deterministic validation, the native application/manual controller flow, and direct Figma comparison where the environment permits.
6. Update `DESIGN.md` with durable results and remaining gaps only after proof passes.

There is no persisted-data migration. Rollback is a revert of this visual/projection/test slice; saved sessions, capability registries, audio graphs, and device configuration remain compatible.

## Open Questions

The lifecycle, activation reading, Detail/Utility transition, return repair, and native-proof questions were resolved by the final 2026-08-25 `decision-result.json`. Whether the existing serialized fields are sufficient for every Detail header/state reading remains resolved by production-path tests; any required addition must remain generic, descriptor-driven, and part of the existing canonical view model.
