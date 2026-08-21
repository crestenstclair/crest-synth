use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::real_time::parameter_snapshot::RtPatchParameters;
use crate::synth::{AssetReference, PreparedSampleVisualization};
use core::fmt;

/// One fully prepared Patch-specific synthesis runtime.
///
/// Implementations own all voice state and scratch needed by one Patch. Every
/// method is called from the hard real-time path and must use only bounded,
/// preallocated work with no allocation, destruction, locking, blocking, I/O,
/// logging, formatting, panic, or unwind.
pub trait PreparedInstrument: Send {
    /// Returns the immutable Patch identity prepared into this instrument.
    fn patch_id(&self) -> PatchId;

    /// Delivers one normalized MIDI message to this instrument only.
    fn dispatch(
        &mut self,
        message: MidiMessage,
        parameters: &RtPatchParameters,
    ) -> Result<(), PreparedInstrumentError>;

    /// Fills exactly `frame_count` frames in caller-owned interleaved stereo
    /// storage. The rack validates storage identity and capacity before this
    /// operation is called.
    fn render(
        &mut self,
        interleaved_stereo: &mut [f32],
        frame_count: usize,
        parameters: &RtPatchParameters,
    );

    /// Silences this instrument's voices with bounded work.
    fn all_notes_off(&mut self);

    /// Reports one immutable shared prepared asset to the worker-side graph
    /// builder. Callback rendering never calls this method. The generic
    /// reference and numeric preparation key let the builder deduplicate and
    /// budget assets without switching on a concrete capability identity.
    fn prepared_asset_footprint(&self) -> Option<&PreparedAssetFootprint> {
        None
    }

    /// Optional bounded control-side visualization produced during
    /// preparation. The callback never calls this method.
    fn prepared_sample_visualization(&self) -> Option<&PreparedSampleVisualization> {
        None
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PreparedAssetFootprint {
    reference: AssetReference,
    preparation_key: u64,
    bytes: usize,
}

impl PreparedAssetFootprint {
    pub const fn new(reference: AssetReference, preparation_key: u64, bytes: usize) -> Self {
        Self {
            reference,
            preparation_key,
            bytes,
        }
    }

    pub const fn reference(&self) -> &AssetReference {
        &self.reference
    }

    pub const fn preparation_key(&self) -> u64 {
        self.preparation_key
    }

    pub const fn bytes(&self) -> usize {
        self.bytes
    }
}

/// A fixed-size failure returned by callback-side instrument dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreparedInstrumentError {
    UnsupportedMidiKind { kind: MidiMessageKind },
    DispatchRejected,
}

impl fmt::Display for PreparedInstrumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMidiKind { kind } => {
                write!(formatter, "prepared instrument does not support {kind:?}")
            }
            Self::DispatchRejected => formatter.write_str("prepared instrument rejected MIDI"),
        }
    }
}

impl std::error::Error for PreparedInstrumentError {}

#[cfg(test)]
mod tests {
    use super::PreparedInstrumentError;

    #[test]
    fn callback_status_is_copyable_and_has_no_destructor() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<PreparedInstrumentError>();
        assert!(!core::mem::needs_drop::<PreparedInstrumentError>());
    }
}
