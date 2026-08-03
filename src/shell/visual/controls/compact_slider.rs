//! The compact slider.
//!
//! The continuous control a Utility/Inspector panel entry carries where a full
//! parameter row does not fit.
//!
//! # Specimen
//!
//! Design file `kdQMw8dYUZtv2UxJPo0sXU`, frame `SCREEN · Instrument Detail ·
//! 1920×1080` (`37:7`), the `Utility MASTER VOLUME` entry (`37:53`) and its four
//! siblings. The entry is 380 × 48 on a 60 px pitch: a 16 px top line carrying
//! the parameter name on the left and its value on the right, then a 5 px value
//! bar whose fill runs from the left edge. The variables bound to it are
//! `color/bg/elevated` for the bar, `color/accent/positive` for the fill, and
//! `color/text/secondary` for the name.
//!
//! Those four numbers — 380, 48, 60, 5 — are exactly what
//! [`ViewportDensityPolicy::utility_control`] already declares, so this control
//! asks the policy rather than restating the specimen. The compact viewport is
//! the policy's authored 280 × 48; the design file carries no compact frames,
//! which is why that policy records itself as authored rather than measured.
//!
//! The specimen carries **no separate position indicator**: the fill's leading
//! edge is the position, in the utility entry and in the design file's
//! `PARAMETER / COMPACT SLIDER` component set alike. Two specimens agreeing is
//! an answer rather than a gap, so nothing here paints a handle.
//!
//! # Painted, not assembled
//!
//! `research.md` R-02: egui supplies layout, hit-testing, and the response; the
//! appearance is painted. An `egui::Slider` would carry its own rounding, fill,
//! and drag handle out of `egui::Style`, which is a second visual vocabulary
//! beside [`SemanticVisualToken`](crate::shell::visual::token) — and one the
//! literal guard cannot see.
//!
//! # What it does not do
//!
//! It owns no value. It reads the scalar, the range, and the typed failure out
//! of the immutable [`SemanticControlViewModel`] it is handed, and paints
//! nothing it did not read there. **A control whose view data declares no range
//! is painted explicitly unpositioned** — an assumed `0..1` would be an invented
//! value, and an invented value is indistinguishable from a real one once it is
//! on screen (C-003). It returns [`ControlIntent`]: it neither clamps the
//! adjustment it reports nor decides whether that adjustment is legal.
//!
//! # The shared readings
//!
//! [`scalar_value`], [`value_text`], [`value_fraction`], [`status_detail`],
//! [`granularity`], and [`direction`] are `pub(super)` because the fader and the
//! meter read a value exactly the same way. One reading shared by the three
//! continuous controls is the point: three private copies would be three places
//! for "what number is this" to drift apart.

use eframe::egui::{pos2, Rect, Response, Sense, Ui, Vec2};

use super::{AdjustDirection, AdjustGranularity, ControlIntent, PresentationRole};
use crate::control::{SemanticControlValue, SemanticControlViewModel};
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::{focus, status, text, value};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{
    Radius, SemanticColor, SpacingStep, TypeStyle, MIN_INTERACTIVE_TARGET_PX,
};
use crate::synth::ParameterValue;

/// The color role a value bar's fill paints in for a given state.
///
/// Declared as data in the shape
/// [`value::value_color`](crate::shell::visual::primitives::value::value_color)
/// already uses, rather than branched at the call site. `Resting` paints
/// `color/accent/positive`, which is the variable the design file binds to the
/// utility entry's fill; every other state paints the accent its
/// [`ComponentState`] already declares, so the bar never disagrees with the
/// keyline around it.
///
/// `Muted` and `Soloed` are named because the match is exhaustive and carries no
/// wildcard, not because a compact slider can be handed them — it declares
/// neither applicable.
pub(super) const fn fill_color(state: ComponentState) -> SemanticColor {
    match state {
        ComponentState::Resting | ComponentState::Soloed => SemanticColor::AccentPositive,
        ComponentState::Focused => SemanticColor::AccentFocus,
        ComponentState::Adjusting | ComponentState::Loading => SemanticColor::AccentAdjust,
        ComponentState::Disabled => SemanticColor::TextMuted,
        ComponentState::Error | ComponentState::Muted => SemanticColor::AccentWarning,
        ComponentState::Selected => SemanticColor::TextPrimary,
    }
}

