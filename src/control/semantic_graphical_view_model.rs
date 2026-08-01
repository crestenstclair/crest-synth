use crate::control::{
    AppState, EngineSelectionFailure, EngineSelectionRequestId, EngineSelectionStatusKind,
    FocusCapabilityId, FocusPath, MixerControlId, PatchControlId, ReturnPath, SemanticResolver,
    SurfaceId, TopLevelContext, ValidAction,
};
use crate::kernel::PatchId;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::{MixerTrackParameter, MixerTrackParameterKind};
use crate::mixer::patch_output::PatchOutputParameter;
use crate::real_time::GraphRevision;
use crate::synth::{
    AssetReference, CapabilityId, ParameterKind, ParameterSpec, ParameterValue, PatchInteraction,
};
use core::fmt;
use serde::{Serialize, Serializer};
use std::collections::HashSet;
use std::sync::Arc;

/// Host-neutral semantic rendering kind for one control.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticControlKind {
    Continuous,
    Stepped,
    Choice,
    Toggle,
    Asset,
    Identity,
    Surface,
}

impl From<ParameterKind> for SemanticControlKind {
    fn from(kind: ParameterKind) -> Self {
        match kind {
            ParameterKind::Continuous => Self::Continuous,
            ParameterKind::Stepped => Self::Stepped,
            ParameterKind::Choice => Self::Choice,
            ParameterKind::Toggle => Self::Toggle,
            ParameterKind::Asset => Self::Asset,
        }
    }
}

/// Typed canonical value carried by a semantic control.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum SemanticControlValue {
    Scalar(f64),
    Parameter(ParameterValue),
    Asset(AssetReference),
    Identity(String),
    Summary(String),
}

/// Inclusive adjustment metadata when a control owns a numeric value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticNumericRange {
    minimum: f64,
    maximum: f64,
    fine_step: f64,
    coarse_step: f64,
}

impl SemanticNumericRange {
    pub const fn new(minimum: f64, maximum: f64, fine_step: f64, coarse_step: f64) -> Self {
        Self {
            minimum,
            maximum,
            fine_step,
            coarse_step,
        }
    }

    pub const fn minimum(self) -> f64 {
        self.minimum
    }

    pub const fn maximum(self) -> f64 {
        self.maximum
    }

    pub const fn fine_step(self) -> f64 {
        self.fine_step
    }

    pub const fn coarse_step(self) -> f64 {
        self.coarse_step
    }
}

/// Stable typed lifecycle status projected without prepared ownership.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticLifecycleStatus {
    kind: EngineSelectionStatusKind,
    label: String,
    request_id: Option<EngineSelectionRequestId>,
    graph_revision: GraphRevision,
    target_graph_revision: Option<GraphRevision>,
}

impl SemanticLifecycleStatus {
    pub const fn kind(&self) -> EngineSelectionStatusKind {
        self.kind
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn request_id(&self) -> Option<EngineSelectionRequestId> {
        self.request_id
    }

    pub const fn graph_revision(&self) -> GraphRevision {
        self.graph_revision
    }

    pub const fn target_graph_revision(&self) -> Option<GraphRevision> {
        self.target_graph_revision
    }
}

/// Closed semantic error code; healthy state is represented by no entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "failure", rename_all = "camelCase")]
pub enum SemanticErrorCode {
    EngineSelection(EngineSelectionFailure),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticError {
    code: SemanticErrorCode,
    label: String,
    source_path: Option<FocusPath>,
}

impl SemanticError {
    pub const fn code(&self) -> SemanticErrorCode {
        self.code
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn source_path(&self) -> Option<&FocusPath> {
        self.source_path.as_ref()
    }
}

/// One immutable semantic control with no widget or geometry state.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticControlViewModel {
    path: FocusPath,
    label: String,
    kind: SemanticControlKind,
    value: SemanticControlValue,
    numeric_range: Option<SemanticNumericRange>,
    unit: Option<String>,
    enabled: bool,
    visible: bool,
    focusable: bool,
    editable: bool,
    focused: bool,
    status: Option<SemanticLifecycleStatus>,
    error: Option<SemanticError>,
}

impl SemanticControlViewModel {
    pub const fn path(&self) -> &FocusPath {
        &self.path
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn kind(&self) -> SemanticControlKind {
        self.kind
    }

    pub const fn value(&self) -> &SemanticControlValue {
        &self.value
    }

    pub const fn numeric_range(&self) -> Option<SemanticNumericRange> {
        self.numeric_range
    }

    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    pub const fn visible(&self) -> bool {
        self.visible
    }

    pub const fn focusable(&self) -> bool {
        self.focusable
    }

    pub const fn editable(&self) -> bool {
        self.editable
    }

    pub const fn focused(&self) -> bool {
        self.focused
    }

    pub const fn status(&self) -> Option<&SemanticLifecycleStatus> {
        self.status.as_ref()
    }

    pub const fn error(&self) -> Option<&SemanticError> {
        self.error.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticSurfaceRole {
    Main,
    PersistentSide,
}

/// Typed, read-only canonical summary for one semantic surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SemanticSurfaceSummary {
    Patch {
        patch_id: PatchId,
        patch_name: String,
        capability_id: CapabilityId,
        effect_count: usize,
    },
    Mixer {
        patch_count: usize,
        global_parameter_count: usize,
    },
    PatchUtility {
        patch_id: PatchId,
        capability_id: CapabilityId,
        effect_count: usize,
    },
    MixerInspector {
        focused_control: MixerControlId,
        focused_track: MixerTrackId,
        patch_count: usize,
        routed_patches: Vec<SemanticRoutedPatch>,
    },
}

/// One Patch identity routed to the selected mixer track.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticRoutedPatch {
    patch_id: PatchId,
    patch_name: String,
}

impl SemanticRoutedPatch {
    pub const fn patch_id(&self) -> PatchId {
        self.patch_id
    }

