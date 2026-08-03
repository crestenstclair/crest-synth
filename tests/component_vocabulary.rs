//! Measured proof of the authored component vocabulary.
//!
//! Realizes `asset.ComponentVocabularyAcceptanceTests` and the declared project
//! validation `validation.component_vocabulary`, which asserts exit code 0 and
//! the exact marker [`ACCEPTANCE_MARKER`] in stdout.
//!
//! # What makes this non-vacuous
//!
//! A test that asserts token *names* exist, while never comparing a rendered
//! *value*, passes forever and proves nothing (`C-006`). Two disciplines keep
//! that from happening here:
//!
//! 1. **The expected values are written independently of the thing under
//!    test.** [`AUTHORED_COLORS`], [`AUTHORED_TYPE_STYLES`],
//!    [`AUTHORED_SPACING`], and [`AUTHORED_RADII`] are transcribed from
//!    `DESIGN.md` § Colors and § Type and geometry — the product authority —
//!    not read back from `src/shell/visual/token.rs`. Colors are written as the
//!    `#rrggbb` strings the design file publishes and parsed here by
//!    [`authored_rgb`], so nothing in this file shares a derivation with the
//!    `Color32::from_rgb(0x.., 0x.., 0x..)` the vocabulary declares.
//!
//! 2. **The comparison happens where the pixels are.** Every value check runs
//!    twice: once against the declaration, and once against what the production
//!    shell actually painted. [`paint_production_frames`] drives the real
//!    `EframeGraphicalApplication` through a real `egui::Context` at both
//!    authored viewports in both top-level contexts, and the assertions read
//!    the emitted `epaint` shapes — fills, strokes, glyph runs, and their
//!    resolved font metrics.
//!
//! Counts are asserted alongside values, so a silently dropped token fails
//! rather than passing by absence.
//!
//! # The guard that has to be able to fail
//!
//! [`scan_source`] is the literal-absence guard for `NFR-002`. A guard that has
//! never failed is indistinguishable from no guard, so
//! [`the_literal_guard_reports_a_planted_literal`] plants each family of literal
//! in a source sample and asserts the guard reports it with file, line, and
//! kind. [`the_literal_guard_reads_the_delivered_tree`] additionally asserts the
//! scan read a non-trivial number of files and lines, so it cannot pass by
//! scanning nothing.
//!
//! # Recorded limitations
//!
//! These are stated rather than papered over, because a proof that claims more
//! than it measured is worse than one that says where it stops:
//!
//! - **Spacing steps do not reach the shape stream.** `ui.add_space` moves the
//!   layout cursor; it emits no shape. What is measured for spacing is the band
//!   and split arithmetic the density policy produces
//!   ([`check_viewport_integrity`]) plus the declared step values. The step
//!   values themselves are compared against the authored table, not against a
//!   painted gap.
//! - **Corner radii are not asserted through the render path.** The rendering
//!   stack composes its own corner radii for the widgets it owns (a button's
//!   2 px, a progress bar's pill), and those are its geometry rather than the
//!   vocabulary's. Radii are compared against the authored table at the
//!   declaration.
//! - **Interactive-target measurement is split in two.** The shell's own framed
//!   targets are measured as painted rects ([`MIN_TARGET_RULE_FRAMED`]); the
//!   pointer targets whose layout the rendering stack owns are measured through
//!   that stack's own interactive-widget registry ([`MIN_TARGET_RULE_CLICK`]).
//!   The registry also contains every text run, because the stack registers
//!   labels as click-and-drag widgets for text selection; a text run is not a
//!   product target, so the click-and-drag sense is excluded by name and the
//!   exclusion is what this paragraph exists to disclose.
//! - **Clipping is asserted where nothing scrolls.** The shell composes three
//!   scroll regions — the patch parameter list, the mixer track strip, and the
//!   footer's valid actions — whose content legitimately exceeds their viewport
//!   at 1280×800, and a shape stream cannot tell "scrolled out of view" from
//!   "cut off". So [`check_no_text_clips_or_overlaps`] asserts containment only
//!   for the two bands with no scroll region inside them, counts every other
//!   run that left its container, and reports the count as
//!   `runs_scrolled_out_of_view` in the observation line. It is 14 on the
//!   delivered tree: long diagnostic bodies and the track columns and action
//!   hints past the right edge of the Steam Deck viewport. Whether the Steam
//!   Deck footer should scroll its hints at all is a design question this
//!   target records rather than answers.
//! - **Overlap is asserted between runs that are both fully visible.** A run
//!   already partly out of view is excluded, because where its remainder lands
//!   says nothing about whether two readable runs collide.
//! - **Per-page painted specimen coverage is proven elsewhere.** The gallery's
//!   paint pass is private to `src/testing/component_gallery_scene.rs`, so this
//!   target proves the page and digit vocabulary is total in both directions
//!   and leaves "every state painted a specimen on some page at both authored
//!   sizes" to that module's own tests over its real paint pass, which the
//!   declared `test` project validation runs.
//!
//! # What this target found
//!
//! Measuring the production path rather than the declarations turned up six
//! defects, all of them fixed in `src/adapter/eframe_graphical_window.rs` in
//! the same change so that the assertions below could be written at full
//! strength rather than weakened to fit:
//!
//! - Every interactive target was below the authored 48 px minimum: valid-action
//!   buttons at 18 px, control rows at 38 px, the diagnostic header at 22 px.
//! - The meter's unfilled track, the rule between panels, the rule beside an
//!   indented body, and the disclosure triangle painted in four grays the
//!   vocabulary does not declare.
//! - A mixer track's controls ran left to right inside the column instead of
//!   stacking, which laid each row's right-aligned value on top of its label.
//! - The meter's text was taller than the bar holding it.
//!
//! # The one deliberate modification, and why NFR-005 still stands
//!
//! `NFR-005` says no existing shell, projection, or focus test is modified to
//! accommodate the component-controls-and-compositions mission. This file is the
//! single exception, and it is an exception by declaration rather than by
//! convenience: the crest-spec's `ComponentGalleryPage` grew from eight variants
//! to fifteen against ten digit keys, so **the rule this target encoded — one
//! digit binding per page — became false by design**, not inconvenient.
//! [`check_every_gallery_page_is_reachable`] replaces it with the rule that
//! survives: every declared page is reachable, by its binding where it has one
//! and by stepping in every case.
//!
//! The replacement keeps every assertion that is still true, derives its counts
//! from [`GALLERY_DIGIT_BINDING_COUNT`] and [`GALLERY_PAGE_COUNT`] rather than
//! from restated numbers, and adds four: stepping reaches every page in declared
//! order, stepping wraps at neither end, the two step keys select no page, and
//! the union of both routes is exactly the declared page set. Nothing was
//! weakened to fit. **No other test file in the repository was touched.**

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crest_synth::adapter::eframe_graphical_window::{
    install_authored_typeface, EframeGraphicalApplication,
};
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_soundfont_capability,
};
use crest_synth::control::app_event::AppEvent;
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::AppState;
use crest_synth::control::state_projector::StateProjector;
use crest_synth::control::TopLevelContext;
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::patch_id::PatchId;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};
use crest_synth::real_time::audio_command::AudioCommand;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::shell::app_window::{
    AppInputCallback, FrameObservationCallback, ProjectionCallback, TickCallback,
};
use crest_synth::shell::visual::primitives::status::{LoadingPhase, StatusDetail};
use crest_synth::shell::visual::primitives::{focus, status, value};
use crest_synth::shell::visual::typeface::{family_for, family_name, AuthoredTypeface};
use crest_synth::shell::visual::{
    ComponentState, NonColorSignal, Radius, SemanticColor, SpacingStep, TypeStyle, TypefaceError,
    ViewportDensityPolicy, ALL_COLORS, ALL_COMPONENT_STATES, ALL_DENSITY_POLICIES, ALL_RADII,
    ALL_SPACING_STEPS, ALL_TYPE_STYLES, ALL_WEIGHTS, AUTHORED_FAMILY, COMPONENT_STATE_COUNT,
    FOCUS_HALO_OPACITY, FOCUS_HALO_RADIUS_PX, FOCUS_HALO_SPREAD_PX, KEYLINE_EMPHASIS_PX,
    KEYLINE_RESTING_PX, LOADING_PROGRESS_WORDS, MIN_INTERACTIVE_TARGET_PX,
};
use crest_synth::shell::{ShellFrameObservation, ShellRegionId};
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::Patch;
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use crest_synth::testing::component_gallery_scene::{
    ComponentGalleryPage, PageStep, ALL_GALLERY_PAGES, GALLERY_DIGIT_BINDING_COUNT,
    GALLERY_PAGE_COUNT,
};
use eframe::egui;
use eframe::App;
use std::cell::RefCell;
use std::rc::Rc;

/// The exact string `validation.component_vocabulary` asserts on stdout.
///
/// Printed by [`component_vocabulary_acceptance`] and nowhere else, strictly
/// after every declared check has run and passed.
const ACCEPTANCE_MARKER: &str = "CREST_ACCEPTANCE component_vocabulary passed";

// ===========================================================================
// The authored table, transcribed from DESIGN.md
// ===========================================================================
//
// `DESIGN.md` is the product authority for what the interface looks like. The
// values below are copied from it, not from the vocabulary, so a change to the
// vocabulary that drifts from the design fails here rather than agreeing with
// itself.

/// Every authored color: the vocabulary's role, the canonical name the design
/// file publishes, and the authored value as `DESIGN.md` § Colors writes it.
///
/// The canonical-name column is the design file's naming, which `DESIGN.md`
/// abbreviates in its own table (`canvas` for `color/bg/canvas`, `instrument`
/// for `color/accent/instrument/plates`); both denote the same value and the
/// design file's name is the one that ships in code.
const AUTHORED_COLORS: [(SemanticColor, &str, &str); 17] = [
    (SemanticColor::BgCanvas, "color/bg/canvas", "#0c1015"),
    (SemanticColor::BgSurface, "color/bg/surface", "#121821"),
    (SemanticColor::BgPanel, "color/bg/panel", "#17202a"),
    (SemanticColor::BgElevated, "color/bg/elevated", "#1d2733"),
    (SemanticColor::BgSelected, "color/bg/selected", "#2a3745"),
    (
        SemanticColor::BorderDefault,
        "color/border/default",
        "#2a3745",
    ),
    (
        SemanticColor::BorderStrong,
        "color/border/strong",
        "#415166",
    ),
    (SemanticColor::TextPrimary, "color/text/primary", "#f2f6f8"),
    (
        SemanticColor::TextSecondary,
        "color/text/secondary",
        "#b8c4d1",
    ),
    (SemanticColor::TextMuted, "color/text/muted", "#6f8095"),
    (SemanticColor::AccentFocus, "color/accent/focus", "#65e5ff"),
    (
        SemanticColor::AccentAdjust,
        "color/accent/adjust",
        "#ffb454",
    ),
    (
        SemanticColor::AccentPositive,
        "color/accent/positive",
        "#58e887",
    ),
    (
        SemanticColor::AccentWarning,
        "color/accent/warning",
        "#ff6868",
    ),
    (
        SemanticColor::AccentInstrument,
        "color/accent/instrument/plates",
        "#b894ff",
    ),
    (SemanticColor::AccentPatch, "color/accent/patch", "#ff6fbe"),
    (
        SemanticColor::AccentChorus,
        "color/accent/chorus",
        "#f6f178",
    ),
];

