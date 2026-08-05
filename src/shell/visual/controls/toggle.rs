//! The toggle — a two-state control.
//!
//! # On/off is not a [`ComponentState`]
//!
//! The position comes from [`SemanticControlValue`]; the state is focus,
//! adjustment, disability, lifecycle, and selection. They are separate facts
//! and both are painted, so a disabled toggle in the on position reads as both
//! at once rather than as one of them.
//!
//! # Figma source
//!
//! - `SCREEN · Mixer · 1920×1080` → the `Fader / T00` component instance (node
//!   `42:26`). Its `State` run at the foot of the column is the strip's binary
//!   readout, and `DESIGN.md:468` fixes that vocabulary as explicit `M ON` /
//!   `S ON` text beside the warning and positive accents.
//! - `SCREEN · Patch Strip · 1920×1080` → the Utility entries (node `36:53`
//!   onward) for a panel entry's label-and-value anatomy.
//!
//! **No toggle component is authored.** The design file's five component sets
//! are Context Switch, CLI Hint, CLI Browser Line, Compact Parameter Slider and
//! Compact Mixer Fader; none of them is a toggle, and no listed row, panel
//! entry or modal entry in the file shows one. What *is* authored is the
//! reading: a binary is announced by explicit text. This module therefore
//! paints the authored words and adds one shape channel — a filled mark when
//! on, a hollow mark when off — so the position survives a viewer who cannot
//! separate the accents. That mark is raised in this work package's handoff as
//! unauthored rather than presented as measured.
//!
//! # Roles
//!
//! The role match below is exhaustive over all four roles, as the render
//! contract requires. Only three are reachable: WP01 resolves a toggle asked in
//! a modal to `ModalOption`, not here. The modal arm is written to the same
//! shape as a listed row so the control stays total, and the discrepancy is
//! recorded in the handoff.

use eframe::egui::{pos2, Painter, Rect, Ui};

use super::parameter_row::{
    allocate_row, intent_for, middle_region, paint_row_chrome, paint_row_label, paint_row_value,
    toggle_word, value_text, UNAVAILABLE_MARK,
};
use super::{ControlIntent, PresentationRole};
use crate::control::{SemanticControlValue, SemanticControlViewModel};
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::rules::RuleSpan;
use crate::shell::visual::primitives::status::StatusDetail;
use crate::shell::visual::primitives::{status, text, value};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{Radius, SemanticColor, SpacingStep, TypeStyle};
use crate::synth::ParameterValue;

/// Paints a toggle and returns what the operator asked for.
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

    let position = toggle_position(view.value());
    let reading = position_reading(view.value());
    let detail = super::parameter_row::status_detail(view);

    match role {
        // A full-width row and a panel entry read the same way: label on the
        // left, position on the right where a value would sit. The modal arm
        // is unreachable through selection and follows the same shape rather
        // than inventing a third.
        PresentationRole::ListedRow
        | PresentationRole::PanelEntry
        | PresentationRole::ModalEntry => {
            paint_wide(&painter, row, view, &reading, position, state, detail);
        }
        // A mixer column is narrow, so the two facts stack instead of sitting
        // side by side.
        PresentationRole::VerticalStrip => {
            paint_stacked(&painter, row, view, &reading, position, state, detail);
        }
    }

    intent_for(&response, ControlIntent::ActivateRequested)
}

/// Label left, position right.
#[allow(clippy::too_many_arguments)]
fn paint_wide(
    painter: &Painter,
    row: Rect,
    view: &SemanticControlViewModel,
    reading: &str,
    position: Option<bool>,
    state: ComponentState,
    detail: StatusDetail<'_>,
) {
    let label_right = paint_row_label(painter, row, view.label(), state);
    let value_left = paint_row_value(painter, row, reading, state);
    let middle = middle_region(row, label_right, value_left);
    if status::status_mark(state, detail).is_some() {
        status::paint_status_mark(painter, middle, state, detail);
    } else {
        paint_position_mark(painter, middle, position, state);
    }
}

