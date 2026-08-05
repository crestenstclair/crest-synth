//! The modal option.
//!
//! One selectable entry inside a focus-trapped option modal.
//!
//! # Specimen
//!
//! Design file `kdQMw8dYUZtv2UxJPo0sXU`, frame `STATE · ENGINE TYPE OPTIONS`
//! (`48:173`) and its twin `STATE · POST FX OPTIONS` (`48:207`). The modal is
//! 920 × 680 and its option rows are 872 × 48 on a 56 px pitch, so the row sits
//! on the authored interactive minimum with an authored `space/8` between rows —
//! denser than the 52 px surface row, which is what a list of near-identical
//! entries needs. Inside a row the cursor occupies x=10 and the option name
//! starts at x=18, which is the reserved cursor column
//! ([`focus::LABEL_START_X_PX`]) the vocabulary already declares, and a hairline
//! closes each row.
//!
//! # Only the name, and the mark
//!
//! A row in both specimens carries the option name and, on the current row, a
//! right-hand mark — `PLAITS … CURRENT`, `SAMPLE`, `MELTY SYNTH`. There is no
//! value readout and no leader rule between the two. Neither is painted here.
//!
//! An earlier pass added both, reasoning that a control generic over
//! [`SemanticControlKind`](crate::control::SemanticControlKind) might be handed
//! a value worth showing. That was an addition to an unambiguous specimen made
//! without raising it, which is the drift this vocabulary exists to prevent: a
//! modal is a list of choices, and a choice is named rather than measured. If a
//! modal is later authored with a value column, the specimen for it decides what
//! that column is.
//!
//! # `Selected` is not `Focused`
//!
//! In a modal these two co-occur — the current value is one of the rows the
//! cursor moves over — so they must not look alike. They do not:
//!
//! - `Focused` draws the `>` in the reserved column plus the authored 3 px
//!   keyline and halo, and fills nothing.
//! - `Selected` fills its row and paints the authored selection mark at the
//!   right, and draws no keyline and no cursor.
//!
//! Neither difference is a color. The specimen marks its current row with the
//! word `CURRENT`; the state vocabulary declares `Selected`'s signal as a shape
//! rather than a word, so the authored shape is what is painted here and the
//! divergence is recorded as a finding rather than resolved by inventing a word
//! the vocabulary does not declare.
//!
//! # What it does not do
//!
//! It does not trap focus, does not own the modal, and does not know what
//! opened it or where closing it returns to. Focus trapping and the return path
//! are reducer and composition concerns (`DESIGN.md:458`); this control paints
//! one entry and reports that the operator asked to commit it.

use eframe::egui::{pos2, Painter, Rect, Response, Sense, Ui, Vec2};

use super::compact_slider::status_detail;
use super::{ControlIntent, PresentationRole};
use crate::control::SemanticControlViewModel;
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::rules::{hairline, RuleSpan};
use crate::shell::visual::primitives::{focus, status, text};
use crate::shell::visual::state::{ComponentState, LOADING_PROGRESS_WORDS};
use crate::shell::visual::token::{
    SemanticColor, SpacingStep, TypeStyle, MIN_INTERACTIVE_TARGET_PX,
};

/// The height of one option row.
///
/// A modal row reads denser than a surface row, and the specimen puts it
/// exactly on the authored interactive minimum. Taking the denser of the
/// policy's row height and that minimum is what says so: the compact viewport's
/// rhythm is already at the floor, and the desktop's 52 px is not what a modal
/// list uses.
fn row_height_px(density: &ViewportDensityPolicy) -> f32 {
    density
        .rhythm()
        .row_height_px
        .min(MIN_INTERACTIVE_TARGET_PX)
}

/// The horizontal span content occupies inside a row.
///
/// The left inset is the reserved cursor column, reserved for every state so
/// that focusing a row never shifts the option name under the cursor.
fn content_span(row: Rect) -> Rect {
    Rect::from_min_max(
        pos2(row.min.x + focus::LABEL_START_X_PX, row.min.y),
        pos2(row.max.x - SpacingStep::S12.resolve(), row.max.y),
    )
}

/// The right-hand column the state's status mark is painted in.
///
/// Its width is measured from the longest word any state can say, in the style
/// it says it in, so the column is exactly wide enough for the authored
/// vocabulary rather than a width chosen here. The status primitive left-aligns
/// inside whatever rect it is handed, so this is what puts the mark at the
/// right-hand edge the specimen shows.
fn mark_column(painter: &Painter, content: Rect, state: ComponentState) -> Rect {
    let widest = LOADING_PROGRESS_WORDS
        .iter()
        .map(|word| {
            text::layout(
                painter,
                *word,
                TypeStyle::LabelControl,
                SemanticColor::TextMuted,
                state,
            )
            .size()
            .x
        })
        .fold(0.0_f32, f32::max);
    Rect::from_min_max(pos2(content.max.x - widest, content.min.y), content.max)
}