/// Every authored type style: the vocabulary's style, the canonical name, and
/// `DESIGN.md` § Type and geometry's size / line / weight / tracking row.
///
/// The weight is carried twice — as the upstream face name the family resolves
/// to and as the numeric weight the design file declares — because a style can
/// drift in either without drifting in the other.
const AUTHORED_TYPE_STYLES: [(TypeStyle, &str, f32, f32, &str, u16, f32); 8] = [
    (
        TypeStyle::DisplayScreen,
        "Display/Screen",
        32.0,
        40.0,
        "SemiBold",
        600,
        0.4,
    ),
    (
        TypeStyle::HeadingSection,
        "Heading/Section",
        18.0,
        24.0,
        "SemiBold",
        600,
        1.4,
    ),
    (
        TypeStyle::HeadingPanel,
        "Heading/Panel",
        14.0,
        20.0,
        "Bold",
        700,
        1.2,
    ),
    (
        TypeStyle::BodyDefault,
        "Body/Default",
        15.0,
        22.0,
        "Regular",
        400,
        0.0,
    ),
    (
        TypeStyle::BodyCompact,
        "Body/Compact",
        13.0,
        18.0,
        "Regular",
        400,
        0.0,
    ),
    (
        TypeStyle::LabelControl,
        "Label/Control",
        12.0,
        16.0,
        "Medium",
        500,
        0.8,
    ),
    (
        TypeStyle::CodeValue,
        "Code/Value",
        14.0,
        20.0,
        "SemiBold",
        600,
        0.2,
    ),
    (
        TypeStyle::InstructionHint,
        "Instruction/Hint",
        11.0,
        16.0,
        "Medium",
        500,
        0.8,
    ),
];

/// `DESIGN.md`: "Spacing: 4, 8, 12, 16, 24, 32 px."
const AUTHORED_SPACING: [(SpacingStep, &str, f32); 6] = [
    (SpacingStep::S4, "space/4", 4.0),
    (SpacingStep::S8, "space/8", 8.0),
    (SpacingStep::S12, "space/12", 12.0),
    (SpacingStep::S16, "space/16", 16.0),
    (SpacingStep::S24, "space/24", 24.0),
    (SpacingStep::S32, "space/32", 32.0),
];

/// `DESIGN.md`: "Radius: 0, 4, 8 px."
const AUTHORED_RADII: [(Radius, f32); 3] = [
    (Radius::None, 0.0),
    (Radius::Small, 4.0),
    (Radius::Large, 8.0),
];

/// `DESIGN.md`: "Minimum interactive target: 48 px."
const AUTHORED_MIN_TARGET_PX: f32 = 48.0;
/// `DESIGN.md`: "Resting keyline: 1 px."
const AUTHORED_KEYLINE_RESTING_PX: f32 = 1.0;
/// `DESIGN.md`: "Focus: 3 px cyan keyline"; "Adjustment: 3 px amber keyline."
const AUTHORED_KEYLINE_EMPHASIS_PX: f32 = 3.0;
/// `DESIGN.md`: "halo radius 8, spread 1, opacity 0.28."
const AUTHORED_HALO_RADIUS_PX: f32 = 8.0;
const AUTHORED_HALO_SPREAD_PX: f32 = 1.0;
const AUTHORED_HALO_OPACITY: f32 = 0.28;

/// `DESIGN.md`: the two authored viewports.
const AUTHORED_VIEWPORTS: [([f32; 2], ViewportDensityPolicy); 2] = [
    ([1_920.0, 1_080.0], ViewportDensityPolicy::Desktop),
    ([1_280.0, 800.0], ViewportDensityPolicy::SteamDeck),
];

/// The palette the adapter painted before this mission.
///
/// These are what regression looks like: not an arbitrary wrong color, but the
/// specific values the shell used to carry. `#6ecdae` is the focus green the
/// authored cyan replaces.
const RETIRED_COLORS: [(&str, &str); 7] = [
    ("focus green", "#6ecdae"),
    ("canvas", "#101216"),
    ("surface", "#181b20"),
    ("elevated", "#1d2127"),
    ("text primary", "#e6eaef"),
    ("text muted", "#969ea9"),
    ("adjust amber", "#e8ae4c"),
];

/// Parses one authored `#rrggbb` string into its channels.
///
/// Written here rather than reached for from the vocabulary: the vocabulary
/// declares its colors as `Color32::from_rgb(0x.., 0x.., 0x..)`, so comparing
/// against a value produced by that same expression would compare the
/// vocabulary to itself.
fn authored_rgb(hex: &str) -> [u8; 3] {
    let digits = hex
        .strip_prefix('#')
        .unwrap_or_else(|| panic!("{hex} is not written as #rrggbb"));
    assert_eq!(digits.len(), 6, "{hex} is not six hex digits");
    let channel = |at: usize| {
        u8::from_str_radix(&digits[at..at + 2], 16)
            .unwrap_or_else(|_| panic!("{hex} carries a non-hex digit"))
    };
    [channel(0), channel(2), channel(4)]
}

/// The authored color as `epaint` carries it.
fn authored_color32(hex: &str) -> egui::Color32 {
    let [r, g, b] = authored_rgb(hex);
    egui::Color32::from_rgb(r, g, b)
}

// ===========================================================================
// The production render path
// ===========================================================================

struct NullBoundary;

impl ControlAudioBoundary for NullBoundary {
    fn push_command(&mut self, _command: AudioCommand) -> Result<(), BoundaryFull> {
        Ok(())
    }

    fn publish_parameters(&mut self, _parameters: ParameterSnapshot) {}
}

/// The production reducer with one installed patch, so the shell has a real
/// projection to paint rather than an empty one.
fn installed_state() -> AppState {
    let provider = production_soundfont_capability().expect("the production SoundFont capability");
    let config =
        create_soundfont_config(&provider, SoundFontInstrument::new(0, 40, false).unwrap())
            .expect("an installed instrument configuration");
    let patch = Patch::new(
        PatchId::new(1).unwrap(),
        "Component Vocabulary".to_owned(),
        config,
        MidiChannel::new(0).unwrap(),
        PatchOutput::new(MixerTrackId::new(0).unwrap(), -6.0).unwrap(),
    );
    let mut state = AppState::new(
        production_capability_registry().expect("the production capability registry"),
        GlobalParameters::new(-3.0).unwrap(),
    );
    state
        .apply(AppEvent::InstallPatches(vec![patch]))
        .expect("the patch installs");
    state
}

fn key_event(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

fn raw_input(size: [f32; 2], events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(size[0], size[1]),
        )),
        predicted_dt: 0.0,
        events,
        ..Default::default()
    }
}

/// One glyph run the production shell put on screen, with the metrics it
/// resolved to.
#[derive(Clone, Debug)]
struct PaintedRun {
    content: String,
    family: egui::FontFamily,
    size_px: f32,
    line_height_px: Option<f32>,
    tracking_px: f32,
    color: egui::Color32,
    rect: egui::Rect,
    clip: egui::Rect,
}

/// One rectangle the production shell put on screen.
#[derive(Clone, Copy, Debug)]
struct PaintedRect {
    rect: egui::Rect,
    stroke_color: egui::Color32,
    stroke_width: f32,
}

