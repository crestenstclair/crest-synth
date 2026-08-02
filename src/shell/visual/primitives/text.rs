//! Text roles.
//!
//! Every text run the interface paints resolves through [`text_format`]. Size,
//! weight, family, line height, and tracking come from the authored
//! [`TypeStyle`]; a caller supplies the content, the color role, and the state,
//! and nothing else. A call site that could choose a point size is the failure
//! the vocabulary exists to remove, so no argument here accepts one.
//!
//! The state treatment is applied once, in [`resolved_color`], rather than
//! branching inside each caller: `Disabled` mutes, `Error` warns, and every
//! other state keeps the color it was handed because its keyline, halo, fill,
//! or mark is what announces it.
//!
//! The API was designed against `paint_context_line` in
//! `src/adapter/eframe_graphical_window.rs`, the first surface WP04 converts.
//! That conversion reads:
//!
//! ```text
//! product label  → paint_body(ui, projection.context_line().product_label(), state)
//! context label  → paint_text(ui, label, TypeStyle::HeadingPanel, mode_color, state)
//! status label   → paint_label(ui, projection.context_line().status_label(), state)
//! ```

use std::sync::Arc;

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Align, FontId, Galley, Painter, Response, Ui};

use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};
use crate::shell::visual::typeface::family_for;

/// Resolves the color a run paints in from the role the caller asked for and
/// the state the component was handed.
///
/// `Focused` and `Adjusting` keep their content color deliberately: the
/// keyline and halo carry those states, and recoloring the text as well would
/// make focus and adjustment read as two different kinds of content.
pub const fn resolved_color(requested: SemanticColor, state: ComponentState) -> SemanticColor {
    match state {
        ComponentState::Resting
        | ComponentState::Focused
        | ComponentState::Adjusting
        | ComponentState::Loading
        | ComponentState::Muted
        | ComponentState::Soloed
        | ComponentState::Selected => requested,
        ComponentState::Disabled => SemanticColor::TextMuted,
        ComponentState::Error => SemanticColor::AccentWarning,
    }
}

/// Builds the authored text format for one run.
///
/// This is the single place the vocabulary becomes an `epaint` format. Every
/// other primitive that puts text on screen goes through it, so a style
/// changes in one file and changes everywhere.
pub fn text_format(style: TypeStyle, color: SemanticColor, state: ComponentState) -> TextFormat {
    let metrics = style.metrics();
    TextFormat {
        font_id: FontId::new(metrics.size_px, family_for(style)),
        extra_letter_spacing: metrics.tracking_px,
        line_height: Some(metrics.line_height_px),
        color: resolved_color(color, state).resolve(),
        valign: Align::Center,
        ..TextFormat::default()
    }
}

/// Builds the layout job for one single-style run.
pub fn text_run(
    content: impl Into<String>,
    style: TypeStyle,
    color: SemanticColor,
    state: ComponentState,
) -> LayoutJob {
    LayoutJob::single_section(content.into(), text_format(style, color, state))
}

/// Paints one text run into `ui`'s layout.
pub fn paint_text(
    ui: &mut Ui,
    content: impl Into<String>,
    style: TypeStyle,
    color: SemanticColor,
    state: ComponentState,
) -> Response {
    ui.label(text_run(content, style, color, state))
}

/// Lays out one run for a primitive that places text at an explicit position
/// rather than in a `Ui`'s layout.
pub fn layout(
    painter: &Painter,
    content: impl Into<String>,
    style: TypeStyle,
    color: SemanticColor,
    state: ComponentState,
) -> Arc<Galley> {
    painter.layout_job(text_run(content, style, color, state))
}

/// Paints a control label: `Label/Control` in `text/muted`.
pub fn paint_label(ui: &mut Ui, content: impl Into<String>, state: ComponentState) -> Response {
    paint_text(
        ui,
        content,
        TypeStyle::LabelControl,
        SemanticColor::TextMuted,
        state,
    )
}

/// Paints body content: `Body/Default` in `text/primary`.
pub fn paint_body(ui: &mut Ui, content: impl Into<String>, state: ComponentState) -> Response {
    paint_text(
        ui,
        content,
        TypeStyle::BodyDefault,
        SemanticColor::TextPrimary,
        state,
    )
}

/// Paints a panel heading: `Heading/Panel` in `text/primary`.
pub fn paint_heading(ui: &mut Ui, content: impl Into<String>, state: ComponentState) -> Response {
    paint_text(
        ui,
        content,
        TypeStyle::HeadingPanel,
        SemanticColor::TextPrimary,
        state,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::visual::state::ALL_COMPONENT_STATES;
    use crate::shell::visual::token::ALL_TYPE_STYLES;

    #[test]
    fn every_style_carries_its_authored_metrics_into_the_text_format() {
        for style in ALL_TYPE_STYLES {
            let metrics = style.metrics();
            let format = text_format(style, SemanticColor::TextPrimary, ComponentState::Resting);
            assert_eq!(
                format.font_id.size,
                metrics.size_px,
                "{} size",
                style.canonical_name()
            );
            assert_eq!(
                format.font_id.family,
                family_for(style),
                "{} family",
                style.canonical_name()
            );
            assert_eq!(
                format.extra_letter_spacing,
                metrics.tracking_px,
                "{} tracking",
                style.canonical_name()
            );
            assert_eq!(
                format.line_height,
                Some(metrics.line_height_px),
                "{} line height",
                style.canonical_name()
            );
        }
    }

    #[test]
    fn the_weight_reaches_the_family_rather_than_being_declared_and_dropped() {
        // `Heading/Panel` is the only Bold style; `Body/Default` is Regular.
        // If both resolved to the same family the vocabulary would declare a
        // weight the renderer never applies.
        assert_ne!(
            text_format(
                TypeStyle::HeadingPanel,
                SemanticColor::TextPrimary,
                ComponentState::Resting
            )
            .font_id
            .family,
            text_format(
                TypeStyle::BodyDefault,
                SemanticColor::TextPrimary,
                ComponentState::Resting
            )
            .font_id
            .family
        );
    }

    #[test]
    fn disabled_mutes_and_error_warns_whatever_color_the_caller_asked_for() {
        for requested in [
            SemanticColor::TextPrimary,
            SemanticColor::AccentFocus,
            SemanticColor::AccentPositive,
        ] {
            assert_eq!(
                resolved_color(requested, ComponentState::Disabled),
                SemanticColor::TextMuted
            );
            assert_eq!(
                resolved_color(requested, ComponentState::Error),
                SemanticColor::AccentWarning
            );
        }
    }

    #[test]
    fn focus_and_adjustment_leave_the_content_color_alone() {
        for state in [ComponentState::Focused, ComponentState::Adjusting] {
            assert_eq!(
                resolved_color(SemanticColor::TextPrimary, state),
                SemanticColor::TextPrimary,
                "{} recolored its text",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn every_state_resolves_to_a_declared_color() {
        for state in ALL_COMPONENT_STATES {
            let resolved = resolved_color(SemanticColor::TextSecondary, state);
            assert!(
                crate::shell::visual::token::ALL_COLORS.contains(&resolved),
                "{} resolved outside the vocabulary",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn a_run_carries_its_content_verbatim_in_one_section() {
        let job = text_run(
            "PATCH 01",
            TypeStyle::CodeValue,
            SemanticColor::TextPrimary,
            ComponentState::Resting,
        );
        assert_eq!(job.text, "PATCH 01");
        assert_eq!(job.sections.len(), 1);
        assert_eq!(
            job.sections[0].format.color,
            SemanticColor::TextPrimary.resolve()
        );
    }
}
