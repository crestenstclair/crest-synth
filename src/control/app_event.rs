use crate::control::top_level_context::TopLevelContext;
use crate::kernel::midi_message::MidiMessage;
use crate::kernel::patch_id::PatchId;
use crate::synth::patch::Patch;

/// A semantic direction emitted by an input adapter.
///
/// The meaning of a direction depends on the event variant: navigation moves
/// selection, while adjustment changes the currently selected bounded value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Self; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];
}

/// The payload types carried by non-directional application events.
///
/// Keeping these shapes typed avoids a second string-based event schema in
/// deterministic demos and acceptance tests.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppEventPayloadShape {
    PatchList,
    PatchId,
    MidiMessage,
}

/// One typed entry in the exhaustive application-event surface.
///
/// Directional variants have one entry for every direction. The remaining
/// variants retain the complete names and types of their payload fields.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppEventSurfaceDescriptor {
    SelectContext {
        context: TopLevelContext,
    },
    Navigate {
        direction: Direction,
    },
    Adjust {
        direction: Direction,
    },
    InstallPatches {
        patches: AppEventPayloadShape,
    },
    Midi {
        patch_id: AppEventPayloadShape,
        message: AppEventPayloadShape,
    },
}

const APP_EVENT_SURFACE_DESCRIPTOR: [AppEventSurfaceDescriptor; 12] = [
    AppEventSurfaceDescriptor::SelectContext {
        context: TopLevelContext::Patch,
    },
    AppEventSurfaceDescriptor::SelectContext {
        context: TopLevelContext::Mixer,
    },
    AppEventSurfaceDescriptor::Navigate {
        direction: Direction::Up,
    },
    AppEventSurfaceDescriptor::Navigate {
        direction: Direction::Down,
    },
    AppEventSurfaceDescriptor::Navigate {
        direction: Direction::Left,
    },
    AppEventSurfaceDescriptor::Navigate {
        direction: Direction::Right,
    },
    AppEventSurfaceDescriptor::Adjust {
        direction: Direction::Up,
    },
    AppEventSurfaceDescriptor::Adjust {
        direction: Direction::Down,
    },
    AppEventSurfaceDescriptor::Adjust {
        direction: Direction::Left,
    },
    AppEventSurfaceDescriptor::Adjust {
        direction: Direction::Right,
    },
    AppEventSurfaceDescriptor::InstallPatches {
        patches: AppEventPayloadShape::PatchList,
    },
    AppEventSurfaceDescriptor::Midi {
        patch_id: AppEventPayloadShape::PatchId,
        message: AppEventPayloadShape::MidiMessage,
    },
];
/// The closed semantic input union accepted by the application reducer.
///
/// This type intentionally contains only domain values. Raw key codes, window
/// state, clocks, files, and audio devices are normalized by adapters before an
/// event reaches the control layer.
#[derive(Clone, Debug, PartialEq)]
pub enum AppEvent {
    /// Select one of the two reducer-owned top-level contexts directly.
    SelectContext(TopLevelContext),
    /// Move the current selection without changing a synth parameter.
    Navigate(Direction),
    /// Adjust exactly the currently selected bounded parameter.
    Adjust(Direction),
    /// Install the startup patch set in fixture discovery order.
    ///
    /// Whether installation is still permitted is enforced by AppState::apply
    /// on the control thread.
    InstallPatches(Vec<Patch>),
    /// Route one normalized MIDI message to its target patch.
    Midi {
        patch_id: PatchId,
        message: MidiMessage,
    },
}

