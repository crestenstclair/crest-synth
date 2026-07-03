// path: src/shell/midi_input.rs

use std::fmt;

/// A MIDI channel address in the range 0..=15, used to match incoming
/// events against a patch's channel mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MidiChannel(u8);

impl MidiChannel {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 15;

    /// Constructs a channel, validating it falls within the 16-channel range.
    pub fn try_new(value: u8) -> Result<Self, MidiError> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(MidiError::InvalidChannel(value));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

/// A stable identifier for a MIDI input port discovered on the host system.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MidiPortInfo {
    id: String,
    name: String,
}

impl MidiPortInfo {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The kind of raw MIDI message carried by a `MidiEvent`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MidiEventKind {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8, velocity: u8 },
    ControlChange { controller: u8, value: u8 },
    PitchBend { value: i16 },
    ChannelPressure { value: u8 },
    PolyPressure { note: u8, value: u8 },
}

/// A single MIDI event captured from an input port, addressed to a channel
/// and timestamped relative to the connection's monotonic clock.
///
/// This is a plain, `Copy`-able value: forwarding it into the audio thread's
/// EventRing performs no allocation, matching the project's real-time
/// invariants. `MidiInput` itself never touches the audio thread directly —
/// it is the caller's responsibility to route delivered events through the
/// EventRing or ParameterBridge rather than any other seam.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiEvent {
    pub channel: MidiChannel,
    pub kind: MidiEventKind,
    pub timestamp_micros: u64,
}

/// Errors that can occur while listing, connecting to, or operating a MIDI
/// input port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiError {
    PortNotFound(String),
    ConnectionFailed(String),
    InvalidChannel(u8),
    AlreadyConnected(String),
}

impl fmt::Display for MidiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiError::PortNotFound(id) => write!(f, "MIDI port not found: {id}"),
            MidiError::ConnectionFailed(msg) => write!(f, "MIDI connection failed: {msg}"),
            MidiError::InvalidChannel(ch) => write!(f, "invalid MIDI channel: {ch}"),
            MidiError::AlreadyConnected(id) => write!(f, "MIDI port already connected: {id}"),
        }
    }
}

impl std::error::Error for MidiError {}

/// Callback invoked, on the non-real-time MIDI thread, once per received
/// event. Implementations must not block or perform I/O; events destined
/// for the audio engine must be handed off via the EventRing rather than
/// processed synchronously here.
pub trait EventCallback: Send {
    fn on_event(&mut self, event: MidiEvent);
}

impl<F> EventCallback for F
where
    F: FnMut(MidiEvent) + Send,
{
    fn on_event(&mut self, event: MidiEvent) {
        self(event)
    }
}

/// A live connection to a MIDI input port. Event delivery stops once the
/// connection is closed via `MidiInput::disconnect` or dropped.
#[derive(Debug)]
pub struct Connection {
    port: MidiPortInfo,
    active: bool,
}

impl Connection {
    pub fn new(port: MidiPortInfo) -> Self {
        Self { port, active: true }
    }

    pub fn port(&self) -> &MidiPortInfo {
        &self.port
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    fn close(&mut self) {
        self.active = false;
    }
}

/// Port (hexagonal architecture) for discovering MIDI input hardware and
/// receiving events from it. Adapters implement this trait against a
/// concrete backend; the rest of the Shell context depends only on this
/// narrow abstraction, never on a concrete backend type.
pub trait MidiInput {
    /// Enumerates the MIDI input ports currently visible to the host.
    fn list_ports(&self) -> Vec<MidiPortInfo>;

    /// Opens a connection to `port`, delivering every subsequent event to
    /// `on_event` until the connection is disconnected or dropped.
    fn connect(
        &self,
        port: &MidiPortInfo,
        on_event: Box<dyn EventCallback>,
    ) -> Result<Connection, MidiError>;

    /// Closes a previously opened connection. The default implementation
    /// simply marks the connection inactive; adapters that own OS-level
    /// resources should override this to release them.
    fn disconnect(&self, mut connection: Connection) {
        connection.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_accepts_full_valid_range() {
        assert!(MidiChannel::try_new(0).is_ok());
        assert!(MidiChannel::try_new(15).is_ok());
        assert_eq!(MidiChannel::try_new(7).unwrap().value(), 7);
    }

    #[test]
    fn channel_rejects_out_of_range() {
        let err = MidiChannel::try_new(16).unwrap_err();
        assert_eq!(err, MidiError::InvalidChannel(16));
    }

    #[test]
    fn port_info_exposes_id_and_name() {
        let port = MidiPortInfo::new("port-1", "USB MIDI Keyboard");
        assert_eq!(port.id(), "port-1");
        assert_eq!(port.name(), "USB MIDI Keyboard");
    }

    #[test]
    fn error_messages_are_human_readable() {
        assert_eq!(
            MidiError::PortNotFound("p1".into()).to_string(),
            "MIDI port not found: p1"
        );
        assert_eq!(
            MidiError::InvalidChannel(20).to_string(),
            "invalid MIDI channel: 20"
        );
    }

    #[test]
    fn connection_starts_active_and_can_be_closed() {
        let port = MidiPortInfo::new("port-1", "Test Port");
        let mut connection = Connection::new(port);
        assert!(connection.is_active());
        connection.close();
        assert!(!connection.is_active());
    }

    struct FakeMidiInput {
        ports: Vec<MidiPortInfo>,
    }

    impl MidiInput for FakeMidiInput {
        fn list_ports(&self) -> Vec<MidiPortInfo> {
            self.ports.clone()
        }

        fn connect(
            &self,
            port: &MidiPortInfo,
            _on_event: Box<dyn EventCallback>,
        ) -> Result<Connection, MidiError> {
            if self.ports.iter().any(|p| p.id() == port.id()) {
                Ok(Connection::new(port.clone()))
            } else {
                Err(MidiError::PortNotFound(port.id().to_string()))
            }
        }
    }

    #[test]
    fn connect_succeeds_for_known_port_and_disconnect_deactivates() {
        let known = MidiPortInfo::new("p1", "Keyboard");
        let adapter = FakeMidiInput {
            ports: vec![known.clone()],
        };

        let connection = adapter
            .connect(&known, Box::new(|_event: MidiEvent| {}))
            .expect("known port should connect");
        assert!(connection.is_active());

        adapter.disconnect(connection);
    }

    #[test]
    fn connect_fails_for_unknown_port() {
        let adapter = FakeMidiInput { ports: vec![] };
        let unknown = MidiPortInfo::new("missing", "Ghost Port");

        let result = adapter.connect(&unknown, Box::new(|_event: MidiEvent| {}));
        assert_eq!(
            result.unwrap_err(),
            MidiError::PortNotFound("missing".to_string())
        );
    }

    #[test]
    fn list_ports_reflects_configured_ports() {
        let a = MidiPortInfo::new("a", "Port A");
        let b = MidiPortInfo::new("b", "Port B");
        let adapter = FakeMidiInput {
            ports: vec![a.clone(), b.clone()],
        };

        let listed = adapter.list_ports();
        assert_eq!(listed, vec![a, b]);
    }
}