/// The scalar a control's value carries, or `None` when it carries no number.
///
/// A value that is not a number has no position on a track, and this returning
/// `None` is how that stays true all the way to the paint.
pub(super) fn scalar_value(value: &SemanticControlValue) -> Option<f64> {
    match value {
        SemanticControlValue::Scalar(scalar) => Some(*scalar),
        SemanticControlValue::Parameter(parameter) => match parameter {
            ParameterValue::Continuous(scalar) => Some(*scalar),
            ParameterValue::Stepped(step) => Some(*step as f64),
            ParameterValue::Choice(_) | ParameterValue::Toggle(_) => None,
        },
        SemanticControlValue::Asset(_)
        | SemanticControlValue::Identity(_)
        | SemanticControlValue::Summary(_) => None,
    }
}

/// The finished text a control shows for its value.
///
/// The number becomes a string here because the semantic view model carries a
/// canonical `f64` rather than finished text, and the shell has always rendered
/// it at this edge. It is the same rendering the adapter's row painter has
/// produced since before this mission, so no surface starts disagreeing with
/// another about the same value.
pub(super) fn value_text(value: &SemanticControlValue) -> String {
    match value {
        SemanticControlValue::Scalar(scalar) => format!("{scalar:.3}"),
        SemanticControlValue::Parameter(parameter) => match parameter {
            ParameterValue::Continuous(scalar) => format!("{scalar:.3}"),
            ParameterValue::Stepped(step) => step.to_string(),
            ParameterValue::Choice(choice) => choice.clone(),
            ParameterValue::Toggle(on) => if *on { "ON" } else { "OFF" }.to_owned(),
        },
        SemanticControlValue::Asset(reference) => reference.locator().to_owned(),
        SemanticControlValue::Identity(identity) => identity.clone(),
        SemanticControlValue::Summary(summary) => summary.clone(),
    }
}

/// Where this control's value sits inside its declared range, as `0.0..=1.0`.
///
/// `None` when the view data declares no range, carries no number, or declares
/// a range with no width. Each of those is a reason there is no position to
/// paint, and none of them is a reason to invent one: a caller that assumed
/// `0..1` would put a plausible fill on screen for a value nobody reported.
///
/// The fraction is clamped for painting only. That does not clamp the value —
/// the control reports the adjustment that was asked for, and what the value
/// becomes is decided on the other side of this boundary.
pub(super) fn value_fraction(view: &SemanticControlViewModel) -> Option<f32> {
    let range = view.numeric_range()?;
    let scalar = scalar_value(view.value())?;
    let width = range.maximum() - range.minimum();
    if !width.is_finite() || width <= 0.0 || !scalar.is_finite() {
        return None;
    }
    Some((((scalar - range.minimum()) / width) as f32).clamp(0.0, 1.0))
}

/// The typed failure an erroring control reports, if it reports one.
///
/// A loading control resolves to [`status::StatusDetail::None`], which the
/// status primitive already renders as the first authored progress word. Which
/// structural-edit phase a control is in is a composition's reading of the
/// lifecycle status; a control derives no phase of its own.
pub(super) fn status_detail(view: &SemanticControlViewModel) -> status::StatusDetail<'_> {
    match view.error() {
        Some(error) => status::StatusDetail::Failure(error.label()),
        None => status::StatusDetail::None,
    }
}

/// How far one reported adjustment step moves.
///
/// Read from the live modifier state, which is input — egui's half of the
/// boundary `research.md` R-02 draws. The control reports which step was asked
/// for; what that step *is* belongs to the parameter descriptor the view data
/// carries and is resolved elsewhere.
pub(super) fn granularity(ui: &Ui) -> AdjustGranularity {
    if ui.input(|input| input.modifiers.shift) {
        AdjustGranularity::Coarse
    } else {
        AdjustGranularity::Fine
    }
}

