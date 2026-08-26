## Purpose

Defines the controller-first Engine and Post FX option surfaces, their exact reducer-owned selection lifecycle, responsive presentation, and production-path evidence against the live Figma contract.

## ADDED Requirements

### Requirement: Option entry is reducer-owned and origin-specific
Engine Options and Post FX Options SHALL open only through admitted semantic actions from the active Patch's canonical Engine or effect-slot occupancy control, and the open option subject and return origin SHALL retain that exact stable semantic identity.

#### Scenario: Enter Engine Options
- **WHEN** Edit+Up or the equivalent admitted semantic action is applied while the canonical Engine control is focused on Patch Overview
- **THEN** Engine Options SHALL open for that Patch and SHALL retain the Engine control as its exact return origin

#### Scenario: Enter Post FX Options from every slot
- **WHEN** option entry is applied from each canonical Post FX slot in turn
- **THEN** Post FX Options SHALL open for the exact originating slot whether that slot is occupied or empty

#### Scenario: Duplicate effect capabilities remain slot-specific
- **WHEN** two slots contain the same effect capability and Post FX Options is opened from each slot in turn
- **THEN** each option subject, modal identity, request target, and return origin SHALL retain its own canonical slot identity without using the repeated capability name, DOM position, or list index as identity

#### Scenario: Noncanonical origin cannot fabricate options
- **WHEN** option entry is requested from a control that does not resolve to an Engine, effect-slot occupancy, or other existing generic choice subject
- **THEN** the action SHALL be absent or rejected without opening an Engine or Post FX option surface and without mutating product state

### Requirement: Option content is installed-registry driven
The installed instrument and effect registries SHALL be the only source of production option identities, display labels, ordering, enabled or unavailable state, and counts. Post FX Options SHALL additionally include the canonical empty-occupancy choice; Figma examples SHALL remain fixtures rather than an exhaustive list.

#### Scenario: Engine registry order
- **WHEN** Engine Options opens with a particular installed instrument registry
- **THEN** it SHALL expose exactly that registry's instrument entries in registry order with registry-authored labels and no fixture-derived additions or omissions

#### Scenario: Post FX registry order and empty choice
- **WHEN** Post FX Options opens with a particular installed effect registry
- **THEN** it SHALL expose the canonical empty-occupancy choice and exactly the installed effect entries in one deterministic canonical order with registry-authored effect labels

#### Scenario: Different registry shapes
- **WHEN** acceptance uses registries with different option counts, long labels, and enabled or unavailable entries
- **THEN** the same option projection and presentation SHALL preserve their canonical identities, order, labels, availability, and count without a concrete capability-name branch or fixed count

#### Scenario: No silent option substitution
- **WHEN** a requested registry entry cannot be resolved, validated, prepared, or supplied
- **THEN** the requested identity and typed outcome SHALL remain observable and no other capability or empty occupancy SHALL be selected as a fallback

### Requirement: Modal focus and navigation are singular and deterministic
An open option surface SHALL own exactly one stable semantic focus. Opening, navigation, action availability, selection, close, and return SHALL be derived from reducer-projected state rather than DOM coordinates, visual order, scroll position, or local JavaScript selection state.

#### Scenario: Initial focus on current option
- **WHEN** the current option is enabled when an option surface opens
- **THEN** that exact option SHALL receive the sole modal focus and SHALL be marked independently as both focused and current

#### Scenario: Deterministic initial repair
- **WHEN** the current option cannot receive focus under the current canonical option set
- **THEN** the reducer SHALL focus the first enabled option in canonical order or reject entry if no enabled option exists

#### Scenario: Up and Down navigation
- **WHEN** unmodified Up or Down is applied in an option surface
- **THEN** focus SHALL move non-wrapping among enabled options in canonical order and spatial Left or Right SHALL not escape the modal

#### Scenario: Reducer-projected valid actions
- **WHEN** an option row is projected
- **THEN** it SHALL advertise only the actions the production reducer accepts for that exact row and lifecycle state

### Requirement: Choose and close have exact transactional behavior
Edit SHALL choose the focused enabled option immediately, while Shift+Down SHALL close without choosing. Both outcomes SHALL leave the option surface and restore the exact live origin or a deterministic repaired origin through the reducer.

#### Scenario: Choose the current option
- **WHEN** Edit activates the option already marked current
- **THEN** the modal SHALL close to its exact origin without starting a structural request or changing the acknowledged configuration

#### Scenario: Choose a different Engine
- **WHEN** Edit activates a different enabled Engine option
- **THEN** one correlated structural request SHALL begin and the modal SHALL close to the exact Engine origin while the acknowledged Engine remains active

