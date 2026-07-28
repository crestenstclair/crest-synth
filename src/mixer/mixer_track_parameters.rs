use core::fmt;
use serde::{Deserialize, Deserializer, Serialize};

/// Stable identity of one configurable mixer-track field.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MixerTrackParameter {
    Level,
    Pan,
    Mute,
    Solo,
    ReverbSend,
    DelaySend,
}

impl MixerTrackParameter {
    pub const ALL: [Self; 6] = [
        Self::Level,
        Self::Pan,
        Self::Mute,
        Self::Solo,
        Self::ReverbSend,
        Self::DelaySend,
    ];
    pub const MAIN: [Self; 4] = [Self::Level, Self::Pan, Self::Mute, Self::Solo];
    pub const INSPECTOR: [Self; 2] = [Self::ReverbSend, Self::DelaySend];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Level => "levelDb",
            Self::Pan => "pan",
            Self::Mute => "mute",
            Self::Solo => "solo",
            Self::ReverbSend => "reverbSend",
            Self::DelaySend => "delaySend",
        }
    }

    pub const fn descriptor(self) -> &'static MixerTrackParameterDescriptor {
        match self {
            Self::Level => &MIXER_TRACK_SURFACE_DESCRIPTOR[0],
            Self::Pan => &MIXER_TRACK_SURFACE_DESCRIPTOR[1],
            Self::Mute => &MIXER_TRACK_SURFACE_DESCRIPTOR[2],
            Self::Solo => &MIXER_TRACK_SURFACE_DESCRIPTOR[3],
            Self::ReverbSend => &MIXER_TRACK_SURFACE_DESCRIPTOR[4],
            Self::DelaySend => &MIXER_TRACK_SURFACE_DESCRIPTOR[5],
        }
    }
}

impl fmt::Display for MixerTrackParameter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MixerTrackParameterKind {
    Continuous,
    Toggle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixerTrackParameterDescriptor {
    parameter: MixerTrackParameter,
    label: &'static str,
    kind: MixerTrackParameterKind,
    minimum: f32,
    maximum: f32,
    default: f32,
    fine_step: f32,
    coarse_step: f32,
    unit: Option<&'static str>,
}

impl MixerTrackParameterDescriptor {
    const fn continuous(
        parameter: MixerTrackParameter,
        label: &'static str,
        range: (f32, f32),
        default: f32,
        steps: (f32, f32),
        unit: Option<&'static str>,
    ) -> Self {
        Self {
            parameter,
            label,
            kind: MixerTrackParameterKind::Continuous,
            minimum: range.0,
            maximum: range.1,
            default,
            fine_step: steps.0,
            coarse_step: steps.1,
            unit,
        }
    }

    const fn toggle(parameter: MixerTrackParameter, label: &'static str) -> Self {
        Self {
            parameter,
            label,
            kind: MixerTrackParameterKind::Toggle,
            minimum: 0.0,
            maximum: 1.0,
            default: 0.0,
            fine_step: 1.0,
            coarse_step: 1.0,
            unit: None,
        }
    }

    pub const fn parameter(self) -> MixerTrackParameter {
        self.parameter
    }

    pub const fn name(self) -> &'static str {
        self.parameter.name()
    }

    pub const fn label(self) -> &'static str {
        self.label
    }

    pub const fn kind(self) -> MixerTrackParameterKind {
        self.kind
    }

    pub const fn minimum(self) -> f32 {
        self.minimum
    }

    pub const fn maximum(self) -> f32 {
        self.maximum
    }

    pub const fn default(self) -> f32 {
        self.default
    }

    pub const fn fine_step(self) -> f32 {
        self.fine_step
    }

    pub const fn coarse_step(self) -> f32 {
        self.coarse_step
    }

    pub const fn unit(self) -> Option<&'static str> {
        self.unit
    }

    pub fn contains(self, value: f32) -> bool {
        value.is_finite() && (self.minimum..=self.maximum).contains(&value)
    }
}

