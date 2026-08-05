//! The mixer strip bank — the sixteen fixed track columns, side by side.
//!
//! This is the one composition whose entries are themselves groups. Every other
//! composition in the family arranges controls; a bank arranges *columns*, and a
//! column is a titled group of controls asked in the vertical-strip role. That
//! nesting is the whole reason the bank exists rather than being a [`section`]
//! with a layout axis: an axis flag does not change entry type, and a horizontal
//! section handed the flat MIXER list would paint all sixteen tracks' controls in
//! one run with no per-column title and no column boundary.
//!
//! # Figma source
//!
//! - `SCREEN · Mixer · 1920×1080` → `Faders` (`42:20`, 1500 × 896) and the
//!   `16 Fader Grid` inside it (`42:25`), which holds sixteen `Fader / Txx`
//!   instances 82 px wide on an 86 px pitch, starting at the authored inset.
//! - `Mixer Legend` (`42:21`): x=24, y=20, 1452 × 36, holding a left run
//!   (`42:22`) and a right-aligned run (`42:24`). The spacer between them
//!   (`42:23`) carries no fill, so nothing is painted between the two — it is
//!   layout, not a leader.
//!
//! # It allocates; it does not consume
//!
//! Stacking consumes extent: each entry takes what it needs and the next starts
//! below. Sixteen columns cannot work that way — they *divide* the surface. So
//! the bank resolves [`ViewportDensityPolicy::mixer_column`] once, computes the
//! content rect once, and places column *i* at one pitch times *i* from the
//! content's left edge. Nothing here scrolls in either axis: the product design
//! authority requires all sixteen tracks visible at the desktop viewport
//! (`DESIGN.md:462`), a scrolled column is a column an operator cannot see is
//! empty, and the declared overflow rule is uniform narrowing — which the policy
//! performs, not this module.
//!
//! # Two levels of everything
//!
//! A single header band can name the mixer or name a track, but not both, and an
//! operator reading a column needs to know which track it is. So the bank titles
//! twice: its own projected legend across the surface, and each column's track
//! identity above that column's cells.
//!
//! Marking is two-level for the mirrored reason (C-003). A bank that could mark
//! only itself would report one empty track as an empty mixer; a column that
//! could mark only itself would leave the bank silent about which tracks it
//! could not draw. Both levels go through [`section::mark_unavailable`], which
//! is the shared mechanism — a second spelling of "absent" would be a second
//! thing to keep in step, and `--` already means *off* in the fader specimen.
//!
//! # What this module owns
//!
//! Nothing. Every extent resolves through [`ViewportDensityPolicy`] and every
//! color and type style through the authored token vocabulary. The bank reads
//! the immutable projection it is handed, arranges, and returns aggregated
//! [`CompositionIntent`].
//!
//! # Two seams recorded rather than closed
//!
//! **The fader paints a control label of its own** at the top of the column it
//! is given — the projected `T00 Level`, which is the label a column is titled
//! `T00` in the design file. The bank's column title therefore sits above a
//! second name for the same track. Removing the fader's label is not this
//! composition's to do, and shortening the projected one is a change to the view
//! model no work package in this mission owns. Recorded for WP06.
//!
//! **Pan selects a second fader.** `(Continuous, VerticalStrip)` resolves to the
//! fader, so a column's Pan cell renders as a fader while the specimen shows a
//! label-and-value frame (`42:26` → `0:8`). The bank supplies the role and does
//! not get to pick a different control for one cell; what it does own is the
//! extent each cell is allocated, and it allocates Pan its share like any other.
//! Recorded rather than worked around.

use eframe::egui::{pos2, Rect, Sense, Ui, UiBuilder, Vec2};

use super::{section, CompositionIntent};
use crate::control::{
    GraphicalShellProjection, InteractionMode, SemanticControlViewModel, SemanticSurfaceViewModel,
    SurfaceId,
};
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::shell::visual::controls::PresentationRole;
use crate::shell::visual::density::{MixerColumnGeometry, ViewportDensityPolicy};
use crate::shell::visual::primitives::hint::HINT_SEPARATOR;
use crate::shell::visual::primitives::rules::{self, RuleSpan};
use crate::shell::visual::primitives::text;
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};

/// Names the structure a bank with no track view data at all cannot draw.
///
/// The legend says which surface; this says which part of it is missing, so the
/// two runs do not read as the same word twice.
pub(super) const BANK_TRACKS_LABEL: &str = "TRACKS";

/// Names the structure a column with no track view data cannot draw.
///
/// Singular, matching the declaration's own wording — a track whose designed
/// *row* has no view data behind it marks *that row* unavailable inside its own
/// column.
///
/// The length is load-bearing, not a preference. This run and the authored
/// unavailable mark share one line inside a column that is 52 px wide at the
/// compact viewport, and the mark is right-aligned against the column's far
/// edge. `ROWS` measures 34.59 px against an 18.13 px mark, which overlaps by
/// 0.72 px; `ROW` measures 25.59 px and clears it by 8.28 px.
/// `the_marked_column_never_collides_its_name_with_its_mark` holds that.
pub(super) const COLUMN_ROW_LABEL: &str = "ROW";

/// The role every cell in a bank is asked in.
///
/// A column is read down rather than across, so its cells are vertical strips.
/// The bank supplies this; no cell decides what it is.
const CELL_ROLE: PresentationRole = PresentationRole::VerticalStrip;

