## Why

PATCH navigation currently stops at the final installed Patch, so it cannot express the Figma workflow in which the next position is visibly empty and becomes real only after an edit. Phase 01 now provides the New, Save, and Open boundary needed to add that interaction without serializing a sentinel or exposing state before its audio graph is ready.

## What Changes

- Add one reducer-owned trailing empty Patch position after the final created Patch, reached by the existing semantic Patch-navigation action.
- Project the empty position explicitly as interaction state with one stable focus and truthful empty/capacity/lifecycle treatment; navigation to it does not change saved content or prepare a graph.
- Define the accepted Patch-owned edits that implicitly create a Patch, while focus movement, mode/surface entry, return, MIDI performance, preview, and global-only edits do not.
- Build the candidate from a product-authored creation blueprint with a checked stable identity, deterministic label/channel/track/trim/envelope/voice-limit/effect defaults, and exact registry validation rather than registry-order fallback.
- Route creation request, lifecycle, and final Patch insertion through `AppState::apply`; append a completely prepared graph at a block boundary before the reducer exposes the created Patch as successful.
- Rekey the empty position's focus and any return origin to the created Patch in the same reducer commit, retaining the semantic control and surface instead of jumping to an unrelated row.
- Keep failed preparation, staging, activation, and identity/default resolution recoverable on the empty position, with the prior session and graph unchanged and the precise cause visible.
- Persist only created Patches. Saving while empty or while creation is pending captures the prior created set; a later successful creation becomes a saved-content change, and reopen reconstructs only created content.
- Preserve the prepared graph's hard maximum of 16 active Patches. The trailing empty position remains reachable at capacity, but Patch-creating actions are explicitly unavailable with a visible capacity cause. This phase does not introduce dormant Patches, paging, or an audio-inactive Patch model; it records the bounded mismatch with Figma's effectively unlimited workflow instead of silently weakening real-time limits or pretending parity.

## Capabilities

### New Capabilities

- `empty-patch-implicit-creation`: Trailing-empty interaction state, creation-trigger classification, deterministic candidate construction, correlated graph activation, atomic reducer commit, capacity behavior, and failure recovery.

### Modified Capabilities

- `patch-overview`: Extend non-wrapping sibling-Patch navigation and projection to the trailing empty position while preserving singular semantic focus and truthful valid actions.
- `session-lifecycle`: Define capture, dirty-state, Save/Open round-trip, replacement interleavings, and exclusion of the empty sentinel and pending candidate from persisted session content.

## Impact

- Control/application changes affect semantic Patch-position and focus identity, `SemanticAction`/`AppEvent` classification, `AppState::apply`, structural lifecycle correlation, projection, dirty-state observation, and event recording.
- Real-time/control-side changes affect complete candidate-graph preparation, append-only layout admission, parameter snapshot correlation, block-boundary activation, and off-thread retirement while retaining `MAX_PATCHES = 16` and all callback constraints.
- Shell/webview changes add an explicit empty/capacity/loading/failure Patch Overview variant without creating a second domain Patch model or a layout-owned sentinel.
- Persistence keeps the current saved-session schema unless implementation proves a format change is necessary; created Patches use the existing canonical Patch representation and interaction/runtime facts remain excluded.
- Deterministic and native evidence will cross input normalization, reducer, projector, worker, graph coordinator, renderer, serialization, Save/Open, and failure paths. `DESIGN.md` will be updated with durable as-built decisions and measured proof during implementation, not by this proposal.
