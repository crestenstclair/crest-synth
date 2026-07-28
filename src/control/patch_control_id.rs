use crate::mixer::patch_output::PatchOutputParameter;
use crate::synth::{
    CapabilityDescriptor, EffectCapabilityRegistry, EffectSlotId, InstrumentConfig, ParameterId,
    PatchInteraction, PostEffectConfig, VoiceEnvelope, VoiceEnvelopeParameter,
};
use core::fmt;
use core::str::FromStr;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::cmp::Ordering;

/// Stable semantic focus identity inside the PATCH context.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PatchControlId {
    Engine,
    Output(PatchOutputParameter),
    Envelope(VoiceEnvelopeParameter),
    Capability(ParameterId),
    Effect(EffectSlotId, ParameterId),
}

impl PatchControlId {
    pub const ALL: [Self; 1 + VoiceEnvelope::surface_descriptor().len()] = [
        Self::Engine,
        Self::Envelope(VoiceEnvelopeParameter::AttackMilliseconds),
        Self::Envelope(VoiceEnvelopeParameter::DecayMilliseconds),
        Self::Envelope(VoiceEnvelopeParameter::Sustain),
        Self::Envelope(VoiceEnvelopeParameter::ReleaseMilliseconds),
    ];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }

    pub const UTILITY: [Self; 2] = [
        Self::Output(PatchOutputParameter::TrimGain),
        Self::Output(PatchOutputParameter::OutputTrack),
    ];

    pub const fn utility_surface_descriptor() -> &'static [Self] {
        &Self::UTILITY
    }

    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            Self::Engine => Cow::Borrowed("patch.engine"),
            Self::Output(PatchOutputParameter::TrimGain) => {
                Cow::Borrowed("patch.output.trimGainDb")
            }
            Self::Output(PatchOutputParameter::OutputTrack) => {
                Cow::Borrowed("patch.output.outputTrack")
            }
            Self::Envelope(VoiceEnvelopeParameter::AttackMilliseconds) => {
                Cow::Borrowed("patch.envelope.attackMilliseconds")
            }
            Self::Envelope(VoiceEnvelopeParameter::DecayMilliseconds) => {
                Cow::Borrowed("patch.envelope.decayMilliseconds")
            }
            Self::Envelope(VoiceEnvelopeParameter::Sustain) => {
                Cow::Borrowed("patch.envelope.sustain")
            }
            Self::Envelope(VoiceEnvelopeParameter::ReleaseMilliseconds) => {
                Cow::Borrowed("patch.envelope.releaseMilliseconds")
            }
            Self::Capability(parameter_id) => {
                Cow::Owned(format!("patch.capability.{parameter_id}"))
            }
            Self::Effect(slot_id, parameter_id) => {
                Cow::Owned(format!("patch.effect.{slot_id}.{parameter_id}"))
            }
        }
    }

    /// Resolves the complete PATCH focus surface from the active descriptor and
    /// canonical config. This is the sole ordering/filtering authority used by
    /// reducer navigation and projections.
    pub fn resolve(
        descriptor: &CapabilityDescriptor,
        config: &InstrumentConfig,
        effect_registry: &EffectCapabilityRegistry,
        post_effects: &[PostEffectConfig],
    ) -> Vec<Self> {
        let mut controls = Self::surface_descriptor().to_vec();
        for spec in descriptor.parameters() {
            let predicate_satisfied = |predicate: Option<&crate::synth::ParameterPredicate>| {
                predicate.is_none_or(|predicate| {
                    config.value(predicate.parameter_id()) == Some(predicate.equals())
                })
            };
            if spec.patch_interaction() == PatchInteraction::StructuralChoice
                && predicate_satisfied(spec.visible_when())
                && predicate_satisfied(spec.enabled_when())
            {
                controls.push(Self::Capability(spec.id().clone()));
            }
        }
        for effect in post_effects {
            let Some(effect_descriptor) = effect_registry.descriptor(effect.capability_id()) else {
                continue;
            };
            for spec in effect_descriptor.parameters() {
                let predicate_satisfied = |predicate: Option<&crate::synth::ParameterPredicate>| {
                    predicate.is_none_or(|predicate| {
                        effect.value(predicate.parameter_id()) == Some(predicate.equals())
                    })
                };
                if spec.patch_interaction() == PatchInteraction::ScalarEdit
                    && predicate_satisfied(spec.visible_when())
                    && predicate_satisfied(spec.enabled_when())
                {
                    controls.push(Self::Effect(effect.slot_id(), spec.id().clone()));
                }
            }
        }
        controls
    }
}

impl Ord for PatchControlId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(&other.as_str())
    }
}

