# patch-overview Specification

## Purpose
TBD - created by archiving change responsive-shell-patch-overview. Update Purpose after archive.
## Requirements
### Requirement: Patch Overview is the PATCH root
When PATCH has no subordinate Detail, Choice, or Sample Browser surface open, the main workspace SHALL render Patch Overview rather than the legacy flat/grouped Patch Strip.

#### Scenario: Resting PATCH context
- **WHEN** an initialized Patch is active and no subordinate PATCH session is open
- **THEN** the main surface SHALL identify itself as Patch Overview and render the overview composition

#### Scenario: Return from subordinate surface
- **WHEN** the user closes Detail, Choice, or Sample Browser
- **THEN** the workspace SHALL return to Patch Overview with the reducer-restored semantic origin focused

### Requirement: Overview section anatomy
Patch Overview SHALL contain one Engine section followed by one Post FX section containing exactly the three canonical ordered Patch effect slots.

#### Scenario: Fully occupied chain
- **WHEN** all three Post FX slots are occupied
- **THEN** the overview SHALL render Engine, then Post FX slots 1, 2, and 3 in render order with each projected occupant identified

#### Scenario: Empty slots
- **WHEN** one or more Post FX slots are empty
- **THEN** every canonical slot position SHALL remain visible in order and each empty position SHALL say `EMPTY` or the projected equivalent

### Requirement: Canonical overview controls and sections
The overview SHALL reuse canonical Engine and Effect Slot control identities and SHALL receive its section/order metadata from the semantic projector rather than reconstructing product composition from labels or capability names in JavaScript.

#### Scenario: Serialized overview structure
- **WHEN** `StateProjector` projects an active Patch Main surface
- **THEN** the surface SHALL expose ordered Engine and Post FX section metadata whose control paths resolve to controls in the same projection

#### Scenario: Renderer consumes sections
- **WHEN** the webview renders the Patch Main surface
- **THEN** it SHALL iterate projected sections and control paths and SHALL not switch on SoundFont, Sample, Braids, Chorus, Reverb, Delay, or any other concrete capability name

### Requirement: Detail parameters are not duplicate root targets
Envelope, descriptor-declared instrument parameters, and effect parameters SHALL be focused and edited on the existing Detail surface rather than retained as hidden or duplicate Patch Overview focus targets.

#### Scenario: Instrument parameters
- **WHEN** the Engine control opens Instrument Detail
- **THEN** the descriptor and shared envelope parameter identities SHALL be reachable on Detail and SHALL not remain focusable as invisible Patch Overview rows

#### Scenario: Effect parameters
- **WHEN** an occupied Post FX slot opens FX Detail
- **THEN** that effect descriptor's parameters SHALL be reachable on Detail and SHALL not remain focusable as invisible Patch Overview rows

### Requirement: Overview focus and spatial navigation
Patch Overview SHALL expose exactly one focused semantic target and SHALL use the reducer's semantic adjacency to move among Engine, ordered Post FX slots, and persistent Utility controls without coordinate-based navigation.

#### Scenario: Move through Post FX slots
- **WHEN** unmodified navigation moves from the Engine region through Post FX
- **THEN** focus SHALL visit the canonical occupied or empty slot controls in slot order

#### Scenario: Cross into Utility
- **WHEN** navigation crosses from the overview's rightmost semantic region into Utility
- **THEN** focus SHALL move to the projected Utility target while retaining the active Patch identity

#### Scenario: Responsive reflow
- **WHEN** the same focused overview control is rendered in Wide, Standard, and Compact composition
- **THEN** the focus path SHALL remain identical even if its visual row or column changes

### Requirement: Existing subordinate workflows remain reducer-owned
Engine and Post FX controls SHALL advertise and invoke only valid actions resolved from the current production reducer for opening Choice or Detail surfaces, choosing or closing options, changing supported values, returning, and navigating sibling Patches. Choice subject, focus, and return identity SHALL remain stable semantic identities owned by the reducer.

#### Scenario: Open Engine options
- **WHEN** the focused canonical Engine control invokes its projected Choice action
- **THEN** Engine Options SHALL open through the reducer with the Engine control as its exact return origin and the current enabled registry option as the sole modal focus

#### Scenario: Open Engine Detail
- **WHEN** the focused Engine control invokes its projected Detail action
- **THEN** Instrument Detail SHALL open for the active capability through the existing subordinate session

#### Scenario: Open Post FX options from each canonical slot
- **WHEN** the projected Choice action is invoked from any canonical Post FX slot
- **THEN** Post FX Options SHALL open through the reducer for that exact slot whether occupied or empty

#### Scenario: Open Post FX workflows
- **WHEN** a Post FX slot invokes a projected occupancy Choice or occupied-slot Detail action
- **THEN** the existing Choice or Detail surface SHALL open through the reducer for that exact canonical slot without direct DOM state mutation

