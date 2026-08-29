## ADDED Requirements

### Requirement: Session persistence excludes empty and pending Patch positions
Saved-session capture SHALL serialize only created Patches. The trailing empty position, its focus and return identity, prospective creation defaults, reserved candidate identity, pending creation values and lifecycle, prepared candidate graph, capacity status, and creation failure MUST remain interaction/runtime data outside session files and the clean baseline.

#### Scenario: Save while resting on empty
- **WHEN** Save or Save As captures a session while the trailing empty position is focused and no creation is pending
- **THEN** the written file SHALL contain exactly the created Patches and SHALL contain no empty sentinel, prospective default, focus, or capacity field

#### Scenario: Save while creation is pending
- **WHEN** Save captures a session while implicit creation is Loading, Validating, Preparing, or Activating
- **THEN** the written capture SHALL contain the acknowledged created Patch set only and a later successful creation commit SHALL leave the active document dirty against that written baseline

#### Scenario: Failed creation does not dirty the document
- **WHEN** implicit creation fails or is refused before a Patch commit and no other saved value changed
- **THEN** the document's clean or dirty status and saved-session capture SHALL remain exactly as before the request

#### Scenario: Successful creation dirties the document
- **WHEN** matching graph activation acknowledgement commits one created Patch
- **THEN** the saved-session capture SHALL gain exactly that Patch and the document SHALL become dirty unless the new capture already equals its clean baseline

### Requirement: Save and Open round-trip only created Patches
A Save followed by Open SHALL preserve every successfully created Patch identity, order, label, Engine/effect configuration, routing, Patch controls, Mixer data, and stable asset reference through the existing versioned saved-session format. Open SHALL reconstruct the created Patch set and complete graph only; it SHALL derive a fresh trailing empty interaction position rather than decoding one.

#### Scenario: Created Patch round-trip
- **WHEN** a session with one or more implicitly created Patches is saved successfully and reopened
- **THEN** every created Patch SHALL retain its exact canonical saved values and stable identity, and the active graph SHALL contain those Patches in saved order

#### Scenario: Empty position is reconstructed, not restored
- **WHEN** a saved session is reopened after Save occurred while the user had focused the trailing empty position
- **THEN** Open SHALL restore the normal deterministic PATCH focus required by session replacement and SHALL make a fresh trailing empty position reachable after the final created Patch without restoring empty focus or pending lifecycle

#### Scenario: Capacity-valid file reopens
- **WHEN** a saved session contains exactly 16 valid created Patches
- **THEN** Open SHALL prepare and activate all 16, expose a fresh capacity-marked trailing empty position, and SHALL NOT decode or synthesize a seventeenth Patch

### Requirement: Session replacement and implicit creation share structural exclusion
New, Open, Engine/effect topology edits, and implicit Patch creation SHALL use one application-wide structural exclusion policy. Once New, Open, or guarded Close has been authorized, a Patch-creating edit SHALL be treated as a saved-field action and refused until that lifecycle decision completes. A session replacement SHALL not commit over a pending creation graph, and a creation commit SHALL not install after a different session has replaced its source revision.

#### Scenario: Replacement authorization blocks new creation
- **WHEN** New, Open, or guarded Close has been authorized and a first Patch-owned edit is attempted on the empty position
- **THEN** the edit SHALL receive the same saved-field unavailability treatment as other edits and SHALL NOT reserve an identity or submit a graph candidate

#### Scenario: New or Open meets pending creation
- **WHEN** New or Open is requested while an implicit creation candidate owns the structural lifecycle
- **THEN** the lifecycle coordinator SHALL defer or reject replacement visibly rather than run two candidates or silently cancel acknowledged work

#### Scenario: Stale creation result follows replacement
- **WHEN** a creation worker result names a source state or graph revision that is no longer active after session replacement
- **THEN** the result SHALL be rejected as stale, retired off the callback, and SHALL NOT append its Patch to the replacement session

### Requirement: Lifecycle evidence covers implicit creation persistence
Session-lifecycle acceptance SHALL extend the production New, Save, Save As, and Open matrix with empty navigation, pending creation, successful creation, controlled failure, capacity, and structural-interleaving cases.

#### Scenario: Save and reopen production proof
- **WHEN** production-boundary acceptance creates a Patch, saves it, moves to the empty position, closes, and reopens the file
- **THEN** evidence SHALL prove the file contains the created Patch but no sentinel, the reopened graph renders the exact created set, and a fresh empty position is reachable

#### Scenario: Pending and failed capture proof
- **WHEN** controlled Save operations capture during pending creation and after creation failure
- **THEN** evidence SHALL prove exact baseline/dirty behavior, unchanged prior bytes where applicable, no phantom Patch, and no prepared/runtime fields in the encoded session

