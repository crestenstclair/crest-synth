//! The mixer fader.
//!
//! The tall vertical level control of one mixer track column.
//!
//! # Specimen
//!
//! Design file `kdQMw8dYUZtv2UxJPo0sXU`, frame `SCREEN · Mixer · 1920×1080`
//! (`42:3`), the `16 Fader Grid` (`42:25`) and its sixteen `Fader / T00…T0F`
//! instances (`42:26` and siblings), plus the four-variant component specimen
//! exported as `figma-functional-interpretation/assets/mixer-faders.png`.
//!
//! The grid sits at x=24 in the 1500 px main surface and is 1452 px wide — the
//! main width less the authored inset on both sides. Each column is 82 px wide
//! on an 86 px pitch, so the gutter between two columns is the authored
//! `space/4`. Inside a column the track is a 14 px rounded rect, its level fill
//! is 8 px inset 3 px on every side, and the cap above the fill is 34 × 6 px.
//! Level, value, and the mute/solo line are stacked top to bottom, and the
//! columns are separated by hairlines rather than by cards (`DESIGN.md:462`).
//!
//! # Every run in the column is centered on it
//!
//! The component set `34:45 Compact Mixer Fader` lays its column out as
//! `flex-col` inside 82 × 560, and each text layer is a centered box carrying
//! centered text: `Track` at 9,10 64×18, `Value` at 9,428 64×18, and `State` at
//! 5,566 72×18, each inset equally on both sides of an 82 px column. All four
//! variants agree, and both renders — `mixer.png` and `mixer-faders.png` — show
//! `T00`, `7F`, and `M -- · S --` sitting on the same axis as the groove.
//!
//! A column is read down rather than across, so there is no neighbouring column
//! for an edge to line up with. That is what separates a strip from a listed
//! row, where the name pins left and the value pins right so the values read as
//! a column of their own.
//!
//! The two primitives that place that text are edge-anchored for the row case —
//! [`value::paint_value`] from a right edge, [`status::paint_status_mark`] from
//! a left one — and neither is changed here, because a row still needs them that
//! way. Instead the column measures each run and hands the primitive the edge
//! that lands the run centered.
//!
//! The bound variables are `color/bg/elevated` for the track,
//! `color/text/primary` for the cap, and — for the level fill —
//! `color/accent/patch` at rest, `color/accent/focus` when focused,
//! `color/accent/warning` when muted, and `color/accent/positive` when soloed.
//!
//! # What the policy resolves and what the specimen gave up
//!
//! [`ViewportDensityPolicy::mixer_column`] carries the column's width, its
//! pitch, and the floor it may not narrow past, and this control declares no
//! constant of its own: it asks the policy. Desktop returns the specimen's own
//! 82 px column on an 86 px pitch, so the 80 px of slack the design file leaves
//! at the right of the grid stays slack rather than being divided into the
//! columns. The compact viewport has no authored frames at all; there the policy
//! narrows the width and the pitch together, holding the same authored `space/4`
//! gutter, and the arithmetic for both is written where the values are.
//!
//! Track thickness is `space/12` and the fill is `space/8`: the specimen's 14 px
//! track falls between two authored spacing steps and there is no token for it.
//! The 3 px shoulder the specimen shows is reproduced exactly as the difference
//! between the two, halved.
//!
//! # Mute and solo
//!
//! `Muted` and `Soloed` are applicable here because they are facts about the
//! track a column represents. Each paints the authored word its
//! [`ComponentState`] declares — `M ON` and `S ON` (`DESIGN.md:468`) — in the
//! status band, so a muted fader, a soloed fader, and a resting one are three
//! different things on a monochrome screen.
//!
//! A track can be both muted and soloed, and [`ComponentState`] is one value.
//! The component specimen paints **both** facts at once (`M ON · S --`), which
//! this vocabulary cannot express; that is recorded as a finding rather than
//! solved here by adding a state.
//!
//! # No cursor column
//!
//! The `>` cursor and its reserved column belong to a row: they exist so that a
//! row's text does not shift when it gains focus, and a column has no room for
//! them beside a centered name. The specimen agrees — a focused column is a
//! keyline and a fill, not a glyph. Focus is therefore carried here by the
//! shape `Focused` already declares, the authored 3 px keyline plus the halo,
//! which is what separates it from `Adjusting`'s keyline without one.
//!
//! # A defect this control is the wrong place to fix
//!
//! Everything is painted into a painter clipped to the column, so a projected
//! label longer than the column is cut at the column edge rather than bleeding
//! into its neighbour. The clipping is deliberate. What it hides is not.
//!
//! At the compact viewport the projected label `T00 Level` lays out at 79.59 px
//! in a 52 px column and is cut at the column's own edges; at desktop it fits,
//! 79.59 in 82. **This is a real defect, recorded and unowned.** The specimen labels
//! the column `T00` rather than `T00 Level`, so the faithful repair is a shorter
//! projected label, and the label is built in
//! `src/control/semantic_graphical_view_model.rs` — a file **no work package in
//! this mission owns**. WP06 owns only `src/adapter/eframe_graphical_window.rs`,
//! and WP08 owns only tests, so WP08's T044 — no clipped or overlapping text at
//! either viewport — currently has no owner able to make it true.
//!
//! Truncating here instead would satisfy T044 by hiding the same text one layer
//! further down, and would put this control in the business of editing the
//! projection's words. That is why it is recorded rather than done.
//!
//! # Painted, not assembled
//!
//! `research.md` R-02. No `egui::Slider`: its handle, rounding, and hover tint
//! come out of `egui::Style`, a second visual vocabulary the literal guard
//! cannot see.

