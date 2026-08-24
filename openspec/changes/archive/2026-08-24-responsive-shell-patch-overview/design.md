## Context

The live Figma file now defines responsive composition as a fluid, content-driven front-end contract. Its 1920×1080 frames are wide reference compositions rather than fixed implementation targets. The repository still encodes a closed `ViewportDensityPolicy` with Desktop and SteamDeck geometry, exports those fixed splits into `tokens.css`, and renders PATCH at rest with `patchStripHtml`. That implementation is structurally functional but conflicts with the current visual hierarchy and workflow.

The change crosses reducer navigation, semantic projection, shell geometry, token export, webview composition, and production-path acceptance. It must preserve the architecture recorded in `DESIGN.md`: physical input resolves to semantic actions, `AppState::apply` remains the only product-state mutation path, views remain immutable projections, exactly one semantic focus survives reflow, installed registries determine content, and audio/real-time boundaries remain untouched.

The authoritative Figma references for this slice are the Responsive Front-End Contract (`98:2`), Patch Overview (`95:202`), and Interaction Map (`49:3`). `DESIGN.md` remains the durable repository authority and is updated in the implementation commit; these change artifacts are temporary implementation guidance.

## Goals / Non-Goals

**Goals:**

- Replace fixed shell geometry with one bounded responsive composition system that supports Wide, Standard, and Compact presentation without changing product state.
- Make Patch Overview the PATCH resting/root composition with Engine, three ordered Post FX slots, and persistent Utility.
- Reuse canonical Patch control IDs, focus paths, valid actions, capability descriptors, lifecycle state, and surface projections rather than creating a front-end-owned model.
- Keep every region and focus target reachable across representative widths and display/text scaling while retaining the 48px target floor.
- Produce falsifiable evidence through the production reducer, semantic projector, serialized webview document, DOM renderer, and native window path.
- Establish responsive shell and section primitives that later detail, browser, and Mixer slices can adopt.

**Non-Goals:**

- Redesign Instrument Detail, FX Detail, Sample Detail, Sample Browser, option modals, or Mixer content in this change.
- Define Mixer multi-select semantics.
- Introduce new capability names, fixed example counts, or renderer-side capability switches.
- Add a second application state, coordinate-based focus, resize actions, or viewport state to `AppState`.
- Change saved-session schema, audio rendering, real-time transports, graph preparation, capability adapters, assets, MIDI, or device ownership.
- Use coordinate comparison as visual acceptance; reference frames calibrate hierarchy, density, and behavior.

## Decisions

### 1. Layout mode is computed presentation, not product state

Wide, Standard, and Compact are CSS presentation outcomes. The reducer, semantic projection, serialized document, and focus path are identical for the same accepted generation at every width. The webview may expose the computed mode in render observations for testing, but it does not dispatch an action or store the mode in `AppState`.

CSS owns reflow because Grid and Flexbox can respond to the actual containing block, font metrics, and content. Rust continues to own named visual/layout bounds and representative viewport fixtures, not the currently painted mode.

Alternatives considered:

- Reducer-owned layout mode was rejected because resizing would mutate product state and could disturb focus/return identity.
- JavaScript resize listeners that rebuild different DOM trees were rejected because they create incidental presentation state and increase the chance that controls disappear or change identity.

### 2. Retain representative viewports but retire them as geometry authorities

The existing Desktop/SteamDeck policy currently supplies exact band, split, and control geometry to production CSS. It will be replaced by:

- named minimum, preferred, and maximum bounds for shell tracks and interactive geometry;
- representative viewport fixtures used only to open witnesses and parameterize acceptance runs; and
- generated CSS custom properties/rules consumed by the static composition stylesheet.

Wide and Standard use a flexible main track plus a bounded side track. Compact changes the workspace to one ordered column so main content and Utility remain present and reachable. Structural thresholds are defined once in the shell/token source and emitted with the generated CSS rather than duplicated across Rust, JavaScript, and handwritten CSS.

Alternatives considered:

- Uniformly scaling a 1920×1080 canvas was rejected because it breaks target floors and legibility.
- Keeping two discrete fixed policies was rejected because intermediate widths inherit arbitrary geometry and the implementation continues to treat reference frames as canvases.
- Hand-copying breakpoint numbers into `page.css` was rejected because token export would no longer be the single source of structural values.

### 3. The DOM keeps one semantic structure across layout modes

The five shell regions remain context line, identity header, main workspace, persistent side region, and footer. `#main-region` becomes a CSS Grid container. Wide and Standard use two tracks; Compact uses one track and ordered stacking. Region IDs, `data-focus-path` values, control DOM order within semantic sections, and serialized projection content do not change when CSS reflows.

`revealSemanticFocus` continues to locate the focused node by serialized focus identity after paint/reflow. Scrolling is presentation-only. No row index or rectangle becomes state.

Alternatives considered:

- Separate desktop and compact markup was rejected because it creates two renderers and weakens focus/structure parity.
- Hiding Utility in Compact was rejected because the persistent-side contract requires it to remain available.

### 4. Patch Main becomes a schema-backed overview surface

The PATCH root will no longer paint every envelope, capability, and effect parameter as a long strip. The reducer's Patch Main semantic order will contain the overview-level controls already represented by canonical identities:

1. Engine occupancy/type control.
2. Post FX slot 1 occupancy.
3. Post FX slot 2 occupancy.
4. Post FX slot 3 occupancy.
5. The existing transition into persistent Utility controls through semantic navigation.

