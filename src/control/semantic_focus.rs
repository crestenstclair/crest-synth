use crate::control::{PatchControlId, TopLevelContext};
use crate::kernel::PatchId;
use crate::mixer::bus_id::BusId;
use crate::mixer::global_parameters::GlobalParameter;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::MixerTrackParameter;
use crate::synth::{CapabilityId, EffectCapabilityId, EffectSlotId, ParameterId};
use core::fmt;
use serde::{Deserialize, Serialize};

/// Stable graphical surfaces independent of host layout or rectangle placement.
///
/// Four are persistent — two mains and two sides — and one, [`Self::PatchDetail`],
/// is subordinate: it is entered only from a PatchMain path whose control
/// resolves a [`PatchDetailSubject`], it is never the resting surface of a
/// context, and leaving it restores the exact origin. One detail surface
/// identity serves both instrument and effect subjects, because the surface is
/// the shell and the subject supplies the content.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SurfaceId {
    PatchMain,
    PatchUtility,
    PatchDetail,
    MixerMain,
    MixerInspector,
}

impl SurfaceId {
    pub const ALL: [Self; 5] = [
        Self::PatchMain,
        Self::PatchUtility,
        Self::PatchDetail,
        Self::MixerMain,
        Self::MixerInspector,
    ];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }

    pub const fn context(self) -> TopLevelContext {
        match self {
            Self::PatchMain | Self::PatchUtility | Self::PatchDetail => TopLevelContext::Patch,
            Self::MixerMain | Self::MixerInspector => TopLevelContext::Mixer,
        }
    }

    pub const fn main_for(context: TopLevelContext) -> Self {
        match context {
            TopLevelContext::Patch => Self::PatchMain,
            TopLevelContext::Mixer => Self::MixerMain,
        }
    }

    /// Returns the context's *persistent* side surface. `PatchDetail` is never
    /// a result: it is subordinate, so it has no resting place to be returned
    /// as a context's side.
    pub const fn side_for(context: TopLevelContext) -> Self {
        match context {
            TopLevelContext::Patch => Self::PatchUtility,
            TopLevelContext::Mixer => Self::MixerInspector,
        }
    }

    pub const fn is_main(self) -> bool {
        matches!(self, Self::PatchMain | Self::MixerMain)
    }

    pub const fn is_persistent_side(self) -> bool {
        matches!(self, Self::PatchUtility | Self::MixerInspector)
    }

    /// Reports whether this surface is subordinate: entered from a main path,
    /// remembered by exactly one [`ReturnPath`], and left through `Return`.
    pub const fn is_subordinate(self) -> bool {
        matches!(self, Self::PatchDetail)
    }

    /// Reports whether the surface can be entered from a main path at all.
    ///
    /// This is the single predicate `EnterSurface` admission is decided by, so
    /// the two persistent sides and the one subordinate surface cannot drift
    /// apart from the action vocabulary that reaches them.
    pub const fn is_enterable(self) -> bool {
        self.is_persistent_side() || self.is_subordinate()
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::PatchMain => "PATCH",
            Self::PatchUtility => "UTILITY",
            Self::PatchDetail => "DETAIL",
            Self::MixerMain => "MIXER",
            Self::MixerInspector => "INSPECTOR",
        }
    }
}

/// The capability whose schema fills the polymorphic PATCH detail surface.
///
/// The subject names a capability identity **only**. It carries no section
/// list, control list, value, label, accent, geometry, or descriptor copy:
/// those resolve from the installed descriptor at projection time and would
/// otherwise become a second schema. `Effect` carries the exact occupied slot
/// identity, so two positions holding the same registry entry are distinct
/// subjects.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PatchDetailSubject {
    Instrument {
        capability_id: CapabilityId,
    },
    Effect {
        slot_id: EffectSlotId,
        capability_id: EffectCapabilityId,
    },
}

impl PatchDetailSubject {
    /// Names one instrument capability as the detail subject.
    pub const fn instrument(capability_id: CapabilityId) -> Self {
        Self::Instrument { capability_id }
    }

