use crate::mixer::bus_id::{BusId, MAX_BUS_RETURNS};
use crate::synth::{EffectCapabilityId, EffectCapabilityRegistry, EffectSlotId, PostEffectConfig};
use core::fmt;

/// Failures that can occur while preparing bounded effect storage.
///
/// Relocated verbatim from the retired `global_effects_processor` module: the
/// four original variants are the preparation vocabulary the return-rack and
/// mix-engine preparers still speak. `InvalidReturnLevel` extends that
/// vocabulary for the return-owned output level introduced with [`BusReturn`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectError {
    /// The sample rate is not finite and strictly positive.
    InvalidSampleRate,
    /// The configured maximum block size is zero.
    InvalidMaxFrames,
    /// The maximum delay is not finite and strictly positive.
    InvalidMaxDelayMilliseconds,
    /// The return level is not finite or is outside 0.0..=1.0.
    InvalidReturnLevel,
    /// The implementation could not reserve all storage before processing.
    StorageAllocationFailed,
}

impl fmt::Display for EffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidSampleRate => "sample rate must be finite and greater than zero",
            Self::InvalidMaxFrames => "maximum frame count must be greater than zero",
            Self::InvalidMaxDelayMilliseconds => {
                "maximum delay must be finite and greater than zero milliseconds"
            }
            Self::InvalidReturnLevel => "return level must be finite and in 0.0..=1.0",
            Self::StorageAllocationFailed => "global effect storage allocation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for EffectError {}

/// Bounds and edit steps shared by every return-owned output level.
///
/// Copied exactly from the retired `ReverbReturn`/`DelayReturn` global
/// descriptors so the generalization changes no bound: 0.0..=1.0, fine 0.01,
/// coarse 0.1. All eight returns share this one descriptor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReturnLevelDescriptor {
    minimum: f32,
    maximum: f32,
    default: f32,
    fine_step: f32,
    coarse_step: f32,
}

impl ReturnLevelDescriptor {
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

    pub fn contains(self, value: f32) -> bool {
        value.is_finite() && (self.minimum..=self.maximum).contains(&value)
    }
}

/// The one shared descriptor for every return-owned level.
///
/// The default of 0.5 is the retired production initial value of both
/// `reverbReturn` and `delayReturn`.
pub const RETURN_LEVEL_DESCRIPTOR: ReturnLevelDescriptor = ReturnLevelDescriptor {
    minimum: 0.0,
    maximum: 1.0,
    default: 0.5,
    fine_step: 0.01,
    coarse_step: 0.1,
};

/// A violation of one bus-return invariant.
#[derive(Clone, Debug, PartialEq)]
pub enum BusReturnError {
    /// The return level is not finite or is outside the shared descriptor bounds.
    InvalidReturnLevel { value: f32 },
    /// The requested registry entry is not installed in the shared registry.
    UnknownRegistryEntry { id: EffectCapabilityId },
    /// The registry rejected building a default configuration for the entry.
    ConfigurationRejected { id: EffectCapabilityId },
    /// A value edit targeted a return that holds no effect.
    Unoccupied { id: BusId },
}

impl fmt::Display for BusReturnError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReturnLevel { value } => write!(
                formatter,
                "return level must be finite and in {}..={}, got {value}",
                RETURN_LEVEL_DESCRIPTOR.minimum(),
                RETURN_LEVEL_DESCRIPTOR.maximum()
            ),
            Self::UnknownRegistryEntry { id } => {
                write!(formatter, "registry entry {id} is not installed")
            }
            Self::ConfigurationRejected { id } => {
                write!(formatter, "registry entry {id} rejected its default config")
            }
            Self::Unoccupied { id } => {
                write!(formatter, "bus return {id} holds no effect to edit")
            }
        }
    }
}

impl std::error::Error for BusReturnError {}

/// One bounded routing destination holding zero or one registry effect and its
/// own output level.
///
/// The effect is drawn from the same `EffectCapabilityRegistry` that fills
/// Patch effect slots; there is no role marker, send-suitability flag, or
/// return-only registry (FR-010). The return level is owned by the return
/// rather than declared as an effect descriptor scalar, so replacing the
/// occupying effect neither resets nor loses it (C-BR-10, R-04). An unoccupied
/// return contributes silence and never passes its accumulated input through
/// (C-BR-6).
#[derive(Clone, Debug, PartialEq)]
pub struct BusReturn {
    id: BusId,
    effect: Option<PostEffectConfig>,
    return_level: f32,
}

impl BusReturn {
    /// Creates one validated return; the level is checked against the shared
    /// descriptor and never clamped.
    pub fn new(
        id: BusId,
        effect: Option<PostEffectConfig>,
        return_level: f32,
    ) -> Result<Self, BusReturnError> {
        if !RETURN_LEVEL_DESCRIPTOR.contains(return_level) {
            return Err(BusReturnError::InvalidReturnLevel {
                value: return_level,
            });
        }
        Ok(Self {
            id,
            effect,
            return_level,
        })
    }

