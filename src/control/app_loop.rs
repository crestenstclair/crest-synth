use crate::control::app_event::AppEvent;
use crate::control::app_state::{AppState, EventRejection, StateAccepted};
use crate::control::event_log::EventLog;
use crate::control::event_record::{EventRecord, EventSource};
use crate::control::state_projector::{StateProjectionError, StateProjector};
use crate::control::state_snapshot::StateSnapshot;
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};

const DEFAULT_EVENT_LOG_CAPACITY: usize = 1024;

/// Observable effects of one accepted application event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchResult {
    accepted: StateAccepted,
    snapshot: StateSnapshot,
    boundary_full: Option<BoundaryFull>,
}

impl DispatchResult {
    /// Returns the reducer event identifying the accepted generation.
    pub const fn accepted(&self) -> StateAccepted {
        self.accepted
    }

    /// Returns the canonical serialization of the accepted state.
    pub const fn snapshot(&self) -> &StateSnapshot {
        &self.snapshot
    }

    /// Returns a rejected audio command when the bounded queue was full.
    ///
    /// Queue saturation is not an EventRejection because AppState was already
    /// accepted and its complete parameter projection was already published.
    pub const fn boundary_full(&self) -> Option<BoundaryFull> {
        self.boundary_full
    }

    /// Reports whether every accepted real-time effect was transferred.
    pub const fn audio_effects_published(&self) -> bool {
        self.boundary_full.is_none()
    }
}

/// The one-way control application service.
///
/// The loop is the only owner that exposes mutation of AppState. Input and view
/// adapters receive only dispatch and immutable projection and observation
/// operations.
pub struct AppLoop<Boundary>
where
    Boundary: ControlAudioBoundary,
{
    state: AppState,
    projector: StateProjector,
    boundary: Boundary,
    current_text: TextProjection,
    current_state_tree: StateTree,
    event_log: EventLog,
}