/// One complete production frame: what the adapter observed about itself, and
/// everything it actually emitted.
struct PaintedFrame {
    viewport: [f32; 2],
    policy: ViewportDensityPolicy,
    context: TopLevelContext,
    observation: ShellFrameObservation,
    runs: Vec<PaintedRun>,
    rects: Vec<PaintedRect>,
    /// Every color that reached the screen, with where it came from, so a
    /// failure names the shape rather than only the value.
    colors: Vec<(egui::Color32, &'static str)>,
    /// Interact rectangles of the click-sensing widgets the rendering stack
    /// registered, collected only when the frame was painted with the stack's
    /// interactive-widget overlay enabled.
    click_targets: Vec<egui::Rect>,
}

/// The stroke colors the rendering stack's interactive-widget overlay paints,
/// by sense. Only [`DEBUG_SENSE_CLICK`] is a product target: the stack
/// registers every label as click-and-drag for text selection, and every
/// scroll surface as drag.
const DEBUG_SENSE_CLICK: [u8; 3] = [0x88, 0x00, 0x00];
const DEBUG_SENSE_CLICK_AND_DRAG: [u8; 3] = [0x88, 0x00, 0x88];
const DEBUG_SENSE_DRAG: [u8; 3] = [0x00, 0x00, 0x88];

fn debug_overlay_sense(stroke: egui::Color32) -> Option<[u8; 3]> {
    let [r, g, b, a] = stroke.to_array();
    if a != 0xff {
        return None;
    }
    let rgb = [r, g, b];
    [
        DEBUG_SENSE_CLICK,
        DEBUG_SENSE_CLICK_AND_DRAG,
        DEBUG_SENSE_DRAG,
    ]
    .into_iter()
    .find(|sense| *sense == rgb)
}

/// Collects everything one emitted shape put on screen.
fn collect_shape(shape: &egui::Shape, clip: egui::Rect, overlay: bool, frame: &mut PaintedFrame) {
    match shape {
        egui::Shape::Rect(rect) => {
            if overlay {
                // The overlay's own rectangles are the measurement, not part of
                // the shell's painting: record the click targets and discard
                // everything else this pass emitted.
                if debug_overlay_sense(rect.stroke.color) == Some(DEBUG_SENSE_CLICK)
                    && rect.rect.width() > 0.0
                    && rect.rect.height() > 0.0
                {
                    frame.click_targets.push(rect.rect);
                }
                return;
            }
            frame.colors.push((rect.fill, "rect fill"));
            frame.colors.push((rect.stroke.color, "rect stroke"));
            frame.rects.push(PaintedRect {
                rect: rect.rect,
                stroke_color: rect.stroke.color,
                stroke_width: rect.stroke.width,
            });
        }
        egui::Shape::Text(text) => {
            if overlay {
                return;
            }
            // `Galley::rect` is expressed relative to the anchor and already
            // accounts for the job's horizontal alignment: a right-aligned run
            // has `rect.right() == 0.0` and extends leftward from `pos`.
            // Composing the two is what makes this the rectangle the glyphs
            // actually occupy rather than one assumed to start at the anchor.
            let rect = text.galley.rect.translate(text.pos.to_vec2());
            for section in &text.galley.job.sections {
                let color = if section.format.color == egui::Color32::PLACEHOLDER {
                    text.fallback_color
                } else {
                    section.format.color
                };
                frame.colors.push((color, "glyph run"));
                frame.runs.push(PaintedRun {
                    content: text.galley.job.text.clone(),
                    family: section.format.font_id.family.clone(),
                    size_px: section.format.font_id.size,
                    line_height_px: section.format.line_height,
                    tracking_px: section.format.extra_letter_spacing,
                    color,
                    rect,
                    clip,
                });
            }
        }
        egui::Shape::Circle(circle) => {
            if !overlay {
                frame.colors.push((circle.fill, "circle fill"));
                frame.colors.push((circle.stroke.color, "circle stroke"));
            }
        }
        egui::Shape::Path(path) => {
            if !overlay {
                frame.colors.push((path.fill, "path fill"));
                if let egui::epaint::ColorMode::Solid(color) = path.stroke.color {
                    frame.colors.push((color, "path stroke"));
                }
            }
        }
        egui::Shape::LineSegment { stroke, .. } => {
            if !overlay {
                frame.colors.push((stroke.color, "line segment"));
            }
        }
        egui::Shape::Vec(children) => {
            for child in children {
                collect_shape(child, clip, overlay, frame);
            }
        }
        // Meshes, ellipses, beziers, and callbacks carry no flat color this
        // frame. A color reaching the screen only through one of them would be
        // invisible to this scan, so nothing is claimed about them.
        _ => {}
    }
}

/// Drives the production shell through both authored viewports in both
/// top-level contexts and returns what each frame painted.
///
/// This is the production path, not a parallel one: the same
/// `EframeGraphicalApplication` the binary runs, the same
/// `install_authored_typeface`, the same `AppLoop` reducer, and the same
/// `ShellFrameObservation` the adapter emits after painting.
///
/// `overlay` enables the rendering stack's interactive-widget overlay. It is a
/// separate pass because the overlay paints rectangles and labels of its own,
/// which must never be mistaken for something the shell painted.
fn paint_production_frames(overlay: bool) -> Vec<PaintedFrame> {
    let app_loop = AppLoop::new(installed_state(), StateProjector::new(), NullBoundary)
        .expect("the production reducer");
    let shared = Rc::new(RefCell::new(app_loop));

    let input_loop = Rc::clone(&shared);
    let rejections = Rc::new(RefCell::new(Vec::new()));
    let input_rejections = Rc::clone(&rejections);
    let on_input: AppInputCallback = Box::new(move |event| {
        if let Err(rejection) = input_loop.borrow_mut().dispatch_action(event) {
            input_rejections.borrow_mut().push(rejection);
        }
    });
    let projection_loop = Rc::clone(&shared);
    let projection: ProjectionCallback =
        Box::new(move || projection_loop.borrow().current_graphical_shell());
    let on_tick: TickCallback = Box::new(|_| true);
    let observations = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&observations);
    let on_frame: FrameObservationCallback = Box::new(move |observation| {
        observed.borrow_mut().push(observation);
    });

    let mut application = EframeGraphicalApplication::new(on_input, projection, on_tick, on_frame);
    let context = egui::Context::default();
    install_authored_typeface(&context).expect("the authored typeface installs");
    if overlay {
        context.style_mut(|style| style.debug.show_interactive_widgets = true);
    }
    let mut eframe_frame = eframe::Frame::_new_kittest();

    let mut frames = Vec::new();
    for (viewport, policy) in AUTHORED_VIEWPORTS {
        for (key, expected_context) in [
            (egui::Key::Num2, TopLevelContext::Patch),
            (egui::Key::Num1, TopLevelContext::Mixer),
        ] {
            let before = observations.borrow().len();
            context.begin_pass(raw_input(viewport, vec![key_event(key)]));
            application.update(&context, &mut eframe_frame);
            let output = context.end_pass();
            assert_eq!(
                observations.borrow().len(),
                before + 1,
                "the adapter emitted no frame observation at {viewport:?}"
            );
            assert_eq!(
                shared.borrow().current_graphical_shell().context(),
                expected_context,
                "the reducer did not reach {expected_context:?} at {viewport:?}"
            );

            let mut frame = PaintedFrame {
                viewport,
                policy,
                context: expected_context,
                observation: observations.borrow().last().unwrap().clone(),
                runs: Vec::new(),
                rects: Vec::new(),
                colors: Vec::new(),
                click_targets: Vec::new(),
            };
            for clipped in &output.shapes {
                collect_shape(&clipped.shape, clipped.clip_rect, overlay, &mut frame);
            }
            frames.push(frame);
        }
    }

    assert!(
        rejections.borrow().is_empty(),
        "the shell rejected an input while painting: {:?}",
        rejections.borrow()
    );
    assert_eq!(
        frames.len(),
        AUTHORED_VIEWPORTS.len() * 2,
        "both authored viewports must paint both top-level contexts"
    );
    assert_eq!(
        application.frame_observation_error(),
        None,
        "the adapter rejected its own post-paint observation"
    );
    frames
}

/// Every color that reached the screen, ignoring the fully transparent
/// no-fill and no-stroke sentinel.
///
/// `Color32::TRANSPARENT` is the absence of paint, not a color choice, so it is
/// not a literal for the vocabulary to own.
fn opaque_painted_colors(frames: &[PaintedFrame]) -> Vec<(egui::Color32, &'static str)> {
    frames
        .iter()
        .flat_map(|frame| frame.colors.iter().copied())
        .filter(|(color, _)| color.a() != 0)
        .collect()
}

/// Whether a painted color is an authored role, or an authored role at the
/// authored halo opacity.
///
/// The halo is the one place the vocabulary paints a role at less than full
/// alpha, and it is declared: `focus::halo_color` is the authored accent at
/// [`AUTHORED_HALO_OPACITY`].
fn resolves_through_the_authored_table(painted: egui::Color32) -> bool {
    AUTHORED_COLORS.into_iter().any(|(_, _, hex)| {
        let authored = authored_color32(hex);
        painted == authored
    }) || ALL_COLORS
        .into_iter()
        .any(|role| focus::halo_color(role) == painted)
}

// ===========================================================================
// The literal-absence guard
// ===========================================================================

/// One visual literal found outside the vocabulary module.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LiteralViolation {
    file: String,
    line: usize,
    kind: &'static str,
    evidence: String,
}

impl LiteralViolation {
    /// The actionable one-line report a failure prints.
    fn report(&self) -> String {
        format!(
            "{}:{}: {} literal outside the vocabulary — {}",
            self.file, self.line, self.kind, self.evidence
        )
    }
}

/// The one file allowed to spell a raw visual value.
const VOCABULARY_FILE: &str = "src/shell/visual/token.rs";

/// The trees the guard scans: every adapter, view, scene, and shell source.
const SCAN_ROOTS: [&str; 3] = ["src/adapter", "src/shell", "src/testing"];

/// Color constructors that build a color out of raw channels.
///
/// Matched on the constructor name rather than on the receiver, so a color
/// built through a re-export or an alias is still seen. `Color32::TRANSPARENT`
/// and every other named constant are deliberately absent: a named constant is
/// not a literal, and the rule is construction from raw numbers.
const COLOR_CONSTRUCTORS: [&str; 12] = [
    "from_rgb(",
    "from_rgba_unmultiplied(",
    "from_rgba_premultiplied(",
    "from_gray(",
    "from_black_alpha(",
    "from_white_alpha(",
    "from_additive_luminance(",
    "from_srgba_unmultiplied(",
    "from_srgba_premultiplied(",
    "from_luminance_alpha(",
    "from_rgb_additive(",
    "gray(",
];

/// Call positions whose numeric argument is a type size.
const FONT_SIZE_POSITIONS: [&str; 4] = [
    "FontId::new(",
    "FontId::proportional(",
    "FontId::monospace(",
    "set_font_size(",
];

/// Call positions whose numeric argument is a spacing, margin, keyline, or
/// target-size constant.
const SPACING_POSITIONS: [&str; 12] = [
    "add_space(",
    "Margin::same(",
    "Margin::symmetric(",
    "CornerRadius::same(",
    "Stroke::new(",
    "set_min_width(",
    "set_min_height(",
    "Size::exact(",
    "min_size(",
    "Vec2::splat(",
    "rect_filled(",
    "rect_stroke(",
];

/// Assignment targets whose numeric right-hand side is a spacing or target
/// constant.
const SPACING_ASSIGNMENTS: [&str; 4] = [
    "item_spacing",
    "interact_size",
    "button_padding",
    "indent =",
];

/// Strips line comments, and optionally the contents of string literals.
///
/// Comments are always stripped: they narrate the retired design as history,
/// and history is not a value the interface paints with — the same reason
/// `scripts/check_no_name_enumerated_identity.sh` exempts them.
///
/// String contents are kept only for the palette rule, which exists precisely
/// to catch an authored hex spelled as a string. Every other rule reads the
/// code the compiler sees, so a visible label can say anything.
fn strip(line: &str, keep_strings: bool) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    while let Some(character) = chars.next() {
        if in_string {
            if character == '\\' {
                if let Some(escaped) = chars.next() {
                    if keep_strings {
                        out.push(escaped);
                    }
                }
            } else if character == '"' {
                in_string = false;
                out.push('"');
            } else if keep_strings {
                out.push(character);
            }
            continue;
        }
        match character {
            '"' => {
                in_string = true;
                out.push('"');
            }
            '/' if chars.peek() == Some(&'/') => break,
            other => out.push(other),
        }
    }
    out
}

