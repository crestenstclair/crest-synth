## Purpose

Defines readable, responsive operation of the canonical sixteen-track Mixer and its correlated Inspector, using Figma as a hierarchy and interaction guide.

## ADDED Requirements

### Requirement: Mixer visual content remains canonical
The Mixer SHALL paint the canonical fixed bank of sixteen tracks, the selected `(MixerTrackId, MixerTrackParameter)` focus, correlated Inspector content, routing, sends, returns, global controls, and compatible observations supplied by the semantic projection. The renderer MUST NOT infer track identity or selection from a column index, scroll position, label, meter value, viewport, or Figma fixture count.

#### Scenario: Complete production track bank
- **WHEN** Mixer Main is projected
- **THEN** tracks T00 through T0F SHALL remain present in stable order regardless of Patch routing or viewport width, and exactly one projected Mixer control SHALL own semantic focus

#### Scenario: Installed return content differs from the fixture
- **WHEN** installed return capabilities, labels, parameters, occupancy, or routing differ from the Figma examples
- **THEN** the Mixer and Inspector SHALL preserve the authored composition while painting the exact registry-driven projected content without a capability-name branch or silent substitution

### Requirement: Mixer Main follows the authored track-bank hierarchy
Mixer Main SHALL reproduce the hierarchy and interaction emphasis of live Figma node `42:3`. Each track column SHALL visibly distinguish track identity, level fader and readout, pan reading, mute and solo state, passive compatible meter state, and the exact focused parameter while retaining the canonical navigation order.

#### Scenario: Resting sixteen-track composition
- **WHEN** all sixteen tracks render in the acknowledged Mixer state
- **THEN** the native track bank SHALL preserve authored column rhythm and hierarchy, stable T00 through T0F order, readable values and state labels, and one structurally emphasized focused parameter

#### Scenario: Focus and track correlation are independent
- **WHEN** focus moves between Level, Pan, Mute, Solo, or into the Inspector
- **THEN** the focused control SHALL have a text-and-shape focus treatment and the owning or correlated track SHALL remain identifiable without relying on accent color alone

#### Scenario: Mute and solo remain explicit
- **WHEN** a track is muted, soloed, both affected by the global solo gate, or resting
- **THEN** the applicable MUTE, SOLO, gated, or resting reading and structural treatment SHALL be visible in addition to semantic color

### Requirement: Mixer Inspector follows the authored correlated hierarchy
The persistent Inspector SHALL reproduce the selected-track and selected-control hierarchy of live Figma node `42:3`: correlation identity, focused value and range, compatible meter reading, mute and solo status, routed Patch identities, ordered sends, ordered return occupancy and parameters, and global controls supplied by the projection.

#### Scenario: Inspector follows track and parameter focus
- **WHEN** canonical focus changes track or parameter in Mixer Main
- **THEN** the next native paint SHALL correlate the Inspector to that exact track and control without changing product state, losing scroll reachability, or presenting data from the prior focus

#### Scenario: Inspector owns focus
- **WHEN** navigation enters any projected send, return, or global Inspector control
- **THEN** exactly that semantic target SHALL own focus while the originating track remains visibly correlated and Left returns according to the canonical focus contract

#### Scenario: Empty routing and return states
- **WHEN** a selected track has no routed Patches or a bus return is empty, unavailable, loading, or failed
- **THEN** the Inspector SHALL retain the authored region and show explicit empty or lifecycle text and shape without fabricating routing, an effect, or a value

### Requirement: Meter presentation is observation-only and truthful
Track and Inspector meters SHALL paint only compatible decimated observations matching the projected parameter generation and graph revision. Meter painting, animation, resize, and stale handling MUST NOT mutate `AppState`, semantic focus, Mixer values, routing, or serialized session content.

#### Scenario: Compatible meter observation
- **WHEN** a meter observation matches the painted generation and graph revision
- **THEN** the corresponding track and Inspector readings SHALL update together while retaining the same semantic projection and focus identity

#### Scenario: Missing or stale observation
- **WHEN** no compatible meter observation is available
- **THEN** meter regions SHALL show an explicit zero or stale state through text or shape and SHALL NOT retain a visually active reading that implies current signal

### Requirement: Mixer composition is fluid and presentation-only
Mixer Main and Inspector SHALL use one semantic DOM across varied constrained, intermediate, and expanded native widths and representative height and text-scale conditions. Flexible tracks, intrinsic sizing, wrapping, clamping, and bounded scrolling SHALL adapt the track bank and Inspector from their content rather than fixed frame coordinates or viewport-specific markup. Track-bank and Inspector scrolling SHALL be bounded presentation state only.

#### Scenario: Fluid resize without input
- **WHEN** one serialized Mixer document is resized repeatedly through varied widths without physical or semantic input
- **THEN** track order, focused path, Inspector correlation, scroll targets, meter compatibility key, valid actions, and product generation SHALL remain unchanged and no semantic action SHALL be dispatched

#### Scenario: All tracks remain reachable
- **WHEN** sixteen track columns cannot fit at their authored minimum width
- **THEN** the bounded track bank SHALL expose reachable T00 and T0F endpoints without shrinking interactive targets below the canonical minimum target size, overlapping the Inspector, or creating document-level horizontal overflow

#### Scenario: Compact and enlarged Inspector content
- **WHEN** Compact composition or enlarged text renders maximum return, send, routing, and long-label fixtures
- **THEN** Mixer Main SHALL precede Inspector, both regions and their first and last semantic controls SHALL remain reachable, and required status and values SHALL wrap or scroll without overlap or unintended clipping

### Requirement: Mixer functionality is accepted through native review
Mixer completion SHALL be established through the production reducer, projector, serialization channel, committed renderer, compatible observation path, and native operation. Figma node `42:3` and responsive contract `98:2` SHALL guide hierarchy and interaction; exact typography, spacing, and pixel correspondence SHALL NOT be completion gates. Object construction or DOM presence alone MUST NOT establish workflow acceptance.

#### Scenario: Representative native comparison
- **WHEN** representative resting, focused, adjusting, muted, soloed, routed, empty, active-meter, and stale-meter Mixer states are captured natively
- **THEN** native review SHALL verify readable track and Inspector identity, usable faders and meters, values, explicit state, focus and correlation, and action guidance, using Figma as a hierarchy guide

#### Scenario: Responsive native evidence
- **WHEN** Mixer is resized fluidly through varied widths and exercised with representative constrained, intermediate, expanded, and enlarged-text conditions
- **THEN** evidence SHALL record native scale, observed responsive compositions, all sixteen identities, singular focus, Inspector correlation, minimum-target compliance, overflow and overlap, scroll endpoints, observation revision, deterministic paint acknowledgement, and unchanged generation

#### Scenario: Completion claim is bounded by evidence
- **WHEN** a Mixer workflow has an unresolved functional defect or lacks production-path evidence
- **THEN** that workflow SHALL remain reported as incomplete rather than inferring functionality from visual appearance or fixture presence
