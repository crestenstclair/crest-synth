## MODIFIED Requirements

### Requirement: One trailing empty Patch position follows created Patches
PATCH SHALL expose exactly one semantic empty position immediately after the final created Patch. The empty position SHALL be interaction state rather than a Patch, SHALL have no `PatchId`, MIDI subscription, route, voice, effect instance, parameter-snapshot entry, or prepared-graph slot, and SHALL be identified visibly as empty rather than active or persisted.

#### Scenario: Navigate beyond the final Patch
- **WHEN** E or the equivalent semantic next-Patch action is accepted on the final created Patch
- **THEN** PATCH SHALL move to the one trailing empty position with exactly one stable semantic focus and SHALL identify the position as empty

#### Scenario: Return to the final created Patch
- **WHEN** Q or the equivalent semantic previous-Patch action is accepted on the trailing empty position
- **THEN** PATCH SHALL return to the final created Patch while retaining the nearest valid semantic control identity

#### Scenario: Empty position is the non-wrapping endpoint
- **WHEN** next-Patch navigation is requested while the trailing empty position is already active
- **THEN** the request SHALL be rejected as an unchanged boundary and SHALL NOT create a second empty position

#### Scenario: Navigation is session and graph neutral
- **WHEN** the user enters, leaves, or moves focus within the trailing empty position without a creation-triggering edit
- **THEN** the created Patch collection, saved-session capture, dirty status, parameter snapshot, active graph revision, and rendered audio SHALL remain unchanged

#### Scenario: Performance input while empty is focused
- **WHEN** keyboard, controller, or physical MIDI performance input is accepted while the trailing empty position is focused
- **THEN** it SHALL use the existing created-Patch subscription path and SHALL NOT target, create, or activate the empty position


### Requirement: Active Patch capacity remains explicit and bounded
The active and persisted Patch collection SHALL remain bounded to the prepared graph capacity of 16 in this phase. The trailing empty position SHALL remain reachable when 16 Patches exist, but it SHALL show an explicit capacity-reached status, omit Patch-creating actions from its valid actions, and reject a stale or direct creation command without mutation. The system MUST NOT create an audio-inactive Patch, evict another Patch, page the active set, raise the callback bound implicitly, or serialize a seventeenth Patch.

#### Scenario: Navigate to empty at capacity
- **WHEN** E or the equivalent semantic next-Patch action is accepted on the sixteenth created Patch
- **THEN** the trailing empty position SHALL open and visibly state that the 16-Patch active-audio capacity has been reached

#### Scenario: Creation is refused at capacity
- **WHEN** a Patch-creating action is dispatched while 16 Patches exist
- **THEN** it SHALL receive a typed unchanged capacity rejection and SHALL NOT prepare a graph, alter saved content, or change the active graph

#### Scenario: Capacity is not hidden as a Figma success
- **WHEN** acceptance evaluates the effectively unlimited Figma navigation workflow against the bounded production graph
- **THEN** evidence SHALL report navigation parity through the empty position and the explicit 16-Patch creation limit as a scoped product mismatch rather than claiming unlimited creation
