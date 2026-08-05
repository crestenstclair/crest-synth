//! The parameter row — the most common control in the product.
//!
//! A labelled row carrying one parameter's value: label on the left in
//! `Label/Control`, value right-aligned in `Code/Value`, and, when the view
//! data carries bounds, a position indicator between them.
//!
//! # Figma source
//!
//! - `SCREEN · Instrument Detail · 1920×1080` → the `Parameter / …` component
//!   instances (node `37:101`). Its columns are the row's anatomy: a reserved
//!   prefix column, an index, the label, the position track with its fill and
//!   handle, then the value.
//! - `SCREEN · Patch Strip · 1920×1080` → the `Instrument` row (node `36:28`)
//!   for the reserved cursor column and the leader that runs from the label to
//!   the value.
//!
//! Height and pitch are *not* taken from those frames. The specimen is authored
//! at one viewport; [`ViewportDensityPolicy`] carries both, so the row asks the
//! policy and the frame supplies only the horizontal anatomy.
//!
//! # What this module owns
//!
//! Nothing. It reads the immutable view model it is handed, paints, and returns
//! [`ControlIntent`]. It resolves no Patch value, caches nothing, and reaches no
//! reducer or audio state.
//!
//! This file also carries the row scaffolding its three sibling controls
//! compose — [`allocate_row`], [`paint_row_chrome`], [`value_text`] — because a
//! listed row is one shape with four contents, and four private copies of it
//! would be four things to keep in step.

use eframe::egui::{pos2, Painter, Rect, Response, Sense, Ui};

use super::{ControlIntent, PresentationRole};
use crate::control::{SemanticControlValue, SemanticControlViewModel};
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::status::{LoadingPhase, StatusDetail};
use crate::shell::visual::primitives::{focus, rules, status, text, value};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{Radius, SemanticColor, SpacingStep, TypeStyle};
use crate::synth::ParameterValue;

/// How many decimal places a continuous value reads to.
///
/// This is the presentation the shell already ships — the graphical adapter's
/// row has formatted continuous values to three places since before this
/// vocabulary existed. It is named here rather than spelled at each call site
/// so the two cannot drift apart while both are alive; the adapter's copy
/// retires when WP04 converts that surface to this control.
pub const VALUE_DECIMAL_PLACES: usize = 3;

/// What a control paints where the view data carries nothing to paint.
///
/// A reserved region that is simply left blank reads as "there is nothing
/// here"; one filled with a decorative mark reads as data. Neither is true of a
/// region whose data did not arrive, so it is marked explicitly.
///
/// This mark is not authored in the design file. It is raised in this work
/// package's handoff rather than left to look authoritative.
pub const UNAVAILABLE_MARK: &str = "--";

/// Paints a parameter row and returns what the operator asked for.
///
/// `state` and `role` are handed in and never derived. The row asks to become
/// the focus target when it is addressed; whether that is allowed is the
/// reducer's decision, on the other side of this boundary.
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

    let label_right = paint_row_label(&painter, row, view.label(), state);
    let rendered = value_text(view.value());
    let value_left = paint_row_value(&painter, row, &rendered, state);

    // The middle region carries the row's lifecycle when it has one and its
    // position when it does not. A row mid-structural-edit is reporting
    // something the operator has to read; the indicator can wait.
    let middle = middle_region(row, label_right, value_left);
    let detail = status_detail(view);
    if status::status_mark(state, detail).is_some() {
        status::paint_status_mark(&painter, middle, state, detail);
    } else if let Some(fraction) = position_fraction(view) {
        paint_position(&painter, middle, fraction, state, density);
    }

    intent_for(&response, ControlIntent::FocusRequested)
}

