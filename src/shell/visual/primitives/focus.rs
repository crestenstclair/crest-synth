//! The focus frame, the authored halo, and the cursor column.
//!
//! Focus is the primary interaction state in a controller-first product, so it
//! is the one appearance that has to be exactly right. A focused component
//! draws a 3 px keyline in `color/accent/focus` plus the authored halo; an
//! adjusting one draws 3 px in `color/accent/adjust`.
//!
//! The frame paints only for states that declare the emphasis width. A state
//! declaring the resting width is not framed — that 1 px is the row's own
//! hairline and belongs to [`super::rules`], so `Resting` is an absence rather
//! than a suppressed frame.
//!
//! Color alone never announces focus. The design's `>` cursor occupies a
//! reserved column to the left of the label, and the column is reserved
//! whether or not the cursor is drawn, so focusing a row never shifts its
//! text (`DESIGN.md:575`).

use eframe::egui::epaint::Shadow;
use eframe::egui::{pos2, Align2, Color32, CornerRadius, Painter, Rect, Stroke, StrokeKind};

use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{
    Radius, SemanticColor, TypeStyle, FOCUS_HALO_OPACITY, FOCUS_HALO_RADIUS_PX,
    FOCUS_HALO_SPREAD_PX,
};

/// The glyph the design file places before a focused row's label.
pub const CURSOR_GLYPH: &str = ">";

/// Where the reserved cursor column starts, measured from the design file.
pub const CURSOR_COLUMN_X_PX: f32 = 10.0;

/// How wide the reserved cursor column is, measured from the design file.
pub const CURSOR_COLUMN_WIDTH_PX: f32 = 9.0;

/// Where a row's label starts, after the reserved cursor column.
pub const LABEL_START_X_PX: f32 = 19.0;

/// Whether this state's declared keyline paints as a frame.
///
/// True exactly for the states that declare [`KEYLINE_EMPHASIS_PX`]; the test
/// in this module holds that correspondence rather than letting the two drift.
pub const fn frames(state: ComponentState) -> bool {
    match state {
        ComponentState::Focused
        | ComponentState::Adjusting
        | ComponentState::Loading
        | ComponentState::Error => true,
        ComponentState::Resting
        | ComponentState::Disabled
        | ComponentState::Muted
        | ComponentState::Soloed
        | ComponentState::Selected => false,
    }
}

/// Whether this state draws the `>` cursor in the reserved column.
///
/// Adjustment is focus that is being edited, so it keeps the cursor: losing it
/// mid-edit would read as losing the row.
pub const fn draws_cursor(state: ComponentState) -> bool {
    match state {
        ComponentState::Focused | ComponentState::Adjusting => true,
        ComponentState::Resting
        | ComponentState::Disabled
        | ComponentState::Loading
        | ComponentState::Error
        | ComponentState::Muted
        | ComponentState::Soloed
        | ComponentState::Selected => false,
    }
}

/// The reserved cursor column inside a row.
///
/// Reserved for every state. The column's presence is what keeps a row's text
/// still when it gains focus.
pub fn cursor_column(row: Rect) -> Rect {
    Rect::from_min_max(
        pos2(row.min.x + CURSOR_COLUMN_X_PX, row.min.y),
        pos2(
            row.min.x + CURSOR_COLUMN_X_PX + CURSOR_COLUMN_WIDTH_PX,
            row.max.y,
        ),
    )
}

/// The authored halo color: the state's accent at [`FOCUS_HALO_OPACITY`].
///
/// The alpha is unmultiplied so the opacity is the authored one rather than a
/// tint of it — 0.28 × 255 rounds to 71, the alpha the design file carries.
pub fn halo_color(accent: SemanticColor) -> Color32 {
    let opaque = accent.resolve();
    Color32::from_rgba_unmultiplied(opaque.r(), opaque.g(), opaque.b(), halo_alpha())
}

/// The authored halo, as an `epaint` shadow.
///
/// egui has no drop-shadow primitive matching the design file's blur/spread/
/// opacity triple directly. [`Shadow`] is the faithful approximation: its
/// `blur` is the penumbra width, which is the role the design file's radius 8
/// plays, and its `spread` expands the caster before blurring, which is what
/// spread 1 means there. Both are `u8`, and both authored values are whole
/// numbers, so nothing is lost in the conversion. The halo is not omitted and
/// not faked with a second stroke: a wider stroke would read as a thicker
/// keyline rather than as a glow.
pub fn halo(accent: SemanticColor) -> Shadow {
    Shadow {
        offset: [0, 0],
        blur: FOCUS_HALO_RADIUS_PX as u8,
        spread: FOCUS_HALO_SPREAD_PX as u8,
        color: halo_color(accent),
    }
}

/// Paints the frame and halo `state` declares around `rect`.
///
/// A state that declares no frame paints nothing at all, so calling this for
/// every row is correct and cheap.
pub fn focus_frame(painter: &Painter, rect: Rect, state: ComponentState) {
    if !frames(state) {
        return;
    }
    let appearance = state.appearance();
    let corner = CornerRadius::same(Radius::Small.resolve() as u8);
    if appearance.draws_halo {
        painter.add(halo(appearance.accent).as_shape(rect, corner));
    }
    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(appearance.keyline_px, appearance.accent.resolve()),
        StrokeKind::Inside,
    );
}

