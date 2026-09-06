## Context

See `proposal.md` for motivation and the three delta specs for acceptance. Figma Page Layout frame `153:184` originally left Settings return unmapped. On 2026-09-06 the user selected Shift+Right, which is now authored at `155:208` and `157:200`; `155:205` specifies suspended PATCH/MIXER page and exact valid focus restoration or nearest-enabled repair. Direct inspection and screenshots confirmed the updated contract and no remaining unmapped annotation. Phase 03 and SoundFont loading are archived and synced. The accepted Phase 03 functional surfaces provide the starting point; its deferred visual work is not a prerequisite to this change.

Before this change, the implementation already owned most destination behavior:

- `AppState::select_patch` follows canonical Patch order and the trailing empty endpoint, lands in Overview, and repairs the highlighted control against the destination schema.
- `InteractionState::select_context` restores remembered PATCH/MIXER roots. Detail and temporary Settings already retain stable origins and use canonical repair.
- `KeyboardInputTranslator` and `ControllerInputTranslator` emit `OpenRelated`, `Return`, or `SelectPatch` for Shift directions. `OpenRelated` also opens Sample/SoundFont file pages; `Return` also closes Choice/file pages and stops browser preview.
- `AppState::open_related_surface` currently rejects Mixer, and root Overview has no subordinate return. These paths do not implement the two vertical context connections.
- Native macOS capture normalizes WASD and Q/E, but actual arrow keycodes currently become `WindowKey::Other`. Shift+Left still selects the previous Patch. Shift+Start opens MIDI Settings; Shift+Down currently returns from Settings.

`DESIGN.md` records current as-built behavior and evidence; the baseline above explains the change rather than claiming implementation proof.

## Goals / Non-Goals

**Goals:**

- Keep page intent in the existing semantic action/event vocabulary and resolve its destination from canonical reducer state.
- Reuse existing context roots, subordinate ownership, Settings suspension, and repair without duplicating their state or effects.
- Preserve input admission and existing asset/choice/preview workflows while making the authored page chords work through native capture.
- Prove page identity and session/audio neutrality with focused deterministic and native coverage.

**Non-Goals:**

- No page router, navigation stack, page registry, new context, or UI-owned destination model.
- No new audio transport, persistence format, device lifecycle, dependency, or physical gamepad integration.
- No visual reconstruction, resize campaign, or change to in-page editing behavior.

## Decisions

### 1. Express Shift page intent once; resolve it in the reducer

Add `NavigatePage(Direction)` to the existing `SemanticAction`/`AppEvent` vocabulary, using the canonical `Direction` type. Keyboard Shift directions and host-neutral controller Shift-direction gestures produce this action without inspecting the current page. Q/E continue to produce the existing `SelectPatch(Left/Right)` action.

Handle page intent in a small source-aware branch owned by `AppState`, delegating to the existing transition helpers. Include it in action kinds, event conversion, admission, event/schema descriptors, and projected valid-action handling. Do not create a second navigation model or recursively call public `apply`: one accepted event produces one state generation and one transition. Where legacy branches contain reusable return cleanup or Settings scan effects, extract a private helper within their current owner and share it.

This additional action distinguishes authored page chords from existing generic `OpenRelated`, `Return`, and compatibility Settings commands. Globally widening those commands would also change callers used by subordinate workflows; making adapters choose concrete destinations would move product logic outside the reducer. A bounded semantic action with existing helpers preserves both ownership and compatibility.

### 2. Traverse only the edge applicable to the source

Evaluate the active surface before its underlying context so a temporary or subordinate surface cannot accidentally dispatch a root-page transition. Apply existing mode, modal, busy, and held-preview admission before mutation; unavailable requests retain typed unchanged rejection.

| Source | Semantic request | Existing transition to reuse |
| --- | --- | --- |
| Patch Overview | Page Up | Enter Detail for the exact highlighted Instrument or occupied effect slot; preserve Overview origin |
| Patch Detail | Page Down | Close Detail to its valid Overview origin or canonical repair |
| Patch Overview | Page Down | Select MIXER at its remembered root |
| Mixer main/Inspector, when admitted | Page Up | Select PATCH at its remembered Overview root |
| Patch Overview | Page Left | Open temporary MIDI Settings and perform existing discovery effects |
| Admitted PATCH navigation | Previous/next Patch | Existing `select_patch`, including Overview landing and trailing empty semantics |
| MIDI Settings | Page Right | Restore the exact valid suspended PATCH/MIXER page and focus, or existing nearest-enabled repair |

Resolve Detail subjects by Patch and slot identity, not capability name: two slots with the same effect remain distinct. Empty effect slots do not fabricate Detail. Reuse prospective empty-Patch behavior without creating a Patch merely by navigating.

For inputs outside the authored root graph, preserve existing admitted subordinate behavior through the same helpers: Page Up from an eligible asset control in Detail opens its file page; Page Down from Choice/file pages returns through existing cancellation and preview cleanup. Retain Shift+Down as a Settings compatibility escape. Retain non-conflicting direct context, Shift+Start Settings, and Shift+Right next-Patch shortcuts outside Settings where admitted. Page Left outside Overview may retain previous-Patch behavior only where already admitted; it cannot bypass a modal or Settings surface. These compatibility paths do not establish new Figma connections.

Do not cascade transitions: Detail return stops at Overview, and Mixer return stops at Overview without opening Detail. A page request does not also execute `Navigate` or `Adjust`.

### 3. Preserve identity in its existing owner