    pub fn patch_name(&self) -> &str {
        &self.patch_name
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSurfaceViewModel {
    id: SurfaceId,
    label: String,
    role: SemanticSurfaceRole,
    controls: Vec<SemanticControlViewModel>,
    summary: SemanticSurfaceSummary,
}

impl SemanticSurfaceViewModel {
    pub const fn id(&self) -> SurfaceId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn role(&self) -> SemanticSurfaceRole {
        self.role
    }

    pub fn controls(&self) -> &[SemanticControlViewModel] {
        &self.controls
    }

    pub const fn summary(&self) -> &SemanticSurfaceSummary {
        &self.summary
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SemanticGraphicalData {
    generation: u64,
    state_hash: String,
    context: TopLevelContext,
    active_surface: SurfaceId,
    focus_path: FocusPath,
    interaction_mode: crate::control::InteractionMode,
    return_path: Option<ReturnPath>,
    valid_actions: Vec<ValidAction>,
    status: SemanticLifecycleStatus,
    errors: Vec<SemanticError>,
    surfaces: Vec<SemanticSurfaceViewModel>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticGraphicalViewModelError {
    MissingPatch,
    InvalidInstrumentConfig,
    InvalidEffectConfig,
    InvalidFocusPath,
    DuplicateControlPath,
    DuplicateValidAction,
    IncoherentSurface,
}

impl fmt::Display for SemanticGraphicalViewModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingPatch => "semantic PATCH projection has no installed Patch",
            Self::InvalidInstrumentConfig => "semantic projection cannot resolve instrument config",
            Self::InvalidEffectConfig => "semantic projection cannot resolve effect config",
            Self::InvalidFocusPath => "semantic projection focus does not resolve",
            Self::DuplicateControlPath => "semantic projection contains a duplicate control path",
            Self::DuplicateValidAction => "semantic projection contains a duplicate valid action",
            Self::IncoherentSurface => "semantic projection context, surface, and focus differ",
        })
    }
}

impl std::error::Error for SemanticGraphicalViewModelError {}

/// Immutable layout-neutral graphical contract shared by every host.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticGraphicalViewModel {
    data: Arc<SemanticGraphicalData>,
}

impl Serialize for SemanticGraphicalViewModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.data.as_ref().serialize(serializer)
    }
}

impl SemanticGraphicalViewModel {
    pub const SERIALIZED_PROPERTY_DESCRIPTOR: &'static [&'static str] = &[
        "generation",
        "stateHash",
        "context",
        "activeSurface",
        "focusPath",
        "interactionMode",
        "returnPath",
        "validActions",
        "status",
        "errors",
        "surfaces",
    ];

