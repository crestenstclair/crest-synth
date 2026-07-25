use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::graph_revision::GraphRevision;
use core::fmt;

/// The maximum number of Patch parameter values carried across the real-time
/// boundary.
///
/// SoundFont playback is addressed through MIDI's sixteen bounded channels, so
/// the callback never needs dynamically sized Patch storage.
pub const MAX_PATCHES: usize = 16;

/// The fixed-size audio parameters for one active Patch.
///
/// The value is copyable and owns no heap storage. An absent Patch identity is
/// the canonical inactive value used for unused ParameterSnapshot entries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RtPatchParameters {
    patch_id: Option<PatchId>,
    parameters: ChannelParameters,
}

impl RtPatchParameters {
    /// Copies one active Patch's identity and validated mixer parameters into a
    /// real-time-safe value.
    pub const fn new(patch_id: PatchId, parameters: ChannelParameters) -> Self {
        Self {
            patch_id: Some(patch_id),
            parameters,
        }
    }

    /// Returns whether this entry contains one active Patch.
    pub const fn is_active(&self) -> bool {
        self.patch_id.is_some()
    }

    /// Returns the active Patch identity, or None for unused storage.
    pub const fn patch_id(&self) -> Option<PatchId> {
        self.patch_id
    }

    /// Returns the Patch's copied, validated channel parameters.
    pub const fn parameters(&self) -> &ChannelParameters {
        &self.parameters
    }

    fn inactive() -> Self {
        Self {
            patch_id: None,
            parameters: ChannelParameters::default(),
        }
    }
}

/// The reason a complete real-time parameter snapshot could not be built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterSnapshotError {
    /// The control state contains more Patch values than the fixed capacity.
    TooManyPatches { count: usize, capacity: usize },
    /// An inactive value was supplied inside the active Patch prefix.
    InactivePatch { index: usize },
}

impl fmt::Display for ParameterSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooManyPatches { count, capacity } => write!(
                formatter,
                "parameter snapshot has {count} patches; maximum is {capacity}"
            ),
            Self::InactivePatch { index } => {
                write!(formatter, "parameter snapshot patch {index} is inactive")
            }
        }
    }
}

impl std::error::Error for ParameterSnapshotError {}

/// The newest complete control state required for rendering.
///
/// Every field is fully owned, fixed-size, and copyable. Audio-thread readers
/// can therefore consume one coherent value without allocation, locking,
/// blocking, I/O, logging, or destruction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterSnapshot {
    generation: u64,
    graph_revision: GraphRevision,
    global: GlobalParameters,
    patch_count: usize,
    patches: [RtPatchParameters; MAX_PATCHES],
}

impl ParameterSnapshot {
    /// Copies a complete accepted control projection into bounded storage.
    ///
    /// Patch values in the input slice must all be active. Remaining array
    /// entries are initialized to the canonical inactive value.
    pub fn new(
        generation: u64,
        global: GlobalParameters,
        patches: &[RtPatchParameters],
    ) -> Result<Self, ParameterSnapshotError> {
        Self::for_graph(generation, GraphRevision::INITIAL, global, patches)
    }

    /// Copies one complete projection for a specific prepared graph revision.
    pub fn for_graph(
        generation: u64,
        graph_revision: GraphRevision,
        global: GlobalParameters,
        patches: &[RtPatchParameters],
    ) -> Result<Self, ParameterSnapshotError> {
        if patches.len() > MAX_PATCHES {
            return Err(ParameterSnapshotError::TooManyPatches {
                count: patches.len(),
                capacity: MAX_PATCHES,
            });
        }
        if let Some(index) = patches.iter().position(|patch| !patch.is_active()) {
            return Err(ParameterSnapshotError::InactivePatch { index });
        }

        let mut storage = [RtPatchParameters::inactive(); MAX_PATCHES];
        storage[..patches.len()].copy_from_slice(patches);

        Ok(Self {
            generation,
            graph_revision,
            global,
            patch_count: patches.len(),
            patches: storage,
        })
    }

    /// Returns the AppState generation from which this snapshot was projected.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the prepared graph revision targeted by this snapshot.
    pub const fn graph_revision(&self) -> GraphRevision {
        self.graph_revision
    }

    /// Returns the copied parameters for the one shared global mix.
    pub const fn global(&self) -> &GlobalParameters {
        &self.global
    }

    /// Returns the number of active entries in the fixed Patch array.
    pub const fn patch_count(&self) -> usize {
        self.patch_count
    }