use eframe::egui::{pos2, Painter, Rect, Response, Sense, Ui, Vec2};

use super::compact_slider::{direction, granularity, status_detail, value_fraction, value_text};
use super::{ControlIntent, PresentationRole};
use crate::control::SemanticControlViewModel;
#[cfg(test)]
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::{focus, status, text, value};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{
    Radius, SemanticColor, SpacingStep, TypeStyle, MIN_INTERACTIVE_TARGET_PX,
};

/// How many track columns a mixer strip carries.
///
/// Read from the canonical count rather than spelled here. The interface and
/// the canonical mixer state have exactly sixteen tracks (`DESIGN.md:462`), and
/// a second copy of that number in the visual vocabulary is a second thing to
/// keep in step.
///
/// Since the column geometry moved onto `ViewportDensityPolicy::mixer_column`,
/// nothing outside this file's own assertions reads it: the policy reads the
/// canonical count itself, and the composition that stacks the columns iterates
/// `MixerTrackId::ALL`. It is kept, and scoped to the assertions that still use
/// it, rather than deleted — deleting it would mean editing those assertions.
#[cfg(test)]
pub(super) const STRIP_COLUMN_COUNT: usize = MixerTrackId::COUNT;

/// The distance between the left edges of two adjacent track columns.
///
/// Asked of the policy, which is the one place mixer-column geometry is
/// declared. A control that divided the main surface for itself would be a
/// second answer to a question the policy already answers, and the two would
/// disagree the moment either changed.
pub(super) fn column_pitch_px(density: &ViewportDensityPolicy) -> f32 {
    density.mixer_column().pitch_px
}

/// The width of one track column, gutter excluded.
///
/// The gutter is the authored `space/4` the design file's grid measures, and
/// the policy holds it: the pitch less the width is that step at both authored
/// viewports. What is painted in it is a hairline, which belongs to the
/// composition that stacks the columns, not to a column.
pub(super) fn column_width_px(density: &ViewportDensityPolicy) -> f32 {
    column_pitch_px(density) - density.mixer_column().gutter_px()
}

