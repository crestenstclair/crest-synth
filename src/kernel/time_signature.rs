// path: src/kernel/time_signature.rs

//! `TimeSignature` — a musical time signature such as 4/4 or 6/8.
//!
//! Both the numerator and denominator must be positive (non-zero). This is a
//! plain value type: it holds no dependencies and performs no I/O, so it
//! needs no dependency injection per the project's SOLID conventions.

use std::error::Error;
use std::fmt;

/// A musical time signature, e.g. 4/4 or 6/8.
///
/// Construct via [`TimeSignature::try_new`], which enforces that both the
/// numerator and denominator are positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSignature {
    numerator: u8,
    denominator: u8,
}

/// Error returned when constructing a [`TimeSignature`] with an invalid
/// numerator or denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSignatureError {
    /// The numerator was zero; numerators must be positive.
    ZeroNumerator,
    /// The denominator was zero; denominators must be positive.
    ZeroDenominator,
}

impl fmt::Display for TimeSignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeSignatureError::ZeroNumerator => {
                write!(f, "time signature numerator must be positive, got 0")
            }
            TimeSignatureError::ZeroDenominator => {
                write!(f, "time signature denominator must be positive, got 0")
            }
        }
    }
}

impl Error for TimeSignatureError {}

impl TimeSignature {
    /// Constructs a [`TimeSignature`], validating that both the numerator and
    /// denominator are positive.
    pub fn try_new(numerator: u8, denominator: u8) -> Result<Self, TimeSignatureError> {
        if numerator == 0 {
            return Err(TimeSignatureError::ZeroNumerator);
        }
        if denominator == 0 {
            return Err(TimeSignatureError::ZeroDenominator);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// The common 4/4 ("common time") time signature.
    pub fn common_time() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }

    /// The numerator (beats per measure).
    pub fn numerator(&self) -> u8 {
        self.numerator
    }

    /// The denominator (note value that receives one beat).
    pub fn denominator(&self) -> u8 {
        self.denominator
    }
}

impl fmt::Display for TimeSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self::common_time()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_positive_values() {
        let ts = TimeSignature::try_new(4, 4).expect("4/4 is valid");
        assert_eq!(ts.numerator(), 4);
        assert_eq!(ts.denominator(), 4);
    }

    #[test]
    fn try_new_accepts_six_eight() {
        let ts = TimeSignature::try_new(6, 8).expect("6/8 is valid");
        assert_eq!(ts.numerator(), 6);
        assert_eq!(ts.denominator(), 8);
    }

    #[test]
    fn try_new_rejects_zero_numerator() {
        let err = TimeSignature::try_new(0, 4).expect_err("zero numerator must be rejected");
        assert_eq!(err, TimeSignatureError::ZeroNumerator);
    }

    #[test]
    fn try_new_rejects_zero_denominator() {
        let err = TimeSignature::try_new(4, 0).expect_err("zero denominator must be rejected");
        assert_eq!(err, TimeSignatureError::ZeroDenominator);
    }

    #[test]
    fn try_new_rejects_both_zero() {
        assert!(TimeSignature::try_new(0, 0).is_err());
    }

    #[test]
    fn common_time_is_four_four() {
        let ts = TimeSignature::common_time();
        assert_eq!(ts.numerator(), 4);
        assert_eq!(ts.denominator(), 4);
    }

    #[test]
    fn default_matches_common_time() {
        assert_eq!(TimeSignature::default(), TimeSignature::common_time());
    }

    #[test]
    fn display_formats_as_fraction() {
        let ts = TimeSignature::try_new(6, 8).unwrap();
        assert_eq!(ts.to_string(), "6/8");
    }

    #[test]
    fn error_messages_are_descriptive() {
        assert_eq!(
            TimeSignatureError::ZeroNumerator.to_string(),
            "time signature numerator must be positive, got 0"
        );
        assert_eq!(
            TimeSignatureError::ZeroDenominator.to_string(),
            "time signature denominator must be positive, got 0"
        );
    }
}
