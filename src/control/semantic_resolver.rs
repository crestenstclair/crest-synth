use crate::control::{
    AppState, EventRejection, FocusCapabilityId, FocusPath, MixerControlId, PatchControlId,
    SemanticAction, SemanticControlId, SurfaceId, ValidAction,
};
use crate::kernel::PatchId;
use crate::mixer::bus_id::BusId;
use crate::mixer::global_parameters::GlobalParameters;
use crate::mixer::mixer_track_id::{MixerTrackId, MixerTrackId as TrackId};
use crate::mixer::mixer_track_parameters::MixerTrackParameter;
use crate::mixer::patch_output::PatchOutputParameter;
use crate::synth::PatchInteraction;
use std::collections::HashSet;

/// Pure descriptor-backed authority for semantic focus order and recovery.
///
/// The resolver borrows one immutable accepted state. It never owns interaction
/// state, layout data, or runtime objects, and every returned path is expressed
/// only with stable domain identities.
pub struct SemanticResolver<'a> {
    state: &'a AppState,
}

impl<'a> SemanticResolver<'a> {
    pub const fn new(state: &'a AppState) -> Self {
        Self { state }
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
                PatchControlId::Output(_) => {
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

    pub fn patch_utility_paths(&self, patch_id: PatchId) -> Result<Vec<FocusPath>, EventRejection> {
        if !self
            .state
            .patches()
            .iter()
            .any(|patch| patch.id() == patch_id)
        {
            return Err(EventRejection::NoPatchesInstalled);
        }
        let paths = PatchOutputParameter::ALL
            .into_iter()
            .map(|parameter| FocusPath::patch_utility(patch_id, PatchControlId::Output(parameter)))
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
                let predicate_satisfied = |predicate: Option<&crate::synth::ParameterPredicate>| {
                    predicate.is_none_or(|predicate| {
                        config.value(predicate.parameter_id()) == Some(predicate.equals())
                    })
                };
                if spec.patch_interaction() == PatchInteraction::ScalarEdit
                    && predicate_satisfied(spec.visible_when())
                    && predicate_satisfied(spec.enabled_when())
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
    pub fn valid_actions(&self) -> Vec<ValidAction> {
        SemanticAction::surface_descriptor()
            .iter()
            .filter(|action| self.state.accepts_semantic_action(action))
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
        if new_order.iter().any(|candidate| candidate == old_path) {
            return Some(old_path.clone());
        }
        let old_index = old_order
            .iter()
            .position(|candidate| candidate == old_path)?;
        for distance in 1..old_order.len() {
            if let Some(next) = old_order.get(old_index + distance) {
                if new_order.iter().any(|candidate| candidate == next) {
                    return Some(next.clone());
                }
            }
            if let Some(previous_index) = old_index.checked_sub(distance) {
                let previous = &old_order[previous_index];
                if new_order.iter().any(|candidate| candidate == previous) {
                    return Some(previous.clone());
                }
            }
        }
        new_order.first().cloned()
    }
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
        SemanticAction::SetSlotOccupancy { .. } => ("Set slot occupancy", None),
        SemanticAction::SetReturnOccupancy { .. } => ("Set return occupancy", None),
        SemanticAction::EnterSurface(SurfaceId::PatchUtility) => ("Open Utility", Some("D")),
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