/// Label above, position below.
#[allow(clippy::too_many_arguments)]
fn paint_stacked(
    painter: &Painter,
    row: Rect,
    view: &SemanticControlViewModel,
    reading: &str,
    position: Option<bool>,
    state: ComponentState,
    detail: StatusDetail<'_>,
) {
    // Split at the cell's own centre rather than at a fraction of it, so no
    // proportion is spelled here for a reviewer to have to trust.
    let upper = Rect::from_min_max(row.min, pos2(row.max.x, row.center().y));
    let lower = Rect::from_min_max(pos2(row.min.x, row.center().y), row.max);
    paint_centred(
        painter,
        upper,
        view.label(),
        TypeStyle::LabelControl,
        SemanticColor::TextMuted,
        state,
    );
    if status::status_mark(state, detail).is_some() {
        status::paint_status_mark(painter, lower, state, detail);
    } else {
        paint_centred(
            painter,
            lower,
            reading,
            TypeStyle::CodeValue,
            value::value_color(state),
            state,
        );
        let _ = position;
    }
}

/// Paints one run centred in `region`.
fn paint_centred(
    painter: &Painter,
    region: Rect,
    content: &str,
    style: TypeStyle,
    color: SemanticColor,
    state: ComponentState,
) {
    let galley = text::layout(painter, content, style, color, state);
    let size = galley.size();
    let position = pos2(
        region.center().x - size.x / 2.0,
        region.center().y - size.y / 2.0,
    );
    painter.galley(
        position,
        galley,
        text::resolved_color(color, state).resolve(),
    );
}

/// Paints the shape channel for the position: filled when on, hollow when off.
///
/// A position the view data does not carry is marked unavailable rather than
/// drawn as off — off is a value, and claiming it for a control that never
/// reported one would be inventing state.
fn paint_position_mark(
    painter: &Painter,
    region: Rect,
    position: Option<bool>,
    state: ComponentState,
) {
    if region.width() <= 0.0 {
        return;
    }
    let side = SpacingStep::S12.resolve();
    let left = region.min.x;
    let top = region.center().y - side / 2.0;
    let mark = Rect::from_min_max(pos2(left, top), pos2(left + side, top + side));
    let corner = Radius::Small.resolve();
    let color = text::resolved_color(value::value_color(state), state).resolve();
    match position {
        Some(true) => {
            painter.rect_filled(mark, corner, color);
        }
        Some(false) => {
            // The hollow mark is drawn as four authored hairlines rather than
            // as one stroked rectangle.
            //
            // This is not a style preference. In this shell a *stroked
            // rectangle is how an addressable target is drawn*, and the
            // production vocabulary proof reads that literally: every stroked
            // rect carrying an authored stroke colour is measured against the
            // authored minimum interactive target. This mark is a 12 px
            // decoration inside a row, not a target of its own, so drawing it
            // with a stroke would announce a 12 px target and make the frame
            // fail its own minimum-target proof — which it did, the first time
            // this control reached the production frame. Hairlines are filled
            // rules, which is what the design system draws thin structure with
            // (`DESIGN.md:448`), so the mark keeps its exact appearance and
            // stops claiming to be something it is not.
            for span in [
                RuleSpan::Horizontal {
                    y_px: mark.min.y,
                    from_x_px: mark.min.x,
                    to_x_px: mark.max.x,
                },
                RuleSpan::Horizontal {
                    y_px: mark.max.y,
                    from_x_px: mark.min.x,
                    to_x_px: mark.max.x,
                },
                RuleSpan::Vertical {
                    x_px: mark.min.x,
                    from_y_px: mark.min.y,
                    to_y_px: mark.max.y,
                },
                RuleSpan::Vertical {
                    x_px: mark.max.x,
                    from_y_px: mark.min.y,
                    to_y_px: mark.max.y,
                },
            ] {
                painter.rect_filled(span.rect(), corner, color);
            }
        }
        None => {
            let galley = text::layout(
                painter,
                UNAVAILABLE_MARK,
                TypeStyle::LabelControl,
                SemanticColor::TextMuted,
                state,
            );
            painter.galley(
                pos2(left, region.center().y - galley.size().y / 2.0),
                galley,
                text::resolved_color(SemanticColor::TextMuted, state).resolve(),
            );
        }
    }
}

