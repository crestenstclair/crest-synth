//! The section — a titled group of entries inside one surface.
//!
//! A section is the unit both the PATCH main surface and the Utility/Inspector
//! are built from: a header band carrying the surface's own title and, when the
//! projection reports one, the focused entry, then the surface's controls in the
//! order the projection supplies them.
//!
//! # Figma source
//!
//! - `SCREEN · Patch Strip · 1920×1080` → `Main Content` (`36:20`) and its
//!   `Section Header` (`36:21`). The header is a title on the left, a
//!   transparent spacer, and a right-aligned focus annotation; the rows follow
//!   beneath it on the content rhythm.
//! - `SCREEN · Mixer · 1920×1080` → `Mixer Legend` (`42:21`), the same header
//!   anatomy on the other main surface.
//!
//! The spacer between the two runs carries no fill in either frame, so nothing
//! is painted between them. It is layout, not a leader.
//!
//! # What this module owns
//!
//! Nothing. Every extent resolves through [`ViewportDensityPolicy`] and every
//! color and type style through the authored token vocabulary. The section
//! reads the immutable projection it is handed, arranges, and returns
//! aggregated [`CompositionIntent`].
//!
//! # What the specimen gave up
//!
//! The authored header band is 28 px tall above a 14 px gap. Neither is an
//! authored token and reproducing them would mean restating two numbers, so the
//! band is derived: one authored title line plus the authored row gap, which is
//! the same 14 px the design file leaves between the header and the first row
//! (`row_pitch − row_height` = 66 − 52 on desktop). That lands at 30 px against
//! the specimen's 42 px from header top to first row. The divergence is raised
//! in this work package's handoff rather than closed with a constant.
//!
//! The authored title is 12 px SemiBold in `color/text/muted`. The closest
//! authored style is `Label/Control` — 12 px, Medium — so the size and the color
//! role are exact and the weight is one step light. Also raised.
//!
//! # The role is the section's to supply
//!
//! A section declares the [`PresentationRole`] its entries are asked in and
//! never lets an entry decide what it is. A section at
//! [`PresentationRole::ListedRow`] is a stack of full-width rows; one at
//! [`PresentationRole::PanelEntry`] is the side region's entry list; one at
//! [`PresentationRole::VerticalStrip`] is a single mixer track column, titled
//! with the track it represents.
//!
//! # The unavailable marker lives here
//!
//! [`mark_unavailable`] is the shared way a composition says "the design shows
//! this structure and the projection carries nothing behind it" (C-003). It is
//! the only thing any composition in this work package paints where view data
//! did not arrive; a blank region and a plausible default are both forbidden,
//! because neither is distinguishable from a real empty value.

use eframe::egui::{pos2, Rect, Sense, Ui, UiBuilder, Vec2};

use super::{patch_strip_row, CompositionIntent};
use crate::control::{
    GraphicalShellProjection, InteractionMode, SemanticControlViewModel, SemanticSurfaceViewModel,
    SurfaceId,
};
use crate::shell::visual::controls::parameter_row::UNAVAILABLE_MARK;
use crate::shell::visual::controls::PresentationRole;
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::hint::HINT_SEPARATOR;
use crate::shell::visual::primitives::{rules, text};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};

/// The static word that introduces the header's focus annotation.
///
/// The design file writes `FOCUS / INSTRUMENT`; the separator here is the
/// authored [`HINT_SEPARATOR`] rather than a second one invented for this
/// header. The word itself is a static label, not a value: what follows it is
/// the projected label of the focused entry, and the whole annotation is
/// omitted when the surface reports no focused entry.
pub(super) const FOCUS_ANNOTATION_LABEL: &str = "FOCUS";

/// Names the body of a section the projection supplied no entry for.
///
/// The title says which surface; this says which part of it is missing, so the
/// two runs do not read as the same word twice.
pub(super) const ENTRIES_LABEL: &str = "ENTRIES";

/// One titled group of entries, and the role its entries are asked in.
pub(super) struct SectionGroup<'a> {
    /// The projected title of the surface this section arranges.
    pub title: &'a str,
    /// The projected label of the focused entry, when the surface has one.
    pub annotation: Option<&'a str>,
    /// The entries, in the order the projection supplies them.
    pub controls: &'a [SemanticControlViewModel],
    /// The projected interaction mode, which separates focus from adjustment.
    pub mode: InteractionMode,
    /// The role every entry is asked in. The section supplies it.
    pub role: PresentationRole,
}

/// Arranges the active main surface as one section and returns what its entries
/// asked for.
pub fn render(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let model = projection.semantic_model();
    let title = projection.workspace().main_label();
    let surface = model.surface(SurfaceId::main_for(model.context()));
    let group = SectionGroup {
        title,
        annotation: surface.and_then(focused_label),
        controls: surface.map_or(&[], SemanticSurfaceViewModel::controls),
        mode: model.interaction_mode(),
        role: PresentationRole::ListedRow,
    };
    render_group(ui, group, density)
}