/// One bank of track columns, and the projected header it carries.
pub(super) struct BankGroup<'a> {
    /// The projected title of the surface the bank fills.
    pub title: &'a str,
    /// The projected label of the focused cell, when the surface has one.
    pub annotation: Option<&'a str>,
    /// Every projected cell of every track, flat, in projection order.
    pub controls: &'a [SemanticControlViewModel],
    /// The projected interaction mode, which separates focus from adjustment.
    pub mode: InteractionMode,
}

/// Arranges the MIXER main surface as sixteen track columns and returns what
/// their cells asked for.
pub fn render(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let model = projection.semantic_model();
    let surface = model.surface(SurfaceId::MixerMain);
    render_bank(
        ui,
        BankGroup {
            title: projection.workspace().main_label(),
            annotation: surface.and_then(focused_label),
            controls: surface.map_or(&[], SemanticSurfaceViewModel::controls),
            mode: model.interaction_mode(),
        },
        density,
    )
}

/// Arranges one bank: the legend, then the sixteen columns across the surface.
pub(super) fn render_bank(
    ui: &mut Ui,
    bank: BankGroup<'_>,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    section::inset_scope(ui, density, |ui| {
        let gap = section::entry_gap_px(CELL_ROLE, density);
        render_legend(ui, bank.title, bank.annotation, density);
        ui.add_space(gap);

        // Sixteen columns, always, because the mixer has sixteen tracks always.
        // `DESIGN.md:462` is explicit that empty tracks remain visible and
        // configurable, so a track the projection carries nothing for is a
        // present column carrying a mark — never an absent one. Driving this
        // loop off the projection instead would make an empty track disappear,
        // and an operator cannot see that a track they cannot see is empty.
        let columns: Vec<Vec<&SemanticControlViewModel>> = MixerTrackId::ALL
            .into_iter()
            .map(|track| cells_of(bank.controls, track))
            .collect();

        if columns.iter().all(Vec::is_empty) {
            section::mark_unavailable(ui, BANK_TRACKS_LABEL, density);
            ui.add_space(gap);
        }

        let body = ui.available_rect_before_wrap();
        let geometry = density.mixer_column();
        paint_separators(ui, body, geometry);

        let mut intent = CompositionIntent::none();
        for (track, cells) in MixerTrackId::ALL.into_iter().zip(&columns) {
            let column = column_rect(body, track, geometry);
            intent.absorb(
                ui.scope_builder(UiBuilder::new().max_rect(column), |ui| {
                    render_column(ui, track, cells, bank.mode, density)
                })
                .inner,
            );
        }
        intent
    })
}

/// Where one track's column sits inside the bank's content rect.
///
/// One pitch times the track's own index from the content's left edge. The
/// column is as tall as the body it divides: a bank allocates the whole extent
/// it was given and hands each column its share of the width.
fn column_rect(body: Rect, track: MixerTrackId, geometry: MixerColumnGeometry) -> Rect {
    let left = body.min.x + geometry.pitch_px * track.index() as f32;
    Rect::from_min_max(
        pos2(left, body.min.y),
        pos2(left + geometry.width_px, body.max.y),
    )
}

/// Every projected cell that names one track.
///
/// Read off the identity the projection put in each control's path rather than
/// chunked by index: chunking assumes a projection shape the bank does not own,
/// and reading the identity survives a projection that reorders or that carries
/// a different number of cells per track. A MIXER control naming no track is not
/// a column cell, and the bank leaves it alone rather than filing it under a
/// track it does not belong to.
fn cells_of(
    controls: &[SemanticControlViewModel],
    track: MixerTrackId,
) -> Vec<&SemanticControlViewModel> {
    controls
        .iter()
        .filter(|control| control.path().control_id().as_mixer_track_id() == Some(track))
        .collect()
}

/// Arranges one track column: its title, then its cells stacked beneath.
///
/// The cells divide the column's body the way the columns divide the surface.
/// Each is scoped to its own band, because a fader takes the height it is given
/// and four cells sharing one unbounded column would leave the first cell with
/// all of it. That extent is the bank's to allocate; which control fills it is
/// the selector's to decide.
fn render_column(
    ui: &mut Ui,
    track: MixerTrackId,
    cells: &[&SemanticControlViewModel],
    mode: InteractionMode,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let gap = section::entry_gap_px(CELL_ROLE, density);
    render_column_title(ui, track);
    ui.add_space(gap);
    let body = ui.available_rect_before_wrap();

    if cells.is_empty() {
        return section::render_entries(ui, &[], mode, CELL_ROLE, density, COLUMN_ROW_LABEL);
    }

    let count = cells.len() as f32;
    let height = (body.height() - gap * (count - 1.0)) / count;
    let mut intent = CompositionIntent::none();
    for (index, cell) in cells.iter().enumerate() {
        let top = body.min.y + index as f32 * (height + gap);
        let band = Rect::from_min_max(pos2(body.min.x, top), pos2(body.max.x, top + height));
        intent.absorb(
            ui.scope_builder(UiBuilder::new().max_rect(band), |ui| {
                // Through the shared entry path, so the cell reaches its control
                // through the declared selector and is handed a state derived
                // from the projection. A cell the projection reports as not
                // visible marks itself by the name it projects.
                section::render_entries(
                    ui,
                    std::slice::from_ref(*cell),
                    mode,
                    CELL_ROLE,
                    density,
                    cell.label(),
                )
            })
            .inner,
        );
    }
    intent
}

