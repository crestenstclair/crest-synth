//! Hairlines and keylines.
//!
//! Separation in this design is structural: hairlines and keylines, never
//! cards (`DESIGN.md:447`). A hairline is the resting separator in
//! `color/border/default`; a keyline is the stronger structural division in
//! `color/border/strong`.
//!
//! Both are one authored pixel wide. The vocabulary declares exactly two
//! widths — [`KEYLINE_RESTING_PX`] for rest and [`KEYLINE_EMPHASIS_PX`] for
//! focus and adjustment — so a structural separator drawn at the emphasis
//! width would read as a focused element. Strength here is carried by color,
//! which is what `border/strong` is for.
//!
//! Neither computes its own layout. A caller supplies the span; where a
//! separator belongs is a composition decision that lives in a view.

use eframe::egui::{pos2, Painter, Rect};

use crate::shell::visual::token::{SemanticColor, KEYLINE_RESTING_PX};

/// Where a separator runs.
///
/// The coordinate the rule sits on is its center line: a horizontal span at
/// `y_px` paints half its width above and half below, so a rule placed at a
/// row's vertical middle is centered on it rather than hanging off one side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RuleSpan {
    /// A rule running left to right, centered on `y_px`.
    Horizontal {
        /// The center line of the rule.
        y_px: f32,
        /// Where the rule starts.
        from_x_px: f32,
        /// Where the rule ends.
        to_x_px: f32,
    },
    /// A rule running top to bottom, centered on `x_px`.
    Vertical {
        /// The center line of the rule.
        x_px: f32,
        /// Where the rule starts.
        from_y_px: f32,
        /// Where the rule ends.
        to_y_px: f32,
    },
}

impl RuleSpan {
    /// The rect this span occupies at the authored resting width.
    pub fn rect(self) -> Rect {
        let half = KEYLINE_RESTING_PX / 2.0;
        match self {
            Self::Horizontal {
                y_px,
                from_x_px,
                to_x_px,
            } => Rect::from_min_max(pos2(from_x_px, y_px - half), pos2(to_x_px, y_px + half)),
            Self::Vertical {
                x_px,
                from_y_px,
                to_y_px,
            } => Rect::from_min_max(pos2(x_px - half, from_y_px), pos2(x_px + half, to_y_px)),
        }
    }
}

/// Paints a resting separator in `color/border/default`.
pub fn hairline(painter: &Painter, span: RuleSpan) {
    paint_rule(painter, span, SemanticColor::BorderDefault);
}

/// Paints a structural separator in `color/border/strong`.
pub fn keyline(painter: &Painter, span: RuleSpan) {
    paint_rule(painter, span, SemanticColor::BorderStrong);
}

fn paint_rule(painter: &Painter, span: RuleSpan, color: SemanticColor) {
    painter.rect_filled(span.rect(), 0.0, color.resolve());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::visual::token::KEYLINE_EMPHASIS_PX;

    #[test]
    fn a_horizontal_rule_is_one_authored_pixel_centered_on_its_line() {
        let rect = RuleSpan::Horizontal {
            y_px: 100.0,
            from_x_px: 24.0,
            to_x_px: 924.0,
        }
        .rect();
        assert_eq!(rect.height(), KEYLINE_RESTING_PX);
        assert_eq!(rect.width(), 900.0);
        assert_eq!(rect.center().y, 100.0);
        assert_eq!(rect.min.x, 24.0);
        assert_eq!(rect.max.x, 924.0);
    }

    #[test]
    fn a_vertical_rule_is_one_authored_pixel_centered_on_its_line() {
        let rect = RuleSpan::Vertical {
            x_px: 1_500.0,
            from_y_px: 120.0,
            to_y_px: 1_016.0,
        }
        .rect();
        assert_eq!(rect.width(), KEYLINE_RESTING_PX);
        assert_eq!(rect.height(), 896.0);
        assert_eq!(rect.center().x, 1_500.0);
    }

    #[test]
    fn separators_use_the_resting_width_and_never_the_emphasis_width() {
        // A structural separator at the emphasis width would be
        // indistinguishable from a focused element.
        let horizontal = RuleSpan::Horizontal {
            y_px: 0.0,
            from_x_px: 0.0,
            to_x_px: 10.0,
        };
        let vertical = RuleSpan::Vertical {
            x_px: 0.0,
            from_y_px: 0.0,
            to_y_px: 10.0,
        };
        assert_eq!(horizontal.rect().height(), KEYLINE_RESTING_PX);
        assert_eq!(vertical.rect().width(), KEYLINE_RESTING_PX);
        assert_ne!(KEYLINE_RESTING_PX, KEYLINE_EMPHASIS_PX);
    }

    #[test]
    fn a_span_carries_no_layout_of_its_own() {
        // The rect is exactly the span it was handed, extended only by the
        // authored width. Nothing is inset, padded, or snapped.
        let rect = RuleSpan::Horizontal {
            y_px: 7.5,
            from_x_px: -3.0,
            to_x_px: 3.0,
        }
        .rect();
        assert_eq!(rect.min.x, -3.0);
        assert_eq!(rect.max.x, 3.0);
        assert_eq!(rect.min.y, 7.0);
        assert_eq!(rect.max.y, 8.0);
    }
}
