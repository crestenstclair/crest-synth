// path: src/patch/patch_manager.rs

//! PatchManager application service: patch CRUD lifecycle.
//!
//! This is a non-real-time application service. It owns patch identity
//! allocation and the patch collection, and is the boundary through which
//! patches are created, deleted, and reconfigured. It also enforces the
//! cross-aggregate invariant that MPE zones never overlap across patches,
//! since checking that requires visibility into every patch, not just one
//! — the `Patch` aggregate itself only validates a candidate zone against
//! whatever `other_zones` it is given.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::patch::patch::{
    ChannelMapping, MpeZone, Patch, PatchError, PatchEvent, PatchId, VoiceConfig,
};

/// Allocates identities for newly created patches.
///
/// Injected into `PatchManager` so tests can substitute a deterministic or
/// pre-seeded generator without touching production wiring.
pub trait PatchIdGenerator {
    /// Returns a fresh `PatchId`, distinct from every id previously
    /// returned by this generator.
    fn next_id(&mut self) -> PatchId;
}

/// A `PatchIdGenerator` that hands out sequential ids starting from a
/// configurable base.
#[derive(Debug, Clone)]
pub struct SequentialPatchIdGenerator {
    next: u32,
}

impl SequentialPatchIdGenerator {
    /// Builds a generator whose first id is `start`.
    pub fn new(start: u32) -> Self {
        Self { next: start }
    }
}

impl Default for SequentialPatchIdGenerator {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PatchIdGenerator for SequentialPatchIdGenerator {
    fn next_id(&mut self) -> PatchId {
        let id = PatchId::new(self.next);
        self.next = self.next.wrapping_add(1);
        id
    }
}

/// Storage for the collection of patches an application session owns.
///
/// This is the non-real-time side's patch collection; the audio thread
/// never touches it directly — parameter changes reach the audio thread
/// only via the `ParameterBridge` / `EventRing`.
pub trait PatchRepository {
    /// Inserts a newly created patch, keyed by its own id. Overwrites any
    /// existing patch with the same id.
    fn insert(&mut self, patch: Patch);

    /// Removes and returns the patch with `id`, if present.
    fn remove(&mut self, id: PatchId) -> Option<Patch>;

    /// Borrows the patch with `id`, if present.
    fn get(&self, id: PatchId) -> Option<&Patch>;

    /// Mutably borrows the patch with `id`, if present.
    fn get_mut(&mut self, id: PatchId) -> Option<&mut Patch>;

    /// The MPE zones currently claimed by every patch other than
    /// `excluding`. Used to check the cross-patch non-overlap invariant
    /// before committing a zone change.
    fn mpe_zones_excluding(&self, excluding: PatchId) -> Vec<MpeZone>;
}

/// An in-memory `PatchRepository` backed by a hash map.
#[derive(Debug, Default)]
pub struct InMemoryPatchRepository {
    patches: HashMap<PatchId, Patch>,
}

impl InMemoryPatchRepository {
    /// Builds an empty repository.
    pub fn new() -> Self {
        Self::default()
    }
}

impl PatchRepository for InMemoryPatchRepository {
    fn insert(&mut self, patch: Patch) {
        self.patches.insert(patch.id(), patch);
    }

    fn remove(&mut self, id: PatchId) -> Option<Patch> {
        self.patches.remove(&id)
    }

    fn get(&self, id: PatchId) -> Option<&Patch> {
        self.patches.get(&id)
    }

    fn get_mut(&mut self, id: PatchId) -> Option<&mut Patch> {
        self.patches.get_mut(&id)
    }

    fn mpe_zones_excluding(&self, excluding: PatchId) -> Vec<MpeZone> {
        self.patches
            .values()
            .filter(|patch| patch.id() != excluding)
            .filter_map(|patch| patch.mpe_zone())
            .collect()
    }
}

/// Errors raised by `PatchManager` operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchManagerError {
    /// No patch exists with the given id.
    PatchNotFound(PatchId),
    /// The requested change was rejected by the `Patch` aggregate.
    Patch(PatchError),
}

impl fmt::Display for PatchManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatchManagerError::PatchNotFound(id) => write!(f, "no patch found for {id}"),
            PatchManagerError::Patch(err) => write!(f, "{err}"),
        }
    }
}

impl Error for PatchManagerError {}

impl From<PatchError> for PatchManagerError {
    fn from(err: PatchError) -> Self {
        PatchManagerError::Patch(err)
    }
}

/// Default voice configuration assigned to a newly created patch: eight
/// voices of polyphony, a fast attack, no decay, full sustain, and a short
/// release. These values are known valid at compile time.
fn default_voice_config() -> VoiceConfig {
    VoiceConfig::try_new(8, 5.0, 0.0, 1.0, 50.0).expect("default voice config is valid")
}

