## ADDED Requirements

### Requirement: Detail subjects are reducer-owned and canonical
Instrument Detail and FX Detail SHALL exist only as the projection of the reducer's subordinate Detail session. The subject, focus, and return origin MUST be stable semantic identities and MUST NOT be owned or reconstructed by the DOM.

#### Scenario: Instrument Detail entry
- **WHEN** Shift+Up or the equivalent admitted semantic action is applied through `AppState::apply` while the active Patch Engine origin is focused
- **THEN** the subordinate Detail session SHALL identify the active Patch instrument and the projected surface SHALL be Instrument Detail

#### Scenario: FX Detail entry at every occupied position
- **WHEN** Detail entry is applied through `AppState::apply` from each occupied canonical Patch effect-slot position
- **THEN** the subordinate subject SHALL identify that exact slot and active effect and the projected surface SHALL be FX Detail for the originating position

#### Scenario: Duplicate effect identities remain position-specific
- **WHEN** two occupied slots contain the same effect capability and Detail opens from each slot in turn
- **THEN** each Detail subject and return origin SHALL retain its own stable slot identity and canonical position without deriving identity from the shared name, label, DOM index, or visual order

#### Scenario: Empty slot cannot fabricate Detail
- **WHEN** the focused canonical effect slot is unoccupied and Detail entry is requested
- **THEN** the reducer SHALL reject or omit that Detail action according to its valid workflow and no FX Detail subject SHALL be projected

### Requirement: Detail content is descriptor-driven and ordered
Instrument Detail and FX Detail SHALL render the canonical projected sections and controls in declared order. Installed registries SHALL determine capability labels, section labels, controls, values, and counts; JavaScript MUST NOT define an instrument/effect schema or switch on concrete capability names.

#### Scenario: Instrument descriptor order
- **WHEN** an installed instrument descriptor declares multiple parameter sections
- **THEN** Instrument Detail SHALL project and render those sections and their visible controls in descriptor order

#### Scenario: Canonical shared envelope
- **WHEN** the canonical instrument Detail projection supplies a shared envelope section and controls
- **THEN** the renderer SHALL include them at their projected position without manufacturing or duplicating envelope controls in JavaScript

#### Scenario: Different instrument shapes share one path
- **WHEN** two installed instrument descriptors expose different section labels, interactions, control kinds, and parameter counts
- **THEN** both Instrument Detail states SHALL render through the same projection and DOM composition with no capability-name branch

#### Scenario: FX descriptor order and count
- **WHEN** an occupied effect descriptor declares one or more parameter sections
- **THEN** FX Detail SHALL render those sections and parameters in descriptor order and derive its visible count from the projection

#### Scenario: Different effect shapes share one path
- **WHEN** two installed effect descriptors expose different parameter counts and control shapes
- **THEN** both FX Detail states SHALL render through the same projection and DOM composition with no effect-name branch or fixture count

#### Scenario: Detail rows are not hidden Overview targets
- **WHEN** Instrument Detail or FX Detail is open or closed
- **THEN** its descriptor/envelope parameters SHALL exist only in the subordinate Detail focus order and SHALL NOT be duplicated as hidden Patch Overview focus targets

### Requirement: Detail identity and shell hierarchy are explicit
Every Detail composition SHALL present a clear Detail identity, active Patch identity, active subject label/status, current semantic breadcrumb, projected valid footer actions, and persistent PATCH Utility. FX Detail SHALL additionally present the canonical selected slot position.

#### Scenario: Instrument header identity
- **WHEN** Instrument Detail is rendered
- **THEN** its header SHALL identify Detail, the active Patch, the active instrument display name, status, and projected control count without exposing a registry key as the display label

#### Scenario: FX header identity
- **WHEN** FX Detail is rendered for an occupied slot
- **THEN** its header SHALL identify Detail, the active Patch, canonical slot position, active effect display label, status, and projected control count

