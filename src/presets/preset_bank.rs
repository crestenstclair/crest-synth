// path: src/presets/preset_bank.rs

use crate::presets::preset_id::PresetId;

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Command to create a new preset bank with the given name.
pub struct CreateBank {
    pub name: String,
}

/// Command to add a preset to the bank.
pub struct AddPresetToBank {
    pub preset_id: PresetId,
}

/// Command to remove a preset from the bank.
pub struct RemovePresetFromBank {
    pub preset_id: PresetId,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Event emitted when a bank is successfully created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankCreated {
    pub name: String,
}

/// Event emitted when a preset is added to a bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetAddedToBank {
    pub preset_id: PresetId,
}

/// Event emitted when a preset is removed from a bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetRemovedFromBank {
    pub preset_id: PresetId,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur when applying commands to a `PresetBank`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetBankError {
    /// Factory banks are read-only; user cannot modify them.
    FactoryBankIsReadOnly,
    /// The preset is already in this bank.
    PresetAlreadyInBank,
    /// The preset is not in this bank.
    PresetNotInBank,
}

impl std::fmt::Display for PresetBankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresetBankError::FactoryBankIsReadOnly => {
                write!(f, "factory banks are read-only and cannot be modified")
            }
            PresetBankError::PresetAlreadyInBank => {
                write!(f, "preset is already in this bank")
            }
            PresetBankError::PresetNotInBank => {
                write!(f, "preset is not in this bank")
            }
        }
    }
}

impl std::error::Error for PresetBankError {}

// ---------------------------------------------------------------------------
// Aggregate
// ---------------------------------------------------------------------------

/// A named collection of presets for organized browsing.
///
/// `PresetBank` is a DDD aggregate root. All state changes are expressed as
/// commands and produce events. Factory banks (`is_factory = true`) are
/// read-only — any attempt to add or remove presets will return
/// [`PresetBankError::FactoryBankIsReadOnly`].
///
/// # Examples
///
/// ```
/// use crest_synth::presets::preset_bank::{PresetBank, CreateBank, AddPresetToBank};
/// use crest_synth::presets::preset_id::PresetId;
///
/// let mut bank = PresetBank::create(CreateBank { name: "My Bank".to_string() }).unwrap().0;
/// let result = bank.add_preset(AddPresetToBank { preset_id: PresetId::new("pad-001") });
/// assert!(result.is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct PresetBank {
    /// Whether this is a factory (read-only) bank.
    pub is_factory: bool,
    /// Display name of the bank.
    pub name: String,
    /// Ordered list of preset IDs in this bank.
    pub preset_ids: Vec<PresetId>,
}

impl PresetBank {
    // -----------------------------------------------------------------------
    // Command handlers
    // -----------------------------------------------------------------------

    /// Creates a new user-owned `PresetBank` and returns it alongside the
    /// [`BankCreated`] event.
    pub fn create(cmd: CreateBank) -> Result<(Self, BankCreated), PresetBankError> {
        let event = BankCreated {
            name: cmd.name.clone(),
        };
        let bank = Self {
            is_factory: false,
            name: cmd.name,
            preset_ids: Vec::new(),
        };
        Ok((bank, event))
    }

    /// Creates a factory (read-only) `PresetBank` directly — bypasses the
    /// normal command path because factory banks are populated at startup, not
    /// by user commands.
    pub fn create_factory(name: impl Into<String>, preset_ids: Vec<PresetId>) -> Self {
        Self {
            is_factory: true,
            name: name.into(),
            preset_ids,
        }
    }

    /// Handles the [`AddPresetToBank`] command.
    ///
    /// Returns an error if this is a factory bank (read-only) or if the preset
    /// is already present.
    pub fn add_preset(
        &mut self,
        cmd: AddPresetToBank,
    ) -> Result<PresetAddedToBank, PresetBankError> {
        if self.is_factory {
            return Err(PresetBankError::FactoryBankIsReadOnly);
        }
        if self.preset_ids.contains(&cmd.preset_id) {
            return Err(PresetBankError::PresetAlreadyInBank);
        }
        let event = PresetAddedToBank {
            preset_id: cmd.preset_id.clone(),
        };
        self.preset_ids.push(cmd.preset_id);
        Ok(event)
    }

