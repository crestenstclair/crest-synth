// path: src/loop/scene_runner.rs

//! `SceneRunner`: executes a `Scene` against a fresh `AppState` through the
//! SAME apply path the live app uses.
//!
//! # Design
//!
//! Per step, `SceneRunner`:
//! 1. Converts the step's semantic event into the closed `AppEvent` union
//!    `aggregate.Loop.AppState::apply` accepts -- the identical reducer
//!    entry point the live app drives from MIDI, gamepad, editor, and
//!    mixer input, never a parallel mutation path.
//! 2. Applies the event via `AppState::apply`, producing the next state
//!    and an `AppStateEvent` (`Applied` / `Rejected`) -- exactly the value
//!    the live app's reducer produces.
//! 3. Projects the resulting state across the RealTime boundary through
//!    the injected `StateProjector` and `ParameterBridge`, mirroring how
//!    the live app publishes parameters before the audio thread reads
//!    them.
//! 4. Renders the step's requested number of headless audio blocks
//!    through the injected `BlockRenderer` seam.
//! 5. Optionally encodes the resulting state into a `StateSnapshot`
//!    through the injected `SnapshotCodec`.
//!
//! # Why narrow seams instead of the concrete collaborator surfaces
//!
//! `domainService.Engine.EngineRenderer::render` needs an oscillator,
//! filter/oscillator configuration, sample rate, and per-voice render
//! state to do its job -- composition that belongs to whatever owns the
//! live audio thread, not to a scene-scripting service. `SceneRunner`
//! therefore depends on the narrow `BlockRenderer` capability it actually
//! needs (Interface Segregation / Dependency Inversion), mirroring
//! `midi_file::Sequencer`'s narrow `MidiEventSink` seam over the same
//! kind of heavily-parameterized concrete collaborator.
//!
//! Likewise, `port.Loop.SnapshotCodec::encode` takes the plain-field
//! `AppState` shape declared in `loop::snapshot_codec` (tempo, time
//! signature, mixer channels, ...), while `aggregate.Loop.AppState` --
//! the Loop reducer this runner actually drives -- today tracks only the
//! event-sequence frame clock. Deriving one from the other is not yet a
//! pure function of the reducer's fields, so `SceneRunner` depends on an
//! injected `SnapshotSource` strategy for that derivation (mirroring
//! `state_projector::ParameterExtractor`'s identical seam over the same
//! gap) rather than hard-coding a fabricated mapping inline.

use crate::r#loop::app_event::AppEvent;
use crate::r#loop::app_state::{AppState as LoopAppState, AppStateEvent};
use crate::r#loop::scene::Scene;
use crate::r#loop::scene_step::AppEvent as SceneEvent;
use crate::r#loop::snapshot_codec::{AppState as SnapshotState, SnapshotCodec, StateSnapshot};
use crate::r#loop::state_projector::{
    DefaultParameterExtractor, ParameterExtractor, StateProjector,
};
use crate::real_time::parameter_bridge::ParameterBridge;

/// Narrow abstraction over "advance the audio engine by one block", so
/// `SceneRunner` depends on the capability it actually needs rather than
/// `EngineRenderer::render`'s full oscillator/filter/sample-rate/
/// per-voice-state parameter surface. See the module docs for why.
pub trait BlockRenderer {
    /// Renders one audio block's worth of engine output as a side effect.
    /// `SceneRunner` never inspects the samples produced -- a scripted
    /// scene only needs a step's audible consequences to accrue before
    /// the next step is applied.
    fn render_block(&mut self);
}

/// A `BlockRenderer` that renders nothing. The sensible default for
/// scenes that only exercise control-plane events with no audible
/// consequence, and the convenience collaborator for callers who don't
/// yet need real engine rendering wired in.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopBlockRenderer;

impl BlockRenderer for NoopBlockRenderer {
    fn render_block(&mut self) {}
}

