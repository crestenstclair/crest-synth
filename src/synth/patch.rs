use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::patch_id::PatchId;
use crate::mixer::patch_output::PatchOutput;
use crate::synth::instrument_capability::{
    CapabilityDescriptor, CapabilityError, InstrumentConfig, ParameterUpdate,
};
use crate::synth::parameter_id::ParameterId;
use crate::synth::voice_envelope::{VoiceEnvelope, VoiceEnvelopeParameter};
use crate::synth::PostEffectConfig;
use serde::{Deserialize, Serialize};

/// One entry in the canonical schema-derived editable Patch surface.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "parameter", rename_all = "camelCase")]
pub enum PatchEditableTarget {
    Envelope(VoiceEnvelopeParameter),
    Instrument(ParameterId),
}

impl PatchEditableTarget {
    /// Returns the stable semantic identifier used by projection and coverage.
    pub fn name(&self) -> &str {
        match self {
            Self::Envelope(parameter) => parameter.name(),
            Self::Instrument(parameter) => parameter.as_str(),
        }
    }
}

/// Resolves the only editable Patch ordering from immutable schema and config.
pub fn resolve_patch_editable_targets(
    descriptor: &CapabilityDescriptor,
    config: &InstrumentConfig,
) -> Result<Vec<PatchEditableTarget>, CapabilityError> {
    if descriptor.id() != config.capability_id() {
        return Err(CapabilityError::ProviderRegistryMismatch(
            config.capability_id().clone(),
        ));
    }
    let canonical = descriptor.create_config(config.values(), config.asset_references())?;
    if canonical != *config {
        return Err(CapabilityError::ConfigOrderMismatch(
            config.capability_id().clone(),
        ));
    }

    let mut targets = Vec::with_capacity(
        VoiceEnvelope::surface_descriptor().len() + descriptor.scalar_parameter_count(),
    );
    targets.extend(
        VoiceEnvelope::surface_descriptor()
            .iter()
            .map(|descriptor| PatchEditableTarget::Envelope(descriptor.parameter())),
    );
    targets.extend(
        descriptor
            .parameters()
            .filter(|parameter| parameter.update() == ParameterUpdate::Scalar)
            .map(|parameter| PatchEditableTarget::Instrument(parameter.id().clone())),
    );
    Ok(targets)
}

/// One installed, playable instrument capability configuration.
///
/// A patch's identity, display name, instrument, and assigned MIDI channel are
/// fixed at construction time. Output, envelope, and descriptor-classified
/// values can change only through the canonical reducer.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    id: PatchId,
    name: String,
    instrument: InstrumentConfig,
    channel: MidiChannel,
    envelope: VoiceEnvelope,
    output: PatchOutput,
    post_effects: Vec<PostEffectConfig>,
}

impl Patch {
    /// Creates an installed patch from validated domain values.
    pub fn new(
        id: PatchId,
        name: String,
        instrument: InstrumentConfig,
        channel: MidiChannel,
        output: PatchOutput,
    ) -> Self {
        Self {
            id,
            name,
            instrument,
            channel,
            envelope: VoiceEnvelope::default(),
            output,
            post_effects: Vec::new(),
        }
    }

    /// Returns this patch's stable process-lifetime identity.
    pub const fn id(&self) -> PatchId {
        self.id
    }

    /// Returns the immutable display name assigned at installation.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the current generic instrument configuration.
    pub const fn instrument_config(&self) -> &InstrumentConfig {
        &self.instrument
    }

    /// Returns the immutable MIDI channel assigned by the input adapter.
    pub const fn channel(&self) -> MidiChannel {
        self.channel
    }

    /// Returns the canonical Patch-owned ADSR value.
    pub const fn envelope(&self) -> &VoiceEnvelope {
        &self.envelope
    }

    /// Returns this Patch's validated pre-track trim and destination.
    pub const fn output(&self) -> PatchOutput {
        self.output
    }

    /// Returns the canonical ordered Patch-local post-effect configurations.
    pub fn post_effects(&self) -> &[PostEffectConfig] {
        &self.post_effects
    }

    /// Replaces the complete bounded Patch output value through the reducer.
    pub(crate) fn set_output(&mut self, output: PatchOutput) {
        self.output = output;
    }

    /// Supplies a non-default envelope while constructing a Patch fixture.
    pub fn with_envelope(mut self, envelope: VoiceEnvelope) -> Self {
        self.envelope = envelope;
        self
    }

    /// Supplies the ordered post-effect list while constructing a Patch.
    /// Installation validates its registry identity, config, and current capacity.
    pub fn with_post_effects(mut self, post_effects: Vec<PostEffectConfig>) -> Self {
        self.post_effects = post_effects;
        self
    }

    /// Resolves this Patch's common ADSR and live instrument targets.
    pub fn editable_targets(
        &self,
        descriptor: &CapabilityDescriptor,
    ) -> Result<Vec<PatchEditableTarget>, CapabilityError> {
        resolve_patch_editable_targets(descriptor, &self.instrument)
    }

    pub(crate) fn set_envelope(&mut self, envelope: VoiceEnvelope) {
        self.envelope = envelope;
    }

    pub(crate) fn set_instrument_config(&mut self, config: InstrumentConfig) {
        self.instrument = config;
    }

    pub(crate) fn set_post_effect_config(&mut self, index: usize, config: PostEffectConfig) {
        self.post_effects[index] = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::capability_id::CapabilityId;

    #[test]
    fn public_api_keeps_configuration_read_only() {
        let _: fn(PatchId, String, InstrumentConfig, MidiChannel, PatchOutput) -> Patch =
            Patch::new;
        let _: fn(&Patch) -> PatchId = Patch::id;
        let _: for<'a> fn(&'a Patch) -> &'a str = Patch::name;
        let _: for<'a> fn(&'a Patch) -> &'a InstrumentConfig = Patch::instrument_config;
        let _: fn(&Patch) -> MidiChannel = Patch::channel;
        let _: for<'a> fn(&'a Patch) -> &'a VoiceEnvelope = Patch::envelope;
        let _: fn(&Patch) -> PatchOutput = Patch::output;

        let config = InstrumentConfig::from_parts(
            CapabilityId::new("instrument.test").unwrap(),
            Vec::new(),
            Vec::new(),
        );
        let patch = Patch::new(
            PatchId::new(1).unwrap(),
            "Test".to_owned(),
            config.clone(),
            MidiChannel::new(0).unwrap(),
            PatchOutput::default(),
        );
        assert_eq!(patch.instrument_config(), &config);
        assert_eq!(patch.envelope(), &VoiceEnvelope::default());
    }
}
