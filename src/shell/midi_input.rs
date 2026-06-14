//! MIDI input port abstraction for the Shell context.
//!
//! Provides a trait ([`MidiInput`]) that abstracts MIDI port discovery,
//! connection, and raw-message delivery, plus a concrete adapter
//! ([`MidirMidiInput`]) backed by the `midir` crate.
//!
//! # Design
//!
//! * **`MidiInput` trait** – narrow port interface (list_ports, connect, next_event).
//! * **`MidirMidiInput`** – implements the trait using `midir`; incoming bytes are
//!   pushed into an `std::sync::mpsc` channel inside the callback so `next_event`
//!   never blocks.
//! * **Value types** – `MidiPortId`, `MidiPortInfo`, `MidiConnection` are plain
//!   data structs.  `RawMidiMessage` is re-used from [`crate::shell::midi_normalizer`].

use std::sync::mpsc::{self, Receiver, SyncSender};

use crate::shell::midi_normalizer::RawMidiMessage;

// ── Value types ───────────────────────────────────────────────────────────────

/// Opaque identifier for a MIDI input port returned by [`MidiInput::list_ports`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MidiPortId(pub String);

/// Human-readable information about a MIDI input port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiPortInfo {
    pub id: MidiPortId,
    pub name: String,
}

/// An active MIDI connection.  Dropping this value disconnects the port.
///
/// Messages are consumed by calling [`MidiInput::next_event`] after a
/// connection is established.  The `_inner` field holds the live MIDI
/// connection handle; its [`Drop`] impl closes the port automatically.
pub struct MidiConnection {
    /// Opaque connection handle — kept alive for its `Drop` side-effect.
    pub(crate) _inner: Box<dyn std::any::Any + Send>,
}

// ── Port trait ────────────────────────────────────────────────────────────────

/// Narrow MIDI-input port interface.
///
/// Implementors discover available ports, open a connection to one, and
/// deliver raw MIDI bytes on demand.
pub trait MidiInput {
    /// List all available MIDI input ports on the host.
    fn list_ports(&self) -> Vec<MidiPortInfo>;

    /// Open a connection to the given port.
    ///
    /// Returns an error string if the port cannot be opened.  On success the
    /// returned [`MidiConnection`] must be kept alive for the duration of
    /// reception; dropping it closes the port.
    fn connect(&mut self, port_id: MidiPortId) -> Result<MidiConnection, String>;

    /// Return the next buffered [`RawMidiMessage`], or `None` if none has
    /// arrived since the last call.  This method never blocks.
    fn next_event(&self) -> Option<RawMidiMessage>;
}

// ── midir adapter ─────────────────────────────────────────────────────────────

/// Concrete [`MidiInput`] backed by the `midir` crate.
///
/// Incoming MIDI bytes are forwarded from the `midir` callback into a bounded
/// sync channel so that `next_event` is always non-blocking and lock-free
/// (the `SyncSender::try_send` in the callback is non-blocking).
pub struct MidirMidiInput {
    /// The sender half; cloned into each callback closure.
    tx: SyncSender<RawMidiMessage>,
    /// The receiver half; polled by `next_event`.
    rx: Receiver<RawMidiMessage>,
}

impl MidirMidiInput {
    /// Create a new adapter with the given channel capacity.
    ///
    /// `buffer_capacity` is the maximum number of MIDI messages that can be
    /// queued before the callback starts dropping.  A value of 256 is
    /// sufficient for normal real-time use.
    pub fn new(buffer_capacity: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel(buffer_capacity);
        Self { tx, rx }
    }
}

impl Default for MidirMidiInput {
    fn default() -> Self {
        Self::new(256)
    }
}

impl MidiInput for MidirMidiInput {
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

    fn connect(&mut self, port_id: MidiPortId) -> Result<MidiConnection, String> {
        let input = midir::MidiInput::new("crest-synth-input")
            .map_err(|e| format!("midir init error: {e}"))?;

        // Find the port whose name matches the id string.
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
                    // Non-blocking: if the channel is full, drop the message rather
                    // than blocking the audio callback.
                    let _ = tx.try_send(RawMidiMessage::new(bytes.to_vec(), 0));
                },
                (),
            )
            .map_err(|e| format!("midir connect error: {e}"))?;

        Ok(MidiConnection {
            _inner: Box::new(connection),
        })
    }

    fn next_event(&self) -> Option<RawMidiMessage> {
        self.rx.try_recv().ok()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    // ── Value type tests ──────────────────────────────────────────────────────

    #[test]
    fn midi_port_id_equality() {
        let a = MidiPortId("port-a".to_string());
        let b = MidiPortId("port-a".to_string());
        let c = MidiPortId("port-b".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn midi_port_info_fields() {
        let info = MidiPortInfo {
            id: MidiPortId("id".to_string()),
            name: "My MIDI Controller".to_string(),
        };
        assert_eq!(info.id, MidiPortId("id".to_string()));
        assert_eq!(info.name, "My MIDI Controller");
    }

    // ── Channel-based stub that exercises MidiInput trait ─────────────────────

    /// A test double that lets us inject messages directly into the channel,
    /// verifying `next_event` contract without requiring a real MIDI device.
    struct StubMidiInput {
        tx: SyncSender<RawMidiMessage>,
        rx: mpsc::Receiver<RawMidiMessage>,
    }

    impl StubMidiInput {
        fn new() -> Self {
            let (tx, rx) = mpsc::sync_channel(16);
            Self { tx, rx }
        }

        fn inject(&self, msg: RawMidiMessage) {
            self.tx.try_send(msg).expect("channel full in test");
        }
    }

    impl MidiInput for StubMidiInput {
        fn list_ports(&self) -> Vec<MidiPortInfo> {
            vec![MidiPortInfo {
                id: MidiPortId("stub-0".to_string()),
                name: "Stub Port 0".to_string(),
            }]
        }

        fn connect(&mut self, _port_id: MidiPortId) -> Result<MidiConnection, String> {
            struct NoopConn;
            Ok(MidiConnection {
                _inner: Box::new(NoopConn),
            })
        }

        fn next_event(&self) -> Option<RawMidiMessage> {
            self.rx.try_recv().ok()
        }
    }

    #[test]
    fn next_event_returns_none_when_empty() {
        let stub = StubMidiInput::new();
        assert!(stub.next_event().is_none());
    }

    #[test]
    fn next_event_returns_injected_message() {
        let stub = StubMidiInput::new();
        let msg = RawMidiMessage::new(vec![0x80, 0x3C, 0x00], 0);
        stub.inject(msg.clone());
        assert_eq!(stub.next_event(), Some(msg));
    }

    #[test]
    fn next_event_drains_in_fifo_order() {
        let stub = StubMidiInput::new();
        let m1 = RawMidiMessage::new(vec![0x90, 0x3C, 0x7F], 0);
        let m2 = RawMidiMessage::new(vec![0x90, 0x40, 0x64], 0);
        stub.inject(m1.clone());
        stub.inject(m2.clone());
        assert_eq!(stub.next_event(), Some(m1));
        assert_eq!(stub.next_event(), Some(m2));
        assert!(stub.next_event().is_none());
    }

    #[test]
    fn list_ports_returns_stub_port() {
        let stub = StubMidiInput::new();
        let ports = stub.list_ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].name, "Stub Port 0");
    }

    #[test]
    fn connect_returns_connection() {
        let mut stub = StubMidiInput::new();
        let result = stub.connect(MidiPortId("stub-0".to_string()));
        assert!(result.is_ok());
    }
}
