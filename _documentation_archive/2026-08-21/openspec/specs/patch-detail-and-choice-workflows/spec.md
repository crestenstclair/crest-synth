# patch-detail-and-choice-workflows Specification

## Purpose
TBD - created by archiving change implement-phase-7-detail-choice-assets. Update Purpose after archive.
## Requirements
### Requirement: One descriptor-driven Patch detail shell
The system SHALL project instrument and effect detail through one shared subordinate `PatchDetail` shell populated from the active capability descriptor. The descriptor SHALL own the ordered sections, semantic control identities, labels, values, ranges, units, choices, dependency rules, availability, and accent; the reducer, projector, component composition, and renderer MUST NOT enumerate concrete engine or effect identities to determine the detail fields.

#### Scenario: Different capabilities reuse the same shell
- **WHEN** instrument detail and an occupied effect-slot detail are opened for capabilities with different descriptor schemas
- **THEN** both surfaces use the same shell and component vocabulary while each presents its descriptor-owned ordered content

#### Scenario: Descriptor dependencies change
- **WHEN** a canonical edit changes whether a descriptor control is enabled or visible
- **THEN** the production projector re-resolves the ordered detail surface from the descriptor without retaining a UI-owned copy of the prior fields

### Requirement: Detail navigation preserves semantic origin
The system SHALL open instrument detail from the focused instrument control and effect detail from the focused occupied effect-slot control through semantic actions reduced by `AppState::apply`. The subordinate surface SHALL store a stable semantic return path, SHALL keep exactly one enabled control focused, SHALL skip disabled controls, and SHALL return to the exact originating control on Shift+Down while that control remains valid.

#### Scenario: Return from instrument detail
- **WHEN** the user opens instrument detail from a Patch instrument control, moves among detail rows, and invokes Shift+Down
- **THEN** focus returns to that exact Patch and instrument control rather than to a widget index or default row

#### Scenario: Origin disappears during reprojection
- **WHEN** a structural commit removes the control recorded in a detail return path
- **THEN** the reducer selects the nearest valid semantic sibling using the documented deterministic resolver and reports the recovery in explicit status text

### Requirement: Detail visualizations do not acquire focus
The system SHALL render capability-declared informative visualizations outside the focus order. Non-Sample instruments that use the shared `VoiceEnvelope` SHALL project a live ADSR shape, and effect detail SHALL project descriptor-declared informative status or shape data without creating a second editable domain model; Sample-specific waveform behavior is governed by the Sample capability specification.

#### Scenario: Navigate across a visualization
- **WHEN** the detail surface contains a visualization between two ordered editable sections
- **THEN** Up or Down moves directly between semantic controls and the visualization never becomes the active `FocusPath`

### Requirement: Choice modals use installed and descriptor-owned choices
The system SHALL expose engine, effect occupancy, Mixer route, and descriptor-declared structural choices in one shared option-modal model. Engine choices SHALL come from the installed instrument registry; effect choices SHALL contain `EMPTY` plus the installed effect registry; route choices SHALL contain the persistent tracks T00 through T0F; and other choices SHALL come from the owning descriptor or correlated catalog. Missing implementations MUST NOT appear as selectable placeholders, and Figma example lists MUST NOT be treated as exhaustive registries.

#### Scenario: Open engine choices
- **WHEN** Edit+Up is resolved on the Patch engine choice
- **THEN** the option modal contains exactly the currently installed instrument capabilities in stable registry order and marks the active engine with explicit text or shape

#### Scenario: Open route choices
- **WHEN** Edit+Up is resolved on a Patch output-route choice
- **THEN** the option modal exposes all sixteen stable `MixerTrackId` values regardless of Patch occupancy

#### Scenario: Edit+Up targets a scalar
- **WHEN** the focused control is numeric rather than a choice
- **THEN** Edit+Up performs the descriptor-owned coarse increment and does not open a modal

### Requirement: Choice focus is trapped and returns exactly
An option modal SHALL enter reducer-owned `Modal` mode, trap focus within its ordered semantic option identities, move non-wrapping with Up and Down, choose with Edit, and cancel with Shift+Down. Commit and cancel SHALL close exactly one subordinate surface and restore the exact semantic origin while it remains valid. A choice modal MUST NOT open another modal.

#### Scenario: Choose an option
- **WHEN** the user opens a choice modal, focuses a different available option, and presses Edit
- **THEN** the reducer emits the corresponding semantic structural intent, closes the modal, and returns focus to the exact control that opened it

#### Scenario: Cancel an option
- **WHEN** the user moves modal focus and presses Shift+Down without choosing
- **THEN** canonical session state remains unchanged and focus returns to the exact origin

#### Scenario: Attempt to leave the modal spatially
- **WHEN** a D-pad gesture would otherwise cross into Utility or another Patch surface while a choice modal is open
- **THEN** focus remains trapped in the modal and no peer or nested surface is entered

### Requirement: Structural choices keep active and requested state explicit
A structural choice commit SHALL use the correlated preparation and block-boundary activation lifecycle. The originating row and any open detail projection SHALL distinguish the active value from the requested value and SHALL communicate `Preparing`, `Activating`, `Ready`, `Unavailable`, `Invalid`, `Cancelled`, or typed failure using text or shape as well as color. The active graph and active value SHALL remain unchanged until compatible prepared state activates; rejection MUST NOT silently bypass, substitute, or fall back.

#### Scenario: Choice prepares successfully
- **WHEN** a valid engine, effect, route-dependent structure, or descriptor choice is committed
- **THEN** the UI shows the requested value and correlated preparation state until the compatible graph revision activates, then shows that value as active

#### Scenario: Choice preparation fails
- **WHEN** preparation rejects a requested choice
- **THEN** the prior active value and graph remain active and the originating control exposes the typed failure without displaying the request as committed

### Requirement: Detail and modal projection follows the Crest visual reference
At the authored 1920×1080 viewport, the system SHALL preserve the Patch context line, identity header, 1500-pixel task region, persistent 420-pixel Utility region, and command footer established by the linked Crest Synth Figma reference. At the compact policy it SHALL preserve the same semantic regions, hierarchy, minimum targets, and focus identity through controlled density and proportional sizing. Detail rows, group headings, option rows, current markers, focus, edit, disabled, status, and return cues SHALL use the shared component library and SHALL remain explicit in text or shape as well as color.

#### Scenario: Reflow an open detail
- **WHEN** presentation density changes while a detail control is focused
- **THEN** the same semantic `FocusPath`, return path, and canonical values remain active while only presentation geometry changes

#### Scenario: Reflow an open modal
- **WHEN** presentation density changes while an option is focused in a modal
- **THEN** the modal remains trapped on the same semantic option and keeps its origin and current-value markers visible