/// Which way a signed drag along an axis asked the value to move.
pub(super) fn direction(delta: f32) -> AdjustDirection {
    if delta > 0.0 {
        AdjustDirection::Increase
    } else {
        AdjustDirection::Decrease
    }
}

/// The size this control asks for at `density` in `role`.
///
/// In its own role the extent is the policy's authored utility-control
/// geometry. Asked in any other role it fills the width it is given, because
/// choosing a width for a surface it was not authored for would be the control
/// deciding its own layout.
///
/// Both extents are held at or above [`MIN_INTERACTIVE_TARGET_PX`]. A compact
/// slider is compact in one axis, and neither axis is allowed to be the one it
/// is grabbed on.
fn desired_size(ui: &Ui, role: PresentationRole, density: &ViewportDensityPolicy) -> Vec2 {
    let geometry = density.utility_control();
    let width = match role {
        PresentationRole::PanelEntry => geometry.width_px,
        PresentationRole::ListedRow
        | PresentationRole::VerticalStrip
        | PresentationRole::ModalEntry => ui.available_width(),
    };
    Vec2::new(
        width.max(MIN_INTERACTIVE_TARGET_PX),
        geometry.height_px.max(MIN_INTERACTIVE_TARGET_PX),
    )
}

/// The three horizontal bands the entry is laid out on, top to bottom.
struct Bands {
    /// Name on the left, value on the right.
    label: Rect,
    /// The value bar.
    bar: Rect,
    /// The state's status mark.
    status: Rect,
}

/// Resolves the bands from the allocated rect.
///
/// The left inset is the reserved cursor column
/// ([`focus::LABEL_START_X_PX`]), reserved whether or not the `>` is drawn so
/// that focusing the entry never shifts its name. The label and status bands
/// are one authored `Label/Control` line each, pinned to the top and the
/// bottom; the bar is centered in what is left, so it stays centered at any
/// height rather than depending on the authored 48 px arithmetic.
fn bands(rect: Rect, density: &ViewportDensityPolicy) -> Bands {
    let content = Rect::from_min_max(
        pos2(rect.min.x + focus::LABEL_START_X_PX, rect.min.y),
        pos2(rect.max.x - SpacingStep::S12.resolve(), rect.max.y),
    );
    let line = TypeStyle::LabelControl.metrics().line_height_px;
    let label = Rect::from_min_max(content.min, pos2(content.max.x, content.min.y + line));
    let status = Rect::from_min_max(pos2(content.min.x, content.max.y - line), content.max);
    let thickness = density.utility_control().bar_thickness_px;
    let middle = (label.max.y + status.min.y) / 2.0;
    let bar = Rect::from_min_max(
        pos2(content.min.x, middle - thickness / 2.0),
        pos2(content.max.x, middle + thickness / 2.0),
    );
    Bands { label, bar, status }
}

/// What the operator asked this control for during the frame it was painted.
///
/// A drag along the long axis reports the adjustment it asked for; a click asks
/// to become the focus target. Neither is a decision: the control does not know
/// whether it may be focused or whether the value may move.
fn intent(ui: &Ui, response: &Response) -> ControlIntent {
    let dragged = response.drag_delta().x;
    if response.dragged() && dragged != 0.0 {
        return ControlIntent::AdjustRequested {
            direction: direction(dragged),
            granularity: granularity(ui),
        };
    }
    if response.clicked() {
        return ControlIntent::FocusRequested;
    }
    ControlIntent::None
}

