## 1. Guarded Baseline and Focused Acceptance

- [x] 1.1 Add a dedicated `engine_post_fx_option_states` integration target plus cache-guarded Make targets for focused deterministic and native option witnesses; verify `make -n test-engine-post-fx-options` and `make -n test-webview-options-native` both include the repository cache guard before Cargo.
- [x] 1.2 Add focused reducer tests for Engine entry and every canonical occupied/empty Post FX slot, including duplicate effect capabilities, and verify them with `scripts/check_build_cache_size.sh --guard && bash scripts/run_exact_test_validation.sh engine_post_fx_option_states <exact-entry-test> <marker>`.
- [x] 1.3 Add focused tests for registry-derived identities, labels, order, enabled/availability state, counts, the canonical Empty choice, current-option initial focus, deterministic first-enabled repair, and no-options rejection; verify the exact tests through the guarded validation script before product edits.
- [x] 1.4 Add focused tests for non-wrapping modal navigation, projected valid actions, current-option no-op choose, changed-option request choose, disabled activation rejection, Shift+Down unchanged close, and spatial escape rejection; verify each exact test through the guarded validation script.
- [x] 1.5 Add focused tests for exact Engine/slot return and deterministic Choice-origin repair after the origin is removed or disabled; verify the tests distinguish exact return, repaired return, and unchanged rejection through `AppState::apply`.
- [x] 1.6 Add focused Engine and slot lifecycle fixtures covering Ready, Loading, Validating, Preparing, Activating, Unavailable, and every representative typed Failed outcome; verify each fixture retains acknowledged active and requested readings until matching activation acknowledgement.

## 2. Reducer, Resolver, and Stable Identity

- [x] 2.1 Refine the existing reducer-owned `PatchChoice` entry only where the focused tests require it, preserving Engine and exact `EffectSlot(index)` subjects for occupied, empty, and duplicate-capability slots; verify the dedicated focused target passes without a second option-state model.
- [x] 2.2 Make canonical visible option membership and enabled focus membership explicit when registry fixtures include disabled/unavailable entries, while preserving installed-registry order and the canonical Empty choice; verify disabled rows cannot focus or activate and no JavaScript or fixture list participates.
- [x] 2.3 Preserve immediate Edit-to-choose semantics: current closes without a request, changed Engine/occupancy emits exactly one correlated request, and Shift+Down closes without choosing; verify accepted generations/effects and rejected-state equality in focused reducer tests.
- [x] 2.4 Extend the existing stable-order repair path to Choice return origins if the new repair test exposes a gap; verify Engine and every slot return exactly when live and report removed/replacement identities when repaired.
- [x] 2.5 Resolve per-option and focused-footer actions from the production reducer for the exact row and lifecycle state; verify projected Activate, Up/Down, and Return availability matches `accepts_semantic_action` and busy/disabled actions are absent.
- [x] 2.6 Confirm transient Choice/request work adds no saved-session fields or parallel public Patch/focus/lifecycle types; verify existing saved-session round-trip/schema tests and the canonical-type/no-name guards pass through guarded targets.

## 3. Acknowledged Active and Structural Lifecycle

- [x] 3.1 Prove a changed Engine option leaves the acknowledged Engine/configuration and source graph revision canonical through Loading, Validating, Preparing, and Activating; verify focused lifecycle tests observe the requested Engine separately at every phase.
- [x] 3.2 Prove a changed or Empty Post FX option leaves the exact slot's acknowledged occupant/empty state canonical through Loading, Validating, Preparing, and Activating; verify duplicate-capability slots remain independently correlated by stable slot identity.
- [x] 3.3 Preserve commit-at-acknowledgement behavior for Engine and slot occupancy so only a matching activation acknowledgement changes current/active and returns lifecycle to Ready; verify early, stale, mismatched, and uncollected acknowledgements reject unchanged.
- [x] 3.4 Preserve Unavailable and Failed as distinct typed terminal outcomes retaining active/requested/cause with no capability or Empty fallback; verify representative provider, asset, preparation, capacity, and compatibility failures remain attributable to the exact origin.
- [x] 3.5 Verify the option slice does not alter prepared-graph construction, scalar snapshot revision routing, block-boundary activation, or off-thread retirement; run the existing guarded topology/engine lifecycle targets and record no changed real-time contract.

