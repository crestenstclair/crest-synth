## 1. Canonical Contracts and Dependency Setup

- [x] 1.1 Add `midir = "0.11.0"` with documented CoreMIDI/ALSA/WinMM defaults and optional JACK/WinRT feature policy, then verify `cargo check --all-targets` resolves without changing the existing `midly = 0.5.3` or `rtrb = 0.3.4` contracts.
- [x] 1.2 Add the one canonical opaque device ID, descriptor, optional port-facts, scan ID, request ID, connection revision, connection status, failure, connect request, physical event, activity snapshot, active handle, and preference types; verify bounded validation, checked ID exhaustion, serialization descriptors, and duplicate-public-type guards with focused tests.
- [x] 1.3 Prove `PhysicalMidiEvent` is fixed-size, `Copy`, and destructor-free and that neither it nor callback diagnostics contain a `String`, collection, backend type, or owned handle; verify compile-time trait assertions and size/layout tests pass.
- [x] 1.4 Add the object-safe MIDI input device and preference capability ports plus opaque ingress/active-handle ownership; verify trait-object tests cover enumeration, connect, consuming disconnect, preference load/store, and no backend type leakage.
- [x] 1.5 Add `EventSource::PhysicalMidi`, its stable `physicalMidi` serialization, descriptors, exhaustive record/report coverage, and distinction from `AutomaticMidi`; verify focused event-record and live-report schema tests pass.

## 2. Temporary Settings Interaction and Focus

- [x] 2.1 Add the MIDI Devices system surface classification without adding a `TopLevelContext`, update performance-context queries exhaustively, and verify PATCH/MIXER remain the only top-level serialized values.
- [x] 2.2 Extend the canonical `FocusPath` and `SemanticControlId` for device identities and the empty-list/root target; verify valid/invalid shape tests, identity serialization, and exactly-one-focus invariants pass.
- [x] 2.3 Add the reducer-owned `MidiSettingsSession` that suspends exact performance focus, mode, existing return path, and Patch subordinate state; verify PATCH and MIXER entry/return round trips restore exact origins and cannot stack Settings.
- [x] 2.4 Add deterministic Settings focus reconciliation across reorder, disappearance, unavailable retention, permanent removal, empty registry, and return-origin schema repair; verify exact-then-next-then-previous/root focus tests pass.
- [x] 2.5 Add `OpenMidiSettings` semantic action and reducer admission rules that reject Modal, Adjust, held-preview, repeat, and recursive entry; verify rejected actions leave generation, focus, return state, and effects unchanged.
- [x] 2.6 Normalize Shift+Start to one non-repeating Settings action and retain Shift+Down Return through keyboard/controller adapters; verify platform input translation tests include modifier transitions, Sample Browser Start non-regression, and no callback unwind.

## 3. Reducer-Owned Registry and Connection Lifecycle

- [x] 3.1 Add reducer state for selected preference, session connection intent, requested candidate, active identity/revision, scan state, ordered descriptors, and typed failure; verify every product/runtime transition occurs only through `AppState::apply`.
- [x] 3.2 Implement scan started/succeeded/failed events and identity-based reconciliation with normalized initial ordering, prior relative-order retention, selected/focused tombstones, last-success staleness, and duplicate-ID rejection; verify fake scans reordered across polls keep stable order and focus.
- [x] 3.3 Implement initial Connect admission with checked request/revision allocation, Connecting state, exact identity, and typed effects; verify unavailable, duplicate, busy, and exhausted cases reject without fallback or partial state.
- [x] 3.4 Implement candidate switch state that keeps the acknowledged selected/active input until matching activation and reverts no state on candidate failure; verify the old revision remains the sole accepted source while the candidate is disabled.
- [x] 3.5 Implement matching Prepared and Activation Acknowledged events plus stale request/revision rejection; verify every interleaving of current, superseded, cancelled, and duplicate results leaves at most one active revision.
- [x] 3.6 Implement explicit Disconnect, definitive loss, same-identity return, overflow failure, and shutdown reducer events/effects; verify visible Disconnected/Unavailable/Failed states, connection intent, and fresh reconnect correlations match the specification.
- [x] 3.7 Define exhaustive row-state and valid-action derivation for Available, Connecting, Connected, Unavailable, Disconnected, and Failed; verify each state has text, structural marker metadata, and only its allowed Connect/Disconnect/Retry action.

