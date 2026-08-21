use crate::control::{
    AppState, EventRejection, FocusCapabilityId, FocusPath, MixerControlId, PatchChoiceSubject,
    PatchControlId, PatchDetailSubject, PatchSubordinateSession, SemanticAction,
    SemanticActionAvailability, SemanticControlId, SurfaceId, ValidAction,
};
use crate::kernel::PatchId;
use crate::mixer::bus_id::BusId;
use crate::mixer::global_parameters::GlobalParameters;
use crate::mixer::mixer_track_id::{MixerTrackId, MixerTrackId as TrackId};
use crate::mixer::mixer_track_parameters::MixerTrackParameter;
use crate::synth::instrument_capability::{ParameterSpec, ParameterValue};
use crate::synth::{ParameterId, ParameterKind, PatchInteraction};
use std::collections::HashSet;

/// Pure descriptor-backed authority for semantic focus order and recovery.
///
/// The resolver borrows one immutable accepted state. It never owns interaction
/// state, layout data, or runtime objects, and every returned path is expressed
/// only with stable domain identities.
pub struct SemanticResolver<'a> {
    state: &'a AppState,
}

/// One available value in the shared trapped option modal.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedChoiceOption {
    id: String,
    label: String,
    current: bool,
    enabled: bool,
}

impl ResolvedChoiceOption {
    fn enabled(id: impl Into<String>, label: impl Into<String>, current: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            current,
            enabled: true,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn is_current(&self) -> bool {
        self.current
    }

    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Ephemeral resolution of a generic choice subject against canonical state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedChoiceSource {
    subject: PatchChoiceSubject,
    origin_label: String,
    options: Vec<ResolvedChoiceOption>,
}

impl ResolvedChoiceSource {
    pub const fn subject(&self) -> &PatchChoiceSubject {
        &self.subject
    }

    pub fn origin_label(&self) -> &str {
        &self.origin_label
    }

    pub fn options(&self) -> &[ResolvedChoiceOption] {
        &self.options
    }
}

impl<'a> SemanticResolver<'a> {
    pub const fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    /// Derives a generic choice subject from the focused canonical control.
    pub fn choice_subject(&self, path: &FocusPath) -> Option<PatchChoiceSubject> {
        if !matches!(
            path.surface(),
            SurfaceId::PatchMain | SurfaceId::PatchUtility | SurfaceId::PatchDetail
        ) {
            return None;
        }
        let patch_id = path.patch_id()?;
        let SemanticControlId::Patch(control) = path.control_id() else {
            return None;
        };
        let subject = PatchChoiceSubject::new(patch_id, control.clone());
        self.choice_source(&subject).ok().map(|_| subject)
    }

    /// Resolves installed, fixed-domain, or descriptor-owned choices without
    /// storing an option list in interaction state.
    pub fn choice_source(
        &self,
        subject: &PatchChoiceSubject,
    ) -> Result<ResolvedChoiceSource, EventRejection> {
        let patch = self
            .state
            .patches()
            .iter()
            .find(|patch| patch.id() == subject.patch_id())
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let (origin_label, options) = match subject.control_id() {
            PatchControlId::Engine => {
                let current = patch.instrument_config().capability_id();
                (
                    "Instrument".to_owned(),
                    self.state
                        .capabilities()
                        .descriptors()
                        .iter()
                        .map(|descriptor| {
                            ResolvedChoiceOption::enabled(
                                descriptor.id().to_string(),
                                descriptor.label(),
                                descriptor.id() == current,
                            )
                        })
                        .collect(),
                )
            }
            PatchControlId::EffectSlot(slot) => {
                let current = patch
                    .effect_slot(*slot)
                    .map(|effect| effect.capability_id());
                let options: Vec<ResolvedChoiceOption> =
                    core::iter::once(ResolvedChoiceOption::enabled(
                        crate::control::EMPTY_OCCUPANCY_CHOICE_ID,
                        "EMPTY",
                        current.is_none(),
                    ))
                    .chain(self.state.effects().descriptors().iter().map(|descriptor| {
                        ResolvedChoiceOption::enabled(
                            descriptor.id().to_string(),
                            descriptor.label(),
                            current == Some(descriptor.id()),
                        )
                    }))
                    .collect();
                (format!("Effect Slot {}", slot.index() + 1), options)
            }
            PatchControlId::Output(
                crate::mixer::patch_output::PatchOutputParameter::OutputTrack,
            ) => {
                let current = patch.output().track_id();
                (
                    "Output Track".to_owned(),
                    MixerTrackId::ALL
                        .into_iter()
                        .map(|track| {
                            let id = track.to_string();
                            ResolvedChoiceOption::enabled(id.clone(), id, track == current)
                        })
                        .collect(),
                )
            }
            PatchControlId::Capability(parameter_id) => {
                let descriptor = self
                    .state
                    .capabilities()
                    .descriptor(patch.instrument_config().capability_id())
                    .ok_or(EventRejection::InvalidInstrumentConfig)?;
                let spec = descriptor
                    .parameter(parameter_id)
                    .filter(|spec| spec.kind() == ParameterKind::Choice)
                    .ok_or(EventRejection::InvalidSelection)?;
                let current = match patch.instrument_config().value(parameter_id) {
                    Some(ParameterValue::Choice(current)) => current.as_str(),
                    _ => return Err(EventRejection::InvalidInstrumentConfig),
                };
                (
                    spec.label().to_owned(),
                    spec.choices()
                        .iter()
                        .map(|choice| {
                            ResolvedChoiceOption::enabled(
                                choice.id(),
                                choice.label(),
                                choice.id() == current,
                            )
                        })
                        .collect(),
                )
            }
            PatchControlId::Effect(slot_id, parameter_id) => {
                let config = patch
                    .effect_slots()
                    .iter()
                    .flatten()
                    .find(|config| config.slot_id() == *slot_id)
                    .ok_or(EventRejection::InvalidEffectConfig)?;
                let descriptor = self
                    .state
                    .effects()
                    .descriptor(config.capability_id())
                    .ok_or(EventRejection::InvalidEffectConfig)?;
                let spec = descriptor
                    .parameter(parameter_id)
                    .filter(|spec| spec.kind() == ParameterKind::Choice)
                    .ok_or(EventRejection::InvalidSelection)?;
                let current = match config.value(parameter_id) {
                    Some(ParameterValue::Choice(current)) => current.as_str(),
                    _ => return Err(EventRejection::InvalidEffectConfig),
                };
                (
                    spec.label().to_owned(),
                    spec.choices()
                        .iter()
                        .map(|choice| {
                            ResolvedChoiceOption::enabled(
                                choice.id(),
                                choice.label(),
                                choice.id() == current,
                            )
                        })
                        .collect(),
                )
            }
            PatchControlId::Output(_)
            | PatchControlId::Envelope(_)
            | PatchControlId::Global(_)
            | PatchControlId::MidiInput
            | PatchControlId::VoiceLimit => return Err(EventRejection::InvalidSelection),
        };
        if options.is_empty() {
            return Err(EventRejection::ActionUnavailableInContext);
        }
        Ok(ResolvedChoiceSource {
            subject: subject.clone(),
            origin_label,
            options,
        })
    }

    pub fn patch_choice_paths(
        &self,
        subject: &PatchChoiceSubject,
    ) -> Result<Vec<FocusPath>, EventRejection> {
        let source = self.choice_source(subject)?;
        let paths = source
            .options()
            .iter()
            .filter(|option| option.is_enabled())
            .map(|option| {
                FocusPath::patch_choice(
                    subject.patch_id(),
                    subject.stable_id(),
                    option.id().to_owned(),
                )
            })
            .collect::<Vec<_>>();
        ensure_unique(&paths)?;
        Ok(paths)
    }

    pub fn sample_browser_paths(&self) -> Result<Vec<FocusPath>, EventRejection> {
        let (patch_id, parameter_id) = match self.state.interaction().subordinate_session() {
            Some(PatchSubordinateSession::SampleBrowser {
                patch_id,
                asset_parameter_id,
                ..
            }) => (*patch_id, asset_parameter_id),
            _ => return Err(EventRejection::ActionUnavailableInContext),
        };
        if self.state.sample_browser().patch_id() != Some(patch_id)
            || self.state.sample_browser().asset_parameter_id() != Some(parameter_id)
        {
            return Err(EventRejection::InvalidSelection);
        }
        let paths = self
            .state
            .sample_browser()
            .rows()
            .iter()
            .map(|row| {
                FocusPath::sample_browser(
                    patch_id,
                    parameter_id.as_str().to_owned(),
                    row.id().to_owned(),
                )
            })
            .collect::<Vec<_>>();
        ensure_unique(&paths)?;
        Ok(paths)
    }

    /// Returns PATCH Main's canonical focus order for one installed Patch.
    pub fn patch_main_paths(&self, patch_id: PatchId) -> Result<Vec<FocusPath>, EventRejection> {
        let patch = self
            .state
            .patches()
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let descriptor = self
            .state
            .capabilities()
            .descriptor(patch.instrument_config().capability_id())
            .ok_or(EventRejection::InvalidInstrumentConfig)?;
        let controls = PatchControlId::resolve(
            descriptor,
            patch.instrument_config(),
            self.state.effects(),
            patch.effect_slots(),
        );
        let mut paths = Vec::with_capacity(controls.len());
        for control in controls {
            let capability_id = match &control {
                // Engine, common ADSR, and positional occupancy identities
                // survive an instrument or occupancy swap.
                PatchControlId::Engine
                | PatchControlId::Envelope(_)
                | PatchControlId::EffectSlot(_) => None,
                // The five Utility identities never enter the PatchMain order.
                // `resolve` cannot produce one, so reaching here means a
                // descriptor claimed a row that belongs to the other surface.
                PatchControlId::Output(_)
                | PatchControlId::Global(_)
                | PatchControlId::MidiInput
                | PatchControlId::VoiceLimit => {
                    return Err(EventRejection::InvalidSelection);
                }
                PatchControlId::Capability(_) => Some(FocusCapabilityId::Instrument(
                    patch.instrument_config().capability_id().clone(),
                )),
                PatchControlId::Effect(slot_id, _) => {
                    // The occupant is found by stable instance identity over
                    // the per-position chain; empty positions stay in place
                    // and are simply skipped.
                    let effect = patch
                        .effect_slots()
                        .iter()
                        .flatten()
                        .find(|effect| effect.slot_id() == *slot_id)
                        .ok_or(EventRejection::InvalidEffectConfig)?;
                    Some(FocusCapabilityId::Effect(effect.capability_id().clone()))
                }
            };
            paths.push(FocusPath::patch_main(patch_id, capability_id, control));
        }
        ensure_unique(&paths)?;
        Ok(paths)
    }

    /// Derives the detail subject one PatchMain path opens, or `None`.
    ///
    /// The engine row and every active-instrument capability row resolve
    /// `Instrument`; an *occupied* effect slot and its occupant's parameter
    /// rows resolve `Effect` carrying that slot's exact identity, so two
    /// positions holding the same registry entry are distinct subjects. Every
    /// other path resolves `None` — an empty slot, an envelope row, and every
    /// Utility row included — because a subject-less detail surface would be
    /// an empty shell rather than a place to be.
    pub fn detail_subject(&self, path: &FocusPath) -> Option<PatchDetailSubject> {
        if path.surface() != SurfaceId::PatchMain {
            return None;
        }
        let patch = self
            .state
            .patches()
            .iter()
            .find(|patch| Some(patch.id()) == path.patch_id())?;
        let SemanticControlId::Patch(control) = path.control_id() else {
            return None;
        };
        match control {
            PatchControlId::Engine | PatchControlId::Capability(_) => Some(
                PatchDetailSubject::instrument(patch.instrument_config().capability_id().clone()),
            ),
            // The occupancy row names a position; only an occupied one names a
            // capability. An empty slot is deliberately not repaired into a
            // neighbouring subject.
            PatchControlId::EffectSlot(index) => {
                let occupant = patch.effect_slot(*index)?;
                Some(PatchDetailSubject::effect(
                    occupant.slot_id(),
                    occupant.capability_id().clone(),
                ))
            }
            PatchControlId::Effect(slot_id, _) => {
                let occupant = patch
                    .effect_slots()
                    .iter()
                    .flatten()
                    .find(|effect| effect.slot_id() == *slot_id)?;
                Some(PatchDetailSubject::effect(
                    occupant.slot_id(),
                    occupant.capability_id().clone(),
                ))
            }
            PatchControlId::Envelope(_)
            | PatchControlId::Output(_)
            | PatchControlId::Global(_)
            | PatchControlId::MidiInput
            | PatchControlId::VoiceLimit => None,
        }
    }

    /// Reports whether a detail subject still names something live on the
    /// Patch it was opened on.
    ///
    /// An `Instrument` subject is live only while it *is* the Patch's active
    /// instrument capability; an `Effect` subject only while its exact slot is
    /// still occupied by that exact registry entry. Deliberately not the same
    /// question as [`Self::resolves`]: the detail order is resolved from the
    /// subject's own capability id, so a subject left behind by an engine swap
    /// still resolves to a full row list — one belonging to a capability the
    /// Patch no longer has.
    pub fn detail_subject_is_live(&self, patch_id: PatchId, subject: &PatchDetailSubject) -> bool {
        let Some(patch) = self
            .state
            .patches()
            .iter()
            .find(|patch| patch.id() == patch_id)
        else {
            return false;
        };
        match subject {
            PatchDetailSubject::Instrument { capability_id } => {
                patch.instrument_config().capability_id() == capability_id
            }
            PatchDetailSubject::Effect {
                slot_id,
                capability_id,
            } => patch.effect_slots().iter().flatten().any(|effect| {
                effect.slot_id() == *slot_id && effect.capability_id() == capability_id
            }),
        }
    }

    /// Returns the detail surface's focus order for one subject: the subject
    /// descriptor's visible enabled rows, in descriptor order.
    ///
    /// The rows are resolved from the installed descriptor at call time. The
    /// subject supplies only a capability identity, so nothing here is a
    /// second copy of the schema.
    pub fn patch_detail_paths(
        &self,
        patch_id: PatchId,
        subject: &PatchDetailSubject,
    ) -> Result<Vec<FocusPath>, EventRejection> {
        let patch = self
            .state
            .patches()
            .iter()
            .find(|patch| patch.id() == patch_id)
            .ok_or(EventRejection::NoPatchesInstalled)?;
        let capability_id = subject.focus_capability_id();
        let paths = match subject {
            PatchDetailSubject::Instrument { capability_id: id } => {
                let config = patch.instrument_config();
                let descriptor = self
                    .state
                    .capabilities()
                    .descriptor(id)
                    .ok_or(EventRejection::InvalidInstrumentConfig)?;
                let mut paths = descriptor
                    .parameters()
                    .filter(|spec| row_is_visible_and_enabled(spec, |id| config.value(id)))
                    .map(|spec| {
                        FocusPath::patch_detail(
                            patch_id,
                            capability_id.clone(),
                            PatchControlId::Capability(spec.id().clone()),
                        )
                    })
                    .collect::<Vec<_>>();
                paths.extend(
                    crate::synth::VoiceEnvelope::surface_descriptor()
                        .iter()
                        .map(|parameter| {
                            FocusPath::patch_detail(
                                patch_id,
                                capability_id.clone(),
                                PatchControlId::Envelope(parameter.parameter()),
                            )
                        }),
                );
                paths
            }
            PatchDetailSubject::Effect {
                slot_id,
                capability_id: id,
            } => {
                let occupant = patch
                    .effect_slots()
                    .iter()
                    .flatten()
                    .find(|effect| effect.slot_id() == *slot_id)
                    .ok_or(EventRejection::InvalidEffectConfig)?;
                let descriptor = self
                    .state
                    .effects()
                    .descriptor(id)
                    .ok_or(EventRejection::InvalidEffectConfig)?;
                descriptor
                    .parameters()
                    .filter(|spec| row_is_visible_and_enabled(spec, |id| occupant.value(id)))
                    .map(|spec| {
                        FocusPath::patch_detail(
                            patch_id,
                            capability_id.clone(),
                            PatchControlId::Effect(*slot_id, spec.id().clone()),
                        )
                    })
                    .collect::<Vec<_>>()
            }
        };
        ensure_unique(&paths)?;
        Ok(paths)
    }

    /// Returns the sixteen fixed MIXER Main track sections in identity order.
    pub fn mixer_main_sections(&self) -> Result<Vec<Vec<FocusPath>>, EventRejection> {
        let mut sections = Vec::with_capacity(MixerTrackId::COUNT);
        for track_id in MixerTrackId::ALL {
            let paths = MixerTrackParameter::MAIN
                .into_iter()
                .map(|parameter| FocusPath::mixer_track(track_id, parameter))
                .collect::<Vec<_>>();
            ensure_unique(&paths)?;
            sections.push(paths);
        }
        Ok(sections)
    }

    pub fn mixer_main_paths(&self) -> Result<Vec<FocusPath>, EventRejection> {
        Ok(self.mixer_main_sections()?.into_iter().flatten().collect())
    }

    /// Returns PATCH Utility's canonical focus order: the five declared rows.
    ///
    /// The order comes from [`PatchControlId::utility_surface_descriptor`] —
    /// one written declaration — rather than from iterating any parameter
    /// descriptor, so reordering a descriptor cannot reshuffle the panel.
    pub fn patch_utility_paths(&self, patch_id: PatchId) -> Result<Vec<FocusPath>, EventRejection> {
        if !self
            .state
            .patches()
            .iter()
            .any(|patch| patch.id() == patch_id)
        {
            return Err(EventRejection::NoPatchesInstalled);
        }
        let paths = PatchControlId::utility_surface_descriptor()
            .iter()
            .map(|control| FocusPath::patch_utility(patch_id, control.clone()))
            .collect::<Vec<_>>();
        ensure_unique(&paths)?;
        Ok(paths)
    }

    /// Returns MIXER Inspector's canonical focus order: the selected track's
    /// eight indexed sends in ascending `BusId` order, then each of the eight
    /// bus returns' occupancy row, return level, and — when occupied — the
    /// occupying registry entry's visible enabled `ScalarEdit` rows, then the
    /// distinct global controls (master gain alone).
    pub fn mixer_inspector_paths(
        &self,
        track_id: TrackId,
    ) -> Result<Vec<FocusPath>, EventRejection> {
        let mut paths = BusId::ALL
            .into_iter()
            .map(|bus| FocusPath::mixer_send(track_id, bus))
            .collect::<Vec<_>>();
        for bus in BusId::ALL {
            paths.push(FocusPath::mixer_return_occupancy(bus));
            paths.push(FocusPath::mixer_return_level(bus));
            let bus_return = self.state.bus_returns().bus_return(bus);
            let Some(config) = bus_return.effect() else {
                continue;
            };
            let descriptor = self
                .state
                .effects()
                .descriptor(config.capability_id())
                .ok_or(EventRejection::InvalidEffectConfig)?;
            for spec in descriptor.parameters() {
                if spec.patch_interaction() == PatchInteraction::ScalarEdit
                    && row_is_visible_and_enabled(spec, |id| config.value(id))
                {
                    paths.push(FocusPath::mixer_return_effect(
                        bus,
                        spec.id().clone(),
                        config.capability_id().clone(),
                    ));
                }
            }
        }
        paths.extend(
            GlobalParameters::surface_descriptor()
                .iter()
                .map(|descriptor| FocusPath::mixer_global(descriptor.parameter())),
        );
        ensure_unique(&paths)?;
        Ok(paths)
    }

    fn selected_mixer_track(&self) -> Result<MixerTrackId, EventRejection> {
        self.state
            .interaction()
            .remembered_mixer_main()
            .control_id()
            .as_mixer_track_id()
            .ok_or(EventRejection::InvalidSelection)
    }

    /// Returns the canonical focus order for a main surface. Side surfaces
    /// always contain their single read-only root anchor.
    pub fn ordered_paths(&self, surface: SurfaceId) -> Result<Vec<FocusPath>, EventRejection> {
        match surface {
            SurfaceId::PatchMain => {
                let patch_id = self
                    .state
                    .interaction()
                    .remembered_patch_main()
                    .and_then(FocusPath::patch_id)
                    .or_else(|| self.state.patches().first().map(|patch| patch.id()))
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                self.patch_main_paths(patch_id)
            }
            SurfaceId::MixerMain => self.mixer_main_paths(),
            SurfaceId::PatchUtility => {
                let patch_id = self
                    .state
                    .interaction()
                    .patch_focus()
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                self.patch_utility_paths(patch_id)
            }
            // The detail surface's order belongs to the open subject alone.
            // With no entry open there is no order to resolve — which is
            // exactly the state in which no detail path can be valid.
            SurfaceId::PatchDetail => {
                let interaction = self.state.interaction();
                let subject = interaction
                    .detail_subject()
                    .ok_or(EventRejection::ActionUnavailableInContext)?;
                let patch_id = interaction
                    .patch_focus()
                    .ok_or(EventRejection::NoPatchesInstalled)?;
                self.patch_detail_paths(patch_id, subject)
            }
            SurfaceId::PatchChoice => match self.state.interaction().subordinate_session() {
                Some(PatchSubordinateSession::Choice { subject, .. }) => {
                    self.patch_choice_paths(subject)
                }
                _ => Err(EventRejection::ActionUnavailableInContext),
            },
            SurfaceId::SampleBrowser => self.sample_browser_paths(),
            SurfaceId::MixerInspector => self.mixer_inspector_paths(self.selected_mixer_track()?),
        }
    }

    /// Resolves a path against the exact currently installed schema.
    pub fn resolves(&self, path: &FocusPath) -> bool {
        path.validate().is_ok()
            && self
                .ordered_paths(path.surface())
                .is_ok_and(|paths| paths.iter().any(|candidate| candidate == path))
    }

    /// Returns the ordered duplicate-free action set accepted by the exact
    /// current reducer state.
    ///
    /// The sweep reuses one scratch state rather than cloning twice per
    /// question (see [`SemanticActionAvailability`]). The question asked is
    /// bit-for-bit the one [`AppState::accepts_semantic_action`] asks —
    /// literally the same code — so the per-row answer still cannot drift from
    /// the production reducer.
    pub fn valid_actions(&self) -> Vec<ValidAction> {
        let mut availability = SemanticActionAvailability::new(self.state);
        SemanticAction::surface_descriptor()
            .iter()
            .filter(|action| availability.accepts(action))
            .cloned()
            .map(|action| {
                let (label, hint) = action_presentation(&action);
                ValidAction::new(action, label, hint)
            })
            .collect()
    }

    /// Maps one MIXER stable identity back to compatibility coordinates. The
    /// coordinates never become stored interaction identity.
    pub fn mixer_coordinates(&self, path: &FocusPath) -> Option<(usize, usize)> {
        let SemanticControlId::Mixer(control) = path.control_id() else {
            return None;
        };
        let sections = self.mixer_main_sections().ok()?;
        match control {
            MixerControlId::Track { track_id, .. } => {
                if path.surface() != SurfaceId::MixerMain {
                    return None;
                }
                let section = track_id.index();
                let parameter = sections[section]
                    .iter()
                    .position(|candidate| candidate == path)?;
                Some((section, parameter))
            }
            MixerControlId::Send { .. }
            | MixerControlId::ReturnOccupancy { .. }
            | MixerControlId::ReturnLevel { .. }
            | MixerControlId::ReturnEffect { .. }
            | MixerControlId::Global { .. } => None,
        }
    }

    /// Recovers one old path in old canonical order. Exact stable identities
    /// win; otherwise candidates are searched outward, next before previous on
    /// equal distance, with the first surviving path as the final fallback.
    pub fn recover(
        old_path: &FocusPath,
        old_order: &[FocusPath],
        new_order: &[FocusPath],
    ) -> Option<FocusPath> {
        if !old_order.iter().any(|candidate| candidate == old_path)
            && !new_order.iter().any(|candidate| candidate == old_path)
        {
            return None;
        }
        Self::recovered_index(old_path, old_order, new_order)
            .map(|index| new_order[index].clone())
            .or_else(|| new_order.first().cloned())
    }

    /// The one deterministic focus-recovery rule, over any comparable identity.
    ///
    /// Returns the index in `new_keys` the old identity recovers to: its exact
    /// position when the new order still hosts it, otherwise the nearest
    /// surviving sibling walking outward from its old position — next before
    /// previous at equal distance. `None` means nothing near it survived, or
    /// it was never in the old order at all; callers decide what a total miss
    /// means for their surface.
    ///
    /// It is generic so a patch switch can recover over *control* identities,
    /// where the two orders carry different PatchIds and whole paths can never
    /// compare equal — without growing a second, subtly different rule.
    pub fn recovered_index<T: PartialEq>(old: &T, old_keys: &[T], new_keys: &[T]) -> Option<usize> {
        if let Some(index) = new_keys.iter().position(|candidate| candidate == old) {
            return Some(index);
        }
        let old_index = old_keys.iter().position(|candidate| candidate == old)?;
        for distance in 1..old_keys.len() {
            if let Some(next) = old_keys.get(old_index + distance) {
                if let Some(index) = new_keys.iter().position(|candidate| candidate == next) {
                    return Some(index);
                }
            }
            if let Some(previous) = old_index
                .checked_sub(distance)
                .map(|index| &old_keys[index])
            {
                if let Some(index) = new_keys.iter().position(|candidate| candidate == previous) {
                    return Some(index);
                }
            }
        }
        None
    }
}

/// Reports whether one descriptor row's visibility and enablement predicates
/// are both satisfied by the configuration `value` reads.
///
/// This is the one place predicate satisfaction is decided. Every surface
/// resolver — PatchMain, PatchDetail, and MixerInspector — reads it, so the
/// same configuration cannot make a row visible on one surface and hidden on
/// another.
pub(crate) fn row_is_visible_and_enabled<'a>(
    spec: &ParameterSpec,
    value: impl Fn(&ParameterId) -> Option<&'a ParameterValue>,
) -> bool {
    [spec.visible_when(), spec.enabled_when()]
        .into_iter()
        .flatten()
        .all(|predicate| value(predicate.parameter_id()) == Some(predicate.equals()))
}

fn ensure_unique(paths: &[FocusPath]) -> Result<(), EventRejection> {
    let unique = paths.iter().collect::<HashSet<_>>();
    if unique.len() == paths.len() && !paths.is_empty() {
        Ok(())
    } else {
        Err(EventRejection::InvalidSelection)
    }
}

fn action_presentation(action: &SemanticAction) -> (&'static str, Option<&'static str>) {
    use crate::control::{Direction, InteractionMode, TopLevelContext};
    match action {
        SemanticAction::SelectContext(TopLevelContext::Mixer) => ("Open MIXER", Some("1")),
        SemanticAction::SelectContext(TopLevelContext::Patch) => ("Open PATCH", Some("2")),
        SemanticAction::SelectPatch(Direction::Left) => ("Previous patch", Some("Q")),
        SemanticAction::SelectPatch(Direction::Right) => ("Next patch", Some("E")),
        SemanticAction::SelectPatch(Direction::Up)
        | SemanticAction::SelectPatch(Direction::Down) => ("Unavailable patch step", None),
        SemanticAction::Navigate(Direction::Up) => ("Move up", Some("W")),
        SemanticAction::Navigate(Direction::Down) => ("Move down", Some("S")),
        SemanticAction::Navigate(Direction::Left) => ("Move left", Some("A")),
        SemanticAction::Navigate(Direction::Right) => ("Move right", Some("D")),
        SemanticAction::Adjust(Direction::Up) => ("Coarse increase", Some("K+W")),
        SemanticAction::Adjust(Direction::Down) => ("Coarse decrease", Some("K+S")),
        SemanticAction::Adjust(Direction::Left) => ("Fine decrease", Some("K+A")),
        SemanticAction::Adjust(Direction::Right) => ("Fine increase", Some("K+D")),
        SemanticAction::SetInteractionMode(InteractionMode::Navigate) => {
            ("Navigate mode", Some("release K"))
        }
        SemanticAction::SetInteractionMode(InteractionMode::Adjust) => {
            ("Adjust mode", Some("hold K"))
        }
        SemanticAction::SetInteractionMode(InteractionMode::Modal)
        | SemanticAction::SetInteractionMode(InteractionMode::MultiSelect) => {
            ("Unavailable mode", None)
        }
        SemanticAction::OpenRelated => ("Open related", Some("Shift+W")),
        SemanticAction::Activate => ("Choose", Some("Return")),
        SemanticAction::PreviewStart => ("Preview", Some("hold Space")),
        SemanticAction::PreviewStop => ("Stop preview", Some("release Space")),
        SemanticAction::SetSlotOccupancy { .. } => ("Set slot occupancy", None),
        SemanticAction::SetReturnOccupancy { .. } => ("Set return occupancy", None),
        SemanticAction::EnterSurface(SurfaceId::PatchUtility) => ("Open Utility", Some("D")),
        SemanticAction::EnterSurface(SurfaceId::PatchDetail) => ("Open Detail", Some("Return")),
        SemanticAction::EnterSurface(SurfaceId::PatchChoice)
        | SemanticAction::EnterSurface(SurfaceId::SampleBrowser) => ("Unavailable surface", None),
        SemanticAction::EnterSurface(SurfaceId::MixerInspector) => ("Open Inspector", None),
        SemanticAction::EnterSurface(SurfaceId::PatchMain)
        | SemanticAction::EnterSurface(SurfaceId::MixerMain) => ("Unavailable surface", None),
        SemanticAction::Return => ("Return", Some("A / Return")),
    }
}

#[cfg(test)]
mod tests {
    use super::SemanticResolver;
    use crate::control::{FocusPath, PatchControlId};
    use crate::kernel::PatchId;

