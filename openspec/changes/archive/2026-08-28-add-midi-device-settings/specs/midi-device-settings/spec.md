## Purpose

Provide a temporary controller-first Settings surface that safely discovers, selects, connects, observes, remembers, and retires exactly one physical MIDI input without creating a new performance context or bypassing Crest Synth's canonical reducer and real-time boundaries.

## ADDED Requirements

### Requirement: MIDI Devices Settings remains a temporary system surface
The product SHALL present `SETTINGS / MIDI DEVICES` as a temporary configuration surface over the current PATCH or MIXER context. PATCH and MIXER MUST remain the only top-level performance contexts, and Settings MUST contain no preference unrelated to physical MIDI input.

#### Scenario: Enter from PATCH
- **WHEN** the user invokes the admitted Settings-entry action from a non-modal PATCH surface
- **THEN** MIDI Devices Settings SHALL open while retaining the exact PATCH surface, semantic focus, subordinate subject, and return identity as its suspended origin

#### Scenario: Enter from MIXER
- **WHEN** the user invokes the admitted Settings-entry action from a non-modal MIXER surface
- **THEN** MIDI Devices Settings SHALL open while retaining the exact MIXER track/control focus as its suspended origin

#### Scenario: Return from Settings
- **WHEN** the user invokes Shift+Down or the equivalent projected Return action from Settings
- **THEN** the reducer SHALL close Settings and restore the exact suspended PATCH or MIXER semantic origin if it remains valid, or its existing deterministic nearest-enabled repair if it does not

#### Scenario: No third context
- **WHEN** Settings is open, closed, serialized for projection, or reflowed
- **THEN** the top-level performance-context domain SHALL still contain exactly PATCH and MIXER and context switching SHALL remember the same PATCH and MIXER roots as before

### Requirement: Settings entry has one explicit controller decision
Because Figma node `116:2` defines Shift+Down return but no entry gesture, the first version SHALL map Shift+Start to one host-neutral Open MIDI Settings action from any non-modal PATCH or MIXER surface where no momentary preview is held. The action MUST be omitted or rejected while a modal choice, adjustment, or held preview makes entry unsafe.

#### Scenario: Controller entry
- **WHEN** Shift+Start is pressed from an admitted PATCH or MIXER origin
- **THEN** the input adapter SHALL emit one non-repeating Open MIDI Settings semantic action and the Settings surface SHALL open through the reducer

#### Scenario: Unsafe entry is unavailable
- **WHEN** a modal choice, adjustment session, or held preview is active
- **THEN** Open MIDI Settings SHALL not appear in valid actions and Shift+Start SHALL not partially enter Settings or discard the active interaction

### Requirement: Discovery is automatic, truthful, and stably ordered
The system SHALL scan once at startup, immediately when Settings opens, and no less often than once per second while the application is running, with at most one scan in flight. It SHALL expose the last successful scan time and Watching, Scanning, or typed Failed scan state. Repeated scans MUST normalize unchanged device membership into a stable row order independent of backend enumeration order.

#### Scenario: Initial discovery
- **WHEN** the application starts with physical MIDI inputs available
- **THEN** every enumerated input SHALL appear once with an opaque stable identity, human-readable display name, truthful availability, and deterministic order

#### Scenario: Reordered backend results
- **WHEN** two scans return the same identities in different backend order
- **THEN** the projected row order and focused semantic identity SHALL remain unchanged

#### Scenario: New device arrives
- **WHEN** a scan discovers a previously unseen identity
- **THEN** the device SHALL be added deterministically without changing focus from an existing row

#### Scenario: Scan fails
- **WHEN** enumeration returns a typed failure
- **THEN** the last successful registry SHALL remain visible as stale, the scan state SHALL say Failed with an actionable category, and no device SHALL be silently removed, selected, or connected because of that failure

### Requirement: Device identity is opaque and positive
Every input SHALL have one canonical opaque identity distinct from its display name. Matching for selection, activation, disappearance, persistence, or reconnection MUST use exact identity equality; display names, port indexes, list positions, and fuzzy similarity MUST NOT establish identity.

#### Scenario: Duplicate display names
- **WHEN** two available inputs have the same display name but different opaque identities
- **THEN** they SHALL remain distinct rows and selecting one SHALL never connect the other

#### Scenario: Selected identity changes its label
- **WHEN** the same opaque identity returns with an updated display name
- **THEN** the registry SHALL update the displayed name while preserving selection, focus, and identity-based reconnection behavior