const MIXER_TRACK_SURFACE_DESCRIPTOR: [MixerTrackParameterDescriptor; 6] = [
    MixerTrackParameterDescriptor::continuous(
        MixerTrackParameter::Level,
        "Level",
        (-60.0, 6.0),
        0.0,
        (1.0, 6.0),
        Some("dB"),
    ),
    MixerTrackParameterDescriptor::continuous(
        MixerTrackParameter::Pan,
        "Pan",
        (-1.0, 1.0),
        0.0,
        (0.01, 0.1),
        None,
    ),
    MixerTrackParameterDescriptor::toggle(MixerTrackParameter::Mute, "Mute"),
    MixerTrackParameterDescriptor::toggle(MixerTrackParameter::Solo, "Solo"),
    MixerTrackParameterDescriptor::continuous(
        MixerTrackParameter::ReverbSend,
        "Reverb Send",
        (0.0, 1.0),
        0.0,
        (0.01, 0.1),
        None,
    ),
    MixerTrackParameterDescriptor::continuous(
        MixerTrackParameter::DelaySend,
        "Delay Send",
        (0.0, 1.0),
        0.0,
        (0.01, 0.1),
        None,
    ),
];

/// Canonical scalar and toggle state owned by one persistent mixer track.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixerTrackParameters {
    level_db: f32,
    pan: f32,
    mute: bool,
    solo: bool,
    reverb_send: f32,
    delay_send: f32,
}

impl MixerTrackParameters {
    pub fn new(
        level_db: f32,
        pan: f32,
        mute: bool,
        solo: bool,
        reverb_send: f32,
        delay_send: f32,
    ) -> Result<Self, MixerTrackParametersError> {
        validate(MixerTrackParameter::Level, level_db)?;
        validate(MixerTrackParameter::Pan, pan)?;
        validate(MixerTrackParameter::ReverbSend, reverb_send)?;
        validate(MixerTrackParameter::DelaySend, delay_send)?;
        Ok(Self {
            level_db,
            pan,
            mute,
            solo,
            reverb_send,
            delay_send,
        })
    }

    pub const fn surface_descriptor() -> &'static [MixerTrackParameterDescriptor] {
        &MIXER_TRACK_SURFACE_DESCRIPTOR
    }

    pub const fn level_db(self) -> f32 {
        self.level_db
    }

    pub const fn pan(self) -> f32 {
        self.pan
    }

    pub const fn mute(self) -> bool {
        self.mute
    }

    pub const fn solo(self) -> bool {
        self.solo
    }

    pub const fn reverb_send(self) -> f32 {
        self.reverb_send
    }

    pub const fn delay_send(self) -> f32 {
        self.delay_send
    }

    pub const fn scalar_value(self, parameter: MixerTrackParameter) -> Option<f32> {
        match parameter {
            MixerTrackParameter::Level => Some(self.level_db),
            MixerTrackParameter::Pan => Some(self.pan),
            MixerTrackParameter::ReverbSend => Some(self.reverb_send),
            MixerTrackParameter::DelaySend => Some(self.delay_send),
            MixerTrackParameter::Mute | MixerTrackParameter::Solo => None,
        }
    }

    pub const fn toggle_value(self, parameter: MixerTrackParameter) -> Option<bool> {
        match parameter {
            MixerTrackParameter::Mute => Some(self.mute),
            MixerTrackParameter::Solo => Some(self.solo),
            MixerTrackParameter::Level
            | MixerTrackParameter::Pan
            | MixerTrackParameter::ReverbSend
            | MixerTrackParameter::DelaySend => None,
        }
    }

    pub fn with_scalar_value(
        mut self,
        parameter: MixerTrackParameter,
        value: f32,
    ) -> Result<Self, MixerTrackParametersError> {
        if parameter.descriptor().kind() != MixerTrackParameterKind::Continuous {
            return Err(MixerTrackParametersError::WrongValueKind { parameter });
        }
        validate(parameter, value)?;
        match parameter {
            MixerTrackParameter::Level => self.level_db = value,
            MixerTrackParameter::Pan => self.pan = value,
            MixerTrackParameter::ReverbSend => self.reverb_send = value,
            MixerTrackParameter::DelaySend => self.delay_send = value,
            MixerTrackParameter::Mute | MixerTrackParameter::Solo => unreachable!(),
        }
        Ok(self)
    }

    pub fn toggled(
        mut self,
        parameter: MixerTrackParameter,
    ) -> Result<Self, MixerTrackParametersError> {
        match parameter {
            MixerTrackParameter::Mute => self.mute = !self.mute,
            MixerTrackParameter::Solo => self.solo = !self.solo,
            MixerTrackParameter::Level
            | MixerTrackParameter::Pan
            | MixerTrackParameter::ReverbSend
            | MixerTrackParameter::DelaySend => {
                return Err(MixerTrackParametersError::WrongValueKind { parameter });
            }
        }
        Ok(self)
    }
}

impl Default for MixerTrackParameters {
    fn default() -> Self {
        Self {
            level_db: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
            reverb_send: 0.0,
            delay_send: 0.0,
        }
    }
}