## 4. Device Worker, Platform Adapter, and Persistence

- [x] 4.1 Implement a bounded nonblocking MIDI device worker command/result protocol for scan, connect, disconnect/retire, and preference I/O with at most one scan and one connect in flight; verify full/busy/stale channels preserve owned cleanup values and never block control.
- [x] 4.2 Implement the `midir` enumeration adapter using `ports`, opaque `id`, and `port_name`, including identity-schema namespacing and typed init/port-info failures; verify a fake backend covers duplicates/failures and a real host seam reports actual zero-or-more ports truthfully.
- [x] 4.3 Implement exact-ID connection lookup and candidate connect with `Ignore::None`, disabled prebuilt ingress, and opaque active-handle ownership; verify duplicate names never affect lookup and callbacks before activation cannot enqueue accepted events.
- [x] 4.4 Implement consuming midir close/retirement only on the device worker and typed abnormal retirement handling; verify thread/ownership instrumentation reports that active and stale candidate handles reach zero without callback-side destruction.
- [x] 4.5 Implement one-second coalesced scan scheduling at startup, Settings entry, and steady state, with no more than one scan in flight; verify a deterministic clock test records expected scans and no scan is performed by either callback.
- [x] 4.6 Add version-1 `MidiInputPreference` JSON in the shell-resolved per-user config directory with bounded fields and recoverable temporary-file/rename writes; verify absent, round-trip, invalid, unsupported-version, read, and write cases are typed.
- [x] 4.7 Wire preference load and accepted selection/update effects through the worker and reducer, default restart intent to Enabled, and keep manual disconnect/runtime status unpersisted; verify present/absent same identity restart behavior and no name-based match.
- [x] 4.8 Add an exact serialized-field assertion proving `SavedSession` remains version 2 and contains no MIDI device preference, descriptor, handle, connection, callback, queue, timestamp, or observation field.

## 5. Bounded Physical Ingress and MIDI Normalization

- [x] 5.1 Implement one 1024-event `rtrb` SPSC ring per candidate connection, with immutable revision, atomic enable gate, and fixed diagnostic counters; verify old and candidate connections never share a producer and full push never overwrites queued data.
- [x] 5.2 Implement bounded status-byte preclassification so empty/malformed input and System Common/Realtime/SysEx are counted without scanning or storing long payloads; verify a large SysEx fixture performs constant bounded application work and emits no event.
- [x] 5.3 Normalize valid raw channel messages through `midly::live::LiveEvent::parse` into the canonical model, including velocity-zero Note On to Note Off and exact LSB/MSB pitch bend; verify a table test covers every supported kind and all seven-bit boundaries.
- [x] 5.4 Normalize controllers 120 and 123 to canonical All Notes Off, leave other controllers as Control Change, and classify polyphonic pressure as unsupported; verify no unsupported class silently converts or dispatches.
- [x] 5.5 On physical queue full, atomically count capacity failure, disable the ingress gate, and return immediately; verify callback tests show no allocation, deallocation, lock, block, I/O, logging, formatting, panic/unwind, application access, UI work, or owned destruction.
- [x] 5.6 Implement the control-owned 64-event-per-tick drain with exact active-revision rejection and backlog retention; verify 0, 1, 64, and more-than-64 event cases preserve order and bound per-tick work.
- [x] 5.7 Route every matching drained event through `AppLoop::dispatch_midi_from(..., EventSource::PhysicalMidi)` and stop on audio boundary saturation; verify zero/one/multiple same-channel subscribers produce the specified no-op or stable per-Patch reducer/event-record/audio-command order.