impl PartialOrd for PatchControlId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for PatchControlId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str().as_ref())
    }
}

impl FromStr for PatchControlId {
    type Err = ParsePatchControlIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "patch.engine" => Ok(Self::Engine),
            "patch.output.trimGainDb" => Ok(Self::Output(PatchOutputParameter::TrimGain)),
            "patch.output.outputTrack" => Ok(Self::Output(PatchOutputParameter::OutputTrack)),
            "patch.envelope.attackMilliseconds" => {
                Ok(Self::Envelope(VoiceEnvelopeParameter::AttackMilliseconds))
            }
            "patch.envelope.decayMilliseconds" => {
                Ok(Self::Envelope(VoiceEnvelopeParameter::DecayMilliseconds))
            }
            "patch.envelope.sustain" => Ok(Self::Envelope(VoiceEnvelopeParameter::Sustain)),
            "patch.envelope.releaseMilliseconds" => {
                Ok(Self::Envelope(VoiceEnvelopeParameter::ReleaseMilliseconds))
            }
            value if value.starts_with("patch.capability.") => {
                let parameter = value
                    .strip_prefix("patch.capability.")
                    .expect("the prefix was just matched");
                ParameterId::new(parameter)
                    .map(Self::Capability)
                    .map_err(|_| ParsePatchControlIdError)
            }
            value if value.starts_with("patch.effect.") => {
                let suffix = value
                    .strip_prefix("patch.effect.")
                    .expect("the prefix was just matched");
                let (slot, parameter) = suffix.split_once('.').ok_or(ParsePatchControlIdError)?;
                let slot = slot.parse::<u16>().map_err(|_| ParsePatchControlIdError)?;
                let slot = EffectSlotId::new(slot).map_err(|_| ParsePatchControlIdError)?;
                let parameter =
                    ParameterId::new(parameter).map_err(|_| ParsePatchControlIdError)?;
                Ok(Self::Effect(slot, parameter))
            }
            _ => Err(ParsePatchControlIdError),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParsePatchControlIdError;

impl fmt::Display for ParsePatchControlIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown PATCH control identity")
    }
}

impl std::error::Error for ParsePatchControlIdError {}

impl Serialize for PatchControlId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str().as_ref())
    }
}

impl<'de> Deserialize<'de> for PatchControlId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PatchControlIdVisitor;

        impl Visitor<'_> for PatchControlIdVisitor {
            type Value = PatchControlId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a stable PATCH control identity")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(PatchControlIdVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::PatchControlId;
    use crate::synth::{VoiceEnvelope, VoiceEnvelopeParameter};

    #[test]
    fn patch_control_ids_follow_the_canonical_envelope_descriptor() {
        let expected_envelope = VoiceEnvelope::surface_descriptor()
            .iter()
            .map(|descriptor| PatchControlId::Envelope(descriptor.parameter()))
            .collect::<Vec<_>>();

        assert_eq!(PatchControlId::surface_descriptor().len(), 5);
        assert_eq!(
            PatchControlId::surface_descriptor()[0],
            PatchControlId::Engine
        );
        assert_eq!(
            &PatchControlId::surface_descriptor()[1..],
            expected_envelope
        );
        assert_eq!(
            expected_envelope,
            vec![
                PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds),
                PatchControlId::Envelope(VoiceEnvelopeParameter::DecayMilliseconds),
                PatchControlId::Envelope(VoiceEnvelopeParameter::Sustain),
                PatchControlId::Envelope(VoiceEnvelopeParameter::ReleaseMilliseconds),
            ]
        );
    }

    #[test]
    fn every_patch_control_id_has_a_stable_string_round_trip() {
        let names = [
            "patch.engine",
            "patch.envelope.attackMilliseconds",
            "patch.envelope.decayMilliseconds",
            "patch.envelope.sustain",
            "patch.envelope.releaseMilliseconds",
        ];

        for (control, name) in PatchControlId::surface_descriptor().iter().zip(names) {
            assert_eq!(control.as_str(), name);
            assert_eq!(control.to_string(), name);
            assert_eq!(name.parse::<PatchControlId>().unwrap(), *control);
            let json = serde_json::to_string(control).unwrap();
            assert_eq!(json, format!("\"{name}\""));
            assert_eq!(
                serde_json::from_str::<PatchControlId>(&json).unwrap(),
                *control
            );
        }
    }

    #[test]
    fn unknown_patch_control_identity_is_rejected() {
        assert!("patch.envelope.filter".parse::<PatchControlId>().is_err());
        assert!(serde_json::from_str::<PatchControlId>("\"patch.envelope.filter\"").is_err());
    }
}
