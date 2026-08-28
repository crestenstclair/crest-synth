## Why

Crest Synth currently starts only the automatic MIDI fixture and has no production capability for discovering, selecting, or safely receiving from a physical MIDI input. A focused controller-first Settings surface is needed so a performer can choose exactly one device, see truthful connection and activity state, and retain that choice across hot-plug and restart without weakening the reducer or real-time boundaries.

## What Changes

- Add a temporary `SETTINGS / MIDI DEVICES` system surface aligned with Figma node `116:2`; it remembers and returns to the exact PATCH or MIXER semantic origin and does not create a third top-level performance context.
- Discover physical MIDI input ports automatically, preserve stable device identity and list focus across rescans, and expose explicit Available, Connecting, Connected, Unavailable, Disconnected, and Failed states.
- Select and connect at most one physical MIDI input, support explicit disconnect/switch/hot-unplug/same-identity reconnect/shutdown behavior, and never substitute another port by label or proximity.
- Add a capability-backed `midir` platform adapter, normalize accepted MIDI 1.0 channel messages with the existing `midly` parser, and carry fixed-size revision-tagged events over a dedicated bounded `rtrb` SPSC queue.
- Drain physical input with a fixed per-tick budget through `AppLoop::dispatch_midi_from`, attribute it separately from `AutomaticMidi`, and preserve per-recipient `AppState::apply` plus existing ordered audio-command publication.
- Separate persisted selection preference, reducer-owned configuration/lifecycle state, adapter-owned handles, bounded transport health, and decimated non-product MIDI observations.
- Add typed failure/overflow accounting and control-side all-notes-off recovery for disconnect, loss, switch, overflow, and shutdown.
- Add production-path reducer, projector, adapter, transport, renderer, native-layout, callback-safety, and physical-device evidence requirements.

## Capabilities

### New Capabilities

- `midi-device-settings`: Physical MIDI input discovery, single-device lifecycle, stable identity and persistence, MIDI normalization and transport, temporary Settings interaction, observation, failures, and production-path acceptance.

### Modified Capabilities

- `responsive-shell-composition`: Extend the existing presentation-only responsive, focus, accessibility, and native-measurement contract to the MIDI Devices Settings list/inspector composition.

## Impact

- Affected control/application areas include canonical state and events, semantic focus/action resolution, `AppLoop`, event recording, projection/serialization, persistence, standalone composition, and shutdown orchestration.
- Affected adapter/transport areas include a new physical MIDI capability and `midir` implementation, the existing `midly` normalization boundary, a dedicated `rtrb` SPSC queue, and decimated observation/status transport.
- Affected UI and proof areas include the webview Settings composition, controller/keyboard normalization, footer action guidance, responsive/native witnesses, fake capability tests, platform-adapter integration seams, and live physical-device handoff evidence.
- `Cargo.toml` gains `midir` while retaining the existing `midly = 0.5.3` and `rtrb = 0.3.4` dependencies. Platform-specific backend types remain private to adapters.
- Implementation will require a durable `DESIGN.md` update describing the as-built MIDI-device capability, state ownership, transport, persistence boundary, and verified evidence; this proposal itself is not an as-built authority.
