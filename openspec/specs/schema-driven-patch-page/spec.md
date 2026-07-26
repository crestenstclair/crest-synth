# schema-driven-patch-page Specification

## Purpose
TBD - created by archiving change phase-3-patch-page-projection. Update Purpose after archive.
## Requirements
### Requirement: Semantic two-context selection and stable focus
PATCH and MIXER SHALL be the only top-level contexts. Startup SHALL select MIXER, `1` SHALL directly select MIXER, and `2` SHALL directly select PATCH through normalized input, a semantic context event, and the canonical reducer. Reducer-owned interaction state SHALL retain the prior MIXER selection separately from one PATCH focus identified by stable `PatchId`; accepted Patch installation SHALL initialize that focus to the first Patch in stable installation order.

#### Scenario: Player opens PATCH
- **WHEN** the fixture Patches are installed and the player presses `2`
- **THEN** the input translates to semantic PATCH selection, `AppState::apply` accepts it, PATCH becomes the projected context, and focus resolves to the first installed stable PatchId

#### Scenario: Player returns to MIXER
- **WHEN** the player has a retained MIXER selection, opens PATCH, and presses `1`
- **THEN** MIXER becomes active and its exact prior selection is restored without deriving it from PATCH rows or a window-owned index

#### Scenario: PATCH is selected before installation
- **WHEN** PATCH selection is reduced while no Patch focus can exist
- **THEN** the event is rejected as no Patches installed, state and generation remain unchanged, and a later valid event remains processable

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

### Requirement: Generation-coherent audio-neutral context projection
An accepted context selection SHALL change only reducer-owned interaction context, accepted generation, and the generation-coherent serialized/view projections. Patch/config/envelope/mixer/global values, stable Patch focus, fixed ParameterSnapshot values and layout, active GraphRevision, MIDI routing, prepared ownership, audio-command count, structural queues, and rendered behavior SHALL remain unchanged. The existing basic window SHALL render exactly one immutable active-context projection and SHALL own no context or Patch-page state.

#### Scenario: Context-only transition reaches projections
- **WHEN** a valid MIXER-to-PATCH event is accepted
- **THEN** StateSnapshot, PatchPageProjection, TextProjection, StateTree, EventRecord, and ParameterSnapshot agree on the new generation and context while the parameter snapshot retains the exact prior values and graph revision and no AudioCommand is emitted

#### Scenario: Audio consequence is compared
- **WHEN** before-context and after-context parameter values are rendered from identical prepared engine and mixer state
- **THEN** the stereo output is sample-identical and no structural graph is prepared, published, swapped, acknowledged, or retired

#### Scenario: Basic adapter paints each context
- **WHEN** the same eframe update callback receives `2` and then `1`
- **THEN** consecutive frames render the canonical PATCH and MIXER text projections from AppLoop without adapter-owned tabs, mutable AppState, or a second projector