    /// Names one *exact occupied slot* as the detail subject.
    pub const fn effect(slot_id: EffectSlotId, capability_id: EffectCapabilityId) -> Self {
        Self::Effect {
            slot_id,
            capability_id,
        }
    }

    /// Returns the occupied slot identity for an effect subject.
    pub const fn slot_id(&self) -> Option<EffectSlotId> {
        match self {
            Self::Effect { slot_id, .. } => Some(*slot_id),
            Self::Instrument { .. } => None,
        }
    }

    /// Returns the subject as the capability identity a `FocusPath` carries.
    pub fn focus_capability_id(&self) -> FocusCapabilityId {
        match self {
            Self::Instrument { capability_id } => {
                FocusCapabilityId::Instrument(capability_id.clone())
            }
            Self::Effect { capability_id, .. } => {
                FocusCapabilityId::Effect(capability_id.clone())
            }
        }
    }
}

/// Stable identity of one MIXER control, independent of section coordinates.
///
/// Sends and returns are index-addressed: a send is the pair of one
/// `MixerTrackId` and one `BusId`; return rows are addressed by `BusId` and,
/// for the occupying entry's scalar rows, a descriptor `ParameterId`. No
/// variant names a concrete effect, and a return's contents changing never
/// changes its identity.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MixerControlId {
    Track {
        track_id: MixerTrackId,
        parameter: MixerTrackParameter,
    },
    Send {
        track_id: MixerTrackId,
        bus: BusId,
    },
    ReturnOccupancy {
        bus: BusId,
    },
    ReturnLevel {
        bus: BusId,
    },
    ReturnEffect {
        bus: BusId,
        parameter: ParameterId,
    },
    Global {
        parameter: GlobalParameter,
    },
}

impl MixerControlId {
    pub const fn track_id(&self) -> Option<MixerTrackId> {
        match self {
            Self::Track { track_id, .. } | Self::Send { track_id, .. } => Some(*track_id),
            Self::ReturnOccupancy { .. }
            | Self::ReturnLevel { .. }
            | Self::ReturnEffect { .. }
            | Self::Global { .. } => None,
        }
    }

    pub const fn track_parameter(&self) -> Option<MixerTrackParameter> {
        match self {
            Self::Track { parameter, .. } => Some(*parameter),
            Self::Send { .. }
            | Self::ReturnOccupancy { .. }
            | Self::ReturnLevel { .. }
            | Self::ReturnEffect { .. }
            | Self::Global { .. } => None,
        }
    }

    /// Returns the addressed bus for send and return identities.
    pub const fn bus(&self) -> Option<BusId> {
        match self {
            Self::Send { bus, .. }
            | Self::ReturnOccupancy { bus }
            | Self::ReturnLevel { bus }
            | Self::ReturnEffect { bus, .. } => Some(*bus),
            Self::Track { .. } | Self::Global { .. } => None,
        }
    }

    pub const fn global_parameter(&self) -> Option<GlobalParameter> {
        match self {
            Self::Global { parameter } => Some(*parameter),
            Self::Track { .. }
            | Self::Send { .. }
            | Self::ReturnOccupancy { .. }
            | Self::ReturnLevel { .. }
            | Self::ReturnEffect { .. } => None,
        }
    }
}

/// Optional capability identity carried by a semantic PATCH focus path.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum FocusCapabilityId {
    Instrument(CapabilityId),
    Effect(EffectCapabilityId),
}

/// Stable control identity used across PATCH, MIXER, and side-surface roots.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum SemanticControlId {
    Patch(PatchControlId),
    Mixer(MixerControlId),
    SurfaceRoot,
}

impl SemanticControlId {
    pub const fn as_mixer_track_id(&self) -> Option<MixerTrackId> {
        match self {
            Self::Mixer(MixerControlId::Track { track_id, .. }) => Some(*track_id),
            Self::Mixer(_) | Self::Patch(_) | Self::SurfaceRoot => None,
        }
    }
}

/// A malformed semantic path rejected before it can enter canonical state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusPathError {
    ContextSurfaceMismatch,
    ControlSurfaceMismatch,
    PatchIdentityMismatch,
    CapabilityIdentityMismatch,
    ModalIdentityUnavailable,
}

