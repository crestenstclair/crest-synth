use crate::kernel::PatchId;
use crate::real_time::{ParameterSnapshot, PatchAudioBlock, PatchEffectObservation, MAX_PATCHES};
use crate::synth::{EffectSlotId, PreparedEffectError, PreparedPostEffect};
use core::fmt;

pub(crate) struct PreparedPostEffectSlot {
    patch_id: PatchId,
    slot_id: EffectSlotId,
    scalar_count: usize,
    effect: Box<dyn PreparedPostEffect>,
    input_scratch: Vec<f32>,
}

impl PreparedPostEffectSlot {
    pub(crate) fn new(
        patch_id: PatchId,
        slot_id: EffectSlotId,
        scalar_count: usize,
        effect: Box<dyn PreparedPostEffect>,
        input_scratch: Vec<f32>,
    ) -> Self {
        Self {
            patch_id,
            slot_id,
            scalar_count,
            effect,
            input_scratch,
        }
    }
}

/// Fixed-capacity Patch-aligned ownership of zero or one prepared effect per Patch.
pub struct PreparedPostEffectRack {
    patch_count: usize,
    patch_ids: [Option<PatchId>; MAX_PATCHES],
    slots: [Option<PreparedPostEffectSlot>; MAX_PATCHES],
}

impl PreparedPostEffectRack {
    pub(crate) fn from_slots(
        patch_count: usize,
        patch_ids: [Option<PatchId>; MAX_PATCHES],
        slots: [Option<PreparedPostEffectSlot>; MAX_PATCHES],
    ) -> Self {
        Self {
            patch_count,
            patch_ids,
            slots,
        }
    }

    pub const fn patch_count(&self) -> usize {
        self.patch_count
    }

    pub fn patch_id(&self, index: usize) -> Option<PatchId> {
        self.patch_ids.get(index).copied().flatten()
    }

    pub fn slot_id(&self, index: usize) -> Option<EffectSlotId> {
        self.slots
            .get(index)
            .and_then(Option::as_ref)
            .map(|slot| slot.slot_id)
    }

    pub fn scalar_count(&self, index: usize) -> Option<usize> {
        self.slots
            .get(index)
            .and_then(Option::as_ref)
            .map(|slot| slot.scalar_count)
    }

    pub fn matches_parameters(&self, parameters: &ParameterSnapshot) -> bool {
        parameters.patch_count() == self.patch_count
            && parameters
                .patches()
                .iter()
                .enumerate()
                .all(|(index, parameters)| {
                    if parameters.patch_id() != self.patch_id(index) {
                        return false;
                    }
                    match self.slots[index].as_ref() {
                        None => !parameters.effect().is_active(),
                        Some(slot) => {
                            parameters.effect().slot_id() == Some(slot.slot_id)
                                && parameters.effect().scalar_count() == slot.scalar_count
                        }
                    }
                })
    }

    /// Processes each configured matching stem exactly once, in place.
    pub fn process(
        &mut self,
        block: &mut PatchAudioBlock,
        parameters: &ParameterSnapshot,
        observations: &mut [PatchEffectObservation; MAX_PATCHES],
    ) -> Result<(), EffectRackProcessError> {
        observations.fill(PatchEffectObservation::EMPTY);
        if block.patch_count() != self.patch_count {
            return Err(EffectRackProcessError::PatchCountMismatch);
        }
        if !self.matches_parameters(parameters) {
            return Err(EffectRackProcessError::ParameterLayoutMismatch);
        }
        let frame_count = block.frame_count();
        for (index, observation) in observations[..self.patch_count].iter_mut().enumerate() {
            let patch_id =
                self.patch_ids[index].ok_or(EffectRackProcessError::InactivePatch { index })?;
            if block.storage()[index].patch_id() != Some(patch_id) {
                return Err(EffectRackProcessError::StemIdentityMismatch { index });
            }
            let Some(slot) = self.slots[index].as_mut() else {
                continue;
            };
            if slot.patch_id != patch_id {
                return Err(EffectRackProcessError::StemIdentityMismatch { index });
            }
            let stem = block
                .stem_mut(index, patch_id)
                .ok_or(EffectRackProcessError::StemIdentityMismatch { index })?;
            let sample_count = frame_count.saturating_mul(2);
            if sample_count > slot.input_scratch.len() {
                return Err(EffectRackProcessError::FrameCapacityExceeded);
            }
            slot.input_scratch[..sample_count].copy_from_slice(stem);
            slot.effect
                .process(stem, frame_count, parameters.patches()[index].effect())
                .map_err(|source| EffectRackProcessError::Effect { patch_id, source })?;
            *observation = PatchEffectObservation::measured(
                patch_id,
                &slot.input_scratch[..sample_count],
                stem,
            );
        }
        Ok(())
    }
}

impl fmt::Debug for PreparedPostEffectRack {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedPostEffectRack")
            .field("patch_count", &self.patch_count)
            .field("patch_ids", &&self.patch_ids[..self.patch_count])
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectRackProcessError {
    PatchCountMismatch,
    ParameterLayoutMismatch,
    InactivePatch {
        index: usize,
    },
    StemIdentityMismatch {
        index: usize,
    },
    FrameCapacityExceeded,
    Effect {
        patch_id: PatchId,
        source: PreparedEffectError,
    },
}

impl fmt::Display for EffectRackProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PatchCountMismatch => formatter.write_str("effect rack Patch count mismatch"),
            Self::ParameterLayoutMismatch => {
                formatter.write_str("effect rack parameter layout mismatch")
            }
            Self::InactivePatch { index } => {
                write!(formatter, "effect rack Patch {index} is inactive")
            }
            Self::StemIdentityMismatch { index } => {
                write!(
                    formatter,
                    "effect rack stem {index} has the wrong Patch identity"
                )
            }
            Self::FrameCapacityExceeded => {
                formatter.write_str("effect rack frame capacity was exceeded")
            }
            Self::Effect { patch_id, source } => {
                write!(formatter, "Patch {patch_id} effect failed: {source}")
            }
        }
    }
}

impl std::error::Error for EffectRackProcessError {}