/// Application service for patch CRUD: create/delete patches and edit
/// their voice, sample, modulation, and routing configuration.
///
/// Depends on a `PatchIdGenerator` and a `PatchRepository`, both injected
/// via the constructor so tests can substitute fakes without touching
/// production wiring. `Default` provides a convenience wiring for callers
/// who don't care (sequential ids, in-memory storage).
pub struct PatchManager {
    id_generator: Box<dyn PatchIdGenerator>,
    repository: Box<dyn PatchRepository>,
    default_mixer_strip: u32,
}

impl PatchManager {
    /// Builds a `PatchManager` from explicit collaborators.
    pub fn new(
        id_generator: Box<dyn PatchIdGenerator>,
        repository: Box<dyn PatchRepository>,
    ) -> Self {
        Self {
            id_generator,
            repository,
            default_mixer_strip: 0,
        }
    }

    /// `createPatch`: allocates a new `PatchId`, builds a `Patch` with
    /// default voice configuration bound to the default mixer strip, and
    /// stores it. Returns the newly assigned id.
    pub fn create_patch(&mut self) -> PatchId {
        let id = self.id_generator.next_id();
        let patch = Patch::new(id, self.default_mixer_strip, default_voice_config());
        self.repository.insert(patch);
        id
    }

    /// `deletePatch`: removes the patch with `id`. Fails with
    /// `PatchNotFound` if no such patch exists.
    pub fn delete_patch(&mut self, id: PatchId) -> Result<(), PatchManagerError> {
        self.repository
            .remove(id)
            .map(|_| ())
            .ok_or(PatchManagerError::PatchNotFound(id))
    }

    /// Edits a patch's channel mapping.
    pub fn set_mapping(
        &mut self,
        id: PatchId,
        mapping: ChannelMapping,
    ) -> Result<PatchEvent, PatchManagerError> {
        let patch = self
            .repository
            .get_mut(id)
            .ok_or(PatchManagerError::PatchNotFound(id))?;
        Ok(patch.set_mapping(mapping))
    }

    /// Edits a patch's MPE zone, enforcing that MPE zones never overlap
    /// across patches.
    pub fn set_mpe_zone(
        &mut self,
        id: PatchId,
        zone: Option<MpeZone>,
    ) -> Result<PatchEvent, PatchManagerError> {
        if self.repository.get(id).is_none() {
            return Err(PatchManagerError::PatchNotFound(id));
        }
        let other_zones = self.repository.mpe_zones_excluding(id);
        let patch = self
            .repository
            .get_mut(id)
            .ok_or(PatchManagerError::PatchNotFound(id))?;
        Ok(patch.set_mpe_zone(zone, &other_zones)?)
    }

    /// Edits a patch's sample set assignment.
    pub fn assign_sample_set(
        &mut self,
        id: PatchId,
        sample_set: Option<u32>,
    ) -> Result<PatchEvent, PatchManagerError> {
        let patch = self
            .repository
            .get_mut(id)
            .ok_or(PatchManagerError::PatchNotFound(id))?;
        Ok(patch.assign_sample_set(sample_set))
    }

    /// Edits a patch's voice configuration.
    pub fn set_voice_config(
        &mut self,
        id: PatchId,
        voice: VoiceConfig,
    ) -> Result<PatchEvent, PatchManagerError> {
        let patch = self
            .repository
            .get_mut(id)
            .ok_or(PatchManagerError::PatchNotFound(id))?;
        Ok(patch.set_voice_config(voice))
    }

    /// Borrows the patch with `id`, if it exists.
    pub fn get(&self, id: PatchId) -> Option<&Patch> {
        self.repository.get(id)
    }
}