/// Paints the compact slider and returns what it asks for.
pub fn render(
    ui: &mut Ui,
    view: &SemanticControlViewModel,
    state: ComponentState,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> ControlIntent {
    let (rect, response) =
        ui.allocate_exact_size(desired_size(ui, role, density), Sense::click_and_drag());
    let painter = ui.painter().clone();
    let bands = bands(rect, density);

    status::paint_row_fill(&painter, rect, state);
    focus::cursor(&painter, rect, state);

    let name = text::layout(
        &painter,
        view.label(),
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        state,
    );
    painter.galley(
        bands.label.min,
        name,
        text::resolved_color(SemanticColor::TextSecondary, state).resolve(),
    );

    value::paint_value(
        &painter,
        bands.label.max.x,
        bands.label.center().y,
        &value_text(view.value()),
        state,
    );

    let corner = Radius::Small.resolve();
    painter.rect_filled(bands.bar, corner, SemanticColor::BgElevated.resolve());
    if let Some(fraction) = value_fraction(view) {
        let filled = Rect::from_min_max(
            bands.bar.min,
            pos2(
                bands.bar.min.x + bands.bar.width() * fraction,
                bands.bar.max.y,
            ),
        );
        painter.rect_filled(filled, corner, fill_color(state).resolve());
    }

    status::paint_status_mark(&painter, bands.status, state, status_detail(view));
    focus::focus_frame(&painter, rect, state);

    intent(ui, &response)
}

#[cfg(test)]
pub(super) mod harness {
    //! The headless paint harness the four controls in this work package share.
    //!
    //! It runs the production render through a real `egui::Context` with the
    //! authored faces installed and reports what actually reached the shape
    //! list. Nothing here asserts against the arguments it was handed: a
    //! specimen assertion that never painted would pass vacuously, which is the
    //! failure mode a state-coverage check is most prone to.

    use eframe::egui::{self, Pos2, Rect, Shape, Vec2};

    use crate::control::{
        AppState, SemanticControlViewModel, SemanticGraphicalViewModel, SemanticSurfaceViewModel,
    };
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::shell::visual::controls::{ComponentControl, ControlIntent, PresentationRole};
    use crate::shell::visual::density::ViewportDensityPolicy;
    use crate::shell::visual::state::ComponentState;
    use crate::shell::visual::typeface::AuthoredTypeface;
    use crate::synth::{CapabilityRegistry, InstrumentCapabilityProvider};

    /// One rectangle that reached the shape list, described without its color.
    ///
    /// The blur width is what separates the authored focus halo from an
    /// ordinary fill: `Shadow::as_shape` is a blurred rect expanded by the
    /// authored spread, so a haloed state has geometry an unhaloed one does not.
    #[derive(Debug, PartialEq)]
    pub struct PaintedRect {
        /// Where it was painted.
        pub rect: Rect,
        /// The stroke width, or zero for a fill.
        pub stroke_px: f32,
        /// The blur width, or zero for a hard edge.
        pub blur_px: f32,
    }

    /// One text run that reached the shape list, with where it landed.
    ///
    /// [`Painted::texts`] carries what was said; this carries where it was said.
    /// The two are kept apart deliberately: alignment is a specimen question,
    /// and [`Painted::colorless_evidence`] answers a different one, so a run's
    /// position is not folded into the FR-003 comparison.
    #[derive(Debug, PartialEq)]
    pub struct PaintedRun {
        /// What the run says.
        pub text: String,
        /// The box the run occupies.
        pub rect: Rect,
    }

    /// Everything one painted specimen produced.
    pub struct Painted {
        /// Every text run that reached the shape list, in paint order.
        pub texts: Vec<String>,
        /// Every text run that reached the shape list, with its position.
        pub runs: Vec<PaintedRun>,
        /// Every rectangle that reached the shape list, in paint order.
        pub rects: Vec<PaintedRect>,
        /// The variant name of every shape that reached the list, so a specimen
        /// that painted a mesh or a path is not silently unseen.
        pub kinds: Vec<&'static str>,
        /// The debug rendering of the whole shape list, for comparing two
        /// renders of the same input.
        pub signature: String,
        /// What the control asked for.
        pub intent: ControlIntent,
    }

    impl Painted {
        fn empty() -> Self {
            Self {
                texts: Vec::new(),
                runs: Vec::new(),
                rects: Vec::new(),
                kinds: Vec::new(),
                signature: String::new(),
                intent: ControlIntent::None,
            }
        }

        /// Whether any painted run contains `needle`.
        pub fn says(&self, needle: &str) -> bool {
            self.texts.iter().any(|run| run.contains(needle))
        }

        /// Whether anything at all reached the shape list.
        pub fn painted_anything(&self) -> bool {
            !self.texts.is_empty() || !self.rects.is_empty()
        }

        /// The first run saying exactly `text`, or `None` if none does.
        ///
        /// Exact rather than containing, because an alignment assertion that
        /// matched a substring would measure a box belonging to a longer run.
        pub fn run(&self, text: &str) -> Option<&PaintedRun> {
            self.runs.iter().find(|run| run.text == text)
        }

        /// Everything this specimen put on screen that is **not** a color.
        ///
        /// Two states whose evidence strings are equal are two states a viewer
        /// who cannot separate their accents could not tell apart, which is
        /// exactly what FR-003 forbids.
        pub fn colorless_evidence(&self) -> String {
            format!("{:?}|{:?}|{:?}", self.texts, self.rects, self.kinds)
        }
    }

    fn collect(shape: &Shape, painted: &mut Painted) {
        painted.kinds.push(match shape {
            Shape::Noop => "Noop",
            Shape::Vec(_) => "Vec",
            Shape::Circle(_) => "Circle",
            Shape::Ellipse(_) => "Ellipse",
            Shape::LineSegment { .. } => "LineSegment",
            Shape::Path(_) => "Path",
            Shape::Rect(_) => "Rect",
            Shape::Text(_) => "Text",
            Shape::Mesh(_) => "Mesh",
            Shape::QuadraticBezier(_) => "QuadraticBezier",
            Shape::CubicBezier(_) => "CubicBezier",
            Shape::Callback(_) => "Callback",
        });
        match shape {
            Shape::Text(text) => {
                painted.texts.push(text.galley.text().to_owned());
                painted.runs.push(PaintedRun {
                    text: text.galley.text().to_owned(),
                    rect: Rect::from_min_size(text.pos, text.galley.size()),
                });
            }
            Shape::Rect(rect) => painted.rects.push(PaintedRect {
                rect: rect.rect,
                stroke_px: rect.stroke.width,
                blur_px: rect.blur_width,
            }),
            Shape::Vec(shapes) => {
                for nested in shapes {
                    collect(nested, painted);
                }
            }
            Shape::Noop
            | Shape::Circle(_)
            | Shape::Ellipse(_)
            | Shape::LineSegment { .. }
            | Shape::Path(_)
            | Shape::QuadraticBezier(_)
            | Shape::CubicBezier(_)
            | Shape::Mesh(_)
            | Shape::Callback(_) => {}
        }
    }

    /// Runs one control's production render once per supplied state and reports
    /// what each pass painted.
    ///
    /// One context serves the whole sweep, and the first pass is a warm-up whose
    /// output is discarded: a first-frame face miss would otherwise be measured
    /// as a rendering failure. The viewport is the policy's authored size, so a
    /// specimen is measured at the size it was authored for rather than at one
    /// the harness invented.
    pub fn paint_states(
        control: ComponentControl,
        view: &SemanticControlViewModel,
        states: &[ComponentState],
        role: PresentationRole,
        density: ViewportDensityPolicy,
    ) -> Vec<Painted> {
        let context = egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the vendored faces are present")
                .font_definitions(),
        );
        let viewport = density.authored_viewport();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(viewport.width_px, viewport.height_px),
            )),
            ..egui::RawInput::default()
        };
        let pass = |state: ComponentState| {
            let mut painted = Painted::empty();
            let output = context.run(input.clone(), |context| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(egui::Margin::ZERO))
                    .show(context, |ui| {
                        painted.intent = control.render(ui, view, state, role, &density);
                    });
            });
            for clipped in &output.shapes {
                collect(&clipped.shape, &mut painted);
            }
            painted.signature = format!("{:?}", output.shapes);
            painted
        };
        let first = *states.first().expect("a specimen sweep names a state");
        drop(pass(first));
        states.iter().map(|state| pass(*state)).collect()
    }

    /// Runs one control's production render for one state.
    pub fn paint(
        control: ComponentControl,
        view: &SemanticControlViewModel,
        state: ComponentState,
        role: PresentationRole,
        density: ViewportDensityPolicy,
    ) -> Painted {
        paint_states(control, view, &[state], role, density)
            .pop()
            .expect("one state paints one specimen")
    }

    /// The production MIXER projection of a reducer built without any asset
    /// file.
    ///
    /// Real projected view data rather than hand-built values, so a specimen
    /// paints the shape the reducer actually projects. The Braids capability is
    /// used because it needs no SoundFont on disk: these controls paint, they do
    /// not sound, and a paint test that fails because an asset is missing would
    /// couple the two for no gain.
    fn projection() -> SemanticGraphicalViewModel {
        let braids = crate::adapter::braids_capability::BraidsCapability::new()
            .expect("the Braids capability builds without an asset file");
        let registry = CapabilityRegistry::new(vec![braids.descriptor()])
            .expect("the specimen registry is valid");
        let state = AppState::new(
            registry,
            GlobalParameters::new(0.0).expect("the specimen global parameters are valid"),
        );
        SemanticGraphicalViewModel::project(&state, "wp03-specimen")
            .expect("the production MIXER projection resolves")
    }

    fn any_control(
        predicate: impl Fn(&SemanticControlViewModel) -> bool,
    ) -> SemanticControlViewModel {
        projection()
            .surfaces()
            .iter()
            .flat_map(SemanticSurfaceViewModel::controls)
            .find(|control| predicate(control))
            .expect("the production projection carries a control matching the specimen predicate")
            .clone()
    }

    /// A projected control that declares a numeric range — a mixer track level.
    pub fn ranged_control() -> SemanticControlViewModel {
        any_control(|control| control.numeric_range().is_some())
    }

    /// A projected control that declares no numeric range.
    ///
    /// The controls do not branch on the control's kind, so what this specimen
    /// carries beyond the absent range does not matter to the assertions that
    /// use it.
    pub fn unranged_control() -> SemanticControlViewModel {
        any_control(|control| control.numeric_range().is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::harness::{paint, paint_states, ranged_control, unranged_control};
    use super::*;
    use crate::control::SemanticControlKind;
    use crate::shell::visual::controls::{control_for, ComponentControl, ControlSelection};
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::state::NonColorSignal;
    use crate::shell::visual::token::KEYLINE_RESTING_PX;

    const CONTROL: ComponentControl = ComponentControl::CompactSlider;
    const ROLE: PresentationRole = PresentationRole::PanelEntry;

    /// The widest disagreement two float paths may have and still be the same
    /// authored geometry. Expressed against the authored resting keyline rather
    /// than as a number chosen here.
    const TOLERANCE_PX: f32 = KEYLINE_RESTING_PX;

    /// The rect the entry is allocated at one policy, for the geometry
    /// assertions that do not need a paint pass.
    fn entry(density: ViewportDensityPolicy) -> Rect {
        let geometry = density.utility_control();
        Rect::from_min_max(pos2(0.0, 0.0), pos2(geometry.width_px, geometry.height_px))
    }

    /// The control is reachable. A control with passing specimens that nothing
    /// can ask for is dead code.
    #[test]
    fn the_family_asks_for_this_control_in_a_panel_entry() {
        for kind in [
            SemanticControlKind::Continuous,
            SemanticControlKind::Stepped,
        ] {
            assert_eq!(
                control_for(kind, ROLE),
                ControlSelection::Control(CONTROL),
                "a numeric panel entry no longer resolves to the compact slider"
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

    /// Removing a state's non-color treatment fails the assertion above. This
    /// is what makes that claim checkable: the words the sweep looks for are
    /// the ones the state vocabulary declares, so a state that stopped
    /// declaring one would stop being found.
    #[test]
    fn the_words_the_sweep_looks_for_are_the_declared_ones() {
        assert_eq!(
            ComponentState::Disabled.appearance().signal,
            NonColorSignal::Word("Locked")
        );
        assert_eq!(
            ComponentState::Loading.appearance().signal,
            NonColorSignal::ProgressWord
        );
        assert_eq!(
            ComponentState::Error.appearance().signal,
            NonColorSignal::TypedFailure
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
                "the compact slider painted {run:?}, which is not in its view data"
            );
        }
        assert!(
            painted.says(view.label()),
            "the compact slider did not paint the label its view data carries"
        );
        assert!(
            painted.says(&value_text(view.value())),
            "the compact slider did not paint the value its view data carries"
        );
    }

    /// No two applicable states are distinguishable by color alone (FR-003).
    ///
    /// The comparison deliberately excludes every color: what is left is the
    /// text and the geometry, and two states whose remainder is identical are
    /// two states a viewer who cannot separate their accents could not tell
    /// apart.
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
    fn a_declared_range_positions_the_fill_inside_the_bar() {
        let view = ranged_control();
        let density = ViewportDensityPolicy::Desktop;
        let fraction = value_fraction(&view).expect("a ranged specimen has a position");
        let bar = bands(entry(density), &density).bar;
        let painted = paint(CONTROL, &view, ComponentState::Resting, ROLE, density);
        let expected = bar.width() * fraction;
        assert!(
            painted.rects.iter().any(|painted| {
                (painted.rect.width() - expected).abs() < TOLERANCE_PX
                    && (painted.rect.height() - bar.height()).abs() < TOLERANCE_PX
            }),
            "no painted fill matches the position the view data reports"
        );
    }

    #[test]
    fn an_absent_range_leaves_the_value_unpositioned() {
        let view = unranged_control();
        assert_eq!(
            value_fraction(&view),
            None,
            "a control with no declared range must have no painted position"
        );
        let density = ViewportDensityPolicy::Desktop;
        let bar = bands(entry(density), &density).bar;
        let painted = paint(CONTROL, &view, ComponentState::Resting, ROLE, density);
        assert!(
            !painted.rects.iter().any(|painted| {
                (painted.rect.height() - bar.height()).abs() < TOLERANCE_PX
                    && painted.rect.width() < bar.width() - TOLERANCE_PX
            }),
            "an unranged compact slider invented a partial fill"
        );
    }

    #[test]
    fn the_interactive_target_never_drops_below_the_authored_minimum() {
        for density in ALL_DENSITY_POLICIES {
            let geometry = density.utility_control();
            assert!(
                geometry.height_px >= MIN_INTERACTIVE_TARGET_PX,
                "{} shrinks the compact slider below the authored target",
                density.canonical_name()
            );
            assert!(
                geometry.width_px >= MIN_INTERACTIVE_TARGET_PX,
                "{} shrinks the compact slider's grabbed axis below the authored target",
                density.canonical_name()
            );
        }
    }

    #[test]
    fn the_value_bar_carries_the_authored_thickness_at_both_viewports() {
        for density in ALL_DENSITY_POLICIES {
            let bands = bands(entry(density), &density);
            assert_eq!(
                bands.bar.height(),
                density.utility_control().bar_thickness_px,
                "{} paints a bar the policy did not declare",
                density.canonical_name()
            );
            assert!(
                bands.bar.min.y > bands.label.max.y && bands.bar.max.y < bands.status.min.y,
                "{} overlaps the bar with a text band",
                density.canonical_name()
            );
        }
    }

    #[test]
    fn direction_follows_the_sign_of_the_drag() {
        assert_eq!(direction(1.0), AdjustDirection::Increase);
        assert_eq!(direction(-1.0), AdjustDirection::Decrease);
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
        assert_eq!(
            painted.intent,
            ControlIntent::None,
            "a frame in which nothing happened produced a request"
        );
    }
}