    /// Returns exactly the active Patch parameter prefix.
    pub fn patches(&self) -> &[RtPatchParameters] {
        &self.patches[..self.patch_count]
    }

    /// Returns the complete fixed storage, including inactive entries.
    pub const fn storage(&self) -> &[RtPatchParameters; MAX_PATCHES] {
        &self.patches
    }

    /// Finds one active Patch without allocation.
    pub fn patch(&self, patch_id: PatchId) -> Option<&RtPatchParameters> {
        self.patches()
            .iter()
            .find(|patch| patch.patch_id() == Some(patch_id))
    }

    /// Returns whether this complete snapshot targets an exact graph revision
    /// and ordered Patch layout.
    pub fn is_compatible(
        &self,
        graph_revision: GraphRevision,
        ordered_patch_ids: &[PatchId],
    ) -> bool {
        self.graph_revision == graph_revision
            && self.patch_count == ordered_patch_ids.len()
            && self
                .patches()
                .iter()
                .zip(ordered_patch_ids)
                .all(|(patch, patch_id)| patch.patch_id() == Some(*patch_id))
    }

    /// Reuses identical bounded parameter values for a MIDI-only generation.
    pub(crate) const fn with_generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{ParameterSnapshot, ParameterSnapshotError, RtPatchParameters, MAX_PATCHES};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::graph_revision::GraphRevision;

    fn global() -> GlobalParameters {
        GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2).unwrap()
    }

    fn patch(id: u32, gain_db: f32) -> RtPatchParameters {
        RtPatchParameters::new(
            PatchId::new(id).unwrap(),
            ChannelParameters::new(gain_db, 0.0, 0.2, 0.1).unwrap(),
        )
    }

    #[test]
    fn copies_one_complete_accepted_control_projection() {
        let patches = [patch(1, -6.0), patch(2, -12.0)];
        let revision = GraphRevision::new(7).unwrap();
        let snapshot = ParameterSnapshot::for_graph(42, revision, global(), &patches).unwrap();

        assert_eq!(snapshot.generation(), 42);
        assert_eq!(snapshot.graph_revision(), revision);
        assert_eq!(snapshot.global(), &global());
        assert_eq!(snapshot.patch_count(), 2);
        assert_eq!(snapshot.patches(), &patches);
        assert_eq!(snapshot.patch(PatchId::new(2).unwrap()), Some(&patches[1]));
    }

    #[test]
    fn compatibility_requires_revision_count_and_exact_patch_order() {
        let revision = GraphRevision::new(8).unwrap();
        let snapshot = ParameterSnapshot::for_graph(
            42,
            revision,
            global(),
            &[patch(1, -6.0), patch(2, -12.0)],
        )
        .unwrap();

        assert!(snapshot.is_compatible(
            revision,
            &[PatchId::new(1).unwrap(), PatchId::new(2).unwrap()]
        ));
        assert!(!snapshot.is_compatible(
            GraphRevision::new(9).unwrap(),
            &[PatchId::new(1).unwrap(), PatchId::new(2).unwrap()]
        ));
        assert!(!snapshot.is_compatible(revision, &[PatchId::new(1).unwrap()]));
        assert!(!snapshot.is_compatible(
            revision,
            &[PatchId::new(2).unwrap(), PatchId::new(1).unwrap()]
        ));
    }

    #[test]
    fn unused_fixed_entries_are_inactive() {
        let snapshot = ParameterSnapshot::new(1, global(), &[patch(1, 0.0)]).unwrap();

        assert!(snapshot.storage()[0].is_active());
        assert!(snapshot.storage()[1..]
            .iter()
            .all(|entry| !entry.is_active()));
    }

    #[test]
    fn rejects_state_larger_than_the_compile_time_bound() {
        let patches = [patch(1, 0.0); MAX_PATCHES + 1];
        let error = ParameterSnapshot::new(1, global(), &patches).unwrap_err();

        assert_eq!(
            error,
            ParameterSnapshotError::TooManyPatches {
                count: MAX_PATCHES + 1,
                capacity: MAX_PATCHES
            }
        );
    }

    #[test]
    fn snapshot_and_patch_values_need_no_drop_or_dynamic_storage() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<ParameterSnapshot>();
        assert_copy::<RtPatchParameters>();
        assert!(!core::mem::needs_drop::<ParameterSnapshot>());
        assert_eq!(
            core::mem::size_of::<ParameterSnapshot>(),
            core::mem::size_of_val(&ParameterSnapshot::new(0, global(), &[]).unwrap())
        );
    }
}