impl Default for PatchManager {
    fn default() -> Self {
        Self::new(
            Box::new(SequentialPatchIdGenerator::default()),
            Box::new(InMemoryPatchRepository::new()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice() -> VoiceConfig {
        VoiceConfig::try_new(4, 1.0, 1.0, 0.5, 1.0).expect("valid voice config")
    }

    #[test]
    fn create_patch_returns_distinct_ids_and_stores_each_patch() {
        let mut manager = PatchManager::default();

        let first = manager.create_patch();
        let second = manager.create_patch();

        assert_ne!(first, second);
        assert!(manager.get(first).is_some());
        assert!(manager.get(second).is_some());
    }

    #[test]
    fn created_patch_has_default_voice_and_no_mapping() {
        let mut manager = PatchManager::default();

        let id = manager.create_patch();

        let patch = manager.get(id).expect("patch was just created");
        assert_eq!(patch.mapping(), ChannelMapping::none());
        assert_eq!(patch.mpe_zone(), None);
        assert_eq!(patch.sample_set(), None);
    }

    #[test]
    fn delete_patch_removes_it() {
        let mut manager = PatchManager::default();
        let id = manager.create_patch();

        let result = manager.delete_patch(id);

        assert!(result.is_ok());
        assert!(manager.get(id).is_none());
    }

    #[test]
    fn delete_patch_fails_for_unknown_id() {
        let mut manager = PatchManager::default();
        let unknown = PatchId::new(999);

        let result = manager.delete_patch(unknown);

        assert_eq!(result, Err(PatchManagerError::PatchNotFound(unknown)));
    }

    #[test]
    fn set_mapping_fails_for_unknown_patch() {
        let mut manager = PatchManager::default();
        let unknown = PatchId::new(999);
        let mapping = ChannelMapping::single(2).expect("valid channel");

        let result = manager.set_mapping(unknown, mapping);

        assert_eq!(result, Err(PatchManagerError::PatchNotFound(unknown)));
    }

    #[test]
    fn set_mapping_updates_the_patch() {
        let mut manager = PatchManager::default();
        let id = manager.create_patch();
        let mapping = ChannelMapping::single(2).expect("valid channel");

        let event = manager.set_mapping(id, mapping).expect("patch exists");

        assert_eq!(event, PatchEvent::MappingChanged { id });
        assert_eq!(manager.get(id).unwrap().mapping(), mapping);
    }

    #[test]
    fn set_mpe_zone_fails_for_unknown_patch() {
        let mut manager = PatchManager::default();
        let unknown = PatchId::new(999);
        let zone = MpeZone::try_new(0, 1, 6).expect("valid zone");

        let result = manager.set_mpe_zone(unknown, Some(zone));

        assert_eq!(result, Err(PatchManagerError::PatchNotFound(unknown)));
    }

    #[test]
    fn set_mpe_zone_rejects_overlap_with_another_patch() {
        let mut manager = PatchManager::default();
        let first = manager.create_patch();
        let second = manager.create_patch();
        let zone_a = MpeZone::try_new(0, 1, 6).expect("valid zone");
        manager
            .set_mpe_zone(first, Some(zone_a))
            .expect("no conflicts yet");

        let overlapping = MpeZone::try_new(8, 5, 4).expect("valid zone");
        let result = manager.set_mpe_zone(second, Some(overlapping));

        assert_eq!(
            result,
            Err(PatchManagerError::Patch(PatchError::OverlappingMpeZone))
        );
    }

    #[test]
    fn set_mpe_zone_succeeds_for_disjoint_zones() {
        let mut manager = PatchManager::default();
        let first = manager.create_patch();
        let second = manager.create_patch();
        let zone_a = MpeZone::try_new(0, 1, 6).expect("valid zone");
        manager
            .set_mpe_zone(first, Some(zone_a))
            .expect("no conflicts yet");

        let zone_b = MpeZone::try_new(8, 9, 6).expect("valid zone");
        let event = manager
            .set_mpe_zone(second, Some(zone_b))
            .expect("disjoint zone should succeed");

        assert_eq!(event, PatchEvent::ConfigChanged { id: second });
    }

    #[test]
    fn assign_sample_set_updates_the_patch() {
        let mut manager = PatchManager::default();
        let id = manager.create_patch();

        let event = manager
            .assign_sample_set(id, Some(3))
            .expect("patch exists");

        assert_eq!(event, PatchEvent::ConfigChanged { id });
        assert_eq!(manager.get(id).unwrap().sample_set(), Some(3));
    }

    #[test]
    fn set_voice_config_updates_the_patch() {
        let mut manager = PatchManager::default();
        let id = manager.create_patch();
        let new_voice = voice();

        let event = manager
            .set_voice_config(id, new_voice)
            .expect("patch exists");

        assert_eq!(event, PatchEvent::ConfigChanged { id });
        assert_eq!(manager.get(id).unwrap().voice(), new_voice);
    }

    #[test]
    fn sequential_id_generator_produces_increasing_ids() {
        let mut generator = SequentialPatchIdGenerator::new(5);

        assert_eq!(generator.next_id(), PatchId::new(5));
        assert_eq!(generator.next_id(), PatchId::new(6));
    }

    #[test]
    fn in_memory_repository_excludes_requested_patch_from_zone_list() {
        let mut repo = InMemoryPatchRepository::new();
        let zone = MpeZone::try_new(0, 1, 6).expect("valid zone");
        let mut patch_a = Patch::new(PatchId::new(1), 0, voice());
        patch_a.set_mpe_zone(Some(zone), &[]).expect("no conflicts");
        repo.insert(patch_a);

        let zones = repo.mpe_zones_excluding(PatchId::new(1));

        assert!(zones.is_empty());
    }
}
