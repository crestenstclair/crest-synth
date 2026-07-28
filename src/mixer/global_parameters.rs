use core::fmt;
use serde::{Deserialize, Serialize};

/// Identifies one editable value in the shared global mix surface.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GlobalParameter {
    MasterGainDb,
    ReverbRoomSize,
    ReverbDamping,
    ReverbReturn,
    DelayMilliseconds,
    DelayFeedback,
    DelayReturn,
}

impl GlobalParameter {
    /// Returns the stable serialized and projected field name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::MasterGainDb => "masterGainDb",
            Self::ReverbRoomSize => "reverbRoomSize",
            Self::ReverbDamping => "reverbDamping",
            Self::ReverbReturn => "reverbReturn",
            Self::DelayMilliseconds => "delayMilliseconds",
            Self::DelayFeedback => "delayFeedback",
            Self::DelayReturn => "delayReturn",
        }
    }

    /// Returns this field's production-owned bounds and edit steps.
    pub const fn descriptor(self) -> &'static GlobalParameterDescriptor {
        match self {
            Self::MasterGainDb => &GLOBAL_PARAMETER_SURFACE_DESCRIPTOR[0],
            Self::ReverbRoomSize => &GLOBAL_PARAMETER_SURFACE_DESCRIPTOR[1],
            Self::ReverbDamping => &GLOBAL_PARAMETER_SURFACE_DESCRIPTOR[2],
            Self::ReverbReturn => &GLOBAL_PARAMETER_SURFACE_DESCRIPTOR[3],
            Self::DelayMilliseconds => &GLOBAL_PARAMETER_SURFACE_DESCRIPTOR[4],
            Self::DelayFeedback => &GLOBAL_PARAMETER_SURFACE_DESCRIPTOR[5],
            Self::DelayReturn => &GLOBAL_PARAMETER_SURFACE_DESCRIPTOR[6],
        }
    }

    const fn invalid_error(self) -> GlobalParametersError {
        match self {
            Self::MasterGainDb => GlobalParametersError::InvalidMasterGainDb,
            Self::ReverbRoomSize => GlobalParametersError::InvalidReverbRoomSize,
            Self::ReverbDamping => GlobalParametersError::InvalidReverbDamping,
            Self::ReverbReturn => GlobalParametersError::InvalidReverbReturn,
            Self::DelayMilliseconds => GlobalParametersError::InvalidDelayMilliseconds,
            Self::DelayFeedback => GlobalParametersError::InvalidDelayFeedback,
            Self::DelayReturn => GlobalParametersError::InvalidDelayReturn,
        }
    }
}

impl fmt::Display for GlobalParameter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The independent pre-dispatch oracle for one global parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalParameterDescriptor {
    parameter: GlobalParameter,
    minimum: f32,
    maximum: f32,
    fine_step: f32,
    coarse_step: f32,
}

