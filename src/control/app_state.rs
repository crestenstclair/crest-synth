use crate::control::app_event::{AppEvent, Direction};
use crate::control::engine_selection::{
    EngineSelectionEffect, EngineSelectionEffectKind, EngineSelectionFailure,
    EngineSelectionRequestId, EngineSelectionStatus, EngineSelectionStatusKind,
    StructuralEditIntent,
};
use crate::control::interaction_state::{InteractionState, Selection, SelectionSection};
use crate::control::top_level_context::TopLevelContext;
use crate::control::{
    FocusPath, MixerControlId, ModalControlId, PatchControlId, PatchSubordinateSession,
    SampleBrowserState, SamplePreviewState, SemanticAction, SemanticControlId, SemanticResolver,
    SurfaceId,
};
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::patch_id::PatchId;
use crate::mixer::bus_id::BusId;
use crate::mixer::bus_return::{BusReturnBank, RETURN_LEVEL_DESCRIPTOR};
use crate::mixer::global_parameters::{GlobalParameter, GlobalParameters};
use crate::mixer::mixer_state::MixerState;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::{
    MixerTrackParameter, MixerTrackParameterKind, BUS_SEND_DESCRIPTOR,
};
use crate::mixer::patch_output::PatchOutput;
use crate::mixer::patch_output::PatchOutputParameter;
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::GraphRevision;
use crate::synth::effect_slot_id::EffectSlotIndex;
use crate::synth::instrument_capability::{
    CapabilityError, CapabilityRegistry, ParameterAdjustment, ParameterKind, ParameterValue,
    PatchInteraction,
};
use crate::synth::patch::{Patch, PatchEditableTarget};
use crate::synth::voice_limit::VoiceLimit;
use crate::synth::{
    AssetKind, AssetReference, EffectCapabilityError, EffectCapabilityId, EffectCapabilityRegistry,
    EffectSlotId, ParameterId, PostEffectConfig, SampleAssetId, SampleBrowserRowKind,
    VoiceEnvelopeParameter,
};
use core::fmt;

const MAX_PATCH_COUNT: usize = 16;

/// Validates one Patch's per-position effect chain against the registry.
///
/// Every occupied position must hold a registry-canonical configuration and
/// no stable slot identity may occupy two positions. Empty positions are
/// legal anywhere in the chain; validation walks positions as stored and
/// never builds a compacted intermediate.
pub(crate) fn validate_effect_slots(
    effects: &EffectCapabilityRegistry,
    slots: &[Option<PostEffectConfig>],
) -> Result<(), EffectCapabilityError> {
    for (position, slot) in slots.iter().enumerate() {
        let Some(config) = slot.as_ref() else {
            continue;
        };
        if slots[..position]
            .iter()
            .flatten()
            .any(|prior| prior.slot_id() == config.slot_id())
        {
            return Err(EffectCapabilityError::DuplicateSlot(config.slot_id()));
        }
        effects.validate_config(config)?;
    }
    Ok(())
}

/// The domain event emitted after an AppEvent has been accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateAccepted {
    generation: u64,
}

impl StateAccepted {
    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

/// Effects derived by the reducer from an accepted event.
///
/// The caller commits the already-mutated AppState before publishing any
/// command returned here.
#[derive(Clone, Debug, PartialEq)]
pub struct ApplyOutcome {
    accepted: StateAccepted,
    audio_command: Option<AudioCommand>,
    engine_selection_effect: Option<EngineSelectionEffect>,
}

impl ApplyOutcome {
    pub const fn accepted(&self) -> StateAccepted {
        self.accepted
    }

    pub const fn audio_command(&self) -> Option<&AudioCommand> {
        self.audio_command.as_ref()
    }

    pub const fn engine_selection_effect(&self) -> Option<&EngineSelectionEffect> {
        self.engine_selection_effect.as_ref()
    }

    pub fn into_audio_command(self) -> Option<AudioCommand> {
        self.audio_command
    }
}

#[derive(Default)]
struct ReducerEffects {
    audio_command: Option<AudioCommand>,
    engine_selection_effect: Option<EngineSelectionEffect>,
}

/// One visible deterministic repair caused by an enabled-origin schema change.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusRepairStatus {
    removed_origin: FocusPath,
    replacement_origin: FocusPath,
}

impl FocusRepairStatus {
    pub const fn removed_origin(&self) -> &FocusPath {
        &self.removed_origin
    }

    pub const fn replacement_origin(&self) -> &FocusPath {
        &self.replacement_origin
    }
}

/// Reasons an AppEvent can be rejected without changing AppState.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventRejection {
    InstallationClosed,
    TooManyPatches,
    InvalidInstrumentConfig,
    InvalidEffectConfig,
    NoPatchesInstalled,
    UnknownPatch,
    InvalidSelection,
    ParameterAtBoundary,
    InvalidParameterValue,
    ActionUnavailableInContext,
    EngineSelectionUnavailable,
    StructuralEditBusy,
    StaleEngineSelection,
    MismatchedEngineSelection,
    RequestIdOverflow,
    GenerationOverflow,
}

/// Identifies whether a rejection is reachable in the installed production
/// scene or requires an isolated reducer-table state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventRejectionReachability {
    Scene,
    ReducerTable,
}

/// One production-owned entry in the closed rejection surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventRejectionDescriptor {
    rejection: EventRejection,
    name: &'static str,
    reachability: EventRejectionReachability,
}

impl EventRejectionDescriptor {
    const fn new(
        rejection: EventRejection,
        name: &'static str,
        reachability: EventRejectionReachability,
    ) -> Self {
        Self {
            rejection,
            name,
            reachability,
        }
    }

    pub const fn rejection(self) -> EventRejection {
        self.rejection
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn reachability(self) -> EventRejectionReachability {
        self.reachability
    }
}

const EVENT_REJECTION_SURFACE_DESCRIPTOR: [EventRejectionDescriptor; 16] = [
    EventRejectionDescriptor::new(
        EventRejection::InstallationClosed,
        "installationClosed",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::TooManyPatches,
        "tooManyPatches",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::InvalidInstrumentConfig,
        "invalidInstrumentConfig",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::InvalidEffectConfig,
        "invalidEffectConfig",
        EventRejectionReachability::ReducerTable,
    ),
    EventRejectionDescriptor::new(
        EventRejection::NoPatchesInstalled,
        "noPatchesInstalled",
        EventRejectionReachability::ReducerTable,
    ),
    EventRejectionDescriptor::new(
        EventRejection::UnknownPatch,
        "unknownPatch",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::InvalidSelection,
        "invalidSelection",
        EventRejectionReachability::ReducerTable,
    ),
    EventRejectionDescriptor::new(
        EventRejection::ParameterAtBoundary,
        "parameterAtBoundary",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::InvalidParameterValue,
        "invalidParameterValue",
        EventRejectionReachability::ReducerTable,
    ),
    EventRejectionDescriptor::new(
        EventRejection::ActionUnavailableInContext,
        "actionUnavailableInContext",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::EngineSelectionUnavailable,
        "engineSelectionUnavailable",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::StructuralEditBusy,
        "structuralEditBusy",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::StaleEngineSelection,
        "staleEngineSelection",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::MismatchedEngineSelection,
        "mismatchedEngineSelection",
        EventRejectionReachability::Scene,
    ),
    EventRejectionDescriptor::new(
        EventRejection::RequestIdOverflow,
        "requestIdOverflow",
        EventRejectionReachability::ReducerTable,
    ),
    EventRejectionDescriptor::new(
        EventRejection::GenerationOverflow,
        "generationOverflow",
        EventRejectionReachability::ReducerTable,
    ),
];

impl EventRejection {
    /// Returns every rejection exactly once with its verification reachability.
    pub const fn surface_descriptor() -> &'static [EventRejectionDescriptor] {
        &EVENT_REJECTION_SURFACE_DESCRIPTOR
    }

    /// Returns the stable serialized coverage identifier suffix.
    pub const fn name(self) -> &'static str {
        match self {
            Self::InstallationClosed => "installationClosed",
            Self::TooManyPatches => "tooManyPatches",
            Self::InvalidInstrumentConfig => "invalidInstrumentConfig",
            Self::InvalidEffectConfig => "invalidEffectConfig",
            Self::NoPatchesInstalled => "noPatchesInstalled",
            Self::UnknownPatch => "unknownPatch",
            Self::InvalidSelection => "invalidSelection",
            Self::ParameterAtBoundary => "parameterAtBoundary",
            Self::InvalidParameterValue => "invalidParameterValue",
            Self::ActionUnavailableInContext => "actionUnavailableInContext",
            Self::EngineSelectionUnavailable => "engineSelectionUnavailable",
            Self::StructuralEditBusy => "structuralEditBusy",
            Self::StaleEngineSelection => "staleEngineSelection",
            Self::MismatchedEngineSelection => "mismatchedEngineSelection",
            Self::RequestIdOverflow => "requestIdOverflow",
            Self::GenerationOverflow => "generationOverflow",
        }
    }
}

impl fmt::Display for EventRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InstallationClosed => "patch installation is permitted only at startup",
            Self::TooManyPatches => "no more than 16 Patches may be installed",
            Self::InvalidInstrumentConfig => {
                "an installed Patch instrument config does not match the capability registry"
            }
            Self::InvalidEffectConfig => {
                "an installed Patch effect config does not match the effect capability registry"
            }
            Self::NoPatchesInstalled => "no Patch is available for the selected operation",
            Self::UnknownPatch => "the MIDI event targets a Patch that is not installed",
            Self::InvalidSelection => "the current selection is outside the installed state",
            Self::ParameterAtBoundary => "the selected parameter is already at that boundary",
            Self::InvalidParameterValue => "the adjusted parameter value is invalid",
            Self::ActionUnavailableInContext => {
                "the semantic action is unavailable in the active context"
            }
            Self::EngineSelectionUnavailable => {
                "the focused Patch has no adjacent installed engine choice"
            }
            Self::StructuralEditBusy => "another structural engine selection is already in flight",
            Self::StaleEngineSelection => "the engine-selection event names no current request",
            Self::MismatchedEngineSelection => {
                "the engine-selection event does not match the current request"
            }
            Self::RequestIdOverflow => {
                "the engine-selection request identity cannot be incremented"
            }
            Self::GenerationOverflow => "the accepted-state generation cannot be incremented",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for EventRejection {}

/// Exercises rejection variants that cannot occur after the fixed scene has
/// installed its valid Patch set. The exhaustive verifier unions these measured
/// reducer-table outcomes with the scene's public rejection records.
pub(crate) fn exercise_reducer_table_rejections(
    capabilities: &CapabilityRegistry,
    instrument_config: &crate::synth::instrument_capability::InstrumentConfig,
) -> [EventRejection; 8] {
    fn probe_patch(
        id: u32,
        channel: u8,
        instrument_config: &crate::synth::instrument_capability::InstrumentConfig,
    ) -> Patch {
        Patch::new(
            crate::kernel::patch_id::PatchId::new(id).expect("probe PatchId is valid"),
            format!("Reducer probe {id}"),
            instrument_config.clone(),
            crate::kernel::midi_channel::MidiChannel::new(channel).expect("probe channel is valid"),
            PatchOutput::to_track(MixerTrackId::new(channel).expect("probe route is valid")),
        )
    }

    let global = GlobalParameters::new(0.0).expect("reducer probe globals are valid");

    let mut oversized = AppState::new(capabilities.clone(), global);
    let too_many = oversized
        .apply(AppEvent::InstallPatches(
            (1..=17)
                .map(|id| probe_patch(id, ((id - 1) % 16) as u8, instrument_config))
                .collect(),
        ))
        .expect_err("seventeen Patches exceed the reducer bound");

    let invalid_config = crate::synth::instrument_capability::InstrumentConfig::from_parts(
        crate::synth::capability_id::CapabilityId::new("instrument.unknown")
            .expect("probe capability id is valid"),
        Vec::new(),
        Vec::new(),
    );
    let mut invalid = AppState::new(capabilities.clone(), global);
    let invalid_instrument = invalid
        .apply(AppEvent::InstallPatches(vec![probe_patch(
            1,
            0,
            &invalid_config,
        )]))
        .expect_err("unknown config violates registry installation");

    let invalid_effect_config = crate::synth::PostEffectConfig::from_parts(
        crate::synth::EffectSlotId::new(1).expect("probe effect slot is valid"),
        crate::synth::EffectCapabilityId::new("effect.unknown")
            .expect("probe effect capability id is valid"),
        Vec::new(),
        Vec::new(),
    );
    let mut invalid_effect_probe = probe_patch(1, 0, instrument_config);
    invalid_effect_probe
        .set_slot_occupancy(
            EffectSlotIndex::new(0).expect("probe slot position is valid"),
            Some(invalid_effect_config),
        )
        .expect("probe occupancy uses a unique slot identity");
    let mut invalid_effect = AppState::new(capabilities.clone(), global);
    let invalid_effect = invalid_effect
        .apply(AppEvent::InstallPatches(vec![invalid_effect_probe]))
        .expect_err("unknown effect config violates registry installation");

    let mut no_patches = AppState::new(capabilities.clone(), global);
    let no_patch = no_patches
        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
        .expect_err("PATCH has no focus before installation");

    let mut invalid_selection = AppState {
        capabilities: std::sync::Arc::new(capabilities.clone()),
        effects: std::sync::Arc::new(EffectCapabilityRegistry::default()),
        patches: vec![probe_patch(1, 0, instrument_config)],
        mixer: MixerState::default(),
        global,
        returns: BusReturnBank::default(),
        interaction: {
            let mut interaction = InteractionState::new();
            interaction
                .set_active_main(FocusPath::patch_main(
                    crate::kernel::patch_id::PatchId::new(1).unwrap(),
                    None,
                    crate::control::PatchControlId::Capability(
                        ParameterId::new("missing").unwrap(),
                    ),
                ))
                .expect("probe path shape is valid");
            interaction
        },
        disabled_patch_overview_origins: std::collections::BTreeSet::new(),
        focus_repair_status: None,
        sample_browser: SampleBrowserState::default(),
        pending_instrument_config: None,
        sample_visualizations: std::collections::BTreeMap::new(),
        pending_sample_visualization: None,
        engine_selection: EngineSelectionStatus::ready(GraphRevision::INITIAL),
        last_engine_selection_request_id: EngineSelectionRequestId::NONE,
        generation: 0,
    };
    let invalid_selection = invalid_selection
        .apply(AppEvent::Adjust(Direction::Right))
        .expect_err("an out-of-range parameter index is rejected");

    let invalid_parameter = PatchOutput::default()
        .with_trim_gain_db(f32::NAN)
        .map_err(|_| EventRejection::InvalidParameterValue)
        .expect_err("a non-finite typed value is rejected");

    let mut overflow = AppState::new(capabilities.clone(), global);
    overflow.generation = u64::MAX;
    let generation_overflow = overflow
        .apply(AppEvent::Navigate(Direction::Down))
        .expect_err("the accepted generation cannot overflow");

    let mut request_overflow = AppState::new(capabilities.clone(), global);
    request_overflow
        .apply(AppEvent::InstallPatches(vec![probe_patch(
            1,
            0,
            instrument_config,
        )]))
        .expect("request overflow probe installs one Patch");
    request_overflow
        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
        .expect("request overflow probe enters PATCH");
    request_overflow.last_engine_selection_request_id =
        EngineSelectionRequestId::new(u64::MAX).expect("maximum request id is nonzero");
    let request_id_overflow = request_overflow
        .apply(AppEvent::Adjust(Direction::Right))
        .expect_err("the request identity cannot overflow");

    [
        too_many,
        invalid_instrument,
        invalid_effect,
        no_patch,
        invalid_selection,
        invalid_parameter,
        request_id_overflow,
        generation_overflow,
    ]
}

/// The single source of mutable control state.
///
/// State-changing transitions are transactional: apply reduces into a clone and
/// replaces self only after the complete event has been accepted. MIDI validates
/// its target read-only, then commits only the next generation and one command.
#[derive(Clone, Debug, PartialEq)]
pub struct AppState {
    /// The two installed registries are shared rather than copied.
    ///
    /// Neither is ever reassigned after construction — nothing in the reducer
    /// can change what is installed — so sharing them cannot alias a mutation,
    /// and `Arc<T>: PartialEq` still compares contents, leaving state equality
    /// exactly as it was.
    ///
    /// Sharing them is what makes availability affordable to ask per row.
    /// [`Self::accepts_semantic_action`] answers by cloning this state and
    /// running the real reducer over the clone, which is the whole reason the
    /// answer cannot drift; the projection now asks that question once for the
    /// focus and once more for every projected control's counterfactual focus.
    /// Measured on the production registries, copying the capability registry
    /// was 41µs of a 48µs `AppState` clone — 86% of the cost of a question that
    /// never reads a descriptor it did not already have. Sharing takes the
    /// clone to 1.5µs and one full action-availability sweep from 1.46ms to
    /// 139µs, which is the difference between a per-row list and no per-row
    /// list at all.
    capabilities: std::sync::Arc<CapabilityRegistry>,
    effects: std::sync::Arc<EffectCapabilityRegistry>,
    patches: Vec<Patch>,
    mixer: MixerState,
    global: GlobalParameters,
    returns: BusReturnBank,
    interaction: InteractionState,
    disabled_patch_overview_origins: std::collections::BTreeSet<(PatchId, PatchControlId)>,
    focus_repair_status: Option<FocusRepairStatus>,
    engine_selection: EngineSelectionStatus,
    sample_browser: SampleBrowserState,
    /// Complete prepared instrument candidate retained off callback until the
    /// corresponding graph activation is acknowledged. This is transient
    /// control-thread state and is never serialized as acknowledged product state.
    pending_instrument_config: Option<crate::synth::InstrumentConfig>,
    sample_visualizations:
        std::collections::BTreeMap<PatchId, crate::synth::PreparedSampleVisualization>,
    pending_sample_visualization: Option<(
        EngineSelectionRequestId,
        crate::synth::PreparedSampleVisualization,
    )>,
    last_engine_selection_request_id: EngineSelectionRequestId,
    generation: u64,
}

/// One reusable scratch state for asking the availability question many times
/// about the *same* accepted state.
///
/// The question itself is unchanged and is still answered by running the
/// production reducer — that is the property the per-row action list exists to
/// preserve, and nothing here weakens it. What changes is how many `AppState`
/// clones the answer costs.
///
/// [`AppState::apply`] is transactional in the direction that matters here: a
/// rejected event returns before any field is written, so a refused probe
/// leaves the scratch byte-for-byte the state it was given and the next probe
/// may reuse it. An *accepted* probe does mutate the scratch, so the scratch is
/// refreshed from the accepted state before the next question is asked.
///
/// The cost per vocabulary sweep goes from `2·|vocabulary|` clones — one to
/// obtain a `&mut`, one inside `apply` — to `1 + |vocabulary| + |accepted|`.
/// On a MIXER row most of the vocabulary is refused, which is where the
/// difference is largest.
pub struct SemanticActionAvailability<'state> {
    accepted: &'state AppState,
    scratch: AppState,
}

impl<'state> SemanticActionAvailability<'state> {
    /// Prepares one scratch clone of `accepted`.
    #[must_use]
    pub fn new(accepted: &'state AppState) -> Self {
        Self {
            accepted,
            scratch: accepted.clone(),
        }
    }

    /// Answers whether the accepted state would accept `action`, reusing the
    /// scratch across refusals and refreshing it after an acceptance.
    pub fn accepts(&mut self, action: &SemanticAction) -> bool {
        let accepted = self.scratch.apply_semantic_action(action.clone()).is_ok();
        if accepted {
            self.scratch = self.accepted.clone();
        }
        accepted
    }
}

impl AppState {
    pub(crate) fn restore_voice_limits(
        &mut self,
        limits: &[(PatchId, u16)],
    ) -> Result<(), EventRejection> {
        if limits.len() != self.patches.len()
            || limits.iter().any(|(patch_id, value)| {
                self.patches
                    .iter()
                    .find(|patch| patch.id() == *patch_id)
                    .and_then(|patch| {
                        self.capabilities
                            .descriptor(patch.instrument_config().capability_id())
                    })
                    .is_none_or(|descriptor| {
                        *value == 0
                            || *value > VoiceLimit::seeded_from(descriptor.voice_policy()).value()
                    })
            })
        {
            return Err(EventRejection::InvalidInstrumentConfig);
        }
        for (patch_id, value) in limits {
            self.patches
                .iter_mut()
                .find(|patch| patch.id() == *patch_id)
                .ok_or(EventRejection::UnknownPatch)?
                .set_voice_limit(*value)
                .map_err(|_| EventRejection::InvalidParameterValue)?;
        }
        Ok(())
    }

    /// Creates startup state before the fixture Patch set is installed.
    pub fn new(capabilities: CapabilityRegistry, global: GlobalParameters) -> Self {
        Self::for_graph(capabilities, global, GraphRevision::INITIAL)
    }

    /// Creates startup state with the immutable installed effect registry.
    pub fn new_with_effects(
        capabilities: CapabilityRegistry,
        effects: EffectCapabilityRegistry,
        global: GlobalParameters,
    ) -> Self {
        Self::for_graph_with_effects(capabilities, effects, global, GraphRevision::INITIAL)
    }

    /// Creates startup state for the exact complete graph revision already prepared.
    pub fn for_graph(
        capabilities: CapabilityRegistry,
        global: GlobalParameters,
        active_graph_revision: GraphRevision,
    ) -> Self {
        Self::for_graph_with_effects(
            capabilities,
            EffectCapabilityRegistry::default(),
            global,
            active_graph_revision,
        )
    }

    /// Creates startup state for one graph revision and installed effect registry.
    pub fn for_graph_with_effects(
        capabilities: CapabilityRegistry,
        effects: EffectCapabilityRegistry,
        global: GlobalParameters,
        active_graph_revision: GraphRevision,
    ) -> Self {
        Self {
            capabilities: std::sync::Arc::new(capabilities),
            effects: std::sync::Arc::new(effects),
            patches: Vec::new(),
            mixer: MixerState::default(),
            global,
            returns: BusReturnBank::default(),
            interaction: InteractionState::new(),
            disabled_patch_overview_origins: std::collections::BTreeSet::new(),
            focus_repair_status: None,
            engine_selection: EngineSelectionStatus::ready(active_graph_revision),
            sample_browser: SampleBrowserState::default(),
            pending_instrument_config: None,
            sample_visualizations: std::collections::BTreeMap::new(),
            pending_sample_visualization: None,
            last_engine_selection_request_id: EngineSelectionRequestId::NONE,
            generation: 0,
        }
    }

    /// Supplies the complete fixed mixer bank when constructing restored or
    /// deterministic state. The bank's type guarantees exactly sixteen tracks.
    pub fn with_initial_mixer(mut self, mixer: MixerState) -> Self {
        self.mixer = mixer;
        self
    }

