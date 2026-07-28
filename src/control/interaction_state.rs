use crate::control::{
    FocusPath, FocusPathError, InteractionMode, MixerControlId, PatchControlId, ReturnPath,
    SemanticControlId, SurfaceId, TopLevelContext,
};
use crate::kernel::PatchId;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::MixerTrackParameter;
use crate::mixer::patch_output::PatchOutputParameter;

/// Compatibility classification derived from the canonical MixerControlId.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionSection {
    Patch,
    Global,
}

/// Transitional diagnostic coordinates derived from a stable MixerControlId.
///
/// This value is never stored by InteractionState and is not an interaction
/// authority. It remains available only while older text/test projections are
/// migrated to semantic paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    pub(super) section: SelectionSection,
    pub(super) patch_index: usize,
    pub(super) parameter_index: usize,
}

impl Selection {
    pub const fn patch(patch_index: usize) -> Self {
        Self {
            section: SelectionSection::Patch,
            patch_index,
            parameter_index: 0,
        }
    }

    pub const fn global() -> Self {
        Self {
            section: SelectionSection::Global,
            patch_index: 0,
            parameter_index: 0,
        }
    }

    pub const fn section(&self) -> SelectionSection {
        self.section
    }

    pub const fn patch_index(&self) -> usize {
        self.patch_index
    }

    pub const fn parameter_index(&self) -> usize {
        self.parameter_index
    }
}

/// Reducer-owned singular semantic focus, remembered roots, mode, and return.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionState {
    pub(super) active_focus: FocusPath,
    pub(super) remembered_patch_main: Option<FocusPath>,
    pub(super) remembered_mixer_main: FocusPath,
    pub(super) mode: InteractionMode,
    pub(super) return_path: Option<ReturnPath>,
}

impl InteractionState {
    /// Creates startup interaction state on the always-available global mixer.
    pub fn new() -> Self {
        let mixer = FocusPath::mixer_track(MixerTrackId::default(), MixerTrackParameter::Level);
        Self {
            active_focus: mixer.clone(),
            remembered_patch_main: None,
            remembered_mixer_main: mixer,
            mode: InteractionMode::Navigate,
            return_path: None,
        }
    }

    pub const fn context(&self) -> TopLevelContext {
        self.active_focus.context()
    }

    pub const fn active_surface(&self) -> SurfaceId {
        self.active_focus.surface()
    }

    pub const fn focus_path(&self) -> &FocusPath {
        &self.active_focus
    }

    pub const fn remembered_patch_main(&self) -> Option<&FocusPath> {
        self.remembered_patch_main.as_ref()
    }

    pub const fn remembered_mixer_main(&self) -> &FocusPath {
        &self.remembered_mixer_main
    }

    pub const fn mode(&self) -> InteractionMode {
        self.mode
    }

    pub const fn return_path(&self) -> Option<&ReturnPath> {
        self.return_path.as_ref()
    }

    pub const fn patch_focus(&self) -> Option<PatchId> {
        match &self.remembered_patch_main {
            Some(path) => path.patch_id(),
            None => None,
        }
    }

    pub fn patch_control_focus(&self) -> Option<PatchControlId> {
        let path = if self.active_focus.context() == TopLevelContext::Patch {
            &self.active_focus
        } else {
            self.remembered_patch_main.as_ref()?
        };
        match path.control_id() {
            SemanticControlId::Patch(control) => Some(control.clone()),
            SemanticControlId::Mixer(_) | SemanticControlId::SurfaceRoot => None,
        }
    }

    pub fn mixer_control_focus(&self) -> &MixerControlId {
        let path = if self.active_focus.context() == TopLevelContext::Mixer {
            &self.active_focus
        } else {
            &self.remembered_mixer_main
        };
        match path.control_id() {
            SemanticControlId::Mixer(control) => control,
            SemanticControlId::Patch(_) | SemanticControlId::SurfaceRoot => {
                unreachable!("remembered MixerMain path is always a Mixer control")
            }
        }
    }

    pub(super) fn initialize_patch_focus(&mut self, focus: Option<FocusPath>) {
        self.remembered_patch_main = focus;
    }

    pub(super) fn set_active_main(&mut self, focus: FocusPath) -> Result<(), FocusPathError> {
        focus.validate()?;
        if !focus.surface().is_main() {
            return Err(FocusPathError::ControlSurfaceMismatch);
        }
        match focus.context() {
            TopLevelContext::Patch => self.remembered_patch_main = Some(focus.clone()),
            TopLevelContext::Mixer => self.remembered_mixer_main = focus.clone(),
        }
        self.active_focus = focus;
        self.return_path = None;
        Ok(())
    }

    pub(super) fn select_context(
        &mut self,
        context: TopLevelContext,
    ) -> Result<(), FocusPathError> {
        self.active_focus = match context {
            TopLevelContext::Patch => self
                .remembered_patch_main
                .clone()
                .ok_or(FocusPathError::PatchIdentityMismatch)?,
            TopLevelContext::Mixer => self.remembered_mixer_main.clone(),
        };
        self.mode = InteractionMode::Navigate;
        self.return_path = None;
        Ok(())
    }

    pub(super) fn set_mode(&mut self, mode: InteractionMode) -> Result<(), FocusPathError> {
        if !mode.is_phase_two_reachable() {
            return Err(FocusPathError::ModalIdentityUnavailable);
        }
        self.mode = mode;
        Ok(())
    }

