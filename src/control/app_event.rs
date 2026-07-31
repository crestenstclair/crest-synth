use crate::control::top_level_context::TopLevelContext;
use crate::control::{
    EngineSelectionFailure, EngineSelectionRequestId, InteractionMode, SemanticAction,
    StructuralEditIntent, SurfaceId,
};
use crate::kernel::midi_message::MidiMessage;
use crate::kernel::patch_id::PatchId;
use crate::mixer::bus_id::BusId;
use crate::real_time::GraphRevision;
use crate::synth::effect_slot_id::EffectSlotIndex;
use crate::synth::{CapabilityId, EffectCapabilityId, InstrumentConfig, Patch};
use serde::{Deserialize, Serialize};

/// A semantic direction emitted by an input adapter.
///
/// The meaning of a direction depends on the event variant: navigation moves
/// selection, while adjustment changes the currently selected bounded value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
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
    EngineSelectionRequestId,
    CapabilityId,
    GraphRevision,
    InstrumentConfig,
    EngineSelectionFailure,
    Boolean,
    StructuralEditIntent,
    InteractionMode,
    SurfaceId,
    EffectSlotIndex,
    BusId,
    OptionalEffectEntry,
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
    SetInteractionMode {
        mode: InteractionMode,
    },
    EnterSurface {
        surface: SurfaceId,
    },
    Return,
    InstallPatches {
        patches: AppEventPayloadShape,
    },
    Midi {
        patch_id: AppEventPayloadShape,
        message: AppEventPayloadShape,
    },
    EnginePrepared {
        request_id: AppEventPayloadShape,
        patch_id: AppEventPayloadShape,
        intent: AppEventPayloadShape,
        source_capability_id: AppEventPayloadShape,
        target_capability_id: AppEventPayloadShape,
        source_graph_revision: AppEventPayloadShape,
        target_graph_revision: AppEventPayloadShape,
        candidate_config: AppEventPayloadShape,
    },
    EnginePreparationFailed {
        request_id: AppEventPayloadShape,
        patch_id: AppEventPayloadShape,
        intent: AppEventPayloadShape,
        source_capability_id: AppEventPayloadShape,
        target_capability_id: AppEventPayloadShape,
        source_graph_revision: AppEventPayloadShape,
        target_graph_revision: AppEventPayloadShape,
        failure: AppEventPayloadShape,
    },
    EngineActivationAcknowledged {
        request_id: AppEventPayloadShape,
        intent: AppEventPayloadShape,
        target_graph_revision: AppEventPayloadShape,
        retired_graph_revision: AppEventPayloadShape,
        collected: AppEventPayloadShape,
    },
    SetSlotOccupancy {
        patch_id: AppEventPayloadShape,
        slot: AppEventPayloadShape,
        entry: AppEventPayloadShape,
    },
    SetReturnOccupancy {
        bus: AppEventPayloadShape,
        entry: AppEventPayloadShape,
    },
    TopologyPrepared {
        request_id: AppEventPayloadShape,
        intent: AppEventPayloadShape,
        source_graph_revision: AppEventPayloadShape,
        target_graph_revision: AppEventPayloadShape,
    },
    TopologyPreparationFailed {
        request_id: AppEventPayloadShape,
        intent: AppEventPayloadShape,
        source_graph_revision: AppEventPayloadShape,
        target_graph_revision: AppEventPayloadShape,
        failure: AppEventPayloadShape,
    },
}