/// Whether a token is a bare numeric literal.
///
/// Zero is excluded: it is the absence of a value rather than an authored one,
/// and the vocabulary declares no token for "no space".
fn is_nonzero_numeric_literal(token: &str) -> bool {
    let token = token.trim();
    let token = token.strip_suffix("_f32").unwrap_or(token);
    let token = token.strip_suffix("f32").unwrap_or(token);
    let token = token.strip_suffix("f64").unwrap_or(token);
    let token = token.strip_suffix("u8").unwrap_or(token);
    if token.is_empty() {
        return false;
    }
    let digits: String = token
        .chars()
        .filter(|character| *character != '_')
        .collect();
    let numeric = if let Some(hex) = digits.strip_prefix("0x") {
        !hex.is_empty() && hex.chars().all(|character| character.is_ascii_hexdigit())
    } else {
        digits
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
            && digits.chars().any(|character| character.is_ascii_digit())
    };
    if !numeric {
        return false;
    }
    digits.parse::<f64>().map_or(
        // A hex literal never parses as a decimal; treat a nonzero hex as a
        // literal and `0x0` as zero.
        !digits.trim_start_matches("0x").trim_matches('0').is_empty(),
        |value| value != 0.0,
    )
}

/// The top-level arguments of the call opening immediately after `at`.
///
/// Nesting is respected, so `min_size(vec2(0.0, TARGET))` yields the single
/// argument `vec2(0.0, TARGET)` rather than two numbers.
fn call_arguments(code: &str, at: usize) -> Vec<String> {
    let mut depth = 0_i32;
    let mut current = String::new();
    let mut arguments = Vec::new();
    for character in code[at..].chars() {
        match character {
            '(' | '[' | '<' if depth > 0 => {
                depth += i32::from(character == '(' || character == '[');
                current.push(character);
            }
            '(' => {
                depth += 1;
                if depth > 1 {
                    current.push(character);
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    arguments.push(current);
                    return arguments;
                }
                current.push(character);
            }
            '[' => {
                depth += 1;
                current.push(character);
            }
            ']' => {
                depth -= 1;
                current.push(character);
            }
            ',' if depth == 1 => {
                arguments.push(std::mem::take(&mut current));
            }
            other => current.push(other),
        }
    }
    // An unterminated call means the argument list wraps onto the next source
    // line; what was collected is still worth inspecting.
    arguments.push(current);
    arguments
}

/// Scans one source for visual literals.
///
/// Exposed as a plain function over text so the guard can be fed a planted
/// sample and shown failing, which is what makes it a guard rather than a
/// decoration.
fn scan_source(path: &str, source: &str) -> Vec<LiteralViolation> {
    let mut violations = Vec::new();
    let authored_hexes: Vec<(&str, &str)> = AUTHORED_COLORS
        .iter()
        .map(|(_, name, hex)| (*name, *hex))
        .chain(RETIRED_COLORS.iter().map(|(name, hex)| (*name, *hex)))
        .collect();

    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let code = strip(raw, false);
        let lowered = strip(raw, true).to_ascii_lowercase();

        let mut flag = |kind: &'static str, evidence: String| {
            violations.push(LiteralViolation {
                file: path.to_owned(),
                line,
                kind,
                evidence,
            });
        };

        for needle in COLOR_CONSTRUCTORS {
            let mut from = 0;
            while let Some(offset) = code[from..].find(needle) {
                let open = from + offset + needle.len() - 1;
                for argument in call_arguments(&code, open) {
                    if is_nonzero_numeric_literal(&argument) {
                        flag(
                            "color",
                            format!(
                                "{}{} builds a color from raw channels",
                                needle,
                                argument.trim()
                            ),
                        );
                        break;
                    }
                }
                from = open + 1;
            }
        }

        if let Some(offset) = code.find("from_hex(") {
            let open = offset + "from_hex(".len() - 1;
            let arguments = call_arguments(&code, open);
            if arguments.iter().any(|argument| argument.contains('"')) {
                flag("color", "from_hex(\"…\") spells a color".to_owned());
            }
        }

        for (name, hex) in &authored_hexes {
            let bare = hex.trim_start_matches('#');
            let [r, g, b] = authored_rgb(hex);
            let channels = format!("0x{r:02x}, 0x{g:02x}, 0x{b:02x}");
            if lowered.contains(&hex.to_ascii_lowercase())
                || lowered.contains(&format!("0x{bare}"))
                || lowered.replace("  ", " ").contains(&channels)
            {
                flag("palette", format!("{name} ({hex}) is spelled here"));
            }
        }

        for needle in FONT_SIZE_POSITIONS {
            if let Some(offset) = code.find(needle) {
                let open = offset + needle.len() - 1;
                if let Some(first) = call_arguments(&code, open).first() {
                    if is_nonzero_numeric_literal(first) {
                        flag(
                            "type size",
                            format!("{needle}{}) sets a raw point size", first.trim()),
                        );
                    }
                }
            }
        }

        for needle in SPACING_POSITIONS {
            let mut from = 0;
            while let Some(offset) = code[from..].find(needle) {
                let open = from + offset + needle.len() - 1;
                for argument in call_arguments(&code, open) {
                    if is_nonzero_numeric_literal(&argument) {
                        flag(
                            "spacing",
                            format!("{needle}…{}…) sets a raw extent", argument.trim()),
                        );
                        break;
                    }
                }
                from = open + 1;
            }
        }

        for needle in SPACING_ASSIGNMENTS {
            if let Some(offset) = code.find(needle) {
                if let Some(rest) = code[offset..].split_once('=') {
                    let right = rest.1.trim_end_matches(';').trim();
                    if is_nonzero_numeric_literal(right) {
                        flag("spacing", format!("{needle} … = {right} sets a raw extent"));
                    }
                }
            }
        }
    }
    violations.sort();
    violations
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every source the guard scans, repository-relative and sorted.
fn scanned_sources() -> Vec<String> {
    fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("{} is unreadable: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                into.push(path);
            }
        }
    }

    let root = repository_root();
    let mut absolute = Vec::new();
    for scan_root in SCAN_ROOTS {
        walk(&root.join(scan_root), &mut absolute);
    }
    let mut relative: Vec<String> = absolute
        .into_iter()
        .map(|path| {
            path.strip_prefix(&root)
                .expect("a scanned source lives under the repository root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .filter(|path| path != VOCABULARY_FILE)
        .collect();
    relative.sort();
    relative
}

/// Runs the guard over the delivered tree, returning what it read and what it
/// found.
fn scan_delivered_tree() -> (Vec<String>, usize, Vec<LiteralViolation>) {
    let root = repository_root();
    let sources = scanned_sources();
    let mut lines_read = 0;
    let mut violations = Vec::new();
    for path in &sources {
        let source = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|error| panic!("{path} is unreadable: {error}"));
        lines_read += source.lines().count();
        violations.extend(scan_source(path, &source));
    }
    violations.sort();
    (sources, lines_read, violations)
}

// ===========================================================================
// A headless painter for the state specimens
// ===========================================================================

/// Paints one state's specimen through the production primitives and returns
/// every text run and filled rectangle it emitted.
///
/// The primitives are the ones the shell composes with — `focus`, `status`, and
/// `value` — driven through a real `egui::Context` with the authored typeface
/// installed, so what is asserted is what a player would read rather than what
/// the rendering code says it would draw.
fn paint_state_specimen(
    state: ComponentState,
    detail: StatusDetail<'_>,
) -> (Vec<String>, Vec<(egui::Rect, egui::Color32)>) {
    let context = egui::Context::default();
    install_authored_typeface(&context).expect("the authored typeface installs");
    context.begin_pass(raw_input([1_920.0, 1_080.0], Vec::new()));
    let row = egui::Rect::from_min_max(egui::pos2(24.0, 120.0), egui::pos2(1_476.0, 168.0));
    let mark = egui::Rect::from_min_max(egui::pos2(1_200.0, 120.0), egui::pos2(1_320.0, 168.0));
    egui::CentralPanel::default().show(&context, |ui| {
        let painter = ui.painter().clone();
        status::paint_row_fill(&painter, row, state);
        focus::focus_frame(&painter, row, state);
        focus::cursor(&painter, row, state);
        status::paint_status_mark(&painter, mark, state, detail);
        value::paint_value(&painter, row.max.x, row.center().y, "0.750", state);
    });
    let output = context.end_pass();

    let mut texts = Vec::new();
    let mut fills = Vec::new();
    fn walk(
        shape: &egui::Shape,
        texts: &mut Vec<String>,
        fills: &mut Vec<(egui::Rect, egui::Color32)>,
    ) {
        match shape {
            egui::Shape::Text(text) => texts.push(text.galley.job.text.clone()),
            egui::Shape::Rect(rect) => {
                if rect.fill.a() != 0 {
                    fills.push((rect.rect, rect.fill));
                }
            }
            egui::Shape::Vec(children) => {
                for child in children {
                    walk(child, texts, fills);
                }
            }
            _ => {}
        }
    }
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut texts, &mut fills);
    }
    (texts, fills)
}

// ===========================================================================
// T034 — authored-value fidelity
// ===========================================================================