    /// Exhaustive normalized leaf paths across the semantic model's tagged
    /// unions and optional values. `[]` denotes any serialized array element.
    pub const SERIALIZED_LEAF_DESCRIPTOR: &'static [&'static str] = &[
        "activeSurface",
        "context",
        "errors[].code.failure",
        "errors[].code.kind",
        "errors[].label",
        "errors[].sourcePath.capabilityId",
        "errors[].sourcePath.context",
        "errors[].sourcePath.controlId.id",
        "errors[].sourcePath.controlId.kind",
        "errors[].sourcePath.modalId",
        "errors[].sourcePath.patchId",
        "errors[].sourcePath.surface",
        "focusPath.capabilityId",
        "focusPath.capabilityId.id",
        "focusPath.capabilityId.kind",
        "focusPath.context",
        "focusPath.controlId.id",
        "focusPath.controlId.id.bus",
        "focusPath.controlId.id.kind",
        "focusPath.controlId.id.parameter",
        "focusPath.controlId.id.track_id",
        "focusPath.controlId.kind",
        "focusPath.modalId",
        "focusPath.patchId",
        "focusPath.surface",
        "generation",
        "interactionMode",
        "returnPath",
        "returnPath.enteredSurface",
        "returnPath.origin.capabilityId",
        "returnPath.origin.capabilityId.id",
        "returnPath.origin.capabilityId.kind",
        "returnPath.origin.context",
        "returnPath.origin.controlId.id",
        "returnPath.origin.controlId.id.kind",
        "returnPath.origin.controlId.id.parameter",
        "returnPath.origin.controlId.id.track_id",
        "returnPath.origin.controlId.kind",
        "returnPath.origin.modalId",
        "returnPath.origin.patchId",
        "returnPath.origin.surface",
        "stateHash",
        "status.graphRevision",
        "status.kind",
        "status.label",
        "status.requestId",
        "status.targetGraphRevision",
        "surfaces[].controls[].editable",
        "surfaces[].controls[].enabled",
        "surfaces[].controls[].error",
        "surfaces[].controls[].error.code.failure",
        "surfaces[].controls[].error.code.kind",
        "surfaces[].controls[].error.label",
        "surfaces[].controls[].error.sourcePath.capabilityId",
        "surfaces[].controls[].error.sourcePath.context",
        "surfaces[].controls[].error.sourcePath.controlId.id",
        "surfaces[].controls[].error.sourcePath.controlId.kind",
        "surfaces[].controls[].error.sourcePath.modalId",
        "surfaces[].controls[].error.sourcePath.patchId",
        "surfaces[].controls[].error.sourcePath.surface",
        "surfaces[].controls[].focusable",
        "surfaces[].controls[].focused",
        "surfaces[].controls[].kind",
        "surfaces[].controls[].label",
        "surfaces[].controls[].numericRange",
        "surfaces[].controls[].numericRange.coarseStep",
        "surfaces[].controls[].numericRange.fineStep",
        "surfaces[].controls[].numericRange.maximum",
        "surfaces[].controls[].numericRange.minimum",
        "surfaces[].controls[].path.capabilityId",
        "surfaces[].controls[].path.capabilityId.id",
        "surfaces[].controls[].path.capabilityId.kind",
        "surfaces[].controls[].path.context",
        "surfaces[].controls[].path.controlId.id",
        "surfaces[].controls[].path.controlId.id.bus",
        "surfaces[].controls[].path.controlId.id.kind",
        "surfaces[].controls[].path.controlId.id.parameter",
        "surfaces[].controls[].path.controlId.id.track_id",
        "surfaces[].controls[].path.controlId.kind",
        "surfaces[].controls[].path.modalId",
        "surfaces[].controls[].path.patchId",
        "surfaces[].controls[].path.surface",
        "surfaces[].controls[].status",
        "surfaces[].controls[].status.graphRevision",
        "surfaces[].controls[].status.kind",
        "surfaces[].controls[].status.label",
        "surfaces[].controls[].status.requestId",
        "surfaces[].controls[].status.targetGraphRevision",
        "surfaces[].controls[].unit",
        "surfaces[].controls[].value.kind",
        "surfaces[].controls[].value.value",
        "surfaces[].controls[].value.value.kind",
        "surfaces[].controls[].value.value.locator",
        "surfaces[].controls[].value.value.value",
        "surfaces[].controls[].visible",
        "surfaces[].id",
        "surfaces[].label",
        "surfaces[].role",
        "surfaces[].summary.capability_id",
        "surfaces[].summary.effect_count",
        "surfaces[].summary.focused_control.bus",
        "surfaces[].summary.focused_control.kind",
        "surfaces[].summary.focused_control.parameter",
        "surfaces[].summary.focused_control.track_id",
        "surfaces[].summary.focused_track",
        "surfaces[].summary.global_parameter_count",
        "surfaces[].summary.kind",
        "surfaces[].summary.patch_count",
        "surfaces[].summary.patch_id",
        "surfaces[].summary.patch_name",
        "surfaces[].summary.routed_patches[].patchId",
        "surfaces[].summary.routed_patches[].patchName",
        "validActions[].action.kind",
        "validActions[].action.payload",
        "validActions[].hint",
        "validActions[].label",
    ];

    pub const fn serialized_property_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_PROPERTY_DESCRIPTOR
    }

    pub const fn serialized_leaf_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_LEAF_DESCRIPTOR
    }

    pub fn generation(&self) -> u64 {
        self.data.generation
    }

    pub fn state_hash(&self) -> &str {
        &self.data.state_hash
    }

    pub fn context(&self) -> TopLevelContext {
        self.data.context
    }

    pub fn active_surface(&self) -> SurfaceId {
        self.data.active_surface
    }

    pub fn focus_path(&self) -> &FocusPath {
        &self.data.focus_path
    }

    pub fn interaction_mode(&self) -> crate::control::InteractionMode {
        self.data.interaction_mode
    }

    pub fn return_path(&self) -> Option<&ReturnPath> {
        self.data.return_path.as_ref()
    }

    pub fn valid_actions(&self) -> &[ValidAction] {
        &self.data.valid_actions
    }

    pub fn status(&self) -> &SemanticLifecycleStatus {
        &self.data.status
    }

    pub fn errors(&self) -> &[SemanticError] {
        &self.data.errors
    }

    pub fn surfaces(&self) -> &[SemanticSurfaceViewModel] {
        &self.data.surfaces
    }

    pub fn surface(&self, id: SurfaceId) -> Option<&SemanticSurfaceViewModel> {
        self.data.surfaces.iter().find(|surface| surface.id == id)
    }

    pub(crate) fn with_generation(&self, generation: u64, state_hash: String) -> Self {
        let mut data = self.data.as_ref().clone();
        data.generation = generation;
        data.state_hash = state_hash;
        Self {
            data: Arc::new(data),
        }
    }

    /// Builds a coherent minimal value for isolated shell invariant tests. All
    /// production projections use `project` and never this constructor.
    pub(crate) fn fixture(
        generation: u64,
        state_hash: impl Into<String>,
        context: TopLevelContext,
    ) -> Self {
        let focus_path = match context {
            TopLevelContext::Patch => FocusPath::patch_main(
                PatchId::new(1).expect("fixture PatchId is valid"),
                None,
                PatchControlId::Engine,
            ),
            TopLevelContext::Mixer => {
                FocusPath::mixer_track(MixerTrackId::default(), MixerTrackParameter::Level)
            }
        };
        Self::fixture_for_interaction(
            generation,
            state_hash,
            focus_path,
            crate::control::InteractionMode::Navigate,
            None,
            None,
            None,
            GraphRevision::INITIAL,
        )
    }

    /// Builds the coherent minimal semantic shell required by compatibility
    /// tree construction from an already serialized interaction. Production
    /// AppState projection always uses `project` and its complete descriptors.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fixture_for_interaction(
        generation: u64,
        state_hash: impl Into<String>,
        focus_path: FocusPath,
        interaction_mode: crate::control::InteractionMode,
        return_path: Option<ReturnPath>,
        remembered_patch_main: Option<FocusPath>,
        remembered_mixer_main: Option<FocusPath>,
        graph_revision: GraphRevision,
    ) -> Self {
        let state_hash = state_hash.into();
        let context = focus_path.context();
        let (main_path, side_path) = match context {
            TopLevelContext::Patch => {
                let main = if focus_path.surface() == SurfaceId::PatchMain {
                    focus_path.clone()
                } else {
                    return_path
                        .as_ref()
                        .map(ReturnPath::origin)
                        .filter(|path| path.surface() == SurfaceId::PatchMain)
                        .cloned()
                        .or(remembered_patch_main)
                        .unwrap_or_else(|| {
                            FocusPath::patch_main(
                                PatchId::new(1).expect("fixture PatchId is valid"),
                                None,
                                PatchControlId::Engine,
                            )
                        })
                };
                (
                    main,
                    FocusPath::side_root(SurfaceId::PatchUtility)
                        .expect("fixture side surface is valid"),
                )
            }
            TopLevelContext::Mixer => {
                let main = if focus_path.surface() == SurfaceId::MixerMain {
                    focus_path.clone()
                } else {
                    return_path
                        .as_ref()
                        .map(ReturnPath::origin)
                        .filter(|path| path.surface() == SurfaceId::MixerMain)
                        .cloned()
                        .or(remembered_mixer_main)
                        .unwrap_or_else(|| {
                            FocusPath::mixer_track(
                                MixerTrackId::default(),
                                MixerTrackParameter::Level,
                            )
                        })
                };
                (
                    main,
                    FocusPath::side_root(SurfaceId::MixerInspector)
                        .expect("fixture side surface is valid"),
                )
            }
        };
        let surfaces = fixture_surfaces(context, &focus_path, main_path, side_path);
        let data = SemanticGraphicalData {
            generation,
            state_hash,
            context,
            active_surface: focus_path.surface(),
            focus_path,
            interaction_mode,
            return_path,
            valid_actions: Vec::new(),
            status: SemanticLifecycleStatus {
                kind: EngineSelectionStatusKind::Ready,
                label: "READY".to_owned(),
                request_id: None,
                graph_revision,
                target_graph_revision: None,
            },
            errors: Vec::new(),
            surfaces,
        };
        debug_assert!(validate_data(&data).is_ok());
        Self {
            data: Arc::new(data),
        }
    }

    pub(crate) fn project(
        state: &AppState,
        state_hash: &str,
    ) -> Result<Self, SemanticGraphicalViewModelError> {
        let resolver = SemanticResolver::new(state);
        let focus_path = state.interaction().focus_path().clone();
        if !resolver.resolves(&focus_path) {
            return Err(SemanticGraphicalViewModelError::InvalidFocusPath);
        }
        let status = project_status(state);
        let errors = project_errors(state, &resolver, &status)?;
        let surfaces = match state.context() {
            TopLevelContext::Patch => project_patch_surfaces(state, &resolver, &status, &errors)?,
            TopLevelContext::Mixer => project_mixer_surfaces(state, &resolver, &status, &errors)?,
        };
        let valid_actions = resolver.valid_actions();
        let data = SemanticGraphicalData {
            generation: state.generation(),
            state_hash: state_hash.to_owned(),
            context: state.context(),
            active_surface: state.interaction().active_surface(),
            focus_path,
            interaction_mode: state.interaction().mode(),
            return_path: state.interaction().return_path().cloned(),
            valid_actions,
            status,
            errors,
            surfaces,
        };
        validate_data(&data)?;
        Ok(Self {
            data: Arc::new(data),
        })
    }
}

