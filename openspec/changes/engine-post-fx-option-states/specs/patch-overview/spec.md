## MODIFIED Requirements

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