/// Every declared value equals its authored counterpart, and the vocabulary
/// holds exactly the declared number of them.
///
/// Both directions are asserted: every authored entry has a declaration, and
/// every declaration has an authored entry. A token dropped from the vocabulary
/// and a token added to it both fail here rather than passing by absence.
fn check_declared_values_match_the_authored_table() {
    assert_eq!(
        ALL_COLORS.len(),
        AUTHORED_COLORS.len(),
        "the vocabulary declares {} colors where DESIGN.md authors {}",
        ALL_COLORS.len(),
        AUTHORED_COLORS.len()
    );
    for (role, name, hex) in AUTHORED_COLORS {
        let [r, g, b] = authored_rgb(hex);
        let resolved = role.resolve();
        assert_eq!(
            [resolved.r(), resolved.g(), resolved.b()],
            [r, g, b],
            "{name} resolves to #{:02x}{:02x}{:02x} where DESIGN.md authors {hex}",
            resolved.r(),
            resolved.g(),
            resolved.b()
        );
        assert_eq!(resolved.a(), 0xff, "{name} is not fully opaque");
        assert!(
            ALL_COLORS.contains(&role),
            "{name} is authored but absent from ALL_COLORS"
        );
    }
    let authored_roles: BTreeSet<&str> = AUTHORED_COLORS.iter().map(|(_, name, _)| *name).collect();
    assert_eq!(
        authored_roles.len(),
        AUTHORED_COLORS.len(),
        "the authored table names a color twice"
    );
    for role in ALL_COLORS {
        assert!(
            authored_roles.contains(role.canonical_name()),
            "{} is declared but DESIGN.md authors no such color",
            role.canonical_name()
        );
    }

    assert_eq!(
        ALL_TYPE_STYLES.len(),
        AUTHORED_TYPE_STYLES.len(),
        "the vocabulary declares {} type styles where DESIGN.md authors {}",
        ALL_TYPE_STYLES.len(),
        AUTHORED_TYPE_STYLES.len()
    );
    for (style, name, size, line, weight_name, weight_numeric, tracking) in AUTHORED_TYPE_STYLES {
        let metrics = style.metrics();
        assert_eq!(metrics.size_px, size, "{name} size");
        assert_eq!(metrics.line_height_px, line, "{name} line height");
        assert_eq!(metrics.tracking_px, tracking, "{name} tracking");
        assert_eq!(
            metrics.weight.numeric(),
            weight_numeric,
            "{name} numeric weight"
        );
        assert_eq!(
            family_name(metrics.weight),
            format!("{AUTHORED_FAMILY} {weight_name}"),
            "{name} resolves to the wrong face"
        );
        assert!(
            ALL_TYPE_STYLES.contains(&style),
            "{name} is authored but absent from ALL_TYPE_STYLES"
        );
    }

    assert_eq!(ALL_SPACING_STEPS.len(), AUTHORED_SPACING.len());
    for (step, name, expected) in AUTHORED_SPACING {
        assert_eq!(step.resolve(), expected, "{name}");
        assert!(ALL_SPACING_STEPS.contains(&step), "{name} is not declared");
    }

    assert_eq!(ALL_RADII.len(), AUTHORED_RADII.len());
    for (radius, expected) in AUTHORED_RADII {
        assert_eq!(radius.resolve(), expected, "{radius:?} radius");
        assert!(ALL_RADII.contains(&radius));
    }

    assert_eq!(MIN_INTERACTIVE_TARGET_PX, AUTHORED_MIN_TARGET_PX);
    assert_eq!(KEYLINE_RESTING_PX, AUTHORED_KEYLINE_RESTING_PX);
    assert_eq!(KEYLINE_EMPHASIS_PX, AUTHORED_KEYLINE_EMPHASIS_PX);
    assert_eq!(FOCUS_HALO_RADIUS_PX, AUTHORED_HALO_RADIUS_PX);
    assert_eq!(FOCUS_HALO_SPREAD_PX, AUTHORED_HALO_SPREAD_PX);
    assert_eq!(FOCUS_HALO_OPACITY, AUTHORED_HALO_OPACITY);
    assert_eq!(ALL_WEIGHTS.len(), 4);

    // The two values that changed most in this mission, spelled out so the
    // regression they replace cannot come back quietly.
    assert_eq!(
        SemanticColor::AccentFocus.resolve(),
        authored_color32("#65e5ff"),
        "focus is not the authored cyan"
    );
    assert_eq!(
        SemanticColor::BgCanvas.resolve(),
        authored_color32("#0c1015"),
        "the canvas is not the authored value"
    );
    for (name, hex) in RETIRED_COLORS {
        let retired = authored_color32(hex);
        for role in ALL_COLORS {
            assert_ne!(
                role.resolve(),
                retired,
                "{} still resolves to the retired {name} {hex}",
                role.canonical_name()
            );
        }
    }
}

/// Every color the production shell paints resolves through the authored
/// table, and the values that must be on screen are on screen.
fn check_painted_colors_are_authored(frames: &[PaintedFrame]) {
    let painted = opaque_painted_colors(frames);
    assert!(
        painted.len() > 100,
        "the production shell painted only {} colors across {} frames, which is \
         too few to have painted a shell at all",
        painted.len(),
        frames.len()
    );

    let mut unauthored: BTreeMap<String, &'static str> = BTreeMap::new();
    for (color, provenance) in &painted {
        if !resolves_through_the_authored_table(*color) {
            unauthored.insert(format!("{color:?}"), provenance);
        }
    }
    assert!(
        unauthored.is_empty(),
        "the production shell painted colors the vocabulary does not author: {unauthored:?}"
    );

    let distinct: BTreeSet<[u8; 4]> = painted.iter().map(|(color, _)| color.to_array()).collect();
    assert!(
        distinct.len() >= 8,
        "the production shell painted only {} distinct colors, which cannot cover \
         a canvas, panels, borders, text, and an accent",
        distinct.len()
    );

    for hex in ["#0c1015", "#65e5ff", "#f2f6f8", "#2a3745"] {
        assert!(
            distinct.contains(&authored_color32(hex).to_array()),
            "the authored {hex} is painted nowhere in the production shell"
        );
    }
    for (name, hex) in RETIRED_COLORS {
        assert!(
            !distinct.contains(&authored_color32(hex).to_array()),
            "the retired {name} {hex} is still painted by the production shell"
        );
    }
}

/// Every glyph run the production shell paints resolves to exactly one authored
/// type style, in the authored family, at the authored size, line height, and
/// tracking.
fn check_painted_type_is_authored(frames: &[PaintedFrame]) {
    let runs: Vec<&PaintedRun> = frames.iter().flat_map(|frame| frame.runs.iter()).collect();
    assert!(
        runs.len() > 50,
        "the production shell painted only {} glyph runs",
        runs.len()
    );

    let mut styles_seen: BTreeSet<&str> = BTreeSet::new();
    for run in &runs {
        let matched = AUTHORED_TYPE_STYLES.into_iter().find(
            |(_, _, size, line, weight_name, _, tracking)| {
                run.size_px == *size
                    && run.line_height_px == Some(*line)
                    && run.tracking_px == *tracking
                    && run.family
                        == egui::FontFamily::Name(format!("{AUTHORED_FAMILY} {weight_name}").into())
            },
        );
        let (_, name, ..) = matched.unwrap_or_else(|| {
            panic!(
                "the run {:?} painted at {} px / {:?} line / {} tracking in {:?}, which no \
                 authored type style declares",
                run.content, run.size_px, run.line_height_px, run.tracking_px, run.family
            )
        });
        styles_seen.insert(name);
        assert!(
            resolves_through_the_authored_table(run.color),
            "the run {:?} painted in {:?}, which the vocabulary does not author",
            run.content,
            run.color
        );
    }

    // The shell composes a subset of the vocabulary; naming which subset makes
    // a silently dropped style visible rather than invisible.
    for expected in [
        "Heading/Section",
        "Heading/Panel",
        "Body/Compact",
        "Label/Control",
        "Code/Value",
        "Instruction/Hint",
    ] {
        assert!(
            styles_seen.contains(expected),
            "the production shell painted no {expected} run; it painted {styles_seen:?}"
        );
    }
    assert_eq!(
        styles_seen.len(),
        6,
        "the production shell painted {styles_seen:?}; the set of styles it composes changed"
    );
}

/// Every keyline the production shell strokes is an authored keyline width.
fn check_painted_keylines_are_authored(frames: &[PaintedFrame]) {
    let stroked: Vec<&PaintedRect> = frames
        .iter()
        .flat_map(|frame| frame.rects.iter())
        .filter(|rect| rect.stroke_width > 0.0 && rect.stroke_color.a() != 0)
        .collect();
    assert!(
        !stroked.is_empty(),
        "the production shell stroked no rectangle at all"
    );
    let widths: BTreeSet<String> = stroked
        .iter()
        .map(|rect| format!("{}", rect.stroke_width))
        .collect();
    for rect in &stroked {
        assert!(
            rect.stroke_width == AUTHORED_KEYLINE_RESTING_PX
                || rect.stroke_width == AUTHORED_KEYLINE_EMPHASIS_PX,
            "a rectangle is stroked at {} px, which is not an authored keyline width; \
             the frame stroked {widths:?}",
            rect.stroke_width
        );
    }
    assert!(
        widths.contains("1") && widths.contains("3"),
        "the production shell painted only {widths:?}; both authored keyline widths \
         must reach the screen"
    );
}

/// The authored halo reaches the screen at its authored radius, spread, and
/// opacity, in the authored focus accent.
fn check_painted_halo_is_authored(frames: &[PaintedFrame]) {
    let halo = focus::halo_color(SemanticColor::AccentFocus);
    let painted = opaque_painted_colors(frames);
    assert!(
        painted.iter().any(|(color, _)| *color == halo),
        "the authored focus halo is painted nowhere"
    );
    let [.., alpha] = halo.to_srgba_unmultiplied();
    assert_eq!(
        alpha,
        (AUTHORED_HALO_OPACITY * 255.0).round() as u8,
        "the halo does not carry the authored opacity"
    );
}

// ===========================================================================
// T036 — viewport integrity
// ===========================================================================

/// The rule name used when a framed target fails the authored minimum.
const MIN_TARGET_RULE_FRAMED: &str = "framed target";
/// The rule name used when a click target fails the authored minimum.
const MIN_TARGET_RULE_CLICK: &str = "click target";