impl GlobalParameterDescriptor {
    const fn new(
        parameter: GlobalParameter,
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

    pub const fn parameter(&self) -> GlobalParameter {
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

const GLOBAL_PARAMETER_SURFACE_DESCRIPTOR: [GlobalParameterDescriptor; 7] = [
    GlobalParameterDescriptor::new(GlobalParameter::MasterGainDb, -60.0, 6.0, 1.0, 6.0),
    GlobalParameterDescriptor::new(GlobalParameter::ReverbRoomSize, 0.0, 1.0, 0.01, 0.1),
    GlobalParameterDescriptor::new(GlobalParameter::ReverbDamping, 0.0, 1.0, 0.01, 0.1),
    GlobalParameterDescriptor::new(GlobalParameter::ReverbReturn, 0.0, 1.0, 0.01, 0.1),
    GlobalParameterDescriptor::new(GlobalParameter::DelayMilliseconds, 1.0, 2000.0, 1.0, 100.0),
    GlobalParameterDescriptor::new(GlobalParameter::DelayFeedback, 0.0, 1.0, 0.01, 0.1),
    GlobalParameterDescriptor::new(GlobalParameter::DelayReturn, 0.0, 1.0, 0.01, 0.1),
];

/// A violation of one bounded global mixer parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobalParametersError {
    /// masterGainDb is not finite or is outside -60.0..=6.0.
    InvalidMasterGainDb,
    /// reverbRoomSize is not finite or is outside 0.0..=1.0.
    InvalidReverbRoomSize,
    /// reverbDamping is not finite or is outside 0.0..=1.0.
    InvalidReverbDamping,
    /// reverbReturn is not finite or is outside 0.0..=1.0.
    InvalidReverbReturn,
    /// delayMilliseconds is not finite or is outside 1.0..=2000.0.
    InvalidDelayMilliseconds,
    /// delayFeedback is not finite or is outside 0.0..=1.0.
    InvalidDelayFeedback,
    /// delayReturn is not finite or is outside 0.0..=1.0.
    InvalidDelayReturn,
}

impl fmt::Display for GlobalParametersError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidMasterGainDb => "master gain must be finite and in -60.0..=6.0 dB",
            Self::InvalidReverbRoomSize => "reverb room size must be finite and in 0.0..=1.0",
            Self::InvalidReverbDamping => "reverb damping must be finite and in 0.0..=1.0",
            Self::InvalidReverbReturn => "reverb return must be finite and in 0.0..=1.0",
            Self::InvalidDelayMilliseconds => {
                "delay time must be finite and in 1.0..=2000.0 milliseconds"
            }
            Self::InvalidDelayFeedback => "delay feedback must be finite and in 0.0..=1.0",
            Self::InvalidDelayReturn => "delay return must be finite and in 0.0..=1.0",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for GlobalParametersError {}

/// All editable parameters shared by the complete mix.
///
/// Construction validates the complete value so audio and projection consumers
/// can use every getter without repeating bounds checks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalParameters {
    master_gain_db: f32,
    reverb_room_size: f32,
    reverb_damping: f32,
    reverb_return: f32,
    delay_milliseconds: f32,
    delay_feedback: f32,
    delay_return: f32,
}

impl GlobalParameters {
    /// Creates one complete, validated global mixer parameter value.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        master_gain_db: f32,
        reverb_room_size: f32,
        reverb_damping: f32,
        reverb_return: f32,
        delay_milliseconds: f32,
        delay_feedback: f32,
        delay_return: f32,
    ) -> Result<Self, GlobalParametersError> {
        for (parameter, value) in [
            (GlobalParameter::MasterGainDb, master_gain_db),
            (GlobalParameter::ReverbRoomSize, reverb_room_size),
            (GlobalParameter::ReverbDamping, reverb_damping),
            (GlobalParameter::ReverbReturn, reverb_return),
            (GlobalParameter::DelayMilliseconds, delay_milliseconds),
            (GlobalParameter::DelayFeedback, delay_feedback),
            (GlobalParameter::DelayReturn, delay_return),
        ] {
            validate(parameter, value)?;
        }

        Ok(Self {
            master_gain_db,
            reverb_room_size,
            reverb_damping,
            reverb_return,
            delay_milliseconds,
            delay_feedback,
            delay_return,
        })
    }

