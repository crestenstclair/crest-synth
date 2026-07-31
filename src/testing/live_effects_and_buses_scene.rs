//! The retained `demo-live-effects-and-buses` scene (WP08, FR-019).
//!
//! This is the phase-completion gate: the cumulative live scene inherits the
//! complete sixteen-track routing scene — scalar sweep, engine transitions,
//! semantic tail, and teardown contract — and appends the ordered topology
//! transitions that demonstrate the expandable-effects-and-bus-topology
//! behaviors through semantic actions and `AppState::apply` only: filling all
//! three slots of a Patch one at a time, exchanging two genuinely different
//! effects so the order change is audible, two instances of one registry
//! entry, sends raised toward several of the eight destinations, changing the
//! effect occupying a return, one controlled rejection with a visible reason
//! and uninterrupted audio, and a rerouted Patch whose chain follows it.
//!
//! The demonstration subject is the first installed Patch: the production
//! audio observation correlates the first stem (`primaryPatchId`) and the
//! first active effect chain (`patchEffect`), so its slot grid is the one
//! whose changes are measurable per block. Its startup chorus is cleared
//! first — while a held note rings — and the grid is restored to the startup
//! configuration before the frozen semantic tail, preserving the base
//! scene's structural-restoration teardown contract.
//!
//! Registry entries are always addressed positionally through the installed
//! registry — nothing in this scene names an effect.

use crate::control::app_event::{AppEvent, Direction};
use crate::control::state_tree::StateTree;
use crate::control::{
    InteractionMode, MixerControlId, PatchControlId, SemanticAction, SurfaceId, TopLevelContext,
};
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::mixer::bus_id::BusId;
use crate::mixer::patch_output::PatchOutputParameter;
use crate::synth::effect_slot_id::EffectSlotIndex;
use crate::synth::EffectCapabilityId;
use crate::testing::live_demo_scene::{decode_state_tree, LiveDemoScene, LiveDemoSceneError};
use serde::Serialize;
use std::time::Duration;

/// The retained scene's stable name.
pub const EFFECTS_AND_BUSES_SCENE_NAME: &str = "effects-and-buses-live-demo";

/// Maximum elapsed control time for the cumulative retained scene. The base
/// sixteen-track scene keeps its own bound; this scene appends the topology
/// phase and therefore declares a wider one.
pub const EFFECTS_AND_BUSES_TOTAL_TIMEOUT: Duration = Duration::from_secs(240);

/// The identity a refused topology request names: a namespaced id that is
/// deliberately absent from every registry.
pub const ABSENT_ENTRY_ID: &str = "effect.absent";

/// One support item executed before or after a topology transition: either a
/// semantic event that must be accepted, or a projection verification that
/// must match exactly. Support items are pacing and audibility setup — the
/// measured behavior is always the transition itself.
#[derive(Clone, Debug, PartialEq)]
pub enum LiveTopologySupport {
    Event {
        event: AppEvent,
    },
    /// Navigates the MIXER track selection horizontally, one accepted step
    /// per tick, until the named track is selected.
    FocusMixerTrack {
        track_id: crate::mixer::mixer_track_id::MixerTrackId,
    },
    VerifyMixerFocus {
        control: MixerControlId,
    },
    VerifyPatchFocus {
        control: PatchControlId,
    },
}

/// The causal audio measurement demanded by one topology transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LiveTopologyAudibleWitness {
    /// The sounding Patch's effect stage is active and audibly altering the
    /// signal (slot occupancy, exchange, twin instances, reroute).
    PatchChain,
    /// The shared wet path is audible (return occupancy changes and send
    /// raises toward occupied destinations).
    WetReturn,
    /// The dry output stays audible and finite (rejection continuity,
    /// cleared slots, and clears that silence a wet path).
    DryContinuity,
}