impl AppEvent {
    /// Returns the unique exhaustive descriptor entries for the closed event union.
    pub const fn surface_descriptor() -> &'static [AppEventSurfaceDescriptor] {
        &APP_EVENT_SURFACE_DESCRIPTOR
    }

    /// Returns the descriptor entry matching this concrete event.
    ///
    /// The exhaustive match intentionally makes a newly added event variant a
    /// compile error until its surface descriptor is updated as well.
    pub fn surface_entry(&self) -> AppEventSurfaceDescriptor {
        match self {
            Self::SelectContext(context) => {
                AppEventSurfaceDescriptor::SelectContext { context: *context }
            }
            Self::Navigate(direction) => AppEventSurfaceDescriptor::Navigate {
                direction: *direction,
            },
            Self::Adjust(direction) => AppEventSurfaceDescriptor::Adjust {
                direction: *direction,
            },
            Self::InstallPatches(_) => AppEventSurfaceDescriptor::InstallPatches {
                patches: AppEventPayloadShape::PatchList,
            },
            Self::Midi { .. } => AppEventSurfaceDescriptor::Midi {
                patch_id: AppEventPayloadShape::PatchId,
                message: AppEventPayloadShape::MidiMessage,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppEvent, AppEventPayloadShape, AppEventSurfaceDescriptor, Direction};
    use crate::control::TopLevelContext;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::MidiMessage;
    use crate::kernel::patch_id::PatchId;
    use crate::synth::patch::Patch;

    #[test]
    fn navigate_and_adjust_retain_their_semantic_direction() {
        let navigate = AppEvent::Navigate(Direction::Up);
        let adjust = AppEvent::Adjust(Direction::Left);

        assert_eq!(navigate, AppEvent::Navigate(Direction::Up));
        assert_eq!(adjust, AppEvent::Adjust(Direction::Left));
        assert_ne!(navigate, adjust);
    }

    #[test]
    fn all_direction_payloads_are_explicit_and_copyable() {
        let directions = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        let copied = directions;

        assert_eq!(copied.len(), 4);
        assert_eq!(
            copied,
            [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ]
        );
    }

    #[test]
    fn surface_descriptor_is_unique_and_exhaustive() {
        let descriptor = AppEvent::surface_descriptor();

        assert_eq!(descriptor.len(), 12);
        for (index, entry) in descriptor.iter().enumerate() {
            assert!(
                !descriptor[..index].contains(entry),
                "duplicate descriptor entry: {entry:?}"
            );
        }

        for context in TopLevelContext::surface_descriptor() {
            assert!(descriptor
                .contains(&AppEventSurfaceDescriptor::SelectContext { context: *context }));
        }

        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            assert!(descriptor.contains(&AppEventSurfaceDescriptor::Navigate { direction }));
            assert!(descriptor.contains(&AppEventSurfaceDescriptor::Adjust { direction }));
        }
        assert!(
            descriptor.contains(&AppEventSurfaceDescriptor::InstallPatches {
                patches: AppEventPayloadShape::PatchList,
            })
        );
        assert!(descriptor.contains(&AppEventSurfaceDescriptor::Midi {
            patch_id: AppEventPayloadShape::PatchId,
            message: AppEventPayloadShape::MidiMessage,
        }));
    }

    #[test]
    fn context_event_preserves_its_semantic_payload() {
        for context in TopLevelContext::surface_descriptor() {
            let event = AppEvent::SelectContext(*context);
            assert_eq!(
                event.surface_entry(),
                AppEventSurfaceDescriptor::SelectContext { context: *context }
            );
        }
    }
    #[test]
    fn install_event_preserves_patch_order_payload() {
        let patches: Vec<Patch> = Vec::new();
        let event = AppEvent::InstallPatches(patches);

        assert_eq!(
            event.surface_entry(),
            AppEventSurfaceDescriptor::InstallPatches {
                patches: AppEventPayloadShape::PatchList,
            }
        );

        match event {
            AppEvent::InstallPatches(installed) => assert!(installed.is_empty()),
            _ => panic!("expected patch installation event"),
        }
    }

    #[test]
    fn midi_event_preserves_its_typed_target_and_message() {
        let patch_id = PatchId::new(7).unwrap();
        let message = MidiMessage::all_notes_off(MidiChannel::new(3).unwrap());
        let event = AppEvent::Midi { patch_id, message };

        assert_eq!(
            event.surface_entry(),
            AppEventSurfaceDescriptor::Midi {
                patch_id: AppEventPayloadShape::PatchId,
                message: AppEventPayloadShape::MidiMessage,
            }
        );
        assert_eq!(event, AppEvent::Midi { patch_id, message });
    }
}