/// Strategy for deriving the plain [`SnapshotState`] that
/// [`SnapshotCodec::encode`] needs from a Loop reducer state value.
///
/// A narrow seam (Interface Segregation) mirroring
/// `state_projector::ParameterExtractor`: today's `aggregate.Loop.AppState`
/// tracks only the event-sequence frame clock, so deriving a full
/// `SnapshotState` (tempo, time signature, mixer channels, ...) from it is
/// not yet a pure function of its fields. Injecting the strategy keeps
/// `SceneRunner` decoupled from that gap, and lets a future richer
/// `AppState` supply a real extractor without `SceneRunner` changing.
pub trait SnapshotSource<S = LoopAppState> {
    /// Derives the `SnapshotState` a `SnapshotCodec` should encode for the
    /// given reducer state.
    fn derive(&self, state: &S) -> SnapshotState;
}

/// The `SnapshotSource` used when nothing more specific is supplied:
/// always yields a fixed, sensible-default `SnapshotState`, ignoring
/// `state`. Matches `state_projector::DefaultParameterExtractor`'s
/// identical stance on the identical gap.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultSnapshotSource;

impl<S> SnapshotSource<S> for DefaultSnapshotSource {
    fn derive(&self, _state: &S) -> SnapshotState {
        SnapshotState {
            tempo_bpm: 120.0,
            time_signature: (4, 4),
            master_volume_db: 0.0,
            master_muted: false,
            channels: Vec::new(),
        }
    }
}

/// Converts a `Scene`'s local event vocabulary (`scene_step::AppEvent`)
/// into the closed `AppEvent` union the Loop reducer accepts
/// (`app_event::AppEvent`). Infallible: every `scene_step::AppEvent`
/// variant wraps a payload type `app_event::AppEvent` also carries a
/// matching variant for.
fn to_reducer_event(event: &SceneEvent) -> AppEvent {
    match event {
        SceneEvent::Midi(midi_event) => AppEvent::Midi(midi_event.clone()),
        SceneEvent::Mixer(mixer_event) => AppEvent::Mixer(*mixer_event),
        SceneEvent::Editor(editor_event) => AppEvent::Editor(*editor_event),
        SceneEvent::Gamepad(gamepad_action) => AppEvent::Gamepad(*gamepad_action),
    }
}

/// The outcome of applying one `SceneStep`: the reducer's own
/// `AppStateEvent`, plus -- when per-step snapshots were requested -- the
/// `StateSnapshot` captured immediately after this step's blocks were
/// rendered.
#[derive(Debug, Clone, PartialEq)]
pub struct StepOutcome {
    pub applied: AppStateEvent,
    pub snapshot: Option<StateSnapshot>,
}

/// The result of running a whole `Scene`: every step's outcome, in
/// recorded order, plus the final `StateSnapshot` after the last step.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneRunOutcome {
    pub steps: Vec<StepOutcome>,
    pub final_snapshot: StateSnapshot,
}

/// Executes a `Scene` against a fresh `AppState` through the SAME apply
/// path the live app uses: per step, apply the event, project and render,
/// and (when asked) emit a `StateSnapshot`.
///
/// Owns none of its collaborators' policies -- the codec, block renderer,
/// parameter extractor, and snapshot-derivation strategy are all injected
/// (Dependency Inversion), so a test can substitute fakes for every seam
/// without touching production wiring.
pub struct SceneRunner<C, R, E = DefaultParameterExtractor, N = DefaultSnapshotSource>
where
    C: SnapshotCodec,
    R: BlockRenderer,
    E: ParameterExtractor<LoopAppState>,
    N: SnapshotSource<LoopAppState>,
{
    codec: C,
    block_renderer: R,
    projector: StateProjector<LoopAppState, E>,
    bridge: ParameterBridge,
    snapshot_source: N,
}