/// Arranges one titled group: the header band, then the entries.
pub(super) fn render_group(
    ui: &mut Ui,
    group: SectionGroup<'_>,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    inset_scope(ui, density, |ui| {
        render_header(ui, group.title, group.annotation, density);
        ui.add_space(entry_gap_px(group.role, density));
        render_entries(
            ui,
            group.controls,
            group.mode,
            group.role,
            density,
            ENTRIES_LABEL,
        )
    })
}

/// Runs `add_contents` inside the authored content inset.
///
/// The inset belongs to the group rather than to the band around it: the design
/// file places the section header and every row at the authored inset *inside*
/// the 1500 px main surface, not inside the viewport.
pub(super) fn inset_scope<R>(
    ui: &mut Ui,
    density: &ViewportDensityPolicy,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let inset = density.rhythm().inset_px;
    let outer = ui.available_rect_before_wrap();
    let left = outer.min.x + inset;
    let right = (outer.max.x - inset).max(left);
    let content = Rect::from_min_max(pos2(left, outer.min.y), pos2(right, outer.max.y));
    ui.scope_builder(UiBuilder::new().max_rect(content), add_contents)
        .inner
}

/// Arranges every visible entry in the declared role, and marks the group
/// unavailable when the projection supplied none.
///
/// An entry the projection reports as not visible is not painted: visibility is
/// projected state, and painting a hidden row would show the operator something
/// the reducer says is not there. A group with no visible entry paints its
/// declared unavailable marker rather than an invented row.
pub(super) fn render_entries(
    ui: &mut Ui,
    controls: &[SemanticControlViewModel],
    mode: InteractionMode,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
    empty_structure: &str,
) -> CompositionIntent {
    let mut intent = CompositionIntent::none();
    let mut arranged = 0_usize;
    for view in controls.iter().filter(|control| control.visible()) {
        if arranged > 0 {
            ui.add_space(entry_gap_px(role, density));
        }
        intent.absorb(patch_strip_row::render_row(ui, view, mode, role, density));
        arranged += 1;
    }
    if arranged == 0 {
        mark_unavailable(ui, empty_structure, density);
    }
    intent
}

/// The gap between two consecutive entries in one role.
///
/// Exhaustive over the role vocabulary with no wildcard, so a fifth role is a
/// compile error here rather than an entry silently spaced like a listed row.
pub(super) fn entry_gap_px(role: PresentationRole, density: &ViewportDensityPolicy) -> f32 {
    match role {
        // Stacked rows, the modal list, and a mixer column's cells all ride the
        // content rhythm, so the gap is the rhythm's own.
        PresentationRole::ListedRow
        | PresentationRole::ModalEntry
        | PresentationRole::VerticalStrip => density.rhythm().row_gap_px(),
        // A side-region entry rides the utility control's authored pitch.
        PresentationRole::PanelEntry => {
            density.utility_control().pitch_px - density.utility_control().height_px
        }
    }
}

/// The height of the header band.
pub(super) fn header_height_px(density: &ViewportDensityPolicy) -> f32 {
    TypeStyle::LabelControl.metrics().line_height_px + density.rhythm().row_gap_px()
}

/// Paints the header band: title left, focus annotation right.
fn render_header(
    ui: &mut Ui,
    title: &str,
    annotation: Option<&str>,
    density: &ViewportDensityPolicy,
) {
    let response = ui.allocate_response(
        Vec2::new(ui.available_width(), header_height_px(density)),
        Sense::hover(),
    );
    let painter = ui.painter_at(response.rect);
    let band = response.rect;
    let state = ComponentState::Resting;

    let heading = text::layout(
        &painter,
        title,
        TypeStyle::LabelControl,
        SemanticColor::TextMuted,
        state,
    );
    let heading_size = heading.size();
    painter.galley(
        pos2(band.min.x, band.center().y - heading_size.y / 2.0),
        heading,
        text::resolved_color(SemanticColor::TextMuted, state).resolve(),
    );

    let Some(annotation) = annotation else {
        return;
    };
    let run = text::layout(
        &painter,
        format!("{FOCUS_ANNOTATION_LABEL}{HINT_SEPARATOR}{annotation}"),
        TypeStyle::InstructionHint,
        SemanticColor::AccentFocus,
        state,
    );
    let size = run.size();
    painter.galley(
        pos2(band.max.x - size.x, band.center().y - size.y / 2.0),
        run,
        text::resolved_color(SemanticColor::AccentFocus, state).resolve(),
    );
}