impl<'de> Deserialize<'de> for MixerTrackParameters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Value {
            level_db: f32,
            pan: f32,
            mute: bool,
            solo: bool,
            reverb_send: f32,
            delay_send: f32,
        }

        let value = Value::deserialize(deserializer)?;
        Self::new(
            value.level_db,
            value.pan,
            value.mute,
            value.solo,
            value.reverb_send,
            value.delay_send,
        )
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MixerTrackParametersError {
    NonFinite {
        parameter: MixerTrackParameter,
    },
    OutOfRange {
        parameter: MixerTrackParameter,
        value: f32,
    },
    WrongValueKind {
        parameter: MixerTrackParameter,
    },
}

impl fmt::Display for MixerTrackParametersError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NonFinite { parameter } => write!(formatter, "{parameter} must be finite"),
            Self::OutOfRange { parameter, value } => {
                let descriptor = parameter.descriptor();
                write!(
                    formatter,
                    "{parameter} must be in {}..={}, got {value}",
                    descriptor.minimum(),
                    descriptor.maximum()
                )
            }
            Self::WrongValueKind { parameter } => {
                write!(formatter, "{parameter} does not accept that value kind")
            }
        }
    }
}

impl std::error::Error for MixerTrackParametersError {}

fn validate(parameter: MixerTrackParameter, value: f32) -> Result<(), MixerTrackParametersError> {
    if !value.is_finite() {
        return Err(MixerTrackParametersError::NonFinite { parameter });
    }
    if !parameter.descriptor().contains(value) {
        return Err(MixerTrackParametersError::OutOfRange { parameter, value });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MixerTrackParameter, MixerTrackParameterKind, MixerTrackParameters,
        MixerTrackParametersError,
    };

    #[test]
    fn descriptor_has_all_six_fields_in_main_then_inspector_order() {
        let parameters = MixerTrackParameters::surface_descriptor()
            .iter()
            .map(|descriptor| descriptor.parameter())
            .collect::<Vec<_>>();
        assert_eq!(parameters, MixerTrackParameter::ALL);
        assert_eq!(&parameters[..4], MixerTrackParameter::MAIN);
        assert_eq!(&parameters[4..], MixerTrackParameter::INSPECTOR);
        assert_eq!(
            MixerTrackParameter::Mute.descriptor().kind(),
            MixerTrackParameterKind::Toggle
        );
    }

    #[test]
    fn every_continuous_inclusive_boundary_is_valid() {
        let minimums = MixerTrackParameters::new(-60.0, -1.0, false, false, 0.0, 0.0).unwrap();
        let maximums = MixerTrackParameters::new(6.0, 1.0, true, true, 1.0, 1.0).unwrap();
        assert_eq!(minimums.level_db(), -60.0);
        assert_eq!(maximums.level_db(), 6.0);
        assert!(maximums.mute());
        assert!(maximums.solo());
    }

    #[test]
    fn each_invalid_numeric_class_is_rejected() {
        assert!(matches!(
            MixerTrackParameters::new(f32::NAN, 0.0, false, false, 0.0, 0.0),
            Err(MixerTrackParametersError::NonFinite {
                parameter: MixerTrackParameter::Level
            })
        ));
        for (parameter, value) in [
            (MixerTrackParameter::Level, 6.1),
            (MixerTrackParameter::Pan, -1.1),
            (MixerTrackParameter::ReverbSend, 1.1),
            (MixerTrackParameter::DelaySend, -0.1),
        ] {
            assert!(matches!(
                MixerTrackParameters::default().with_scalar_value(parameter, value),
                Err(MixerTrackParametersError::OutOfRange { .. })
            ));
        }
    }

    #[test]
    fn scalar_adjustment_and_toggle_kinds_are_disjoint() {
        let parameters = MixerTrackParameters::default()
            .with_scalar_value(MixerTrackParameter::Pan, 0.5)
            .unwrap()
            .toggled(MixerTrackParameter::Mute)
            .unwrap();
        assert_eq!(parameters.pan(), 0.5);
        assert!(parameters.mute());
        assert!(parameters
            .with_scalar_value(MixerTrackParameter::Solo, 1.0)
            .is_err());
        assert!(parameters.toggled(MixerTrackParameter::Level).is_err());
    }

    #[test]
    fn serde_cannot_bypass_numeric_validation() {
        assert!(serde_json::from_str::<MixerTrackParameters>(
            r#"{"levelDb":0.0,"pan":2.0,"mute":false,"solo":false,"reverbSend":0.0,"delaySend":0.0}"#
        )
        .is_err());
    }
}
