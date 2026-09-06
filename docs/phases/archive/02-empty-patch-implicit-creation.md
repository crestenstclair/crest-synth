# Phase 02 — Empty Patch and Implicit Creation

Status: Planned after Phase 01

## Outcome

PATCH navigation follows the Figma workflow: moving beyond the final existing
Patch reaches an empty Patch position, and the first meaningful edit creates
that Patch.

## Scope

- Represent the empty Patch position without pretending that it is already a
  persisted or audio-active Patch.
- Reach the empty position through the normal semantic Patch-navigation path.
- Define which first meaningful actions create a Patch.
- Create the Patch only through `AppState::apply`.
- Assign stable Patch identity and deterministic defaults at creation.
- Prepare and activate the structural graph before exposing a successful
  structural result.
- Preserve stable focus and return identity as the empty position becomes a
  real Patch.
- Persist only created Patches; the trailing empty position is interaction
  state, not session content.
- Handle preparation failure visibly while keeping the prior session and graph
  valid.

## Out of scope

- Patch deletion, duplication, reordering, or bulk operations unless separately
  specified.
- Multi-select.
- Physical gamepad integration.
- Multi-platform release work.

## Open design boundary

Figma describes Patches as effectively unlimited, while the current prepared
audio graph has a maximum of 16 active Patches. The implementation proposal
must reconcile that contract explicitly without weakening the real-time graph
invariants or silently redefining the Figma workflow.

## Completion signals

- Shift + Right from the final Patch reaches a visibly empty Patch position.
- Navigation alone does not mutate saved session state or rebuild the graph.
- The specified first edit creates exactly one Patch with a stable identity.
- Creation uses the semantic action → `AppState::apply` → projection/audio
  path.
- Failed creation leaves the empty position recoverable and the active graph
  unchanged.
- Saving and reopening preserves created Patches without serializing the empty
  sentinel.

## Dependency

Depends on Phase 01 so creation behavior can be proven across New, Save, and
Open rather than only against the demo fixture.