fn fixture_surfaces(
    context: TopLevelContext,
    active: &FocusPath,
    main_path: FocusPath,
    side_path: FocusPath,
) -> Vec<SemanticSurfaceViewModel> {
    let main_control_path = if active.surface().is_main() {
        active.clone()
    } else {
        main_path.clone()
    };
    let main_control = SemanticControlViewModel {
        focused: active == &main_control_path,
        path: main_control_path,
        label: "Fixture control".to_owned(),
        kind: SemanticControlKind::Identity,
        value: SemanticControlValue::Identity("Fixture".to_owned()),
        numeric_range: None,
        unit: None,
        enabled: true,
        visible: true,
        focusable: true,
        editable: false,
        status: None,
        error: None,
    };
    let side_control_path = if active.surface().is_main() {
        side_path
    } else {
        active.clone()
    };
    let side_control = surface_root_control(side_control_path, "Read-only summary", active);
    match context {
        TopLevelContext::Patch => {
            let patch_id = main_path
                .patch_id()
                .unwrap_or_else(|| PatchId::new(1).expect("fixture PatchId is valid"));
            let capability_id = match main_path.capability_id() {
                Some(FocusCapabilityId::Instrument(id)) => id.clone(),
                _ => {
                    CapabilityId::new("instrument.fixture").expect("fixture capability id is valid")
                }
            };
            vec![
                SemanticSurfaceViewModel {
                    id: SurfaceId::PatchMain,
                    label: "PATCH".to_owned(),
                    role: SemanticSurfaceRole::Main,
                    controls: vec![main_control],
                    summary: SemanticSurfaceSummary::Patch {
                        patch_id,
                        patch_name: "Fixture".to_owned(),
                        capability_id: capability_id.clone(),
                        effect_count: 0,
                    },
                },
                SemanticSurfaceViewModel {
                    id: SurfaceId::PatchUtility,
                    label: "UTILITY".to_owned(),
                    role: SemanticSurfaceRole::PersistentSide,
                    controls: vec![side_control],
                    summary: SemanticSurfaceSummary::PatchUtility {
                        patch_id,
                        capability_id,
                        effect_count: 0,
                    },
                },
            ]
        }
        TopLevelContext::Mixer => {
            let focused_control = match main_path.control_id() {
                crate::control::SemanticControlId::Mixer(control) => control.clone(),
                _ => MixerControlId::Global {
                    parameter: crate::mixer::global_parameters::GlobalParameter::MasterGainDb,
                },
            };
            vec![
                SemanticSurfaceViewModel {
                    id: SurfaceId::MixerMain,
                    label: "MIXER".to_owned(),
                    role: SemanticSurfaceRole::Main,
                    controls: vec![main_control],
                    summary: SemanticSurfaceSummary::Mixer {
                        patch_count: 0,
                        global_parameter_count: 7,
                    },
                },
                SemanticSurfaceViewModel {
                    id: SurfaceId::MixerInspector,
                    label: "INSPECTOR".to_owned(),
                    role: SemanticSurfaceRole::PersistentSide,
                    controls: vec![side_control],
                    summary: SemanticSurfaceSummary::MixerInspector {
                        focused_control,
                        focused_track: MixerTrackId::default(),
                        patch_count: 0,
                        routed_patches: Vec::new(),
                    },
                },
            ]
        }
    }
}

fn project_status(state: &AppState) -> SemanticLifecycleStatus {
    let selection = state.engine_selection();
    let correlation = selection.correlation();
    SemanticLifecycleStatus {
        kind: selection.kind(),
        label: selection.kind().name().to_ascii_uppercase(),
        request_id: correlation.map(|value| value.request_id()),
        graph_revision: selection.projection_graph_revision(),
        target_graph_revision: correlation.and_then(|value| value.target_graph_revision()),
    }
}

fn project_errors(
    state: &AppState,
    resolver: &SemanticResolver<'_>,
    _status: &SemanticLifecycleStatus,
) -> Result<Vec<SemanticError>, SemanticGraphicalViewModelError> {
    let Some(failure) = state.engine_selection().failure() else {
        return Ok(Vec::new());
    };
    let correlation = state
        .engine_selection()
        .correlation()
        .ok_or(SemanticGraphicalViewModelError::InvalidFocusPath)?;
    // Occupancy refusals carry their position in the intent and stay
    // attributable to that exact slot or return row; instrument intents
    // anchor a PATCH-surface source path through the resolver.
    let source_path = match correlation.intent() {
        crate::control::StructuralEditIntent::SetSlotOccupancy { patch_id, slot, .. } => Some(
            FocusPath::patch_main(*patch_id, None, PatchControlId::EffectSlot(*slot)),
        ),
        crate::control::StructuralEditIntent::SetReturnOccupancy { bus, .. } => {
            Some(FocusPath::mixer_return_occupancy(*bus))
        }
        crate::control::StructuralEditIntent::ReplaceCapability { .. }
        | crate::control::StructuralEditIntent::ReplaceParameterChoice { .. } => {
            let source_paths = match correlation.patch_id() {
                Some(patch_id) => resolver
                    .patch_main_paths(patch_id)
                    .map_err(map_resolver_error)?,
                None => Vec::new(),
            };
            source_paths
                .into_iter()
                .find(|path| match (path.control_id(), correlation.intent()) {
                    (
                        crate::control::SemanticControlId::Patch(PatchControlId::Engine),
                        crate::control::StructuralEditIntent::ReplaceCapability { .. },
                    ) => true,
                    (
                        crate::control::SemanticControlId::Patch(PatchControlId::Capability(
                            path_id,
                        )),
                        crate::control::StructuralEditIntent::ReplaceParameterChoice {
                            parameter_id,
                            ..
                        },
                    ) => path_id == parameter_id,
                    _ => false,
                })
        }
    };
    Ok(vec![SemanticError {
        code: SemanticErrorCode::EngineSelection(failure),
        label: failure.name().to_owned(),
        source_path,
    }])
}

