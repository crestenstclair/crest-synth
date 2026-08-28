## ADDED Requirements

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
