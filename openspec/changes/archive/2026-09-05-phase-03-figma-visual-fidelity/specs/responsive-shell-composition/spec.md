## ADDED Requirements

### Requirement: Surfaces use a consistent readable visual vocabulary
Sample Detail, Sample Browser, Mixer, Inspector, Engine Options, Post FX Options, and shared shell chrome SHALL resolve color roles, type styles, spacing, radii, keylines, focus and adjustment treatments, target floors, and bounded region geometry from the shared authored visual vocabulary. Figma SHALL guide hierarchy and interaction without requiring exact typography, spacing, or pixel correspondence. Native display or text scaling MUST NOT change semantic identity or product state.

#### Scenario: Authored scale at native reference conditions
- **WHEN** each Phase 03 surface is painted under representative native conditions
- **THEN** its identity, controls, values, state, focus, and action guidance SHALL remain readable and consistently grouped using the shared visual vocabulary

#### Scenario: Enlarged text and native display scale
- **WHEN** the native host applies enlarged text or a supported display scale
- **THEN** required hierarchy, values, units, status, focus, and action guidance SHALL remain legible and reachable with structural state treatments intact and no resize- or scale-derived semantic action

#### Scenario: State is not communicated by color alone
- **WHEN** focus, adjustment, current, selected, mute, solo, disabled, empty, loading, unavailable, stale, or failed state is painted on a Phase 03 surface
- **THEN** explicit text or a distinct structural shape SHALL accompany the semantic color role

### Requirement: Responsive acceptance exercises fluid native resizing
Changes to layout behavior SHALL test the production native window fluidly through varied constrained, intermediate, and expanded widths, including widths not represented by a named Figma frame. Layout SHALL be driven by intrinsic content, flexible tracks, wrapping, clamping, and content-driven reflow rather than fixed canvases, exhaustive per-pixel cases, coordinate tables, or viewport-specific markup. Passing named snapshots alone MUST NOT satisfy this requirement.

#### Scenario: Live resize preserves semantics
- **WHEN** one unchanged serialized projection per Phase 03 surface is resized repeatedly through varied widths and across every reflow observed during the run
- **THEN** every sampled observation SHALL retain the same generation, semantic focus, surface set, subject, return identity, projected content, valid actions, and compatible observation keys, and the semantic event record SHALL contain no resize-derived action or reducer application

#### Scenario: Unauthored widths preserve layout constraints
- **WHEN** live resize exercises widths between, narrower than, and wider than the named reference frames
- **THEN** every observed composition SHALL retain the canonical minimum target token, reachable first and last required content, bounded region scrolling, zero required-content overlap, and zero document-level horizontal overflow

#### Scenario: Reference frames are not static page contracts
- **WHEN** a native composition is compared with a named Figma frame
- **THEN** acceptance SHALL evaluate hierarchy, relative emphasis, spacing rhythm, type roles, state clarity, and responsive behavior rather than exact resolution, absolute coordinates, or pixel-difference equality

### Requirement: Shared presentation changes preserve accepted functionality
Shared shell changes SHALL preserve accepted workflows and use focused production-path regression evidence for the affected surfaces. Previously accepted resize behavior SHALL NOT require a repeated manual handoff unless a new change or observed regression affects it. Cosmetic comparison alone SHALL NOT block functional completion.

#### Scenario: Cross-surface token refinement
- **WHEN** a shared type, spacing, color, keyline, focus, shell-band, footer, Utility, or Inspector token changes during cohesive polish
- **THEN** the affected Sample, Mixer, option, Patch Overview, or Instrument/FX Detail workflows SHALL retain readable content, stable focus, and reachable controls under the conditions changed

#### Scenario: Completion record is evidence-bounded
- **WHEN** a scoped implementation is reported complete
- **THEN** the as-built record SHALL identify the accepted scope, relevant production-path checks, and remaining functional limitations without citing OpenSpec artifacts as proof or claiming unmeasured surfaces or platforms