fn project_patch_surfaces(
    state: &AppState,
    resolver: &SemanticResolver<'_>,
    status: &SemanticLifecycleStatus,
    errors: &[SemanticError],
) -> Result<Vec<SemanticSurfaceViewModel>, SemanticGraphicalViewModelError> {
    let patch_id = state
        .interaction()
        .patch_focus()
        .ok_or(SemanticGraphicalViewModelError::MissingPatch)?;
    let patch = state
        .patches()
        .iter()
        .find(|patch| patch.id() == patch_id)
        .ok_or(SemanticGraphicalViewModelError::MissingPatch)?;
    let descriptor = state
        .capabilities()
        .descriptor(patch.instrument_config().capability_id())
        .ok_or(SemanticGraphicalViewModelError::InvalidInstrumentConfig)?;
    let focusable_paths = resolver
        .patch_main_paths(patch_id)
        .map_err(map_resolver_error)?;
    let active = state.interaction().focus_path();
    let lifecycle_editable = matches!(
        status.kind(),
        EngineSelectionStatusKind::Ready | EngineSelectionStatusKind::Failed
    );
    let mut controls = Vec::new();

    let engine_path = FocusPath::patch_main(patch_id, None, PatchControlId::Engine);
    controls.push(SemanticControlViewModel {
        path: engine_path.clone(),
        label: "Engine".to_owned(),
        kind: SemanticControlKind::Identity,
        value: SemanticControlValue::Identity(descriptor.label().to_owned()),
        numeric_range: None,
        unit: None,
        enabled: true,
        visible: true,
        focusable: true,
        editable: lifecycle_editable && state.capabilities().descriptors().len() > 1,
        focused: active == &engine_path,
        status: Some(status.clone()),
        error: error_for_path(errors, &engine_path),
    });

    for envelope in crate::synth::VoiceEnvelope::surface_descriptor() {
        let control_id = PatchControlId::Envelope(envelope.parameter());
        let path = FocusPath::patch_main(patch_id, None, control_id);
        controls.push(SemanticControlViewModel {
            path: path.clone(),
            label: envelope.label().to_owned(),
            kind: SemanticControlKind::Continuous,
            value: SemanticControlValue::Scalar(
                patch.envelope().value(envelope.parameter()) as f64,
            ),
            numeric_range: Some(SemanticNumericRange::new(
                envelope.minimum() as f64,
                envelope.maximum() as f64,
                envelope.fine_step() as f64,
                envelope.coarse_step() as f64,
            )),
            unit: envelope.unit().map(str::to_owned),
            enabled: true,
            visible: true,
            focusable: true,
            editable: true,
            focused: active == &path,
            status: None,
            error: None,
        });
    }

    for spec in descriptor.parameters() {
        let path = FocusPath::patch_main(
            patch_id,
            Some(FocusCapabilityId::Instrument(descriptor.id().clone())),
            PatchControlId::Capability(spec.id().clone()),
        );
        let (enabled, visible) = parameter_availability(spec, patch.instrument_config());
        let focusable = focusable_paths.contains(&path);
        let targeted = state
            .engine_selection()
            .correlation()
            .is_some_and(|correlation| {
                correlation.patch_id() == Some(patch_id)
                    && correlation.intent().parameter_id() == Some(spec.id())
            });
        controls.push(control_from_parameter(
            path,
            spec,
            parameter_value(spec, patch.instrument_config())?,
            ParameterControlProjection {
                enabled,
                visible,
                focusable,
                editable: spec.patch_interaction() == PatchInteraction::StructuralChoice
                    && focusable
                    && lifecycle_editable,
                active,
                status: targeted.then(|| status.clone()),
                errors,
            },
        ));
    }

    for slot_index in crate::synth::effect_slot_id::EffectSlotIndex::ALL {
        let occupant = patch.effect_slot(slot_index);
        let occupancy_path =
            FocusPath::patch_main(patch_id, None, PatchControlId::EffectSlot(slot_index));
        let occupancy_label = format!("Slot {}", slot_index.index() + 1);
        let occupancy_value = match occupant {
            None => "Empty".to_owned(),
            Some(effect) => state
                .effects()
                .descriptor(effect.capability_id())
                .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)?
                .label()
                .to_owned(),
        };
        let targeted = state
            .engine_selection()
            .correlation()
            .is_some_and(|correlation| {
                matches!(
                    correlation.intent(),
                    crate::control::StructuralEditIntent::SetSlotOccupancy {
                        patch_id: target_patch,
                        slot,
                        ..
                    } if *target_patch == patch_id && *slot == slot_index
                )
            });
        controls.push(SemanticControlViewModel {
            path: occupancy_path.clone(),
            label: occupancy_label,
            kind: SemanticControlKind::Choice,
            value: SemanticControlValue::Identity(occupancy_value),
            numeric_range: None,
            unit: None,
            enabled: true,
            visible: true,
            focusable: true,
            editable: lifecycle_editable && !state.effects().descriptors().is_empty(),
            focused: active == &occupancy_path,
            status: targeted.then(|| status.clone()),
            error: error_for_path(errors, &occupancy_path),
        });

        let Some(effect) = occupant else {
            continue;
        };
        let effect_descriptor = state
            .effects()
            .descriptor(effect.capability_id())
            .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)?;
        for spec in effect_descriptor.parameters() {
            let path = FocusPath::patch_main(
                patch_id,
                Some(FocusCapabilityId::Effect(effect_descriptor.id().clone())),
                PatchControlId::Effect(effect.slot_id(), spec.id().clone()),
            );
            let (enabled, visible) = effect_parameter_availability(spec, effect);
            let focusable = focusable_paths.contains(&path);
            controls.push(control_from_parameter(
                path,
                spec,
                effect_parameter_value(spec, effect)?,
                ParameterControlProjection {
                    enabled,
                    visible,
                    focusable,
                    editable: spec.patch_interaction() == PatchInteraction::ScalarEdit && focusable,
                    active,
                    status: None,
                    errors,
                },
            ));
        }
    }

    // The summary counts configured effects: occupied positions of the
    // per-position chain, wherever they sit. Empty positions never count.
    let configured_effect_count = patch.effect_slots().iter().flatten().count();
    let summary = SemanticSurfaceSummary::Patch {
        patch_id,
        patch_name: patch.name().to_owned(),
        capability_id: descriptor.id().clone(),
        effect_count: configured_effect_count,
    };
    let side_summary = SemanticSurfaceSummary::PatchUtility {
        patch_id,
        capability_id: descriptor.id().clone(),
        effect_count: configured_effect_count,
    };
    let utility_paths = resolver
        .patch_utility_paths(patch_id)
        .map_err(map_resolver_error)?;
    let utility_controls = utility_paths
        .into_iter()
        .map(|path| {
            let crate::control::SemanticControlId::Patch(PatchControlId::Output(parameter)) =
                path.control_id()
            else {
                return Err(SemanticGraphicalViewModelError::InvalidFocusPath);
            };
            let descriptor = parameter.descriptor();
            let (kind, value, numeric_range, unit) = match parameter {
                PatchOutputParameter::TrimGain => (
                    SemanticControlKind::Continuous,
                    SemanticControlValue::Scalar(patch.output().trim_gain_db() as f64),
                    Some(SemanticNumericRange::new(
                        descriptor.minimum().unwrap_or(0.0) as f64,
                        descriptor.maximum().unwrap_or(0.0) as f64,
                        descriptor.fine_step().unwrap_or(1.0) as f64,
                        descriptor.coarse_step().unwrap_or(1.0) as f64,
                    )),
                    descriptor.unit().map(str::to_owned),
                ),
                PatchOutputParameter::OutputTrack => (
                    SemanticControlKind::Choice,
                    SemanticControlValue::Identity(patch.output().track_id().to_string()),
                    None,
                    None,
                ),
            };
            Ok(SemanticControlViewModel {
                focused: active == &path,
                path,
                label: descriptor.label().to_owned(),
                kind,
                value,
                numeric_range,
                unit,
                enabled: true,
                visible: true,
                focusable: true,
                editable: true,
                status: None,
                error: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(vec![
        SemanticSurfaceViewModel {
            id: SurfaceId::PatchMain,
            label: SurfaceId::PatchMain.label().to_owned(),
            role: SemanticSurfaceRole::Main,
            controls,
            summary,
        },
        SemanticSurfaceViewModel {
            id: SurfaceId::PatchUtility,
            label: SurfaceId::PatchUtility.label().to_owned(),
            role: SemanticSurfaceRole::PersistentSide,
            controls: utility_controls,
            summary: side_summary,
        },
    ])
}

fn project_mixer_surfaces(
    state: &AppState,
    resolver: &SemanticResolver<'_>,
    status: &SemanticLifecycleStatus,
    errors: &[SemanticError],
) -> Result<Vec<SemanticSurfaceViewModel>, SemanticGraphicalViewModelError> {
    let active = state.interaction().focus_path();
    let mut controls = Vec::with_capacity(MixerTrackId::COUNT * MixerTrackParameter::MAIN.len());
    for track_id in MixerTrackId::ALL {
        let values = *state.mixer().track(track_id);
        for parameter in MixerTrackParameter::MAIN {
            controls.push(track_control(track_id, parameter, values, active));
        }
    }

    let focused_track = state
        .interaction()
        .remembered_mixer_main()
        .control_id()
        .as_mixer_track_id()
        .ok_or(SemanticGraphicalViewModelError::InvalidFocusPath)?;
    let inspector_paths = resolver
        .mixer_inspector_paths(focused_track)
        .map_err(map_resolver_error)?;
    let lifecycle_editable = matches!(
        status.kind(),
        EngineSelectionStatusKind::Ready | EngineSelectionStatusKind::Failed
    );
    let mut inspector_controls = Vec::with_capacity(inspector_paths.len());
    for path in inspector_paths {
        let crate::control::SemanticControlId::Mixer(control) = path.control_id() else {
            return Err(SemanticGraphicalViewModelError::InvalidFocusPath);
        };
        let control_view = match control.clone() {
            MixerControlId::Track {
                track_id,
                parameter,
            } => track_control(track_id, parameter, *state.mixer().track(track_id), active),
            MixerControlId::Send { track_id, bus } => {
                let descriptor = crate::mixer::mixer_track_parameters::BUS_SEND_DESCRIPTOR;
                SemanticControlViewModel {
                    focused: active == &path,
                    path: path.clone(),
                    label: format!("{track_id} Send {bus}"),
                    kind: SemanticControlKind::Continuous,
                    value: SemanticControlValue::Scalar(
                        state.mixer().track(track_id).send(bus) as f64
                    ),
                    numeric_range: Some(SemanticNumericRange::new(
                        descriptor.minimum() as f64,
                        descriptor.maximum() as f64,
                        descriptor.fine_step() as f64,
                        descriptor.coarse_step() as f64,
                    )),
                    unit: None,
                    enabled: true,
                    visible: true,
                    focusable: true,
                    editable: true,
                    status: None,
                    error: None,
                }
            }
            MixerControlId::ReturnOccupancy { bus } => {
                let occupancy_value = match state.bus_returns().bus_return(bus).effect() {
                    None => "Empty".to_owned(),
                    Some(config) => state
                        .effects()
                        .descriptor(config.capability_id())
                        .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)?
                        .label()
                        .to_owned(),
                };
                let targeted = state
                    .engine_selection()
                    .correlation()
                    .is_some_and(|correlation| {
                        matches!(
                            correlation.intent(),
                            crate::control::StructuralEditIntent::SetReturnOccupancy {
                                bus: target_bus,
                                ..
                            } if *target_bus == bus
                        )
                    });
                SemanticControlViewModel {
                    focused: active == &path,
                    path: path.clone(),
                    label: format!("Return {bus}"),
                    kind: SemanticControlKind::Choice,
                    value: SemanticControlValue::Identity(occupancy_value),
                    numeric_range: None,
                    unit: None,
                    enabled: true,
                    visible: true,
                    focusable: true,
                    editable: lifecycle_editable && !state.effects().descriptors().is_empty(),
                    status: targeted.then(|| status.clone()),
                    error: error_for_path(errors, &path),
                }
            }
            MixerControlId::ReturnLevel { bus } => {
                let descriptor = crate::mixer::bus_return::RETURN_LEVEL_DESCRIPTOR;
                SemanticControlViewModel {
                    focused: active == &path,
                    path: path.clone(),
                    label: format!("Return {bus} Level"),
                    kind: SemanticControlKind::Continuous,
                    value: SemanticControlValue::Scalar(
                        state.bus_returns().bus_return(bus).return_level() as f64,
                    ),
                    numeric_range: Some(SemanticNumericRange::new(
                        descriptor.minimum() as f64,
                        descriptor.maximum() as f64,
                        descriptor.fine_step() as f64,
                        descriptor.coarse_step() as f64,
                    )),
                    unit: None,
                    enabled: true,
                    visible: true,
                    focusable: true,
                    editable: true,
                    status: None,
                    error: None,
                }
            }
            MixerControlId::ReturnEffect { bus, parameter } => {
                let config = state
                    .bus_returns()
                    .bus_return(bus)
                    .effect()
                    .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)?;
                let descriptor = state
                    .effects()
                    .descriptor(config.capability_id())
                    .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)?;
                let spec = descriptor
                    .parameter(&parameter)
                    .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)?;
                let (enabled, visible) = effect_parameter_availability(spec, config);
                control_from_parameter(
                    path.clone(),
                    spec,
                    effect_parameter_value(spec, config)?,
                    ParameterControlProjection {
                        enabled,
                        visible,
                        focusable: true,
                        editable: spec.patch_interaction() == PatchInteraction::ScalarEdit,
                        active,
                        status: None,
                        errors,
                    },
                )
            }
            MixerControlId::Global { parameter } => {
                let descriptor = parameter.descriptor();
                SemanticControlViewModel {
                    path: path.clone(),
                    label: descriptor.name().to_owned(),
                    kind: SemanticControlKind::Continuous,
                    value: SemanticControlValue::Scalar(state.global_row_value(parameter) as f64),
                    numeric_range: Some(SemanticNumericRange::new(
                        descriptor.minimum() as f64,
                        descriptor.maximum() as f64,
                        descriptor.fine_step() as f64,
                        descriptor.coarse_step() as f64,
                    )),
                    unit: None,
                    enabled: true,
                    visible: true,
                    focusable: true,
                    editable: true,
                    focused: active == &path,
                    status: None,
                    error: None,
                }
            }
        };
        inspector_controls.push(control_view);
    }
    let routed_patches = state
        .patches()
        .iter()
        .filter(|patch| patch.output().track_id() == focused_track)
        .map(|patch| SemanticRoutedPatch {
            patch_id: patch.id(),
            patch_name: patch.name().to_owned(),
        })
        .collect();
    Ok(vec![
        SemanticSurfaceViewModel {
            id: SurfaceId::MixerMain,
            label: SurfaceId::MixerMain.label().to_owned(),
            role: SemanticSurfaceRole::Main,
            controls,
            summary: SemanticSurfaceSummary::Mixer {
                patch_count: state.patches().len(),
                global_parameter_count:
                    crate::mixer::global_parameters::GlobalParameters::surface_descriptor().len(),
            },
        },
        SemanticSurfaceViewModel {
            id: SurfaceId::MixerInspector,
            label: SurfaceId::MixerInspector.label().to_owned(),
            role: SemanticSurfaceRole::PersistentSide,
            controls: inspector_controls,
            summary: SemanticSurfaceSummary::MixerInspector {
                focused_control: state.interaction().mixer_control_focus().clone(),
                focused_track,
                patch_count: state.patches().len(),
                routed_patches,
            },
        },
    ])
}

