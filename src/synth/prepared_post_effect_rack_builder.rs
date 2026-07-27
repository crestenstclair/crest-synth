use crate::kernel::PatchId;
use crate::real_time::{PreparedPostEffectRack, PreparedPostEffectSlot, MAX_PATCHES};
use crate::synth::{
    EffectCapabilityError, EffectCapabilityId, EffectCapabilityRegistry, EffectPreparationError,
    EffectPreparer, EffectSlotId, Patch,
};
use core::fmt;

pub struct PreparedPostEffectRackBuilder;

impl PreparedPostEffectRackBuilder {
    pub fn build(
        patches: &[Patch],
        registry: &EffectCapabilityRegistry,
        preparers: &[Box<dyn EffectPreparer>],
        sample_rate: f32,
        max_frames: usize,
    ) -> Result<PreparedPostEffectRack, EffectRackPreparationError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(EffectRackPreparationError::InvalidSampleRate);
        }
        if max_frames == 0 {
            return Err(EffectRackPreparationError::InvalidFrameCapacity);
        }
        if patches.len() > MAX_PATCHES {
            return Err(EffectRackPreparationError::PatchCapacityExceeded {
                count: patches.len(),
                capacity: MAX_PATCHES,
            });
        }
        for (index, patch) in patches.iter().enumerate() {
            if patches[..index]
                .iter()
                .any(|prior| prior.id() == patch.id())
            {
                return Err(EffectRackPreparationError::DuplicatePatchId {
                    patch_id: patch.id(),
                });
            }
            registry
                .validate_patch_effects(patch.post_effects())
                .map_err(|source| EffectRackPreparationError::InvalidConfiguration {
                    patch_id: patch.id(),
                    source,
                })?;
        }
        for (index, preparer) in preparers.iter().enumerate() {
            if preparers[..index]
                .iter()
                .any(|prior| prior.capability_id() == preparer.capability_id())
            {
                return Err(EffectRackPreparationError::DuplicatePreparer {
                    capability_id: preparer.capability_id().clone(),
                });
            }
            if registry.descriptor(preparer.capability_id()).is_none() {
                return Err(EffectRackPreparationError::ExtraPreparer {
                    capability_id: preparer.capability_id().clone(),
                });
            }
        }
        for descriptor in registry.descriptors() {
            if !preparers
                .iter()
                .any(|preparer| preparer.capability_id() == descriptor.id())
            {
                return Err(EffectRackPreparationError::MissingPreparer {
                    capability_id: descriptor.id().clone(),
                });
            }
        }

        let mut patch_ids = [None; MAX_PATCHES];
        let mut slots = std::array::from_fn(|_| None);
        let sample_capacity = max_frames
            .checked_mul(2)
            .ok_or(EffectRackPreparationError::InvalidFrameCapacity)?;
        for (index, patch) in patches.iter().enumerate() {
            patch_ids[index] = Some(patch.id());
            let Some(config) = patch.post_effects().first() else {
                continue;
            };
            let descriptor = registry
                .descriptor(config.capability_id())
                .expect("validated effect config is registered");
            let preparer = preparers
                .iter()
                .find(|preparer| preparer.capability_id() == config.capability_id())
                .expect("exact preparer registration was validated");
            let effect = preparer
                .prepare(patch.id(), config, sample_rate, max_frames)
                .map_err(|source| EffectRackPreparationError::Effect {
                    patch_id: patch.id(),
                    source,
                })?;
            if effect.patch_id() != patch.id() || effect.slot_id() != config.slot_id() {
                return Err(EffectRackPreparationError::PreparedIdentityMismatch {
                    expected_patch_id: patch.id(),
                    actual_patch_id: effect.patch_id(),
                    expected_slot_id: config.slot_id(),
                    actual_slot_id: effect.slot_id(),
                });
            }
            let mut scratch = Vec::new();
            scratch.try_reserve_exact(sample_capacity).map_err(|_| {
                EffectRackPreparationError::StorageAllocationFailed {
                    patch_id: patch.id(),
                }
            })?;
            scratch.resize(sample_capacity, 0.0);
            slots[index] = Some(PreparedPostEffectSlot::new(
                patch.id(),
                config.slot_id(),
                descriptor.scalar_parameter_count(),
                effect,
                scratch,
            ));
        }
        Ok(PreparedPostEffectRack::from_slots(
            patches.len(),
            patch_ids,
            slots,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectRackPreparationError {
    InvalidSampleRate,
    InvalidFrameCapacity,
    PatchCapacityExceeded {
        count: usize,
        capacity: usize,
    },
    DuplicatePatchId {
        patch_id: PatchId,
    },
    DuplicatePreparer {
        capability_id: EffectCapabilityId,
    },
    MissingPreparer {
        capability_id: EffectCapabilityId,
    },
    ExtraPreparer {
        capability_id: EffectCapabilityId,
    },
    InvalidConfiguration {
        patch_id: PatchId,
        source: EffectCapabilityError,
    },
    Effect {
        patch_id: PatchId,
        source: EffectPreparationError,
    },
    PreparedIdentityMismatch {
        expected_patch_id: PatchId,
        actual_patch_id: PatchId,
        expected_slot_id: EffectSlotId,
        actual_slot_id: EffectSlotId,
    },
    StorageAllocationFailed {
        patch_id: PatchId,
    },
}

impl fmt::Display for EffectRackPreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate => formatter.write_str("effect rack sample rate is invalid"),
            Self::InvalidFrameCapacity => formatter.write_str("effect rack frame capacity is invalid"),
            Self::PatchCapacityExceeded { count, capacity } => {
                write!(formatter, "effect rack has {count} Patches; capacity is {capacity}")
            }
            Self::DuplicatePatchId { patch_id } => write!(formatter, "duplicate Patch {patch_id}"),
            Self::DuplicatePreparer { capability_id } => write!(formatter, "duplicate effect preparer {capability_id}"),
            Self::MissingPreparer { capability_id } => write!(formatter, "missing effect preparer {capability_id}"),
            Self::ExtraPreparer { capability_id } => write!(formatter, "extra effect preparer {capability_id}"),
            Self::InvalidConfiguration { patch_id, source } => write!(formatter, "Patch {patch_id} effect config is invalid: {source}"),
            Self::Effect { patch_id, source } => write!(formatter, "Patch {patch_id} effect preparation failed: {source}"),
            Self::PreparedIdentityMismatch { expected_patch_id, actual_patch_id, expected_slot_id, actual_slot_id } => write!(formatter, "prepared effect identity {actual_patch_id}/{actual_slot_id} does not match {expected_patch_id}/{expected_slot_id}"),
            Self::StorageAllocationFailed { patch_id } => write!(formatter, "Patch {patch_id} effect scratch allocation failed"),
        }
    }
}

impl std::error::Error for EffectRackPreparationError {}