#### Scenario: Persistent Utility
- **WHEN** either Detail subject is active in any responsive layout
- **THEN** Master Volume, Patch Volume, MIDI Input, Output Track, and Voice Limit SHALL remain projected, visible or scroll-reachable, and semantically reachable

#### Scenario: Breadcrumb and footer
- **WHEN** the focused Detail target or interaction mode changes through an accepted semantic action
- **THEN** the visible breadcrumb and footer SHALL reflect the current projected focus path and only the reducer-resolved valid actions

### Requirement: Detail controls use one complete state vocabulary
Instrument and effect controls SHALL share one parameter-row grammar for labels, values, optional units, bounds, normalized position, action hints, lifecycle, focus, adjustment, disabled/read-only, unavailable, and typed failure. Important state MUST be expressed with text or shape as well as color.

#### Scenario: Scalar reading
- **WHEN** a projected Detail control has a numeric value, unit, and bounds
- **THEN** its row SHALL render the active value, unit, range, and a position indicator derived from the projected value and bounds

#### Scenario: Supported interactions
- **WHEN** a projected Detail control supports fine, coarse, adjacent-choice, confirm, or toggle actions
- **THEN** its row and the focused footer SHALL present only those projected valid actions and SHALL NOT infer them from the label or control kind

#### Scenario: Focus and adjustment treatments
- **WHEN** a Detail control is focused in Navigate mode or Adjust mode
- **THEN** exactly that visible control SHALL show the corresponding structural focus or adjustment marker and textual breadcrumb reading in addition to color

#### Scenario: Disabled unavailable and read-only treatments
- **WHEN** a Detail control is disabled, unavailable, or descriptor-declared read-only
- **THEN** the row SHALL distinguish the applicable state with an explicit text/shape marker and SHALL NOT present an unsupported adjustment as available

#### Scenario: Requested and active structural readings
- **WHEN** a structural control has an in-flight requested value
- **THEN** the active value and requested value SHALL remain simultaneously visible and distinct with the projected lifecycle phase

#### Scenario: Loading and preparation phases
- **WHEN** a projected structural lifecycle is loading, validating, preparing, or activating
- **THEN** Detail SHALL render that phase as in progress and SHALL NOT present the requested capability/value as active

#### Scenario: Typed structural failure
- **WHEN** a structural request fails with a typed cause
- **THEN** the active value SHALL remain unchanged, the requested/failed state and cause SHALL remain visible, and no substitute capability or fallback value SHALL be selected

### Requirement: Detail navigation preserves singular focus and return identity
Detail interaction SHALL use existing semantic actions and reducer-resolved adjacency. Exactly one semantic target MUST be focused at all times, and closing Detail SHALL restore the originating Engine or effect-slot identity or the reducer's deterministic repair.

#### Scenario: Unmodified focus movement
- **WHEN** unmodified arrows move within projected Detail controls or between Detail and persistent Utility
- **THEN** semantic focus SHALL move according to the resolver's adjacency without using DOM coordinates or scroll position as selection authority

#### Scenario: Fine and coarse editing
- **WHEN** Edit+Left/Right or Edit+Up/Down is applied to a focused control that projects the corresponding action
- **THEN** the semantic action SHALL pass through `AppState::apply`, update only canonical product state, and reproject the accepted value

#### Scenario: Confirm or toggle
- **WHEN** Edit is applied to a focused control that projects confirm or toggle
- **THEN** only the reducer-defined semantic behavior SHALL occur and the DOM SHALL remain an immutable projection consumer

#### Scenario: Close Instrument Detail
- **WHEN** Shift+Down closes Instrument Detail
- **THEN** Patch Overview SHALL return with the exact originating Engine identity focused

#### Scenario: Close FX Detail
- **WHEN** Shift+Down closes FX Detail opened from any occupied slot position
- **THEN** Patch Overview SHALL return with that exact canonical effect-slot identity focused

#### Scenario: Return origin removed by schema change
- **WHEN** a schema change removes or disables the stored return origin before Detail closes
- **THEN** the reducer SHALL select the nearest enabled semantic sibling deterministically and expose the repair in projected status

