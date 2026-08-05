//! The choice row — a row whose value is one of a declared set.
//!
//! # Figma source
//!
//! - `SCREEN · Patch Strip · 1920×1080` → the `Instrument` row (node `36:28`)
//!   and the three `FX` rows (nodes `36:34`, `36:39`, `36:44`). These are the
//!   product's archetypal adjacent-choice rows.
//! - `SCREEN · Sample Detail · 1920×1080` → `Sample Parameter / LOOP MODE`
//!   (node `39:288`), the one genuinely choice-valued parameter row in the
//!   file.
//!
//! Both specimens carry the *same* anatomy as a parameter row: a reserved
//! cursor column, a label, and the active value. `LOOP MODE` reuses the numeric
//! parameter component verbatim. The design file therefore says a choice row is
//! a listed row whose value happens to be a name, and this module renders
//! exactly that.
//!
//! # The adjacency affordance is not authored
//!
//! This control's subtask asked for a visible "there are neighbours in this
//! direction" affordance that reads *unavailable* rather than absent at the
//! first and last choice. The design file does not author one:
//!
//! - no choice-row or adjacency component exists in the file's component sets;
//! - the Patch Strip's choice rows carry only the action-hint text
//!   `L/R:change`, which is a hint and not a per-direction affordance;
//! - `LOOP MODE` shows no direction marks at all.
//!
//! The view model is silent too: [`SemanticControlViewModel`] carries no choice
//! set, no neighbour, and no at-a-boundary flag, so this control could not
//! report availability per direction even if a treatment were authored. Both
//! gaps are raised in this work package's handoff rather than approximated. An
//! invented chevron pair would look authoritative and would be believed.
//!
//! # What this module owns
//!
//! Nothing. It paints what it is handed and returns [`ControlIntent`]. It does
//! not decide whether a neighbouring choice is legal — that is the reducer's,
//! and this control never reaches it.

use eframe::egui::Ui;

use super::parameter_row::{
    allocate_row, intent_for, middle_region, paint_row_chrome, paint_row_label, paint_row_value,
    parameter_text, status_detail,
};
use super::{ControlIntent, PresentationRole};
use crate::control::{SemanticControlValue, SemanticControlViewModel};
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::status;
use crate::shell::visual::state::ComponentState;