### Requirement: Exactly one selected connection is active
At most one physical MIDI input connection SHALL be active or eligible to deliver accepted events at a time. Activating a focused Available, Disconnected, or retryable Failed row SHALL request connection to that exact identity; activating the Connected row SHALL request disconnection. Selecting a different device while one is connected SHALL perform a correlated switch and MUST keep the previous connection active until the candidate is prepared for controlled activation.

#### Scenario: Connect an available device
- **WHEN** the focused row is Available and the user invokes Edit or the equivalent projected Connect action
- **THEN** that identity SHALL become selected and Connecting, and no other identity SHALL be requested

#### Scenario: Disconnect the connected device
- **WHEN** the focused row is Connected and the user invokes Edit or the equivalent projected Disconnect action
- **THEN** physical ingress SHALL be disabled, stuck-note recovery SHALL run, the handle SHALL retire off callback, and the row SHALL become Disconnected without selecting another device

#### Scenario: Switch devices
- **WHEN** a different Available row is activated while one device is Connected
- **THEN** the candidate SHALL be prepared under a new request and revision, the old device SHALL remain active until controlled switch activation, and no instant SHALL accept both revisions

#### Scenario: Candidate switch fails
- **WHEN** a switch candidate cannot connect
- **THEN** the candidate row SHALL expose Failed and the previously connected device SHALL remain the sole active input without substitution

### Requirement: Connection activation rejects stale work and stale messages
Every connection attempt SHALL carry a nonzero monotonic request identity and a distinct nonzero monotonic connection revision. Completion, activation, callback messages, loss reports, and retirement reports MUST be accepted only when they match the current request and revision. Exhaustion MUST fail explicitly rather than wrap.

#### Scenario: Superseded completion
- **WHEN** a connect result arrives for a request that has been replaced or cancelled
- **THEN** the result SHALL not change selected or active state, its ingress SHALL remain disabled, and its handle SHALL retire off callback

#### Scenario: Retired revision message
- **WHEN** a message arrives from a revision that is inactive, stale, or retired
- **THEN** it SHALL be rejected before application dispatch and SHALL produce no Patch event or audio command

#### Scenario: Matching activation
- **WHEN** the current candidate is prepared and its request and revision match reducer state
- **THEN** activation SHALL occur at one controlled control-loop boundary, exactly one revision SHALL become eligible for input, and the prior handle SHALL be queued for off-callback retirement

### Requirement: Device loss and same-identity return are safe
When the selected device disappears, the system SHALL retain its requested identity and last-known display metadata, expose Unavailable, reject its ingress, perform stuck-note recovery, and retire its handle off callback. If connection intent remains enabled, automatic reconnection MUST target only the same exact identity when it returns.

#### Scenario: Hot unplug
- **WHEN** the active identity is absent from a successful scan or the adapter reports definitive loss
- **THEN** the selected row SHALL become Unavailable, the active revision SHALL be invalidated, its events SHALL stop, and all-notes-off recovery SHALL be requested

#### Scenario: Same identity returns
- **WHEN** the unavailable selected identity appears in a later successful scan and automatic connection intent remains enabled
- **THEN** a fresh request and connection revision SHALL attempt reconnection to that exact identity

#### Scenario: Similar device appears
- **WHEN** a different identity with the same or similar display name appears while the selected identity is unavailable
- **THEN** the selected row SHALL remain Unavailable and the different device SHALL remain Available

#### Scenario: Intentional disconnect suppresses session reconnect
- **WHEN** the user explicitly disconnects the selected device
- **THEN** automatic rescans SHALL leave it Disconnected for the remainder of that process until the user explicitly connects it again

### Requirement: Startup and restart reconcile the persisted preference
On application start, a valid persisted selected identity SHALL be restored independently of live device state. If the exact identity is available, the system SHALL attempt one automatic connection; if it is absent, the system SHALL show it as Unavailable and continue watching. A process restart MAY reset runtime request and revision counters but MUST NOT restore a live handle or runtime status.

#### Scenario: Preferred device is present at restart
- **WHEN** startup discovery includes the exact persisted identity
- **THEN** a new request and revision SHALL attempt connection to that identity

#### Scenario: Preferred device is absent at restart
- **WHEN** startup discovery does not include the persisted identity
- **THEN** the preference SHALL remain visible as Unavailable and no other device SHALL be selected or connected

