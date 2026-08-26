## Why

Engine and Post FX choices already pass through a reducer-owned generic modal, but the shipped webview remains a functional blockout and does not yet demonstrate the live Figma hierarchy, complete state vocabulary, responsive behavior, or production-path evidence for this remaining visual slice. This change makes those option workflows explicit and falsifiable without replacing `PatchChoice` or treating existing tests and surfaces as acceptance.

## What Changes

- Refine the existing `PatchChoice` workflow for Engine Options and Post FX Options entered from the canonical Engine control and each exact canonical effect-slot control, whether occupied or empty.
- Preserve slot-position identity when capabilities repeat, initialize one modal focus deterministically, choose immediately with Edit, close unchanged with Shift+Down, and restore or deterministically repair the exact reducer-owned return origin.
- Source option identities, display labels, order, enabled/available state, and counts only from installed registries plus the canonical `EMPTY` occupancy choice; add no JavaScript capability schema, capability-name branches, fixture counts, or silent substitution.
- Present acknowledged active, requested, loading, validating, preparing, activating, ready, unavailable, and typed-failure readings without treating a prepared candidate as active before graph activation acknowledgement.
- Align focus, current selection, disabled/unavailable, progress, and failure treatments with the live Figma option hierarchy and shared visual vocabulary using text or shape as well as color.
- Extend the one semantic DOM and responsive shell for Wide, Standard, Intermediate, Compact, 1280×800, enlarged-text, long-label, and maximum-registry conditions with 48px target floors, bounded scrolling, no required-content overlap, and no document-level horizontal overflow.
- Add focused reducer/resolver/projection/serialization/DOM/native evidence, deterministic rerender and resize-neutrality checks, physical keyboard/controller handoff, Shift regression coverage, direct live-Figma comparison, and Patch Detail/Mixer non-regression.
- Keep Sample Detail/Browser redesign, Mixer composition and multi-select, broad native polish, saved-session changes, audio callback work, real-time transports, graph-preparation design, assets/devices, and capability substitution out of scope.

## Capabilities

### New Capabilities

- `engine-post-fx-option-states`: Defines the reducer-owned Engine/Post FX option surface, exact selection/cancellation and return behavior, registry-driven content, lifecycle/state presentation, responsive composition, and production-path acceptance evidence.

### Modified Capabilities

- `patch-overview`: Strengthens option entry from the canonical Engine origin and every exact occupied or empty Post FX slot, including duplicate-capability slot identity and the post-choice active/requested lifecycle reading.
- `responsive-shell-composition`: Adds option-surface-specific one-DOM, bounded-list, enlarged-text, and representative native-window acceptance requirements without making layout product state.

## Impact

The implementation phase will primarily affect the production reducer/resolver and semantic graphical projection only where the existing generic contract lacks required facts, plus serialized projection coverage, `webview-page/page.js`, `webview-page/page.css`, webview/native observations, input handoff tests, and durable as-built status in `DESIGN.md` after evidence exists. Capability registries remain the content authority; saved sessions, audio callback and transports, prepared-graph architecture, asset/device ports, and external dependencies do not change.
