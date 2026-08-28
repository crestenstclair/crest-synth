# responsive-shell-composition Specification

## Purpose
TBD - created by archiving change responsive-shell-patch-overview. Update Purpose after archive.
## Requirements
### Requirement: Fluid shell regions
The front-end SHALL compose the context line, identity header, main workspace, persistent side region, and footer with responsive Grid/Flex layout using intrinsic sizing and bounded tracks rather than a scaled fixed canvas.

#### Scenario: Wide composition
- **WHEN** the available shell width satisfies the Wide layout bounds
- **THEN** the main workspace and persistent side region SHALL render as adjacent tracks with a flexible main track and a bounded side track

#### Scenario: Intermediate width
- **WHEN** the shell width lies between representative reference viewports
- **THEN** tracks, gaps, and wrapping SHALL resolve continuously within their declared bounds without selecting fixed reference-frame coordinates

### Requirement: Presentation-only layout modes
Wide, Standard, and Compact layout modes MUST be derived by the presentation layer and MUST NOT be stored in `AppState`, dispatched as a semantic action, or alter the accepted product generation.

#### Scenario: Resize across every mode
- **WHEN** one accepted semantic projection is rendered and the viewport crosses Wide, Standard, and Compact thresholds
- **THEN** the focus path, return path, product generation, surface set, and projected controls SHALL remain unchanged

#### Scenario: Repeated reflow
- **WHEN** the viewport repeatedly resizes without physical or semantic input
- **THEN** the application event log SHALL contain no resize-derived product event and `AppState::apply` SHALL not be invoked for layout

### Requirement: Compact region preservation
Compact composition SHALL preserve the main workspace and persistent side region in semantic order and SHALL keep both reachable rather than hiding, substituting, or dropping either region.

#### Scenario: Patch Utility in Compact
- **WHEN** Patch Overview is rendered in Compact composition
- **THEN** the overview and Utility controls SHALL both remain present and reachable in the same semantic projection

#### Scenario: Existing Mixer in Compact
- **WHEN** the existing Mixer blockout is rendered in Compact composition during this slice
- **THEN** all sixteen projected tracks and the Inspector SHALL remain present and reachable even though final Mixer visual composition is deferred

### Requirement: Stable semantic focus across reflow
The shell SHALL preserve exactly one stable semantic focus identity across layout, density, wrapping, scrolling, and pane changes; geometry, DOM position, and collection index MUST NOT become focus state.

#### Scenario: Focused control moves on screen
- **WHEN** CSS reflow moves the focused control to a different row, track, or stacked region
- **THEN** the same serialized focus path SHALL remain focused and the presentation SHALL reveal that element without dispatching a semantic action

#### Scenario: Return identity survives resize
- **WHEN** a subordinate PATCH surface is open, the viewport reflows, and the user returns
- **THEN** the reducer SHALL restore the stored semantic origin according to the existing return-path rules

### Requirement: Bounded accessibility and reachability
Every interactive control SHALL retain a minimum 48px target on its active axes, and required content SHALL remain legible and reachable without overlap, unintended clipping, or inaccessible off-screen controls.

#### Scenario: Compact with long content
- **WHEN** Compact composition renders long labels and the maximum installed registry content used by acceptance fixtures
- **THEN** controls SHALL wrap or scroll within declared regions while preserving target floors and without overlapping adjacent required content

#### Scenario: Display or text scaling
- **WHEN** the native window runs with a scaled-display or enlarged-text condition
- **THEN** required status, focus, values, and action hints SHALL remain legible and reachable

### Requirement: Explicit state beyond color
Focus, adjusting, disabled, loading, failure, selected, empty, and unavailable states SHALL be communicated through text or shape in addition to semantic color roles.

#### Scenario: Non-color focus evidence
- **WHEN** a control owns focus
- **THEN** it SHALL have a structural focus treatment and a textual focus/breadcrumb reading in addition to the focus color

#### Scenario: Lifecycle failure
- **WHEN** a projected structural request fails
- **THEN** the typed failure SHALL be visible in text and a non-color state treatment SHALL distinguish the control from resting state

### Requirement: Registry-driven responsive content
Responsive composition MUST arrange the controls, rows, choices, values, and counts supplied by installed capability registries and MUST NOT treat Figma fixture names or counts as exhaustive production data.

#### Scenario: Different installed instrument schema
- **WHEN** two installed instrument descriptors expose different labels, sections, and parameter counts
- **THEN** the same responsive primitives SHALL compose each projected schema without a capability-name branch

### Requirement: Representative viewport acceptance
Acceptance SHALL exercise the production reducer, projector, serialization channel, webview renderer, and native window at Wide, Standard, Compact, intermediate, 1280×800, and scaled conditions; reference-frame coordinate identity SHALL NOT be an acceptance criterion.

#### Scenario: Production-path reflow evidence
- **WHEN** the acceptance harness renders one accepted generation at each representative condition
- **THEN** it SHALL record layout mode, region bounds, focus identity, target sizes, overflow/overlap results, and successful paint acknowledgement

#### Scenario: Deterministic rendering
- **WHEN** the same projection is rendered twice at the same viewport and observation state
- **THEN** the structural DOM observation SHALL be identical

### Requirement: Responsive option surfaces use one bounded semantic DOM
Engine Options and Post FX Options SHALL render through one semantic DOM inside the existing responsive shell. The option region and persistent Patch Utility SHALL adapt through fluid tracks, wrapping, stacking, and bounded scrolling without duplicating product controls or using viewport state as semantic authority.

#### Scenario: Wide and Standard option layout
- **WHEN** Engine or Post FX Options is rendered at Wide or Standard width
- **THEN** the option region and bounded Utility region SHALL remain adjacent, readable, and reachable while projected option order and focus identity remain unchanged

