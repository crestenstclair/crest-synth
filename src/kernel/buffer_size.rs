//! `BufferSize` — frames per audio callback.
//!
//! A small newtype around `u32` that guarantees the wrapped value is
//! positive. The audio callback receives a fixed number of frames per
//! invocation; a buffer size of zero is meaningless and would make every
//! downstream frame-count calculation (block iteration, ring buffer
//! capacity, etc.) degenerate.

use std::fmt;

/// Number of frames processed per audio callback.
///
/// Invariant: the wrapped value is always positive (never zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferSize(u32);

/// Error returned when constructing a `BufferSize` from an invalid value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferSizeError {
    /// The supplied value was zero, which is not a valid frame count.
    Zero,
}

impl fmt::Display for BufferSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufferSizeError::Zero => write!(f, "buffer size must be positive (got 0)"),
        }
    }
}

impl std::error::Error for BufferSizeError {}

impl BufferSize {
    /// Attempts to construct a `BufferSize` from a raw frame count.
    ///
    /// Returns `Err(BufferSizeError::Zero)` if `frames` is zero.
    ///
    /// ```
    /// use crest_synth::kernel::buffer_size::BufferSize;
    ///
    /// assert!(BufferSize::try_new(256).is_ok());
    /// assert!(BufferSize::try_new(0).is_err());
    /// ```
    pub fn try_new(frames: u32) -> Result<Self, BufferSizeError> {
        if frames == 0 {
            Err(BufferSizeError::Zero)
        } else {
            Ok(Self(frames))
        }
    }

    /// Returns the number of frames per callback as a raw `u32`.
    pub fn frames(&self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for BufferSize {
    type Error = BufferSizeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<BufferSize> for u32 {
    fn from(value: BufferSize) -> Self {
        value.0
    }
}

impl fmt::Display for BufferSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_positive_values() {
        let size = BufferSize::try_new(128).expect("128 is positive");
        assert_eq!(size.frames(), 128);
    }

    #[test]
    fn try_new_rejects_zero() {
        let result = BufferSize::try_new(0);
        assert_eq!(result, Err(BufferSizeError::Zero));
    }

    #[test]
    fn try_from_u32_matches_try_new() {
        let size: Result<BufferSize, _> = BufferSize::try_from(512);
        assert_eq!(size.expect("512 is positive").frames(), 512);

        let err: Result<BufferSize, _> = BufferSize::try_from(0);
        assert!(err.is_err());
    }

    #[test]
    fn into_u32_round_trips() {
        let size = BufferSize::try_new(64).expect("64 is positive");
        let raw: u32 = size.into();
        assert_eq!(raw, 64);
    }

    #[test]
    fn display_shows_raw_value() {
        let size = BufferSize::try_new(1024).expect("1024 is positive");
        assert_eq!(size.to_string(), "1024");
    }

    #[test]
    fn ordering_and_equality_are_derived_from_value() {
        let small = BufferSize::try_new(64).expect("64 is positive");
        let large = BufferSize::try_new(256).expect("256 is positive");
        assert!(small < large);
        assert_eq!(small, BufferSize::try_new(64).expect("64 is positive"));
    }

    #[test]
    fn error_message_is_descriptive() {
        let err = BufferSize::try_new(0).expect_err("0 is invalid");
        assert_eq!(err.to_string(), "buffer size must be positive (got 0)");
    }
}
