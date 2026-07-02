// path: src/shell/midi_port_info.rs

//! `MidiPortInfo` describes one connectable MIDI input port as reported by
//! the host MIDI subsystem (e.g. `midir`). It is a plain, immutable value
//! object: no I/O, no allocation beyond the `String` it owns, and no
//! interior mutability. Discovery of ports (which does perform I/O) lives
//! behind a separate port-listing abstraction in `shell::midi_input`; this
//! type only carries the resulting facts.

use std::fmt;

/// One connectable MIDI input port: its stable index within the host's
/// enumeration and its human-readable name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MidiPortInfo {
    index: u32,
    name: String,
}

impl MidiPortInfo {
    /// Construct a `MidiPortInfo` from an enumeration index and a name.
    ///
    /// The name is trimmed of leading/trailing whitespace. An empty (or
    /// all-whitespace) name is not a meaningful port identity, so
    /// construction fails in that case.
    pub fn try_new(index: u32, name: impl Into<String>) -> Result<Self, MidiPortInfoError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(MidiPortInfoError::EmptyName);
        }
        Ok(Self {
            index,
            name: trimmed.to_string(),
        })
    }

    /// The port's stable index within the host's MIDI port enumeration.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// The port's human-readable name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for MidiPortInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.index, self.name)
    }
}

/// Reasons construction of a `MidiPortInfo` can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiPortInfoError {
    /// The supplied name was empty or only whitespace.
    EmptyName,
}

impl fmt::Display for MidiPortInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiPortInfoError::EmptyName => write!(f, "MIDI port name must not be empty"),
        }
    }
}

impl std::error::Error for MidiPortInfoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_a_valid_name() {
        let port = MidiPortInfo::try_new(0, "IAC Driver Bus 1").expect("valid port");
        assert_eq!(port.index(), 0);
        assert_eq!(port.name(), "IAC Driver Bus 1");
    }

    #[test]
    fn try_new_trims_whitespace() {
        let port = MidiPortInfo::try_new(2, "  My Keyboard  ").expect("valid port");
        assert_eq!(port.name(), "My Keyboard");
    }

    #[test]
    fn try_new_rejects_empty_name() {
        let err = MidiPortInfo::try_new(1, "").unwrap_err();
        assert_eq!(err, MidiPortInfoError::EmptyName);
    }

    #[test]
    fn try_new_rejects_whitespace_only_name() {
        let err = MidiPortInfo::try_new(1, "   ").unwrap_err();
        assert_eq!(err, MidiPortInfoError::EmptyName);
    }

    #[test]
    fn equality_is_by_value() {
        let a = MidiPortInfo::try_new(3, "Port A").unwrap();
        let b = MidiPortInfo::try_new(3, "Port A").unwrap();
        let c = MidiPortInfo::try_new(4, "Port A").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn display_formats_index_and_name() {
        let port = MidiPortInfo::try_new(5, "USB MIDI").unwrap();
        assert_eq!(port.to_string(), "[5] USB MIDI");
    }

    #[test]
    fn error_display_is_descriptive() {
        let err = MidiPortInfoError::EmptyName;
        assert_eq!(err.to_string(), "MIDI port name must not be empty");
    }
}
