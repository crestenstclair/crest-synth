## 1. Durable Contract and Baseline

- [x] 1.1 Update `DESIGN.md` to name Figma nodes `98:2`, `95:202`, and `49:3`, replace the fixed-canvas density language with the fluid responsive contract, and record Patch Overview as the PATCH root.
- [x] 1.2 Record the current deterministic reducer, projector, webview, and native-window baseline commands and confirm unrelated user work is preserved before implementation edits.
- [x] 1.3 Add failing acceptance assertions for Patch Overview section anatomy and presentation-only reflow before replacing the shipped renderer.

## 2. Patch Main Semantic Navigation

- [x] 2.1 Change `SemanticResolver::patch_main_paths` so Patch Main exposes Engine and the three canonical Effect Slot occupancy controls in semantic order while descriptor/envelope/effect parameters remain on Detail.
- [x] 2.2 Update PATCH-to-Utility adjacency and subordinate return handling so Engine, ordered Post FX slots, and Utility remain reachable with one stable focus and deterministic return repair.
- [x] 2.3 Exercise Engine Choice, Engine Detail, Post FX Choice, occupied-slot Detail, Back, and sibling-Patch navigation through `AppState::apply` using the new overview roots.
- [x] 2.4 Update reducer/focus tests to prove removed strip parameters cannot remain hidden Patch Main focus targets and all existing Detail parameter identities remain reachable.

## 3. Canonical Patch Overview Projection

- [x] 3.1 Generalize `SemanticDetailSectionViewModel` into one canonical surface-section view model usable by Patch Main and Detail without creating a parallel overview model.
- [x] 3.2 Populate Patch Main with ordered Engine and Post FX section metadata whose control paths resolve to controls in the same projection; include all three canonical slot positions.
- [x] 3.3 Project descriptor-derived Engine/effect parameter summary data needed by the overview without enumerating capability names or duplicating registry schema in JavaScript.
- [x] 3.4 Preserve active/requested values, lifecycle status, typed errors, empty state, editability, valid actions, and Patch identity on every overview control.
- [x] 3.5 Add projector/serialization tests for multiple instrument/effect descriptors, empty and occupied slot combinations, section/control-path coherence, and one focused visible target.

## 4. Responsive Shell Contract and Tokens

- [x] 4.1 Replace the closed Desktop/SteamDeck production geometry policy with named minimum/preferred/maximum layout bounds plus representative viewport fixtures used only by witnesses and acceptance.
- [x] 4.2 Define Wide, Standard, and Compact structural thresholds once in the Rust shell/token contract and ensure no layout mode is added to `AppState` or the semantic projection.
- [x] 4.3 Update `token_export.rs` to emit bounded shell tracks and responsive structural rules/properties; regenerate `webview-page/tokens.css` and remove obsolete fixed split/column geometry exports where no longer consumed.
- [x] 4.4 Update window sizing/reference helpers to retain 1920×1080 and 1280×800 witness entry points without treating either viewport as production geometry authority.
- [x] 4.5 Replace density/token tests with assertions for bounds, threshold ordering, 48px target floors, representative fixtures, and single-source generated CSS.

## 5. Responsive Webview Shell

- [x] 5.1 Convert `#main-region` and its child regions to CSS Grid/Flex composition with intrinsic sizing, `minmax()`, `clamp()`, wrapping, and bounded gaps.
- [x] 5.2 Implement Wide and Standard two-track composition and Compact ordered stacking while retaining one DOM structure, the five region IDs, and unchanged `data-focus-path` identities.
- [x] 5.3 Make workspace/Utility scrolling and `revealSemanticFocus` keep the focused semantic element visible after reflow without dispatching resize-derived actions.
- [x] 5.4 Preserve the existing Mixer blockout inside the new shell and prove all sixteen tracks and Inspector remain present and reachable pending the later Mixer visual slice.
- [x] 5.5 Remove comments and CSS assumptions that describe the two reference viewports as the only authored sizes or require coordinate reproduction.

## 6. Patch Overview Renderer

- [x] 6.1 Replace the resting `patchStripHtml` branch with `patchOverviewHtml` that iterates projected sections and control paths rather than local capability names or labels.
- [x] 6.2 Build the Engine overview region with projected identity, descriptor-derived parameter summary, lifecycle/requested/active readings, focus treatment, and valid-action hints.
- [x] 6.3 Build the ordered Post FX region with all three slot controls, explicit empty/occupied state, descriptor-derived summaries, lifecycle/error readings, and valid-action hints.
- [x] 6.4 Add token-bound responsive Overview styling that follows the Figma hierarchy and state grammar, preserves 48px targets, and contains no placeholders.
- [x] 6.5 Keep persistent Utility rendering projection-driven and verify its five controls remain unchanged across Overview, Detail, Choice, Browser, and all layout modes.

## 7. Production-Path Automated Evidence

- [x] 7.1 Extend `renderObservation` to report computed layout mode, region bounds/order, Patch Overview sections/controls, focused identity, target sizes, scroll reachability, and overflow/overlap results from painted DOM.
- [x] 7.2 Render one accepted projection at Wide, Standard, Compact, 1280×800, and at least one intermediate width; assert unchanged generation, focus, return path, surfaces, and projected controls across reflow.
- [x] 7.3 Add long-label, maximum registry-content, empty/occupied slot, loading, failure, and scaled-text/display fixtures; assert legibility, explicit state, target floors, and no required-content overlap or clipping.
- [x] 7.4 Prove deterministic repeated rendering at a fixed projection/viewport and prove resize/reflow emits no semantic event or `AppState::apply` mutation.
- [x] 7.5 Run focused reducer, semantic projection, shell event dispatch, component vocabulary, webview projection, and no-name-enumeration test suites; fix regressions without weakening invariants.

## 8. Native Acceptance and Handoff

- [x] 8.1 Run native-window witnesses at 1920×1080, 1280×800, an intermediate width, and a scaled-display/text condition; capture paint acknowledgement and structural observations for each.
- [ ] 8.2 Visually review Patch Overview and shell composition against the Figma responsive contract for hierarchy, state treatment, wrapping, reachability, and bounded scaling rather than coordinate identity.
- [x] 8.3 Run the repository's aggregate deterministic validation and relevant native input/window/teardown checks, documenting any environment-dependent exclusions truthfully.
- [x] 8.4 Update the as-built alignment/evidence sections of `DESIGN.md` with completed production-path proof, remaining subordinate/Mixer visual gaps, and links to the next vertical slices.
