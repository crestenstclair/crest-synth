## 1. Baseline and Reducer Acceptance

- [x] 1.1 Preserve the existing user changes in `AGENTS.md` and `DESIGN.md`, record the implementation baseline, and add failing acceptance coverage before product edits.
- [x] 1.2 Prove Instrument Detail opens from the Engine origin through `AppState::apply` with the active instrument subject, one Detail focus, and an exact Engine return origin.
- [x] 1.3 Prove FX Detail opens through `AppState::apply` from every occupied canonical effect-slot position, including duplicate effect capabilities whose stable subjects and origins remain position-specific.
- [x] 1.4 Prove an unoccupied slot advertises no valid Detail action and cannot project a fabricated FX Detail subject.
- [x] 1.5 Prove Shift+Down restores the exact Engine or slot origin and that schema-change return repair selects the nearest enabled semantic sibling deterministically with visible status.

## 2. Descriptor and Canonical Projection Contract

- [x] 2.1 Add instrument fixtures with different descriptor section/control shapes and assert descriptor order, canonical shared-envelope placement, and one shared projection path.
- [x] 2.2 Add effect fixtures with different section shapes and parameter counts and assert descriptor order, exact slot subject identity, and one shared projection path.
- [x] 2.3 Assert Detail parameter identities are absent from Patch Overview focus membership while remaining visible, enabled, and reachable in the subordinate Detail order.
- [x] 2.4 Prove the existing Patch summary, Detail summary, return origin, sections, controls, lifecycle/error/requested fields, and valid actions supply every Detail presentation fact; if one fact is missing, add only the smallest generic field to the existing canonical view model.
- [x] 2.5 Add exact serialization tests for Detail subject kind, stable effect-slot identity, canonical slot position source, Patch/subject labels, ordered section/control paths, units/ranges, lifecycle/error/requested state, and any generic field added by 2.4.
- [x] 2.6 Assert requested, active, loading/validating/preparing/activating, ready, unavailable/disabled, and typed-failure states remain distinct and never select a substitute capability or fallback value.

## 3. Shared Instrument and FX Detail Renderer

- [x] 3.1 Refine `detailShellHtml` into one shared production composition that reads subject kind, exact origin, Patch identity, display labels, status, and projected control count without capability-name or DOM-index inference.
- [x] 3.2 Implement the Figma Detail header and section hierarchy with projected section IDs/labels/counts and rows in canonical projected order.
- [x] 3.3 Extend the shared parameter row to present active value, optional unit, projected bounds, normalized position, declared interaction, state marker, and the row's exact projected valid-action hints.
- [x] 3.4 Extend the shared lifecycle area so active and requested readings, graph/lifecycle state, unavailable/disabled state, and typed failure remain simultaneously explicit without optimistic substitution.
- [x] 3.5 Apply one non-color Detail state vocabulary for focused, adjusting, disabled, unavailable, read-only, loading/preparing, and failed controls, using text plus keyline/shape markers.
- [x] 3.6 Keep projected envelope/waveform/status visualizations non-focusable and descriptor-driven, and ensure absent visualizations do not create placeholders or focus targets.
- [x] 3.7 Preserve the projected breadcrumb, footer actions, persistent PATCH Utility, and immutable DOM adapter path for both Detail subjects.

## 4. Responsive Detail Composition

- [x] 4.1 Reuse the current responsive shell and shared tokens to compose Detail with Grid/Flex, intrinsic sizing, `minmax()`, `clamp()`, wrapping, and bounded tracks/gaps rather than a fixed or scaled canvas.
- [x] 4.2 Make Wide and Standard Detail layouts use available horizontal space for readable label/rail/value/action rows while keeping the Utility track bounded and independently reachable.
- [x] 4.3 Make Compact Detail stack main content then Utility in semantic document order with one DOM and unchanged focus-path identities.
- [x] 4.4 Keep Detail content independently scrollable, reveal the focused semantic node after reflow, and prevent document-level horizontal overflow, required-content clipping, or overlap.
- [x] 4.5 Preserve the 48px interactive-target floor while allowing rows, headers, lifecycle text, long labels, and action hints to grow/wrap under compact and enlarged-text conditions.
- [x] 4.6 Assert layout mode remains presentation-only: resize, wrap, scroll, and mode changes emit no semantic action, invoke no `AppState::apply`, and add no viewport state to the projection.

