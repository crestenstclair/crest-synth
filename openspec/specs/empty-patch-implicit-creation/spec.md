# Empty Patch Implicit Creation Specification

## Purpose

Defines the trailing empty Patch interaction and the atomic implicit-creation workflow that turns its first accepted Patch-owned edit into one prepared, persisted, audio-active Patch.

## Requirements

### Requirement: One trailing empty Patch position follows created Patches
PATCH SHALL expose exactly one semantic empty position immediately after the final created Patch. The empty position SHALL be interaction state rather than a Patch, SHALL have no `PatchId`, MIDI subscription, route, voice, effect instance, parameter-snapshot entry, or prepared-graph slot, and SHALL be identified visibly as empty rather than active or persisted.

#### Scenario: Navigate beyond the final Patch
- **WHEN** Shift+Right or the equivalent semantic Patch-navigation action is accepted on the final created Patch
- **THEN** PATCH SHALL move to the one trailing empty position with exactly one stable semantic focus and SHALL identify the position as empty

#### Scenario: Return to the final created Patch
- **WHEN** Shift+Left or the equivalent semantic Patch-navigation action is accepted on the trailing empty position
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

### Requirement: The empty position projects prospective values without pretending they are active
The empty position SHALL project the Patch Overview, applicable default-Engine Detail, and persistent Utility interaction needed to begin editing, using the product-authored creation blueprint as prospective data. Prospective Patch-owned values MUST be marked as defaults for a new Patch and MUST NOT be described as current, active, persisted, requested, or audio-rendered before creation commits. Global Utility values SHALL remain the acknowledged active global values.

#### Scenario: Empty Overview is rendered
- **WHEN** the trailing empty position is active with no subordinate surface open
- **THEN** the shell SHALL render an explicit empty Patch Overview with Engine and three ordered Post FX positions, one focus treatment, a new-Patch/default marker, and no fabricated Patch identity

#### Scenario: Prospective Detail is inspected
- **WHEN** the user opens Detail from the empty Engine without changing a value
- **THEN** the default Engine descriptor and envelope controls SHALL be inspectable as prospective creation values without creating a Patch or preparing a graph

#### Scenario: Empty Utility distinguishes ownership
- **WHEN** Utility is reached from the trailing empty position
- **THEN** Master Volume SHALL show the active global value while Patch Volume, MIDI Input, Output Track, and Voice Limit SHALL show explicitly prospective new-Patch defaults

#### Scenario: Empty position reflows
- **WHEN** the same empty-position projection is rendered in Wide, Standard, or Compact composition or at enlarged text scale
- **THEN** its semantic position, focus, prospective/active distinction, status, and valid actions SHALL remain unchanged and every required control SHALL remain reachable

### Requirement: Only accepted Patch-owned edits trigger implicit creation
An accepted edit against a prospective Patch-owned value SHALL request implicit creation. Creation-triggering edits SHALL include explicit confirmation or selection of an enabled Engine, selection of a non-empty Post FX occupant, and an accepted change to a Patch-owned instrument, envelope, effect, Patch Volume, MIDI Input, Output Track, or Voice Limit value. Focus navigation, Patch navigation, mode changes, opening or closing a surface, preview, performance input, global-only edits, rejected boundary edits, and choosing `EMPTY` for an already empty Post FX position MUST NOT request creation.

#### Scenario: Patch-owned Utility edit triggers creation
- **WHEN** an accepted edit changes prospective Patch Volume, MIDI Input, Output Track, or Voice Limit
- **THEN** exactly one correlated creation request SHALL contain that edited value and the remaining deterministic creation defaults

#### Scenario: Engine confirmation triggers creation
- **WHEN** the user explicitly confirms the authored default Engine or selects another enabled Engine from the empty position
- **THEN** exactly one correlated creation request SHALL use the confirmed Engine and its exact validated provider-authored configuration

#### Scenario: Detail edit triggers creation
- **WHEN** an accepted edit changes a prospective default-Engine descriptor or envelope value
- **THEN** exactly one correlated creation request SHALL contain that edit applied to the deterministic candidate

#### Scenario: Non-empty effect selection triggers creation
- **WHEN** an enabled effect is selected for any prospective Post FX position
- **THEN** exactly one correlated creation request SHALL contain that occupant at the exact canonical position