/// Allocates the row this control occupies, in the role it was asked in.
///
/// Width is whatever the composing view gives it: a control that chose its own
/// width would be deciding a layout that belongs to the surface around it.
/// Height comes from the density policy, which is the only place an authored
/// size lives.
pub(super) fn allocate_row(
    ui: &mut Ui,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> Response {
    let height = row_height(role, density);
    let width = ui.available_width();
    ui.allocate_response(eframe::egui::vec2(width, height), Sense::click())
}

/// The authored height of one row in each role.
///
/// Exhaustive over the role vocabulary with no wildcard, so a fifth role is a
/// compile error here rather than a row silently sized like a listed one.
pub(super) fn row_height(role: PresentationRole, density: &ViewportDensityPolicy) -> f32 {
    match role {
        // Stacked full-width rows on a main surface, and the modal list, both
        // ride the content rhythm.
        PresentationRole::ListedRow | PresentationRole::ModalEntry => {
            density.rhythm().row_height_px
        }
        // A side-region entry carries the utility control's authored height.
        PresentationRole::PanelEntry => density.utility_control().height_px,
        // A mixer column cell is a row of the content rhythm inside a column
        // whose width the strip owns.
        PresentationRole::VerticalStrip => density.rhythm().row_height_px,
    }
}

/// Paints the chrome every listed row shares: its selection fill, its
/// separating hairline, the frame its state declares, and the focus cursor.
///
/// Each of the four is a Phase 4a primitive. None of them is redrawn here.
pub(super) fn paint_row_chrome(painter: &Painter, row: Rect, state: ComponentState) {
    status::paint_row_fill(painter, row, state);
    rules::hairline(
        painter,
        rules::RuleSpan::Horizontal {
            y_px: row.max.y,
            from_x_px: row.min.x,
            to_x_px: row.max.x,
        },
    );
    focus::focus_frame(painter, row, state);
    focus::cursor(painter, row, state);
}

/// Paints the row's label after the reserved cursor column and returns the x
/// its text ends at.
pub(super) fn paint_row_label(
    painter: &Painter,
    row: Rect,
    label: &str,
    state: ComponentState,
) -> f32 {
    let galley = text::layout(
        painter,
        label,
        TypeStyle::LabelControl,
        SemanticColor::TextPrimary,
        state,
    );
    let size = galley.size();
    let left = row.min.x + focus::LABEL_START_X_PX;
    let position = pos2(left, row.center().y - size.y / 2.0);
    painter.galley(
        position,
        galley,
        text::resolved_color(SemanticColor::TextPrimary, state).resolve(),
    );
    left + size.x
}

/// Paints a finished value string right-aligned inside the row and returns the
/// x its text begins at.
pub(super) fn paint_row_value(
    painter: &Painter,
    row: Rect,
    rendered: &str,
    state: ComponentState,
) -> f32 {
    let right = row.max.x - SpacingStep::S12.resolve();
    value::paint_value(painter, right, row.center().y, rendered, state);
    let galley = text::layout(
        painter,
        rendered,
        TypeStyle::CodeValue,
        value::value_color(state),
        state,
    );
    right - galley.size().x
}

/// The region between the label and the value.
///
/// Empty when the two meet, so a narrow row drops its indicator rather than
/// painting one over its own text.
pub(super) fn middle_region(row: Rect, label_right: f32, value_left: f32) -> Rect {
    let left = label_right + SpacingStep::S16.resolve();
    let right = value_left - SpacingStep::S16.resolve();
    Rect::from_min_max(pos2(left.min(right), row.min.y), pos2(right, row.max.y))
}

/// The lifecycle detail this row reports, drawn entirely from its view data.
///
/// A typed failure outranks a progress phase: a row that failed is no longer
/// preparing, and reporting both would leave the operator to decide which is
/// current.
pub(super) fn status_detail(view: &SemanticControlViewModel) -> StatusDetail<'_> {
    if let Some(error) = view.error() {
        return StatusDetail::Failure(error.label());
    }
    match view.status().map(|status| status.kind()) {
        Some(crate::control::EngineSelectionStatusKind::Preparing) => {
            StatusDetail::Progress(LoadingPhase::Preparing)
        }
        Some(crate::control::EngineSelectionStatusKind::Activating) => {
            StatusDetail::Progress(LoadingPhase::Activating)
        }
        Some(crate::control::EngineSelectionStatusKind::Ready)
        | Some(crate::control::EngineSelectionStatusKind::Failed)
        | None => StatusDetail::None,
    }
}

