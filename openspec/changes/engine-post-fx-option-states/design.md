## Context

See `proposal.md` for motivation and the delta specs for observable requirements. This design is constrained by the live Figma file as the product, visual, and interaction authority and by `DESIGN.md` as the as-built architecture and invariant authority.

The live Engine Options (`48:173`) and Post FX Options (`48:207`) nodes share one authored hierarchy: an identity header with the `EDIT + UP` entry gesture, an origin annotation, a rule, ordered 48px option rows, one `>` focus shape, a separate `CURRENT` reading, and persistent D-pad/Edit/Shift+Down guidance. The Interaction Map (`49:3`) says Edit+Up opens the option modal, unmodified Up/Down moves, Edit chooses, and Shift+Down returns. Patch Overview (`95:202`) identifies the canonical Engine row and three ordered Post FX slot rows as the entry origins. The Responsive Front-End Contract (`98:2`) requires content-driven Wide/Standard/Compact composition, one stable focus and return identity, 48px target floors, reachable content, registry-driven counts, and presentation-only resize. These frames contain no prototype reactions; their hierarchy, annotations, and interaction map are the available design evidence, and their capability names and counts are fixtures.

The production baseline is more functional than visual:

- `PatchSubordinateSession::Choice` is the one generic reducer-owned modal state. Its `PatchChoiceSubject` contains the active `PatchId` and exact `PatchControlId`; an effect choice therefore already carries `EffectSlotIndex` rather than an effect label or dense DOM index.
- Edit+Up reaches `open_patch_choice` through physical-input normalization, semantic Adjust, and `AppState::apply`. Engine choices are resolved from the installed instrument descriptors. Slot choices are canonical `EMPTY` followed by the installed effect descriptors. The current option receives focus when enabled, otherwise the first enabled option does; modal Up/Down is non-wrapping and Left/Right is rejected.
- Edit activates immediately. Activating the current option closes with no structural request. Activating another Engine or slot occupancy starts one correlated structural request and closes to the exact origin. Shift+Down returns unchanged. The modal does not provide a second confirmation stage and does not cancel a structural request that was already submitted.
- The structural lifecycle already owns Loading, Validating, Preparing, Activating, Ready, Unavailable, and Failed. Patch Overview retains the acknowledged active Engine/slot value and projects the requested value separately; canonical configuration changes only after matching graph activation acknowledgement.
- `SemanticGraphicalViewModel` projects stable option paths, labels, identity values, enabled/focusable/editable flags, the `CURRENT` marker, focus, requested values, lifecycle/error data on the targeted origin, and reducer-resolved valid actions. The webview consumes this immutable JSON and the shell keeps Patch Utility present.
- The current `modalShellHtml` is shared with Sample Browser and paints a generic centered `SELECT OPTION` block. It marks focus/current/disabled textually, but it does not reproduce the option-specific Figma identity/origin/footer hierarchy, does not expose the complete option-result state vocabulary as an integrated composition, and its `.modal-options` region uses hidden overflow rather than proven bounded scrolling for maximum registry content. Existing reducer and blockout tests do not constitute responsive or visual acceptance for these nodes.
- The production native Detail/Mixer witness already measures several viewports, enlarged text, singular focus, 48px targets, overflow/overlap, scroll reachability, repeat rendering, resize neutrality, and paint acknowledgement. Option surfaces are not yet covered by an equally specific native matrix or physical handoff.

## Goals / Non-Goals

**Goals:**

- Reuse and tighten the existing generic Choice reducer/resolver/projection path rather than introducing Engine- or effect-specific modal state.
- Make the complete option transaction observable from canonical origin through request lifecycle and acknowledgement, including exact slot identity and typed failure without fallback.
- Give Engine and Post FX one shared option composition shaped by projected subject roles, registry content, and shell tokens.
- Extend the existing measurement and native-input seams with focused, falsifiable evidence before broad validation.

**Non-Goals:**

- Change Sample Detail/Browser behavior or composition; shared helper changes must preserve them as regressions only.
- Redefine Instrument/FX Detail, Mixer composition, Select/multi-select, or broad native polish.
- Add a second confirmation step, an in-modal draft selection, a request-cancellation command, viewport state, or DOM-owned focus.
- Change installed capability sets, persistence, asset/device ports, graph preparation/activation/retirement, real-time transports, or audio callback work.

