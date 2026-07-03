// path: src/loop/scene_step.rs

//! One step of a scripted `Scene` run: apply a single control-plane event,
//! then render some number of audio blocks headlessly so its audible
//! consequences accrue before the next step runs.
//!
//! `AppEvent` is the closed vocabulary a `Scene` speaks. It is defined here,
//! local to `SceneStep`, as the seam through which every existing
//! control-plane event kind (normalized MIDI, mixer navigation, editor
//! navigation, mapped gamepad input) can be scripted uniformly. `SceneStep`
//! never invents a parallel event vocabulary of its own -- each `AppEvent`
//! variant wraps the payload type its owning aggregate/view already uses.

use crate::editor::editor_event::EditorEvent;
use crate::kernel::midi_event::MidiEvent;
use crate::mixer::mixer_view_event::MixerViewEvent;
use crate::shell::gamepad_action::GamepadAction;

/// A single semantic control-plane event a `Scene` can script.
///
/// Each variant wraps the existing, aggregate- or view-owned event payload
/// for that domain, so scripting a scene never requires knowledge beyond
/// what the live app itself already emits.
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// A normalized external MIDI event feeding the audio model.
    Midi(MidiEvent),
    /// A semantic mixer-view navigation/edit event.
    Mixer(MixerViewEvent),
    /// A semantic editor navigation/edit event.
    Editor(EditorEvent),
    /// A mapped gamepad action.
    Gamepad(GamepadAction),
}

/// One step of a `Scene`: apply `event`, then render `render_blocks`
/// headless audio blocks so its audible consequences accrue before the
/// next step in the scene is applied.
///
/// `render_blocks` may be `0` when a step's only purpose is to change
/// control-plane state (e.g. a rapid sequence of navigation events) with no
/// need to observe audio before the next event lands.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneStep {
    event: AppEvent,
    render_blocks: u32,
}

impl SceneStep {
    /// Builds a step that applies `event` and then renders `render_blocks`
    /// headless audio blocks.
    pub fn new(event: AppEvent, render_blocks: u32) -> Self {
        Self {
            event,
            render_blocks,
        }
    }

    /// The event this step applies.
    pub fn event(&self) -> &AppEvent {
        &self.event
    }

    /// The number of headless audio blocks to render after applying the
    /// event.
    pub fn render_blocks(&self) -> u32 {
        self.render_blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::channel_address::ChannelAddress;
    use crate::kernel::channel_address::MidiChannel;
    use crate::kernel::channel_address::MidiGroup;
    use crate::kernel::midi_event_kind::MidiEventKind;
    use crate::kernel::note_id::NoteId;
    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::velocity::Velocity;

    fn sample_midi_event() -> MidiEvent {
        MidiEvent::new(
            ChannelAddress::new(
                MidiChannel::try_new(0).unwrap(),
                MidiGroup::try_new(0).unwrap(),
            ),
            MidiEventKind::NoteOn,
            NoteNumber::try_new(60).unwrap(),
            NoteId::new(1),
            Velocity::try_new(0.8).unwrap(),
        )
    }

    #[test]
    fn new_stores_event_and_render_blocks() {
        let step = SceneStep::new(AppEvent::Midi(sample_midi_event()), 4);
        assert_eq!(step.event(), &AppEvent::Midi(sample_midi_event()));
        assert_eq!(step.render_blocks(), 4);
    }

    #[test]
    fn render_blocks_may_be_zero() {
        let step = SceneStep::new(AppEvent::Mixer(MixerViewEvent::NavUp), 0);
        assert_eq!(step.render_blocks(), 0);
    }

    #[test]
    fn wraps_mixer_editor_and_gamepad_events() {
        let mixer_step = SceneStep::new(AppEvent::Mixer(MixerViewEvent::ToggleFocusedParam), 1);
        assert_eq!(
            mixer_step.event(),
            &AppEvent::Mixer(MixerViewEvent::ToggleFocusedParam)
        );

        let editor_step = SceneStep::new(AppEvent::Editor(EditorEvent::NavDown), 2);
        assert_eq!(editor_step.event(), &AppEvent::Editor(EditorEvent::NavDown));

        let gamepad_step = SceneStep::new(AppEvent::Gamepad(GamepadAction::Select), 3);
        assert_eq!(
            gamepad_step.event(),
            &AppEvent::Gamepad(GamepadAction::Select)
        );
    }

    #[test]
    fn equality_compares_by_event_and_render_blocks() {
        let a = SceneStep::new(AppEvent::Mixer(MixerViewEvent::NavUp), 2);
        let b = SceneStep::new(AppEvent::Mixer(MixerViewEvent::NavUp), 2);
        let c = SceneStep::new(AppEvent::Mixer(MixerViewEvent::NavUp), 3);
        let d = SceneStep::new(AppEvent::Mixer(MixerViewEvent::NavDown), 2);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn steps_are_debug_formattable() {
        let step = SceneStep::new(AppEvent::Mixer(MixerViewEvent::NavUp), 1);
        let formatted = format!("{step:?}");
        assert!(formatted.contains("SceneStep"));
        assert!(formatted.contains("NavUp"));
    }
}