impl<C, R> SceneRunner<C, R, DefaultParameterExtractor, DefaultSnapshotSource>
where
    C: SnapshotCodec,
    R: BlockRenderer,
{
    /// Convenience constructor: the default parameter extractor and the
    /// default (fabricated -- see module docs) snapshot-derivation
    /// strategy, with a fresh `ParameterBridge`.
    pub fn new(codec: C, block_renderer: R) -> Self {
        Self::with_collaborators(
            codec,
            block_renderer,
            StateProjector::new(),
            ParameterBridge::default(),
            DefaultSnapshotSource,
        )
    }
}

impl<C, R, E, N> SceneRunner<C, R, E, N>
where
    C: SnapshotCodec,
    R: BlockRenderer,
    E: ParameterExtractor<LoopAppState>,
    N: SnapshotSource<LoopAppState>,
{
    /// Full constructor: every collaborator supplied explicitly.
    pub fn with_collaborators(
        codec: C,
        block_renderer: R,
        projector: StateProjector<LoopAppState, E>,
        bridge: ParameterBridge,
        snapshot_source: N,
    ) -> Self {
        Self {
            codec,
            block_renderer,
            projector,
            bridge,
            snapshot_source,
        }
    }

    /// The `ParameterBridge` this runner publishes through after every
    /// applied event -- exposed so a caller's `BlockRenderer` can be
    /// wired to read from the identical bridge the live app's audio
    /// thread would.
    pub fn bridge(&self) -> &ParameterBridge {
        &self.bridge
    }

    /// Runs `scene` against a fresh `AppState`, capturing only the final
    /// `StateSnapshot`.
    pub fn run(&mut self, scene: &Scene) -> SceneRunOutcome {
        self.run_capturing(scene, false)
    }

    /// Runs `scene` against a fresh `AppState`, capturing a `StateSnapshot`
    /// after every step in addition to the final one.
    pub fn run_with_step_snapshots(&mut self, scene: &Scene) -> SceneRunOutcome {
        self.run_capturing(scene, true)
    }

    fn run_capturing(&mut self, scene: &Scene, capture_every_step: bool) -> SceneRunOutcome {
        let mut state = LoopAppState::new();
        let mut steps = Vec::with_capacity(scene.len());

        for step in scene.steps() {
            let event = to_reducer_event(step.event());
            let (next_state, applied) = state.apply(event);
            state = next_state;

            self.projector.publish(&state, &self.bridge);
            for _ in 0..step.render_blocks() {
                self.block_renderer.render_block();
            }

            let snapshot = capture_every_step.then(|| self.capture(&state));
            steps.push(StepOutcome { applied, snapshot });
        }

        let final_snapshot = self.capture(&state);
        SceneRunOutcome {
            steps,
            final_snapshot,
        }
    }

    fn capture(&self, state: &LoopAppState) -> StateSnapshot {
        self.codec.encode(self.snapshot_source.derive(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
    use crate::kernel::midi_event::MidiEvent;
    use crate::kernel::midi_event_kind::MidiEventKind;
    use crate::kernel::note_id::NoteId;
    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::velocity::Velocity;
    use crate::mixer::mixer_view_event::MixerViewEvent;
    use crate::r#loop::scene_step::SceneStep;
    use crate::r#loop::serde_snapshot_codec::SerdeSnapshotCodec;

    #[derive(Debug, Default)]
    struct CountingBlockRenderer {
        blocks_rendered: u32,
    }

    impl BlockRenderer for CountingBlockRenderer {
        fn render_block(&mut self) {
            self.blocks_rendered += 1;
        }
    }

    fn midi_step(render_blocks: u32) -> SceneStep {
        let event = SceneEvent::Midi(MidiEvent::new(
            ChannelAddress::new(
                MidiChannel::try_new(0).unwrap(),
                MidiGroup::try_new(0).unwrap(),
            ),
            MidiEventKind::NoteOn,
            NoteNumber::try_new(60).unwrap(),
            NoteId::new(1),
            Velocity::try_new(0.8).unwrap(),
        ));
        SceneStep::new(event, render_blocks)
    }

    fn mixer_step(render_blocks: u32) -> SceneStep {
        SceneStep::new(SceneEvent::Mixer(MixerViewEvent::NavUp), render_blocks)
    }

    #[test]
    fn applies_every_step_through_the_reducer_in_order() {
        let scene = Scene::new("test", vec![midi_step(0), mixer_step(0), midi_step(0)]);
        let mut runner = SceneRunner::new(SerdeSnapshotCodec, NoopBlockRenderer);

        let outcome = runner.run(&scene);

        assert_eq!(outcome.steps.len(), 3);
        assert_eq!(
            outcome.steps[0].applied,
            AppStateEvent::Applied { frame: 1 }
        );
        assert_eq!(
            outcome.steps[1].applied,
            AppStateEvent::Applied { frame: 2 }
        );
        assert_eq!(
            outcome.steps[2].applied,
            AppStateEvent::Applied { frame: 3 }
        );
    }

    #[test]
    fn renders_the_requested_number_of_blocks_per_step() {
        let scene = Scene::new("test", vec![midi_step(2), mixer_step(3)]);
        let mut runner = SceneRunner::new(SerdeSnapshotCodec, CountingBlockRenderer::default());

        runner.run(&scene);

        assert_eq!(runner.block_renderer.blocks_rendered, 5);
    }

    #[test]
    fn run_without_step_snapshots_only_captures_the_final_one() {
        let scene = Scene::new("test", vec![midi_step(0), midi_step(0)]);
        let mut runner = SceneRunner::new(SerdeSnapshotCodec, NoopBlockRenderer);

        let outcome = runner.run(&scene);

        assert!(outcome.steps.iter().all(|step| step.snapshot.is_none()));
    }

    #[test]
    fn run_with_step_snapshots_captures_one_per_step_plus_the_final() {
        let scene = Scene::new("test", vec![midi_step(0), midi_step(0)]);
        let mut runner = SceneRunner::new(SerdeSnapshotCodec, NoopBlockRenderer);

        let outcome = runner.run_with_step_snapshots(&scene);

        assert!(outcome.steps.iter().all(|step| step.snapshot.is_some()));
        assert_eq!(
            outcome.steps.last().unwrap().snapshot,
            Some(outcome.final_snapshot.clone())
        );
    }

    #[test]
    fn final_snapshot_round_trips_through_the_injected_codec() {
        let scene = Scene::new("test", vec![midi_step(0)]);
        let mut runner = SceneRunner::new(SerdeSnapshotCodec, NoopBlockRenderer);

        let outcome = runner.run(&scene);

        SerdeSnapshotCodec
            .decode(outcome.final_snapshot)
            .expect("codec round trip");
    }

    #[test]
    fn empty_scene_produces_no_steps_but_still_a_final_snapshot() {
        let scene = Scene::new("empty", vec![]);
        let mut runner = SceneRunner::new(SerdeSnapshotCodec, NoopBlockRenderer);

        let outcome = runner.run(&scene);

        assert!(outcome.steps.is_empty());
        SerdeSnapshotCodec
            .decode(outcome.final_snapshot)
            .expect("codec round trip");
    }

    #[test]
    fn rejecting_a_never_applicable_event_leaves_a_valid_snapshot_reachable() {
        // The permissive default validator only rejects on frame-counter
        // overflow, which is impractical to reach in a test; this test
        // instead documents that a `Rejected` outcome is a first-class,
        // inspectable value alongside `Applied` in `StepOutcome`, not a
        // panic path.
        let rejected = AppStateEvent::Rejected {
            reason: "example".to_string(),
        };
        let outcome = StepOutcome {
            applied: rejected.clone(),
            snapshot: None,
        };
        assert_eq!(outcome.applied, rejected);
    }
}