impl fmt::Display for FocusPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ContextSurfaceMismatch => "focus context and surface are incompatible",
            Self::ControlSurfaceMismatch => "focus control and surface are incompatible",
            Self::PatchIdentityMismatch => "focus path has an invalid Patch identity shape",
            Self::CapabilityIdentityMismatch => {
                "focus path has an invalid capability identity shape"
            }
            Self::ModalIdentityUnavailable => "Phase 2 focus paths cannot contain a modal identity",
        })
    }
}

impl std::error::Error for FocusPathError {}

/// The single canonical semantic location of interaction.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusPath {
    context: TopLevelContext,
    surface: SurfaceId,
    patch_id: Option<PatchId>,
    capability_id: Option<FocusCapabilityId>,
    control_id: SemanticControlId,
    modal_id: Option<String>,
}

impl FocusPath {
    pub fn patch_main(
        patch_id: PatchId,
        capability_id: Option<FocusCapabilityId>,
        control_id: PatchControlId,
    ) -> Self {
        Self {
            context: TopLevelContext::Patch,
            surface: SurfaceId::PatchMain,
            patch_id: Some(patch_id),
            capability_id,
            control_id: SemanticControlId::Patch(control_id),
            modal_id: None,
        }
    }

    pub const fn patch_utility(patch_id: PatchId, control_id: PatchControlId) -> Self {
        Self {
            context: TopLevelContext::Patch,
            surface: SurfaceId::PatchUtility,
            patch_id: Some(patch_id),
            capability_id: None,
            control_id: SemanticControlId::Patch(control_id),
            modal_id: None,
        }
    }

    /// One control on the subordinate PATCH detail surface.
    ///
    /// The path carries the subject's capability identity, so a row belonging
    /// to one capability is never mistaken for the same-named row of another.
    pub const fn patch_detail(
        patch_id: PatchId,
        capability_id: FocusCapabilityId,
        control_id: PatchControlId,
    ) -> Self {
        Self {
            context: TopLevelContext::Patch,
            surface: SurfaceId::PatchDetail,
            patch_id: Some(patch_id),
            capability_id: Some(capability_id),
            control_id: SemanticControlId::Patch(control_id),
            modal_id: None,
        }
    }

    pub const fn mixer_track(track_id: MixerTrackId, parameter: MixerTrackParameter) -> Self {
        Self {
            context: TopLevelContext::Mixer,
            surface: SurfaceId::MixerMain,
            patch_id: None,
            capability_id: None,
            control_id: SemanticControlId::Mixer(MixerControlId::Track {
                track_id,
                parameter,
            }),
            modal_id: None,
        }
    }

    /// One indexed send row on the MIXER Inspector: `(track, bus)`.
    pub const fn mixer_send(track_id: MixerTrackId, bus: BusId) -> Self {
        Self::mixer_inspector_control(MixerControlId::Send { track_id, bus }, None)
    }

    /// One bus return's occupancy row on the MIXER Inspector.
    pub const fn mixer_return_occupancy(bus: BusId) -> Self {
        Self::mixer_inspector_control(MixerControlId::ReturnOccupancy { bus }, None)
    }

    /// One bus return's return-owned level row on the MIXER Inspector.
    pub const fn mixer_return_level(bus: BusId) -> Self {
        Self::mixer_inspector_control(MixerControlId::ReturnLevel { bus }, None)
    }

    /// One occupying registry entry's scalar row on the MIXER Inspector. The
    /// path carries the occupant's registry identity so a different occupant's
    /// row is a different focus target.
    pub const fn mixer_return_effect(
        bus: BusId,
        parameter: ParameterId,
        capability_id: EffectCapabilityId,
    ) -> Self {
        Self::mixer_inspector_control(
            MixerControlId::ReturnEffect { bus, parameter },
            Some(FocusCapabilityId::Effect(capability_id)),
        )
    }

    const fn mixer_inspector_control(
        control: MixerControlId,
        capability_id: Option<FocusCapabilityId>,
    ) -> Self {
        Self {
            context: TopLevelContext::Mixer,
            surface: SurfaceId::MixerInspector,
            patch_id: None,
            capability_id,
            control_id: SemanticControlId::Mixer(control),
            modal_id: None,
        }
    }

