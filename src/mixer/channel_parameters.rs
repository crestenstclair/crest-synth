use core::fmt;
use core::ops::RangeInclusive;

/// Identifies one editable value in a Patch's channel mix surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelParameter {
    GainDb,
    Pan,
    ReverbSend,
    DelaySend,
}

impl ChannelParameter {
    fn bounds(self) -> RangeInclusive<f32> {
        match self {
            Self::GainDb => ChannelParameters::MIN_GAIN_DB..=ChannelParameters::MAX_GAIN_DB,
            Self::Pan => ChannelParameters::MIN_PAN..=ChannelParameters::MAX_PAN,
            Self::ReverbSend | Self::DelaySend => {
                ChannelParameters::MIN_SEND..=ChannelParameters::MAX_SEND
            }
        }
    }
}

impl fmt::Display for ChannelParameter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::GainDb => "gainDb",
            Self::Pan => "pan",
            Self::ReverbSend => "reverbSend",
            Self::DelaySend => "delaySend",
        })
    }
}

/// The reason a set of channel parameters could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelParametersError {
    NonFinite {
        parameter: ChannelParameter,
    },
    OutOfRange {
        parameter: ChannelParameter,
        value: f32,
    },
}

impl fmt::Display for ChannelParametersError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NonFinite { parameter } => {
                write!(formatter, "{parameter} must be finite")
            }
            Self::OutOfRange { parameter, value } => {
                let bounds = parameter.bounds();
                write!(
                    formatter,
                    "{parameter} must be in {}..={}, got {value}",
                    bounds.start(),
                    bounds.end()
                )
            }
        }
    }
}

impl std::error::Error for ChannelParametersError {}

/// All editable mixer parameters owned by one Patch.
///
/// Construction validates every field so readers on the real-time path can use
/// the value without further checking, allocation, locking, or error handling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelParameters {
    gain_db: f32,
    pan: f32,
    reverb_send: f32,
    delay_send: f32,
}

impl ChannelParameters {
    pub const MIN_GAIN_DB: f32 = -60.0;
    pub const MAX_GAIN_DB: f32 = 6.0;
    pub const MIN_PAN: f32 = -1.0;
    pub const MAX_PAN: f32 = 1.0;
    pub const MIN_SEND: f32 = 0.0;
    pub const MAX_SEND: f32 = 1.0;

    /// Creates a complete channel parameter value when all fields are finite
    /// and within their declared inclusive ranges.
    pub fn new(
        gain_db: f32,
        pan: f32,
        reverb_send: f32,
        delay_send: f32,
    ) -> Result<Self, ChannelParametersError> {
        validate(ChannelParameter::GainDb, gain_db)?;
        validate(ChannelParameter::Pan, pan)?;
        validate(ChannelParameter::ReverbSend, reverb_send)?;
        validate(ChannelParameter::DelaySend, delay_send)?;

        Ok(Self {
            gain_db,
            pan,
            reverb_send,
            delay_send,
        })
    }

    pub const fn gain_db(&self) -> f32 {
        self.gain_db
    }

    pub const fn pan(&self) -> f32 {
        self.pan
    }

    pub const fn reverb_send(&self) -> f32 {
        self.reverb_send
    }

    pub const fn delay_send(&self) -> f32 {
        self.delay_send
    }
}

impl Default for ChannelParameters {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            pan: 0.0,
            reverb_send: 0.0,
            delay_send: 0.0,
        }
    }
}

fn validate(parameter: ChannelParameter, value: f32) -> Result<(), ChannelParametersError> {
    if !value.is_finite() {
        return Err(ChannelParametersError::NonFinite { parameter });
    }
    if !parameter.bounds().contains(&value) {
        return Err(ChannelParametersError::OutOfRange { parameter, value });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ChannelParameter, ChannelParameters, ChannelParametersError};

    #[test]
    fn accepts_every_inclusive_boundary() {
        let minimums = ChannelParameters::new(-60.0, -1.0, 0.0, 0.0).unwrap();
        let maximums = ChannelParameters::new(6.0, 1.0, 1.0, 1.0).unwrap();

        assert_eq!(minimums.gain_db(), -60.0);
        assert_eq!(minimums.pan(), -1.0);
        assert_eq!(minimums.reverb_send(), 0.0);
        assert_eq!(minimums.delay_send(), 0.0);
        assert_eq!(maximums.gain_db(), 6.0);
        assert_eq!(maximums.pan(), 1.0);
        assert_eq!(maximums.reverb_send(), 1.0);
        assert_eq!(maximums.delay_send(), 1.0);
    }

    #[test]
    fn rejects_each_value_outside_its_range() {
        assert!(matches!(
            ChannelParameters::new(-60.1, 0.0, 0.0, 0.0),
            Err(ChannelParametersError::OutOfRange {
                parameter: ChannelParameter::GainDb,
                ..
            })
        ));
        assert!(matches!(
            ChannelParameters::new(0.0, 1.1, 0.0, 0.0),
            Err(ChannelParametersError::OutOfRange {
                parameter: ChannelParameter::Pan,
                ..
            })
        ));
        assert!(matches!(
            ChannelParameters::new(0.0, 0.0, -0.1, 0.0),
            Err(ChannelParametersError::OutOfRange {
                parameter: ChannelParameter::ReverbSend,
                ..
            })
        ));
        assert!(matches!(
            ChannelParameters::new(0.0, 0.0, 0.0, 1.1),
            Err(ChannelParametersError::OutOfRange {
                parameter: ChannelParameter::DelaySend,
                ..
            })
        ));
    }

    #[test]
    fn rejects_non_finite_values_before_range_validation() {
        let cases = [
            ChannelParameters::new(f32::NAN, 0.0, 0.0, 0.0),
            ChannelParameters::new(0.0, f32::INFINITY, 0.0, 0.0),
            ChannelParameters::new(0.0, 0.0, f32::NEG_INFINITY, 0.0),
            ChannelParameters::new(0.0, 0.0, 0.0, f32::NAN),
        ];

        assert!(cases
            .iter()
            .all(|result| matches!(result, Err(ChannelParametersError::NonFinite { .. }))));
    }

    #[test]
    fn default_is_a_valid_neutral_mix_value() {
        let default = ChannelParameters::default();

        assert_eq!(
            ChannelParameters::new(
                default.gain_db(),
                default.pan(),
                default.reverb_send(),
                default.delay_send(),
            ),
            Ok(default)
        );
    }

    #[test]
    fn validation_errors_name_the_invalid_parameter_and_constraint() {
        assert_eq!(
            ChannelParameters::new(0.0, 0.0, f32::NAN, 0.0)
                .unwrap_err()
                .to_string(),
            "reverbSend must be finite"
        );
        assert_eq!(
            ChannelParameters::new(0.0, 2.0, 0.0, 0.0)
                .unwrap_err()
                .to_string(),
            "pan must be in -1..=1, got 2"
        );
    }
}
