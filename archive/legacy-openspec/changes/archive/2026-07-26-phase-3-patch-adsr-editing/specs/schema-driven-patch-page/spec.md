## MODIFIED Requirements

### Requirement: Semantic two-context selection and stable focus
PATCH and MIXER SHALL be the only top-level contexts. Startup SHALL select MIXER, `1` SHALL directly select MIXER, and `2` SHALL directly select PATCH through normalized input, a semantic context event, and the canonical reducer. Reducer-owned interaction state SHALL retain the prior MIXER selection separately from one PATCH focus identified by stable `PatchId` and one PATCH control focus in the nonwrapping order Engine, Attack, Decay, Sustain, Release. Accepted Patch installation SHALL initialize the Patch focus to the first Patch in stable installation order and the control focus to Engine. An accepted PATCH focus navigation SHALL change only reducer-owned interaction focus, accepted generation, and generation-coherent serialized/view projections; Patch/config/envelope/mixer/global values, fixed parameter values and layout, target graph revision, MIDI routing, prepared ownership, audio-command count, structural queues, and rendered behavior MUST remain unchanged.

#### Scenario: Player opens PATCH
- **WHEN** the fixture Patches are installed and the player presses `2`
- **THEN** the input translates to semantic PATCH selection, `AppState::apply` accepts it, PATCH becomes the projected context, Patch focus resolves to the first installed stable PatchId, and control focus resolves to Engine

#### Scenario: Player returns to MIXER
- **WHEN** the player has retained MIXER and PATCH selections, opens PATCH, and presses `1`
- **THEN** MIXER becomes active and its exact prior selection is restored without deriving it from PATCH rows or a window-owned index

#### Scenario: Player returns to PATCH
- **WHEN** the player moves PATCH control focus to an ADSR row, selects MIXER, and later reselects PATCH
- **THEN** the exact stable PatchId and PATCH control focus are restored independently of the retained MIXER selection

#### Scenario: PATCH is selected before installation
- **WHEN** PATCH selection is reduced while no Patch focus can exist
- **THEN** the event is rejected as no Patches installed, state and generation remain unchanged, and a later valid event remains processable

#### Scenario: Focus moves from Engine to Attack
- **WHEN** the player presses bare Down with Engine focused
- **THEN** StateSnapshot, PatchPageProjection, TextProjection, StateTree, EventRecord, and the same-revision ParameterSnapshot agree on Attack focus while no parameter, command, graph, or rendered sample changes

#### Scenario: Temporarily disabled Engine remains focused
- **WHEN** Engine is focused while engine selection is Preparing or Activating
- **THEN** the engine row remains the sole selected text line even though another structural request is disabled

### Requirement: Exact descriptor-driven Patch projection
PATCH SHALL project the focused Patch and active descriptor exactly, one stable focused-control identity, one engine control with registry-ordered choices, and four editable envelope rows derived from the canonical `VoiceEnvelope` descriptor. The projected engine SHALL include active/requested capability, request identity, Ready/Preparing/Activating/Failed status, target revision, editability, and typed failure. The projected envelope SHALL include the exact control identity, semantic id, label, value, bounds, fine/coarse steps, and unit for Attack, Decay, Sustain, and Release in canonical order. Capability-provided Scalar and Structural rows SHALL remain read-only, and projection SHALL contain no engine-specific branch, second envelope value, or runtime owner.

#### Scenario: SoundFont Patch is projected
- **WHEN** the focused Patch uses `instrument.soundfont.hidef`
- **THEN** the page contains exact identity, channel, focused control, ADSR rows, both registry choices, one engine row, and bank, program, percussion, and fixed asset rows in descriptor order

#### Scenario: Braids Patch is projected
- **WHEN** the focused Patch uses `instrument.braids`
- **THEN** the same contract contains exact identity, channel, focused control, ADSR rows, both registry choices, one engine row, and Model, Timbre, and Color exactly once in descriptor order

#### Scenario: Descriptor shape changes
- **WHEN** a conforming capability or envelope descriptor adds, removes, renames, reorders, hides, or disables a declared row
- **THEN** PATCH follows the applicable production descriptor and canonical config and exact schema verification rejects stale duplicated rows or focus identities

#### Scenario: Player navigates the PATCH controls
- **WHEN** PATCH receives bare Up or Down from a non-endpoint control
- **THEN** exactly one adjacent Engine-or-ADSR control becomes focused, the canonical Patch and parameter values remain unchanged, and the text projection marks exactly that row

#### Scenario: Player edits an envelope row
- **WHEN** an ADSR row receives Edit+Left/Right or Edit+Down/Up within its declared bounds
- **THEN** the focused Patch's canonical envelope changes by the descriptor's fine or coarse step, every projection shows the same accepted value, and no audio command or structural effect is emitted

#### Scenario: Player requests the adjacent installed engine
- **WHEN** a ready or recoverable-failed Engine row receives Edit+Left/Right within registry bounds
- **THEN** one semantic adjustment enters `AppState::apply`, generation advances to Preparing, and the active config and graph revision remain unchanged

#### Scenario: Player requests beyond a boundary or another PATCH control
- **WHEN** PATCH receives horizontal Navigate, navigation beyond Engine or Release, Edit+Up/Down on Engine, an engine choice beyond a registry boundary, or an edit of MIDI or capability-provided rows
- **THEN** the applicable typed unchanged rejection starts no preparation, publication, fallback, or alternate mutation and later valid input remains processable

#### Scenario: Selection is preparing
- **WHEN** a correlated request awaits worker completion while an ADSR row is focused or edited
- **THEN** PATCH shows the old accepted config as active, the requested choice and Preparing separately, preserves exact focus, keeps ADSR rows editable, disables another engine request, and contains no worker or prepared object

#### Scenario: Selection is activating
- **WHEN** a validated candidate has committed but activation, retirement, or collection is incomplete
- **THEN** PATCH shows the target as active with Activating and its target revision while preserving Patch identity, control focus, channel, mixer route, canonical ADSR, and every untargeted Patch

#### Scenario: Selection fails and recovers
- **WHEN** matching preparation fails before commit
- **THEN** PATCH shows the typed failure with unchanged source config and graph, preserves control focus and editable ADSR rows, publishes no replacement, and permits a later valid adjacent request without fallback
