//! One row of the PATCH strip.
//!
//! A row is one projected control, selected through the control family in the
//! role its section supplies, plus the structural-edit lifecycle the row is
//! reporting. It is the unit the Phase 5 Patch editor is built from.
//!
//! # Figma source
//!
//! - `SCREEN · Patch Strip · 1920×1080` → the `Instrument` row (`36:28`) and
//!   the three `FX n` rows (`36:34`, `36:39`, `36:44`). Each is a reserved
//!   cursor column, the label and its value on the left, a transparent spacer,
//!   and a right-hand column.
//!
//! The horizontal anatomy inside the row belongs to the control, not to this
//! composition: the cursor column, the label start, the value column and the
//! leader are all the parameter row's, measured from the same frames.
//!
//! **The authored right-hand column is a per-row action hint**
//! (`L/R:change   EDIT+UP:options   SHIFT+UP:detail`), and it is omitted. A
//! `SemanticControlViewModel` carries no hints, and the projection's
//! `validActions` are the *currently* valid ones for the focused control, not a
//! per-row set — painting them on every row would attribute the focused row's
//! affordances to rows that do not have them. Recorded in this work package's
//! handoff rather than approximated.
//!
//! # This composition adds no row
//!
//! The reducer owns the PATCH focus order (`DESIGN.md:309`). A row renders what
//! it is given; it inserts nothing, reorders nothing, and hides nothing the
//! projection reports as visible.
//!
//! # Structural edits are shown, not summarised
//!
//! `DESIGN.md:454` — a targeted structural row displays its active and
//! requested value plus `Preparing`, `Activating`, or a typed failure, with the
//! active graph explicit. The control paints the phase word in its own status
//! column; what it cannot paint is the graph identity, because a
//! `ComponentState` carries none. So the row adds a lifecycle band beneath
//! itself carrying the phase, the active graph revision, and the requested one.
//!
//! That band is a readout, not a row: it takes no focus, allocates no
//! interactive target, and appears only while an edit is in flight or has
//! failed.
//!
//! **The requested value is not projected.** `SemanticLifecycleStatus` carries
//! the target *graph revision* and nothing else, and the control's own value is
//! the active one. So the band marks the requested value explicitly unavailable
//! (C-003) rather than showing the active value twice under two names. Recorded
//! for Phase 5.
//!
//! # What this module owns
//!
//! Nothing. It reads the immutable view model it is handed and returns
//! aggregated [`CompositionIntent`]; the focus path it records is identity read
//! back from the projection, never cached focus.

use eframe::egui::{pos2, Sense, Ui, Vec2};

use super::{section, CompositionIntent};
use crate::control::{
    EngineSelectionStatusKind, GraphicalShellProjection, InteractionMode, SemanticControlViewModel,
    SemanticSurfaceViewModel,
};
use crate::shell::visual::controls::parameter_row::UNAVAILABLE_MARK;
use crate::shell::visual::controls::{control_for, ControlSelection, PresentationRole};
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::hint::HINT_SEPARATOR;
use crate::shell::visual::primitives::text;
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};

/// Names the graph the shell is rendering right now.
pub(super) const ACTIVE_GRAPH_LABEL: &str = "ACTIVE GRAPH";

/// Names the graph a structural edit is moving toward.
pub(super) const REQUESTED_GRAPH_LABEL: &str = "REQUESTED GRAPH";

/// Names the value a structural edit is moving toward.
pub(super) const REQUESTED_VALUE_LABEL: &str = "REQUESTED VALUE";

/// Names the row this composition renders when it is asked for on its own.
pub(super) const FOCUSED_ENTRY_LABEL: &str = "FOCUSED ENTRY";