/// Renders one semantic value as the finished text a row displays.
///
/// This does not invent a format. Continuous values read to
/// [`VALUE_DECIMAL_PLACES`], a toggle reads `ON` or `OFF`, an asset reads its
/// locator, and every already-finished string is passed through — which is the
/// presentation the shell ships today.
///
/// Exhaustive over the value union with no wildcard, so a new value kind is a
/// compile error here rather than a row that silently renders nothing.
pub(super) fn value_text(value: &SemanticControlValue) -> String {
    match value {
        SemanticControlValue::Scalar(scalar) => format!("{scalar:.VALUE_DECIMAL_PLACES$}"),
        SemanticControlValue::Parameter(parameter) => parameter_text(parameter),
        SemanticControlValue::Asset(reference) => reference.locator().to_owned(),
        SemanticControlValue::Identity(rendered) | SemanticControlValue::Summary(rendered) => {
            rendered.clone()
        }
    }
}

/// The authored word a binary parameter reads as when it is on.
pub(super) const TOGGLE_ON_WORD: &str = "ON";

/// The authored word a binary parameter reads as when it is off.
pub(super) const TOGGLE_OFF_WORD: &str = "OFF";

/// Renders one typed parameter value as finished text.
pub(super) fn parameter_text(parameter: &ParameterValue) -> String {
    match parameter {
        ParameterValue::Continuous(scalar) => format!("{scalar:.VALUE_DECIMAL_PLACES$}"),
        ParameterValue::Stepped(step) => step.to_string(),
        ParameterValue::Choice(choice) => choice.clone(),
        ParameterValue::Toggle(on) => toggle_word(*on).to_owned(),
    }
}

/// The authored word for one binary position.
pub(super) const fn toggle_word(on: bool) -> &'static str {
    if on {
        TOGGLE_ON_WORD
    } else {
        TOGGLE_OFF_WORD
    }
}

/// Where this row's value sits inside its declared bounds, when it has both.
///
/// `None` when the view data carries no range or carries a value that is not a
/// number. A missing range renders without an indicator rather than against
/// assumed bounds — a row drawn half-full because nobody said how full it is
/// would be a claim the projection never made.
pub(super) fn position_fraction(view: &SemanticControlViewModel) -> Option<f32> {
    let range = view.numeric_range()?;
    let magnitude = numeric_magnitude(view.value())?;
    let span = range.maximum() - range.minimum();
    if span <= 0.0 {
        return None;
    }
    Some((((magnitude - range.minimum()) / span) as f32).clamp(0.0, 1.0))
}

/// The number behind a value, when the value carries one.
fn numeric_magnitude(value: &SemanticControlValue) -> Option<f64> {
    match value {
        SemanticControlValue::Scalar(scalar) => Some(*scalar),
        SemanticControlValue::Parameter(ParameterValue::Continuous(scalar)) => Some(*scalar),
        SemanticControlValue::Parameter(ParameterValue::Stepped(step)) => Some(*step as f64),
        SemanticControlValue::Parameter(ParameterValue::Choice(_))
        | SemanticControlValue::Parameter(ParameterValue::Toggle(_))
        | SemanticControlValue::Asset(_)
        | SemanticControlValue::Identity(_)
        | SemanticControlValue::Summary(_) => None,
    }
}

