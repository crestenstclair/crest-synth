//! Action hints.
//!
//! The footer shows only the actions valid at the focused target, in the
//! design's compact form: `D-PAD NAV · A CONFIRM · B BACK`. Hints are how a
//! player learns the grammar of the interface, so the separator, the order,
//! and the tones are settled here once rather than at each footer.
//!
//! This primitive never decides which actions are valid. Validity is
//! reducer-owned and arrives through the view model as an immutable slice.

use eframe::egui::text::LayoutJob;
use eframe::egui::{Response, Ui};

use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};

/// What separates two hints on the line.
pub const HINT_SEPARATOR: &str = " · ";

/// The tone a hint reads in.
///
/// Four, as the design file's CLI Hint component set declares.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HintTone {
    /// An ordinary available action.
    Neutral,
    /// An action that moves or confirms focus.
    Focus,
    /// An action that adjusts a value.
    Adjust,
    /// An action that leaves or cancels.
    Back,
}

/// Every declared tone, in declaration order.
pub const ALL_HINT_TONES: [HintTone; 4] = [
    HintTone::Neutral,
    HintTone::Focus,
    HintTone::Adjust,
    HintTone::Back,
];

impl HintTone {
    /// The color role this tone paints in.
    pub const fn color(self) -> SemanticColor {
        match self {
            Self::Neutral => SemanticColor::TextMuted,
            Self::Focus => SemanticColor::AccentFocus,
            Self::Adjust => SemanticColor::AccentAdjust,
            Self::Back => SemanticColor::TextSecondary,
        }
    }

    /// The canonical name of this tone.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Focus => "focus",
            Self::Adjust => "adjust",
            Self::Back => "back",
        }
    }
}

/// One valid action, as it reads in the footer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionHint<'a> {
    /// The control that performs it, e.g. `D-PAD` or `A`.
    pub chord: &'a str,
    /// What it does, e.g. `NAV` or `CONFIRM`.
    pub action: &'a str,
    /// How it reads.
    pub tone: HintTone,
}

impl<'a> ActionHint<'a> {
    /// Builds a hint.
    pub const fn new(chord: &'a str, action: &'a str, tone: HintTone) -> Self {
        Self {
            chord,
            action,
            tone,
        }
    }

    /// The rendered text of this hint, e.g. `D-PAD NAV`.
    pub fn text(&self) -> String {
        format!("{} {}", self.chord, self.action)
    }
}

/// Composes the hint line: one section per hint in its own tone, separators in
/// between.
///
/// The separator is neutral regardless of the tones it sits between, so a line
/// of mixed tones still reads as one line.
pub fn hint_line(hints: &[ActionHint<'_>], state: ComponentState) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (index, hint) in hints.iter().enumerate() {
        if index > 0 {
            job.append(
                HINT_SEPARATOR,
                0.0,
                super::text::text_format(
                    TypeStyle::InstructionHint,
                    HintTone::Neutral.color(),
                    state,
                ),
            );
        }
        job.append(
            &hint.text(),
            0.0,
            super::text::text_format(TypeStyle::InstructionHint, hint.tone.color(), state),
        );
    }
    job
}

/// Paints the hint line into `ui`'s layout.
pub fn paint_action_hints(
    ui: &mut Ui,
    hints: &[ActionHint<'_>],
    state: ComponentState,
) -> Response {
    ui.label(hint_line(hints, state))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn design_file_example() -> [ActionHint<'static>; 3] {
        [
            ActionHint::new("D-PAD", "NAV", HintTone::Focus),
            ActionHint::new("A", "CONFIRM", HintTone::Neutral),
            ActionHint::new("B", "BACK", HintTone::Back),
        ]
    }

    #[test]
    fn the_line_reads_as_the_design_file_writes_it() {
        let job = hint_line(&design_file_example(), ComponentState::Resting);
        assert_eq!(job.text, "D-PAD NAV · A CONFIRM · B BACK");
    }

    #[test]
    fn separators_sit_between_hints_and_never_at_the_ends() {
        let single = hint_line(
            &[ActionHint::new("A", "CONFIRM", HintTone::Neutral)],
            ComponentState::Resting,
        );
        assert_eq!(single.text, "A CONFIRM");
        assert!(!single.text.starts_with(HINT_SEPARATOR));
        assert!(!single.text.ends_with(HINT_SEPARATOR));

        let three = hint_line(&design_file_example(), ComponentState::Resting);
        assert_eq!(three.text.matches(HINT_SEPARATOR).count(), 2);
    }

    #[test]
    fn an_empty_hint_list_paints_an_empty_line() {
        let job = hint_line(&[], ComponentState::Resting);
        assert!(job.text.is_empty());
        assert!(job.sections.is_empty());
    }

    #[test]
    fn each_hint_keeps_its_own_tone_and_the_separator_stays_neutral() {
        let job = hint_line(&design_file_example(), ComponentState::Resting);
        // hint, separator, hint, separator, hint
        assert_eq!(job.sections.len(), 5);
        let colors: Vec<_> = job
            .sections
            .iter()
            .map(|section| section.format.color)
            .collect();
        assert_eq!(
            colors,
            vec![
                HintTone::Focus.color().resolve(),
                HintTone::Neutral.color().resolve(),
                HintTone::Neutral.color().resolve(),
                HintTone::Neutral.color().resolve(),
                HintTone::Back.color().resolve(),
            ]
        );
    }

    #[test]
    fn the_four_declared_tones_are_visually_distinct() {
        assert_eq!(ALL_HINT_TONES.len(), 4);
        let colors: std::collections::HashSet<_> =
            ALL_HINT_TONES.iter().map(|tone| tone.color()).collect();
        assert_eq!(colors.len(), 4, "two tones share a color role");
        let names: std::collections::HashSet<_> = ALL_HINT_TONES
            .iter()
            .map(|tone| tone.canonical_name())
            .collect();
        assert_eq!(names.len(), 4);
    }

    #[test]
    fn hints_are_set_in_the_authored_instruction_style() {
        let job = hint_line(&design_file_example(), ComponentState::Resting);
        let metrics = TypeStyle::InstructionHint.metrics();
        for section in &job.sections {
            assert_eq!(section.format.font_id.size, metrics.size_px);
            assert_eq!(section.format.extra_letter_spacing, metrics.tracking_px);
            assert_eq!(section.format.line_height, Some(metrics.line_height_px));
        }
    }

    #[test]
    fn a_disabled_hint_line_mutes_every_tone() {
        // The state treatment applies to hints too: a footer shown while its
        // target is disabled must not read as four live actions.
        let job = hint_line(&design_file_example(), ComponentState::Disabled);
        for section in &job.sections {
            assert_eq!(section.format.color, SemanticColor::TextMuted.resolve());
        }
    }
}