## 6. Recovery, Activation, Observation, and Shutdown Orchestration

- [x] 6.1 Reserve one existing audio-command queue slot for `AudioCommand::AllNotesOff`, require capacity at least two, and coalesce duplicate pending recovery; verify normal commands cannot consume the reserve and recovery remains retryable when already pending.
- [x] 6.2 Implement controlled candidate activation that keeps ingress disabled until matching reducer acknowledgement, disables old ingress before switch, completes recovery before enabling the new gate, and retires old ownership off callback; verify no interleaving accepts both revisions.
- [x] 6.3 Implement explicit disconnect and hot-loss orchestration that disables/discards ingress, invalidates revision through the reducer, publishes recovery, and retires the handle; verify queued/racing retired messages produce no event or audio command.
- [x] 6.4 Implement physical-queue and audio-command capacity recovery that invalidates the whole revision, exposes typed failure, clears voices, retires, and reconnects only the same present identity under a fresh request/revision; verify renderer/engine observations show no stuck notes.
- [x] 6.5 Implement shutdown ordering that stops work, disables every gate, cancels candidates, publishes/coalesces recovery, discards ingress, joins retirement/device workers, and preserves existing audio graph retirement; verify zero retained handles/graphs and zero callback destruction.
- [x] 6.6 Add a fixed latest-compatible MIDI activity observation boundary with accepted count, last message/timestamp, diagnostic totals, overflow epoch, and revision; verify stale revisions are rejected and no observation mutates `AppState`.
- [x] 6.7 Decimate activity publication to at most 30 Hz and derive Receiving only within 500 ms of the last control-side receipt; verify burst tests produce bounded snapshots/renders and idle Connected state changes to Waiting without a reducer event.

## 7. Projection, Serialization, and Renderer

- [x] 7.1 Add canonical Settings projection data for suspended context, title/summary, scan state, stable rows, statuses/failures, focus/repair, active inspector, revision, and footer actions; verify projection contains installed registry content and no backend handle or observation history.
- [x] 7.2 Update semantic model/state-tree/event-record schema descriptors and versions for the new system surface and lifecycle fields; verify exact serialization, exhaustive leaf/property descriptors, deterministic round trips, and no-label-as-serialization-key guards.
- [x] 7.3 Add projection tests for empty registry, duplicate/long names, maximum acceptance registry, every row/scan/failure state, unknown facts, selected unavailable, active-vs-requested switch, and exact focus/return identity.
- [x] 7.4 Add the Settings header, identity band, Available Inputs list, active Input Inspector, and sole footer action guide to the committed webview renderer using existing tokens and one semantic DOM; verify no fixture device/value/count is hardcoded.
- [ ] 7.5 Render every status with text and structural shape/keyline as well as color, render Unknown/Not Reported facts truthfully, and keep the inspector anchored to the exact active revision; verify non-color DOM assertions and stale-observation fixtures pass.
- [ ] 7.6 Combine only a matching decimated `MidiActivitySnapshot` into the activity card, including count/kind/channel/data and applicable value visualization; verify mismatched observations paint Waiting/stale and cause no product serialization or action.
- [ ] 7.7 Implement one continuously adaptive list-inspector workspace: an approximately 80/20 starting fraction while both regions fit, content-driven inspector clamps, a flexible list remainder, and list-then-inspector stacking only when the declared minima and required gaps cannot coexist; verify arbitrary incremental resize uses no aspect-ratio or named-viewport layout switch, emits no semantic action, and preserves projection/focus/return identity.

## 8. Standalone Composition and Production-Path Integration

