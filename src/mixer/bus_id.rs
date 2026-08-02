use core::fmt;
use serde::{Deserialize, Deserializer, Serialize};

/// Number of bus returns in the fixed bank addressed by [`BusId`].
pub const MAX_BUS_RETURNS: usize = 8;

/// Stable positional identity of one bus return in the fixed eight-return bank.
///
/// The identity is positional and independent of whichever effect currently
/// occupies the return, so changing a return's contents changes no `BusId`.
/// There are deliberately no named constructors: `BusId::reverb()` and
/// equivalents would re-encode a destination's contents in its identity, which
/// is exactly the fault the indexed bus topology removes (B-3).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct BusId(u8);

impl BusId {
    pub const COUNT: usize = MAX_BUS_RETURNS;
    pub const MIN: u8 = 0;
    pub const MAX: u8 = (MAX_BUS_RETURNS - 1) as u8;
    pub const ALL: [Self; Self::COUNT] = [
        Self(0),
        Self(1),
        Self(2),
        Self(3),
        Self(4),
        Self(5),
        Self(6),
        Self(7),
    ];

    /// Creates a bus identity without clamping, wrapping, or substitution.
    pub const fn new(value: u8) -> Result<Self, BusIdError> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(BusIdError { value })
        }
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl Default for BusId {
    fn default() -> Self {
        Self(Self::MIN)
    }
}

impl fmt::Display for BusId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "B{}", self.0)
    }
}

impl TryFrom<u8> for BusId {
    type Error = BusIdError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for BusId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl From<BusId> for u8 {
    fn from(value: BusId) -> Self {
        value.value()
    }
}

/// Rejection of one out-of-range bus identity (B-1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BusIdError {
    value: u8,
}

impl BusIdError {
    pub const fn value(self) -> u8 {
        self.value
    }
}

impl fmt::Display for BusIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "bus id must be in {}..={}, got {}",
            BusId::MIN,
            BusId::MAX,
            self.value
        )
    }
}

impl std::error::Error for BusIdError {}

#[cfg(test)]
mod tests {
    use super::{BusId, BusIdError, MAX_BUS_RETURNS};

    #[test]
    fn fixed_bank_is_exact_and_positional() {
        assert_eq!(MAX_BUS_RETURNS, 8);
        assert_eq!(BusId::ALL.len(), 8);
        for (index, bus) in BusId::ALL.into_iter().enumerate() {
            assert_eq!(bus.index(), index);
            assert_eq!(bus.value(), index as u8);
            assert_eq!(BusId::new(index as u8), Ok(bus));
        }
        assert_eq!(BusId::ALL[3].to_string(), "B3");
    }

    #[test]
    fn out_of_range_identity_is_rejected_without_clamping() {
        assert_eq!(BusId::new(8), Err(BusIdError { value: 8 }));
        assert_eq!(BusId::new(u8::MAX).unwrap_err().value(), u8::MAX);
        assert_eq!(
            BusId::new(8).unwrap_err().to_string(),
            "bus id must be in 0..=7, got 8"
        );
    }

    #[test]
    fn serde_round_trip_preserves_numeric_identity_and_rejects_invalid() {
        let bus = BusId::new(6).unwrap();
        assert_eq!(serde_json::to_string(&bus).unwrap(), "6");
        assert_eq!(serde_json::from_str::<BusId>("6").unwrap(), bus);
        assert!(serde_json::from_str::<BusId>("8").is_err());
    }
}