### Requirement: Physical MIDI follows the canonical application and audio path
Every accepted physical channel message SHALL travel from the platform callback through parsing and normalization, a dedicated bounded physical-MIDI transport, a bounded control-loop drain, channel-based recipient resolution, the canonical application reducer, and the existing ordered audio-command projection. Physical messages SHALL be attributed distinctly from automatic fixture MIDI.

#### Scenario: One subscriber
- **WHEN** one Patch subscribes to the normalized message channel
- **THEN** exactly one physical-source application event SHALL cross the reducer for that Patch before exactly one ordered Patch MIDI command is published

#### Scenario: Multiple subscribers
- **WHEN** multiple Patches subscribe to the same channel
- **THEN** one physical message SHALL fan out to every matching Patch in stable Patch installation order and each recipient SHALL independently cross the reducer before its command is published

#### Scenario: No subscribers
- **WHEN** no Patch subscribes to the message channel
- **THEN** the drain SHALL complete as a measured no-op with zero targeted reducer events, zero audio commands, unchanged product generation, and one input-activity observation for the accepted physical message

#### Scenario: Event-source proof
- **WHEN** a physical message reaches one or more Patch recipients
- **THEN** every corresponding event record SHALL identify a Physical MIDI source distinct from Automatic MIDI

### Requirement: MIDI parsing and normalization is exact
The adapter SHALL accept only complete MIDI 1.0 channel messages represented by the canonical MIDI model. Note On with velocity zero MUST normalize to Note Off. Controller 120 (All Sound Off) and controller 123 (All Notes Off) MUST normalize to the canonical All Notes Off recovery message. Unsupported or malformed input MUST be classified and counted without conversion, substitution, or application dispatch.

#### Scenario: Supported channel messages
- **WHEN** the input is Note On, Note Off, Control Change, Program Change, Channel Pressure, or Pitch Bend with valid seven-bit data
- **THEN** it SHALL normalize to the corresponding canonical kind, channel, and data-byte representation

#### Scenario: Zero-velocity Note On
- **WHEN** the input is Note On with velocity zero
- **THEN** the accepted canonical message SHALL be Note Off for the same channel and key with release velocity zero

#### Scenario: All-notes-off controllers
- **WHEN** controller 120 or 123 is received
- **THEN** it SHALL normalize to canonical All Notes Off for that channel rather than remain a generic control change

#### Scenario: Polyphonic key pressure
- **WHEN** a valid polyphonic key-pressure message is received
- **THEN** it SHALL be classified as Unsupported Message because the canonical model has no equivalent and MUST NOT be converted to channel pressure

#### Scenario: System message
- **WHEN** System Exclusive, clock, transport, active-sensing, reset, or another System Common or System Realtime message is received
- **THEN** it SHALL be classified by unsupported class, counted, and omitted from application and audio dispatch

#### Scenario: Malformed channel message
- **WHEN** bytes do not form one complete valid channel message
- **THEN** a Malformed Message diagnostic SHALL be counted and no partial or guessed message SHALL be emitted

### Requirement: Physical MIDI work is bounded and overflow is recoverable
The callback-to-control transport SHALL be preallocated and fixed capacity, with one producer per active or candidate connection and one consumer owned by control. The callback MUST do bounded, nonblocking parsing, fixed-size normalization, gate/revision checks, queue push, and atomic accounting only. The control loop SHALL drain at most a declared fixed event budget per tick. Queue overflow MUST be explicit and MUST trigger safe recovery rather than silent loss.

#### Scenario: Callback queue has capacity
- **WHEN** a supported message arrives for the active revision and a queue slot is available
- **THEN** one fixed-size event containing revision, connection-relative timestamp, and canonical MIDI message SHALL be pushed without allocation, locking, blocking, logging, formatting, application-state access, UI work, or I/O

#### Scenario: Queue is full
- **WHEN** the producer cannot enqueue a supported message
- **THEN** it SHALL atomically record Transport Capacity failure, disable acceptance for that revision, and return immediately without overwriting queued events

#### Scenario: Control observes overflow
- **WHEN** control observes overflow for the active revision
- **THEN** it SHALL discard that revision's remaining ingress, request all-notes-off using reserved recovery behavior, retire the connection off callback, expose the failure, and use a fresh request/revision for any same-identity recovery attempt

#### Scenario: Drain budget is reached
- **WHEN** more accepted events are queued than one control tick may drain
- **THEN** the loop SHALL stop at the fixed budget and leave the remaining ordered events for a later tick without unbounded reducer, serialization, or render work