## Decisions

### 1. Refine the existing Choice session and stable subject

`PatchSubordinateSession::Choice`, `PatchChoiceSubject`, `FocusPath`, and `ReturnPath` remain the only authorities for the open surface, its Patch and origin, one modal focus, and close destination. Engine Options is the subject whose control is canonical Engine. Post FX Options is the subject whose control is canonical `EffectSlot(index)`, including an empty slot. Duplicate effect capabilities therefore remain distinct automatically because the subject and request target are position-owned.

The renderer may select Engine-versus-Post-FX presentation from the projected generic subject/control variant. It must not compare capability IDs or labels, infer the slot from a row number, or cache a selected slot in JavaScript. If the current summary does not make the generic origin role or display annotation conveniently available, extend the existing Choice summary with the smallest stable subject/origin field rather than adding `EngineOptionsState` or `PostFxOptionsState`.

Alternatives considered:

- Separate Engine and Post FX reducers were rejected because they would duplicate modal navigation, return, and lifecycle behavior.
- Capability- or label-keyed slot identity was rejected because repeated effects and registry relabeling would collapse distinct positions.

### 2. Preserve immediate choose and unchanged close semantics

The Figma text and Interaction Map's “Edit chooses” resolve selection as one immediate activation, consistent with the current reducer. Opening focuses the current enabled option; if that cannot focus, canonical order supplies the first enabled option. Up/Down moves non-wrapping among reducer-resolved focusable rows. Edit on the current option closes with no request. Edit on a different enabled option starts exactly one admitted structural request and closes. Shift+Down closes without a request or configuration change.

Once Edit submits a structural request, the Choice session is over. Shift+Down is therefore modal cancellation only; it is not a new command for cancelling an already-submitted worker/graph request. The footer and per-row hints come from projected valid actions so busy, disabled, unavailable, or otherwise invalid activation is not advertised.

Alternatives considered:

- A focus-then-confirm-then-apply sequence was rejected because neither Figma nor the reducer defines the additional state or gesture.
- Treating Shift+Down as asynchronous request cancellation was rejected because it would materially change the existing structural lifecycle and graph contract.

### 3. Keep registry resolution as the only option schema

The semantic resolver continues to build Engine options from the installed instrument registry and Post FX options from the canonical empty occupancy plus installed effect registry. Stable IDs, labels, order, option count, and any enabled/unavailable decision travel through the canonical resolved option projection. JavaScript iterates projected controls and never defines capability names, labels, counts, effect palettes, or availability rules.

Visible option membership and focus membership remain distinct concepts. If canonical resolution supplies a disabled/unavailable option, projection may keep the row visible with `enabled=false`, an explicit reason/status, no focus path, and no Activate action; navigation still traverses only enabled stable paths. The existing option/control fields (`enabled`, `focusable`, `editable`, `status`, `error`, `selectedLabel`, and `validActions`) are used first. Add only a generic availability label/cause if a production fixture proves those fields cannot express the registry-owned fact.

Alternatives considered:

- Recreating the Figma lists in JavaScript was rejected because those names and counts are examples.
- Hiding every unavailable row was rejected as a universal policy because it makes an installed-but-unavailable registry fact indistinguishable from an uninstalled capability; the canonical resolver decides membership and presentation receives the result.

### 4. Treat the origin row as the transaction status anchor

Choice is a short-lived selection surface; the request continues after the modal closes. The canonical Engine or exact effect-slot Overview row remains the status anchor for the transaction. It displays acknowledged active, requested, lifecycle, target revision where present, and typed cause from the existing correlated state. The option surface may repeat projected origin status in its identity/status band when it is open after a terminal failure, but it must not manufacture a second lifecycle.

The established sequence remains Loading → Validating → Preparing → Activating → Ready, with Unavailable and Failed as typed terminal outcomes. The old configuration and acknowledged graph revision remain active throughout Activating. Only matching activation acknowledgement commits the prepared candidate and changes the `CURRENT` option. Unavailable or Failed retains the old current option and failed requested identity; no fallback or implicit Empty selection occurs.