/// Both authored viewports render every structural band and the persistent
/// side region, with the geometry the density policy declares.
fn check_viewport_integrity(frames: &[PaintedFrame]) {
    let mut viewports_seen = BTreeSet::new();
    for frame in frames {
        let [width, height] = frame.viewport;
        let observation = &frame.observation;
        let label = format!("{width}x{height} {:?}", frame.context);
        viewports_seen.insert(format!("{width}x{height}"));

        assert_eq!(
            ViewportDensityPolicy::resolve(width),
            frame.policy,
            "{label}: the shell resolved the wrong density policy"
        );
        assert_eq!(observation.viewport_width(), width, "{label} width");
        assert_eq!(observation.viewport_height(), height, "{label} height");

        // Every required region is present exactly once, in canonical order.
        assert_eq!(
            observation
                .regions()
                .iter()
                .map(|region| region.id())
                .collect::<Vec<_>>(),
            ShellRegionId::surface_descriptor(),
            "{label}: the shell dropped or reordered a structural region"
        );
        for region in observation.regions() {
            let rect = region.rect();
            assert!(
                rect.width() > 0.0 && rect.height() > 0.0,
                "{label}: {} painted an empty rectangle",
                region.id().name()
            );
            assert!(
                !region.visible_label().trim().is_empty(),
                "{label}: {} painted no visible label",
                region.id().name()
            );
        }
        assert!(
            observation.regions_are_non_overlapping(),
            "{label}: two structural regions overlap"
        );

        let bands = frame.policy.bands();
        let split = frame.policy.split();
        let context_line = observation.region(ShellRegionId::ContextLine).rect();
        let identity = observation.region(ShellRegionId::IdentityHeader).rect();
        let main = observation.region(ShellRegionId::MainWorkspace).rect();
        let side = observation
            .region(ShellRegionId::PersistentSideRegion)
            .rect();
        let footer = observation.region(ShellRegionId::Footer).rect();

        assert_eq!(
            context_line.height(),
            bands.context_line_px,
            "{label} context line"
        );
        assert_eq!(
            identity.height(),
            bands.identity_header_px,
            "{label} identity header"
        );
        assert_eq!(footer.height(), bands.footer_px, "{label} footer");
        assert_eq!(side.width(), split.side_px, "{label} side region width");

        // Bands plus workspace tile the viewport exactly, top to bottom.
        assert_eq!(
            context_line.height() + identity.height() + main.height() + footer.height(),
            height,
            "{label}: the bands and workspace do not sum to the viewport height"
        );
        // Main plus side tile the viewport exactly, left to right.
        assert_eq!(
            main.width() + side.width(),
            width,
            "{label}: the workspace and side region do not sum to the viewport width"
        );
        assert_eq!(
            main.max_x(),
            side.min_x(),
            "{label}: a gap splits the workspace"
        );
        assert_eq!(
            context_line.max_y(),
            identity.min_y(),
            "{label}: a gap under the context line"
        );
        assert_eq!(
            identity.max_y(),
            main.min_y(),
            "{label}: a gap under the identity header"
        );
        assert_eq!(
            main.max_y(),
            footer.min_y(),
            "{label}: a gap above the footer"
        );
        assert_eq!(
            main.min_y(),
            side.min_y(),
            "{label}: the side region is not aligned"
        );
        assert_eq!(
            main.max_y(),
            side.max_y(),
            "{label}: the side region is not aligned"
        );

        // The persistent side region is narrowed by density, never hidden.
        assert!(
            side.width() >= 320.0,
            "{label}: the side region narrowed to {} px",
            side.width()
        );

        // The policy's own declared interactive targets clear the authored
        // minimum at this viewport.
        for (name, extent) in [
            ("row height", frame.policy.rhythm().row_height_px),
            (
                "utility control height",
                frame.policy.utility_control().height_px,
            ),
        ] {
            assert!(
                extent >= AUTHORED_MIN_TARGET_PX,
                "{label}: the declared {name} is {extent} px, below the authored minimum"
            );
        }
    }
    assert_eq!(
        viewports_seen.len(),
        AUTHORED_VIEWPORTS.len(),
        "both authored viewports must be measured; saw {viewports_seen:?}"
    );
}

/// No target the production shell paints is smaller than the authored minimum.
///
/// Two rules, because two different owners decide the geometry:
///
/// - [`MIN_TARGET_RULE_FRAMED`] covers every rectangle the shell strokes in an
///   authored role at an authored keyline width — its control rows, its framed
///   buttons, its track columns. These are the focus and adjustment targets a
///   controller reaches, and the shell composes them itself.
/// - [`MIN_TARGET_RULE_CLICK`] covers every click-sensing widget the rendering
///   stack registered, read from that stack's own interactive-widget registry.
///   These are the pointer targets whose height the stack computes.
fn check_no_target_is_below_the_authored_minimum(
    frames: &[PaintedFrame],
    overlay_frames: &[PaintedFrame],
) {
    let mut framed = 0_usize;
    for frame in frames {
        let label = format!("{:?} {:?}", frame.viewport, frame.context);
        for rect in &frame.rects {
            if rect.stroke_width <= 0.0 || rect.stroke_color.a() == 0 {
                continue;
            }
            if !resolves_through_the_authored_table(rect.stroke_color) {
                continue;
            }
            framed += 1;
            assert!(
                rect.rect.height() >= AUTHORED_MIN_TARGET_PX,
                "{label}: a {MIN_TARGET_RULE_FRAMED} is {} px tall, below the authored \
                 {AUTHORED_MIN_TARGET_PX} px minimum (rect {:?})",
                rect.rect.height(),
                rect.rect
            );
        }
    }
    assert!(
        framed >= 8,
        "only {framed} framed targets were measured across both viewports, which is \
         too few to have measured the shell's rows and buttons"
    );

    let mut clicks = 0_usize;
    for frame in overlay_frames {
        let label = format!("{:?} {:?}", frame.viewport, frame.context);
        for rect in &frame.click_targets {
            clicks += 1;
            assert!(
                rect.height() >= AUTHORED_MIN_TARGET_PX,
                "{label}: a {MIN_TARGET_RULE_CLICK} is {} px tall, below the authored \
                 {AUTHORED_MIN_TARGET_PX} px minimum (rect {rect:?})",
                rect.height()
            );
        }
    }
    assert!(
        clicks >= 8,
        "only {clicks} click targets were read from the rendering stack's registry; the \
         overlay pass measured nothing"
    );
}

/// No glyph run escapes the container it was painted into, and no two runs in
/// the same container overlap.
///
/// The container is the clip rectangle the rendering stack attached to the
/// shape, which is exactly the region the shell composed the run inside.
/// Overlap is compared within a container rather than across the frame,
/// because two runs in different clipped regions cannot collide.
fn check_no_text_clips_or_overlaps(frames: &[PaintedFrame]) -> usize {
    let mut measured = 0_usize;
    let mut scrolled_out_of_view = 0_usize;
    let mut defects: BTreeSet<String> = BTreeSet::new();

    for frame in frames {
        let label = format!("{:?} {:?}", frame.viewport, frame.context);
        // The two chrome bands the shell composes with no scroll region inside
        // them. A run painted into one of these has nowhere to go: if it does
        // not fit, it is cut, and there is no gesture that reveals the rest.
        let fixed_bands: Vec<egui::Rect> =
            [ShellRegionId::ContextLine, ShellRegionId::IdentityHeader]
                .into_iter()
                .map(|id| {
                    let rect = frame.observation.region(id).rect();
                    egui::Rect::from_min_max(
                        egui::pos2(rect.min_x(), rect.min_y()),
                        egui::pos2(rect.max_x(), rect.max_y()),
                    )
                })
                .collect();

        let mut by_container: BTreeMap<String, Vec<&PaintedRun>> = BTreeMap::new();
        for run in &frame.runs {
            if run.content.trim().is_empty() {
                continue;
            }
            measured += 1;
            let contained = run.clip.contains_rect(run.rect);
            if !contained {
                scrolled_out_of_view += 1;
                if fixed_bands.contains(&run.clip) {
                    defects.insert(format!(
                        "{label}: clipped — {:?} at {:?} escapes the fixed band {:?}",
                        run.content, run.rect, run.clip
                    ));
                }
                // A run that does not fit its container is not compared for
                // overlap: it is already partly out of view, and where its
                // remainder lands says nothing about whether two readable runs
                // collide.
                continue;
            }
            by_container
                .entry(format!("{:?}", run.clip))
                .or_default()
                .push(run);
        }

        for (container, runs) in &by_container {
            for (index, first) in runs.iter().enumerate() {
                for second in runs.iter().skip(index + 1) {
                    let overlap = first.rect.intersect(second.rect);
                    if overlap.width() > 0.0 && overlap.height() > 0.0 {
                        defects.insert(format!(
                            "{label}: overlapping — {:?} at {:?} and {:?} at {:?} inside {container}",
                            first.content, first.rect, second.content, second.rect
                        ));
                    }
                }
            }
        }
    }

    assert!(
        measured > 50,
        "only {measured} glyph runs were measured for clipping and overlap"
    );
    assert!(
        defects.is_empty(),
        "the production shell painted {} clipped or overlapping text run(s):\n{}",
        defects.len(),
        defects.iter().cloned().collect::<Vec<_>>().join("\n")
    );
    scrolled_out_of_view
}

// ===========================================================================
// T037 — state exhaustiveness, non-color legibility, page totality
// ===========================================================================

/// The state vocabulary is closed at nine and exhaustive iteration yields every
/// one of them.
fn check_state_set_is_closed_and_exhaustive() {
    assert_eq!(COMPONENT_STATE_COUNT, 9);
    assert_eq!(ALL_COMPONENT_STATES.len(), COMPONENT_STATE_COUNT);

    // The match is exhaustive with no wildcard arm, so a tenth variant fails to
    // compile here; naming every variant is what makes the count load-bearing.
    let mut named = BTreeSet::new();
    for state in ALL_COMPONENT_STATES {
        let name = match state {
            ComponentState::Resting => "Resting",
            ComponentState::Focused => "Focused",
            ComponentState::Adjusting => "Adjusting",
            ComponentState::Disabled => "Disabled",
            ComponentState::Loading => "Loading",
            ComponentState::Error => "Error",
            ComponentState::Muted => "Muted",
            ComponentState::Soloed => "Soloed",
            ComponentState::Selected => "Selected",
        };
        assert!(
            named.insert(name),
            "{name} appears twice in ALL_COMPONENT_STATES"
        );
    }
    assert_eq!(named.len(), COMPONENT_STATE_COUNT);
}