    /// Supplies the canonical startup bus-return bank chosen by the
    /// composition root. Occupancy is composition, not identity: the reducer
    /// itself installs nothing by default.
    pub fn with_initial_returns(mut self, returns: BusReturnBank) -> Self {
        self.returns = returns;
        self
    }

    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }

    pub fn capabilities(&self) -> &CapabilityRegistry {
        &self.capabilities
    }

    pub fn effects(&self) -> &EffectCapabilityRegistry {
        &self.effects
    }

    pub const fn global(&self) -> &GlobalParameters {
        &self.global
    }

    pub const fn mixer(&self) -> &MixerState {
        &self.mixer
    }

    pub fn sample_visualization(
        &self,
        patch_id: PatchId,
    ) -> Option<&crate::synth::PreparedSampleVisualization> {
        self.pending_sample_visualization
            .as_ref()
            .filter(|(request_id, _)| {
                self.sample_browser.preview_request_id() == Some(*request_id)
                    || self.sample_browser.request_id() == Some(*request_id)
            })
            .map(|(_, visualization)| visualization)
            .or_else(|| self.sample_visualizations.get(&patch_id))
    }

    /// Returns the canonical eight-return bank in ascending `BusId` order.
    pub const fn bus_returns(&self) -> &BusReturnBank {
        &self.returns
    }

    /// Supplies the transient controller-native Sample catalog discovered by
    /// the composition root. Listings are control state only and are excluded
    /// from saved Patch/session data.
    pub fn with_sample_catalog(
        mut self,
        listings: impl IntoIterator<
            Item = (
                crate::synth::SampleFolderId,
                Result<crate::synth::SampleCatalogListing, crate::synth::SampleAssetError>,
            ),
        >,
    ) -> Self {
        self.sample_browser = self.sample_browser.with_catalog(listings);
        self
    }

    pub const fn sample_browser(&self) -> &SampleBrowserState {
        &self.sample_browser
    }

    /// Resolves one MIXER global row's current value: master gain alone.
    pub fn global_row_value(&self, parameter: GlobalParameter) -> f32 {
        match parameter {
            GlobalParameter::MasterGainDb => self.global.master_gain_db(),
        }
    }

    pub fn selection(&self) -> Selection {
        let path = self.interaction.remembered_mixer_main();
        let (section, patch_index) = match path.control_id() {
            SemanticControlId::Mixer(MixerControlId::Track { track_id, .. }) => {
                (SelectionSection::Patch, track_id.index())
            }
            SemanticControlId::Mixer(MixerControlId::Global { .. }) => {
                (SelectionSection::Global, 0)
            }
            _ => (SelectionSection::Global, usize::MAX),
        };
        let parameter_index = SemanticResolver::new(self)
            .mixer_coordinates(path)
            .map(|(_, parameter)| parameter)
            .unwrap_or(usize::MAX);
        Selection {
            section,
            patch_index,
            parameter_index,
        }
    }

    /// Returns the complete reducer-owned interaction state.
    pub const fn interaction(&self) -> &InteractionState {
        &self.interaction
    }

    pub fn patch_overview_origin_enabled(
        &self,
        patch_id: PatchId,
        control: &PatchControlId,
    ) -> bool {
        !self
            .disabled_patch_overview_origins
            .contains(&(patch_id, control.clone()))
    }

    pub const fn focus_repair_status(&self) -> Option<&FocusRepairStatus> {
        self.focus_repair_status.as_ref()
    }

    /// Returns the selected top-level context as a thin compatibility view.
    pub const fn context(&self) -> TopLevelContext {
        self.interaction.context()
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn engine_selection(&self) -> &EngineSelectionStatus {
        &self.engine_selection
    }

    pub(crate) const fn pending_instrument_config(
        &self,
    ) -> Option<&crate::synth::InstrumentConfig> {
        self.pending_instrument_config.as_ref()
    }

    /// Tests one normalized user action against a clone of the exact accepted
    /// reducer state. Availability is therefore pure and cannot drift from
    /// reducer bounds, dependencies, lifecycle, or surface rules.
    ///
    /// This is the one-shot form. Asking the same question for a whole
    /// vocabulary — which is what a per-row action list does — goes through
    /// [`SemanticActionAvailability`], which reuses one scratch state instead
    /// of taking a fresh clone per question. Both forms run the same reducer
    /// over the same scratch, because this one *is* the other one.
    pub fn accepts_semantic_action(&self, action: &SemanticAction) -> bool {
        SemanticActionAvailability::new(self).accepts(action)
    }

    /// Returns this exact accepted state with the focus moved to `path`, or
    /// `None` when no coherent state has that path focused.
    ///
    /// This is the counterfactual the per-row action list is resolved against:
    /// *if this control were the focused one, what would the reducer accept?*
    /// Nothing else changes — the same Patches, the same mixer, the same
    /// lifecycle, the same interaction mode — so the difference between one
    /// row's list and another's is the focus and nothing else.
    ///
    /// The move reaches each surface the way a player does, so every
    /// counterfactual is a state the reducer could really be in rather than an
    /// assembled one: a main path lands through `set_active_main`, a
    /// persistent-side path is *entered* from the remembered main origin, and a
    /// detail path is only reachable while the entry it belongs to is already
    /// open. Entering a surface resets the mode, so the accepted mode is
    /// restored afterwards: the counterfactual differs in focus, and a mode
    /// change would silently answer a different question.
    ///
    /// Once the right surface is active, the row within it is assigned to
    /// `active_focus` directly, exactly as the reducer's own two side-surface
    /// movers do (see [`InteractionState`]'s note): the branch has already
    /// established that the surface is the one the path names, so the write
    /// moves within one already-open surface's order and cannot change which
    /// surface is active.
    ///
    /// The final resolver check is what makes the result trustworthy — a path
    /// the installed schema does not host yields `None` rather than a state
    /// focused on a control that is not there.
    pub(crate) fn with_counterfactual_focus(&self, path: &FocusPath) -> Option<Self> {
        if self.interaction.focus_path() == path {
            return Some(self.clone());
        }
        if path.context() != self.context() || path.validate().is_err() {
            return None;
        }
        let mode = self.interaction.mode();
        let mut candidate = self.clone();
        let surface = path.surface();
        if surface.is_main() {
            candidate.interaction.set_active_main(path.clone()).ok()?;
        } else if surface.is_persistent_side() {
            let origin = match path.context() {
                TopLevelContext::Patch => self.interaction.remembered_patch_main().cloned()?,
                TopLevelContext::Mixer => self.interaction.remembered_mixer_main().clone(),
            };
            candidate.interaction.set_active_main(origin).ok()?;
            candidate.interaction.enter_surface(surface).ok()?;
            candidate.interaction.active_focus = path.clone();
        } else {
            // The subordinate detail surface carries a subject, and only
            // `enter_detail` may set one. A detail row is projected exactly
            // while that entry is open, so the counterfactual moves within the
            // already-open surface's own order and never opens one.
            if self.interaction.active_surface() != surface {
                return None;
            }
            candidate.interaction.active_focus = path.clone();
        }
        candidate.interaction.set_mode(mode).ok()?;
        debug_assert!(
            candidate.interaction.detail_invariant_holds(),
            "a counterfactual focus must leave the detail facts agreeing"
        );
        SemanticResolver::new(&candidate)
            .resolves(path)
            .then_some(candidate)
    }

    /// Applies a normalized user intent through the same transactional reducer
    /// while enforcing the currently projected interaction mode.
    pub fn apply_semantic_action(
        &mut self,
        action: SemanticAction,
    ) -> Result<ApplyOutcome, EventRejection> {
        if !action.is_phase_two_admitted()
            || matches!(
                action,
                SemanticAction::Navigate(_)
                    if !matches!(
                        self.interaction.mode(),
                        crate::control::InteractionMode::Navigate
                            | crate::control::InteractionMode::Modal
                    )
            )
            || matches!(
                action,
                SemanticAction::Adjust(_)
                    if self.interaction.mode() != crate::control::InteractionMode::Adjust
            )
        {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        self.apply(AppEvent::from_semantic_action(action))
    }

    /// Resolves one installed Patch's complete editable surface from its active schema.
    pub fn patch_editable_targets(
        &self,
        patch_index: usize,
    ) -> Result<Vec<PatchEditableTarget>, EventRejection> {
        let patch = self
            .patches
            .get(patch_index)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let descriptor = self
            .capabilities
            .descriptor(patch.instrument_config().capability_id())
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        patch
            .editable_targets(descriptor)
            .map_err(|_| EventRejection::InvalidInstrumentConfig)
    }

    /// Resolves the active PATCH surface through the canonical descriptor-owned
    /// control resolver.
    pub fn focused_patch_controls(
        &self,
    ) -> Result<Vec<crate::control::PatchControlId>, EventRejection> {
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        SemanticResolver::new(self)
            .patch_main_paths(patch_id)
            .map(|paths| {
                paths
                    .into_iter()
                    .filter_map(|path| match path.control_id() {
                        SemanticControlId::Patch(control) => Some(control.clone()),
                        _ => None,
                    })
                    .collect()
            })
    }

    /// Applies the only permitted control-state mutation.
    ///
    /// Rejected events leave every field byte-for-byte logically identical.
    /// Accepted events increment generation exactly once.
    pub fn apply(&mut self, event: AppEvent) -> Result<ApplyOutcome, EventRejection> {
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(EventRejection::GenerationOverflow)?;

        if let AppEvent::Midi { patch_id, message } = event {
            if !self.patches.iter().any(|patch| patch.id() == patch_id) {
                return Err(EventRejection::UnknownPatch);
            }
            self.generation = generation;
            return Ok(ApplyOutcome {
                accepted: StateAccepted { generation },
                audio_command: Some(AudioCommand::PatchMidi { patch_id, message }),
                engine_selection_effect: None,
            });
        }

        let mut next = self.clone();
        let effects = next.reduce(event)?;
        next.generation = generation;

        *self = next;
        Ok(ApplyOutcome {
            accepted: StateAccepted { generation },
            audio_command: effects.audio_command,
            engine_selection_effect: effects.engine_selection_effect,
        })
    }

    fn reduce(&mut self, event: AppEvent) -> Result<ReducerEffects, EventRejection> {
        if matches!(
            &event,
            AppEvent::SelectContext(_)
                | AppEvent::SelectPatch(_)
                | AppEvent::Navigate(_)
                | AppEvent::Adjust(_)
                | AppEvent::SetInteractionMode(_)
                | AppEvent::OpenRelated
                | AppEvent::Activate
                | AppEvent::PreviewStart
                | AppEvent::PreviewStop
                | AppEvent::EnterSurface(_)
                | AppEvent::Return
                | AppEvent::SetSlotOccupancy { .. }
                | AppEvent::SetReturnOccupancy { .. }
        ) {
            self.focus_repair_status = None;
        }
        match event {
            AppEvent::SelectContext(context) => {
                self.select_context(context)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::SelectPatch(direction) => {
                self.select_patch(direction)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::Navigate(direction) => {
                let audio_command = if self.interaction.active_surface() == SurfaceId::SampleBrowser
                {
                    self.preview_stop_command()
                } else {
                    None
                };
                match self.context() {
                    TopLevelContext::Mixer => self.navigate(direction)?,
                    TopLevelContext::Patch => self.navigate_patch_control(direction)?,
                }
                Ok(ReducerEffects {
                    audio_command,
                    engine_selection_effect: None,
                })
            }
            AppEvent::Adjust(direction) => match self.context() {
                TopLevelContext::Mixer => self.adjust(direction),
                TopLevelContext::Patch => self.adjust_patch_control(direction),
            },
            AppEvent::SetInteractionMode(mode) => {
                self.interaction
                    .set_mode(mode)
                    .map_err(|_| EventRejection::ActionUnavailableInContext)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::OpenRelated => {
                self.open_related_surface()?;
                Ok(ReducerEffects::default())
            }
            AppEvent::Activate => self.activate_focused_subordinate(),
            AppEvent::PreviewStart => {
                let engine_selection_effect = self.preview_start()?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(engine_selection_effect),
                })
            }
            AppEvent::PreviewStop => {
                let audio_command = self.preview_stop()?;
                Ok(ReducerEffects {
                    audio_command,
                    engine_selection_effect: None,
                })
            }
            AppEvent::EnterSurface(SurfaceId::PatchDetail) => {
                self.enter_patch_detail()?;
                Ok(ReducerEffects::default())
            }
            AppEvent::EnterSurface(surface) => {
                self.interaction
                    .enter_surface(surface)
                    .map_err(|_| EventRejection::ActionUnavailableInContext)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::Return => {
                let audio_command = if self.interaction.active_surface() == SurfaceId::SampleBrowser
                {
                    self.preview_stop_command()
                } else {
                    None
                };
                if self.interaction.active_surface() == SurfaceId::SampleBrowser {
                    self.sample_browser.cancelled();
                }
                self.interaction
                    .return_to_origin()
                    .map_err(|_| EventRejection::ActionUnavailableInContext)?;
                Ok(ReducerEffects {
                    audio_command,
                    engine_selection_effect: None,
                })
            }
            AppEvent::InstallPatches(patches) => {
                self.install_patches(patches)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::Midi { .. } => unreachable!("MIDI is reduced by apply's read-only fast path"),
            AppEvent::SetPatchOverviewOriginEnabled {
                patch_id,
                control,
                enabled,
            } => {
                self.set_patch_overview_origin_enabled(patch_id, control, enabled)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::EngineSelectionLifecycleAdvanced {
                request_id,
                lifecycle,
            } => {
                self.engine_selection_lifecycle_advanced(request_id, lifecycle)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::EnginePrepared {
                request_id,
                patch_id,
                intent,
                source_capability_id,
                target_capability_id,
                source_graph_revision,
                target_graph_revision,
                candidate_config,
                prepared_visualization,
            } => {
                let engine_selection_effect = self.engine_prepared(
                    request_id,
                    patch_id,
                    intent,
                    source_capability_id,
                    target_capability_id,
                    source_graph_revision,
                    target_graph_revision,
                    candidate_config,
                    prepared_visualization,
                )?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(engine_selection_effect),
                })
            }
            AppEvent::SampleAssetLifecycleAdvanced {
                request_id,
                lifecycle,
            } => {
                if !self
                    .sample_browser
                    .assignment_lifecycle_advanced(request_id, lifecycle)
                {
                    return Err(EventRejection::MismatchedEngineSelection);
                }
                Ok(ReducerEffects::default())
            }
            AppEvent::SampleCatalogRefreshed { folder, listing } => {
                let refreshing_visible = self.interaction.active_surface()
                    == SurfaceId::SampleBrowser
                    && self.sample_browser.folder() == &folder;
                let (held, old_order, audio_command) = if refreshing_visible {
                    (
                        Some(self.interaction.focus_path().clone()),
                        Some(SemanticResolver::new(self).sample_browser_paths()?),
                        self.preview_stop_command(),
                    )
                } else {
                    (None, None, None)
                };
                let reloaded = self.sample_browser.refresh_listing(folder, listing);
                if reloaded {
                    let held = held.ok_or(EventRejection::InvalidSelection)?;
                    let old_order = old_order.ok_or(EventRejection::InvalidSelection)?;
                    let new_order = SemanticResolver::new(self).sample_browser_paths()?;
                    let repaired = if new_order.contains(&held) {
                        held
                    } else {
                        SemanticResolver::recover(&held, &old_order, &new_order)
                            .ok_or(EventRejection::InvalidSelection)?
                    };
                    self.interaction.active_focus = repaired;
                }
                Ok(ReducerEffects {
                    audio_command,
                    engine_selection_effect: None,
                })
            }
            AppEvent::EnginePreparationFailed {
                request_id,
                patch_id,
                intent,
                source_capability_id,
                target_capability_id,
                source_graph_revision,
                target_graph_revision,
                failure,
            } => {
                self.engine_preparation_failed(
                    request_id,
                    patch_id,
                    &intent,
                    &source_capability_id,
                    &target_capability_id,
                    source_graph_revision,
                    target_graph_revision,
                    failure,
                )?;
                Ok(ReducerEffects::default())
            }
            AppEvent::EngineActivationAcknowledged {
                request_id,
                intent,
                target_graph_revision,
                retired_graph_revision,
                collected,
            } => {
                let (engine_selection_effect, audio_command) = self
                    .engine_activation_acknowledged(
                        request_id,
                        &intent,
                        target_graph_revision,
                        retired_graph_revision,
                        collected,
                    )?;
                Ok(ReducerEffects {
                    audio_command,
                    engine_selection_effect: Some(engine_selection_effect),
                })
            }
            AppEvent::SetSlotOccupancy {
                patch_id,
                slot,
                entry,
            } => {
                let effect =
                    self.request_topology_change(StructuralEditIntent::SetSlotOccupancy {
                        patch_id,
                        slot,
                        entry,
                    })?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(effect),
                })
            }
            AppEvent::SetReturnOccupancy { bus, entry } => {
                let effect =
                    self.request_topology_change(StructuralEditIntent::SetReturnOccupancy {
                        bus,
                        entry,
                    })?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(effect),
                })
            }
            AppEvent::TopologyPrepared {
                request_id,
                intent,
                source_graph_revision,
                target_graph_revision,
            } => {
                let effect = self.topology_prepared(
                    request_id,
                    intent,
                    source_graph_revision,
                    target_graph_revision,
                )?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(effect),
                })
            }
            AppEvent::TopologyPreparationFailed {
                request_id,
                intent,
                source_graph_revision,
                target_graph_revision,
                failure,
            } => {
                self.topology_preparation_failed(
                    request_id,
                    &intent,
                    source_graph_revision,
                    target_graph_revision,
                    failure,
                )?;
                Ok(ReducerEffects::default())
            }
        }
    }

    fn install_patches(&mut self, mut patches: Vec<Patch>) -> Result<(), EventRejection> {
        if self.generation != 0 || !self.patches.is_empty() {
            return Err(EventRejection::InstallationClosed);
        }
        if patches.len() > MAX_PATCH_COUNT {
            return Err(EventRejection::TooManyPatches);
        }
        if patches.iter().any(|patch| {
            self.capabilities
                .validate_config(patch.instrument_config())
                .is_err()
        }) {
            return Err(EventRejection::InvalidInstrumentConfig);
        }
        if patches
            .iter()
            .any(|patch| validate_effect_slots(&self.effects, patch.effect_slots()).is_err())
        {
            return Err(EventRejection::InvalidEffectConfig);
        }
        // This is the only installation site with registry access, so it is
        // the only place a Patch can learn what its own engine can honour.
        // Without this, every Patch would carry `Patch::new`'s unconditional
        // engine-managed ceiling — including a Braids Patch whose capability
        // declares far fewer voices.
        for patch in &mut patches {
            let policy = self
                .capabilities
                .descriptor(patch.instrument_config().capability_id())
                .ok_or(EventRejection::InvalidInstrumentConfig)?
                .voice_policy();
            patch.seed_voice_limit(policy);
        }

        self.patches = patches;
        let resolver = SemanticResolver::new(self);
        let mixer_focus = resolver
            .mixer_main_paths()?
            .into_iter()
            .next()
            .expect("the global MIXER section is never empty");
        let patch_focus = match self.patches.first() {
            Some(patch) => resolver.patch_main_paths(patch.id())?.into_iter().next(),
            None => None,
        };
        self.interaction.initialize_patch_focus(patch_focus);
        self.interaction
            .set_active_main(mixer_focus)
            .map_err(|_| EventRejection::InvalidSelection)?;
        Ok(())
    }

    /// Opens the subordinate PATCH detail surface on the focused row's subject.
    ///
    /// Entry is accepted only from a PatchMain path whose control resolves a
    /// [`PatchDetailSubject`] — the engine row, an active-instrument
    /// capability row, an occupied effect slot, or one of its occupant's
    /// parameter rows. Every other origin is a typed unchanged rejection
    /// rather than an empty detail surface: an empty slot names no capability,
    /// and a Utility row is not on the main surface at all.
    fn enter_patch_detail(&mut self) -> Result<(), EventRejection> {
        if self.interaction.active_surface() != SurfaceId::PatchMain {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        let origin = self.interaction.focus_path().clone();
        let resolver = SemanticResolver::new(self);
        let subject = resolver
            .detail_subject(&origin)
            .ok_or(EventRejection::ActionUnavailableInContext)?;
        let patch_id = origin
            .patch_id()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        // The subject's first visible enabled control, resolved from the
        // installed descriptor rather than assumed.
        let focus = resolver
            .patch_detail_paths(patch_id, &subject)?
            .into_iter()
            .next()
            .ok_or(EventRejection::InvalidSelection)?;
        self.interaction
            .enter_detail(subject, focus)
            .map_err(|_| EventRejection::ActionUnavailableInContext)
    }

    fn open_related_surface(&mut self) -> Result<(), EventRejection> {
        match self.interaction.active_surface() {
            SurfaceId::PatchMain => self.enter_patch_detail(),
            SurfaceId::PatchDetail => self.open_sample_browser(),
            SurfaceId::PatchUtility
            | SurfaceId::PatchChoice
            | SurfaceId::SampleBrowser
            | SurfaceId::MixerMain
            | SurfaceId::MixerInspector => Err(EventRejection::ActionUnavailableInContext),
        }
    }

    fn open_sample_browser(&mut self) -> Result<(), EventRejection> {
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let parameter_id = match self.interaction.focus_path().control_id() {
            SemanticControlId::Patch(crate::control::PatchControlId::Capability(id)) => id.clone(),
            _ => return Err(EventRejection::ActionUnavailableInContext),
        };
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let descriptor = self
            .capabilities
            .descriptor(patch.instrument_config().capability_id())
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let spec = descriptor
            .parameter(&parameter_id)
            .filter(|spec| spec.kind() == ParameterKind::Asset)
            .ok_or(EventRejection::ActionUnavailableInContext)?;
        let reference = patch
            .instrument_config()
            .asset_reference(spec.id())
            .filter(|reference| reference.kind() == AssetKind::Sample)
            .ok_or(EventRejection::ActionUnavailableInContext)?;
        let _ = SampleAssetId::new(reference.locator())
            .map_err(|_| EventRejection::InvalidInstrumentConfig)?;
        self.sample_browser.begin(patch_id, parameter_id.clone());
        let row = self
            .sample_browser
            .rows()
            .first()
            .ok_or(EventRejection::InvalidSelection)?;
        let focus = FocusPath::sample_browser(
            patch_id,
            parameter_id.as_str().to_owned(),
            row.id().to_owned(),
        );
        self.interaction
            .enter_sample_browser(patch_id, parameter_id, focus)
            .map_err(|_| EventRejection::ActionUnavailableInContext)
    }

    fn open_patch_choice(&mut self) -> Result<(), EventRejection> {
        let origin = self.interaction.focus_path().clone();
        let resolver = SemanticResolver::new(self);
        let subject = resolver
            .choice_subject(&origin)
            .ok_or(EventRejection::ActionUnavailableInContext)?;
        let source = resolver.choice_source(&subject)?;
        let paths = resolver.patch_choice_paths(&subject)?;
        let focus = source
            .options()
            .iter()
            .find(|option| option.is_current() && option.is_enabled())
            .map(|option| {
                FocusPath::patch_choice(
                    subject.patch_id(),
                    subject.stable_id(),
                    option.id().to_owned(),
                )
            })
            .or_else(|| paths.first().cloned())
            .ok_or(EventRejection::ActionUnavailableInContext)?;
        self.interaction
            .enter_choice(subject, focus)
            .map_err(|_| EventRejection::ActionUnavailableInContext)
    }

    fn activate_focused_subordinate(&mut self) -> Result<ReducerEffects, EventRejection> {
        if matches!(
            self.interaction.subordinate_session(),
            Some(PatchSubordinateSession::SampleBrowser { .. })
        ) {
            return self.activate_sample_browser_row();
        }
        let subject = match self.interaction.subordinate_session() {
            Some(PatchSubordinateSession::Choice { subject, .. }) => subject.clone(),
            Some(PatchSubordinateSession::SampleBrowser { .. })
            | Some(PatchSubordinateSession::Detail { .. })
            | Some(PatchSubordinateSession::UtilityFromDetail { .. })
            | None => return Err(EventRejection::ActionUnavailableInContext),
        };
        let option_id = match self.interaction.focus_path().control_id() {
            SemanticControlId::Modal(ModalControlId::Choice(id)) => id.clone(),
            _ => return Err(EventRejection::InvalidSelection),
        };
        let source = SemanticResolver::new(self).choice_source(&subject)?;
        let option = source
            .options()
            .iter()
            .find(|option| option.id() == option_id && option.is_enabled())
            .ok_or(EventRejection::InvalidSelection)?;
        if option.is_current() {
            self.interaction
                .return_to_origin()
                .map_err(|_| EventRejection::ActionUnavailableInContext)?;
            return Ok(ReducerEffects::default());
        }

        let effect = match subject.control_id().clone() {
            crate::control::PatchControlId::Engine => {
                let target = self
                    .capabilities
                    .descriptors()
                    .iter()
                    .find(|descriptor| descriptor.id().to_string() == option_id)
                    .map(|descriptor| descriptor.id().clone())
                    .ok_or(EventRejection::EngineSelectionUnavailable)?;
                Some(self.request_engine_selection_to(target)?)
            }
            crate::control::PatchControlId::EffectSlot(slot) => {
                let entry = if option_id == crate::control::EMPTY_OCCUPANCY_CHOICE_ID {
                    None
                } else {
                    Some(
                        self.effects
                            .descriptors()
                            .iter()
                            .find(|descriptor| descriptor.id().to_string() == option_id)
                            .map(|descriptor| descriptor.id().clone())
                            .ok_or(EventRejection::InvalidEffectConfig)?,
                    )
                };
                Some(
                    self.request_topology_change(StructuralEditIntent::SetSlotOccupancy {
                        patch_id: subject.patch_id(),
                        slot,
                        entry,
                    })?,
                )
            }
            crate::control::PatchControlId::Output(PatchOutputParameter::OutputTrack) => {
                let track = MixerTrackId::ALL
                    .into_iter()
                    .find(|track| track.to_string() == option_id)
                    .ok_or(EventRejection::InvalidParameterValue)?;
                let patch = self
                    .patches
                    .iter_mut()
                    .find(|patch| patch.id() == subject.patch_id())
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                patch.set_output(patch.output().with_track_id(track));
                None
            }
            crate::control::PatchControlId::Capability(parameter_id) => {
                let patch = self
                    .patches
                    .iter()
                    .find(|patch| patch.id() == subject.patch_id())
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                let descriptor = self
                    .capabilities
                    .descriptor(patch.instrument_config().capability_id())
                    .ok_or(EventRejection::InvalidInstrumentConfig)?;
                let update = descriptor
                    .parameter(&parameter_id)
                    .filter(|spec| spec.kind() == ParameterKind::Choice)
                    .map(crate::synth::ParameterSpec::update)
                    .ok_or(EventRejection::InvalidSelection)?;
                if update == crate::synth::ParameterUpdate::Scalar {
                    self.set_instrument_scalar_choice(
                        subject.patch_id(),
                        &parameter_id,
                        option_id,
                    )?;
                    None
                } else {
                    Some(self.request_parameter_choice_to(parameter_id, option_id)?)
                }
            }
            crate::control::PatchControlId::Effect(slot_id, parameter_id) => {
                self.set_effect_choice(slot_id, &parameter_id, option_id)?;
                None
            }
            crate::control::PatchControlId::Output(_)
            | crate::control::PatchControlId::Envelope(_)
            | crate::control::PatchControlId::Global(_)
            | crate::control::PatchControlId::MidiInput
            | crate::control::PatchControlId::VoiceLimit => {
                return Err(EventRejection::InvalidSelection)
            }
        };
        self.interaction
            .return_to_origin()
            .map_err(|_| EventRejection::ActionUnavailableInContext)?;
        Ok(ReducerEffects {
            audio_command: None,
            engine_selection_effect: effect,
        })
    }

    fn activate_sample_browser_row(&mut self) -> Result<ReducerEffects, EventRejection> {
        let audio_command = self.preview_stop_command();
        let entry_id = match self.interaction.focus_path().control_id() {
            SemanticControlId::Modal(ModalControlId::BrowserEntry(id)) => id.clone(),
            _ => return Err(EventRejection::InvalidSelection),
        };
        let row = self
            .sample_browser
            .row(&entry_id)
            .cloned()
            .ok_or(EventRejection::InvalidSelection)?;
        match row.kind().clone() {
            SampleBrowserRowKind::Parent(folder) | SampleBrowserRowKind::Folder(folder) => {
                self.sample_browser.load_folder(folder);
                let focus = SemanticResolver::new(self)
                    .sample_browser_paths()?
                    .into_iter()
                    .next()
                    .ok_or(EventRejection::InvalidSelection)?;
                self.interaction.active_focus = focus;
                Ok(ReducerEffects {
                    audio_command,
                    engine_selection_effect: None,
                })
            }
            SampleBrowserRowKind::File(asset_id) => {
                let effect = self.request_asset_assignment(asset_id.clone())?;
                self.sample_browser
                    .assignment_requested(asset_id, effect.request_id());
                self.interaction
                    .return_to_origin()
                    .map_err(|_| EventRejection::ActionUnavailableInContext)?;
                Ok(ReducerEffects {
                    audio_command,
                    engine_selection_effect: Some(effect),
                })
            }
            SampleBrowserRowKind::Cancel => {
                self.sample_browser.cancelled();
                self.interaction
                    .return_to_origin()
                    .map_err(|_| EventRejection::ActionUnavailableInContext)?;
                Ok(ReducerEffects {
                    audio_command,
                    engine_selection_effect: None,
                })
            }
        }
    }

    fn request_asset_assignment(
        &mut self,
        asset_id: SampleAssetId,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if self.engine_selection.is_in_flight() {
            return Err(EventRejection::StructuralEditBusy);
        }
        let (patch_id, parameter_id) = match self.interaction.subordinate_session() {
            Some(PatchSubordinateSession::SampleBrowser {
                patch_id,
                asset_parameter_id,
                ..
            }) => (*patch_id, asset_parameter_id.clone()),
            _ => return Err(EventRejection::ActionUnavailableInContext),
        };
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let capability_id = patch.instrument_config().capability_id().clone();
        let descriptor = self
            .capabilities
            .descriptor(&capability_id)
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let spec = descriptor
            .parameter(&parameter_id)
            .filter(|spec| {
                spec.kind() == ParameterKind::Asset
                    && spec.update() == crate::synth::ParameterUpdate::Structural
            })
            .ok_or(EventRejection::InvalidSelection)?;
        let reference = AssetReference::new(AssetKind::Sample, asset_id.as_str())
            .map_err(|_| EventRejection::InvalidParameterValue)?;
        if patch.instrument_config().asset_reference(spec.id()) == Some(&reference) {
            return Err(EventRejection::ParameterAtBoundary);
        }
        let request_id = self
            .last_engine_selection_request_id
            .checked_next()
            .map_err(|_| EventRejection::RequestIdOverflow)?;
        let intent = StructuralEditIntent::ReplaceAsset {
            capability_id: capability_id.clone(),
            parameter_id,
            reference,
        };
        let status = EngineSelectionStatus::preparing_with_intent(
            self.engine_selection.active_graph_revision(),
            request_id,
            patch_id,
            intent,
            capability_id.clone(),
            capability_id,
        )
        .map_err(|_| EventRejection::InvalidInstrumentConfig)?;
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::PrepareRequested,
            status
                .correlation()
                .ok_or(EventRejection::MismatchedEngineSelection)?,
        )
        .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        self.engine_selection = status;
        self.last_engine_selection_request_id = request_id;
        Ok(effect)
    }

    fn select_context(&mut self, context: TopLevelContext) -> Result<(), EventRejection> {
        if context == TopLevelContext::Patch {
            let focus = self
                .interaction
                .patch_focus()
                .ok_or(EventRejection::NoPatchesInstalled)?;
            if !self.patches.iter().any(|patch| patch.id() == focus) {
                return Err(EventRejection::NoPatchesInstalled);
            }
        }
        self.interaction
            .select_context(context)
            .map_err(|_| EventRejection::InvalidSelection)?;
        Ok(())
    }

    /// Moves the focused Patch one position along the installed order.
    ///
    /// This is the only path by which the focused Patch changes. The fixture
    /// installs one Patch per MIDI part, so without it every instrument after
    /// the first is unreachable from the controller.
    ///
    /// Any open subordinate surface is left first: a detail subject belongs to
    /// the Patch it was opened on, and carrying it across would show one
    /// Patch's capability under another Patch's identity.
    ///
    /// Focus is then recovered against the destination Patch's own descriptor
    /// schema: the same control is kept when that Patch offers it, otherwise
    /// the one deterministic next-before-previous sibling rule finds the
    /// nearest row the destination does host. A control identity the
    /// destination cannot host is never carried across, and recovery never
    /// falls back to "first row" while a real sibling survives.
    ///
    /// `MidiInput` and `VoiceLimit` are Patch-local and follow the switch
    /// simply by being read from the newly focused Patch; master gain is not
    /// Patch-local and is untouched here.
    ///
    /// The whole switch is one accepted event, so it advances the generation
    /// exactly once and no intermediate state pairs one Patch's identity with
    /// another's schema.
    fn select_patch(&mut self, direction: Direction) -> Result<(), EventRejection> {
        if self.context() != TopLevelContext::Patch {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        if self.interaction.mode() != crate::control::InteractionMode::Navigate {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        // Stepping through the installed order is a horizontal adjacent
        // choice; vertical directions belong to control navigation.
        let step: isize = match direction {
            Direction::Left => -1,
            Direction::Right => 1,
            Direction::Up | Direction::Down => {
                return Err(EventRejection::ActionUnavailableInContext)
            }
        };

        let focused = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let current = self
            .patches
            .iter()
            .position(|patch| patch.id() == focused)
            .ok_or(EventRejection::UnknownPatch)?;

        // No wrapping: at either end this is an unchanged rejection, exactly
        // as an adjacent-choice parameter behaves at its boundary.
        let target = isize::try_from(current)
            .map_err(|_| EventRejection::UnknownPatch)?
            .checked_add(step)
            .ok_or(EventRejection::ParameterAtBoundary)?;
        let target = usize::try_from(target).map_err(|_| EventRejection::ParameterAtBoundary)?;
        let target_id = self
            .patches
            .get(target)
            .ok_or(EventRejection::ParameterAtBoundary)?
            .id();

        // Any open subordinate surface is left as part of landing on the
        // destination: `set_active_main` below clears the return path and the
        // detail subject together, because a main path is by definition not a
        // detail path. Clearing here as well would be a second owner of the
        // same fact, and the invariant assertion could not tell them apart.
        let held = self.interaction.patch_control_focus();
        let resolver = SemanticResolver::new(self);
        let source_order = resolver.patch_main_paths(focused)?;
        let candidates = resolver.patch_main_paths(target_id)?;

        // Recovery runs over *control* identities, because the two orders
        // carry different PatchIds and so can never compare equal as whole
        // paths. Exact identity wins; otherwise the shared next-before-
        // previous rule walks outward through the source order for the
        // nearest control the destination also hosts.
        let recovered = held
            .and_then(|control| {
                let source_controls = source_order
                    .iter()
                    .map(FocusPath::control_id)
                    .collect::<Vec<_>>();
                let destination_controls = candidates
                    .iter()
                    .map(FocusPath::control_id)
                    .collect::<Vec<_>>();
                let held = SemanticControlId::Patch(control);
                SemanticResolver::recovered_index(&&held, &source_controls, &destination_controls)
                    .map(|index| &candidates[index])
            })
            .or_else(|| candidates.first())
            .ok_or(EventRejection::NoPatchesInstalled)?
            .clone();

        self.interaction
            .set_active_main(recovered)
            .map_err(|_| EventRejection::InvalidSelection)?;
        Ok(())
    }

    fn request_engine_selection(
        &mut self,
        direction: Direction,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if matches!(direction, Direction::Up | Direction::Down) {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        if self.engine_selection.is_in_flight() {
            return Err(EventRejection::StructuralEditBusy);
        }
        if self.interaction.patch_control_focus() != Some(crate::control::PatchControlId::Engine) {
            return Err(EventRejection::EngineSelectionUnavailable);
        }
        let request_id = self
            .last_engine_selection_request_id
            .checked_next()
            .map_err(|_| EventRejection::RequestIdOverflow)?;

        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::EngineSelectionUnavailable)?;
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::EngineSelectionUnavailable)?;
        let source_capability_id = patch.instrument_config().capability_id().clone();
        let descriptors = self.capabilities.descriptors();
        if descriptors.len() < 2 {
            return Err(EventRejection::EngineSelectionUnavailable);
        }
        let source_index = descriptors
            .iter()
            .position(|descriptor| descriptor.id() == &source_capability_id)
            .ok_or(EventRejection::EngineSelectionUnavailable)?;
        let target_index = match direction {
            Direction::Left => source_index.checked_sub(1),
            Direction::Right => source_index
                .checked_add(1)
                .filter(|index| *index < descriptors.len()),
            Direction::Up | Direction::Down => unreachable!("vertical PATCH edits were rejected"),
        }
        .ok_or(EventRejection::EngineSelectionUnavailable)?;
        let target_capability_id = descriptors[target_index].id().clone();
        let intent = StructuralEditIntent::ReplaceCapability {
            target_capability_id: target_capability_id.clone(),
        };
        let status = EngineSelectionStatus::preparing_with_intent(
            self.engine_selection.active_graph_revision(),
            request_id,
            patch_id,
            intent,
            source_capability_id,
            target_capability_id,
        )
        .map_err(|_| EventRejection::EngineSelectionUnavailable)?;
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::PrepareRequested,
            status
                .correlation()
                .expect("Preparing status always owns correlation"),
        )
        .expect("Preparing correlation has no target revision");
        self.engine_selection = status;
        self.last_engine_selection_request_id = request_id;
        Ok(effect)
    }

    fn request_engine_selection_to(
        &mut self,
        target_capability_id: crate::synth::CapabilityId,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if self.engine_selection.is_in_flight() {
            return Err(EventRejection::StructuralEditBusy);
        }
        if self
            .capabilities
            .descriptor(&target_capability_id)
            .is_none()
        {
            return Err(EventRejection::EngineSelectionUnavailable);
        }
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::EngineSelectionUnavailable)?;
        let source_capability_id = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .map(|patch| patch.instrument_config().capability_id().clone())
            .ok_or(EventRejection::EngineSelectionUnavailable)?;
        if source_capability_id == target_capability_id {
            return Err(EventRejection::EngineSelectionUnavailable);
        }
        let request_id = self
            .last_engine_selection_request_id
            .checked_next()
            .map_err(|_| EventRejection::RequestIdOverflow)?;
        let intent = StructuralEditIntent::ReplaceCapability {
            target_capability_id: target_capability_id.clone(),
        };
        let status = EngineSelectionStatus::preparing_with_intent(
            self.engine_selection.active_graph_revision(),
            request_id,
            patch_id,
            intent,
            source_capability_id,
            target_capability_id,
        )
        .map_err(|_| EventRejection::EngineSelectionUnavailable)?;
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::PrepareRequested,
            status
                .correlation()
                .expect("Preparing status always owns correlation"),
        )
        .expect("Preparing correlation has no target revision");
        self.engine_selection = status;
        self.last_engine_selection_request_id = request_id;
        Ok(effect)
    }

    #[allow(clippy::too_many_arguments)]
    fn engine_prepared(
        &mut self,
        request_id: EngineSelectionRequestId,
        patch_id: crate::kernel::patch_id::PatchId,
        intent: StructuralEditIntent,
        source_capability_id: crate::synth::CapabilityId,
        target_capability_id: crate::synth::CapabilityId,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
        candidate_config: crate::synth::InstrumentConfig,
        prepared_visualization: Option<crate::synth::PreparedSampleVisualization>,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        let correlation = self.pending_correlation(request_id)?.clone();
        if correlation.patch_id() != Some(patch_id)
            || correlation.intent() != &intent
            || correlation.source_capability_id() != Some(&source_capability_id)
            || correlation.target_capability_id() != Some(&target_capability_id)
            || correlation.source_graph_revision() != source_graph_revision
            || target_graph_revision <= source_graph_revision
            || candidate_config.capability_id() != &target_capability_id
            || self
                .capabilities
                .validate_config(&candidate_config)
                .is_err()
        {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::MismatchedEngineSelection)?;
        if patch.instrument_config().capability_id() != &source_capability_id
            || !candidate_matches_intent(patch.instrument_config(), &candidate_config, &intent)
        {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        let status = self
            .engine_selection
            .activating(target_graph_revision)
            .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::CandidatePrepared,
            status
                .correlation()
                .expect("Activating status always owns correlation"),
        )
        .expect("Activating correlation owns a target revision");
        let asset_assignment = matches!(intent, StructuralEditIntent::ReplaceAsset { .. });
        let audition = matches!(intent, StructuralEditIntent::PrepareAudition { .. });
        if asset_assignment
            && self.sample_browser.lifecycle() != crate::control::SampleAssetLifecycle::Preparing
        {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        if !audition {
            // The prepared graph owns the candidate while canonical Patch
            // state continues to describe the acknowledged active graph.
            self.pending_instrument_config = Some(candidate_config);
        }
        self.pending_sample_visualization = prepared_visualization.map(|value| (request_id, value));
        self.engine_selection = status;
        if asset_assignment {
            self.sample_browser.assignment_activating(request_id);
        } else if audition {
            self.sample_browser.preview_activating(request_id);
        }
        Ok(effect)
    }

    fn engine_selection_lifecycle_advanced(
        &mut self,
        request_id: EngineSelectionRequestId,
        lifecycle: EngineSelectionStatusKind,
    ) -> Result<(), EventRejection> {
        let correlation = self
            .engine_selection
            .correlation()
            .ok_or(EventRejection::StaleEngineSelection)?;
        if correlation.request_id() != request_id {
            return Err(EventRejection::StaleEngineSelection);
        }
        self.engine_selection = self
            .engine_selection
            .advance_admission(lifecycle)
            .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn engine_preparation_failed(
        &mut self,
        request_id: EngineSelectionRequestId,
        patch_id: crate::kernel::patch_id::PatchId,
        intent: &StructuralEditIntent,
        source_capability_id: &crate::synth::CapabilityId,
        target_capability_id: &crate::synth::CapabilityId,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
        failure: EngineSelectionFailure,
    ) -> Result<(), EventRejection> {
        let correlation = self.pending_correlation(request_id)?;
        if correlation.patch_id() != Some(patch_id)
            || correlation.intent() != intent
            || correlation.source_capability_id() != Some(source_capability_id)
            || correlation.target_capability_id() != Some(target_capability_id)
            || correlation.source_graph_revision() != source_graph_revision
            || target_graph_revision <= source_graph_revision
        {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        self.engine_selection = if unavailable_failure(failure) {
            self.engine_selection.unavailable(failure)
        } else {
            self.engine_selection.failed(failure)
        }
        .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        if !matches!(intent, StructuralEditIntent::PrepareAudition { .. }) {
            self.pending_instrument_config = None;
        }
        if matches!(intent, StructuralEditIntent::ReplaceAsset { .. }) {
            self.pending_instrument_config = None;
            self.pending_sample_visualization = None;
            self.sample_browser.assignment_failed(request_id, failure);
        } else if matches!(intent, StructuralEditIntent::PrepareAudition { .. }) {
            self.pending_sample_visualization = None;
            self.sample_browser.preview_failed(request_id, failure);
        }
        Ok(())
    }

    fn engine_activation_acknowledged(
        &mut self,
        request_id: EngineSelectionRequestId,
        intent: &StructuralEditIntent,
        target_graph_revision: GraphRevision,
        retired_graph_revision: GraphRevision,
        collected: bool,
    ) -> Result<(EngineSelectionEffect, Option<AudioCommand>), EventRejection> {
        if self.engine_selection.kind() != EngineSelectionStatusKind::Activating {
            return Err(EventRejection::StaleEngineSelection);
        }
        let correlation = self
            .engine_selection
            .correlation()
            .ok_or(EventRejection::StaleEngineSelection)?
            .clone();
        if correlation.request_id() != request_id {
            return Err(EventRejection::StaleEngineSelection);
        }
        if correlation.intent() != intent {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        if correlation.target_graph_revision() != Some(target_graph_revision)
            || correlation.source_graph_revision() != retired_graph_revision
            || !collected
        {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        if !self.pending_intent_matches_state(&correlation) {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        let asset_assignment = matches!(intent, StructuralEditIntent::ReplaceAsset { .. });
        let audition = matches!(intent, StructuralEditIntent::PrepareAudition { .. });
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::ActivationAcknowledged,
            &correlation,
        )
        .expect("Activating correlation owns a target revision");
        if !audition && !intent.is_occupancy() {
            let candidate = self
                .pending_instrument_config
                .clone()
                .ok_or(EventRejection::MismatchedEngineSelection)?;
            let patch_id = correlation
                .patch_id()
                .ok_or(EventRejection::MismatchedEngineSelection)?;
            let old_patch_order = SemanticResolver::new(self).patch_main_paths(patch_id)?;
            let old_mixer_order = SemanticResolver::new(self).mixer_main_paths()?;
            let target_policy = self
                .capabilities
                .descriptor(candidate.capability_id())
                .ok_or(EventRejection::MismatchedEngineSelection)?
                .voice_policy();
            let patch = self
                .patches
                .iter_mut()
                .find(|patch| patch.id() == patch_id)
                .ok_or(EventRejection::MismatchedEngineSelection)?;
            patch.replace_instrument_config(candidate, target_policy);
            if asset_assignment {
                if let Some((pending_request, visualization)) =
                    self.pending_sample_visualization.take()
                {
                    if pending_request == request_id {
                        self.sample_visualizations.insert(patch_id, visualization);
                    }
                }
            }
            self.pending_instrument_config = None;
            self.repair_semantic_paths(&old_patch_order, &old_mixer_order)?;
        } else if let StructuralEditIntent::SetSlotOccupancy {
            patch_id,
            slot,
            entry,
        } = intent
        {
            let old_patch_order = SemanticResolver::new(self).patch_main_paths(*patch_id)?;
            let old_mixer_order = SemanticResolver::new(self).mixer_main_paths()?;
            let occupant = match entry {
                None => None,
                Some(entry_id) => Some(
                    self.effects
                        .descriptor(entry_id)
                        .ok_or(EventRejection::MismatchedEngineSelection)?
                        .default_config(slot.instance_identity())
                        .map_err(|_| EventRejection::MismatchedEngineSelection)?,
                ),
            };
            self.patches
                .iter_mut()
                .find(|patch| patch.id() == *patch_id)
                .ok_or(EventRejection::MismatchedEngineSelection)?
                .set_slot_occupancy(*slot, occupant)
                .map_err(|_| EventRejection::MismatchedEngineSelection)?;
            self.repair_semantic_paths(&old_patch_order, &old_mixer_order)?;
        } else if let StructuralEditIntent::SetReturnOccupancy { bus, entry } = intent {
            let old_inspector_order = self.mixer_inspector_order();
            self.returns
                .set_return_occupancy(&self.effects, *bus, entry.as_ref())
                .map_err(|_| EventRejection::MismatchedEngineSelection)?;
            self.repair_inspector_focus(old_inspector_order.as_deref())?;
        }
        self.engine_selection = self
            .engine_selection
            .acknowledged()
            .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        if asset_assignment {
            self.sample_browser.assignment_ready(request_id);
        }
        let audio_command = if audition && self.sample_browser.preview_ready(request_id) {
            Some(AudioCommand::preview_start(
                correlation
                    .patch_id()
                    .ok_or(EventRejection::MismatchedEngineSelection)?,
                request_id.value(),
            ))
        } else {
            None
        };
        if audition && audio_command.is_none() {
            self.pending_sample_visualization = None;
        }
        Ok((effect, audio_command))
    }

    /// Enters the shared correlated structural lifecycle for one occupancy
    /// change. Everything is validated before any state mutates or anything
    /// can be published (FR-013): the position is in range by construction,
    /// the target Patch must exist, and the registry entry must resolve and
    /// yield a valid default configuration — refused, never substituted.
    fn request_topology_change(
        &mut self,
        intent: StructuralEditIntent,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if self.engine_selection.is_in_flight() {
            return Err(EventRejection::StructuralEditBusy);
        }
        match &intent {
            StructuralEditIntent::SetSlotOccupancy {
                patch_id,
                slot,
                entry,
            } => {
                let patch = self
                    .patches
                    .iter()
                    .find(|patch| patch.id() == *patch_id)
                    .ok_or(EventRejection::UnknownPatch)?;
                if let Some(entry_id) = entry {
                    let descriptor = self
                        .effects
                        .descriptor(entry_id)
                        .ok_or(EventRejection::InvalidEffectConfig)?;
                    let occupant = descriptor
                        .default_config(slot.instance_identity())
                        .map_err(|_| EventRejection::InvalidEffectConfig)?;
                    // Dry-run the domain transition on a clone so a rejected
                    // occupancy (duplicate instance identity) refuses the
                    // request without mutating anything.
                    let mut probe = patch.clone();
                    probe
                        .set_slot_occupancy(*slot, Some(occupant))
                        .map_err(|_| EventRejection::InvalidEffectConfig)?;
                }
            }
            StructuralEditIntent::SetReturnOccupancy { bus, entry } => {
                if let Some(entry_id) = entry {
                    if self.effects.descriptor(entry_id).is_none() {
                        return Err(EventRejection::InvalidEffectConfig);
                    }
                    let mut probe = self.returns.clone();
                    probe
                        .set_return_occupancy(&self.effects, *bus, Some(entry_id))
                        .map_err(|_| EventRejection::InvalidEffectConfig)?;
                }
            }
            StructuralEditIntent::ReplaceCapability { .. }
            | StructuralEditIntent::ReplaceParameterChoice { .. }
            | StructuralEditIntent::ReplaceAsset { .. }
            | StructuralEditIntent::PrepareAudition { .. } => {
                return Err(EventRejection::InvalidSelection)
            }
        }
        let request_id = self
            .last_engine_selection_request_id
            .checked_next()
            .map_err(|_| EventRejection::RequestIdOverflow)?;
        let status = EngineSelectionStatus::preparing_for_occupancy(
            self.engine_selection.active_graph_revision(),
            request_id,
            intent,
        )
        .map_err(|_| EventRejection::InvalidSelection)?;
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::PrepareRequested,
            status
                .correlation()
                .expect("Preparing status always owns correlation"),
        )
        .expect("Preparing correlation has no target revision");
        self.engine_selection = status;
        self.last_engine_selection_request_id = request_id;
        Ok(effect)
    }

    /// Records one prepared occupancy candidate and enters Activating.
    /// Canonical occupancy remains on the acknowledged graph until activation.
    fn topology_prepared(
        &mut self,
        request_id: EngineSelectionRequestId,
        intent: StructuralEditIntent,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        let correlation = self.pending_correlation(request_id)?;
        if correlation.intent() != &intent
            || correlation.source_graph_revision() != source_graph_revision
            || target_graph_revision <= source_graph_revision
        {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        match &intent {
            StructuralEditIntent::SetSlotOccupancy { .. }
            | StructuralEditIntent::SetReturnOccupancy { .. } => {
                let status = self
                    .engine_selection
                    .activating(target_graph_revision)
                    .map_err(|_| EventRejection::MismatchedEngineSelection)?;
                let effect = EngineSelectionEffect::from_correlation(
                    EngineSelectionEffectKind::CandidatePrepared,
                    status
                        .correlation()
                        .expect("Activating status always owns correlation"),
                )
                .expect("Activating correlation owns a target revision");
                self.engine_selection = status;
                Ok(effect)
            }
            StructuralEditIntent::ReplaceCapability { .. }
            | StructuralEditIntent::ReplaceParameterChoice { .. }
            | StructuralEditIntent::ReplaceAsset { .. }
            | StructuralEditIntent::PrepareAudition { .. } => {
                Err(EventRejection::MismatchedEngineSelection)
            }
        }
    }

    /// Records one correlated occupancy refusal: the lifecycle enters Failed
    /// with its reason and position while every canonical value stays intact.
    fn topology_preparation_failed(
        &mut self,
        request_id: EngineSelectionRequestId,
        intent: &StructuralEditIntent,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
        failure: EngineSelectionFailure,
    ) -> Result<(), EventRejection> {
        let correlation = self.pending_correlation(request_id)?;
        if correlation.intent() != intent
            || correlation.source_graph_revision() != source_graph_revision
            || target_graph_revision <= source_graph_revision
            || !intent.is_occupancy()
        {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        self.engine_selection = if unavailable_failure(failure) {
            self.engine_selection.unavailable(failure)
        } else {
            self.engine_selection.failed(failure)
        }
        .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        Ok(())
    }

    /// Verifies acknowledged structure against canonical state: the committed
    /// instrument config for instrument intents, the committed occupancy for
    /// slot and return intents.
    fn pending_intent_matches_state(
        &self,
        correlation: &crate::control::EngineSelectionCorrelation,
    ) -> bool {
        match correlation.intent() {
            StructuralEditIntent::ReplaceCapability { .. }
            | StructuralEditIntent::ReplaceParameterChoice { .. }
            | StructuralEditIntent::ReplaceAsset { .. } => {
                let Some(patch_id) = correlation.patch_id() else {
                    return false;
                };
                let Some(patch) = self.patches.iter().find(|patch| patch.id() == patch_id) else {
                    return false;
                };
                let Some(candidate) = self.pending_instrument_config.as_ref() else {
                    return false;
                };
                correlation.source_capability_id()
                    == Some(patch.instrument_config().capability_id())
                    && candidate_matches_intent(
                        patch.instrument_config(),
                        candidate,
                        correlation.intent(),
                    )
            }
            StructuralEditIntent::PrepareAudition { capability_id, .. } => correlation
                .patch_id()
                .and_then(|patch_id| self.patches.iter().find(|patch| patch.id() == patch_id))
                .is_some_and(|patch| patch.instrument_config().capability_id() == capability_id),
            StructuralEditIntent::SetSlotOccupancy { patch_id, .. } => {
                let Some(patch) = self.patches.iter().find(|patch| patch.id() == *patch_id) else {
                    return false;
                };
                correlation.source_graph_revision() == self.engine_selection.active_graph_revision()
                    && patch.id() == *patch_id
            }
            StructuralEditIntent::SetReturnOccupancy { .. } => {
                correlation.source_graph_revision() == self.engine_selection.active_graph_revision()
            }
        }
    }

    fn pending_correlation(
        &self,
        request_id: EngineSelectionRequestId,
    ) -> Result<&crate::control::EngineSelectionCorrelation, EventRejection> {
        if !matches!(
            self.engine_selection.kind(),
            EngineSelectionStatusKind::Loading
                | EngineSelectionStatusKind::Validating
                | EngineSelectionStatusKind::Preparing
        ) {
            return Err(EventRejection::StaleEngineSelection);
        }
        let correlation = self
            .engine_selection
            .correlation()
            .ok_or(EventRejection::StaleEngineSelection)?;
        if correlation.request_id() != request_id {
            return Err(EventRejection::StaleEngineSelection);
        }
        Ok(correlation)
    }

    fn navigate(&mut self, direction: Direction) -> Result<(), EventRejection> {
        match self.interaction.active_surface() {
            SurfaceId::MixerMain => match direction {
                Direction::Left | Direction::Right => {
                    let MixerControlId::Track {
                        track_id,
                        parameter,
                    } = self.interaction.mixer_control_focus().clone()
                    else {
                        return Err(EventRejection::InvalidSelection);
                    };
                    let track_id = track_id
                        .adjacent(direction == Direction::Right)
                        .map_err(|_| EventRejection::ActionUnavailableInContext)?;
                    self.interaction
                        .set_active_main(FocusPath::mixer_track(track_id, parameter))
                        .map_err(|_| EventRejection::InvalidSelection)
                }
                Direction::Up | Direction::Down => self.navigate_parameter_nonwrapping(
                    &SemanticResolver::new(self).mixer_main_paths()?,
                    direction == Direction::Down,
                ),
            },
            SurfaceId::MixerInspector => match direction {
                Direction::Up | Direction::Down => {
                    let track_id = self
                        .interaction
                        .remembered_mixer_main()
                        .control_id()
                        .as_mixer_track_id()
                        .ok_or(EventRejection::InvalidSelection)?;
                    self.navigate_side_nonwrapping(
                        &SemanticResolver::new(self).mixer_inspector_paths(track_id)?,
                        direction == Direction::Down,
                    )
                }
                Direction::Left | Direction::Right => {
                    Err(EventRejection::ActionUnavailableInContext)
                }
            },
            SurfaceId::PatchMain
            | SurfaceId::PatchUtility
            | SurfaceId::PatchDetail
            | SurfaceId::PatchChoice
            | SurfaceId::SampleBrowser => Err(EventRejection::ActionUnavailableInContext),
        }
    }

    fn navigate_patch_control(&mut self, direction: Direction) -> Result<(), EventRejection> {
        match self.interaction.active_surface() {
            // Detail owns vertical row adjacency. Left closes to Overview;
            // Right switches to persistent Utility while suspending Detail.
            SurfaceId::PatchDetail => {
                if direction == Direction::Left {
                    self.interaction
                        .return_to_origin()
                        .map_err(|_| EventRejection::ActionUnavailableInContext)
                } else if direction == Direction::Right {
                    let patch_id = self
                        .interaction
                        .patch_focus()
                        .ok_or(EventRejection::NoPatchesInstalled)?;
                    let utility_focus = SemanticResolver::new(self)
                        .patch_utility_paths(patch_id)?
                        .into_iter()
                        .next()
                        .ok_or(EventRejection::ActionUnavailableInContext)?;
                    self.interaction
                        .switch_detail_to_utility(utility_focus)
                        .map_err(|_| EventRejection::ActionUnavailableInContext)
                } else if matches!(direction, Direction::Up | Direction::Down) {
                    let paths =
                        SemanticResolver::new(self).ordered_paths(SurfaceId::PatchDetail)?;
                    self.navigate_side_nonwrapping(&paths, direction == Direction::Down)
                } else {
                    Err(EventRejection::ActionUnavailableInContext)
                }
            }
            SurfaceId::PatchUtility => {
                if direction == Direction::Left {
                    if self.interaction.utility_suspends_detail() {
                        self.interaction
                            .restore_detail_from_utility()
                            .map_err(|_| EventRejection::ActionUnavailableInContext)
                    } else {
                        self.interaction
                            .return_to_origin()
                            .map_err(|_| EventRejection::ActionUnavailableInContext)
                    }
                } else if matches!(direction, Direction::Up | Direction::Down) {
                    let patch_id = self
                        .interaction
                        .patch_focus()
                        .ok_or(EventRejection::NoPatchesInstalled)?;
                    self.navigate_side_nonwrapping(
                        &SemanticResolver::new(self).patch_utility_paths(patch_id)?,
                        direction == Direction::Down,
                    )
                } else {
                    Err(EventRejection::ActionUnavailableInContext)
                }
            }
            SurfaceId::PatchChoice => {
                if matches!(direction, Direction::Up | Direction::Down) {
                    let paths =
                        SemanticResolver::new(self).ordered_paths(SurfaceId::PatchChoice)?;
                    self.navigate_side_nonwrapping(&paths, direction == Direction::Down)
                } else {
                    Err(EventRejection::ActionUnavailableInContext)
                }
            }
            SurfaceId::SampleBrowser => {
                if matches!(direction, Direction::Up | Direction::Down) {
                    let paths = SemanticResolver::new(self).sample_browser_paths()?;
                    self.navigate_side_nonwrapping(&paths, direction == Direction::Down)?;
                    self.sample_browser.preview_stop();
                    Ok(())
                } else {
                    Err(EventRejection::ActionUnavailableInContext)
                }
            }
            SurfaceId::PatchMain => match direction {
                Direction::Right => self
                    .interaction
                    .enter_surface(SurfaceId::PatchUtility)
                    .map_err(|_| EventRejection::ActionUnavailableInContext),
                Direction::Left => Err(EventRejection::ActionUnavailableInContext),
                Direction::Up | Direction::Down => {
                    let patch_id = self
                        .interaction
                        .patch_focus()
                        .ok_or(EventRejection::NoPatchesInstalled)?;
                    let paths = SemanticResolver::new(self).patch_main_paths(patch_id)?;
                    let current = paths
                        .iter()
                        .position(|path| path == self.interaction.focus_path())
                        .ok_or(EventRejection::InvalidSelection)?;
                    let next = match direction {
                        Direction::Up => current.checked_sub(1),
                        Direction::Down => {
                            current.checked_add(1).filter(|index| *index < paths.len())
                        }
                        Direction::Left | Direction::Right => unreachable!(),
                    }
                    .ok_or(EventRejection::ActionUnavailableInContext)?;
                    self.interaction
                        .set_active_main(paths[next].clone())
                        .map_err(|_| EventRejection::InvalidSelection)
                }
            },
            SurfaceId::MixerMain | SurfaceId::MixerInspector => {
                Err(EventRejection::ActionUnavailableInContext)
            }
        }
    }

    fn adjust_patch_control(
        &mut self,
        direction: Direction,
    ) -> Result<ReducerEffects, EventRejection> {
        if direction == Direction::Up
            && SemanticResolver::new(self)
                .choice_subject(self.interaction.focus_path())
                .is_some()
        {
            self.open_patch_choice()?;
            return Ok(ReducerEffects::default());
        }
        if self.interaction.active_surface() == SurfaceId::PatchDetail {
            return self.adjust_patch_detail(direction);
        }
        if self.interaction.active_surface() == SurfaceId::PatchUtility {
            let SemanticControlId::Patch(control) = self.interaction.focus_path().control_id()
            else {
                return Err(EventRejection::InvalidSelection);
            };
            match control.clone() {
                crate::control::PatchControlId::Output(parameter) => {
                    self.adjust_patch_output(parameter, direction)?;
                }
                // Master volume from PATCH Utility reaches the one canonical
                // GlobalParameters value through `adjust_global` — the very
                // method the MIXER Inspector's own arm calls. PATCH adds no
                // field and no second setter, so an edit made on either
                // surface is the same edit.
                crate::control::PatchControlId::Global(parameter) => {
                    self.adjust_global(parameter, direction)?;
                }
                crate::control::PatchControlId::MidiInput => {
                    self.adjust_patch_midi_input(direction)?;
                }
                crate::control::PatchControlId::VoiceLimit => {
                    self.adjust_patch_voice_limit(direction)?;
                }
                crate::control::PatchControlId::Engine
                | crate::control::PatchControlId::Envelope(_)
                | crate::control::PatchControlId::Capability(_)
                | crate::control::PatchControlId::EffectSlot(_)
                | crate::control::PatchControlId::Effect(..) => {
                    return Err(EventRejection::InvalidSelection);
                }
            }
            return Ok(ReducerEffects::default());
        }
        if self.interaction.active_surface() != SurfaceId::PatchMain {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        match self.interaction.patch_control_focus() {
            Some(crate::control::PatchControlId::Engine) => {
                let engine_selection_effect = self.request_engine_selection(direction)?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(engine_selection_effect),
                })
            }
            Some(crate::control::PatchControlId::Envelope(parameter)) => {
                let patch_id = self
                    .interaction
                    .patch_focus()
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                let patch_index = self
                    .patches
                    .iter()
                    .position(|patch| patch.id() == patch_id)
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                self.adjust_patch_envelope(patch_index, parameter, direction)?;
                Ok(ReducerEffects::default())
            }
            Some(crate::control::PatchControlId::Capability(parameter_id)) => {
                let structural_effect = self.request_parameter_choice(parameter_id, direction)?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(structural_effect),
                })
            }
            Some(crate::control::PatchControlId::EffectSlot(slot)) => {
                let structural_effect = self.request_slot_occupancy_choice(slot, direction)?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(structural_effect),
                })
            }
            Some(crate::control::PatchControlId::Effect(slot_id, parameter_id)) => {
                self.adjust_patch_effect(slot_id, &parameter_id, direction)?;
                Ok(ReducerEffects::default())
            }
            // The five Utility identities are never focused on PatchMain.
            Some(crate::control::PatchControlId::Output(_))
            | Some(crate::control::PatchControlId::Global(_))
            | Some(crate::control::PatchControlId::MidiInput)
            | Some(crate::control::PatchControlId::VoiceLimit) => {
                Err(EventRejection::InvalidSelection)
            }
            None => Err(EventRejection::NoPatchesInstalled),
        }
    }

    fn preview_start(&mut self) -> Result<EngineSelectionEffect, EventRejection> {
        if self.interaction.active_surface() != SurfaceId::SampleBrowser
            || !matches!(self.sample_browser.preview(), SamplePreviewState::Idle)
            || self.engine_selection.is_in_flight()
        {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        let entry_id = match self.interaction.focus_path().control_id() {
            SemanticControlId::Modal(ModalControlId::BrowserEntry(id)) => id,
            _ => return Err(EventRejection::InvalidSelection),
        };
        let asset_id = match self.sample_browser.row(entry_id).map(|row| row.kind()) {
            Some(SampleBrowserRowKind::File(asset_id)) => asset_id.clone(),
            _ => return Err(EventRejection::ActionUnavailableInContext),
        };
        let (patch_id, parameter_id) = match self.interaction.subordinate_session() {
            Some(PatchSubordinateSession::SampleBrowser {
                patch_id,
                asset_parameter_id,
                ..
            }) => (*patch_id, asset_parameter_id.clone()),
            _ => return Err(EventRejection::ActionUnavailableInContext),
        };
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let capability_id = patch.instrument_config().capability_id().clone();
        let descriptor = self
            .capabilities
            .descriptor(&capability_id)
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        descriptor
            .parameter(&parameter_id)
            .filter(|spec| {
                spec.kind() == ParameterKind::Asset
                    && spec.update() == crate::synth::ParameterUpdate::Structural
            })
            .ok_or(EventRejection::InvalidSelection)?;
        let reference = AssetReference::new(AssetKind::Sample, asset_id.as_str())
            .map_err(|_| EventRejection::InvalidParameterValue)?;
        let request_id = self
            .last_engine_selection_request_id
            .checked_next()
            .map_err(|_| EventRejection::RequestIdOverflow)?;
        let intent = StructuralEditIntent::PrepareAudition {
            capability_id: capability_id.clone(),
            parameter_id,
            reference,
        };
        let status = EngineSelectionStatus::preparing_with_intent(
            self.engine_selection.active_graph_revision(),
            request_id,
            patch_id,
            intent,
            capability_id.clone(),
            capability_id,
        )
        .map_err(|_| EventRejection::InvalidInstrumentConfig)?;
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::PrepareRequested,
            status
                .correlation()
                .ok_or(EventRejection::MismatchedEngineSelection)?,
        )
        .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        self.engine_selection = status;
        self.last_engine_selection_request_id = request_id;
        self.sample_browser.preview_requested(asset_id, request_id);
        Ok(effect)
    }

    fn preview_stop(&mut self) -> Result<Option<AudioCommand>, EventRejection> {
        if self.interaction.active_surface() != SurfaceId::SampleBrowser
            || matches!(self.sample_browser.preview(), SamplePreviewState::Idle)
        {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        Ok(self.preview_stop_command())
    }

    fn preview_stop_command(&mut self) -> Option<AudioCommand> {
        let patch_id = self.sample_browser.patch_id()?;
        let request_id = self.sample_browser.preview_request_id()?;
        let command = self
            .sample_browser
            .preview_stop()
            .then(|| AudioCommand::preview_stop(patch_id, request_id.value()));
        if command.is_some() {
            self.pending_sample_visualization = None;
        }
        command
    }

    fn adjust_patch_output(
        &mut self,
        parameter: PatchOutputParameter,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch = self
            .patches
            .iter_mut()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let output = patch.output();
        let updated = match parameter {
            PatchOutputParameter::TrimGain => {
                let descriptor = parameter.descriptor();
                let value = adjusted_value(
                    output.trim_gain_db(),
                    descriptor
                        .minimum()
                        .ok_or(EventRejection::InvalidSelection)?,
                    descriptor
                        .maximum()
                        .ok_or(EventRejection::InvalidSelection)?,
                    direction,
                    descriptor
                        .fine_step()
                        .ok_or(EventRejection::InvalidSelection)?,
                    descriptor
                        .coarse_step()
                        .ok_or(EventRejection::InvalidSelection)?,
                )?;
                output
                    .with_trim_gain_db(value)
                    .map_err(|_| EventRejection::InvalidParameterValue)?
            }
            PatchOutputParameter::OutputTrack => {
                if matches!(direction, Direction::Up | Direction::Down) {
                    return Err(EventRejection::ActionUnavailableInContext);
                }
                output
                    .with_adjacent_track(direction == Direction::Right)
                    .map_err(|_| EventRejection::ParameterAtBoundary)?
            }
        };
        patch.set_output(updated);
        Ok(())
    }

    fn adjust_patch_detail(
        &mut self,
        direction: Direction,
    ) -> Result<ReducerEffects, EventRejection> {
        let control = match self.interaction.focus_path().control_id() {
            SemanticControlId::Patch(control) => control.clone(),
            _ => return Err(EventRejection::InvalidSelection),
        };
        match control {
            crate::control::PatchControlId::Capability(parameter_id) => {
                if SemanticResolver::new(self)
                    .choice_subject(self.interaction.focus_path())
                    .is_some()
                {
                    let engine_selection_effect =
                        self.request_parameter_choice(parameter_id, direction)?;
                    Ok(ReducerEffects {
                        audio_command: None,
                        engine_selection_effect: Some(engine_selection_effect),
                    })
                } else {
                    self.adjust_instrument_parameter(&parameter_id, direction)?;
                    Ok(ReducerEffects::default())
                }
            }
            crate::control::PatchControlId::Effect(slot_id, parameter_id) => {
                self.adjust_patch_effect(slot_id, &parameter_id, direction)?;
                Ok(ReducerEffects::default())
            }
            crate::control::PatchControlId::Envelope(parameter) => {
                let patch_id = self
                    .interaction
                    .patch_focus()
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                let patch_index = self
                    .patches
                    .iter()
                    .position(|patch| patch.id() == patch_id)
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                self.adjust_patch_envelope(patch_index, parameter, direction)?;
                Ok(ReducerEffects::default())
            }
            crate::control::PatchControlId::Engine
            | crate::control::PatchControlId::Output(_)
            | crate::control::PatchControlId::EffectSlot(_)
            | crate::control::PatchControlId::Global(_)
            | crate::control::PatchControlId::MidiInput
            | crate::control::PatchControlId::VoiceLimit => Err(EventRejection::InvalidSelection),
        }
    }

    fn adjust_instrument_parameter(
        &mut self,
        parameter_id: &ParameterId,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch_index = self
            .patches
            .iter()
            .position(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let config = self.patches[patch_index].instrument_config();
        let descriptor = self
            .capabilities
            .descriptor(config.capability_id())
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let spec = descriptor
            .parameter(parameter_id)
            .filter(|spec| {
                spec.patch_interaction() == PatchInteraction::ScalarEdit
                    && spec.update() == crate::synth::ParameterUpdate::Scalar
                    && spec.kind() != ParameterKind::Choice
            })
            .ok_or(EventRejection::InvalidSelection)?;
        let current = config
            .value(parameter_id)
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let value = spec
            .adjusted_scalar_value(current, parameter_adjustment(direction))
            .map_err(map_scalar_adjustment_error)?;
        let candidate = config
            .with_scalar_value(descriptor, parameter_id, value)
            .map_err(map_scalar_adjustment_error)?;
        self.patches[patch_index].set_instrument_config(candidate);
        Ok(())
    }

    /// Changes the incoming MIDI channel the focused Patch subscribes to.
    ///
    /// The channel is an adjacent choice over `0..=15` and refuses at both
    /// ends rather than wrapping, exactly like the output-track row. Multiple
    /// Patches may subscribe to the same channel; an incoming message fans out
    /// to all of them. Nothing else on the Patch is touched: identity,
    /// instrument config, envelope, effect slots, output routing, and the
    /// active graph revision all stay as they were.
    fn adjust_patch_midi_input(&mut self, direction: Direction) -> Result<(), EventRejection> {
        if matches!(direction, Direction::Up | Direction::Down) {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let current = patch.channel().value();
        let channel = if direction == Direction::Right {
            current
                .checked_add(1)
                .filter(|value| *value <= MidiChannel::MAX)
        } else {
            current.checked_sub(1)
        }
        .ok_or(EventRejection::ParameterAtBoundary)?;
        let channel = MidiChannel::new(channel).map_err(|_| EventRejection::ParameterAtBoundary)?;
        self.patches
            .iter_mut()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?
            .set_channel(channel);
        Ok(())
    }

    /// Adjusts the focused Patch's own ceiling on simultaneously sounding notes.
    ///
    /// Bounds and both step sizes come from the canonical
    /// [`VoiceLimit::descriptor`], never from literals here, so the reducer
    /// and every projection move the value by the same amounts. The arithmetic
    /// is integral: a voice count has no fractional position, so it is not
    /// routed through the float scalar path.
    fn adjust_patch_voice_limit(&mut self, direction: Direction) -> Result<(), EventRejection> {
        let descriptor = VoiceLimit::descriptor();
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch = self
            .patches
            .iter_mut()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let current = patch.voice_limit().value();
        let (step, increasing) = match direction {
            Direction::Right => (descriptor.fine_step(), true),
            Direction::Left => (descriptor.fine_step(), false),
            Direction::Up => (descriptor.coarse_step(), true),
            Direction::Down => (descriptor.coarse_step(), false),
        };
        let value = if increasing {
            current.saturating_add(step).min(descriptor.maximum())
        } else {
            current.saturating_sub(step).max(descriptor.minimum())
        };
        if value == current {
            return Err(EventRejection::ParameterAtBoundary);
        }
        patch
            .set_voice_limit(value)
            .map_err(|_| EventRejection::InvalidParameterValue)?;
        Ok(())
    }

    fn adjust_patch_effect(
        &mut self,
        slot_id: EffectSlotId,
        parameter_id: &ParameterId,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch_index = self
            .patches
            .iter()
            .position(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let slot_index = self.patches[patch_index]
            .effect_slots()
            .iter()
            .position(|slot| {
                slot.as_ref()
                    .is_some_and(|effect| effect.slot_id() == slot_id)
            })
            .and_then(|position| EffectSlotIndex::new(position).ok())
            .ok_or(EventRejection::InvalidSelection)?;
        let config = self.patches[patch_index]
            .effect_slot(slot_index)
            .ok_or(EventRejection::InvalidSelection)?;
        let descriptor = self
            .effects
            .descriptor(config.capability_id())
            .ok_or(EventRejection::InvalidEffectConfig)?;
        let spec = descriptor
            .parameter(parameter_id)
            .ok_or(EventRejection::InvalidSelection)?;
        if spec.patch_interaction() != PatchInteraction::ScalarEdit
            || spec.update() != crate::synth::ParameterUpdate::Scalar
        {
            return Err(EventRejection::InvalidSelection);
        }
        let current = config
            .value(parameter_id)
            .ok_or(EventRejection::InvalidEffectConfig)?;
        let value = spec
            .adjusted_scalar_value(current, parameter_adjustment(direction))
            .map_err(map_scalar_adjustment_error)?;
        let candidate = config
            .with_scalar_value(descriptor, parameter_id, value)
            .map_err(|error| match error {
                crate::synth::EffectCapabilityError::Capability(error) => {
                    map_scalar_adjustment_error(error)
                }
                _ => EventRejection::InvalidEffectConfig,
            })?;
        // Replacing an occupant's configuration at its own validated position
        // keeps the identity and every other position untouched; the identity
        // duplicate check cannot fire because the slot id already lives here.
        self.patches[patch_index]
            .set_slot_occupancy(slot_index, Some(candidate))
            .map_err(|_| EventRejection::InvalidEffectConfig)?;
        Ok(())
    }

    fn set_effect_choice(
        &mut self,
        slot_id: EffectSlotId,
        parameter_id: &ParameterId,
        choice_id: String,
    ) -> Result<(), EventRejection> {
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch_index = self
            .patches
            .iter()
            .position(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let slot_index = self.patches[patch_index]
            .effect_slots()
            .iter()
            .position(|slot| {
                slot.as_ref()
                    .is_some_and(|effect| effect.slot_id() == slot_id)
            })
            .and_then(|position| EffectSlotIndex::new(position).ok())
            .ok_or(EventRejection::InvalidSelection)?;
        let config = self.patches[patch_index]
            .effect_slot(slot_index)
            .ok_or(EventRejection::InvalidSelection)?;
        let descriptor = self
            .effects
            .descriptor(config.capability_id())
            .ok_or(EventRejection::InvalidEffectConfig)?;
        let spec = descriptor
            .parameter(parameter_id)
            .filter(|spec| {
                spec.kind() == ParameterKind::Choice
                    && spec.update() == crate::synth::ParameterUpdate::Scalar
            })
            .ok_or(EventRejection::InvalidSelection)?;
        if !spec.choices().iter().any(|choice| choice.id() == choice_id) {
            return Err(EventRejection::InvalidParameterValue);
        }
        let candidate = config
            .with_scalar_value(descriptor, parameter_id, ParameterValue::Choice(choice_id))
            .map_err(|_| EventRejection::InvalidEffectConfig)?;
        self.patches[patch_index]
            .set_slot_occupancy(slot_index, Some(candidate))
            .map_err(|_| EventRejection::InvalidEffectConfig)
    }

    fn set_instrument_scalar_choice(
        &mut self,
        patch_id: PatchId,
        parameter_id: &ParameterId,
        choice_id: String,
    ) -> Result<(), EventRejection> {
        let patch_index = self
            .patches
            .iter()
            .position(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let config = self.patches[patch_index].instrument_config();
        let descriptor = self
            .capabilities
            .descriptor(config.capability_id())
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let spec = descriptor
            .parameter(parameter_id)
            .filter(|spec| {
                spec.kind() == ParameterKind::Choice
                    && spec.update() == crate::synth::ParameterUpdate::Scalar
                    && spec.patch_interaction() == PatchInteraction::ScalarEdit
            })
            .ok_or(EventRejection::InvalidSelection)?;
        if !spec.choices().iter().any(|choice| choice.id() == choice_id) {
            return Err(EventRejection::InvalidParameterValue);
        }
        let candidate = config
            .with_scalar_value(descriptor, parameter_id, ParameterValue::Choice(choice_id))
            .map_err(map_scalar_adjustment_error)?;
        self.patches[patch_index].set_instrument_config(candidate);
        Ok(())
    }

    fn request_parameter_choice(
        &mut self,
        parameter_id: ParameterId,
        direction: Direction,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if matches!(direction, Direction::Up | Direction::Down) {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        if self.engine_selection.is_in_flight() {
            return Err(EventRejection::StructuralEditBusy);
        }
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let source_capability_id = patch.instrument_config().capability_id().clone();
        let descriptor = self
            .capabilities
            .descriptor(&source_capability_id)
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let spec = descriptor
            .parameter(&parameter_id)
            .ok_or(EventRejection::InvalidSelection)?;
        if spec.kind() != ParameterKind::Choice
            || spec.patch_interaction() != PatchInteraction::StructuralChoice
        {
            return Err(EventRejection::InvalidSelection);
        }
        let current_choice = match patch.instrument_config().value(&parameter_id) {
            Some(ParameterValue::Choice(choice)) => choice,
            _ => return Err(EventRejection::InvalidInstrumentConfig),
        };
        let current_index = spec
            .choices()
            .iter()
            .position(|choice| choice.id() == current_choice)
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let target_index = match direction {
            Direction::Left => current_index.checked_sub(1),
            Direction::Right => current_index
                .checked_add(1)
                .filter(|index| *index < spec.choices().len()),
            Direction::Up | Direction::Down => unreachable!("vertical edits were rejected"),
        }
        .ok_or(EventRejection::ParameterAtBoundary)?;
        let request_id = self
            .last_engine_selection_request_id
            .checked_next()
            .map_err(|_| EventRejection::RequestIdOverflow)?;
        let intent = StructuralEditIntent::ReplaceParameterChoice {
            capability_id: source_capability_id.clone(),
            parameter_id,
            choice_id: spec.choices()[target_index].id().to_owned(),
        };
        let status = EngineSelectionStatus::preparing_with_intent(
            self.engine_selection.active_graph_revision(),
            request_id,
            patch_id,
            intent,
            source_capability_id.clone(),
            source_capability_id,
        )
        .map_err(|_| EventRejection::InvalidInstrumentConfig)?;
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::PrepareRequested,
            status
                .correlation()
                .expect("Preparing status always owns correlation"),
        )
        .expect("Preparing correlation has no target revision");
        self.engine_selection = status;
        self.last_engine_selection_request_id = request_id;
        Ok(effect)
    }

    fn request_parameter_choice_to(
        &mut self,
        parameter_id: ParameterId,
        choice_id: String,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if self.engine_selection.is_in_flight() {
            return Err(EventRejection::StructuralEditBusy);
        }
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let source_capability_id = patch.instrument_config().capability_id().clone();
        let descriptor = self
            .capabilities
            .descriptor(&source_capability_id)
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let spec = descriptor
            .parameter(&parameter_id)
            .filter(|spec| {
                spec.kind() == ParameterKind::Choice
                    && spec.patch_interaction() == PatchInteraction::StructuralChoice
                    && spec.choices().iter().any(|choice| choice.id() == choice_id)
            })
            .ok_or(EventRejection::InvalidSelection)?;
        if patch.instrument_config().value(&parameter_id)
            == Some(&ParameterValue::Choice(choice_id.clone()))
        {
            return Err(EventRejection::ParameterAtBoundary);
        }
        debug_assert!(!spec.choices().is_empty());
        let request_id = self
            .last_engine_selection_request_id
            .checked_next()
            .map_err(|_| EventRejection::RequestIdOverflow)?;
        let intent = StructuralEditIntent::ReplaceParameterChoice {
            capability_id: source_capability_id.clone(),
            parameter_id,
            choice_id,
        };
        let status = EngineSelectionStatus::preparing_with_intent(
            self.engine_selection.active_graph_revision(),
            request_id,
            patch_id,
            intent,
            source_capability_id.clone(),
            source_capability_id,
        )
        .map_err(|_| EventRejection::InvalidInstrumentConfig)?;
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::PrepareRequested,
            status
                .correlation()
                .expect("Preparing status always owns correlation"),
        )
        .expect("Preparing correlation has no target revision");
        self.engine_selection = status;
        self.last_engine_selection_request_id = request_id;
        Ok(effect)
    }

    fn navigate_parameter_nonwrapping(
        &mut self,
        all_paths: &[FocusPath],
        forward: bool,
    ) -> Result<(), EventRejection> {
        let MixerControlId::Track {
            track_id,
            parameter,
        } = self.interaction.mixer_control_focus().clone()
        else {
            return Err(EventRejection::InvalidSelection);
        };
        let row = MixerTrackParameter::MAIN
            .iter()
            .position(|candidate| *candidate == parameter)
            .ok_or(EventRejection::InvalidSelection)?;
        let next = if forward {
            row.checked_add(1)
                .filter(|index| *index < MixerTrackParameter::MAIN.len())
        } else {
            row.checked_sub(1)
        }
        .ok_or(EventRejection::ActionUnavailableInContext)?;
        let path = FocusPath::mixer_track(track_id, MixerTrackParameter::MAIN[next]);
        if !all_paths.contains(&path) {
            return Err(EventRejection::InvalidSelection);
        }
        self.interaction
            .set_active_main(path)
            .map_err(|_| EventRejection::InvalidSelection)
    }

    fn navigate_side_nonwrapping(
        &mut self,
        paths: &[FocusPath],
        forward: bool,
    ) -> Result<(), EventRejection> {
        let current = paths
            .iter()
            .position(|path| path == self.interaction.focus_path())
            .ok_or(EventRejection::InvalidSelection)?;
        let next = if forward {
            current.checked_add(1).filter(|index| *index < paths.len())
        } else {
            current.checked_sub(1)
        }
        .ok_or(EventRejection::ActionUnavailableInContext)?;
        self.interaction.active_focus = paths[next].clone();
        Ok(())
    }

    fn adjust(&mut self, direction: Direction) -> Result<ReducerEffects, EventRejection> {
        if !matches!(
            self.interaction.active_surface(),
            SurfaceId::MixerMain | SurfaceId::MixerInspector
        ) {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        let SemanticControlId::Mixer(control) = self.interaction.focus_path().control_id() else {
            return Err(EventRejection::InvalidSelection);
        };
        match control.clone() {
            MixerControlId::Track {
                track_id,
                parameter,
            } => {
                self.adjust_track(track_id, parameter, direction)?;
                Ok(ReducerEffects::default())
            }
            MixerControlId::Send { track_id, bus } => {
                self.adjust_send(track_id, bus, direction)?;
                Ok(ReducerEffects::default())
            }
            MixerControlId::ReturnOccupancy { bus } => {
                let effect = self.request_return_occupancy_choice(bus, direction)?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(effect),
                })
            }
            MixerControlId::ReturnLevel { bus } => {
                self.adjust_return_level(bus, direction)?;
                Ok(ReducerEffects::default())
            }
            MixerControlId::ReturnEffect { bus, parameter } => {
                self.adjust_return_effect(bus, &parameter, direction)?;
                Ok(ReducerEffects::default())
            }
            MixerControlId::Global { parameter } => {
                self.adjust_global(parameter, direction)?;
                Ok(ReducerEffects::default())
            }
        }
    }

    /// Fine/coarse edit of one indexed send `(track, bus)` through the one
    /// shared send descriptor.
    fn adjust_send(
        &mut self,
        track_id: MixerTrackId,
        bus: BusId,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let current = *self.mixer.track(track_id);
        let value = adjusted_value(
            current.send(bus),
            BUS_SEND_DESCRIPTOR.minimum(),
            BUS_SEND_DESCRIPTOR.maximum(),
            direction,
            BUS_SEND_DESCRIPTOR.fine_step(),
            BUS_SEND_DESCRIPTOR.coarse_step(),
        )?;
        let updated = current
            .with_send(bus, value)
            .map_err(|_| EventRejection::InvalidParameterValue)?;
        self.mixer.set_track(track_id, updated);
        Ok(())
    }

    /// Fine/coarse edit of one return-owned level through the shared
    /// return-level descriptor.
    fn adjust_return_level(
        &mut self,
        bus: BusId,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let value = adjusted_value(
            self.returns.bus_return(bus).return_level(),
            RETURN_LEVEL_DESCRIPTOR.minimum(),
            RETURN_LEVEL_DESCRIPTOR.maximum(),
            direction,
            RETURN_LEVEL_DESCRIPTOR.fine_step(),
            RETURN_LEVEL_DESCRIPTOR.coarse_step(),
        )?;
        self.returns
            .set_return_level(bus, value)
            .map_err(|_| EventRejection::InvalidParameterValue)
    }

    /// Fine/coarse edit of one occupying registry entry's descriptor scalar.
    fn adjust_return_effect(
        &mut self,
        bus: BusId,
        parameter_id: &ParameterId,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let config = self
            .returns
            .bus_return(bus)
            .effect()
            .ok_or(EventRejection::InvalidSelection)?
            .clone();
        let descriptor = self
            .effects
            .descriptor(config.capability_id())
            .ok_or(EventRejection::InvalidEffectConfig)?;
        let spec = descriptor
            .parameter(parameter_id)
            .ok_or(EventRejection::InvalidSelection)?;
        if spec.patch_interaction() != PatchInteraction::ScalarEdit
            || spec.update() != crate::synth::ParameterUpdate::Scalar
        {
            return Err(EventRejection::InvalidSelection);
        }
        let current = config
            .value(parameter_id)
            .ok_or(EventRejection::InvalidEffectConfig)?;
        let value = spec
            .adjusted_scalar_value(current, parameter_adjustment(direction))
            .map_err(map_scalar_adjustment_error)?;
        let candidate = config
            .with_scalar_value(descriptor, parameter_id, value)
            .map_err(|error| match error {
                crate::synth::EffectCapabilityError::Capability(error) => {
                    map_scalar_adjustment_error(error)
                }
                _ => EventRejection::InvalidEffectConfig,
            })?;
        self.returns
            .replace_occupant_values(bus, candidate)
            .map_err(|_| EventRejection::InvalidParameterValue)
    }

    /// Resolves the adjacent nonwrapping occupancy choice — empty, then each
    /// installed registry entry in declared order — shared by Patch effect
    /// slots and bus returns. Edit+Up/Down is unavailable on occupancy rows,
    /// exactly as on the engine row, and a choice endpoint is a boundary.
    fn adjacent_occupancy_entry(
        &self,
        current: Option<&EffectCapabilityId>,
        direction: Direction,
    ) -> Result<Option<EffectCapabilityId>, EventRejection> {
        if matches!(direction, Direction::Up | Direction::Down) {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        let entries = self.effects.descriptors();
        // Choice index 0 is empty; entry N sits at index N + 1.
        let current_index = match current {
            None => 0,
            Some(entry_id) => {
                entries
                    .iter()
                    .position(|descriptor| descriptor.id() == entry_id)
                    .ok_or(EventRejection::InvalidEffectConfig)?
                    + 1
            }
        };
        let target_index = match direction {
            Direction::Left => current_index.checked_sub(1),
            Direction::Right => current_index
                .checked_add(1)
                .filter(|index| *index <= entries.len()),
            Direction::Up | Direction::Down => unreachable!("vertical edits were rejected"),
        }
        .ok_or(EventRejection::ParameterAtBoundary)?;
        Ok(match target_index {
            0 => None,
            index => Some(entries[index - 1].id().clone()),
        })
    }

    /// Adjacent-choice occupancy edit on one bus return's occupancy row,
    /// entering the same correlated one-in-flight lifecycle as every other
    /// structural edit.
    fn request_return_occupancy_choice(
        &mut self,
        bus: BusId,
        direction: Direction,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if matches!(direction, Direction::Up | Direction::Down) {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        if self.engine_selection.is_in_flight() {
            return Err(EventRejection::StructuralEditBusy);
        }
        let entry = self.adjacent_occupancy_entry(
            self.returns
                .bus_return(bus)
                .effect()
                .map(crate::synth::PostEffectConfig::capability_id),
            direction,
        )?;
        self.request_topology_change(StructuralEditIntent::SetReturnOccupancy { bus, entry })
    }

    /// Adjacent-choice occupancy edit on one Patch effect slot's occupancy
    /// row, entering the same correlated one-in-flight lifecycle.
    fn request_slot_occupancy_choice(
        &mut self,
        slot: EffectSlotIndex,
        direction: Direction,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if matches!(direction, Direction::Up | Direction::Down) {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        if self.engine_selection.is_in_flight() {
            return Err(EventRejection::StructuralEditBusy);
        }
        let patch_id = self
            .interaction
            .patch_focus()
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let entry = self.adjacent_occupancy_entry(
            patch
                .effect_slot(slot)
                .map(crate::synth::PostEffectConfig::capability_id),
            direction,
        )?;
        self.request_topology_change(StructuralEditIntent::SetSlotOccupancy {
            patch_id,
            slot,
            entry,
        })
    }

    fn adjust_track(
        &mut self,
        track_id: crate::mixer::mixer_track_id::MixerTrackId,
        parameter: MixerTrackParameter,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let current = *self.mixer.track(track_id);
        let updated = if parameter.descriptor().kind() == MixerTrackParameterKind::Toggle {
            current
                .toggled(parameter)
                .map_err(|_| EventRejection::InvalidParameterValue)?
        } else {
            let descriptor = parameter.descriptor();
            let value = adjusted_value(
                current
                    .scalar_value(parameter)
                    .ok_or(EventRejection::InvalidSelection)?,
                descriptor.minimum(),
                descriptor.maximum(),
                direction,
                descriptor.fine_step(),
                descriptor.coarse_step(),
            )?;
            current
                .with_scalar_value(parameter, value)
                .map_err(|_| EventRejection::InvalidParameterValue)?
        };
        self.mixer.set_track(track_id, updated);
        Ok(())
    }

    fn adjust_patch_envelope(
        &mut self,
        patch_index: usize,
        parameter: VoiceEnvelopeParameter,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let descriptor = parameter.descriptor();
        let patch = self
            .patches
            .get_mut(patch_index)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let envelope = *patch.envelope();
        let value = adjusted_value(
            envelope.value(parameter),
            descriptor.minimum(),
            descriptor.maximum(),
            direction,
            descriptor.fine_step(),
            descriptor.coarse_step(),
        )?;
        let updated = envelope
            .with_value(parameter, value)
            .map_err(|_| EventRejection::InvalidParameterValue)?;
        patch.set_envelope(updated);
        Ok(())
    }

    fn adjust_global(
        &mut self,
        parameter: GlobalParameter,
        direction: Direction,
    ) -> Result<(), EventRejection> {
        let descriptor = parameter.descriptor();
        let value = adjusted_value(
            self.global.master_gain_db(),
            descriptor.minimum(),
            descriptor.maximum(),
            direction,
            descriptor.fine_step(),
            descriptor.coarse_step(),
        )?;
        self.global = self
            .global
            .with_master_gain_db(value)
            .map_err(|_| EventRejection::InvalidParameterValue)?;
        Ok(())
    }

    fn repair_semantic_paths(
        &mut self,
        old_patch_order: &[FocusPath],
        old_mixer_order: &[FocusPath],
    ) -> Result<(), EventRejection> {
        let patch_id = old_patch_order
            .first()
            .and_then(FocusPath::patch_id)
            .ok_or(EventRejection::InvalidSelection)?;
        let resolver = SemanticResolver::new(self);
        let new_patch_order = resolver.patch_main_paths(patch_id)?;
        let new_mixer_order = resolver.mixer_main_paths()?;

        let repair = |path: &FocusPath| -> Result<FocusPath, EventRejection> {
            if resolver.resolves(path) {
                return Ok(path.clone());
            }
            let (old_order, new_order) = match path.surface() {
                SurfaceId::PatchMain => (old_patch_order, new_patch_order.as_slice()),
                SurfaceId::MixerMain => (old_mixer_order, new_mixer_order.as_slice()),
                // Only main paths are repaired here: this repairs remembered
                // roots and one return *origin*, all of which are main by
                // construction. A subordinate surface's own focus is repaired
                // where that surface's order is resolved.
                SurfaceId::PatchUtility
                | SurfaceId::PatchDetail
                | SurfaceId::PatchChoice
                | SurfaceId::SampleBrowser
                | SurfaceId::MixerInspector => return Err(EventRejection::InvalidSelection),
            };
            SemanticResolver::recover(path, old_order, new_order)
                .ok_or(EventRejection::InvalidSelection)
        };

        let repaired_patch = self
            .interaction
            .remembered_patch_main()
            .map(&repair)
            .transpose()?;
        let repaired_mixer = repair(self.interaction.remembered_mixer_main())?;
        let repaired_return_origin = self
            .interaction
            .return_path()
            .map(|path| repair(path.origin()))
            .transpose()?;

        if let Some(path) = repaired_patch {
            self.interaction.replace_remembered_patch_main(path);
        }
        self.interaction
            .replace_remembered_mixer_main(repaired_mixer);
        if let Some(origin) = repaired_return_origin {
            self.interaction
                .replace_return_origin(origin)
                .map_err(|_| EventRejection::InvalidSelection)?;
        }
        self.leave_stale_detail_surface();
        Ok(())
    }

    fn set_patch_overview_origin_enabled(
        &mut self,
        patch_id: PatchId,
        control: PatchControlId,
        enabled: bool,
    ) -> Result<(), EventRejection> {
        if !self.patches.iter().any(|patch| patch.id() == patch_id)
            || !matches!(
                control,
                PatchControlId::Engine | PatchControlId::EffectSlot(_)
            )
        {
            return Err(EventRejection::InvalidSelection);
        }
        let key = (patch_id, control.clone());
        if self.patch_overview_origin_enabled(patch_id, &control) == enabled {
            return Err(EventRejection::InvalidSelection);
        }
        if enabled {
            self.disabled_patch_overview_origins.remove(&key);
            self.focus_repair_status = None;
            return Ok(());
        }

        let old_patch_order = SemanticResolver::new(self).patch_main_paths(patch_id)?;
        if old_patch_order.len() <= 1 {
            return Err(EventRejection::InvalidSelection);
        }
        let old_mixer_order = SemanticResolver::new(self).mixer_main_paths()?;
        let removed_origin = FocusPath::patch_main(patch_id, None, control);
        let repairs_return = self
            .interaction
            .return_path()
            .is_some_and(|path| path.origin() == &removed_origin);
        let repairs_remembered = self.interaction.remembered_patch_main() == Some(&removed_origin);

        self.disabled_patch_overview_origins.insert(key);
        self.repair_semantic_paths(&old_patch_order, &old_mixer_order)?;

        let replacement_origin = if repairs_return {
            self.interaction
                .return_path()
                .map(|path| path.origin().clone())
        } else if repairs_remembered {
            self.interaction.remembered_patch_main().cloned()
        } else {
            None
        };
        self.focus_repair_status = replacement_origin.map(|replacement_origin| FocusRepairStatus {
            removed_origin,
            replacement_origin,
        });
        Ok(())
    }

    /// Leaves an open detail surface whose subject the structural commit just
    /// invalidated.
    ///
    /// From `valueObject.Control.PatchDetailSubject`: a subject whose
    /// capability leaves the registry, or whose slot is cleared, is *not*
    /// repaired into a neighbouring subject — the surface is left through the
    /// deterministic focus resolver back to its origin, because silently
    /// retargeting a detail view would show one capability's values under
    /// another's title. The origin restored here is the one
    /// [`Self::repair_semantic_paths`] has already repaired against the new
    /// schema, so leaving never lands on a row the destination cannot host.
    ///
    /// A detail row that stopped resolving while its subject stayed live — a
    /// committed parameter choice that hides rows — leaves for the same
    /// reason: the entry can no longer be shown as it was opened, and the
    /// exact remembered origin is where the declaration says leaving lands.
    fn leave_stale_detail_surface(&mut self) {
        let Some(subject) = self.interaction.detail_subject().cloned() else {
            return;
        };
        let resolver = SemanticResolver::new(self);
        let live = self
            .interaction
            .focus_path()
            .patch_id()
            .is_some_and(|patch_id| resolver.detail_subject_is_live(patch_id, &subject))
            && resolver.resolves(self.interaction.focus_path());
        if !live {
            self.interaction.leave_detail_to_origin();
        }
    }

    /// Captures the current MIXER Inspector focus order for the selected
    /// track, or `None` when no track selection resolves yet.
    fn mixer_inspector_order(&self) -> Option<Vec<FocusPath>> {
        let track_id = self
            .interaction
            .remembered_mixer_main()
            .control_id()
            .as_mixer_track_id()?;
        SemanticResolver::new(self)
            .mixer_inspector_paths(track_id)
            .ok()
    }

    /// Repairs an active MIXER Inspector focus after a committed return
    /// occupancy change through the one deterministic next-before-previous
    /// recovery rule. Rows on every other surface are unaffected by return
    /// occupancy and keep their exact identity.
    fn repair_inspector_focus(
        &mut self,
        old_order: Option<&[FocusPath]>,
    ) -> Result<(), EventRejection> {
        if self.interaction.active_surface() != SurfaceId::MixerInspector {
            return Ok(());
        }
        let resolver = SemanticResolver::new(self);
        if resolver.resolves(self.interaction.focus_path()) {
            return Ok(());
        }
        let old_order = old_order.ok_or(EventRejection::InvalidSelection)?;
        let track_id = self
            .interaction
            .remembered_mixer_main()
            .control_id()
            .as_mixer_track_id()
            .ok_or(EventRejection::InvalidSelection)?;
        let new_order = resolver.mixer_inspector_paths(track_id)?;
        let repaired =
            SemanticResolver::recover(self.interaction.focus_path(), old_order, &new_order)
                .ok_or(EventRejection::InvalidSelection)?;
        self.interaction.active_focus = repaired;
        Ok(())
    }
}

fn parameter_adjustment(direction: Direction) -> ParameterAdjustment {
    match direction {
        Direction::Left => ParameterAdjustment::FineDecrease,
        Direction::Right => ParameterAdjustment::FineIncrease,
        Direction::Down => ParameterAdjustment::CoarseDecrease,
        Direction::Up => ParameterAdjustment::CoarseIncrease,
    }
}

fn map_scalar_adjustment_error(error: CapabilityError) -> EventRejection {
    match error {
        CapabilityError::ScalarValueAtBoundary(_) => EventRejection::ParameterAtBoundary,
        _ => EventRejection::InvalidParameterValue,
    }
}

fn unavailable_failure(failure: EngineSelectionFailure) -> bool {
    matches!(
        failure,
        EngineSelectionFailure::WorkerUnavailable
            | EngineSelectionFailure::PresetUnavailable
            | EngineSelectionFailure::AssetUnavailable
            | EngineSelectionFailure::PreparerMissing
    )
}

fn adjusted_value(
    current: f32,
    minimum: f32,
    maximum: f32,
    direction: Direction,
    fine_step: f32,
    coarse_step: f32,
) -> Result<f32, EventRejection> {
    let scale = decimal_scale(fine_step);
    let current_units = (current * scale).round();
    let fine_units = (fine_step * scale).round();
    let coarse_units = (coarse_step * scale).round();
    let delta_units = match direction {
        Direction::Left => -fine_units,
        Direction::Right => fine_units,
        Direction::Down => -coarse_units,
        Direction::Up => coarse_units,
    };
    let adjusted = ((current_units + delta_units) / scale).clamp(minimum, maximum);
    if adjusted == current {
        Err(EventRejection::ParameterAtBoundary)
    } else {
        Ok(adjusted)
    }
}

fn candidate_matches_intent(
    source: &crate::synth::InstrumentConfig,
    candidate: &crate::synth::InstrumentConfig,
    intent: &StructuralEditIntent,
) -> bool {
    match intent {
        StructuralEditIntent::SetSlotOccupancy { .. }
        | StructuralEditIntent::SetReturnOccupancy { .. } => false,
        StructuralEditIntent::ReplaceCapability {
            target_capability_id,
        } => {
            source.capability_id() != target_capability_id
                && candidate.capability_id() == target_capability_id
        }
        StructuralEditIntent::ReplaceParameterChoice {
            capability_id,
            parameter_id,
            choice_id,
        } => {
            source.capability_id() == capability_id
                && candidate.capability_id() == capability_id
                && source.asset_references() == candidate.asset_references()
                && source.values().len() == candidate.values().len()
                && matches!(
                    candidate.value(parameter_id),
                    Some(ParameterValue::Choice(value)) if value == choice_id
                )
                && !matches!(
                    source.value(parameter_id),
                    Some(ParameterValue::Choice(value)) if value == choice_id
                )
                && candidate
                    .values()
                    .iter()
                    .zip(source.values())
                    .all(|(next, prior)| {
                        next.parameter_id() == prior.parameter_id()
                            && (next.parameter_id() == parameter_id || next == prior)
                    })
        }
        StructuralEditIntent::ReplaceAsset {
            capability_id,
            parameter_id,
            reference,
        } => {
            source.capability_id() == capability_id
                && candidate.capability_id() == capability_id
                && source.values() == candidate.values()
                && source.asset_references().len() == candidate.asset_references().len()
                && source.asset_reference(parameter_id) != Some(reference)
                && candidate.asset_reference(parameter_id) == Some(reference)
                && candidate
                    .asset_references()
                    .iter()
                    .zip(source.asset_references())
                    .all(|(next, prior)| {
                        next.parameter_id() == prior.parameter_id()
                            && (next.parameter_id() == parameter_id || next == prior)
                    })
        }
        StructuralEditIntent::PrepareAudition {
            capability_id,
            parameter_id,
            reference,
        } => {
            source.capability_id() == capability_id
                && candidate.capability_id() == capability_id
                && source.values() == candidate.values()
                && source.asset_references().len() == candidate.asset_references().len()
                && candidate.asset_reference(parameter_id) == Some(reference)
                && candidate
                    .asset_references()
                    .iter()
                    .zip(source.asset_references())
                    .all(|(next, prior)| {
                        next.parameter_id() == prior.parameter_id()
                            && (next.parameter_id() == parameter_id || next == prior)
                    })
        }
    }
}

fn decimal_scale(step: f32) -> f32 {
    let mut scale = 1.0;
    while scale < 1_000_000.0 && (step * scale - (step * scale).round()).abs() > f32::EPSILON {
        scale *= 10.0;
    }
    scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::braids_capability::{BraidsCapability, BRAIDS_CAPABILITY_ID};
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID, SOUNDFONT_PRESET_PARAMETER_ID,
    };
    use crate::control::{InteractionMode, PatchControlId, PatchDetailSubject, StateProjector};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_parameters::GlobalParameter;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::{DescriptorDefaultConfigFactory, InstrumentCapabilityProvider, ParameterId};
    use crate::testing::automatic_midi_test::create_soundfont_config;

    fn provider() -> HiDefSoundFontCapability {
        crate::adapter::production_instruments::production_soundfont_capability().unwrap()
    }

    fn registry() -> CapabilityRegistry {
        provider().registry().unwrap()
    }

    fn global_parameters() -> GlobalParameters {
        GlobalParameters::new(0.0).unwrap()
    }

    fn patch(id: u32, gain_db: f32) -> Patch {
        patch_on_channel(id, gain_db, (id - 1) as u8)
    }

    fn patch_on_channel(id: u32, gain_db: f32, channel: u8) -> Patch {
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider(),
                SoundFontInstrument::new(0, id as u8, false).unwrap(),
            )
            .unwrap(),
            MidiChannel::new(channel).unwrap(),
            PatchOutput::new(MixerTrackId::new(channel).unwrap(), gain_db).unwrap(),
        )
    }

    fn installed_state() -> AppState {
        let mut state = AppState::new(registry(), global_parameters());
        state
            .apply(AppEvent::InstallPatches(vec![
                patch(1, 0.0),
                patch(2, -3.0),
            ]))
            .unwrap();
        state
    }

    /// One installed patch whose chain occupies only slot 1: slot 0 is empty
    /// and stays empty. This is exactly the shape a compacting view would
    /// silently squeeze down to position 0.
    fn gapped_effects_state() -> AppState {
        let mut state = AppState::new_with_effects(
            registry(),
            crate::adapter::production_effects::production_effect_registry().unwrap(),
            global_parameters(),
        );
        let mut gapped = patch(1, 0.0);
        gapped
            .set_slot_occupancy(
                EffectSlotIndex::new(1).unwrap(),
                Some(
                    crate::adapter::production_effects::production_chorus_config(
                        EffectSlotId::new(2).unwrap(),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        state.apply(AppEvent::InstallPatches(vec![gapped])).unwrap();
        state
    }

    #[test]
    fn install_accepts_a_gapped_chain_and_keeps_every_position() {
        let state = gapped_effects_state();
        let slots = state.patches()[0].effect_slots();
        assert!(slots[0].is_none(), "slot 0 must stay empty after install");
        assert_eq!(
            slots[1].as_ref().map(|config| config.slot_id().value()),
            Some(2),
            "the occupant must keep its position and stable identity"
        );
        assert!(slots[2].is_none());
    }

    #[test]
    fn adjust_edits_a_gapped_occupant_at_its_true_position() {
        let mut state = gapped_effects_state();
        let amount =
            ParameterId::new(crate::adapter::chorus_capability::CHORUS_AMOUNT_PARAMETER_ID)
                .unwrap();
        let before = state.patches()[0].effect_slots()[1]
            .as_ref()
            .and_then(|config| config.value(&amount))
            .cloned()
            .expect("chorus declares an amount value");

        state
            .adjust_patch_effect(EffectSlotId::new(2).unwrap(), &amount, Direction::Up)
            .unwrap();

        let slots = state.patches()[0].effect_slots();
        assert!(
            slots[0].is_none(),
            "a scalar edit must never compact the occupant into slot 0"
        );
        let occupant = slots[1]
            .as_ref()
            .expect("the occupant stays at position 1 after the edit");
        assert_eq!(occupant.slot_id().value(), 2);
        assert_ne!(occupant.value(&amount), Some(&before));
        assert!(slots[2].is_none());
    }

    #[test]
    fn validate_effect_slots_accepts_gaps_and_rejects_duplicate_identities() {
        let effects = crate::adapter::production_effects::production_effect_registry().unwrap();
        let occupant = |slot: u16| {
            crate::adapter::production_effects::production_chorus_config(
                EffectSlotId::new(slot).unwrap(),
            )
            .unwrap()
        };

        validate_effect_slots(&effects, &[Some(occupant(1)), None, Some(occupant(3))])
            .expect("an interior gap is a legal chain shape");
        assert!(matches!(
            validate_effect_slots(&effects, &[Some(occupant(1)), None, Some(occupant(1))]),
            Err(crate::synth::EffectCapabilityError::DuplicateSlot(_))
        ));
    }

    fn mixed_registry() -> CapabilityRegistry {
        CapabilityRegistry::new(vec![
            provider().descriptor(),
            BraidsCapability::new().unwrap().descriptor(),
        ])
        .unwrap()
    }

    fn descriptor_default_config(capability_id: &str) -> crate::synth::InstrumentConfig {
        let providers: Vec<Box<dyn InstrumentCapabilityProvider>> = vec![
            Box::new(provider()),
            Box::new(BraidsCapability::new().unwrap()),
        ];
        let registry = CapabilityRegistry::new(
            providers
                .iter()
                .map(|provider| provider.descriptor())
                .collect(),
        )
        .unwrap();
        DescriptorDefaultConfigFactory::new(registry, providers)
            .create(&crate::synth::CapabilityId::new(capability_id).unwrap())
            .unwrap()
    }

    fn mixed_state() -> AppState {
        let mut state = AppState::new(mixed_registry(), global_parameters());
        let braids_patch = Patch::new(
            PatchId::new(2).unwrap(),
            "Patch 2".to_owned(),
            descriptor_default_config(BRAIDS_CAPABILITY_ID),
            MidiChannel::new(1).unwrap(),
            PatchOutput::new(MixerTrackId::new(1).unwrap(), -3.0).unwrap(),
        );
        state
            .apply(AppEvent::InstallPatches(vec![patch(1, -1.0), braids_patch]))
            .unwrap();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
    }

    fn focus_instrument_detail_control(state: &mut AppState, target: PatchControlId) {
        if state.context() != TopLevelContext::Patch {
            state
                .apply(AppEvent::SelectContext(TopLevelContext::Patch))
                .unwrap();
        }
        assert_eq!(
            state.interaction().patch_control_focus(),
            Some(PatchControlId::Engine),
            "instrument Detail entry starts from the Overview Engine control"
        );
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        let paths = SemanticResolver::new(state)
            .ordered_paths(SurfaceId::PatchDetail)
            .unwrap();
        let target_index = paths
            .iter()
            .position(|path| path.control_id() == &SemanticControlId::Patch(target.clone()))
            .expect("the instrument Detail descriptor contains the target control");
        for _ in 0..target_index {
            state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        }
        assert_eq!(state.interaction().patch_control_focus(), Some(target));
    }

    fn prepared_event(
        state: &AppState,
        target_graph_revision: GraphRevision,
        candidate_config: crate::synth::InstrumentConfig,
    ) -> AppEvent {
        let correlation = state.engine_selection().correlation().unwrap();
        AppEvent::EnginePrepared {
            request_id: correlation.request_id(),
            patch_id: correlation.patch_id().unwrap(),
            intent: correlation.intent().clone(),
            source_capability_id: correlation.source_capability_id().unwrap().clone(),
            target_capability_id: correlation.target_capability_id().unwrap().clone(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision,
            candidate_config,
            prepared_visualization: None,
        }
    }

    fn advance_engine_to_preparing(state: &mut AppState) {
        let request_id = state
            .engine_selection()
            .correlation()
            .expect("an accepted structural request owns correlation")
            .request_id();
        state
            .apply(AppEvent::EngineSelectionLifecycleAdvanced {
                request_id,
                lifecycle: EngineSelectionStatusKind::Validating,
            })
            .unwrap();
        state
            .apply(AppEvent::EngineSelectionLifecycleAdvanced {
                request_id,
                lifecycle: EngineSelectionStatusKind::Preparing,
            })
            .unwrap();
    }

    fn failed_event(
        state: &AppState,
        target_graph_revision: GraphRevision,
        failure: EngineSelectionFailure,
    ) -> AppEvent {
        let correlation = state.engine_selection().correlation().unwrap();
        AppEvent::EnginePreparationFailed {
            request_id: correlation.request_id(),
            patch_id: correlation.patch_id().unwrap(),
            intent: correlation.intent().clone(),
            source_capability_id: correlation.source_capability_id().unwrap().clone(),
            target_capability_id: correlation.target_capability_id().unwrap().clone(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision,
            failure,
        }
    }

    #[test]
    fn app_state_adjustment_uses_fine_and_coarse_directions() {
        assert_eq!(
            adjusted_value(0.0, -1.0, 1.0, Direction::Right, 0.01, 0.1),
            Ok(0.01)
        );
        assert_eq!(
            adjusted_value(0.0, -1.0, 1.0, Direction::Up, 0.01, 0.1),
            Ok(0.1)
        );
        assert_eq!(
            adjusted_value(0.0, -1.0, 1.0, Direction::Left, 0.01, 0.1),
            Ok(-0.01)
        );
        assert_eq!(
            adjusted_value(0.0, -1.0, 1.0, Direction::Down, 0.01, 0.1),
            Ok(-0.1)
        );
    }

    #[test]
    fn app_state_adjustment_rejects_a_clamped_no_op() {
        assert_eq!(
            adjusted_value(1.0, -1.0, 1.0, Direction::Right, 0.01, 0.1),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(
            adjusted_value(-1.0, -1.0, 1.0, Direction::Down, 0.01, 0.1),
            Err(EventRejection::ParameterAtBoundary)
        );
    }

    #[test]
    fn rejection_descriptor_is_unique_and_reducer_table_exercises_its_partition() {
        let descriptor = EventRejection::surface_descriptor();
        assert_eq!(descriptor.len(), 16);
        for (index, entry) in descriptor.iter().enumerate() {
            assert!(!descriptor[..index].iter().any(|prior| prior.rejection()
                == entry.rejection()
                || prior.name() == entry.name()));
        }
        for entry in descriptor {
            let expected_reachability = match entry.rejection() {
                EventRejection::InstallationClosed
                | EventRejection::TooManyPatches
                | EventRejection::InvalidInstrumentConfig
                | EventRejection::UnknownPatch
                | EventRejection::ParameterAtBoundary
                | EventRejection::ActionUnavailableInContext
                | EventRejection::EngineSelectionUnavailable
                | EventRejection::StructuralEditBusy
                | EventRejection::StaleEngineSelection
                | EventRejection::MismatchedEngineSelection => EventRejectionReachability::Scene,
                EventRejection::NoPatchesInstalled
                | EventRejection::InvalidEffectConfig
                | EventRejection::InvalidSelection
                | EventRejection::InvalidParameterValue
                | EventRejection::RequestIdOverflow
                | EventRejection::GenerationOverflow => EventRejectionReachability::ReducerTable,
            };
            assert_eq!(entry.reachability(), expected_reachability);
        }

        let expected = [
            EventRejection::TooManyPatches,
            EventRejection::InvalidInstrumentConfig,
            EventRejection::InvalidEffectConfig,
            EventRejection::NoPatchesInstalled,
            EventRejection::InvalidSelection,
            EventRejection::InvalidParameterValue,
            EventRejection::RequestIdOverflow,
            EventRejection::GenerationOverflow,
        ];
        let state = installed_state();
        assert_eq!(
            exercise_reducer_table_rejections(
                state.capabilities(),
                state.patches()[0].instrument_config(),
            )
            .as_slice(),
            expected.as_slice()
        );
    }

    #[test]
    fn app_state_selection_is_read_only_and_typed() {
        let patch = Selection::patch(2);
        let global = Selection::global();

        assert_eq!(patch.section(), SelectionSection::Patch);
        assert_eq!(patch.patch_index(), 2);
        assert_eq!(patch.parameter_index(), 0);
        assert_eq!(global.section(), SelectionSection::Global);
        assert_eq!(global.parameter_index(), 0);
    }

    #[test]
    fn app_state_context_defaults_to_mixer_and_installation_sets_stable_focus() {
        let mut state = AppState::new(registry(), global_parameters());
        assert_eq!(state.context(), TopLevelContext::Mixer);
        assert_eq!(state.interaction().patch_focus(), None);
        assert_eq!(state.selection(), Selection::patch(0));

        state
            .apply(AppEvent::InstallPatches(vec![
                patch(2, -3.0),
                patch(1, 0.0),
            ]))
            .unwrap();
        assert_eq!(state.context(), TopLevelContext::Mixer);
        assert_eq!(
            state.interaction().patch_focus(),
            Some(PatchId::new(2).unwrap())
        );
        assert_eq!(state.selection(), Selection::patch(0));
    }

    #[test]
    fn direct_and_repeated_context_selection_preserves_independent_focus() {
        let mut state = installed_state();
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        let mixer_selection = state.selection();
        let patch_focus = state.interaction().patch_focus();
        let generation = state.generation();

        let first = state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        assert_eq!(first.audio_command(), None);
        assert_eq!(first.accepted().generation(), generation + 1);
        assert_eq!(state.context(), TopLevelContext::Patch);
        assert_eq!(state.selection(), mixer_selection);
        assert_eq!(state.interaction().patch_focus(), patch_focus);

        let repeated = state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        assert_eq!(repeated.accepted().generation(), generation + 2);
        assert_eq!(repeated.audio_command(), None);
        assert_eq!(state.selection(), mixer_selection);
        assert_eq!(state.interaction().patch_focus(), patch_focus);

        state
            .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
            .unwrap();
        assert_eq!(state.context(), TopLevelContext::Mixer);
        assert_eq!(state.selection(), mixer_selection);
    }

    #[test]
    fn patch_selection_before_installation_and_patch_actions_reject_transactionally() {
        let mut empty = AppState::new(registry(), global_parameters());
        let initial = empty.clone();
        assert_eq!(
            empty.apply(AppEvent::SelectContext(TopLevelContext::Patch)),
            Err(EventRejection::NoPatchesInstalled)
        );
        assert_eq!(empty, initial);
        empty.apply(AppEvent::Navigate(Direction::Down)).unwrap();

        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        let before = state.clone();
        assert_eq!(
            state.apply(AppEvent::Navigate(Direction::Left)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, before);
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::EngineSelectionUnavailable)
        );
        assert_eq!(state, before);
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(
            state.interaction().patch_control_focus(),
            Some(crate::control::PatchControlId::EffectSlot(
                EffectSlotIndex::ALL[0]
            ))
        );
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
            .unwrap();
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(state.context(), TopLevelContext::Mixer);
    }

    #[test]
    fn patch_navigation_and_adjustment_route_by_focused_control() {
        let mut state = mixed_state();
        let engine = state.clone();

        assert_eq!(
            state.apply(AppEvent::Navigate(Direction::Left)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, engine);
        assert_eq!(
            state.apply(AppEvent::Navigate(Direction::Up)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, engine);

        focus_instrument_detail_control(
            &mut state,
            PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds),
        );

        for (direction, expected) in [
            (Direction::Right, 1.0),
            (Direction::Up, 101.0),
            (Direction::Left, 100.0),
            (Direction::Down, 0.0),
        ] {
            let outcome = state.apply(AppEvent::Adjust(direction)).unwrap();
            assert_eq!(outcome.audio_command(), None);
            assert_eq!(outcome.engine_selection_effect(), None);
            assert_eq!(
                state.patches()[0].envelope().attack_milliseconds(),
                expected
            );
        }

        let boundary = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Down)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, boundary);
    }

    /// The fixture installs one Patch per MIDI part, so without a selection
    /// gesture every instrument after the first is unreachable. This proves
    /// the focused Patch actually moves through the production reducer.
    #[test]
    fn select_patch_moves_the_focused_patch_through_the_reducer_without_wrapping() {
        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();

        let first = state.interaction.patch_focus().unwrap();
        assert_eq!(first, PatchId::new(1).unwrap());

        // Left at the first position is an unchanged rejection, not a wrap
        // onto the last Patch.
        assert_eq!(
            state.apply(AppEvent::SelectPatch(Direction::Left)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state.interaction.patch_focus().unwrap(), first);

        state
            .apply(AppEvent::SelectPatch(Direction::Right))
            .unwrap();
        let second = state.interaction.patch_focus().unwrap();
        assert_eq!(second, PatchId::new(2).unwrap());
        assert_ne!(second, first);

        // ...and the active focus followed, not just the remembered path.
        assert_eq!(
            state.interaction.active_focus.patch_id(),
            Some(PatchId::new(2).unwrap())
        );

        // Right at the last position is likewise unchanged.
        assert_eq!(
            state.apply(AppEvent::SelectPatch(Direction::Right)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state.interaction.patch_focus().unwrap(), second);

        state.apply(AppEvent::SelectPatch(Direction::Left)).unwrap();
        assert_eq!(state.interaction.patch_focus().unwrap(), first);
    }

    /// Stepping between Patches is a horizontal adjacent choice and belongs to
    /// the Patch context in Navigate mode only.
    #[test]
    fn select_patch_is_refused_outside_its_context_mode_and_axis() {
        let mut state = installed_state();

        // MIXER context: the action does not exist here.
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
            .unwrap();
        assert_eq!(
            state.apply(AppEvent::SelectPatch(Direction::Right)),
            Err(EventRejection::ActionUnavailableInContext)
        );

        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();

        // Vertical directions belong to control navigation, not Patch order.
        for direction in [Direction::Up, Direction::Down] {
            assert_eq!(
                state.apply(AppEvent::SelectPatch(direction)),
                Err(EventRejection::ActionUnavailableInContext)
            );
        }

        // Adjust mode is editing a value, not moving between instruments.
        state
            .apply(AppEvent::SetInteractionMode(
                crate::control::InteractionMode::Adjust,
            ))
            .unwrap();
        assert_eq!(
            state.apply(AppEvent::SelectPatch(Direction::Right)),
            Err(EventRejection::ActionUnavailableInContext)
        );
    }

    #[test]
    fn patch_focus_reducer_covers_every_control_edge_and_direction_without_wrapping() {
        let controls = mixed_state().focused_patch_controls().unwrap();
        assert_eq!(
            controls,
            vec![
                PatchControlId::Engine,
                PatchControlId::EffectSlot(EffectSlotIndex::ALL[0]),
                PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]),
                PatchControlId::EffectSlot(EffectSlotIndex::ALL[2]),
            ]
        );
        for (index, expected) in controls.iter().cloned().enumerate() {
            let mut focused = mixed_state();
            for _ in 0..index {
                focused.apply(AppEvent::Navigate(Direction::Down)).unwrap();
            }
            assert_eq!(focused.interaction().patch_control_focus(), Some(expected));

            let mut left = focused.clone();
            let before = left.clone();
            assert_eq!(
                left.apply(AppEvent::Navigate(Direction::Left)),
                Err(EventRejection::ActionUnavailableInContext)
            );
            assert_eq!(left, before);

            let origin = focused.interaction().focus_path().clone();
            let mut right = focused.clone();
            right.apply(AppEvent::Navigate(Direction::Right)).unwrap();
            assert_eq!(
                right.interaction().active_surface(),
                SurfaceId::PatchUtility
            );
            assert_eq!(
                right.interaction().return_path().map(|path| path.origin()),
                Some(&origin)
            );
            right.apply(AppEvent::Return).unwrap();
            assert_eq!(right.interaction().focus_path(), &origin);

            let mut up = focused.clone();
            if index == 0 {
                let before = up.clone();
                assert_eq!(
                    up.apply(AppEvent::Navigate(Direction::Up)),
                    Err(EventRejection::ActionUnavailableInContext)
                );
                assert_eq!(up, before);
            } else {
                up.apply(AppEvent::Navigate(Direction::Up)).unwrap();
                assert_eq!(
                    up.interaction().patch_control_focus(),
                    Some(controls[index - 1].clone())
                );
            }

            let mut down = focused.clone();
            if index + 1 == controls.len() {
                let before = down.clone();
                assert_eq!(
                    down.apply(AppEvent::Navigate(Direction::Down)),
                    Err(EventRejection::ActionUnavailableInContext)
                );
                assert_eq!(down, before);
            } else {
                down.apply(AppEvent::Navigate(Direction::Down)).unwrap();
                assert_eq!(
                    down.interaction().patch_control_focus(),
                    Some(controls[index + 1].clone())
                );
            }
        }
    }

    #[test]
    fn every_patch_envelope_field_uses_descriptor_steps_bounds_and_target_isolation() {
        let middle = crate::synth::VoiceEnvelope::new(500.0, 600.0, 0.5, 700.0).unwrap();

        for descriptor in crate::synth::VoiceEnvelope::surface_descriptor() {
            for direction in [
                Direction::Left,
                Direction::Right,
                Direction::Down,
                Direction::Up,
            ] {
                let mut state = installed_state();
                state.patches[0].set_envelope(middle);
                let comparison = state.patches()[1].clone();
                state
                    .apply(AppEvent::SelectContext(TopLevelContext::Patch))
                    .unwrap();
                focus_instrument_detail_control(
                    &mut state,
                    PatchControlId::Envelope(descriptor.parameter()),
                );

                let expected = adjusted_value(
                    middle.value(descriptor.parameter()),
                    descriptor.minimum(),
                    descriptor.maximum(),
                    direction,
                    descriptor.fine_step(),
                    descriptor.coarse_step(),
                )
                .unwrap();
                let outcome = state.apply(AppEvent::Adjust(direction)).unwrap();

                assert_eq!(outcome.audio_command(), None);
                assert_eq!(outcome.engine_selection_effect(), None);
                assert_eq!(
                    state.patches()[0].envelope().value(descriptor.parameter()),
                    expected
                );
                for other in crate::synth::VoiceEnvelope::surface_descriptor() {
                    if other.parameter() != descriptor.parameter() {
                        assert_eq!(
                            state.patches()[0].envelope().value(other.parameter()),
                            middle.value(other.parameter())
                        );
                    }
                }
                assert_eq!(state.patches()[1], comparison);
            }

            for (boundary, directions) in [
                (descriptor.minimum(), [Direction::Left, Direction::Down]),
                (descriptor.maximum(), [Direction::Right, Direction::Up]),
            ] {
                for direction in directions {
                    let mut state = installed_state();
                    let envelope = middle.with_value(descriptor.parameter(), boundary).unwrap();
                    state.patches[0].set_envelope(envelope);
                    state
                        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
                        .unwrap();
                    focus_instrument_detail_control(
                        &mut state,
                        PatchControlId::Envelope(descriptor.parameter()),
                    );
                    let before = state.clone();
                    assert_eq!(
                        state.apply(AppEvent::Adjust(direction)),
                        Err(EventRejection::ParameterAtBoundary)
                    );
                    assert_eq!(state, before);

                    let recovery_direction = match direction {
                        Direction::Left => Direction::Right,
                        Direction::Right => Direction::Left,
                        Direction::Down => Direction::Up,
                        Direction::Up => Direction::Down,
                    };
                    state
                        .apply(AppEvent::Adjust(recovery_direction))
                        .expect("a valid later adjustment recovers after rejection");
                }
            }
        }
    }

    #[test]
    fn mixer_tracks_and_patch_envelopes_have_disjoint_ownership() {
        let middle = crate::synth::VoiceEnvelope::new(500.0, 600.0, 0.5, 700.0).unwrap();

        for descriptor in crate::synth::VoiceEnvelope::surface_descriptor() {
            let mut mixer = installed_state();
            mixer.patches[0].set_envelope(middle);
            let patches_before = mixer.patches().to_vec();
            mixer.apply(AppEvent::Adjust(Direction::Left)).unwrap();
            assert_eq!(mixer.patches(), patches_before);
            assert_eq!(
                mixer.mixer().track(MixerTrackId::default()).level_db(),
                -1.0
            );

            let mut patch = installed_state();
            patch.patches[0].set_envelope(middle);
            let mixer_before = *patch.mixer();
            let unrelated_before = patch.patches()[1].clone();
            patch
                .apply(AppEvent::SelectContext(TopLevelContext::Patch))
                .unwrap();
            focus_instrument_detail_control(
                &mut patch,
                PatchControlId::Envelope(descriptor.parameter()),
            );
            patch.apply(AppEvent::Adjust(Direction::Right)).unwrap();

            assert_eq!(*patch.mixer(), mixer_before);
            assert_eq!(patch.patches()[1], unrelated_before);
            assert_eq!(
                patch.patches()[0].envelope().value(descriptor.parameter()),
                middle.value(descriptor.parameter()) + descriptor.fine_step()
            );
        }
    }

    #[test]
    fn unavailable_patch_actions_reject_then_valid_focus_and_adsr_events_recover() {
        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        let engine = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::EngineSelectionUnavailable)
        );
        assert_eq!(state, engine);

        focus_instrument_detail_control(
            &mut state,
            PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds),
        );
        let adjusted = state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(adjusted.audio_command(), None);
        assert_eq!(adjusted.engine_selection_effect(), None);
        assert_eq!(state.patches()[0].envelope().attack_milliseconds(), 1.0);

        assert_eq!(
            crate::control::PatchControlId::surface_descriptor().len(),
            5
        );
        assert!(crate::control::PatchControlId::surface_descriptor()
            .iter()
            .all(|control| !control.as_str().contains("soundFont")
                && !control.as_str().contains("braids")));
    }

    #[test]
    fn patch_focus_and_adsr_generation_and_effect_accounting_is_exact() {
        let mut state = mixed_state();
        let patches = state.patches().to_vec();
        let engine_status = state.engine_selection().clone();
        focus_instrument_detail_control(
            &mut state,
            PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds),
        );
        let before_adjust = state.generation();
        let adjusted = state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(adjusted.accepted().generation(), before_adjust + 1);
        assert_eq!(state.generation(), before_adjust + 1);
        assert_eq!(adjusted.audio_command(), None);
        assert_eq!(adjusted.engine_selection_effect(), None);
        assert_eq!(state.engine_selection(), &engine_status);
        assert_ne!(state.patches(), patches);

        let detail_focus = state.interaction().focus_path().clone();
        state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        assert_eq!(
            state.interaction().active_surface(),
            SurfaceId::PatchUtility
        );
        state.apply(AppEvent::Navigate(Direction::Left)).unwrap();
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchDetail);
        assert_eq!(state.interaction().focus_path(), &detail_focus);

        state.apply(AppEvent::Return).unwrap();
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        let origin = state.interaction().focus_path().clone();
        let before_utility = state.generation();
        let entered = state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        assert_eq!(entered.accepted().generation(), before_utility + 1);
        assert_eq!(
            state.interaction().active_surface(),
            SurfaceId::PatchUtility
        );
        state.apply(AppEvent::Return).unwrap();
        assert_eq!(state.interaction().focus_path(), &origin);

        let mut endpoint = mixed_state();
        let before = endpoint.clone();
        assert_eq!(
            endpoint.apply(AppEvent::Navigate(Direction::Up)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(endpoint, before);

        state.apply(AppEvent::Navigate(Direction::Up)).unwrap();
        focus_instrument_detail_control(
            &mut state,
            PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds),
        );
        let boundary_envelope = (*state.patches[0].envelope())
            .with_value(VoiceEnvelopeParameter::AttackMilliseconds, 0.0)
            .unwrap();
        state.patches[0].set_envelope(boundary_envelope);
        let boundary = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Left)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, boundary);
    }

    #[test]
    fn app_state_installation_preserves_order_and_is_startup_only() {
        let mut state = AppState::new(registry(), global_parameters());
        let outcome = state
            .apply(AppEvent::InstallPatches(vec![
                patch(2, -3.0),
                patch(1, 0.0),
            ]))
            .unwrap();

        assert_eq!(outcome.accepted().generation(), 1);
        assert_eq!(state.generation(), 1);
        assert_eq!(state.patches()[0].id(), PatchId::new(2).unwrap());
        assert_eq!(state.patches()[1].id(), PatchId::new(1).unwrap());

        let accepted = state.clone();
        assert_eq!(
            state.apply(AppEvent::InstallPatches(Vec::new())),
            Err(EventRejection::InstallationClosed)
        );
        assert_eq!(state, accepted);
    }

    #[test]
    fn app_state_installation_accepts_shared_midi_channels() {
        let mut state = AppState::new(registry(), global_parameters());
        state
            .apply(AppEvent::InstallPatches(vec![
                patch_on_channel(1, 0.0, 3),
                patch_on_channel(2, -3.0, 3),
            ]))
            .unwrap();

        assert_eq!(state.patches().len(), 2);
        assert!(
            state
                .patches()
                .iter()
                .all(|patch| patch.channel() == MidiChannel::new(3).unwrap()),
            "every Patch may subscribe to the same MIDI channel"
        );
    }

    #[test]
    fn app_state_rejects_invalid_instrument_config_atomically_and_remains_processable() {
        let mut state = AppState::new(registry(), global_parameters());
        let initial = state.clone();
        let invalid_config = crate::synth::InstrumentConfig::from_parts(
            crate::synth::CapabilityId::new("instrument.unknown").unwrap(),
            Vec::new(),
            Vec::new(),
        );
        let invalid_patch = Patch::new(
            PatchId::new(1).unwrap(),
            "Invalid".to_owned(),
            invalid_config,
            MidiChannel::new(0).unwrap(),
            PatchOutput::default(),
        );

        assert_eq!(
            state.apply(AppEvent::InstallPatches(vec![invalid_patch])),
            Err(EventRejection::InvalidInstrumentConfig)
        );
        assert_eq!(state, initial);

        let accepted = state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(accepted.accepted().generation(), 1);
        assert_eq!(state.generation(), 1);
        assert_eq!(state.capabilities(), initial.capabilities());
        assert!(state.patches().is_empty());
    }

    #[test]
    fn app_state_installation_rejects_more_than_sixteen_patches() {
        let mut state = AppState::new(registry(), global_parameters());
        let initial = state.clone();
        let patches = (1..=17)
            .map(|id| patch_on_channel(id, 0.0, ((id - 1) % 16) as u8))
            .collect();

        assert_eq!(
            state.apply(AppEvent::InstallPatches(patches)),
            Err(EventRejection::TooManyPatches)
        );
        assert_eq!(state, initial);
    }

    #[test]
    fn app_state_navigation_changes_selection_without_parameters() {
        let mut state = installed_state();
        let patches = state.patches().to_vec();
        let mixer = *state.mixer();
        let global = *state.global();

        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(state.selection().parameter_index(), 1);
        assert_eq!(state.patches(), patches.as_slice());
        assert_eq!(*state.mixer(), mixer);
        assert_eq!(*state.global(), global);

        state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        assert_eq!(state.selection().section(), SelectionSection::Patch);
        assert_eq!(state.selection().patch_index(), 1);

        state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        assert_eq!(state.selection().section(), SelectionSection::Patch);
        assert_eq!(state.selection().patch_index(), 2);
        assert_eq!(state.selection().parameter_index(), 1);
    }

    #[test]
    fn app_state_adjusts_exactly_one_value_and_rejects_at_the_bound() {
        let mut state = installed_state();
        let patches = state.patches().to_vec();
        let second_track = *state.mixer().track(MixerTrackId::new(1).unwrap());
        let global = *state.global();

        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(state.mixer().track(MixerTrackId::default()).level_db(), 1.0);
        assert_eq!(state.mixer().track(MixerTrackId::default()).pan(), 0.0);
        assert_eq!(
            state.mixer().track(MixerTrackId::new(1).unwrap()),
            &second_track
        );
        assert_eq!(state.patches(), patches.as_slice());
        assert_eq!(*state.global(), global);

        state.apply(AppEvent::Adjust(Direction::Up)).unwrap();
        assert_eq!(state.mixer().track(MixerTrackId::default()).level_db(), 6.0);

        let accepted = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Up)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, accepted);

        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        state.apply(AppEvent::Adjust(Direction::Left)).unwrap();
        assert_eq!(state.mixer().track(MixerTrackId::default()).level_db(), 6.0);
        assert_eq!(state.mixer().track(MixerTrackId::default()).pan(), -0.01);
    }

    #[test]
    fn patch_editable_surface_is_patch_owned_and_reduces_transactionally() {
        let mut state = installed_state();
        let targets = state.patch_editable_targets(0).unwrap();
        assert!(targets.len() >= 4);
        assert!(matches!(
            targets[0],
            PatchEditableTarget::Envelope(crate::synth::VoiceEnvelopeParameter::AttackMilliseconds)
        ));

        let original_output = state.patches()[0].output();
        let original_mixer = *state.mixer();
        let original_config = state.patches()[0].instrument_config().clone();
        let other_patch = state.patches()[1].clone();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        focus_instrument_detail_control(
            &mut state,
            PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds),
        );
        state.apply(AppEvent::Adjust(Direction::Up)).unwrap();

        assert_eq!(state.patches()[0].envelope().attack_milliseconds(), 100.0);
        assert_eq!(state.patches()[0].output(), original_output);
        assert_eq!(*state.mixer(), original_mixer);
        assert_eq!(state.patches()[0].instrument_config(), &original_config);
        assert_eq!(state.patches()[1], other_patch);
    }

    #[test]
    fn app_state_midi_acceptance_returns_one_effect_without_parameter_mutation() {
        let mut state = installed_state();
        let patch_id = PatchId::new(1).unwrap();
        let message = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap();
        let patches = state.patches().to_vec();
        let global = *state.global();
        let registry_address = state.capabilities() as *const CapabilityRegistry;
        let patch_storage_address = state.patches().as_ptr();

        let outcome = state.apply(AppEvent::Midi { patch_id, message }).unwrap();

        assert_eq!(outcome.accepted().generation(), 2);
        assert_eq!(
            outcome.audio_command(),
            Some(&AudioCommand::PatchMidi { patch_id, message })
        );
        assert_eq!(state.patches(), patches.as_slice());
        assert_eq!(*state.global(), global);
        assert_eq!(state.capabilities() as *const _, registry_address);
        assert_eq!(state.patches().as_ptr(), patch_storage_address);

        let accepted = state.clone();
        assert_eq!(
            state.apply(AppEvent::Midi {
                patch_id: PatchId::new(99).unwrap(),
                message,
            }),
            Err(EventRejection::UnknownPatch)
        );
        assert_eq!(state, accepted);

        state.generation = u64::MAX;
        let overflow = state.clone();
        assert_eq!(
            state.apply(AppEvent::Midi { patch_id, message }),
            Err(EventRejection::GenerationOverflow)
        );
        assert_eq!(state, overflow);
    }

    #[test]
    fn app_state_engine_request_is_adjacent_nonwrapping_correlated_and_busy_without_early_commit() {
        let mut state = mixed_state();
        let source_config = state.patches()[0].instrument_config().clone();
        let source_revision = state.engine_selection().active_graph_revision();
        let generation = state.generation();

        let outcome = state.apply(AppEvent::Adjust(Direction::Right)).unwrap();

        assert_eq!(outcome.accepted().generation(), generation + 1);
        assert_eq!(outcome.audio_command(), None);
        let effect = outcome.engine_selection_effect().unwrap();
        assert_eq!(effect.kind(), EngineSelectionEffectKind::PrepareRequested);
        assert_eq!(effect.request_id(), EngineSelectionRequestId::FIRST);
        assert_eq!(effect.patch_id(), PatchId::new(1).ok());
        assert_eq!(
            effect.source_capability_id().unwrap().as_str(),
            HIDEF_CAPABILITY_ID
        );
        assert_eq!(
            effect.target_capability_id().unwrap().as_str(),
            BRAIDS_CAPABILITY_ID
        );
        assert_eq!(effect.source_graph_revision(), source_revision);
        assert_eq!(effect.target_graph_revision(), None);
        assert_eq!(
            state.engine_selection().kind(),
            EngineSelectionStatusKind::Loading
        );
        assert_eq!(
            state.engine_selection().active_graph_revision(),
            source_revision
        );
        assert_eq!(state.patches()[0].instrument_config(), &source_config);

        let pending = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::StructuralEditBusy)
        );
        assert_eq!(state, pending);

        let mut boundary = mixed_state();
        assert_eq!(
            boundary.apply(AppEvent::Adjust(Direction::Left)),
            Err(EventRejection::EngineSelectionUnavailable)
        );
        assert_eq!(
            boundary.engine_selection().kind(),
            EngineSelectionStatusKind::Ready
        );

        let mut open_choice = mixed_state();
        open_choice.apply(AppEvent::Adjust(Direction::Up)).unwrap();
        assert_eq!(
            open_choice.interaction().active_surface(),
            SurfaceId::PatchChoice
        );
        let mut vertical = mixed_state();
        let before = vertical.clone();
        assert_eq!(
            vertical.apply(AppEvent::Adjust(Direction::Down)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(vertical, before);
    }

    #[test]
    fn app_state_engine_failure_and_stale_or_mismatched_outcomes_preserve_source_and_recover() {
        let mut state = mixed_state();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        advance_engine_to_preparing(&mut state);
        let source = state.clone();
        let target_revision = GraphRevision::INITIAL.checked_next().unwrap();
        let stale_failure = failed_event(
            &state,
            target_revision,
            EngineSelectionFailure::AssetUnavailable,
        );

        let invalid_candidate = crate::synth::InstrumentConfig::from_parts(
            state
                .engine_selection()
                .correlation()
                .unwrap()
                .target_capability_id()
                .unwrap()
                .clone(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            state.apply(prepared_event(&state, target_revision, invalid_candidate,)),
            Err(EventRejection::MismatchedEngineSelection)
        );
        assert_eq!(state, source);

        let outcome = state.apply(stale_failure.clone()).unwrap();
        assert_eq!(outcome.engine_selection_effect(), None);
        assert_eq!(
            state.engine_selection().kind(),
            EngineSelectionStatusKind::Unavailable
        );
        assert_eq!(
            state.engine_selection().failure(),
            Some(EngineSelectionFailure::AssetUnavailable)
        );
        assert_eq!(
            state.engine_selection().active_graph_revision(),
            GraphRevision::INITIAL
        );
        assert_eq!(state.patches(), source.patches());

        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(
            state
                .engine_selection()
                .correlation()
                .unwrap()
                .request_id()
                .value(),
            2
        );
        let pending = state.clone();
        assert_eq!(
            state.apply(stale_failure),
            Err(EventRejection::StaleEngineSelection)
        );
        assert_eq!(state, pending);

        let correlation = state.engine_selection().correlation().unwrap();
        let mismatched = AppEvent::EnginePreparationFailed {
            request_id: correlation.request_id(),
            patch_id: PatchId::new(99).unwrap(),
            intent: correlation.intent().clone(),
            source_capability_id: correlation.source_capability_id().unwrap().clone(),
            target_capability_id: correlation.target_capability_id().unwrap().clone(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision: target_revision,
            failure: EngineSelectionFailure::PreparationFailed,
        };
        assert_eq!(
            state.apply(mismatched),
            Err(EventRejection::MismatchedEngineSelection)
        );
        assert_eq!(state, pending);
    }

    #[test]
    fn app_state_commits_only_the_target_config_then_acknowledges_both_directions() {
        let mut state = mixed_state();
        let patch_id = state.patches()[0].id();
        let channel = state.patches()[0].channel();
        let envelope = *state.patches()[0].envelope();
        let output = state.patches()[0].output();
        let mixer = *state.mixer();
        let unrelated = state.patches()[1].clone();
        let original_soundfont = state.patches()[0].instrument_config().clone();
        assert_eq!(
            original_soundfont.value(&ParameterId::new(SOUNDFONT_PRESET_PARAMETER_ID).unwrap()),
            Some(&crate::synth::ParameterValue::Choice(
                SoundFontInstrument::new(0, 1, false)
                    .unwrap()
                    .preset_id()
                    .choice_id()
            ))
        );

        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        advance_engine_to_preparing(&mut state);
        let braids = descriptor_default_config(BRAIDS_CAPABILITY_ID);
        let revision_two = GraphRevision::INITIAL.checked_next().unwrap();
        let committed = state
            .apply(prepared_event(&state, revision_two, braids.clone()))
            .unwrap();
        assert_eq!(
            committed.engine_selection_effect().unwrap().kind(),
            EngineSelectionEffectKind::CandidatePrepared
        );
        assert_eq!(
            state.engine_selection().kind(),
            EngineSelectionStatusKind::Activating
        );
        assert_eq!(
            state.engine_selection().active_graph_revision(),
            GraphRevision::INITIAL
        );
        assert_eq!(state.patches()[0].instrument_config(), &original_soundfont);

        let request_id = state.engine_selection().correlation().unwrap().request_id();
        let intent = state
            .engine_selection()
            .correlation()
            .unwrap()
            .intent()
            .clone();
        let activating = state.clone();
        assert_eq!(
            state.apply(AppEvent::EngineActivationAcknowledged {
                request_id,
                intent: intent.clone(),
                target_graph_revision: revision_two,
                retired_graph_revision: GraphRevision::INITIAL,
                collected: false,
            }),
            Err(EventRejection::MismatchedEngineSelection)
        );
        assert_eq!(state, activating);
        let acknowledged = state
            .apply(AppEvent::EngineActivationAcknowledged {
                request_id,
                intent: intent.clone(),
                target_graph_revision: revision_two,
                retired_graph_revision: GraphRevision::INITIAL,
                collected: true,
            })
            .unwrap();
        assert_eq!(
            acknowledged.engine_selection_effect().unwrap().kind(),
            EngineSelectionEffectKind::ActivationAcknowledged
        );
        assert_eq!(
            state.engine_selection(),
            &EngineSelectionStatus::ready(revision_two)
        );
        assert_eq!(state.patches()[0].instrument_config(), &braids);

        let ready = state.clone();
        assert_eq!(
            state.apply(AppEvent::EngineActivationAcknowledged {
                request_id,
                intent: intent.clone(),
                target_graph_revision: revision_two,
                retired_graph_revision: GraphRevision::INITIAL,
                collected: true,
            }),
            Err(EventRejection::StaleEngineSelection)
        );
        assert_eq!(state, ready);

        state.apply(AppEvent::Adjust(Direction::Left)).unwrap();
        advance_engine_to_preparing(&mut state);
        let default_soundfont = descriptor_default_config(HIDEF_CAPABILITY_ID);
        let revision_three = revision_two.checked_next().unwrap();
        state
            .apply(prepared_event(
                &state,
                revision_three,
                default_soundfont.clone(),
            ))
            .unwrap();
        let reverse_request_id = state.engine_selection().correlation().unwrap().request_id();
        let reverse_intent = state
            .engine_selection()
            .correlation()
            .unwrap()
            .intent()
            .clone();
        state
            .apply(AppEvent::EngineActivationAcknowledged {
                request_id: reverse_request_id,
                intent: reverse_intent,
                target_graph_revision: revision_three,
                retired_graph_revision: revision_two,
                collected: true,
            })
            .unwrap();

        assert_eq!(
            state.engine_selection(),
            &EngineSelectionStatus::ready(revision_three)
        );
        assert_eq!(state.patches()[0].instrument_config(), &default_soundfont);
        assert_ne!(state.patches()[0].instrument_config(), &original_soundfont);
        assert_eq!(state.patches()[0].id(), patch_id);
        assert_eq!(state.patches()[0].channel(), channel);
        assert_eq!(*state.patches()[0].envelope(), envelope);
        assert_eq!(state.patches()[0].output(), output);
        assert_eq!(*state.mixer(), mixer);
        assert_eq!(state.patches()[1], unrelated);
    }

    #[test]
    fn app_state_keeps_midi_context_and_mixer_control_available_while_preparing() {
        let mut state = mixed_state();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        advance_engine_to_preparing(&mut state);
        let correlation = state.engine_selection().correlation().unwrap().clone();
        let message = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap();

        let midi = state
            .apply(AppEvent::Midi {
                patch_id: PatchId::new(1).unwrap(),
                message,
            })
            .unwrap();
        assert_eq!(
            midi.audio_command(),
            Some(&AudioCommand::PatchMidi {
                patch_id: PatchId::new(1).unwrap(),
                message,
            })
        );
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
            .unwrap();
        state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        let patches_before = state.patches().to_vec();
        state.apply(AppEvent::Adjust(Direction::Left)).unwrap();
        assert_eq!(state.patches(), patches_before);
        assert_eq!(
            state
                .mixer()
                .track(MixerTrackId::new(1).unwrap())
                .level_db(),
            -1.0
        );
        assert_eq!(state.engine_selection().correlation(), Some(&correlation));
        assert_eq!(
            state.engine_selection().kind(),
            EngineSelectionStatusKind::Preparing
        );
    }

    // ===================================================================
    // WP02: the widened PATCH control surface
    // ===================================================================

    /// Navigates PATCH Utility from its entry row to `control`.
    fn focus_utility_row(state: &mut AppState, control: &PatchControlId) {
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchUtility))
            .unwrap();
        let order = PatchControlId::utility_surface_descriptor();
        let index = |target: &PatchControlId| {
            order
                .iter()
                .position(|candidate| candidate == target)
                .expect("the declared Utility order hosts this row")
        };
        let entry = index(&PatchControlId::Output(PatchOutputParameter::TrimGain));
        let target = index(control);
        let (direction, steps) = if target >= entry {
            (Direction::Down, target - entry)
        } else {
            (Direction::Up, entry - target)
        };
        for _ in 0..steps {
            state.apply(AppEvent::Navigate(direction)).unwrap();
        }
        assert_eq!(
            state.interaction().focus_path().control_id(),
            &SemanticControlId::Patch(control.clone()),
            "navigation must land on the requested Utility row"
        );
    }

    /// T008: the panel is exactly five declared rows, and navigation refuses
    /// at both ends rather than wrapping.
    #[test]
    fn patch_utility_resolves_exactly_five_declared_rows_without_wrapping() {
        let mut state = installed_state();
        let patch_id = state.patches()[0].id();
        let paths = SemanticResolver::new(&state)
            .patch_utility_paths(patch_id)
            .unwrap();

        assert_eq!(
            paths
                .iter()
                .map(|path| path.control_id().clone())
                .collect::<Vec<_>>(),
            vec![
                SemanticControlId::Patch(PatchControlId::Global(GlobalParameter::MasterGainDb)),
                SemanticControlId::Patch(PatchControlId::Output(PatchOutputParameter::TrimGain)),
                SemanticControlId::Patch(PatchControlId::MidiInput),
                SemanticControlId::Patch(PatchControlId::Output(PatchOutputParameter::OutputTrack)),
                SemanticControlId::Patch(PatchControlId::VoiceLimit),
            ]
        );

        // None of the five enters the PatchMain order.
        let main = SemanticResolver::new(&state)
            .patch_main_paths(patch_id)
            .unwrap();
        for control in PatchControlId::utility_surface_descriptor() {
            assert!(
                !main
                    .iter()
                    .any(|path| path.control_id() == &SemanticControlId::Patch(control.clone())),
                "{control} must not appear in the PatchMain order"
            );
        }

        // Upward from the first row and downward from the last both refuse,
        // leaving state identical.
        focus_utility_row(
            &mut state,
            &PatchControlId::Global(GlobalParameter::MasterGainDb),
        );
        let at_top = state.clone();
        assert_eq!(
            state.apply(AppEvent::Navigate(Direction::Up)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, at_top);

        focus_utility_row(&mut state, &PatchControlId::VoiceLimit);
        let at_bottom = state.clone();
        assert_eq!(
            state.apply(AppEvent::Navigate(Direction::Down)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, at_bottom);
    }

    /// T009: master gain has exactly one canonical owner, reachable from both
    /// surfaces. Asserted in both directions through the production reducer.
    #[test]
    fn master_gain_is_one_canonical_value_reached_from_patch_and_mixer() {
        let mut state = installed_state();

        // PATCH edits it; MIXER reads the same value back.
        focus_utility_row(
            &mut state,
            &PatchControlId::Global(GlobalParameter::MasterGainDb),
        );
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        let after_patch_edit = state.global().master_gain_db();
        assert_ne!(after_patch_edit, 0.0, "the PATCH edit must move the value");
        assert_eq!(
            state.global_row_value(GlobalParameter::MasterGainDb),
            after_patch_edit,
            "the MIXER row reads the value PATCH just edited"
        );

        // MIXER edits it; PATCH reads the same value back.
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Navigate))
            .unwrap();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
            .unwrap();
        let mixer_paths = SemanticResolver::new(&state)
            .mixer_inspector_paths(
                state
                    .interaction()
                    .remembered_mixer_main()
                    .control_id()
                    .as_mixer_track_id()
                    .unwrap(),
            )
            .unwrap();
        let global_path = mixer_paths
            .iter()
            .find(|path| {
                matches!(
                    path.control_id(),
                    SemanticControlId::Mixer(MixerControlId::Global { .. })
                )
            })
            .expect("the Inspector hosts the canonical global row")
            .clone();
        state.interaction.active_focus = global_path;
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        let after_mixer_edit = state.global().master_gain_db();
        assert_ne!(after_mixer_edit, after_patch_edit);

        // Both surfaces moved the value by the same descriptor step, which is
        // only possible if they address one value through one descriptor.
        let step = GlobalParameter::MasterGainDb.descriptor().fine_step();
        assert!((after_patch_edit - step).abs() < 1.0e-6);
        assert!((after_mixer_edit - 2.0 * step).abs() < 1.0e-6);
    }

    /// T009: no PATCH-side type re-declares a master-gain field.
    ///
    /// **This is a text scan of the production sources, not a structural
    /// proof.** It catches the one shape it names — a second `master_gain_db`
    /// field appearing on a PATCH-owned type — and it cannot see aliasing,
    /// shadowing through another name, or two setters over one field. The
    /// proof that both surfaces reach *one* canonical value is the
    /// reducer-driven
    /// `master_gain_is_one_canonical_value_reached_from_patch_and_mixer`
    /// above, which edits from PATCH and from MIXER through `AppState::apply`
    /// and reads the same value moving by the same descriptor step. Cite that
    /// one; this is a cheap tripwire beside it.
    #[test]
    fn exactly_one_master_gain_owner_exists_and_patch_holds_no_copy() {
        // No PATCH-owned type declares the field. These are the three places a
        // PATCH-side copy would have to live to shadow the mixer's.
        for (name, source) in [
            ("Patch", include_str!("../synth/patch.rs")),
            ("PatchControlId", include_str!("patch_control_id.rs")),
            (
                "PatchPageProjection",
                include_str!("patch_page_projection.rs"),
            ),
        ] {
            let declarations = source
                .lines()
                .map(str::trim_start)
                .filter(|line| {
                    !line.starts_with("//")
                        && (line.starts_with("master_gain_db:")
                            || line.starts_with("pub master_gain_db:")
                            || line.starts_with("pub(crate) master_gain_db:"))
                })
                .count();
            assert_eq!(
                declarations, 0,
                "{name} must not declare a master-gain field; the one canonical \
                 owner is GlobalParameters"
            );
        }

        // The canonical storage owner declares it exactly once.
        let canonical = include_str!("../mixer/global_parameters.rs")
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("master_gain_db: f32,"))
            .count();
        assert_eq!(canonical, 1, "GlobalParameters owns the value exactly once");

        // And no Patch serializes one.
        let state = installed_state();
        let patch_json = serde_json::to_value(
            crate::control::serialized_state::SerializedPatch::from(&state.patches()[0]),
        )
        .unwrap();
        assert!(
            patch_json.get("masterGainDb").is_none(),
            "a Patch must not carry a master-gain copy"
        );
    }

    /// T009: the PATCH Utility row and the MIXER Inspector row project the
    /// same value through the same descriptor bounds — the observable
    /// consequence of there being one owner.
    #[test]
    fn both_surfaces_project_one_master_gain_value_and_one_set_of_bounds() {
        let mut state = installed_state();
        // Edit through the canonical global owner, then read what the PATCH
        // Utility projection shows.
        state.global = state.global.with_master_gain_db(-4.5).unwrap();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();

        let semantic =
            crate::control::SemanticGraphicalViewModel::project(&state, "state-hash").unwrap();
        let utility_row = semantic
            .surface(SurfaceId::PatchUtility)
            .expect("PATCH exposes its Utility surface")
            .controls()
            .iter()
            .find(|control| {
                control.path().control_id()
                    == &SemanticControlId::Patch(PatchControlId::Global(
                        GlobalParameter::MasterGainDb,
                    ))
            })
            .expect("the Utility panel hosts the master-volume row")
            .clone();

        assert_eq!(
            utility_row.value(),
            &crate::control::SemanticControlValue::Scalar(-4.5),
            "the PATCH row reads the value the MIXER edit wrote"
        );
        let descriptor = GlobalParameter::MasterGainDb.descriptor();
        let range = utility_row
            .numeric_range()
            .expect("the master-volume row carries its descriptor bounds");
        assert_eq!(range.minimum(), f64::from(descriptor.minimum()));
        assert_eq!(range.maximum(), f64::from(descriptor.maximum()));
        assert_eq!(range.fine_step(), f64::from(descriptor.fine_step()));
        assert_eq!(range.coarse_step(), f64::from(descriptor.coarse_step()));
    }

    /// T009: a MIDI channel change may join another Patch's channel and
    /// changes nothing else about the edited Patch.
    #[test]
    fn midi_input_accepts_a_shared_channel_and_touches_nothing_else() {
        let mut state = installed_state();
        let before = state.patches()[0].clone();
        let graph_before = state.engine_selection().projection_graph_revision();

        focus_utility_row(&mut state, &PatchControlId::MidiInput);
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();
        // Patch 2 already holds channel 1. Patch 1 joins it: a MIDI channel is
        // a shared subscription, not an exclusive owner.
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(state.patches()[0].channel(), state.patches()[1].channel());
        let shared = state.patches()[0].clone();

        // Return to channel 0, then prove the lower bound still refuses.
        state.apply(AppEvent::Adjust(Direction::Left)).unwrap();
        let restored = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Left)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, restored);

        // Vertical adjustment is not this row's gesture.
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Up)),
            Err(EventRejection::ActionUnavailableInContext)
        );

        assert_eq!(shared.channel().value(), 1);
        assert_eq!(shared.id(), before.id());
        assert_eq!(shared.instrument_config(), before.instrument_config());
        assert_eq!(shared.envelope(), before.envelope());
        assert_eq!(shared.effect_slots(), before.effect_slots());
        assert_eq!(shared.output(), before.output());
        assert_eq!(shared.voice_limit(), before.voice_limit());
        assert_eq!(
            state.engine_selection().projection_graph_revision(),
            graph_before,
            "a channel change publishes no new graph"
        );
        assert_eq!(state.patches()[0], before);
    }

    /// T009: the voice limit honours the descriptor's own fine and coarse
    /// steps — not literals — and refuses at both bounds.
    #[test]
    fn voice_limit_uses_descriptor_steps_and_refuses_at_both_bounds() {
        let descriptor = VoiceLimit::descriptor();
        let mut state = installed_state();
        let seeded = state.patches()[0].voice_limit().value();

        focus_utility_row(&mut state, &PatchControlId::VoiceLimit);
        state
            .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
            .unwrap();

        // Fine down, then coarse down: each moves by the descriptor's own step.
        state.apply(AppEvent::Adjust(Direction::Left)).unwrap();
        assert_eq!(
            state.patches()[0].voice_limit().value(),
            seeded - descriptor.fine_step()
        );
        state.apply(AppEvent::Adjust(Direction::Down)).unwrap();
        assert_eq!(
            state.patches()[0].voice_limit().value(),
            seeded - descriptor.fine_step() - descriptor.coarse_step()
        );
        // And back up by the same amounts.
        state.apply(AppEvent::Adjust(Direction::Up)).unwrap();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(state.patches()[0].voice_limit().value(), seeded);

        // At the maximum, increasing refuses and leaves state identical.
        while state.patches()[0].voice_limit().value() < descriptor.maximum() {
            state.apply(AppEvent::Adjust(Direction::Up)).unwrap();
        }
        let at_max = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, at_max);

        // At the minimum, decreasing refuses and leaves state identical.
        while state.patches()[0].voice_limit().value() > descriptor.minimum() {
            state.apply(AppEvent::Adjust(Direction::Down)).unwrap();
        }
        assert_eq!(
            state.patches()[0].voice_limit().value(),
            descriptor.minimum()
        );
        let at_min = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Left)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, at_min);
    }

    /// H1: installation seeds every Patch's limit from its own engine's
    /// declared ceiling — proved against both real production descriptors.
    #[test]
    fn installation_seeds_each_patch_limit_from_its_own_engine_ceiling() {
        use crate::adapter::braids_capability::BRAIDS_FIXED_VOICES;
        use crate::adapter::hidef_soundfont_capability::HIDEF_POLYPHONY_CEILING;

        let registry =
            crate::adapter::production_instruments::production_capability_registry().unwrap();
        let braids = BraidsCapability::new().unwrap().default_config().unwrap();
        let soundfont =
            create_soundfont_config(&provider(), SoundFontInstrument::new(0, 1, false).unwrap())
                .unwrap();

        let mut state = AppState::new(registry, global_parameters());
        state
            .apply(AppEvent::InstallPatches(vec![
                Patch::new(
                    PatchId::new(1).unwrap(),
                    "SoundFont".to_owned(),
                    soundfont,
                    MidiChannel::new(0).unwrap(),
                    PatchOutput::to_track(MixerTrackId::new(0).unwrap()),
                ),
                Patch::new(
                    PatchId::new(2).unwrap(),
                    "Braids".to_owned(),
                    braids,
                    MidiChannel::new(1).unwrap(),
                    PatchOutput::to_track(MixerTrackId::new(1).unwrap()),
                ),
            ]))
            .unwrap();

        assert_eq!(
            state.patches()[0].voice_limit().value(),
            HIDEF_POLYPHONY_CEILING
        );
        assert_eq!(
            state.patches()[1].voice_limit().value(),
            BRAIDS_FIXED_VOICES,
            "a Braids Patch must carry its own engine's ceiling, not 64"
        );
        assert_ne!(
            state.patches()[0].voice_limit(),
            state.patches()[1].voice_limit(),
            "seeding must discriminate between the two installed engines"
        );
    }

    /// H2: an engine swap to a narrower engine clamps the limit in canonical
    /// state, and the reported carry-over is what canonical state holds.
    #[test]
    fn an_engine_swap_to_a_narrower_engine_clamps_the_limit_in_canonical_state() {
        use crate::adapter::braids_capability::BRAIDS_FIXED_VOICES;
        use crate::synth::instrument_capability::VoicePolicy;

        // The reducer's clamp is the same one `replace_instrument_config`
        // performs; proved here on the canonical aggregate the reducer mutates.
        let mut patch = patch(1, 0.0);
        patch.seed_voice_limit(VoicePolicy::EngineManaged);
        assert_eq!(patch.voice_limit().value(), VoiceLimit::MAXIMUM);

        let braids = BraidsCapability::new().unwrap().default_config().unwrap();
        let carry_over = patch.replace_instrument_config(
            braids,
            VoicePolicy::FixedPerPatch {
                voices: BRAIDS_FIXED_VOICES,
            },
        );

        assert_eq!(
            carry_over,
            crate::synth::patch::VoiceLimitCarryOver::Clamped {
                previous: VoiceLimit::new(VoiceLimit::MAXIMUM).unwrap(),
                limit: VoiceLimit::new(BRAIDS_FIXED_VOICES).unwrap(),
            }
        );
        assert_eq!(
            patch.voice_limit(),
            carry_over.limit(),
            "canonical state must hold exactly the reported carry-over limit"
        );
        assert_eq!(patch.voice_limit().value(), BRAIDS_FIXED_VOICES);
    }

    /// T010/T011: entry is accepted only from a PatchMain row whose control
    /// resolves a subject, and the three detail facts move together.
    #[test]
    fn detail_entry_is_accepted_only_from_a_row_that_resolves_a_subject() {
        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        assert!(state.interaction().detail_invariant_holds());
        assert_eq!(state.interaction().detail_subject(), None);

        // The engine row resolves the active instrument capability.
        let origin = state.interaction().focus_path().clone();
        assert_eq!(
            origin.control_id(),
            &SemanticControlId::Patch(PatchControlId::Engine)
        );
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();

        // All three facts moved together.
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchDetail);
        assert_eq!(
            state.interaction().return_path().unwrap().entered_surface(),
            SurfaceId::PatchDetail
        );
        assert_eq!(state.interaction().return_path().unwrap().origin(), &origin);
        assert!(matches!(
            state.interaction().detail_subject(),
            Some(PatchDetailSubject::Instrument { .. })
        ));
        assert!(state.interaction().detail_invariant_holds());

        // Return restores the exact originating row and clears both together.
        state.apply(AppEvent::Return).unwrap();
        assert_eq!(state.interaction().focus_path(), &origin);
        assert_eq!(state.interaction().return_path(), None);
        assert_eq!(state.interaction().detail_subject(), None);
        assert!(state.interaction().detail_invariant_holds());
    }

    /// T010/T011: an empty slot and every Utility row resolve no subject, so
    /// entry from them is a typed unchanged rejection rather than an empty
    /// detail surface.
    #[test]
    fn detail_entry_from_a_subjectless_row_is_a_typed_unchanged_rejection() {
        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        // Move off the engine row onto the first empty slot.
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert!(matches!(
            state.interaction().focus_path().control_id(),
            SemanticControlId::Patch(PatchControlId::EffectSlot(_))
        ));

        let before = state.clone();
        assert_eq!(
            state.apply(AppEvent::EnterSurface(SurfaceId::PatchDetail)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, before, "a refused entry leaves state identical");
        assert_eq!(state.interaction().detail_subject(), None);

        // Every Utility row refuses too — and from Utility the origin is not
        // even a main path, so the surfaces cannot nest.
        for control in PatchControlId::utility_surface_descriptor() {
            let mut utility = installed_state();
            focus_utility_row(&mut utility, control);
            let before = utility.clone();
            assert_eq!(
                utility.apply(AppEvent::EnterSurface(SurfaceId::PatchDetail)),
                Err(EventRejection::ActionUnavailableInContext),
                "entering detail from the {control} Utility row must be refused"
            );
            assert_eq!(utility, before);
            assert!(utility.interaction().detail_invariant_holds());
        }
    }

    /// T010/T011: an empty effect slot names no capability, so entry from it is
    /// refused; an occupied one resolves its exact slot identity.
    #[test]
    fn detail_entry_distinguishes_an_empty_slot_from_an_occupied_one() {
        let mut state = gapped_effects_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        let patch_id = state.patches()[0].id();
        let resolver = SemanticResolver::new(&state);

        // Slot 0 is empty: no subject.
        let empty = FocusPath::patch_main(
            patch_id,
            None,
            PatchControlId::EffectSlot(EffectSlotIndex::new(0).unwrap()),
        );
        assert_eq!(resolver.detail_subject(&empty), None);

        // Slot 1 is occupied: the subject carries that slot's exact identity.
        let occupied = FocusPath::patch_main(
            patch_id,
            None,
            PatchControlId::EffectSlot(EffectSlotIndex::new(1).unwrap()),
        );
        let subject = resolver
            .detail_subject(&occupied)
            .expect("an occupied slot resolves an Effect subject");
        assert_eq!(subject.slot_id(), Some(EffectSlotId::new(2).unwrap()));

        // Entering from the empty slot is a typed unchanged rejection.
        state.interaction.active_focus = empty;
        let before = state.clone();
        assert_eq!(
            state.apply(AppEvent::EnterSurface(SurfaceId::PatchDetail)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, before);

        // Entering from the occupied slot opens the surface on that subject.
        state.interaction.active_focus = occupied;
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        assert_eq!(state.interaction().detail_subject(), Some(&subject));
        assert!(state.interaction().detail_invariant_holds());
    }

    /// T011: the subordinate surfaces do not nest in either direction.
    #[test]
    fn subordinate_surfaces_refuse_to_nest_in_either_direction() {
        // Utility from an open detail surface.
        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        let in_detail = state.clone();
        assert_eq!(
            state.apply(AppEvent::EnterSurface(SurfaceId::PatchUtility)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, in_detail);
        assert_eq!(
            state.interaction().return_path().unwrap().entered_surface(),
            SurfaceId::PatchDetail,
            "the one remembered origin is unchanged by the refusal"
        );

        // Detail from an open Utility surface is covered by
        // `detail_entry_from_a_subjectless_row_is_a_typed_unchanged_rejection`.
    }

    /// B1, resolved: the detail surface is reducer-owned **and** offered.
    ///
    /// `EnterSurface(PatchDetail)` was held out of the admitted semantic action
    /// vocabulary for exactly as long as no projection could render a detail
    /// focus. It now reaches `validActions` through the same passive boundary
    /// as every other action, and a row that resolves no subject still refuses
    /// it — the gate's removal widened what is offered, not what is accepted.
    #[test]
    fn the_detail_surface_is_reducer_owned_and_offered_through_the_passive_boundary() {
        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        let entry = SemanticAction::EnterSurface(SurfaceId::PatchDetail);

        // Advertised: accepted, and present in the projected action list.
        assert!(state.accepts_semantic_action(&entry));
        assert!(SemanticResolver::new(&state)
            .valid_actions()
            .iter()
            .any(|valid| valid.action() == &entry));

        // Accepted through the passive boundary, not only through `AppEvent`.
        state.apply_semantic_action(entry.clone()).unwrap();
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchDetail);
        assert!(state.interaction().detail_subject().is_some());

        // Still refused where no subject resolves: from an empty slot the
        // action is neither offered nor accepted, and refusing it changes
        // nothing. Offering the vocabulary did not widen the entry rule.
        state.apply_semantic_action(SemanticAction::Return).unwrap();
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert!(
            matches!(
                state.interaction().focus_path().control_id(),
                SemanticControlId::Patch(PatchControlId::EffectSlot(_))
            ),
            "the first empty slot resolves no detail subject"
        );
        assert!(!state.accepts_semantic_action(&entry));
        assert!(!SemanticResolver::new(&state)
            .valid_actions()
            .iter()
            .any(|valid| valid.action() == &entry));
        let before = state.clone();
        assert_eq!(
            state.apply_semantic_action(entry),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, before);
    }

    /// B1, resolved: a **Braids** detail state projects, which is the whole
    /// reason the entry gate existed.
    ///
    /// Patch Main contains only the shared Overview identities, while this
    /// engine's Detail order is capability-owned, so the two share nothing.
    ///
    /// The assertions below therefore *establish* the discriminating shape
    /// before projecting: the focused detail control is one the main order does
    /// not host. Only then is a successful projection evidence of anything.
    #[test]
    fn a_braids_detail_state_projects_even_though_its_row_is_absent_from_the_main_order() {
        let mut state = mixed_state();
        state
            .apply(AppEvent::SelectPatch(Direction::Right))
            .unwrap();
        assert_eq!(
            state.patches()[1]
                .instrument_config()
                .capability_id()
                .as_str(),
            BRAIDS_CAPABILITY_ID
        );

        // Offered, through the passive boundary a player actually reaches.
        state
            .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();

        let focused = state.interaction().patch_control_focus().unwrap();
        let main_order = state.focused_patch_controls().unwrap();
        assert!(
            !main_order
                .iter()
                .any(|control| matches!(control, PatchControlId::Capability(_))),
            "Braids' main order must host no Capability row at all"
        );
        assert!(
            !main_order.contains(&focused),
            "the discriminating case requires a detail row the main order does not host"
        );

        let (_, page, text, _shell, _parameters, _tree) = crate::control::StateProjector::new()
            .project_with_shell_tree(&state)
            .expect("a Braids detail state must project");

        // The text projection's selected line is the detail row itself, not
        // some main row that happens to share its identity: without a detail
        // line to select, `render_patch_text` would have failed above rather
        // than picked a neighbour, and this pins which line it picked.
        let page = page.expect("PATCH context always projects a page");
        assert_eq!(page.focused_control_id(), focused);
        let selected = text
            .body()
            .lines()
            .nth(text.selected_line())
            .expect("the selected line is inside the body");
        assert!(
            selected.starts_with("> DETAIL_PARAMETER "),
            "the selected line must be the detail row, got {selected}"
        );
    }

    /// B1a: an engine swap committing under an open detail entry leaves the
    /// surface rather than showing the old capability's rows under the new
    /// engine's identity.
    #[test]
    fn an_engine_swap_under_an_open_detail_entry_leaves_the_surface() {
        let mut state = mixed_state();
        let origin = state.interaction().focus_path().clone();
        assert_eq!(
            origin.control_id(),
            &SemanticControlId::Patch(PatchControlId::Engine)
        );

        // Request the swap from the engine row, then open detail on the row
        // that is still showing the *source* capability.
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        assert_eq!(
            state.interaction().detail_subject(),
            Some(&PatchDetailSubject::instrument(
                crate::synth::CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap()
            ))
        );

        advance_engine_to_preparing(&mut state);
        let target = GraphRevision::new(2).unwrap();
        state
            .apply(prepared_event(
                &state.clone(),
                target,
                descriptor_default_config(BRAIDS_CAPABILITY_ID),
            ))
            .unwrap();

        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchDetail);
        let correlation = state.engine_selection().correlation().unwrap().clone();
        state
            .apply(AppEvent::EngineActivationAcknowledged {
                request_id: correlation.request_id(),
                intent: correlation.intent().clone(),
                target_graph_revision: target,
                retired_graph_revision: correlation.source_graph_revision(),
                collected: true,
            })
            .unwrap();

        // The subject no longer names the Patch's capability, so the surface
        // was left back to the exact remembered origin.
        assert_eq!(state.interaction().detail_subject(), None);
        assert_eq!(state.interaction().return_path(), None);
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchMain);
        assert_eq!(state.interaction().focus_path(), &origin);
        assert!(state.interaction().detail_invariant_holds());
        assert!(SemanticResolver::new(&state).resolves(state.interaction().focus_path()));
        crate::control::StateProjector::new()
            .project_with_shell_tree(&state)
            .expect("the state after leaving must project");
    }

    #[test]
    fn detail_and_utility_switch_horizontally_without_losing_the_detail_session() {
        let mut state = mixed_state();
        let overview_origin = state.interaction().focus_path().clone();
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();

        let project_and_assert_one_focus = |state: &AppState| {
            let projection = StateProjector::new()
                .project_with_shell(state)
                .expect("each accepted adjacency step projects")
                .3;
            let document = serde_json::to_value(projection.semantic_model())
                .expect("the semantic projection serializes");
            let focused = document
                .get("surfaces")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .flat_map(|surface| {
                    surface
                        .get("controls")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .filter(|control| {
                    control.get("visible").and_then(serde_json::Value::as_bool) == Some(true)
                        && control.get("focused").and_then(serde_json::Value::as_bool) == Some(true)
                })
                .collect::<Vec<_>>();
            assert_eq!(
                focused.len(),
                1,
                "every accepted Detail/Utility adjacency has one visible focus"
            );
            assert_eq!(
                focused[0].get("path"),
                document.get("focusPath"),
                "the only painted focus is the reducer's stable semantic focus"
            );
            document
        };

        let first = project_and_assert_one_focus(&state);
        let detail = first
            .get("surfaces")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .find(|surface| {
                surface.get("id").and_then(serde_json::Value::as_str) == Some("patchDetail")
            })
            .expect("the entered Detail surface projects");
        let declared_detail_order = detail
            .get("sections")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .flat_map(|section| {
                section
                    .get("controlPaths")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .cloned()
            })
            .collect::<Vec<_>>();
        assert!(!declared_detail_order.is_empty());

        let mut visited = Vec::with_capacity(declared_detail_order.len());
        for index in 0..declared_detail_order.len() {
            let document = project_and_assert_one_focus(&state);
            visited.push(document.get("focusPath").cloned().unwrap());
            if index + 1 < declared_detail_order.len() {
                state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
            }
        }
        assert_eq!(
            visited, declared_detail_order,
            "unmodified arrows traverse every projected Detail section in descriptor order"
        );
        let detail_focus = state.interaction().focus_path().clone();
        let detail_subject = state.interaction().detail_subject().cloned();

        state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        project_and_assert_one_focus(&state);
        assert_eq!(
            state.interaction().active_surface(),
            SurfaceId::PatchUtility
        );
        assert_eq!(
            state.interaction().detail_subject(),
            detail_subject.as_ref()
        );
        assert_eq!(
            state.interaction().return_path().unwrap().origin(),
            &overview_origin
        );
        let utility = state.clone();
        assert_eq!(
            state.apply(AppEvent::Navigate(Direction::Right)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, utility);

        state.apply(AppEvent::Navigate(Direction::Left)).unwrap();
        project_and_assert_one_focus(&state);
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchDetail);
        assert_eq!(state.interaction().focus_path(), &detail_focus);
        assert_eq!(
            state.interaction().detail_subject(),
            detail_subject.as_ref()
        );
        assert_eq!(
            state.interaction().return_path().unwrap().origin(),
            &overview_origin
        );

        state.apply(AppEvent::Navigate(Direction::Left)).unwrap();
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchMain);
        assert_eq!(state.interaction().focus_path(), &overview_origin);
        assert!(state.interaction().detail_invariant_holds());
    }

    #[test]
    fn disabling_an_open_detail_origin_repairs_return_and_projects_visible_status() {
        let mut state = mixed_state();
        let patch_id = state.patches()[0].id();
        let removed = state.interaction().focus_path().clone();
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();

        state
            .apply(AppEvent::SetPatchOverviewOriginEnabled {
                patch_id,
                control: PatchControlId::Engine,
                enabled: false,
            })
            .unwrap();
        let replacement = FocusPath::patch_main(
            patch_id,
            None,
            PatchControlId::EffectSlot(EffectSlotIndex::new(0).unwrap()),
        );
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchDetail);
        assert_eq!(
            state.interaction().return_path().unwrap().origin(),
            &replacement
        );
        let repair = state.focus_repair_status().unwrap();
        assert_eq!(repair.removed_origin(), &removed);
        assert_eq!(repair.replacement_origin(), &replacement);

        let (_, _, _, shell, _, _) = StateProjector::new()
            .project_with_shell_tree(&state)
            .unwrap();
        let projected = shell.semantic_model().focus_repair().unwrap();
        assert_eq!(projected.patch_id(), patch_id);
        assert_eq!(projected.removed_control_id(), &PatchControlId::Engine);
        assert_eq!(
            projected.replacement_control_id(),
            &PatchControlId::EffectSlot(EffectSlotIndex::new(0).unwrap())
        );
        assert!(projected.label().contains("Focus repaired"));

        state.apply(AppEvent::Return).unwrap();
        assert_eq!(state.interaction().focus_path(), &replacement);
        assert!(state.focus_repair_status().is_none());
    }

    /// B1b: clearing the subject's slot under an open detail entry leaves the
    /// surface. The subject names an *exact* slot, so a cleared slot leaves it
    /// naming nothing — and a neighbouring occupant is deliberately not
    /// substituted.
    #[test]
    fn clearing_the_subject_slot_under_an_open_detail_entry_leaves_the_surface() {
        let mut state = gapped_effects_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        let patch_id = state.patches()[0].id();
        let occupied_slot = EffectSlotIndex::new(1).unwrap();
        let origin =
            FocusPath::patch_main(patch_id, None, PatchControlId::EffectSlot(occupied_slot));
        state.interaction.active_focus = origin.clone();

        // Request the clear, then open detail on the still-occupied slot.
        state
            .apply(AppEvent::SetSlotOccupancy {
                patch_id,
                slot: occupied_slot,
                entry: None,
            })
            .unwrap();
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        assert_eq!(
            state
                .interaction()
                .detail_subject()
                .and_then(crate::control::PatchDetailSubject::slot_id),
            Some(EffectSlotId::new(2).unwrap())
        );

        let correlation = state.engine_selection().correlation().unwrap().clone();
        let source = correlation.source_graph_revision();
        advance_engine_to_preparing(&mut state);
        state
            .apply(AppEvent::TopologyPrepared {
                request_id: correlation.request_id(),
                intent: correlation.intent().clone(),
                source_graph_revision: source,
                target_graph_revision: source.checked_next().unwrap(),
            })
            .unwrap();

        assert!(state.patches()[0].effect_slot(occupied_slot).is_some());
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchDetail);
        state
            .apply(AppEvent::EngineActivationAcknowledged {
                request_id: correlation.request_id(),
                intent: correlation.intent().clone(),
                target_graph_revision: source.checked_next().unwrap(),
                retired_graph_revision: source,
                collected: true,
            })
            .unwrap();

        assert!(
            state.patches()[0].effect_slot(occupied_slot).is_none(),
            "the commit must actually have cleared the slot"
        );
        assert_eq!(state.interaction().detail_subject(), None);
        assert_eq!(state.interaction().return_path(), None);
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchMain);
        assert_eq!(state.interaction().focus_path(), &origin);
        assert!(state.interaction().detail_invariant_holds());
        crate::control::StateProjector::new()
            .project_with_shell_tree(&state)
            .expect("the state after leaving must project");
    }

    /// T012: a patch switch closes any open detail surface, lands on a valid
    /// destination path, reprojects the Patch-local Utility values, leaves the
    /// non-Patch-local one alone, and advances the generation exactly once.
    #[test]
    fn a_patch_switch_closes_the_detail_surface_and_reprojects_patch_local_values() {
        let mut state = installed_state();
        // Set after installation: installation seeds every limit from its own
        // engine's ceiling, so a distinguishing value has to be written on top
        // of that seed rather than before it.
        state.patches[1].set_voice_limit(21).unwrap();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
            .apply(AppEvent::EnterSurface(SurfaceId::PatchDetail))
            .unwrap();
        assert!(state.interaction().detail_subject().is_some());

        let master_before = state.global().master_gain_db();
        let generation_before = state.generation();

        state
            .apply(AppEvent::SelectPatch(Direction::Right))
            .unwrap();

        // The detail surface closed and the three facts moved together.
        assert_eq!(state.interaction().detail_subject(), None);
        assert_eq!(state.interaction().return_path(), None);
        assert_eq!(state.interaction().active_surface(), SurfaceId::PatchMain);
        assert!(state.interaction().detail_invariant_holds());

        // Exactly one generation.
        assert_eq!(
            state.generation() - generation_before,
            1,
            "the whole switch is one advanced generation"
        );

        // The destination path resolves against the destination's own schema.
        assert!(SemanticResolver::new(&state).resolves(state.interaction().focus_path()));
        assert_eq!(
            state.interaction().patch_focus(),
            Some(PatchId::new(2).unwrap())
        );

        // MidiInput and VoiceLimit are Patch-local and follow the switch;
        // master gain is not Patch-local and did not change.
        let focused = state
            .patches()
            .iter()
            .find(|candidate| Some(candidate.id()) == state.interaction().patch_focus())
            .unwrap();
        assert_eq!(focused.voice_limit().value(), 21);
        assert_eq!(focused.channel().value(), 1);
        assert_eq!(state.global().master_gain_db(), master_before);
    }

    /// T012: a switch at either end of the installed order is a typed
    /// unchanged rejection.
    #[test]
    fn a_patch_switch_at_either_end_is_a_typed_unchanged_rejection() {
        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();

        let at_first = state.clone();
        assert_eq!(
            state.apply(AppEvent::SelectPatch(Direction::Left)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, at_first, "a refused switch leaves state identical");

        state
            .apply(AppEvent::SelectPatch(Direction::Right))
            .unwrap();
        let at_last = state.clone();
        assert_eq!(
            state.apply(AppEvent::SelectPatch(Direction::Right)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, at_last);
    }

    /// The shared Overview identities survive a switch between heterogeneous
    /// instrument descriptors without falling back to the first row.
    #[test]
    fn a_switch_between_descriptor_shapes_preserves_the_overview_identity() {
        let braids = BraidsCapability::new().unwrap().default_config().unwrap();
        let mut state = AppState::new(
            crate::adapter::production_instruments::production_capability_registry().unwrap(),
            global_parameters(),
        );
        state
            .apply(AppEvent::InstallPatches(vec![
                patch(1, 0.0),
                Patch::new(
                    PatchId::new(2).unwrap(),
                    "Braids".to_owned(),
                    braids,
                    MidiChannel::new(1).unwrap(),
                    PatchOutput::to_track(MixerTrackId::new(1).unwrap()),
                ),
            ]))
            .unwrap();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();

        for _ in 0..3 {
            state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        }
        let held_control = state.interaction().focus_path().control_id().clone();

        state
            .apply(AppEvent::SelectPatch(Direction::Right))
            .unwrap();

        let landed = state.interaction().focus_path().clone();
        let destination_order = SemanticResolver::new(&state)
            .patch_main_paths(PatchId::new(2).unwrap())
            .unwrap();
        assert!(
            destination_order.contains(&landed),
            "focus must land on a control the destination actually hosts"
        );
        assert_eq!(landed.control_id(), &held_control);
        assert_ne!(
            landed, destination_order[0],
            "the retained third-slot identity is not a first-row fallback"
        );
    }
}