const APP_EVENT_SURFACE_DESCRIPTOR: [AppEventSurfaceDescriptor; 24] = [
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
    AppEventSurfaceDescriptor::SetInteractionMode {
        mode: InteractionMode::Navigate,
    },
    AppEventSurfaceDescriptor::SetInteractionMode {
        mode: InteractionMode::Adjust,
    },
    AppEventSurfaceDescriptor::EnterSurface {
        surface: SurfaceId::PatchUtility,
    },
    AppEventSurfaceDescriptor::EnterSurface {
        surface: SurfaceId::MixerInspector,
    },
    AppEventSurfaceDescriptor::Return,
    AppEventSurfaceDescriptor::InstallPatches {
        patches: AppEventPayloadShape::PatchList,
    },
    AppEventSurfaceDescriptor::Midi {
        patch_id: AppEventPayloadShape::PatchId,
        message: AppEventPayloadShape::MidiMessage,
    },
    AppEventSurfaceDescriptor::EnginePrepared {
        request_id: AppEventPayloadShape::EngineSelectionRequestId,
        patch_id: AppEventPayloadShape::PatchId,
        intent: AppEventPayloadShape::StructuralEditIntent,
        source_capability_id: AppEventPayloadShape::CapabilityId,
        target_capability_id: AppEventPayloadShape::CapabilityId,
        source_graph_revision: AppEventPayloadShape::GraphRevision,
        target_graph_revision: AppEventPayloadShape::GraphRevision,
        candidate_config: AppEventPayloadShape::InstrumentConfig,
    },
    AppEventSurfaceDescriptor::EnginePreparationFailed {
        request_id: AppEventPayloadShape::EngineSelectionRequestId,
        intent: AppEventPayloadShape::StructuralEditIntent,
        patch_id: AppEventPayloadShape::PatchId,
        source_capability_id: AppEventPayloadShape::CapabilityId,
        target_capability_id: AppEventPayloadShape::CapabilityId,
        source_graph_revision: AppEventPayloadShape::GraphRevision,
        target_graph_revision: AppEventPayloadShape::GraphRevision,
        failure: AppEventPayloadShape::EngineSelectionFailure,
    },
    AppEventSurfaceDescriptor::EngineActivationAcknowledged {
        request_id: AppEventPayloadShape::EngineSelectionRequestId,
        intent: AppEventPayloadShape::StructuralEditIntent,
        target_graph_revision: AppEventPayloadShape::GraphRevision,
        retired_graph_revision: AppEventPayloadShape::GraphRevision,
        collected: AppEventPayloadShape::Boolean,
    },
    AppEventSurfaceDescriptor::SetSlotOccupancy {
        patch_id: AppEventPayloadShape::PatchId,
        slot: AppEventPayloadShape::EffectSlotIndex,
        entry: AppEventPayloadShape::OptionalEffectEntry,
    },
    AppEventSurfaceDescriptor::SetReturnOccupancy {
        bus: AppEventPayloadShape::BusId,
        entry: AppEventPayloadShape::OptionalEffectEntry,
    },
    AppEventSurfaceDescriptor::TopologyPrepared {
        request_id: AppEventPayloadShape::EngineSelectionRequestId,
        intent: AppEventPayloadShape::StructuralEditIntent,
        source_graph_revision: AppEventPayloadShape::GraphRevision,
        target_graph_revision: AppEventPayloadShape::GraphRevision,
    },
    AppEventSurfaceDescriptor::TopologyPreparationFailed {
        request_id: AppEventPayloadShape::EngineSelectionRequestId,
        intent: AppEventPayloadShape::StructuralEditIntent,
        source_graph_revision: AppEventPayloadShape::GraphRevision,
        target_graph_revision: AppEventPayloadShape::GraphRevision,
        failure: AppEventPayloadShape::EngineSelectionFailure,
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
    /// Select the reducer-owned interpretation of subsequent directions.
    SetInteractionMode(InteractionMode),
    /// Enter one context-compatible persistent side surface.
    EnterSurface(SurfaceId),
    /// Restore the exact main-surface origin of the current side surface.
    Return,
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
    /// Commits one provider-validated candidate after off-callback preparation.
    EnginePrepared {
        request_id: EngineSelectionRequestId,
        patch_id: PatchId,
        intent: StructuralEditIntent,
        source_capability_id: CapabilityId,
        target_capability_id: CapabilityId,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
        candidate_config: InstrumentConfig,
    },
    /// Records one correlated typed preparation failure without adapter detail.
    EnginePreparationFailed {
        request_id: EngineSelectionRequestId,
        patch_id: PatchId,
        intent: StructuralEditIntent,
        source_capability_id: CapabilityId,
        target_capability_id: CapabilityId,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
        failure: EngineSelectionFailure,
    },
    /// Reaches Ready only after activation, retirement, and control collection.
    EngineActivationAcknowledged {
        request_id: EngineSelectionRequestId,
        intent: StructuralEditIntent,
        target_graph_revision: GraphRevision,
        retired_graph_revision: GraphRevision,
        collected: bool,
    },
    /// Requests one Patch effect-slot occupancy change: occupy or replace the
    /// validated position with a registry entry, or clear it with `None`.
    /// Structural: the change enters the prepared-graph lifecycle, never the
    /// scalar snapshot path.
    SetSlotOccupancy {
        patch_id: PatchId,
        slot: EffectSlotIndex,
        entry: Option<EffectCapabilityId>,
    },
    /// Requests one bus-return occupancy change: occupy or replace the
    /// validated bus with a registry entry, or clear it with `None`.
    /// Structural, exactly like a slot change.
    SetReturnOccupancy {
        bus: BusId,
        entry: Option<EffectCapabilityId>,
    },
    /// Commits one prepared occupancy change after off-callback preparation.
    TopologyPrepared {
        request_id: EngineSelectionRequestId,
        intent: StructuralEditIntent,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
    },
    /// Records one correlated typed occupancy refusal without adapter detail.
    TopologyPreparationFailed {
        request_id: EngineSelectionRequestId,
        intent: StructuralEditIntent,
        source_graph_revision: GraphRevision,
        target_graph_revision: GraphRevision,
        failure: EngineSelectionFailure,
    },
}

impl AppEvent {
    /// Performs the single typed user-intent to reducer-event translation.
    pub fn from_semantic_action(action: SemanticAction) -> Self {
        match action {
            SemanticAction::SelectContext(context) => Self::SelectContext(context),
            SemanticAction::Navigate(direction) => Self::Navigate(direction),
            SemanticAction::Adjust(direction) => Self::Adjust(direction),
            SemanticAction::SetInteractionMode(mode) => Self::SetInteractionMode(mode),
            SemanticAction::EnterSurface(surface) => Self::EnterSurface(surface),
            SemanticAction::Return => Self::Return,
            SemanticAction::SetSlotOccupancy {
                patch_id,
                slot,
                entry,
            } => Self::SetSlotOccupancy {
                patch_id,
                slot,
                entry,
            },
            SemanticAction::SetReturnOccupancy { bus, entry } => {
                Self::SetReturnOccupancy { bus, entry }
            }
        }
    }

    /// Reports whether accepting this event publishes a new scalar snapshot
    /// to the real-time boundary.
    ///
    /// Semantic navigation still advances the canonical control generation,
    /// but it cannot alter audio values and therefore must not publish across
    /// any audio transport.
    pub const fn publishes_parameters_on_acceptance(&self) -> bool {
        !matches!(
            self,
            Self::SelectContext(_)
                | Self::Navigate(_)
                | Self::SetInteractionMode(_)
                | Self::EnterSurface(_)
                | Self::Return
        )
    }

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
            Self::SetInteractionMode(mode) => {
                AppEventSurfaceDescriptor::SetInteractionMode { mode: *mode }
            }
            Self::EnterSurface(surface) => {
                AppEventSurfaceDescriptor::EnterSurface { surface: *surface }
            }
            Self::Return => AppEventSurfaceDescriptor::Return,
            Self::InstallPatches(_) => AppEventSurfaceDescriptor::InstallPatches {
                patches: AppEventPayloadShape::PatchList,
            },
            Self::Midi { .. } => AppEventSurfaceDescriptor::Midi {
                patch_id: AppEventPayloadShape::PatchId,
                message: AppEventPayloadShape::MidiMessage,
            },
            Self::EnginePrepared { .. } => AppEventSurfaceDescriptor::EnginePrepared {
                request_id: AppEventPayloadShape::EngineSelectionRequestId,
                patch_id: AppEventPayloadShape::PatchId,
                intent: AppEventPayloadShape::StructuralEditIntent,
                source_capability_id: AppEventPayloadShape::CapabilityId,
                target_capability_id: AppEventPayloadShape::CapabilityId,
                source_graph_revision: AppEventPayloadShape::GraphRevision,
                target_graph_revision: AppEventPayloadShape::GraphRevision,
                candidate_config: AppEventPayloadShape::InstrumentConfig,
            },
            Self::EnginePreparationFailed { .. } => {
                AppEventSurfaceDescriptor::EnginePreparationFailed {
                    request_id: AppEventPayloadShape::EngineSelectionRequestId,
                    patch_id: AppEventPayloadShape::PatchId,
                    intent: AppEventPayloadShape::StructuralEditIntent,
                    source_capability_id: AppEventPayloadShape::CapabilityId,
                    target_capability_id: AppEventPayloadShape::CapabilityId,
                    source_graph_revision: AppEventPayloadShape::GraphRevision,
                    target_graph_revision: AppEventPayloadShape::GraphRevision,
                    failure: AppEventPayloadShape::EngineSelectionFailure,
                }
            }
            Self::EngineActivationAcknowledged { .. } => {
                AppEventSurfaceDescriptor::EngineActivationAcknowledged {
                    request_id: AppEventPayloadShape::EngineSelectionRequestId,
                    intent: AppEventPayloadShape::StructuralEditIntent,
                    target_graph_revision: AppEventPayloadShape::GraphRevision,
                    retired_graph_revision: AppEventPayloadShape::GraphRevision,
                    collected: AppEventPayloadShape::Boolean,
                }
            }
            Self::SetSlotOccupancy { .. } => AppEventSurfaceDescriptor::SetSlotOccupancy {
                patch_id: AppEventPayloadShape::PatchId,
                slot: AppEventPayloadShape::EffectSlotIndex,
                entry: AppEventPayloadShape::OptionalEffectEntry,
            },
            Self::SetReturnOccupancy { .. } => AppEventSurfaceDescriptor::SetReturnOccupancy {
                bus: AppEventPayloadShape::BusId,
                entry: AppEventPayloadShape::OptionalEffectEntry,
            },
            Self::TopologyPrepared { .. } => AppEventSurfaceDescriptor::TopologyPrepared {
                request_id: AppEventPayloadShape::EngineSelectionRequestId,
                intent: AppEventPayloadShape::StructuralEditIntent,
                source_graph_revision: AppEventPayloadShape::GraphRevision,
                target_graph_revision: AppEventPayloadShape::GraphRevision,
            },
            Self::TopologyPreparationFailed { .. } => {
                AppEventSurfaceDescriptor::TopologyPreparationFailed {
                    request_id: AppEventPayloadShape::EngineSelectionRequestId,
                    intent: AppEventPayloadShape::StructuralEditIntent,
                    source_graph_revision: AppEventPayloadShape::GraphRevision,
                    target_graph_revision: AppEventPayloadShape::GraphRevision,
                    failure: AppEventPayloadShape::EngineSelectionFailure,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppEvent, AppEventPayloadShape, AppEventSurfaceDescriptor, Direction};
    use crate::control::{InteractionMode, SurfaceId, TopLevelContext};
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

        assert_eq!(descriptor.len(), 24);
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
        for mode in InteractionMode::PHASE_TWO {
            assert!(descriptor.contains(&AppEventSurfaceDescriptor::SetInteractionMode { mode }));
        }
        for surface in [SurfaceId::PatchUtility, SurfaceId::MixerInspector] {
            assert!(descriptor.contains(&AppEventSurfaceDescriptor::EnterSurface { surface }));
        }
        assert!(descriptor.contains(&AppEventSurfaceDescriptor::Return));
        assert!(
            descriptor.contains(&AppEventSurfaceDescriptor::InstallPatches {
                patches: AppEventPayloadShape::PatchList,
            })
        );
        assert!(descriptor.contains(&AppEventSurfaceDescriptor::Midi {
            patch_id: AppEventPayloadShape::PatchId,
            message: AppEventPayloadShape::MidiMessage,
        }));
        assert!(
            descriptor.contains(&AppEventSurfaceDescriptor::EnginePrepared {
                request_id: AppEventPayloadShape::EngineSelectionRequestId,
                patch_id: AppEventPayloadShape::PatchId,
                intent: AppEventPayloadShape::StructuralEditIntent,
                source_capability_id: AppEventPayloadShape::CapabilityId,
                target_capability_id: AppEventPayloadShape::CapabilityId,
                source_graph_revision: AppEventPayloadShape::GraphRevision,
                target_graph_revision: AppEventPayloadShape::GraphRevision,
                candidate_config: AppEventPayloadShape::InstrumentConfig,
            })
        );
        assert!(
            descriptor.contains(&AppEventSurfaceDescriptor::EnginePreparationFailed {
                request_id: AppEventPayloadShape::EngineSelectionRequestId,
                patch_id: AppEventPayloadShape::PatchId,
                intent: AppEventPayloadShape::StructuralEditIntent,
                source_capability_id: AppEventPayloadShape::CapabilityId,
                target_capability_id: AppEventPayloadShape::CapabilityId,
                source_graph_revision: AppEventPayloadShape::GraphRevision,
                target_graph_revision: AppEventPayloadShape::GraphRevision,
                failure: AppEventPayloadShape::EngineSelectionFailure,
            })
        );
        assert!(
            descriptor.contains(&AppEventSurfaceDescriptor::EngineActivationAcknowledged {
                request_id: AppEventPayloadShape::EngineSelectionRequestId,
                intent: AppEventPayloadShape::StructuralEditIntent,
                target_graph_revision: AppEventPayloadShape::GraphRevision,
                retired_graph_revision: AppEventPayloadShape::GraphRevision,
                collected: AppEventPayloadShape::Boolean,
            })
        );
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