/// Every state announces itself with something a player could read with no
/// color vision at all.
///
/// Asserted on the painted output of the production primitives, not on the
/// declaration: a specimen that dropped its mark fails here even though the
/// declaration still carries one.
fn check_every_state_is_legible_without_color() {
    let typed_failure = "ENGINE UNAVAILABLE";
    for state in ALL_COMPONENT_STATES {
        // Exhaustive with no wildcard arm, for the same reason the vocabulary
        // is: a tenth state must fail to compile here rather than fall through
        // to "carries no detail".
        let detail = match state {
            ComponentState::Loading => StatusDetail::Progress(LoadingPhase::Preparing),
            ComponentState::Error => StatusDetail::Failure(typed_failure),
            ComponentState::Resting
            | ComponentState::Focused
            | ComponentState::Adjusting
            | ComponentState::Disabled
            | ComponentState::Muted
            | ComponentState::Soloed
            | ComponentState::Selected => StatusDetail::None,
        };
        let (texts, fills) = paint_state_specimen(state, detail);
        let joined = texts.join(" | ");
        let name = state.canonical_name();

        match state {
            ComponentState::Focused | ComponentState::Adjusting => assert!(
                texts.iter().any(|text| text == focus::CURSOR_GLYPH),
                "{name} painted no {:?} cursor; it painted {joined}",
                focus::CURSOR_GLYPH
            ),
            ComponentState::Disabled => assert!(
                texts.iter().any(|text| text == "Locked"),
                "{name} painted no word; it painted {joined}"
            ),
            ComponentState::Loading => assert!(
                texts.iter().any(|text| text == LOADING_PROGRESS_WORDS[0]),
                "{name} painted no progress word; it painted {joined}"
            ),
            ComponentState::Error => assert!(
                texts.iter().any(|text| text == typed_failure),
                "{name} painted no typed failure; it painted {joined}"
            ),
            ComponentState::Muted => assert!(
                texts.iter().any(|text| text == "M ON"),
                "{name} painted no M ON; it painted {joined}"
            ),
            ComponentState::Soloed => assert!(
                texts.iter().any(|text| text == "S ON"),
                "{name} painted no S ON; it painted {joined}"
            ),
            ComponentState::Selected => {
                assert!(
                    fills
                        .iter()
                        .any(|(_, fill)| *fill == SemanticColor::BgSelected.resolve()),
                    "{name} painted no row fill"
                );
                assert!(
                    fills
                        .iter()
                        .any(|(_, fill)| *fill == SemanticColor::TextPrimary.resolve()),
                    "{name} painted no selection mark distinct from its fill"
                );
            }
            // Resting is the baseline the other eight read against. It carries
            // no mark of its own by declaration, so what is asserted is that it
            // stays the absence: no cursor, no word, no fill.
            ComponentState::Resting => {
                assert!(
                    !texts.iter().any(|text| text == focus::CURSOR_GLYPH),
                    "{name} drew the focus cursor"
                );
                assert_eq!(
                    state.appearance().signal,
                    NonColorSignal::Shape,
                    "{name} is no longer the declared baseline"
                );
            }
        }

        // Every value run reaches the screen in every state, so no state is a
        // blank row.
        assert!(
            texts.iter().any(|text| text == "0.750"),
            "{name} painted no value; it painted {joined}"
        );
    }

    // The loading vocabulary is the structural-edit vocabulary, not a second
    // one, and both phases reach the screen.
    for (phase, word) in [
        (LoadingPhase::Preparing, LOADING_PROGRESS_WORDS[0]),
        (LoadingPhase::Activating, LOADING_PROGRESS_WORDS[1]),
    ] {
        let (texts, _) =
            paint_state_specimen(ComponentState::Loading, StatusDetail::Progress(phase));
        assert!(
            texts.iter().any(|text| text == word),
            "the {phase:?} phase painted no {word}"
        );
    }

    // No two states are told apart by color alone: their painted colorless
    // evidence differs.
    for (index, first) in ALL_COMPONENT_STATES.iter().enumerate() {
        for second in &ALL_COMPONENT_STATES[index + 1..] {
            let a = first.appearance();
            let b = second.appearance();
            let shape_of = |appearance: crest_synth::shell::visual::StateAppearance| {
                (
                    format!("{}", appearance.keyline_px),
                    appearance.draws_halo,
                    appearance.fills_row,
                )
            };
            assert!(
                shape_of(a) != shape_of(b) || a.signal != b.signal,
                "{} and {} differ only in color",
                first.canonical_name(),
                second.canonical_name()
            );
        }
    }
}

/// Every gallery page is reachable — by its digit binding where one exists, and
/// by stepping in every case.
///
/// This replaces the "exactly one digit binding per page" rule, which is now
/// false by design: there are fifteen declared pages and ten digits, so five
/// pages carry no binding and are reached by stepping. The rule that survives is
/// reachability, and the two halves of it are asserted separately — a page
/// reachable by neither would be a page nobody can see.
///
/// **This function is WP08 T045's, and it was written here by WP07 only because
/// growing the page set is a compile-level change to it.** WP07 owns
/// `src/testing/component_gallery_scene.rs` and `src/shell/window_input.rs`; the
/// page count could not grow without this target failing to build, and a target
/// that does not build takes the whole suite with it.
///
/// WP08 completed it by stating the disjunction directly. Stepping reaching all
/// fifteen implies per-page reachability, but only to a reader who notices; the
/// union below says it, with a denominator on each route, so a page reachable by
/// neither is a named failure rather than an inference nobody draws.
fn check_every_gallery_page_is_reachable() {
    assert_eq!(ALL_GALLERY_PAGES.len(), GALLERY_PAGE_COUNT);
    assert_eq!(GALLERY_PAGE_COUNT, 15);
    assert_eq!(GALLERY_DIGIT_BINDING_COUNT, 10);

    let mut digits = BTreeSet::new();
    let mut labels = BTreeSet::new();
    let mut names = BTreeSet::new();
    for page in ALL_GALLERY_PAGES {
        if let Some(digit) = page.digit() {
            assert!(
                digits.insert(format!("{digit:?}")),
                "{} shares its digit with another page",
                page.canonical_name()
            );
            assert_eq!(
                ComponentGalleryPage::for_digit(digit),
                Some(page),
                "{}'s digit does not select it back",
                page.canonical_name()
            );
            assert!(labels.insert(
                page.digit_label()
                    .expect("a page with a digit reads as one")
                    .to_owned()
            ));
        } else {
            assert!(
                page.digit_label().is_none(),
                "{} reads as a digit it does not bind",
                page.canonical_name()
            );
        }
        assert!(names.insert(page.canonical_name()));
        assert!(!page.title().trim().is_empty());
        assert!(!page.index_label().trim().is_empty());
    }
    assert_eq!(
        digits.len(),
        GALLERY_DIGIT_BINDING_COUNT,
        "a digit binds two pages"
    );
    assert_eq!(
        labels.len(),
        GALLERY_DIGIT_BINDING_COUNT,
        "two pages read as one digit"
    );
    assert_eq!(names.len(), GALLERY_PAGE_COUNT);

    // The digit labels are exactly 1..=9 then 0, so the on-screen index and the
    // binding cannot disagree.
    let mut expected: Vec<String> = (1..=9).map(|digit| digit.to_string()).collect();
    expected.push("0".to_owned());
    expected.sort();
    assert_eq!(labels.into_iter().collect::<Vec<_>>(), expected);

    // Stepping alone reaches all fifteen, forwards from the first page.
    let mut reached = vec![ALL_GALLERY_PAGES[0]];
    while let Some(next) = PageStep::Next.apply(*reached.last().expect("a visited page")) {
        reached.push(next);
    }
    assert_eq!(
        reached,
        ALL_GALLERY_PAGES.to_vec(),
        "stepping does not reach every declared page in declared order"
    );
    // And it does not wrap at either end.
    assert_eq!(PageStep::Previous.apply(ALL_GALLERY_PAGES[0]), None);
    assert_eq!(
        PageStep::Next.apply(ALL_GALLERY_PAGES[GALLERY_PAGE_COUNT - 1]),
        None
    );

    // The rule itself, stated as a disjunction with a denominator on each
    // route. A shape scan cannot tell a page that was culled from a page that
    // was never declared, so what is counted is what each route *supplied*: ten
    // pages carry a binding, fifteen are stepped to, and their union is exactly
    // the declared set.
    let by_digit: BTreeSet<&str> = ALL_GALLERY_PAGES
        .into_iter()
        .filter(|page| page.digit().is_some())
        .map(ComponentGalleryPage::canonical_name)
        .collect();
    let by_stepping: BTreeSet<&str> = reached.iter().map(|page| page.canonical_name()).collect();
    let declared: BTreeSet<&str> = ALL_GALLERY_PAGES
        .into_iter()
        .map(ComponentGalleryPage::canonical_name)
        .collect();
    assert_eq!(by_digit.len(), GALLERY_DIGIT_BINDING_COUNT);
    assert_eq!(by_stepping.len(), GALLERY_PAGE_COUNT);
    assert_eq!(declared.len(), GALLERY_PAGE_COUNT);
    let unreachable: Vec<&str> = declared
        .iter()
        .filter(|name| !by_digit.contains(*name) && !by_stepping.contains(*name))
        .copied()
        .collect();
    assert!(
        unreachable.is_empty(),
        "no route reaches: {}",
        unreachable.join(", ")
    );
    assert!(
        by_digit.is_subset(&declared) && by_stepping.is_subset(&declared),
        "a route reaches a page the vocabulary does not declare"
    );

    // No key outside the declared bindings reaches a page: an unbound key
    // normalizes to `Other`, a mapped semantic key binds nothing here, and the
    // two step keys move rather than select.
    for key in [
        crest_synth::shell::window_input::WindowKey::Other,
        crest_synth::shell::window_input::WindowKey::Q,
        crest_synth::shell::window_input::WindowKey::K,
        crest_synth::shell::window_input::WindowKey::BracketLeft,
        crest_synth::shell::window_input::WindowKey::BracketRight,
    ] {
        assert_eq!(
            ComponentGalleryPage::for_digit(key),
            None,
            "{key:?} selects a page it should not bind"
        );
    }
}

// ===========================================================================
// T038 — the typed typeface failure
// ===========================================================================

/// An unavailable face is a typed error naming the face, never a substitution.
fn check_missing_typeface_is_a_typed_failure() {
    let missing = repository_root().join("vendor/no-such-typeface-for-component-vocabulary");
    assert!(
        !missing.exists(),
        "the missing-face fixture path must not exist; this test never mutates the \
         repository, it only points the loader somewhere empty"
    );

    let error = AuthoredTypeface::load_from_dir(&missing)
        .expect_err("a missing face directory must not load successfully");
    match &error {
        TypefaceError::FaceUnavailable { weight, path, .. } => {
            assert_eq!(
                *weight, ALL_WEIGHTS[0],
                "the failure must name the first face it could not read"
            );
            assert!(
                path.ends_with("AzeretMono-Regular.ttf"),
                "the failure must name the file, got {}",
                path.display()
            );
        }
        other => panic!("expected a typed FaceUnavailable, got {other:?}"),
    }
    let message = error.to_string();
    assert!(
        message.contains(AUTHORED_FAMILY),
        "the visible failure must name the unavailable face: {message}"
    );
    assert!(
        message.contains("Regular"),
        "the visible failure must name the weight: {message}"
    );
    assert!(
        !message.to_ascii_lowercase().contains("fallback")
            && !message.to_ascii_lowercase().contains("substitut"),
        "the failure must not describe a substitution: {message}"
    );

    // An unreadable face is not silently accepted either.
    let empty = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("component-vocabulary-empty-faces");
    std::fs::create_dir_all(&empty).expect("the fixture directory is creatable");
    for weight in ALL_WEIGHTS {
        let name = format!(
            "AzeretMono-{}.ttf",
            family_name(weight)
                .strip_prefix(AUTHORED_FAMILY)
                .expect("the family name carries the authored prefix")
                .trim()
        );
        std::fs::write(empty.join(name), b"").expect("the empty face is writable");
    }
    assert!(
        matches!(
            AuthoredTypeface::load_from_dir(&empty),
            Err(TypefaceError::FaceUnreadable { .. })
        ),
        "an empty face must be a typed failure rather than an accepted face"
    );
    std::fs::remove_dir_all(&empty).ok();
}