/// Marks one designed structure the projection does not drive.
///
/// C-003, made mechanical. The structure's static name stays on screen — the
/// operator is told *what* is missing, not merely that something is — and the
/// authored [`UNAVAILABLE_MARK`] sits where its value would be.
///
/// The mark is the row vocabulary's, not the status primitive's. `Disabled`
/// declares the word `Locked`, which means present-but-not-editable; a structure
/// the projection never supplied is not locked, it is absent, and the two must
/// not read the same. The state handed to the text primitive is still `Disabled`
/// so the run mutes exactly as every other unavailable value does.
pub(super) fn mark_unavailable(ui: &mut Ui, structure: &str, density: &ViewportDensityPolicy) {
    let response = ui.allocate_response(
        Vec2::new(ui.available_width(), density.rhythm().row_height_px),
        Sense::hover(),
    );
    let painter = ui.painter_at(response.rect);
    let row = response.rect;
    let state = ComponentState::Disabled;
    let muted = text::resolved_color(SemanticColor::TextMuted, state).resolve();

    let label = text::layout(
        &painter,
        structure,
        TypeStyle::LabelControl,
        SemanticColor::TextMuted,
        state,
    );
    let label_size = label.size();
    painter.galley(
        pos2(row.min.x, row.center().y - label_size.y / 2.0),
        label,
        muted,
    );

    let mark = text::layout(
        &painter,
        UNAVAILABLE_MARK,
        TypeStyle::CodeValue,
        SemanticColor::TextMuted,
        state,
    );
    let mark_size = mark.size();
    painter.galley(
        pos2(row.max.x - mark_size.x, row.center().y - mark_size.y / 2.0),
        mark,
        muted,
    );
    rules::hairline(
        &painter,
        rules::RuleSpan::Horizontal {
            y_px: row.max.y,
            from_x_px: row.min.x,
            to_x_px: row.max.x,
        },
    );
}

