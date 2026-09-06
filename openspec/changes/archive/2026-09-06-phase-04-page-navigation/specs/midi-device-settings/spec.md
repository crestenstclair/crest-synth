## MODIFIED Requirements

### Requirement: MIDI Devices Settings remains a temporary system surface
The product SHALL present `SETTINGS / MIDI DEVICES` as a temporary configuration surface over the current PATCH or MIXER context. PATCH and MIXER MUST remain the only top-level performance contexts, and Settings MUST contain no preference unrelated to physical MIDI input.

#### Scenario: Enter from PATCH
- **WHEN** the user invokes the admitted Settings-entry action from a non-modal PATCH surface
- **THEN** MIDI Devices Settings SHALL open while retaining the exact PATCH surface, semantic focus, subordinate subject, and return identity as its suspended origin

#### Scenario: Enter from MIXER
- **WHEN** the user invokes the admitted Settings-entry action from a non-modal MIXER surface
- **THEN** MIDI Devices Settings SHALL open while retaining the exact MIXER track/control focus as its suspended origin

#### Scenario: Return from Settings
- **WHEN** the user invokes Shift+Right (or its Shift+D keyboard equivalent) from Settings through the projected semantic page action
- **THEN** the reducer SHALL close Settings and restore the exact suspended PATCH or MIXER semantic origin if it remains valid, or its existing deterministic nearest-enabled repair if it does not

#### Scenario: No third context
- **WHEN** Settings is open, closed, serialized for projection, or reflowed
- **THEN** the top-level performance-context domain SHALL still contain exactly PATCH and MIXER and context switching SHALL remember the same PATCH and MIXER roots as before


### Requirement: Settings entry has one explicit controller decision
Figma Page Layout frame `153:184` SHALL govern primary page entry: Shift+Left from Patch Overview SHALL request temporary MIDI Settings through one host-neutral semantic page action. Existing Shift+Start entry from admitted non-modal PATCH or MIXER surfaces MAY remain as a compatibility shortcut, using the same Settings suspension behavior and requiring that no momentary preview is held. The return SHALL use Shift+Right as authored at `155:208` and `157:200`, restoring the suspended page and focus or existing repair described at `155:205`. The entry action MUST be omitted or rejected while a modal choice, adjustment, or held preview makes entry unsafe.

#### Scenario: Controller entry
- **WHEN** Shift+Left is pressed from an admitted Patch Overview origin
- **THEN** the input adapter SHALL emit one non-repeating semantic Settings-entry action and the Settings surface SHALL open through the reducer

#### Scenario: Unsafe entry is unavailable
- **WHEN** a modal choice, adjustment session, or held preview is active
- **THEN** Open MIDI Settings SHALL not appear in valid actions and neither the page-entry chord nor a retained compatibility shortcut SHALL partially enter Settings or discard the active interaction


### Requirement: Settings is controller-first and accessible
The device list SHALL own exactly one stable semantic focus. Unmodified D-pad/arrows SHALL move through focusable device identities, Edit SHALL perform only the focused row's projected Connect or Disconnect action, and Shift+Right SHALL restore the suspended performance identity. The footer SHALL advertise that authored return and MAY retain Shift+Down as a compatibility escape. The footer SHALL be the sole visible valid-action guide. Every interactive target MUST retain at least 48 px on its active axes, and every important focus/status/activity condition MUST be understandable without color.

#### Scenario: Focus survives refresh
- **WHEN** discovery, connection lifecycle, observation, reprojection, or density changes while a device row is focused
- **THEN** the same opaque device identity SHALL remain the sole focus even if its row moves or becomes Unavailable

#### Scenario: Focused row is removed permanently
- **WHEN** a focused unselected device disappears and cannot remain as the selected unavailable row
- **THEN** focus SHALL repair deterministically to the next row, then previous row, or a stable empty-list target and SHALL expose the repair

#### Scenario: Non-color comprehension
- **WHEN** the Settings surface is viewed without semantic colors
- **THEN** row markers, status words, focus keylines/shapes, breadcrumbs, and inspector text SHALL still distinguish Available, Connecting, Connected, Unavailable, Disconnected, Failed, focus, and activity
