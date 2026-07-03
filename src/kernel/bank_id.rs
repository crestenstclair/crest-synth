// path: src/kernel/bank_id.rs

//! `BankId` identifies a Bank of presets within the Kernel context.
//!
//! This is a plain newtype over `u32`. It carries no behavior beyond
//! identity, ordering, and (de)serialization of the raw value, so a
//! constructor/getter pair is sufficient — there is no invariant to
//! enforce beyond "is a u32".

/// Identifies a Bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BankId(u32);

impl BankId {
    /// Constructs a `BankId` from a raw `u32` value.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw `u32` value.
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl From<u32> for BankId {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<BankId> for u32 {
    fn from(id: BankId) -> Self {
        id.0
    }
}

impl std::fmt::Display for BankId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_the_raw_value() {
        let id = BankId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn from_u32_round_trips() {
        let id: BankId = 7u32.into();
        let raw: u32 = id.into();
        assert_eq!(raw, 7);
    }

    #[test]
    fn equality_is_by_value() {
        assert_eq!(BankId::new(3), BankId::new(3));
        assert_ne!(BankId::new(3), BankId::new(4));
    }

    #[test]
    fn ordering_matches_underlying_value() {
        assert!(BankId::new(1) < BankId::new(2));
        assert!(BankId::new(5) > BankId::new(2));
    }

    #[test]
    fn display_shows_raw_value() {
        let id = BankId::new(99);
        assert_eq!(format!("{}", id), "99");
    }
}
