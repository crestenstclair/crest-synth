use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::synth::sound_font_instrument::SoundFontInstrument;

/// One installed, playable SoundFont instrument.
///
/// A patch's identity, display name, instrument, and assigned MIDI channel are
/// fixed at construction time. Only its channel parameters can change after
/// installation.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    id: PatchId,
    name: String,
    instrument: SoundFontInstrument,
    channel: MidiChannel,
    parameters: ChannelParameters,
}

impl Patch {
    /// Creates an installed patch from validated domain values.
    pub fn new(
        id: PatchId,
        name: String,
        instrument: SoundFontInstrument,
        channel: MidiChannel,
        parameters: ChannelParameters,
    ) -> Self {
        Self {
            id,
            name,
            instrument,
            channel,
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

    /// Returns the immutable SoundFont preset identity.
    pub const fn instrument(&self) -> &SoundFontInstrument {
        &self.instrument
    }

    /// Returns the immutable MIDI channel assigned by the input adapter.
    pub const fn channel(&self) -> MidiChannel {
        self.channel
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_keeps_configuration_read_only() {
        let _: fn(PatchId, String, SoundFontInstrument, MidiChannel, ChannelParameters) -> Patch =
            Patch::new;
        let _: fn(&Patch) -> PatchId = Patch::id;
        let _: for<'a> fn(&'a Patch) -> &'a str = Patch::name;
        let _: for<'a> fn(&'a Patch) -> &'a SoundFontInstrument = Patch::instrument;
        let _: fn(&Patch) -> MidiChannel = Patch::channel;
        let _: for<'a> fn(&'a Patch) -> &'a ChannelParameters = Patch::parameters;
        let _: for<'a> fn(&'a mut Patch) -> &'a mut ChannelParameters = Patch::parameters_mut;
        let _: fn(&mut Patch, ChannelParameters) = Patch::set_parameters;
    }
}