/// Arranges the row the projection's focus path names.
///
/// Asked for on its own rather than through a section, a row composition has to
/// be told which row; the projection already says which one the operator is on,
/// so that is the one it renders. A projection carrying no control at its own
/// focus path is incoherent, and the row marks that rather than painting a
/// plausible row.
pub fn render(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let model = projection.semantic_model();
    let focus = model.focus_path();
    let focused = model
        .surfaces()
        .iter()
        .flat_map(SemanticSurfaceViewModel::controls)
        .find(|control| control.path() == focus);
    match focused {
        Some(view) => render_row(
            ui,
            view,
            model.interaction_mode(),
            PresentationRole::ListedRow,
            density,
        ),
        None => {
            section::mark_unavailable(ui, FOCUSED_ENTRY_LABEL, density);
            CompositionIntent::none()
        }
    }
}

/// Arranges one projected control in the role its section supplies, and returns
/// what it asked for.
///
/// The role is never inferred here, and the state is derived only from the
/// immutable view data plus the projected interaction mode — both facts the
/// reducer already decided.
pub(super) fn render_row(
    ui: &mut Ui,
    view: &SemanticControlViewModel,
    mode: InteractionMode,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let state = component_state(view, mode);
    let mut intent = CompositionIntent::none();
    match control_for(view.kind(), role) {
        ControlSelection::Control(control) => {
            intent.record(view.path(), control.render(ui, view, state, role, density));
        }
        // A kind the model declares un-askable in this role has no shape here.
        // Falling back to a generic row would paint a control nobody selected,
        // so the row says what is missing instead.
        ControlSelection::NotAskableInRole => {
            section::mark_unavailable(ui, view.label(), density);
        }
    }
    render_lifecycle(ui, view, density);
    intent
}

/// The state this row hands its control, derived from the immutable projection.
///
/// Precedence is the operator's reading order. A failed edit outranks an
/// in-flight one, because a row that failed is no longer preparing. Focus
/// outranks read-only-ness, because where the operator is standing is the fact
/// they need first, and a read-only row is still reachable in the focus order.
///
/// Exhaustive over the interaction mode with no wildcard, so a mode added to the
/// reducer fails compilation here rather than silently reading as navigation.
pub(super) fn component_state(
    view: &SemanticControlViewModel,
    mode: InteractionMode,
) -> ComponentState {
    if view.error().is_some() {
        return ComponentState::Error;
    }
    if in_flight(view) {
        return ComponentState::Loading;
    }
    if view.focused() {
        return match mode {
            InteractionMode::Adjust => ComponentState::Adjusting,
            InteractionMode::Navigate | InteractionMode::Modal | InteractionMode::MultiSelect => {
                ComponentState::Focused
            }
        };
    }
    if view.enabled() && view.editable() {
        ComponentState::Resting
    } else {
        // `DESIGN.md:454` — Braids instrument capability rows stay read-only on
        // PATCH, and a SoundFont file row stays locked. The composition does not
        // decide that; the projection already did.
        ComponentState::Disabled
    }
}

/// Whether a structural edit this row requested is in flight.
///
/// Exhaustive over the lifecycle vocabulary with no wildcard.
pub(super) fn in_flight(view: &SemanticControlViewModel) -> bool {
    view.status().is_some_and(|status| match status.kind() {
        EngineSelectionStatusKind::Preparing | EngineSelectionStatusKind::Activating => true,
        EngineSelectionStatusKind::Ready | EngineSelectionStatusKind::Failed => false,
    })
}

/// Whether this row is reporting a structural edit the operator has to read.
pub(super) fn reports_structural_edit(view: &SemanticControlViewModel) -> bool {
    view.error().is_some() || in_flight(view)
}