    pub const fn mixer_global(parameter: GlobalParameter) -> Self {
        Self::mixer_inspector_control(MixerControlId::Global { parameter }, None)
    }

    pub fn side_root(surface: SurfaceId) -> Result<Self, FocusPathError> {
        if !surface.is_persistent_side() {
            return Err(FocusPathError::ControlSurfaceMismatch);
        }
        Ok(Self {
            context: surface.context(),
            surface,
            patch_id: None,
            capability_id: None,
            control_id: SemanticControlId::SurfaceRoot,
            modal_id: None,
        })
    }

    /// Revalidates a deserialized or externally constructed path shape.
    pub fn validate(&self) -> Result<(), FocusPathError> {
        if self.surface.context() != self.context {
            return Err(FocusPathError::ContextSurfaceMismatch);
        }
        if self.modal_id.is_some() {
            return Err(FocusPathError::ModalIdentityUnavailable);
        }
        match (&self.surface, &self.control_id) {
            (SurfaceId::PatchMain, SemanticControlId::Patch(control)) => {
                if self.patch_id.is_none() {
                    return Err(FocusPathError::PatchIdentityMismatch);
                }
                // The five Utility identities and the PatchMain order are
                // disjoint by declaration; the split is decided in exactly one
                // place so the two surfaces cannot both claim a row.
                if control.is_utility() {
                    return Err(FocusPathError::ControlSurfaceMismatch);
                }
            }
            (SurfaceId::PatchUtility, SemanticControlId::Patch(control))
                if control.is_utility() =>
            {
                if self.patch_id.is_none() || self.capability_id.is_some() {
                    return Err(FocusPathError::PatchIdentityMismatch);
                }
            }
            // The detail surface hosts the subject capability's own descriptor
            // rows: an instrument capability parameter, or one occupant's
            // parameter at its exact slot. The path always carries the
            // subject's capability identity, matched to the row's kind.
            (
                SurfaceId::PatchDetail,
                SemanticControlId::Patch(PatchControlId::Capability(_)),
            ) => {
                if self.patch_id.is_none() {
                    return Err(FocusPathError::PatchIdentityMismatch);
                }
                if !matches!(self.capability_id, Some(FocusCapabilityId::Instrument(_))) {
                    return Err(FocusPathError::CapabilityIdentityMismatch);
                }
            }
            (SurfaceId::PatchDetail, SemanticControlId::Patch(PatchControlId::Effect(..))) => {
                if self.patch_id.is_none() {
                    return Err(FocusPathError::PatchIdentityMismatch);
                }
                if !matches!(self.capability_id, Some(FocusCapabilityId::Effect(_))) {
                    return Err(FocusPathError::CapabilityIdentityMismatch);
                }
            }
            (
                SurfaceId::MixerMain,
                SemanticControlId::Mixer(MixerControlId::Track { parameter, .. }),
            ) => {
                if !MixerTrackParameter::MAIN.contains(parameter)
                    || self.patch_id.is_some()
                    || self.capability_id.is_some()
                {
                    return Err(FocusPathError::ControlSurfaceMismatch);
                }
            }
            (
                SurfaceId::MixerInspector,
                SemanticControlId::Mixer(
                    MixerControlId::Send { .. }
                    | MixerControlId::ReturnOccupancy { .. }
                    | MixerControlId::ReturnLevel { .. },
                ),
            ) => {
                if self.patch_id.is_some() || self.capability_id.is_some() {
                    return Err(FocusPathError::ControlSurfaceMismatch);
                }
            }
            (
                SurfaceId::MixerInspector,
                SemanticControlId::Mixer(MixerControlId::ReturnEffect { .. }),
            ) => {
                if self.patch_id.is_some()
                    || !matches!(self.capability_id, Some(FocusCapabilityId::Effect(_)))
                {
                    return Err(FocusPathError::CapabilityIdentityMismatch);
                }
            }
            (
                SurfaceId::MixerInspector,
                SemanticControlId::Mixer(MixerControlId::Global { .. }),
            ) => {
                if self.patch_id.is_some() || self.capability_id.is_some() {
                    return Err(FocusPathError::PatchIdentityMismatch);
                }
            }
            (surface, SemanticControlId::SurfaceRoot) if surface.is_persistent_side() => {
                if self.patch_id.is_some() || self.capability_id.is_some() {
                    return Err(FocusPathError::CapabilityIdentityMismatch);
                }
            }
            _ => return Err(FocusPathError::ControlSurfaceMismatch),
        }
        Ok(())
    }