fn track_control(
    track_id: MixerTrackId,
    parameter: MixerTrackParameter,
    values: crate::mixer::mixer_track_parameters::MixerTrackParameters,
    active: &FocusPath,
) -> SemanticControlViewModel {
    let descriptor = parameter.descriptor();
    let path = FocusPath::mixer_track(track_id, parameter);
    let is_toggle = descriptor.kind() == MixerTrackParameterKind::Toggle;
    SemanticControlViewModel {
        focused: active == &path,
        path,
        label: format!("{track_id} {}", descriptor.label()),
        kind: if is_toggle {
            SemanticControlKind::Toggle
        } else {
            SemanticControlKind::Continuous
        },
        value: if is_toggle {
            SemanticControlValue::Parameter(ParameterValue::Toggle(
                values.toggle_value(parameter).unwrap_or(false),
            ))
        } else {
            SemanticControlValue::Scalar(values.scalar_value(parameter).unwrap_or(0.0) as f64)
        },
        numeric_range: (!is_toggle).then(|| {
            SemanticNumericRange::new(
                descriptor.minimum() as f64,
                descriptor.maximum() as f64,
                descriptor.fine_step() as f64,
                descriptor.coarse_step() as f64,
            )
        }),
        unit: descriptor.unit().map(str::to_owned),
        enabled: true,
        visible: true,
        focusable: true,
        editable: true,
        status: None,
        error: None,
    }
}