- [x] 8.1 Compose the device worker, midir adapter, preference store, ingress runtime, observation reader, and Settings projection into `StandaloneApplication` without changing the automatic fixture capability; verify both sources remain distinct and use the shared AppLoop fan-out only after their own boundaries.
- [x] 8.2 Poll structural work, device results, physical ingress, diagnostics, and observations in a fixed documented control-tick order; verify a trace test proves bounded ordering and that no worker/callback directly mutates application state.
- [x] 8.3 Add production startup reconciliation for absent, present, unavailable, invalid-preference, and adapter-init states; verify the application still opens with truthful Settings state and never auto-selects a different port.
- [x] 8.4 Add application error/status mapping and clean teardown propagation for worker, platform, persistence, transport, and retirement failures; verify no typed failure is swallowed, formatted in a callback, or converted to a fallback.
- [ ] 8.5 Add Linux ALSA/JACK and Windows WinMM/WinRT build notes/feature checks plus macOS CoreMIDI host coverage; verify supported CI/cross-check targets compile or report a scoped environmental limitation without backend-type leakage.

## 9. Deterministic, Native, and Physical Acceptance Evidence

- [ ] 9.1 Build a deterministic fake-capability lifecycle matrix covering reordered rescans, stable identity/focus, duplicate names, connect, switch success/failure, disconnect, loss, same-identity return, similar-name non-match, stale completion/revision, overflow, and shutdown; verify all tests exercise the production reducer/effect orchestrator rather than a parallel model.
- [ ] 9.2 Add a controlled raw-callback end-to-end witness correlating bytes through `midly`, the real `rtrb` ingress, bounded control drain, `dispatch_midi_from`, `AppState::apply`, Physical MIDI records, existing audio command boundary, prepared graph, and rendered audio; verify zero/one/multiple subscriber cases and negative unsupported/malformed cases.
- [ ] 9.3 Add measured callback-safety instrumentation for physical input, bursts, overflow, switch, loss, and shutdown; verify the MIDI callback's bounded allowed operations and zero forbidden audio-callback allocation/deallocation/lock/block/I/O/log/format/panic/unwind/destruction.
- [ ] 9.4 Extend the native webview witness with a continuous Settings width sweep across and beyond the content-driven stack threshold, retaining Wide, Standard, Intermediate, Compact, 1280×800, and enlarged-text points only as named samples; verify proportional track behavior, monotonic region bounds, exact focus, 48 px floors, overflow/overlap, scroll endpoints, footer ownership, observation revision, repeated paint, and resize neutrality without aspect-ratio branching.
- [ ] 9.5 Capture readable native screenshots at the Figma reference size and several non-reference arbitrary widths; compare the reference sample directly with live Figma node `116:2`, verify that every other sample follows the same fluid constraints and preserves hierarchy, typography/tokens, row/status anatomy, inspector/activity, and footer, and record every remaining discrepancy without broad parity claims.
- [x] 9.6 Run a physical keyboard/controller handoff for Shift+Start entry, device navigation, Connect/Disconnect/Retry, live activity, hot unplug/return, and Shift+Down exact return; verify semantic actions, reducer records, visible states, audio response, and clean exit in the production app.
- [x] 9.7 Run at least one attached physical MIDI device through the real platform adapter and production audio graph; verify only the active selected identity is accepted, retired/similar ports are rejected, supported messages sound, unsupported/malformed counters are truthful, and no stuck note survives loss/switch/overflow.
- [x] 9.8 Run focused tests, exact-validation checks, formatting, warnings-denied Clippy, JavaScript syntax checks, `cargo test --all-targets`, and the relevant native/live targets; verify every required gate passes or a typed environmental skip remains explicitly incomplete acceptance.

## 10. Durable Documentation and Handoff

- [x] 10.1 Update `DESIGN.md` in the implementation commit with only verified as-built physical MIDI capability, state ownership, bounds, lifecycle/recovery, persistence, Settings interaction, platform support, and evidence; verify it names remaining gaps and does not cite OpenSpec as proof.
- [x] 10.2 Add scoped developer/operator commands for real-adapter enumeration, native Settings rendering, and physical MIDI handoff, including safe teardown and environmental requirements; verify each command is bounded, repeatable, and reports success/failure evidence explicitly.
- [ ] 10.3 Re-run `openspec validate add-midi-device-settings --strict` after implementation documentation changes and verify every acceptance item has linked production-path evidence before requesting archive.