#### Scenario: Empty effect confirmation is not meaningful
- **WHEN** `EMPTY` is confirmed for a prospective Post FX position that is already empty
- **THEN** the option session MAY close but no Patch creation, saved-content change, or graph preparation SHALL occur

#### Scenario: Global edit does not consume the empty position
- **WHEN** Master Volume is edited from Utility while the trailing empty position is active
- **THEN** the accepted global saved value SHALL change normally and the trailing position SHALL remain empty

#### Scenario: Additional edits are refused while creation is pending
- **WHEN** one implicit creation is Loading, Validating, Preparing, or Activating and another creation-triggering edit is requested
- **THEN** the later request SHALL be rejected as structurally busy without creating, replacing, or merging a second candidate

### Requirement: Creation identity and defaults are deterministic and exact
Each creation request SHALL reserve one candidate identity equal to one greater than the greatest created `PatchId`, failing visibly if that checked identity cannot be represented. The appended position SHALL use label `Patch {id}`, MIDI channel equal to its one-based appended ordinal, the correspondingly indexed Mixer track T00 through T0F, 0 dB Patch trim, the product-designated default Engine and its exact provider-authored default configuration unless the triggering edit selected another Engine, the neutral default envelope, the Engine's seeded voice limit, and three empty ordered Post FX positions unless the triggering edit changes one of those values. Capability resolution and validation MUST NOT use registry order, a similar identity, or fallback content.

#### Scenario: Second Patch receives deterministic defaults
- **WHEN** a one-Patch session implicitly creates its next Patch from a Patch-owned scalar edit
- **THEN** the candidate SHALL reserve identity 2, label `Patch 2`, displayed MIDI channel 2, output track T01, 0 dB trim except for the triggering edit when applicable, the exact designated default Engine, its neutral envelope and seeded voice limit, and three empty Post FX positions

#### Scenario: Sparse identities remain monotonic
- **WHEN** the created Patch collection has sparse identities and its greatest identity is N
- **THEN** the candidate SHALL reserve identity N+1 rather than deriving identity from collection index, label, MIDI channel, track, or a reused gap

#### Scenario: Trigger overrides only its target
- **WHEN** the first meaningful edit selects an alternate Engine, adds one effect, or changes one Patch-owned scalar
- **THEN** the candidate SHALL apply exactly that edit over the creation blueprint and SHALL retain every unrelated deterministic default

#### Scenario: Default capability is unavailable
- **WHEN** a triggering edit depends on the designated default Engine and that exact capability or its authored default cannot resolve or validate
- **THEN** creation SHALL fail with the exact typed cause and SHALL NOT select another installed Engine

#### Scenario: Identity range is exhausted
- **WHEN** the greatest created Patch identity has no representable successor
- **THEN** creation SHALL fail visibly before graph preparation and the created Patch collection SHALL remain unchanged

### Requirement: A Patch commits only after its complete graph is active
Implicit creation SHALL use the shared single structural lifecycle. The complete candidate graph, including all prior Patches, the candidate Patch, ordered effects, returns, routing, voices, scratch, and initial scalar snapshot, SHALL be validated, allocated, and warmed off the audio callback. The callback SHALL activate it only at a block boundary. Only matching activation and retirement acknowledgement SHALL permit one canonical reducer commit that appends the candidate Patch; prior lifecycle states MUST NOT expose it as created or saved.

#### Scenario: Successful implicit creation
- **WHEN** a creation candidate validates, prepares, stages, activates at a block boundary, and receives matching acknowledgement
- **THEN** one reducer commit SHALL append exactly the reserved Patch, publish the matching canonical projection and parameter snapshot, and mark saved session content changed

#### Scenario: Creation remains pending before acknowledgement
- **WHEN** creation is Loading, Validating, Preparing, or Activating
- **THEN** the empty position SHALL remain visibly empty with requested values and lifecycle shown separately, while the prior Patch collection and acknowledged graph remain canonical

#### Scenario: Candidate preparation fails
- **WHEN** capability, asset, effect, routing, allocation, graph-capacity, or other complete preparation fails
- **THEN** no Patch SHALL be appended, the candidate SHALL retire off the callback, the prior state/snapshot/graph/audio SHALL remain valid, and the exact failure SHALL be visible on the recoverable empty position