/// Paints one read-only caption line — a static label or a projected identity
/// that is not itself a control.
///
/// # It wraps, because what it carries is as long as the catalog made it
///
/// A caption's content is frequently a projected identity: a Patch name, a route
/// entry. Those are not bounded by anything this composition controls — the
/// production fixture's own route line is
/// `PATCH 01 · Xylophone 1 (bank 0, program 13, percussion)`.
///
/// Laid out on one forced line and painted through a painter clipped to that
/// line, an overlong caption was **hard-cut mid-word with no ellipsis and no
/// gesture that revealed the rest**. That was a real regression from the shipped
/// shell, whose `chrome_text` was `ui.label` and therefore wrapped, and it is
/// what `WP08`'s T044 — no clipped or overlapping text at either viewport —
/// asserts against.
///
/// So the run is laid out against the width it actually has and allocates the
/// height it actually needs. A caption that fits is untouched and still occupies
/// exactly one authored line; one that does not runs onto a second rather than
/// losing its tail. Wrapping is the right treatment here and truncation is not:
/// unlike a structural band, a side-region entry list has somewhere to go
/// vertically, and a Patch name cut short is a Patch the operator cannot
/// identify.
pub(super) fn caption(ui: &mut Ui, content: impl Into<String>, color: SemanticColor) {
    let state = ComponentState::Resting;
    let width = ui.available_width();
    let mut run = text::text_run(content, TypeStyle::InstructionHint, color, state);
    // `text_run` leaves `max_width` at infinity, which is what every run placed
    // by an explicit painter wants. A caption is the one that does not: it is
    // given a width by the region that contains it, and must fold to it.
    run.wrap.max_width = width;
    let galley = ui.fonts(|fonts| fonts.layout_job(run));
    let size = galley.size();
    let height = size
        .y
        .max(TypeStyle::InstructionHint.metrics().line_height_px);
    let response = ui.allocate_response(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(response.rect);
    let band = response.rect;
    painter.galley(
        pos2(band.min.x, band.center().y - size.y / 2.0),
        galley,
        text::resolved_color(color, state).resolve(),
    );
}

/// The projected label of the surface's focused entry, when it has one.
fn focused_label(surface: &SemanticSurfaceViewModel) -> Option<&str> {
    surface
        .controls()
        .iter()
        .find(|control| control.focused())
        .map(SemanticControlViewModel::label)
}

#[cfg(test)]
pub(super) mod probe {
    //! The shared composition render probe.
    //!
    //! Every composition in this work package is checked the same way: paint it
    //! through a real `egui` context with the authored faces installed, against
    //! a projection built by the production `StateProjector` from a real
    //! `AppState`, then read back the text runs and shape count that actually
    //! reached the output. Constructing a composition without rendering it
    //! proves nothing, so nothing here stops short of a frame.
    //!
    //! The projection is production-built rather than assembled: the view
    //! model's fields are private precisely so no surface can invent one, and a
    //! composition test that fabricated its input would not be testing the
    //! contract the composition actually receives. It is also what makes the
    //! sparse case real — the production PATCH Utility genuinely carries two of
    //! the five entries the design file draws.

    use eframe::egui::epaint::Shape;
    use eframe::egui::{Margin, Pos2, RawInput, Rect, Vec2};

    use super::*;
    use crate::control::{AppEvent, AppState, StateProjector, TopLevelContext};
    use crate::shell::visual::compositions::ShellComposition;
    use crate::shell::visual::typeface::AuthoredTypeface;

    /// What one paint pass put on screen, with the color channel discarded.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub(in crate::shell::visual::compositions) struct Painted {
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

        /// How many runs carry `needle`.
        pub fn count(&self, needle: &str) -> usize {
            self.texts.iter().filter(|run| run.contains(needle)).count()
        }
    }

    /// One installed Patch on the production registries, in the given context.
    pub(in crate::shell::visual::compositions) fn installed_state(
        context: TopLevelContext,
    ) -> AppState {
        let mut state = AppState::new_with_effects(
            crate::adapter::production_instruments::production_capability_registry()
                .expect("the production instrument registry"),
            crate::adapter::production_effects::production_effect_registry()
                .expect("the production effect registry"),
            crate::mixer::global_parameters::GlobalParameters::new(0.0)
                .expect("neutral global parameters"),
        );
        let patch = crate::synth::Patch::new(
            crate::kernel::PatchId::new(1).expect("probe PatchId"),
            "Probe".to_owned(),
            crate::adapter::braids_capability::BraidsCapability::new()
                .expect("the Braids capability")
                .default_config()
                .expect("its default config"),
            crate::kernel::MidiChannel::new(3).expect("probe MIDI channel"),
            crate::mixer::patch_output::PatchOutput::to_track(
                crate::mixer::mixer_track_id::MixerTrackId::new(3).expect("probe track"),
            ),
        );
        state
            .apply(AppEvent::InstallPatches(vec![patch]))
            .expect("the probe Patch installs");
        state
            .apply(AppEvent::SelectContext(context))
            .expect("the probe context selects");
        state
    }

    /// The production graphical shell projection for one context.
    pub(in crate::shell::visual::compositions) fn projection(
        context: TopLevelContext,
    ) -> GraphicalShellProjection {
        let state = installed_state(context);
        let (_, _, _, shell, _) = StateProjector::new()
            .project_with_shell(&state)
            .expect("the production projector derives a coherent shell");
        shell
    }

    /// Paints one declared composition once and returns what reached the output.
    pub(in crate::shell::visual::compositions) fn paint(
        composition: ShellComposition,
        projection: &GraphicalShellProjection,
        density: ViewportDensityPolicy,
    ) -> Painted {
        paint_with(density, |ui| composition.render(ui, projection, &density))
    }

    /// Paints whatever `add_contents` arranges and returns what reached the
    /// output.
    ///
    /// The aggregated intent is asserted empty on every pass: no pointer press
    /// reached this frame, so a composition reporting a request here would be
    /// reporting one nobody made.
    pub(in crate::shell::visual::compositions) fn paint_with(
        density: ViewportDensityPolicy,
        mut add_contents: impl FnMut(&mut Ui) -> CompositionIntent,
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
        run(&context, &input, &mut add_contents);
        let output = run(&context, &input, &mut add_contents);
        let mut painted = Painted::default();
        for clipped in &output {
            collect(&clipped.shape, &mut painted);
        }
        painted
    }

    fn run(
        context: &eframe::egui::Context,
        input: &RawInput,
        add_contents: &mut impl FnMut(&mut Ui) -> CompositionIntent,
    ) -> Vec<eframe::egui::epaint::ClippedShape> {
        let output = context.run(input.clone(), |context| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::new().inner_margin(Margin::ZERO))
                .show(context, |ui| {
                    let intent = add_contents(ui);
                    assert!(
                        intent.is_empty(),
                        "a composition asked for something without being addressed"
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

    /// Every source in this work package, and the part of each that ships.
    ///
    /// The boundary scans below read only what precedes the first test module:
    /// the probe above deliberately builds an `AppState` to make a real
    /// projection, and a whole-file scan would find that and call it a leak.
    pub(in crate::shell::visual::compositions) fn production_sources() -> Vec<(String, String)> {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell/visual/compositions");
        let mut owned = [
            "patch_strip_row.rs",
            "section.rs",
            "utility_inspector_panel.rs",
        ]
        .into_iter()
        .map(|name| {
            let path = directory.join(name);
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("{} is missing", path.display()));
            let marker = format!("#[cfg({})]", "test");
            let production = match source.find(&marker) {
                Some(offset) => source[..offset].to_owned(),
                None => source,
            };
            (name.to_owned(), production)
        })
        .collect::<Vec<_>>();
        owned.sort();
        owned
    }
}

#[cfg(test)]
mod tests {
    use super::probe::{paint, paint_with, production_sources, projection, Painted};
    use super::*;
    use crate::control::TopLevelContext;
    use crate::shell::visual::compositions::ShellComposition;
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;

    const COMPOSITION: ShellComposition = ShellComposition::Section;

    fn painted(context: TopLevelContext, density: ViewportDensityPolicy) -> Painted {
        paint(COMPOSITION, &projection(context), density)
    }

    /// The section is wired: asking the family for it reaches this module and
    /// paints. A composition whose own tests pass but which nothing can ask for
    /// is dead code, and so is one still resolving to the shared stub.
    #[test]
    fn the_declared_section_composition_renders_through_this_module() {
        let shown = painted(TopLevelContext::Patch, ViewportDensityPolicy::Desktop);
        assert!(
            !shown.texts.is_empty(),
            "the section painted nothing; it is still the stub"
        );
    }

    /// The header carries the projected surface title, and the body carries
    /// every visible projected entry.
    #[test]
    fn the_section_paints_its_projected_title_and_every_visible_entry() {
        let projection = projection(TopLevelContext::Patch);
        let shown = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);
        let model = projection.semantic_model();
        assert!(
            shown.says(projection.workspace().main_label()),
            "the projected surface title did not reach the header"
        );
        let surface = model
            .surface(crate::control::SurfaceId::main_for(model.context()))
            .expect("the PATCH main surface projects");
        for control in surface.controls().iter().filter(|c| c.visible()) {
            assert!(
                shown.says(control.label()),
                "{:?} did not reach the section",
                control.label()
            );
        }
    }

    /// The focus annotation is projected, and it is omitted rather than
    /// invented when the surface reports no focused entry.
    #[test]
    fn the_focus_annotation_is_projected_and_omitted_when_nothing_is_focused() {
        let projection = projection(TopLevelContext::Patch);
        let model = projection.semantic_model();
        let surface = model
            .surface(crate::control::SurfaceId::main_for(model.context()))
            .expect("the PATCH main surface projects");
        let focused = surface
            .controls()
            .iter()
            .find(|control| control.focused())
            .expect("the PATCH main surface carries the focus");
        let with_focus = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);
        assert!(
            with_focus.says(FOCUS_ANNOTATION_LABEL),
            "the focus annotation did not reach the header"
        );
        assert!(with_focus.says(focused.label()));

        let without_focus = paint_with(ViewportDensityPolicy::Desktop, |ui| {
            render_group(
                ui,
                SectionGroup {
                    title: projection.workspace().main_label(),
                    annotation: None,
                    controls: surface.controls(),
                    mode: model.interaction_mode(),
                    role: PresentationRole::ListedRow,
                },
                &ViewportDensityPolicy::Desktop,
            )
        });
        assert!(
            !without_focus.says(FOCUS_ANNOTATION_LABEL),
            "the section painted a focus annotation for a surface that reported none"
        );
    }

    /// A section handed no entries marks its body unavailable. It does not
    /// invent a row, and it does not leave a blank region that reads as an
    /// empty value.
    #[test]
    fn a_section_with_no_entries_marks_its_body_rather_than_inventing_a_row() {
        let title = "EMPTY SURFACE";
        let shown = paint_with(ViewportDensityPolicy::Desktop, |ui| {
            render_group(
                ui,
                SectionGroup {
                    title,
                    annotation: None,
                    controls: &[],
                    mode: crate::control::InteractionMode::Navigate,
                    role: PresentationRole::ListedRow,
                },
                &ViewportDensityPolicy::Desktop,
            )
        });
        // Three runs and nothing else: the title, the name of the missing
        // structure, and the authored marker. Anything more is a row the
        // section invented.
        assert_eq!(
            shown.texts,
            vec![
                title.to_owned(),
                ENTRIES_LABEL.to_owned(),
                UNAVAILABLE_MARK.to_owned()
            ],
            "the empty section painted {:?}",
            shown.texts
        );
    }

    /// A section asked in `VerticalStrip` is one mixer track column: the track
    /// it represents as the title, and that track's own controls beneath it.
    ///
    /// This is the demonstration WP06 needs. What the closed family still has
    /// no composition for is the *strip* — the sixteen columns side by side —
    /// and that gap is recorded in this work package's handoff rather than
    /// closed here by inventing an eighth variant.
    #[test]
    fn a_section_in_the_vertical_strip_role_is_one_mixer_track_column() {
        let projection = projection(TopLevelContext::Mixer);
        let model = projection.semantic_model();
        let surface = model
            .surface(crate::control::SurfaceId::MixerMain)
            .expect("the MIXER main surface projects");
        let track = crate::mixer::mixer_track_id::MixerTrackId::default();
        let column: Vec<crate::control::SemanticControlViewModel> = surface
            .controls()
            .iter()
            .filter(|control| control.path().control_id().as_mixer_track_id() == Some(track))
            .cloned()
            .collect();
        assert!(
            !column.is_empty(),
            "the projection carries no controls for the first track"
        );
        let title = track.to_string();
        let shown = paint_with(ViewportDensityPolicy::Desktop, |ui| {
            render_group(
                ui,
                SectionGroup {
                    title: &title,
                    annotation: None,
                    controls: &column,
                    mode: model.interaction_mode(),
                    role: PresentationRole::VerticalStrip,
                },
                &ViewportDensityPolicy::Desktop,
            )
        });
        assert!(
            shown.says(&title),
            "the column dropped the track it represents"
        );
        for control in &column {
            assert!(
                shown.says(control.label()),
                "{:?} did not reach the column",
                control.label()
            );
        }
    }

    /// The no-invention predicate is not vacuous: a value nothing projected is
    /// rejected.
    ///
    /// Without this, a predicate that answered `true` for everything would leave
    /// every "nothing was invented" assertion in this work package green while a
    /// composition painted whatever it liked.
    #[test]
    fn the_no_invention_predicate_rejects_a_value_nothing_projected() {
        let projection = projection(TopLevelContext::Patch);
        for fabricated in [
            "MASTER VOLUME 200",
            "REVERB 102",
            "SELECT enters multi-select",
        ] {
            assert!(
                !tests_support::from_projection_or_vocabulary(&projection, fabricated),
                "the predicate accepted {fabricated:?}, which nothing projected"
            );
        }
        // And it accepts what the projection actually carries.
        assert!(tests_support::from_projection_or_vocabulary(
            &projection,
            projection.workspace().main_label()
        ));
        assert!(tests_support::from_projection_or_vocabulary(
            &projection,
            UNAVAILABLE_MARK
        ));
    }

    /// Both authored viewports paint the same content.
    ///
    /// Only the resolved sizing differs, so a resolution-specific branch would
    /// show up here as a different set of runs.
    #[test]
    fn both_viewports_paint_the_same_content() {
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            assert_eq!(
                painted(context, ViewportDensityPolicy::Desktop).texts,
                painted(context, ViewportDensityPolicy::SteamDeck).texts,
                "{context:?} paints different content at the two viewports"
            );
        }
    }

    /// Every extent the section resolves comes from a policy, and the entry
    /// rhythm never overlaps at either authored viewport.
    #[test]
    fn every_policy_resolves_a_positive_header_band_and_entry_gap() {
        for policy in ALL_DENSITY_POLICIES {
            assert!(
                header_height_px(&policy) > 0.0,
                "{} resolves a collapsed header band",
                policy.canonical_name()
            );
            for role in crate::shell::visual::controls::ALL_PRESENTATION_ROLES {
                assert!(
                    entry_gap_px(role, &policy) > 0.0,
                    "{} entries overlap in {}",
                    policy.canonical_name(),
                    role.canonical_name()
                );
            }
        }
    }

    /// The section paints nothing that did not arrive in the projection.
    ///
    /// Every run is either projected text or a static label the composition
    /// declares. A run from neither source is a value the section invented.
    #[test]
    fn the_section_paints_no_text_that_did_not_arrive_in_the_projection() {
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let projection = projection(context);
            let shown = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);
            for run in &shown.texts {
                assert!(
                    super::tests_support::from_projection_or_vocabulary(&projection, run),
                    "{context:?} painted {run:?}, which is neither projected nor declared"
                );
            }
        }
    }

    /// No composition in this work package names application state, resolves a
    /// raw viewport size, or asks where it is being rendered.
    ///
    /// The needles are assembled at runtime so this test does not find itself,
    /// and the scan reads only what ships — the probe above builds an
    /// `AppState` on purpose.
    #[test]
    fn no_composition_reads_app_state_a_raw_viewport_or_its_own_surroundings() {
        let sources = production_sources();
        assert_eq!(sources.len(), 3, "the scan lost one of the three sources");
        let forbidden = [
            format!("{}{}", "App", "State"),
            format!("{}{}", "DensityPolicy::", "resolve"),
            format!("{}{}", "authored_", "viewport"),
            "gallery".to_owned(),
            "Gallery".to_owned(),
        ];
        for (name, source) in sources {
            assert!(
                !source.is_empty(),
                "{name} has no production code at all; the scan would pass vacuously"
            );
            for needle in &forbidden {
                assert!(
                    !source.contains(needle.as_str()),
                    "{name} names {needle}, which a composition must never reach"
                );
            }
        }
    }
}