/// One frozen ordered topology transition of the retained scene.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveTopologyTransition {
    id: String,
    /// The occupancy action under proof; `None` for support-driven scalar
    /// transitions (send raises and the reroute).
    action: Option<SemanticAction>,
    /// A focused-control scalar adjustment dispatched as the measured cause
    /// (the chain-follows-reroute transition).
    adjust: Option<Direction>,
    expected_rejection: Option<&'static str>,
    sounding_patch: PatchId,
    channel: MidiChannel,
    probe_note: u8,
    support_before: Vec<LiveTopologySupport>,
    support_after: Vec<LiveTopologySupport>,
    audible: LiveTopologyAudibleWitness,
    /// WP10 voice carry-over: the probe note sounded before the dispatch is
    /// held through preparation, activation, and the audible capture — the
    /// runner must NOT re-sound it after Ready, so the captured witness can
    /// only hold if the sounding voice itself survived the swap.
    hold_note_through_activation: bool,
}

impl LiveTopologyTransition {
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: impl Into<String>,
        action: Option<SemanticAction>,
        adjust: Option<Direction>,
        expected_rejection: Option<&'static str>,
        sounding_patch: PatchId,
        channel: MidiChannel,
        probe_note: u8,
        support_before: Vec<LiveTopologySupport>,
        support_after: Vec<LiveTopologySupport>,
        audible: LiveTopologyAudibleWitness,
    ) -> Self {
        Self {
            id: id.into(),
            action,
            adjust,
            expected_rejection,
            sounding_patch,
            channel,
            probe_note,
            support_before,
            support_after,
            audible,
            hold_note_through_activation: false,
        }
    }

    /// Marks this transition's probe note as held across activation (WP10).
    fn holding_note_through_activation(mut self) -> Self {
        self.hold_note_through_activation = true;
        self
    }

    pub fn identifier(&self) -> &str {
        &self.id
    }

    pub const fn action(&self) -> Option<&SemanticAction> {
        self.action.as_ref()
    }

    pub const fn adjust(&self) -> Option<Direction> {
        self.adjust
    }

    pub const fn expected_rejection(&self) -> Option<&'static str> {
        self.expected_rejection
    }

    pub const fn sounding_patch(&self) -> PatchId {
        self.sounding_patch
    }

    pub const fn channel(&self) -> MidiChannel {
        self.channel
    }

    pub const fn probe_note(&self) -> u8 {
        self.probe_note
    }

    pub fn support_before(&self) -> &[LiveTopologySupport] {
        &self.support_before
    }

    pub fn support_after(&self) -> &[LiveTopologySupport] {
        &self.support_after
    }

    pub const fn audible(&self) -> LiveTopologyAudibleWitness {
        self.audible
    }

    /// Whether the probe note is held — never re-sounded — through
    /// activation, so the audible capture measures voice carry-over (WP10).
    pub const fn hold_note_through_activation(&self) -> bool {
        self.hold_note_through_activation
    }

    /// EventLog budget for this transition: every support event, the probe
    /// note pair, the dispatch, and the worker-side lifecycle events.
    pub(crate) fn event_budget(&self) -> usize {
        self.support_before
            .len()
            .saturating_add(self.support_after.len())
            .saturating_add(8)
    }

    pub(crate) fn probe_note_on(&self) -> AppEvent {
        AppEvent::Midi {
            patch_id: self.sounding_patch,
            message: MidiMessage::try_new(
                self.channel,
                MidiMessageKind::NoteOn,
                self.probe_note,
                112,
            )
            .expect("the frozen topology probe MIDI constants are valid"),
        }
    }

    pub(crate) fn probe_note_off(&self) -> AppEvent {
        AppEvent::Midi {
            patch_id: self.sounding_patch,
            message: MidiMessage::try_new(
                self.channel,
                MidiMessageKind::NoteOff,
                self.probe_note,
                0,
            )
            .expect("the frozen topology probe MIDI constants are valid"),
        }
    }
}