/// The height one strip control asks for.
///
/// The composition owns how tall the strip band is, so the column takes the
/// height it is given, bounded above by the authored workspace band — an
/// unbounded layout would otherwise paint an unbounded column — and below by
/// the authored interactive minimum, which is the axis a fader is grabbed on.
pub(super) fn column_height_px(ui: &Ui, density: &ViewportDensityPolicy) -> f32 {
    ui.available_height()
        .min(density.bands().workspace_px)
        .max(MIN_INTERACTIVE_TARGET_PX)
}

/// The color role a track's level fill paints in for a given state.
///
/// `Resting` paints `color/accent/patch`, which is the variable the design file
/// binds to a resting column's fill. Every other state paints the accent its
/// [`ComponentState`] already declares, so the fill never disagrees with the
/// keyline around it or the word beneath it.
pub(super) const fn level_color(state: ComponentState) -> SemanticColor {
    match state {
        ComponentState::Resting => SemanticColor::AccentPatch,
        ComponentState::Focused => SemanticColor::AccentFocus,
        ComponentState::Adjusting | ComponentState::Loading => SemanticColor::AccentAdjust,
        ComponentState::Disabled => SemanticColor::TextMuted,
        ComponentState::Error | ComponentState::Muted => SemanticColor::AccentWarning,
        ComponentState::Soloed => SemanticColor::AccentPositive,
        ComponentState::Selected => SemanticColor::TextPrimary,
    }
}

/// The four vertical bands a strip column is laid out on, top to bottom.
pub(super) struct Bands {
    /// The track's own name.
    pub label: Rect,
    /// Where the level track runs.
    pub track: Rect,
    /// The value readout.
    pub value: Rect,
    /// The state's status mark — where `M ON` and `S ON` are painted.
    pub status: Rect,
}

/// Resolves the bands from the allocated column.
///
/// The label pins to the top and the value and status bands pin to the bottom,
/// each one authored line tall; the track takes what is left. A column too
/// short to hold all four leaves the track with no height, and the paint below
/// skips it rather than drawing an inverted rectangle.
pub(super) fn bands(column: Rect) -> Bands {
    let label_line = TypeStyle::LabelControl.metrics().line_height_px;
    let value_line = TypeStyle::CodeValue.metrics().line_height_px;
    let gap = SpacingStep::S8.resolve();
    let tight = SpacingStep::S4.resolve();
    let label = Rect::from_min_max(column.min, pos2(column.max.x, column.min.y + label_line));
    let status = Rect::from_min_max(pos2(column.min.x, column.max.y - label_line), column.max);
    let value = Rect::from_min_max(
        pos2(column.min.x, status.min.y - tight - value_line),
        pos2(column.max.x, status.min.y - tight),
    );
    let track = Rect::from_min_max(
        pos2(column.min.x, label.max.y + gap),
        pos2(column.max.x, value.min.y - gap),
    );
    Bands {
        label,
        track,
        value,
        status,
    }
}

/// The level track's own rectangle inside its band: a vertical bar of the
/// authored thickness, centered on the column.
pub(super) fn track_bar(band: Rect) -> Rect {
    let thickness = SpacingStep::S12.resolve();
    let center = band.center().x;
    Rect::from_min_max(
        pos2(center - thickness / 2.0, band.min.y),
        pos2(center + thickness / 2.0, band.max.y),
    )
}

/// The span the level fill may occupy inside `bar`.
///
/// Inset by half the difference between the authored track and fill thickness,
/// which reproduces the specimen's shoulder on every side without declaring it.
pub(super) fn fill_span(bar: Rect) -> Rect {
    bar.shrink((SpacingStep::S12.resolve() - SpacingStep::S8.resolve()) / 2.0)
}

/// The width one finished run occupies in the style it is set in.
///
/// Measuring is what lets a centered column reuse the edge-anchored primitives
/// unchanged: the run's own width is the only thing that turns "centered on the
/// column" into the edge each primitive already accepts.
fn run_width_px(
    painter: &Painter,
    content: &str,
    style: TypeStyle,
    color: SemanticColor,
    state: ComponentState,
) -> f32 {
    text::layout(painter, content, style, color, state).size().x
}