/// What the operator asked this control for during the frame it was painted.
///
/// Choosing an entry is a commit, so a click reports exactly that. Whether the
/// choice is legal, and what it does to focus and to the modal, is decided
/// elsewhere.
fn intent(response: &Response) -> ControlIntent {
    if response.clicked() {
        ControlIntent::ActivateRequested
    } else {
        ControlIntent::None
    }
}

/// Paints one modal option and returns what it asks for.
pub fn render(
    ui: &mut Ui,
    view: &SemanticControlViewModel,
    state: ComponentState,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> ControlIntent {
    let width = match role {
        PresentationRole::ModalEntry
        | PresentationRole::ListedRow
        | PresentationRole::PanelEntry
        | PresentationRole::VerticalStrip => ui.available_width(),
    };
    let size = Vec2::new(width.max(MIN_INTERACTIVE_TARGET_PX), row_height_px(density));
    let (row, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter().clone();
    let content = content_span(row);

    status::paint_row_fill(&painter, row, state);
    focus::cursor(&painter, row, state);

    let name = text::layout(
        &painter,
        view.label(),
        TypeStyle::BodyDefault,
        SemanticColor::TextPrimary,
        state,
    );
    painter.galley(
        pos2(content.min.x, content.center().y - name.size().y / 2.0),
        name,
        text::resolved_color(SemanticColor::TextPrimary, state).resolve(),
    );

    let mark = mark_column(&painter, content, state);
    status::paint_status_mark(&painter, mark, state, status_detail(view));
    // Entries are separated by hairlines rather than by cards
    // (`DESIGN.md:447`).
    hairline(
        &painter,
        RuleSpan::Horizontal {
            y_px: row.max.y,
            from_x_px: row.min.x,
            to_x_px: row.max.x,
        },
    );
    focus::focus_frame(&painter, row, state);

    intent(&response)
}

#[cfg(test)]
mod tests {
    use super::super::compact_slider::harness::{paint, paint_states, ranged_control};
    use super::super::compact_slider::value_text;
    use super::*;
    use crate::control::SemanticControlKind;
    use crate::shell::visual::controls::{control_for, ComponentControl, ControlSelection};
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::state::NonColorSignal;
    use crate::shell::visual::token::KEYLINE_RESTING_PX;

    const CONTROL: ComponentControl = ComponentControl::ModalOption;
    const ROLE: PresentationRole = PresentationRole::ModalEntry;

    /// Every kind is askable inside a modal, and every one of them resolves
    /// here. A control nothing can ask for is dead code.
    #[test]
    fn every_kind_asked_in_a_modal_resolves_to_this_control() {
        for kind in [
            SemanticControlKind::Continuous,
            SemanticControlKind::Stepped,
            SemanticControlKind::Choice,
            SemanticControlKind::Toggle,
            SemanticControlKind::Identity,
            SemanticControlKind::Surface,
        ] {
            assert_eq!(
                control_for(kind, ROLE),
                ControlSelection::Control(CONTROL),
                "a modal entry no longer resolves to the modal option"
            );
        }
    }

    #[test]
    fn every_applicable_state_paints_its_declared_non_color_signal() {
        let view = ranged_control();
        for density in ALL_DENSITY_POLICIES {
            let states = CONTROL.applicable_states();
            for (state, painted) in states
                .iter()
                .zip(paint_states(CONTROL, &view, states, ROLE, density))
            {
                assert!(
                    painted.painted_anything(),
                    "{} painted nothing at {}",
                    state.canonical_name(),
                    density.canonical_name()
                );
                match state.appearance().signal {
                    NonColorSignal::Word(word) => assert!(
                        painted.says(word),
                        "{} did not paint its authored word {word} at {}",
                        state.canonical_name(),
                        density.canonical_name()
                    ),
                    NonColorSignal::ProgressWord => assert!(
                        painted.says("Preparing") || painted.says("Activating"),
                        "{} painted no authored progress word at {}",
                        state.canonical_name(),
                        density.canonical_name()
                    ),
                    NonColorSignal::TypedFailure => assert!(
                        painted.says("Failed"),
                        "{} painted no failure text at {}",
                        state.canonical_name(),
                        density.canonical_name()
                    ),
                    NonColorSignal::Shape => assert!(
                        !painted.rects.is_empty(),
                        "{} painted no shape at {}",
                        state.canonical_name(),
                        density.canonical_name()
                    ),
                }
            }
        }
    }

    #[test]
    fn no_two_applicable_states_look_alike_without_color() {
        let view = ranged_control();
        let states = CONTROL.applicable_states();
        let painted = paint_states(CONTROL, &view, states, ROLE, ViewportDensityPolicy::Desktop);
        for (index, first) in states.iter().enumerate() {
            for (offset, second) in states.iter().enumerate().skip(index + 1) {
                assert_ne!(
                    painted[index].colorless_evidence(),
                    painted[offset].colorless_evidence(),
                    "{} and {} are distinguishable only by color",
                    first.canonical_name(),
                    second.canonical_name()
                );
            }
        }
    }

    /// The pair the modal exists to keep apart. Stated on its own rather than
    /// left to the sweep above, because these two co-occur on the row a modal
    /// opens on and this is the one confusion that would make the list
    /// unreadable.
    #[test]
    fn selected_and_focused_are_told_apart_without_color() {
        let view = ranged_control();
        let states = [ComponentState::Focused, ComponentState::Selected];
        let painted = paint_states(
            CONTROL,
            &view,
            &states,
            ROLE,
            ViewportDensityPolicy::Desktop,
        );
        assert_ne!(
            painted[0].colorless_evidence(),
            painted[1].colorless_evidence(),
            "a selected modal option and a focused one are distinguishable only by color"
        );
        assert!(
            painted[0].says(focus::CURSOR_GLYPH),
            "a focused modal option did not draw the cursor"
        );
        assert!(
            !painted[1].says(focus::CURSOR_GLYPH),
            "an unfocused selected option drew the focus cursor"
        );
        assert!(
            painted[0]
                .rects
                .iter()
                .any(|painted| painted.stroke_px > 0.0),
            "a focused modal option drew no keyline"
        );
        assert!(
            painted[1]
                .rects
                .iter()
                .all(|painted| painted.stroke_px == 0.0),
            "a selected modal option drew a keyline it does not declare"
        );
    }

    #[test]
    fn a_row_sits_on_the_authored_interactive_minimum_at_both_viewports() {
        for density in ALL_DENSITY_POLICIES {
            assert_eq!(
                row_height_px(&density),
                MIN_INTERACTIVE_TARGET_PX,
                "{} moved the modal row off the authored minimum",
                density.canonical_name()
            );
            assert!(
                row_height_px(&density) <= density.rhythm().row_height_px,
                "{} makes a modal row taller than a surface row",
                density.canonical_name()
            );
        }
    }

    /// A resting row says the option's name and nothing else.
    ///
    /// Exact equality rather than a subset check, because the thing worth
    /// holding here is not only that nothing outside the view data was painted
    /// but that nothing outside the *specimen* was: the value readout and the
    /// leader rule an earlier pass added would both fail this.
    #[test]
    fn a_resting_row_paints_the_option_name_and_nothing_else() {
        let view = ranged_control();
        let painted = paint(
            CONTROL,
            &view,
            ComponentState::Resting,
            ROLE,
            ViewportDensityPolicy::Desktop,
        );
        assert_eq!(
            painted.texts,
            vec![view.label().to_owned()],
            "a resting modal option painted text beside the option name — the view \
             data carries the value {:?}, which the specimen's rows do not show",
            value_text(view.value())
        );
        // One rule, the hairline closing the row. The leader an earlier pass ran
        // between the name and the mark was a second rule at the row's middle,
        // and it is counted here rather than looked for by position so that any
        // rule the specimen does not show fails this.
        let rules = painted
            .rects
            .iter()
            .filter(|painted| (painted.rect.height() - KEYLINE_RESTING_PX).abs() < f32::EPSILON)
            .count();
        assert_eq!(
            rules, 1,
            "a resting modal option painted {rules} rules; the specimen shows one, \
             the hairline closing the row, and no leader between name and mark"
        );
    }

    /// The control carries no focus trap and no modal lifecycle: the only thing
    /// it can report is that an entry was committed.
    #[test]
    fn a_frame_with_no_input_asks_for_nothing() {
        let painted = paint(
            CONTROL,
            &ranged_control(),
            ComponentState::Resting,
            ROLE,
            ViewportDensityPolicy::Desktop,
        );
        assert_eq!(painted.intent, ControlIntent::None);
    }
}
