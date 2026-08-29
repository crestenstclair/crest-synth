## ADDED Requirements

### Requirement: Sibling Patch navigation includes the trailing empty position
Patch Overview sibling navigation SHALL use the reducer-owned ordered sequence of every created Patch followed by exactly one trailing empty position. The sequence SHALL remain non-wrapping, and entering the empty position SHALL use the same semantic Patch-navigation path as switching between created Patches.

#### Scenario: Right from final created Patch
- **WHEN** next-Patch navigation is accepted from the final created Patch Overview, Detail, Choice, Sample Browser, or Utility surface
- **THEN** any Patch-owned subordinate session SHALL close as required and focus SHALL land on the corresponding valid empty Overview control through the canonical navigation transition

#### Scenario: Left from trailing empty position
- **WHEN** previous-Patch navigation is accepted from the empty position
- **THEN** focus SHALL land on the final created Patch's exact corresponding control when it resolves or on the deterministic nearest enabled semantic sibling

#### Scenario: Navigation endpoints do not wrap
- **WHEN** previous-Patch navigation is requested from the first created Patch or next-Patch navigation is requested from the trailing empty position
- **THEN** the request SHALL be an unchanged boundary rejection and SHALL NOT wrap, create content, or add another empty position

### Requirement: Patch Overview renders empty state truthfully
The Patch Overview projection SHALL distinguish a created Patch from the trailing empty position as a tagged semantic state. Empty-state headers, controls, status, requested values, and valid actions SHALL come from the immutable projection; the renderer MUST NOT manufacture a Patch identity, infer creation state from collection length, or treat prospective values as acknowledged content.

#### Scenario: Empty semantic document
- **WHEN** the trailing empty position is projected
- **THEN** the serialized semantic document SHALL identify an empty new-Patch position, one focused control, prospective defaults, structural lifecycle and failure facts, capacity availability, and only currently admitted actions

#### Scenario: Empty DOM correspondence
- **WHEN** the committed webview renders the empty semantic document
- **THEN** its header, Engine/Post FX anatomy, Utility ownership, focus marker, new/default labels, lifecycle, capacity or error text, and footer actions SHALL correspond exactly to that projection

#### Scenario: Created Patch does not retain empty treatment
- **WHEN** matching creation activation is acknowledged and the empty position becomes a created Patch
- **THEN** the next projected and painted Overview SHALL show the stable created identity and active values while removing the empty/new/default treatment

### Requirement: Empty Patch Overview production evidence is responsive and semantic
Patch Overview acceptance SHALL include the trailing empty position and its transition to a created Patch in the native render matrix without treating DOM presence as workflow or Figma parity.

#### Scenario: Native empty-state matrix
- **WHEN** the empty, pending, failed, capacity-reached, and newly created projections are painted at Wide, Standard, Intermediate, Compact, and enlarged-text conditions
- **THEN** evidence SHALL measure one focus, explicit non-color state treatment, 48 px target floors, complete scroll reachability, no required-content overlap or document horizontal overflow, and resize-neutral semantic identity

#### Scenario: Normalized input journey
- **WHEN** the production input normalization drives Shift+Right to empty, navigation and surface inspection without creation, a representative first edit, failure/retry, and Shift+Left return
- **THEN** every painted state SHALL correspond to the reducer/projection sequence and the journey SHALL record the explicit capacity boundary separately from Figma workflow parity

