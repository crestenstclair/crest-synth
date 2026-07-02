// path: src/shell/midir_midi_input.rs

//! Adapter implementing `port::Shell::MidiInput` against the `midir` crate.
//!
//! This adapter owns no audio-thread state: it lives entirely on the
//! non-real-time MIDI/UI side. Incoming MIDI bytes are decoded on midir's
//! own background thread and handed to the caller-supplied `EventCallback`;
//! it is the caller's responsibility (per the port's contract) to forward
//! events into the EventRing or ParameterBridge rather than touching the
//! audio thread directly from here.
//!
//! `midir::MidiInputPort` values are backend handles with no stable
//! cross-enumeration identity of their own, so this adapter derives a
//! `MidiPortInfo::id()` from each port's position in a fresh enumeration and
//! re-resolves that id back to a concrete `MidiInputPort` at connect time.
//! Live native connections are tracked in a `Mutex`-guarded map keyed by
//! that id (fine here: this type is never invoked from the audio thread),
//! so `disconnect` can find and drop the right one.

use std::collections::HashMap;
use std::sync::Mutex;

use midir::{Ignore, MidiInput as MidirBackend, MidiInputConnection, MidiInputPort};

use crate::shell::midi_input::{
    Connection, EventCallback, MidiChannel, MidiError, MidiEvent, MidiEventKind, MidiInput,
    MidiPortInfo,
};

/// `port::Shell::MidiInput` adapter backed by the cross-platform `midir`
/// crate (CoreMIDI on macOS, ALSA on Linux, WinMM on Windows).
pub struct MidirMidiInput {
    client_name: String,
    connections: Mutex<HashMap<String, MidiInputConnection<()>>>,
}

impl MidirMidiInput {
    /// Creates an adapter that identifies itself to the OS MIDI subsystem
    /// as `client_name`.
    pub fn new(client_name: impl Into<String>) -> Self {
        Self {
            client_name: client_name.into(),
            connections: Mutex::new(HashMap::new()),
        }
    }

    fn open_backend(&self) -> Result<MidirBackend, MidiError> {
        let mut backend = MidirBackend::new(&self.client_name)
            .map_err(|err| MidiError::ConnectionFailed(err.to_string()))?;
        // We decode every message type ourselves; don't let midir filter
        // sysex/timing/active-sensing bytes out from under us.
        backend.ignore(Ignore::None);
        Ok(backend)
    }

    /// Re-resolves a `MidiPortInfo` (produced by a prior `list_ports` call)
    /// back to a concrete `midir::MidiInputPort` in a fresh enumeration.
    fn resolve_port(
        backend: &MidirBackend,
        target: &MidiPortInfo,
    ) -> Result<MidiInputPort, MidiError> {
        backend
            .ports()
            .into_iter()
            .enumerate()
            .find(|(index, _)| index.to_string() == target.id())
            .map(|(_, port)| port)
            .ok_or_else(|| MidiError::PortNotFound(target.id().to_string()))
    }
}

impl MidiInput for MidirMidiInput {
    fn list_ports(&self) -> Vec<MidiPortInfo> {
        let backend = match self.open_backend() {
            Ok(backend) => backend,
            // No MIDI subsystem available on this host: report no ports
            // rather than propagating an error the trait has no room for.
            Err(_) => return Vec::new(),
        };

        backend
            .ports()
            .iter()
            .enumerate()
            .map(|(index, port)| {
                let name = backend
                    .port_name(port)
                    .unwrap_or_else(|_| format!("Unknown MIDI Port {index}"));
                MidiPortInfo::new(index.to_string(), name)
            })
            .collect()
    }

    fn connect(
        &self,
        port: &MidiPortInfo,
        on_event: Box<dyn EventCallback>,
    ) -> Result<Connection, MidiError> {
        {
            let connections = self
                .connections
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if connections.contains_key(port.id()) {
                return Err(MidiError::AlreadyConnected(port.id().to_string()));
            }
        }

        let backend = self.open_backend()?;
        let midir_port = Self::resolve_port(&backend, port)?;

        let mut callback = on_event;
        let connection_name = format!("{}-input", self.client_name);
        let native_connection = backend
            .connect(
                &midir_port,
                &connection_name,
                move |timestamp_micros, message, _| {
                    if let Some(event) = decode_midi_event(timestamp_micros, message) {
                        callback.on_event(event);
                    }
                },
                (),
            )
            .map_err(|err| MidiError::ConnectionFailed(err.to_string()))?;

        let mut connections = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        connections.insert(port.id().to_string(), native_connection);

        Ok(Connection::new(port.clone()))
    }

    fn disconnect(&self, connection: Connection) {
        let mut connections = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Dropping the removed `MidiInputConnection` closes the native
        // port; if it was already gone (e.g. double-disconnect) this is a
        // harmless no-op.
        connections.remove(connection.port().id());
    }
}