#### Scenario: Open occupied Post FX Detail
- **WHEN** an occupied Post FX slot invokes its projected Detail action
- **THEN** FX Detail SHALL open for that canonical slot without direct DOM state mutation

#### Scenario: Duplicate effect capabilities retain position
- **WHEN** multiple Post FX slots contain the same capability and an option or Detail surface is opened from each
- **THEN** every subject and return origin SHALL retain its exact canonical slot identity without deriving identity from the shared capability or label

#### Scenario: Choose and return
- **WHEN** Edit chooses a current or different enabled option
- **THEN** the Choice surface SHALL close through the reducer to the exact live Engine or slot origin, starting at most the one structural request admitted for a changed option

#### Scenario: Close unchanged
- **WHEN** Shift+Down closes Engine or Post FX Options before a choice is activated
- **THEN** Patch Overview SHALL return with the exact origin focused and the acknowledged configuration unchanged

### Requirement: Persistent Utility remains present
The PATCH Utility surface SHALL remain the persistent side region while Patch Overview, Detail, Choice, or Sample Browser is active, subject only to responsive placement.

#### Scenario: Overview Utility
- **WHEN** Patch Overview is active
- **THEN** Master Volume, Patch Volume, MIDI Input, Output Track, and Voice Limit SHALL remain projected and reachable from the active Patch Utility surface

#### Scenario: Utility after compact stacking
- **WHEN** Compact composition places Utility after the main workspace
- **THEN** its controls, values, focus identities, and valid actions SHALL be unchanged from the same projection in Wide composition

### Requirement: Registry-driven summaries and counts
Engine and Post FX summaries SHALL derive labels, occupancy, status, and parameter counts from the installed descriptor registries and the semantic projection. Figma fixture values MUST NOT be treated as an exhaustive list or fixed count.

#### Scenario: Instrument parameter summary
- **WHEN** the active instrument descriptor exposes a given number of detail parameters
- **THEN** the Engine overview SHALL report a summary derived from that descriptor rather than a hardcoded fixture count

#### Scenario: Effect parameter summary
- **WHEN** an occupied Post FX slot's descriptor exposes a given number of parameters
- **THEN** that slot SHALL report the descriptor-derived summary and an empty slot SHALL not fabricate a parameter count

### Requirement: Requested, active, and failed structural state remain distinct
Patch Overview SHALL preserve the projected distinction among acknowledged active values, requested values, Loading, Validating, Preparing, Activating, Ready, Unavailable, and typed Failed outcomes after an Engine or Post FX option is chosen. It SHALL retain the acknowledged active configuration until matching graph activation acknowledgement and SHALL never silently select a fallback.

#### Scenario: Engine change in flight
- **WHEN** an Engine option request is loading, validating, preparing, or activating
- **THEN** the Overview Engine control SHALL show the acknowledged Engine and requested Engine as distinct readings with the exact current lifecycle phase

#### Scenario: Post FX occupancy change in flight
- **WHEN** an effect or empty occupancy request for a canonical slot is loading, validating, preparing, or activating
- **THEN** that exact slot SHALL continue to show its acknowledged occupant or empty state and SHALL show the requested occupancy and exact lifecycle phase separately

#### Scenario: Activation acknowledgement commits the option
- **WHEN** matching graph activation is acknowledged
- **THEN** the requested Engine or slot occupancy SHALL become the acknowledged active reading and lifecycle SHALL return to Ready

#### Scenario: Requested option is unavailable
- **WHEN** an Engine or Post FX option request ends with a typed unavailable cause
- **THEN** the originating control SHALL retain its acknowledged active reading and SHALL display the unavailable requested reading and cause without substitution

#### Scenario: Option request fails
- **WHEN** an Engine or Post FX option request ends with another typed failure
- **THEN** the originating control SHALL retain its acknowledged active reading and SHALL display the failed requested reading and exact cause without substitution

#### Scenario: Slot occupancy change fails
- **WHEN** a Post FX occupancy request fails with a typed cause
- **THEN** the exact slot SHALL continue to show its acknowledged occupant or empty state and SHALL display the failed requested occupancy and cause without substitution

### Requirement: Patch Overview production-path evidence
Patch Overview acceptance SHALL pass through `AppState::apply`, `StateProjector`, serialized `SemanticGraphicalViewModel`, the committed webview renderer, and native paint acknowledgement.

#### Scenario: Reducer-to-DOM proof
- **WHEN** acceptance drives Engine, Post FX, Utility, Choice, Detail, return, and sibling-Patch actions
- **THEN** each accepted state SHALL project and paint the corresponding overview focus, value, lifecycle, and return identity

#### Scenario: Capability fixtures
- **WHEN** acceptance runs with multiple installed instrument/effect descriptor fixtures and empty/occupied slot combinations
- **THEN** the overview SHALL retain its section anatomy and derive all production content from those registries
