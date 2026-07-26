use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::synth::instrument_capability::{
    CapabilityDescriptor, CapabilityError, InstrumentConfig, ParameterUpdate,
};
use crate::synth::parameter_id::ParameterId;
use crate::synth::voice_envelope::{VoiceEnvelope, VoiceEnvelopeParameter};
use serde::Serialize;

/// One entry in the canonical schema-derived editable Patch surface.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "parameter", rename_all = "camelCase")]
pub enum PatchEditableTarget {
    Mixer(crate::mixer::channel_parameters::ChannelParameter),
    Envelope(VoiceEnvelopeParameter),
    Instrument(ParameterId),
}

impl PatchEditableTarget {
    /// Returns the stable semantic identifier used by projection and coverage.
    pub fn name(&self) -> &str {
        match self {
            Self::Mixer(parameter) => parameter.name(),
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
        ChannelParameters::surface_descriptor().len()
            + VoiceEnvelope::surface_descriptor().len()
            + descriptor.scalar_parameter_count(),
    );
    targets.extend(
        ChannelParameters::surface_descriptor()
            .iter()
            .map(|descriptor| PatchEditableTarget::Mixer(descriptor.parameter())),
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
/// fixed at construction time. Mixer, envelope, and descriptor-classified
/// Scalar values can change only through the canonical reducer.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    id: PatchId,
    name: String,
    instrument: InstrumentConfig,
    channel: MidiChannel,
    envelope: VoiceEnvelope,
    parameters: ChannelParameters,
}

impl Patch {
    /// Creates an installed patch from validated domain values.
    pub fn new(
        id: PatchId,
        name: String,
        instrument: InstrumentConfig,
        channel: MidiChannel,
        parameters: ChannelParameters,
    ) -> Self {
        Self {
            id,
            name,
            instrument,
            channel,
            envelope: VoiceEnvelope::default(),
            parameters,
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

    /// Returns the current editable channel parameters.
    pub const fn parameters(&self) -> &ChannelParameters {
        &self.parameters
    }

    /// Provides the aggregate's only mutable state surface.
    pub fn parameters_mut(&mut self) -> &mut ChannelParameters {
        &mut self.parameters
    }

    /// Replaces the complete bounded channel-parameter value.
    pub fn set_parameters(&mut self, parameters: ChannelParameters) {
        self.parameters = parameters;
    }

    /// Supplies a non-default envelope while constructing a Patch fixture.
    pub fn with_envelope(mut self, envelope: VoiceEnvelope) -> Self {
        self.envelope = envelope;
        self
    }

    /// Resolves this Patch's mixer, common ADSR, and live instrument targets.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::capability_id::CapabilityId;

    #[test]
    fn public_api_keeps_configuration_read_only() {
        let _: fn(PatchId, String, InstrumentConfig, MidiChannel, ChannelParameters) -> Patch =
            Patch::new;
        let _: fn(&Patch) -> PatchId = Patch::id;
        let _: for<'a> fn(&'a Patch) -> &'a str = Patch::name;
        let _: for<'a> fn(&'a Patch) -> &'a InstrumentConfig = Patch::instrument_config;
        let _: fn(&Patch) -> MidiChannel = Patch::channel;
        let _: for<'a> fn(&'a Patch) -> &'a VoiceEnvelope = Patch::envelope;
        let _: for<'a> fn(&'a Patch) -> &'a ChannelParameters = Patch::parameters;
        let _: for<'a> fn(&'a mut Patch) -> &'a mut ChannelParameters = Patch::parameters_mut;
        let _: fn(&mut Patch, ChannelParameters) = Patch::set_parameters;

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
            ChannelParameters::default(),
        );
        assert_eq!(patch.instrument_config(), &config);
        assert_eq!(patch.envelope(), &VoiceEnvelope::default());
    }
}
