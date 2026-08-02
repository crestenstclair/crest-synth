//! The value display.
//!
//! Values are set in `Code/Value` and right-aligned to a content edge so they
//! read as a column. That alignment is load-bearing in a controller UI: the
//! player scans down the values, not across the rows.
//!
//! This primitive does not format. A number becomes a string in the capability
//! that owns the parameter, where its unit, precision, and bounds are declared;
//! by the time it arrives here it is finished text. Formatting it a second time
//! is how two surfaces come to disagree about the same value.
//!
//! The right edge is a parameter. The design file right-aligns to x=1442 inside
//! a 1452-wide desktop content area, but that number belongs to a layout, not
//! to this primitive.

use eframe::egui::{pos2, Painter};

use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};

/// The color role a value is requested in for a given state.
///
/// [`super::text::resolved_color`] applies the state treatment on top, so an
/// erroring value still warns without this having to say so twice.
pub const fn value_color(state: ComponentState) -> SemanticColor {
    match state {
        ComponentState::Adjusting => SemanticColor::AccentAdjust,
        ComponentState::Disabled => SemanticColor::TextMuted,
        ComponentState::Resting
        | ComponentState::Focused
        | ComponentState::Loading
        | ComponentState::Error
        | ComponentState::Muted
        | ComponentState::Soloed
        | ComponentState::Selected => SemanticColor::TextPrimary,
    }
}

/// Paints a finished value string right-aligned to `right_edge_x_px` and
/// vertically centered on `center_y_px`.
pub fn paint_value(
    painter: &Painter,
    right_edge_x_px: f32,
    center_y_px: f32,
    value: &str,
    state: ComponentState,
) {
    let color = value_color(state);
    let galley = super::text::layout(painter, value, TypeStyle::CodeValue, color, state);
    let size = galley.size();
    let position = pos2(right_edge_x_px - size.x, center_y_px - size.y / 2.0);
    painter.galley(
        position,
        galley,
        super::text::resolved_color(color, state).resolve(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::visual::state::ALL_COMPONENT_STATES;

    /// The color a value actually paints in, after the text primitive applies
    /// the state treatment.
    fn painted_color(state: ComponentState) -> SemanticColor {
        super::super::text::resolved_color(value_color(state), state)
    }

    #[test]
    fn adjustment_paints_the_adjust_accent() {
        assert_eq!(
            painted_color(ComponentState::Adjusting),
            SemanticColor::AccentAdjust
        );
    }

    #[test]
    fn a_disabled_value_is_muted() {
        assert_eq!(
            painted_color(ComponentState::Disabled),
            SemanticColor::TextMuted
        );
    }

    #[test]
    fn every_other_state_paints_primary_text_except_error_which_warns() {
        for state in ALL_COMPONENT_STATES {
            let expected = match state {
                ComponentState::Adjusting => SemanticColor::AccentAdjust,
                ComponentState::Disabled => SemanticColor::TextMuted,
                ComponentState::Error => SemanticColor::AccentWarning,
                ComponentState::Resting
                | ComponentState::Focused
                | ComponentState::Loading
                | ComponentState::Muted
                | ComponentState::Soloed
                | ComponentState::Selected => SemanticColor::TextPrimary,
            };
            assert_eq!(
                painted_color(state),
                expected,
                "{} value color",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn a_value_is_set_in_the_authored_code_style() {
        let metrics = TypeStyle::CodeValue.metrics();
        let format = super::super::text::text_format(
            TypeStyle::CodeValue,
            value_color(ComponentState::Resting),
            ComponentState::Resting,
        );
        assert_eq!(format.font_id.size, metrics.size_px);
        assert_eq!(format.extra_letter_spacing, metrics.tracking_px);
        assert_eq!(format.line_height, Some(metrics.line_height_px));
    }
}
