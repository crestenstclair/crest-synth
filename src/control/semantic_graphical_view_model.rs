use crate::control::{
    AppState, EngineSelectionFailure, EngineSelectionRequestId, EngineSelectionStatusKind,
    FocusCapabilityId, FocusPath, MixerControlId, PatchControlId, PatchDetailSubject, ReturnPath,
    SemanticResolver, SurfaceId, TopLevelContext, ValidAction,
};
use crate::kernel::{MidiChannel, PatchId};
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::{MixerTrackParameter, MixerTrackParameterKind};
use crate::mixer::patch_output::PatchOutputParameter;
use crate::real_time::GraphRevision;
use crate::synth::voice_limit::VoiceLimit;
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
///
/// Every field except the last two is resolved from the row's own descriptor
/// and canonical value where the row is built. `requested_value` and
/// `valid_actions` are not: they are facts about this row's relationship to the
/// model as a whole — what an in-flight structural edit is moving it toward,
/// and what the reducer would accept were it the focused row — so both are
/// resolved for every control in exactly one later pass
/// ([`project_control_intent`]) and nowhere else. Each row-building site leaves
/// them unresolved rather than answering the question locally, because a dozen
/// local answers is exactly the second vocabulary the declaration forbids.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticControlViewModel {
    path: FocusPath,
    label: String,
    kind: SemanticControlKind,
    value: SemanticControlValue,
    selected_label: Option<String>,
    numeric_range: Option<SemanticNumericRange>,
    unit: Option<String>,
    enabled: bool,
    visible: bool,
    focusable: bool,
    editable: bool,
    focused: bool,
    status: Option<SemanticLifecycleStatus>,
    error: Option<SemanticError>,
    requested_value: Option<SemanticControlValue>,
    requested_label: Option<String>,
    patch_interaction: Option<PatchInteraction>,
    valid_actions: Vec<ValidAction>,
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

    /// The authored label the owning descriptor gives the option this row's
    /// stored choice id names, or `None` on a row whose value is not a choice.
    ///
    /// The canonical value stays the stored id — [`Self::value`] still carries
    /// `ParameterValue::Choice("sf2.bank-0.program-40")`, because that is what
    /// the config holds and what the reducer edits. This carries the *name*
    /// beside it, from the one producer that owns it
    /// ([`ParameterSpec::choices`]), exactly as
    /// [`crate::control::PatchPageParameterRow::selected_label`] already does.
    ///
    /// It exists because without it a choice id was on screen. The webview
    /// consumes exactly the serde serialization of this model, so the PATCH page's
    /// `selectedLabel` never reached it and the shipped Preset row painted
    /// `sf2.bank-0.program-40` while `DESIGN.md` calls it "the **authored-name**
    /// Preset row" and declares SoundFont presets "labeled with exact authored
    /// SF2 names". The product contract forbids a serialization key on
    /// screen as a *label*; this key reached the screen as a *value*, which the
    /// label guard could not see.
    pub fn selected_label(&self) -> Option<&str> {
        self.selected_label.as_deref()
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

    /// The value an in-flight structural edit correlated to this control is
    /// moving it toward, or `None` on a settled row.
    ///
    /// Always sourced from the correlated lifecycle already in canonical state,
    /// never from the input that triggered the edit: a locally optimistic value
    /// would break the one-way loop while appearing to work.
    pub const fn requested_value(&self) -> Option<&SemanticControlValue> {
        self.requested_value.as_ref()
    }

    /// [`Self::selected_label`] for [`Self::requested_value`]: the authored
    /// name for the choice id an in-flight structural edit is moving this row
    /// toward, or `None` when the requested value is not a choice.
    ///
    /// The lifecycle band renders the requested value through the same value
    /// presentation the active value uses, so without this the row would paint
    /// its authored name above and the raw choice id below.
    pub fn requested_label(&self) -> Option<&str> {
        self.requested_label.as_deref()
    }

    /// The capability-declared interaction this row participates in, or `None`
    /// for a row no descriptor parameter stands behind (the engine row, the
    /// envelope rows, the slot occupancy rows, the Utility rows).
    ///
    /// This is a *different fact* from [`Self::editable`] and is why the
    /// detail surface needs it. `editable` answers "would the reducer accept
    /// an adjustment here, now" and is uniformly `false` on every detail row
    /// in this phase, so it discriminates nothing; `patch_interaction`
    /// answers "what did the capability declare this parameter to be", which
    /// is what the detail shell marks a read-only section from. It is
    /// projected from the same single producer the PATCH page reads
    /// ([`ParameterSpec::patch_interaction`]) rather than re-derived, so the
    /// two documents cannot disagree.
    pub const fn patch_interaction(&self) -> Option<PatchInteraction> {
        self.patch_interaction
    }

    /// This control's own accepted action list — what the reducer would accept
    /// were this the focused row.
    ///
    /// Resolved by the same pure resolver that computes the model-level list,
    /// so at the actually-focused row the two are the same value.
    pub fn valid_actions(&self) -> &[ValidAction] {
        &self.valid_actions
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticSurfaceRole {
    Main,
    PersistentSide,
    /// The subordinate PATCH detail surface: present exactly while a detail
    /// entry is open, never a context's resting place.
    Detail,
}

/// Typed, read-only canonical summary for one semantic surface.
// `rename_all` renames a tagged enum's *variants*, never a struct variant's
// fields, so without `rename_all_fields` every summary leaf below serializes
// snake_case inside an otherwise camelCase schema. The three tagged unions
// carrying that defect — `PatchDetailSubject`, this one, and `MixerControlId`
// — moved together rather than one at a time, because a half-fixed schema is
// harder to read than a uniformly wrong one.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
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
    /// The open detail surface names only its Patch and its subject: the
    /// content resolves from the installed descriptor the subject names, so
    /// the summary is not a second copy of the schema.
    PatchDetail {
        patch_id: PatchId,
        subject: PatchDetailSubject,
    },
}

impl SemanticSurfaceSummary {
    /// The Patch this surface speaks for, or `None` for a MIXER surface.
    pub const fn patch_id(&self) -> Option<PatchId> {
        match self {
            Self::Patch { patch_id, .. }
            | Self::PatchUtility { patch_id, .. }
            | Self::PatchDetail { patch_id, .. } => Some(*patch_id),
            Self::Mixer { .. } | Self::MixerInspector { .. } => None,
        }
    }
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
    DuplicateControlValidAction,
    PatchIdentityDisagreement,
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
            Self::DuplicateControlValidAction => {
                "semantic projection control contains a duplicate valid action"
            }
            Self::PatchIdentityDisagreement => {
                "semantic projection pairs one Patch's identity with another's"
            }
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
        "focusPath.controlId.id.trackId",
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
        "returnPath.origin.controlId.id.trackId",
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
        "surfaces[].controls[].patchInteraction",
        "surfaces[].controls[].path.capabilityId",
        "surfaces[].controls[].path.capabilityId.id",
        "surfaces[].controls[].path.capabilityId.kind",
        "surfaces[].controls[].path.context",
        "surfaces[].controls[].path.controlId.id",
        "surfaces[].controls[].path.controlId.id.bus",
        "surfaces[].controls[].path.controlId.id.kind",
        "surfaces[].controls[].path.controlId.id.parameter",
        "surfaces[].controls[].path.controlId.id.trackId",
        "surfaces[].controls[].path.controlId.kind",
        "surfaces[].controls[].path.modalId",
        "surfaces[].controls[].path.patchId",
        "surfaces[].controls[].path.surface",
        "surfaces[].controls[].requestedLabel",
        "surfaces[].controls[].requestedValue",
        "surfaces[].controls[].requestedValue.kind",
        "surfaces[].controls[].requestedValue.value",
        "surfaces[].controls[].requestedValue.value.kind",
        "surfaces[].controls[].requestedValue.value.value",
        // The authored name for a choice row's stored id. Present on every
        // control and `null` wherever the value is not a choice, so a fixture
        // that never opens a choice row still discovers the leaf.
        "surfaces[].controls[].selectedLabel",
        "surfaces[].controls[].status",
        "surfaces[].controls[].status.graphRevision",
        "surfaces[].controls[].status.kind",
        "surfaces[].controls[].status.label",
        "surfaces[].controls[].status.requestId",
        "surfaces[].controls[].status.targetGraphRevision",
        "surfaces[].controls[].unit",
        "surfaces[].controls[].validActions[].action.kind",
        "surfaces[].controls[].validActions[].action.payload",
        "surfaces[].controls[].validActions[].hint",
        "surfaces[].controls[].validActions[].label",
        "surfaces[].controls[].value.kind",
        "surfaces[].controls[].value.value",
        "surfaces[].controls[].value.value.kind",
        "surfaces[].controls[].value.value.locator",
        "surfaces[].controls[].value.value.value",
        "surfaces[].controls[].visible",
        "surfaces[].id",
        "surfaces[].label",
        "surfaces[].role",
        "surfaces[].summary.capabilityId",
        "surfaces[].summary.effectCount",
        "surfaces[].summary.focusedControl.bus",
        "surfaces[].summary.focusedControl.kind",
        "surfaces[].summary.focusedControl.parameter",
        "surfaces[].summary.focusedControl.trackId",
        "surfaces[].summary.focusedTrack",
        "surfaces[].summary.globalParameterCount",
        "surfaces[].summary.kind",
        "surfaces[].summary.patchCount",
        "surfaces[].summary.patchId",
        "surfaces[].summary.patchName",
        "surfaces[].summary.routedPatches[].patchId",
        "surfaces[].summary.routedPatches[].patchName",
        // The open detail surface's subject. `capability_id` and `kind` are
        // discovered for either subject variant; `slot_id` only for `Effect`,
        // which is the variant that names an exact occupied position — so a
        // fixture that opens only an instrument detail entry cannot see it.
        "surfaces[].summary.subject.capabilityId",
        "surfaces[].summary.subject.kind",
        "surfaces[].summary.subject.slotId",
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

    /// The one Patch identity every PATCH field of this model agrees on, or
    /// `None` in MIXER.
    ///
    /// Safe to read as a single value because [`validate_data`] refuses to
    /// build a model whose focus path, PATCH surface summaries, and control
    /// paths name more than one Patch (NFR-005).
    pub fn patch_identity(&self) -> Option<PatchId> {
        self.data.focus_path.patch_id()
    }

    pub fn surface(&self, id: SurfaceId) -> Option<&SemanticSurfaceViewModel> {
        self.data.surfaces.iter().find(|surface| surface.id == id)
    }

    /// The row the active focus names, or `None` when the focus rests on a
    /// surface root that owns no row.
    ///
    /// The one place a projection consumer may read the focused row's authored
    /// label. Any screen string naming the cursor's position — the shell footer
    /// breadcrumb is the only one today — composes from this rather than from
    /// the control identity, because the identity is a serialization key and a
    /// key on screen is the defect T016 exists to close.
    pub fn focused_control(&self) -> Option<&SemanticControlViewModel> {
        self.surface(self.active_surface())?
            .controls()
            .iter()
            .find(|control| control.focused)
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
        let mut surfaces = match state.context() {
            TopLevelContext::Patch => project_patch_surfaces(state, &resolver, &status, &errors)?,
            TopLevelContext::Mixer => project_mixer_surfaces(state, &resolver, &status, &errors)?,
        };
        project_control_intent(state, &resolver, &mut surfaces)?;
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
        requested_value: None,
        requested_label: None,
        patch_interaction: None,
        selected_label: None,
        valid_actions: Vec::new(),
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

/// Resolves, for every projected control, the two facts that belong to the
/// model as a whole rather than to one row's descriptor.
///
/// **This is the only site that answers either question.** A per-row action
/// list computed anywhere else would be a second input vocabulary, and a
/// requested value written anywhere else would be a locally optimistic guess —
/// the two failure modes the declaration names by name. Both are greppable:
/// nothing else in this crate assigns `valid_actions` or `requested_value` on a
/// [`SemanticControlViewModel`].
///
/// The row's action list comes from [`SemanticResolver::valid_actions`] — the
/// same function, not a lookalike — run over the counterfactual state in which
/// this row is the focused one. At the row that really *is* focused there is no
/// counterfactual to build: the model-level list is reused by value, so the two
/// agree by construction rather than by two computations happening to match.
///
/// A row whose counterfactual does not exist projects an empty list. That is a
/// projected fact and not an omission: it says the reducer would accept nothing
/// there.
fn project_control_intent(
    state: &AppState,
    resolver: &SemanticResolver<'_>,
    surfaces: &mut [SemanticSurfaceViewModel],
) -> Result<(), SemanticGraphicalViewModelError> {
    let focused_path = state.interaction().focus_path();
    let focused_actions = resolver.valid_actions();
    for surface in surfaces.iter_mut() {
        for control in &mut surface.controls {
            let (requested_value, requested_label) = project_requested_value(state, &control.path)?;
            control.requested_value = requested_value;
            control.requested_label = requested_label;
            control.valid_actions = if &control.path == focused_path {
                focused_actions.clone()
            } else {
                state
                    .with_counterfactual_focus(&control.path)
                    .map(|candidate| SemanticResolver::new(&candidate).valid_actions())
                    .unwrap_or_default()
            };
        }
    }
    Ok(())
}

/// Resolves what an in-flight structural edit is moving one control toward.
///
/// Everything here is read from the correlated lifecycle already in canonical
/// state — the intent the reducer accepted and the registries it validated the
/// intent against. Nothing is read from the input that requested the edit, so a
/// row can only claim to be moving toward a value the reducer has already
/// agreed to move it toward.
///
/// The requested value always carries the same shape as the row's active value,
/// so a page can show the pair without knowing which control it is looking at.
///
/// Returns the value and, when that value is a choice, the authored name for
/// it — resolved from the *intent's own* capability and parameter, so the
/// requested name comes from the same descriptor the reducer will edit through
/// rather than from a search for whichever spec happens to declare that id.
fn project_requested_value(
    state: &AppState,
    path: &FocusPath,
) -> Result<(Option<SemanticControlValue>, Option<String>), SemanticGraphicalViewModelError> {
    let Some(correlation) = state.engine_selection().correlation() else {
        return Ok((None, None));
    };
    let occupancy_value = |entry: Option<&crate::synth::EffectCapabilityId>| {
        entry.map_or_else(
            || Ok("Empty".to_owned()),
            |id| {
                state
                    .effects()
                    .descriptor(id)
                    .map(|descriptor| descriptor.label().to_owned())
                    .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)
            },
        )
    };
    let targets_focused_patch =
        correlation.patch_id().is_some() && correlation.patch_id() == path.patch_id();
    let requested = match (path.control_id(), correlation.intent()) {
        (
            crate::control::SemanticControlId::Patch(PatchControlId::Engine),
            crate::control::StructuralEditIntent::ReplaceCapability {
                target_capability_id,
            },
        ) if targets_focused_patch => {
            let descriptor = state
                .capabilities()
                .descriptor(target_capability_id)
                .ok_or(SemanticGraphicalViewModelError::InvalidInstrumentConfig)?;
            Some(SemanticControlValue::Identity(
                descriptor.label().to_owned(),
            ))
        }
        // The voice-limit row is not the row the swap was requested from, but
        // it is a row the swap moves: the carry-over is lossy by design, and a
        // narrowing that the player only discovers afterwards is exactly the
        // silent loss the declaration forbids. The same typed outcome the
        // commit will apply decides whether there is anything to say — a
        // widening swap preserves the player's value, so it says nothing.
        (
            crate::control::SemanticControlId::Patch(PatchControlId::VoiceLimit),
            crate::control::StructuralEditIntent::ReplaceCapability {
                target_capability_id,
            },
        ) if targets_focused_patch => {
            let patch_id = path
                .patch_id()
                .ok_or(SemanticGraphicalViewModelError::MissingPatch)?;
            let patch = state
                .patches()
                .iter()
                .find(|patch| patch.id() == patch_id)
                .ok_or(SemanticGraphicalViewModelError::MissingPatch)?;
            let policy = state
                .capabilities()
                .descriptor(target_capability_id)
                .ok_or(SemanticGraphicalViewModelError::InvalidInstrumentConfig)?
                .voice_policy();
            match crate::synth::VoiceLimitCarryOver::resolve(patch.voice_limit(), policy) {
                crate::synth::VoiceLimitCarryOver::Preserved(_) => None,
                crate::synth::VoiceLimitCarryOver::Clamped { limit, .. } => {
                    Some(SemanticControlValue::Scalar(f64::from(limit.value())))
                }
            }
        }
        (
            crate::control::SemanticControlId::Patch(PatchControlId::Capability(id)),
            crate::control::StructuralEditIntent::ReplaceParameterChoice {
                capability_id,
                parameter_id,
                choice_id,
            },
        ) if targets_focused_patch
            && id == parameter_id
            && path.capability_id().is_none_or(|focus_capability| {
                focus_capability == &FocusCapabilityId::Instrument(capability_id.clone())
            }) =>
        {
            Some(SemanticControlValue::Parameter(ParameterValue::Choice(
                choice_id.clone(),
            )))
        }
        (
            crate::control::SemanticControlId::Patch(PatchControlId::EffectSlot(index)),
            crate::control::StructuralEditIntent::SetSlotOccupancy {
                patch_id,
                slot,
                entry,
            },
        ) if Some(*patch_id) == path.patch_id() && index == slot => Some(
            SemanticControlValue::Identity(occupancy_value(entry.as_ref())?),
        ),
        (
            crate::control::SemanticControlId::Mixer(MixerControlId::ReturnOccupancy { bus }),
            crate::control::StructuralEditIntent::SetReturnOccupancy {
                bus: target_bus,
                entry,
            },
        ) if bus == target_bus => Some(SemanticControlValue::Identity(occupancy_value(
            entry.as_ref(),
        )?)),
        _ => None,
    };
    // The name for the requested choice id, from the descriptor the intent
    // names. Without it the lifecycle band paints `sf2.bank-0.program-41`
    // beneath a row whose active value reads "Violin" — the same defect F-33
    // closes on the active value, one line lower on the same row.
    let requested_label = match (&requested, correlation.intent()) {
        (
            Some(SemanticControlValue::Parameter(ParameterValue::Choice(choice_id))),
            crate::control::StructuralEditIntent::ReplaceParameterChoice {
                capability_id,
                parameter_id,
                ..
            },
        ) => state
            .capabilities()
            .descriptor(capability_id)
            .and_then(|descriptor| descriptor.parameter(parameter_id))
            .and_then(|spec| {
                spec.choices()
                    .iter()
                    .find(|choice| choice.id() == choice_id)
                    .map(|choice| choice.label().to_owned())
            }),
        _ => None,
    };
    Ok((requested, requested_label))
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
        requested_value: None,
        requested_label: None,
        patch_interaction: None,
        selected_label: None,
        valid_actions: Vec::new(),
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
            requested_value: None,
            requested_label: None,
            patch_interaction: None,
            selected_label: None,
            valid_actions: Vec::new(),
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
            requested_value: None,
            requested_label: None,
            patch_interaction: None,
            selected_label: None,
            valid_actions: Vec::new(),
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
            let crate::control::SemanticControlId::Patch(control) = path.control_id() else {
                return Err(SemanticGraphicalViewModelError::InvalidFocusPath);
            };
            // Every row reads its label, bounds, and steps from the same
            // canonical descriptor the reducer edits through, so the panel
            // cannot present a range the reducer will not honour.
            let (label, kind, value, numeric_range, unit) = match control {
                PatchControlId::Output(parameter) => {
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
                    (
                        descriptor.label().to_owned(),
                        kind,
                        value,
                        numeric_range,
                        unit,
                    )
                }
                // The one canonical master gain, read through the one global
                // descriptor — the same value and bounds the MIXER Inspector's
                // own row projects. PATCH holds no copy of it.
                PatchControlId::Global(parameter) => {
                    let descriptor = parameter.descriptor();
                    (
                        // The descriptor's authored label. `name()` is the
                        // serialization key `masterGainDb`, which is what used
                        // to reach the screen here.
                        descriptor.label().to_owned(),
                        SemanticControlKind::Continuous,
                        SemanticControlValue::Scalar(state.global_row_value(*parameter) as f64),
                        Some(SemanticNumericRange::new(
                            descriptor.minimum() as f64,
                            descriptor.maximum() as f64,
                            descriptor.fine_step() as f64,
                            descriptor.coarse_step() as f64,
                        )),
                        None,
                    )
                }
                PatchControlId::MidiInput => (
                    "MIDI Input".to_owned(),
                    SemanticControlKind::Stepped,
                    SemanticControlValue::Scalar(f64::from(patch.channel().value())),
                    Some(SemanticNumericRange::new(
                        f64::from(MidiChannel::MIN),
                        f64::from(MidiChannel::MAX),
                        1.0,
                        1.0,
                    )),
                    None,
                ),
                PatchControlId::VoiceLimit => {
                    let descriptor = VoiceLimit::descriptor();
                    (
                        descriptor.label().to_owned(),
                        SemanticControlKind::from(descriptor.kind()),
                        SemanticControlValue::Scalar(f64::from(patch.voice_limit().value())),
                        Some(SemanticNumericRange::new(
                            f64::from(descriptor.minimum()),
                            f64::from(descriptor.maximum()),
                            f64::from(descriptor.fine_step()),
                            f64::from(descriptor.coarse_step()),
                        )),
                        descriptor.unit().map(str::to_owned),
                    )
                }
                PatchControlId::Engine
                | PatchControlId::Envelope(_)
                | PatchControlId::Capability(_)
                | PatchControlId::EffectSlot(_)
                | PatchControlId::Effect(..) => {
                    return Err(SemanticGraphicalViewModelError::InvalidFocusPath);
                }
            };
            Ok(SemanticControlViewModel {
                focused: active == &path,
                path,
                label,
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
                requested_value: None,
                requested_label: None,
                patch_interaction: None,
                selected_label: None,
                valid_actions: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut surfaces = vec![
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
    ];

    // The detail surface is present exactly while the reducer holds a detail
    // entry, and absent otherwise: hosts never render a stale one and never
    // synthesize one. Its whole content resolves from the installed descriptor
    // the subject names, so this shell branches on no capability.
    if let Some(subject) = state.interaction().detail_subject() {
        let detail_paths = resolver
            .patch_detail_paths(patch_id, subject)
            .map_err(map_resolver_error)?;
        // A capability mid-preparation reports its typed lifecycle on its own
        // rows rather than leaving them looking settled. The section set is
        // never empty or stale — it is resolved from the installed descriptor
        // just above — so what a preparing subject needs to say is *that it is
        // preparing*, which is exactly what this projects.
        let subject_status = (!lifecycle_editable
            && state
                .engine_selection()
                .correlation()
                .is_some_and(|correlation| correlation.patch_id() == Some(patch_id)))
        .then(|| status.clone());
        // No detail row is editable in this phase: the reducer accepts no
        // adjustment on `PatchDetail`, so an editable row would advertise an
        // edit that is refused.
        //
        // This is a *surface-level* fact about what the reducer accepts, and it
        // is uniform — a `StructuralChoice` row and a `ReadOnly` row project
        // `editable: false` alike here, so this field discriminates nothing
        // about the capability's own declaration.
        //
        // The capability-declared read-only fact is a different fact, and it
        // now rides every descriptor-backed row as `patchInteraction` — the
        // same single producer `PatchPageParameterRow` reads
        // (`ParameterSpec::patch_interaction`), projected rather than
        // re-derived. All three Braids rows and SoundFont's `file` row declare
        // `ReadOnly`; SoundFont's `preset` declares `StructuralChoice`, so one
        // detail surface carries both. A page marking a read-only row "in text
        // or shape" reads that leaf, never this bool.
        //
        // Keeping the fact on `patchPage` alone does not reach *this* screen:
        // the webview consumes exactly the serde serialization of
        // `SemanticGraphicalViewModel`, and `patchPage`
        // belongs to the StateTree observation, which no shipped surface
        // paints. Without this leaf the declared "read-only marked in text or
        // shape" rule is unrenderable.
        let detail_editable = false;
        let mut detail_controls = Vec::with_capacity(detail_paths.len());
        for path in detail_paths {
            let control = match (subject, path.control_id()) {
                (
                    PatchDetailSubject::Instrument { capability_id },
                    crate::control::SemanticControlId::Patch(PatchControlId::Capability(id)),
                ) => {
                    let subject_descriptor = state
                        .capabilities()
                        .descriptor(capability_id)
                        .ok_or(SemanticGraphicalViewModelError::InvalidInstrumentConfig)?;
                    let spec = subject_descriptor
                        .parameter(id)
                        .ok_or(SemanticGraphicalViewModelError::InvalidFocusPath)?;
                    let (enabled, visible) =
                        parameter_availability(spec, patch.instrument_config());
                    control_from_parameter(
                        path.clone(),
                        spec,
                        parameter_value(spec, patch.instrument_config())?,
                        ParameterControlProjection {
                            enabled,
                            visible,
                            focusable: true,
                            editable: detail_editable,
                            active,
                            status: subject_status.clone(),
                            errors,
                        },
                    )
                }
                (
                    PatchDetailSubject::Effect {
                        slot_id,
                        capability_id,
                    },
                    crate::control::SemanticControlId::Patch(PatchControlId::Effect(_, id)),
                ) => {
                    let occupant = patch
                        .effect_slots()
                        .iter()
                        .flatten()
                        .find(|effect| effect.slot_id() == *slot_id)
                        .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)?;
                    let subject_descriptor = state
                        .effects()
                        .descriptor(capability_id)
                        .ok_or(SemanticGraphicalViewModelError::InvalidEffectConfig)?;
                    let spec = subject_descriptor
                        .parameter(id)
                        .ok_or(SemanticGraphicalViewModelError::InvalidFocusPath)?;
                    let (enabled, visible) = effect_parameter_availability(spec, occupant);
                    control_from_parameter(
                        path.clone(),
                        spec,
                        effect_parameter_value(spec, occupant)?,
                        ParameterControlProjection {
                            enabled,
                            visible,
                            focusable: true,
                            editable: detail_editable,
                            active,
                            status: subject_status.clone(),
                            errors,
                        },
                    )
                }
                _ => return Err(SemanticGraphicalViewModelError::InvalidFocusPath),
            };
            detail_controls.push(control);
        }
        surfaces.push(SemanticSurfaceViewModel {
            id: SurfaceId::PatchDetail,
            label: SurfaceId::PatchDetail.label().to_owned(),
            role: SemanticSurfaceRole::Detail,
            controls: detail_controls,
            summary: SemanticSurfaceSummary::PatchDetail {
                patch_id,
                subject: subject.clone(),
            },
        });
    }

    Ok(surfaces)
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
                    requested_value: None,
                    requested_label: None,
                    patch_interaction: None,
                    selected_label: None,
                    valid_actions: Vec::new(),
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
                    requested_value: None,
                    requested_label: None,
                    patch_interaction: None,
                    selected_label: None,
                    valid_actions: Vec::new(),
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
                    requested_value: None,
                    requested_label: None,
                    patch_interaction: None,
                    selected_label: None,
                    valid_actions: Vec::new(),
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
                    label: descriptor.label().to_owned(),
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
                    requested_value: None,
                    requested_label: None,
                    patch_interaction: None,
                    selected_label: None,
                    valid_actions: Vec::new(),
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
        requested_value: None,
        requested_label: None,
        patch_interaction: None,
        selected_label: None,
        valid_actions: Vec::new(),
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
    // The authored name for the stored choice id, from the descriptor that
    // declared both. The canonical value below stays the id; this carries the
    // name beside it, the way `PatchPageParameterRow::selected_label` already
    // does — one producer, `ParameterSpec::choices`, and no second vocabulary.
    //
    // Resolved here rather than at each caller because this is the one site
    // that builds a descriptor-backed row, and therefore the only site whose
    // value can be a choice at all.
    let selected_label = match &value {
        SemanticControlValue::Parameter(ParameterValue::Choice(choice_id)) => spec
            .choices()
            .iter()
            .find(|choice| choice.id() == choice_id)
            .map(|choice| choice.label().to_owned()),
        _ => None,
    };
    SemanticControlViewModel {
        error: error_for_path(projection.errors, &path),
        focused: projection.active == &path,
        path,
        label: spec.label().to_owned(),
        kind: spec.kind().into(),
        value,
        selected_label,
        numeric_range,
        unit: spec.unit().map(str::to_owned),
        enabled: projection.enabled,
        visible: projection.visible,
        focusable: projection.focusable,
        editable: projection.editable,
        status: projection.status,
        requested_value: None,
        requested_label: None,
        patch_interaction: Some(spec.patch_interaction()),
        valid_actions: Vec::new(),
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
        requested_value: None,
        requested_label: None,
        patch_interaction: None,
        selected_label: None,
        valid_actions: Vec::new(),
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
    // A per-row list is held to the same shape rule as the model-level one:
    // ordered and duplicate-free. It comes from the same resolver, so this can
    // only fail if that resolver stopped being duplicate-free — in which case
    // both lists are wrong and the model should not be built at all.
    if controls.iter().any(|control| {
        control
            .valid_actions
            .iter()
            .map(ValidAction::action)
            .collect::<HashSet<_>>()
            .len()
            != control.valid_actions.len()
    }) {
        return Err(SemanticGraphicalViewModelError::DuplicateControlValidAction);
    }
    // NFR-005: within one projected model, every PATCH identity agrees — the
    // focus path, every PATCH surface's summary, and every control path. A
    // patch switch is one accepted event and one reprojection, so a projection
    // that named two Patches would be one that had shown a half-applied switch.
    // Checking the set size rather than comparing against the focus makes the
    // claim symmetric: no field is privileged, and a disagreement anywhere is
    // the same defect.
    let identities = core::iter::once(data.focus_path.patch_id())
        .chain(
            data.surfaces
                .iter()
                .map(|surface| surface.summary.patch_id()),
        )
        .chain(controls.iter().map(|control| control.path.patch_id()))
        .flatten()
        .collect::<HashSet<_>>();
    if identities.len() > 1 {
        return Err(SemanticGraphicalViewModelError::PatchIdentityDisagreement);
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

/// WP03 acceptance for the two per-row facts, the detail surface, authored
/// labels, and one-generation identity agreement.
#[cfg(test)]
mod projection_enrichment_tests {
    use super::*;
    use crate::adapter::braids_capability::{BraidsCapability, BRAIDS_CAPABILITY_ID};
    use crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
    use crate::adapter::production_effects::{
        production_chorus_config, production_effect_registry,
    };
    use crate::adapter::production_instruments::{
        production_capability_registry, production_soundfont_capability,
    };
    use crate::control::{
        AppEvent, Direction, EventRejection, InteractionMode, PatchPageSection,
        PatchPageSlotOccupancy, SemanticAction, SemanticControlId,
    };
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::patch_output::PatchOutput;
    use crate::synth::effect_slot_id::EffectSlotIndex;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::{EffectSlotId, Patch};
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use std::collections::BTreeSet;

    /// Two installed Patches whose engines disagree about everything that
    /// matters here: SoundFont's first detail row is also a PATCH Main row and
    /// its voice policy is engine-managed; Braids hosts no `Capability` row on
    /// PATCH Main at all and caps at sixteen voices. Patch 1's first slot is
    /// occupied so an *effect* detail subject is reachable too.
    fn mixed_state() -> AppState {
        let soundfont = create_soundfont_config(
            &production_soundfont_capability().unwrap(),
            SoundFontInstrument::new(0, 40, false).unwrap(),
        )
        .unwrap();
        let braids = BraidsCapability::new().unwrap().default_config().unwrap();
        let mut state = AppState::new_with_effects(
            production_capability_registry().unwrap(),
            production_effect_registry().unwrap(),
            GlobalParameters::new(-3.0).unwrap(),
        );
        state
            .apply(AppEvent::InstallPatches(vec![
                Patch::new(
                    PatchId::new(1).unwrap(),
                    "Lead".to_owned(),
                    soundfont,
                    MidiChannel::new(0).unwrap(),
                    PatchOutput::to_track(MixerTrackId::new(0).unwrap()),
                )
                .with_effect_slot(
                    EffectSlotIndex::ALL[0],
                    production_chorus_config(EffectSlotId::new(1).unwrap()).unwrap(),
                ),
                Patch::new(
                    PatchId::new(2).unwrap(),
                    "Bass".to_owned(),
                    braids,
                    MidiChannel::new(1).unwrap(),
                    PatchOutput::to_track(MixerTrackId::new(1).unwrap()),
                ),
            ]))
            .unwrap();
        state
    }

    fn patch_state() -> AppState {
        let mut state = mixed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
    }

    fn project(state: &AppState) -> SemanticGraphicalViewModel {
        SemanticGraphicalViewModel::project(state, "wp03-state-hash")
            .expect("the fixture state must project")
    }

    fn controls(model: &SemanticGraphicalViewModel) -> Vec<&SemanticControlViewModel> {
        model
            .surfaces()
            .iter()
            .flat_map(SemanticSurfaceViewModel::controls)
            .collect()
    }

    fn control_at<'a>(
        model: &'a SemanticGraphicalViewModel,
        control: &SemanticControlId,
    ) -> &'a SemanticControlViewModel {
        controls(model)
            .into_iter()
            .find(|candidate| candidate.path().control_id() == control)
            .unwrap_or_else(|| panic!("{control:?} must be projected"))
    }

    /// Walks the reducer until `predicate` holds, refusing to loop forever.
    fn navigate_until(state: &mut AppState, predicate: impl Fn(&FocusPath) -> bool) {
        for _ in 0..256 {
            if predicate(state.interaction().focus_path()) {
                return;
            }
            state
                .apply(AppEvent::Navigate(Direction::Down))
                .expect("the fixture order is long enough to reach the target row");
        }
        panic!("no row in the canonical order satisfied the predicate");
    }

    // -----------------------------------------------------------------
    // T013 — per-row valid actions
    // -----------------------------------------------------------------

    /// The contract, not a coincidence: at the focused row the two lists are
    /// the same value, because the projection reuses the model-level list there
    /// rather than computing a second one.
    #[test]
    fn the_focused_rows_action_list_is_the_model_level_list_element_for_element() {
        for state in [patch_state(), mixed_state()] {
            let model = project(&state);
            let focused = controls(&model)
                .into_iter()
                .filter(|control| control.focused())
                .collect::<Vec<_>>();
            assert_eq!(focused.len(), 1, "exactly one control is focused");
            assert_eq!(focused[0].path(), model.focus_path());
            assert!(
                !model.valid_actions().is_empty(),
                "the fixture must offer something, or this proves nothing"
            );
            assert_eq!(focused[0].valid_actions(), model.valid_actions());
        }
    }

    /// The discriminating half: an unfocused row's list is *its own*. Entering
    /// the detail surface is accepted only from a row that resolves a subject,
    /// so the engine row offers it and an envelope row does not — and neither
    /// of those rows is the focused one in the same projection.
    #[test]
    fn each_rows_action_list_reflects_that_row_rather_than_the_focus() {
        let mut state = patch_state();
        // Focus a row that is neither of the two rows under test.
        navigate_until(&mut state, |path| {
            matches!(
                path.control_id(),
                SemanticControlId::Patch(PatchControlId::EffectSlot(_))
            )
        });
        let model = project(&state);
        let enter_detail = SemanticAction::EnterSurface(SurfaceId::PatchDetail);
        let offers_detail = |control: &SemanticControlViewModel| {
            control
                .valid_actions()
                .iter()
                .any(|valid| valid.action() == &enter_detail)
        };

        let engine = control_at(&model, &SemanticControlId::Patch(PatchControlId::Engine));
        assert!(!engine.focused());
        assert!(
            offers_detail(engine),
            "the engine row resolves an instrument subject, so it offers entry"
        );

        let envelope = controls(&model)
            .into_iter()
            .find(|control| {
                matches!(
                    control.path().control_id(),
                    SemanticControlId::Patch(PatchControlId::Envelope(_))
                )
            })
            .expect("the ADSR rows are projected");
        assert!(!envelope.focused());
        assert!(
            !offers_detail(envelope),
            "an envelope row resolves no subject, so it must not offer entry"
        );

        // A Utility row is on another surface entirely: its counterfactual is
        // one where that surface is open, so it offers Return and the main
        // rows do not.
        let voice_limit = control_at(
            &model,
            &SemanticControlId::Patch(PatchControlId::VoiceLimit),
        );
        let offers_return = |control: &SemanticControlViewModel| {
            control
                .valid_actions()
                .iter()
                .any(|valid| valid.action() == &SemanticAction::Return)
        };
        assert!(offers_return(voice_limit));
        assert!(!offers_return(engine));
    }

    /// A row at a value boundary excludes the adjustment that would exceed it —
    /// on a row that is *not* the focused one, so the exclusion cannot have
    /// come from the model-level list.
    #[test]
    fn an_unfocused_row_at_its_boundary_excludes_the_direction_that_would_exceed_it() {
        let mut state = mixed_state();
        // Drive the first MIXER track's level to its ceiling.
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        let mut raised = 0;
        while state.apply(AppEvent::Adjust(Direction::Up)).is_ok() {
            raised += 1;
            assert!(raised < 512, "the level bound must be reachable");
        }
        assert!(raised > 0, "the fixture level must start below its ceiling");
        let raised_path = state.interaction().focus_path().clone();

        // Move the focus off it, then come back into Adjust mode so adjustment
        // is part of the offered vocabulary at all.
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Navigate))
            .unwrap();
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        assert_ne!(state.interaction().focus_path(), &raised_path);

        let model = project(&state);
        let at_bound = controls(&model)
            .into_iter()
            .find(|control| control.path() == &raised_path)
            .expect("the raised row is still projected");
        assert!(!at_bound.focused());
        let offers = |control: &SemanticControlViewModel, action: SemanticAction| {
            control
                .valid_actions()
                .iter()
                .any(|valid| valid.action() == &action)
        };
        assert!(
            !offers(at_bound, SemanticAction::Adjust(Direction::Up)),
            "a row at its ceiling must not offer the coarse increase"
        );
        assert!(
            !offers(at_bound, SemanticAction::Adjust(Direction::Right)),
            "a row at its ceiling must not offer the fine increase"
        );
        assert!(
            offers(at_bound, SemanticAction::Adjust(Direction::Down)),
            "the direction away from the bound is still available"
        );
    }

    /// Every per-row list is ordered like the descriptor and duplicate-free —
    /// the same shape rule the model-level list is held to, which
    /// `validate_data` refuses to build a model without.
    #[test]
    fn every_projected_rows_action_list_is_ordered_and_duplicate_free() {
        for state in [patch_state(), mixed_state()] {
            let model = project(&state);
            let order = SemanticAction::surface_descriptor();
            for control in controls(&model) {
                let positions = control
                    .valid_actions()
                    .iter()
                    .map(|valid| {
                        order
                            .iter()
                            .position(|candidate| candidate == valid.action())
                            .expect("a projected action is in the closed descriptor")
                    })
                    .collect::<Vec<_>>();
                assert!(
                    positions.windows(2).all(|pair| pair[0] < pair[1]),
                    "{:?} must project its actions in descriptor order without repeats",
                    control.path().control_id()
                );
            }
        }
    }

    // -----------------------------------------------------------------
    // T014 — requested value
    // -----------------------------------------------------------------

    /// Every row of a settled projection projects `None`. Counted rather than
    /// spot-checked, because "no row claims to be mid-edit" is the half of the
    /// contract a positive test cannot cover.
    #[test]
    fn a_settled_projection_has_exactly_zero_rows_claiming_a_requested_value() {
        for state in [patch_state(), mixed_state()] {
            let model = project(&state);
            let claiming = controls(&model)
                .into_iter()
                .filter(|control| control.requested_value().is_some())
                .count();
            assert_eq!(claiming, 0, "a settled projection moves nothing");
        }
    }

    /// An engine row mid-swap projects both the active capability and the
    /// requested one, and it is the only row that does.
    #[test]
    fn an_engine_row_mid_swap_projects_the_active_and_the_requested_capability() {
        let mut state = patch_state();
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert!(state.engine_selection().is_in_flight());

        let model = project(&state);
        let engine = control_at(&model, &SemanticControlId::Patch(PatchControlId::Engine));
        assert_eq!(
            engine.value(),
            &SemanticControlValue::Identity("HiDef SoundFont".to_owned()),
            "the active capability is still the source engine"
        );
        assert_eq!(
            engine.requested_value(),
            Some(&SemanticControlValue::Identity(
                "Mutable Instruments Braids".to_owned()
            )),
            "the requested capability is the one the correlated intent names"
        );

        // The requested capability is read from canonical state, not from the
        // input that triggered the edit: it equals the correlation's own target.
        let target = state
            .engine_selection()
            .correlation()
            .and_then(|correlation| correlation.target_capability_id().cloned())
            .expect("an in-flight capability swap correlates a target");
        assert_eq!(target.as_str(), BRAIDS_CAPABILITY_ID);
        assert_eq!(
            state
                .capabilities()
                .descriptor(&target)
                .unwrap()
                .label()
                .to_owned(),
            match engine.requested_value() {
                Some(SemanticControlValue::Identity(label)) => label.clone(),
                other => panic!("expected an identity requested value, got {other:?}"),
            }
        );
    }

    /// An engine swap that narrows the voice ceiling says so on the row that
    /// shows the limit. The carry-over is lossy by design, so the loss is
    /// reported through the same typed outcome the commit will apply rather
    /// than left for a player to find afterwards.
    #[test]
    fn a_swap_that_narrows_the_voice_limit_projects_the_narrowed_value_on_that_row() {
        let mut state = patch_state();
        let patch_id = state.interaction().patch_focus().unwrap();
        let before = state
            .patches()
            .iter()
            .find(|patch| patch.id() == patch_id)
            .unwrap()
            .voice_limit();
        assert_eq!(
            before.value(),
            64,
            "the SoundFont Patch starts at the engine-managed ceiling"
        );

        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();

        let model = project(&state);
        let limit = control_at(
            &model,
            &SemanticControlId::Patch(PatchControlId::VoiceLimit),
        );
        assert_eq!(
            limit.value(),
            &SemanticControlValue::Scalar(64.0),
            "the active limit is still the player's value"
        );
        assert_eq!(
            limit.requested_value(),
            Some(&SemanticControlValue::Scalar(16.0)),
            "Braids caps at sixteen, and a swap that costs the player 48 voices must say so"
        );

        // The widening direction says nothing: preserving the player's value is
        // not a change, so there is nothing to report.
        let mut widening = mixed_state();
        widening
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        widening
            .apply(AppEvent::SelectPatch(Direction::Right))
            .unwrap();
        assert_eq!(
            widening.patches()[1]
                .instrument_config()
                .capability_id()
                .as_str(),
            BRAIDS_CAPABILITY_ID
        );
        widening
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        widening.apply(AppEvent::Adjust(Direction::Left)).unwrap();
        assert_eq!(
            widening
                .engine_selection()
                .correlation()
                .and_then(|correlation| correlation.target_capability_id())
                .map(crate::synth::CapabilityId::as_str),
            Some(HIDEF_CAPABILITY_ID)
        );
        let widened = project(&widening);
        assert_eq!(
            control_at(
                &widened,
                &SemanticControlId::Patch(PatchControlId::VoiceLimit),
            )
            .requested_value(),
            None,
            "a widening swap preserves the value, so the row reports no move"
        );
    }

    /// An effect-slot occupancy row mid-change projects both what occupies the
    /// position now and what is being installed into it.
    #[test]
    fn an_effect_slot_row_mid_occupancy_change_projects_the_active_and_the_requested_entry() {
        let mut state = patch_state();
        navigate_until(&mut state, |path| {
            matches!(
                path.control_id(),
                SemanticControlId::Patch(PatchControlId::EffectSlot(slot)) if slot.index() == 1
            )
        });
        let slot_path = state.interaction().focus_path().clone();
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();

        let model = project(&state);
        let slot = controls(&model)
            .into_iter()
            .find(|control| control.path() == &slot_path)
            .expect("the occupancy row is projected");
        assert_eq!(
            slot.value(),
            &SemanticControlValue::Identity("Empty".to_owned()),
            "the position is still empty until the change commits"
        );
        let requested = slot
            .requested_value()
            .expect("a slot mid-occupancy-change reports what it is moving toward");
        assert_ne!(requested, slot.value());

        // Exactly one row is in flight.
        assert_eq!(
            controls(&model)
                .into_iter()
                .filter(|control| control.requested_value().is_some())
                .count(),
            1
        );
    }

    // -----------------------------------------------------------------
    // T015 — the detail surface
    // -----------------------------------------------------------------

    #[test]
    fn the_detail_surface_is_present_exactly_while_an_entry_is_open() {
        let mut state = patch_state();
        assert!(project(&state).surface(SurfaceId::PatchDetail).is_none());

        state
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        let open = project(&state);
        let detail = open
            .surface(SurfaceId::PatchDetail)
            .expect("an open entry projects its surface");
        assert_eq!(detail.role(), SemanticSurfaceRole::Detail);
        assert_eq!(open.active_surface(), SurfaceId::PatchDetail);

        state.apply_semantic_action(SemanticAction::Return).unwrap();
        assert!(
            project(&state).surface(SurfaceId::PatchDetail).is_none(),
            "leaving the surface removes it; hosts never see a stale one"
        );
    }

    /// One surface identity, two subjects. Both project under the same
    /// `SurfaceId` with the same role, and each one's controls and order come
    /// from the descriptor its subject names.
    #[test]
    fn an_instrument_subject_and_an_effect_subject_share_one_surface_identity() {
        let mut instrument = patch_state();
        instrument
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();

        let mut effect = patch_state();
        navigate_until(&mut effect, |path| {
            matches!(
                path.control_id(),
                SemanticControlId::Patch(PatchControlId::EffectSlot(slot)) if slot.index() == 0
            )
        });
        effect
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();

        let instrument_model = project(&instrument);
        let effect_model = project(&effect);
        let instrument_detail = instrument_model.surface(SurfaceId::PatchDetail).unwrap();
        let effect_detail = effect_model.surface(SurfaceId::PatchDetail).unwrap();

        assert_eq!(instrument_detail.id(), effect_detail.id());
        assert_eq!(instrument_detail.role(), effect_detail.role());
        assert_eq!(instrument_detail.label(), effect_detail.label());

        let SemanticSurfaceSummary::PatchDetail { subject, .. } = instrument_detail.summary()
        else {
            panic!("the detail surface carries a detail summary");
        };
        assert!(matches!(subject, PatchDetailSubject::Instrument { .. }));
        let SemanticSurfaceSummary::PatchDetail { subject, .. } = effect_detail.summary() else {
            panic!("the detail surface carries a detail summary");
        };
        assert!(matches!(subject, PatchDetailSubject::Effect { .. }));

        // Content is the descriptor's, in the descriptor's order, for both.
        let soundfont = instrument
            .capabilities()
            .descriptor(instrument.patches()[0].instrument_config().capability_id())
            .unwrap();
        let expected = soundfont
            .parameters()
            .map(|spec| spec.label().to_owned())
            .collect::<Vec<_>>();
        let projected = instrument_detail
            .controls()
            .iter()
            .map(|control| control.label().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(projected, expected, "section order is descriptor order");
        assert!(!effect_detail.controls().is_empty());
    }

    /// The detail surface is uneditable in this phase, and it says so rather
    /// than inviting an edit the reducer refuses.
    ///
    /// Deliberately *not* a read-only proof: `editable` is uniform here, so it
    /// cannot tell a `ReadOnly` row from a `StructuralChoice` one. The
    /// capability's own declaration reaches the screen through
    /// `patchInteraction` on the PATCH page, proved in
    /// `patch_page_projection.rs` by
    /// `the_declared_patch_interaction_reaches_the_detail_page_and_discriminates`.
    #[test]
    fn detail_controls_project_editable_false_because_the_reducer_refuses_to_adjust_them() {
        let mut state = patch_state();
        state
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        let model = project(&state);
        let detail = model.surface(SurfaceId::PatchDetail).unwrap();
        assert!(!detail.controls().is_empty());
        for control in detail.controls() {
            assert!(
                !control.editable(),
                "{} claims to be editable on a surface the reducer will not adjust",
                control.label()
            );
        }

        // The claim is derived, not asserted: adjustment really is refused.
        let mut adjusting = state.clone();
        adjusting
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        assert_eq!(
            adjusting.apply(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::ActionUnavailableInContext)
        );
    }

    /// A subject mid-preparation projects its typed lifecycle rather than an
    /// empty or stale section set.
    #[test]
    fn a_mid_preparation_subject_projects_its_lifecycle_and_keeps_its_sections() {
        let mut state = patch_state();
        navigate_until(&mut state, |path| {
            matches!(
                path.control_id(),
                SemanticControlId::Patch(PatchControlId::Capability(_))
            )
        });
        state
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        let settled = project(&state);
        let settled_rows = settled
            .surface(SurfaceId::PatchDetail)
            .unwrap()
            .controls()
            .len();
        assert!(settled_rows > 0);
        assert!(settled
            .surface(SurfaceId::PatchDetail)
            .unwrap()
            .controls()
            .iter()
            .all(|control| control.status().is_none()));

        // Request a structural choice from PATCH Main under the open entry.
        state.apply_semantic_action(SemanticAction::Return).unwrap();
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Navigate))
            .unwrap();
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        assert!(state.engine_selection().is_in_flight());

        let preparing = project(&state);
        let detail = preparing.surface(SurfaceId::PatchDetail).unwrap();
        assert_eq!(
            detail.controls().len(),
            settled_rows,
            "the section set is the descriptor's, not something preparation empties"
        );
        assert!(
            detail.controls().iter().all(|control| matches!(
                control.status().map(SemanticLifecycleStatus::kind),
                Some(EngineSelectionStatusKind::Preparing | EngineSelectionStatusKind::Activating)
            )),
            "a preparing subject reports its typed lifecycle on its own rows"
        );
    }

    // -----------------------------------------------------------------
    // T016 — authored labels, never serialization keys
    // -----------------------------------------------------------------

    /// Every serialization key this system addresses a value by: the name a
    /// descriptor carries in the state tree and the parameter snapshot, plus
    /// every leaf name the projections themselves are addressed by.
    ///
    /// Built as a *set* rather than a list of known offenders, so a future key
    /// leaking into a label fails this too — the reported `masterGainDb` is one
    /// member of it and gets no special treatment.
    fn serialization_keys(state: &AppState) -> BTreeSet<String> {
        let mut keys = BTreeSet::new();
        for descriptor in GlobalParameters::surface_descriptor() {
            keys.insert(descriptor.name().to_owned());
        }
        for descriptor in crate::synth::VoiceEnvelope::surface_descriptor() {
            keys.insert(descriptor.name().to_owned());
        }
        for descriptor in VoiceLimit::surface_descriptor() {
            keys.insert(descriptor.name().to_owned());
        }
        for descriptor in PatchOutput::surface_descriptor() {
            keys.insert(descriptor.name().to_owned());
        }
        for descriptor in
            crate::mixer::mixer_track_parameters::MixerTrackParameters::surface_descriptor()
        {
            keys.insert(descriptor.name().to_owned());
        }
        for descriptor in state.capabilities().descriptors() {
            keys.extend(descriptor.parameters().map(|spec| spec.id().to_string()));
        }
        for descriptor in state.effects().descriptors() {
            keys.extend(descriptor.parameters().map(|spec| spec.id().to_string()));
        }
        // Capability and section identities. A serialization key is the name a
        // value carries in the state tree, parameter snapshot, or leaf descriptor;
        // `patchPage.sections[].id`, `patchPage.engine.activeCapabilityId`, and
        // `patchPage.effects[].capabilityId` are all such names — so a label
        // reverted to one of them is the same defect as `masterGainDb` on a row.
        //
        // Without these the guard *walked* seven label sites it could not
        // *fail* on: three section labels, the engine choice and active labels,
        // and the two occupancy labels. A 17-site mutation sweep caught 10 and
        // missed these 7. The exact case the first
        // diagnosis named — a descriptor whose `label()` equalled its `id()` —
        // was unexpressible in the key set, so widening the *fixtures* bought
        // nothing for it.
        for descriptor in state.capabilities().descriptors() {
            keys.insert(descriptor.id().to_string());
            for section in descriptor.sections() {
                keys.insert(section.id().to_owned());
            }
        }
        for descriptor in state.effects().descriptors() {
            keys.insert(descriptor.id().to_string());
            for section in descriptor.sections() {
                keys.insert(section.id().to_owned());
            }
        }
        for path in SemanticGraphicalViewModel::serialized_leaf_descriptor()
            .iter()
            .chain(crate::control::PatchPageProjection::serialized_leaf_descriptor())
        {
            let leaf = path.rsplit('.').next().unwrap_or(path);
            keys.insert(leaf.trim_end_matches("[]").to_owned());
        }
        // Every control identity's own serialized form — `patch.voiceLimit`,
        // `patch.global.masterGainDb`. These are the keys the *footer* used to
        // compose its breadcrumb from, which is a different vocabulary from the
        // descriptor names the rows used, and a set that saw only the latter
        // could not fail on the former.
        for control in PatchControlId::UTILITY
            .iter()
            .cloned()
            .chain([PatchControlId::Engine])
        {
            keys.insert(control.as_str().into_owned());
        }
        for descriptor in state.capabilities().descriptors() {
            for spec in descriptor.parameters() {
                keys.insert(
                    PatchControlId::Capability(spec.id().clone())
                        .as_str()
                        .into_owned(),
                );
            }
        }
        for parameter in crate::synth::VoiceEnvelope::surface_descriptor() {
            keys.insert(
                PatchControlId::Envelope(parameter.parameter())
                    .as_str()
                    .into_owned(),
            );
        }
        keys
    }

    /// Every screen string a full production projection of `state` produces
    /// that is supposed to be an **authored label**, tagged with where it came
    /// from.
    ///
    /// Walks all three projections, not just the semantic model: cycle 1's
    /// guard walked `SemanticGraphicalViewModel` alone, so
    /// `PatchPageProjection`'s master-gain row — one of the two production
    /// sites T016 fixed — could be reverted to `descriptor.name()` with the
    /// whole suite still green. A guard that cannot fail on a site is not
    /// guarding it.
    fn projected_labels(state: &AppState) -> Vec<(String, String)> {
        fn push(labels: &mut Vec<(String, String)>, site: impl Into<String>, label: &str) {
            labels.push((site.into(), label.to_owned()));
        }

        fn push_sections(
            labels: &mut Vec<(String, String)>,
            where_: &str,
            sections: &[PatchPageSection],
        ) {
            for section in sections {
                push(
                    labels,
                    format!("{where_} section {}", section.id()),
                    section.label(),
                );
                for row in section.parameters() {
                    let site = format!("{where_} row {}", row.id());
                    push(labels, site.clone(), row.label());
                    if let Some(label) = row.selected_label() {
                        push(labels, format!("{site} selected"), label);
                    }
                    if let Some(label) = row.requested_label() {
                        push(labels, format!("{site} requested"), label);
                    }
                    for choice in row.choices() {
                        push(labels, format!("{site} choice"), choice.label());
                    }
                }
            }
        }

        let (_, page, _, shell, _) = crate::control::StateProjector::new()
            .project_with_shell(state)
            .expect("the fixture state must project");
        let model = shell.semantic_model();
        let labels = &mut Vec::new();

        for surface in model.surfaces() {
            push(
                labels,
                format!("semantic surface {:?}", surface.id()),
                surface.label(),
            );
            for control in surface.controls() {
                push(
                    labels,
                    format!(
                        "semantic {:?} on {:?}",
                        control.path().control_id(),
                        surface.id()
                    ),
                    control.label(),
                );
            }
        }

        // The footer breadcrumb: a composed string, so every `/`-separated
        // segment is checked. F-25 — this is where `"MIXER / GLOBAL /
        // masterGainDb"` and `"PATCH / patch.voiceLimit"` were composed.
        for segment in shell.footer().path_label().split(" / ") {
            push(labels, "shell footer pathLabel segment", segment);
        }

        if let Some(page) = page {
            push(labels, "page engine active", page.engine().active_label());
            for choice in page.engine().choices() {
                push(labels, "page engine choice", choice.label());
            }
            for row in page.envelope() {
                push(labels, format!("page envelope {}", row.id()), row.label());
            }
            for row in page.output() {
                push(labels, format!("page output {}", row.id()), row.label());
            }
            push_sections(labels, "page main", page.sections());
            for slot in page.effects() {
                let where_ = format!("page effect slot {}", slot.slot_index().index());
                if let PatchPageSlotOccupancy::Occupied { label, .. } = slot.occupancy() {
                    push(labels, format!("{where_} occupancy"), label);
                }
                for choice in slot.choices() {
                    push(labels, format!("{where_} choice"), choice.label());
                }
                push_sections(labels, &where_, slot.sections());
            }
            if let Some(detail) = page.detail() {
                push(labels, "page detail", detail.label());
                push_sections(labels, "page detail", detail.sections());
            }
        }
        std::mem::take(labels)
    }

    /// Every fixture the label guard walks, and the surface each one opens.
    ///
    /// Two Patches with different engines, because a descriptor whose `label()`
    /// equalled its `id()` would ship if only one were ever projected; both
    /// detail subjects, because an instrument subject and an effect subject
    /// read different descriptors; and every surface, because the guard is only
    /// as wide as the surfaces it saw.
    fn label_guard_fixtures() -> Vec<(&'static str, AppState)> {
        let braids = |surface: Option<SurfaceId>| {
            let mut state = patch_state();
            state
                .apply_semantic_action(SemanticAction::SelectPatch(Direction::Right))
                .expect("the fixture installs a second Patch");
            let focused = state.interaction().patch_focus().unwrap();
            assert_eq!(
                state
                    .patches()
                    .iter()
                    .find(|patch| patch.id() == focused)
                    .unwrap()
                    .instrument_config()
                    .capability_id()
                    .as_str(),
                BRAIDS_CAPABILITY_ID,
                "the second Patch is the Braids one"
            );
            if let Some(surface) = surface {
                state
                    .apply_semantic_action(SemanticAction::EnterSurface(surface))
                    .expect("the fixture surface is enterable");
            }
            state
        };
        let entered = |surface: SurfaceId| {
            let mut state = patch_state();
            state
                .apply_semantic_action(SemanticAction::EnterSurface(surface))
                .expect("the fixture surface is enterable");
            state
        };
        let mut effect_detail = patch_state();
        navigate_until(&mut effect_detail, |path| {
            matches!(
                path.control_id(),
                SemanticControlId::Patch(PatchControlId::EffectSlot(slot)) if slot.index() == 0
            )
        });
        effect_detail
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchDetail))
            .expect("the occupied slot row resolves an effect subject");
        let mut inspector = mixed_state();
        inspector
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::MixerInspector))
            .unwrap();
        // Both surfaces that host master gain, focused on it. `masterGainDb` is
        // the key F-25 recorded reaching `graphicalShell.footer.pathLabel`, and
        // a breadcrumb is only checkable on the row the cursor is actually on.
        //
        // Side-surface navigation does not wrap, and neither surface's entry
        // focus is above its master-gain row, so this walks both directions.
        fn seek(state: &mut AppState, predicate: impl Fn(&FocusPath) -> bool) {
            for direction in [Direction::Up, Direction::Down] {
                for _ in 0..256 {
                    if predicate(state.interaction().focus_path()) {
                        return;
                    }
                    if state.apply(AppEvent::Navigate(direction)).is_err() {
                        break;
                    }
                }
            }
            panic!("no row on this surface satisfied the predicate");
        }
        let mut utility_global = entered(SurfaceId::PatchUtility);
        seek(&mut utility_global, |path| {
            matches!(
                path.control_id(),
                SemanticControlId::Patch(PatchControlId::Global(_))
            )
        });
        let mut inspector_global = inspector.clone();
        seek(&mut inspector_global, |path| {
            matches!(
                path.control_id(),
                SemanticControlId::Mixer(MixerControlId::Global { .. })
            )
        });
        vec![
            ("soundfont PATCH Main", patch_state()),
            ("MIXER Main", mixed_state()),
            (
                "soundfont instrument detail",
                entered(SurfaceId::PatchDetail),
            ),
            ("chorus effect detail", effect_detail),
            ("MIXER Inspector", inspector),
            ("MIXER Inspector master gain", inspector_global),
            ("PATCH Utility", entered(SurfaceId::PatchUtility)),
            ("PATCH Utility master gain", utility_global),
            ("braids PATCH Main", braids(None)),
            (
                "braids instrument detail",
                braids(Some(SurfaceId::PatchDetail)),
            ),
        ]
    }

    /// T016's guard, over **every** label-producing projection: the semantic
    /// model, the PATCH page, and the shell footer's composed breadcrumb.
    ///
    /// Cycle 1's version walked the semantic model only, so
    /// `patch_page_projection.rs`'s master-gain row could be reverted to
    /// `descriptor.name()` and the entire suite stayed green — one of the two
    /// production sites T016 fixed had no coverage at all. T019's rule applies
    /// to this guard as much as to the channel's: a test that passes with and
    /// without the code it claims to prove is not a proof.
    #[test]
    fn no_projected_label_on_any_surface_is_a_serialization_key() {
        let mut covered = BTreeSet::new();
        let mut checked = 0_usize;
        for (fixture, state) in label_guard_fixtures() {
            let keys = serialization_keys(&state);
            for surface in project(&state).surfaces() {
                covered.insert(format!("{:?}", surface.id()));
            }
            for (site, label) in projected_labels(&state) {
                checked += 1;
                assert!(
                    !keys.contains(&label),
                    "{fixture}: {site} is labelled with the serialization key {label}"
                );
            }
        }
        // The guard is only as wide as the surfaces it saw. All five, or the
        // row that was reported is the only one anybody ever checks.
        assert_eq!(
            covered,
            SurfaceId::ALL
                .into_iter()
                .map(|surface| format!("{surface:?}"))
                .collect::<BTreeSet<_>>(),
            "the label guard must cover every surface, not just the reported row"
        );
        assert!(
            checked > 200,
            "only {checked} labels walked — the guard stopped seeing most of the projection"
        );
    }

    /// The footer breadcrumb is *derived* from the focused row's authored
    /// label, not merely absent from a key set.
    ///
    /// The set check above can only fail on a key it knows. This one fails on
    /// any composition from any other source — a control identity, an ad-hoc
    /// literal like `send[1]`, a descriptor name — because there is exactly one
    /// string it accepts. F-25 recorded that this field composed
    /// `"MIXER / GLOBAL / masterGainDb"`; it is harmless today only because
    /// `page.js` ignores it, and this is what stops it becoming harmful when
    /// WP04 reads it.
    #[test]
    fn the_footer_breadcrumb_is_the_focused_rows_authored_label() {
        for (fixture, state) in label_guard_fixtures() {
            let (_, _, _, shell, _) = crate::control::StateProjector::new()
                .project_with_shell(&state)
                .expect("the fixture state must project");
            let model = shell.semantic_model();
            let expected = format!(
                "{} / {}",
                model.context().label(),
                model.focused_control().map_or_else(
                    || model.active_surface().label().to_owned(),
                    |control| control.label().to_owned()
                )
            );
            assert_eq!(
                shell.footer().path_label(),
                expected,
                "{fixture}: the breadcrumb must name the focused row by its authored label"
            );
        }
    }

    /// The reported defect itself: master gain reaches the screen as its
    /// authored label on both surfaces that host it, and as the serialization
    /// key on neither.
    #[test]
    fn master_gain_projects_its_authored_label_on_both_surfaces_that_host_it() {
        let mut utility = patch_state();
        utility
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchUtility))
            .unwrap();
        let mut inspector = mixed_state();
        inspector
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::MixerInspector))
            .unwrap();

        for model in [project(&utility), project(&inspector)] {
            let row = controls(&model)
                .into_iter()
                .find(|control| {
                    matches!(
                        control.path().control_id(),
                        SemanticControlId::Patch(PatchControlId::Global(_))
                            | SemanticControlId::Mixer(MixerControlId::Global { .. })
                    )
                })
                .expect("master gain is projected on both surfaces");
            assert_eq!(row.label(), "Master Volume");
            assert_ne!(row.label(), "masterGainDb");
        }
    }

    // -----------------------------------------------------------------
    // T017 — one Patch identity, one generation
    // -----------------------------------------------------------------

    #[test]
    fn every_patch_identity_in_one_projection_agrees() {
        let mut state = patch_state();
        for step in 0..2 {
            if step > 0 {
                state
                    .apply(AppEvent::SelectPatch(Direction::Right))
                    .unwrap();
            }
            let model = project(&state);
            let mut identities = BTreeSet::new();
            identities.extend(model.focus_path().patch_id());
            for surface in model.surfaces() {
                identities.extend(surface.summary().patch_id());
                identities.extend(
                    surface
                        .controls()
                        .iter()
                        .filter_map(|control| control.path().patch_id()),
                );
            }
            assert_eq!(identities.len(), 1, "one projection names one Patch");
            assert_eq!(model.patch_identity(), identities.into_iter().next());
        }
    }

    /// NFR-005 from the outside: a patch-selection gesture advances the
    /// projected generation by exactly one, and the projection at that
    /// generation names only the destination Patch. Both halves — reprojecting
    /// twice, or reprojecting once against a half-switched state — are defects,
    /// and neither is visible from the other's evidence.
    #[test]
    fn a_patch_switch_advances_exactly_one_projected_generation() {
        let mut state = patch_state();
        let before = project(&state);
        let source = before.patch_identity().unwrap();

        state
            .apply_semantic_action(SemanticAction::SelectPatch(Direction::Right))
            .unwrap();
        let after = project(&state);

        assert_eq!(after.generation(), before.generation() + 1);
        let destination = after.patch_identity().unwrap();
        assert_ne!(destination, source);
        for surface in after.surfaces() {
            if let Some(patch_id) = surface.summary().patch_id() {
                assert_eq!(patch_id, destination);
            }
            for control in surface.controls() {
                if let Some(patch_id) = control.path().patch_id() {
                    assert_eq!(
                        patch_id, destination,
                        "no projected row may carry the source Patch's identity"
                    );
                }
            }
        }
    }

    /// The diagnostic text projection and the graphical projection cannot come
    /// from different accepted generations: they are derived together and the
    /// shell refuses to hold a pair that disagrees.
    #[test]
    fn the_text_and_graphical_projections_come_from_one_accepted_generation() {
        let mut state = patch_state();
        state
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        let (snapshot, page, text, shell, _parameters) = crate::control::StateProjector::new()
            .project_with_shell(&state)
            .expect("an open detail entry projects the whole set");

        assert_eq!(shell.generation(), state.generation());
        assert_eq!(shell.semantic_model().generation(), state.generation());
        assert_eq!(text.state_hash(), snapshot.hash());
        assert_eq!(shell.state_hash(), snapshot.hash());
        assert_eq!(shell.semantic_model().state_hash(), snapshot.hash());
        assert_eq!(
            shell.patch_identity(),
            page.map(|page| page.patch().id()),
            "the page and the semantic model name one Patch"
        );
    }
}