    pub const fn context(&self) -> TopLevelContext {
        self.context
    }

    pub const fn surface(&self) -> SurfaceId {
        self.surface
    }

    pub const fn patch_id(&self) -> Option<PatchId> {
        self.patch_id
    }

    pub const fn capability_id(&self) -> Option<&FocusCapabilityId> {
        self.capability_id.as_ref()
    }

    pub const fn control_id(&self) -> &SemanticControlId {
        &self.control_id
    }

    pub fn modal_id(&self) -> Option<&str> {
        self.modal_id.as_deref()
    }
}

/// Exact main-surface origin restored after leaving a subordinate surface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnPath {
    origin: FocusPath,
    entered_surface: SurfaceId,
}

impl ReturnPath {
    /// Captures one exact main-surface origin before entering a subordinate
    /// or persistent side surface.
    ///
    /// The origin must be a *main* path in the entered surface's own context.
    /// That single requirement is what makes the surfaces non-nesting: a path
    /// already on `PatchUtility` or `PatchDetail` is not main, so it can never
    /// become a second stacked origin.
    pub fn new(origin: FocusPath, entered_surface: SurfaceId) -> Result<Self, FocusPathError> {
        origin.validate()?;
        if !origin.surface().is_main()
            || !entered_surface.is_enterable()
            || origin.context() != entered_surface.context()
        {
            return Err(FocusPathError::ContextSurfaceMismatch);
        }
        Ok(Self {
            origin,
            entered_surface,
        })
    }

    pub const fn origin(&self) -> &FocusPath {
        &self.origin
    }

    pub const fn entered_surface(&self) -> SurfaceId {
        self.entered_surface
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FocusPath, FocusPathError, MixerControlId, PatchDetailSubject, ReturnPath,
        SemanticControlId, SurfaceId,
    };
    use crate::control::PatchControlId;
    use crate::kernel::PatchId;
    use crate::mixer::global_parameters::GlobalParameter;

    #[test]
    fn five_surfaces_are_context_compatible_and_layout_neutral() {
        use crate::control::TopLevelContext;

        assert_eq!(SurfaceId::surface_descriptor().len(), 5);
        assert_eq!(SurfaceId::PatchMain.context(), TopLevelContext::Patch);
        assert_eq!(
            SurfaceId::MixerInspector.context(),
            crate::control::TopLevelContext::Mixer
        );
        assert!(SurfaceId::PatchUtility.is_persistent_side());
        assert!(SurfaceId::MixerMain.is_main());

        // Every surface is exactly one of main, persistent side, or
        // subordinate — a surface that were two at once would let the reducer
        // treat it as a resting place and a return target simultaneously.
        for surface in SurfaceId::ALL {
            let roles = usize::from(surface.is_main())
                + usize::from(surface.is_persistent_side())
                + usize::from(surface.is_subordinate());
            assert_eq!(roles, 1, "{surface:?} must hold exactly one surface role");
            assert_eq!(
                surface.is_enterable(),
                !surface.is_main(),
                "{surface:?}: entry lands on every non-main surface and no main one"
            );
        }
    }

