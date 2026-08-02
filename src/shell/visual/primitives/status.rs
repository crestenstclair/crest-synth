//! Status marks.
//!
//! This is the primitive most likely to be built color-only, so it is built the
//! other way round: the mark is resolved from the [`NonColorSignal`] WP02
//! declared on the state, and the accent comes along with it. No state's status
//! is invented here and no second visual language is introduced.
//!
//! Every state that carries a status carries text or shape:
//!
//! | State      | Mark                                    |
//! |------------|-----------------------------------------|
//! | `Disabled` | `Locked`                                |
//! | `Loading`  | `Preparing` / `Activating`              |
//! | `Error`    | short typed failure text                |
//! | `Muted`    | `M ON` (`DESIGN.md:468`)                |
//! | `Soloed`   | `S ON` (`DESIGN.md:468`)                |
//! | `Selected` | `bg/selected` fill plus a filled mark   |
//!
//! `Resting`, `Focused`, and `Adjusting` carry no status: rest is the baseline,
//! and focus and adjustment are carried by the frame and the cursor.
//!
//! `Disabled` departs from the WP03 prompt's note, which grouped it with the
//! unmarked states. WP02 declared its signal as the word `Locked`, and FR-005
//! names disabled among the states that must read without color, so the word is
//! painted. Dropping it would have left disabled as muted color and nothing
//! else.

use eframe::egui::{pos2, Align2, Painter, Rect};

use crate::shell::visual::state::{ComponentState, NonColorSignal, LOADING_PROGRESS_WORDS};
use crate::shell::visual::token::{SemanticColor, SpacingStep, TypeStyle};

/// The word painted when an erroring component supplies no failure text.
///
/// A caller should supply typed text. This exists so that the absence of it
/// still reads without color, which is the whole point of the primitive.
pub const GENERIC_FAILURE_WORD: &str = "Failed";

/// Which phase of a structural edit a loading component is reporting.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LoadingPhase {
    /// The edit has been prepared but is not yet active.
    Preparing,
    /// The prepared edit is being made active.
    Activating,
}

impl LoadingPhase {
    /// The authored word for this phase.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Preparing => LOADING_PROGRESS_WORDS[0],
            Self::Activating => LOADING_PROGRESS_WORDS[1],
        }
    }
}

/// The caller-varying text behind a status mark.
///
/// Only two states need it: the phase a loading component reports and the
/// typed failure an erroring one does. Every other state ignores it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusDetail<'a> {
    /// This component's mark is a fixed authored word or a shape.
    None,
    /// The structural-edit phase a `Loading` component reports.
    Progress(LoadingPhase),
    /// The short typed failure an `Error` component reports.
    Failure(&'a str),
}

impl<'a> StatusDetail<'a> {
    /// The progress word a loading component shows.
    ///
    /// Missing detail falls back to the first authored phase rather than to
    /// nothing: a loading component showing only color is the failure this
    /// primitive exists to prevent.
    const fn progress_word(self) -> &'a str {
        match self {
            Self::Progress(phase) => phase.word(),
            Self::None | Self::Failure(_) => LoadingPhase::Preparing.word(),
        }
    }

    /// The failure text an erroring component shows.
    ///
    /// Missing or empty detail falls back to [`GENERIC_FAILURE_WORD`], for the
    /// same reason.
    const fn failure_text(self) -> &'a str {
        match self {
            Self::Failure(text) if !text.is_empty() => text,
            Self::Failure(_) | Self::None | Self::Progress(_) => GENERIC_FAILURE_WORD,
        }
    }
}

/// The mark a state puts on screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusMark<'a> {
    /// Text painted in the state's declared accent.
    Text {
        /// What the mark says.
        text: &'a str,
        /// The role it paints in.
        color: SemanticColor,
    },
    /// A filled row plus a filled mark, for the state that fills its row.
    Selection {
        /// The row fill role.
        fill: SemanticColor,
        /// The mark role, which is not the fill: a mark painted in the fill
        /// color would be invisible on it, leaving selection color-only.
        mark: SemanticColor,
    },
}

