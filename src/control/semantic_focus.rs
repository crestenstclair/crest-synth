use crate::control::{PatchControlId, TopLevelContext};
use crate::kernel::PatchId;
use crate::mixer::bus_id::BusId;
use crate::mixer::global_parameters::GlobalParameter;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::MixerTrackParameter;
use crate::synth::{CapabilityId, EffectCapabilityId, ParameterId};
use core::fmt;
use serde::{Deserialize, Serialize};

/// Stable graphical surfaces independent of host layout or rectangle placement.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SurfaceId {
    PatchMain,
    PatchUtility,
    MixerMain,
    MixerInspector,
}

impl SurfaceId {
    pub const ALL: [Self; 4] = [
        Self::PatchMain,
        Self::PatchUtility,
        Self::MixerMain,
        Self::MixerInspector,
    ];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }

    pub const fn context(self) -> TopLevelContext {
        match self {
            Self::PatchMain | Self::PatchUtility => TopLevelContext::Patch,
            Self::MixerMain | Self::MixerInspector => TopLevelContext::Mixer,
        }
    }

    pub const fn main_for(context: TopLevelContext) -> Self {
        match context {
            TopLevelContext::Patch => Self::PatchMain,
            TopLevelContext::Mixer => Self::MixerMain,
        }
    }

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

    pub const fn label(self) -> &'static str {
        match self {
            Self::PatchMain => "PATCH",
            Self::PatchUtility => "UTILITY",
            Self::MixerMain => "MIXER",
            Self::MixerInspector => "INSPECTOR",
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
                if matches!(control, PatchControlId::Output(_)) {
                    return Err(FocusPathError::ControlSurfaceMismatch);
                }
            }
            (SurfaceId::PatchUtility, SemanticControlId::Patch(PatchControlId::Output(_))) => {
                if self.patch_id.is_none() || self.capability_id.is_some() {
                    return Err(FocusPathError::PatchIdentityMismatch);
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
    pub fn new(origin: FocusPath, entered_surface: SurfaceId) -> Result<Self, FocusPathError> {
        origin.validate()?;
        if !origin.surface().is_main()
            || !entered_surface.is_persistent_side()
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
        FocusPath, FocusPathError, MixerControlId, ReturnPath, SemanticControlId, SurfaceId,
    };
    use crate::control::PatchControlId;
    use crate::kernel::PatchId;
    use crate::mixer::global_parameters::GlobalParameter;

    #[test]
    fn four_surfaces_are_context_compatible_and_layout_neutral() {
        assert_eq!(SurfaceId::surface_descriptor().len(), 4);
        assert_eq!(
            SurfaceId::PatchMain.context(),
            crate::control::TopLevelContext::Patch
        );
        assert_eq!(
            SurfaceId::MixerInspector.context(),
            crate::control::TopLevelContext::Mixer
        );
        assert!(SurfaceId::PatchUtility.is_persistent_side());
        assert!(SurfaceId::MixerMain.is_main());
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