    /// Creates the canonical unoccupied return for one bus.
    pub fn unoccupied(id: BusId) -> Self {
        Self {
            id,
            effect: None,
            return_level: RETURN_LEVEL_DESCRIPTOR.default(),
        }
    }

    pub const fn id(&self) -> BusId {
        self.id
    }

    pub const fn effect(&self) -> Option<&PostEffectConfig> {
        self.effect.as_ref()
    }

    pub const fn is_occupied(&self) -> bool {
        self.effect.is_some()
    }

    pub const fn return_level(&self) -> f32 {
        self.return_level
    }

    /// Replaces the occupying effect while preserving the return-owned level.
    pub fn with_effect(mut self, effect: Option<PostEffectConfig>) -> Self {
        self.effect = effect;
        self
    }

    /// Replaces the return level after validating it against the shared descriptor.
    pub fn with_return_level(mut self, return_level: f32) -> Result<Self, BusReturnError> {
        if !RETURN_LEVEL_DESCRIPTOR.contains(return_level) {
            return Err(BusReturnError::InvalidReturnLevel {
                value: return_level,
            });
        }
        self.return_level = return_level;
        Ok(self)
    }

    /// Returns the slot identity a return derives for its prepared effect.
    ///
    /// Slot ids are non-zero, so the bus index shifts by one. The value is a
    /// preparation detail; the routing identity remains the `BusId`.
    pub const fn slot_id(id: BusId) -> EffectSlotId {
        match EffectSlotId::new(id.value() as u16 + 1) {
            Ok(slot) => slot,
            // Unreachable: every BusId value is in 0..=7, so value + 1 is non-zero.
            Err(_) => panic!("bus-derived slot ids are always non-zero"),
        }
    }
}

/// The canonical fixed bank of eight bus returns.
///
/// All eight returns exist before any Patch is installed and remain
/// addressable whether occupied or empty. Array position is a bounded storage
/// detail whose canonical semantic identity is the matching `BusId` (B-2).
#[derive(Clone, Debug, PartialEq)]
pub struct BusReturnBank {
    returns: [BusReturn; MAX_BUS_RETURNS],
}

impl BusReturnBank {
    pub fn returns(&self) -> &[BusReturn; MAX_BUS_RETURNS] {
        &self.returns
    }

    pub fn bus_return(&self, id: BusId) -> &BusReturn {
        &self.returns[id.index()]
    }

    /// The `SetReturnOccupancy(BusId, Option<RegistryEntryId>)` domain
    /// operation: occupies, replaces, or clears one return's contents.
    ///
    /// The entry resolves through the same shared registry that fills Patch
    /// effect slots — no role filter is applied (FR-010). The return-owned
    /// level is deliberately untouched by every occupancy transition
    /// (C-BR-10). WP06 wires this operation to a semantic action.
    pub fn set_return_occupancy(
        &mut self,
        registry: &EffectCapabilityRegistry,
        id: BusId,
        entry: Option<&EffectCapabilityId>,
    ) -> Result<(), BusReturnError> {
        let effect = match entry {
            None => None,
            Some(entry_id) => {
                let descriptor = registry.descriptor(entry_id).ok_or_else(|| {
                    BusReturnError::UnknownRegistryEntry {
                        id: entry_id.clone(),
                    }
                })?;
                let config = descriptor
                    .default_config(BusReturn::slot_id(id))
                    .map_err(|_| BusReturnError::ConfigurationRejected {
                        id: entry_id.clone(),
                    })?;
                Some(config)
            }
        };
        let current = self.returns[id.index()].clone();
        self.returns[id.index()] = current.with_effect(effect);
        Ok(())
    }

    /// Replaces one return-owned level after validation.
    pub fn set_return_level(&mut self, id: BusId, return_level: f32) -> Result<(), BusReturnError> {
        let current = self.returns[id.index()].clone();
        self.returns[id.index()] = current.with_return_level(return_level)?;
        Ok(())
    }

    /// Replaces the occupying instance's configuration values in place.
    ///
    /// This is the scalar-value edit path: the replacement must preserve the
    /// occupant's identity (slot id and registry entry). Occupancy transitions
    /// — changing what exists — go through `set_return_occupancy` and the
    /// prepared structural path instead.
    pub fn replace_occupant_values(
        &mut self,
        id: BusId,
        config: PostEffectConfig,
    ) -> Result<(), BusReturnError> {
        let current = self.returns[id.index()].clone();
        let Some(existing) = current.effect() else {
            return Err(BusReturnError::Unoccupied { id });
        };
        if existing.slot_id() != config.slot_id()
            || existing.capability_id() != config.capability_id()
        {
            return Err(BusReturnError::ConfigurationRejected {
                id: config.capability_id().clone(),
            });
        }
        self.returns[id.index()] = current.with_effect(Some(config));
        Ok(())
    }
}