#### Scenario: Choose a different or empty Post FX occupancy
- **WHEN** Edit activates an enabled effect or empty choice for a canonical slot
- **THEN** one correlated structural request SHALL target that exact slot and the modal SHALL close to that slot origin while its acknowledged occupancy remains active

#### Scenario: Close unchanged
- **WHEN** Shift+Down is applied before choosing
- **THEN** the modal SHALL close to its exact origin with no structural request, no configuration change, and no product mutation beyond the reducer-owned interaction transition

#### Scenario: Disabled or unavailable option cannot be chosen
- **WHEN** activation is attempted on an option the canonical projection marks disabled or unavailable
- **THEN** activation SHALL be absent or rejected and the acknowledged configuration SHALL remain unchanged

### Requirement: Return identity is exact and repair is visible
Option return identity SHALL survive navigation, reprojection, density, and viewport changes. If the exact origin is no longer valid, the reducer SHALL repair it to the nearest enabled canonical sibling deterministically and SHALL expose that repair in the projected status.

#### Scenario: Exact Engine return
- **WHEN** Engine Options closes or accepts a choice while the Engine origin remains valid
- **THEN** Patch Overview SHALL return with that exact Engine control focused

#### Scenario: Exact slot return
- **WHEN** Post FX Options opened from any occupied or empty slot closes or accepts a choice while that origin remains valid
- **THEN** Patch Overview SHALL return with that exact canonical slot focused

#### Scenario: Origin removed while modal is open
- **WHEN** a schema change removes or disables the stored option origin before return
- **THEN** the reducer SHALL choose the nearest enabled sibling in the prior canonical order and the UI SHALL present the removed and replacement identities in explicit repair status

### Requirement: Structural readings retain acknowledged truth
The option workflow and its originating Overview control SHALL distinguish the acknowledged active configuration from the requested configuration and the Loading, Validating, Preparing, Activating, Ready, Unavailable, and Failed lifecycle states. A prepared candidate SHALL NOT become active before graph activation acknowledgement.

#### Scenario: Request begins in Loading
- **WHEN** a different Engine or Post FX option is chosen successfully
- **THEN** the originating control SHALL continue to show the acknowledged active value and SHALL show the requested value and `LOADING` as distinct readings

#### Scenario: Validation and preparation progress
- **WHEN** the correlated request advances through Validating and Preparing
- **THEN** the same active and requested identities SHALL remain visible with the exact `VALIDATING` or `PREPARING` phase and unchanged request correlation

#### Scenario: Candidate is activating
- **WHEN** a complete candidate has been prepared and awaits graph activation acknowledgement
- **THEN** the acknowledged active value SHALL remain active, the requested value SHALL remain distinct, and the exact `ACTIVATING` phase and source/target graph correlation SHALL be observable

#### Scenario: Activation is acknowledged
- **WHEN** graph activation is acknowledged for the matching request and revision
- **THEN** the requested configuration SHALL become the acknowledged active configuration, lifecycle SHALL read `READY`, and the prior active configuration SHALL no longer be presented as current

#### Scenario: Requested capability is unavailable
- **WHEN** a request terminates with a typed unavailable outcome
- **THEN** the acknowledged active value, requested value, `UNAVAILABLE` state, and typed cause SHALL remain distinct and visible with no fallback or substitution

#### Scenario: Structural request fails
- **WHEN** a request terminates with any other typed failure
- **THEN** the acknowledged active value, failed requested value, `FAILED` state, and exact typed cause SHALL remain distinct and visible with no fallback or substitution

### Requirement: Option hierarchy and states align with Figma
Engine Options and Post FX Options SHALL reproduce the live Figma option hierarchy and interaction grammar: option identity header, origin annotation, ordered option list, persistent action guidance, one focused row, and an explicit current reading. Focused, current, disabled, unavailable, loading, and failure states SHALL use text or shape as well as color.

#### Scenario: Engine Options hierarchy
- **WHEN** Engine Options is rendered
- **THEN** its visible hierarchy SHALL identify Engine Options, state that it was opened from the Patch Engine selector, render the projected ordered options, and show only the projected movement, choose, and close guidance

#### Scenario: Post FX Options hierarchy
- **WHEN** Post FX Options is rendered for a canonical slot
- **THEN** its visible hierarchy SHALL identify Post FX Options and the exact originating slot, render the projected ordered occupancy options, and show only the projected movement, choose, and close guidance