/// Resolves the mark `state` carries, or `None` when it carries no status.
pub fn status_mark<'a>(state: ComponentState, detail: StatusDetail<'a>) -> Option<StatusMark<'a>> {
    let appearance = state.appearance();
    match state {
        ComponentState::Resting | ComponentState::Focused | ComponentState::Adjusting => None,
        ComponentState::Disabled | ComponentState::Muted | ComponentState::Soloed => {
            Some(StatusMark::Text {
                text: declared_word(appearance.signal),
                color: appearance.accent,
            })
        }
        ComponentState::Loading => Some(StatusMark::Text {
            text: detail.progress_word(),
            color: appearance.accent,
        }),
        ComponentState::Error => Some(StatusMark::Text {
            text: detail.failure_text(),
            color: appearance.accent,
        }),
        ComponentState::Selected => Some(StatusMark::Selection {
            fill: appearance.accent,
            mark: SemanticColor::TextPrimary,
        }),
    }
}

/// Paints the row fill `state` declares, if it declares one.
///
/// Call before the row's content; the fill sits behind it.
pub fn paint_row_fill(painter: &Painter, row: Rect, state: ComponentState) {
    let appearance = state.appearance();
    if appearance.fills_row {
        painter.rect_filled(row, 0.0, appearance.accent.resolve());
    }
}

/// Paints `state`'s status mark, left-aligned and vertically centered in
/// `mark_rect`.
///
/// A state carrying no status paints nothing. Where the mark column sits is a
/// composition decision; this primitive computes no layout of its own.
pub fn paint_status_mark(
    painter: &Painter,
    mark_rect: Rect,
    state: ComponentState,
    detail: StatusDetail<'_>,
) {
    let Some(mark) = status_mark(state, detail) else {
        return;
    };
    match mark {
        StatusMark::Text { text, color } => {
            let galley = super::text::layout(painter, text, TypeStyle::LabelControl, color, state);
            let placed = Align2::LEFT_CENTER.anchor_size(mark_rect.left_center(), galley.size());
            let fallback = super::text::resolved_color(color, state).resolve();
            painter.galley(placed.min, galley, fallback);
        }
        StatusMark::Selection { fill: _, mark } => {
            let side = SpacingStep::S8.resolve();
            let left = mark_rect.min.x + SpacingStep::S4.resolve();
            let top = mark_rect.center().y - side / 2.0;
            painter.rect_filled(
                Rect::from_min_max(pos2(left, top), pos2(left + side, top + side)),
                0.0,
                mark.resolve(),
            );
        }
    }
}