/// The band [`status::paint_status_mark`] must be handed for its mark to land
/// centered on the column.
///
/// That primitive places its content from the left edge of the rect it is given:
/// text anchored `LEFT_CENTER` at that edge, and the selection square inset from
/// it by the authored `space/4`. Shifting the band left by half the mark plus
/// that inset is what centers the mark rather than the band. A state carrying no
/// mark is handed its band untouched, since there is nothing to place.
///
/// The inset is restated here because the primitive takes no alignment argument.
/// `a_selected_column_centers_its_mark_like_every_other_run` is what holds the
/// two in step: it measures the square that actually reached the shape list, so
/// a primitive that changed where it insets fails here rather than drifting.
fn centered_status_band(
    painter: &Painter,
    band: Rect,
    state: ComponentState,
    detail: status::StatusDetail<'_>,
) -> Rect {
    let (inset_px, width_px) = match status::status_mark(state, detail) {
        None => return band,
        Some(status::StatusMark::Text { text, color }) => (
            0.0,
            run_width_px(painter, text, TypeStyle::LabelControl, color, state),
        ),
        Some(status::StatusMark::Selection { .. }) => {
            (SpacingStep::S4.resolve(), SpacingStep::S8.resolve())
        }
    };
    Rect::from_min_max(
        pos2(band.center().x - width_px / 2.0 - inset_px, band.min.y),
        band.max,
    )
}

/// Paints the label, the value, and the status mark of one strip column, each
/// centered on the column as the specimen sets them.
///
/// Shared with the meter, which stacks the same three bands around a read-only
/// level. Painting them in one place is what keeps the two columns of a mixer
/// track reading as one column rather than as two designs — and it is why the
/// centering the specimen shows is one change rather than two.
pub(super) fn paint_column_text(
    painter: &Painter,
    bands: &Bands,
    view: &SemanticControlViewModel,
    state: ComponentState,
) {
    let label = text::layout(
        painter,
        view.label(),
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        state,
    );
    let label_width = label.size().x;
    painter.galley(
        pos2(
            bands.label.center().x - label_width / 2.0,
            bands.label.min.y,
        ),
        label,
        text::resolved_color(SemanticColor::TextSecondary, state).resolve(),
    );
    let value = value_text(view.value());
    let value_width = run_width_px(
        painter,
        &value,
        TypeStyle::CodeValue,
        value::value_color(state),
        state,
    );
    value::paint_value(
        painter,
        bands.value.center().x + value_width / 2.0,
        bands.value.center().y,
        &value,
        state,
    );
    let detail = status_detail(view);
    status::paint_status_mark(
        painter,
        centered_status_band(painter, bands.status, state, detail),
        state,
        detail,
    );
}

/// What the operator asked this control for during the frame it was painted.
///
/// A fader is grabbed along its long axis, and that axis runs the other way
/// from the screen's: dragging up asks the level to increase.
fn intent(ui: &Ui, response: &Response) -> ControlIntent {
    let dragged = response.drag_delta().y;
    if response.dragged() && dragged != 0.0 {
        return ControlIntent::AdjustRequested {
            direction: direction(-dragged),
            granularity: granularity(ui),
        };
    }
    if response.clicked() {
        return ControlIntent::FocusRequested;
    }
    ControlIntent::None
}