Alternatives considered:

- Optimistically marking the chosen option current at Edit time was rejected because it contradicts the acknowledged audio graph.
- Keeping the modal open through graph activation was rejected because it changes the resolved interaction contract and conflates selection focus with request progress.

### 5. Reuse exact return and deterministic repair

Normal choose or close calls the existing reducer return transition and restores the stored Engine or slot origin. Reflow and scrolling do not touch it. Schema mutation while Choice is open uses the same stable-order recovery policy already used by Detail: recover the exact origin when live, otherwise choose the nearest enabled sibling deterministically and project the removed and replacement identities in `focusRepair`.

Implementation begins with a failing reachability test for Choice-origin repair. If current repair plumbing only covers Detail origins, it is extended generically in the existing interaction/repair path; it is not implemented by querying the DOM after close.

Alternatives considered:

- Falling back to Engine or the first slot unconditionally was rejected because it loses origin locality and makes schema changes nondeterministic.
- Letting the renderer choose a visible neighbor was rejected because geometry and layout cannot own product focus.

### 6. Build one option composition, not one per capability

The production webview retains one option renderer for both subject kinds. It consumes the projected surface label/summary, exact return origin, projected controls, selection/focus markers, status/error/requested fields, and valid actions. Generic subject role selects the authored Engine or Post FX identity wording and semantic tone. A concrete instrument/effect capability never selects markup or CSS.

The composition maps the live Figma hierarchy into the existing application shell:

1. option identity plus entry/source context;
2. origin annotation including exact slot position for Post FX;
3. ordered option rows with a structural focus pointer and independent current marker;
4. an optional projected lifecycle/failure band anchored to the origin;
5. the existing shell footer presenting only projected modal actions.

The existing context line, Patch identity band, workspace, persistent Utility, and global footer remain shared. The Figma node's footer guidance maps to the shell footer rather than being duplicated as a second action authority. Current, focused, disabled, unavailable, loading, and failure each receive explicit text and a keyline/shape treatment in addition to color.

The Sample Browser remains on its current branch. Shared row/helper changes are permitted only with targeted browser non-regression evidence; the change does not redesign its header, preview, waveform, or file behavior.

Alternatives considered:

- Two copied Engine/Post FX renderers were rejected because their differences are subject role and projected content, not separate behavior.
- Copying the generated React/Tailwind reference was rejected because the production renderer is static JavaScript/HTML with repository CSS and Rust-exported tokens.

### 7. Extend the existing projection only when a fact is missing

Implementation first demonstrates whether current surfaces already provide every necessary fact: Patch identity, Choice subject and origin, exact slot index, registry labels/order, focused/current/enabled state, requested/active readings, lifecycle/error, focus repair, and valid actions. Cross-surface lookup is acceptable only when it follows stable projected paths, such as resolving the Choice return origin to the canonical Patch Main control.

If that audit exposes a missing fact, add the smallest generic field to the existing semantic summary/control/status vocabulary and freeze its serialized shape with exact tests. Do not add an option-only copy of Patch, registry, lifecycle, focus, or return state. Saved-session serialization is untouched because Choice and request progress remain transient interaction/runtime data.

### 8. Use one responsive DOM with bounded option scrolling

The existing responsive shell and Rust-authored tokens remain the layout system. Wide and Standard seat a flexible main workspace beside bounded Utility. Intermediate widths compress tracks and token gaps continuously. Compact stacks main then Utility in semantic document order. The option header and rows use intrinsic sizing and wrapping; row height is a minimum of 48px rather than a fixed ceiling.

The option list becomes an independently scrollable `minmax(0, 1fr)` region with start/end reachability, while the document itself has no horizontal overflow. Focus reveal may scroll the list after paint, but scroll position never becomes selection authority and never emits an action. One markup tree serves every mode and text scale; CSS presentation determines composition.

Alternatives considered:

- Scaling the 920×680 reference frame was rejected because it breaks target and text floors.
- Mode-specific duplicate markup was rejected because it risks duplicate focus, divergent action lists, and nondeterministic observations.
- Document-level scrolling as the only overflow policy was rejected because persistent shell bands and Utility must remain predictably reachable.