Use `InteractionState` remembered roots, `PatchSubordinateSession`, Settings suspension, and existing nearest-enabled repair. Preserve the exact Patch/slot/control identity when valid; use the established next-before-previous sibling repair when independent schema/session changes invalidate it and expose the repair through current status projection.

Keep Patch cycling in `AppState::select_patch`. Q/E remain non-wrapping and traverse created identities in their canonical order, followed by one trailing empty position. Do not derive IDs from indices, create on navigation, or alter the 16-Patch bound. A new destination cache or unconditional first-row selection would compete with the existing focus owner and is unnecessary.

Page transitions change interaction state only. Settings entry may request existing device discovery, but navigation itself must not connect a device, change content or dirty state, change MIDI subscriptions/routes, prepare a graph, or interrupt performance audio. Browser preview cleanup remains required when closing its own workflow; it is separate from the performance-page neutrality assertion.

### 4. Normalize physical directions at the existing input boundary

Map actual macOS arrow keycodes to the existing normalized directional key equivalents alongside WASD in `webview/input_capture.rs`; the keyboard translator gives both the same semantics. Keep AppKit modifier handling, event redispatch suppression, repeat admission, key release, and focus-loss cleanup in their current input owners. Verify a single physical activation cannot cross multiple page edges.

Align the host-neutral controller translator with the same semantic page action. This prepares consistent controller meaning without adding device discovery or physical gamepad support. Leave bare-direction focus and Edit-direction behavior unchanged. Adding browser-side keyboard navigation would create a second input path and is unnecessary.

### 5. Project truthful source-specific guidance

Extend existing semantic valid-action projection rather than hardcoding a parallel key table in the webview. The Overview footer must advertise Q/E and its admitted page chords, Detail must describe its valid return/open-related actions, and Mixer must describe its Overview connection. Keep labels aligned with the Page Layout and make keyboard arrow/WASD equivalents understandable. Settings must advertise Shift+Right / Shift+D as return to performance; the existing Shift+Down escape may remain as compatibility guidance.

Update affected serialized action/event surface fixtures and schema coverage together with the new action. The webview continues to paint immutable projection and emit semantic intent, with no local context or return-state mutation.

### 6. Implement the authored Settings return through existing suspension

The user-selected Shift+Right return is authored in Page Layout `153:184`: `155:208` names the chord, `155:205` specifies the suspended page/focus destination and repair, and `157:199`–`157:200` pair Left entry with Right return. This resolves the original design prerequisite; the proposal alone did not select the key or authorize that edit.

Resolve Page Right from Settings before any horizontal Patch-cycling branch, and reuse `return_from_midi_settings` through the existing return helper. Restore valid suspended PATCH/MIXER identity or canonical repair without connecting devices, changing selection, or cascading into another page edge. Keep Shift+Down compatibility, but deterministic and native acceptance must exercise the authored Shift+Right return.

### 7. Prove behavior with the smallest production-path checks

Extend focused translator/reducer/projection tests to cover each edge, exact Instrument/effect origins, duplicate effect capabilities, unavailable slots, sparse Patch IDs, endpoints, root round trips, Settings suspension/repair, and admission. Add regressions around Choice and Sample/SoundFont file workflows where the shared Shift actions change. Compare saved-session/dirty state, subscriptions/routes, graph identity, and ongoing render behavior for navigation-only journeys.

Extend the existing `webview_projection_shell` native witness with a bounded page-navigation journey and a focused invocation. Drive actual window Q/E, arrow/Shift capture, and a WASD equivalence check; observe semantic acceptance and visible paint identity after each edge, including Shift+Right from Settings. The witness owns and closes its window and reports a specific failing edge rather than launching unrelated soak or resize scenes.

Native evidence proves input, page identity, singular focus, subject, return, and truthful guidance. It does not require pixel matching or repeat Phase 03's accepted manual resize handoff. Planning validation checks artifact coherence only and is not implementation evidence.

## Risks / Trade-offs

- **Shift+Right also has next-Patch compatibility elsewhere.** Mitigation: dispatch from Settings first and prove return restores the suspended Patch without stepping to its neighbor.
- **Shift+Left changes a familiar Overview shortcut.** Mitigation: present Q as previous Patch, update affected main-spec deltas and input guidance together, and cover return from the trailing empty position explicitly.
- **Shared Shift gestures could break file/Choice return or preview cleanup.** Mitigation: keep surface-first reducer dispatch, reuse existing effect-owning helpers, and test the affected workflows without redesigning them.
- **Remembered roots can be invalidated asynchronously.** Mitigation: retain canonical validation/repair and assert exact identity or an exposed deterministic repair, never screen coordinates.
- **A native witness can pass without exercising physical capture.** Mitigation: use actual key/modifier delivery and correlate accepted state with painted identity; direct reducer fixtures alone are insufficient for native acceptance.
- **A new semantic variant expands exhaustive matches.** Mitigation: update descriptors, conversion, admission, projected actions, and schema tests as one coherent change; introduce no parallel public navigation types.

## Migration Plan

Implement only after a subsequent apply request. Land the semantic action, reducer dispatch, input mappings, projection guidance, and affected tests coherently so no advertised chord points to the old behavior. Preserve all non-conflicting compatibility shortcuts and existing subordinate cleanup.

No saved-session migration or dependency change is needed. The Settings return contract is now authored; implementation and final acceptance must match it. After focused deterministic/native proof, update the durable input and navigation description in `DESIGN.md` with actual evidence and remaining limits. Sync the delta specs and archive only after completion is accepted.

If the input change must be rolled back, revert the action/mapping/guidance change together; existing saved sessions and audio graphs require no conversion. Do not roll back the previously accepted Sample or SoundFont work.