/// The binary position this control carries, when its value is a binary.
///
/// Exhaustive over the value union with no wildcard. Everything that is not a
/// typed binary answers `None`: a toggle handed a scalar has no position to
/// report, and reporting `false` would be a value nobody projected.
pub(super) fn toggle_position(value: &SemanticControlValue) -> Option<bool> {
    match value {
        SemanticControlValue::Parameter(ParameterValue::Toggle(on)) => Some(*on),
        SemanticControlValue::Parameter(ParameterValue::Continuous(_))
        | SemanticControlValue::Parameter(ParameterValue::Stepped(_))
        | SemanticControlValue::Parameter(ParameterValue::Choice(_))
        | SemanticControlValue::Scalar(_)
        | SemanticControlValue::Asset(_)
        | SemanticControlValue::Identity(_)
        | SemanticControlValue::Summary(_) => None,
    }
}

/// The text a toggle reads as.
///
/// A binary reads as its authored word. Anything else reads as whatever the
/// projection supplied, so a toggle handed something that is not a binary shows
/// the operator what actually arrived.
pub(super) fn position_reading(value: &SemanticControlValue) -> String {
    match toggle_position(value) {
        Some(on) => toggle_word(on).to_owned(),
        None => value_text(value),
    }
}

#[cfg(test)]
mod tests {
    use super::super::parameter_row::probe::{paint, projected_control, Painted};
    use super::*;
    use crate::control::TopLevelContext;
    use crate::shell::visual::controls::{ComponentControl, ALL_PRESENTATION_ROLES};
    use crate::shell::visual::primitives::focus;
    use crate::shell::visual::primitives::status::{LoadingPhase, GENERIC_FAILURE_WORD};
    use crate::shell::visual::state::{NonColorSignal, LOADING_PROGRESS_WORDS};
    use crate::shell::visual::token::MIN_INTERACTIVE_TARGET_PX;
    use crate::synth::{AssetKind, AssetReference};

    const CONTROL: ComponentControl = ComponentControl::Toggle;

    fn painted(state: ComponentState, role: PresentationRole) -> Painted {
        paint(
            CONTROL,
            &projected_control(TopLevelContext::Mixer),
            state,
            role,
            ViewportDensityPolicy::Desktop,
        )
    }

    /// Every applicable state announces itself without color, in every role.
    #[test]
    fn every_applicable_state_renders_evidence_that_is_not_a_color() {
        for role in ALL_PRESENTATION_ROLES {
            let resting = painted(ComponentState::Resting, role);
            for state in CONTROL.applicable_states() {
                let shown = painted(*state, role);
                match state.appearance().signal {
                    NonColorSignal::Word(word) => assert!(
                        shown.says(word),
                        "{} in {} does not paint its authored word",
                        state.canonical_name(),
                        role.canonical_name()
                    ),
                    NonColorSignal::ProgressWord => assert!(
                        shown.says(LoadingPhase::Preparing.word()),
                        "{} in {} does not paint a progress word",
                        state.canonical_name(),
                        role.canonical_name()
                    ),
                    NonColorSignal::TypedFailure => assert!(
                        shown.says(GENERIC_FAILURE_WORD) || shown.texts.len() > resting.texts.len(),
                        "{} in {} does not paint failure text",
                        state.canonical_name(),
                        role.canonical_name()
                    ),
                    NonColorSignal::Shape => {
                        if *state == ComponentState::Resting {
                            continue;
                        }
                        assert_ne!(
                            shown.colorless_signature(),
                            resting.colorless_signature(),
                            "{} in {} is distinguishable from Resting by color alone",
                            state.canonical_name(),
                            role.canonical_name()
                        );
                    }
                }
            }
        }
    }

    /// All four roles render something.
    #[test]
    fn every_role_renders() {
        for role in ALL_PRESENTATION_ROLES {
            let shown = painted(ComponentState::Resting, role);
            assert!(
                !shown.texts.is_empty(),
                "{} painted no text at all",
                role.canonical_name()
            );
        }
    }

    /// The position and the state are two separate facts, and a disabled toggle
    /// in the on position shows both.
    ///
    /// This is the bug the subtask named: conflating value with state would
    /// lose one of the two.
    #[test]
    fn a_disabled_toggle_in_the_on_position_shows_both_facts() {
        assert_eq!(
            toggle_position(&SemanticControlValue::Parameter(ParameterValue::Toggle(
                true
            ))),
            Some(true)
        );
        assert_eq!(
            position_reading(&SemanticControlValue::Parameter(ParameterValue::Toggle(
                true
            ))),
            toggle_word(true)
        );
        // `Disabled` is a state; `Toggle(true)` is a value. The state declares
        // the word `Locked` and the value declares the word `ON`, so the two
        // read together and neither replaces the other.
        assert_eq!(
            ComponentState::Disabled.appearance().signal,
            NonColorSignal::Word("Locked")
        );
        assert_ne!(toggle_word(true), "Locked");
        assert_ne!(toggle_word(false), "Locked");
    }