Envelope, descriptor parameters, and effect parameters remain available on the existing Detail surface, where their current canonical IDs and descriptor-owned sections are reused. Patch Overview does not create duplicate controls for those parameters.

The projector will populate Patch Main section metadata instead of leaving `sections` empty. The currently detail-named section view model will be generalized to one canonical semantic surface-section type used by Main and Detail surfaces. Section metadata identifies Engine and each ordered Post FX slot through control paths; it contains no hardcoded capability name. The renderer uses this metadata and projected controls to paint summaries, counts, lifecycle, selected/current values, focus state, and valid-action hints.

Alternatives considered:

- Deriving all grouping from `PatchControlId` in `page.js` was rejected because the view adapter would own product composition.
- Adding a second Patch Overview JSON model was rejected because it would duplicate canonical controls and allow state/projection disagreement.
- Keeping hidden Patch Main parameter focus targets was rejected because focus could land on an unreachable control.

### 5. Existing semantic actions drive all overview interactions

Overview controls advertise only actions accepted by the existing semantic resolver for the current state. Engine and occupied effect slots can enter their existing Detail surface; Engine and Post FX occupancy controls can open their existing Choice modal; fine/coarse changes, selection, back, and sibling Patch navigation remain reducer-owned. Returning from a subordinate surface restores the stored semantic origin, with the existing deterministic repair behavior if a schema change invalidated it.

The implementation may adjust Patch Main adjacency and visible/focusable membership to match the new root, but it will not invent viewport-specific navigation or direct DOM mutations of product values.

### 6. Patch Overview rendering is registry-driven and explicit about absence/lifecycle

Engine identity and parameter summaries resolve from the installed instrument descriptor. Post FX slots are emitted in canonical slot order and resolve occupancy and parameter summaries from the installed effect registry. Empty, unavailable, loading, failed, requested, and active states remain explicit in text/shape as well as color. No sample, SoundFont, Braids, Chorus, Reverb, or Delay name controls composition.

### 7. Acceptance proves invariants, not screenshot coordinates

Automated evidence renders the same accepted semantic projection at representative Wide, Standard, Compact, and intermediate widths. Assertions cover:

- no semantic action or generation change caused by resize/reflow;
- identical focus and return identities across widths;
- one focused visible target;
- all Patch Overview and Utility controls present and reachable;
- no bounding-box overlap or clipping of required content;
- minimum interactive target height/width;
- section order, explicit empty/lifecycle state, and descriptor-driven counts; and
- deterministic DOM observations for a fixed projection and viewport.

Native evidence runs at 1920×1080, 1280×800, at least one intermediate width, and a scaled-display/text condition. The run records layout mode, region bounds, focused identity, overflow/overlap checks, and paint acknowledgement. Visual review compares composition, hierarchy, state treatment, and legibility to Figma.

## Risks / Trade-offs

- [Risk] Refactoring Patch Main focus order invalidates assumptions in reducer/projector tests. → Update the semantic resolver first, prove the new order through `AppState::apply`, and retain canonical IDs and deterministic return repair.
- [Risk] Compact stacking can place the focused Utility control outside the current scrollport. → Keep one DOM, use identity-based `scrollIntoView`, and assert the focused element is visible after every tested reflow.
- [Risk] Global shell changes regress the existing Mixer blockout before the later Mixer redesign. → Preserve Mixer semantic content and add non-regression checks for sixteen tracks, Inspector presence, focus identity, and reachability at representative widths.
- [Risk] Structural thresholds become duplicated literals. → Define them in one Rust shell/token contract and generate the CSS rules/properties consumed by `page.css`.
- [Risk] Example parameter counts leak into the renderer. → Derive counts and labels only from projected descriptor sections/controls and test multiple installed capability fixtures.
- [Risk] Native WKWebView behavior differs from the headless DOM harness. → Require native paint/reflow witnesses before acceptance and keep rollback to the prior shell composition possible as one commit revert.
- [Trade-off] The first slice leaves subordinate screens and Mixer visually incomplete. → The shared shell and section primitives intentionally establish the base; later slices replace content without another layout architecture change.

## Migration Plan

1. Update `DESIGN.md` with the responsive contract and add the new Figma node references.
2. Change Patch Main semantic focus order and populate canonical surface sections; update reducer/projector tests before changing paint.
3. Replace fixed production geometry with bounded shell tokens and representative viewport fixtures; regenerate `tokens.css`.
4. Convert the shell to responsive Grid/Flex composition while retaining the same semantic DOM identities.
5. Replace the PATCH resting renderer with `patchOverviewHtml`; keep Detail, Choice, Browser, Utility, and Mixer paths on their existing projections.
6. Extend structural observations and automated viewport/reflow assertions.
7. Run deterministic test suites, native-window witnesses, and visual review against Figma.

There is no persisted-data migration. Rollback is a revert of this implementation slice; saved sessions, audio graphs, and device configuration are unchanged.

## Open Questions

- Compact Mixer content composition remains deferred to the Mixer responsive slice; this change only guarantees that the existing Mixer content remains reachable inside the new shell.
- Final threshold tuning may move within the named layout bounds after native evidence, but it may not change semantic order, target floors, or region presence.

## Implementation Baseline

Before product edits, the worktree contained only this untracked change directory. The existing deterministic focus and Patch projection suites passed, and the webview production-path suite compiled with:

- `cargo test --test semantic_focus_and_projection`
- `cargo test --test patch_page_projection`
- `cargo test --test webview_projection_shell --no-run`

The retained native entry point for the slice is `make demo-live-patch-editor`. Native viewport and scaling witnesses are recorded only after the automated production path is green.