/// Freezes the retained cumulative effects-and-buses scene from the installed
/// state. The base sixteen-track scene supplies the fixture preconditions:
/// exactly three registry entries with the startup chorus occupying the first
/// Patch's first slot, and every other Patch's slot grid empty.
pub fn from_installed_state(tree: &StateTree) -> Result<LiveDemoScene, LiveDemoSceneError> {
    let base = LiveDemoScene::from_installed_state(tree)?;
    let state = decode_state_tree(tree)?;

    // Positional registry entries; the base scene requires exactly three.
    let entries: Vec<EffectCapabilityId> = state
        .effects
        .descriptors()
        .iter()
        .map(|descriptor| descriptor.id().clone())
        .collect();
    if entries.len() < 3 {
        return Err(LiveDemoSceneError::InvalidEffectConfig);
    }
    let (first_entry, second_entry, third_entry) =
        (entries[0].clone(), entries[1].clone(), entries[2].clone());

    // The demonstration subject: the first installed Patch, whose stem and
    // effect chain the production audio observation correlates per block.
    let subject = state
        .patches
        .first()
        .ok_or(LiveDemoSceneError::NoInstalledPatches)?;
    let subject_id = PatchId::new(subject.id).map_err(|_| LiveDemoSceneError::InvalidPatchId)?;
    let subject_channel = MidiChannel::new(subject.channel)
        .map_err(|_| LiveDemoSceneError::InvalidMidiChannel(subject.channel))?;
    let subject_track = subject.output.track_id();
    let absent = EffectCapabilityId::new(ABSENT_ENTRY_ID)
        .map_err(|_| LiveDemoSceneError::InvalidEffectConfig)?;

    let slot = |index: usize| EffectSlotIndex::ALL[index];
    let occupy =
        |slot_index: usize, entry: Option<EffectCapabilityId>| SemanticAction::SetSlotOccupancy {
            patch_id: subject_id,
            slot: slot(slot_index),
            entry,
        };

    // The send-raise support script. The frozen post-scalar MIXER context
    // still selects the subject's track (the base scene restores it), and
    // Inspector entry always focuses the selected track's first indexed
    // send, so the walk below is deterministic and loudly verified.
    // The MIXER context is already selected when this script runs: the base
    // scene's engine phase ends by restoring it, and no earlier topology
    // transition changes context.
    let raised_buses = [BusId::ALL[0], BusId::ALL[1], BusId::ALL[2]];
    let mut raise_sends = vec![
        LiveTopologySupport::FocusMixerTrack {
            track_id: subject_track,
        },
        LiveTopologySupport::Event {
            event: AppEvent::EnterSurface(SurfaceId::MixerInspector),
        },
    ];
    for bus in raised_buses {
        if bus.index() > 0 {
            raise_sends.push(LiveTopologySupport::Event {
                event: AppEvent::Navigate(Direction::Down),
            });
        }
        raise_sends.push(LiveTopologySupport::VerifyMixerFocus {
            control: MixerControlId::Send {
                track_id: subject_track,
                bus,
            },
        });
        raise_sends.push(LiveTopologySupport::Event {
            event: AppEvent::SetInteractionMode(InteractionMode::Adjust),
        });
        for _ in 0..4 {
            raise_sends.push(LiveTopologySupport::Event {
                event: AppEvent::Adjust(Direction::Up),
            });
        }
        raise_sends.push(LiveTopologySupport::Event {
            event: AppEvent::SetInteractionMode(InteractionMode::Navigate),
        });
    }
    raise_sends.push(LiveTopologySupport::Event {
        event: AppEvent::Return,
    });

    // The mirror script lowering the same sends back to their frozen zeros.
    let mut lower_sends = vec![
        LiveTopologySupport::FocusMixerTrack {
            track_id: subject_track,
        },
        LiveTopologySupport::Event {
            event: AppEvent::EnterSurface(SurfaceId::MixerInspector),
        },
    ];
    for bus in raised_buses {
        if bus.index() > 0 {
            lower_sends.push(LiveTopologySupport::Event {
                event: AppEvent::Navigate(Direction::Down),
            });
        }
        lower_sends.push(LiveTopologySupport::VerifyMixerFocus {
            control: MixerControlId::Send {
                track_id: subject_track,
                bus,
            },
        });
        lower_sends.push(LiveTopologySupport::Event {
            event: AppEvent::SetInteractionMode(InteractionMode::Adjust),
        });
        for _ in 0..4 {
            lower_sends.push(LiveTopologySupport::Event {
                event: AppEvent::Adjust(Direction::Down),
            });
        }
        lower_sends.push(LiveTopologySupport::Event {
            event: AppEvent::SetInteractionMode(InteractionMode::Navigate),
        });
    }
    lower_sends.push(LiveTopologySupport::Event {
        event: AppEvent::Return,
    });

    let transitions = vec![
        // FR-003/AS-1.5 (WP10): clearing the startup occupant travels the
        // full lifecycle while the requesting note is HELD — it rings through
        // preparation, survives the block-boundary activation via voice
        // carry-over, and is never re-sounded, so the dry-continuity witness
        // measures the carried voice itself.
        LiveTopologyTransition::new(
            "Slot.startupOccupantCleared",
            Some(occupy(0, None)),
            None,
            None,
            subject_id,
            subject_channel,
            60,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::DryContinuity,
        )
        .holding_note_through_activation(),
        // FR-002/FR-004: fill all three slots, one at a time, each audibly.
        LiveTopologyTransition::new(
            "SlotFill.first",
            Some(occupy(0, Some(first_entry.clone()))),
            None,
            None,
            subject_id,
            subject_channel,
            60,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::PatchChain,
        ),
        LiveTopologyTransition::new(
            "SlotFill.second",
            Some(occupy(1, Some(second_entry.clone()))),
            None,
            None,
            subject_id,
            subject_channel,
            62,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::PatchChain,
        ),
        LiveTopologyTransition::new(
            "SlotFill.third",
            Some(occupy(2, Some(third_entry.clone()))),
            None,
            None,
            subject_id,
            subject_channel,
            64,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::PatchChain,
        ),
        // FR-005/SC-002: exchange two genuinely different effects.
        LiveTopologyTransition::new(
            "SlotOrder.exchangeFirst",
            Some(occupy(0, Some(second_entry.clone()))),
            None,
            None,
            subject_id,
            subject_channel,
            65,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::PatchChain,
        ),
        LiveTopologyTransition::new(
            "SlotOrder.exchangeSecond",
            Some(occupy(1, Some(first_entry.clone()))),
            None,
            None,
            subject_id,
            subject_channel,
            65,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::PatchChain,
        ),
        // FR-006/SC-003: two instances of one registry entry (positions 0
        // and 2 both carry the third entry after this transition).
        LiveTopologyTransition::new(
            "SlotTwin.sameEntry",
            Some(occupy(0, Some(third_entry.clone()))),
            None,
            None,
            subject_id,
            subject_channel,
            67,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::PatchChain,
        ),
        // FR-010/FR-011: raise sends toward several of the eight
        // destinations through the focused Inspector controls.
        LiveTopologyTransition::new(
            "Send.towardDestinations",
            None,
            None,
            None,
            subject_id,
            subject_channel,
            69,
            raise_sends,
            Vec::new(),
            LiveTopologyAudibleWitness::WetReturn,
        ),
        // FR-013: change the effect occupying an already-occupied return.
        LiveTopologyTransition::new(
            "Return.contentChanged",
            Some(SemanticAction::SetReturnOccupancy {
                bus: BusId::ALL[1],
                entry: Some(first_entry.clone()),
            }),
            None,
            None,
            subject_id,
            subject_channel,
            69,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::WetReturn,
        ),
        // FR-013/C-BR-6: occupy a previously empty destination the raised
        // send already feeds.
        LiveTopologyTransition::new(
            "Return.emptyOccupied",
            Some(SemanticAction::SetReturnOccupancy {
                bus: BusId::ALL[2],
                entry: Some(third_entry.clone()),
            }),
            None,
            None,
            subject_id,
            subject_channel,
            69,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::WetReturn,
        ),
        // FR-015/SC-006: one controlled rejection with a visible reason and
        // uninterrupted audio.
        LiveTopologyTransition::new(
            "Topology.refused",
            Some(SemanticAction::SetSlotOccupancy {
                patch_id: subject_id,
                slot: slot(1),
                entry: Some(absent),
            }),
            None,
            Some("invalidEffectConfig"),
            subject_id,
            subject_channel,
            71,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::DryContinuity,
        ),
        // SC-006: a valid change immediately after the rejection succeeds —
        // restoring the changed return's startup occupant.
        LiveTopologyTransition::new(
            "Topology.recoveredAfterRefusal",
            Some(SemanticAction::SetReturnOccupancy {
                bus: BusId::ALL[1],
                entry: Some(third_entry.clone()),
            }),
            None,
            None,
            subject_id,
            subject_channel,
            71,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::WetReturn,
        ),
        // FR-018: reroute the subject Patch through the focused Utility
        // route control and show its chain follows; the mirror adjustment
        // restores the route after the checkpoint.
        LiveTopologyTransition::new(
            "Reroute.chainFollows",
            None,
            Some(Direction::Right),
            None,
            subject_id,
            subject_channel,
            72,
            vec![
                LiveTopologySupport::Event {
                    event: AppEvent::SelectContext(TopLevelContext::Patch),
                },
                LiveTopologySupport::Event {
                    event: AppEvent::EnterSurface(SurfaceId::PatchUtility),
                },
                LiveTopologySupport::Event {
                    event: AppEvent::Navigate(Direction::Down),
                },
                LiveTopologySupport::VerifyPatchFocus {
                    control: PatchControlId::Output(PatchOutputParameter::OutputTrack),
                },
            ],
            vec![
                LiveTopologySupport::Event {
                    event: AppEvent::Adjust(Direction::Left),
                },
                LiveTopologySupport::Event {
                    event: AppEvent::Return,
                },
            ],
            LiveTopologyAudibleWitness::PatchChain,
        ),
        // Restore the subject grid to the startup configuration: the first
        // entry back at its startup position, the other slots cleared, so
        // the frozen teardown's structural-restoration check observes the
        // exact pre-engine baseline.
        LiveTopologyTransition::new(
            "Slot.startupOccupantRestored",
            Some(occupy(0, Some(first_entry.clone()))),
            None,
            None,
            subject_id,
            subject_channel,
            74,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::PatchChain,
        ),
        LiveTopologyTransition::new(
            "Slot.secondCleared",
            Some(occupy(1, None)),
            None,
            None,
            subject_id,
            subject_channel,
            74,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::DryContinuity,
        ),
        LiveTopologyTransition::new(
            "Slot.thirdCleared",
            Some(occupy(2, None)),
            None,
            None,
            subject_id,
            subject_channel,
            74,
            Vec::new(),
            Vec::new(),
            LiveTopologyAudibleWitness::DryContinuity,
        ),
        // Restore the raised sends and the extra destination so the frozen
        // semantic tail observes the documented end state.
        LiveTopologyTransition::new(
            "Return.emptyRestored",
            Some(SemanticAction::SetReturnOccupancy {
                bus: BusId::ALL[2],
                entry: None,
            }),
            None,
            None,
            subject_id,
            subject_channel,
            76,
            vec![LiveTopologySupport::Event {
                event: AppEvent::SelectContext(TopLevelContext::Mixer),
            }],
            lower_sends,
            LiveTopologyAudibleWitness::DryContinuity,
        ),
    ];

    Ok(base.with_topology_extension(
        EFFECTS_AND_BUSES_SCENE_NAME,
        transitions,
        EFFECTS_AND_BUSES_TOTAL_TIMEOUT,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_entry_id_is_namespaced() {
        let id = EffectCapabilityId::new(ABSENT_ENTRY_ID).unwrap();
        assert_eq!(id.as_str(), "effect.absent");
    }
}