/// Paints the position indicator: a track, its fill, and a handle at the fill
/// edge.
///
/// The handle is what makes the indicator read without color — a fill and its
/// track differ in tone, but the handle differs in shape, so the position
/// survives a viewer who cannot separate the two tones.
fn paint_position(
    painter: &Painter,
    region: Rect,
    fraction: f32,
    state: ComponentState,
    density: &ViewportDensityPolicy,
) {
    if region.width() <= 0.0 {
        return;
    }
    let thickness = density.utility_control().bar_thickness_px;
    let corner = Radius::Small.resolve();
    let centre = region.center().y;
    let track = Rect::from_min_max(
        pos2(region.min.x, centre - thickness / 2.0),
        pos2(region.max.x, centre + thickness / 2.0),
    );
    painter.rect_filled(track, corner, SemanticColor::BgElevated.resolve());

    let filled_right = track.min.x + track.width() * fraction;
    let filled = Rect::from_min_max(track.min, pos2(filled_right, track.max.y));
    let accent = text::resolved_color(value::value_color(state), state);
    painter.rect_filled(filled, corner, accent.resolve());

    let half_handle = crate::shell::visual::token::KEYLINE_EMPHASIS_PX / 2.0;
    let handle_half_height = SpacingStep::S16.resolve() / 2.0;
    let handle = Rect::from_min_max(
        pos2(filled_right - half_handle, centre - handle_half_height),
        pos2(filled_right + half_handle, centre + handle_half_height),
    );
    painter.rect_filled(handle, corner, accent.resolve());
}

/// Turns an addressed row into the request it makes.
///
/// A control reports what was asked for and never whether it was allowed, so
/// this carries no notion of enablement, focus order, or validity.
pub(super) fn intent_for(response: &Response, when_addressed: ControlIntent) -> ControlIntent {
    if response.clicked() {
        when_addressed
    } else {
        ControlIntent::None
    }
}

#[cfg(test)]
pub(super) mod probe {
    //! The shared render probe.
    //!
    //! Every control in this work package is checked the same way: paint it
    //! through a real `egui` context with the authored faces installed, then
    //! read back the text runs and shape count that actually reached the
    //! output. Nothing here inspects a color, so a control that announced a
    //! state with color alone fails.

    use eframe::egui::epaint::Shape;
    use eframe::egui::{Margin, Pos2, RawInput, Rect, Vec2};

    use super::*;
    use crate::control::{SemanticGraphicalViewModel, TopLevelContext};
    use crate::shell::visual::controls::ComponentControl;
    use crate::shell::visual::typeface::AuthoredTypeface;