impl<Boundary> AppLoop<Boundary>
where
    Boundary: ControlAudioBoundary,
{
    /// Creates the loop with a bounded interactive event journal and publishes
    /// the complete startup parameter state.
    pub fn new(
        state: AppState,
        projector: StateProjector,
        boundary: Boundary,
    ) -> Result<Self, StateProjectionError> {
        let event_log = EventLog::new(DEFAULT_EVENT_LOG_CAPACITY)
            .expect("the default event-log capacity is nonzero");
        Self::with_event_log(state, projector, boundary, event_log)
    }

    /// Creates the loop with a caller-sized event journal.
    ///
    /// Deterministic scenes can supply an EventLog large enough for their full
    /// trace, while interactive composition roots use `new`'s bounded default.
    pub fn with_event_log(
        state: AppState,
        projector: StateProjector,
        mut boundary: Boundary,
        event_log: EventLog,
    ) -> Result<Self, StateProjectionError> {
        let (_, current_text, parameters, current_state_tree) =
            projector.project_with_tree(&state)?;
        boundary.publish_parameters(parameters);

        Ok(Self {
            state,
            projector,
            boundary,
            current_text,
            current_state_tree,
            event_log,
        })
    }

    /// Applies one event using the stable source for legacy callers.
    pub fn dispatch(&mut self, event: AppEvent) -> Result<DispatchResult, EventRejection> {
        self.dispatch_from(event, EventSource::System)
    }

    /// Applies one sourced event and publishes effects only after complete acceptance.
    pub fn dispatch_from(
        &mut self,
        event: AppEvent,
        source: EventSource,
    ) -> Result<DispatchResult, EventRejection> {
        let generation_before = self.state.generation();
        let state_hash_before = self.current_state_tree.state_hash().to_owned();

        let outcome = match self.state.apply(event.clone()) {
            Ok(outcome) => outcome,
            Err(rejection) => {
                let record = EventRecord::rejected(
                    self.event_log.next_sequence(),
                    source,
                    &event,
                    generation_before,
                    state_hash_before,
                    generation_before,
                    &self.current_text,
                    rejection,
                )
                .expect("cached projections must describe the current rejected state");
                self.event_log
                    .append(record)
                    .expect("AppLoop must append a contiguous rejected event record");
                return Err(rejection);
            }
        };

        let (snapshot, text, parameters, state_tree) = self
            .projector
            .project_with_tree(&self.state)
            .expect("an accepted AppState must produce coherent projections");
        let accepted = outcome.accepted();
        let audio_command = outcome.audio_command().copied();
        let record = EventRecord::accepted(
            self.event_log.next_sequence(),
            source,
            &event,
            generation_before,
            state_hash_before,
            accepted,
            &snapshot,
            parameters.generation(),
            &text,
            audio_command,
        )
        .expect("accepted reducer output and projections must form one coherent record");

        self.boundary.publish_parameters(parameters);
        let boundary_full =
            audio_command.and_then(|command| self.boundary.push_command(command).err());
        self.current_text = text;
        self.current_state_tree = state_tree;
        self.boundary.collect();
        self.event_log
            .append(record)
            .expect("AppLoop must append a contiguous accepted event record");

        Ok(DispatchResult {
            accepted,
            snapshot,
            boundary_full,
        })
    }

    /// Returns the newest complete immutable text projection.
    pub fn current_text(&self) -> TextProjection {
        self.current_text.clone()
    }

    /// Returns the newest canonical state and projection tree.
    pub fn current_state_tree(&self) -> StateTree {
        self.current_state_tree.clone()
    }

    /// Returns an immutable snapshot of the bounded control event journal.
    pub fn event_log(&self) -> EventLog {
        self.event_log.clone()
    }

    /// Enqueues a bounded system-recovery command without inventing a state
    /// transition. Deterministic verification uses this for the renderer-wide
    /// all-notes-off command, which is distinct from Patch-scoped MIDI.
    pub(crate) fn push_recovery_command(
        &mut self,
        command: crate::real_time::audio_command::AudioCommand,
    ) -> Result<(), BoundaryFull> {
        self.boundary.push_command(command)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppLoop, DispatchResult};
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::app_state::{AppState, EventRejection};
    use crate::control::event_record::{EventOutcome, EventSource};
    use crate::control::state_projector::StateProjector;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug, Default, PartialEq)]
    struct BoundaryObservations {
        parameters: Vec<ParameterSnapshot>,
        commands: Vec<AudioCommand>,
        order: Vec<&'static str>,
        collections: usize,
        reject_commands: bool,
    }

    #[derive(Clone, Debug)]
    struct TestBoundary {
        observations: Arc<Mutex<BoundaryObservations>>,
    }

    impl TestBoundary {
        fn new(observations: Arc<Mutex<BoundaryObservations>>) -> Self {
            Self { observations }
        }
    }

    impl ControlAudioBoundary for TestBoundary {
        fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
            let mut observations = self.observations.lock().unwrap();
            observations.order.push("command");
            if observations.reject_commands {
                Err(BoundaryFull::new(command))
            } else {
                observations.commands.push(command);
                Ok(())
            }
        }

        fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
            let mut observations = self.observations.lock().unwrap();
            observations.order.push("parameters");
            observations.parameters.push(parameters);
        }

        fn collect(&mut self) {
            self.observations.lock().unwrap().collections += 1;
        }
    }

    fn global_parameters() -> GlobalParameters {
        GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap()
    }

    fn patch(id: u32, gain_db: f32) -> Patch {
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            SoundFontInstrument::new(0, (id - 1) as u8, false).unwrap(),
            MidiChannel::new(((id - 1) % 16) as u8).unwrap(),
            ChannelParameters::new(gain_db, 0.0, 0.0, 0.0).unwrap(),
        )
    }

    fn installed_state_with_gains(gains: &[f32]) -> AppState {
        let mut state = AppState::new(global_parameters());
        let patches = gains
            .iter()
            .enumerate()
            .map(|(index, gain_db)| patch(index as u32 + 1, *gain_db))
            .collect();
        state.apply(AppEvent::InstallPatches(patches)).unwrap();
        state
    }

    fn installed_state() -> AppState {
        installed_state_with_gains(&[0.0])
    }

    fn loop_with_state(
        state: AppState,
    ) -> (AppLoop<TestBoundary>, Arc<Mutex<BoundaryObservations>>) {
        let observations = Arc::new(Mutex::new(BoundaryObservations::default()));
        let app_loop = AppLoop::new(
            state,
            StateProjector::new(),
            TestBoundary::new(Arc::clone(&observations)),
        )
        .unwrap();
        (app_loop, observations)
    }

    fn loop_with_observations() -> (AppLoop<TestBoundary>, Arc<Mutex<BoundaryObservations>>) {
        loop_with_state(installed_state())
    }

    #[test]
    fn one_way_control_loop_publishes_one_coherent_edit() {
        let (mut app_loop, observations) = loop_with_observations();
        let initial_text = app_loop.current_text();

        let result = app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        let current_text = app_loop.current_text();
        let observations = observations.lock().unwrap();
        let published = observations.parameters.last().unwrap();

        assert_eq!(result.accepted().generation(), 2);
        assert_eq!(published.generation(), result.accepted().generation());
        assert_eq!(published.patches()[0].parameters().gain_db(), 1.0);
        assert!(result.snapshot().json().contains("\"gainDb\":1.0"));
        assert_eq!(current_text.state_hash(), result.snapshot().hash());
        assert_ne!(current_text, initial_text);
        assert!(current_text.body().contains("> gainDb=1"));
        assert!(observations.commands.is_empty());
        assert!(result.audio_effects_published());
    }

    #[test]
    fn one_way_control_loop_edits_only_the_selected_non_first_patch() {
        let (mut app_loop, observations) =
            loop_with_state(installed_state_with_gains(&[0.0, -12.0]));

        app_loop
            .dispatch(AppEvent::Navigate(Direction::Right))
            .unwrap();
        let result = app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        let observations = observations.lock().unwrap();
        let published = observations.parameters.last().unwrap();

        assert_eq!(published.patches()[0].parameters().gain_db(), 0.0);
        assert_eq!(published.patches()[1].parameters().gain_db(), -11.0);
        assert!(result.snapshot().json().contains("\"gainDb\":0.0"));
        assert!(result.snapshot().json().contains("\"gainDb\":-11.0"));
    }

    #[test]
    fn one_way_control_loop_publishes_parameters_before_midi_command() {
        let (mut app_loop, observations) = loop_with_observations();
        let message = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap();
        let command = AudioCommand::patch_midi(PatchId::new(1).unwrap(), message);

        let result = app_loop
            .dispatch(AppEvent::Midi {
                patch_id: PatchId::new(1).unwrap(),
                message,
            })
            .unwrap();
        let observations = observations.lock().unwrap();

        assert_eq!(
            &observations.order[observations.order.len() - 2..],
            &["parameters", "command"]
        );
        assert_eq!(observations.commands.last(), Some(&command));
        assert!(result.audio_effects_published());
        assert_eq!(observations.collections, 1);
    }

    #[test]
    fn one_way_control_loop_rejection_has_no_effects_or_view_change() {
        let (mut app_loop, observations) = loop_with_observations();
        let initial_text = app_loop.current_text();
        let initial_observations = observations.lock().unwrap().clone();

        let result = app_loop.dispatch(AppEvent::InstallPatches(Vec::new()));

        assert_eq!(result, Err(EventRejection::InstallationClosed));
        assert_eq!(app_loop.current_text(), initial_text);
        assert_eq!(*observations.lock().unwrap(), initial_observations);
        assert_eq!(app_loop.event_log().len(), 1);
    }

    #[test]
    fn one_way_control_loop_boundary_rejection_is_nonfatal() {
        let (mut app_loop, observations) = loop_with_state(installed_state_with_gains(&[
            ChannelParameters::MAX_GAIN_DB,
        ]));
        let initial_observations = observations.lock().unwrap().clone();

        assert_eq!(
            app_loop.dispatch(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(*observations.lock().unwrap(), initial_observations);

        let result = app_loop
            .dispatch(AppEvent::Adjust(Direction::Left))
            .unwrap();
        let log = app_loop.event_log();

        assert_eq!(result.accepted().generation(), 2);
        assert_eq!(
            observations
                .lock()
                .unwrap()
                .parameters
                .last()
                .unwrap()
                .patches()[0]
                .parameters()
                .gain_db(),
            5.0
        );
        assert_eq!(log.len(), 2);
        assert_eq!(log.records()[0].outcome(), EventOutcome::Rejected);
        assert_eq!(log.records()[1].outcome(), EventOutcome::Accepted);
        assert_eq!(
            log.records()[0].state_hash_after(),
            log.records()[1].state_hash_before()
        );
    }

    #[test]
    fn one_way_control_loop_reports_queue_saturation_after_acceptance() {
        let (mut app_loop, observations) = loop_with_observations();
        observations.lock().unwrap().reject_commands = true;
        let message = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            64,
            90,
        )
        .unwrap();
        let command = AudioCommand::patch_midi(PatchId::new(1).unwrap(), message);

        let result: DispatchResult = app_loop
            .dispatch(AppEvent::Midi {
                patch_id: PatchId::new(1).unwrap(),
                message,
            })
            .unwrap();

        assert_eq!(result.boundary_full(), Some(BoundaryFull::new(command)));
        assert!(!result.audio_effects_published());
        assert_eq!(
            observations
                .lock()
                .unwrap()
                .parameters
                .last()
                .unwrap()
                .generation(),
            result.accepted().generation()
        );
    }

    #[test]
    fn control_observation_trace_records_exact_sources_hashes_and_generations() {
        let (mut app_loop, _) = loop_with_observations();
        let initial_tree = app_loop.current_state_tree();

        assert_eq!(
            app_loop.dispatch_from(AppEvent::InstallPatches(Vec::new()), EventSource::DemoScene,),
            Err(EventRejection::InstallationClosed)
        );
        let accepted = app_loop
            .dispatch_from(AppEvent::Adjust(Direction::Right), EventSource::Keyboard)
            .unwrap();

        let log = app_loop.event_log();
        let tree = app_loop.current_state_tree();
        assert_eq!(log.records().len(), 2);
        assert_eq!(log.records()[0].sequence(), 0);
        assert_eq!(log.records()[0].source(), EventSource::DemoScene);
        assert_eq!(log.records()[0].outcome(), EventOutcome::Rejected);
        assert_eq!(
            log.records()[0].generation_before(),
            initial_tree.generation()
        );
        assert_eq!(
            log.records()[0].generation_after(),
            initial_tree.generation()
        );
        assert_eq!(
            log.records()[0].state_hash_before(),
            log.records()[0].state_hash_after()
        );
        assert_eq!(log.records()[1].sequence(), 1);
        assert_eq!(log.records()[1].source(), EventSource::Keyboard);
        assert_eq!(log.records()[1].outcome(), EventOutcome::Accepted);
        assert_eq!(
            log.records()[0].generation_after(),
            log.records()[1].generation_before()
        );
        assert_eq!(
            log.records()[0].state_hash_after(),
            log.records()[1].state_hash_before()
        );
        assert_eq!(tree.generation(), accepted.accepted().generation());
        assert_eq!(tree.state_hash(), accepted.snapshot().hash());
        assert_eq!(
            log.records()[1].parameter_generation(),
            accepted.accepted().generation()
        );
        assert_eq!(
            log.records()[1].projection_state_hash(),
            accepted.snapshot().hash()
        );

        for property in [
            "\"patches\"",
            "\"global\"",
            "\"selection\"",
            "\"projection\"",
            "\"parameters\"",
        ] {
            assert!(tree.json().contains(property));
        }
    }
}
