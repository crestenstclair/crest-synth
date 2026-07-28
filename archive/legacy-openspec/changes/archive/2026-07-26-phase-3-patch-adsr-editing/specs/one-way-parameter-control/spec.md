## MODIFIED Requirements

### Requirement: Exact keyboard vocabulary
Digit 1 SHALL select MIXER and Digit 2 SHALL select PATCH. Existing bare and K-modified W/S/A/D behavior SHALL remain exact in MIXER. In PATCH, bare W/S SHALL emit semantic Up/Down navigation through the ordered Engine, Attack, Decay, Sustain, Release controls; bare A/D SHALL remain unavailable. On Engine, K+A/D SHALL request the adjacent installed engine and K+W/S SHALL remain unavailable. On an ADSR row, K+A/D SHALL apply the canonical fine decrement/increment and K+S/W SHALL apply the canonical coarse decrement/increment through the existing semantic Adjust event and `AppState::apply`. Worker outcomes and acknowledgements SHALL also enter through correlated semantic events before projection or publication. Every accepted PATCH ADSR adjustment SHALL commit one canonical envelope field before serialization and view projection, publish the complete same-target-revision fixed parameter snapshot, and emit no discrete audio command, preparation request, structural graph, or alternate DSP state. A rejected boundary edit SHALL leave state and all publications unchanged.

#### Scenario: Select a context
- **WHEN** the player presses `1` or `2`
- **THEN** the shared translator emits the matching semantic `SelectContext` without reading AppState or owning page state

#### Scenario: Navigate without editing
- **WHEN** MIXER is active and the player presses bare W, S, A, or D
- **THEN** selection follows the declared wrap and clamp rules without changing a synth parameter

#### Scenario: Edit the selected value
- **WHEN** MIXER is active and the player holds K and presses W, S, A, or D
- **THEN** exactly the selected bounded parameter receives the matching fine or coarse adjustment through the semantic event path

#### Scenario: Release a page key
- **WHEN** the player releases `1` or `2`, including while K is held
- **THEN** no semantic event is emitted and context remains reducer-owned

#### Scenario: Navigate PATCH vertically
- **WHEN** PATCH is active on a non-endpoint Engine-or-ADSR row and the player presses bare W or S
- **THEN** the translator emits Navigate Up or Down and the reducer moves exactly one control without changing a Patch value

#### Scenario: PATCH navigation reaches a boundary or uses horizontal input
- **WHEN** the player presses W on Engine, S on Release, or bare A/D anywhere in PATCH
- **THEN** the semantic Navigate is rejected unchanged as unavailable in context and later valid input remains processable

#### Scenario: Select an engine in PATCH
- **WHEN** PATCH is active on Engine and the player holds K and presses A or D
- **THEN** one normalized Adjust requests the adjacent nonwrapping registry choice without the translator inspecting a capability, worker, or graph

#### Scenario: Edit ADSR finely in PATCH
- **WHEN** PATCH is active on an ADSR row and the player holds K and presses A or D within its bounds
- **THEN** exactly the focused Patch envelope value receives the descriptor-owned fine decrement or increment

#### Scenario: Edit ADSR coarsely in PATCH
- **WHEN** PATCH is active on an ADSR row and the player holds K and presses S or W within its bounds
- **THEN** exactly the focused Patch envelope value receives the descriptor-owned coarse decrement or increment

#### Scenario: PATCH ADSR edit reaches every scalar projection
- **WHEN** a focused ADSR adjustment remains within its typed bounds
- **THEN** accepted state, PatchPageProjection, TextProjection, StateTree, EventRecord, and ParameterSnapshot agree on one new generation and exact value while structural and discrete effect sets are empty

#### Scenario: PATCH ADSR edit reaches a boundary
- **WHEN** a focused ADSR adjustment points beyond an already reached typed bound
- **THEN** the reducer records the normal nonfatal parameter-boundary rejection, makes no publication, and accepts the next valid adjustment

#### Scenario: Accepted engine request preserves the source
- **WHEN** the reducer accepts an adjacent engine choice
- **THEN** it commits Preparing and emits one correlated preparation request before projection while accepted config, graph revision, parameter values, control focus, and active audio remain unchanged

#### Scenario: Prepared result commits before graph publication
- **WHEN** a complete matching candidate arrives through the worker outcome event
- **THEN** `AppState::apply` commits the target config and Activating state before target-revision state, text, parameters, structural effects, or graph ownership are published and preserves canonical envelope and control focus

#### Scenario: Failed, stale, or early lifecycle input arrives
- **WHEN** a worker failure, stale or mismatched result, busy engine request, or early acknowledgement is reduced
- **THEN** it records visible source-preserving failure or rejects unchanged, emits no invalid structural effect, and leaves later MIDI, context, PATCH focus, and valid scalar events processable

#### Scenario: Text view follows canonical lifecycle
- **WHEN** PATCH renders Ready, Preparing, Activating, or Failed state and the player later returns to MIXER
- **THEN** the shell displays only the canonical immutable projection, marks exactly the reducer-focused PATCH row, owns no lifecycle or graph state, and restores the exact retained MIXER selection

### Requirement: Single complete text view
The basic user interface SHALL remain one scrollable text shell rendering exactly one reducer-selected projection at a time. MIXER SHALL preserve the complete existing descriptor-derived Patch/global diagnostic body, separators, editable selection, and selected-line marker. PATCH SHALL render the focused immutable PatchPageProjection with Patch identity, MIDI channel, active engine and complete installed choices, canonical ADSR, active descriptor fields, stable IDs, editability, and exactly one selected Engine-or-ADSR line. Capability-provided rows SHALL remain read-only. Neither context SHALL contain a hard-coded SoundFont/Braids field list, and the window SHALL own no page or focus state.

#### Scenario: View is projected from current state
- **WHEN** MIXER is active and installed Patches, generic configs, envelope/mixer values, Scalar engine values, global values, or MIXER selection change
- **THEN** the text body contains every exact current value, retains required Patch separators, follows production descriptor order, and places one marker at the projected selected line

#### Scenario: Patch capabilities expose different shapes
- **WHEN** Patch projection changes between a SoundFont Patch and a Braids Patch
- **THEN** MIXER derives its editable selection count from the active descriptor, skips SoundFont Structural fields, includes Braids Model, Timbre, and Color, and PATCH projects each different descriptor shape without a hard-coded engine field list

#### Scenario: PATCH is projected from current state
- **WHEN** PATCH is active for a focused SoundFont or Braids Patch
- **THEN** the text body losslessly renders the canonical PatchPageProjection, marks the exact focused Engine-or-ADSR row, exposes the four ADSR rows as editable, and displays capability-provided Structural and Scalar values as read-only

#### Scenario: Disabled Engine remains selected
- **WHEN** Engine is focused while its structural action is disabled during Preparing or Activating
- **THEN** the shell still marks Engine as the sole selected line and separately renders its disabled lifecycle state

#### Scenario: Context is round-tripped
- **WHEN** the player switches from a discriminating MIXER selection to a discriminating PATCH control focus and back
- **THEN** the same shell renders each canonical body in turn and later restores each context's exact retained selection and values
