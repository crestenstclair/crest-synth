## ADDED Requirements

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