## 4. Canonical Projection and Serialization

- [x] 4.1 Audit the existing semantic surfaces, Choice summary, exact return origin, Overview origin control, option controls, lifecycle/error/requested fields, focus repair, and valid actions against every required presentation fact; verify the audit with an exact serialized fixture before adding a field.
- [x] 4.2 If 4.1 proves a fact is missing, add only the smallest generic field to the existing summary/control/status vocabulary and verify Engine, Post FX, other generic choices, and Sample Browser serialize one coherent schema without a new capability-specific view model.
- [x] 4.3 Add exact Engine Options serialization fixtures for Patch/origin identity, registry order/count, stable row IDs, display labels, focused/current/enabled state, valid actions, active/requested lifecycle, typed error, and focus repair; verify byte-stable JSON leaf names and deterministic ordering.
- [x] 4.4 Add exact Post FX Options fixtures for every slot, occupied and Empty origins, duplicate capabilities, canonical Empty plus registry order, disabled/unavailable rows, changed/terminal lifecycle, and repair; verify slot position never comes from label, DOM order, or repeated capability identity.
- [x] 4.5 Ensure requested labels and display labels come from the same installed descriptors used by the reducer and raw registry IDs do not leak as visible names; verify the projected-screen-string and no-name-enumeration guards cover both active and requested values.
- [x] 4.6 Verify exactly one visible/focusable option matches the model focus, `CURRENT` remains independent of focus, and every visible row's action set matches a reducer counterfactual; run the guarded semantic graphical model target before renderer work.

## 5. Shared Figma-Aligned Option Renderer

- [x] 5.1 Refine the committed option branch of `modalShellHtml` (or a shared option helper) into one Engine/Post FX composition that reads only projected subject/origin facts; verify Engine and Post FX use the same DOM hierarchy with no concrete capability-name branch.
- [x] 5.2 Implement the authored option identity header, entry/source annotation, separator, ordered rows, exact slot annotation for Post FX, and projected footer guidance while preserving the existing context, Patch identity, Utility, and shell footer; verify headless observations match the serialized hierarchy.
- [x] 5.3 Render each row with a structural focus pointer, independent current marker, stable focus-path identity, authored label, explicit available/disabled/unavailable state, and only projected actions; verify focus can differ from current with exactly one of each applicable marker.
- [x] 5.4 Render the exact origin-anchored active/requested/lifecycle/revision/failure facts for Loading, Validating, Preparing, Activating, Ready, Unavailable, and Failed without optimistic current state or fallback; verify every lifecycle fixture produces distinct DOM text and structural state.
- [x] 5.5 Apply token-backed non-color treatments for focus, current selection, disabled, unavailable, loading, and failure using keylines, shapes, and explicit text; verify observation assertions can distinguish every state with computed color removed from the comparison.
- [x] 5.6 Keep Sample Browser on its existing behavior and composition branch; verify focused browser tests retain folder/file/cancel rows, preview lifecycle, waveform, hold/release input, exact return, and visible focus after shared helper changes.

## 6. Responsive DOM and Measured Webview Evidence

- [x] 6.1 Compose option header/status/list/footer inside the existing responsive shell with one semantic DOM, intrinsic sizing, `minmax()`, `clamp()`, wrapping, token bounds, and the persistent Utility region; verify DOM node identities and projected paths are identical across layout modes.
- [x] 6.2 Replace hidden option-list overflow with a bounded independent scrolling region whose first and last registry rows are reachable while every interactive row retains a 48px minimum target; verify start/end scroll observations for short, long, and maximum-registry fixtures.
- [x] 6.3 Allow long labels, state/cause text, and action hints to wrap and grow without required-content or descendant-painted overlap and without document-level horizontal overflow; verify Compact and 1280×800 enlarged-text geometry observations.
- [x] 6.4 Reveal the focused option after navigation or presentation reflow without using scroll position as focus authority; verify reveal changes only list scroll offsets and emits zero semantic action or reducer application.
- [x] 6.5 Extend `renderObservation` with option subject/origin/slot, ordered row identity/label/state/bounds, focused/current markers, active/requested lifecycle and cause, valid actions, list scroll endpoints, workspace/Utility bounds, target floors, overlap/overflow, and paint acknowledgement; verify observation-to-projection correlation for both subjects.
- [x] 6.6 Add Wide, Standard, Intermediate, Compact, 1280×800, and 1280×800 enlarged-text fixtures with long labels and maximum registry content; verify every condition preserves one focus, all rows/Utility reachability, 48px floors, zero required overlap, and zero document horizontal overflow.
- [x] 6.7 Render each representative projection repeatedly at a fixed viewport and run one resize-only sequence across every mode; verify deterministic structural observations/acknowledgement identity and unchanged generation, subject, focus, return, option order, current, lifecycle, and actions.