struct ParameterControlProjection<'a> {
    enabled: bool,
    visible: bool,
    focusable: bool,
    editable: bool,
    active: &'a FocusPath,
    status: Option<SemanticLifecycleStatus>,
    errors: &'a [SemanticError],
}

fn control_from_parameter(
    path: FocusPath,
    spec: &ParameterSpec,
    value: SemanticControlValue,
    projection: ParameterControlProjection<'_>,
) -> SemanticControlViewModel {
    let numeric_range = spec.range().map(|range| {
        SemanticNumericRange::new(
            range.minimum(),
            range.maximum(),
            spec.fine_step().unwrap_or(1.0),
            spec.coarse_step().unwrap_or(1.0),
        )
    });
    SemanticControlViewModel {
        error: error_for_path(projection.errors, &path),
        focused: projection.active == &path,
        path,
        label: spec.label().to_owned(),
        kind: spec.kind().into(),
        value,
        numeric_range,
        unit: spec.unit().map(str::to_owned),
        enabled: projection.enabled,
        visible: projection.visible,
        focusable: projection.focusable,
        editable: projection.editable,
        status: projection.status,
    }
}

fn surface_root_control(
    path: FocusPath,
    label: &str,
    active: &FocusPath,
) -> SemanticControlViewModel {
    SemanticControlViewModel {
        focused: active == &path,
        path,
        label: label.to_owned(),
        kind: SemanticControlKind::Surface,
        value: SemanticControlValue::Summary("Read-only in Phase 2".to_owned()),
        numeric_range: None,
        unit: None,
        enabled: true,
        visible: true,
        focusable: true,
        editable: false,
        status: None,
        error: None,
    }
}

fn parameter_value(
    spec: &ParameterSpec,
    config: &crate::synth::InstrumentConfig,
) -> Result<SemanticControlValue, SemanticGraphicalViewModelError> {
    if spec.kind() == ParameterKind::Asset {
        config
            .asset_reference(spec.id())
            .cloned()
            .map(SemanticControlValue::Asset)
            .ok_or(SemanticGraphicalViewModelError::InvalidInstrumentConfig)
    } else {
        config
            .value(spec.id())
            .cloned()
            .map(SemanticControlValue::Parameter)
            .ok_or(SemanticGraphicalViewModelError::InvalidInstrumentConfig)
    }
}

fn effect_parameter_value(
    spec: &ParameterSpec,
    config: &crate::synth::PostEffectConfig,
) -> Result<SemanticControlValue, SemanticGraphicalViewModelError> {
    if spec.kind() == ParameterKind::Asset {
        config
            .asset_reference(spec.id())
            .cloned()
            .map(SemanticControlValue::Asset)
            .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)
    } else {
        config
            .value(spec.id())
            .cloned()
            .map(SemanticControlValue::Parameter)
            .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)
    }
}