### Requirement: Product, runtime, and observation state remain separate
The selected identity and connection intent SHALL be canonical configuration state changed only through the reducer. Enumerated descriptors, availability, request/revision lifecycle, scan state, and typed connection status SHALL be reducer-owned runtime state. Active handles, callback gates, queue endpoints, and retirement work MUST remain adapter/control runtime objects. High-rate activity, timestamps, latest message, and diagnostic counters MUST remain ephemeral observation state and MUST NOT create a second product model.

#### Scenario: Activity burst
- **WHEN** the active device emits a burst of supported messages
- **THEN** product dispatch SHALL remain bounded by the drain budget and the UI SHALL receive only a latest-compatible decimated activity snapshot rather than an event history or one render request per callback

#### Scenario: Stale observation
- **WHEN** an observation revision does not match the active connection revision
- **THEN** the inspector SHALL show no live activity from that observation and the observation SHALL not mutate product state

#### Scenario: Idle connected device
- **WHEN** no accepted message has been drained for the declared receiving interval
- **THEN** the inspector SHALL say Connected and Waiting rather than continuing to claim Receiving

### Requirement: Selected-device persistence excludes live state
The selected-device preference SHALL be stored in a separately versioned application-preference document, not in the saved synth session. It SHALL contain only the opaque selected identity, identity-schema version, and last-known display metadata needed for migration and truthful presentation. It MUST NOT serialize an active handle, descriptor registry, availability, connection/scan lifecycle, request or revision, queue/callback state, timestamps, counters, or observations.

#### Scenario: Preference capture
- **WHEN** a selection change is accepted
- **THEN** a persistence effect SHALL write the minimal preference after reducer acceptance without serializing runtime state

#### Scenario: Session save
- **WHEN** the synth session is saved
- **THEN** its existing canonical Patch, Mixer, return, and asset document SHALL remain free of MIDI-device preference and live device state

#### Scenario: Invalid preference
- **WHEN** the preference document has an unsupported version or invalid identity encoding
- **THEN** startup SHALL expose a typed preference failure and continue with no selected device rather than guessing or modifying the synth session

### Requirement: Failures and statuses are typed and visible
The system SHALL distinguish enumeration, port-information, connection, disconnection/retirement, device-loss, malformed-message, unsupported-message, transport-capacity, persistence, stale-correlation, and identity-exhaustion failures. Available, Connecting, Connected, Unavailable, Disconnected, and Failed MUST be projected in text and structural shape as well as color where applicable. Message diagnostics MUST NOT collapse a healthy connection into Failed unless they require connection retirement.

#### Scenario: Connect fails
- **WHEN** the selected present device cannot be connected
- **THEN** its row SHALL say Failed with a typed connection category and a retry action while no active handle is claimed

#### Scenario: Unsupported input on healthy connection
- **WHEN** an unsupported message is counted but the connection remains usable
- **THEN** the row SHALL remain Connected, the inspector SHALL expose a decimated unsupported diagnostic, and no unsupported message SHALL reach the reducer

#### Scenario: Enumeration fails while connected
- **WHEN** discovery fails but the active handle has not been definitively lost
- **THEN** the row SHALL remain Connected, the registry SHALL be marked stale, and the scan failure SHALL be displayed separately

### Requirement: The connected inspector is truthful and decimated
The inspector SHALL describe only the exact selected device and active revision. It SHALL show stable descriptor facts supplied by the adapter, explicit Unknown or Not Reported values for unavailable facts, current connection state, and a decimated latest MIDI observation with count, kind, channel/data, and value visualization where applicable. It MUST NOT infer port type from the display name or treat Figma fixture values as production constants.

#### Scenario: Connected inspector
- **WHEN** the selected device is Connected
- **THEN** the inspector SHALL show its exact display name, connection state, positively known port facts, accepted MIDI format/channel capability, and correlated activity state

#### Scenario: Unknown port type
- **WHEN** the backend does not report whether a port is USB, virtual, Bluetooth, network, or another transport
- **THEN** the inspector SHALL say Unknown or Not Reported rather than guessing from the name

#### Scenario: Registry differs from Figma fixture
- **WHEN** installed devices have names, counts, identities, port facts, or messages unlike node `116:2`
- **THEN** the same list and inspector SHALL render the installed registry without truncating it to the fixture or inventing fixture content