#### Scenario: Focus and current are independent
- **WHEN** focus moves away from the current option
- **THEN** one row SHALL retain an explicit current marker and a different single row SHALL retain an explicit focus marker and structural focus treatment

#### Scenario: Non-color progress and failure treatment
- **WHEN** the originating control reports progress, unavailable, or failure
- **THEN** the applicable lifecycle word or typed cause and a structural marker SHALL accompany the semantic color treatment

### Requirement: Option composition is responsive and presentation-only
The option surfaces SHALL use one semantic DOM and fluid responsive composition at Wide, Standard, Intermediate, Compact, 1280×800, and enlarged-text conditions. Every interactive target SHALL retain a 48px minimum on its active axes, and all required content SHALL remain reachable through bounded region scrolling without overlap, unintended clipping, or document-level horizontal overflow.

#### Scenario: Wide and Standard option composition
- **WHEN** an option surface is rendered in Wide or Standard composition
- **THEN** the option hierarchy and persistent Utility SHALL remain readable and reachable within flexible main and bounded side regions

#### Scenario: Intermediate and 1280×800 composition
- **WHEN** an option surface is rendered at an intermediate width or 1280×800
- **THEN** tracks, wrapping, and gaps SHALL remain within responsive bounds while every projected option and required status remains reachable

#### Scenario: Compact composition
- **WHEN** an option surface is rendered in Compact composition
- **THEN** the option region and persistent Utility SHALL stack in semantic document order without creating a second modal tree or changing focus identity

#### Scenario: Maximum registry content and enlarged text
- **WHEN** long labels and the maximum acceptance registry content are rendered at Compact or 1280×800 enlarged text
- **THEN** headers, rows, current/focus markers, lifecycle/failure text, and action guidance SHALL wrap or scroll inside bounded regions while preserving 48px targets and scroll endpoints

#### Scenario: Resize and reflow are presentation only
- **WHEN** one open option projection is resized and reflowed across representative conditions without physical or semantic input
- **THEN** generation, option subject, focus path, return path, option order, current selection, lifecycle, and projected actions SHALL remain unchanged and no semantic action or product mutation SHALL occur

### Requirement: Acceptance follows the production reducer-to-native path
Acceptance SHALL exercise physical input normalization, semantic resolution, the production reducer, projection and serialization, the committed webview renderer, measured DOM observations, and native paint/input evidence. Model-only, hidden-DOM, scaffold-only, or planning-artifact evidence SHALL NOT establish completion.

#### Scenario: Reducer-to-DOM option proof
- **WHEN** Engine and Post FX option workflows are driven from canonical origins through accepted application events
- **THEN** the painted DOM SHALL correspond to the same projected Patch, exact subject/origin, registry option set/order, focus/current markers, valid actions, lifecycle, requested/active readings, typed error, and repair status

#### Scenario: Deterministic rerendering
- **WHEN** the same option projection and observation state are rendered repeatedly at the same viewport
- **THEN** the structural DOM observation and native render acknowledgement identity SHALL be identical

#### Scenario: Native representative measurements
- **WHEN** the native webview runs Wide, Standard, Intermediate, Compact, 1280×800, and enlarged-text fixtures
- **THEN** evidence SHALL record composition mode, region and row bounds, focus identity, target floors, overlap, document and required-region overflow, bounded-scroll reachability, and paint acknowledgement

#### Scenario: Physical keyboard and controller handoff
- **WHEN** a physical keyboard or controller drives Edit+Up entry, Up/Down navigation, Edit choose, and Shift+Down close in the native application
- **THEN** the observed semantic actions and reducer-to-paint results SHALL match the projected workflow for Engine and every occupied or empty Post FX origin

#### Scenario: Shift regression
- **WHEN** the native platform reports Shift modifier transitions separately from directional key transitions
- **THEN** repeated entry and close gestures SHALL remain non-repeatable at the modifier edge and SHALL not crash or unwind through the native input boundary

#### Scenario: Patch Detail and Mixer non-regression
- **WHEN** option-state acceptance runs through the production shell
- **THEN** Instrument/FX Detail subject and return behavior, all sixteen Mixer tracks, Inspector correlation, singular focus, and responsive reachability SHALL remain unchanged

#### Scenario: Direct live-Figma comparison
- **WHEN** final acceptance evidence is reviewed
- **THEN** Engine Options and Post FX Options SHALL be compared directly with live Figma nodes `48:173` and `48:207`, the interaction map `49:3`, Patch Overview `95:202`, and the responsive contract `98:2`, and the result SHALL NOT claim parity for deferred product slices