/// Paints the bank's legend band: the projected title left, the focus
/// annotation right.
///
/// The same anatomy [`section`] paints, at the same derived band height and in
/// the same authored styles, because on MIXER the section header *is* the
/// `Mixer Legend` and the legend moves to the bank rather than being duplicated
/// beside it. It is composed here rather than called because the section's
/// header painter is private to its own module and this work package does not
/// edit it.
fn render_legend(
    ui: &mut Ui,
    title: &str,
    annotation: Option<&str>,
    density: &ViewportDensityPolicy,
) {
    let response = ui.allocate_response(
        Vec2::new(ui.available_width(), section::header_height_px(density)),
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
        format!(
            "{}{HINT_SEPARATOR}{annotation}",
            section::FOCUS_ANNOTATION_LABEL
        ),
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

/// Paints one column's own title: the identity of the track it represents.
///
/// Centered on the column, because a column is read down rather than across and
/// every run in the authored column sits on its axis.
fn render_column_title(ui: &mut Ui, track: MixerTrackId) {
    let response = ui.allocate_response(
        Vec2::new(
            ui.available_width(),
            TypeStyle::LabelControl.metrics().line_height_px,
        ),
        Sense::hover(),
    );
    let painter = ui.painter_at(response.rect);
    let band = response.rect;
    let state = ComponentState::Resting;
    let run = text::layout(
        &painter,
        track.to_string(),
        TypeStyle::LabelControl,
        SemanticColor::TextMuted,
        state,
    );
    let size = run.size();
    painter.galley(
        pos2(
            band.center().x - size.x / 2.0,
            band.center().y - size.y / 2.0,
        ),
        run,
        text::resolved_color(SemanticColor::TextMuted, state).resolve(),
    );
}

/// Paints the hairline that separates two adjacent columns.
///
/// `DESIGN.md:462` — compact columns with hairline separators, not cards. The
/// hairline belongs to the composition that stacks the columns rather than to a
/// column, and it sits in the middle of the gutter the policy leaves between
/// them.
fn paint_separators(ui: &Ui, body: Rect, geometry: MixerColumnGeometry) {
    let painter = ui.painter_at(body);
    for track in MixerTrackId::ALL.into_iter().skip(1) {
        rules::hairline(
            &painter,
            RuleSpan::Vertical {
                x_px: column_rect(body, track, geometry).min.x - geometry.gutter_px() / 2.0,
                from_y_px: body.min.y,
                to_y_px: body.max.y,
            },
        );
    }
}

/// The projected label of the surface's focused cell, when it has one.
fn focused_label(surface: &SemanticSurfaceViewModel) -> Option<&str> {
    surface
        .controls()
        .iter()
        .find(|control| control.focused())
        .map(SemanticControlViewModel::label)
}

#[cfg(test)]
mod tests {
    use eframe::egui::epaint::Shape;
    use eframe::egui::{CentralPanel, Frame, Margin, Pos2, RawInput};

    use super::super::section::probe::{paint, projection};
    use super::super::section::tests_support::from_projection_or_vocabulary;
    use super::*;
    use crate::control::TopLevelContext;
    use crate::shell::visual::compositions::ShellComposition;
    use crate::shell::visual::controls::parameter_row::UNAVAILABLE_MARK;
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::token::KEYLINE_RESTING_PX;
    use crate::shell::visual::typeface::AuthoredTypeface;

    const COMPOSITION: ShellComposition = ShellComposition::MixerStripBank;

    /// The widest disagreement two float paths may have and still be the same
    /// authored geometry.
    ///
    /// A float epsilon, and nothing else. It was the authored hairline width
    /// (1.0) in cycle 1, which is a *thickness* borrowed as a tolerance: applied
    /// on the permissive side of every inequality it accepted a 47 px column
    /// against a hard 48 px floor and absorbed a real 0.72 px overlap inside a
    /// 52 px column. Every coordinate this module asserts on is exactly integral
    /// — Desktop `24 + 86i` at width 82, Steam Deck `16 + 56i` at width 52 — so
    /// the epsilon exists only to survive float equality, and 0.0 is not usable
    /// because a strict `<` on exactly equal values reports "24 is not 24".
    const EPSILON_PX: f32 = 0.001;

    /// One text run that reached the shape list, with where it landed and the
    /// clip it landed under.
    ///
    /// The clip is what makes a column measurable. Everything a column paints
    /// goes through a painter clipped to that column, so the clip of a run is
    /// the extent of the column that emitted it — read off the output rather
    /// than recomputed from the policy the output was supposed to use.
    struct SeenRun {
        text: String,
        rect: Rect,
        clip: Rect,
    }

    /// What one paint pass put on screen.
    ///
    /// [`section::probe`] is the shared harness and it is used below for every
    /// assertion that only has to read what was *said*. It reports non-text
    /// output as a bare count and matches text by substring, and this work
    /// package has to measure *where* sixteen columns landed, *where* fifteen
    /// hairlines landed, and whether a run is a whole label or a fragment of
    /// one. Extending the shared probe would mean editing an approved work
    /// package's file, so the geometry pass lives here and changes nothing else:
    /// the same real `egui::Context`, the same authored faces, the same
    /// production-built projection from `section::probe::projection`, and the
    /// same refusal to fabricate a view model.
    struct Seen {
        runs: Vec<SeenRun>,
        /// Every filled rectangle that reached the shape list. The separators
        /// are in here, and cycle 1 discarded them — deleting every hairline
        /// passed the whole suite.
        rects: Vec<Rect>,
    }

    impl Seen {
        /// The one run saying exactly `text`.
        ///
        /// Exact rather than containing: a column is titled `T00` and the fader
        /// inside it is labelled `T00 Level`, and an assertion that matched a
        /// substring would measure whichever came first.
        fn run(&self, text: &str) -> Option<&SeenRun> {
            self.runs.iter().find(|run| run.text == text)
        }

        /// Every run saying exactly `text`.
        fn every(&self, text: &str) -> Vec<&SeenRun> {
            self.runs.iter().filter(|run| run.text == text).collect()
        }

        fn says(&self, text: &str) -> bool {
            self.run(text).is_some()
        }

        /// Every run, in paint order.
        fn texts(&self) -> Vec<&str> {
            self.runs.iter().map(|run| run.text.as_str()).collect()
        }

        /// The x of every vertical hairline that reached the output.
        ///
        /// A separator is the only thing this composition paints exactly one
        /// authored pixel wide and taller than it is wide; the horizontal
        /// hairline `mark_unavailable` draws under a marked row is one pixel
        /// *tall* and a column wide, so it does not answer here.
        fn separator_axes(&self) -> Vec<f32> {
            let mut axes: Vec<f32> = self
                .rects
                .iter()
                .filter(|rect| {
                    (rect.width() - KEYLINE_RESTING_PX).abs() < EPSILON_PX
                        && rect.height() > rect.width()
                })
                .map(|rect| rect.center().x)
                .collect();
            axes.sort_by(f32::total_cmp);
            axes
        }
    }

    /// The region the shell hands the main workspace.
    ///
    /// The bank is asserted inside this rather than inside the whole viewport,
    /// because "sixteen columns fit" is a claim about the main surface. A test
    /// that handed the bank the whole window would prove a weaker thing and call
    /// it the same name.
    fn main_surface(density: ViewportDensityPolicy) -> Rect {
        Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(density.split().main_px, density.bands().workspace_px),
        )
    }

    /// The content rect the bank arranges inside: the main surface less the
    /// authored inset on both sides.
    fn content_of(density: ViewportDensityPolicy) -> Rect {
        let surface = main_surface(density);
        let inset = density.rhythm().inset_px;
        Rect::from_min_max(
            pos2(surface.min.x + inset, surface.min.y),
            pos2(surface.max.x - inset, surface.max.y),
        )
    }

    /// Paints whatever `add_contents` arranges into the main surface and reports
    /// what actually reached the output.
    ///
    /// The aggregated intent is asserted empty on every pass: no pointer press
    /// reached this frame, so a bank reporting a request here would be reporting
    /// one nobody made.
    fn paint_main_surface(
        density: ViewportDensityPolicy,
        mut add_contents: impl FnMut(&mut Ui) -> CompositionIntent,
    ) -> Seen {
        let context = eframe::egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the vendored faces are present")
                .font_definitions(),
        );
        let size = density.authored_viewport();
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(size.width_px, size.height_px),
            )),
            ..RawInput::default()
        };
        let surface = main_surface(density);
        // One warm-up pass so the requested faces are resident; a first-frame
        // miss would be a harness artifact rather than a rendering failure.
        run(&context, &input, surface, &mut add_contents);
        let shapes = run(&context, &input, surface, &mut add_contents);
        let mut seen = Seen {
            runs: Vec::new(),
            rects: Vec::new(),
        };
        for clipped in &shapes {
            collect(&clipped.shape, clipped.clip_rect, &mut seen);
        }
        seen
    }

    fn run(
        context: &eframe::egui::Context,
        input: &RawInput,
        surface: Rect,
        add_contents: &mut impl FnMut(&mut Ui) -> CompositionIntent,
    ) -> Vec<eframe::egui::epaint::ClippedShape> {
        context
            .run(input.clone(), |context| {
                CentralPanel::default()
                    .frame(Frame::new().inner_margin(Margin::ZERO))
                    .show(context, |ui| {
                        let intent = ui
                            .scope_builder(UiBuilder::new().max_rect(surface), |ui| {
                                add_contents(ui)
                            })
                            .inner;
                        assert!(
                            intent.is_empty(),
                            "the bank asked for something without being addressed"
                        );
                    });
            })
            .shapes
    }

    fn collect(shape: &Shape, clip: Rect, seen: &mut Seen) {
        match shape {
            Shape::Text(run) => seen.runs.push(SeenRun {
                text: run.galley.text().to_owned(),
                rect: Rect::from_min_size(run.pos, run.galley.size()),
                clip,
            }),
            Shape::Vec(shapes) => {
                for nested in shapes {
                    collect(nested, clip, seen);
                }
            }
            Shape::Rect(rect) => seen.rects.push(rect.rect),
            other => {
                let _ = other;
            }
        }
    }

    /// The MIXER main surface's projected cells, production-built.
    fn mixer_cells() -> (GraphicalShellProjection, Vec<SemanticControlViewModel>) {
        let projection = projection(TopLevelContext::Mixer);
        let cells = projection
            .semantic_model()
            .surface(SurfaceId::MixerMain)
            .expect("the MIXER main surface projects")
            .controls()
            .to_vec();
        (projection, cells)
    }

    /// Paints one bank over a supplied cell list, through a real render pass.
    fn paint_cells(
        projection: &GraphicalShellProjection,
        cells: &[SemanticControlViewModel],
        density: ViewportDensityPolicy,
    ) -> Seen {
        paint_main_surface(density, |ui| {
            render_bank(
                ui,
                BankGroup {
                    title: projection.workspace().main_label(),
                    annotation: None,
                    controls: cells,
                    mode: projection.semantic_model().interaction_mode(),
                },
                &density,
            )
        })
    }

    /// The extents of the sixteen painted columns, in track order, read off the
    /// clip each column's own title was painted under.
    fn painted_columns(seen: &Seen, density: ViewportDensityPolicy) -> Vec<Rect> {
        MixerTrackId::ALL
            .into_iter()
            .map(|track| {
                let name = track.to_string();
                let titles = seen.every(&name);
                assert_eq!(
                    titles.len(),
                    1,
                    "{} painted {} columns titled {name}",
                    density.canonical_name(),
                    titles.len()
                );
                titles[0].clip
            })
            .collect()
    }

    /// The bank is wired: asking the family for it reaches this module and
    /// paints. A composition whose own tests pass but which nothing can ask for
    /// is dead code, and so is one still resolving to the shared stub.
    #[test]
    fn the_declared_bank_composition_renders_through_this_module() {
        let shown = paint(
            COMPOSITION,
            &projection(TopLevelContext::Mixer),
            ViewportDensityPolicy::Desktop,
        );
        assert!(
            !shown.texts.is_empty(),
            "the bank painted nothing; it is still the stub"
        );
    }

    /// **The headline.** Sixteen columns actually seat inside the main surface
    /// at both authored viewports, at the geometry the policy declares.
    ///
    /// Both halves are load-bearing and they are not in tension. The extents are
    /// **measured from the paint** — the clip each column's own title landed
    /// under — so nothing here re-derives the layout and calls the derivation a
    /// proof. They are then **bound to the declaration**, because measurement
    /// alone is self-consistent with whatever geometry the bank chose: cycle 1
    /// asserted only that the painted columns were ordered, disjoint, and inside
    /// the surface, and a bank that ignored the policy and divided the content
    /// rect sixteen ways itself — the exact 90.75 px derivation this work package
    /// exists to retire, and the one T048 removed from `fader.rs` — satisfied all
    /// of it with 729 tests green.
    ///
    /// So the painted pitch and the painted width are pinned to
    /// `mixer_column()`, and `density.rs` separately pins `mixer_column()` to the
    /// design file's own 82-on-86 grid. That chain is what makes the crest-spec's
    /// "mixerColumn resolves through this policy and nowhere else" true on the
    /// side that paints, rather than only on the side that declares.
    ///
    /// This is the requirement the shipped mixer silently violates by putting a
    /// horizontal scroll behind a column more than twice the authored width.
    #[test]
    fn sixteen_columns_seat_inside_the_main_surface_at_both_viewports() {
        let projection = projection(TopLevelContext::Mixer);
        for density in ALL_DENSITY_POLICIES {
            let seen = paint_main_surface(density, |ui| render(ui, &projection, &density));
            let columns = painted_columns(&seen, density);
            let content = content_of(density);
            let declared = density.mixer_column();
            let name = density.canonical_name();

            assert_eq!(columns.len(), MixerTrackId::COUNT);
            for (index, column) in columns.iter().enumerate() {
                // The painted column is the *declared* column. Without this the
                // bank may divide the surface itself and every other assertion
                // in this test still holds.
                assert!(
                    (column.width() - declared.width_px).abs() < EPSILON_PX,
                    "{name} painted column {index} {} px wide against the declared {}",
                    column.width(),
                    declared.width_px
                );
                assert!(
                    column.width() >= declared.floor_px,
                    "{name} painted column {index} at {} px, below the {} px floor",
                    column.width(),
                    declared.floor_px
                );
                if index > 0 {
                    let pitch = column.min.x - columns[index - 1].min.x;
                    assert!(
                        (pitch - declared.pitch_px).abs() < EPSILON_PX,
                        "{name} painted a {pitch} px pitch at column {index} against the declared {}",
                        declared.pitch_px
                    );
                    assert!(
                        columns[index - 1].max.x <= column.min.x + EPSILON_PX,
                        "{name} overlaps columns {} and {index}",
                        index - 1
                    );
                }
            }
            assert!(
                (columns[0].min.x - content.min.x).abs() < EPSILON_PX,
                "{name} starts its bank at {} rather than at the authored inset {}",
                columns[0].min.x,
                content.min.x
            );
            let last = columns[MixerTrackId::COUNT - 1];
            assert!(
                last.max.x <= content.max.x + EPSILON_PX,
                "{name} runs its sixteenth column to {} past the content edge {}",
                last.max.x,
                content.max.x
            );
            // Nothing was elided or cut to achieve it: every column title landed
            // whole inside the column that painted it.
            for track in MixerTrackId::ALL {
                let title = seen.run(&track.to_string()).expect("a column title");
                assert!(
                    title.rect.min.x >= title.clip.min.x - EPSILON_PX
                        && title.rect.max.x <= title.clip.max.x + EPSILON_PX,
                    "{name} cut the title of {track} at its column edge"
                );
            }
        }
    }

    /// The bank arranges groups, not one flat run of every track's controls.
    ///
    /// Each track's own cells are painted inside that track's own column. A bank
    /// that flattened the projected list would put all sixty-four controls in one
    /// horizontal run, which looks plausible in a screenshot and has no columns
    /// in it.
    #[test]
    fn every_track_cell_is_painted_inside_that_track_own_column() {
        let projection = projection(TopLevelContext::Mixer);
        for density in ALL_DENSITY_POLICIES {
            let seen = paint_main_surface(density, |ui| render(ui, &projection, &density));
            let columns = painted_columns(&seen, density);
            // Sixteen distinct groups, left to right. Without this the
            // membership check below would be self-consistent: a bank that
            // flattened every track's cells into one run would hand every column
            // the same extent, and every cell would then sit inside "its" column
            // trivially. Proven by mutation — dropping this assertion is what
            // let a flattened bank pass.
            for index in 1..columns.len() {
                assert!(
                    columns[index - 1].min.x < columns[index].min.x
                        && columns[index - 1].max.x <= columns[index].min.x + EPSILON_PX,
                    "{} does not separate columns {} and {index}",
                    density.canonical_name(),
                    index - 1
                );
            }
            let model = projection.semantic_model();
            let surface = model
                .surface(SurfaceId::MixerMain)
                .expect("the MIXER main surface projects");
            for cell in surface.controls().iter().filter(|cell| cell.visible()) {
                let track = cell
                    .path()
                    .control_id()
                    .as_mixer_track_id()
                    .expect("a MIXER main cell names its track");
                let column = columns[track.index()];
                let run = seen
                    .run(cell.label())
                    .unwrap_or_else(|| panic!("{:?} did not reach the bank", cell.label()));
                assert!(
                    (run.clip.min.x - column.min.x).abs() < EPSILON_PX,
                    "{} painted {:?} outside the {track} column",
                    density.canonical_name(),
                    cell.label()
                );
            }
        }
    }

    /// Titling happens at two levels: the bank's own projected legend, and every
    /// column's track identity.
    #[test]
    fn the_bank_titles_at_the_bank_level_and_at_every_column() {
        let projection = projection(TopLevelContext::Mixer);
        let seen = paint_main_surface(ViewportDensityPolicy::Desktop, |ui| {
            render(ui, &projection, &ViewportDensityPolicy::Desktop)
        });
        assert!(
            seen.says(projection.workspace().main_label()),
            "the projected surface title did not reach the legend"
        );
        let focused = projection
            .semantic_model()
            .surface(SurfaceId::MixerMain)
            .and_then(focused_label)
            .expect("the MIXER main surface carries the focus");
        assert!(
            seen.says(&format!(
                "{}{HINT_SEPARATOR}{focused}",
                section::FOCUS_ANNOTATION_LABEL
            )),
            "the legend dropped its projected focus annotation"
        );
        for track in MixerTrackId::ALL {
            assert!(seen.says(&track.to_string()), "no column is titled {track}");
        }
    }

    /// One track without view data marks its own column, and leaves the other
    /// fifteen and the bank itself alone.
    ///
    /// The withheld column is **present and marked**, never absent: an operator
    /// has to be able to see that a track they can see is empty.
    #[test]
    fn a_track_with_no_view_data_marks_its_own_column_and_not_the_bank() {
        let (projection, cells) = mixer_cells();
        let absent = MixerTrackId::new(3).expect("a track to withhold");
        let carried: Vec<SemanticControlViewModel> = cells
            .iter()
            .filter(|cell| cell.path().control_id().as_mixer_track_id() != Some(absent))
            .cloned()
            .collect();
        assert!(carried.len() < cells.len(), "nothing was actually withheld");

        for density in ALL_DENSITY_POLICIES {
            let seen = paint_cells(&projection, &carried, density);
            let columns = painted_columns(&seen, density);
            let column = columns[absent.index()];
            let name = density.canonical_name();

            let marked = seen
                .every(COLUMN_ROW_LABEL)
                .into_iter()
                .find(|run| (run.clip.min.x - column.min.x).abs() < EPSILON_PX)
                .unwrap_or_else(|| panic!("{name} left the {absent} column unmarked"));
            let mark = seen
                .every(UNAVAILABLE_MARK)
                .into_iter()
                .find(|run| (run.clip.min.x - column.min.x).abs() < EPSILON_PX)
                .unwrap_or_else(|| panic!("{name} marked the {absent} column with no mark"));
            let _ = (marked, mark);
            assert_eq!(
                seen.every(COLUMN_ROW_LABEL).len(),
                1,
                "{name} marked a column that carries data"
            );
            assert!(
                !seen.says(BANK_TRACKS_LABEL),
                "{name} reported one empty track as an empty mixer"
            );
            // The other fifteen still paint their cells.
            for track in MixerTrackId::ALL.into_iter().filter(|it| *it != absent) {
                assert!(
                    seen.says(&format!("{track} Level")),
                    "{name} dropped the {track} column's cells"
                );
            }
        }
    }

    /// A marked column's name and its mark share one line and must not collide.
    ///
    /// They sit inside a column 52 px wide at the compact viewport, the name
    /// anchored left and the authored mark anchored right. Cycle 1 asserted this
    /// with a 1.0 px tolerance borrowed from the hairline width, which absorbed a
    /// real 0.72 px overlap: `ROWS` laid out at 34.59 px against an 18.13 px
    /// mark inside 52 px. The label is now singular, which is both what the
    /// declaration says and 9 px shorter.
    #[test]
    fn the_marked_column_never_collides_its_name_with_its_mark() {
        let (projection, cells) = mixer_cells();
        let absent = MixerTrackId::new(3).expect("a track to withhold");
        let carried: Vec<SemanticControlViewModel> = cells
            .iter()
            .filter(|cell| cell.path().control_id().as_mixer_track_id() != Some(absent))
            .cloned()
            .collect();
        for density in ALL_DENSITY_POLICIES {
            let seen = paint_cells(&projection, &carried, density);
            let column = painted_columns(&seen, density)[absent.index()];
            let name = density.canonical_name();
            let named = seen
                .run(COLUMN_ROW_LABEL)
                .unwrap_or_else(|| panic!("{name} left the {absent} column unnamed"));
            let mark = seen
                .every(UNAVAILABLE_MARK)
                .into_iter()
                .find(|run| (run.clip.min.x - column.min.x).abs() < EPSILON_PX)
                .unwrap_or_else(|| panic!("{name} marked the {absent} column with no mark"));
            assert!(
                named.rect.max.x <= mark.rect.min.x,
                "{name} overlaps the {absent} column's name and its mark by {} px",
                named.rect.max.x - mark.rect.min.x
            );
            // Both runs stayed inside the column that owns them.
            assert!(
                named.rect.min.x >= column.min.x - EPSILON_PX
                    && mark.rect.max.x <= column.max.x + EPSILON_PX,
                "{name} painted the {absent} column's mark outside the column"
            );
        }
    }

    /// A bank with no track view data at all marks the bank, and still paints
    /// sixteen marked columns beneath it.
    ///
    /// Both levels speak: the bank says the mixer carried nothing, and each
    /// column says which track it could not draw.
    #[test]
    fn a_bank_with_no_track_view_data_marks_the_bank_and_every_column() {
        let projection = projection(TopLevelContext::Mixer);
        let seen = paint_cells(&projection, &[], ViewportDensityPolicy::Desktop);
        assert!(
            seen.says(BANK_TRACKS_LABEL),
            "an empty bank did not mark itself"
        );
        assert_eq!(
            seen.every(COLUMN_ROW_LABEL).len(),
            MixerTrackId::COUNT,
            "an empty bank did not mark every column"
        );
        let columns = painted_columns(&seen, ViewportDensityPolicy::Desktop);
        assert_eq!(columns.len(), MixerTrackId::COUNT);
    }

    /// Neither marking level invents a value.
    ///
    /// Asserted as the **exact** run list, in paint order, rather than by asking
    /// a predicate whether each run looks legitimate. The shared
    /// `from_projection_or_vocabulary` accepts a run when any projected text
    /// *contains* it, so a fabricated `"0"` walks through on the strength of
    /// being a substring of `"T00 Level"` — as does `"0.0"`, or any fragment of
    /// a real label. C-003 is the constraint this mission enforces hardest and
    /// that predicate is its guard, so the case it is weakest on is spelled out
    /// here instead: an empty bank paints its legend, the two structure names it
    /// declares, the sixteen track identities, and the authored mark. Nothing
    /// else — no level, no zero, and no dash standing in for a reading nobody
    /// reported.
    ///
    /// The shared predicate stays in use for the populated cases below, where
    /// enumerating every projected run is not practical; the defect is recorded
    /// for WP05's file rather than worked around there.
    #[test]
    fn a_marked_bank_paints_exactly_its_structure_names_and_nothing_else() {
        let projection = projection(TopLevelContext::Mixer);
        let seen = paint_cells(&projection, &[], ViewportDensityPolicy::Desktop);

        let mut expected = vec![
            projection.workspace().main_label().to_owned(),
            BANK_TRACKS_LABEL.to_owned(),
            UNAVAILABLE_MARK.to_owned(),
        ];
        for track in MixerTrackId::ALL {
            expected.push(track.to_string());
            expected.push(COLUMN_ROW_LABEL.to_owned());
            expected.push(UNAVAILABLE_MARK.to_owned());
        }
        assert_eq!(
            seen.texts(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "an empty bank painted something it was not handed"
        );
    }

    /// Fifteen hairlines separate the sixteen columns, each on its gutter's
    /// midpoint.
    ///
    /// `DESIGN.md:462` requires compact columns separated by hairlines rather
    /// than cards, and T049 required them explicitly. Cycle 1 asserted nothing
    /// about them: making `paint_separators` a no-op passed all 810 tests, and
    /// so did moving every hairline off the gutter onto the column edge. The
    /// local probe discarded every non-text shape, which is why this needed the
    /// `rects` channel above.
    ///
    /// The expected axes are derived from the *painted* columns, so this asserts
    /// the separators agree with the grid that actually reached the screen.
    #[test]
    fn fifteen_hairlines_sit_on_the_gutter_midpoints_at_both_viewports() {
        let projection = projection(TopLevelContext::Mixer);
        for density in ALL_DENSITY_POLICIES {
            let seen = paint_main_surface(density, |ui| render(ui, &projection, &density));
            let columns = painted_columns(&seen, density);
            let gutter = density.mixer_column().gutter_px();
            let name = density.canonical_name();

            let painted = seen.separator_axes();
            assert_eq!(
                painted.len(),
                MixerTrackId::COUNT - 1,
                "{name} painted {} separators between {} columns",
                painted.len(),
                MixerTrackId::COUNT
            );
            for index in 1..columns.len() {
                let midpoint = columns[index].min.x - gutter / 2.0;
                assert!(
                    painted
                        .iter()
                        .any(|axis| (axis - midpoint).abs() < EPSILON_PX),
                    "{name} painted no hairline on the gutter midpoint {midpoint} \
                     before column {index}; it painted {painted:?}"
                );
                // On the midpoint, not on either column's edge.
                assert!(
                    (midpoint - columns[index - 1].max.x).abs() > EPSILON_PX
                        && (midpoint - columns[index].min.x).abs() > EPSILON_PX,
                    "{name} has no gutter to put a separator in"
                );
            }
        }
    }

    /// A column's cells stack down it, ordered and disjoint.
    ///
    /// The y-axis mirror of the column ordering. Every geometric claim in cycle 1
    /// was x-only, so collapsing all four cells of every column onto one `y` —
    /// a completely broken column — passed the module's whole proof.
    ///
    /// Read from the clip each cell's own control painted under, which is the
    /// band the bank allocated it, in the order the projection supplied.
    #[test]
    fn a_column_cells_stack_down_it_without_overlapping_at_both_viewports() {
        let projection = projection(TopLevelContext::Mixer);
        let model = projection.semantic_model();
        let surface = model
            .surface(SurfaceId::MixerMain)
            .expect("the MIXER main surface projects");
        for density in ALL_DENSITY_POLICIES {
            let seen = paint_main_surface(density, |ui| render(ui, &projection, &density));
            let name = density.canonical_name();
            for track in MixerTrackId::ALL {
                let cells: Vec<&SemanticControlViewModel> = surface
                    .controls()
                    .iter()
                    .filter(|cell| cell.visible())
                    .filter(|cell| cell.path().control_id().as_mixer_track_id() == Some(track))
                    .collect();
                assert!(
                    cells.len() > 1,
                    "the {track} column carries {} cells, too few to stack",
                    cells.len()
                );
                let bands: Vec<Rect> = cells
                    .iter()
                    .map(|cell| {
                        seen.run(cell.label())
                            .unwrap_or_else(|| panic!("{:?} did not reach the bank", cell.label()))
                            .clip
                    })
                    .collect();
                for index in 1..bands.len() {
                    assert!(
                        bands[index - 1].max.y <= bands[index].min.y + EPSILON_PX,
                        "{name} overlaps cells {} and {index} of the {track} column: \
                         {:?} then {:?}",
                        index - 1,
                        bands[index - 1],
                        bands[index]
                    );
                    assert!(
                        bands[index - 1].min.y < bands[index].min.y,
                        "{name} stacks cells {} and {index} of the {track} column on one line",
                        index - 1
                    );
                }
            }
        }
    }

    /// The bank paints nothing that did not arrive in the projection.
    #[test]
    fn the_bank_paints_no_text_that_did_not_arrive_in_the_projection() {
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let projection = projection(context);
            let shown = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);
            let declared = [BANK_TRACKS_LABEL, COLUMN_ROW_LABEL];
            for run in &shown.texts {
                let named = declared.contains(&run.as_str())
                    || MixerTrackId::ALL
                        .into_iter()
                        .any(|track| track.to_string() == *run);
                assert!(
                    named || from_projection_or_vocabulary(&projection, run),
                    "{context:?} painted {run:?}, which is neither projected nor declared"
                );
            }
        }
    }

    /// Both authored viewports paint the same content; only the resolved sizing
    /// differs, so a resolution-specific branch would show up here as a
    /// different set of runs.
    #[test]
    fn both_viewports_paint_the_same_content() {
        let projection = projection(TopLevelContext::Mixer);
        let texts = |density| {
            paint_main_surface(density, |ui| render(ui, &projection, &density))
                .runs
                .into_iter()
                .map(|run| run.text)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            texts(ViewportDensityPolicy::Desktop),
            texts(ViewportDensityPolicy::SteamDeck),
            "the bank paints different content at the two viewports"
        );
    }

    /// The bank scrolls nothing, reads no application state, resolves no raw
    /// viewport size, and reaches no control directly.
    ///
    /// The needles are assembled at runtime so this test does not find itself,
    /// and the scan reads only what ships — the harness above deliberately builds
    /// a real context.
    #[test]
    fn the_bank_scrolls_nothing_and_reaches_past_no_boundary() {
        let source = production_source();
        assert!(
            !source.is_empty(),
            "the scan found no production code; it would pass vacuously"
        );
        for needle in [
            // The shipped mixer's horizontal scroll is the defect this
            // composition retires, and a vertical one would hide a column's
            // lower cells for the same reason. The container is not the only
            // way to scroll, so its three siblings are named too — a bare
            // `scroll_with_delta` is the same defect without the type.
            format!("{}{}", "Scroll", "Area"),
            format!("{}{}", "scroll_with", "_delta"),
            format!("{}{}", "scroll_to", "_rect"),
            format!("{}{}", "scroll_to", "_cursor"),
            format!("{}{}", "App", "State"),
            format!("{}{}", "DensityPolicy::", "resolve"),
            format!("{}{}", "authored_", "viewport"),
            // Every cell reaches its control through the declared selector; a
            // bank that named one would be choosing a control for a cell.
            format!("{}{}", "controls::", "fader"),
            format!("{}{}", "controls::", "toggle"),
            format!("{}{}", "controls::", "meter"),
            format!("{}{}", "Component", "Control"),
        ] {
            assert!(
                !source.contains(&needle),
                "the bank names {needle}, which it must never reach"
            );
        }
    }

    /// This module's production half — everything before its own tests.
    fn production_source() -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/shell/visual/compositions/mixer_strip_bank.rs");
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} is missing", path.display()));
        let marker = format!("#[cfg({})]", "test");
        match source.find(&marker) {
            Some(offset) => source[..offset].to_owned(),
            None => source,
        }
    }
}
