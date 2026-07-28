## MODIFIED Requirements

### Requirement: Semantic two-context selection and stable focus
PATCH and MIXER SHALL be the only top-level contexts. Startup SHALL select MIXER, `1` SHALL directly select MIXER, and `2` SHALL directly select PATCH through normalized `SemanticAction`, matching `AppEvent`, and the canonical reducer. Reducer-owned `InteractionState` SHALL retain exactly one active stable `FocusPath`, separate remembered PatchMain and MixerMain paths, one explicit mode, and at most one side-surface `ReturnPath`; PATCH paths SHALL use stable `PatchId`, descriptor capability identity where applicable, and `PatchControlId`, while MIXER paths SHALL use stable typed target identity rather than position. Accepted Patch installation SHALL initialize PATCH focus to the first Patch in stable installation order and Engine. Accepted PATCH navigation, side-surface entry/return, and focus recovery SHALL change only reducer-owned interaction state, accepted generation, and generation-coherent serialized/view projections; Patch/config/envelope/mixer/global values, fixed parameter values and layout, target graph revision, MIDI routing, prepared ownership, audio-command count, structural queues, and rendered behavior MUST remain unchanged. After committed schema change, invalid PATCH paths SHALL recover in canonical descriptor order by nearest surviving visible focusable sibling with next-before-previous tie breaking.

#### Scenario: Player opens PATCH
- **WHEN** the fixture Patches are installed and the player presses `2`
- **THEN** input becomes semantic PATCH selection, `AppState::apply` accepts it, PATCH becomes projected context, and the remembered PatchMain path resolves to the first installed stable PatchId and Engine

#### Scenario: Player returns to MIXER
- **WHEN** the player has retained distinct MixerMain and PatchMain paths, opens PATCH, and presses `1`
- **THEN** MIXER becomes active and its exact prior stable path is restored without deriving it from PATCH rows or a window-owned index

#### Scenario: Player returns to PATCH
- **WHEN** the player moves PATCH focus to an ADSR row, selects MIXER, and later reselects PATCH
- **THEN** the exact stable PatchId, capability identity where applicable, and `PatchControlId` are restored independently of the retained MIXER path

#### Scenario: PATCH is selected before installation
- **WHEN** PATCH selection is reduced while no Patch focus can exist
- **THEN** the event is rejected as no Patches installed, state and generation remain unchanged, and a later valid event remains processable

#### Scenario: Focus moves from Engine to Attack
- **WHEN** the player presses Down in Navigate mode with Engine focused
- **THEN** StateSnapshot, PatchPageProjection, SemanticGraphicalViewModel, TextProjection, StateTree, EventRecord, and same-revision ParameterSnapshot agree on Attack focus while no parameter, command, graph, or rendered sample changes

#### Scenario: Temporarily disabled Engine remains focused
- **WHEN** Engine is focused while engine selection is Preparing or Activating
- **THEN** the engine row remains the sole selected, visible, focus-eligible control while another structural Adjust action is unavailable

#### Scenario: Player enters and returns from Utility
- **WHEN** a PatchMain control is focused and the player enters PatchUtility, then invokes Return
- **THEN** the reducer first focuses Utility's `SurfaceRoot` and records the exact PatchMain path, then restores that same path and clears the return state

#### Scenario: Focused capability row disappears
- **WHEN** a committed engine replacement removes the focused capability-specific row
- **THEN** `AppState` repairs active, remembered, and return paths through the shared next-before-previous resolver before projecting the accepted generation

#### Scenario: Viewport layout changes
- **WHEN** the PATCH model is composed at 1920×1080 and 1280×800
- **THEN** the exact semantic path and remembered/return state remain unchanged even when control rectangles, wrapping, density, and scroll positions differ
