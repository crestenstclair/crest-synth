## Purpose

Define the controller-oriented page graph connecting Patch Overview, highlighted Detail, Mixer, and temporary MIDI Settings while preserving semantic identity and keeping page movement separate from focus movement and editing.

## ADDED Requirements

### Requirement: Patch Overview is the default performance hub
Normal application startup SHALL present Patch Overview in the PATCH context with one valid highlighted subject. PATCH and MIXER SHALL remain the only top-level contexts. Detail SHALL be subordinate to its highlighted Instrument or effect-slot subject, and Settings · MIDI Devices SHALL be a temporary system surface rather than a third context.

#### Scenario: Normal startup
- **WHEN** the normal application opens its initialized session
- **THEN** Patch Overview SHALL be the active main page with exactly one semantic focus and no automatically opened Detail or Settings surface

#### Scenario: Page hierarchy survives projection
- **WHEN** any page transition is accepted and projected
- **THEN** its page, context, subject, focus, and return identity SHALL describe the same canonical state without UI-owned page selection

### Requirement: Shift directions traverse the authored page connections
Admitted Shift-direction inputs SHALL implement the connections in Figma Page Layout frame `153:184`. Shift+Up SHALL open highlighted Detail from Patch Overview and restore Patch Overview from Mixer. Shift+Down SHALL return from Detail to its Overview origin and move from Overview to Mixer. Shift+Left SHALL open Settings · MIDI Devices from Overview. Shift+Right from Settings SHALL restore its suspended performance identity, as authored in nodes `155:205`, `155:208`, and `157:200`. One input SHALL traverse only its applicable connection, without also moving a row or editing a value.

#### Scenario: Overview to Instrument Detail
- **WHEN** Shift+Up is activated with the Instrument highlighted in Patch Overview
- **THEN** Detail SHALL open for that Patch's active Instrument while preserving the exact Overview origin

#### Scenario: Overview to effect Detail
- **WHEN** Shift+Up is activated with an occupied effect slot highlighted in Patch Overview
- **THEN** Detail SHALL open for the effect at that exact Patch and slot, including when another slot contains the same effect capability

#### Scenario: Empty effect slot
- **WHEN** Shift+Up is activated on an empty effect slot
- **THEN** Detail entry SHALL be unavailable or rejected unchanged, with no fabricated effect subject or substitution

#### Scenario: Detail to Overview
- **WHEN** Shift+Down is activated from Instrument or effect Detail
- **THEN** the subordinate Detail SHALL close to its exact valid Overview origin or the existing documented deterministic repair, without continuing to Mixer in the same activation

#### Scenario: Overview to Mixer
- **WHEN** Shift+Down is activated from Patch Overview
- **THEN** MIXER SHALL become active at its remembered semantic root, preserving the PATCH root for return

#### Scenario: Mixer to Overview
- **WHEN** Shift+Up is activated from Mixer
- **THEN** PATCH SHALL become active at its remembered Overview root without opening Detail in the same activation

#### Scenario: Overview to Settings
- **WHEN** Shift+Left is activated from an admitted Patch Overview origin
- **THEN** Settings · MIDI Devices SHALL open with that exact performance identity suspended and the selected Patch unchanged

### Requirement: Patch cycling preserves canonical order and Overview identity
Q and E SHALL request previous and next Patch respectively through the canonical Patch-navigation action. From Patch Overview, an accepted request SHALL land in Patch Overview using the existing created-Patch order followed by its one trailing empty position. Navigation SHALL remain non-wrapping and preserve stable identities rather than deriving them from labels or collection indices.

#### Scenario: Previous and next created Patch
- **WHEN** Q or E selects an adjacent created Patch from Overview
- **THEN** Overview SHALL identify that exact Patch and retain the corresponding valid highlighted control or the established deterministic repair

#### Scenario: Sparse Patch identities
- **WHEN** adjacent created Patches have nonconsecutive identities
- **THEN** Q/E SHALL follow canonical order without synthesizing, renumbering, or reusing a Patch identity

#### Scenario: Trailing empty position
- **WHEN** E is accepted from the final created Patch or Q is accepted from the trailing empty position
- **THEN** navigation SHALL enter or leave the existing empty Overview endpoint without creating a Patch or changing saved content

#### Scenario: Navigation boundary
- **WHEN** Q is requested on the first created Patch or E on the trailing empty position
- **THEN** the request SHALL be an unchanged boundary rejection with no wrap or extra empty position

### Requirement: Page transitions preserve exact semantic roots and repair explicitly
PATCH and MIXER SHALL remember their prior semantic roots across context movement. Detail return SHALL use the exact Patch, subject position, and Overview control where valid. If that identity becomes invalid through an independently accepted schema or session change, the existing deterministic nearest-enabled repair SHALL be applied and exposed; coordinates, DOM order, display names, and unconditional first-row resets MUST NOT choose the destination.

