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
