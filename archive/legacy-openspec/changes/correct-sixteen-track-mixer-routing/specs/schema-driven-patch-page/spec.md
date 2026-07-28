## MODIFIED Requirements

### Requirement: Semantic two-context selection and stable focus
PATCH and MIXER SHALL be the only top-level contexts. Startup SHALL select MIXER with T00 Level focused, `1` SHALL directly select MIXER, and `2` SHALL directly select PATCH through normalized input, a semantic context event, and the canonical reducer. Reducer-owned interaction state SHALL retain the prior MIXER focus by stable `MixerTrackId` plus `MixerTrackParameter`, separately from one PATCH focus identified by stable `PatchId`, surface, and `PatchControlId`. PATCH Main SHALL retain its descriptor-derived nonwrapping order beginning Engine, Attack, Decay, Sustain, Release, while PATCH Utility SHALL resolve the same Patch's Trim Gain followed by Output Track. Accepted Patch installation SHALL initialize PATCH focus to the first Patch in stable installation order and PATCH Main Engine without changing the fixed mixer bank. Accepted context, surface, or focus navigation SHALL change only reducer-owned interaction focus, accepted generation, and generation-coherent serialized/view projections; Patch/output/envelope/track/global values, fixed parameter values and layout, target graph revision, MIDI routing, prepared ownership, audio-command count, structural queues, and rendered behavior MUST remain unchanged.

#### Scenario: Player opens PATCH
- **WHEN** the fixture Patches are installed and the player presses `2`
- **THEN** the input translates to semantic PATCH selection, `AppState::apply` accepts it, PATCH becomes the projected context, Patch focus resolves to the first installed stable PatchId, and PATCH Main control focus resolves to Engine

#### Scenario: Player returns to MIXER
- **WHEN** the player has focused one track parameter in MIXER, opens PATCH, and presses `1`
- **THEN** MIXER becomes active and the exact prior `MixerTrackId` plus `MixerTrackParameter` selection is restored without deriving it from PATCH rows or a window-owned index

#### Scenario: Player returns to PATCH
- **WHEN** the player moves PATCH control focus to an ADSR row, selects MIXER, and later reselects PATCH
- **THEN** the exact stable PatchId, PATCH surface, and PATCH control focus are restored independently of the retained MIXER selection

#### Scenario: Player edits Patch output in Utility
- **WHEN** the player enters PATCH Utility for the focused Patch and adjusts Trim Gain or Output Track
- **THEN** the canonical Patch output changes through `AppState::apply`, the Utility focus remains a stable `PatchControlId`, and no mixer-track parameter or graph revision changes

#### Scenario: PATCH is selected before installation
- **WHEN** PATCH selection is reduced while no Patch focus can exist
- **THEN** the event is rejected as no Patches installed, state and generation remain unchanged, and a later valid event remains processable

#### Scenario: Focus moves from Engine to Attack
- **WHEN** the player presses bare Down with Engine focused in PATCH Main
- **THEN** StateSnapshot, PatchPageProjection, TextProjection, StateTree, EventRecord, and the same-revision ParameterSnapshot agree on Attack focus while no parameter, command, graph, or rendered sample changes

#### Scenario: Temporarily disabled Engine remains focused
- **WHEN** Engine is focused while engine selection is Preparing or Activating
- **THEN** the engine row remains the sole selected text line even though another structural request is disabled