/// Paints the fader and returns what it asks for.
pub fn render(
    ui: &mut Ui,
    view: &SemanticControlViewModel,
    state: ComponentState,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> ControlIntent {
    let width = match role {
        PresentationRole::VerticalStrip => column_width_px(density),
        PresentationRole::ListedRow
        | PresentationRole::PanelEntry
        | PresentationRole::ModalEntry => ui.available_width(),
    };
    let size = Vec2::new(
        width.max(MIN_INTERACTIVE_TARGET_PX),
        column_height_px(ui, density),
    );
    let (column, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(column);
    let bands = bands(column);

    status::paint_row_fill(&painter, column, state);

    if bands.track.height() > 0.0 {
        let bar = track_bar(bands.track);
        let corner = Radius::Small.resolve();
        painter.rect_filled(bar, corner, SemanticColor::BgElevated.resolve());
        if let Some(fraction) = value_fraction(view) {
            let span = fill_span(bar);
            let top = span.max.y - span.height() * fraction;
            painter.rect_filled(
                Rect::from_min_max(pos2(span.min.x, top), span.max),
                corner,
                level_color(state).resolve(),
            );
            let cap_width = SpacingStep::S32.resolve();
            let cap_thickness = density.utility_control().bar_thickness_px;
            painter.rect_filled(
                Rect::from_min_max(
                    pos2(bar.center().x - cap_width / 2.0, top - cap_thickness / 2.0),
                    pos2(bar.center().x + cap_width / 2.0, top + cap_thickness / 2.0),
                ),
                corner,
                SemanticColor::TextPrimary.resolve(),
            );
        }
    }

    paint_column_text(&painter, &bands, view, state);
    focus::focus_frame(&painter, column, state);

    intent(ui, &response)
}

#[cfg(test)]
mod tests {
    use super::super::compact_slider::harness::{
        paint, paint_states, ranged_control, unranged_control, Painted,
    };
    use super::*;
    use crate::control::SemanticControlKind;
    use crate::shell::visual::controls::{control_for, ComponentControl, ControlSelection};
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::primitives::rules::RuleSpan;
    use crate::shell::visual::state::{ComponentState, NonColorSignal, ALL_COMPONENT_STATES};
    use crate::shell::visual::token::KEYLINE_RESTING_PX;

    const CONTROL: ComponentControl = ComponentControl::Fader;
    const ROLE: PresentationRole = PresentationRole::VerticalStrip;

    /// The widest disagreement two float paths may have and still be the same
    /// authored geometry. Expressed against the authored resting keyline rather
    /// than as a number chosen here.
    const TOLERANCE_PX: f32 = KEYLINE_RESTING_PX;

    /// The tallest column the authored workspace band can hold.
    fn column(density: ViewportDensityPolicy) -> Rect {
        Rect::from_min_max(
            pos2(0.0, 0.0),
            pos2(column_width_px(&density), density.bands().workspace_px),
        )
    }

    /// The axis a column is centered on, read off the groove that actually
    /// reached the shape list.
    ///
    /// Taken from the paint rather than from the allocation so the assertions
    /// that use it measure what a viewer sees, and so they do not depend on
    /// where the harness's panel happened to place the column. The groove is
    /// the only rect an authored `space/12` wide: the fill and the selection
    /// mark are `space/8`, the cap is `space/32`, and the frame and hairlines
    /// span the column or one pixel.
    fn painted_axis_x(painted: &Painted) -> f32 {
        painted
            .rects
            .iter()
            .find(|painted| {
                (painted.rect.width() - SpacingStep::S12.resolve()).abs() < TOLERANCE_PX
            })
            .expect("a strip column paints its level groove")
            .rect
            .center()
            .x
    }

    #[test]
    fn the_family_asks_for_this_control_in_a_mixer_column() {
        for kind in [
            SemanticControlKind::Continuous,
            SemanticControlKind::Stepped,
        ] {
            assert_eq!(
                control_for(kind, ROLE),
                ControlSelection::Control(CONTROL),
                "a numeric mixer column no longer resolves to the fader"
            );
        }
    }

    /// The fader accepts every declared state, so its sweep is the whole
    /// vocabulary rather than a subset of it.
    #[test]
    fn the_fader_declares_every_state_applicable() {
        assert_eq!(CONTROL.applicable_states(), ALL_COMPONENT_STATES);
        for state in [ComponentState::Muted, ComponentState::Soloed] {
            assert!(
                CONTROL.accepts(state),
                "a mixer fader must accept {}",
                state.canonical_name()
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

    /// No two applicable states are distinguishable by color alone (FR-003),
    /// which is what makes `M ON` and `S ON` load-bearing rather than
    /// decorative.
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

    #[test]
    fn muted_soloed_and_resting_are_three_different_things_without_color() {
        let view = ranged_control();
        let states = [
            ComponentState::Resting,
            ComponentState::Muted,
            ComponentState::Soloed,
        ];
        let painted = paint_states(
            CONTROL,
            &view,
            &states,
            ROLE,
            ViewportDensityPolicy::Desktop,
        );
        assert!(painted[1].says("M ON"), "a muted fader did not say M ON");
        assert!(painted[2].says("S ON"), "a soloed fader did not say S ON");
        assert!(
            !painted[0].says("M ON") && !painted[0].says("S ON"),
            "a resting fader claimed a mute or solo it was not handed"
        );
    }

    /// The specimen centers `Track`, `Value`, and `State` on the column, over a
    /// centered groove. All three runs therefore sit on the column's axis — the
    /// name is not flush left, the value is not flush right, and the status line
    /// is not flush left.
    #[test]
    fn the_three_column_runs_sit_on_the_column_axis_at_both_viewports() {
        let view = ranged_control();
        for density in ALL_DENSITY_POLICIES {
            // `Muted` rather than `Resting`: it is the state whose status band
            // carries a run, so all three of the specimen's text layers are on
            // screen in one pass.
            let painted = paint(CONTROL, &view, ComponentState::Muted, ROLE, density);
            let axis = painted_axis_x(&painted);
            for content in [
                view.label().to_owned(),
                value_text(view.value()),
                "M ON".to_owned(),
            ] {
                let run = painted.run(&content).unwrap_or_else(|| {
                    panic!(
                        "{} painted no run saying {content:?}",
                        density.canonical_name()
                    )
                });
                assert!(
                    (run.rect.center().x - axis).abs() < TOLERANCE_PX,
                    "{} paints {content:?} at {} rather than on the column axis {axis}",
                    density.canonical_name(),
                    run.rect.center().x
                );
            }
        }
    }

    /// `Selected` marks its status band with a shape rather than a run, and it
    /// is centered too.
    ///
    /// This is also what holds [`centered_status_band`]'s restated inset in step
    /// with the primitive it compensates for: the square measured here is the
    /// one that actually reached the shape list, so a primitive that changed
    /// where it insets fails here instead of quietly going off-axis.
    #[test]
    fn a_selected_column_centers_its_mark_like_every_other_run() {
        let view = ranged_control();
        let density = ViewportDensityPolicy::Desktop;
        let painted = paint(CONTROL, &view, ComponentState::Selected, ROLE, density);
        let axis = painted_axis_x(&painted);
        let side = SpacingStep::S8.resolve();
        let groove_bottom = painted
            .rects
            .iter()
            .find(|painted| {
                (painted.rect.width() - SpacingStep::S12.resolve()).abs() < TOLERANCE_PX
            })
            .expect("a strip column paints its level groove")
            .rect
            .max
            .y;
        let mark = painted
            .rects
            .iter()
            .find(|painted| {
                painted.rect.min.y > groove_bottom
                    && (painted.rect.width() - side).abs() < TOLERANCE_PX
                    && (painted.rect.height() - side).abs() < TOLERANCE_PX
            })
            .expect("a selected column paints the authored selection mark below its track");
        assert!(
            (mark.rect.center().x - axis).abs() < TOLERANCE_PX,
            "a selected column puts its mark at {} rather than on the column axis {axis}",
            mark.rect.center().x
        );
    }

    /// Sixteen columns and their hairline gutters fit the authored main surface
    /// at both viewports — discovered here rather than when the mixer workspace
    /// is recomposed.
    #[test]
    fn sixteen_columns_and_their_hairlines_fit_the_main_surface() {
        for density in ALL_DENSITY_POLICIES {
            let content = density.split().main_px - density.rhythm().inset_px * 2.0;
            let columns = column_width_px(&density) * STRIP_COLUMN_COUNT as f32;
            let gutters = SpacingStep::S4.resolve() * (STRIP_COLUMN_COUNT - 1) as f32;
            assert!(
                columns + gutters <= content,
                "{} cannot show {STRIP_COLUMN_COUNT} columns in {content} px",
                density.canonical_name()
            );
            // The gutter is wide enough to carry the authored hairline the
            // design file separates columns with.
            let hairline = RuleSpan::Vertical {
                x_px: column_width_px(&density),
                from_y_px: 0.0,
                to_y_px: density.bands().workspace_px,
            }
            .rect();
            assert!(
                hairline.width() <= SpacingStep::S4.resolve(),
                "{} has no room for a separator between columns",
                density.canonical_name()
            );
            assert_eq!(hairline.width(), KEYLINE_RESTING_PX);
        }
    }

    #[test]
    fn a_column_is_never_narrower_than_the_authored_interactive_target() {
        for density in ALL_DENSITY_POLICIES {
            assert!(
                column_width_px(&density) >= MIN_INTERACTIVE_TARGET_PX,
                "{} shrinks a mixer column below the authored target",
                density.canonical_name()
            );
        }
    }

    #[test]
    fn the_four_bands_stack_without_overlapping_at_both_viewports() {
        for density in ALL_DENSITY_POLICIES {
            let bands = bands(column(density));
            assert!(bands.label.max.y < bands.track.min.y);
            assert!(bands.track.max.y < bands.value.min.y);
            assert!(bands.value.max.y <= bands.status.min.y);
            assert!(
                bands.track.height() > 0.0,
                "{} leaves the fader no track",
                density.canonical_name()
            );
            let bar = track_bar(bands.track);
            assert_eq!(bar.width(), SpacingStep::S12.resolve());
            assert_eq!(fill_span(bar).width(), SpacingStep::S8.resolve());
        }
    }

    #[test]
    fn a_declared_range_positions_the_fill_inside_the_track() {
        let view = ranged_control();
        let density = ViewportDensityPolicy::Desktop;
        let fraction = value_fraction(&view).expect("a ranged specimen has a position");
        let span = fill_span(track_bar(bands(column(density)).track));
        let painted = paint(CONTROL, &view, ComponentState::Resting, ROLE, density);
        let expected = span.height() * fraction;
        assert!(
            painted.rects.iter().any(|painted| {
                (painted.rect.height() - expected).abs() < KEYLINE_RESTING_PX
                    && (painted.rect.width() - span.width()).abs() < KEYLINE_RESTING_PX
            }),
            "no painted level matches the position the view data reports"
        );
    }

    #[test]
    fn an_absent_range_leaves_the_level_unpositioned() {
        let view = unranged_control();
        assert_eq!(value_fraction(&view), None);
        let density = ViewportDensityPolicy::Desktop;
        let span = fill_span(track_bar(bands(column(density)).track));
        let painted = paint(CONTROL, &view, ComponentState::Resting, ROLE, density);
        assert!(
            !painted.rects.iter().any(|painted| {
                (painted.rect.width() - span.width()).abs() < KEYLINE_RESTING_PX
                    && painted.rect.height() < span.height() - KEYLINE_RESTING_PX
            }),
            "an unranged fader invented a level"
        );
    }

    #[test]
    fn it_paints_only_what_the_view_data_carries() {
        let view = ranged_control();
        let painted = paint(
            CONTROL,
            &view,
            ComponentState::Resting,
            ROLE,
            ViewportDensityPolicy::Desktop,
        );
        let carried = [view.label().to_owned(), value_text(view.value())];
        for run in &painted.texts {
            assert!(
                carried.iter().any(|allowed| allowed == run),
                "the fader painted {run:?}, which is not in its view data"
            );
        }
    }

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
