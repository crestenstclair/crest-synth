use core::fmt;
use core::ops::RangeInclusive;
use serde::Serialize;

/// Identifies one editable value in a Patch's channel mix surface.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelParameter {
    GainDb,
    Pan,
    ReverbSend,
    DelaySend,
}

impl ChannelParameter {
    /// Returns the stable serialized and projected field name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::GainDb => "gainDb",
            Self::Pan => "pan",
            Self::ReverbSend => "reverbSend",
            Self::DelaySend => "delaySend",
        }
    }

    /// Returns this field's production-owned bounds and edit steps.
    pub const fn descriptor(self) -> &'static ChannelParameterDescriptor {
        match self {
            Self::GainDb => &CHANNEL_PARAMETER_SURFACE_DESCRIPTOR[0],
            Self::Pan => &CHANNEL_PARAMETER_SURFACE_DESCRIPTOR[1],
            Self::ReverbSend => &CHANNEL_PARAMETER_SURFACE_DESCRIPTOR[2],
            Self::DelaySend => &CHANNEL_PARAMETER_SURFACE_DESCRIPTOR[3],
        }
    }

    fn bounds(self) -> RangeInclusive<f32> {
        let descriptor = self.descriptor();
        descriptor.minimum()..=descriptor.maximum()
    }
}

impl fmt::Display for ChannelParameter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The independent pre-dispatch oracle for one Patch parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelParameterDescriptor {
    parameter: ChannelParameter,
    minimum: f32,
    maximum: f32,
    fine_step: f32,
    coarse_step: f32,
}

impl ChannelParameterDescriptor {
    const fn new(
        parameter: ChannelParameter,
        minimum: f32,
        maximum: f32,
        fine_step: f32,
        coarse_step: f32,
    ) -> Self {
        Self {
            parameter,
            minimum,
            maximum,
            fine_step,
            coarse_step,
        }
    }

    pub const fn parameter(&self) -> ChannelParameter {
        self.parameter
    }

    pub const fn name(&self) -> &'static str {
        self.parameter.name()
    }

    pub const fn minimum(&self) -> f32 {
        self.minimum
    }

    pub const fn maximum(&self) -> f32 {
        self.maximum
    }

    pub const fn fine_step(&self) -> f32 {
        self.fine_step
    }

    pub const fn coarse_step(&self) -> f32 {
        self.coarse_step
    }

    pub fn contains(&self, value: f32) -> bool {
        value.is_finite() && (self.minimum..=self.maximum).contains(&value)
    }
}

const CHANNEL_PARAMETER_SURFACE_DESCRIPTOR: [ChannelParameterDescriptor; 4] = [
    ChannelParameterDescriptor::new(ChannelParameter::GainDb, -60.0, 6.0, 1.0, 6.0),
    ChannelParameterDescriptor::new(ChannelParameter::Pan, -1.0, 1.0, 0.01, 0.1),
    ChannelParameterDescriptor::new(ChannelParameter::ReverbSend, 0.0, 1.0, 0.01, 0.1),
    ChannelParameterDescriptor::new(ChannelParameter::DelaySend, 0.0, 1.0, 0.01, 0.1),
];

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

    /// Returns each editable field exactly once in canonical projection order.
    pub const fn surface_descriptor() -> &'static [ChannelParameterDescriptor] {
        &CHANNEL_PARAMETER_SURFACE_DESCRIPTOR
    }

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

    /// Returns the current value of one typed Patch parameter.
    pub const fn value(&self, parameter: ChannelParameter) -> f32 {
        match parameter {
            ChannelParameter::GainDb => self.gain_db,
            ChannelParameter::Pan => self.pan,
            ChannelParameter::ReverbSend => self.reverb_send,
            ChannelParameter::DelaySend => self.delay_send,
        }
    }

    /// Replaces one field after validating it against the shared descriptor.
    pub fn with_value(
        mut self,
        parameter: ChannelParameter,
        value: f32,
    ) -> Result<Self, ChannelParametersError> {
        validate(parameter, value)?;
        match parameter {
            ChannelParameter::GainDb => self.gain_db = value,
            ChannelParameter::Pan => self.pan = value,
            ChannelParameter::ReverbSend => self.reverb_send = value,
            ChannelParameter::DelaySend => self.delay_send = value,
        }
        Ok(self)
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