/// The fixed authored word a state's declared signal carries.
///
/// A state routed here declares [`NonColorSignal::Word`]. The empty string is
/// unreachable; the test in this module holds that every state routed here
/// declares a word, so this stays total without a panic in a paint path.
const fn declared_word(signal: NonColorSignal) -> &'static str {
    match signal {
        NonColorSignal::Word(word) => word,
        NonColorSignal::Shape | NonColorSignal::ProgressWord | NonColorSignal::TypedFailure => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::visual::state::ALL_COMPONENT_STATES;

    #[test]
    fn every_state_that_carries_a_status_says_something_that_is_not_a_color() {
        for state in ALL_COMPONENT_STATES {
            let Some(mark) = status_mark(state, StatusDetail::None) else {
                continue;
            };
            match mark {
                StatusMark::Text { text, .. } => assert!(
                    !text.is_empty(),
                    "{} carries an empty status mark",
                    state.canonical_name()
                ),
                StatusMark::Selection { fill, mark } => assert_ne!(
                    fill,
                    mark,
                    "{} paints its mark in its fill, leaving it invisible",
                    state.canonical_name()
                ),
            }
        }
    }

    #[test]
    fn rest_focus_and_adjustment_carry_no_status_mark() {
        for state in [
            ComponentState::Resting,
            ComponentState::Focused,
            ComponentState::Adjusting,
        ] {
            assert!(
                status_mark(state, StatusDetail::None).is_none(),
                "{} invented a status mark",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn the_five_status_bearing_states_and_disabled_all_mark() {
        for state in [
            ComponentState::Disabled,
            ComponentState::Loading,
            ComponentState::Error,
            ComponentState::Muted,
            ComponentState::Soloed,
            ComponentState::Selected,
        ] {
            assert!(
                status_mark(state, StatusDetail::None).is_some(),
                "{} carries no status mark",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn mute_and_solo_carry_the_authored_words_in_the_authored_accents() {
        assert_eq!(
            status_mark(ComponentState::Muted, StatusDetail::None),
            Some(StatusMark::Text {
                text: "M ON",
                color: SemanticColor::AccentWarning
            })
        );
        assert_eq!(
            status_mark(ComponentState::Soloed, StatusDetail::None),
            Some(StatusMark::Text {
                text: "S ON",
                color: SemanticColor::AccentPositive
            })
        );
    }

    #[test]
    fn a_disabled_component_says_it_is_locked() {
        assert_eq!(
            status_mark(ComponentState::Disabled, StatusDetail::None),
            Some(StatusMark::Text {
                text: "Locked",
                color: SemanticColor::TextMuted
            })
        );
    }

    #[test]
    fn loading_reports_the_phase_the_caller_supplied() {
        assert_eq!(
            status_mark(
                ComponentState::Loading,
                StatusDetail::Progress(LoadingPhase::Activating)
            ),
            Some(StatusMark::Text {
                text: "Activating",
                color: SemanticColor::AccentAdjust
            })
        );
        assert_eq!(LoadingPhase::Preparing.word(), "Preparing");
        assert_eq!(LoadingPhase::Activating.word(), "Activating");
    }

    #[test]
    fn loading_without_a_phase_still_says_a_word() {
        assert_eq!(
            status_mark(ComponentState::Loading, StatusDetail::None),
            Some(StatusMark::Text {
                text: "Preparing",
                color: SemanticColor::AccentAdjust
            })
        );
    }

    #[test]
    fn error_reports_the_typed_failure_the_caller_supplied() {
        assert_eq!(
            status_mark(
                ComponentState::Error,
                StatusDetail::Failure("PRESET MISSING")
            ),
            Some(StatusMark::Text {
                text: "PRESET MISSING",
                color: SemanticColor::AccentWarning
            })
        );
    }

    #[test]
    fn error_without_typed_text_still_says_a_word() {
        for detail in [StatusDetail::None, StatusDetail::Failure("")] {
            assert_eq!(
                status_mark(ComponentState::Error, detail),
                Some(StatusMark::Text {
                    text: GENERIC_FAILURE_WORD,
                    color: SemanticColor::AccentWarning
                })
            );
        }
    }

    #[test]
    fn selection_is_a_fill_plus_a_mark_and_never_the_fill_alone() {
        assert_eq!(
            status_mark(ComponentState::Selected, StatusDetail::None),
            Some(StatusMark::Selection {
                fill: SemanticColor::BgSelected,
                mark: SemanticColor::TextPrimary
            })
        );
    }

    #[test]
    fn selection_is_the_only_state_that_fills_its_row() {
        for state in ALL_COMPONENT_STATES {
            assert_eq!(
                state.appearance().fills_row,
                state == ComponentState::Selected,
                "{} row fill",
                state.canonical_name()
            );
        }
    }

    /// The `declared_word` fallback is unreachable in production. This is what
    /// holds that: every state routed to it declares a word.
    #[test]
    fn every_state_routed_to_the_declared_word_declares_one() {
        for state in [
            ComponentState::Disabled,
            ComponentState::Muted,
            ComponentState::Soloed,
        ] {
            assert!(
                matches!(state.appearance().signal, NonColorSignal::Word(word) if !word.is_empty()),
                "{} does not declare a fixed word",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn the_mark_reuses_the_accent_the_state_declared() {
        for state in ALL_COMPONENT_STATES {
            let Some(StatusMark::Text { color, .. }) = status_mark(state, StatusDetail::None)
            else {
                continue;
            };
            assert_eq!(
                color,
                state.appearance().accent,
                "{} re-derived its accent instead of reading the declared one",
                state.canonical_name()
            );
        }
    }
}