### 9. Extend evidence from reducer outward, focused before broad

Evidence is added in layers using the same production seams:

1. reducer/resolver tests for every origin, duplicate effects, occupied/empty slots, registry order/count, focus/current initialization, non-wrapping navigation, choose/current/no-op, choose/request, close/unchanged, valid actions, exact return, and repair;
2. lifecycle tests proving active/requested and every phase/terminal outcome on Engine and slot origins through acknowledgement;
3. exact semantic-model/serialization fixtures for subject/origin/slot, rows, labels/order, focus/current/enabled, actions, status/error/requested, and repair;
4. headless committed-renderer observations for hierarchy and state treatments;
5. native webview observations at Wide, Standard, Intermediate, Compact, 1280×800, and 1280×800 enlarged text, including long-label and maximum-registry cases, repeat render, resize-only neutrality, target floors, overlap/overflow, bounded scroll endpoints, and paint acknowledgement;
6. physical keyboard/controller handoff for entry, navigation, choose, close, occupied/empty origins, exact return, resize invariance, and repeated Shift;
7. direct comparison with the five live Figma nodes, recording option-slice findings without claiming broader parity.

Patch Detail and Mixer stay in the native run as non-regression witnesses. Sample Browser receives focused regression coverage where shared modal helpers change. Browser-only scaffolds may accelerate iteration but cannot satisfy native paint/input evidence.

### 10. Keep build-cache and architecture guardrails explicit

No implementation task enters the audio callback or changes real-time transport, prepared-graph construction, activation/retirement, device/asset ports, persistence, or capability substitution. `DESIGN.md` changes only after as-built evidence exists.

Every future Cargo invocation must use a repository Make target that depends on `cache-guard`, or must run `scripts/check_build_cache_size.sh --guard` immediately before the Cargo command. Focused exact tests precede broad `make test`; no task runs unguarded Cargo commands.

## Risks / Trade-offs

- [Risk] The existing Choice projection filters to enabled focus paths and may not yet represent a visible disabled registry entry. → Separate canonical visible membership from focus membership only if a real registry fixture requires it, using generic fields and reducer-projected reasons.
- [Risk] Looking up the origin control from the return path could become fragile or duplicate projection logic. → Resolve by exact stable path in the existing serialized graph; if that is not sufficient, add one generic summary field and freeze it with serialization tests.
- [Risk] Shared `modalShellHtml` and CSS changes could unintentionally redesign or break Sample Browser. → Isolate option composition where practical and retain browser hierarchy, preview, waveform, focus, and native reachability regression tests.
- [Risk] Long registry labels and simultaneous current/focus/status/action text can exceed the 48px reference rhythm. → Treat 48px as a floor, allow row growth/wrapping, use a bounded scrolling list, and measure descendant-painted overlap rather than boxes alone.
- [Risk] A terminal failure correlation can persist while a new modal opens, making current versus previously requested state visually ambiguous. → Anchor all status to the exact origin/request correlation and label acknowledged current, requested, and terminal cause independently.
- [Risk] Native screenshots can look plausible while rows are inaccessible or resize mutates semantics. → Require DOM geometry/scroll/event observations and deterministic reducer-to-paint correlation in addition to direct visual review.
- [Trade-off] A single generic option grammar limits capability-specific ornament. → This is intentional: installed registries control content and the slice claims Figma hierarchy/state parity, not bespoke per-capability art direction.

## Migration Plan

1. Add failing focused reducer, resolver, projection, serialization, and renderer observations for the specified option contract without changing production behavior.
2. Refine the existing Choice/return/projection path only where those tests expose missing generic facts.
3. Implement the shared option hierarchy and responsive CSS within the current shell, preserving Sample Browser, Detail, Utility, and Mixer paths.
4. Add headless and native responsive measurements, then fix measured focus, target, overlap, overflow, and bounded-scroll failures.
5. Run guarded focused validation, guarded broad validation, physical native handoff, and direct live-Figma comparison.
6. Update `DESIGN.md` with actual as-built results and remaining gaps only after evidence passes.

There is no persisted-data migration. Rollback is a revert of the reducer/projection/renderer/test slice; acknowledged Patch/session data and prepared audio graphs remain compatible.
