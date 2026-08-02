use crate::control::app_event::{AppEvent, Direction};
use crate::control::engine_selection::{
    EngineSelectionEffect, EngineSelectionEffectKind, EngineSelectionFailure,
    EngineSelectionRequestId, EngineSelectionStatus, EngineSelectionStatusKind,
    StructuralEditIntent,
};
use crate::control::interaction_state::{InteractionState, Selection, SelectionSection};
use crate::control::top_level_context::TopLevelContext;
use crate::control::{
    FocusPath, MixerControlId, SemanticAction, SemanticControlId, SemanticResolver, SurfaceId,
};
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
use crate::synth::{
    EffectCapabilityError, EffectCapabilityId, EffectCapabilityRegistry, EffectSlotId, ParameterId,
    PostEffectConfig, VoiceEnvelopeParameter,
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

/// Reasons an AppEvent can be rejected without changing AppState.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventRejection {
    InstallationClosed,
    TooManyPatches,
    DuplicateMidiChannel,
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

const EVENT_REJECTION_SURFACE_DESCRIPTOR: [EventRejectionDescriptor; 17] = [
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
        EventRejection::DuplicateMidiChannel,
        "duplicateMidiChannel",
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
            Self::DuplicateMidiChannel => "duplicateMidiChannel",
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
            Self::DuplicateMidiChannel => "installed Patches must use distinct MIDI channels",
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
) -> [EventRejection; 9] {
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

    let mut duplicate = AppState::new(capabilities.clone(), global);
    let duplicate_channel = duplicate
        .apply(AppEvent::InstallPatches(vec![
            probe_patch(1, 0, instrument_config),
            probe_patch(2, 0, instrument_config),
        ]))
        .expect_err("duplicate channels violate installation");

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
        capabilities: capabilities.clone(),
        effects: EffectCapabilityRegistry::default(),
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
        duplicate_channel,
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
    capabilities: CapabilityRegistry,
    effects: EffectCapabilityRegistry,
    patches: Vec<Patch>,
    mixer: MixerState,
    global: GlobalParameters,
    returns: BusReturnBank,
    interaction: InteractionState,
    engine_selection: EngineSelectionStatus,
    last_engine_selection_request_id: EngineSelectionRequestId,
    generation: u64,
}

impl AppState {
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
            capabilities,
            effects,
            patches: Vec::new(),
            mixer: MixerState::default(),
            global,
            returns: BusReturnBank::default(),
            interaction: InteractionState::new(),
            engine_selection: EngineSelectionStatus::ready(active_graph_revision),
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

    pub const fn capabilities(&self) -> &CapabilityRegistry {
        &self.capabilities
    }

    pub const fn effects(&self) -> &EffectCapabilityRegistry {
        &self.effects
    }

    pub const fn global(&self) -> &GlobalParameters {
        &self.global
    }

    pub const fn mixer(&self) -> &MixerState {
        &self.mixer
    }

    /// Returns the canonical eight-return bank in ascending `BusId` order.
    pub const fn bus_returns(&self) -> &BusReturnBank {
        &self.returns
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

    /// Tests one normalized user action against a clone of the exact accepted
    /// reducer state. Availability is therefore pure and cannot drift from
    /// reducer bounds, dependencies, lifecycle, or surface rules.
    pub fn accepts_semantic_action(&self, action: &SemanticAction) -> bool {
        let mut candidate = self.clone();
        candidate.apply_semantic_action(action.clone()).is_ok()
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
                    if self.interaction.mode() != crate::control::InteractionMode::Navigate
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
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let descriptor = self
            .capabilities
            .descriptor(patch.instrument_config().capability_id())
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        Ok(crate::control::PatchControlId::resolve(
            descriptor,
            patch.instrument_config(),
            &self.effects,
            patch.effect_slots(),
        ))
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
                match self.context() {
                    TopLevelContext::Mixer => self.navigate(direction)?,
                    TopLevelContext::Patch => self.navigate_patch_control(direction)?,
                }
                Ok(ReducerEffects::default())
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
            AppEvent::EnterSurface(surface) => {
                self.interaction
                    .enter_surface(surface)
                    .map_err(|_| EventRejection::ActionUnavailableInContext)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::Return => {
                self.interaction
                    .return_to_origin()
                    .map_err(|_| EventRejection::ActionUnavailableInContext)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::InstallPatches(patches) => {
                self.install_patches(patches)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::Midi { .. } => unreachable!("MIDI is reduced by apply's read-only fast path"),
            AppEvent::EnginePrepared {
                request_id,
                patch_id,
                intent,
                source_capability_id,
                target_capability_id,
                source_graph_revision,
                target_graph_revision,
                candidate_config,
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
                )?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(engine_selection_effect),
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
                let engine_selection_effect = self.engine_activation_acknowledged(
                    request_id,
                    &intent,
                    target_graph_revision,
                    retired_graph_revision,
                    collected,
                )?;
                Ok(ReducerEffects {
                    audio_command: None,
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

    fn install_patches(&mut self, patches: Vec<Patch>) -> Result<(), EventRejection> {
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
        for (index, patch) in patches.iter().enumerate() {
            if patches[..index]
                .iter()
                .any(|installed| installed.channel() == patch.channel())
            {
                return Err(EventRejection::DuplicateMidiChannel);
            }
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
    /// Focus is recovered against the destination Patch's own descriptor
    /// schema: the same control is kept when that Patch offers it, otherwise
    /// the first valid control is taken. A control identity the destination
    /// cannot host is never carried across.
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

        let held = self.interaction.patch_control_focus();
        let resolver = SemanticResolver::new(self);
        let candidates = resolver.patch_main_paths(target_id)?;
        let recovered = held
            .and_then(|control| {
                candidates
                    .iter()
                    .find(|path| path.control_id() == &SemanticControlId::Patch(control.clone()))
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
    ) -> Result<EngineSelectionEffect, EventRejection> {
        let old_patch_order = SemanticResolver::new(self).patch_main_paths(patch_id)?;
        let old_mixer_order = SemanticResolver::new(self).mixer_main_paths()?;
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
            .iter_mut()
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
            EngineSelectionEffectKind::CandidateCommitted,
            status
                .correlation()
                .expect("Activating status always owns correlation"),
        )
        .expect("Activating correlation owns a target revision");
        patch.set_instrument_config(candidate_config);
        self.engine_selection = status;
        self.repair_semantic_paths(&old_patch_order, &old_mixer_order)?;
        Ok(effect)
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
        self.engine_selection = self
            .engine_selection
            .failed(failure)
            .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        Ok(())
    }

    fn engine_activation_acknowledged(
        &mut self,
        request_id: EngineSelectionRequestId,
        intent: &StructuralEditIntent,
        target_graph_revision: GraphRevision,
        retired_graph_revision: GraphRevision,
        collected: bool,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        if self.engine_selection.kind() != EngineSelectionStatusKind::Activating {
            return Err(EventRejection::StaleEngineSelection);
        }
        let correlation = self
            .engine_selection
            .correlation()
            .ok_or(EventRejection::StaleEngineSelection)?;
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
        if !self.committed_intent_matches_state(correlation) {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        let effect = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::ActivationAcknowledged,
            correlation,
        )
        .expect("Activating correlation owns a target revision");
        self.engine_selection = self
            .engine_selection
            .acknowledged()
            .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        Ok(effect)
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
            | StructuralEditIntent::ReplaceParameterChoice { .. } => {
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

    /// Commits one prepared occupancy change to canonical state and enters
    /// Activating, exactly as `engine_prepared` commits a candidate config.
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
            StructuralEditIntent::SetSlotOccupancy {
                patch_id,
                slot,
                entry,
            } => {
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
                let patch = self
                    .patches
                    .iter_mut()
                    .find(|patch| patch.id() == *patch_id)
                    .ok_or(EventRejection::MismatchedEngineSelection)?;
                patch
                    .set_slot_occupancy(*slot, occupant)
                    .map_err(|_| EventRejection::MismatchedEngineSelection)?;
                let status = self
                    .engine_selection
                    .activating(target_graph_revision)
                    .map_err(|_| EventRejection::MismatchedEngineSelection)?;
                let effect = EngineSelectionEffect::from_correlation(
                    EngineSelectionEffectKind::CandidateCommitted,
                    status
                        .correlation()
                        .expect("Activating status always owns correlation"),
                )
                .expect("Activating correlation owns a target revision");
                self.engine_selection = status;
                self.repair_semantic_paths(&old_patch_order, &old_mixer_order)?;
                Ok(effect)
            }
            StructuralEditIntent::SetReturnOccupancy { bus, entry } => {
                let old_inspector_order = self.mixer_inspector_order();
                self.returns
                    .set_return_occupancy(&self.effects, *bus, entry.as_ref())
                    .map_err(|_| EventRejection::MismatchedEngineSelection)?;
                let status = self
                    .engine_selection
                    .activating(target_graph_revision)
                    .map_err(|_| EventRejection::MismatchedEngineSelection)?;
                let effect = EngineSelectionEffect::from_correlation(
                    EngineSelectionEffectKind::CandidateCommitted,
                    status
                        .correlation()
                        .expect("Activating status always owns correlation"),
                )
                .expect("Activating correlation owns a target revision");
                self.engine_selection = status;
                self.repair_inspector_focus(old_inspector_order.as_deref())?;
                Ok(effect)
            }
            StructuralEditIntent::ReplaceCapability { .. }
            | StructuralEditIntent::ReplaceParameterChoice { .. } => {
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
        self.engine_selection = self
            .engine_selection
            .failed(failure)
            .map_err(|_| EventRejection::MismatchedEngineSelection)?;
        Ok(())
    }

    /// Verifies acknowledged structure against canonical state: the committed
    /// instrument config for instrument intents, the committed occupancy for
    /// slot and return intents.
    fn committed_intent_matches_state(
        &self,
        correlation: &crate::control::EngineSelectionCorrelation,
    ) -> bool {
        match correlation.intent() {
            StructuralEditIntent::ReplaceCapability { .. }
            | StructuralEditIntent::ReplaceParameterChoice { .. } => {
                let Some(patch_id) = correlation.patch_id() else {
                    return false;
                };
                let Some(patch) = self.patches.iter().find(|patch| patch.id() == patch_id) else {
                    return false;
                };
                config_matches_committed_intent(patch.instrument_config(), correlation.intent())
            }
            StructuralEditIntent::SetSlotOccupancy {
                patch_id,
                slot,
                entry,
            } => {
                let Some(patch) = self.patches.iter().find(|patch| patch.id() == *patch_id) else {
                    return false;
                };
                match (patch.effect_slot(*slot), entry) {
                    (None, None) => true,
                    (Some(config), Some(entry_id)) => {
                        config.capability_id() == entry_id
                            && config.slot_id() == slot.instance_identity()
                    }
                    _ => false,
                }
            }
            StructuralEditIntent::SetReturnOccupancy { bus, entry } => {
                match (self.returns.bus_return(*bus).effect(), entry) {
                    (None, None) => true,
                    (Some(config), Some(entry_id)) => config.capability_id() == entry_id,
                    _ => false,
                }
            }
        }
    }

    fn pending_correlation(
        &self,
        request_id: EngineSelectionRequestId,
    ) -> Result<&crate::control::EngineSelectionCorrelation, EventRejection> {
        if self.engine_selection.kind() != EngineSelectionStatusKind::Preparing {
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
            SurfaceId::PatchMain | SurfaceId::PatchUtility => {
                Err(EventRejection::ActionUnavailableInContext)
            }
        }
    }

    fn navigate_patch_control(&mut self, direction: Direction) -> Result<(), EventRejection> {
        match self.interaction.active_surface() {
            SurfaceId::PatchUtility => {
                if direction == Direction::Left {
                    self.interaction
                        .return_to_origin()
                        .map_err(|_| EventRejection::ActionUnavailableInContext)
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
        if self.interaction.active_surface() == SurfaceId::PatchUtility {
            let SemanticControlId::Patch(crate::control::PatchControlId::Output(parameter)) =
                self.interaction.focus_path().control_id()
            else {
                return Err(EventRejection::InvalidSelection);
            };
            self.adjust_patch_output(*parameter, direction)?;
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
            Some(crate::control::PatchControlId::Output(_)) => {
                Err(EventRejection::InvalidSelection)
            }
            None => Err(EventRejection::NoPatchesInstalled),
        }
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
                SurfaceId::PatchUtility | SurfaceId::MixerInspector => {
                    return Err(EventRejection::InvalidSelection)
                }
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
            self.interaction.replace_return_origin(origin);
        }
        Ok(())
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
    }
}

fn config_matches_committed_intent(
    config: &crate::synth::InstrumentConfig,
    intent: &StructuralEditIntent,
) -> bool {
    match intent {
        StructuralEditIntent::SetSlotOccupancy { .. }
        | StructuralEditIntent::SetReturnOccupancy { .. } => false,
        StructuralEditIntent::ReplaceCapability {
            target_capability_id,
        } => config.capability_id() == target_capability_id,
        StructuralEditIntent::ReplaceParameterChoice {
            capability_id,
            parameter_id,
            choice_id,
        } => {
            config.capability_id() == capability_id
                && matches!(
                    config.value(parameter_id),
                    Some(ParameterValue::Choice(value)) if value == choice_id
                )
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
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
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
        }
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
        assert_eq!(descriptor.len(), 17);
        for (index, entry) in descriptor.iter().enumerate() {
            assert!(!descriptor[..index].iter().any(|prior| prior.rejection()
                == entry.rejection()
                || prior.name() == entry.name()));
        }
        for entry in descriptor {
            let expected_reachability = match entry.rejection() {
                EventRejection::InstallationClosed
                | EventRejection::TooManyPatches
                | EventRejection::DuplicateMidiChannel
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
            EventRejection::DuplicateMidiChannel,
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
            Some(crate::control::PatchControlId::Envelope(
                VoiceEnvelopeParameter::AttackMilliseconds
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

        let navigated = state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(navigated.audio_command(), None);
        assert_eq!(navigated.engine_selection_effect(), None);
        assert_eq!(
            state.interaction().patch_control_focus(),
            Some(crate::control::PatchControlId::Envelope(
                VoiceEnvelopeParameter::AttackMilliseconds
            ))
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
        // After the instrument's StructuralChoice rows, every one of the
        // three slots contributes its occupancy row whether occupied or
        // empty, so the surface always ends on the last occupancy row here.
        assert!(
            controls.contains(&crate::control::PatchControlId::Capability(
                ParameterId::new(SOUNDFONT_PRESET_PARAMETER_ID).unwrap()
            ))
        );
        assert_eq!(
            controls.last(),
            Some(&crate::control::PatchControlId::EffectSlot(
                crate::synth::effect_slot_id::EffectSlotIndex::new(2).unwrap()
            ))
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

        for (parameter_index, descriptor) in crate::synth::VoiceEnvelope::surface_descriptor()
            .iter()
            .enumerate()
        {
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
                for _ in 0..=parameter_index {
                    state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
                }

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
                    for _ in 0..=parameter_index {
                        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
                    }
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

        for (parameter_index, descriptor) in crate::synth::VoiceEnvelope::surface_descriptor()
            .iter()
            .enumerate()
        {
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
            for _ in 0..=parameter_index {
                patch.apply(AppEvent::Navigate(Direction::Down)).unwrap();
            }
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

        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
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
        let start_generation = state.generation();

        let focus = state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(focus.accepted().generation(), start_generation + 1);
        assert_eq!(state.generation(), start_generation + 1);
        assert_eq!(focus.audio_command(), None);
        assert_eq!(focus.engine_selection_effect(), None);
        assert_eq!(state.patches(), patches);
        assert_eq!(state.engine_selection(), &engine_status);

        let adjusted = state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(adjusted.accepted().generation(), start_generation + 2);
        assert_eq!(state.generation(), start_generation + 2);
        assert_eq!(adjusted.audio_command(), None);
        assert_eq!(adjusted.engine_selection_effect(), None);
        assert_eq!(state.engine_selection(), &engine_status);

        let before = state.clone();
        assert_eq!(
            state.apply(AppEvent::Navigate(Direction::Left)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state.generation(), start_generation + 2);
        assert_eq!(state, before);

        let origin = state.interaction().focus_path().clone();
        let entered = state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        assert_eq!(entered.accepted().generation(), start_generation + 3);
        assert_eq!(
            state.interaction().active_surface(),
            SurfaceId::PatchUtility
        );
        state.apply(AppEvent::Return).unwrap();
        assert_eq!(state.generation(), start_generation + 4);
        assert_eq!(state.interaction().focus_path(), &origin);

        let mut endpoint = mixed_state();
        let before = endpoint.clone();
        assert_eq!(
            endpoint.apply(AppEvent::Navigate(Direction::Up)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(endpoint, before);

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
    fn app_state_installation_rejects_duplicate_midi_channels() {
        let mut state = AppState::new(registry(), global_parameters());
        let initial = state.clone();

        assert_eq!(
            state.apply(AppEvent::InstallPatches(vec![
                patch_on_channel(1, 0.0, 3),
                patch_on_channel(2, -3.0, 3),
            ])),
            Err(EventRejection::DuplicateMidiChannel)
        );
        assert_eq!(state, initial);
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
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
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
            EngineSelectionStatusKind::Preparing
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

        for direction in [Direction::Up, Direction::Down] {
            let mut vertical = mixed_state();
            let before = vertical.clone();
            assert_eq!(
                vertical.apply(AppEvent::Adjust(direction)),
                Err(EventRejection::ActionUnavailableInContext)
            );
            assert_eq!(vertical, before);
        }
    }

    #[test]
    fn app_state_engine_failure_and_stale_or_mismatched_outcomes_preserve_source_and_recover() {
        let mut state = mixed_state();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
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
            EngineSelectionStatusKind::Failed
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
        let braids = descriptor_default_config(BRAIDS_CAPABILITY_ID);
        let revision_two = GraphRevision::INITIAL.checked_next().unwrap();
        let committed = state
            .apply(prepared_event(&state, revision_two, braids.clone()))
            .unwrap();
        assert_eq!(
            committed.engine_selection_effect().unwrap().kind(),
            EngineSelectionEffectKind::CandidateCommitted
        );
        assert_eq!(
            state.engine_selection().kind(),
            EngineSelectionStatusKind::Activating
        );
        assert_eq!(
            state.engine_selection().active_graph_revision(),
            GraphRevision::INITIAL
        );
        assert_eq!(state.patches()[0].instrument_config(), &braids);

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
}