/// Decodes a raw MIDI 1.0 message (status byte + up to two data bytes) into
/// a `MidiEvent`. Returns `None` for messages this adapter does not model
/// (e.g. system common/realtime bytes) or for a channel value midir's
/// backend could not have produced (defensive, keeps this function total).
fn decode_midi_event(timestamp_micros: u64, message: &[u8]) -> Option<MidiEvent> {
    let &status = message.first()?;
    let channel = MidiChannel::try_new(status & 0x0F).ok()?;

    let kind = match status & 0xF0 {
        0x80 => MidiEventKind::NoteOff {
            note: *message.get(1)?,
            velocity: *message.get(2)?,
        },
        0x90 => {
            let note = *message.get(1)?;
            let velocity = *message.get(2)?;
            // A note-on with velocity 0 is a running-status note-off by
            // MIDI 1.0 convention.
            if velocity == 0 {
                MidiEventKind::NoteOff { note, velocity }
            } else {
                MidiEventKind::NoteOn { note, velocity }
            }
        }
        0xA0 => MidiEventKind::PolyPressure {
            note: *message.get(1)?,
            value: *message.get(2)?,
        },
        0xB0 => MidiEventKind::ControlChange {
            controller: *message.get(1)?,
            value: *message.get(2)?,
        },
        0xD0 => MidiEventKind::ChannelPressure {
            value: *message.get(1)?,
        },
        0xE0 => {
            let lsb = i16::from(*message.get(1)?);
            let msb = i16::from(*message.get(2)?);
            let raw = (msb << 7) | lsb; // 0..=16383, center at 8192
            MidiEventKind::PitchBend { value: raw - 8192 }
        }
        _ => return None,
    };

    Some(MidiEvent {
        channel,
        kind,
        timestamp_micros,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // `decode_midi_event` is pure and backend-free, so it is exercised
    // directly here. `MidirMidiInput`'s own methods open a real OS MIDI
    // backend and are exercised via `MidiInput`-generic integration tests
    // elsewhere; hitting them here would make unit tests depend on host
    // MIDI subsystem availability.

    #[test]
    fn decodes_note_on_with_nonzero_velocity() {
        let event = decode_midi_event(1_000, &[0x91, 60, 100]).expect("valid note on");
        assert_eq!(event.channel, MidiChannel::try_new(1).unwrap());
        assert_eq!(
            event.kind,
            MidiEventKind::NoteOn {
                note: 60,
                velocity: 100
            }
        );
        assert_eq!(event.timestamp_micros, 1_000);
    }

    #[test]
    fn note_on_with_zero_velocity_decodes_as_note_off() {
        let event = decode_midi_event(0, &[0x90, 60, 0]).expect("valid note on/off");
        assert_eq!(
            event.kind,
            MidiEventKind::NoteOff {
                note: 60,
                velocity: 0
            }
        );
    }

    #[test]
    fn decodes_note_off() {
        let event = decode_midi_event(0, &[0x82, 64, 40]).expect("valid note off");
        assert_eq!(event.channel, MidiChannel::try_new(2).unwrap());
        assert_eq!(
            event.kind,
            MidiEventKind::NoteOff {
                note: 64,
                velocity: 40
            }
        );
    }

    #[test]
    fn decodes_control_change() {
        let event = decode_midi_event(0, &[0xB3, 7, 127]).expect("valid CC");
        assert_eq!(
            event.kind,
            MidiEventKind::ControlChange {
                controller: 7,
                value: 127
            }
        );
    }

    #[test]
    fn decodes_poly_pressure() {
        let event = decode_midi_event(0, &[0xA0, 48, 90]).expect("valid poly pressure");
        assert_eq!(
            event.kind,
            MidiEventKind::PolyPressure {
                note: 48,
                value: 90
            }
        );
    }

    #[test]
    fn decodes_channel_pressure() {
        let event = decode_midi_event(0, &[0xD5, 77]).expect("valid channel pressure");
        assert_eq!(event.kind, MidiEventKind::ChannelPressure { value: 77 });
    }

    #[test]
    fn decodes_pitch_bend_centered_at_zero() {
        // 0x00, 0x40 => raw 8192 => centered value 0.
        let event = decode_midi_event(0, &[0xE0, 0x00, 0x40]).expect("valid pitch bend");
        assert_eq!(event.kind, MidiEventKind::PitchBend { value: 0 });
    }

    #[test]
    fn decodes_pitch_bend_minimum() {
        let event = decode_midi_event(0, &[0xE2, 0x00, 0x00]).expect("valid pitch bend");
        assert_eq!(event.kind, MidiEventKind::PitchBend { value: -8192 });
    }

    #[test]
    fn returns_none_for_system_realtime_bytes() {
        assert!(decode_midi_event(0, &[0xF8]).is_none());
    }

    #[test]
    fn returns_none_for_empty_message() {
        assert!(decode_midi_event(0, &[]).is_none());
    }

    #[test]
    fn returns_none_when_data_bytes_are_missing() {
        // Note-on status byte but no note/velocity bytes.
        assert!(decode_midi_event(0, &[0x90]).is_none());
    }

    #[test]
    fn new_adapter_has_no_live_connections() {
        let adapter = MidirMidiInput::new("crest-synth-test");
        assert!(adapter.connections.lock().unwrap().is_empty());
    }
}
