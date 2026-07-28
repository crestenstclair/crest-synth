## MODIFIED Requirements

### Requirement: Exact keyboard vocabulary
Digit 1 SHALL emit `SelectContext(MIXER)` and Digit 2 SHALL emit `SelectContext(PATCH)`. Edit-key down SHALL emit `SetInteractionMode(Adjust)`; Edit-key up and physical focus loss SHALL emit `SetInteractionMode(Navigate)`. Directional input SHALL emit `Navigate` or `Adjust` according to the reducer-owned mode without the translator reading `AppState`. Existing bare and Edit-modified W/S/A/D behavior SHALL remain exact in MIXER. In PATCH Main Navigate mode, bare W/S SHALL move through descriptor order, bare D SHALL enter PatchUtility with the exact origin recorded, and bare A SHALL remain unavailable; Return or bare A from PatchUtility SHALL restore that origin. In PATCH Main Adjust mode, A/D on Engine SHALL request the adjacent installed engine, W/S on Engine SHALL remain unavailable, A/D on an ADSR row SHALL apply its canonical fine decrement/increment, and S/W on an ADSR row SHALL apply its canonical coarse decrement/increment through `AppState::apply`. Worker outcomes and acknowledgements SHALL continue to enter as correlated `AppEvent` values rather than `SemanticAction`. Every accepted PATCH ADSR adjustment SHALL commit one canonical envelope field before serialization and view projection, publish the complete same-target-revision fixed parameter snapshot, and emit no discrete audio command, preparation request, structural graph, or alternate DSP state. Context, focus, surface, mode, and return-only transitions SHALL be audio-neutral, and a rejected boundary action SHALL leave state publications unchanged.

#### Scenario: Select a context
- **WHEN** the player presses `1` or `2`
- **THEN** the shared translator emits the matching semantic `SelectContext` action without reading AppState or owning page state, and `AppLoop` maps it to the matching reducer event

#### Scenario: Navigate without editing
- **WHEN** MIXER is active in Navigate mode and the player presses W, S, A, or D
- **THEN** selection follows the declared wrap and clamp rules without changing a synth parameter

#### Scenario: Edit the selected value
- **WHEN** MIXER is active and the player presses Edit followed by W, S, A, or D
- **THEN** Adjust mode is reducer-owned and exactly the selected bounded parameter receives the matching fine or coarse adjustment through the semantic action/event path

#### Scenario: Press and release Edit
- **WHEN** the player presses and releases Edit without a direction, or the window loses physical focus while Edit is down
- **THEN** Adjust and then Navigate are accepted as explicit interaction modes without changing focus, parameters, graph revision, routing, commands, or rendered audio

#### Scenario: Release a page key
- **WHEN** the player releases `1` or `2`, including while Edit is held
- **THEN** no semantic action is emitted and context remains reducer-owned

#### Scenario: Navigate PATCH vertically
- **WHEN** PATCH Main is active in Navigate mode on a non-endpoint descriptor control and the player presses W or S
- **THEN** the translator emits Navigate Up or Down and the reducer moves exactly one focusable control without changing a Patch value

#### Scenario: PATCH navigation reaches a boundary or uses horizontal input
- **WHEN** the player presses W on the first focusable PATCH control, S on the last focusable control, or bare A in PATCH Main
- **THEN** the semantic Navigate is rejected unchanged as unavailable in context and later valid input remains processable

#### Scenario: Enter and leave PATCH Utility
- **WHEN** PATCH Main is active and the player presses bare D, then presses bare A or invokes Return from Utility
- **THEN** the reducer focuses PatchUtility's `SurfaceRoot`, records the exact main origin, and then restores that origin and clears the return path without changing audio or parameter state

#### Scenario: Select an engine in PATCH
- **WHEN** PATCH Main is active on Engine in Adjust mode and the player presses A or D
- **THEN** one normalized Adjust requests the adjacent nonwrapping registry choice without the translator inspecting a capability, worker, or graph

#### Scenario: Edit ADSR finely in PATCH
- **WHEN** PATCH Main is active on an ADSR row in Adjust mode and the player presses A or D within its bounds
- **THEN** exactly the focused Patch envelope value receives the descriptor-owned fine decrement or increment

#### Scenario: Edit ADSR coarsely in PATCH
- **WHEN** PATCH Main is active on an ADSR row in Adjust mode and the player presses S or W within its bounds
- **THEN** exactly the focused Patch envelope value receives the descriptor-owned coarse decrement or increment

#### Scenario: PATCH ADSR edit reaches every scalar projection
- **WHEN** a focused ADSR adjustment remains within its typed bounds
- **THEN** accepted state, PatchPageProjection, SemanticGraphicalViewModel, TextProjection, StateTree, EventRecord, and ParameterSnapshot agree on one new generation and exact value while structural and discrete effect sets are empty

#### Scenario: PATCH ADSR edit reaches a boundary
- **WHEN** a focused ADSR adjustment points beyond an already reached typed bound
- **THEN** the reducer records the normal nonfatal parameter-boundary rejection, makes no publication, and accepts the next valid adjustment

#### Scenario: Accepted engine request preserves the source
- **WHEN** the reducer accepts an adjacent engine choice
- **THEN** it commits Preparing and emits one correlated preparation request before projection while accepted config, graph revision, parameter values, semantic focus, and active audio remain unchanged

#### Scenario: Prepared result commits before graph publication
- **WHEN** a complete matching candidate arrives through the worker outcome event
- **THEN** `AppState::apply` commits the target config and Activating state before target-revision state, semantic/text projections, parameters, structural effects, or graph ownership are published and preserves canonical envelope plus valid semantic focus or its deterministic repair

#### Scenario: Failed, stale, or early lifecycle input arrives
- **WHEN** a worker failure, stale or mismatched result, busy engine request, or early acknowledgement is reduced
- **THEN** it records visible source-preserving failure or rejects unchanged, emits no invalid structural effect, and leaves later MIDI, context, PATCH focus, and valid scalar actions processable

#### Scenario: Text view follows canonical lifecycle
- **WHEN** PATCH renders Ready, Preparing, Activating, or Failed state and the player later returns to MIXER
- **THEN** the shell and retained text display only canonical immutable projections, mark exactly the reducer-focused semantic target, own no lifecycle or graph state, and restore the exact retained MIXER focus path