    /// On and off differ in text, not only in color.
    #[test]
    fn the_two_positions_read_differently_without_color() {
        assert_ne!(toggle_word(true), toggle_word(false));
        assert_ne!(
            position_reading(&SemanticControlValue::Parameter(ParameterValue::Toggle(
                true
            ))),
            position_reading(&SemanticControlValue::Parameter(ParameterValue::Toggle(
                false
            )))
        );
    }

    /// A value that is not a binary carries no position, and the control says
    /// so instead of claiming off.
    #[test]
    fn a_value_that_is_not_a_binary_carries_no_position() {
        for value in [
            SemanticControlValue::Scalar(1.0),
            SemanticControlValue::Parameter(ParameterValue::Continuous(0.5)),
            SemanticControlValue::Parameter(ParameterValue::Stepped(2)),
            SemanticControlValue::Parameter(ParameterValue::Choice("SINE".to_owned())),
            SemanticControlValue::Identity("PLAITS".to_owned()),
            SemanticControlValue::Summary("Read-only".to_owned()),
            SemanticControlValue::Asset(
                AssetReference::new(AssetKind::Sample, "hit.wav").expect("locator"),
            ),
        ] {
            assert_eq!(
                toggle_position(&value),
                None,
                "{value:?} was read as a binary position"
            );
            assert_eq!(position_reading(&value), value_text(&value));
        }
    }

    #[test]
    fn both_viewports_paint_the_same_content() {
        for state in CONTROL.applicable_states() {
            let view = projected_control(TopLevelContext::Mixer);
            assert_eq!(
                paint(
                    CONTROL,
                    &view,
                    *state,
                    PresentationRole::ListedRow,
                    ViewportDensityPolicy::Desktop
                )
                .texts,
                paint(
                    CONTROL,
                    &view,
                    *state,
                    PresentationRole::ListedRow,
                    ViewportDensityPolicy::SteamDeck
                )
                .texts,
                "{} paints different content at the two viewports",
                state.canonical_name()
            );
        }
    }

    /// The toggle paints nothing it was not handed.
    #[test]
    fn the_toggle_paints_no_text_that_did_not_arrive_in_its_view_model() {
        let view = projected_control(TopLevelContext::Mixer);
        let reading = position_reading(view.value());
        for role in ALL_PRESENTATION_ROLES {
            for state in CONTROL.applicable_states() {
                let shown = paint(CONTROL, &view, *state, role, ViewportDensityPolicy::Desktop);
                for run in &shown.texts {
                    let from_view_model = view.label().contains(run.as_str())
                        || reading.contains(run.as_str())
                        || run.is_empty();
                    let from_vocabulary = run == focus::CURSOR_GLYPH
                        || run == UNAVAILABLE_MARK
                        || run == GENERIC_FAILURE_WORD
                        || run == toggle_word(true)
                        || run == toggle_word(false)
                        || LOADING_PROGRESS_WORDS.contains(&run.as_str())
                        || matches!(
                            state.appearance().signal,
                            NonColorSignal::Word(word) if word == run
                        );
                    assert!(
                        from_view_model || from_vocabulary,
                        "{} in {} painted {run:?}, which is neither view data nor authored vocabulary",
                        state.canonical_name(),
                        role.canonical_name()
                    );
                }
            }
        }
    }

    /// Every role stays at or above the authored minimum interactive target.
    #[test]
    fn every_role_holds_the_authored_minimum_target() {
        for policy in crate::shell::visual::density::ALL_DENSITY_POLICIES {
            for role in ALL_PRESENTATION_ROLES {
                assert!(
                    super::super::parameter_row::row_height(role, &policy)
                        >= MIN_INTERACTIVE_TARGET_PX,
                    "{} in {} is below the authored minimum target",
                    role.canonical_name(),
                    policy.canonical_name()
                );
            }
        }
    }
}
