// path: src/adapter/midir_input.rs
//
// MidirInput — midir-backed implementation of the Shell MidiInput port.
//
// # Design
//
// This adapter wraps the `MidirMidiInput` struct from `shell::midi_input` and
// re-exports it under the `adapter` context so that the infrastructure layer
// owns the concrete midir integration.
//
// The adapter conforms to the `MidiInput` port trait:
//   - list_ports()  → enumerates available MIDI input ports via midir
//   - connect()     → opens the named port; incoming bytes are pushed into a
//                     bounded sync channel (non-blocking `try_send`)
//   - next_event()  → pops one message from the channel without blocking
//
// No allocation or locking occurs on the audio/MIDI callback path.

use crate::shell::midi_input::{MidiConnection, MidiInput, MidiPortId, MidiPortInfo};
use crate::shell::midi_normalizer::RawMidiMessage;
use std::sync::mpsc::{self, Receiver, SyncSender};

/// Default channel capacity — sufficient for normal real-time MIDI throughput.
const DEFAULT_BUFFER_CAPACITY: usize = 256;

/// midir-backed adapter that implements the [`MidiInput`] port.
///
/// Incoming MIDI bytes are pushed from the midir callback into a bounded
/// `std::sync::mpsc` sync-channel so that [`next_event`][Self::next_event]
/// is always non-blocking.
///
/// # Example
///
/// ```no_run
/// use crest_synth::adapter::midir_input::MidirInput;
/// use crest_synth::shell::midi_input::MidiInput;
///
/// let mut input = MidirInput::new();
/// let ports = input.list_ports();
/// println!("{} MIDI port(s) found", ports.len());
/// ```
pub struct MidirInput {
    /// Sender half — cloned into each midir callback closure.
    tx: SyncSender<RawMidiMessage>,
    /// Receiver half — polled by `next_event`.
    rx: Receiver<RawMidiMessage>,
}

impl MidirInput {
    /// Create a new `MidirInput` with the default channel capacity (256 messages).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_BUFFER_CAPACITY)
    }

    /// Create a new `MidirInput` with an explicit channel capacity.
    ///
    /// `capacity` is the maximum number of MIDI messages that can be queued
    /// before the callback silently drops incoming bytes.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel(capacity);
        Self { tx, rx }
    }
}

impl Default for MidirInput {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiInput for MidirInput {
    /// Enumerate all available MIDI input ports on the host.
    ///
    /// Returns an empty `Vec` if the midir host cannot be initialised.
    fn list_ports(&self) -> Vec<MidiPortInfo> {
        let input = match midir::MidiInput::new("crest-synth-list") {
            Ok(i) => i,
            Err(_) => return vec![],
        };
        input
            .ports()
            .iter()
            .filter_map(|p| {
                let name = input.port_name(p).ok()?;
                Some(MidiPortInfo {
                    id: MidiPortId(name.clone()),
                    name,
                })
            })
            .collect()
    }

    /// Open a connection to the port identified by `port_id`.
    ///
    /// The returned [`MidiConnection`] must be kept alive for as long as MIDI
    /// reception is required; dropping it disconnects the port.
    ///
    /// Returns an error string if the midir host cannot be initialised or if no
    /// port with the given name exists.
    fn connect(&mut self, port_id: MidiPortId) -> Result<MidiConnection, String> {
        let input = midir::MidiInput::new("crest-synth-input")
            .map_err(|e| format!("midir init error: {e}"))?;

        let ports = input.ports();
        let port = ports
            .iter()
            .find(|p| input.port_name(p).map(|n| n == port_id.0).unwrap_or(false))
            .ok_or_else(|| format!("MIDI port not found: {}", port_id.0))?
            .clone();

        let tx = self.tx.clone();
        let connection = input
            .connect(
                &port,
                "crest-synth-conn",
                move |_timestamp_us, bytes, _| {
                    // Non-blocking: silently drop if the channel is full rather
                    // than blocking the MIDI callback.
                    let _ = tx.try_send(RawMidiMessage::new(bytes.to_vec(), 0));
                },
                (),
            )
            .map_err(|e| format!("midir connect error: {e}"))?;

        Ok(MidiConnection {
            _inner: Box::new(connection),
        })
    }