### Requirement: Detail composition is responsive presentation only
Instrument Detail and FX Detail SHALL reuse the responsive shell and shared layout tokens with Grid/Flex, intrinsic sizing, `minmax()`, `clamp()`, wrapping, bounded gaps/tracks, independent content scrolling, and the existing 48px interactive-target floor. Layout mode and viewport changes MUST NOT become product state or semantic actions.

#### Scenario: Wide and Standard composition
- **WHEN** Detail is rendered at a wide desktop width, 1440px, 1280×800, or an intermediate width
- **THEN** the main Detail region and bounded Utility region SHALL remain readable and reachable while parameter rows use available horizontal space without overlapping required content

#### Scenario: Compact composition
- **WHEN** Detail is rendered at a compact width
- **THEN** main Detail content and Utility SHALL stack in semantic document order and all projected controls SHALL remain reachable

#### Scenario: Enlarged text
- **WHEN** Detail is rendered at 1280×800 with enlarged text
- **THEN** required identity, section, value, unit, lifecycle, failure, focus, and action content SHALL wrap or scroll without document-level horizontal overflow or overlapping controls

#### Scenario: Interactive target floor
- **WHEN** any representative viewport or text scale is measured
- **THEN** every interactive Detail and Utility target SHALL retain a minimum 48px target on its active axes

#### Scenario: Resize preserves semantic projection
- **WHEN** an open Detail surface is resized across presentation modes without physical or semantic input
- **THEN** generation, focus path, return path, subject identity, surface set, and projected controls SHALL remain unchanged

#### Scenario: Resize emits no product action
- **WHEN** resize, wrapping, scrolling, or presentation-mode changes occur
- **THEN** no semantic action SHALL be emitted and `AppState::apply` SHALL NOT be invoked for layout

### Requirement: Detail acceptance uses the production path and isolates regressions
Acceptance SHALL exercise `AppState::apply`, semantic resolution, state projection, serialized `SemanticGraphicalViewModel`, the committed webview renderer, measured DOM observations, and native paint/controller paths. Passing model-only or hidden-DOM tests SHALL NOT be sufficient evidence.

#### Scenario: Reducer-to-DOM subject proof
- **WHEN** Instrument Detail and FX Detail fixtures are opened from canonical origins through accepted application events
- **THEN** the rendered DOM SHALL correspond to the same projected Patch, subject, sections, controls, focus, lifecycle, return path, and valid actions

#### Scenario: One focused visible target
- **WHEN** every accepted Instrument Detail and FX Detail state is rendered
- **THEN** the DOM SHALL contain exactly one focused visible semantic target matching the projected focus path

#### Scenario: Deterministic repeated rendering
- **WHEN** the same Detail projection and observation state are rendered repeatedly at the same viewport
- **THEN** the structural DOM observation and render acknowledgement identity SHALL be identical

#### Scenario: Measured layout evidence
- **WHEN** representative wide, 1440px, 1280×800, intermediate, compact, and enlarged-text conditions are exercised
- **THEN** observations SHALL record layout mode, region bounds, focus identity, target sizes, overflow, overlap, scroll reachability, and paint acknowledgement

#### Scenario: Native Shift regression
- **WHEN** AppKit delivers Shift modifier transitions as `FlagsChanged` events
- **THEN** the input adapter SHALL treat them as non-repeatable, SHALL never query the key-repeat property, and SHALL not unwind through the native callback boundary

#### Scenario: Mixer non-regression
- **WHEN** this slice's production projection and responsive renderer tests exercise MIXER
- **THEN** the Mixer projection, all sixteen track identities, Inspector correlation, focus, and reachability SHALL remain unaffected

#### Scenario: Real-time boundary remains unchanged
- **WHEN** the Detail slice is implemented and validated
- **THEN** no Detail rendering, resize, observation, allocation, lock, I/O, logging, formatting, or destruction work SHALL be introduced into the audio callback