fn parameter_availability(
    spec: &ParameterSpec,
    config: &crate::synth::InstrumentConfig,
) -> (bool, bool) {
    let satisfied = |predicate: Option<&crate::synth::ParameterPredicate>| {
        predicate.is_none_or(|predicate| {
            config.value(predicate.parameter_id()) == Some(predicate.equals())
        })
    };
    (
        satisfied(spec.enabled_when()),
        satisfied(spec.visible_when()),
    )
}

fn effect_parameter_availability(
    spec: &ParameterSpec,
    config: &crate::synth::PostEffectConfig,
) -> (bool, bool) {
    let satisfied = |predicate: Option<&crate::synth::ParameterPredicate>| {
        predicate.is_none_or(|predicate| {
            config.value(predicate.parameter_id()) == Some(predicate.equals())
        })
    };
    (
        satisfied(spec.enabled_when()),
        satisfied(spec.visible_when()),
    )
}

fn error_for_path(errors: &[SemanticError], path: &FocusPath) -> Option<SemanticError> {
    errors
        .iter()
        .find(|error| error.source_path.as_ref() == Some(path))
        .cloned()
}

fn map_resolver_error(error: crate::control::EventRejection) -> SemanticGraphicalViewModelError {
    match error {
        crate::control::EventRejection::InvalidEffectConfig => {
            SemanticGraphicalViewModelError::InvalidEffectConfig
        }
        crate::control::EventRejection::InvalidInstrumentConfig => {
            SemanticGraphicalViewModelError::InvalidInstrumentConfig
        }
        crate::control::EventRejection::NoPatchesInstalled
        | crate::control::EventRejection::UnknownPatch => {
            SemanticGraphicalViewModelError::MissingPatch
        }
        _ => SemanticGraphicalViewModelError::InvalidFocusPath,
    }
}

fn validate_data(data: &SemanticGraphicalData) -> Result<(), SemanticGraphicalViewModelError> {
    if data.context != data.focus_path.context()
        || data.active_surface != data.focus_path.surface()
        || data.active_surface.context() != data.context
        || !data
            .surfaces
            .iter()
            .any(|surface| surface.id == data.active_surface)
        || !data.interaction_mode.is_phase_two_reachable()
    {
        return Err(SemanticGraphicalViewModelError::IncoherentSurface);
    }
    match (data.active_surface.is_main(), data.return_path.as_ref()) {
        (true, None) => {}
        (false, Some(path))
            if path.entered_surface() == data.active_surface
                && path.origin().context() == data.context
                && path.origin().surface().is_main() => {}
        _ => return Err(SemanticGraphicalViewModelError::IncoherentSurface),
    }
    let controls = data
        .surfaces
        .iter()
        .flat_map(|surface| surface.controls.iter())
        .collect::<Vec<_>>();
    let unique_paths = controls
        .iter()
        .map(|control| &control.path)
        .collect::<HashSet<_>>();
    if unique_paths.len() != controls.len() {
        return Err(SemanticGraphicalViewModelError::DuplicateControlPath);
    }
    let focused = controls
        .iter()
        .filter(|control| control.focused)
        .collect::<Vec<_>>();
    if focused.len() != 1
        || focused[0].path != data.focus_path
        || !focused[0].visible
        || !focused[0].enabled
        || !focused[0].focusable
    {
        return Err(SemanticGraphicalViewModelError::InvalidFocusPath);
    }
    let unique_actions = data
        .valid_actions
        .iter()
        .map(ValidAction::action)
        .collect::<HashSet<_>>();
    if unique_actions.len() != data.valid_actions.len() {
        return Err(SemanticGraphicalViewModelError::DuplicateValidAction);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::AppEvent;

    /// One installed patch whose chain occupies only slot 1: slot 0 is empty
    /// and stays empty. This is exactly the shape a compacting view would
    /// silently squeeze down to position 0.
    fn gapped_patch_state() -> AppState {
        let mut state = AppState::new_with_effects(
            crate::adapter::production_instruments::production_capability_registry().unwrap(),
            crate::adapter::production_effects::production_effect_registry().unwrap(),
            crate::mixer::global_parameters::GlobalParameters::new(0.0).unwrap(),
        );
        let mut patch = crate::synth::Patch::new(
            PatchId::new(7).unwrap(),
            "Gapped".to_owned(),
            crate::adapter::braids_capability::BraidsCapability::new()
                .unwrap()
                .default_config()
                .unwrap(),
            crate::kernel::MidiChannel::new(3).unwrap(),
            crate::mixer::patch_output::PatchOutput::to_track(
                crate::mixer::mixer_track_id::MixerTrackId::new(3).unwrap(),
            ),
        );
        patch
            .set_slot_occupancy(
                crate::synth::effect_slot_id::EffectSlotIndex::new(1).unwrap(),
                Some(
                    crate::adapter::production_effects::production_chorus_config(
                        crate::synth::EffectSlotId::new(2).unwrap(),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        state.apply(AppEvent::InstallPatches(vec![patch])).unwrap();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
    }

    #[test]
    fn gapped_chain_counts_occupants_and_projects_rows_per_position() {
        let state = gapped_patch_state();
        let model = SemanticGraphicalViewModel::project(&state, "gapped-state-hash").unwrap();
        let patch_surface = model
            .surfaces()
            .iter()
            .find(|surface| surface.id() == SurfaceId::PatchMain)
            .expect("PATCH Main projects for the focused patch");

        let SemanticSurfaceSummary::Patch { effect_count, .. } = patch_surface.summary() else {
            panic!("PATCH Main carries a Patch summary");
        };
        assert_eq!(
            *effect_count, 1,
            "only occupied positions count as configured effects"
        );

        let slot_value = |position: usize| {
            patch_surface
                .controls()
                .iter()
                .find(|control| {
                    matches!(
                        control.path().control_id(),
                        crate::control::SemanticControlId::Patch(PatchControlId::EffectSlot(slot))
                            if slot.index() == position
                    )
                })
                .map(|control| control.value().clone())
        };
        assert_eq!(
            slot_value(0),
            Some(SemanticControlValue::Identity("Empty".to_owned())),
            "slot 0 must project as empty, never receive the squeezed occupant"
        );
        assert_eq!(
            slot_value(1),
            Some(SemanticControlValue::Identity("Chorus".to_owned())),
            "the occupant must project at its true position"
        );
        assert_eq!(
            slot_value(2),
            Some(SemanticControlValue::Identity("Empty".to_owned()))
        );
    }
}
