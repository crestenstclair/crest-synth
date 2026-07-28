## MODIFIED Requirements

### Requirement: Exact descriptor-driven Patch projection
PATCH SHALL project the focused Patch and active descriptor exactly, plus one stable engine control with registry-ordered choices, active/requested capability, request identity, Ready/Preparing/Activating/Failed status, target revision, editability, and typed failure. Only ready or recoverable-failed Edit+Left/Right SHALL request an adjacent choice; all non-engine rows and directions SHALL remain read-only, and projection SHALL contain no engine-specific branch or runtime owner.

#### Scenario: SoundFont Patch is projected
- **WHEN** the focused Patch uses `instrument.soundfont.hidef`
- **THEN** the page contains exact identity, channel, ADSR, both registry choices, one engine row, and bank, program, percussion, and fixed asset rows in descriptor order

#### Scenario: Braids Patch is projected
- **WHEN** the focused Patch uses `instrument.braids`
- **THEN** the same contract contains exact identity, channel, ADSR, both registry choices, one engine row, and Model, Timbre, and Color exactly once in descriptor order

#### Scenario: Descriptor shape changes
- **WHEN** a conforming descriptor adds, removes, renames, reorders, hides, or disables a parameter
- **THEN** PATCH follows the production descriptor and active config and exact schema verification rejects stale duplicated rows

#### Scenario: Player requests the adjacent installed engine
- **WHEN** a ready or recoverable-failed engine row receives Edit+Left/Right within registry bounds
- **THEN** one semantic adjustment enters `AppState::apply`, generation advances to Preparing, and the active config and graph revision remain unchanged

#### Scenario: Player requests beyond a boundary or another PATCH control
- **WHEN** the choice would wrap or PATCH receives Navigate, Edit+Up/Down, or an edit of MIDI, ADSR, Scalar, or Structural rows
- **THEN** the typed unchanged rejection starts no preparation, publication, fallback, or alternate PATCH mutation and later valid input remains processable

#### Scenario: Selection is preparing
- **WHEN** a correlated request awaits worker completion
- **THEN** PATCH shows the old accepted config as active, the requested choice and Preparing separately, disables another request, and contains no worker or prepared object

#### Scenario: Selection is activating
- **WHEN** a validated candidate has committed but activation, retirement, or collection is incomplete
- **THEN** PATCH shows the target as active with Activating and its target revision while preserving Patch identity, channel, mixer route, ADSR, and every untargeted Patch

#### Scenario: Selection fails and recovers
- **WHEN** matching preparation fails before commit
- **THEN** PATCH shows the typed failure with unchanged source config and graph, publishes no replacement, and permits a later valid adjacent request without fallback

## REMOVED Requirements

### Requirement: Read-only first Patch-page increment
Every PATCH identity, routing, engine, ADSR, Scalar, and Structural row SHALL be read-only in this increment. Navigate or Adjust received while PATCH is active SHALL be rejected as `ActionUnavailableInContext` without changing state or generation, preparing an engine, publishing a graph, or selecting a fallback; later context selection SHALL still succeed.

#### Scenario: Adjustment is attempted in PATCH
- **WHEN** PATCH is active and an Adjust event is reduced
- **THEN** the typed rejection is recorded, canonical state and every projection generation remain unchanged, no audio or structural effect is emitted, and a following MIXER selection is accepted

#### Scenario: Engine choice is inspected
- **WHEN** the player inspects the engine row and its installed choices
- **THEN** the active engine and complete registry are visible but no choice can request preparation, replace the instrument, or change the graph

**Reason**: The completed read-only projection increment is superseded by bounded asynchronous engine selection.

**Migration**: Keep every non-engine PATCH row read-only and admit only the stable engine row with Edit+Left/Right through canonical lifecycle state.
