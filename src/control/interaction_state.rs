use crate::control::top_level_context::TopLevelContext;
use crate::control::{Direction, PatchControlId};
use crate::kernel::patch_id::PatchId;

/// The kind of section selected in the transitional MIXER text projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionSection {
    Patch,
    Global,
}

/// A typed position in the complete Patch-plus-GLOBAL parameter list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    pub(super) section: SelectionSection,
    pub(super) patch_index: usize,
    pub(super) parameter_index: usize,
}

impl Selection {
    /// Selects the first parameter of one Patch section.
    pub const fn patch(patch_index: usize) -> Self {
        Self {
            section: SelectionSection::Patch,
            patch_index,
            parameter_index: 0,
        }
    }

    /// Selects the first global parameter.
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

/// Reducer-owned context and the independent focus retained for each context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionState {
    pub(super) context: TopLevelContext,
    pub(super) mixer_selection: Selection,
    pub(super) patch_focus: Option<PatchId>,
    pub(super) patch_control_focus: Option<PatchControlId>,
}

impl InteractionState {
    /// Creates the startup interaction state before any Patch is installed.
    pub const fn new() -> Self {
        Self {
            context: TopLevelContext::Mixer,
            mixer_selection: Selection::global(),
            patch_focus: None,
            patch_control_focus: None,
        }
    }

    pub const fn context(&self) -> TopLevelContext {
        self.context
    }

    pub const fn mixer_selection(&self) -> Selection {
        self.mixer_selection
    }

    pub const fn patch_focus(&self) -> Option<PatchId> {
        self.patch_focus
    }

    pub fn patch_control_focus(&self) -> Option<PatchControlId> {
        self.patch_control_focus.clone()
    }

    pub(super) fn select_context(&mut self, context: TopLevelContext) {
        self.context = context;
    }

    pub(super) fn set_mixer_selection(&mut self, selection: Selection) {
        self.mixer_selection = selection;
    }

    pub(super) fn initialize_patch_focus(&mut self, patch_focus: Option<PatchId>) {
        self.patch_focus = patch_focus;
        self.patch_control_focus = if patch_focus.is_some() {
            Some(PatchControlId::Engine)
        } else {
            None
        };
    }

    /// Moves the reducer-owned PATCH focus one row without wrapping.
    ///
    /// Returns `false` for horizontal movement, missing focus, and either
    /// endpoint so the reducer can preserve transactional rejection semantics.
    #[cfg(test)]
    pub(super) fn move_patch_control_focus(&mut self, direction: Direction) -> bool {
        self.move_patch_control_focus_with(direction, PatchControlId::surface_descriptor())
    }

    pub(super) fn move_patch_control_focus_with(
        &mut self,
        direction: Direction,
        controls: &[PatchControlId],
    ) -> bool {
        let amount = match direction {
            Direction::Up => -1_isize,
            Direction::Down => 1,
            Direction::Left | Direction::Right => return false,
        };
        let Some(current) = self.patch_control_focus.as_ref() else {
            return false;
        };
        let Some(index) = controls.iter().position(|control| control == current) else {
            return false;
        };
        let Some(next) = index.checked_add_signed(amount) else {
            return false;
        };
        let Some(control) = controls.get(next) else {
            return false;
        };
        self.patch_control_focus = Some(control.clone());
        true
    }

    pub(super) fn clamp_patch_control_focus(&mut self, controls: &[PatchControlId]) {
        let Some(last) = controls.last() else {
            self.patch_control_focus = None;
            return;
        };
        let Some(current) = self.patch_control_focus.as_ref() else {
            self.patch_control_focus = Some(PatchControlId::Engine);
            return;
        };
        if controls.contains(current) {
            return;
        }
        self.patch_control_focus = Some(last.clone());
    }

    pub(super) fn mixer_selection_mut(&mut self) -> &mut Selection {
        &mut self.mixer_selection
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{InteractionState, PatchControlId, Selection, SelectionSection};
    use crate::control::TopLevelContext;
    use crate::kernel::PatchId;

    #[test]
    fn interaction_state_starts_in_mixer_without_patch_focus() {
        let state = InteractionState::new();
        assert_eq!(state.context(), TopLevelContext::Mixer);
        assert_eq!(state.mixer_selection(), Selection::global());
        assert_eq!(state.patch_focus(), None);
        assert_eq!(state.patch_control_focus(), None);
    }

    #[test]
    fn context_and_patch_focus_do_not_overwrite_mixer_selection() {
        let mut state = InteractionState::new();
        let selection = Selection::patch(2);
        state.set_mixer_selection(selection);
        state.initialize_patch_focus(Some(PatchId::new(7).unwrap()));
        state.select_context(TopLevelContext::Patch);

        assert_eq!(state.context(), TopLevelContext::Patch);
        assert_eq!(state.mixer_selection(), selection);
        assert_eq!(state.mixer_selection().section(), SelectionSection::Patch);
        assert_eq!(state.patch_focus(), Some(PatchId::new(7).unwrap()));
        assert_eq!(state.patch_control_focus(), Some(PatchControlId::Engine));
    }

    #[test]
    fn patch_control_focus_moves_in_canonical_order_without_wrapping() {
        let mut state = InteractionState::new();
        state.initialize_patch_focus(Some(PatchId::new(7).unwrap()));

        assert!(!state.move_patch_control_focus(crate::control::Direction::Up));
        assert_eq!(state.patch_control_focus(), Some(PatchControlId::Engine));

        for expected in &PatchControlId::surface_descriptor()[1..] {
            assert!(state.move_patch_control_focus(crate::control::Direction::Down));
            assert_eq!(state.patch_control_focus(), Some(expected.clone()));
        }
        assert!(!state.move_patch_control_focus(crate::control::Direction::Down));
        assert_eq!(
            state.patch_control_focus(),
            PatchControlId::surface_descriptor().last().cloned()
        );

        for expected in PatchControlId::surface_descriptor()[..4].iter().rev() {
            assert!(state.move_patch_control_focus(crate::control::Direction::Up));
            assert_eq!(state.patch_control_focus(), Some(expected.clone()));
        }
        assert!(!state.move_patch_control_focus(crate::control::Direction::Left));
        assert!(!state.move_patch_control_focus(crate::control::Direction::Right));
        assert_eq!(state.patch_control_focus(), Some(PatchControlId::Engine));
    }

    #[test]
    fn context_round_trip_preserves_independent_patch_and_mixer_focus() {
        let mut state = InteractionState::new();
        let mixer = Selection::patch(2);
        state.set_mixer_selection(mixer);
        state.initialize_patch_focus(Some(PatchId::new(7).unwrap()));
        assert!(state.move_patch_control_focus(crate::control::Direction::Down));
        let patch_control = state.patch_control_focus();

        state.select_context(TopLevelContext::Patch);
        state.select_context(TopLevelContext::Mixer);
        state.select_context(TopLevelContext::Patch);

        assert_eq!(state.patch_focus(), Some(PatchId::new(7).unwrap()));
        assert_eq!(state.patch_control_focus(), patch_control);
        assert_eq!(state.mixer_selection(), mixer);
    }
}