impl Default for BusReturnBank {
    /// Eight unoccupied returns; occupancy is composition, not identity.
    fn default() -> Self {
        Self {
            returns: BusId::ALL.map(BusReturn::unoccupied),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BusReturn, BusReturnBank, BusReturnError, EffectError, RETURN_LEVEL_DESCRIPTOR};
    use crate::adapter::production_effects::production_effect_registry;
    use crate::mixer::bus_id::BusId;
    use crate::synth::EffectCapabilityId;

    #[test]
    fn preparation_errors_have_actionable_messages() {
        assert_eq!(
            EffectError::InvalidSampleRate.to_string(),
            "sample rate must be finite and greater than zero"
        );
        assert_eq!(
            EffectError::InvalidMaxFrames.to_string(),
            "maximum frame count must be greater than zero"
        );
        assert_eq!(
            EffectError::InvalidMaxDelayMilliseconds.to_string(),
            "maximum delay must be finite and greater than zero milliseconds"
        );
        assert_eq!(
            EffectError::InvalidReturnLevel.to_string(),
            "return level must be finite and in 0.0..=1.0"
        );
        assert_eq!(
            EffectError::StorageAllocationFailed.to_string(),
            "global effect storage allocation failed"
        );
    }

    #[test]
    fn return_level_descriptor_matches_the_retired_return_bounds() {
        assert_eq!(RETURN_LEVEL_DESCRIPTOR.minimum(), 0.0);
        assert_eq!(RETURN_LEVEL_DESCRIPTOR.maximum(), 1.0);
        assert_eq!(RETURN_LEVEL_DESCRIPTOR.default(), 0.5);
        assert_eq!(RETURN_LEVEL_DESCRIPTOR.fine_step(), 0.01);
        assert_eq!(RETURN_LEVEL_DESCRIPTOR.coarse_step(), 0.1);
    }

    #[test]
    fn return_level_is_validated_and_never_clamped() {
        let bus = BusId::new(2).unwrap();
        assert!(BusReturn::new(bus, None, 1.0).is_ok());
        assert!(BusReturn::new(bus, None, 0.0).is_ok());
        assert_eq!(
            BusReturn::new(bus, None, 1.1),
            Err(BusReturnError::InvalidReturnLevel { value: 1.1 })
        );
        assert!(matches!(
            BusReturn::new(bus, None, f32::NAN),
            Err(BusReturnError::InvalidReturnLevel { .. })
        ));
        assert!(BusReturn::unoccupied(bus).with_return_level(-0.1).is_err());
    }

    #[test]
    fn occupancy_can_be_set_replaced_and_cleared_on_each_of_eight_returns() {
        let registry = production_effect_registry().unwrap();
        let first = EffectCapabilityId::new("effect.reverb").unwrap();
        let second = EffectCapabilityId::new("effect.delay").unwrap();
        let mut bank = BusReturnBank::default();

        for bus in BusId::ALL {
            assert!(!bank.bus_return(bus).is_occupied());

            bank.set_return_occupancy(&registry, bus, Some(&first))
                .unwrap();
            assert_eq!(
                bank.bus_return(bus).effect().unwrap().capability_id(),
                &first
            );

            bank.set_return_occupancy(&registry, bus, Some(&second))
                .unwrap();
            assert_eq!(
                bank.bus_return(bus).effect().unwrap().capability_id(),
                &second
            );

            bank.set_return_occupancy(&registry, bus, None).unwrap();
            assert!(!bank.bus_return(bus).is_occupied());
            assert_eq!(bank.bus_return(bus).id(), bus);
        }
    }

    #[test]
    fn return_level_survives_every_occupancy_transition() {
        let registry = production_effect_registry().unwrap();
        let entry = EffectCapabilityId::new("effect.chorus").unwrap();
        let bus = BusId::new(5).unwrap();
        let mut bank = BusReturnBank::default();
        bank.set_return_level(bus, 0.75).unwrap();

        bank.set_return_occupancy(&registry, bus, Some(&entry))
            .unwrap();
        assert_eq!(bank.bus_return(bus).return_level(), 0.75);

        bank.set_return_occupancy(&registry, bus, None).unwrap();
        assert_eq!(bank.bus_return(bus).return_level(), 0.75);
    }

    #[test]
    fn unknown_registry_entries_are_rejected_without_substitution() {
        let registry = production_effect_registry().unwrap();
        let unknown = EffectCapabilityId::new("effect.absent").unwrap();
        let mut bank = BusReturnBank::default();

        assert_eq!(
            bank.set_return_occupancy(&registry, BusId::default(), Some(&unknown)),
            Err(BusReturnError::UnknownRegistryEntry { id: unknown })
        );
        assert!(!bank.bus_return(BusId::default()).is_occupied());
    }

    #[test]
    fn derived_slot_identity_is_stable_and_non_zero() {
        for bus in BusId::ALL {
            assert_eq!(BusReturn::slot_id(bus).value(), u16::from(bus.value()) + 1);
        }
    }
}