/// Paints the lifecycle band beneath a row that is reporting a structural edit.
fn render_lifecycle(ui: &mut Ui, view: &SemanticControlViewModel, density: &ViewportDensityPolicy) {
    if !reports_structural_edit(view) {
        return;
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(error) = view.error() {
        parts.push(error.label().to_owned());
    }
    if let Some(status) = view.status() {
        parts.push(status.label().to_owned());
        parts.push(ACTIVE_GRAPH_LABEL.to_owned());
        parts.push(status.graph_revision().to_string());
        parts.push(REQUESTED_GRAPH_LABEL.to_owned());
        parts.push(match status.target_graph_revision() {
            Some(revision) => revision.to_string(),
            None => UNAVAILABLE_MARK.to_owned(),
        });
    }
    // Not projected. Marked rather than filled, so an absent requested value is
    // never mistaken for the active one under another name.
    parts.push(REQUESTED_VALUE_LABEL.to_owned());
    parts.push(UNAVAILABLE_MARK.to_owned());

    let color = if view.error().is_some() {
        SemanticColor::AccentWarning
    } else {
        SemanticColor::AccentAdjust
    };
    let state = ComponentState::Resting;
    let height =
        TypeStyle::InstructionHint.metrics().line_height_px + density.rhythm().row_gap_px();
    let response = ui.allocate_response(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(response.rect);
    let band = response.rect;
    let run = text::layout(
        &painter,
        parts.join(HINT_SEPARATOR),
        TypeStyle::InstructionHint,
        color,
        state,
    );
    let size = run.size();
    painter.galley(
        pos2(
            band.min.x + crate::shell::visual::primitives::focus::LABEL_START_X_PX,
            band.center().y - size.y / 2.0,
        ),
        run,
        text::resolved_color(color, state).resolve(),
    );
}

#[cfg(test)]
mod tests {
    use super::super::section::probe::{paint, paint_with, projection, Painted};
    use super::super::section::tests_support::from_projection_or_vocabulary;
    use super::*;
    use crate::control::{SemanticControlKind, SurfaceId, TopLevelContext};
    use crate::shell::visual::compositions::ShellComposition;
    use crate::shell::visual::controls::ALL_PRESENTATION_ROLES;
    use crate::shell::visual::token::MIN_INTERACTIVE_TARGET_PX;

    const COMPOSITION: ShellComposition = ShellComposition::PatchStripRow;

    fn patch_main_controls() -> Vec<SemanticControlViewModel> {
        let projection = projection(TopLevelContext::Patch);
        projection
            .semantic_model()
            .surface(SurfaceId::PatchMain)
            .expect("the PATCH main surface projects")
            .controls()
            .to_vec()
    }

    /// The row is wired: asking the family for it reaches this module and
    /// paints.
    #[test]
    fn the_declared_row_composition_renders_through_this_module() {
        let shown = paint(
            COMPOSITION,
            &projection(TopLevelContext::Patch),
            ViewportDensityPolicy::Desktop,
        );
        assert!(
            !shown.texts.is_empty(),
            "the row painted nothing; it is still the stub"
        );
    }

    /// The row the projection's focus path names is the row that paints.
    #[test]
    fn the_row_paints_the_control_the_projection_focuses() {
        let projection = projection(TopLevelContext::Patch);
        let focus = projection.semantic_model().focus_path();
        let focused = projection
            .semantic_model()
            .surfaces()
            .iter()
            .flat_map(SemanticSurfaceViewModel::controls)
            .find(|control| control.path() == focus)
            .expect("the projection carries its own focused control");
        let shown = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);
        assert!(
            shown.says(focused.label()),
            "the focused row's projected label did not reach the output"
        );
    }

    /// A read-only row is handed `Disabled` and says so without color.
    ///
    /// The composition does not decide read-only-ness — the projection's
    /// `editable` flag does — and the state vocabulary's declared word is what
    /// reaches the screen.
    #[test]
    fn a_read_only_row_is_handed_disabled_and_reads_as_locked() {
        let controls = patch_main_controls();
        let read_only = controls
            .iter()
            .find(|control| !control.editable() && !control.focused())
            .expect("the production PATCH surface carries a read-only row");
        assert_eq!(
            component_state(read_only, InteractionMode::Navigate),
            ComponentState::Disabled
        );
        let shown = paint_with(ViewportDensityPolicy::Desktop, |ui| {
            render_row(
                ui,
                read_only,
                InteractionMode::Navigate,
                PresentationRole::ListedRow,
                &ViewportDensityPolicy::Desktop,
            )
        });
        let crate::shell::visual::state::NonColorSignal::Word(word) =
            ComponentState::Disabled.appearance().signal
        else {
            panic!("Disabled declares a fixed word");
        };
        assert!(shown.says(word), "a read-only row painted no state word");
    }

    /// Focus and adjustment are separate states, and the projected interaction
    /// mode is what tells them apart.
    #[test]
    fn the_projected_interaction_mode_separates_focus_from_adjustment() {
        let controls = patch_main_controls();
        let focused = controls
            .iter()
            .find(|control| control.focused())
            .expect("the PATCH surface carries the focus");
        assert_eq!(
            component_state(focused, InteractionMode::Navigate),
            ComponentState::Focused
        );
        assert_eq!(
            component_state(focused, InteractionMode::Adjust),
            ComponentState::Adjusting
        );
    }

    /// A row mid-structural-edit paints its phase and its active graph, and
    /// marks the requested value the projection does not carry.
    ///
    /// This is the C-003 case the row exists to get right: everything the
    /// lifecycle actually reports is shown, and the one field it does not report
    /// is marked rather than filled with the active value under another name.
    #[test]
    fn a_row_mid_structural_edit_shows_its_phase_and_marks_its_unprojected_request() {
        let mut state = super::super::section::probe::installed_state(TopLevelContext::Patch);
        let entry = state
            .effects()
            .descriptors()
            .first()
            .expect("the production effect registry carries an entry")
            .id()
            .clone();
        state
            .apply(crate::control::AppEvent::SetSlotOccupancy {
                patch_id: crate::kernel::PatchId::new(1).expect("probe PatchId"),
                slot: crate::synth::effect_slot_id::EffectSlotIndex::new(0).expect("first slot"),
                entry: Some(entry),
            })
            .expect("the occupancy request enters the prepared-graph lifecycle");
        let (_, _, _, projection, _) = crate::control::StateProjector::new()
            .project_with_shell(&state)
            .expect("the production projector derives a coherent shell");
        let model = projection.semantic_model();
        let targeted = model
            .surfaces()
            .iter()
            .flat_map(SemanticSurfaceViewModel::controls)
            .find(|control| reports_structural_edit(control))
            .expect("a targeted structural row is in flight")
            .clone();

        let shown = paint_with(ViewportDensityPolicy::Desktop, |ui| {
            render_row(
                ui,
                &targeted,
                model.interaction_mode(),
                PresentationRole::ListedRow,
                &ViewportDensityPolicy::Desktop,
            )
        });
        let status = targeted
            .status()
            .expect("the targeted row carries a status");
        assert!(
            shown.says(status.label()),
            "the lifecycle band dropped the projected phase"
        );
        assert!(
            shown.says(ACTIVE_GRAPH_LABEL) && shown.says(&status.graph_revision().to_string()),
            "the lifecycle band did not make the active graph explicit"
        );
        assert!(
            shown.says(REQUESTED_VALUE_LABEL) && shown.says(UNAVAILABLE_MARK),
            "the lifecycle band invented a requested value instead of marking it"
        );
    }

    /// A resting row carries no lifecycle band at all.
    ///
    /// The band is a readout of an edit in flight; painting it always would put
    /// a permanent `--` on every row, which is the placeholder C-003 forbids.
    #[test]
    fn a_resting_row_carries_no_lifecycle_band() {
        let controls = patch_main_controls();
        let resting = controls
            .iter()
            .find(|control| !reports_structural_edit(control))
            .expect("the PATCH surface carries a row with no edit in flight");
        let shown = paint_with(ViewportDensityPolicy::Desktop, |ui| {
            render_row(
                ui,
                resting,
                InteractionMode::Navigate,
                PresentationRole::ListedRow,
                &ViewportDensityPolicy::Desktop,
            )
        });
        assert!(
            !shown.says(ACTIVE_GRAPH_LABEL),
            "a resting row painted a lifecycle band"
        );
        assert!(
            !shown.says(REQUESTED_VALUE_LABEL),
            "a resting row painted a requested-value marker"
        );
    }

    /// A pair the model declares un-askable in a role marks unavailable rather
    /// than falling back to a generic label-and-value row.
    #[test]
    fn an_unaskable_pair_marks_unavailable_instead_of_painting_a_generic_row() {
        let controls = patch_main_controls();
        let choice = controls
            .iter()
            .find(|control| control.kind() == SemanticControlKind::Choice)
            .expect("the production PATCH surface carries an effect-slot choice");
        assert_eq!(
            control_for(choice.kind(), PresentationRole::VerticalStrip),
            ControlSelection::NotAskableInRole
        );
        let shown = paint_with(ViewportDensityPolicy::Desktop, |ui| {
            render_row(
                ui,
                choice,
                InteractionMode::Navigate,
                PresentationRole::VerticalStrip,
                &ViewportDensityPolicy::Desktop,
            )
        });
        assert!(
            shown.says(UNAVAILABLE_MARK),
            "an un-askable pair painted no unavailable marker"
        );
        assert!(
            shown.says(choice.label()),
            "the marked structure did not name itself"
        );
    }

    /// The row adds nothing and reorders nothing.
    ///
    /// Rendering every projected control through this composition emits its
    /// labels in exactly the projected order, once each.
    #[test]
    fn the_row_adds_no_row_and_reorders_nothing() {
        let controls = patch_main_controls();
        let expected: Vec<String> = controls
            .iter()
            .filter(|control| control.visible())
            .map(|control| control.label().to_owned())
            .collect();
        let shown = paint_with(ViewportDensityPolicy::Desktop, |ui| {
            let mut intent = CompositionIntent::none();
            for control in controls.iter().filter(|control| control.visible()) {
                intent.absorb(render_row(
                    ui,
                    control,
                    InteractionMode::Navigate,
                    PresentationRole::ListedRow,
                    &ViewportDensityPolicy::Desktop,
                ));
            }
            intent
        });
        let emitted: Vec<String> = shown
            .texts
            .iter()
            .filter(|run| expected.iter().any(|label| label == *run))
            .cloned()
            .collect();
        assert_eq!(
            emitted, expected,
            "the row composition inserted, dropped, or reordered a projected row"
        );
    }

    /// Both authored viewports paint the same content, in every role the row can
    /// be asked in.
    #[test]
    fn both_viewports_paint_the_same_content_in_every_role() {
        let controls = patch_main_controls();
        let subject = controls.first().expect("the PATCH surface has a row");
        for role in ALL_PRESENTATION_ROLES {
            let render = |density: ViewportDensityPolicy| {
                paint_with(density, |ui| {
                    render_row(ui, subject, InteractionMode::Navigate, role, &density)
                })
            };
            assert_eq!(
                render(ViewportDensityPolicy::Desktop).texts,
                render(ViewportDensityPolicy::SteamDeck).texts,
                "{} paints different content at the two viewports",
                role.canonical_name()
            );
        }
    }

    /// Every role holds the authored minimum interactive target at both
    /// viewports, so the compact layout narrows rather than shrinking a target.
    #[test]
    fn every_role_holds_the_authored_minimum_target_at_both_viewports() {
        for policy in crate::shell::visual::density::ALL_DENSITY_POLICIES {
            for role in ALL_PRESENTATION_ROLES {
                let height = match role {
                    PresentationRole::ListedRow
                    | PresentationRole::ModalEntry
                    | PresentationRole::VerticalStrip => policy.rhythm().row_height_px,
                    PresentationRole::PanelEntry => policy.utility_control().height_px,
                };
                assert!(
                    height >= MIN_INTERACTIVE_TARGET_PX,
                    "{} in {} resolves {height} px",
                    role.canonical_name(),
                    policy.canonical_name()
                );
            }
        }
    }

    /// The row paints nothing that did not arrive in the projection.
    #[test]
    fn the_row_paints_no_text_that_did_not_arrive_in_the_projection() {
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let projection = projection(context);
            let shown: Painted = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);
            for run in &shown.texts {
                assert!(
                    from_projection_or_vocabulary(&projection, run),
                    "{context:?} painted {run:?}, which is neither projected nor declared"
                );
            }
        }
    }
}