    /// The subordinate surface belongs to PATCH, is never a context's resting
    /// place, and is reachable only by entering it from a main path.
    #[test]
    fn the_detail_surface_is_subordinate_and_never_a_resting_surface() {
        use crate::control::TopLevelContext;

        assert_eq!(SurfaceId::PatchDetail.context(), TopLevelContext::Patch);
        assert!(SurfaceId::PatchDetail.is_subordinate());
        assert!(!SurfaceId::PatchDetail.is_main());
        assert!(!SurfaceId::PatchDetail.is_persistent_side());

        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            assert_ne!(SurfaceId::main_for(context), SurfaceId::PatchDetail);
            assert_ne!(SurfaceId::side_for(context), SurfaceId::PatchDetail);
        }
    }

    /// One detail surface identity serves both subject kinds, and an effect
    /// subject is distinguished by its *slot*, not just its capability — so
    /// two positions holding the same registry entry are distinct subjects.
    #[test]
    fn a_detail_subject_names_a_capability_and_an_effect_names_its_exact_slot() {
        use crate::synth::{CapabilityId, EffectCapabilityId, EffectSlotId};

        let capability = EffectCapabilityId::new("effect.fixture").unwrap();
        let first = PatchDetailSubject::effect(EffectSlotId::new(1).unwrap(), capability.clone());
        let second = PatchDetailSubject::effect(EffectSlotId::new(2).unwrap(), capability);
        assert_ne!(
            first, second,
            "the same registry entry in two slots must be two subjects"
        );
        assert_eq!(first.slot_id(), Some(EffectSlotId::new(1).unwrap()));

        let instrument =
            PatchDetailSubject::instrument(CapabilityId::new("instrument.fixture").unwrap());
        assert_eq!(instrument.slot_id(), None);
        assert_ne!(
            instrument.focus_capability_id(),
            first.focus_capability_id()
        );
    }

    #[test]
    fn focus_paths_round_trip_stable_semantic_ids() {
        let patch_id = PatchId::new(7).unwrap();
        let patch = FocusPath::patch_main(patch_id, None, PatchControlId::Engine);
        let global = FocusPath::mixer_global(GlobalParameter::MasterGainDb);
        let patch_json = serde_json::to_string(&patch).unwrap();
        let global_json = serde_json::to_string(&global).unwrap();
        assert_eq!(
            serde_json::from_str::<FocusPath>(&patch_json).unwrap(),
            patch
        );
        assert_eq!(
            serde_json::from_str::<FocusPath>(&global_json).unwrap(),
            global
        );
        assert_eq!(patch.patch_id(), Some(patch_id));
        assert_eq!(
            global.control_id(),
            &SemanticControlId::Mixer(MixerControlId::Global {
                parameter: GlobalParameter::MasterGainDb
            })
        );
    }

    #[test]
    fn send_and_return_paths_are_index_addressed_inspector_targets() {
        use crate::mixer::bus_id::BusId;
        let send = FocusPath::mixer_send(
            crate::mixer::mixer_track_id::MixerTrackId::default(),
            BusId::new(5).unwrap(),
        );
        assert!(send.validate().is_ok());
        assert_eq!(send.surface(), SurfaceId::MixerInspector);

        let occupancy = FocusPath::mixer_return_occupancy(BusId::new(7).unwrap());
        let level = FocusPath::mixer_return_level(BusId::new(7).unwrap());
        assert!(occupancy.validate().is_ok());
        assert!(level.validate().is_ok());
        assert_ne!(occupancy, level);

        let scalar = FocusPath::mixer_return_effect(
            BusId::new(0).unwrap(),
            crate::synth::ParameterId::new("amount").unwrap(),
            crate::synth::EffectCapabilityId::new("effect.fixture").unwrap(),
        );
        assert!(scalar.validate().is_ok());
        let json = serde_json::to_string(&scalar).unwrap();
        assert_eq!(serde_json::from_str::<FocusPath>(&json).unwrap(), scalar);
    }

    #[test]
    fn return_path_requires_exact_same_context_main_origin() {
        let origin = FocusPath::patch_main(PatchId::new(1).unwrap(), None, PatchControlId::Engine);
        let path = ReturnPath::new(origin.clone(), SurfaceId::PatchUtility).unwrap();
        assert_eq!(path.origin(), &origin);
        assert_eq!(path.entered_surface(), SurfaceId::PatchUtility);
        assert_eq!(
            ReturnPath::new(origin, SurfaceId::MixerInspector),
            Err(FocusPathError::ContextSurfaceMismatch)
        );
        assert_eq!(
            FocusPath::side_root(SurfaceId::PatchMain),
            Err(FocusPathError::ControlSurfaceMismatch)
        );
    }
}
