use crate::kernel::midi_message::MidiMessage;
use crate::kernel::patch_id::PatchId;
use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters, MAX_PATCHES};
use crate::real_time::patch_audio_block::PatchAudioBlock;
use crate::synth::prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
use core::fmt;

pub(crate) struct PreparedEngineSlot {
    patch_id: PatchId,
    scalar_count: usize,
    instrument: Box<dyn PreparedInstrument>,
}

impl PreparedEngineSlot {
    pub(crate) fn new(
        patch_id: PatchId,
        scalar_count: usize,
        instrument: Box<dyn PreparedInstrument>,
    ) -> Self {
        Self {
            patch_id,
            scalar_count,
            instrument,
        }
    }
}

/// Fixed-capacity ordered ownership of one prepared instrument per Patch.
///
/// The rack contains no engine-specific identity or branch. Every callback
/// operation is bounded by `MAX_PATCHES`, and rendering uses only the matching
/// caller-owned stem.
pub struct PreparedEngineRack {
    patch_count: usize,
    slots: [Option<PreparedEngineSlot>; MAX_PATCHES],
}

impl PreparedEngineRack {
    pub(crate) fn from_slots(
        patch_count: usize,
        slots: [Option<PreparedEngineSlot>; MAX_PATCHES],
    ) -> Self {
        Self { patch_count, slots }
    }

    /// Returns the number of active prepared slots.
    pub const fn patch_count(&self) -> usize {
        self.patch_count
    }

    /// Returns the Patch identity at one canonical slot index.
    pub fn patch_id(&self, index: usize) -> Option<PatchId> {
        self.slots
            .get(index)
            .and_then(Option::as_ref)
            .map(|slot| slot.patch_id)
    }

    /// Returns the fixed descriptor-ordered Scalar layout for one slot.
    pub fn scalar_count(&self, index: usize) -> Option<usize> {
        self.slots
            .get(index)
            .and_then(Option::as_ref)
            .map(|slot| slot.scalar_count)
    }

    /// Returns whether a parameter snapshot has the exact rack revision and
    /// ordered Patch identities.
    pub fn matches_parameters(&self, parameters: &ParameterSnapshot) -> bool {
        parameters.patch_count() == self.patch_count
            && parameters
                .patches()
                .iter()
                .enumerate()
                .all(|(index, patch)| {
                    patch.patch_id() == self.patch_id(index)
                        && Some(patch.instrument().count()) == self.scalar_count(index)
                })
    }

    /// Routes one message to only the slot with the exact Patch identity.
    pub fn dispatch(
        &mut self,
        patch_id: PatchId,
        message: MidiMessage,
        parameters: &RtPatchParameters,
    ) -> Result<(), RackDispatchError> {
        let Some(slot) = self.find_slot_mut(patch_id) else {
            return Err(RackDispatchError::UnknownPatch { patch_id });
        };
        if parameters.patch_id() != Some(patch_id)
            || parameters.instrument().count() != slot.scalar_count
        {
            return Err(RackDispatchError::ParameterLayoutMismatch { patch_id });
        }
        slot.instrument
            .dispatch(message, parameters)
            .map_err(|source| RackDispatchError::Instrument { patch_id, source })
    }

    /// Silences only the slot with the exact Patch identity.
    pub fn all_notes_off_for(&mut self, patch_id: PatchId) -> Result<(), RackDispatchError> {
        let Some(slot) = self.find_slot_mut(patch_id) else {
            return Err(RackDispatchError::UnknownPatch { patch_id });
        };
        slot.instrument.all_notes_off();
        Ok(())
    }

    /// Silences every active instrument with work bounded by `MAX_PATCHES`.
    pub fn all_notes_off(&mut self) {
        for slot in self.slots[..self.patch_count].iter_mut().flatten() {
            slot.instrument.all_notes_off();
        }
    }

    /// Clears and fills every exact caller-owned Patch stem once.
    pub fn render(
        &mut self,
        block: &mut PatchAudioBlock,
        parameters: &ParameterSnapshot,
    ) -> Result<(), RackRenderError> {
        if block.patch_count() != self.patch_count {
            return Err(RackRenderError::PatchCountMismatch {
                rack: self.patch_count,
                stems: block.patch_count(),
            });
        }
        if !self.matches_parameters(parameters) {
            return Err(RackRenderError::ParameterLayoutMismatch);
        }

        for (index, slot) in self.slots[..self.patch_count].iter().enumerate() {
            let Some(slot) = slot else {
                return Err(RackRenderError::InactiveSlot { index });
            };
            if block.storage()[index].patch_id() != Some(slot.patch_id) {
                return Err(RackRenderError::StemIdentityMismatch {
                    index,
                    expected: slot.patch_id,
                    actual: block.storage()[index].patch_id(),
                });
            }
        }

        block.clear();
        let frame_count = block.frame_count();
        for (index, slot) in self.slots[..self.patch_count].iter_mut().enumerate() {
            let Some(slot) = slot else {
                return Err(RackRenderError::InactiveSlot { index });
            };
            let Some(stem) = block.stem_mut(index, slot.patch_id) else {
                return Err(RackRenderError::StemIdentityMismatch {
                    index,
                    expected: slot.patch_id,
                    actual: None,
                });
            };
            slot.instrument
                .render(stem, frame_count, &parameters.patches()[index]);
        }
        Ok(())
    }