## 7. Native Input and Regression Proof

- [x] 7.1 Extend production keyboard/controller translation tests for Edit+Up entry, unmodified Up/Down, Edit activation, and Shift+Down close without adding a browser-specific or DOM input path; verify physical gestures normalize to the existing semantic actions.
- [x] 7.2 Retain the macOS `FlagsChanged` regression proving Shift transitions are non-repeatable, never query key-repeat state, and cannot unwind through the native boundary; verify the guarded focused input-capture target passes with repeated Shift gestures.
- [x] 7.3 Extend the native webview witness to drive Engine Options plus every occupied/empty Post FX origin, including duplicate effects, current/no-op choose, changed request, close, exact return, and repaired return; verify native observations correlate input, accepted generation, projection, and paint.
- [x] 7.4 Keep Instrument/FX Detail and Mixer in the native option run as non-regression scenes; verify exact Detail subjects/returns, all sixteen Mixer tracks, Inspector correlation, singular focus, overflow bounds, and complete reachability remain unchanged.
- [ ] 7.5 Add a bounded physical keyboard/controller handoff script or checklist for entry, navigation, choose, unchanged close, occupied/empty slot return, resize invariance, and repeated Shift; verify the real application exits cleanly and records every required observation rather than treating an environmental skip as acceptance.

## 8. Guarded Validation, Figma Review, and Durable Status

- [x] 8.1 Run focused deterministic validation first with `make test-engine-post-fx-options`, `scripts/check_build_cache_size.sh --guard && bash scripts/run_exact_test_validation.sh semantic_graphical_view_model <exact-option-test> <marker>`, and `node --check webview-page/page.js`; verify no focused failure remains before any broad suite.
- [x] 8.2 Run formatting and static validation with `scripts/check_build_cache_size.sh --guard && make fmt-check`, `make lint`, `git diff --check`, and `scripts/check_no_name_enumerated_identity.sh`; verify there are no warnings, schema/name branches, fixed Figma counts, or formatting errors.
- [x] 8.3 Run broader deterministic validation only after 8.1–8.2 with `make test` and the repository exact-validation self-test; verify all targets pass and record every typed environmental exclusion honestly.
- [x] 8.4 Run `make test-webview-options-native` on an interactive macOS host and inspect readable captures at Wide, Standard, Intermediate, Compact, 1280×800, and enlarged text; verify native paint, geometry, scroll, repeat-render, resize-neutrality, Detail, and Mixer predicates all pass.
- [ ] 8.5 Run the bounded physical handoff from 7.5 in the production native application; verify keyboard/controller entry, navigation, choose/close, exact Engine/slot return, resize invariance, repeated Shift, and clean teardown without claiming completion for an unavailable device/host.
- [x] 8.6 Compare live Figma Engine Options `48:173`, Post FX Options `48:207`, Interaction Map `49:3`, Patch Overview `95:202`, and Responsive Contract `98:2` directly against the native hierarchy, state treatments, action grammar, and responsive composition; record scoped discrepancies and do not claim Sample, Mixer-composition, or broad native parity.
- [x] 8.7 Confirm the final diff contains no saved-session change, audio callback work, real-time transport or graph-preparation redesign, asset/device change, capability substitution, Mixer semantics, or Sample redesign; verify targeted architecture guards and affected-file review.
- [ ] 8.8 After all production-path and native evidence passes, update `DESIGN.md` with the actual Engine/Post FX option as-built baseline, any generic projection additions, measured responsive/input evidence, honest exclusions, and remaining visual gaps; verify the document cites production evidence rather than OpenSpec as proof.
