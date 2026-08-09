use crate::control::{
    FocusPath, FocusPathError, InteractionMode, MixerControlId, PatchControlId, PatchDetailSubject,
    ReturnPath, SemanticControlId, SurfaceId, TopLevelContext,
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
///
/// # The detail invariant
///
/// `detail_subject` is `Some` **exactly** while `active_focus.surface` is
/// `PatchDetail` and `return_path.entered_surface` is `PatchDetail`. The three
/// facts move together in one transition, so no reachable state pairs an open
/// detail surface with no subject, or a subject with no surface to live on.
///
/// That is enforced structurally rather than by convention, for one of the
/// three fields: `detail_subject` is private to this module, so — unlike its
/// `pub(super)` siblings — the reducer cannot assign it, and
/// [`Self::enter_detail`], [`Self::leave_subordinate`], and
/// [`Self::return_to_origin`] are the only transitions that can set or clear
/// it. `active_focus` and `return_path` remain `pub(super)`: the reducer
/// assigns `active_focus` directly at two sites, each of which moves within one
/// already-open surface's own resolved order and so cannot change which surface
/// is active (`app_state::navigate_side_nonwrapping` and
/// `app_state::repair_inspector_focus`). The invariant is therefore
/// held by *every* mutator on this type ending in an
/// [`Self::assert_detail_invariant`] call, plus those two guarded reducer
/// writes — not by the privacy of one field alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionState {
    pub(super) active_focus: FocusPath,
    pub(super) remembered_patch_main: Option<FocusPath>,
    pub(super) remembered_mixer_main: FocusPath,
    pub(super) mode: InteractionMode,
    pub(super) return_path: Option<ReturnPath>,
    /// Deliberately private, not `pub(super)`: see the type's detail invariant.
    detail_subject: Option<PatchDetailSubject>,
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
            detail_subject: None,
        }
    }

    /// Reports whether the three detail facts agree.
    ///
    /// Exposed so tests can assert the invariant over reachable states rather
    /// than trusting that every transition remembered to maintain it.
    pub fn detail_invariant_holds(&self) -> bool {
        let focus_is_detail = self.active_focus.surface() == SurfaceId::PatchDetail;
        let return_is_detail = self
            .return_path
            .as_ref()
            .is_some_and(|path| path.entered_surface() == SurfaceId::PatchDetail);
        let subject_is_open = self.detail_subject.is_some();
        focus_is_detail == subject_is_open && return_is_detail == subject_is_open
    }

    /// Panics in debug builds if a transition left the three facts disagreeing.
    fn assert_detail_invariant(&self) {
        debug_assert!(
            self.detail_invariant_holds(),
            "detail surface, subject, and return path must move together: \
             surface={:?} subject={:?} return={:?}",
            self.active_focus.surface(),
            self.detail_subject,
            self.return_path.as_ref().map(ReturnPath::entered_surface),
        );
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

    /// Returns the capability whose schema the open detail surface shows, or
    /// `None` when no detail entry is open.
    pub const fn detail_subject(&self) -> Option<&PatchDetailSubject> {
        self.detail_subject.as_ref()
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
        self.assert_detail_invariant();
    }

    /// Lands the active focus on a main path, leaving any subordinate surface.
    ///
    /// A main path is by definition not a detail path, so the subject is
    /// cleared with the return path rather than left to outlive its surface.
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
        self.leave_subordinate();
        self.assert_detail_invariant();
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
        // A detail subject belongs to the context it was opened in; switching
        // context leaves the detail surface rather than carrying it.
        self.leave_subordinate();
        self.assert_detail_invariant();
        Ok(())
    }

    pub(super) fn set_mode(&mut self, mode: InteractionMode) -> Result<(), FocusPathError> {
        if !mode.is_phase_two_reachable() {
            return Err(FocusPathError::ModalIdentityUnavailable);
        }
        self.mode = mode;
        self.assert_detail_invariant();
        Ok(())
    }

    /// Enters one *persistent* side surface from a main path.
    ///
    /// The subordinate detail surface is not reachable here: it needs a
    /// subject, which only [`Self::enter_detail`] can supply. Requiring a main
    /// origin is also what makes the surfaces non-nesting — a path already on
    /// `PatchUtility` or `PatchDetail` is not main, so neither can stack a
    /// second origin on the other.
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
                // The selected track's first send: Send(trackId, BusId 0).
                FocusPath::mixer_send(*track_id, crate::mixer::bus_id::BusId::default())
            }
            SurfaceId::PatchDetail | SurfaceId::PatchMain | SurfaceId::MixerMain => {
                return Err(FocusPathError::ControlSurfaceMismatch)
            }
        };
        self.mode = InteractionMode::Navigate;
        self.assert_detail_invariant();
        Ok(())
    }

    /// Opens the subordinate detail surface on one subject.
    ///
    /// This is the *only* transition that can set `detail_subject`, and it
    /// sets all three detail facts together: the remembered origin, the
    /// subject, and the focus on the subject's first control. The origin must
    /// be the current main path, so entering from `PatchUtility` or from an
    /// already-open detail surface is refused rather than stacked.
    pub(super) fn enter_detail(
        &mut self,
        subject: PatchDetailSubject,
        focus: FocusPath,
    ) -> Result<(), FocusPathError> {
        if !self.active_focus.surface().is_main()
            || self.active_focus.context() != TopLevelContext::Patch
        {
            return Err(FocusPathError::ContextSurfaceMismatch);
        }
        focus.validate()?;
        if focus.surface() != SurfaceId::PatchDetail {
            return Err(FocusPathError::ControlSurfaceMismatch);
        }
        let return_path = ReturnPath::new(self.active_focus.clone(), SurfaceId::PatchDetail)?;
        self.return_path = Some(return_path);
        self.detail_subject = Some(subject);
        self.active_focus = focus;
        self.mode = InteractionMode::Navigate;
        self.assert_detail_invariant();
        Ok(())
    }

    /// Leaves whichever subordinate or side surface is open, restoring the
    /// exact remembered origin.
    ///
    /// The return path and the detail subject are cleared together, so leaving
    /// a detail surface can never strand its subject.
    pub(super) fn return_to_origin(&mut self) -> Result<(), FocusPathError> {
        let path = self
            .return_path
            .take()
            .ok_or(FocusPathError::ContextSurfaceMismatch)?;
        if path.entered_surface() != self.active_focus.surface() {
            // Put it back: a refused Return leaves state identical.
            self.return_path = Some(path);
            return Err(FocusPathError::ContextSurfaceMismatch);
        }
        self.active_focus = path.origin().clone();
        self.detail_subject = None;
        self.mode = InteractionMode::Navigate;
        self.assert_detail_invariant();
        Ok(())
    }

    /// Abandons any open subordinate surface without restoring its origin.
    ///
    /// Used where the origin is about to stop existing — a patch switch leaves
    /// the Patch the subject belonged to — so the caller supplies the new
    /// focus itself. Clears the return path and the subject together.
    pub(super) fn leave_subordinate(&mut self) {
        self.return_path = None;
        self.detail_subject = None;
        self.assert_detail_invariant();
    }

    /// Leaves an open detail surface by restoring its remembered origin.
    ///
    /// Used where the subject stopped naming a live capability or slot — an
    /// engine swap or a slot clear committed underneath the open entry. The
    /// declaration is explicit that such a subject is *not* repaired into a
    /// neighbouring one: the surface is left back to its origin, because
    /// silently retargeting a detail view would show one capability's values
    /// under another's title. Reports whether a surface was actually left.
    pub(super) fn leave_detail_to_origin(&mut self) -> bool {
        if self.detail_subject.is_none() {
            return false;
        }
        let origin = self
            .return_path
            .take()
            .expect("the detail invariant pairs an open subject with a return path")
            .origin()
            .clone();
        self.detail_subject = None;
        self.active_focus = origin;
        self.mode = InteractionMode::Navigate;
        self.assert_detail_invariant();
        true
    }

    pub(super) fn replace_remembered_patch_main(&mut self, focus: FocusPath) {
        let was_active = self.active_focus.surface() == SurfaceId::PatchMain;
        self.remembered_patch_main = Some(focus.clone());
        if was_active {
            self.active_focus = focus;
        }
        self.assert_detail_invariant();
    }

    pub(super) fn replace_remembered_mixer_main(&mut self, focus: FocusPath) {
        let was_active = self.active_focus.surface() == SurfaceId::MixerMain;
        self.remembered_mixer_main = focus.clone();
        if was_active {
            self.active_focus = focus;
        }
        self.assert_detail_invariant();
    }

    /// Replaces the remembered origin of whichever surface is open.
    ///
    /// A construction failure is returned, never swallowed: assigning `None`
    /// here would clear the return path while an open detail surface still
    /// held its subject, which is precisely the state the detail invariant
    /// forbids. No caller can reach the failure today — every origin it is
    /// given is a repaired main path in the entered surface's own context —
    /// and this is what keeps that true rather than assuming it.
    pub(super) fn replace_return_origin(
        &mut self,
        origin: FocusPath,
    ) -> Result<(), FocusPathError> {
        let Some(return_path) = self.return_path.as_ref() else {
            return Ok(());
        };
        self.return_path = Some(ReturnPath::new(origin, return_path.entered_surface())?);
        self.assert_detail_invariant();
        Ok(())
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

    /// A return origin that cannot be constructed is refused, not swallowed.
    ///
    /// Assigning `None` here would clear the return path while an open detail
    /// surface still held its subject — the state the detail invariant
    /// forbids. No reducer path can reach this today, because every origin the
    /// repair hands over is a main path in the entered surface's own context;
    /// this is what keeps that a fact rather than an assumption.
    #[test]
    fn a_return_origin_that_cannot_be_constructed_is_refused_and_changes_nothing() {
        let mut state = InteractionState::new();
        state.enter_surface(SurfaceId::MixerInspector).unwrap();
        let before = state.clone();

        // An Inspector path is not a main path, so it cannot be an origin.
        let not_a_main_path = FocusPath::mixer_send(
            MixerTrackId::default(),
            crate::mixer::bus_id::BusId::default(),
        );
        assert!(state.replace_return_origin(not_a_main_path).is_err());
        assert_eq!(
            state, before,
            "a refused replacement leaves state identical"
        );
        assert!(state.return_path().is_some());
        assert!(state.detail_invariant_holds());
    }

    #[test]
    fn phase_two_rejects_reserved_modes_and_cross_context_surface_entry() {
        let mut state = InteractionState::new();
        assert!(state.set_mode(InteractionMode::Modal).is_err());
        assert!(state.enter_surface(SurfaceId::PatchUtility).is_err());
        assert_eq!(state.mode(), InteractionMode::Navigate);
    }
}
