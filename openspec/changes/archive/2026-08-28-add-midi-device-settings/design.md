## Context

See `proposal.md` for motivation and `specs/midi-device-settings/spec.md` for the behavior contract.

The current production path is deliberately narrow:

- `src/testing/midi_event_source.rs` defines a bounded caller-owned batch and a fixture-specific `MidiEventSource`; it is not a physical-device capability.
- `src/testing/automatic_midi_test.rs` polls that fixture and calls `AppLoop::dispatch_midi_from(..., EventSource::AutomaticMidi)`.
- `src/control/app_loop.rs::dispatch_midi_from` already resolves all Patches subscribed to the message channel in stable installation order. Every recipient is dispatched as `AppEvent::Midi`, crosses `AppState::apply`, projects accepted state, records the source, and publishes its ordered `AudioCommand::PatchMidi`. Zero subscribers are an unchanged no-op.
- `src/kernel/midi_message.rs` is the one normalized, fixed-size MIDI value. It represents Note On, Note Off, Control Change, Program Change, Channel Pressure, Pitch Bend, and All Notes Off; it does not represent polyphonic key pressure, System Exclusive, System Common, or System Realtime.
- `src/control/event_record.rs::EventSource` distinguishes Automatic MIDI but has no Physical MIDI variant.
- `src/shell/standalone_application.rs` composes only the automatic fixture. It already polls bounded work on the control tick, owns audio/device teardown off callback, and exposes a separate decimated audio-observation path.
- `Cargo.toml` already uses `midly = 0.5.3` and `rtrb = 0.3.4`; it has no physical MIDI dependency.
- `SavedSession` version 2 intentionally excludes device and interaction state. That invariant remains correct: the MIDI-device selection is an application preference, not synth-session content.

Figma node `116:2`, `REFERENCE SCREEN · Settings · MIDI Devices · Wide Composition`, is the normative visual and interaction reference. Its 1920×1080 frame contains the existing context header, a Settings identity band, an Available Inputs list, a bounded Input Inspector, and one footer-owned action guide. It explicitly shows automatic watching/last scan, identity-rich device rows, Connected/Available text plus shape markers, stable configuration facts, receiving activity, Edit disconnect, and Shift+Down return. The frame does not define an entry gesture, disconnected/failure compositions, or responsive variants. This design resolves entry as Shift+Start and derives missing state compositions from the existing semantic status vocabulary without treating the fixture names, three-row count, port facts, or MIDI values as product limits.

Primary API verification performed for this design:

| Dependency/API | Verified responsibility |
| --- | --- |
| [`midir` 0.11.0 `MidiInput`](https://docs.rs/midir/0.11.0/midir/struct.MidiInput.html) | `ports`, `port_name`, `find_port_by_id`, and consuming `connect`; the callback receives a connection-relative microsecond `u64` timestamp and one raw message slice. |
| [`midir` 0.11.0 `MidiInputPort`](https://docs.rs/midir/0.11.0/midir/struct.MidiInputPort.html) | `id()` returns a stable opaque string; no port index or label matching is required. |
| [`midir` platform matrix](https://github.com/Boddlnagg/midir#features) | CoreMIDI on macOS/iOS, ALSA on Linux, WinMM on Windows; JACK and WinRT are opt-in features. |
| [`midly` 0.5.3 `LiveEvent`](https://docs.rs/midly/0.5.3/midly/live/enum.LiveEvent.html) | Parses complete live MIDI byte messages and distinguishes channel, System Common/SysEx, and System Realtime classes. |
| [`midly` 0.5.3 `MidiMessage`](https://docs.rs/midly/0.5.3/midly/enum.MidiMessage.html) | Enumerates the channel-message variants and documents zero-velocity Note On as Note Off convention. |
| [`rtrb` 0.3.4](https://docs.rs/rtrb/0.3.4/rtrb/) | Preallocated fixed-capacity SPSC ring; `push`/`pop` are lock-free, wait-free, immediate, and return full/empty explicitly without overwrite. |
| [CoreMIDI change notifications](https://developer.apple.com/documentation/coremidi/midinotificationmessageid/msgobjectadded) | Object-added, object-removed, setup-changed, and property-changed notifications can accelerate a later adapter-specific rescan but are not required for correctness. |

## Goals / Non-Goals

**Goals:**

- Add one adapter-neutral physical MIDI input capability and one `midir` implementation while preserving the automatic fixture as a distinct capability and source.
- Keep selected-device configuration, dynamic runtime lifecycle, backend ownership, and high-rate observations in explicitly different state layers.
- Correlate all asynchronous connection work and every queued message so stale candidates and retired callbacks cannot become active.
- Bound callback and control work, make overflow visible, and guarantee all-notes-off recovery before physical input resumes.
- Project the complete Figma Settings hierarchy with singular semantic focus, truthful non-color status, responsive reachability, and one footer action owner.
- Produce production-path evidence rather than treating model, mock, or planning tests as completion.

**Non-Goals:**

- MIDI output devices.
- More than one simultaneously active physical input port.
- MIDI learn, controller mapping, per-device channel remapping, filtering, MPE, or MIDI 2.0/UMP.
- Polyphonic key pressure until the canonical `MidiMessage` model deliberately expands.
- System Exclusive storage/routing, MIDI clock/transport routing, or conversion of unsupported messages.
- Bluetooth pairing UI, network MIDI session management, OS driver installation, or backend configuration UI.
- General application settings or preferences unrelated to MIDI input.
- Replacing, broadening, or reusing the automatic fixture trait as the hardware boundary.
- Native hot-plug notification code in version one; portable scanning is the correctness baseline.
- Redefining the current Patch registry/composition or using Figma fixture devices as a registry.

## Decisions

### 1. Extend the existing semantic system with a temporary system session

Settings is neither a third `TopLevelContext` nor a PATCH-only subordinate. Add `SurfaceId::MidiDeviceSettings`, classify it as a system surface, and change `SurfaceId::context()` into an explicit performance-context query returning `Option<TopLevelContext>`. Existing performance surfaces return `Some`; the Settings surface returns `None`. Callers that require a performance surface must handle `None` rather than assigning Settings to PATCH or MIXER.

Extend the one canonical `FocusPath` rather than creating a parallel focus model:

- add `SemanticControlId::MidiInputDevice(MidiInputDeviceId)` plus a stable empty-list/root identity;
- add a Settings constructor carrying the origin performance context for breadcrumb/return correlation, the system surface, and exact device identity;
- update focus validation and descriptors exhaustively;
- keep exactly one `active_focus` in `InteractionState`.

Add one `MidiSettingsSession` to `InteractionState`. It owns the suspended performance interaction snapshot: active origin focus, mode, existing `ReturnPath`, and any `PatchSubordinateSession`. Opening Settings is admitted only outside Modal/Adjust and held preview state, moves the Settings focus into `active_focus`, and stores the snapshot. Returning restores it atomically or runs the existing stable sibling repair if current schema no longer admits the origin. Settings cannot stack or open recursively.

The physical binding is Shift+Start, normalized to a new host-neutral `SemanticAction::OpenMidiSettings`. It is non-repeating. Shift+Down continues to resolve to the existing Return action. This uses an otherwise unassigned chord, avoids stealing unmodified Start from Sample Browser preview, and makes entry globally available from PATCH and MIXER without introducing focusable header chrome.

Alternative considered: make Settings a third `TopLevelContext`. Rejected because Figma and `DESIGN.md` explicitly preserve PATCH and MIXER as the only performance contexts and because it would corrupt independent remembered roots.

Alternative considered: duplicate a PATCH Settings and MIXER Settings surface. Rejected because the one configuration surface would gain two semantic identities and adapters could accidentally diverge.

### 2. Use canonical adapter-neutral MIDI-device types

The public concepts are defined once in the control/kernel boundary and re-exported from one module. Suggested names are normative in responsibility; final placement may follow repository module conventions.

| Canonical type | Responsibility and invariants |
| --- | --- |
| `MidiInputDeviceId` | Nonempty bounded opaque identity string with identity schema/version namespace. Equality is the only matching rule; it has no display semantics. |
| `MidiInputDescriptor` | Identity, display name, and optional positively known `MidiInputPortFacts`; no backend port handle. |
| `MidiInputPortFacts` | Adapter-reported transport kind, MIDI byte format, and other stable facts as typed optional values; unknown remains explicit. |
| `MidiInputScanId` | Nonzero monotonic scan correlation; no wrap. |
| `MidiConnectionRequestId` | Nonzero monotonic effect/completion correlation; no wrap. |
| `MidiConnectionRevision` | Nonzero monotonic activation and message authority; distinct from request identity and never persisted. |
| `MidiInputConnectionStatus` | Reducer-owned Disconnected/Connecting/Connected/Unavailable/Failed state with requested/active identity and correlation where applicable. Internal Prepared/Activating lifecycle projects as Connecting. |
| `MidiDeviceFailure` | Canonical typed failure described in Decision 11; no backend error type escapes. |
| `ConnectMidiInput` | Exact identity, request id, and revision passed to the capability. |
| `PhysicalMidiEvent` | `Copy`, destructor-free fixed-size `{ revision, timestamp_micros, message: MidiMessage }`. |
| `PhysicalMidiIngress` | Opaque callback-side producer, disabled/enabled gate, fixed atomic diagnostic fields, and immutable revision. It exposes no product state. |
| `ActiveMidiInput` | Non-cloneable, non-serializable owned connection handle with private backend implementation; destruction only through the retirement worker. |
| `MidiActivitySnapshot` | Fixed-size latest observation correlated to revision: accepted count, last event, callback diagnostic totals, control receipt age/status, and overflow epoch. |
| `MidiInputPreference` | Versioned selected identity, identity schema version, and last-known display name only. |

Strings are bounded and validated when crossing from the adapter into descriptors/preferences; no string enters either callback event type. Backend types such as `midir::MidiInputPort` and `midir::MidiInputConnection` stay private to `adapter`.

Alternative considered: reuse display names as identity. Rejected because names can collide and change and `midir` provides an opaque port ID specifically for positive matching.

### 3. Add physical-device and preference capabilities, not a generalized fixture source

Use an object-safe capability equivalent to:

```rust
pub trait MidiInputDevicePort: Send {
    fn enumerate(&mut self) -> Result<Vec<MidiInputDescriptor>, MidiDeviceFailure>;

    fn connect(
        &mut self,
        request: ConnectMidiInput,
        ingress: PhysicalMidiIngress,
    ) -> Result<ActiveMidiInput, MidiDeviceFailure>;

    fn disconnect(
        &mut self,
        active: ActiveMidiInput,
    ) -> Result<(), MidiDeviceFailure>;
}

pub trait MidiInputPreferencePort: Send {
    fn load(&mut self) -> Result<Option<MidiInputPreference>, MidiDeviceFailure>;
    fn store(&mut self, value: &MidiInputPreference) -> Result<(), MidiDeviceFailure>;
}
```

`disconnect` consumes the handle so it cannot remain active after retirement starts. The `midir` adapter calls the owned connection's close path on the worker; midir's normal close is infallible at the public API, while worker termination/panic or adapter bookkeeping failures map to typed retirement failure. No `Drop` of the connection occurs on the MIDI callback, control thread hot path, or audio callback.

A dedicated `MidiInputDeviceWorker` owns both ports and bounded nonblocking command/result channels. It performs enumeration, connect, preference I/O, and disconnect/retirement. The control loop schedules scans and polls results; no scan or connect blocks the UI tick. There is at most one scan and one connect request in flight. A stale result still carries complete owned cleanup data so the worker can retire it.

`src/testing/midi_event_source.rs` and `AutomaticMidiTest` remain unchanged in responsibility. A physical adapter is never implemented as `MidiEventSource`, and the fixture never becomes a hardware adapter. Both call the shared `dispatch_midi_from` only after their own distinct ingress boundary.

Alternative considered: extend `MidiEventSource` with enumerate/connect methods. Rejected because it couples run-once SMF preparation and Patch fixture installation to long-lived hot-pluggable hardware state.

### 4. Normalize `midir` identity and platform behavior behind the adapter

Add `midir = "0.11.0"` to the repository's normal dependencies and retain the existing `midly` and `rtrb` versions. The adapter:

1. creates a `midir::MidiInput` off callback;
2. enumerates `ports()` and obtains `port.id()` plus `port_name()`;
3. namespaces the opaque value as identity schema `midir-v1` plus a private platform/backend tag so future backend migrations can be explicit;
4. uses exact `find_port_by_id()` or exact ID equality to locate the request;
5. sets `Ignore::None` so unsupported messages are counted rather than silently filtered;
6. passes the prebuilt `PhysicalMidiIngress` as callback data; and
7. returns an opaque `ActiveMidiInput` owning the connection.

Platform packaging strategy:

- macOS uses midir's CoreMIDI backend. Version one relies on scanning; CoreMIDI object/setup/property notifications MAY later trigger a coalesced immediate scan behind the adapter.
- Linux defaults to ALSA and documents the native `libasound` build/runtime prerequisite. A Cargo feature MAY select midir's `jack` backend for JACK-specific distributions; application types do not change.
- Windows defaults to WinMM. A build feature MAY select midir's `winrt` backend where the packaging target requires it; application types do not change.

No backend is silently substituted at runtime. A build that does not support the selected backend fails with typed adapter initialization/enumeration status.

### 5. Reconcile discovery into a stable reducer-owned registry

Control schedules a startup scan, an immediate Settings-entry scan, and a one-second periodic scan. Native notification acceleration only requests the same coalesced scan. Scan work remains off callback and at most one scan is active.

Before dispatch, the control adapter validates uniqueness and normalizes result order by case-folded display name, then opaque identity. The reducer reconciles by identity:

- unchanged identities retain their prior relative order and current focus;
- newly seen identities are appended in normalized order;
- absent unselected/unfocused identities are removed;
- the selected identity is retained as an Unavailable tombstone with last-known name;
- a focused disappearing identity is retained long enough to preserve focus as Unavailable; once focus moves and it is not selected, it may be pruned;
- the same returning identity reuses its selected/focus identity, while a different identity is always a new row.

On enumeration failure, the reducer keeps the last successful registry and marks it stale. It does not infer device loss from a failed scan. Definitive loss is either a successful scan missing the active identity or an explicit adapter loss result.

Alternative considered: sort from scratch on every scan. Rejected because label changes and backend ordering could move focus unnecessarily.

### 6. Model selection, request, active revision, and auto-connect separately

Reducer state distinguishes:

- `selected`: the established user preference/active target, persisted;
- `requested`: an optional candidate being connected, not yet selected during a switch;
- `active`: the identity and revision currently eligible to deliver messages;
- `connection_intent`: session-only Enabled/ManuallyDisconnected; and
- `status`: projected Available/Connecting/Connected/Unavailable/Disconnected/Failed.

Initial connection with no prior selection establishes and persists the requested identity when the request is accepted, so a failed/absent chosen device remains visible. During a switch, the current selected/active identity remains authoritative while the other row is Connecting. Selection and persistence move to the candidate only after controlled activation. If the candidate fails, the old device remains the selected sole active input; this is preservation of acknowledged state, not fallback.

Manual Disconnect retains the selected identity and sets `connection_intent = ManuallyDisconnected` for the rest of the process. A process restart restores only identity and defaults intent to Enabled, so it attempts the same identity once if available. Loss and overflow keep intent Enabled and may reconnect the same identity after recovery. No live intent flag is persisted.

### 7. Use request/revision-gated prepare and controlled activation

Every connect or reconnect admission allocates both a request ID and a connection revision with checked increment. They are not interchangeable: the request rejects stale worker completion; the revision rejects stale callback data. Overflow recovery always uses a new pair.

Each candidate owns its own `rtrb::Producer` and `Consumer`. This preserves SPSC when a candidate and old connection overlap. The candidate ingress gate starts disabled, so a backend that begins callbacks during `connect` cannot publish preactivation data.

Connection sequence:

```text
Activate focused row
  -> AppState::apply(request event)
  -> ConnectMidiInput effect(request, revision, exact id)
  -> worker allocates ring + disabled ingress and calls adapter.connect
  -> matching Prepared result crosses AppState::apply
  -> controlled activation boundary
       disable old gate (if any)
       publish all-notes-off recovery before switch (if old active)
       install candidate consumer/handle while gate remains disabled
       AppState::apply(activation acknowledgement)
       enable candidate gate
       queue old handle for worker retirement
  -> project Connected and persist selected preference
```

If recovery publication cannot complete immediately, both old and candidate gates remain disabled and the state remains Connecting/Activating; control retries recovery before activation. Any event that races after a gate disable is still rejected by the drain's exact active-revision check. A stale prepared handle never reaches activation and is returned to the retirement worker.

Alternative considered: share one `Producer` among old and candidate callbacks. Rejected because `rtrb` is strictly SPSC and concurrent callbacks would violate ownership.

Alternative considered: connect and immediately accept callbacks on the worker. Rejected because preactivation and stale messages could mutate the acknowledged application state.

### 8. Define disconnect, loss, switch, overflow, and shutdown sequences

All sequences use the same recovery primitive and off-callback retirement owner.

**Explicit disconnect**

1. The focused Connected row resolves Edit to a reducer event.
2. `AppState::apply` invalidates the active revision, sets intent ManuallyDisconnected, and emits recovery/retirement effects.
3. Control disables the ingress gate before any later drain, discards its queue, and publishes global all-notes-off.
4. The handle moves to the worker and the row projects Disconnected. A later retirement failure projects Failed without resurrecting the handle.

**Switch**

1. Old selected/active input remains the only accepted source while candidate connects disabled.
2. Candidate failure leaves old state unchanged and marks the requested row Failed.
3. Matching candidate activation disables old ingress, completes all-notes-off, acknowledges candidate, enables candidate, commits/persists selection, and retires old.

**Hot loss**

1. A successful scan missing the active ID or definitive adapter result crosses `AppState::apply`.
2. The reducer invalidates the revision, retains identity/name, projects Unavailable, and emits recovery/retirement effects.
3. Control disables and discards ingress, publishes all-notes-off, and retires the handle.
4. A later successful scan of the same identity creates a fresh request/revision only if intent remains Enabled.

**Physical queue or audio-command capacity failure**

1. Callback-side queue full sets fixed atomics, disables that ingress, and returns; control-side audio command rejection is detected from `MidiFanOutResult`.
2. Control stops the drain, invalidates/discards the entire revision, and crosses a typed failure event through `AppState::apply`.
3. Global all-notes-off is published through reserved recovery capacity before any physical gate can re-enable.
4. The handle retires. If intent is Enabled and exact identity is still present, a fresh request/revision attempts same-identity recovery; the Failed cause remains observable through the retry transition.

**Shutdown**

1. Stop scheduling scans/connects and reject new Settings actions.
2. Disable every active/candidate ingress gate and invalidate active revision.
3. Publish/coalesce all-notes-off, discard physical queues, and cancel worker commands.
4. Move every handle to the worker; join retirement and device worker on the control/owning thread.
5. Drop audio stream and graphs according to the existing ordering, then prove zero retained MIDI handles and callback-side destructions.

### 9. Parse and normalize in bounded callback work

The midir callback is not the hard audio callback, but it is treated as a strict ingress callback. Its callback data is fully prepared before `connect`: one producer, immutable revision, atomic gate/counters, and no references to `AppState`, window, logger, allocator-owned scratch, or device registry.

The callback algorithm is fixed:

1. If the gate is disabled, increment the fixed stale/inactive diagnostic if needed and return.
2. Read only the first status byte. Empty input is Malformed. A System Common/Realtime status is classified from that byte and counted without traversing a potentially large SysEx slice.
3. For channel status, reject lengths outside the complete one-to-three-byte live-message bound, then call `midly::live::LiveEvent::parse` on the bounded slice.
4. Normalize only canonical variants:
   - Note Off -> Note Off `(key, velocity)`;
   - Note On velocity 1..127 -> Note On;
   - Note On velocity 0 -> Note Off `(key, 0)`;
   - Controller 120 or 123 -> All Notes Off;
   - other Controller -> Control Change;
   - Program Change -> `(program, 0)`;
   - Channel Aftertouch -> Channel Pressure `(pressure, 0)`;
   - Pitch Bend -> existing LSB/MSB seven-bit representation;
   - Polyphonic Aftertouch -> Unsupported Polyphonic Pressure.
5. Push one `PhysicalMidiEvent { revision, timestamp_micros, message }`.
6. On full, set Transport Capacity counters/epoch, disable the gate, and return the rejected value to local stack scope without formatting or destruction of owned state.

`PhysicalMidiEvent` and every value dropped on either push result are `Copy` and destructor-free. System classes, malformed input, and unsupported channel variants never enter the queue and never become `AppEvent` values. Callback atomics use fixed numeric reason codes; control converts them to `MidiDeviceFailure` and text.

### 10. Use explicit queue, drain, fan-out, and recovery bounds

Version-one constants:

- physical queue capacity: 1024 `PhysicalMidiEvent` values per connection;
- control drain budget: 64 physical events per tick;
- Patch subscriber bound: existing `MAX_PATCHES = 16`, so at most 1024 targeted reducer applications can result from one full physical drain;
- scan cadence: 1 second, at most one scan in flight;
- activity publication: at most 30 Hz;
- Receiving freshness: 500 ms since last accepted control-side receipt.

The control drain owns the active `Consumer`. For each of at most 64 pops it checks active revision before doing any work. A matching event updates the fixed observation accumulator and calls:

```rust
app_loop.dispatch_midi_from(event.message(), EventSource::PhysicalMidi)
```

That existing service retains stable Patch installation order and its per-recipient `AppState::apply` path. The zero-subscriber result updates activity only and remains an unchanged application/audio no-op. A boundary-full result stops the drain and enters the overflow sequence above.

Reserve one slot in the existing audio command ring for `AudioCommand::AllNotesOff`: normal commands are rejected when only the reserved slot remains; the recovery command may consume it. Duplicate pending global recovery is coalesced because All Notes Off is idempotent. Require command capacity of at least two. If recovery is already queued/full, ingress remains disabled and control retries; it never resumes on an assumption.

Alternative considered: increase queue size and silently drop overflow. Rejected because no finite capacity prevents bursts and a lost Note Off can stick a voice.

Alternative considered: publish MIDI directly into the audio ring from midir. Rejected because channel subscription resolution and every recipient's reducer transition would be bypassed, and the command ring would acquire a second producer.

### 11. Use one typed failure model with nonfatal message diagnostics

`MidiDeviceFailure` is adapter-neutral and serializes stable categories, not backend strings/types:

- `InitializationUnavailable`
- `EnumerationFailed`
- `PortInformationFailed { identity? }`
- `IdentityUnavailable { identity }`
- `ConnectionFailed { identity, class }`
- `DisconnectionFailed { identity, revision }`
- `DeviceLost { identity, revision }`
- `MalformedMessage { class }`
- `UnsupportedMessage { class }`, where class distinguishes Polyphonic Pressure, SysEx, System Common, and System Realtime
- `TransportCapacity { stage, dropped }`, where stage is PhysicalIngress or AudioCommand
- `PreferenceReadFailed`, `PreferenceDecodeFailed`, `PreferenceVersionUnsupported`, `PreferenceWriteFailed`
- `StaleCorrelation { operation }`
- `IdentifierExhausted { kind }`

Failures own display formatting on control/projection threads. Callback-side values are fixed codes/counters only. Malformed/unsupported messages update diagnostics and the inspector but do not change Connected to Failed. Capacity, definitive loss, connect, and retirement failures do change lifecycle because safe ingress is no longer assured. Enumeration failure remains scan-scoped and does not tear down a connection without positive loss evidence.

Visible row derivation is exhaustive:

| Condition | Row state |
| --- | --- |
| present, not current/requested | Available |
| exact current request in flight or prepared | Connecting |
| exact active identity and revision | Connected |
| selected identity absent after successful scan | Unavailable |
| selected identity present, no handle, manual intent | Disconnected |
| operation for the row ended in typed failure | Failed |

All states provide status text and a marker/keyline shape in addition to color. Global scan failure is shown beside Last Scan/Watching; it does not falsify each row.

### 12. Keep configuration, runtime ownership, and observation state separate

**Reducer-owned configuration/product state**

- selected opaque identity and last-known display metadata;
- session connection intent;
- Settings open/suspended origin and active semantic focus.

**Reducer-owned runtime/status state**

- reconciled descriptor registry and stable order;
- scan correlation/last-success/staleness;
- requested and active identities, request id, revision, lifecycle, and typed failure.

These values still mutate only through `AppState::apply`. Worker results are inputs, never direct writes.

**Control/adapter runtime objects**

- `midir` clients, ports, connections;
- worker threads/channels;
- ingress gates, queue endpoints, pending candidate, active handle, and retirement ownership.

These are not serializable or projectable product state.

**Ephemeral observation state**

- accepted physical message count;
- last matching event and receipt instant;
- malformed/unsupported/stale/overflow counts and latest category;
- observation revision/generation.

The control accumulator publishes `MidiActivitySnapshot` through a latest-compatible observation boundary at no more than 30 Hz, or immediately for revision/status invalidation. Like meters, the UI polls it separately from the immutable semantic projection. The renderer accepts activity only when snapshot revision equals projected active revision. No event history, raw SysEx bytes, or callback strings are retained. Idle state is derived from the 500 ms control receipt age.

### 13. Project and render the Figma hierarchy without copying its fixture data

Add one Settings projection to the canonical semantic model. It contains:

- suspended PATCH/MIXER context indicator;
- Settings title and port summary;
- scan state and last successful scan;
- stable ordered device rows with identity, label, summary facts, status, failure, focus, and valid row action;
- exact active-device inspector with stable facts and revision;
- focus path and one footer action list;
- focus-repair status when required.

The observation snapshot is not embedded in the serialized product projection. The webview combines it only for the correlated activity card, just as a latest meter is combined with compatible generation/revision.

The workspace uses one continuously adaptive track definition rather than authored layouts per viewport or aspect ratio. While both regions fit, the device list and inspector begin from the approximately 80/20 relationship visible in node `116:2` (1520/400 at the reference sample): the list consumes the remaining fraction and the inspector is clamped by content-driven minimum and maximum widths. Header and identity contents use the same fill, hug, wrap, and alignment constraints instead of fixed coordinates. Only when the combined list minimum, inspector minimum, divider, and required gaps no longer fit does the same DOM stack list then inspector in semantic order. The 1920×1080, 1440×900, 1280×800, 900×800, and enlarged-text conditions are falsification samples along that continuous function, not distinct modes, aspect-ratio targets, or sources of layout values.

At every width, the hierarchy remains header/identity/workspace/footer, the authored row rhythm is 78 px while preserving a 48 px minimum, and the action legend is never repeated per row. Resizing or crossing the content-driven stack threshold preserves stable focus identity and dispatches no action.

The inspector is anchored to the active device, not whichever inactive row happens to be focused. When no device is active it renders an explicit Disconnected/Unavailable/Failed empty state. Port type is shown only if positively supplied; otherwise `UNKNOWN`/`NOT REPORTED`. `MIDI 1.0` describes the accepted byte format, and `CHANNELS 01–16` describes canonical input acceptance, not a guessed hardware capability.

The footer is the only visible action guide:

- D-pad/arrows: move;
- Edit: connect, disconnect, or retry only when valid for focused row;
- Shift+Down: return.

Shift+Start entry is projected on the suspended performance surface when admitted, not repeated inside Settings.

### 14. Add reducer events, effects, projection, and source attribution coherently

Suggested canonical event responsibilities:

- `OpenMidiSettings`, `MidiSettingsReturned`, navigation/activation through existing semantic actions;
- `MidiInputPreferenceRestored`;
- `MidiInputScanStarted`, `MidiInputRegistryRefreshed`, `MidiInputScanFailed`;
- `MidiInputConnectRequested`, `MidiInputConnectionPrepared`, `MidiInputActivationAcknowledged`;
- `MidiInputDisconnectRequested`, `MidiInputDisconnected`;
- `MidiInputConnectionLost`, `MidiInputOperationFailed`.

No event contains `ActiveMidiInput`, queue endpoints, gates, or backend types. Reducer outcomes emit typed effects such as Scan, Connect, Activate, Recover, Retire, and Persist. `AppLoop` is the sole effect orchestrator and records accepted/rejected lifecycle events. Worker results use `EventSource::Worker`; UI uses Keyboard/System as appropriate; actual Patch-targeted physical messages add and use `EventSource::PhysicalMidi`. Update `EventSource::ALL`, stable serialized names (`physicalMidi`), descriptors, exhaustive tests, live-report coverage, and any state-tree schema version in the same change.

For actual messages, do not add a second dispatch implementation. The physical drain calls `AppLoop::dispatch_midi_from`; each returned recipient still crosses `AppState::apply` before its command. The generation-only projection optimization remains valid. Event-log storage stays bounded; physical rates do not introduce an unbounded journal.

### 15. Persist a separate minimal preference document

Create version 1 of a dedicated document, for example:

```json
{
  "version": 1,
  "selectedInput": {
    "identitySchema": "midir-v1",
    "identity": "<opaque>",
    "lastKnownDisplayName": "<display only>"
  }
}
```

The document lives under the shell-provided per-user application configuration directory and is read/written by `MidiInputPreferencePort`. Use existing serde/JSON support and recoverable temporary-file-plus-rename writes. An absent file means no preference. Invalid/unsupported content becomes a typed visible startup preference failure and no selection; it never alters `SavedSession` or selects a similarly named device.

Persist after reducer acceptance:

- initial selection is accepted;
- a switch activation commits the new selection;
- the same identity reports an updated display name.

Do not persist manual disconnected state, availability, status, request/revision, adapter facts, or activity. Restart defaults connection intent to Enabled and reconciles the exact identity against the first successful scan. There is no migration of `SavedSession` v2 because its exclusion of device state remains correct.

### 16. Verification must prove each boundary that owns the claim

The implementation tasks must build a cumulative proof matrix:

| Claim | Minimum sufficient evidence |
| --- | --- |
| Descriptor identity/order | Fake capability returns duplicate names and reordered scans; reducer reconciliation plus projection retain exact IDs/order/focus. |
| Real platform seam | `midir` adapter initializes/enumerates on the host through the production capability. Zero ports is a valid measured result; adapter failure is typed. |
| Parser/normalizer | Raw byte table covers every supported canonical class, velocity-zero, CC120/123, malformed lengths/data, poly pressure, SysEx, System Common, and Realtime. |
| Bounded ingress | Real `rtrb` producer/consumer, capacity 1024, 64-event drain budget, stale gates/revisions, non-overwrite, and callback allocation/lock/log instrumentation. |
| Reducer/audio route | Controlled raw callback bytes correlate through queue, `dispatch_midi_from`, zero/one/multiple subscribers, per-recipient `AppState::apply`, event records, audio commands, prepared graph dispatch, and rendered audio response. |
| Race safety | Interleaved request/revision completion, candidate failure, switch, disconnect, loss, return, overflow, and shutdown demonstrate no stale accepted command. |
| Recovery | Queue and audio-command saturation both disable ingress, publish reserved All Notes Off, retire, and only then use a fresh revision. Renderer/engine observations prove voices clear. |
| Persistence | Separate document round-trip/rejection, present/absent exact identity restart, no session-device fields, and no name fallback. |
| UI/focus | Production reducer/projector/serialization/DOM/native paint across all states, registry shapes, viewport modes, long text, scaled text, and decimated observations. |
| Figma fidelity | Readable native captures compared directly with live node `116:2`; hierarchy, row/status anatomy, inspector, footer, focus, tokens, and discrepancies recorded. |
| Teardown | Live or controlled platform connection closes on worker, candidate/active ownership reaches zero, and audio/MIDI callbacks report no destruction. |

Fake-adapter tests establish determinism but are not platform proof. A real-adapter test that sees no attached devices proves only initialization/enumeration behavior, not physical message delivery. A typed native-environment skip is incomplete evidence for the physical handoff, not acceptance. Reducer, projector, or planning artifacts alone prove only their own layer.

### 17. `DESIGN.md` remains the durable as-built record

The implementation commit must update `DESIGN.md` only after the behavior exists and is verified. The update must record:

- physical MIDI capability and platform adapter installed in the production composition;
- canonical selected/runtime/observation ownership;
- queue capacity, drain, revision gating, recovery, and retirement invariants;
- separate preference document and unchanged `SavedSession` boundary;
- Settings temporary-surface and Shift+Start/Shift+Down interaction decision;
- exact production-path evidence completed and any remaining platform/visual gaps.

OpenSpec remains temporary planning material and must not be cited as as-built proof.

## Risks / Trade-offs

- [Backend IDs are only as stable as the OS/midir backend] -> Namespace identities with `midir-v1`, treat them as opaque, test reconnect on every supported backend, and fail Unavailable instead of falling back when an OS changes an ID.
- [One-second scanning can expose up to roughly one second of hot-plug latency] -> Scan immediately on Settings entry and operation completion; allow coalesced native notifications later without making them correctness-critical.
- [A large SysEx callback could tempt unbounded parsing] -> Classify System status from the first byte and return without traversing or storing the payload.
- [`midir` connection close may join or block internally] -> Always close on the retirement worker and make shutdown wait outside both callbacks.
- [A 64-event drain can fan out to 1024 reducer applications] -> Keep both constants explicit, benchmark the production loop, record tick latency/backlog, and reduce the drain budget only through an evidence-backed design update if targets are missed.
- [Overflow recovery causes an audible interruption] -> Prefer a truthful all-notes-off and fresh same-identity revision over silent Note Off loss or stuck notes; expose the failure and recovery.
- [Reserving an audio-command slot reduces normal burst capacity by one] -> Require capacity >=2 and measure existing automatic/demo workloads; safety recovery takes priority over one additional normal command.
- [Extending `SurfaceId` and focus validation touches exhaustive code] -> Land canonical types and descriptor/compiler failures first, then reducer/projector/renderer in dependency order; do not create a parallel Settings-only focus model.
- [Observation/projector correlation can drift] -> Require exact active revision match, immediately invalidate on lifecycle changes, and test stale snapshots like existing Mixer meter correlation.
- [Figma supplies only a Wide Settings frame] -> Reuse the established responsive contract for Standard/Compact, preserve hierarchy/semantics, and record responsive choices as derived behavior rather than claiming authored responsive parity.
- [New preference I/O can fail independently of a usable live connection] -> Keep the accepted in-process selection, expose typed persistence failure, and never write into or invalidate the synth session.

## Migration Plan

1. Add canonical types, exhaustive descriptors, failure vocabulary, source attribution, and compile-time `Copy`/no-drop assertions without composing a real adapter.
2. Add reducer-owned registry/lifecycle/Settings session and fake worker capability; bump serialization/state-tree versions where the public projection changes.
3. Add `midir` 0.11.0 and the platform adapter behind the capability, with a real initialization/enumeration seam and documented Linux/Windows feature prerequisites.
4. Add the per-connection `rtrb` ingress, parser/normalizer, gated lifecycle, bounded drain, reserved recovery policy, observation boundary, and off-callback retirement worker.
5. Add the separate preference document/store and startup reconciliation. No `SavedSession` migration is performed.
6. Add projection, webview composition, native observation hooks, controller input normalization, and direct Figma comparison.
7. Enable the capability in the standalone composition only after deterministic, platform seam, callback-safety, recovery, and shutdown tests pass.
8. Run the bounded physical-device handoff on supported interactive hosts and record any unavailable platform evidence honestly.
9. Update `DESIGN.md` with only verified as-built behavior and evidence.

Rollback disables physical MIDI capability composition and Settings entry while retaining backward-compatible ignored preference data. Because `SavedSession` is unchanged, synth sessions require no rollback migration. If the new preference file must be removed, delete only that explicitly resolved per-user file through a recoverable operation; do not touch the application configuration directory broadly.

## Open Questions

None. The product-affecting choices left undefined by the single Figma frame—entry gesture, return scope, reconnection, manual-disconnect behavior, discovery cadence, inspector truthfulness, and responsive derivation—are resolved above so implementation and acceptance do not depend on a later scope decision.