/// Paints the `>` cursor into the reserved column of `row`, when `state` draws
/// one.
///
/// The column is reserved either way; this only fills it.
pub fn cursor(painter: &Painter, row: Rect, state: ComponentState) {
    if !draws_cursor(state) {
        return;
    }
    let column = cursor_column(row);
    let galley = super::text::layout(
        painter,
        CURSOR_GLYPH,
        TypeStyle::LabelControl,
        state.appearance().accent,
        state,
    );
    let position = Align2::LEFT_CENTER.anchor_size(column.left_center(), galley.size());
    painter.galley(position.min, galley, state.appearance().accent.resolve());
}

/// The authored halo alpha as a byte.
fn halo_alpha() -> u8 {
    (FOCUS_HALO_OPACITY * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::visual::state::ALL_COMPONENT_STATES;
    use crate::shell::visual::token::{KEYLINE_EMPHASIS_PX, KEYLINE_RESTING_PX};

    fn row() -> Rect {
        Rect::from_min_max(pos2(24.0, 120.0), pos2(1_476.0, 172.0))
    }

    #[test]
    fn a_state_is_framed_exactly_when_it_declares_the_emphasis_width() {
        for state in ALL_COMPONENT_STATES {
            assert_eq!(
                frames(state),
                state.appearance().keyline_px == KEYLINE_EMPHASIS_PX,
                "{} frames disagrees with its declared keyline width",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn resting_is_an_absence_rather_than_a_suppressed_frame() {
        assert!(!frames(ComponentState::Resting));
        assert_eq!(
            ComponentState::Resting.appearance().keyline_px,
            KEYLINE_RESTING_PX
        );
    }

    #[test]
    fn focus_and_adjustment_frame_at_three_pixels_in_their_authored_accents() {
        let focused = ComponentState::Focused.appearance();
        assert!(frames(ComponentState::Focused));
        assert_eq!(focused.keyline_px, KEYLINE_EMPHASIS_PX);
        assert_eq!(focused.accent, SemanticColor::AccentFocus);

        let adjusting = ComponentState::Adjusting.appearance();
        assert!(frames(ComponentState::Adjusting));
        assert_eq!(adjusting.keyline_px, KEYLINE_EMPHASIS_PX);
        assert_eq!(adjusting.accent, SemanticColor::AccentAdjust);
    }

    #[test]
    fn the_halo_carries_the_authored_radius_spread_and_opacity() {
        let halo = halo(SemanticColor::AccentFocus);
        assert_eq!(f32::from(halo.blur), FOCUS_HALO_RADIUS_PX);
        assert_eq!(f32::from(halo.spread), FOCUS_HALO_SPREAD_PX);
        assert_eq!(halo.offset, [0, 0]);
        // 0.28 × 255 = 71.4, the 0x47 alpha the design file carries.
        assert_eq!(halo.color.a(), 71);
        // epaint stores premultiplied channels; unmultiplying recovers the
        // authored accent, so the halo is that color at that opacity and not a
        // darkened approximation of it. Premultiplying into 8 bits and back
        // costs at most one level per channel — the green here round-trips
        // 229 → 230 — so the tolerance is that rounding and nothing wider.
        let opaque = SemanticColor::AccentFocus.resolve();
        let [r, g, b, a] = halo.color.to_srgba_unmultiplied();
        assert_eq!(a, 71);
        for (recovered, authored) in [(r, opaque.r()), (g, opaque.g()), (b, opaque.b())] {
            assert!(
                recovered.abs_diff(authored) <= 1,
                "the halo is {recovered} where the accent is {authored}"
            );
        }
    }

    #[test]
    fn focus_is_the_only_state_whose_frame_carries_a_halo() {
        for state in ALL_COMPONENT_STATES {
            assert_eq!(
                state.appearance().draws_halo,
                state == ComponentState::Focused,
                "{} halo",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn the_cursor_column_is_reserved_for_every_state() {
        let expected = Rect::from_min_max(pos2(34.0, 120.0), pos2(43.0, 172.0));
        for state in ALL_COMPONENT_STATES {
            assert_eq!(
                cursor_column(row()),
                expected,
                "{} shifts the cursor column",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn the_label_starts_where_the_reserved_column_ends() {
        assert_eq!(
            CURSOR_COLUMN_X_PX + CURSOR_COLUMN_WIDTH_PX,
            LABEL_START_X_PX,
            "the cursor column and the label overlap or leave a gap"
        );
        assert_eq!(cursor_column(row()).max.x - row().min.x, LABEL_START_X_PX);
    }

    #[test]
    fn focus_is_legible_without_color() {
        // The cursor is the non-color signal. Without it, focus would be a
        // cyan keyline and nothing else.
        assert!(draws_cursor(ComponentState::Focused));
        assert!(draws_cursor(ComponentState::Adjusting));
        assert_eq!(CURSOR_GLYPH, ">");
    }

    #[test]
    fn no_unfocused_state_draws_the_cursor() {
        for state in ALL_COMPONENT_STATES {
            if state == ComponentState::Focused || state == ComponentState::Adjusting {
                continue;
            }
            assert!(
                !draws_cursor(state),
                "{} draws the focus cursor",
                state.canonical_name()
            );
        }
    }
}
