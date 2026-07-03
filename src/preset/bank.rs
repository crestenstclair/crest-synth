//! Bank aggregate: an ordered collection of presets, like a soundfont bank or user folder.

use std::fmt;

/// Unique identifier for a Bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BankId(u64);

impl BankId {
    /// Constructs a BankId from a raw value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for BankId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BankId({})", self.0)
    }
}

/// Unique identifier for a Preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PresetId(u64);

impl PresetId {
    /// Constructs a PresetId from a raw value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for PresetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PresetId({})", self.0)
    }
}

/// Commands that mutate a Bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankCommand {
    AddPreset { preset: PresetId },
    RemovePreset { preset: PresetId },
}

/// Events raised by a Bank in response to commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankEvent {
    PresetAdded { preset: PresetId },
    PresetRemoved { preset: PresetId },
}

/// Errors returned when a command cannot be applied to a Bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankError {
    /// The bank is read-only and rejects all mutating commands.
    ReadOnly,
    /// The preset is already present in the bank.
    DuplicatePreset(PresetId),
    /// The preset is not present in the bank.
    PresetNotFound(PresetId),
}

impl fmt::Display for BankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BankError::ReadOnly => write!(f, "bank is read-only and rejects mutating commands"),
            BankError::DuplicatePreset(id) => write!(f, "bank already contains preset {id}"),
            BankError::PresetNotFound(id) => write!(f, "bank does not contain preset {id}"),
        }
    }
}

impl std::error::Error for BankError {}

/// An ordered collection of presets, like a soundfont bank or user folder.
///
/// # Invariants
/// - a bank never contains the same preset twice
/// - a read-only bank rejects all mutating commands
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank {
    id: BankId,
    presets: Vec<PresetId>,
    read_only: bool,
}

impl Bank {
    /// Creates a new, empty, writable Bank with the given id.
    pub fn new(id: BankId) -> Self {
        Self {
            id,
            presets: Vec::new(),
            read_only: false,
        }
    }

    /// Reconstructs a Bank from its full persisted state.
    pub fn from_state(id: BankId, presets: Vec<PresetId>, read_only: bool) -> Self {
        Self {
            id,
            presets,
            read_only,
        }
    }

    pub fn id(&self) -> BankId {
        self.id
    }

    pub fn presets(&self) -> &[PresetId] {
        &self.presets
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn contains(&self, preset: PresetId) -> bool {
        self.presets.contains(&preset)
    }

    /// Validates a command against current state and returns the event it
    /// would raise, without mutating state.
    pub fn handle(&self, command: &BankCommand) -> Result<BankEvent, BankError> {
        if self.read_only {
            return Err(BankError::ReadOnly);
        }

        match command {
            BankCommand::AddPreset { preset } => {
                if self.contains(*preset) {
                    Err(BankError::DuplicatePreset(*preset))
                } else {
                    Ok(BankEvent::PresetAdded { preset: *preset })
                }
            }
            BankCommand::RemovePreset { preset } => {
                if self.contains(*preset) {
                    Ok(BankEvent::PresetRemoved { preset: *preset })
                } else {
                    Err(BankError::PresetNotFound(*preset))
                }
            }
        }
    }

    /// Applies a previously-validated event, mutating state.
    pub fn apply(&mut self, event: &BankEvent) {
        match event {
            BankEvent::PresetAdded { preset } => {
                if !self.presets.contains(preset) {
                    self.presets.push(*preset);
                }
            }
            BankEvent::PresetRemoved { preset } => {
                self.presets.retain(|p| p != preset);
            }
        }
    }

    /// Validates and applies a command in one step, returning the raised event.
    pub fn apply_command(&mut self, command: BankCommand) -> Result<BankEvent, BankError> {
        let event = self.handle(&command)?;
        self.apply(&event);
        Ok(event)
    }

    pub fn add_preset(&mut self, preset: PresetId) -> Result<BankEvent, BankError> {
        self.apply_command(BankCommand::AddPreset { preset })
    }

    pub fn remove_preset(&mut self, preset: PresetId) -> Result<BankEvent, BankError> {
        self.apply_command(BankCommand::RemovePreset { preset })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bank() -> Bank {
        Bank::new(BankId::new(1))
    }

    #[test]
    fn new_bank_is_empty_and_writable() {
        let b = bank();
        assert!(b.presets().is_empty());
        assert!(!b.read_only());
    }

    #[test]
    fn add_preset_succeeds_and_raises_event() {
        let mut b = bank();
        let p = PresetId::new(10);
        let event = b.add_preset(p).expect("add should succeed");
        assert_eq!(event, BankEvent::PresetAdded { preset: p });
        assert!(b.contains(p));
    }

    #[test]
    fn adding_same_preset_twice_is_rejected() {
        let mut b = bank();
        let p = PresetId::new(10);
        b.add_preset(p).unwrap();
        let err = b.add_preset(p).unwrap_err();
        assert_eq!(err, BankError::DuplicatePreset(p));
        assert_eq!(b.presets().len(), 1);
    }

    #[test]
    fn remove_preset_succeeds_and_raises_event() {
        let mut b = bank();
        let p = PresetId::new(10);
        b.add_preset(p).unwrap();
        let event = b.remove_preset(p).expect("remove should succeed");
        assert_eq!(event, BankEvent::PresetRemoved { preset: p });
        assert!(!b.contains(p));
    }

    #[test]
    fn removing_absent_preset_is_rejected() {
        let mut b = bank();
        let p = PresetId::new(10);
        let err = b.remove_preset(p).unwrap_err();
        assert_eq!(err, BankError::PresetNotFound(p));
    }

    #[test]
    fn read_only_bank_rejects_add() {
        let mut b = Bank::from_state(BankId::new(1), vec![], true);
        let err = b.add_preset(PresetId::new(5)).unwrap_err();
        assert_eq!(err, BankError::ReadOnly);
    }

    #[test]
    fn read_only_bank_rejects_remove() {
        let mut b = Bank::from_state(BankId::new(1), vec![PresetId::new(5)], true);
        let err = b.remove_preset(PresetId::new(5)).unwrap_err();
        assert_eq!(err, BankError::ReadOnly);
        assert!(b.contains(PresetId::new(5)));
    }

    #[test]
    fn presets_preserve_insertion_order() {
        let mut b = bank();
        let p1 = PresetId::new(1);
        let p2 = PresetId::new(2);
        let p3 = PresetId::new(3);
        b.add_preset(p1).unwrap();
        b.add_preset(p2).unwrap();
        b.add_preset(p3).unwrap();
        assert_eq!(b.presets(), &[p1, p2, p3]);
    }
}
