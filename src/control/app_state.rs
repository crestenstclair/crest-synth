use crate::control::app_event::{AppEvent, Direction};
use crate::control::engine_selection::{
    EngineSelectionEffect, EngineSelectionEffectKind, EngineSelectionFailure,
    EngineSelectionRequestId, EngineSelectionStatus, EngineSelectionStatusKind,
};
use crate::control::interaction_state::{InteractionState, Selection, SelectionSection};
use crate::control::top_level_context::TopLevelContext;
use crate::mixer::channel_parameters::{ChannelParameter, ChannelParameters};
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::GraphRevision;
use crate::synth::instrument_capability::{
    CapabilityError, CapabilityRegistry, ParameterAdjustment,
};
use crate::synth::patch::{Patch, PatchEditableTarget};
use core::fmt;

const GLOBAL_PARAMETER_COUNT: usize = GlobalParameters::surface_descriptor().len();
const MAX_PATCH_COUNT: usize = 16;

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
            ChannelParameters::default(),
        )
    }

    let global = GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5)
        .expect("reducer probe globals are valid");

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

    let mut no_patches = AppState::new(capabilities.clone(), global);
    no_patches
        .interaction
        .set_mixer_selection(Selection::patch(0));
    let no_patch = no_patches
        .apply(AppEvent::Adjust(Direction::Right))
        .expect_err("an invalid Patch selection has no installed Patch");

    let mut invalid_selection = AppState {
        capabilities: capabilities.clone(),
        patches: vec![probe_patch(1, 0, instrument_config)],
        global,
        interaction: InteractionState {
            context: TopLevelContext::Mixer,
            mixer_selection: Selection {
                section: SelectionSection::Patch,
                patch_index: 0,
                parameter_index: usize::MAX,
            },
            patch_focus: Some(crate::kernel::patch_id::PatchId::new(1).unwrap()),
            patch_control_focus: Some(crate::control::PatchControlId::Engine),
        },
        engine_selection: EngineSelectionStatus::ready(GraphRevision::INITIAL),
        last_engine_selection_request_id: EngineSelectionRequestId::NONE,
        generation: 0,
    };
    let invalid_selection = invalid_selection
        .apply(AppEvent::Adjust(Direction::Right))
        .expect_err("an out-of-range parameter index is rejected");

    let invalid_parameter = ChannelParameters::default()
        .with_value(ChannelParameter::GainDb, f32::NAN)
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
    patches: Vec<Patch>,
    global: GlobalParameters,
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

    /// Creates startup state for the exact complete graph revision already prepared.
    pub fn for_graph(
        capabilities: CapabilityRegistry,
        global: GlobalParameters,
        active_graph_revision: GraphRevision,
    ) -> Self {
        Self {
            capabilities,
            patches: Vec::new(),
            global,
            interaction: InteractionState::new(),
            engine_selection: EngineSelectionStatus::ready(active_graph_revision),
            last_engine_selection_request_id: EngineSelectionRequestId::NONE,
            generation: 0,
        }
    }

    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }

    pub const fn capabilities(&self) -> &CapabilityRegistry {
        &self.capabilities
    }

    pub const fn global(&self) -> &GlobalParameters {
        &self.global
    }

    pub const fn selection(&self) -> Selection {
        self.interaction.mixer_selection()
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
            AppEvent::Navigate(direction) => {
                self.require_mixer_context()?;
                self.navigate(direction)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::Adjust(direction) => {
                if self.context() == TopLevelContext::Patch {
                    let engine_selection_effect = self.request_engine_selection(direction)?;
                    Ok(ReducerEffects {
                        audio_command: None,
                        engine_selection_effect: Some(engine_selection_effect),
                    })
                } else {
                    self.adjust(direction)?;
                    Ok(ReducerEffects::default())
                }
            }
            AppEvent::InstallPatches(patches) => {
                self.install_patches(patches)?;
                Ok(ReducerEffects::default())
            }
            AppEvent::Midi { .. } => unreachable!("MIDI is reduced by apply's read-only fast path"),
            AppEvent::EnginePrepared {
                request_id,
                patch_id,
                source_capability_id,
                target_capability_id,
                source_graph_revision,
                target_graph_revision,
                candidate_config,
            } => {
                let engine_selection_effect = self.engine_prepared(
                    request_id,
                    patch_id,
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
                source_capability_id,
                target_capability_id,
                source_graph_revision,
                target_graph_revision,
                failure,
            } => {
                self.engine_preparation_failed(
                    request_id,
                    patch_id,
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
                target_graph_revision,
                retired_graph_revision,
                collected,
            } => {
                let engine_selection_effect = self.engine_activation_acknowledged(
                    request_id,
                    target_graph_revision,
                    retired_graph_revision,
                    collected,
                )?;
                Ok(ReducerEffects {
                    audio_command: None,
                    engine_selection_effect: Some(engine_selection_effect),
                })
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
        for (index, patch) in patches.iter().enumerate() {
            if patches[..index]
                .iter()
                .any(|installed| installed.channel() == patch.channel())
            {
                return Err(EventRejection::DuplicateMidiChannel);
            }
        }

        self.patches = patches;
        let mixer_selection = if self.patches.is_empty() {
            Selection::global()
        } else {
            Selection::patch(0)
        };
        self.interaction.set_mixer_selection(mixer_selection);
        self.interaction
            .initialize_patch_focus(self.patches.first().map(Patch::id));
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
        self.interaction.select_context(context);
        Ok(())
    }

    fn require_mixer_context(&self) -> Result<(), EventRejection> {
        if self.interaction.context() == TopLevelContext::Mixer {
            Ok(())
        } else {
            Err(EventRejection::ActionUnavailableInContext)
        }
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
        let status = EngineSelectionStatus::preparing(
            self.engine_selection.active_graph_revision(),
            request_id,
            patch_id,
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
        source_capability_id: crate::synth::CapabilityId,
        target_capability_id: crate::synth::CapabilityId,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
        candidate_config: crate::synth::InstrumentConfig,
    ) -> Result<EngineSelectionEffect, EventRejection> {
        let correlation = self.pending_correlation(request_id)?.clone();
        if correlation.patch_id() != patch_id
            || correlation.source_capability_id() != &source_capability_id
            || correlation.target_capability_id() != &target_capability_id
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
        if patch.instrument_config().capability_id() != &source_capability_id {
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
        Ok(effect)
    }

    #[allow(clippy::too_many_arguments)]
    fn engine_preparation_failed(
        &mut self,
        request_id: EngineSelectionRequestId,
        patch_id: crate::kernel::patch_id::PatchId,
        source_capability_id: &crate::synth::CapabilityId,
        target_capability_id: &crate::synth::CapabilityId,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
        failure: EngineSelectionFailure,
    ) -> Result<(), EventRejection> {
        let correlation = self.pending_correlation(request_id)?;
        if correlation.patch_id() != patch_id
            || correlation.source_capability_id() != source_capability_id
            || correlation.target_capability_id() != target_capability_id
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
        if correlation.target_graph_revision() != Some(target_graph_revision)
            || correlation.source_graph_revision() != retired_graph_revision
            || !collected
        {
            return Err(EventRejection::MismatchedEngineSelection);
        }
        let patch = self
            .patches
            .iter()
            .find(|patch| patch.id() == correlation.patch_id())
            .ok_or(EventRejection::MismatchedEngineSelection)?;
        if patch.instrument_config().capability_id() != correlation.target_capability_id() {
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
        match direction {
            Direction::Left => self.navigate_section(-1),
            Direction::Right => self.navigate_section(1),
            Direction::Up => self.navigate_parameter(-1),
            Direction::Down => self.navigate_parameter(1),
        }
    }

    fn navigate_section(&mut self, amount: isize) -> Result<(), EventRejection> {
        let section_count = self.patches.len() + 1;
        let selection = self.interaction.mixer_selection();
        let current = match selection.section {
            SelectionSection::Patch => {
                if selection.patch_index >= self.patches.len() {
                    return Err(EventRejection::InvalidSelection);
                }
                selection.patch_index
            }
            SelectionSection::Global => self.patches.len(),
        };
        let next = wrapped_index(current, section_count, amount);

        if next == self.patches.len() {
            let selection = self.interaction.mixer_selection_mut();
            selection.section = SelectionSection::Global;
            selection.parameter_index = selection.parameter_index.min(GLOBAL_PARAMETER_COUNT - 1);
        } else {
            let parameter_count = self.patch_editable_targets(next)?.len();
            let selection = self.interaction.mixer_selection_mut();
            selection.section = SelectionSection::Patch;
            selection.patch_index = next;
            selection.parameter_index = selection.parameter_index.min(parameter_count - 1);
        }
        Ok(())
    }

    fn navigate_parameter(&mut self, amount: isize) -> Result<(), EventRejection> {
        let selection = self.interaction.mixer_selection();
        let count = match selection.section {
            SelectionSection::Patch => {
                if selection.patch_index >= self.patches.len() {
                    return Err(EventRejection::NoPatchesInstalled);
                }
                self.patch_editable_targets(selection.patch_index)?.len()
            }
            SelectionSection::Global => GLOBAL_PARAMETER_COUNT,
        };
        self.interaction.mixer_selection_mut().parameter_index =
            wrapped_index(selection.parameter_index, count, amount);
        Ok(())
    }

    fn adjust(&mut self, direction: Direction) -> Result<(), EventRejection> {
        match self.interaction.mixer_selection().section {
            SelectionSection::Patch => self.adjust_patch(direction),
            SelectionSection::Global => self.adjust_global(direction),
        }
    }

    fn adjust_patch(&mut self, direction: Direction) -> Result<(), EventRejection> {
        let patch_index = self.interaction.mixer_selection().patch_index;
        let parameter_index = self.interaction.mixer_selection().parameter_index;
        let target = self
            .patch_editable_targets(patch_index)?
            .get(parameter_index)
            .cloned()
            .ok_or(EventRejection::InvalidSelection)?;

        match target {
            PatchEditableTarget::Mixer(parameter) => {
                let descriptor = parameter.descriptor();
                let patch = self
                    .patches
                    .get_mut(patch_index)
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                let parameters = patch.parameters();
                let value = adjusted_value(
                    parameters.value(parameter),
                    descriptor.minimum(),
                    descriptor.maximum(),
                    direction,
                    descriptor.fine_step(),
                    descriptor.coarse_step(),
                )?;
                let updated = parameters
                    .with_value(parameter, value)
                    .map_err(|_| EventRejection::InvalidParameterValue)?;
                patch.set_parameters(updated);
                Ok(())
            }
            PatchEditableTarget::Envelope(parameter) => {
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
            PatchEditableTarget::Instrument(parameter_id) => {
                let updated = {
                    let patch = self
                        .patches
                        .get(patch_index)
                        .ok_or(EventRejection::NoPatchesInstalled)?;
                    let descriptor = self
                        .capabilities
                        .descriptor(patch.instrument_config().capability_id())
                        .ok_or(EventRejection::InvalidInstrumentConfig)?;
                    let spec = descriptor
                        .parameter(&parameter_id)
                        .ok_or(EventRejection::InvalidSelection)?;
                    let current = patch
                        .instrument_config()
                        .value(&parameter_id)
                        .ok_or(EventRejection::InvalidInstrumentConfig)?;
                    let adjustment = parameter_adjustment(direction);
                    let value = spec
                        .adjusted_scalar_value(current, adjustment)
                        .map_err(map_scalar_adjustment_error)?;
                    patch
                        .instrument_config()
                        .with_scalar_value(descriptor, &parameter_id, value)
                        .map_err(map_scalar_adjustment_error)?
                };
                self.patches
                    .get_mut(patch_index)
                    .ok_or(EventRejection::NoPatchesInstalled)?
                    .set_instrument_config(updated);
                Ok(())
            }
        }
    }

    fn adjust_global(&mut self, direction: Direction) -> Result<(), EventRejection> {
        let parameter_index = self.interaction.mixer_selection().parameter_index;
        let descriptor = GlobalParameters::surface_descriptor()
            .get(parameter_index)
            .ok_or(EventRejection::InvalidSelection)?;
        let parameter = descriptor.parameter();
        let value = adjusted_value(
            self.global.value(parameter),
            descriptor.minimum(),
            descriptor.maximum(),
            direction,
            descriptor.fine_step(),
            descriptor.coarse_step(),
        )?;
        self.global = self
            .global
            .with_value(parameter, value)
            .map_err(|_| EventRejection::InvalidParameterValue)?;
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

fn wrapped_index(current: usize, count: usize, amount: isize) -> usize {
    debug_assert!(count > 0);
    if amount < 0 {
        (current + count - 1) % count
    } else {
        (current + 1) % count
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
    use crate::adapter::braids_capability::{
        BraidsCapability, BRAIDS_CAPABILITY_ID, BRAIDS_MODEL_PARAMETER_ID,
    };
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID, SOUNDFONT_PROGRAM_PARAMETER_ID,
    };
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::{DescriptorDefaultConfigFactory, InstrumentCapabilityProvider, ParameterId};
    use crate::testing::automatic_midi_test::create_soundfont_config;

    fn provider() -> HiDefSoundFontCapability {
        HiDefSoundFontCapability::new().unwrap()
    }

    fn registry() -> CapabilityRegistry {
        provider().registry().unwrap()
    }

    fn global_parameters() -> GlobalParameters {
        GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap()
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
            ChannelParameters::new(gain_db, 0.0, 0.0, 0.0).unwrap(),
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
            ChannelParameters::new(-3.0, 0.0, 0.0, 0.0).unwrap(),
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
            patch_id: correlation.patch_id(),
            source_capability_id: correlation.source_capability_id().clone(),
            target_capability_id: correlation.target_capability_id().clone(),
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
            patch_id: correlation.patch_id(),
            source_capability_id: correlation.source_capability_id().clone(),
            target_capability_id: correlation.target_capability_id().clone(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision,
            failure,
        }
    }

    #[test]
    fn app_state_section_navigation_wraps_across_global() {
        assert_eq!(wrapped_index(0, 3, -1), 2);
        assert_eq!(wrapped_index(2, 3, 1), 0);
        assert_eq!(wrapped_index(1, 3, 1), 2);
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
        assert_eq!(state.selection(), Selection::global());

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
            state.apply(AppEvent::Navigate(Direction::Down)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(state, before);
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::EngineSelectionUnavailable)
        );
        assert_eq!(state, before);
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
            .unwrap();
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(state.context(), TopLevelContext::Mixer);
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
            ChannelParameters::default(),
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
        let global = *state.global();

        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(state.selection().parameter_index(), 1);
        assert_eq!(state.patches(), patches.as_slice());
        assert_eq!(*state.global(), global);

        state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        assert_eq!(state.selection().section(), SelectionSection::Patch);
        assert_eq!(state.selection().patch_index(), 1);

        state.apply(AppEvent::Navigate(Direction::Right)).unwrap();
        assert_eq!(state.selection().section(), SelectionSection::Global);
        assert_eq!(state.selection().parameter_index(), 1);
    }

    #[test]
    fn app_state_adjusts_exactly_one_value_and_rejects_at_the_bound() {
        let mut state = installed_state();
        let second_patch = state.patches()[1].clone();
        let global = *state.global();

        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(state.patches()[0].parameters().gain_db(), 1.0);
        assert_eq!(state.patches()[0].parameters().pan(), 0.0);
        assert_eq!(state.patches()[1], second_patch);
        assert_eq!(*state.global(), global);

        state.apply(AppEvent::Adjust(Direction::Up)).unwrap();
        assert_eq!(state.patches()[0].parameters().gain_db(), 6.0);

        let accepted = state.clone();
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Up)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(state, accepted);

        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        state.apply(AppEvent::Adjust(Direction::Left)).unwrap();
        assert_eq!(state.patches()[0].parameters().gain_db(), 6.0);
        assert_eq!(state.patches()[0].parameters().pan(), -0.01);
    }

    #[test]
    fn patch_editable_surface_orders_mixer_then_envelope_and_reduces_transactionally() {
        let mut state = installed_state();
        let targets = state.patch_editable_targets(0).unwrap();
        assert_eq!(targets.len(), 8);
        assert!(matches!(
            targets[0],
            PatchEditableTarget::Mixer(crate::mixer::channel_parameters::ChannelParameter::GainDb)
        ));
        assert!(matches!(
            targets[4],
            PatchEditableTarget::Envelope(crate::synth::VoiceEnvelopeParameter::AttackMilliseconds)
        ));

        let original_mixer = *state.patches()[0].parameters();
        let original_config = state.patches()[0].instrument_config().clone();
        let other_patch = state.patches()[1].clone();
        for _ in 0..4 {
            state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        }
        assert_eq!(state.selection().parameter_index(), 4);
        state.apply(AppEvent::Adjust(Direction::Up)).unwrap();

        assert_eq!(state.patches()[0].envelope().attack_milliseconds(), 100.0);
        assert_eq!(*state.patches()[0].parameters(), original_mixer);
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
        assert_eq!(effect.patch_id(), PatchId::new(1).unwrap());
        assert_eq!(effect.source_capability_id().as_str(), HIDEF_CAPABILITY_ID);
        assert_eq!(effect.target_capability_id().as_str(), BRAIDS_CAPABILITY_ID);
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
            source_capability_id: correlation.source_capability_id().clone(),
            target_capability_id: correlation.target_capability_id().clone(),
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
        let mixer = *state.patches()[0].parameters();
        let unrelated = state.patches()[1].clone();
        let original_soundfont = state.patches()[0].instrument_config().clone();
        assert_eq!(
            original_soundfont.value(&ParameterId::new(SOUNDFONT_PROGRAM_PARAMETER_ID).unwrap()),
            Some(&crate::synth::ParameterValue::Stepped(1))
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
        let activating = state.clone();
        assert_eq!(
            state.apply(AppEvent::EngineActivationAcknowledged {
                request_id,
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
        state
            .apply(AppEvent::EngineActivationAcknowledged {
                request_id: reverse_request_id,
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
        assert_eq!(*state.patches()[0].parameters(), mixer);
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
        for _ in 0..8 {
            state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        }
        let model_id = ParameterId::new(BRAIDS_MODEL_PARAMETER_ID).unwrap();
        let model = state.patches()[1]
            .instrument_config()
            .value(&model_id)
            .cloned();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_ne!(
            state.patches()[1].instrument_config().value(&model_id),
            model.as_ref()
        );
        assert_eq!(state.engine_selection().correlation(), Some(&correlation));
        assert_eq!(
            state.engine_selection().kind(),
            EngineSelectionStatusKind::Preparing
        );
    }
}