/// The success path: all four weights register and all eight styles resolve to
/// a registered authored family.
fn check_the_authored_typeface_registers_completely() {
    let typeface = AuthoredTypeface::load().expect("the vendored faces are present");
    assert_eq!(typeface.registered_weights(), ALL_WEIGHTS.to_vec());

    let definitions = typeface.font_definitions();
    for weight in ALL_WEIGHTS {
        let name = family_name(weight);
        assert!(
            definitions.font_data.contains_key(&name),
            "{name} carries no face data"
        );
        assert!(
            definitions
                .families
                .contains_key(&egui::FontFamily::Name(name.clone().into())),
            "{name} has no family entry"
        );
    }
    for (style, name, ..) in AUTHORED_TYPE_STYLES {
        assert!(
            definitions.families.contains_key(&family_for(style)),
            "{name} resolves to an unregistered family"
        );
    }

    // The stack's defaults resolve to the authored face, so no run can fall
    // through to a system font without anyone noticing.
    let regular = family_name(ALL_WEIGHTS[0]);
    assert_eq!(
        definitions.families.get(&egui::FontFamily::Proportional),
        Some(&vec![regular.clone()])
    );
    assert_eq!(
        definitions.families.get(&egui::FontFamily::Monospace),
        Some(&vec![regular])
    );

    // And it takes in a real context: the production installer is what the
    // binary calls, and after it the authored families are the registered ones.
    // The font store only exists once a pass has run, so one is run first.
    let context = egui::Context::default();
    install_authored_typeface(&context).expect("the authored typeface installs");
    context.begin_pass(raw_input([1_920.0, 1_080.0], Vec::new()));
    let _ = context.end_pass();
    let families = context.fonts(|fonts| fonts.families());
    for weight in ALL_WEIGHTS {
        assert!(
            families.contains(&egui::FontFamily::Name(family_name(weight).into())),
            "{} is absent from the installed context",
            family_name(weight)
        );
    }
}

// ===========================================================================
// The declared checks, and the marker
// ===========================================================================

#[test]
fn every_declared_value_equals_its_authored_counterpart() {
    check_declared_values_match_the_authored_table();
}

#[test]
fn the_production_render_path_paints_only_authored_values() {
    let frames = paint_production_frames(false);
    check_painted_colors_are_authored(&frames);
    check_painted_type_is_authored(&frames);
    check_painted_keylines_are_authored(&frames);
    check_painted_halo_is_authored(&frames);
}

#[test]
fn the_literal_guard_reads_the_delivered_tree() {
    let (sources, lines, violations) = scan_delivered_tree();
    // A scan that read nothing passes vacuously, so what it read is asserted
    // before anything is asserted about what it found.
    assert!(
        sources.len() >= 25,
        "the guard scanned only {} sources: {sources:?}",
        sources.len()
    );
    assert!(lines > 5_000, "the guard read only {lines} lines");
    assert!(
        sources
            .iter()
            .any(|path| path == "src/adapter/eframe_graphical_window.rs"),
        "the guard did not scan the production graphical adapter"
    );
    assert!(
        sources
            .iter()
            .any(|path| path == "src/testing/component_gallery_scene.rs"),
        "the guard did not scan the component gallery scene"
    );
    assert!(
        !sources.iter().any(|path| path == VOCABULARY_FILE),
        "the guard scanned the vocabulary, which is the one file allowed to spell values"
    );
    assert!(
        violations.is_empty(),
        "visual literals survive outside {VOCABULARY_FILE}:\n{}",
        violations
            .iter()
            .map(LiteralViolation::report)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_literal_guard_reports_a_planted_literal() {
    // A guard that has never failed is indistinguishable from no guard. Each
    // family is planted, and each must be reported with file, line, and kind.
    let planted = concat!(
        "use eframe::egui::Color32;\n",
        "pub const ACCENT: Color32 = Color32::from_rgb(0x65, 0xe5, 0xff);\n",
        "pub fn paint(ui: &mut egui::Ui) {\n",
        "    ui.add_space(12.0);\n",
        "    let id = FontId::new(14.0, FontFamily::Proportional);\n",
        "    let hex = Color32::from_hex(\"#0c1015\");\n",
        "}\n",
    );
    let violations = scan_source("src/adapter/planted.rs", planted);
    let reported: Vec<String> = violations.iter().map(LiteralViolation::report).collect();
    let joined = reported.join("\n");

    for expected in [
        ("color", 2_usize),
        ("palette", 2),
        ("spacing", 4),
        ("type size", 5),
        ("color", 6),
        ("palette", 6),
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.kind == expected.0 && violation.line == expected.1),
            "the guard did not report a {} literal on line {}:\n{joined}",
            expected.0,
            expected.1
        );
    }
    assert!(
        reported
            .iter()
            .all(|report| report.starts_with("src/adapter/planted.rs:")),
        "every report must name its file so a failure is actionable:\n{joined}"
    );

    // The retired palette is caught too: reintroducing the pre-mission focus
    // green is exactly the regression this guard exists to stop.
    let regression = "let focus = Color32::from_rgb(110, 205, 174);\n";
    let caught = scan_source("src/adapter/regression.rs", regression);
    assert!(
        caught.iter().any(|violation| violation.kind == "color"),
        "the guard accepted a raw-channel color: {caught:?}"
    );
    let retired_hex = "// nothing here\nconst OLD: &str = \"#6ecdae\";\n";
    assert!(
        scan_source("src/adapter/retired.rs", retired_hex)
            .iter()
            .any(|violation| violation.kind == "palette"),
        "the guard accepted the retired focus green spelled as hex"
    );
}

#[test]
fn the_literal_guard_allows_what_the_vocabulary_permits() {
    // Named constants, the transparent sentinel, resolved tokens, zero, and
    // narration in comments are not literals. A guard that flagged these would
    // be turned off within a week, which is the other way a guard stops
    // guarding.
    let permitted = concat!(
        "//! The retired #6ecdae green and Color32::from_rgb(0x65, 0xe5, 0xff) are\n",
        "//! narrated here as history, which is not a value the shell paints.\n",
        "use crate::shell::visual::{SemanticColor, SpacingStep, MIN_INTERACTIVE_TARGET_PX};\n",
        "pub fn paint(ui: &mut egui::Ui) {\n",
        "    ui.add_space(SpacingStep::S12.resolve());\n",
        "    let clear = Color32::TRANSPARENT;\n",
        "    let halo = Color32::from_rgba_unmultiplied(o.r(), o.g(), o.b(), alpha());\n",
        "    let id = FontId::new(style.metrics().size_px, family_for(style));\n",
        "    let corner = CornerRadius::same(Radius::Small.resolve() as u8);\n",
        "    painter.rect_filled(row, 0.0, SemanticColor::BgPanel.resolve());\n",
        "    let button = Button::new(label).min_size(vec2(0.0, MIN_INTERACTIVE_TARGET_PX));\n",
        "    let label = \"PATCH 01 · Lead\";\n",
        "}\n",
    );
    let violations = scan_source("src/adapter/permitted.rs", permitted);
    assert!(
        violations.is_empty(),
        "the guard flagged a permitted construction:\n{}",
        violations
            .iter()
            .map(LiteralViolation::report)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn both_authored_viewports_render_intact() {
    let frames = paint_production_frames(false);
    check_viewport_integrity(&frames);
    check_no_text_clips_or_overlaps(&frames);
}

#[test]
fn no_interactive_target_is_below_the_authored_minimum() {
    let frames = paint_production_frames(false);
    let overlay = paint_production_frames(true);
    check_no_target_is_below_the_authored_minimum(&frames, &overlay);
}

#[test]
fn the_state_vocabulary_is_closed_exhaustive_and_legible_without_color() {
    check_state_set_is_closed_and_exhaustive();
    check_every_state_is_legible_without_color();
}

#[test]
fn every_gallery_page_is_reachable_by_digit_or_by_stepping() {
    check_every_gallery_page_is_reachable();
}

#[test]
fn an_unavailable_typeface_is_a_typed_visible_failure() {
    check_missing_typeface_is_a_typed_failure();
    check_the_authored_typeface_registers_completely();
}

/// The declared acceptance target.
///
/// Every check above runs here, in order, and the marker
/// `validation.component_vocabulary` asserts on is printed strictly after the
/// last of them returns. A failing check panics before the print, so the marker
/// cannot appear on a red run. The checks are also exposed individually so a
/// failure names which claim broke rather than only that something did.
#[test]
fn component_vocabulary_acceptance() {
    check_declared_values_match_the_authored_table();

    let frames = paint_production_frames(false);
    check_painted_colors_are_authored(&frames);
    check_painted_type_is_authored(&frames);
    check_painted_keylines_are_authored(&frames);
    check_painted_halo_is_authored(&frames);

    let (sources, lines, violations) = scan_delivered_tree();
    assert!(
        sources.len() >= 25 && lines > 5_000,
        "the guard read too little"
    );
    assert!(
        violations.is_empty(),
        "visual literals survive outside {VOCABULARY_FILE}:\n{}",
        violations
            .iter()
            .map(LiteralViolation::report)
            .collect::<Vec<_>>()
            .join("\n")
    );

    check_viewport_integrity(&frames);
    let scrolled = check_no_text_clips_or_overlaps(&frames);
    let overlay = paint_production_frames(true);
    check_no_target_is_below_the_authored_minimum(&frames, &overlay);

    check_state_set_is_closed_and_exhaustive();
    check_every_state_is_legible_without_color();
    check_every_gallery_page_is_reachable();

    check_missing_typeface_is_a_typed_failure();
    check_the_authored_typeface_registers_completely();

    println!(
        "CREST_COMPONENT_VOCABULARY_OBSERVATION colors={} type_styles={} spacing_steps={} \
         radii={} states={} pages={} density_policies={} frames={} glyph_runs={} \
         runs_scrolled_out_of_view={} sources_scanned={} lines_scanned={}",
        ALL_COLORS.len(),
        ALL_TYPE_STYLES.len(),
        ALL_SPACING_STEPS.len(),
        ALL_RADII.len(),
        COMPONENT_STATE_COUNT,
        GALLERY_PAGE_COUNT,
        ALL_DENSITY_POLICIES.len(),
        frames.len(),
        frames.iter().map(|frame| frame.runs.len()).sum::<usize>(),
        scrolled,
        sources.len(),
        lines,
    );
    println!("{ACCEPTANCE_MARKER}");
}