## 5. Controller, Focus, and Native Input Evidence

- [x] 5.1 Drive unmodified arrows across every projected Detail section and between Detail and Utility through semantic resolver adjacency, asserting exactly one visible focused target after each action.
- [x] 5.2 Drive representative Edit+Left/Right fine edits, Edit+Up/Down coarse edits, adjacent choices, confirms, and toggles only where the projected valid-action list admits them.
- [x] 5.3 Drive Shift+Up entry and Shift+Down close through the production physical-input → semantic action/event → `AppState::apply` path for Instrument Detail and every occupied FX slot.
- [x] 5.4 Retain and run the macOS regression proving AppKit `FlagsChanged` modifier transitions are non-repeatable and never query the key-repeat property or unwind through the native callback boundary.

## 6. Reducer-to-DOM and Responsive Measurements

- [x] 6.1 Extend `renderObservation` to report Detail subject kind, Patch identity, canonical slot position, section order, row identity/state/value/unit/range/position/hints, lifecycle readings, visualization focusability, and focused visible identity.
- [x] 6.2 Extend observations with workspace/Utility bounds, target sizes, overlap, document/region overflow, independent scroll reachability, computed layout mode, and paint acknowledgement.
- [x] 6.3 Add reducer-to-DOM fixtures for multiple instrument shapes, multiple effect shapes/counts, all occupied positions, duplicate effects, an empty slot, long content, and requested/loading/preparing/failed/unavailable/disabled states.
- [x] 6.4 Exercise a wide desktop condition, 1440px, 1280×800, an intermediate width, a compact width, and 1280×800 enlarged text; assert unchanged generation, focus path, return path, subject, surfaces, and projected controls across reflow.
- [x] 6.5 Prove exactly one focused visible target, the 48px floor, no required-content overlap/document-level horizontal overflow, and complete scroll reachability for every Detail fixture and viewport.
- [x] 6.6 Render the same Detail projection repeatedly at a fixed viewport and assert identical structural observations and render-acknowledgement identity.
- [x] 6.7 Prove resize-only observations emit zero semantic events and zero reducer applications, then retain all sixteen Mixer tracks, Inspector correlation, focus identity, and reachability as non-regression evidence.

## 7. Durable Documentation and Scope Guardrails

- [x] 7.1 Run or extend the repository's no-name-enumeration checks so Rust/JavaScript/CSS contain no capability-specific Detail schema, label, branch, or fixed fixture count.
- [x] 7.2 Confirm implementation changes do not enter the audio callback, alter real-time transports, graph preparation/activation/retirement, capability availability, persistence, asset/device ports, or Mixer product semantics.
- [x] 7.3 After production-path evidence passes, update `DESIGN.md` with the as-built Instrument/FX Detail visual status, any actual generic projection addition, verified responsive/controller behavior, native exclusions, and remaining Sample, Browser, option-state, Mixer, and native-polish gaps without claiming broader Figma parity.

## 8. Validation and Manual Handoff

- [x] 8.1 Run `cargo fmt --all -- --check`, `node --check webview-page/page.js`, `cargo clippy --all-targets -- -D warnings`, and `git diff --check` with no failures.
- [x] 8.2 Run `cargo test --lib input_capture -- --nocapture`, `cargo test --test semantic_focus_and_projection`, `cargo test --test semantic_graphical_view_model`, `cargo test --test patch_page_projection`, and `cargo test --test webview_projection_shell -- --nocapture` with no deterministic failures.
- [x] 8.3 Run broader deterministic validation, including `cargo test --all-targets` and repository exact/no-name guards when practical, and record every native or environment-dependent exclusion honestly.
- [x] 8.4 Run `cargo run --bin crest-synth` and manually verify physical keyboard/controller entry, navigation, representative fine/coarse edits, exact Engine/slot return, resize invariance while Detail is open, and repeated Shift use without a crash.
- [x] 8.5 Compare Instrument Detail and FX Detail directly with the live Figma hierarchy, shared visual grammar, state treatments, responsive composition, and interaction map; record that remaining visual slices are Sample Detail/Browser, option states, Mixer composition, and native polish.
