use crate::control::{Direction, SurfaceId, TopLevelContext};
use crate::kernel::patch_id::PatchId;
use crate::mixer::bus_id::BusId;
use crate::synth::effect_slot_id::EffectSlotIndex;
use crate::synth::EffectCapabilityId;
use serde::{Deserialize, Serialize};

/// The explicit reducer-owned interpretation of directional input.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InteractionMode {
    #[default]
    Navigate,
    Adjust,
    Modal,
    MultiSelect,
}

impl InteractionMode {
    pub const ALL: [Self; 4] = [Self::Navigate, Self::Adjust, Self::Modal, Self::MultiSelect];
    pub const PHASE_TWO: [Self; 2] = [Self::Navigate, Self::Adjust];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }

    pub const fn is_phase_two_reachable(self) -> bool {
        matches!(self, Self::Navigate | Self::Adjust)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Navigate => "NAVIGATE",
            Self::Adjust => "ADJUST",
            Self::Modal => "MODAL",
            Self::MultiSelect => "MULTI SELECT",
        }
    }
}

/// The semantic user-intent variants accepted at passive input boundaries.
///
/// The two occupancy variants are structural topology edits: each names one
/// exact position (a Patch effect slot or a bus return) and carries an
/// optional registry entry, where `None` clears the position. The vocabulary
/// stays generic — positions and registry identities, never effect names.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SemanticAction {
    SelectContext(TopLevelContext),
    /// Moves the focused Patch one position along the installed order. This is
    /// the only way the focused Patch changes: every installed instrument is
    /// reached through the same physical input → semantic action → reducer
    /// path as any other edit, never by a UI-local selection or a backstage
    /// lookup.
    SelectPatch(Direction),
    Navigate(Direction),
    Adjust(Direction),
    SetInteractionMode(InteractionMode),
    EnterSurface(SurfaceId),
    Return,
    SetSlotOccupancy {
        patch_id: PatchId,
        slot: EffectSlotIndex,
        entry: Option<EffectCapabilityId>,
    },
    SetReturnOccupancy {
        bus: BusId,
        entry: Option<EffectCapabilityId>,
    },
}

/// The variant-level descriptor for the closed semantic action union.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticActionKind {
    SelectContext,
    SelectPatch,
    Navigate,
    Adjust,
    SetInteractionMode,
    EnterSurface,
    Return,
    SetSlotOccupancy,
    SetReturnOccupancy,
}

impl SemanticActionKind {
    pub const ALL: [Self; 9] = [
        Self::SelectContext,
        Self::SelectPatch,
        Self::Navigate,
        Self::Adjust,
        Self::SetInteractionMode,
        Self::EnterSurface,
        Self::Return,
        Self::SetSlotOccupancy,
        Self::SetReturnOccupancy,
    ];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }
}

const SEMANTIC_ACTION_SURFACE_DESCRIPTOR: [SemanticAction; 17] = [
    SemanticAction::SelectContext(TopLevelContext::Patch),
    SemanticAction::SelectContext(TopLevelContext::Mixer),
    // Only the horizontal pair: moving along the installed Patch order is an
    // adjacent choice, exactly like every other adjacent-choice surface.
    SemanticAction::SelectPatch(Direction::Left),
    SemanticAction::SelectPatch(Direction::Right),
    SemanticAction::Navigate(Direction::Up),
    SemanticAction::Navigate(Direction::Down),
    SemanticAction::Navigate(Direction::Left),
    SemanticAction::Navigate(Direction::Right),
    SemanticAction::Adjust(Direction::Up),
    SemanticAction::Adjust(Direction::Down),
    SemanticAction::Adjust(Direction::Left),
    SemanticAction::Adjust(Direction::Right),
    SemanticAction::SetInteractionMode(InteractionMode::Navigate),
    SemanticAction::SetInteractionMode(InteractionMode::Adjust),
    SemanticAction::EnterSurface(SurfaceId::PatchUtility),
    SemanticAction::EnterSurface(SurfaceId::MixerInspector),
    SemanticAction::Return,
];

impl SemanticAction {
    /// Returns the closed Phase 2 directional/navigation surface exactly once.
    ///
    /// The occupancy actions are deliberately outside this const descriptor:
    /// their registry-entry payload is an open identity, so no finite list of
    /// concrete instances exists. Their availability is proved through the
    /// reducer, like every other action.
    pub const fn surface_descriptor() -> &'static [Self] {
        &SEMANTIC_ACTION_SURFACE_DESCRIPTOR
    }

    pub const fn kind(&self) -> SemanticActionKind {
        match self {
            Self::SelectContext(_) => SemanticActionKind::SelectContext,
            Self::SelectPatch(_) => SemanticActionKind::SelectPatch,
            Self::Navigate(_) => SemanticActionKind::Navigate,
            Self::Adjust(_) => SemanticActionKind::Adjust,
            Self::SetInteractionMode(_) => SemanticActionKind::SetInteractionMode,
            Self::EnterSurface(_) => SemanticActionKind::EnterSurface,
            Self::Return => SemanticActionKind::Return,
            Self::SetSlotOccupancy { .. } => SemanticActionKind::SetSlotOccupancy,
            Self::SetReturnOccupancy { .. } => SemanticActionKind::SetReturnOccupancy,
        }
    }

    /// Reports whether the action belongs to the Phase 2 user-intent surface.
    pub const fn is_phase_two_admitted(&self) -> bool {
        match self {
            Self::SetInteractionMode(mode) => mode.is_phase_two_reachable(),
            Self::EnterSurface(surface) => surface.is_persistent_side(),
            _ => true,
        }
    }
}

/// One currently accepted semantic action plus host-neutral display metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidAction {
    action: SemanticAction,
    label: String,
    hint: Option<String>,
}

impl ValidAction {
    pub fn new(
        action: SemanticAction,
        label: impl Into<String>,
        hint: Option<impl Into<String>>,
    ) -> Self {
        Self {
            action,
            label: label.into(),
            hint: hint.map(Into::into),
        }
    }

    pub const fn action(&self) -> &SemanticAction {
        &self.action
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn hint(&self) -> Option<&str> {
        self.hint.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::{InteractionMode, SemanticAction, SemanticActionKind, ValidAction};
    use crate::control::{Direction, SurfaceId};
    use std::collections::HashSet;

    #[test]
    fn semantic_action_descriptors_are_closed_unique_and_phase_two_safe() {
        assert_eq!(SemanticActionKind::surface_descriptor().len(), 9);
        assert_eq!(InteractionMode::surface_descriptor().len(), 4);
        assert_eq!(InteractionMode::PHASE_TWO.len(), 2);
        assert!(SemanticAction::surface_descriptor()
            .iter()
            .all(SemanticAction::is_phase_two_admitted));
        let unique = SemanticAction::surface_descriptor()
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        assert_eq!(unique.len(), SemanticAction::surface_descriptor().len());
        assert!(!SemanticAction::SetInteractionMode(InteractionMode::Modal).is_phase_two_admitted());
        assert!(!SemanticAction::EnterSurface(SurfaceId::PatchMain).is_phase_two_admitted());
    }

    #[test]
    fn valid_action_round_trips_typed_action_not_its_label() {
        let valid = ValidAction::new(
            SemanticAction::Adjust(Direction::Right),
            "Increase",
            Some("K+D"),
        );
        let json = serde_json::to_string(&valid).unwrap();
        let decoded: ValidAction = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, valid);
        assert_eq!(decoded.action(), &SemanticAction::Adjust(Direction::Right));
        assert_eq!(decoded.hint(), Some("K+D"));
    }
}