#### Scenario: Publication or activation fails
- **WHEN** a prepared candidate is busy, stale, incompatible, cannot be published, or does not receive matching activation acknowledgement
- **THEN** no creation commit SHALL occur, candidate ownership SHALL return for off-thread retirement, and the prior state and active graph SHALL remain correlated and usable

#### Scenario: Retry after failure
- **WHEN** the cause of a failed creation is corrected and a creation-triggering edit is accepted again from the empty position
- **THEN** one fresh correlated request SHALL retry without a phantom Patch or duplicate identity left by the failure

### Requirement: Focus and return identity survive the empty-to-created transition
The empty position SHALL use stable semantic control and surface identities independent of layout. On successful creation, the reducer SHALL rekey the active empty focus, remembered PATCH root, suspended system focus, and any subordinate return origin that still refer to that empty position to the reserved `PatchId` in the same commit. The semantic control, subject position, and surface SHALL be retained whenever they resolve for the created candidate; repair SHALL be explicit if the triggering Engine schema removes an origin.

#### Scenario: Overview edit commits in place
- **WHEN** creation succeeds while an empty Overview control is the active or remembered origin
- **THEN** focus SHALL identify the same Overview control on the created Patch without landing on a first-row or coordinate-based fallback

#### Scenario: Detail or option origin commits in place
- **WHEN** creation was triggered from prospective Detail or Choice state
- **THEN** the established close/commit behavior SHALL retain a return origin for the same Engine, parameter, or exact effect-slot position on the created Patch

#### Scenario: User leaves while preparation continues
- **WHEN** the user navigates back to an existing Patch after requesting creation and the candidate later commits
- **THEN** the new Patch SHALL append without stealing the user's current focus, while the remembered empty identity SHALL be rekeyed only where it still exists

#### Scenario: Failed creation retains empty identity
- **WHEN** creation fails before commit
- **THEN** the empty position's active focus and return identity SHALL remain valid for navigation, inspection, correction, and retry

### Requirement: Active Patch capacity remains explicit and bounded
The active and persisted Patch collection SHALL remain bounded to the prepared graph capacity of 16 in this phase. The trailing empty position SHALL remain reachable when 16 Patches exist, but it SHALL show an explicit capacity-reached status, omit Patch-creating actions from its valid actions, and reject a stale or direct creation command without mutation. The system MUST NOT create an audio-inactive Patch, evict another Patch, page the active set, raise the callback bound implicitly, or serialize a seventeenth Patch.

#### Scenario: Navigate to empty at capacity
- **WHEN** Shift+Right is accepted on the sixteenth created Patch
- **THEN** the trailing empty position SHALL open and visibly state that the 16-Patch active-audio capacity has been reached

#### Scenario: Creation is refused at capacity
- **WHEN** a Patch-creating action is dispatched while 16 Patches exist
- **THEN** it SHALL receive a typed unchanged capacity rejection and SHALL NOT prepare a graph, alter saved content, or change the active graph

#### Scenario: Capacity is not hidden as a Figma success
- **WHEN** acceptance evaluates the effectively unlimited Figma navigation workflow against the bounded production graph
- **THEN** evidence SHALL report navigation parity through the empty position and the explicit 16-Patch creation limit as a scoped product mismatch rather than claiming unlimited creation

### Requirement: Production evidence crosses reducer, projection, graph, and render paths
Acceptance SHALL drive empty navigation and implicit creation through normalized input, semantic action, the canonical reducer, immutable projection and serialization, off-thread complete graph preparation, block-boundary activation, the production audio renderer, and saved-session capture. It SHALL distinguish navigation no-op, pending request, successful commit, capacity refusal, and each controlled failure without using a parallel Patch or graph path.

#### Scenario: First-edit production proof
- **WHEN** representative Engine, effect, Detail, and Patch Utility first edits are driven from the empty position
- **THEN** each SHALL prove exactly one created identity, matching state/parameter/graph revisions, audible renderability where applicable, singular focus, and the expected saved-content transition

#### Scenario: Real-time safety proof
- **WHEN** creation preparation, publication, activation, failure, and retirement are exercised while audio renders
- **THEN** the callback SHALL perform bounded preallocated work with no allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwinding, or graph destruction