/// Paints a choice row and returns what the operator asked for.
pub fn render(
    ui: &mut Ui,
    view: &SemanticControlViewModel,
    state: ComponentState,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> ControlIntent {
    let response = allocate_row(ui, role, density);
    let painter = ui.painter_at(response.rect);
    let row = response.rect;
    paint_row_chrome(&painter, row, state);

    let label_right = paint_row_label(&painter, row, view.label(), state);
    let active = active_choice_text(view.value());
    let value_left = paint_row_value(&painter, row, &active, state);

    // A choice has no position inside bounds, so the middle region carries the
    // row's lifecycle and nothing else. Where a parameter row would draw an
    // indicator, this one stays empty.
    let detail = status_detail(view);
    if status::status_mark(state, detail).is_some() {
        status::paint_status_mark(
            &painter,
            middle_region(row, label_right, value_left),
            state,
            detail,
        );
    }

    intent_for(&response, ControlIntent::FocusRequested)
}

/// The active choice, as finished text.
///
/// A choice arrives either as a typed [`ParameterValue::Choice`] or — for the
/// effect-slot occupancy and output-track rows the projection builds — as an
/// already-resolved [`SemanticControlValue::Identity`]. Both are names the
/// projection chose; neither is re-derived here.
///
/// Exhaustive over the value union with no wildcard, so a new value kind is a
/// compile error rather than a row that renders nothing.
///
/// [`ParameterValue::Choice`]: crate::synth::ParameterValue::Choice
pub(super) fn active_choice_text(value: &SemanticControlValue) -> String {
    match value {
        SemanticControlValue::Parameter(parameter) => parameter_text(parameter),
        SemanticControlValue::Identity(name) | SemanticControlValue::Summary(name) => name.clone(),
        SemanticControlValue::Asset(reference) => reference.locator().to_owned(),
        SemanticControlValue::Scalar(scalar) => {
            let places = super::parameter_row::VALUE_DECIMAL_PLACES;
            format!("{scalar:.places$}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::parameter_row::probe::{paint, projected_control, Painted};
    use super::*;
    use crate::control::TopLevelContext;
    use crate::shell::visual::controls::ComponentControl;
    use crate::shell::visual::primitives::focus;
    use crate::shell::visual::primitives::status::{LoadingPhase, GENERIC_FAILURE_WORD};
    use crate::shell::visual::state::{NonColorSignal, LOADING_PROGRESS_WORDS};
    use crate::synth::{AssetKind, AssetReference, ParameterValue};

    const CONTROL: ComponentControl = ComponentControl::ChoiceRow;

    fn painted(state: ComponentState, density: ViewportDensityPolicy) -> Painted {
        paint(
            CONTROL,
            &projected_control(TopLevelContext::Patch),
            state,
            PresentationRole::ListedRow,
            density,
        )
    }

    /// Every applicable state announces itself without color.
    #[test]
    fn every_applicable_state_renders_evidence_that_is_not_a_color() {
        let resting = painted(ComponentState::Resting, ViewportDensityPolicy::Desktop);
        for state in CONTROL.applicable_states() {
            let shown = painted(*state, ViewportDensityPolicy::Desktop);
            match state.appearance().signal {
                NonColorSignal::Word(word) => assert!(
                    shown.says(word),
                    "{} does not paint its authored word",
                    state.canonical_name()
                ),
                NonColorSignal::ProgressWord => assert!(
                    shown.says(LoadingPhase::Preparing.word()),
                    "{} does not paint a progress word",
                    state.canonical_name()
                ),
                NonColorSignal::TypedFailure => assert!(
                    shown.says(GENERIC_FAILURE_WORD) || shown.texts.len() > resting.texts.len(),
                    "{} does not paint failure text",
                    state.canonical_name()
                ),
                NonColorSignal::Shape => {
                    if *state == ComponentState::Resting {
                        continue;
                    }
                    assert_ne!(
                        shown.colorless_signature(),
                        resting.colorless_signature(),
                        "{} is distinguishable from Resting by color alone",
                        state.canonical_name()
                    );
                }
            }
        }
    }

    /// A structural choice mid-edit reports the authored progress word beside
    /// its active value, so the two are readable together.
    #[test]
    fn a_choice_mid_structural_edit_shows_its_active_value_and_its_progress() {
        let view = projected_control(TopLevelContext::Patch);
        let shown = paint(
            CONTROL,
            &view,
            ComponentState::Loading,
            PresentationRole::ListedRow,
            ViewportDensityPolicy::Desktop,
        );
        assert!(
            shown.says(&active_choice_text(view.value())),
            "the active choice is not readable while the edit runs"
        );
        assert!(
            LOADING_PROGRESS_WORDS.iter().any(|word| shown.says(word)),
            "no authored progress word reached the row"
        );
    }

    #[test]
    fn focus_and_adjustment_paint_the_authored_cursor() {
        for state in [ComponentState::Focused, ComponentState::Adjusting] {
            assert!(
                painted(state, ViewportDensityPolicy::Desktop).says(focus::CURSOR_GLYPH),
                "{} does not paint the cursor",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn both_viewports_paint_the_same_content() {
        for state in CONTROL.applicable_states() {
            assert_eq!(
                painted(*state, ViewportDensityPolicy::Desktop).texts,
                painted(*state, ViewportDensityPolicy::SteamDeck).texts,
                "{} paints different content at the two viewports",
                state.canonical_name()
            );
        }
    }

    /// The row paints nothing it was not handed.
    #[test]
    fn the_row_paints_no_text_that_did_not_arrive_in_its_view_model() {
        let view = projected_control(TopLevelContext::Patch);
        let active = active_choice_text(view.value());
        for state in CONTROL.applicable_states() {
            let shown = paint(
                CONTROL,
                &view,
                *state,
                PresentationRole::ListedRow,
                ViewportDensityPolicy::Desktop,
            );
            for run in &shown.texts {
                let from_view_model = view.label().contains(run.as_str())
                    || active.contains(run.as_str())
                    || run.is_empty();
                let from_vocabulary = run == focus::CURSOR_GLYPH
                    || run == GENERIC_FAILURE_WORD
                    || LOADING_PROGRESS_WORDS.contains(&run.as_str())
                    || matches!(
                        state.appearance().signal,
                        NonColorSignal::Word(word) if word == run
                    );
                assert!(
                    from_view_model || from_vocabulary,
                    "{} painted {run:?}, which is neither view data nor authored vocabulary",
                    state.canonical_name()
                );
            }
        }
    }

    /// The active choice is read from the view data and never re-derived.
    #[test]
    fn every_value_kind_resolves_to_the_name_the_projection_supplied() {
        assert_eq!(
            active_choice_text(&SemanticControlValue::Parameter(ParameterValue::Choice(
                "SINE".to_owned()
            ))),
            "SINE"
        );
        // The effect-slot occupancy rows the projection builds carry their
        // resolved label as an identity rather than a typed choice.
        assert_eq!(
            active_choice_text(&SemanticControlValue::Identity("Chorus".to_owned())),
            "Chorus"
        );
        assert_eq!(
            active_choice_text(&SemanticControlValue::Identity("Empty".to_owned())),
            "Empty"
        );
        assert_eq!(
            active_choice_text(&SemanticControlValue::Asset(
                AssetReference::new(AssetKind::SoundFont, "hidef.sf2").expect("locator")
            )),
            "hidef.sf2"
        );
        for value in [
            SemanticControlValue::Scalar(0.5),
            SemanticControlValue::Parameter(ParameterValue::Stepped(3)),
            SemanticControlValue::Parameter(ParameterValue::Toggle(true)),
            SemanticControlValue::Summary("Read-only".to_owned()),
        ] {
            assert!(
                !active_choice_text(&value).is_empty(),
                "{value:?} rendered as nothing"
            );
        }
    }

    /// This control reports no opinion about whether a neighbouring choice
    /// exists.
    ///
    /// The view model carries no choice set and no boundary flag, so there is
    /// nothing to read; this records that the control does not fabricate one.
    #[test]
    fn the_row_reports_no_adjacency_it_was_not_given() {
        let view = projected_control(TopLevelContext::Patch);
        let shown = paint(
            CONTROL,
            &view,
            ComponentState::Focused,
            PresentationRole::ListedRow,
            ViewportDensityPolicy::Desktop,
        );
        let active = active_choice_text(view.value());
        for run in &shown.texts {
            assert!(
                run != "<" && run != ">" || run == focus::CURSOR_GLYPH,
                "the row painted a direction mark the design file does not author"
            );
            let _ = &active;
        }
    }
}