    #[test]
    fn gapped_chain_resolves_slot_rows_per_position_and_the_occupant_by_identity() {
        // Slot 0 empty, slot 1 occupied: the shape a compacting view would
        // silently squeeze down to position 0.
        let mut state = crate::control::AppState::new_with_effects(
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
        state
            .apply(crate::control::AppEvent::InstallPatches(vec![patch]))
            .unwrap();

        let patch_id = PatchId::new(7).unwrap();
        let paths = SemanticResolver::new(&state)
            .patch_main_paths(patch_id)
            .unwrap();

        // Every position contributes its occupancy row, occupied or empty.
        for index in crate::synth::effect_slot_id::EffectSlotIndex::ALL {
            assert!(
                paths.contains(&FocusPath::patch_main(
                    patch_id,
                    None,
                    PatchControlId::EffectSlot(index),
                )),
                "position {index} must keep its occupancy row"
            );
        }
        // The occupant's parameter row resolves by stable identity at its
        // true position, with slot 0 still empty around it.
        assert!(paths.contains(&FocusPath::patch_main(
            patch_id,
            Some(crate::control::FocusCapabilityId::Effect(
                crate::synth::EffectCapabilityId::new(
                    crate::adapter::chorus_capability::CHORUS_CAPABILITY_ID,
                )
                .unwrap(),
            )),
            PatchControlId::Effect(
                crate::synth::EffectSlotId::new(2).unwrap(),
                crate::synth::ParameterId::new(
                    crate::adapter::chorus_capability::CHORUS_AMOUNT_PARAMETER_ID,
                )
                .unwrap(),
            ),
        )));
    }

    #[test]
    fn recovery_is_exact_then_next_before_previous() {
        let patch = PatchId::new(1).unwrap();
        let engine = FocusPath::patch_main(patch, None, PatchControlId::Engine);
        let attack = FocusPath::patch_main(
            patch,
            None,
            PatchControlId::Envelope(crate::synth::VoiceEnvelopeParameter::AttackMilliseconds),
        );
        let decay = FocusPath::patch_main(
            patch,
            None,
            PatchControlId::Envelope(crate::synth::VoiceEnvelopeParameter::DecayMilliseconds),
        );
        let old = vec![engine.clone(), attack.clone(), decay.clone()];
        assert_eq!(
            SemanticResolver::recover(&attack, &old, &[engine.clone(), decay.clone()]),
            Some(decay.clone())
        );
        assert_eq!(
            SemanticResolver::recover(&engine, &old, &[engine.clone(), decay]),
            Some(engine)
        );
    }
}