    fn find_slot_mut(&mut self, patch_id: PatchId) -> Option<&mut PreparedEngineSlot> {
        self.slots[..self.patch_count]
            .iter_mut()
            .flatten()
            .find(|slot| slot.patch_id == patch_id)
    }

    /// Exchanges the still-live prepared instrument from the superseded rack
    /// into this replacement at every slot the structural delta leaves
    /// unchanged, so sounding voices survive block-boundary activation
    /// (WP10 voice carry-over).
    ///
    /// Callback-safe: the work is bounded by `MAX_PATCHES` pointer-sized
    /// `mem::swap`s with no allocation, deallocation, locking, blocking, or
    /// destruction. The freshly prepared (never sounded) instruments ride
    /// into the superseded rack, which retires off-callback as before.
    ///
    /// `exclude` names the one Patch whose engine identity the delta changed;
    /// that slot keeps its fresh instrument (the permitted local restart).
    /// Every exchange requires exact PatchId and scalar-layout agreement at
    /// the same index — a non-matching position keeps its fresh instrument
    /// rather than substituting anything.
    pub(crate) fn carry_live_instruments_from(
        &mut self,
        superseded: &mut Self,
        exclude: Option<PatchId>,
    ) {
        if self.patch_count != superseded.patch_count {
            return;
        }
        for index in 0..self.patch_count {
            let (Some(target), Some(source)) =
                (self.slots[index].as_mut(), superseded.slots[index].as_mut())
            else {
                continue;
            };
            if target.patch_id != source.patch_id
                || Some(target.patch_id) == exclude
                || target.scalar_count != source.scalar_count
            {
                continue;
            }
            core::mem::swap(&mut target.instrument, &mut source.instrument);
        }
    }
}

impl fmt::Debug for PreparedEngineRack {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedEngineRack")
            .field("patch_count", &self.patch_count)
            .field(
                "patch_ids",
                &&self.slots[..self.patch_count]
                    .iter()
                    .map(|slot| slot.as_ref().map(|slot| slot.patch_id))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Fixed-size callback status for targeted rack dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RackDispatchError {
    UnknownPatch {
        patch_id: PatchId,
    },
    Instrument {
        patch_id: PatchId,
        source: PreparedInstrumentError,
    },
    ParameterLayoutMismatch {
        patch_id: PatchId,
    },
}

impl fmt::Display for RackDispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPatch { patch_id } => {
                write!(formatter, "prepared rack has no Patch {patch_id}")
            }
            Self::Instrument { patch_id, source } => {
                write!(
                    formatter,
                    "prepared Patch {patch_id} rejected MIDI: {source}"
                )
            }
            Self::ParameterLayoutMismatch { patch_id } => write!(
                formatter,
                "prepared Patch {patch_id} received an incompatible parameter projection"
            ),
        }
    }
}

impl std::error::Error for RackDispatchError {}

/// Fixed-size callback status for caller-owned stem validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RackRenderError {
    PatchCountMismatch {
        rack: usize,
        stems: usize,
    },
    InactiveSlot {
        index: usize,
    },
    StemIdentityMismatch {
        index: usize,
        expected: PatchId,
        actual: Option<PatchId>,
    },
    ParameterLayoutMismatch,
}

impl fmt::Display for RackRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PatchCountMismatch { rack, stems } => {
                write!(
                    formatter,
                    "prepared rack has {rack} Patches but block has {stems} stems"
                )
            }
            Self::InactiveSlot { index } => {
                write!(formatter, "prepared rack slot {index} is empty")
            }
            Self::StemIdentityMismatch {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "prepared rack slot {index} expects Patch {expected}, got {actual:?}"
            ),
            Self::ParameterLayoutMismatch => {
                formatter.write_str("prepared rack parameter layout is incompatible")
            }
        }
    }
}

impl std::error::Error for RackRenderError {}

#[cfg(test)]
mod tests {
    use super::{RackDispatchError, RackRenderError};

    #[test]
    fn callback_statuses_are_copyable_and_have_no_destructors() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<RackDispatchError>();
        assert_copy::<RackRenderError>();
        assert!(!core::mem::needs_drop::<RackDispatchError>());
        assert!(!core::mem::needs_drop::<RackRenderError>());
    }
}