#### Scenario: Repeated context round trip
- **WHEN** the user highlights a noninitial Overview slot and a noninitial Mixer track/control, then repeatedly traverses Overview to Mixer and back
- **THEN** each context SHALL recover its own remembered semantic root with exactly one focused target

#### Scenario: Origin becomes unavailable
- **WHEN** a Detail origin becomes unavailable while Detail is open and return is requested
- **THEN** the established next-before-previous stable sibling repair SHALL select the nearest valid origin and the projected state SHALL disclose the repair

#### Scenario: Temporary Settings suspension
- **WHEN** Settings opens and its device list changes focus or receives asynchronous updates
- **THEN** the suspended performance identity and remembered PATCH/MIXER roots SHALL remain intact except for existing canonical repairs caused by independent product changes

### Requirement: Settings return follows the authored Shift Right contract
Shift+Right from Settings SHALL restore the exact suspended PATCH or MIXER page and semantic focus when valid, or the existing deterministic nearest-enabled repair when invalid. Page Layout `153:184`, specifically `155:205`, `155:208`, and `157:200`, SHALL govern this return. The user selected this binding on 2026-09-06; it is not inferred from an older Settings frame or the existing Shift+Down escape. Shift+Down MAY remain as compatibility behavior, but acceptance SHALL exercise Shift+Right.

#### Scenario: Settings return does not select the next Patch
- **WHEN** Shift+Right is activated while Settings is open
- **THEN** the reducer SHALL restore the suspended performance identity without stepping to the next Patch, opening Detail, or crossing a second page edge

#### Scenario: Return after independent origin change
- **WHEN** an independently accepted product change invalidates the suspended focus before Shift+Right return
- **THEN** the reducer SHALL restore the canonical nearest-enabled identity and expose its existing repair without changing device discovery or connection ownership

### Requirement: Physical page input remains distinct from focus and editing
Physical input SHALL normalize to semantic page intent before product logic and pass through the canonical reducer before view or audio projection. Physical arrow keys and the existing WASD directional equivalents SHALL reach the same authored Shift-direction meanings. Unmodified focus movement, Edit interactions, modal choice/asset workflows, and held-preview cleanup SHALL retain their existing meanings. Non-conflicting existing shortcuts MAY remain as compatibility bindings but MUST NOT substitute for mapped-input acceptance or establish a missing Figma connection.

#### Scenario: Page chord does not edit or move a row
- **WHEN** an authored page chord is accepted
- **THEN** only the page transition and its required focus restoration SHALL occur, with no additional in-page navigation, parameter edit, or implicit Patch creation

#### Scenario: Input admission and modifier cleanup
- **WHEN** an input is unavailable in the current mode, its modifier is repeated or released, or the window loses focus
- **THEN** input handling SHALL preserve canonical admission and hold cleanup without leaking a page chord into a value edit or dispatching duplicate transitions for one physical activation

#### Scenario: Existing subordinate workflow
- **WHEN** the user opens or closes an existing choice or Sample/SoundFont file page using its admitted controls
- **THEN** its origin, modal trapping, confirm/cancel behavior, and preview cleanup SHALL remain unchanged by page-navigation work

#### Scenario: Navigation is session and audio neutral
- **WHEN** the user traverses the authored page graph without a separate accepted content or device edit
- **THEN** saved content, dirty status, active Patch assignments, routing, active graph identity, and ongoing audio SHALL remain unchanged; navigation SHALL not request a structural graph or perform callback-side work

### Requirement: Acceptance covers the authored page graph through production input and rendering
Acceptance SHALL include deterministic coverage of every mapped edge and a bounded native journey using actual window key/modifier capture, semantic actions, the canonical reducer, immutable projection, and visible paint acknowledgement. Evidence SHALL identify source and destination pages, Patch/subject/focus identity, return or repair, and unchanged session/audio ownership. Settings-return acceptance SHALL use the authored Shift+Right binding. Readability and correct behavior SHALL determine visual acceptance; exact pixel matching and repetition of closed resize acceptance SHALL not be required.

#### Scenario: Deterministic edge coverage
- **WHEN** navigation acceptance runs
- **THEN** it SHALL cover all mapped connections, occupied and empty effect origins, duplicate effect capabilities, sparse Patch identities, navigation endpoints, root round trips, Settings suspension, and explicit repair without a parallel navigation implementation

#### Scenario: Native page journey
- **WHEN** the native witness drives Q/E and the authored Shift chords through production key capture
- **THEN** the displayed page, singular focus, highlighted subject, return destination, and projected input guidance SHALL match the reducer's accepted state, and the witness SHALL close its owned window

#### Scenario: Completion requires the full contract
- **WHEN** Phase 04 is marked complete
- **THEN** every authored page connection, including Shift+Right from Settings, SHALL have passing deterministic and native evidence, with no unresolved binding or failing native edge presented as accepted