    /// Returns each editable field exactly once in canonical projection order.
    pub const fn surface_descriptor() -> &'static [GlobalParameterDescriptor] {
        &GLOBAL_PARAMETER_SURFACE_DESCRIPTOR
    }

    /// Returns the current value of one typed global parameter.
    pub const fn value(&self, parameter: GlobalParameter) -> f32 {
        match parameter {
            GlobalParameter::MasterGainDb => self.master_gain_db,
            GlobalParameter::ReverbRoomSize => self.reverb_room_size,
            GlobalParameter::ReverbDamping => self.reverb_damping,
            GlobalParameter::ReverbReturn => self.reverb_return,
            GlobalParameter::DelayMilliseconds => self.delay_milliseconds,
            GlobalParameter::DelayFeedback => self.delay_feedback,
            GlobalParameter::DelayReturn => self.delay_return,
        }
    }

    /// Replaces one field after validating it against the shared descriptor.
    pub fn with_value(
        mut self,
        parameter: GlobalParameter,
        value: f32,
    ) -> Result<Self, GlobalParametersError> {
        validate(parameter, value)?;
        match parameter {
            GlobalParameter::MasterGainDb => self.master_gain_db = value,
            GlobalParameter::ReverbRoomSize => self.reverb_room_size = value,
            GlobalParameter::ReverbDamping => self.reverb_damping = value,
            GlobalParameter::ReverbReturn => self.reverb_return = value,
            GlobalParameter::DelayMilliseconds => self.delay_milliseconds = value,
            GlobalParameter::DelayFeedback => self.delay_feedback = value,
            GlobalParameter::DelayReturn => self.delay_return = value,
        }
        Ok(self)
    }
    /// Returns the final level applied after both global effect returns.
    pub const fn master_gain_db(&self) -> f32 {
        self.master_gain_db
    }

    /// Returns the normalized room size of the one shared reverb.
    pub const fn reverb_room_size(&self) -> f32 {
        self.reverb_room_size
    }

    /// Returns the normalized damping of the one shared reverb.
    pub const fn reverb_damping(&self) -> f32 {
        self.reverb_damping
    }

    /// Returns the normalized level of the one shared reverb return.
    pub const fn reverb_return(&self) -> f32 {
        self.reverb_return
    }

    /// Returns the delay time of the one shared delay in milliseconds.
    pub const fn delay_milliseconds(&self) -> f32 {
        self.delay_milliseconds
    }

    /// Returns the normalized feedback of the one shared delay.
    pub const fn delay_feedback(&self) -> f32 {
        self.delay_feedback
    }

    /// Returns the normalized level of the one shared delay return.
    pub const fn delay_return(&self) -> f32 {
        self.delay_return
    }
}