    /// What one paint pass put on screen, with the color channel discarded.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub(in crate::shell::visual::controls) struct Painted {
        /// Every text run, in paint order.
        pub texts: Vec<String>,
        /// How many non-text shapes were emitted.
        pub shapes: usize,
    }

    impl Painted {
        /// Whether any run carries `needle`.
        pub fn says(&self, needle: &str) -> bool {
            self.texts.iter().any(|run| run.contains(needle))
        }

        /// Everything that reads without color, as one comparable value.
        pub fn colorless_signature(&self) -> (Vec<String>, usize) {
            (self.texts.clone(), self.shapes)
        }
    }

    /// A real projected control of the given context's main surface.
    ///
    /// Built through the production projection rather than assembled here: the
    /// view model's fields are private precisely so no surface can invent one,
    /// and a control test that fabricated its input would not be testing the
    /// contract the control actually receives.
    pub(in crate::shell::visual::controls) fn projected_control(
        context: TopLevelContext,
    ) -> SemanticControlViewModel {
        let model = SemanticGraphicalViewModel::fixture(1, "probe-state-hash", context);
        model
            .surfaces()
            .iter()
            .flat_map(|surface| surface.controls().iter())
            .next()
            .expect("the projection carries at least one control")
            .clone()
    }

    /// A real projected read-only surface summary control.
    pub(in crate::shell::visual::controls) fn projected_summary() -> SemanticControlViewModel {
        let model =
            SemanticGraphicalViewModel::fixture(1, "probe-state-hash", TopLevelContext::Patch);
        model
            .surfaces()
            .iter()
            .flat_map(|surface| surface.controls().iter())
            .find(|control| matches!(control.value(), SemanticControlValue::Summary(_)))
            .expect("the projection carries a surface summary")
            .clone()
    }

    /// Paints one control once and returns what reached the output.
    pub(in crate::shell::visual::controls) fn paint(
        control: ComponentControl,
        view: &SemanticControlViewModel,
        state: ComponentState,
        role: PresentationRole,
        density: ViewportDensityPolicy,
    ) -> Painted {
        let context = eframe::egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the vendored faces are present")
                .font_definitions(),
        );
        let viewport = density.authored_viewport();
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(viewport.width_px, viewport.height_px),
            )),
            ..RawInput::default()
        };
        // One warm-up pass so the requested faces are resident; a first-frame
        // miss would be a harness artifact rather than a rendering failure.
        run(&context, &input, control, view, state, role, density);
        let output = run(&context, &input, control, view, state, role, density);
        let mut painted = Painted::default();
        for clipped in &output {
            collect(&clipped.shape, &mut painted);
        }
        painted
    }

    fn run(
        context: &eframe::egui::Context,
        input: &RawInput,
        control: ComponentControl,
        view: &SemanticControlViewModel,
        state: ComponentState,
        role: PresentationRole,
        density: ViewportDensityPolicy,
    ) -> Vec<eframe::egui::epaint::ClippedShape> {
        let output = context.run(input.clone(), |context| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::new().inner_margin(Margin::ZERO))
                .show(context, |ui| {
                    let intent = control.render(ui, view, state, role, &density);
                    // Painting alone never asks for anything: no pointer press
                    // reached this pass, so a control reporting a request here
                    // would be reporting one nobody made.
                    assert_eq!(
                        intent,
                        ControlIntent::None,
                        "{} asked for something without being addressed",
                        control.canonical_name()
                    );
                });
        });
        output.shapes
    }

    fn collect(shape: &Shape, painted: &mut Painted) {
        match shape {
            Shape::Text(run) => painted.texts.push(run.galley.text().to_owned()),
            Shape::Vec(shapes) => {
                for nested in shapes {
                    collect(nested, painted);
                }
            }
            other => {
                let _ = other;
                painted.shapes += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::probe::{paint, projected_control, projected_summary, Painted};
    use super::*;
    use crate::control::TopLevelContext;
    use crate::shell::visual::controls::ComponentControl;
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::state::NonColorSignal;
    use crate::synth::{AssetKind, AssetReference};

    const CONTROL: ComponentControl = ComponentControl::ParameterRow;

    fn painted(state: ComponentState, density: ViewportDensityPolicy) -> Painted {
        paint(
            CONTROL,
            &projected_control(TopLevelContext::Patch),
            state,
            PresentationRole::ListedRow,
            density,
        )
    }

    /// Every applicable state announces itself without color.
    ///
    /// A state whose signal is a word must paint that word. A state whose
    /// signal is shape must differ from rest in text or in shape count —
    /// removing its treatment collapses it onto rest and fails here.
    #[test]
    fn every_applicable_state_renders_evidence_that_is_not_a_color() {
        let resting = painted(ComponentState::Resting, ViewportDensityPolicy::Desktop);
        for state in CONTROL.applicable_states() {
            let shown = painted(*state, ViewportDensityPolicy::Desktop);
            match state.appearance().signal {
                NonColorSignal::Word(word) => assert!(
                    shown.says(word),
                    "{} does not paint its authored word",
                    state.canonical_name()
                ),
                NonColorSignal::ProgressWord => assert!(
                    shown.says(LoadingPhase::Preparing.word()),
                    "{} does not paint a progress word",
                    state.canonical_name()
                ),
                NonColorSignal::TypedFailure => assert!(
                    shown.says(status::GENERIC_FAILURE_WORD)
                        || shown.texts.len() > resting.texts.len(),
                    "{} does not paint failure text",
                    state.canonical_name()
                ),
                NonColorSignal::Shape => {
                    if *state == ComponentState::Resting {
                        continue;
                    }
                    assert_ne!(
                        shown.colorless_signature(),
                        resting.colorless_signature(),
                        "{} is distinguishable from Resting by color alone",
                        state.canonical_name()
                    );
                }
            }
        }
    }

    /// Focus and adjustment both paint the authored cursor, which is what makes
    /// them legible without their accents.
    #[test]
    fn focus_and_adjustment_paint_the_authored_cursor() {
        for state in [ComponentState::Focused, ComponentState::Adjusting] {
            assert!(
                painted(state, ViewportDensityPolicy::Desktop).says(focus::CURSOR_GLYPH),
                "{} does not paint the cursor",
                state.canonical_name()
            );
        }
    }

    /// Both authored viewports render the same row.
    ///
    /// The text is identical and only the resolved sizing differs, so a
    /// resolution-specific branch would show up as a different set of runs.
    #[test]
    fn both_viewports_paint_the_same_content() {
        for state in CONTROL.applicable_states() {
            let desktop = painted(*state, ViewportDensityPolicy::Desktop);
            let deck = painted(*state, ViewportDensityPolicy::SteamDeck);
            assert_eq!(
                desktop.texts,
                deck.texts,
                "{} paints different content at the two viewports",
                state.canonical_name()
            );
        }
    }

    /// Every policy resolves a height, and every one of them is at or above the
    /// authored minimum interactive target.
    #[test]
    fn every_role_and_policy_resolves_an_authored_row_height() {
        for policy in ALL_DENSITY_POLICIES {
            for role in crate::shell::visual::controls::ALL_PRESENTATION_ROLES {
                let height = row_height(role, &policy);
                assert!(
                    height >= crate::shell::visual::token::MIN_INTERACTIVE_TARGET_PX,
                    "{} in {} resolves {height} px",
                    role.canonical_name(),
                    policy.canonical_name()
                );
            }
        }
    }

    /// The row paints nothing it was not handed.
    ///
    /// Every run is either the view model's own text or a word the state
    /// vocabulary declares. A run from neither source is a value the control
    /// invented.
    #[test]
    fn the_row_paints_no_text_that_did_not_arrive_in_its_view_model() {
        let view = projected_control(TopLevelContext::Patch);
        let rendered = value_text(view.value());
        for state in CONTROL.applicable_states() {
            let shown = paint(
                CONTROL,
                &view,
                *state,
                PresentationRole::ListedRow,
                ViewportDensityPolicy::Desktop,
            );
            for run in &shown.texts {
                let from_view_model = view.label().contains(run.as_str())
                    || rendered.contains(run.as_str())
                    || run.is_empty();
                let from_vocabulary = run == focus::CURSOR_GLYPH
                    || run == UNAVAILABLE_MARK
                    || run == status::GENERIC_FAILURE_WORD
                    || crate::shell::visual::state::LOADING_PROGRESS_WORDS.contains(&run.as_str())
                    || matches!(
                        state.appearance().signal,
                        NonColorSignal::Word(word) if word == run
                    );
                assert!(
                    from_view_model || from_vocabulary,
                    "{} painted {run:?}, which is neither view data nor authored vocabulary",
                    state.canonical_name()
                );
            }
        }
    }

    /// A read-only surface summary is a parameter row too, and it renders.
    #[test]
    fn a_surface_summary_renders_as_a_row() {
        let summary = projected_summary();
        let shown = paint(
            CONTROL,
            &summary,
            ComponentState::Resting,
            PresentationRole::PanelEntry,
            ViewportDensityPolicy::Desktop,
        );
        assert!(
            shown.says(&value_text(summary.value())),
            "the summary's own text did not reach the row"
        );
    }

    #[test]
    fn a_continuous_value_reads_to_the_authored_precision() {
        assert_eq!(
            value_text(&SemanticControlValue::Scalar(0.5)),
            format!("{:.VALUE_DECIMAL_PLACES$}", 0.5)
        );
        assert_eq!(
            value_text(&SemanticControlValue::Parameter(
                ParameterValue::Continuous(0.25)
            )),
            format!("{:.VALUE_DECIMAL_PLACES$}", 0.25)
        );
    }

    /// Every value kind renders as finished text, and none of them renders as
    /// nothing.
    #[test]
    fn every_value_kind_renders_as_finished_text() {
        let values = [
            SemanticControlValue::Scalar(1.0),
            SemanticControlValue::Parameter(ParameterValue::Stepped(7)),
            SemanticControlValue::Parameter(ParameterValue::Choice("SINE".to_owned())),
            SemanticControlValue::Parameter(ParameterValue::Toggle(true)),
            SemanticControlValue::Asset(
                AssetReference::new(AssetKind::Sample, "pad-cloud-C3.wav").expect("locator"),
            ),
            SemanticControlValue::Identity("PLAITS".to_owned()),
            SemanticControlValue::Summary("Read-only".to_owned()),
        ];
        for value in values {
            assert!(
                !value_text(&value).is_empty(),
                "{value:?} rendered as nothing"
            );
        }
        assert_eq!(
            value_text(&SemanticControlValue::Parameter(ParameterValue::Toggle(
                true
            ))),
            TOGGLE_ON_WORD
        );
        assert_eq!(
            value_text(&SemanticControlValue::Parameter(ParameterValue::Toggle(
                false
            ))),
            TOGGLE_OFF_WORD
        );
    }

    /// A row with no declared range renders no position indicator rather than
    /// assuming bounds.
    #[test]
    fn a_row_without_a_declared_range_carries_no_position() {
        let view = projected_control(TopLevelContext::Patch);
        assert!(
            view.numeric_range().is_none(),
            "the probe control is expected to carry no range"
        );
        assert_eq!(position_fraction(&view), None);
    }

    /// The middle region collapses rather than overlapping the row's own text.
    #[test]
    fn the_middle_region_never_runs_backwards() {
        let row = Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 52.0));
        let region = middle_region(row, 190.0, 20.0);
        assert!(
            region.width() <= 0.0,
            "a crowded row produced a middle region {} wide",
            region.width()
        );
    }

    /// Every pair that selects one of this work package's four controls
    /// actually reaches its module and paints.
    ///
    /// This is the check that a control is wired rather than merely written. A
    /// module whose tests pass but which nothing can ask for is dead code, and
    /// so is one still resolving to the do-nothing stub: both would leave the
    /// per-control assertions above green while the product painted nothing.
    #[test]
    fn every_pair_that_selects_one_of_these_controls_renders_through_it() {
        use crate::shell::visual::controls::{
            control_for, ALL_PRESENTATION_ROLES, ALL_SEMANTIC_CONTROL_KINDS,
        };

        let mine = [
            ComponentControl::ParameterRow,
            ComponentControl::ChoiceRow,
            ComponentControl::Toggle,
            ComponentControl::BrowserRow,
        ];
        let view = projected_control(TopLevelContext::Patch);
        let mut exercised = std::collections::HashSet::new();
        for kind in ALL_SEMANTIC_CONTROL_KINDS {
            for role in ALL_PRESENTATION_ROLES {
                let Some(control) = control_for(kind, role).control() else {
                    continue;
                };
                if !mine.contains(&control) {
                    continue;
                }
                let shown = paint(
                    control,
                    &view,
                    ComponentState::Resting,
                    role,
                    ViewportDensityPolicy::Desktop,
                );
                assert!(
                    !shown.texts.is_empty(),
                    "{} selected in {} painted nothing; it is still the stub",
                    control.canonical_name(),
                    role.canonical_name()
                );
                exercised.insert(control);
            }
        }
        let missing: Vec<&str> = mine
            .into_iter()
            .filter(|control| !exercised.contains(control))
            .map(ComponentControl::canonical_name)
            .collect();
        assert!(
            missing.is_empty(),
            "no kind and role pair asks for: {}",
            missing.join(", ")
        );
    }

    /// Painting alone asks for nothing; being addressed asks to take focus.
    #[test]
    fn an_unaddressed_row_asks_for_nothing() {
        for state in CONTROL.applicable_states() {
            // `paint` asserts this on every pass; naming it here records it as
            // the contract rather than a harness detail.
            let _ = painted(*state, ViewportDensityPolicy::Desktop);
        }
    }
}