    pub(super) fn enter_surface(&mut self, surface: SurfaceId) -> Result<(), FocusPathError> {
        if !surface.is_persistent_side()
            || !self.active_focus.surface().is_main()
            || self.active_focus.context() != surface.context()
        {
            return Err(FocusPathError::ContextSurfaceMismatch);
        }
        let origin = self.active_focus.clone();
        self.return_path = Some(ReturnPath::new(origin, surface)?);
        self.active_focus = match surface {
            SurfaceId::PatchUtility => FocusPath::patch_utility(
                self.active_focus
                    .patch_id()
                    .ok_or(FocusPathError::PatchIdentityMismatch)?,
                PatchControlId::Output(PatchOutputParameter::TrimGain),
            ),
            SurfaceId::MixerInspector => {
                let SemanticControlId::Mixer(MixerControlId::Track { track_id, .. }) =
                    self.active_focus.control_id()
                else {
                    return Err(FocusPathError::ControlSurfaceMismatch);
                };
                FocusPath::mixer_inspector(*track_id, MixerTrackParameter::ReverbSend)
            }
            SurfaceId::PatchMain | SurfaceId::MixerMain => {
                return Err(FocusPathError::ControlSurfaceMismatch)
            }
        };
        self.mode = InteractionMode::Navigate;
        Ok(())
    }

    pub(super) fn return_to_origin(&mut self) -> Result<(), FocusPathError> {
        let path = self
            .return_path
            .take()
            .ok_or(FocusPathError::ContextSurfaceMismatch)?;
        if path.entered_surface() != self.active_focus.surface() {
            return Err(FocusPathError::ContextSurfaceMismatch);
        }
        self.active_focus = path.origin().clone();
        self.mode = InteractionMode::Navigate;
        Ok(())
    }

    pub(super) fn replace_remembered_patch_main(&mut self, focus: FocusPath) {
        let was_active = self.active_focus.surface() == SurfaceId::PatchMain;
        self.remembered_patch_main = Some(focus.clone());
        if was_active {
            self.active_focus = focus;
        }
    }

    pub(super) fn replace_remembered_mixer_main(&mut self, focus: FocusPath) {
        let was_active = self.active_focus.surface() == SurfaceId::MixerMain;
        self.remembered_mixer_main = focus.clone();
        if was_active {
            self.active_focus = focus;
        }
    }

    pub(super) fn replace_return_origin(&mut self, origin: FocusPath) {
        if let Some(return_path) = self.return_path.as_ref() {
            self.return_path = ReturnPath::new(origin, return_path.entered_surface()).ok();
        }
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::InteractionState;
    use crate::control::{FocusPath, InteractionMode, PatchControlId, SurfaceId, TopLevelContext};
    use crate::kernel::PatchId;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::mixer_track_parameters::MixerTrackParameter;

    #[test]
    fn interaction_state_starts_with_one_stable_mixer_focus() {
        let state = InteractionState::new();
        assert_eq!(state.context(), TopLevelContext::Mixer);
        assert_eq!(state.active_surface(), SurfaceId::MixerMain);
        assert_eq!(
            state.focus_path(),
            &FocusPath::mixer_track(MixerTrackId::default(), MixerTrackParameter::Level)
        );
        assert_eq!(state.patch_focus(), None);
        assert_eq!(state.mode(), InteractionMode::Navigate);
        assert_eq!(state.return_path(), None);
    }

    #[test]
    fn context_round_trip_preserves_independent_stable_paths() {
        let mut state = InteractionState::new();
        let patch = FocusPath::patch_main(
            PatchId::new(7).unwrap(),
            None,
            PatchControlId::Envelope(crate::synth::VoiceEnvelopeParameter::AttackMilliseconds),
        );
        state.initialize_patch_focus(Some(patch.clone()));
        state.select_context(TopLevelContext::Patch).unwrap();
        state.select_context(TopLevelContext::Mixer).unwrap();
        state.select_context(TopLevelContext::Patch).unwrap();
        assert_eq!(state.focus_path(), &patch);
        assert_eq!(state.patch_focus(), Some(PatchId::new(7).unwrap()));
    }

    #[test]
    fn side_surface_round_trip_restores_exact_origin_and_navigate_mode() {
        let mut state = InteractionState::new();
        let origin = state.focus_path().clone();
        state.set_mode(InteractionMode::Adjust).unwrap();
        state.enter_surface(SurfaceId::MixerInspector).unwrap();
        assert_eq!(state.active_surface(), SurfaceId::MixerInspector);
        assert_eq!(state.return_path().unwrap().origin(), &origin);
        assert_eq!(state.mode(), InteractionMode::Navigate);
        state.return_to_origin().unwrap();
        assert_eq!(state.focus_path(), &origin);
        assert_eq!(state.return_path(), None);
    }

    #[test]
    fn phase_two_rejects_reserved_modes_and_cross_context_surface_entry() {
        let mut state = InteractionState::new();
        assert!(state.set_mode(InteractionMode::Modal).is_err());
        assert!(state.enter_surface(SurfaceId::PatchUtility).is_err());
        assert_eq!(state.mode(), InteractionMode::Navigate);
    }
}
