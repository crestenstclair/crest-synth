use core::fmt;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

/// Stable non-zero Patch-local identity for one ordered post-effect instance.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct EffectSlotId(u16);

impl EffectSlotId {
    pub const MIN: u16 = 1;

    pub const fn new(value: u16) -> Result<Self, EffectSlotIdError> {
        if value == 0 {
            Err(EffectSlotIdError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl fmt::Display for EffectSlotId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for EffectSlotId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u16::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectSlotIdError {
    Zero,
}

impl fmt::Display for EffectSlotIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("effect slot id must be non-zero")
    }
}

impl std::error::Error for EffectSlotIdError {}