### Requirement: Shutdown retires input without callback-side destruction
Application shutdown SHALL first disable physical ingress, invalidate the active revision, request bounded all-notes-off recovery, drain or discard physical transport according to the recovery policy, and move every active/candidate handle to an off-callback retirement owner. The hard audio callback and the MIDI callback MUST NOT destroy owned connection or graph state.

#### Scenario: Shutdown with active input
- **WHEN** the application exits while a physical device is Connected
- **THEN** no later message SHALL reach application dispatch, recovery SHALL be requested, the connection SHALL close on a non-callback thread, and shutdown evidence SHALL report no retained handle

#### Scenario: Shutdown with stale candidate
- **WHEN** the application exits while a candidate connect result is pending or prepared
- **THEN** the result SHALL be cancelled or retired off callback without activation

### Requirement: Settings is controller-first and accessible
The device list SHALL own exactly one stable semantic focus. Unmodified D-pad/arrows SHALL move through focusable device identities, Edit SHALL perform only the focused row's projected Connect or Disconnect action, and Shift+Down SHALL return. The footer SHALL be the sole visible valid-action guide. Every interactive target MUST retain at least 48 px on its active axes, and every important focus/status/activity condition MUST be understandable without color.

#### Scenario: Focus survives refresh
- **WHEN** discovery, connection lifecycle, observation, reprojection, or density changes while a device row is focused
- **THEN** the same opaque device identity SHALL remain the sole focus even if its row moves or becomes Unavailable

#### Scenario: Focused row is removed permanently
- **WHEN** a focused unselected device disappears and cannot remain as the selected unavailable row
- **THEN** focus SHALL repair deterministically to the next row, then previous row, or a stable empty-list target and SHALL expose the repair

#### Scenario: Non-color comprehension
- **WHEN** the Settings surface is viewed without semantic colors
- **THEN** row markers, status words, focus keylines/shapes, breadcrumbs, and inspector text SHALL still distinguish Available, Connecting, Connected, Unavailable, Disconnected, Failed, focus, and activity

### Requirement: Acceptance evidence is falsifiable and production-path complete
Completion SHALL be established by measured evidence through the production capability seam, parser, bounded physical transport, control drain, reducer, channel fan-out, event record, audio command boundary, projector/serialization, committed renderer, native window, and off-callback retirement path as appropriate to each claim. Reducer-only, projector-only, fake-only, mock-only, hidden-DOM, or planning-artifact tests MUST NOT be sufficient proof of as-built behavior.

#### Scenario: Fake and platform enumeration seams
- **WHEN** enumeration acceptance runs
- **THEN** a deterministic fake adapter SHALL cover identity/order/failure cases and at least one real platform adapter integration seam SHALL enumerate through the production capability without claiming a physical device when none exists

#### Scenario: Raw message end-to-end proof
- **WHEN** a real adapter seam or controlled platform callback supplies raw supported bytes for a subscribed channel
- **THEN** evidence SHALL correlate raw bytes, normalization, queue revision/timestamp, bounded drain, stable fan-out, per-recipient reducer application, Physical MIDI event-source records, ordered audio commands, graph dispatch, and renderer response

#### Scenario: Fan-out matrix
- **WHEN** zero, one, and multiple Patches subscribe to a physical message channel
- **THEN** measured events and commands SHALL prove the specified no-op or stable installation-order fan-out without duplicates

#### Scenario: Lifecycle race matrix
- **WHEN** tests interleave connect, switch, disconnect, disappearance, same-identity return, stale request completion, stale revision messages, overflow, and shutdown
- **THEN** no inactive identity or revision SHALL become active or produce an accepted audio command and every handle SHALL retire off callback

#### Scenario: Callback safety instrumentation
- **WHEN** physical input, overflow, switching, loss, rendering, and shutdown run under callback instrumentation
- **THEN** the hard audio callback SHALL report zero allocation, deallocation, locking, blocking, I/O, logging, formatting, panic/unwind, and destruction, and the MIDI callback SHALL report bounded preallocated nonblocking work with no forbidden operation

#### Scenario: Settings render evidence
- **WHEN** representative registry, lifecycle, long-label, enlarged-text, empty-list, overflow/failure, and activity fixtures render through the native application
- **THEN** measurements SHALL prove exact focus, 48 px targets, status comprehension without color, reachability, no required overlap/overflow, decimated observation correlation, footer ownership, deterministic repeated paint, and visual comparison to Figma node `116:2`

