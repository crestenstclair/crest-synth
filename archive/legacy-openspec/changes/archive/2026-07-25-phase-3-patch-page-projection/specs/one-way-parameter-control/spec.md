## MODIFIED Requirements

### Requirement: Exact keyboard vocabulary
Digit `1` SHALL directly select MIXER and Digit `2` SHALL directly select PATCH. Bare W and S SHALL navigate parameters, bare A and D SHALL navigate Patch or GLOBAL sections, and holding K with W, S, A, or D SHALL adjust only the selected bounded value while MIXER is active. Digit key-up and every other supported key-up SHALL emit no semantic event, K SHALL act only as a modifier, and no physical key SHALL mutate application state directly.

#### Scenario: Select a context
- **WHEN** the player presses `1` or `2`
- **THEN** the shared input translator emits the corresponding semantic `SelectContext` event without reading AppState or changing window-owned page state

#### Scenario: Navigate without editing
- **WHEN** MIXER is active and the player presses bare W, S, A, or D
- **THEN** selection moves according to the declared wrap and clamp rules without changing a synth parameter

#### Scenario: Edit the selected value
- **WHEN** MIXER is active and the player holds K and presses W, S, A, or D
- **THEN** exactly the selected bounded parameter receives the corresponding fine or coarse adjustment through the semantic event path

#### Scenario: Release a page key
- **WHEN** the player releases `1` or `2`, including while K is held
- **THEN** no semantic event is emitted and the current context remains reducer-owned

### Requirement: Single complete text view
The basic user interface SHALL remain one scrollable text shell rendering exactly one reducer-selected projection at a time. MIXER SHALL preserve the complete existing descriptor-derived Patch/global diagnostic body, separators, editable selection, and selected-line marker. PATCH SHALL render the focused immutable PatchPageProjection with Patch identity, MIDI channel, active engine and complete installed choices, canonical ADSR, and active descriptor fields with stable IDs and read-only status. Neither context SHALL contain a hard-coded SoundFont/Braids field list, and the window SHALL own no page state.

#### Scenario: View is projected from current state
- **WHEN** MIXER is active and installed Patches, generic configs, envelope/mixer values, Scalar engine values, global values, or MIXER selection change
- **THEN** the text body contains every exact current value, retains required Patch separators, follows production descriptor order, and places one marker at the projected selected line

#### Scenario: Patch capabilities expose different shapes
- **WHEN** navigation or Patch focus moves between a SoundFont Patch and a Braids Patch
- **THEN** MIXER derives its editable selection count from the active descriptor, skips SoundFont Structural fields, includes Braids Model, Timbre, and Color, and PATCH projects each different descriptor shape without a hard-coded engine field list

#### Scenario: PATCH is projected from current state
- **WHEN** PATCH is active for a focused SoundFont or Braids Patch
- **THEN** the text body is a lossless deterministic rendering of that canonical PatchPageProjection and displays Structural values without making any page row editable

#### Scenario: Context is round-tripped
- **WHEN** the player switches from a discriminating MIXER selection to PATCH and back
- **THEN** the same shell renders each canonical body in turn and returns to the exact retained MIXER selection and diagnostic values