    /// Handles the [`RemovePresetFromBank`] command.
    ///
    /// Returns an error if this is a factory bank (read-only) or if the preset
    /// is not present.
    pub fn remove_preset(
        &mut self,
        cmd: RemovePresetFromBank,
    ) -> Result<PresetRemovedFromBank, PresetBankError> {
        if self.is_factory {
            return Err(PresetBankError::FactoryBankIsReadOnly);
        }
        let pos = self
            .preset_ids
            .iter()
            .position(|id| id == &cmd.preset_id)
            .ok_or(PresetBankError::PresetNotInBank)?;
        self.preset_ids.remove(pos);
        Ok(PresetRemovedFromBank {
            preset_id: cmd.preset_id,
        })
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Returns `true` if the bank contains the given preset.
    pub fn contains(&self, preset_id: &PresetId) -> bool {
        self.preset_ids.contains(preset_id)
    }

    /// Returns the number of presets in this bank.
    pub fn len(&self) -> usize {
        self.preset_ids.len()
    }

    /// Returns `true` if the bank has no presets.
    pub fn is_empty(&self) -> bool {
        self.preset_ids.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod preset_bank_tests {
    use super::*;

    fn make_user_bank() -> PresetBank {
        PresetBank::create(CreateBank {
            name: "Test Bank".to_string(),
        })
        .unwrap()
        .0
    }

    fn make_factory_bank() -> PresetBank {
        PresetBank::create_factory(
            "Factory Pads",
            vec![PresetId::new("pad-001"), PresetId::new("pad-002")],
        )
    }

    // -----------------------------------------------------------------------
    // CreateBank
    // -----------------------------------------------------------------------

    #[test]
    fn create_bank_returns_bank_and_event() {
        let (bank, event) = PresetBank::create(CreateBank {
            name: "My Bank".to_string(),
        })
        .unwrap();
        assert_eq!(bank.name, "My Bank");
        assert!(!bank.is_factory);
        assert!(bank.preset_ids.is_empty());
        assert_eq!(event.name, "My Bank");
    }

    // -----------------------------------------------------------------------
    // AddPresetToBank — happy path
    // -----------------------------------------------------------------------

    #[test]
    fn add_preset_to_user_bank_succeeds() {
        let mut bank = make_user_bank();
        let id = PresetId::new("pad-010");
        let event = bank
            .add_preset(AddPresetToBank {
                preset_id: id.clone(),
            })
            .unwrap();
        assert_eq!(event.preset_id, id);
        assert!(bank.contains(&id));
        assert_eq!(bank.len(), 1);
    }

    #[test]
    fn add_multiple_presets_preserves_order() {
        let mut bank = make_user_bank();
        let ids = [PresetId::new("a"), PresetId::new("b"), PresetId::new("c")];
        for id in &ids {
            bank.add_preset(AddPresetToBank {
                preset_id: id.clone(),
            })
            .unwrap();
        }
        assert_eq!(bank.preset_ids, ids.to_vec());
    }

    // -----------------------------------------------------------------------
    // AddPresetToBank — invariant: factory bank is read-only
    // -----------------------------------------------------------------------

    #[test]
    fn add_preset_to_factory_bank_is_rejected() {
        let mut bank = make_factory_bank();
        let err = bank
            .add_preset(AddPresetToBank {
                preset_id: PresetId::new("new-preset"),
            })
            .unwrap_err();
        assert_eq!(err, PresetBankError::FactoryBankIsReadOnly);
    }

    // -----------------------------------------------------------------------
    // AddPresetToBank — duplicate guard
    // -----------------------------------------------------------------------

    #[test]
    fn add_duplicate_preset_is_rejected() {
        let mut bank = make_user_bank();
        let id = PresetId::new("bass-001");
        bank.add_preset(AddPresetToBank {
            preset_id: id.clone(),
        })
        .unwrap();
        let err = bank
            .add_preset(AddPresetToBank {
                preset_id: id.clone(),
            })
            .unwrap_err();
        assert_eq!(err, PresetBankError::PresetAlreadyInBank);
    }

    // -----------------------------------------------------------------------
    // RemovePresetFromBank — happy path
    // -----------------------------------------------------------------------

    #[test]
    fn remove_preset_from_user_bank_succeeds() {
        let mut bank = make_user_bank();
        let id = PresetId::new("lead-007");
        bank.add_preset(AddPresetToBank {
            preset_id: id.clone(),
        })
        .unwrap();
        let event = bank
            .remove_preset(RemovePresetFromBank {
                preset_id: id.clone(),
            })
            .unwrap();
        assert_eq!(event.preset_id, id);
        assert!(!bank.contains(&id));
        assert!(bank.is_empty());
    }

    #[test]
    fn remove_preserves_order_of_remaining_presets() {
        let mut bank = make_user_bank();
        let a = PresetId::new("a");
        let b = PresetId::new("b");
        let c = PresetId::new("c");
        for id in [&a, &b, &c] {
            bank.add_preset(AddPresetToBank {
                preset_id: id.clone(),
            })
            .unwrap();
        }
        bank.remove_preset(RemovePresetFromBank { preset_id: b })
            .unwrap();
        assert_eq!(bank.preset_ids, vec![a, c]);
    }

    // -----------------------------------------------------------------------
    // RemovePresetFromBank — invariant: factory bank is read-only
    // -----------------------------------------------------------------------

    #[test]
    fn remove_preset_from_factory_bank_is_rejected() {
        let mut bank = make_factory_bank();
        let err = bank
            .remove_preset(RemovePresetFromBank {
                preset_id: PresetId::new("pad-001"),
            })
            .unwrap_err();
        assert_eq!(err, PresetBankError::FactoryBankIsReadOnly);
    }

    // -----------------------------------------------------------------------
    // RemovePresetFromBank — not found
    // -----------------------------------------------------------------------

    #[test]
    fn remove_absent_preset_is_rejected() {
        let mut bank = make_user_bank();
        let err = bank
            .remove_preset(RemovePresetFromBank {
                preset_id: PresetId::new("nonexistent"),
            })
            .unwrap_err();
        assert_eq!(err, PresetBankError::PresetNotInBank);
    }

    // -----------------------------------------------------------------------
    // Factory bank creation
    // -----------------------------------------------------------------------

    #[test]
    fn factory_bank_is_read_only_flag() {
        let bank = make_factory_bank();
        assert!(bank.is_factory);
        assert_eq!(bank.name, "Factory Pads");
        assert_eq!(bank.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    #[test]
    fn contains_returns_true_for_added_preset() {
        let mut bank = make_user_bank();
        let id = PresetId::new("synth-99");
        assert!(!bank.contains(&id));
        bank.add_preset(AddPresetToBank {
            preset_id: id.clone(),
        })
        .unwrap();
        assert!(bank.contains(&id));
    }

    #[test]
    fn is_empty_on_new_bank() {
        let bank = make_user_bank();
        assert!(bank.is_empty());
        assert_eq!(bank.len(), 0);
    }
}