fn validate(parameter: GlobalParameter, value: f32) -> Result<(), GlobalParametersError> {
    if parameter.descriptor().contains(value) {
        Ok(())
    } else {
        Err(parameter.invalid_error())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GlobalParameter, GlobalParameterDescriptor, GlobalParameters, GlobalParametersError,
    };

    fn valid_parameters() -> GlobalParameters {
        GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2)
            .expect("fixture is within every declared bound")
    }

    #[test]
    fn surface_descriptor_is_unique_and_exact() {
        let descriptor = GlobalParameters::surface_descriptor();

        assert_eq!(
            descriptor,
            &[
                GlobalParameterDescriptor::new(GlobalParameter::MasterGainDb, -60.0, 6.0, 1.0, 6.0,),
                GlobalParameterDescriptor::new(
                    GlobalParameter::ReverbRoomSize,
                    0.0,
                    1.0,
                    0.01,
                    0.1,
                ),
                GlobalParameterDescriptor::new(GlobalParameter::ReverbDamping, 0.0, 1.0, 0.01, 0.1,),
                GlobalParameterDescriptor::new(GlobalParameter::ReverbReturn, 0.0, 1.0, 0.01, 0.1,),
                GlobalParameterDescriptor::new(
                    GlobalParameter::DelayMilliseconds,
                    1.0,
                    2000.0,
                    1.0,
                    100.0,
                ),
                GlobalParameterDescriptor::new(GlobalParameter::DelayFeedback, 0.0, 1.0, 0.01, 0.1,),
                GlobalParameterDescriptor::new(GlobalParameter::DelayReturn, 0.0, 1.0, 0.01, 0.1,),
            ]
        );
        for (index, entry) in descriptor.iter().enumerate() {
            assert!(
                !descriptor[..index]
                    .iter()
                    .any(|prior| prior.parameter() == entry.parameter()),
                "duplicate global parameter descriptor: {}",
                entry.name()
            );
            assert!(entry.fine_step() > 0.0);
            assert!(entry.coarse_step() >= entry.fine_step());
        }
    }

    #[test]
    fn descriptor_drives_value_access_and_validated_replacement() {
        let original = valid_parameters();

        for descriptor in GlobalParameters::surface_descriptor() {
            let parameter = descriptor.parameter();
            assert!(descriptor.contains(original.value(parameter)));

            let minimum = original
                .with_value(parameter, descriptor.minimum())
                .expect("descriptor minimum is valid");
            let maximum = original
                .with_value(parameter, descriptor.maximum())
                .expect("descriptor maximum is valid");
            assert_eq!(minimum.value(parameter), descriptor.minimum());
            assert_eq!(maximum.value(parameter), descriptor.maximum());
            assert!(original
                .with_value(parameter, descriptor.minimum() - 1.0)
                .is_err());
        }
    }
    #[test]
    fn retains_every_global_mix_parameter() {
        let parameters = valid_parameters();

        assert_eq!(parameters.master_gain_db(), -3.0);
        assert_eq!(parameters.reverb_room_size(), 0.7);
        assert_eq!(parameters.reverb_damping(), 0.4);
        assert_eq!(parameters.reverb_return(), 0.25);
        assert_eq!(parameters.delay_milliseconds(), 375.0);
        assert_eq!(parameters.delay_feedback(), 0.35);
        assert_eq!(parameters.delay_return(), 0.2);
    }

    #[test]
    fn accepts_every_inclusive_boundary() {
        assert!(GlobalParameters::new(-60.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0).is_ok());
        assert!(GlobalParameters::new(6.0, 1.0, 1.0, 1.0, 2000.0, 1.0, 1.0).is_ok());
    }

    #[test]
    fn rejects_each_out_of_range_parameter() {
        assert_eq!(
            GlobalParameters::new(-60.1, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5),
            Err(GlobalParametersError::InvalidMasterGainDb)
        );
        assert_eq!(
            GlobalParameters::new(0.0, 1.1, 0.5, 0.5, 250.0, 0.5, 0.5),
            Err(GlobalParametersError::InvalidReverbRoomSize)
        );
        assert_eq!(
            GlobalParameters::new(0.0, 0.5, -0.1, 0.5, 250.0, 0.5, 0.5),
            Err(GlobalParametersError::InvalidReverbDamping)
        );
        assert_eq!(
            GlobalParameters::new(0.0, 0.5, 0.5, 1.1, 250.0, 0.5, 0.5),
            Err(GlobalParametersError::InvalidReverbReturn)
        );
        assert_eq!(
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 2000.1, 0.5, 0.5),
            Err(GlobalParametersError::InvalidDelayMilliseconds)
        );
        assert_eq!(
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, -0.1, 0.5),
            Err(GlobalParametersError::InvalidDelayFeedback)
        );
        assert_eq!(
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 1.1),
            Err(GlobalParametersError::InvalidDelayReturn)
        );
    }

    #[test]
    fn rejects_non_finite_values() {
        assert_eq!(
            GlobalParameters::new(f32::NAN, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5),
            Err(GlobalParametersError::InvalidMasterGainDb)
        );
        assert_eq!(
            GlobalParameters::new(0.0, f32::INFINITY, 0.5, 0.5, 250.0, 0.5, 0.5),
            Err(GlobalParametersError::InvalidReverbRoomSize)
        );
        assert_eq!(
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, f32::NEG_INFINITY, 0.5, 0.5),
            Err(GlobalParametersError::InvalidDelayMilliseconds)
        );
    }

    #[test]
    fn errors_explain_the_rejected_bound() {
        assert_eq!(
            GlobalParametersError::InvalidDelayMilliseconds.to_string(),
            "delay time must be finite and in 1.0..=2000.0 milliseconds"
        );
    }
}