#### Scenario: Intermediate and 1280×800 option layout
- **WHEN** an option surface is rendered at an intermediate width or 1280×800
- **THEN** all headers, projected rows, state readings, footer actions, and Utility controls SHALL remain reachable with 48px target floors, no required-content overlap, and no document-level horizontal overflow

#### Scenario: Compact option layout
- **WHEN** an option surface is rendered in Compact composition
- **THEN** its option region and Utility SHALL stack in semantic document order within the same DOM and SHALL preserve exactly one projected focus

#### Scenario: Maximum content and enlarged text
- **WHEN** maximum acceptance registry content, long labels, and enlarged text are rendered
- **THEN** the option list and any other constrained region SHALL expose bounded scrolling with reachable start and end content, while required identity, focus, selection, disabled/unavailable, lifecycle, failure, and action text wraps without overlap or unintended clipping

#### Scenario: Option resize remains presentation-only
- **WHEN** an open option surface crosses Wide, Standard, Intermediate, and Compact compositions through resize alone
- **THEN** no semantic action SHALL be emitted, product generation SHALL not change, and the exact option subject, focus, return origin, option order, active/requested readings, and valid actions SHALL remain unchanged

### Requirement: Representative option-surface acceptance is measured natively
Responsive acceptance SHALL measure Engine Options and Post FX Options through the production serialization channel and native webview at Wide, Standard, Intermediate, Compact, 1280×800, and enlarged-text conditions, including long-label and maximum-registry fixtures.

#### Scenario: Native option observations
- **WHEN** each representative option fixture is painted in the native webview
- **THEN** the observation SHALL record layout mode, workspace and Utility bounds, option-row identities and bounds, singular focus identity, target sizes, overlap, document and region overflow, scroll endpoints, and paint acknowledgement

#### Scenario: Deterministic responsive option rendering
- **WHEN** the same serialized option projection is rendered twice at the same viewport and observation state
- **THEN** its structural DOM observation and native acknowledgement identity SHALL be identical

#### Scenario: Resize-only native sequence
- **WHEN** one serialized open-option projection is resized across every representative mode without input
- **THEN** every observation SHALL retain the same semantic projection and the native input/event record SHALL contain no resize-derived product action or reducer application

### Requirement: MIDI Devices Settings is continuously adaptive without changing product state
MIDI Devices Settings SHALL render through one semantic DOM and one fluid layout system using the existing responsive shell vocabulary. While both workspace regions fit, the device list and inspector SHALL share the available inline space from an approximately 80/20 starting ratio derived from Figma node `116:2`; the list SHALL absorb remaining space and the inspector SHALL be clamped by declared content minimum and maximum widths. The renderer MUST NOT select a layout from aspect ratio, named device class, or a table of pixel-perfect viewport variants. It SHALL stack the same list and inspector in semantic order only when their declared minimum usable widths no longer fit. Viewport geometry, scrolling, wrapping, density, text scaling, continuous track interpolation, and the stack transition MUST remain presentation-only.

#### Scenario: Normative wide composition
- **WHEN** Settings renders at the 1920×1080 Wide reference condition
- **THEN** its header, identity band, available-input list, bounded input inspector, and single footer guide SHALL reproduce the hierarchy and interaction emphasis of Figma node `116:2` without treating its dimensions, aspect ratio, fixture coordinates, names, counts, or values as production limits

#### Scenario: Continuous inline adaptation
- **WHEN** available inline space changes by any amount while the list and inspector minima still fit
- **THEN** the two regions SHALL resize continuously from the declared proportional relationship and clamps, with no discrete aspect-ratio mode selection, no fixed-coordinate redraw, all status, facts, focus, and actions reachable, and no document-level horizontal overflow

#### Scenario: Content-driven stack transition
- **WHEN** available inline space becomes smaller than the combined declared list minimum, inspector minimum, divider, and required gaps
- **THEN** the device list SHALL precede the inspector in document order, both SHALL remain reachable, and exactly one semantic device focus SHALL be preserved

#### Scenario: Resize remains presentation-only
- **WHEN** one Settings projection is resized continuously through arbitrary widths, including but not limited to the representative Wide, Standard, Intermediate, Compact, and enlarged-text witness points
- **THEN** selected identity, active request/revision, connection status, scan state, device order, focus, suspended return origin, valid actions, and product generation SHALL remain unchanged and no reducer action SHALL be dispatched

### Requirement: Settings responsive acceptance is measured natively
Responsive acceptance for MIDI Devices Settings SHALL use the production serialization channel and committed native webview across a continuous resize sweep plus the existing Wide, Standard, Intermediate, Compact, 1280×800, and enlarged-text witness points. Those points are samples for falsification, not distinct authored layouts or aspect-ratio contracts. Acceptance SHALL include empty, maximum acceptance registry, long-label, selected-unavailable, every visible connection state, scan failure, and activity fixtures.

#### Scenario: Native Settings observations
- **WHEN** every representative Settings fixture is painted in the native webview
- **THEN** evidence SHALL record layout mode, list/inspector/footer bounds, row identities and bounds, singular focus identity, 48 px target floors, overlap, document and region overflow, scroll endpoints, footer action ownership, observation revision, and paint acknowledgement

#### Scenario: Deterministic Settings rendering
- **WHEN** the same Settings projection and decimated observation snapshot render twice at the same viewport
- **THEN** their structural DOM observations and native acknowledgement identities SHALL be identical

#### Scenario: Direct Figma comparison
- **WHEN** final Settings visual evidence is reviewed
- **THEN** the reference-size sample SHALL be compared directly with live Figma node `116:2`, arbitrary-width samples SHALL preserve its content hierarchy and interaction priorities through the same fluid constraints, and the evidence SHALL state any remaining discrepancy rather than infer parity from structure alone