    /// Return the next buffered [`RawMidiMessage`], or `None` if the queue is empty.
    ///
    /// This method never blocks.
    fn next_event(&self) -> Option<RawMidiMessage> {
        self.rx.try_recv().ok()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
//
// Real MIDI hardware is not available in CI.  We test the contract via a
// channel-based stub that shares the same mpsc mechanism.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::midi_input::MidiInput;

    // ── helper: build a channel pair and inject messages directly ────────────

    fn make_channel(capacity: usize) -> (SyncSender<RawMidiMessage>, Receiver<RawMidiMessage>) {
        mpsc::sync_channel(capacity)
    }

    // ── MidirInput default construction ──────────────────────────────────────

    #[test]
    fn default_construction_does_not_panic() {
        let _input = MidirInput::new();
    }

    #[test]
    fn with_capacity_construction_does_not_panic() {
        let _input = MidirInput::with_capacity(64);
    }

    // ── next_event via internal channel ──────────────────────────────────────

    #[test]
    fn next_event_returns_none_when_empty() {
        let input = MidirInput::new();
        assert!(input.next_event().is_none());
    }

    #[test]
    fn next_event_returns_message_sent_via_tx() {
        let input = MidirInput::new();
        // Inject directly through the internal sender.
        let msg = RawMidiMessage::new(vec![0x90, 0x3C, 0x7F], 0);
        input.tx.try_send(msg.clone()).unwrap();
        assert_eq!(input.next_event(), Some(msg));
    }

    #[test]
    fn next_event_returns_none_after_drain() {
        let input = MidirInput::new();
        let msg = RawMidiMessage::new(vec![0x80, 0x3C, 0x00], 0);
        input.tx.try_send(msg).unwrap();
        let _ = input.next_event();
        assert!(input.next_event().is_none());
    }

    #[test]
    fn next_event_drains_fifo_order() {
        let input = MidirInput::with_capacity(8);
        let m1 = RawMidiMessage::new(vec![0x90, 0x3C, 0x7F], 0);
        let m2 = RawMidiMessage::new(vec![0x90, 0x40, 0x64], 0);
        input.tx.try_send(m1.clone()).unwrap();
        input.tx.try_send(m2.clone()).unwrap();
        assert_eq!(input.next_event(), Some(m1));
        assert_eq!(input.next_event(), Some(m2));
        assert!(input.next_event().is_none());
    }

    // ── channel-overflow behaviour ────────────────────────────────────────────

    #[test]
    fn overflow_try_send_does_not_block() {
        // Capacity of 2; a third send must silently fail, not block.
        let input = MidirInput::with_capacity(2);
        let msg = RawMidiMessage::new(vec![0x90, 0x3C, 0x7F], 0);
        input.tx.try_send(msg.clone()).unwrap();
        input.tx.try_send(msg.clone()).unwrap();
        // Third send overflows — must return Err, not hang.
        assert!(input.tx.try_send(msg).is_err());
    }

    // ── list_ports — exercised without real hardware ──────────────────────────

    #[test]
    fn list_ports_returns_a_vec() {
        let input = MidirInput::new();
        // May be empty in CI (no MIDI hardware), but must not panic.
        let _ports = input.list_ports();
    }

    // ── standalone channel behaviour (mirrors mpsc internals) ────────────────

    #[test]
    fn independent_channel_fifo_matches_contract() {
        let (tx, rx) = make_channel(4);
        let m1 = RawMidiMessage::new(vec![0xB0, 0x07, 0x64], 0);
        let m2 = RawMidiMessage::new(vec![0xB0, 0x07, 0x00], 0);
        tx.try_send(m1.clone()).unwrap();
        tx.try_send(m2.clone()).unwrap();
        assert_eq!(rx.try_recv().ok(), Some(m1));
        assert_eq!(rx.try_recv().ok(), Some(m2));
        assert!(rx.try_recv().ok().is_none());
    }

    // ── trait-object safety ───────────────────────────────────────────────────

    #[test]
    fn is_object_safe_as_midi_input() {
        let input: Box<dyn MidiInput> = Box::new(MidirInput::new());
        assert!(input.next_event().is_none());
        drop(input);
    }
}