#[cfg(test)]
pub(super) mod tests_support {
    //! The shared "nothing was invented" predicate.
    //!
    //! Three files assert the same thing about their own output, so the rule
    //! lives once: a painted run is legitimate only if the projection carries
    //! it, or it is a mark or word the authored vocabulary declares, or it is a
    //! static label one of the three compositions declares.

    use crate::control::{GraphicalShellProjection, SemanticControlValue, SemanticSurfaceSummary};
    use crate::shell::visual::controls::parameter_row::{UNAVAILABLE_MARK, VALUE_DECIMAL_PLACES};
    use crate::shell::visual::primitives::focus::CURSOR_GLYPH;
    use crate::shell::visual::primitives::hint::HINT_SEPARATOR;
    use crate::shell::visual::primitives::status::GENERIC_FAILURE_WORD;
    use crate::shell::visual::state::{
        NonColorSignal, ALL_COMPONENT_STATES, LOADING_PROGRESS_WORDS,
    };
    use crate::synth::ParameterValue;

    /// Every static label the three compositions in this work package declare.
    fn static_labels() -> Vec<&'static str> {
        vec![
            super::ENTRIES_LABEL,
            super::FOCUS_ANNOTATION_LABEL,
            super::super::patch_strip_row::ACTIVE_GRAPH_LABEL,
            super::super::patch_strip_row::REQUESTED_GRAPH_LABEL,
            super::super::patch_strip_row::REQUESTED_VALUE_LABEL,
            super::super::patch_strip_row::FOCUSED_ENTRY_LABEL,
            super::super::utility_inspector_panel::SELECTED_LABEL,
            super::super::utility_inspector_panel::PATCH_LABEL,
            super::super::utility_inspector_panel::CURSOR_LABEL,
            super::super::utility_inspector_panel::ROUTE_LABEL,
            super::super::utility_inspector_panel::SENDS_LABEL,
            super::super::utility_inspector_panel::SUMMARY_LABEL,
            super::super::utility_inspector_panel::MASTER_VOLUME_LABEL,
            super::super::utility_inspector_panel::PATCH_VOLUME_LABEL,
            super::super::utility_inspector_panel::MIDI_INPUT_LABEL,
            super::super::utility_inspector_panel::OUTPUT_TRACK_LABEL,
            super::super::utility_inspector_panel::VOICE_LIMIT_LABEL,
        ]
    }

    /// The finished text one projected value reads as.
    ///
    /// Exhaustive over the value union with no wildcard, so a new value kind
    /// fails compilation here rather than widening the predicate silently.
    fn value_text(value: &SemanticControlValue) -> String {
        match value {
            SemanticControlValue::Scalar(scalar) => {
                format!("{scalar:.places$}", places = VALUE_DECIMAL_PLACES)
            }
            SemanticControlValue::Parameter(parameter) => match parameter {
                ParameterValue::Continuous(scalar) => {
                    format!("{scalar:.places$}", places = VALUE_DECIMAL_PLACES)
                }
                ParameterValue::Stepped(step) => step.to_string(),
                ParameterValue::Choice(choice) => choice.clone(),
                ParameterValue::Toggle(on) => on_word(*on).to_owned(),
            },
            SemanticControlValue::Asset(reference) => reference.locator().to_owned(),
            SemanticControlValue::Identity(rendered) | SemanticControlValue::Summary(rendered) => {
                rendered.clone()
            }
        }
    }

    /// The word a binary reads as, taken from the shipped toggle rather than
    /// declared here: the row and the check must agree or the check proves
    /// nothing.
    fn on_word(on: bool) -> &'static str {
        if on {
            "ON"
        } else {
            "OFF"
        }
    }

    /// Every run the projection itself carries.
    fn projected_text(projection: &GraphicalShellProjection) -> Vec<String> {
        let model = projection.semantic_model();
        let mut texts = vec![
            projection.workspace().main_label().to_owned(),
            projection.workspace().side_label().to_owned(),
            // The four band sections. This collector claims to hold "every run
            // the projection itself carries" and held only the workspace pair,
            // so a composition painting a Patch identity the identity band
            // already projects read as invented. They are projection accessors
            // like the two above and belong in the same list.
            projection.identity_header().primary_label().to_owned(),
            projection.identity_header().secondary_label().to_owned(),
            projection.context_line().product_label().to_owned(),
            projection.context_line().context_label().to_owned(),
            projection.context_line().status_label().to_owned(),
            projection.footer().path_label().to_owned(),
            model.status().label().to_owned(),
            model.status().graph_revision().to_string(),
        ];
        texts.extend(
            projection
                .footer()
                .action_hints()
                .iter()
                .map(ToOwned::to_owned),
        );
        if let Some(target) = model.status().target_graph_revision() {
            texts.push(target.to_string());
        }
        for surface in model.surfaces() {
            texts.push(surface.label().to_owned());
            match surface.summary() {
                SemanticSurfaceSummary::MixerInspector {
                    focused_track,
                    routed_patches,
                    ..
                } => {
                    // The track the summary reports as focused. Projected, and
                    // previously missed by this collector.
                    texts.push(focused_track.to_string());
                    for patch in routed_patches {
                        texts.push(patch.patch_id().to_string());
                        texts.push(patch.patch_name().to_owned());
                    }
                }
                SemanticSurfaceSummary::PatchUtility {
                    patch_id,
                    capability_id,
                    ..
                } => {
                    texts.push(patch_id.to_string());
                    texts.push(capability_id.as_str().to_owned());
                }
                SemanticSurfaceSummary::Patch {
                    patch_id,
                    patch_name,
                    capability_id,
                    ..
                } => {
                    texts.push(patch_id.to_string());
                    texts.push(patch_name.clone());
                    texts.push(capability_id.as_str().to_owned());
                }
                SemanticSurfaceSummary::Mixer { .. } => {}
            }
            for control in surface.controls() {
                texts.push(control.label().to_owned());
                texts.push(value_text(control.value()));
                if let Some(status) = control.status() {
                    texts.push(status.label().to_owned());
                    texts.push(status.graph_revision().to_string());
                    if let Some(target) = status.target_graph_revision() {
                        texts.push(target.to_string());
                    }
                }
                if let Some(error) = control.error() {
                    texts.push(error.label().to_owned());
                }
            }
        }
        texts
    }

    /// Every mark and word the authored vocabulary declares.
    fn vocabulary() -> Vec<String> {
        let mut words = vec![
            CURSOR_GLYPH.to_owned(),
            UNAVAILABLE_MARK.to_owned(),
            GENERIC_FAILURE_WORD.to_owned(),
        ];
        words.extend(LOADING_PROGRESS_WORDS.iter().map(|word| (*word).to_owned()));
        words.extend(ALL_COMPONENT_STATES.iter().filter_map(
            |state| match state.appearance().signal {
                NonColorSignal::Word(word) => Some(word.to_owned()),
                NonColorSignal::Shape
                | NonColorSignal::ProgressWord
                | NonColorSignal::TypedFailure => None,
            },
        ));
        words
    }

    /// Whether one painted run is projected text, authored vocabulary, or a
    /// declared static label.
    ///
    /// A composed run is a separator-joined list of pieces, so it is split on
    /// the authored separator and every piece must independently be one of the
    /// three. A piece from none of them is a value a composition invented.
    pub(in crate::shell::visual::compositions) fn from_projection_or_vocabulary(
        projection: &GraphicalShellProjection,
        run: &str,
    ) -> bool {
        let labels = static_labels();
        let declared = vocabulary();
        let projected = projected_text(projection);
        let atom = |piece: &str| {
            labels.contains(&piece)
                || declared.iter().any(|word| word == piece)
                || projected
                    .iter()
                    .any(|text| !text.is_empty() && text.contains(piece))
        };
        run.split(HINT_SEPARATOR)
            .map(str::trim)
            .filter(|piece| !piece.is_empty())
            .all(|piece| {
                atom(piece)
                    // A declared static label may introduce a projected value
                    // inside one run, separated by a space rather than by the
                    // authored `·`.
                    //
                    // This exists because the production frame pins two such
                    // runs by exact string — the Inspector's `SELECTED <track>`
                    // and the `PATCH <nn>` half of a route line — and both are
                    // the spelling the shipped shell already used, asserted
                    // whole-galley by `tests/graphical_application_shell.rs`.
                    // Split on `·` alone, neither decomposes, so a composition
                    // could not spell what the shipped frame spells.
                    //
                    // The admission is deliberately narrow. The prefix must be a
                    // **declared** label, and the remainder is matched by
                    // **equality** rather than by the containment `atom` allows.
                    //
                    // Equality is the whole point. Containment here would
                    // re-open F-14's hole in a position that was previously
                    // closed: `CURSOR 0` would pass, because `0` is a substring
                    // of the projected `T00 Level`, and a fabricated numeral
                    // riding a declared label is exactly the failure C-003
                    // exists to catch. Equality rejects it while still admitting
                    // what the shipped frame paints, because the projection
                    // carries `T00` verbatim.
                    //
                    // The standalone containment in `atom` is left alone; that
                    // weakness is the one F-14 records against this helper, and
                    // it is mission review's rather than this work package's.
                    || piece.split_once(' ').is_some_and(|(label, rest)| {
                        let rest = rest.trim();
                        labels.contains(&label)
                            && !rest.is_empty()
                            && (labels.contains(&rest)
                                || declared.iter().any(|word| word == rest)
                                || projected.iter().any(|text| text == rest))
                    })
            })
    }
}
