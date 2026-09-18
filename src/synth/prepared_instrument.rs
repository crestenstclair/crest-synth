use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::real_time::parameter_snapshot::RtPatchParameters;
use crate::synth::{AssetReference, PreparedSampleVisualization};
use core::fmt;

/// One fully prepared Patch-specific synthesis runtime.
///
/// Implementations own all voice state and scratch needed by one Patch. Note
/// admission, dispatch, render, and silence run on the hard real-time path with
/// bounded, preallocated work: no allocation, destruction, locking, blocking,
/// I/O, logging, formatting, panic, or unwind. Asset metadata methods below are
/// explicitly worker-only.
pub trait PreparedInstrument: Send {
    /// Returns the immutable Patch identity prepared into this instrument.
    fn patch_id(&self) -> PatchId;

    /// Static prepared note mapping; unmapped/empty pads do not consume note admission.
    fn accepts_note(&self, _note: u8) -> bool {
        true
    }

    /// Worker-side collection for instruments composed of several resident assets.
    fn prepared_asset_footprints(&self) -> Vec<&PreparedAssetFootprint> {
        self.prepared_asset_footprint().into_iter().collect()
    }

    /// Worker-side summaries; never called by the callback.
    fn prepared_sample_visualizations(&self) -> Vec<PreparedSampleVisualization> {
        self.prepared_sample_visualization()
            .cloned()
            .into_iter()
            .collect()
    }

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
    ) -> Result<(), PreparedInstrumentError>;

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
    private_bytes: usize,
}

impl PreparedAssetFootprint {
    pub const fn new(reference: AssetReference, preparation_key: u64, bytes: usize) -> Self {
        Self {
            reference,
            preparation_key,
            bytes,
            private_bytes: 0,
        }
    }

    /// Media copied into independent upstream instances cannot be deduplicated.
    pub const fn with_private_bytes(mut self, bytes: usize) -> Self {
        self.private_bytes = bytes;
        self
    }
    pub const fn private_bytes(&self) -> usize {
        self.private_bytes
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
    RenderRejected,
    ScalarLayoutMismatch,
    InvalidFrameCapacity,
}

impl fmt::Display for PreparedInstrumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMidiKind { kind } => {
                write!(formatter, "prepared instrument does not support {kind:?}")
            }
            Self::DispatchRejected => formatter.write_str("prepared instrument rejected MIDI"),
            Self::RenderRejected => formatter.write_str("prepared instrument rejected rendering"),
            Self::ScalarLayoutMismatch => {
                formatter.write_str("prepared instrument scalar layout is incompatible")
            }
            Self::InvalidFrameCapacity => {
                formatter.write_str("prepared instrument frame capacity is incompatible")
            }
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
