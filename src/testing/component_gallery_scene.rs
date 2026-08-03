//! The browsable component gallery.
//!
//! One command opens a real window showing every declared component specimen
//! through the authored vocabulary and the reusable primitives. Digit keys
//! `1`–`8` select a page; the active page identity is on screen at all times.
//!
//! # This scene is browsable, not autonomous
//!
//! Every other `demo-live-*` scene is deliberately input-isolated: while active,
//! mapped semantic key input is not dispatched into `AppState`, so an
//! asynchronous edit cannot replace the exact generation a checkpoint awaits
//! (`DESIGN.md:634-644`). This scene is the opposite on purpose. It exists to be
//! driven by hand. It therefore accepts input, makes no exact-generation claim,
//! asserts nothing about audio, does not time out, and is not an alias for
//! `demo-live`.
//!
//! It does not weaken the witness contract because it never claims one. The
//! danger runs both ways: giving this scene the witness contract would break
//! paging, and copying this scene's input handling back into a witness would
//! break the generation correlation those scenes depend on. Do neither.
//!
//! # The hard invariant
//!
//! Page selection is scene-local. It never becomes a `SemanticAction`, never
//! reaches `AppState`, and never changes focus, Patch values, graph revision, or
//! audio behavior. The scene binds its digits itself and never consults
//! [`KeyboardInputTranslator`](crate::shell::keyboard_input_translator::KeyboardInputTranslator);
//! `Digit1` and `Digit2` select PATCH and MIXER in the application and select
//! pages here, which is safe precisely because the binding never leaves this
//! file. The scene holds one production `AppState` purely so the claim is
//! *measured*: its generation is read before the window opens and again after it
//! closes, and the difference is reported.
//!
//! # The observation measures rather than asserts
//!
//! Every counter in [`ComponentGalleryObservation`] is incremented inside the
//! paint pass, from what the pass actually emitted. A page counts as painted
//! only when painting it produced text runs; a state counts as painted only when
//! its specimen emitted one *in every declared density policy*, because NFR-005
//! asks for a specimen at both authored sizes and a flat tally cannot tell a
//! state painted twice from a state painted once in the roomier column. There is
//! no specimen list, expected-page table, or pre-render plan that can satisfy the
//! observation without painting — which is the invariant the crest-spec places on
//! this value object.
//!
//! # The window fits the display it is reviewed on
//!
//! Both authored compositions share one window, which tempts its minimum size
//! upward toward the larger authored viewport. A window as tall as the display
//! does not fit on that display once the system's chrome is counted, and the
//! paint pass cannot see that: it measures text against the egui surface, so the
//! lowest band goes under the screen edge while every predicate still reads
//! clean. [`minimum_gallery_viewport`] is therefore measured from what the pages
//! compose at and bounded against the authored desktop display.
//!
//! Realizes `valueObject.Shell.ComponentGalleryPage` and
//! `valueObject.Shell.ComponentGalleryObservation` over
//! `requirement.browsable_component_gallery`.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use eframe::egui::{
    self, pos2, Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind,
    Vec2,
};
use serde::Serialize;

use crate::control::app_state::AppState;
use crate::control::{
    GraphicalShellProjection, SemanticControlKind, SemanticControlViewModel, TopLevelContext,
};
use crate::mixer::global_parameters::GlobalParameters;
use crate::shell::standalone_application::ApplicationConfig;
use crate::shell::visual::compositions::{
    ShellComposition, ShellRegion, ALL_SHELL_COMPOSITIONS, SHELL_COMPOSITION_COUNT,
};
use crate::shell::visual::controls::{
    control_for, ComponentControl, PresentationRole, ALL_COMPONENT_CONTROLS,
    ALL_PRESENTATION_ROLES, ALL_SEMANTIC_CONTROL_KINDS, COMPONENT_CONTROL_COUNT,
};
use crate::shell::visual::primitives::hint::{ActionHint, HintTone, ALL_HINT_TONES};
use crate::shell::visual::primitives::status::{LoadingPhase, StatusDetail, StatusMark};
use crate::shell::visual::primitives::{focus, hint, rules, status, text, value};
use crate::shell::visual::typeface::AuthoredTypeface;
use crate::shell::visual::{
    ComponentState, Radius, SemanticColor, SpacingStep, TypeStyle, TypefaceError,
    ViewportDensityPolicy, ALL_COLORS, ALL_COMPONENT_STATES, ALL_DENSITY_POLICIES,
    ALL_SPACING_STEPS, ALL_TYPE_STYLES, COMPONENT_STATE_COUNT, KEYLINE_EMPHASIS_PX,
    KEYLINE_RESTING_PX, MIN_INTERACTIVE_TARGET_PX,
};
use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};
use crate::shell::ShellRegionId;
use crate::synth::instrument_capability::{CapabilityError, CapabilityRegistry};
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;

/// The stdout marker the gallery observation is printed behind.
pub const COMPONENT_GALLERY_OBSERVATION_MARKER: &str = "CREST_COMPONENT_GALLERY_OBSERVATION ";

/// The native window title.
pub const COMPONENT_GALLERY_WINDOW_TITLE: &str = "crest-synth — component gallery";

/// How many pages the gallery declares.
///
/// Surfaces that must cover every page assert against this rather than against a
/// number they carry themselves.
pub const GALLERY_PAGE_COUNT: usize = 15;

/// How many pages a digit key selects.
///
/// Ten, because there are ten digits. The remaining pages are reached by
/// stepping, which is why stepping exists: a page count larger than the digit
/// count would otherwise make a declared page unreachable.
pub const GALLERY_DIGIT_BINDING_COUNT: usize = 10;

/// One named group of component specimens, selected by a locally bound digit or
/// reached by stepping.
///
/// The set is closed and has no catch-all, for the same reason
/// [`ComponentState`] is closed: a page added without a specimen fails the
/// declared coverage assertion rather than becoming a page nobody can see.
///
/// The seven control and composition pages are appended after the eight that
/// existed before them, and nothing renumbers. An operator who knows `Digit4`
/// is `InteractionStates` keeps finding it there; that is FR-012, and
/// [`FROZEN_DIGIT_BINDING_BASELINE`] is what holds it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComponentGalleryPage {
    /// Every declared semantic color with its canonical name and hex.
    Colors,
    /// All eight type styles at their authored metrics.
    Type,
    /// The six spacing steps, the radii, the keyline widths, and the minimum target.
    SpacingAndGeometry,
    /// Resting, Focused, and Adjusting side by side.
    InteractionStates,
    /// Text roles at every state, plus hairlines and keylines.
    TextAndHairlines,
    /// Value displays and every status mark.
    ValuesAndStatus,
    /// The four action-hint tones.
    ActionHints,
    /// The five structural bands at both authored viewports.
    ShellBands,
    /// The parameter row and the choice row, in every state they declare.
    ParameterAndChoiceRows,
    /// The toggle and the compact slider, in every state they declare.
    TogglesAndSliders,
    /// The fader and the meter, in every state they declare.
    FadersAndMeters,
    /// The browser row and the modal option, in every state they declare.
    BrowserAndModalOptions,
    /// The application shell and the context switch, with projected content.
    ShellAndContextSwitch,
    /// The identity header and the section, with projected content.
    HeadersAndSections,
    /// The Patch strip row, the mixer strip bank, the Utility/Inspector panel,
    /// and the footer, with projected content.
    StripPanelAndFooter,
}

/// Every declared page, in declared order.
///
/// The first ten carry the ten digit bindings in this order; the last five are
/// reached by stepping.
pub const ALL_GALLERY_PAGES: [ComponentGalleryPage; GALLERY_PAGE_COUNT] = [
    ComponentGalleryPage::Colors,
    ComponentGalleryPage::Type,
    ComponentGalleryPage::SpacingAndGeometry,
    ComponentGalleryPage::InteractionStates,
    ComponentGalleryPage::TextAndHairlines,
    ComponentGalleryPage::ValuesAndStatus,
    ComponentGalleryPage::ActionHints,
    ComponentGalleryPage::ShellBands,
    ComponentGalleryPage::ParameterAndChoiceRows,
    ComponentGalleryPage::TogglesAndSliders,
    ComponentGalleryPage::FadersAndMeters,
    ComponentGalleryPage::BrowserAndModalOptions,
    ComponentGalleryPage::ShellAndContextSwitch,
    ComponentGalleryPage::HeadersAndSections,
    ComponentGalleryPage::StripPanelAndFooter,
];

/// The eight `(page identity, digit)` bindings as they stood before the control
/// and composition pages were added.
///
/// Frozen as data, modelled on `FROZEN_TOPOLOGY_IDENTITY_BASELINE`
/// (`tests/effects_and_buses.rs:59`), which the project already uses for
/// exactly this kind of add-only contract. FR-012 exists because an operator
/// feels a moved binding immediately, so this is a regression gate rather than
/// a promise: reordering [`ALL_GALLERY_PAGES`], renaming a page, or handing an
/// existing page a different digit fails the assertion that reads this.
///
/// New pages append. Nothing here moves.
pub const FROZEN_DIGIT_BINDING_BASELINE: [(&str, WindowKey); 8] = [
    ("Colors", WindowKey::Digit1),
    ("Type", WindowKey::Digit2),
    ("SpacingAndGeometry", WindowKey::Digit3),
    ("InteractionStates", WindowKey::Digit4),
    ("TextAndHairlines", WindowKey::Digit5),
    ("ValuesAndStatus", WindowKey::Digit6),
    ("ActionHints", WindowKey::Digit7),
    ("ShellBands", WindowKey::Digit8),
];

impl ComponentGalleryPage {
    /// The digit that selects this page, or `None` when no digit does.
    ///
    /// There are fifteen pages and ten digits, so five pages carry no binding.
    /// They return `None` rather than a placeholder key: `WindowKey::Other` is
    /// the catch-all for keys the window saw and could not name, and reusing it
    /// here would make "this page has no digit" and "this key is not one we
    /// recognize" the same value.
    pub const fn digit(self) -> Option<WindowKey> {
        match self {
            Self::Colors => Some(WindowKey::Digit1),
            Self::Type => Some(WindowKey::Digit2),
            Self::SpacingAndGeometry => Some(WindowKey::Digit3),
            Self::InteractionStates => Some(WindowKey::Digit4),
            Self::TextAndHairlines => Some(WindowKey::Digit5),
            Self::ValuesAndStatus => Some(WindowKey::Digit6),
            Self::ActionHints => Some(WindowKey::Digit7),
            Self::ShellBands => Some(WindowKey::Digit8),
            Self::ParameterAndChoiceRows => Some(WindowKey::Digit9),
            Self::TogglesAndSliders => Some(WindowKey::Digit0),
            Self::FadersAndMeters
            | Self::BrowserAndModalOptions
            | Self::ShellAndContextSwitch
            | Self::HeadersAndSections
            | Self::StripPanelAndFooter => None,
        }
    }

    /// The page a digit selects, or `None` when the key binds no page.
    ///
    /// This is the whole of the scene's key binding. Nothing here produces a
    /// `SemanticAction`, and no unbound key reaches a default page.
    pub fn for_digit(key: WindowKey) -> Option<Self> {
        ALL_GALLERY_PAGES
            .into_iter()
            .find(|page| page.digit() == Some(key))
    }

    /// The digit as it reads on screen, where one selects this page.
    pub const fn digit_label(self) -> Option<&'static str> {
        match self {
            Self::Colors => Some("1"),
            Self::Type => Some("2"),
            Self::SpacingAndGeometry => Some("3"),
            Self::InteractionStates => Some("4"),
            Self::TextAndHairlines => Some("5"),
            Self::ValuesAndStatus => Some("6"),
            Self::ActionHints => Some("7"),
            Self::ShellBands => Some("8"),
            Self::ParameterAndChoiceRows => Some("9"),
            Self::TogglesAndSliders => Some("0"),
            Self::FadersAndMeters
            | Self::BrowserAndModalOptions
            | Self::ShellAndContextSwitch
            | Self::HeadersAndSections
            | Self::StripPanelAndFooter => None,
        }
    }

    /// The canonical page identity.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Colors => "Colors",
            Self::Type => "Type",
            Self::SpacingAndGeometry => "SpacingAndGeometry",
            Self::InteractionStates => "InteractionStates",
            Self::TextAndHairlines => "TextAndHairlines",
            Self::ValuesAndStatus => "ValuesAndStatus",
            Self::ActionHints => "ActionHints",
            Self::ShellBands => "ShellBands",
            Self::ParameterAndChoiceRows => "ParameterAndChoiceRows",
            Self::TogglesAndSliders => "TogglesAndSliders",
            Self::FadersAndMeters => "FadersAndMeters",
            Self::BrowserAndModalOptions => "BrowserAndModalOptions",
            Self::ShellAndContextSwitch => "ShellAndContextSwitch",
            Self::HeadersAndSections => "HeadersAndSections",
            Self::StripPanelAndFooter => "StripPanelAndFooter",
        }
    }

    /// The page title, as it reads on screen.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Colors => "COLORS",
            Self::Type => "TYPE",
            Self::SpacingAndGeometry => "SPACING AND GEOMETRY",
            Self::InteractionStates => "INTERACTION STATES",
            Self::TextAndHairlines => "TEXT AND HAIRLINES",
            Self::ValuesAndStatus => "VALUES AND STATUS",
            Self::ActionHints => "ACTION HINTS",
            Self::ShellBands => "SHELL BANDS",
            Self::ParameterAndChoiceRows => "PARAMETER AND CHOICE ROWS",
            Self::TogglesAndSliders => "TOGGLES AND SLIDERS",
            Self::FadersAndMeters => "FADERS AND METERS",
            Self::BrowserAndModalOptions => "BROWSER AND MODAL OPTIONS",
            Self::ShellAndContextSwitch => "SHELL AND CONTEXT SWITCH",
            Self::HeadersAndSections => "HEADERS AND SECTIONS",
            Self::StripPanelAndFooter => "STRIP, PANEL AND FOOTER",
        }
    }

    /// The short title used in the page index line.
    pub const fn index_label(self) -> &'static str {
        match self {
            Self::Colors => "COLORS",
            Self::Type => "TYPE",
            Self::SpacingAndGeometry => "SPACING",
            Self::InteractionStates => "STATES",
            Self::TextAndHairlines => "TEXT",
            Self::ValuesAndStatus => "VALUES",
            Self::ActionHints => "HINTS",
            Self::ShellBands => "BANDS",
            Self::ParameterAndChoiceRows => "ROWS",
            Self::TogglesAndSliders => "TOGGLES",
            Self::FadersAndMeters => "FADERS",
            Self::BrowserAndModalOptions => "BROWSER",
            Self::ShellAndContextSwitch => "SHELL",
            Self::HeadersAndSections => "HEADERS",
            Self::StripPanelAndFooter => "STRIP",
        }
    }

    /// This page's position in [`ALL_GALLERY_PAGES`].
    const fn index(self) -> usize {
        match self {
            Self::Colors => 0,
            Self::Type => 1,
            Self::SpacingAndGeometry => 2,
            Self::InteractionStates => 3,
            Self::TextAndHairlines => 4,
            Self::ValuesAndStatus => 5,
            Self::ActionHints => 6,
            Self::ShellBands => 7,
            Self::ParameterAndChoiceRows => 8,
            Self::TogglesAndSliders => 9,
            Self::FadersAndMeters => 10,
            Self::BrowserAndModalOptions => 11,
            Self::ShellAndContextSwitch => 12,
            Self::HeadersAndSections => 13,
            Self::StripPanelAndFooter => 14,
        }
    }

    /// The page one step before this one, or `None` at the first page.
    ///
    /// Non-wrapping, matching the nonwrapping movement the product uses
    /// everywhere else (`DESIGN.md:309`). Returning `None` at the end rather
    /// than the same page is what lets the caller report *retained* instead of
    /// *changed*, so a step that did nothing is visible as a step that did
    /// nothing.
    pub fn previous(self) -> Option<Self> {
        self.index()
            .checked_sub(1)
            .map(|index| ALL_GALLERY_PAGES[index])
    }

    /// The page one step after this one, or `None` at the last page.
    pub fn next(self) -> Option<Self> {
        ALL_GALLERY_PAGES.get(self.index() + 1).copied()
    }
}

/// How the previous-page key reads on screen.
pub const STEP_PREVIOUS_LABEL: &str = "[";

/// How the next-page key reads on screen.
pub const STEP_NEXT_LABEL: &str = "]";

/// What the page index shows beside a page no digit selects.
///
/// The two bracket glyphs together, so the index line says how to reach the
/// page rather than leaving a blank where the other ten carry a number.
pub const STEP_ONLY_LABEL: &str = "[ ]";

/// Which way a scene-local step moves through the declared page order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageStep {
    /// Toward the first page.
    Previous,
    /// Toward the last page.
    Next,
}

impl PageStep {
    /// The key that asks for this step.
    pub const fn key(self) -> WindowKey {
        match self {
            Self::Previous => WindowKey::BracketLeft,
            Self::Next => WindowKey::BracketRight,
        }
    }

    /// The step a key asks for, or `None` when the key steps nothing.
    pub fn for_key(key: WindowKey) -> Option<Self> {
        [Self::Previous, Self::Next]
            .into_iter()
            .find(|step| step.key() == key)
    }

    /// Applies this step to `page`, or `None` at the end it points at.
    pub fn apply(self, page: ComponentGalleryPage) -> Option<ComponentGalleryPage> {
        match self {
            Self::Previous => page.previous(),
            Self::Next => page.next(),
        }
    }
}

/// What a consumed input did to the active page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageSelection {
    /// A bound digit selected this page.
    Changed(ComponentGalleryPage),
    /// A step key moved to this page.
    Stepped(ComponentGalleryPage),
    /// The input bound no page — an unbound key, or a step past an end — so the
    /// current one was kept.
    Retained(ComponentGalleryPage),
}

impl PageSelection {
    /// The page that is active after the input.
    pub const fn page(self) -> ComponentGalleryPage {
        match self {
            Self::Changed(page) | Self::Stepped(page) | Self::Retained(page) => page,
        }
    }
}

/// The scene's page selection.
///
/// This is the entire mutable state paging owns. It is not an `AppState`, it
/// never becomes one, and no value here is projected, persisted, or sent to
/// audio.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GalleryPageSelection {
    active: ComponentGalleryPage,
}

impl Default for GalleryPageSelection {
    fn default() -> Self {
        Self {
            active: ALL_GALLERY_PAGES[0],
        }
    }
}

impl GalleryPageSelection {
    /// The page currently on screen.
    pub const fn active(self) -> ComponentGalleryPage {
        self.active
    }

    /// Applies one normalized window input.
    ///
    /// Only a key-*down* on a bound digit or a step key moves the page. A
    /// key-up, a focus loss, and any key that binds neither retain the current
    /// page and change nothing else.
    ///
    /// Stepping does not wrap. At the first page a previous-step retains the
    /// first page, and at the last a next-step retains the last, which is the
    /// nonwrapping movement the product uses everywhere else
    /// (`DESIGN.md:309`). A wrapping step would let an operator holding one
    /// bracket cycle forever without ever learning they had reached an end.
    pub fn apply(&mut self, input: WindowInput) -> PageSelection {
        if input.kind() != WindowInputKind::KeyDown {
            return PageSelection::Retained(self.active);
        }
        if let Some(page) = ComponentGalleryPage::for_digit(input.key()) {
            self.active = page;
            return PageSelection::Changed(page);
        }
        match PageStep::for_key(input.key()).and_then(|step| step.apply(self.active)) {
            Some(page) => {
                self.active = page;
                PageSelection::Stepped(page)
            }
            None => PageSelection::Retained(self.active),
        }
    }
}

/// What one painted state announced without using color.
///
/// Recorded from the paint pass rather than from the state's declaration: this
/// is what actually reached the screen.
#[derive(Clone, Debug, PartialEq)]
struct PaintedStateEvidence {
    keyline_px: f32,
    halo: bool,
    row_fill: bool,
    cursor: bool,
    mark: Option<String>,
}

/// One painted state with the label a reader sees beside it.
#[derive(Clone, Debug, PartialEq)]
struct PaintedState {
    evidence: PaintedStateEvidence,
    visible_label: String,
}

/// One control identity that reached the screen, with its visible evidence.
///
/// Built from the text runs the control's own render function emitted, read
/// back out of the layer it painted into. There is no path here from a
/// specimen list: a control that was constructed and not painted has no runs,
/// so it has no record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PaintedControlRecord {
    control: &'static str,
    kind: &'static str,
    role: &'static str,
    states_painted: usize,
    states_declared: usize,
    visible_label: String,
    viewports: Vec<&'static str>,
}

impl PaintedControlRecord {
    /// The canonical control name.
    pub fn control(&self) -> &str {
        self.control
    }

    /// The semantic control kind the specimen was asked as.
    pub fn kind(&self) -> &str {
        self.kind
    }

    /// The presentation role the specimen was asked in.
    pub fn role(&self) -> &str {
        self.role
    }

    /// How many of this control's declared states painted at every policy.
    pub const fn states_painted(&self) -> usize {
        self.states_painted
    }

    /// How many states this control declares applicable.
    pub const fn states_declared(&self) -> usize {
        self.states_declared
    }

    /// A label a reader actually sees on this control's specimen.
    pub fn visible_label(&self) -> &str {
        &self.visible_label
    }

    /// The density policies whose composition painted this control.
    pub fn viewports(&self) -> &[&'static str] {
        &self.viewports
    }
}

/// One composition identity that reached the screen, with its visible evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PaintedCompositionRecord {
    composition: &'static str,
    region: &'static str,
    text_runs: usize,
    visible_label: String,
    viewports: Vec<&'static str>,
}

impl PaintedCompositionRecord {
    /// The canonical composition name.
    pub fn composition(&self) -> &str {
        self.composition
    }

    /// The shell region this composition fills.
    pub fn region(&self) -> &str {
        self.region
    }

    /// How many text runs its arrangement emitted, summed over the policies.
    pub const fn text_runs(&self) -> usize {
        self.text_runs
    }

    /// A label a reader actually sees inside this composition's specimen.
    pub fn visible_label(&self) -> &str {
        &self.visible_label
    }

    /// The density policies whose composition painted it.
    pub fn viewports(&self) -> &[&'static str] {
        &self.viewports
    }
}

/// One state identity that reached the screen, with its visible evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PaintedStateRecord {
    state: &'static str,
    visible_label: String,
    non_color_evidence: String,
    viewports: Vec<&'static str>,
}

impl PaintedStateRecord {
    /// The canonical state name.
    pub fn state(&self) -> &str {
        self.state
    }

    /// The label painted beside the specimen.
    pub fn visible_label(&self) -> &str {
        &self.visible_label
    }

    /// What announced the state without color.
    pub fn non_color_evidence(&self) -> &str {
        &self.non_color_evidence
    }

    /// The density policies whose composition painted this state.
    pub fn viewports(&self) -> &[&'static str] {
        &self.viewports
    }
}

/// Everything the paint pass emitted, accumulated across the session.
///
/// Nothing writes to this except the painting code below, and every counter is
/// incremented after the corresponding paint command is issued.
#[derive(Debug)]
struct GalleryPaintLedger {
    pages_painted: [bool; GALLERY_PAGE_COUNT],
    pages_visited: Vec<ComponentGalleryPage>,
    digit_requests: [bool; GALLERY_PAGE_COUNT],
    pages_reached_by_digit: [bool; GALLERY_PAGE_COUNT],
    step_requests: [bool; GALLERY_PAGE_COUNT],
    pages_reached_by_step: [bool; GALLERY_PAGE_COUNT],
    states_painted: [[Option<PaintedState>; COMPONENT_STATE_COUNT]; ALL_DENSITY_POLICIES.len()],
    /// The visible label each control painted, per policy, per declared state.
    ///
    /// Indexed rather than tallied for the same reason the states are: a flat
    /// count cannot tell a control painted in both compositions from one
    /// painted twice in the roomier column, and it cannot tell seven states
    /// from the same state seven times.
    controls_painted: [[[Option<String>; COMPONENT_STATE_COUNT]; COMPONENT_CONTROL_COUNT];
        ALL_DENSITY_POLICIES.len()],
    /// The `(kind, role)` pair each control's specimen was asked as.
    control_pairs: [Option<(&'static str, &'static str)>; COMPONENT_CONTROL_COUNT],
    /// Every text run each composition's arrangement emitted, per policy.
    compositions_painted:
        [[Option<Vec<String>>; SHELL_COMPOSITION_COUNT]; ALL_DENSITY_POLICIES.len()],
    viewports_painted: [bool; ALL_DENSITY_POLICIES.len()],
    bands_painted: [[bool; ShellRegionId::ALL.len()]; ALL_DENSITY_POLICIES.len()],
    painted_colors: BTreeSet<[u8; 4]>,
    text_runs: usize,
    unresolved_text_runs: usize,
    text_defects: BTreeSet<String>,
    unbound_key_presses: usize,
    unbound_key_page_changes: usize,
    active_page: ComponentGalleryPage,
    viewport: Vec2,
}

impl Default for GalleryPaintLedger {
    fn default() -> Self {
        Self {
            pages_painted: [false; GALLERY_PAGE_COUNT],
            pages_visited: Vec::new(),
            digit_requests: [false; GALLERY_PAGE_COUNT],
            pages_reached_by_digit: [false; GALLERY_PAGE_COUNT],
            step_requests: [false; GALLERY_PAGE_COUNT],
            pages_reached_by_step: [false; GALLERY_PAGE_COUNT],
            states_painted: std::array::from_fn(|_| std::array::from_fn(|_| None)),
            controls_painted: std::array::from_fn(|_| {
                std::array::from_fn(|_| std::array::from_fn(|_| None))
            }),
            control_pairs: std::array::from_fn(|_| None),
            compositions_painted: std::array::from_fn(|_| std::array::from_fn(|_| None)),
            viewports_painted: [false; ALL_DENSITY_POLICIES.len()],
            bands_painted: [[false; ShellRegionId::ALL.len()]; ALL_DENSITY_POLICIES.len()],
            painted_colors: BTreeSet::new(),
            text_runs: 0,
            unresolved_text_runs: 0,
            text_defects: BTreeSet::new(),
            unbound_key_presses: 0,
            unbound_key_page_changes: 0,
            active_page: ALL_GALLERY_PAGES[0],
            viewport: Vec2::ZERO,
        }
    }
}

impl GalleryPaintLedger {
    /// Records that a bound digit asked for this page.
    ///
    /// The page is not counted as reached until it also paints; the request
    /// alone proves only that a key was pressed.
    fn record_digit_request(&mut self, page: ComponentGalleryPage) {
        self.digit_requests[page.index()] = true;
    }

    /// Records that a step key asked for this page.
    ///
    /// Same contract as the digit: a step that landed on a page which then
    /// failed to paint is not a page the operator reached.
    fn record_step_request(&mut self, page: ComponentGalleryPage) {
        self.step_requests[page.index()] = true;
    }

    /// Records one key press that bound no page.
    ///
    /// With ten digits bound to ten pages there is no longer an unbound
    /// *digit*, so what this counts is a key press that resolved to no page and
    /// no step — which is the property the observation's
    /// `unbound_digit_retained_page` field exists to establish. A step that ran
    /// into an end is deliberately not counted here: it is a bound key that
    /// correctly declined to move, not a key that binds nothing.
    fn record_unbound_key(&mut self, changed_page: bool) {
        self.unbound_key_presses += 1;
        if changed_page {
            self.unbound_key_page_changes += 1;
        }
    }

    /// Records that `page` finished a paint pass that emitted `runs` text runs.
    ///
    /// A pass that emitted nothing is not a painted page.
    fn record_painted_page(&mut self, page: ComponentGalleryPage, runs: usize) {
        if runs == 0 {
            return;
        }
        if !self.pages_painted[page.index()] {
            self.pages_painted[page.index()] = true;
            self.pages_visited.push(page);
        }
        if self.digit_requests[page.index()] {
            self.pages_reached_by_digit[page.index()] = true;
        }
        if self.step_requests[page.index()] {
            self.pages_reached_by_step[page.index()] = true;
        }
    }

    fn painted_page_count(&self) -> usize {
        self.pages_painted
            .iter()
            .filter(|painted| **painted)
            .count()
    }

    fn digit_reached_page_count(&self) -> usize {
        self.pages_reached_by_digit
            .iter()
            .filter(|reached| **reached)
            .count()
    }

    fn step_reached_page_count(&self) -> usize {
        self.pages_reached_by_step
            .iter()
            .filter(|reached| **reached)
            .count()
    }

    /// The visible label one control painted for one state at one policy.
    fn control_painted_at(
        &self,
        policy: ViewportDensityPolicy,
        control: ComponentControl,
        state: ComponentState,
    ) -> Option<&String> {
        self.controls_painted[policy_index(policy)][control_index(control)][state_index(state)]
            .as_ref()
    }

    /// Whether one control painted every state it declares, at every policy.
    ///
    /// The strong form on purpose. A control counted for painting one state
    /// would let the gallery claim coverage it does not have, and F-01 —
    /// three controls with no authored specimen — is exactly the case where an
    /// operator needs the count to mean what it says.
    fn control_fully_painted(&self, control: ComponentControl) -> bool {
        ALL_DENSITY_POLICIES.into_iter().all(|policy| {
            control
                .applicable_states()
                .iter()
                .all(|state| self.control_painted_at(policy, control, *state).is_some())
        })
    }

    fn painted_control_count(&self) -> usize {
        ALL_COMPONENT_CONTROLS
            .into_iter()
            .filter(|control| self.control_fully_painted(*control))
            .count()
    }

    /// Every text run one composition emitted at one policy.
    fn composition_painted_at(
        &self,
        policy: ViewportDensityPolicy,
        composition: ShellComposition,
    ) -> Option<&Vec<String>> {
        self.compositions_painted[policy_index(policy)][composition_index(composition)].as_ref()
    }

    /// Whether one composition emitted at least one text run at every policy.
    fn composition_fully_painted(&self, composition: ShellComposition) -> bool {
        ALL_DENSITY_POLICIES.into_iter().all(|policy| {
            self.composition_painted_at(policy, composition)
                .is_some_and(|runs| !runs.is_empty())
        })
    }

    fn painted_composition_count(&self) -> usize {
        ALL_SHELL_COMPOSITIONS
            .into_iter()
            .filter(|composition| self.composition_fully_painted(*composition))
            .count()
    }

    /// How many `(kind, role)` pairs the gallery could not put on screen.
    ///
    /// A pair is mapped when it either resolves to a control whose specimen
    /// this session actually painted, or is an explicit
    /// `ControlSelection::NotAskableInRole` — which the control family
    /// documents as a decision rather than a fall-through. A pair resolving to
    /// a control with no painted specimen is the case this counts, because that
    /// is a pair an operator cannot see the answer to.
    ///
    /// Measured against the paint ledger rather than against `control_for`
    /// alone, because a count derived only from a total `match` is a count that
    /// cannot fail.
    fn unmapped_kind_role_pairs(&self) -> usize {
        ALL_SEMANTIC_CONTROL_KINDS
            .into_iter()
            .flat_map(|kind| {
                ALL_PRESENTATION_ROLES
                    .into_iter()
                    .map(move |role| (kind, role))
            })
            .filter(|(kind, role)| {
                control_for(*kind, *role)
                    .control()
                    .is_some_and(|control| !self.control_fully_painted(control))
            })
            .count()
    }

    /// How many declared controls no `(kind, role)` pair resolves to.
    ///
    /// A control nothing can ask for is dead code the gallery would still
    /// happily paint, so this is asked of the selection function rather than of
    /// the ledger.
    fn controls_unreachable_by_any_pair(&self) -> usize {
        ALL_COMPONENT_CONTROLS
            .into_iter()
            .filter(|control| {
                !ALL_SEMANTIC_CONTROL_KINDS.into_iter().any(|kind| {
                    ALL_PRESENTATION_ROLES
                        .into_iter()
                        .any(|role| control_for(kind, role).control() == Some(*control))
                })
            })
            .count()
    }

    /// The controls that painted every declared state at every policy.
    fn controls_rendered(&self) -> Vec<PaintedControlRecord> {
        ALL_COMPONENT_CONTROLS
            .into_iter()
            .filter_map(|control| {
                if !self.control_fully_painted(control) {
                    return None;
                }
                let (kind, role) = self.control_pairs[control_index(control)]?;
                let states = control.applicable_states();
                let visible_label = states
                    .iter()
                    .find_map(|state| {
                        self.control_painted_at(ALL_DENSITY_POLICIES[0], control, *state)
                    })
                    .cloned()?;
                Some(PaintedControlRecord {
                    control: control.canonical_name(),
                    kind,
                    role,
                    states_painted: states.len(),
                    states_declared: states.len(),
                    visible_label,
                    viewports: ALL_DENSITY_POLICIES
                        .into_iter()
                        .map(ViewportDensityPolicy::canonical_name)
                        .collect(),
                })
            })
            .collect()
    }

    /// The compositions that emitted runs at every policy.
    fn compositions_rendered(&self) -> Vec<PaintedCompositionRecord> {
        ALL_SHELL_COMPOSITIONS
            .into_iter()
            .filter_map(|composition| {
                if !self.composition_fully_painted(composition) {
                    return None;
                }
                let text_runs = ALL_DENSITY_POLICIES
                    .into_iter()
                    .filter_map(|policy| self.composition_painted_at(policy, composition))
                    .map(Vec::len)
                    .sum();
                let visible_label = self
                    .composition_painted_at(ALL_DENSITY_POLICIES[0], composition)
                    .and_then(|runs| runs.first())
                    .cloned()?;
                Some(PaintedCompositionRecord {
                    composition: composition.canonical_name(),
                    region: region_name(composition),
                    text_runs,
                    visible_label,
                    viewports: ALL_DENSITY_POLICIES
                        .into_iter()
                        .map(ViewportDensityPolicy::canonical_name)
                        .collect(),
                })
            })
            .collect()
    }

    /// What one state's specimen emitted at one density policy.
    fn state_painted_at(
        &self,
        policy: ViewportDensityPolicy,
        state: ComponentState,
    ) -> Option<&PaintedState> {
        self.states_painted[policy_index(policy)][state_index(state)].as_ref()
    }

    /// How many states painted a specimen at *every* declared policy.
    ///
    /// NFR-005 asks for a specimen at both authored sizes, so a state that
    /// reached only the wider composition is not counted. Recording per policy
    /// is what makes that answerable: a flat tally cannot tell a state painted
    /// twice from a state painted once in the roomier column.
    fn painted_state_count(&self) -> usize {
        ALL_COMPONENT_STATES
            .into_iter()
            .filter(|state| {
                ALL_DENSITY_POLICIES
                    .into_iter()
                    .all(|policy| self.state_painted_at(policy, *state).is_some())
            })
            .count()
    }

    /// Whether no two painted states shared their colorless evidence.
    ///
    /// Compared over what was painted, not over what the vocabulary declares: a
    /// specimen that dropped its status mark fails here even though the
    /// declaration still carries one. Each composition is compared against
    /// itself, because the same state legitimately paints once per policy and
    /// comparing across policies would call that a collision.
    fn states_distinguishable_without_color(&self) -> bool {
        ALL_DENSITY_POLICIES.into_iter().all(|policy| {
            let painted: Vec<&PaintedState> = self.states_painted[policy_index(policy)]
                .iter()
                .flatten()
                .collect();
            painted.iter().enumerate().all(|(index, first)| {
                painted
                    .iter()
                    .skip(index + 1)
                    .all(|second| first.evidence != second.evidence)
            })
        })
    }

    /// Whether every color that reached the screen resolves through the
    /// vocabulary — either as an authored role or as that role at the authored
    /// halo opacity.
    fn token_source_exact(&self) -> bool {
        !self.painted_colors.is_empty()
            && self.painted_colors.iter().all(|painted| {
                let painted = Color32::from_rgba_premultiplied(
                    painted[0], painted[1], painted[2], painted[3],
                );
                ALL_COLORS
                    .into_iter()
                    .any(|role| role.resolve() == painted || focus::halo_color(role) == painted)
            })
    }

    /// Whether every painted text run resolved to a registered authored face.
    fn typeface_resolved(&self) -> bool {
        self.text_runs > 0 && self.unresolved_text_runs == 0
    }

    fn viewport_painted(&self, policy: ViewportDensityPolicy) -> bool {
        self.viewports_painted[policy_index(policy)]
    }

    fn bands_retained_both_viewports(&self) -> bool {
        ALL_DENSITY_POLICIES.into_iter().all(|policy| {
            self.bands_painted[policy_index(policy)]
                .iter()
                .all(|painted| *painted)
        })
    }

    /// The states that painted a specimen at every declared policy, naming the
    /// compositions each one reached.
    ///
    /// A state that reached only one composition is deliberately absent rather
    /// than listed with a shorter viewport list: the count beside it in the
    /// observation must mean the same thing this list does.
    fn states_rendered(&self) -> Vec<PaintedStateRecord> {
        ALL_COMPONENT_STATES
            .into_iter()
            .filter_map(|state| {
                let painted: Vec<(ViewportDensityPolicy, &PaintedState)> = ALL_DENSITY_POLICIES
                    .into_iter()
                    .filter_map(|policy| {
                        self.state_painted_at(policy, state)
                            .map(|record| (policy, record))
                    })
                    .collect();
                if painted.len() < ALL_DENSITY_POLICIES.len() {
                    return None;
                }
                let (_, first) = painted[0];
                Some(PaintedStateRecord {
                    state: state.canonical_name(),
                    visible_label: first.visible_label.clone(),
                    non_color_evidence: describe_evidence(&first.evidence),
                    viewports: painted
                        .iter()
                        .map(|(policy, _)| policy.canonical_name())
                        .collect(),
                })
            })
            .collect()
    }
}

/// Renders one painted state's colorless evidence as readable text.
fn describe_evidence(evidence: &PaintedStateEvidence) -> String {
    let mut parts = vec![format!("keyline {} px", evidence.keyline_px)];
    if evidence.halo {
        parts.push("halo".to_owned());
    }
    if evidence.row_fill {
        parts.push("row fill".to_owned());
    }
    if evidence.cursor {
        parts.push(format!("cursor {}", focus::CURSOR_GLYPH));
    }
    if let Some(mark) = &evidence.mark {
        parts.push(format!("mark {mark}"));
    }
    parts.join(hint::HINT_SEPARATOR)
}

/// A policy's position in [`ALL_DENSITY_POLICIES`].
const fn policy_index(policy: ViewportDensityPolicy) -> usize {
    match policy {
        ViewportDensityPolicy::Desktop => 0,
        ViewportDensityPolicy::SteamDeck => 1,
    }
}

/// A control's position in [`ALL_COMPONENT_CONTROLS`].
///
/// Exhaustive with no wildcard, so a ninth control is a compile error naming
/// this function rather than a specimen the ledger silently indexes past.
const fn control_index(control: ComponentControl) -> usize {
    match control {
        ComponentControl::ParameterRow => 0,
        ComponentControl::ChoiceRow => 1,
        ComponentControl::Toggle => 2,
        ComponentControl::CompactSlider => 3,
        ComponentControl::Fader => 4,
        ComponentControl::Meter => 5,
        ComponentControl::BrowserRow => 6,
        ComponentControl::ModalOption => 7,
    }
}

/// A composition's position in [`ALL_SHELL_COMPOSITIONS`].
const fn composition_index(composition: ShellComposition) -> usize {
    match composition {
        ShellComposition::ApplicationShell => 0,
        ShellComposition::ContextSwitch => 1,
        ShellComposition::IdentityHeader => 2,
        ShellComposition::Section => 3,
        ShellComposition::PatchStripRow => 4,
        ShellComposition::MixerStripBank => 5,
        ShellComposition::UtilityInspectorPanel => 6,
        ShellComposition::Footer => 7,
    }
}

/// The shell region a composition fills, named as the observation names it.
const fn region_name(composition: ShellComposition) -> &'static str {
    match composition.region() {
        ShellRegion::WholeFrame => "wholeFrame",
        ShellRegion::ContextLine => "contextLine",
        ShellRegion::IdentityHeader => "identityHeader",
        ShellRegion::MainWorkspace => "mainWorkspace",
        ShellRegion::PersistentSideRegion => "persistentSideRegion",
        ShellRegion::Footer => "footer",
    }
}

/// The canonical name of one semantic control kind.
///
/// The kind vocabulary is owned by the control layer; this names it for the
/// observation without the observation depending on a `Debug` format.
const fn kind_name(kind: SemanticControlKind) -> &'static str {
    match kind {
        SemanticControlKind::Continuous => "Continuous",
        SemanticControlKind::Stepped => "Stepped",
        SemanticControlKind::Choice => "Choice",
        SemanticControlKind::Toggle => "Toggle",
        SemanticControlKind::Asset => "Asset",
        SemanticControlKind::Identity => "Identity",
        SemanticControlKind::Surface => "Surface",
    }
}

/// A state's position in [`ALL_COMPONENT_STATES`].
const fn state_index(state: ComponentState) -> usize {
    match state {
        ComponentState::Resting => 0,
        ComponentState::Focused => 1,
        ComponentState::Adjusting => 2,
        ComponentState::Disabled => 3,
        ComponentState::Loading => 4,
        ComponentState::Error => 5,
        ComponentState::Muted => 6,
        ComponentState::Soloed => 7,
        ComponentState::Selected => 8,
    }
}

/// One immutable observation of the gallery scene actually painted.
///
/// Constructed only from a [`GalleryPaintLedger`] that a paint pass filled in.
/// There is no constructor that accepts an expected page set, a specimen list,
/// or a coverage plan, which is what keeps this evidence rather than assertion.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ComponentGalleryObservation {
    pages_declared: usize,
    pages_painted: usize,
    pages_reachable_by_digit: usize,
    pages_reachable_by_step: usize,
    unbound_digit_retained_page: bool,
    states_declared: usize,
    states_painted: usize,
    states_distinguishable_without_color: bool,
    controls_declared: usize,
    controls_painted: usize,
    kind_role_pairs_unmapped: usize,
    controls_unreachable_by_any_pair: usize,
    compositions_declared: usize,
    compositions_painted: usize,
    desktop_viewport_painted: bool,
    steam_deck_viewport_painted: bool,
    bands_retained_both_viewports: bool,
    clipped_or_overlapping_text: usize,
    token_source_exact: bool,
    typeface_resolved: bool,
    audio_or_midi_constructed: bool,
    app_state_generation_delta: i64,
    window_closed: bool,
    viewport_width: f32,
    viewport_height: f32,
    active_page: &'static str,
    pages_visited: Vec<&'static str>,
    states_rendered: Vec<PaintedStateRecord>,
    controls_rendered: Vec<PaintedControlRecord>,
    compositions_rendered: Vec<PaintedCompositionRecord>,
    text_runs_painted: usize,
    unbound_key_presses: usize,
    text_defects: Vec<String>,
}

impl ComponentGalleryObservation {
    /// Builds the observation from what the paint pass emitted.
    fn from_paint(
        ledger: &GalleryPaintLedger,
        app_state_generation_delta: i64,
        window_closed: bool,
    ) -> Self {
        Self {
            pages_declared: GALLERY_PAGE_COUNT,
            pages_painted: ledger.painted_page_count(),
            pages_reachable_by_digit: ledger.digit_reached_page_count(),
            pages_reachable_by_step: ledger.step_reached_page_count(),
            unbound_digit_retained_page: ledger.unbound_key_presses > 0
                && ledger.unbound_key_page_changes == 0,
            states_declared: COMPONENT_STATE_COUNT,
            states_painted: ledger.painted_state_count(),
            states_distinguishable_without_color: ledger.states_distinguishable_without_color(),
            controls_declared: COMPONENT_CONTROL_COUNT,
            controls_painted: ledger.painted_control_count(),
            kind_role_pairs_unmapped: ledger.unmapped_kind_role_pairs(),
            controls_unreachable_by_any_pair: ledger.controls_unreachable_by_any_pair(),
            compositions_declared: SHELL_COMPOSITION_COUNT,
            compositions_painted: ledger.painted_composition_count(),
            desktop_viewport_painted: ledger.viewport_painted(ViewportDensityPolicy::Desktop),
            steam_deck_viewport_painted: ledger.viewport_painted(ViewportDensityPolicy::SteamDeck),
            bands_retained_both_viewports: ledger.bands_retained_both_viewports(),
            clipped_or_overlapping_text: ledger.text_defects.len(),
            token_source_exact: ledger.token_source_exact(),
            typeface_resolved: ledger.typeface_resolved(),
            audio_or_midi_constructed: audio_or_midi_constructed(),
            app_state_generation_delta,
            window_closed,
            viewport_width: ledger.viewport.x,
            viewport_height: ledger.viewport.y,
            active_page: ledger.active_page.canonical_name(),
            pages_visited: ledger
                .pages_visited
                .iter()
                .map(|page| page.canonical_name())
                .collect(),
            states_rendered: ledger.states_rendered(),
            controls_rendered: ledger.controls_rendered(),
            compositions_rendered: ledger.compositions_rendered(),
            text_runs_painted: ledger.text_runs,
            unbound_key_presses: ledger.unbound_key_presses,
            text_defects: ledger.text_defects.iter().cloned().collect(),
        }
    }

    /// How many pages a step key brought on screen.
    pub const fn pages_reachable_by_step(&self) -> usize {
        self.pages_reachable_by_step
    }

    /// How many controls the family declares.
    pub const fn controls_declared(&self) -> usize {
        self.controls_declared
    }

    /// How many controls painted every state they declare, at both policies.
    pub const fn controls_painted(&self) -> usize {
        self.controls_painted
    }

    /// How many `(kind, role)` pairs the gallery could not put on screen.
    pub const fn kind_role_pairs_unmapped(&self) -> usize {
        self.kind_role_pairs_unmapped
    }

    /// How many declared controls no pair resolves to.
    pub const fn controls_unreachable_by_any_pair(&self) -> usize {
        self.controls_unreachable_by_any_pair
    }

    /// How many compositions the family declares.
    pub const fn compositions_declared(&self) -> usize {
        self.compositions_declared
    }

    /// How many compositions emitted runs at both policies.
    pub const fn compositions_painted(&self) -> usize {
        self.compositions_painted
    }

    /// Whether the scene constructed an audio output or a MIDI event source.
    ///
    /// Derived from what the scene is built out of, never declared. See
    /// [`audio_or_midi_constructed`].
    pub const fn audio_or_midi_constructed(&self) -> bool {
        self.audio_or_midi_constructed
    }

    /// The controls that reached the screen, with their visible evidence.
    pub fn controls_rendered(&self) -> &[PaintedControlRecord] {
        &self.controls_rendered
    }

    /// The compositions that reached the screen, with their visible evidence.
    pub fn compositions_rendered(&self) -> &[PaintedCompositionRecord] {
        &self.compositions_rendered
    }

    /// How many pages the vocabulary declares.
    pub const fn pages_declared(&self) -> usize {
        self.pages_declared
    }

    /// How many distinct pages actually painted.
    pub const fn pages_painted(&self) -> usize {
        self.pages_painted
    }

    /// How many pages a digit press brought on screen.
    pub const fn pages_reachable_by_digit(&self) -> usize {
        self.pages_reachable_by_digit
    }

    /// Whether a key binding no page kept the current one.
    pub const fn unbound_digit_retained_page(&self) -> bool {
        self.unbound_digit_retained_page
    }

    /// How many states the vocabulary declares.
    pub const fn states_declared(&self) -> usize {
        self.states_declared
    }

    /// How many distinct states actually painted.
    pub const fn states_painted(&self) -> usize {
        self.states_painted
    }

    /// Whether no two painted states shared their colorless evidence.
    pub const fn states_distinguishable_without_color(&self) -> bool {
        self.states_distinguishable_without_color
    }

    /// Whether the desktop composition painted.
    pub const fn desktop_viewport_painted(&self) -> bool {
        self.desktop_viewport_painted
    }

    /// Whether the Steam Deck composition painted.
    pub const fn steam_deck_viewport_painted(&self) -> bool {
        self.steam_deck_viewport_painted
    }

    /// Whether all five structural regions painted at both policies.
    pub const fn bands_retained_both_viewports(&self) -> bool {
        self.bands_retained_both_viewports
    }

    /// How many painted text runs clipped their column or overlapped another.
    pub const fn clipped_or_overlapping_text(&self) -> usize {
        self.clipped_or_overlapping_text
    }

    /// Whether every painted color resolved through the vocabulary.
    pub const fn token_source_exact(&self) -> bool {
        self.token_source_exact
    }

    /// Whether every painted text run resolved to the authored typeface.
    pub const fn typeface_resolved(&self) -> bool {
        self.typeface_resolved
    }

    /// How far the production reducer advanced while the gallery was open.
    pub const fn app_state_generation_delta(&self) -> i64 {
        self.app_state_generation_delta
    }

    /// Whether the window closed and released what it owned.
    pub const fn window_closed(&self) -> bool {
        self.window_closed
    }

    /// The states that reached the screen, with their visible evidence.
    pub fn states_rendered(&self) -> &[PaintedStateRecord] {
        &self.states_rendered
    }

    /// The pages that reached the screen, in the order they were first painted.
    pub fn pages_visited(&self) -> &[&'static str] {
        &self.pages_visited
    }

    /// The specimens that clipped or overlapped, so a failure is actionable.
    pub fn text_defects(&self) -> &[String] {
        &self.text_defects
    }

    /// How many text runs the paint pass emitted in total.
    pub const fn text_runs_painted(&self) -> usize {
        self.text_runs_painted
    }
}

// ===========================================================================
// The silence is derived, not declared
// ===========================================================================

/// This module's own production source, embedded at compile time.
///
/// Embedded rather than read from disk so the derivation below works in a
/// shipped binary and needs no filesystem at demo time — the same reason the
/// authored typeface is vendored rather than looked up.
const GALLERY_SCENE_SOURCE: &str = include_str!("component_gallery_scene.rs");

/// Whether the gallery scene constructs an audio output or a MIDI event source.
///
/// **Derived, not declared.** A hard-coded `false` satisfies the witness
/// predicate and proves nothing: it would keep reporting silence the day
/// somebody wired a stream into this scene. So the answer is computed from what
/// this module is actually built out of — its own production source is searched
/// for any mention of the types that *are* the audio output and the MIDI event
/// source in this product, and the flag is true if it finds one.
///
/// That makes it falsifiable in the only way that matters: adding an audio
/// construction to this scene flips the flag to `true` and the predicate fails,
/// which is exactly the report an operator would want. The test below proves
/// the search is capable of returning `true`, so a scan that had quietly stopped
/// matching anything cannot pass as silence.
///
/// Only the part of the file before the first test module is searched. A test
/// that *names* an audio type — as the one proving this search works must —
/// ships in no binary and constructs nothing at demo time.
fn audio_or_midi_constructed() -> bool {
    source_constructs_audio_or_midi(&production_source(GALLERY_SCENE_SOURCE))
}

/// The part of a source file that ships, with its prose removed.
///
/// Comments are stripped for the same reason the control family strips them
/// before its own boundary scans: prose that *names* the boundary must not be
/// mistaken for code that crosses it. This module's own explanation of why the
/// gallery constructs no MIDI source has to be able to say "MIDI source".
fn production_source(source: &str) -> String {
    let marker = format!("#[cfg({})]", "test");
    let shipping = match source.find(&marker) {
        Some(offset) => &source[..offset],
        None => source,
    };
    shipping
        .lines()
        .map(|line| match line.find("//") {
            Some(offset) => &line[..offset],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `source` names anything that makes sound or reads MIDI.
///
/// The needles are assembled at run time so that this function's own source
/// does not match them; the project already uses that shape for its
/// `SemanticAction` and visual-literal guards.
///
/// The split points are chosen so that **no fragment spelled here is itself one
/// of the needles**. That is not cosmetic: the first version of this function
/// spelled `"MidiEventSource"` as one half of `CorridorsMidiEventSource`, and
/// the search duly found that literal in its own source and reported the gallery
/// as constructing a MIDI source. The tests below are what caught it, and they
/// are what will catch the next bad split — a scene that is genuinely silent
/// reports `false`, so a self-match shows up as a failure rather than as
/// caution.
fn source_constructs_audio_or_midi(source: &str) -> bool {
    let needles = [
        // The two production ports the standalone application constructs.
        format!("{}{}", "Cpal", "AudioOutput"),
        format!("{}{}", "CorridorsMidi", "EventSource"),
        // The port traits themselves, so a scene reaching for either through an
        // abstraction is caught as well as one naming a concrete adapter.
        format!("{}{}", "Audio", "OutputPort"),
        format!("{}{}", "MidiEvent", "Source"),
        // What a prepared stream, a renderer, and a note event are called here.
        format!("{}{}", "Prepared", "Graph"),
        format!("{}{}", "Audio", "Renderer"),
        format!("{}{}", "Midi", "Message"),
    ];
    needles
        .iter()
        .any(|needle| source.contains(needle.as_str()))
}

/// A failure opening or running the gallery.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ComponentGalleryError {
    /// An authored face is unavailable, so no frame may claim the design.
    #[error("authored typeface unavailable: {0}")]
    Typeface(TypefaceError),
    /// The production capability registry could not be constructed, so there is
    /// no reducer to measure the generation delta against.
    #[error("gallery reducer witness unavailable: {0}")]
    Capability(CapabilityError),
    /// The production projector could not derive the content the control and
    /// composition specimens read from, so the gallery would have nothing real
    /// to show them with.
    #[error("gallery specimen projection unavailable: {0}")]
    Projection(String),
    /// The window itself failed.
    #[error("component gallery window failed: {0}")]
    Window(String),
}

/// The one painting surface the gallery uses.
///
/// Every command that reaches the screen goes through a method here, and every
/// method records what it emitted before returning. Painting and counting are
/// the same call, so there is no way to increment a counter without painting and
/// no way to paint without incrementing one.
struct SpecimenPainter<'a> {
    painter: &'a egui::Painter,
    context: &'a egui::Context,
    registered_families: &'a [FontFamily],
    ledger: &'a mut GalleryPaintLedger,
    clip: Rect,
    column_runs: Vec<(Rect, String)>,
}

impl<'a> SpecimenPainter<'a> {
    fn new(
        painter: &'a egui::Painter,
        context: &'a egui::Context,
        registered_families: &'a [FontFamily],
        ledger: &'a mut GalleryPaintLedger,
        clip: Rect,
    ) -> Self {
        Self {
            painter,
            context,
            registered_families,
            ledger,
            clip,
            column_runs: Vec::new(),
        }
    }

    /// Starts measuring a new region's text against `clip`.
    ///
    /// The runs collected so far are evaluated first, so no region inherits
    /// another's overlap checks.
    fn begin_region(&mut self, clip: Rect) {
        self.finish_region();
        self.clip = clip;
    }

    /// Evaluates the current region's text for clipping and overlap.
    ///
    /// A defect is recorded once by name however many frames it survives, so the
    /// reported count is the number of distinct broken specimens rather than a
    /// frame tally.
    fn finish_region(&mut self) {
        let runs = std::mem::take(&mut self.column_runs);
        for (index, (rect, label)) in runs.iter().enumerate() {
            if !self.clip.contains_rect(*rect) {
                self.ledger.text_defects.insert(format!("clipped: {label}"));
            }
            for (other_rect, other_label) in runs.iter().skip(index + 1) {
                if overlaps(*rect, *other_rect) {
                    self.ledger
                        .text_defects
                        .insert(format!("overlapping: {label} / {other_label}"));
                }
            }
        }
    }

    fn record_color(&mut self, color: Color32) {
        self.ledger.painted_colors.insert(color.to_array());
    }

    /// Records the runs a control or composition emitted through its own
    /// painter.
    ///
    /// They are counted and checked against the registered families the same
    /// way this painter's own runs are, because a run the operator sees is a run
    /// the observation must account for however it was emitted.
    ///
    /// They are deliberately *not* entered into the region's clipping and
    /// overlap check. That check is the gallery asserting its own layout — its
    /// bands, headings, captions, and the seats it allocates. What a control or
    /// a composition does inside the seat it was given is the claim of the work
    /// package that owns it, proved by its own tests and, at the two authored
    /// viewports where the claim means something, by the deterministic
    /// composition target. A gallery seat is neither authored viewport; it is
    /// one of two compositions sharing one window. Asserting internal layout
    /// here would be making a claim about a size the product never runs at, and
    /// it would contradict rather than reinforce the claim made where it counts.
    /// What the gallery *does* assert about them is that something the specimen
    /// painted has positive area inside the seat it was given. That is the
    /// gallery's own claim — it chose the rectangle — and a specimen whose
    /// content all lands outside its seat is one the operator cannot see, which
    /// is a gallery layout defect however correct the component's internal
    /// arrangement is.
    fn record_component_runs(&mut self, seat: Rect, name: &str, runs: &[PaintedRun]) {
        let mut visible = false;
        for run in runs {
            self.ledger.text_runs += 1;
            let resolved = run
                .families
                .iter()
                .all(|family| self.registered_families.contains(family));
            if !resolved {
                self.ledger.unresolved_text_runs += 1;
            }
            let inside = seat.intersect(run.rect);
            if inside.width() > 0.0 && inside.height() > 0.0 {
                visible = true;
            }
        }
        if !visible {
            self.ledger.text_defects.insert(format!(
                "nothing visible in its seat: {name} emitted {} runs, none inside {seat:?}",
                runs.len()
            ));
        }
    }

    fn record_run(&mut self, rect: Rect, content: &str, font_id: &FontId) {
        self.ledger.text_runs += 1;
        let registered = self.registered_families.contains(&font_id.family);
        let has_glyphs = self
            .context
            .fonts(|fonts| fonts.has_glyphs(font_id, content));
        if !registered || !has_glyphs {
            self.ledger.unresolved_text_runs += 1;
        }
        self.column_runs.push((rect, content.to_owned()));
    }

    /// Paints one text run and records where it landed.
    fn emit_text(
        &mut self,
        anchor: Pos2,
        align: Align2,
        content: &str,
        style: TypeStyle,
        color: SemanticColor,
        state: ComponentState,
    ) -> Rect {
        if content.is_empty() {
            return Rect::from_min_size(anchor, Vec2::ZERO);
        }
        let format = text::text_format(style, color, state);
        let painted = format.color;
        let font_id = format.font_id.clone();
        let galley = text::layout(self.painter, content.to_owned(), style, color, state);
        let rect = align.anchor_size(anchor, galley.size());
        self.painter.galley(rect.min, galley, painted);
        self.record_color(painted);
        self.record_run(rect, content, &font_id);
        rect
    }

    /// Measures how wide a run will be without painting it.
    ///
    /// Nothing is recorded, because nothing reaches the screen: this exists so
    /// a layout can reserve exactly the room a name needs rather than guess at
    /// a fraction and clip it.
    fn measure_width(&self, content: &str, style: TypeStyle) -> f32 {
        text::layout(
            self.painter,
            content.to_owned(),
            style,
            SemanticColor::TextSecondary,
            ComponentState::Resting,
        )
        .size()
        .x
    }

    /// Paints a run whose top-left corner is `at`.
    fn text_at(
        &mut self,
        at: Pos2,
        content: &str,
        style: TypeStyle,
        color: SemanticColor,
        state: ComponentState,
    ) -> Rect {
        self.emit_text(at, Align2::LEFT_TOP, content, style, color, state)
    }

    /// Paints a run centered vertically on `at` and starting at its x.
    fn text_left_center(
        &mut self,
        at: Pos2,
        content: &str,
        style: TypeStyle,
        color: SemanticColor,
        state: ComponentState,
    ) -> Rect {
        self.emit_text(at, Align2::LEFT_CENTER, content, style, color, state)
    }

    /// Paints a run right-aligned to `at` and centered vertically on it.
    fn text_right_center(
        &mut self,
        at: Pos2,
        content: &str,
        style: TypeStyle,
        color: SemanticColor,
        state: ComponentState,
    ) -> Rect {
        self.emit_text(at, Align2::RIGHT_CENTER, content, style, color, state)
    }

    /// Paints the mixed-tone hint line and records every section's color.
    fn hint_line_at(&mut self, at: Pos2, align: Align2, hints: &[ActionHint<'_>]) -> Rect {
        let state = ComponentState::Resting;
        let job = hint::hint_line(hints, state);
        if job.text.is_empty() {
            return Rect::from_min_size(at, Vec2::ZERO);
        }
        let content = job.text.clone();
        let sections: Vec<(FontId, Color32)> = job
            .sections
            .iter()
            .map(|section| (section.format.font_id.clone(), section.format.color))
            .collect();
        let galley = self.painter.layout_job(job);
        let rect = align.anchor_size(at, galley.size());
        let fallback = sections
            .first()
            .map_or(SemanticColor::TextMuted.resolve(), |(_, color)| *color);
        self.painter.galley(rect.min, galley, fallback);
        for (font_id, color) in &sections {
            self.record_color(*color);
            let registered = self.registered_families.contains(&font_id.family);
            if !registered {
                self.ledger.unresolved_text_runs += 1;
            }
        }
        // The whole line is one run for clipping and overlap purposes: its
        // sections are laid out contiguously by `epaint` and cannot overlap
        // one another.
        self.ledger.text_runs += 1;
        let has_glyphs = sections.first().is_some_and(|(font_id, _)| {
            self.context
                .fonts(|fonts| fonts.has_glyphs(font_id, &content))
        });
        if !has_glyphs {
            self.ledger.unresolved_text_runs += 1;
        }
        self.column_runs.push((rect, content));
        rect
    }

    /// Fills a rect in an authored role.
    fn fill(&mut self, rect: Rect, color: SemanticColor, radius: Radius) {
        let painted = color.resolve();
        self.painter.rect_filled(rect, corner(radius), painted);
        self.record_color(painted);
    }

    /// Strokes a rect in an authored role at an authored keyline width.
    fn stroke(&mut self, rect: Rect, width_px: f32, color: SemanticColor, radius: Radius) {
        let painted = color.resolve();
        self.painter.rect_stroke(
            rect,
            corner(radius),
            Stroke::new(width_px, painted),
            StrokeKind::Inside,
        );
        self.record_color(painted);
    }

    /// Paints the authored halo around `rect`.
    fn halo(&mut self, rect: Rect, accent: SemanticColor, radius: Radius) {
        let halo = focus::halo(accent);
        self.painter.add(halo.as_shape(rect, corner(radius)));
        self.record_color(halo.color);
    }

    /// Paints the resting hairline.
    fn hairline(&mut self, span: rules::RuleSpan) {
        rules::hairline(self.painter, span);
        self.record_color(SemanticColor::BorderDefault.resolve());
    }

    /// Paints the structural keyline.
    fn keyline(&mut self, span: rules::RuleSpan) {
        rules::keyline(self.painter, span);
        self.record_color(SemanticColor::BorderStrong.resolve());
    }

    /// Paints one complete state specimen and records what announced it.
    ///
    /// The row carries the state's declared frame, halo, fill, cursor, label,
    /// value, and status mark. Every one of those is emitted here rather than
    /// derived afterwards, so the recorded evidence is the painted evidence.
    fn state_specimen(&mut self, row: Rect, specimen: Specimen<'_>) {
        let runs_before = self.ledger.text_runs;
        let state = specimen.state;
        let appearance = state.appearance();

        if appearance.fills_row {
            self.fill(row, appearance.accent, Radius::Small);
        }
        if focus::frames(state) {
            if appearance.draws_halo {
                self.halo(row, appearance.accent, Radius::Small);
            }
            self.stroke(row, appearance.keyline_px, appearance.accent, Radius::Small);
        } else {
            self.hairline(rules::RuleSpan::Horizontal {
                y_px: row.max.y,
                from_x_px: row.min.x,
                to_x_px: row.max.x,
            });
        }

        let draws_cursor = focus::draws_cursor(state);
        if draws_cursor {
            let column = focus::cursor_column(row);
            self.text_left_center(
                column.left_center(),
                focus::CURSOR_GLYPH,
                TypeStyle::LabelControl,
                appearance.accent,
                state,
            );
        }

        self.text_left_center(
            pos2(row.min.x + focus::LABEL_START_X_PX, row.center().y),
            specimen.label,
            TypeStyle::LabelControl,
            SemanticColor::TextPrimary,
            state,
        );

        let value_rect = self.text_right_center(
            pos2(row.max.x - SpacingStep::S12.resolve(), row.center().y),
            specimen.value,
            TypeStyle::CodeValue,
            value::value_color(state),
            state,
        );

        // The mark is resolved by the primitive that owns the decision and
        // painted by the recording path that owns the emission, so the two can
        // never disagree about what reached the screen.
        let mark_anchor = pos2(
            value_rect.min.x - SpacingStep::S16.resolve(),
            row.center().y,
        );
        let mark = match status::status_mark(state, specimen.detail) {
            Some(StatusMark::Text { text, color }) => {
                self.text_right_center(mark_anchor, text, TypeStyle::LabelControl, color, state);
                Some(text.to_owned())
            }
            Some(StatusMark::Selection { mark, .. }) => {
                let side = SpacingStep::S8.resolve();
                let square = Align2::RIGHT_CENTER.anchor_size(mark_anchor, Vec2::splat(side));
                self.fill(square, mark, Radius::None);
                Some(format!("selection {}", mark.canonical_name()))
            }
            None => None,
        };

        if self.ledger.text_runs > runs_before {
            // Recorded against the composition that painted it, so a state that
            // reaches only the roomier column cannot be counted as covered.
            self.ledger.states_painted[policy_index(specimen.policy)][state_index(state)] =
                Some(PaintedState {
                    evidence: PaintedStateEvidence {
                        keyline_px: appearance.keyline_px,
                        halo: appearance.draws_halo,
                        row_fill: appearance.fills_row,
                        cursor: draws_cursor,
                        mark,
                    },
                    visible_label: specimen.label.to_owned(),
                });
        }
    }
}

/// One state specimen's content, and the composition it belongs to.
///
/// The policy travels with the specimen rather than living on the painter: every
/// page that paints a specimen already resolved its policy for row geometry, so
/// carrying it here keeps the recording site and the layout site reading from
/// the same value.
struct Specimen<'a> {
    state: ComponentState,
    policy: ViewportDensityPolicy,
    label: &'a str,
    value: &'a str,
    detail: StatusDetail<'a>,
}

/// Whether two rects share any positive area. Touching edges do not overlap.
fn overlaps(first: Rect, second: Rect) -> bool {
    first.min.x < second.max.x
        && second.min.x < first.max.x
        && first.min.y < second.max.y
        && second.min.y < first.max.y
}

/// Resolves an authored radius to the corner value `epaint` carries.
fn corner(radius: Radius) -> CornerRadius {
    CornerRadius::same(radius.resolve() as u8)
}

/// A vertical layout cursor inside one column.
///
/// Rows are allocated top to bottom. Nothing here decides how tall a row is —
/// the caller supplies that from the density policy or an authored step.
struct Stack {
    rect: Rect,
    y: f32,
}

impl Stack {
    fn new(rect: Rect) -> Self {
        Self {
            rect,
            y: rect.min.y,
        }
    }

    fn row(&mut self, height_px: f32) -> Rect {
        let top = self.y;
        self.y += height_px;
        Rect::from_min_max(
            pos2(self.rect.min.x, top),
            pos2(self.rect.max.x, top + height_px),
        )
    }

    fn gap(&mut self, step: SpacingStep) {
        self.y += step.resolve();
    }

    fn remaining(&self) -> Rect {
        Rect::from_min_max(pos2(self.rect.min.x, self.y), self.rect.max)
    }
}

/// The label row height used for a caption above or beside a specimen.
fn caption_height() -> f32 {
    TypeStyle::LabelControl.metrics().line_height_px
}

/// The heading row height used to separate specimen groups.
fn heading_height() -> f32 {
    TypeStyle::HeadingPanel.metrics().line_height_px
}

/// Paints one group heading and returns the row it occupied.
fn heading(painter: &mut SpecimenPainter<'_>, stack: &mut Stack, label: &str) {
    let row = stack.row(heading_height());
    painter.text_left_center(
        pos2(row.min.x, row.center().y),
        label,
        TypeStyle::HeadingPanel,
        SemanticColor::TextSecondary,
        ComponentState::Resting,
    );
}

/// Paints prose across as many rows as it needs to fit the stack's width.
///
/// The gallery's own notes are sentences, not labels, and the two columns are
/// different widths. A note that is one line in the desktop column and does not
/// fit in the compact one would be reported as clipped — correctly, since the
/// operator could not read it — so the note wraps rather than the column
/// widening or the sentence being cut down until it fits the narrower of the
/// two. Word-wrapped by measurement, which is the same layout the paint pass
/// then checks.
fn paint_wrapped(
    painter: &mut SpecimenPainter<'_>,
    stack: &mut Stack,
    text: &str,
    style: TypeStyle,
    color: SemanticColor,
) {
    let width = stack.remaining().width();
    let line_height = style.metrics().line_height_px;
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && painter.measure_width(&candidate, style) > width {
            let row = stack.row(line_height);
            painter.text_left_center(
                pos2(row.min.x, row.center().y),
                &line,
                style,
                color,
                ComponentState::Resting,
            );
            line = word.to_owned();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        let row = stack.row(line_height);
        painter.text_left_center(
            pos2(row.min.x, row.center().y),
            &line,
            style,
            color,
            ComponentState::Resting,
        );
    }
}

/// The gallery's own eframe application.
///
/// It owns the page selection, the ledger, and nothing else. There is no
/// projection callback, no audio observation, no tick, and no checkpoint: this
/// scene correlates with nothing and waits for the operator.
struct ComponentGalleryApplication {
    selection: GalleryPageSelection,
    ledger: Rc<RefCell<GalleryPaintLedger>>,
}

impl ComponentGalleryApplication {
    fn new(ledger: Rc<RefCell<GalleryPaintLedger>>) -> Self {
        Self {
            selection: GalleryPageSelection::default(),
            ledger,
        }
    }

    /// Consumes window input entirely inside the scene.
    ///
    /// Nothing here reaches `KeyboardInputTranslator`, so no key press becomes a
    /// `SemanticAction` and nothing can reach `AppState`.
    fn handle_input(&mut self, context: &egui::Context) {
        let inputs: Vec<WindowInput> = context.input(|input| {
            input
                .events
                .iter()
                .filter_map(normalize_gallery_event)
                .collect()
        });
        for input in inputs {
            let before = self.selection.active();
            let selection = self.selection.apply(input);
            let mut ledger = self.ledger.borrow_mut();
            match selection {
                PageSelection::Changed(page) => ledger.record_digit_request(page),
                PageSelection::Stepped(page) => ledger.record_step_request(page),
                PageSelection::Retained(page) => {
                    // A step that ran into an end is a bound key declining to
                    // move, which is a different fact from a key that binds
                    // nothing; only the latter is what the retention field
                    // reports.
                    let bound_nothing = PageStep::for_key(input.key()).is_none();
                    if bound_nothing && input.kind() == WindowInputKind::KeyDown {
                        ledger.record_unbound_key(page != before);
                    }
                }
            }
        }
    }
}

impl eframe::App for ComponentGalleryApplication {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_input(context);
        let active = self.selection.active();
        let ledger = Rc::clone(&self.ledger);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(SemanticColor::BgCanvas.resolve())
                    .inner_margin(egui::Margin::ZERO)
                    .outer_margin(egui::Margin::ZERO),
            )
            .show(context, |ui| {
                paint_gallery(ui, active, &mut ledger.borrow_mut());
            });
    }
}

/// Normalizes one egui event into the window vocabulary.
///
/// The previous version of this function also reported whether the physical key
/// was a digit, so the scene could tell "the operator pressed 9" — a digit that
/// bound no page — from "the operator pressed a letter". That distinction is
/// gone because its subject is: all ten digits now bind pages. What the scene
/// still has to demonstrate is that a key binding *nothing* changes nothing,
/// and every key beyond the twelve the gallery binds normalizes to
/// [`WindowKey::Other`], which is exactly that key.
fn normalize_gallery_event(event: &egui::Event) -> Option<WindowInput> {
    match event {
        egui::Event::Key { key, pressed, .. } => {
            let normalized = normalize_gallery_key(*key);
            Some(if *pressed {
                WindowInput::key_down(normalized)
            } else {
                WindowInput::key_up(normalized)
            })
        }
        egui::Event::WindowFocused(false) => Some(WindowInput::focus_lost()),
        _ => None,
    }
}

/// Maps a physical key to the window vocabulary.
///
/// The two bracket keys are normalized here for the same reason the digits are:
/// the window sees them and the scene binds them locally. Neither reaches
/// [`KeyboardInputTranslator`](crate::shell::keyboard_input_translator::KeyboardInputTranslator),
/// so neither can become a `SemanticAction`.
fn normalize_gallery_key(key: egui::Key) -> WindowKey {
    match key {
        egui::Key::Num1 => WindowKey::Digit1,
        egui::Key::Num2 => WindowKey::Digit2,
        egui::Key::Num3 => WindowKey::Digit3,
        egui::Key::Num4 => WindowKey::Digit4,
        egui::Key::Num5 => WindowKey::Digit5,
        egui::Key::Num6 => WindowKey::Digit6,
        egui::Key::Num7 => WindowKey::Digit7,
        egui::Key::Num8 => WindowKey::Digit8,
        egui::Key::Num9 => WindowKey::Digit9,
        egui::Key::Num0 => WindowKey::Digit0,
        egui::Key::OpenBracket => WindowKey::BracketLeft,
        egui::Key::CloseBracket => WindowKey::BracketRight,
        _ => WindowKey::Other,
    }
}

// ===========================================================================
// Reading back what a control or a composition actually painted
// ===========================================================================

/// Every text run one closure appended to `ui`'s layer, in paint order.
///
/// The gallery's own [`SpecimenPainter`] records what it emits at the moment it
/// emits it. A control and a composition paint through painters of their own,
/// which that ledger never sees — so what they put on screen is read back out
/// of the layer it landed in, bounded below by the shape index the layer stood
/// at before the render call.
///
/// This is the difference between measuring and declaring. A control that was
/// constructed and never painted appends no shapes and returns no runs, so
/// there is no arrangement of specimen lists that produces coverage here.
fn painted_runs(ui: &mut egui::Ui, render: impl FnOnce(&mut egui::Ui)) -> Vec<PaintedRun> {
    let layer = ui.layer_id();
    let context = ui.ctx().clone();
    let before = context.graphics_mut(|layers| layers.entry(layer).all_entries().len());
    render(ui);
    context.graphics_mut(|layers| {
        let mut runs = Vec::new();
        for clipped in layers.entry(layer).all_entries().skip(before) {
            collect_runs(&clipped.shape, clipped.clip_rect, &mut runs);
        }
        runs
    })
}

/// One text run a control or composition put on screen.
#[derive(Clone, Debug)]
struct PaintedRun {
    text: String,
    rect: Rect,
    families: Vec<FontFamily>,
}

/// Walks one emitted shape, collecting the text runs inside it.
fn collect_runs(shape: &egui::epaint::Shape, clip: Rect, runs: &mut Vec<PaintedRun>) {
    match shape {
        egui::epaint::Shape::Text(text) => {
            let content = text.galley.text().to_owned();
            if content.is_empty() {
                return;
            }
            let families = text
                .galley
                .job
                .sections
                .iter()
                .map(|section| section.format.font_id.family.clone())
                .collect();
            // The painted rect is the galley placed at its anchor, intersected
            // with the clip the painter was given: what is outside the clip did
            // not reach the screen, so counting it as painted would report a
            // run the operator cannot see.
            let placed = Rect::from_min_size(text.pos, text.galley.size());
            runs.push(PaintedRun {
                text: content,
                rect: placed.intersect(clip),
                families,
            });
        }
        egui::epaint::Shape::Vec(nested) => {
            for shape in nested {
                collect_runs(shape, clip, runs);
            }
        }
        _ => {}
    }
}

// ===========================================================================
// The projected content the control and composition specimens are drawn from
// ===========================================================================

/// The production projections the gallery's specimens read their content from.
///
/// Built once, from a real [`AppState`] through the production
/// [`StateProjector`](crate::control::StateProjector) — never assembled.
/// [`SemanticControlViewModel`]'s fields are private precisely so that no
/// surface can invent one, and a gallery that fabricated its input would be
/// showing the operator a control the product cannot actually produce.
///
/// This is where representative content belongs. The production shell is where
/// it does not (C-003): a placeholder there would misrepresent absent state as
/// present, whereas a gallery exists to show what a component looks like when
/// it is given something to show.
struct GallerySpecimenSource {
    patch: GraphicalShellProjection,
    mixer: GraphicalShellProjection,
}

impl GallerySpecimenSource {
    /// Derives both projections, or reports why neither is available.
    fn build() -> Result<Self, ComponentGalleryError> {
        Ok(Self {
            patch: gallery_projection(TopLevelContext::Patch)?,
            mixer: gallery_projection(TopLevelContext::Mixer)?,
        })
    }

    /// The projections, in the order specimens are searched.
    fn projections(&self) -> [&GraphicalShellProjection; 2] {
        [&self.patch, &self.mixer]
    }

    /// The first projected control of one semantic kind, or `None` when the
    /// production projection carries none.
    ///
    /// `None` is a real answer and the page prints it. The alternative — handing
    /// the control a view model of some other kind so the specimen is never
    /// blank — would show the operator a shape the selection rule never
    /// produces.
    fn view_of_kind(&self, kind: SemanticControlKind) -> Option<&SemanticControlViewModel> {
        self.projections().into_iter().find_map(|projection| {
            projection
                .semantic_model()
                .surfaces()
                .iter()
                .flat_map(|surface| surface.controls())
                .find(|control| control.kind() == kind)
        })
    }

    /// The specimen for one declared control, or `None` when the projection
    /// carries nothing of the kind that selects it.
    fn specimen(&self, control: ComponentControl) -> Option<ControlSpecimen<'_>> {
        let (kind, role) = selecting_pair(control)?;
        Some(ControlSpecimen {
            kind,
            role,
            view: self.view_of_kind(kind)?,
        })
    }

    /// The projection a composition is shown with.
    ///
    /// The mixer strip bank is a MIXER structure and has nothing to arrange in
    /// a PATCH projection, so it is shown in the context it belongs to. Every
    /// other composition is shown on PATCH, which is the context the gallery
    /// opens describing.
    const fn projection_for(&self, composition: ShellComposition) -> &GraphicalShellProjection {
        match composition {
            ShellComposition::MixerStripBank => &self.mixer,
            ShellComposition::ApplicationShell
            | ShellComposition::ContextSwitch
            | ShellComposition::IdentityHeader
            | ShellComposition::Section
            | ShellComposition::PatchStripRow
            | ShellComposition::UtilityInspectorPanel
            | ShellComposition::Footer => &self.patch,
        }
    }
}

/// The one specimen source the gallery derives, built on first use.
///
/// Derived once rather than per frame: the projection comes from a real
/// [`AppState`] through the production projector, and rebuilding that sixty
/// times a second would make the gallery's cost a property of its frame rate
/// rather than of what it paints. The value is immutable once built, which is
/// what makes sharing it safe.
static GALLERY_SPECIMENS: std::sync::OnceLock<Option<GallerySpecimenSource>> =
    std::sync::OnceLock::new();

/// The specimen source, or `None` when the production projector could not
/// derive one.
///
/// `None` is not a failure to open the window. The vocabulary pages need no
/// projection at all, and a gallery that refused to start because the projector
/// was unavailable would deny the operator the eight pages that still work. The
/// control and composition pages say what is missing instead, and the
/// observation's coverage counts fall short — which is the accurate report.
fn gallery_specimen_source() -> Option<&'static GallerySpecimenSource> {
    GALLERY_SPECIMENS
        .get_or_init(|| GallerySpecimenSource::build().ok())
        .as_ref()
}

/// One control specimen: the pair that selects the control, and what it paints.
struct ControlSpecimen<'a> {
    kind: SemanticControlKind,
    role: PresentationRole,
    view: &'a SemanticControlViewModel,
}

/// The first `(kind, role)` pair, in declared order, that selects `control`.
///
/// Derived from [`control_for`] rather than written out here. A gallery that
/// hard-coded the pair would keep showing a control in a role the selection
/// rule had stopped putting it in, which is precisely the drift the total
/// `match` in the control family exists to catch.
fn selecting_pair(control: ComponentControl) -> Option<(SemanticControlKind, PresentationRole)> {
    ALL_SEMANTIC_CONTROL_KINDS
        .into_iter()
        .flat_map(|kind| {
            ALL_PRESENTATION_ROLES
                .into_iter()
                .map(move |role| (kind, role))
        })
        .find(|(kind, role)| control_for(*kind, *role).control() == Some(control))
}

/// The production graphical shell projection for one top-level context.
///
/// # Why the SoundFont capability is here, and why it reads no file
///
/// The gallery paints; it does not sound. A visual scene that failed to start
/// because a 247 MB SoundFont was missing would be coupling the two for no
/// gain, so this loads nothing. But the *only* `ParameterKind::Asset` in the
/// product belongs to the SoundFont capability, and the browser row is the
/// control that `(Asset, ListedRow)` selects — so without that capability's
/// descriptor there is no asset row to show, and the gallery would be silently
/// one control short of the family it claims to cover.
///
/// The resolution is to build the *production* capability descriptor from a
/// small in-memory preset list rather than from a parsed SF2. The descriptor
/// shape, the parameter kinds, and the authored asset path are the product's
/// own; only the preset names are representative. That is exactly what a
/// gallery is for, and exactly what the production shell may not do (C-003).
fn gallery_projection(
    context: TopLevelContext,
) -> Result<GraphicalShellProjection, ComponentGalleryError> {
    use crate::control::app_event::AppEvent;

    let braids = crate::adapter::braids_capability::BraidsCapability::new()
        .map_err(ComponentGalleryError::Capability)?;
    let soundfont = crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability::new(
        std::sync::Arc::new(gallery_preset_catalog()?),
    )
    .map_err(ComponentGalleryError::Capability)?;
    let registry = crate::synth::instrument_capability::CapabilityRegistry::new(vec![
        soundfont.descriptor(),
        braids.descriptor(),
    ])
    .map_err(ComponentGalleryError::Capability)?;
    // The production default-config factory, so a specimen Patch is configured
    // the way the application configures one rather than by this file deciding
    // what a default is.
    let factory = crate::synth::DescriptorDefaultConfigFactory::new(
        registry.clone(),
        vec![Box::new(soundfont.clone()), Box::new(braids.clone())],
    );
    let mut state = AppState::new(registry, gallery_global_parameters());
    // Two Patches, so the projection carries both capabilities' parameter
    // kinds: the SoundFont's choice and asset, and the envelope's continuous
    // values that every Patch has.
    let patches = vec![
        gallery_patch(
            1,
            "Lead Pad",
            &factory,
            crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID,
            3,
            3,
        )?,
        gallery_patch(
            2,
            "Wavetable",
            &factory,
            crate::adapter::braids_capability::BRAIDS_CAPABILITY_ID,
            4,
            4,
        )?,
    ];
    state
        .apply(AppEvent::InstallPatches(patches))
        .map_err(|rejection| {
            ComponentGalleryError::Projection(format!(
                "the specimen Patches were rejected: {rejection:?}"
            ))
        })?;
    state
        .apply(AppEvent::SelectContext(context))
        .map_err(|rejection| {
            ComponentGalleryError::Projection(format!(
                "the specimen context was rejected: {rejection:?}"
            ))
        })?;
    let (_, _, _, shell, _) = crate::control::StateProjector::new()
        .project_with_shell(&state)
        .map_err(|error| ComponentGalleryError::Projection(format!("{error:?}")))?;
    Ok(shell)
}

/// One installed specimen Patch, configured through the production factory.
fn gallery_patch(
    id: u32,
    name: &str,
    factory: &crate::synth::DescriptorDefaultConfigFactory,
    capability_id: &str,
    channel: u8,
    track: u8,
) -> Result<crate::synth::Patch, ComponentGalleryError> {
    let capability = crate::synth::capability_id::CapabilityId::new(capability_id)
        .map_err(|_| ComponentGalleryError::Projection(capability_id.to_owned()))?;
    Ok(crate::synth::Patch::new(
        crate::kernel::PatchId::new(id).expect("the gallery specimen PatchId is in range"),
        name.to_owned(),
        factory
            .create(&capability)
            .map_err(ComponentGalleryError::Capability)?,
        crate::kernel::MidiChannel::new(channel)
            .expect("the gallery specimen MIDI channel is in range"),
        crate::mixer::patch_output::PatchOutput::to_track(
            crate::mixer::mixer_track_id::MixerTrackId::new(track)
                .expect("the gallery specimen track is in range"),
        ),
    ))
}

/// A small in-memory preset catalog, so the SoundFont descriptor exists without
/// a SoundFont being read.
///
/// Two entries rather than one: a choice row with a single option cannot show
/// what choosing looks like.
fn gallery_preset_catalog() -> Result<crate::synth::SoundFontPresetCatalog, ComponentGalleryError> {
    crate::synth::SoundFontPresetCatalog::from_sources([
        crate::synth::SoundFontPresetSource::new(0, 0, 0, "Grand Piano", true),
        crate::synth::SoundFontPresetSource::new(1, 0, 48, "String Ensemble", true),
    ])
    .map_err(|error| ComponentGalleryError::Projection(format!("{error:?}")))
}

/// Paints one complete gallery frame into `ui`.
///
/// This is the whole render path. The tests below drive it in a headless
/// `egui::Context`, so what they measure is what the window paints.
fn paint_gallery(ui: &mut egui::Ui, active: ComponentGalleryPage, ledger: &mut GalleryPaintLedger) {
    paint_gallery_with(ui, active, ledger, gallery_specimen_source());
}

/// Paints one frame against an explicitly supplied specimen source.
///
/// Split out so the tests below can drive the same production paint path with a
/// source that failed to build, and see the page say so, rather than only ever
/// seeing the happy path.
fn paint_gallery_with(
    ui: &mut egui::Ui,
    active: ComponentGalleryPage,
    ledger: &mut GalleryPaintLedger,
    specimens: Option<&GallerySpecimenSource>,
) {
    let window = ui.max_rect();
    let context = ui.ctx().clone();
    let families = context.fonts(|fonts| fonts.families());
    let painter = ui.painter().clone();
    ledger.active_page = active;
    ledger.viewport = window.size();

    let mut specimen = SpecimenPainter::new(&painter, &context, &families, ledger, window);
    let bands = ViewportDensityPolicy::Desktop.bands();
    let (identity, rest) = window.split_top_bottom_at_y(window.min.y + bands.identity_header_px);
    let (index, rest) = rest.split_top_bottom_at_y(rest.min.y + bands.context_line_px);
    let (stage, footer) = rest.split_top_bottom_at_y(rest.max.y - bands.footer_px);

    paint_identity_band(&mut specimen, identity, active);
    paint_index_band(&mut specimen, index, active);
    paint_footer_band(&mut specimen, footer);

    let split_x = stage.min.x + stage.width() * desktop_stage_fraction();
    let (desktop_column, deck_column) = stage.split_left_right_at_x(split_x);
    specimen.begin_region(stage);
    specimen.keyline(rules::RuleSpan::Vertical {
        x_px: split_x,
        from_y_px: stage.min.y,
        to_y_px: stage.max.y,
    });
    specimen.finish_region();

    paint_composition(
        &mut specimen,
        ui,
        specimens,
        desktop_column,
        ViewportDensityPolicy::Desktop,
        active,
    );
    paint_composition(
        &mut specimen,
        ui,
        specimens,
        deck_column,
        ViewportDensityPolicy::SteamDeck,
        active,
    );
    specimen.finish_region();
}

/// How much of the stage the desktop composition occupies.
///
/// The two columns are split in the ratio of the authored viewport widths, so
/// the Steam Deck composition is narrower on screen for the same reason it is
/// narrower in life.
fn desktop_stage_fraction() -> f32 {
    let desktop = ViewportDensityPolicy::Desktop.authored_viewport().width_px;
    let deck = ViewportDensityPolicy::SteamDeck
        .authored_viewport()
        .width_px;
    desktop / (desktop + deck)
}

/// Paints the page identity band. The operator always knows where they are.
fn paint_identity_band(
    painter: &mut SpecimenPainter<'_>,
    band: Rect,
    active: ComponentGalleryPage,
) {
    painter.begin_region(band);
    painter.fill(band, SemanticColor::BgPanel, Radius::None);
    let inset = ViewportDensityPolicy::Desktop.rhythm().inset_px;
    painter.text_left_center(
        pos2(band.min.x + inset, band.center().y),
        // The position and the digit were once the same number and no longer
        // are: fifteen pages, ten digits. So the identity names the position —
        // which every page has — and then names the key that reaches it, which
        // is the digit where one is bound and the brackets where none is. An
        // operator on page 11 must be able to read how they got there.
        &format!(
            "PAGE {} / {} · {} · {}",
            active.index() + 1,
            GALLERY_PAGE_COUNT,
            match active.digit_label() {
                Some(digit) => format!("KEY {digit}"),
                None => format!("KEY {} {}", STEP_PREVIOUS_LABEL, STEP_NEXT_LABEL),
            },
            active.title()
        ),
        TypeStyle::HeadingSection,
        SemanticColor::TextPrimary,
        ComponentState::Resting,
    );
    painter.text_right_center(
        pos2(band.max.x - inset, band.center().y),
        "COMPONENT GALLERY · BROWSABLE",
        TypeStyle::LabelControl,
        SemanticColor::TextMuted,
        ComponentState::Resting,
    );
    painter.hairline(rules::RuleSpan::Horizontal {
        y_px: band.max.y,
        from_x_px: band.min.x,
        to_x_px: band.max.x,
    });
    painter.finish_region();
}

/// Paints the page index. Every page is on screen, the active one accented.
fn paint_index_band(painter: &mut SpecimenPainter<'_>, band: Rect, active: ComponentGalleryPage) {
    painter.begin_region(band);
    painter.fill(band, SemanticColor::BgElevated, Radius::None);
    // Every binding is on screen, and so is every page that has none: a page
    // listed without a key is a page the operator learns to step to, and a page
    // left off the index entirely is one they never learn exists.
    //
    // Fifteen entries no longer fit one line at this width, and a clipped index
    // is an index that stops naming the last pages — the very ones without a
    // digit, which are the ones an operator most needs it for. So the index is
    // two lines: the ten digit-bound pages, then the five reached by stepping.
    let hint = |page: ComponentGalleryPage| {
        ActionHint::new(
            page.digit_label().unwrap_or(STEP_ONLY_LABEL),
            page.index_label(),
            if page == active {
                HintTone::Focus
            } else {
                HintTone::Neutral
            },
        )
    };
    let bound: Vec<ActionHint<'static>> = ALL_GALLERY_PAGES
        .into_iter()
        .filter(|page| page.digit().is_some())
        .map(hint)
        .collect();
    let stepped: Vec<ActionHint<'static>> = ALL_GALLERY_PAGES
        .into_iter()
        .filter(|page| page.digit().is_none())
        .map(hint)
        .collect();
    let inset = ViewportDensityPolicy::Desktop.rhythm().inset_px;
    let line = TypeStyle::InstructionHint.metrics().line_height_px;
    let first = band.center().y - line / 2.0;
    painter.hint_line_at(pos2(band.min.x + inset, first), Align2::LEFT_CENTER, &bound);
    painter.hint_line_at(
        pos2(band.min.x + inset, first + line),
        Align2::LEFT_CENTER,
        &stepped,
    );
    painter.hairline(rules::RuleSpan::Horizontal {
        y_px: band.max.y,
        from_x_px: band.min.x,
        to_x_px: band.max.x,
    });
    painter.finish_region();
}

/// Paints the operator's own hints.
fn paint_footer_band(painter: &mut SpecimenPainter<'_>, band: Rect) {
    painter.begin_region(band);
    painter.fill(band, SemanticColor::BgElevated, Radius::None);
    let inset = ViewportDensityPolicy::Desktop.rhythm().inset_px;
    painter.hint_line_at(
        pos2(band.min.x + inset, band.center().y),
        Align2::LEFT_CENTER,
        &[
            ActionHint::new("1-9 0", "PAGE", HintTone::Focus),
            ActionHint::new(
                &format!("{STEP_PREVIOUS_LABEL} {STEP_NEXT_LABEL}"),
                "STEP",
                HintTone::Focus,
            ),
            // Not "9" any more: with ten digits bound to ten pages there is no
            // unbound digit left to press. What still binds nothing is every
            // other key, and that is what this names.
            ActionHint::new("ANY OTHER KEY", "BINDS NO PAGE", HintTone::Adjust),
            ActionHint::new("CLOSE WINDOW", "FINISH", HintTone::Back),
        ],
    );
    painter.text_right_center(
        pos2(band.max.x - inset, band.center().y),
        "SCENE-LOCAL PAGING · NO APPLICATION STATE CHANGES",
        TypeStyle::InstructionHint,
        SemanticColor::TextMuted,
        ComponentState::Resting,
    );
    painter.hairline(rules::RuleSpan::Horizontal {
        y_px: band.min.y,
        from_x_px: band.min.x,
        to_x_px: band.max.x,
    });
    painter.finish_region();
}

/// Paints one policy's composition of the active page.
///
/// The viewport is recorded as painted only after the composition emitted text,
/// and the page only after both compositions did.
fn paint_composition(
    painter: &mut SpecimenPainter<'_>,
    ui: &mut egui::Ui,
    specimens: Option<&GallerySpecimenSource>,
    column: Rect,
    policy: ViewportDensityPolicy,
    active: ComponentGalleryPage,
) {
    painter.begin_region(column);
    let runs_before = painter.ledger.text_runs;
    let inset = policy.rhythm().inset_px;
    let content = column.shrink(inset);
    let mut stack = Stack::new(content);

    let viewport = policy.authored_viewport();
    let caption = stack.row(caption_height());
    painter.text_left_center(
        pos2(caption.min.x, caption.center().y),
        &format!(
            "{} · {} × {} · {}",
            policy.canonical_name(),
            viewport.width_px,
            viewport.height_px,
            provenance_label(policy)
        ),
        TypeStyle::LabelControl,
        SemanticColor::AccentInstrument,
        ComponentState::Resting,
    );
    stack.gap(SpacingStep::S8);

    match active {
        ComponentGalleryPage::Colors => paint_colors_page(painter, &mut stack),
        ComponentGalleryPage::Type => paint_type_page(painter, &mut stack),
        ComponentGalleryPage::SpacingAndGeometry => paint_geometry_page(painter, &mut stack),
        ComponentGalleryPage::InteractionStates => {
            paint_interaction_states_page(painter, &mut stack, policy);
        }
        ComponentGalleryPage::TextAndHairlines => {
            paint_text_and_hairlines_page(painter, &mut stack, policy);
        }
        ComponentGalleryPage::ValuesAndStatus => {
            paint_values_and_status_page(painter, &mut stack, policy);
        }
        ComponentGalleryPage::ActionHints => paint_action_hints_page(painter, &mut stack),
        ComponentGalleryPage::ShellBands => paint_shell_bands_page(painter, &mut stack, policy),
        ComponentGalleryPage::ParameterAndChoiceRows => paint_control_page(
            painter,
            ui,
            specimens,
            &mut stack,
            policy,
            &[ComponentControl::ParameterRow, ComponentControl::ChoiceRow],
        ),
        ComponentGalleryPage::TogglesAndSliders => paint_control_page(
            painter,
            ui,
            specimens,
            &mut stack,
            policy,
            &[ComponentControl::Toggle, ComponentControl::CompactSlider],
        ),
        ComponentGalleryPage::FadersAndMeters => paint_control_page(
            painter,
            ui,
            specimens,
            &mut stack,
            policy,
            &[ComponentControl::Fader, ComponentControl::Meter],
        ),
        ComponentGalleryPage::BrowserAndModalOptions => paint_control_page(
            painter,
            ui,
            specimens,
            &mut stack,
            policy,
            &[ComponentControl::BrowserRow, ComponentControl::ModalOption],
        ),
        ComponentGalleryPage::ShellAndContextSwitch => paint_composition_page(
            painter,
            ui,
            specimens,
            &mut stack,
            policy,
            &[
                ShellComposition::ApplicationShell,
                ShellComposition::ContextSwitch,
            ],
        ),
        ComponentGalleryPage::HeadersAndSections => paint_composition_page(
            painter,
            ui,
            specimens,
            &mut stack,
            policy,
            &[ShellComposition::IdentityHeader, ShellComposition::Section],
        ),
        // Four compositions, because the eighth — the mixer strip bank — was
        // authored into the family after the fifteen pages were declared, and
        // the page set is closed. It belongs here rather than anywhere else:
        // this is the page of main-surface and side-region structure, and the
        // bank is main-surface structure. See F-09.
        ComponentGalleryPage::StripPanelAndFooter => paint_composition_page(
            painter,
            ui,
            specimens,
            &mut stack,
            policy,
            &[
                ShellComposition::PatchStripRow,
                ShellComposition::MixerStripBank,
                ShellComposition::UtilityInspectorPanel,
                ShellComposition::Footer,
            ],
        ),
    }

    let painted_runs = painter.ledger.text_runs - runs_before;
    if painted_runs > 0 {
        painter.ledger.viewports_painted[policy_index(policy)] = true;
        painter.ledger.record_painted_page(active, painted_runs);
    }
    painter.finish_region();
}

/// The three controls the design file authors no specimen for.
///
/// Recorded as F-01: two independent node-type censuses of the whole Screens
/// page found 202 frames, 188 text nodes, 104 rounded rectangles, 39 instances,
/// six vectors — and zero ellipses, polygons, or boolean shapes. Every binary in
/// the file is a text run, and the file defines exactly five component sets.
///
/// All three shipped as flagged minimums built from what the file *does* say.
/// Whether Phase 4 keeps them or the design file is extended first is a product
/// decision, and the operator makes it by looking at these pages — so the flag
/// is painted beside the specimen rather than left in a source comment. Dressing
/// them up, or letting them sit unremarked among the well-specified controls,
/// would be hiding the very thing these pages exist to surface.
const CONTROLS_WITHOUT_AN_AUTHORED_SPECIMEN: [(ComponentControl, &str); 3] = [
    (
        ComponentControl::ChoiceRow,
        "NO AUTHORED SPECIMEN · the Compact Parameter Slider's twelve variants add no directional mark in any state, so this row ships without adjacency affordances",
    ),
    (
        ComponentControl::Toggle,
        "NO AUTHORED SPECIMEN · no toggle, switch, checkbox, or stepper set exists, so ON/OFF is taken from DESIGN.md:468 plus one filled/hollow shape channel",
    ),
    (
        ComponentControl::Meter,
        "NO AUTHORED SPECIMEN · the Mixer screen holds sixteen Faders and no level readout, ladder, or peak mark, so this is a read-only fader twin without the grab cap",
    ),
];

/// The flag one control carries, where the design file authors no specimen.
fn missing_specimen_note(control: ComponentControl) -> Option<&'static str> {
    CONTROLS_WITHOUT_AN_AUTHORED_SPECIMEN
        .into_iter()
        .find_map(|(flagged, note)| (flagged == control).then_some(note))
}

/// Paints the controls named by `controls`, each in every state it declares.
///
/// The state list is the control's own `applicable_states`, so a control that
/// declares nine shows nine and one that declares seven shows seven. The page
/// does not carry a list of its own: a page-local list would keep showing seven
/// states for a control that had grown to eight.
fn paint_control_page(
    painter: &mut SpecimenPainter<'_>,
    ui: &mut egui::Ui,
    specimens: Option<&GallerySpecimenSource>,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
    controls: &[ComponentControl],
) {
    // Side by side rather than stacked. Two controls at up to nine states each
    // is eighteen rows, and eighteen rows do not fit a column that is half a
    // window tall — the specimens would run off the bottom, which the paint pass
    // would report and an operator would simply not see.
    let seats = split_columns(stack.remaining(), controls.len());
    for (control, seat) in controls.iter().zip(seats) {
        let control = *control;
        let mut column = Stack::new(seat);
        let specimen = specimens.and_then(|source| source.specimen(control));
        paint_control_heading(painter, &mut column, control, specimen.as_ref());
        match specimen {
            Some(specimen) => match specimen.role {
                // A strip control takes the column width the policy declares, so
                // its states run across rather than down.
                PresentationRole::VerticalStrip => {
                    paint_strip_specimens(painter, ui, &mut column, policy, control, &specimen);
                }
                PresentationRole::ListedRow
                | PresentationRole::PanelEntry
                | PresentationRole::ModalEntry => {
                    paint_row_specimens(painter, ui, &mut column, policy, control, &specimen);
                }
            },
            None => mark_specimen_unavailable(painter, &mut column, control.canonical_name()),
        }
    }
}

/// Divides one rect into `count` equal columns with an authored gutter.
fn split_columns(area: Rect, count: usize) -> Vec<Rect> {
    let gutter = SpacingStep::S16.resolve();
    let count = count.max(1);
    let width = (area.width() - gutter * (count - 1) as f32) / count as f32;
    (0..count)
        .map(|index| {
            let left = area.min.x + (width + gutter) * index as f32;
            Rect::from_min_max(pos2(left, area.min.y), pos2(left + width, area.max.y))
        })
        .collect()
}

/// Names one control, the pair that selects it, how many states it declares,
/// and — where there is one — the fact that the design file authors no specimen.
fn paint_control_heading(
    painter: &mut SpecimenPainter<'_>,
    stack: &mut Stack,
    control: ComponentControl,
    specimen: Option<&ControlSpecimen<'_>>,
) {
    let pair = specimen.map_or_else(
        || "no projected view data".to_owned(),
        |specimen| {
            format!(
                "{} in {}",
                kind_name(specimen.kind),
                specimen.role.canonical_name()
            )
        },
    );
    heading(painter, stack, control.canonical_name());
    paint_wrapped(
        painter,
        stack,
        &format!("{pair} · {} STATES", control.applicable_states().len()),
        TypeStyle::InstructionHint,
        SemanticColor::TextMuted,
    );
    if let Some(note) = missing_specimen_note(control) {
        paint_wrapped(
            painter,
            stack,
            note,
            TypeStyle::InstructionHint,
            SemanticColor::AccentWarning,
        );
    }
    stack.gap(SpacingStep::S4);
}

/// Paints one row-shaped control once per declared state, with the state named
/// in a gutter beside it.
///
/// The control paints its own label from the view data; the gutter is what makes
/// the *state* readable, because a reader who cannot name the state of a row
/// cannot judge it.
fn paint_row_specimens(
    painter: &mut SpecimenPainter<'_>,
    ui: &mut egui::Ui,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
    control: ComponentControl,
    specimen: &ControlSpecimen<'_>,
) {
    let gutter = ALL_COMPONENT_STATES
        .into_iter()
        .map(|state| painter.measure_width(state.canonical_name(), TypeStyle::LabelControl))
        .fold(0.0_f32, f32::max)
        + SpacingStep::S8.resolve();
    paint_row_seat_note(painter, stack, policy, stack.remaining().width() - gutter);
    for state in control.applicable_states() {
        let row = stack.row(policy.rhythm().row_height_px);
        painter.text_left_center(
            pos2(row.min.x, row.center().y),
            state.canonical_name(),
            TypeStyle::LabelControl,
            SemanticColor::TextMuted,
            ComponentState::Resting,
        );
        let seat = Rect::from_min_max(pos2(row.min.x + gutter, row.min.y), row.max);
        paint_control_specimen(painter, ui, seat, policy, control, specimen, *state);
        stack.gap(SpacingStep::S4);
    }
}

/// States how wide a row specimen's seat is against the narrowest width any
/// authored surface renders a row control at.
///
/// The same honesty [`paint_bank_extent_note`] applies to the mixer strip bank,
/// for the same reason and in the same words: a gallery column is not a product
/// surface. Two controls share a page, each takes half of its policy's column,
/// and the state name in the gutter takes more — so on the compact policy the
/// seat lands under the authored control width, and a control laid out narrower
/// than any surface renders it can collide with itself.
///
/// Naming the arithmetic is what makes that readable. Without it an operator
/// sees a status word touching a value, cannot tell a squeezed seat from a
/// broken control, and reports the wrong defect — which is precisely the
/// mistake a gallery exists to prevent. Widening the window instead was
/// measured and rejected: the full state names need 1958 px, and the gallery
/// must fit the 1920 px display it is reviewed on, which
/// `the_gallery_window_fits_on_the_authored_desktop_display_with_its_chrome`
/// holds.
fn paint_row_seat_note(
    painter: &mut SpecimenPainter<'_>,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
    seat_px: f32,
) {
    let authored = policy.utility_control().width_px;
    if seat_px >= authored {
        return;
    }
    paint_wrapped(
        painter,
        stack,
        &format!(
            "this specimen seat is {seat_px:.0} px against an authored control width of {authored:.0} px, so the control lays out narrower here than any product surface renders it"
        ),
        TypeStyle::InstructionHint,
        SemanticColor::AccentWarning,
    );
}

/// Paints one strip-shaped control once per declared state, laid out across the
/// column on the policy's own mixer pitch and wrapped where the column ends.
fn paint_strip_specimens(
    painter: &mut SpecimenPainter<'_>,
    ui: &mut egui::Ui,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
    control: ComponentControl,
    specimen: &ControlSpecimen<'_>,
) {
    let geometry = policy.mixer_column();
    let area = stack.remaining();
    let per_row = ((area.width() / geometry.pitch_px).floor() as usize).max(1);
    let rows = control.applicable_states().len().div_ceil(per_row);
    // The strips divide what is left of the column between them rather than
    // taking a fixed height. A fixed height that fitted the desktop column would
    // overrun the compact one, and the specimen an operator most needs to judge
    // is the compact one.
    let band = area.height() / rows as f32;
    let strip_height =
        (band - caption_height() - SpacingStep::S4.resolve()).max(MIN_INTERACTIVE_TARGET_PX);
    for chunk in control.applicable_states().chunks(per_row) {
        let captions = stack.row(caption_height());
        let strips = stack.row(strip_height);
        for (offset, state) in chunk.iter().enumerate() {
            let left = strips.min.x + geometry.pitch_px * offset as f32;
            // The state name is abbreviated to what an authored column is wide
            // enough to carry. A column is 82 px on the desktop policy and 52 on
            // the compact one, and "Adjusting" set in the authored control label
            // does not fit either — so the caption is the initial, and the
            // heading above names the full set. An overflowing caption would
            // read as the neighbouring column's.
            painter.text_left_center(
                pos2(left, captions.center().y),
                state_initial(*state),
                TypeStyle::LabelControl,
                SemanticColor::TextMuted,
                ComponentState::Resting,
            );
            let seat = Rect::from_min_max(
                pos2(left, strips.min.y),
                pos2(left + geometry.width_px, strips.max.y),
            );
            paint_control_specimen(painter, ui, seat, policy, control, specimen, *state);
        }
        stack.gap(SpacingStep::S4);
    }
}

/// The one-letter abbreviation a strip caption carries for each state.
///
/// Exhaustive with no wildcard, and every letter distinct, so a tenth state is a
/// compile error here rather than two states reading as the same column.
const fn state_initial(state: ComponentState) -> &'static str {
    match state {
        ComponentState::Resting => "R",
        ComponentState::Focused => "F",
        ComponentState::Adjusting => "A",
        ComponentState::Disabled => "D",
        ComponentState::Loading => "L",
        ComponentState::Error => "E",
        ComponentState::Muted => "M",
        ComponentState::Soloed => "S",
        ComponentState::Selected => "X",
    }
}

/// Paints one control specimen through the production render path and records
/// what it actually put on screen.
///
/// The control is asked through [`ComponentControl::render`] — the same entry
/// the production surfaces use — so a control still resolving to a stub would
/// emit nothing here and be counted as nothing.
fn paint_control_specimen(
    painter: &mut SpecimenPainter<'_>,
    ui: &mut egui::Ui,
    seat: Rect,
    policy: ViewportDensityPolicy,
    control: ComponentControl,
    specimen: &ControlSpecimen<'_>,
    state: ComponentState,
) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(seat)
            .layout(egui::Layout::top_down(egui::Align::Min))
            .id_salt((
                control.canonical_name(),
                state.canonical_name(),
                policy.canonical_name(),
            )),
    );
    // Clipped to its seat, so a specimen that wants more room than the gallery
    // has is cut off at its own boundary rather than painted over the specimen
    // beside it. Which of the two an operator is looking at must never be in
    // doubt.
    child.set_clip_rect(seat);
    let view = specimen.view;
    let role = specimen.role;
    let runs = painted_runs(&mut child, |ui| {
        control.render(ui, view, state, role, &policy);
    });
    painter.record_component_runs(seat, control.canonical_name(), &runs);
    if let Some(first) = runs.first() {
        painter.ledger.controls_painted[policy_index(policy)][control_index(control)]
            [state_index(state)] = Some(first.text.clone());
        painter.ledger.control_pairs[control_index(control)] =
            Some((kind_name(specimen.kind), role.canonical_name()));
    }
}

/// Paints the compositions named by `compositions`, each with projected content.
fn paint_composition_page(
    painter: &mut SpecimenPainter<'_>,
    ui: &mut egui::Ui,
    specimens: Option<&GallerySpecimenSource>,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
    compositions: &[ShellComposition],
) {
    // The compositions on a page do not divide its height equally, because they
    // are not equally tall structures. A footer is one band; a bank of sixteen
    // titled columns is a whole workspace. Splitting evenly gives the bank the
    // same room as the footer and its columns collapse into each other, which
    // shows the operator a defect the composition does not have.
    let total: f32 = compositions
        .iter()
        .map(|composition| composition_seat_weight(*composition))
        .sum();
    let available = stack.remaining().height();
    for composition in compositions {
        let composition = *composition;
        let top = stack.remaining().min.y;
        let share = available * composition_seat_weight(composition) / total;
        heading(
            painter,
            stack,
            &format!(
                "{} · {}",
                composition.canonical_name(),
                region_name(composition)
            ),
        );
        if composition == ShellComposition::MixerStripBank {
            paint_bank_extent_note(painter, stack, policy);
        }
        match specimens {
            Some(source) => {
                let used = stack.remaining().min.y - top;
                let seat = stack.row((share - used).max(policy.rhythm().row_height_px));
                paint_composition_specimen(painter, ui, seat, policy, composition, source);
            }
            None => mark_specimen_unavailable(painter, stack, composition.canonical_name()),
        }
    }
}

/// How much of a page's height one composition's specimen is worth.
///
/// Exhaustive with no wildcard, so a ninth composition is a compile error naming
/// this function rather than a specimen silently squeezed into whatever a
/// division by the new count leaves. The numbers are a ratio between structures,
/// not a geometry: nothing here is a size, and no surface resolves a layout from
/// them — they only say that a bank of sixteen columns needs more of a gallery
/// page than a one-band footer does.
const fn composition_seat_weight(composition: ShellComposition) -> f32 {
    match composition {
        // A whole frame: four bands and a workspace split.
        ShellComposition::ApplicationShell => 5.0,
        // Sixteen titled columns, each a group of cells.
        ShellComposition::MixerStripBank => 4.0,
        // A titled group of rows, and a titled panel of entries.
        ShellComposition::Section | ShellComposition::UtilityInspectorPanel => 3.0,
        // One band each.
        ShellComposition::ContextSwitch
        | ShellComposition::IdentityHeader
        | ShellComposition::PatchStripRow
        | ShellComposition::Footer => 1.0,
    }
}

/// States, in the gallery's own register, how much width the bank needs and how
/// much this column has.
///
/// The bank allocates the main surface rather than consuming it, so it divides
/// whatever width it is given into sixteen columns at the policy's authored
/// pitch. A gallery column is not a main surface — it is one of two compositions
/// sharing one window — so some of the sixteen fall outside it. Naming the
/// arithmetic is the honest form of that: the alternative is a specimen that
/// silently shows nine columns and lets a reader believe the bank has nine.
fn paint_bank_extent_note(
    painter: &mut SpecimenPainter<'_>,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
) {
    let geometry = policy.mixer_column();
    let available = stack.remaining().width();
    let seated = (available / geometry.pitch_px).floor().max(0.0);
    paint_wrapped(
        painter,
        stack,
        &format!(
            "sixteen columns need {:.0} px at {:.0} px pitch; this gallery column is {available:.0} px, so {seated:.0} seat here and the rest fall outside it",
            geometry.bank_width_px(),
            geometry.pitch_px,
        ),
        TypeStyle::InstructionHint,
        SemanticColor::AccentWarning,
    );
}

/// Paints one composition through the production render path and records what
/// it actually put on screen.
fn paint_composition_specimen(
    painter: &mut SpecimenPainter<'_>,
    ui: &mut egui::Ui,
    seat: Rect,
    policy: ViewportDensityPolicy,
    composition: ShellComposition,
    specimens: &GallerySpecimenSource,
) {
    let projection = specimens.projection_for(composition);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(seat)
            .layout(egui::Layout::top_down(egui::Align::Min))
            .id_salt((composition.canonical_name(), policy.canonical_name())),
    );
    child.set_clip_rect(seat);
    let runs = painted_runs(&mut child, |ui| {
        composition.render(ui, projection, &policy);
    });
    painter.record_component_runs(seat, composition.canonical_name(), &runs);
    painter.ledger.compositions_painted[policy_index(policy)][composition_index(composition)] =
        Some(runs.iter().map(|run| run.text.clone()).collect());
}

/// Marks a specimen the gallery could not paint, naming what is missing.
///
/// An empty region reads as "there is nothing here", which is not what happened.
/// The page says which specimen is absent so the shortfall the observation then
/// reports is actionable rather than mysterious.
fn mark_specimen_unavailable(painter: &mut SpecimenPainter<'_>, stack: &mut Stack, name: &str) {
    paint_wrapped(
        painter,
        stack,
        &format!(
            "{name} · SPECIMEN UNAVAILABLE · the production projection carried nothing to paint it with"
        ),
        TypeStyle::InstructionHint,
        SemanticColor::AccentWarning,
    );
}

/// How a policy's numbers were arrived at, in the words the vocabulary uses.
fn provenance_label(policy: ViewportDensityPolicy) -> &'static str {
    match policy.provenance() {
        crate::shell::visual::PolicyProvenance::MeasuredFromAuthoredDesign => {
            "measured from authored design"
        }
        crate::shell::visual::PolicyProvenance::AuthoredFromDesktopFrames => {
            "authored from desktop frames"
        }
    }
}

/// Page 1 — every declared color, grouped by role.
fn paint_colors_page(painter: &mut SpecimenPainter<'_>, stack: &mut Stack) {
    let groups: [(&str, &[SemanticColor]); 4] = [
        (
            "BACKGROUNDS",
            &[
                SemanticColor::BgCanvas,
                SemanticColor::BgSurface,
                SemanticColor::BgPanel,
                SemanticColor::BgElevated,
                SemanticColor::BgSelected,
            ],
        ),
        (
            "BORDERS",
            &[SemanticColor::BorderDefault, SemanticColor::BorderStrong],
        ),
        (
            "TEXT",
            &[
                SemanticColor::TextPrimary,
                SemanticColor::TextSecondary,
                SemanticColor::TextMuted,
            ],
        ),
        (
            "ACCENTS",
            &[
                SemanticColor::AccentFocus,
                SemanticColor::AccentAdjust,
                SemanticColor::AccentPositive,
                SemanticColor::AccentWarning,
                SemanticColor::AccentInstrument,
                SemanticColor::AccentPatch,
                SemanticColor::AccentChorus,
            ],
        ),
    ];
    for (title, colors) in groups {
        heading(painter, stack, title);
        for color in colors {
            let row = stack.row(SpacingStep::S24.resolve());
            let swatch = Rect::from_min_size(
                pos2(row.min.x, row.center().y - SpacingStep::S8.resolve()),
                Vec2::splat(SpacingStep::S16.resolve()),
            );
            painter.fill(swatch, *color, Radius::Small);
            painter.stroke(
                swatch,
                KEYLINE_RESTING_PX,
                SemanticColor::BorderDefault,
                Radius::Small,
            );
            painter.text_left_center(
                pos2(swatch.max.x + SpacingStep::S8.resolve(), row.center().y),
                color.canonical_name(),
                TypeStyle::LabelControl,
                SemanticColor::TextSecondary,
                ComponentState::Resting,
            );
            painter.text_right_center(
                pos2(row.max.x, row.center().y),
                &hex_of(*color),
                TypeStyle::CodeValue,
                SemanticColor::TextMuted,
                ComponentState::Resting,
            );
        }
        stack.gap(SpacingStep::S8);
    }
}

/// The authored color as the design file writes it.
fn hex_of(color: SemanticColor) -> String {
    let resolved = color.resolve();
    format!(
        "#{:02x}{:02x}{:02x}",
        resolved.r(),
        resolved.g(),
        resolved.b()
    )
}

/// Page 2 — every type style, set in itself.
fn paint_type_page(painter: &mut SpecimenPainter<'_>, stack: &mut Stack) {
    for style in ALL_TYPE_STYLES {
        let metrics = style.metrics();
        let caption = stack.row(caption_height());
        painter.text_left_center(
            pos2(caption.min.x, caption.center().y),
            &format!(
                "{} · {}/{} · {:?} · {}",
                style.canonical_name(),
                metrics.size_px,
                metrics.line_height_px,
                metrics.weight,
                metrics.tracking_px
            ),
            TypeStyle::LabelControl,
            SemanticColor::TextMuted,
            ComponentState::Resting,
        );
        let specimen = stack.row(metrics.line_height_px);
        painter.text_left_center(
            pos2(specimen.min.x, specimen.center().y),
            type_specimen(style),
            style,
            SemanticColor::TextPrimary,
            ComponentState::Resting,
        );
        stack.gap(SpacingStep::S8);
    }
}

/// Representative content for one type style.
///
/// Drawn from the vocabulary the design file uses, not from lorem ipsum: a
/// reviewer judging tracking on `PATCH 00 · LEAD PAD` is judging the string the
/// product actually paints.
const fn type_specimen(style: TypeStyle) -> &'static str {
    match style {
        TypeStyle::DisplayScreen => "PATCH 00 · LEAD PAD",
        TypeStyle::HeadingSection => "PATCH · MIXER",
        TypeStyle::HeadingPanel => "UTILITY / INSPECTOR",
        TypeStyle::BodyDefault => "HiDef SoundFont · Bank 0",
        TypeStyle::BodyCompact => "Braids · WAVETABLE",
        TypeStyle::LabelControl => "CUTOFF",
        TypeStyle::CodeValue => "0.750",
        TypeStyle::InstructionHint => "D-PAD NAV · A CONFIRM",
    }
}

/// Page 3 — the spacing scale, the radii, the keyline widths, the target bound.
fn paint_geometry_page(painter: &mut SpecimenPainter<'_>, stack: &mut Stack) {
    heading(painter, stack, "SPACING");
    for step in ALL_SPACING_STEPS {
        let row = stack.row(SpacingStep::S24.resolve());
        let bar = Rect::from_min_size(
            pos2(row.min.x, row.center().y - SpacingStep::S4.resolve()),
            Vec2::new(step.resolve(), SpacingStep::S8.resolve()),
        );
        painter.fill(bar, SemanticColor::AccentFocus, Radius::None);
        painter.text_left_center(
            pos2(
                row.min.x + SpacingStep::S32.resolve() + SpacingStep::S8.resolve(),
                row.center().y,
            ),
            &format!("{} · {} px", step.canonical_name(), step.resolve()),
            TypeStyle::LabelControl,
            SemanticColor::TextSecondary,
            ComponentState::Resting,
        );
    }
    stack.gap(SpacingStep::S8);

    heading(painter, stack, "RADII");
    for (radius, name) in [
        (Radius::None, "none"),
        (Radius::Small, "small · controls"),
        (Radius::Large, "large · panels"),
    ] {
        let row = stack.row(SpacingStep::S32.resolve());
        let sample = Rect::from_min_size(
            pos2(row.min.x, row.min.y),
            Vec2::new(SpacingStep::S32.resolve(), SpacingStep::S24.resolve()),
        );
        painter.fill(sample, SemanticColor::BgElevated, radius);
        painter.stroke(
            sample,
            KEYLINE_RESTING_PX,
            SemanticColor::BorderStrong,
            radius,
        );
        painter.text_left_center(
            pos2(sample.max.x + SpacingStep::S8.resolve(), row.center().y),
            &format!("{name} · {} px", radius.resolve()),
            TypeStyle::LabelControl,
            SemanticColor::TextSecondary,
            ComponentState::Resting,
        );
    }
    stack.gap(SpacingStep::S8);

    heading(painter, stack, "KEYLINES");
    let rule_width = SpacingStep::S32.resolve() * ALL_SPACING_STEPS.len() as f32;
    let resting = stack.row(SpacingStep::S24.resolve());
    painter.hairline(rules::RuleSpan::Horizontal {
        y_px: resting.center().y,
        from_x_px: resting.min.x,
        to_x_px: resting.min.x + rule_width,
    });
    painter.text_left_center(
        pos2(
            resting.min.x + rule_width + SpacingStep::S8.resolve(),
            resting.center().y,
        ),
        &format!("hairline · border/default · {KEYLINE_RESTING_PX} px"),
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        ComponentState::Resting,
    );
    let structural = stack.row(SpacingStep::S24.resolve());
    painter.keyline(rules::RuleSpan::Horizontal {
        y_px: structural.center().y,
        from_x_px: structural.min.x,
        to_x_px: structural.min.x + rule_width,
    });
    painter.text_left_center(
        pos2(
            structural.min.x + rule_width + SpacingStep::S8.resolve(),
            structural.center().y,
        ),
        &format!("keyline · border/strong · {KEYLINE_RESTING_PX} px"),
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        ComponentState::Resting,
    );
    let emphasis = stack.row(SpacingStep::S24.resolve());
    let emphasis_bar = Rect::from_min_size(
        pos2(
            emphasis.min.x,
            emphasis.center().y
                - KEYLINE_EMPHASIS_PX / SpacingStep::S8.resolve() * SpacingStep::S4.resolve(),
        ),
        Vec2::new(rule_width, KEYLINE_EMPHASIS_PX),
    );
    painter.fill(emphasis_bar, SemanticColor::AccentFocus, Radius::None);
    painter.text_left_center(
        pos2(
            emphasis.min.x + rule_width + SpacingStep::S8.resolve(),
            emphasis.center().y,
        ),
        &format!("emphasis · accent/focus · {KEYLINE_EMPHASIS_PX} px"),
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        ComponentState::Resting,
    );
    stack.gap(SpacingStep::S8);

    heading(painter, stack, "MINIMUM INTERACTIVE TARGET");
    let target_row = stack.row(MIN_INTERACTIVE_TARGET_PX);
    let target = Rect::from_min_size(target_row.min, Vec2::splat(MIN_INTERACTIVE_TARGET_PX));
    painter.fill(target, SemanticColor::BgElevated, Radius::Small);
    painter.stroke(
        target,
        KEYLINE_EMPHASIS_PX,
        SemanticColor::AccentPositive,
        Radius::Small,
    );
    painter.text_left_center(
        pos2(
            target.max.x + SpacingStep::S8.resolve(),
            target_row.center().y,
        ),
        &format!("{MIN_INTERACTIVE_TARGET_PX} px · every interactive target's floor"),
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        ComponentState::Resting,
    );
}

/// Page 4 — the three interaction states, side by side.
fn paint_interaction_states_page(
    painter: &mut SpecimenPainter<'_>,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
) {
    heading(painter, stack, "INTERACTION STATES");
    let rhythm = policy.rhythm();
    for state in [
        ComponentState::Resting,
        ComponentState::Focused,
        ComponentState::Adjusting,
    ] {
        let caption = stack.row(caption_height());
        let appearance = state.appearance();
        painter.text_left_center(
            pos2(caption.min.x, caption.center().y),
            &format!(
                "{} · {} · keyline {} px{}",
                state.canonical_name(),
                appearance.accent.canonical_name(),
                appearance.keyline_px,
                if appearance.draws_halo {
                    " · halo"
                } else {
                    ""
                }
            ),
            TypeStyle::LabelControl,
            SemanticColor::TextMuted,
            ComponentState::Resting,
        );
        let row = stack.row(rhythm.row_height_px);
        painter.state_specimen(
            row,
            Specimen {
                state,
                policy,
                label: "CUTOFF",
                value: "0.750",
                detail: StatusDetail::None,
            },
        );
        stack.gap(SpacingStep::S12);
    }
}

/// Page 5 — text roles at every state, plus the two separator weights.
fn paint_text_and_hairlines_page(
    painter: &mut SpecimenPainter<'_>,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
) {
    heading(painter, stack, "TEXT ROLES BY STATE");
    let rhythm = policy.rhythm();
    for state in ALL_COMPONENT_STATES {
        let row = stack.row(rhythm.row_height_px);
        painter.state_specimen(
            row,
            Specimen {
                state,
                policy,
                label: state.canonical_name(),
                value: "0.750",
                detail: state_detail(state),
            },
        );
        stack.gap(SpacingStep::S4);
    }
    stack.gap(SpacingStep::S8);

    heading(painter, stack, "SEPARATORS");
    let rule_width = SpacingStep::S32.resolve() * ALL_SPACING_STEPS.len() as f32;
    let hairline_row = stack.row(SpacingStep::S24.resolve());
    painter.hairline(rules::RuleSpan::Horizontal {
        y_px: hairline_row.center().y,
        from_x_px: hairline_row.min.x,
        to_x_px: hairline_row.min.x + rule_width,
    });
    painter.text_left_center(
        pos2(
            hairline_row.min.x + rule_width + SpacingStep::S8.resolve(),
            hairline_row.center().y,
        ),
        "horizontal hairline",
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        ComponentState::Resting,
    );
    let keyline_row = stack.row(SpacingStep::S24.resolve());
    painter.keyline(rules::RuleSpan::Horizontal {
        y_px: keyline_row.center().y,
        from_x_px: keyline_row.min.x,
        to_x_px: keyline_row.min.x + rule_width,
    });
    painter.text_left_center(
        pos2(
            keyline_row.min.x + rule_width + SpacingStep::S8.resolve(),
            keyline_row.center().y,
        ),
        "horizontal keyline",
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        ComponentState::Resting,
    );
    let vertical_row = stack.row(SpacingStep::S32.resolve());
    painter.hairline(rules::RuleSpan::Vertical {
        x_px: vertical_row.min.x,
        from_y_px: vertical_row.min.y,
        to_y_px: vertical_row.max.y,
    });
    painter.keyline(rules::RuleSpan::Vertical {
        x_px: vertical_row.min.x + SpacingStep::S16.resolve(),
        from_y_px: vertical_row.min.y,
        to_y_px: vertical_row.max.y,
    });
    painter.text_left_center(
        pos2(
            vertical_row.min.x + SpacingStep::S32.resolve(),
            vertical_row.center().y,
        ),
        "vertical hairline and keyline",
        TypeStyle::LabelControl,
        SemanticColor::TextSecondary,
        ComponentState::Resting,
    );
}

/// Page 6 — the value column and every status mark.
fn paint_values_and_status_page(
    painter: &mut SpecimenPainter<'_>,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
) {
    heading(painter, stack, "VALUES AND STATUS MARKS");
    let rhythm = policy.rhythm();
    for (state, detail, label, value) in status_specimens() {
        let row = stack.row(rhythm.row_height_px);
        // The state name leads the realistic content rather than replacing it.
        // This page exists to show the marks against the strings the product
        // paints, but a row a reader cannot name the state of is not judgable —
        // so `Loading · ENGINE` says both what it is and which state it is in.
        let named = format!("{} · {label}", state.canonical_name());
        painter.state_specimen(
            row,
            Specimen {
                state,
                policy,
                label: &named,
                value,
                detail,
            },
        );
        stack.gap(SpacingStep::S4);
    }
}

/// Every status specimen the page renders.
///
/// `Loading` appears twice because it carries two authored progress words and a
/// reviewer must be able to see both.
fn status_specimens() -> Vec<(
    ComponentState,
    StatusDetail<'static>,
    &'static str,
    &'static str,
)> {
    vec![
        (
            ComponentState::Resting,
            StatusDetail::None,
            "MASTER GAIN",
            "-6.000",
        ),
        (
            ComponentState::Focused,
            StatusDetail::None,
            "CUTOFF",
            "0.750",
        ),
        (
            ComponentState::Adjusting,
            StatusDetail::None,
            "RESONANCE",
            "0.312",
        ),
        (
            ComponentState::Disabled,
            StatusDetail::None,
            "ASSET SLOT",
            "HiDef.sf2",
        ),
        (
            ComponentState::Loading,
            StatusDetail::Progress(LoadingPhase::Preparing),
            "ENGINE",
            "Braids",
        ),
        (
            ComponentState::Loading,
            StatusDetail::Progress(LoadingPhase::Activating),
            "ENGINE",
            "Braids",
        ),
        (
            ComponentState::Error,
            StatusDetail::Failure("PRESET MISSING"),
            "ENGINE",
            "HiDef",
        ),
        (
            ComponentState::Muted,
            StatusDetail::None,
            "TRACK 03",
            "0.500",
        ),
        (
            ComponentState::Soloed,
            StatusDetail::None,
            "TRACK 04",
            "0.800",
        ),
        (
            ComponentState::Selected,
            StatusDetail::None,
            "TRACK 05",
            "0.640",
        ),
    ]
}

/// The detail a state carries when a page renders it generically.
fn state_detail(state: ComponentState) -> StatusDetail<'static> {
    match state {
        ComponentState::Loading => StatusDetail::Progress(LoadingPhase::Preparing),
        ComponentState::Error => StatusDetail::Failure("PRESET MISSING"),
        ComponentState::Resting
        | ComponentState::Focused
        | ComponentState::Adjusting
        | ComponentState::Disabled
        | ComponentState::Muted
        | ComponentState::Soloed
        | ComponentState::Selected => StatusDetail::None,
    }
}

/// Page 7 — the four hint tones and the composed line.
fn paint_action_hints_page(painter: &mut SpecimenPainter<'_>, stack: &mut Stack) {
    heading(painter, stack, "HINT TONES");
    for tone in ALL_HINT_TONES {
        let row = stack.row(SpacingStep::S32.resolve());
        painter.text_left_center(
            pos2(row.min.x, row.center().y),
            &format!(
                "{} · {}",
                tone.canonical_name(),
                tone.color().canonical_name()
            ),
            TypeStyle::LabelControl,
            SemanticColor::TextMuted,
            ComponentState::Resting,
        );
        painter.hint_line_at(
            pos2(row.max.x, row.center().y),
            Align2::RIGHT_CENTER,
            &[ActionHint::new(tone_chord(tone), tone_action(tone), tone)],
        );
    }
    stack.gap(SpacingStep::S8);

    heading(painter, stack, "COMPOSED LINE");
    let composed = stack.row(SpacingStep::S32.resolve());
    painter.hint_line_at(
        pos2(composed.min.x, composed.center().y),
        Align2::LEFT_CENTER,
        &[
            ActionHint::new("D-PAD", "NAV", HintTone::Focus),
            ActionHint::new("A", "CONFIRM", HintTone::Neutral),
            ActionHint::new("LEFT/RIGHT", "ADJUST", HintTone::Adjust),
            ActionHint::new("B", "BACK", HintTone::Back),
        ],
    );
}

/// The control a tone is demonstrated with.
const fn tone_chord(tone: HintTone) -> &'static str {
    match tone {
        HintTone::Neutral => "A",
        HintTone::Focus => "D-PAD",
        HintTone::Adjust => "LEFT/RIGHT",
        HintTone::Back => "B",
    }
}

/// The action a tone is demonstrated with.
const fn tone_action(tone: HintTone) -> &'static str {
    match tone {
        HintTone::Neutral => "CONFIRM",
        HintTone::Focus => "NAV",
        HintTone::Adjust => "ADJUST",
        HintTone::Back => "BACK",
    }
}

/// Page 8 — the five structural regions, drawn to scale for this policy.
///
/// This is the page the Steam Deck composition is reviewed on: it is the only
/// human check on the policy the vocabulary records as authored rather than
/// measured, so both compositions are on screen at once and each is named.
fn paint_shell_bands_page(
    painter: &mut SpecimenPainter<'_>,
    stack: &mut Stack,
    policy: ViewportDensityPolicy,
) {
    heading(painter, stack, "STRUCTURAL BANDS");
    let viewport = policy.authored_viewport();
    let bands = policy.bands();
    let split = policy.split();
    // The region names are the canonical `ShellRegionId` names rather than
    // prose, so the diagram and the frame observation the production shell
    // emits name the same five regions the same way.
    let names: [(ShellRegionId, SemanticColor, String); 5] = [
        (
            ShellRegionId::ContextLine,
            SemanticColor::BgElevated,
            format!(
                "{} · {} px",
                ShellRegionId::ContextLine.name(),
                bands.context_line_px
            ),
        ),
        (
            ShellRegionId::IdentityHeader,
            SemanticColor::BgPanel,
            format!(
                "{} · {} px",
                ShellRegionId::IdentityHeader.name(),
                bands.identity_header_px
            ),
        ),
        (
            ShellRegionId::MainWorkspace,
            SemanticColor::BgCanvas,
            format!(
                "{} · {}×{}",
                ShellRegionId::MainWorkspace.name(),
                split.main_px,
                bands.workspace_px
            ),
        ),
        (
            ShellRegionId::PersistentSideRegion,
            SemanticColor::BgPanel,
            format!(
                "{} · {}×{}",
                ShellRegionId::PersistentSideRegion.name(),
                split.side_px,
                bands.workspace_px
            ),
        ),
        (
            ShellRegionId::Footer,
            SemanticColor::BgElevated,
            format!("{} · {} px", ShellRegionId::Footer.name(), bands.footer_px),
        ),
    ];

    // The name column is sized to the longest name as it will actually be laid
    // out, so the diagram takes every pixel the names do not need. A guessed
    // fraction either clips a name or wastes the space that makes the Steam
    // Deck composition reviewable, and this page is the only human check on it.
    let name_column_px = names
        .iter()
        .map(|(_, _, name)| painter.measure_width(name, TypeStyle::LabelControl))
        .fold(0.0_f32, f32::max);
    let available = stack.remaining();
    let diagram_width = available.width() - name_column_px - SpacingStep::S16.resolve();
    let scale = (diagram_width / viewport.width_px).min(available.height() / viewport.height_px);
    let diagram = Rect::from_min_size(
        available.min,
        Vec2::new(viewport.width_px * scale, viewport.height_px * scale),
    );

    let (context_line, rest) =
        diagram.split_top_bottom_at_y(diagram.min.y + bands.context_line_px * scale);
    let (identity_header, rest) =
        rest.split_top_bottom_at_y(rest.min.y + bands.identity_header_px * scale);
    let (workspace, footer) = rest.split_top_bottom_at_y(rest.max.y - bands.footer_px * scale);
    let (main, side) = workspace.split_left_right_at_x(workspace.min.x + split.main_px * scale);
    let rects = [context_line, identity_header, main, side, footer];

    // Names are stacked beside the diagram rather than inside it: the side
    // region is narrower than its own name, so a name placed inside would
    // overflow the region it names. Each name starts at its band's top edge, or
    // below the previous name when two bands are closer together than one line
    // — so no two names collide however far the diagram shrinks.
    let name_x = diagram.max.x + SpacingStep::S16.resolve();
    let mut name_top = diagram.min.y;
    for ((id, fill, name), rect) in names.into_iter().zip(rects) {
        painter.fill(rect, fill, Radius::None);
        painter.stroke(
            rect,
            KEYLINE_RESTING_PX,
            SemanticColor::BorderStrong,
            Radius::None,
        );
        let top = rect.min.y.max(name_top);
        let painted = painter.text_at(
            pos2(name_x, top),
            &name,
            TypeStyle::LabelControl,
            SemanticColor::TextSecondary,
            ComponentState::Resting,
        );
        name_top = painted.max.y + SpacingStep::S4.resolve();
        painter.ledger.bands_painted[policy_index(policy)][id as usize] = true;
    }

    let summary_top = diagram.max.y.max(name_top) + SpacingStep::S8.resolve();
    painter.text_at(
        pos2(available.min.x, summary_top),
        &format!(
            "bands total {} px · split total {} px",
            bands.total_height_px(),
            split.total_width_px()
        ),
        TypeStyle::LabelControl,
        SemanticColor::TextMuted,
        ComponentState::Resting,
    );
}

/// What a native window spends on chrome the gallery never paints.
///
/// Measured on macOS: a 30 px menu bar above the frame and a 32 px title bar
/// inside it, 62 px in total. Declared at 96 so a taller system bar or a
/// different scale factor cannot silently reintroduce the failure it guards
/// against. A window 40 px smaller than it could be costs a reviewer nothing;
/// a window one pixel taller than the display puts the footer band — which
/// carries the only on-screen browsing affordance — under the screen edge,
/// where a pinned minimum size makes it unreachable.
const GALLERY_WINDOW_CHROME_PX: f32 = 96.0;

/// The measured height at which every gallery page composes without a defect.
///
/// Read off the paint pass rather than estimated, the way the row geometry in
/// the density policies was: at 848 px the vertical separator on page 5 runs
/// past the stage and the pass reports it clipped; at 856 px all fifteen pages
/// compose clean in both columns. The tests below hold both ends, so this
/// cannot drift away from the layout it describes.
const MINIMUM_GALLERY_HEIGHT_PX: f32 = 856.0;

// The window must fit the display it is reviewed on. Held at compile time
// rather than only in a test, because the two constants above are edited by
// people measuring layout, and the consequence of getting this wrong is
// invisible from inside the paint pass.
const _: () = assert!(
    MINIMUM_GALLERY_HEIGHT_PX + GALLERY_WINDOW_CHROME_PX
        <= ViewportDensityPolicy::Desktop.authored_viewport().height_px,
    "the gallery window is taller than the authored desktop display it is reviewed on"
);

/// The narrowest window at which both columns seat two specimens side by side.
///
/// Derived, not chosen. A control page shows two controls at up to nine states
/// each, which does not fit stacked, so each control takes half of its policy's
/// column — and half a column narrower than the narrowest *authored* control
/// extent shows the operator a control colliding with itself at a width the
/// product never renders it at. So the floor is the width at which each policy's
/// column seats two of its own authored control widths with a gutter between
/// them, divided back out through the stage split.
///
/// The compact authored viewport width is the other floor, kept because the
/// gallery was reviewed at it and nothing should shrink below it.
fn minimum_gallery_width_px() -> f32 {
    let required = |policy: ViewportDensityPolicy| {
        let control = policy.utility_control().width_px;
        let inset = policy.rhythm().inset_px;
        2.0f32.mul_add(control + inset, SpacingStep::S16.resolve())
    };
    let desktop_fraction = desktop_stage_fraction();
    let desktop = required(ViewportDensityPolicy::Desktop) / desktop_fraction;
    let deck = required(ViewportDensityPolicy::SteamDeck) / (1.0 - desktop_fraction);
    desktop.max(deck).max(
        ViewportDensityPolicy::SteamDeck
            .authored_viewport()
            .width_px,
    )
}

/// The smallest window the gallery composes without clipping a specimen.
///
/// The width comes from [`minimum_gallery_width_px`] and the height from
/// [`MINIMUM_GALLERY_HEIGHT_PX`].
///
/// This is deliberately *not* the desktop authored viewport. Both compositions
/// share one window, which tempts the minimum upward toward the larger authored
/// size — but a window as tall as the display it is reviewed on does not fit on
/// that display once the system's own chrome is counted, and pinning the
/// minimum there removes the operator's only remedy. The two authored *product*
/// viewports are not what this bounds: they are both rendered inside this
/// window, side by side, at their own densities.
pub fn minimum_gallery_viewport() -> crate::shell::visual::AuthoredViewport {
    crate::shell::visual::AuthoredViewport {
        width_px: minimum_gallery_width_px(),
        height_px: MINIMUM_GALLERY_HEIGHT_PX,
    }
}

/// The browsable component gallery scene.
///
/// Owns the window, the page selection, and one production `AppState` held only
/// so the generation claim is measured rather than asserted.
pub struct ComponentGalleryScene {
    app_state: AppState,
}

impl ComponentGalleryScene {
    /// Builds the scene and the production reducer its observation measures
    /// against.
    ///
    /// The reducer is built from a production capability provider that needs no
    /// asset file: the gallery paints, it does not sound, and a visual scene
    /// that fails to start because a SoundFont is missing would be coupling the
    /// two for no gain.
    pub fn new() -> Result<Self, ComponentGalleryError> {
        let braids = crate::adapter::braids_capability::BraidsCapability::new()
            .map_err(ComponentGalleryError::Capability)?;
        let registry = CapabilityRegistry::new(vec![braids.descriptor()])
            .map_err(ComponentGalleryError::Capability)?;
        Ok(Self {
            app_state: AppState::new(registry, gallery_global_parameters()),
        })
    }

    /// Opens the window and returns what was actually painted.
    ///
    /// The authored faces are read before the window exists, so an unavailable
    /// face is a typed startup failure naming the file rather than a window that
    /// opens and paints in a substituted face.
    ///
    /// There is no milestone timeout and no total timeout. Those exist to stop
    /// an autonomous witness hanging; this scene waits for the operator by
    /// design and finishes when the window closes.
    pub fn run(self) -> Result<ComponentGalleryObservation, ComponentGalleryError> {
        let typeface = AuthoredTypeface::load().map_err(ComponentGalleryError::Typeface)?;
        let generation_before = self.app_state.generation();
        let ledger = Rc::new(RefCell::new(GalleryPaintLedger::default()));
        let ledger_for_window = Rc::clone(&ledger);

        let smallest = minimum_gallery_viewport();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([smallest.width_px, smallest.height_px])
                .with_min_inner_size([smallest.width_px, smallest.height_px]),
            ..Default::default()
        };
        eframe::run_native(
            COMPONENT_GALLERY_WINDOW_TITLE,
            options,
            Box::new(move |creation_context| {
                creation_context
                    .egui_ctx
                    .set_fonts(typeface.font_definitions());
                Ok(Box::new(ComponentGalleryApplication::new(
                    ledger_for_window,
                )))
            }),
        )
        .map_err(|error| ComponentGalleryError::Window(error.to_string()))?;

        // The window has closed and released everything it owned. Only now is
        // the reducer read again, so the delta covers the whole session.
        let generation_after = self.app_state.generation();
        let delta = i64::try_from(generation_after).unwrap_or(i64::MAX)
            - i64::try_from(generation_before).unwrap_or(i64::MAX);
        let painted = ledger.borrow();
        Ok(ComponentGalleryObservation::from_paint(
            &painted, delta, true,
        ))
    }
}

/// The globals the gallery's reducer witness starts from.
fn gallery_global_parameters() -> GlobalParameters {
    ApplicationConfig::default().global_parameters()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::app_event::AppEvent;
    use crate::control::TopLevelContext;
    use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
    use eframe::egui::RawInput;

    /// Runs the real paint path headlessly at one window size, walking the
    /// supplied pages, and returns the ledger the paint pass filled in.
    ///
    /// This is the production render path: the same `paint_gallery` the window
    /// calls, through a real `egui::Context` with the authored faces installed.
    fn paint_pages(size: Vec2, pages: &[ComponentGalleryPage]) -> GalleryPaintLedger {
        let context = egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the vendored faces are present")
                .font_definitions(),
        );
        let screen = Rect::from_min_size(Pos2::ZERO, size);
        let input = RawInput {
            screen_rect: Some(screen),
            ..RawInput::default()
        };
        let mut ledger = GalleryPaintLedger::default();
        // One warm-up pass so the requested faces are resident before anything
        // is measured; a first-frame miss would be a harness artifact rather
        // than a rendering failure.
        run_pass(
            &context,
            &input,
            ALL_GALLERY_PAGES[0],
            &mut GalleryPaintLedger::default(),
        );
        for page in pages {
            run_pass(&context, &input, *page, &mut ledger);
        }
        ledger
    }

    /// Runs one real pass and tessellates it, so what the ledger recorded is
    /// backed by shapes that actually reached the tessellator.
    fn run_pass(
        context: &egui::Context,
        input: &RawInput,
        page: ComponentGalleryPage,
        ledger: &mut GalleryPaintLedger,
    ) {
        let output = context.run(input.clone(), |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::ZERO))
                .show(context, |ui| {
                    paint_gallery(ui, page, ledger);
                });
        });
        let primitives = context.tessellate(output.shapes, output.pixels_per_point);
        assert!(
            !primitives.is_empty(),
            "{} produced no tessellated geometry",
            page.canonical_name()
        );
    }

    /// The smallest window the gallery may be opened at, which is also the size
    /// it opens at. Everything is measured here: a specimen that fits only in a
    /// larger window is a specimen the operator can be shown clipped.
    fn gallery_window() -> Vec2 {
        let smallest = minimum_gallery_viewport();
        Vec2::new(smallest.width_px, smallest.height_px)
    }

    #[test]
    fn the_gallery_declares_exactly_fifteen_pages() {
        assert_eq!(GALLERY_PAGE_COUNT, 15);
        assert_eq!(ALL_GALLERY_PAGES.len(), GALLERY_PAGE_COUNT);
        let names: BTreeSet<&str> = ALL_GALLERY_PAGES
            .iter()
            .map(|page| page.canonical_name())
            .collect();
        assert_eq!(names.len(), GALLERY_PAGE_COUNT, "two pages share a name");
        let pages: BTreeSet<ComponentGalleryPage> = ALL_GALLERY_PAGES.into_iter().collect();
        assert_eq!(pages.len(), GALLERY_PAGE_COUNT, "a page appears twice");
    }

    /// T037 — FR-012 as a regression gate.
    ///
    /// The eight bindings that existed before the control and composition pages
    /// were added are compared against the frozen baseline exactly and in
    /// order. Reordering [`ALL_GALLERY_PAGES`], renaming one of the eight, or
    /// handing one a different digit fails here.
    #[test]
    fn the_eight_pre_existing_digit_bindings_are_exactly_the_frozen_baseline() {
        let current: Vec<(&str, WindowKey)> = ALL_GALLERY_PAGES
            .into_iter()
            .take(FROZEN_DIGIT_BINDING_BASELINE.len())
            .map(|page| {
                (
                    page.canonical_name(),
                    page.digit().expect("one of the first eight pages"),
                )
            })
            .collect();
        // Exact ordered equality, not a containment check: a containment check
        // passes when two of the eight swap digits, and swapping two digits is
        // exactly the change FR-012 exists to catch.
        assert_eq!(
            current,
            FROZEN_DIGIT_BINDING_BASELINE.to_vec(),
            "a pre-existing gallery binding moved; FR-012 forbids it"
        );
    }

    /// Every page is reachable, the ten digit bindings are unique, and no key
    /// outside them selects a page.
    #[test]
    fn ten_digits_bind_ten_pages_and_stepping_reaches_the_rest() {
        let mut digits = std::collections::HashSet::new();
        let mut bound = 0;
        for page in ALL_GALLERY_PAGES {
            match page.digit() {
                Some(digit) => {
                    bound += 1;
                    assert!(
                        digits.insert(digit),
                        "{} shares its digit with another page",
                        page.canonical_name()
                    );
                    assert_eq!(
                        ComponentGalleryPage::for_digit(digit),
                        Some(page),
                        "{} is not reachable by its own digit",
                        page.canonical_name()
                    );
                    assert!(page.digit_label().is_some());
                }
                None => assert!(
                    page.digit_label().is_none(),
                    "{} reads as a digit it does not bind",
                    page.canonical_name()
                ),
            }
        }
        assert_eq!(bound, GALLERY_DIGIT_BINDING_COUNT);
        assert_eq!(digits.len(), GALLERY_DIGIT_BINDING_COUNT, "a digit repeats");

        // The ten bindings are the first ten pages in declared order.
        for (index, page) in ALL_GALLERY_PAGES.into_iter().enumerate() {
            assert_eq!(page.index(), index);
            assert_eq!(page.digit().is_some(), index < GALLERY_DIGIT_BINDING_COUNT);
        }

        for key in [
            WindowKey::Q,
            WindowKey::E,
            WindowKey::W,
            WindowKey::S,
            WindowKey::A,
            WindowKey::D,
            WindowKey::K,
            WindowKey::BracketLeft,
            WindowKey::BracketRight,
            WindowKey::Other,
        ] {
            assert_eq!(
                ComponentGalleryPage::for_digit(key),
                None,
                "{key:?} binds a page it should not"
            );
        }
    }

    /// The ninth and tenth pages in declared order carry the two new digits.
    #[test]
    fn the_two_added_digits_bind_the_ninth_and_tenth_pages() {
        assert_eq!(
            ComponentGalleryPage::for_digit(WindowKey::Digit9),
            Some(ComponentGalleryPage::ParameterAndChoiceRows)
        );
        assert_eq!(
            ComponentGalleryPage::for_digit(WindowKey::Digit0),
            Some(ComponentGalleryPage::TogglesAndSliders)
        );
        assert_eq!(
            ALL_GALLERY_PAGES[8],
            ComponentGalleryPage::ParameterAndChoiceRows
        );
        assert_eq!(
            ALL_GALLERY_PAGES[9],
            ComponentGalleryPage::TogglesAndSliders
        );
    }

    #[test]
    fn a_bound_digit_changes_the_page_and_anything_else_retains_it() {
        let mut selection = GalleryPageSelection::default();
        assert_eq!(selection.active(), ComponentGalleryPage::Colors);

        for page in ALL_GALLERY_PAGES {
            let Some(digit) = page.digit() else {
                continue;
            };
            assert_eq!(
                selection.apply(WindowInput::key_down(digit)),
                PageSelection::Changed(page)
            );
            assert_eq!(selection.active(), page);
        }

        let settled = selection.active();
        for input in [
            WindowInput::key_down(WindowKey::Other),
            WindowInput::key_down(WindowKey::Q),
            WindowInput::key_up(WindowKey::Digit1),
            WindowInput::focus_lost(),
        ] {
            assert_eq!(selection.apply(input), PageSelection::Retained(settled));
            assert_eq!(selection.active(), settled);
        }
    }

    /// T036 — stepping alone reaches all fifteen, both ways.
    #[test]
    fn stepping_from_either_end_visits_every_declared_page() {
        let mut selection = GalleryPageSelection::default();
        assert_eq!(selection.active(), ALL_GALLERY_PAGES[0]);

        let mut forward = vec![selection.active()];
        for _ in 1..GALLERY_PAGE_COUNT {
            let step = selection.apply(WindowInput::key_down(PageStep::Next.key()));
            assert!(
                matches!(step, PageSelection::Stepped(_)),
                "a forward step short of the last page did not move: {step:?}"
            );
            forward.push(selection.active());
        }
        assert_eq!(
            forward,
            ALL_GALLERY_PAGES.to_vec(),
            "stepping forward did not visit the declared order exactly"
        );

        let mut backward = vec![selection.active()];
        for _ in 1..GALLERY_PAGE_COUNT {
            selection.apply(WindowInput::key_down(PageStep::Previous.key()));
            backward.push(selection.active());
        }
        backward.reverse();
        assert_eq!(
            backward,
            ALL_GALLERY_PAGES.to_vec(),
            "stepping back did not visit the declared order exactly"
        );
    }

    /// T036 — stepping does not wrap, at either end.
    #[test]
    fn a_step_past_either_end_retains_the_end_page() {
        let first = ALL_GALLERY_PAGES[0];
        let last = ALL_GALLERY_PAGES[GALLERY_PAGE_COUNT - 1];

        let mut selection = GalleryPageSelection::default();
        assert_eq!(selection.active(), first);
        for _ in 0..3 {
            assert_eq!(
                selection.apply(WindowInput::key_down(PageStep::Previous.key())),
                PageSelection::Retained(first),
                "a previous-step at the first page wrapped"
            );
            assert_eq!(selection.active(), first);
        }

        for _ in 1..GALLERY_PAGE_COUNT {
            selection.apply(WindowInput::key_down(PageStep::Next.key()));
        }
        assert_eq!(selection.active(), last);
        for _ in 0..3 {
            assert_eq!(
                selection.apply(WindowInput::key_down(PageStep::Next.key())),
                PageSelection::Retained(last),
                "a next-step at the last page wrapped"
            );
            assert_eq!(selection.active(), last);
        }
    }

    /// A step key is only a step on key-down, like every other binding here.
    #[test]
    fn a_step_key_up_and_a_focus_loss_move_nothing() {
        let mut selection = GalleryPageSelection::default();
        selection.apply(WindowInput::key_down(PageStep::Next.key()));
        let settled = selection.active();
        for input in [
            WindowInput::key_up(PageStep::Next.key()),
            WindowInput::key_up(PageStep::Previous.key()),
            WindowInput::focus_lost(),
        ] {
            assert_eq!(selection.apply(input), PageSelection::Retained(settled));
        }
    }

    /// The scene normalizes the two bracket keys, and every key it does not
    /// bind still normalizes to the catch-all.
    #[test]
    fn the_bracket_keys_normalize_and_every_other_key_binds_nothing() {
        assert_eq!(
            normalize_gallery_key(egui::Key::OpenBracket),
            WindowKey::BracketLeft
        );
        assert_eq!(
            normalize_gallery_key(egui::Key::CloseBracket),
            WindowKey::BracketRight
        );
        assert_eq!(normalize_gallery_key(egui::Key::Num9), WindowKey::Digit9);
        assert_eq!(normalize_gallery_key(egui::Key::Num0), WindowKey::Digit0);
        assert_eq!(normalize_gallery_key(egui::Key::Z), WindowKey::Other);
    }

    /// A key that binds no page retains the current one.
    ///
    /// This used to be demonstrated with `9`, and cannot be any more: all ten
    /// digits now select pages. What still binds nothing is every key beyond
    /// the twelve the scene knows, and the property being held is unchanged —
    /// an input the scene has no meaning for changes nothing.
    #[test]
    fn a_key_binding_no_page_retains_the_current_page() {
        let mut selection = GalleryPageSelection::default();
        selection.apply(WindowInput::key_down(
            ComponentGalleryPage::ValuesAndStatus
                .digit()
                .expect("ValuesAndStatus carries a digit"),
        ));
        let before = selection.active();
        assert_eq!(before, ComponentGalleryPage::ValuesAndStatus);

        let unbound = normalize_gallery_key(egui::Key::Z);
        assert_eq!(ComponentGalleryPage::for_digit(unbound), None);
        assert_eq!(PageStep::for_key(unbound), None);
        assert_eq!(
            selection.apply(WindowInput::key_down(unbound)),
            PageSelection::Retained(before)
        );
        assert_eq!(selection.active(), before);
    }

    /// The `app_state_generation_delta = 0` predicate, measured across a full
    /// traversal by digit *and* by step.
    #[test]
    fn a_full_page_walk_never_advances_the_production_reducer() {
        let scene = ComponentGalleryScene::new().expect("the gallery reducer witness is buildable");
        let before = scene.app_state.generation();
        let mut selection = GalleryPageSelection::default();
        for page in ALL_GALLERY_PAGES {
            if let Some(digit) = page.digit() {
                assert_eq!(
                    selection.apply(WindowInput::key_down(digit)),
                    PageSelection::Changed(page)
                );
            }
        }
        for _ in 0..GALLERY_PAGE_COUNT {
            selection.apply(WindowInput::key_down(PageStep::Next.key()));
        }
        for _ in 0..GALLERY_PAGE_COUNT {
            selection.apply(WindowInput::key_down(PageStep::Previous.key()));
        }
        selection.apply(WindowInput::key_down(WindowKey::Other));
        assert_eq!(
            scene.app_state.generation(),
            before,
            "paging advanced the reducer"
        );
    }

    /// The measurement above is capable of detecting a change: the same reducer
    /// advances when an event actually reaches it. Without this, a delta of zero
    /// would be indistinguishable from a reducer that never moves.
    #[test]
    fn the_reducer_the_delta_is_measured_against_does_advance_when_an_event_reaches_it() {
        let mut scene =
            ComponentGalleryScene::new().expect("the gallery reducer witness is buildable");
        let before = scene.app_state.generation();
        scene
            .app_state
            .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
            .expect("selecting the other top-level context is accepted");
        assert_eq!(scene.app_state.generation(), before + 1);
    }

    /// The scene binds its own digits precisely because the translator does not.
    #[test]
    fn the_translator_leaves_the_paging_digits_unbound() {
        let mut translator = KeyboardInputTranslator::new();
        for key in [
            WindowKey::Digit3,
            WindowKey::Digit4,
            WindowKey::Digit5,
            WindowKey::Digit6,
            WindowKey::Digit7,
            WindowKey::Digit8,
        ] {
            assert_eq!(
                translator.translate(WindowInput::key_down(key)),
                None,
                "{key:?} became a semantic action"
            );
        }
        // `Digit1` and `Digit2` do map to application contexts, which is exactly
        // why the scene never routes its input through the translator.
        assert!(translator
            .translate(WindowInput::key_down(WindowKey::Digit1))
            .is_some());
        assert!(translator
            .translate(WindowInput::key_down(WindowKey::Digit2))
            .is_some());
    }

    /// NFR-005 in full: every declared state paints a specimen in *both*
    /// authored compositions, not just in whichever one had the most room.
    #[test]
    fn walking_every_page_paints_every_declared_page_and_state_at_both_authored_sizes() {
        let ledger = paint_pages(gallery_window(), &ALL_GALLERY_PAGES);
        assert_eq!(
            ledger.painted_page_count(),
            GALLERY_PAGE_COUNT,
            "pages painted: {:?}",
            ledger.pages_visited
        );
        for policy in ALL_DENSITY_POLICIES {
            let missing: Vec<&str> = ALL_COMPONENT_STATES
                .into_iter()
                .filter(|state| ledger.state_painted_at(policy, *state).is_none())
                .map(ComponentState::canonical_name)
                .collect();
            assert!(
                missing.is_empty(),
                "the {} composition never painted: {missing:?}",
                policy.canonical_name()
            );
        }
        assert_eq!(ledger.painted_state_count(), COMPONENT_STATE_COUNT);
    }

    /// The count is capable of reporting a shortfall: a state recorded by one
    /// composition alone does not count, which is the whole point of indexing
    /// the record by policy.
    #[test]
    fn a_state_painted_in_only_one_composition_is_not_counted_as_covered() {
        let mut ledger = paint_pages(gallery_window(), &ALL_GALLERY_PAGES);
        assert_eq!(ledger.painted_state_count(), COMPONENT_STATE_COUNT);

        ledger.states_painted[policy_index(ViewportDensityPolicy::SteamDeck)]
            [state_index(ComponentState::Disabled)] = None;
        assert_eq!(ledger.painted_state_count(), COMPONENT_STATE_COUNT - 1);
        assert!(ledger
            .states_rendered()
            .iter()
            .all(|record| record.state() != ComponentState::Disabled.canonical_name()));

        let observation = ComponentGalleryObservation::from_paint(&ledger, 0, true);
        assert_eq!(observation.states_painted(), COMPONENT_STATE_COUNT - 1);
    }

    /// Every reported state names both compositions it reached, so the JSON
    /// carries the evidence rather than only the tally.
    #[test]
    fn every_reported_state_names_both_authored_compositions() {
        let ledger = paint_pages(gallery_window(), &ALL_GALLERY_PAGES);
        let rendered = ledger.states_rendered();
        assert_eq!(rendered.len(), COMPONENT_STATE_COUNT);
        let expected: Vec<&str> = ALL_DENSITY_POLICIES
            .into_iter()
            .map(ViewportDensityPolicy::canonical_name)
            .collect();
        for record in &rendered {
            assert_eq!(
                record.viewports(),
                expected.as_slice(),
                "{} did not name both compositions",
                record.state()
            );
        }
    }

    #[test]
    fn every_page_paints_both_authored_viewport_compositions() {
        for page in ALL_GALLERY_PAGES {
            let ledger = paint_pages(gallery_window(), &[page]);
            for policy in ALL_DENSITY_POLICIES {
                assert!(
                    ledger.viewport_painted(policy),
                    "{} did not paint the {} composition",
                    page.canonical_name(),
                    policy.canonical_name()
                );
            }
        }
    }

    #[test]
    fn the_shell_bands_page_paints_all_five_regions_at_both_policies() {
        let ledger = paint_pages(gallery_window(), &[ComponentGalleryPage::ShellBands]);
        assert!(
            ledger.bands_retained_both_viewports(),
            "band coverage: {:?}",
            ledger.bands_painted
        );
        for policy in ALL_DENSITY_POLICIES {
            for id in ShellRegionId::ALL {
                assert!(
                    ledger.bands_painted[policy_index(policy)][id as usize],
                    "{} lost the {} region",
                    policy.canonical_name(),
                    id.name()
                );
            }
        }
        // A page that does not draw the bands must not claim them.
        let colors = paint_pages(gallery_window(), &[ComponentGalleryPage::Colors]);
        assert!(!colors.bands_retained_both_viewports());
    }

    /// No specimen clips its column or overlaps another, in either authored
    /// composition, at the smallest window the gallery can be opened at.
    ///
    /// Both compositions are measured in one pass because both are on screen in
    /// one window: the desktop column and the Steam Deck column are separate
    /// regions with their own clip rects, and a defect in either is reported.
    #[test]
    fn no_specimen_clips_its_column_or_overlaps_another_in_either_composition() {
        let size = gallery_window();
        let ledger = paint_pages(size, &ALL_GALLERY_PAGES);
        assert!(
            ledger.text_defects.is_empty(),
            "at {size:?} the gallery reported: {:?}",
            ledger.text_defects
        );
        for policy in ALL_DENSITY_POLICIES {
            assert!(
                ledger.viewport_painted(policy),
                "the {} composition never painted, so its columns were never checked",
                policy.canonical_name()
            );
        }
    }

    /// The declared minimum is the size the content actually fits at, and one
    /// step below it does not fit — so the constant is bound to the measurement
    /// rather than to a guess.
    #[test]
    fn the_declared_minimum_window_is_the_size_the_gallery_composes_at() {
        let smallest = minimum_gallery_viewport();
        assert_eq!(smallest.width_px, minimum_gallery_width_px());
        assert!(
            smallest.width_px
                >= ViewportDensityPolicy::SteamDeck
                    .authored_viewport()
                    .width_px,
            "the gallery minimum width fell below the authored compact width"
        );
        // Each policy's column seats two of that policy's own authored control
        // widths. This is the property the width is derived from, asserted
        // rather than trusted: a control page splits its column in two, and half
        // a column narrower than an authored control is a control shown
        // colliding with itself at a width the product never renders it at.
        let stage_width = smallest.width_px;
        for (policy, column) in [
            (
                ViewportDensityPolicy::Desktop,
                stage_width * desktop_stage_fraction(),
            ),
            (
                ViewportDensityPolicy::SteamDeck,
                stage_width * (1.0 - desktop_stage_fraction()),
            ),
        ] {
            let content = column - 2.0 * policy.rhythm().inset_px;
            let seat = (content - SpacingStep::S16.resolve()) / 2.0;
            assert!(
                seat >= policy.utility_control().width_px,
                "a {} specimen seat is {seat} px against an authored control width of {}",
                policy.canonical_name(),
                policy.utility_control().width_px
            );
        }
        assert_eq!(smallest.height_px, MINIMUM_GALLERY_HEIGHT_PX);

        let composed = paint_pages(
            Vec2::new(smallest.width_px, smallest.height_px),
            &ALL_GALLERY_PAGES,
        );
        assert!(
            composed.text_defects.is_empty(),
            "the gallery does not compose at its own declared minimum: {:?}",
            composed.text_defects
        );

        let cramped = paint_pages(
            Vec2::new(
                smallest.width_px,
                smallest.height_px - SpacingStep::S8.resolve(),
            ),
            &ALL_GALLERY_PAGES,
        );
        assert!(
            !cramped.text_defects.is_empty(),
            "the gallery still composes one step below its declared minimum, so the minimum is larger than it needs to be"
        );
    }

    /// The window the gallery opens must fit on the display it is reviewed on.
    ///
    /// This is the check that was missing. The paint pass measures text against
    /// the egui surface, so a window taller than the screen reports zero clipped
    /// text while the operator cannot see its lowest band — and because the
    /// minimum size is pinned, they cannot resize it into view either. Bounding
    /// the declared minimum against the authored desktop display, chrome
    /// included, is what makes that unrepresentable.
    #[test]
    fn the_gallery_window_fits_on_the_authored_desktop_display_with_its_chrome() {
        let smallest = minimum_gallery_viewport();
        let display = ViewportDensityPolicy::Desktop.authored_viewport();
        assert!(
            smallest.width_px <= display.width_px,
            "the gallery minimum is {} px wide on a {} px display",
            smallest.width_px,
            display.width_px
        );
        assert!(
            smallest.height_px + GALLERY_WINDOW_CHROME_PX <= display.height_px,
            "the gallery minimum is {} px tall; with {} px of window chrome that does not fit a {} px display",
            smallest.height_px,
            GALLERY_WINDOW_CHROME_PX,
            display.height_px
        );
        // The gallery is reviewed on a desktop display, not on the handheld the
        // compact policy describes — that policy is a composition inside this
        // window, never the screen the window opens on.
        assert!(
            smallest.height_px
                > ViewportDensityPolicy::SteamDeck
                    .authored_viewport()
                    .height_px,
            "the minimum shrank far enough to suggest the compact viewport bounds it; it does not"
        );
    }

    /// Judged in each composition separately: NFR-005 asks for legibility
    /// without color at both authored sizes, and the narrower column is the one
    /// where a mark is most likely to have been dropped for room.
    #[test]
    fn every_painted_state_is_distinguishable_without_color_in_both_compositions() {
        let ledger = paint_pages(gallery_window(), &ALL_GALLERY_PAGES);
        assert!(ledger.states_distinguishable_without_color());
        for policy in ALL_DENSITY_POLICIES {
            for state in ALL_COMPONENT_STATES {
                let painted = ledger.state_painted_at(policy, state).unwrap_or_else(|| {
                    panic!(
                        "{} never painted in the {} composition",
                        state.canonical_name(),
                        policy.canonical_name()
                    )
                });
                assert!(
                    !painted.visible_label.is_empty(),
                    "{} painted no visible label at {}",
                    state.canonical_name(),
                    policy.canonical_name()
                );
                assert!(
                    painted.visible_label.contains(state.canonical_name()),
                    "{} is not named by the label a reader sees ({:?})",
                    state.canonical_name(),
                    painted.visible_label
                );
            }

            // Every state other than `Resting` announces itself with text or shape.
            let resting = ledger
                .state_painted_at(policy, ComponentState::Resting)
                .expect("Resting painted")
                .evidence
                .clone();
            for state in ALL_COMPONENT_STATES {
                if state == ComponentState::Resting {
                    continue;
                }
                let painted = ledger
                    .state_painted_at(policy, state)
                    .expect("state painted");
                assert_ne!(
                    painted.evidence,
                    resting,
                    "{} is distinguishable from Resting by color alone at {}",
                    state.canonical_name(),
                    policy.canonical_name()
                );
            }
        }
    }

    #[test]
    fn every_painted_color_resolves_through_the_vocabulary() {
        let ledger = paint_pages(gallery_window(), &ALL_GALLERY_PAGES);
        assert!(
            ledger.token_source_exact(),
            "a painted color is outside the vocabulary"
        );
        // Every role reached the screen. Counting distinct values instead would
        // undercount: `bg/selected` and `border/default` deliberately share
        // `#2a3745`, so the seventeen roles resolve to sixteen values.
        for role in ALL_COLORS {
            assert!(
                ledger.painted_colors.contains(&role.resolve().to_array()),
                "{} never reached the screen",
                role.canonical_name()
            );
        }
        assert!(
            ledger
                .painted_colors
                .contains(&focus::halo_color(SemanticColor::AccentFocus).to_array()),
            "the authored focus halo never reached the screen"
        );
    }

    #[test]
    fn every_painted_text_run_resolves_to_the_authored_typeface() {
        let ledger = paint_pages(gallery_window(), &ALL_GALLERY_PAGES);
        assert!(ledger.text_runs > 0, "the paint pass emitted no text");
        assert_eq!(
            ledger.unresolved_text_runs, 0,
            "{} text runs did not resolve to a registered authored face",
            ledger.unresolved_text_runs
        );
        assert!(ledger.typeface_resolved());
    }

    /// The observation cannot be satisfied without painting.
    #[test]
    fn an_unpainted_gallery_observes_nothing() {
        let empty = GalleryPaintLedger::default();
        let observation = ComponentGalleryObservation::from_paint(&empty, 0, false);
        assert_eq!(observation.pages_declared(), GALLERY_PAGE_COUNT);
        assert_eq!(observation.states_declared(), COMPONENT_STATE_COUNT);
        assert_eq!(observation.controls_declared(), COMPONENT_CONTROL_COUNT);
        assert_eq!(observation.compositions_declared(), SHELL_COMPOSITION_COUNT);
        assert_eq!(observation.pages_painted(), 0);
        assert_eq!(observation.pages_reachable_by_digit(), 0);
        assert_eq!(observation.pages_reachable_by_step(), 0);
        assert_eq!(observation.states_painted(), 0);
        assert_eq!(observation.controls_painted(), 0);
        assert_eq!(observation.compositions_painted(), 0);
        // Every askable pair is unmapped when nothing painted, which is what
        // makes the zero this predicate wants evidence rather than a constant.
        assert!(observation.kind_role_pairs_unmapped() > 0);
        assert!(!observation.token_source_exact());
        assert!(!observation.typeface_resolved());
        assert!(!observation.desktop_viewport_painted());
        assert!(!observation.steam_deck_viewport_painted());
        assert!(!observation.bands_retained_both_viewports());
        assert!(!observation.unbound_digit_retained_page());
        assert!(!observation.window_closed());
        assert!(observation.states_rendered().is_empty());
        assert!(observation.controls_rendered().is_empty());
        assert!(observation.compositions_rendered().is_empty());
    }

    /// A page requested by its digit counts as reached only once it also paints.
    #[test]
    fn a_digit_request_alone_does_not_make_a_page_reachable() {
        let mut ledger = GalleryPaintLedger::default();
        ledger.record_digit_request(ComponentGalleryPage::ActionHints);
        assert_eq!(ledger.digit_reached_page_count(), 0);
        ledger.record_painted_page(ComponentGalleryPage::ActionHints, 0);
        assert_eq!(ledger.digit_reached_page_count(), 0);
        assert_eq!(ledger.painted_page_count(), 0);
        ledger.record_painted_page(ComponentGalleryPage::ActionHints, 1);
        assert_eq!(ledger.digit_reached_page_count(), 1);
        assert_eq!(ledger.painted_page_count(), 1);
        // A page painted without ever being asked for is painted, not reached.
        ledger.record_painted_page(ComponentGalleryPage::Colors, 1);
        assert_eq!(ledger.painted_page_count(), 2);
        assert_eq!(ledger.digit_reached_page_count(), 1);
    }

    #[test]
    fn the_unbound_digit_predicate_needs_a_press_that_changed_nothing() {
        let mut ledger = GalleryPaintLedger::default();
        assert!(!ComponentGalleryObservation::from_paint(&ledger, 0, true)
            .unbound_digit_retained_page());
        ledger.record_unbound_key(false);
        assert!(
            ComponentGalleryObservation::from_paint(&ledger, 0, true).unbound_digit_retained_page()
        );
        ledger.record_unbound_key(true);
        assert!(!ComponentGalleryObservation::from_paint(&ledger, 0, true)
            .unbound_digit_retained_page());
    }

    /// Drives a complete browsing session and returns the ledger it filled.
    ///
    /// Every page is painted, every page a digit binds is *requested* by that
    /// digit, and every page is *stepped* to, so what the reachability counters
    /// report is what a real traversal produced.
    fn complete_session() -> GalleryPaintLedger {
        let mut ledger = paint_pages(gallery_window(), &ALL_GALLERY_PAGES);
        let mut selection = GalleryPageSelection::default();
        for page in ALL_GALLERY_PAGES {
            if let Some(digit) = page.digit() {
                if let PageSelection::Changed(reached) =
                    selection.apply(WindowInput::key_down(digit))
                {
                    ledger.record_digit_request(reached);
                }
            }
        }
        // Back to the first page by stepping, then forward through all fifteen,
        // so the step counter sees every page.
        for _ in 0..GALLERY_PAGE_COUNT {
            selection.apply(WindowInput::key_down(PageStep::Previous.key()));
        }
        for _ in 0..GALLERY_PAGE_COUNT {
            if let PageSelection::Stepped(reached) =
                selection.apply(WindowInput::key_down(PageStep::Next.key()))
            {
                ledger.record_step_request(reached);
            }
        }
        // The first page is the one the scene opens on rather than one stepped
        // to, so it is requested here the way the window requests it.
        ledger.record_step_request(ALL_GALLERY_PAGES[0]);
        for page in ALL_GALLERY_PAGES {
            ledger.record_painted_page(page, 1);
        }
        ledger.record_unbound_key(false);
        ledger
    }

    /// A complete browsing session satisfies every predicate the witness asserts.
    ///
    /// The numbers are read from the declared families rather than written out,
    /// so growing a family is a failure here until the gallery grows with it —
    /// which is the point. A hard-coded `8` would keep passing on the day a
    /// ninth control landed with no specimen.
    #[test]
    fn a_complete_session_satisfies_the_declared_witness_predicates() {
        let ledger = complete_session();
        let observation = ComponentGalleryObservation::from_paint(&ledger, 0, true);

        assert_eq!(observation.pages_declared(), GALLERY_PAGE_COUNT);
        assert_eq!(observation.pages_painted(), GALLERY_PAGE_COUNT);
        assert_eq!(
            observation.pages_reachable_by_digit(),
            GALLERY_DIGIT_BINDING_COUNT
        );
        assert_eq!(observation.pages_reachable_by_step(), GALLERY_PAGE_COUNT);
        assert!(observation.unbound_digit_retained_page());
        assert_eq!(observation.states_declared(), COMPONENT_STATE_COUNT);
        assert_eq!(observation.states_painted(), COMPONENT_STATE_COUNT);
        assert!(observation.states_distinguishable_without_color());
        assert_eq!(observation.controls_declared(), COMPONENT_CONTROL_COUNT);
        assert_eq!(observation.controls_painted(), COMPONENT_CONTROL_COUNT);
        assert_eq!(observation.kind_role_pairs_unmapped(), 0);
        assert_eq!(observation.controls_unreachable_by_any_pair(), 0);
        assert_eq!(observation.compositions_declared(), SHELL_COMPOSITION_COUNT);
        assert_eq!(observation.compositions_painted(), SHELL_COMPOSITION_COUNT);
        assert!(observation.desktop_viewport_painted());
        assert!(observation.steam_deck_viewport_painted());
        assert!(observation.bands_retained_both_viewports());
        assert_eq!(observation.clipped_or_overlapping_text(), 0);
        assert!(observation.token_source_exact());
        assert!(observation.typeface_resolved());
        assert!(!observation.audio_or_midi_constructed());
        assert_eq!(observation.app_state_generation_delta(), 0);
        assert!(observation.window_closed());
    }

    /// Every declared control and composition has a specimen, named exactly.
    ///
    /// Generic over the declared families rather than over a list this test
    /// carries: an added variant is absent from the rendered set and fails here,
    /// which is the coverage invariant the crest-spec places on the page set.
    /// Exact set equality rather than a count, because a count is satisfied by
    /// painting one control twice.
    #[test]
    fn every_declared_control_and_composition_has_a_painted_specimen() {
        let ledger = complete_session();
        let observation = ComponentGalleryObservation::from_paint(&ledger, 0, true);

        let painted: BTreeSet<&str> = observation
            .controls_rendered()
            .iter()
            .map(PaintedControlRecord::control)
            .collect();
        let declared: BTreeSet<&str> = ALL_COMPONENT_CONTROLS
            .into_iter()
            .map(ComponentControl::canonical_name)
            .collect();
        assert_eq!(painted, declared, "a declared control has no specimen");

        let painted: BTreeSet<&str> = observation
            .compositions_rendered()
            .iter()
            .map(PaintedCompositionRecord::composition)
            .collect();
        let declared: BTreeSet<&str> = ALL_SHELL_COMPOSITIONS
            .into_iter()
            .map(ShellComposition::canonical_name)
            .collect();
        assert_eq!(painted, declared, "a declared composition has no specimen");

        // Each control reports every state it declares, and a label a reader
        // actually sees beside the specimen.
        for record in observation.controls_rendered() {
            let control = ALL_COMPONENT_CONTROLS
                .into_iter()
                .find(|control| control.canonical_name() == record.control())
                .expect("a rendered control is a declared control");
            assert_eq!(record.states_painted(), control.applicable_states().len());
            assert_eq!(record.states_declared(), control.applicable_states().len());
            assert!(
                !record.visible_label().is_empty(),
                "{} painted no visible label",
                record.control()
            );
        }
    }

    /// The coverage counts are measured: removing one painted specimen drops
    /// them.
    ///
    /// This is the assertion that separates a measured observation from a
    /// declared one. If deleting a specimen left the count unchanged, the count
    /// would be reporting the specimen *list* rather than the paint pass, which
    /// is exactly the vacuity the crest-spec forbids.
    #[test]
    fn removing_one_painted_specimen_drops_the_coverage_counts() {
        let mut ledger = complete_session();
        assert_eq!(ledger.painted_control_count(), COMPONENT_CONTROL_COUNT);
        assert_eq!(ledger.painted_composition_count(), SHELL_COMPOSITION_COUNT);
        assert_eq!(ledger.unmapped_kind_role_pairs(), 0);

        // One state of one control, in one composition only.
        ledger.controls_painted[policy_index(ViewportDensityPolicy::SteamDeck)]
            [control_index(ComponentControl::Toggle)][state_index(ComponentState::Disabled)] = None;
        assert_eq!(ledger.painted_control_count(), COMPONENT_CONTROL_COUNT - 1);
        assert!(
            ledger.unmapped_kind_role_pairs() > 0,
            "a control with no specimen left every pair that selects it mapped"
        );
        let observation = ComponentGalleryObservation::from_paint(&ledger, 0, true);
        assert_eq!(observation.controls_painted(), COMPONENT_CONTROL_COUNT - 1);
        assert!(observation
            .controls_rendered()
            .iter()
            .all(|record| record.control() != ComponentControl::Toggle.canonical_name()));

        // One composition, in one composition-of-the-viewport only.
        ledger.compositions_painted[policy_index(ViewportDensityPolicy::Desktop)]
            [composition_index(ShellComposition::Footer)] = None;
        assert_eq!(
            ledger.painted_composition_count(),
            SHELL_COMPOSITION_COUNT - 1
        );
        let observation = ComponentGalleryObservation::from_paint(&ledger, 0, true);
        assert_eq!(
            observation.compositions_painted(),
            SHELL_COMPOSITION_COUNT - 1
        );
        assert!(observation
            .compositions_rendered()
            .iter()
            .all(|record| record.composition() != ShellComposition::Footer.canonical_name()));
    }

    /// A composition that emitted no text is not a painted composition.
    #[test]
    fn a_composition_that_emitted_nothing_is_not_counted_as_painted() {
        let mut ledger = complete_session();
        for policy in ALL_DENSITY_POLICIES {
            ledger.compositions_painted[policy_index(policy)]
                [composition_index(ShellComposition::Section)] = Some(Vec::new());
        }
        assert_eq!(
            ledger.painted_composition_count(),
            SHELL_COMPOSITION_COUNT - 1,
            "an empty run list counted as a painted composition"
        );
    }

    /// The seats the gallery allocates do not overlap and stay inside the area
    /// they divide.
    ///
    /// Positions, not counts: F-14 records that a shape *count* passed while
    /// every hairline sat in the wrong place. The same applies here — two
    /// specimens both painting is no evidence that they are not on top of each
    /// other.
    #[test]
    fn allocated_specimen_seats_are_disjoint_and_inside_the_area_they_divide() {
        let area = Rect::from_min_max(pos2(10.0, 20.0), pos2(730.0, 640.0));
        for count in 1..=4 {
            let seats = split_columns(area, count);
            assert_eq!(seats.len(), count);
            for (index, seat) in seats.iter().enumerate() {
                assert!(
                    area.contains_rect(*seat),
                    "seat {index} of {count} left the area it divides"
                );
                assert!(seat.width() > 0.0);
                for other in seats.iter().skip(index + 1) {
                    assert!(
                        !overlaps(*seat, *other),
                        "two of {count} seats overlap: {seat:?} and {other:?}"
                    );
                }
            }
        }
    }

    /// The serialized observation carries every field the witness reads.
    #[test]
    fn the_serialized_observation_carries_every_declared_field() {
        let ledger = paint_pages(gallery_window(), &[ComponentGalleryPage::Colors]);
        let observation = ComponentGalleryObservation::from_paint(&ledger, 0, true);
        let json = serde_json::to_value(&observation).expect("the observation serializes");
        for field in [
            "pages_declared",
            "pages_painted",
            "pages_reachable_by_digit",
            "pages_reachable_by_step",
            "unbound_digit_retained_page",
            "states_declared",
            "states_painted",
            "states_distinguishable_without_color",
            "controls_declared",
            "controls_painted",
            "kind_role_pairs_unmapped",
            "controls_unreachable_by_any_pair",
            "compositions_declared",
            "compositions_painted",
            "desktop_viewport_painted",
            "steam_deck_viewport_painted",
            "bands_retained_both_viewports",
            "clipped_or_overlapping_text",
            "token_source_exact",
            "typeface_resolved",
            "audio_or_midi_constructed",
            "app_state_generation_delta",
            "window_closed",
            "controls_rendered",
            "compositions_rendered",
        ] {
            assert!(
                json.get(field).is_some(),
                "the observation is missing the declared field {field}"
            );
        }
        assert_eq!(
            COMPONENT_GALLERY_OBSERVATION_MARKER,
            "CREST_COMPONENT_GALLERY_OBSERVATION "
        );
    }

    // =======================================================================
    // T039 — the silence is derived, and the derivation can say otherwise
    // =======================================================================

    /// The gallery constructs no audio output and no MIDI event source.
    #[test]
    fn the_gallery_scene_constructs_no_audio_output_and_no_midi_source() {
        assert!(
            !audio_or_midi_constructed(),
            "the gallery scene names an audio output or a MIDI event source"
        );
        let observation =
            ComponentGalleryObservation::from_paint(&GalleryPaintLedger::default(), 0, true);
        assert!(!observation.audio_or_midi_constructed());
    }

    /// The derivation is capable of reporting `true`.
    ///
    /// Without this the flag would be indistinguishable from a hard-coded
    /// `false`: a scan that had quietly stopped matching anything would report
    /// silence just as convincingly as a scene that is actually silent. Each
    /// needle is exercised on its own, so a search that had lost one of them
    /// still fails here.
    #[test]
    fn the_silence_derivation_reports_true_when_a_construction_is_present() {
        for source in [
            format!("let output = {}{}::new();", "Cpal", "AudioOutput"),
            format!("let midi = {}{}::new();", "CorridorsMidi", "EventSource"),
            format!("fn take(port: &dyn {}{})", "Audio", "OutputPort"),
            format!("fn take(source: &dyn {}{})", "MidiEvent", "Source"),
            format!("let graph: {}{};", "Prepared", "Graph"),
            format!("let renderer: {}{};", "Audio", "Renderer"),
            format!("let message: {}{};", "Midi", "Message"),
        ] {
            assert!(
                source_constructs_audio_or_midi(&source),
                "the derivation did not notice {source:?}"
            );
        }
        assert!(!source_constructs_audio_or_midi(
            "let painter = ui.painter().clone();"
        ));
    }

    /// The derivation does not find its own needles.
    ///
    /// The function that searches for these names necessarily spells fragments
    /// of them. If a fragment were itself a needle, the search would report
    /// every source that contains the search as constructing audio — which is
    /// exactly the defect this pair of tests caught during implementation. Held
    /// explicitly rather than left to the silence assertion, so the failure
    /// names its cause.
    #[test]
    fn the_silence_derivation_does_not_match_its_own_needles() {
        let production = production_source(GALLERY_SCENE_SOURCE);
        assert!(
            production.contains("fn source_constructs_audio_or_midi"),
            "the derivation's own source was excluded from the scan"
        );
        assert!(
            !source_constructs_audio_or_midi(&production),
            "the derivation matched a fragment it spells itself"
        );
    }

    /// The derivation reads this module's shipping source, and reads something.
    ///
    /// A scan that read an empty string would report silence for the same reason
    /// it reports it now, so what it read is checked as well as what it found.
    #[test]
    fn the_silence_derivation_reads_this_modules_shipping_source() {
        let production = production_source(GALLERY_SCENE_SOURCE);
        assert!(
            production.len() > 10_000,
            "the silence derivation read only {} bytes of this module",
            production.len()
        );
        assert!(
            production.contains("fn paint_gallery"),
            "the silence derivation did not read this module's paint path"
        );
        // The test module is excluded, so the needles this file spells below do
        // not answer for the production path.
        assert!(
            !production
                .contains("the_silence_derivation_reports_true_when_a_construction_is_present"),
            "the silence derivation read the tests as if they shipped"
        );
        // And the prose is excluded, so a comment explaining the boundary is not
        // mistaken for code crossing it.
        assert!(
            !production.contains("Realizes `valueObject.Shell.ComponentGalleryPage`"),
            "the silence derivation read this module's prose as if it were code"
        );
    }

    /// Building the scene and traversing every page opens no stream and
    /// dispatches no note.
    ///
    /// Measured through the one thing the scene owns that could observe either:
    /// the production reducer. A note dispatched or a graph published would
    /// advance it, and it does not move.
    #[test]
    fn building_and_traversing_the_gallery_opens_no_stream_and_dispatches_no_note() {
        let scene = ComponentGalleryScene::new().expect("the gallery reducer witness is buildable");
        let before = scene.app_state.generation();
        let ledger = complete_session();
        assert_eq!(
            scene.app_state.generation(),
            before,
            "a full traversal advanced the reducer, so something reached it"
        );
        let observation = ComponentGalleryObservation::from_paint(&ledger, 0, true);
        assert!(!observation.audio_or_midi_constructed());
        assert_eq!(observation.app_state_generation_delta(), 0);
    }

    /// Every text run one real pass put on screen, in paint order.
    ///
    /// Read back off the layer the pass painted into, the same way the control
    /// and composition specimens are, so what this returns is what reached the
    /// screen rather than what the page meant to say.
    fn painted_text(size: Vec2, page: ComponentGalleryPage) -> Vec<String> {
        let context = egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the vendored faces are present")
                .font_definitions(),
        );
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..RawInput::default()
        };
        // The same warm-up pass `paint_pages` uses, for the same reason.
        run_pass(&context, &input, page, &mut GalleryPaintLedger::default());
        let mut runs = Vec::new();
        let output = context.run(input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::ZERO))
                .show(context, |ui| {
                    runs = painted_runs(ui, |ui| {
                        paint_gallery(ui, page, &mut GalleryPaintLedger::default());
                    });
                });
        });
        // Tessellated for the same reason `run_pass` tessellates: what this
        // returns must be backed by geometry that actually reached the
        // tessellator, not by shapes that were merely appended.
        assert!(
            !context
                .tessellate(output.shapes, output.pixels_per_point)
                .is_empty(),
            "{} produced no tessellated geometry",
            page.canonical_name()
        );
        runs.into_iter().map(|run| run.text).collect()
    }

    /// A row specimen seat narrower than the authored control width says so on
    /// the page, in the numbers it actually has.
    ///
    /// Two controls share a page and the state name takes a gutter, so neither
    /// policy's seat reaches its authored control width at the declared minimum
    /// window. That is a real shortfall — a control laid out narrower than any
    /// surface renders it can collide with itself, and an operator who cannot
    /// see why would report the control rather than the seat.
    ///
    /// Asserted against the arithmetic rather than against a fixed string, so a
    /// note that stopped tracking the geometry fails here.
    #[test]
    fn a_short_row_specimen_seat_names_its_own_shortfall_on_the_page() {
        // Joined, because the note wraps: a seat too narrow to hold the control
        // is also too narrow to hold one line about it, so the sentence a reader
        // sees spans several painted runs.
        let joined =
            painted_text(gallery_window(), ComponentGalleryPage::TogglesAndSliders).join(" ");
        let reported: Vec<(f32, f32)> = joined
            .match_indices("this specimen seat is ")
            .map(|(at, marker)| {
                let tail = &joined[at + marker.len()..];
                let seat = read_px(tail).expect("the note names its seat in px");
                let authored_at = tail
                    .find("control width of ")
                    .expect("the note names the authored width");
                let authored = read_px(&tail[authored_at + "control width of ".len()..])
                    .expect("the note names the authored width in px");
                (seat, authored)
            })
            .collect();

        // Two controls on the page, each in both compositions.
        assert_eq!(
            reported.len(),
            2 * ALL_DENSITY_POLICIES.len(),
            "expected one note per row specimen per composition, got {reported:?}"
        );
        for policy in ALL_DENSITY_POLICIES {
            let authored = policy.utility_control().width_px;
            let mine: Vec<(f32, f32)> = reported
                .iter()
                .copied()
                .filter(|(_, against)| *against == authored)
                .collect();
            assert_eq!(
                mine.len(),
                2,
                "the {} column did not name both specimen seats against its authored {authored} px width; reported {reported:?}",
                policy.canonical_name()
            );
            // The number the note reports is the seat, not the authored width:
            // a note that echoed the authored width back would read as a page
            // with nothing wrong on it.
            for (seat, _) in mine {
                assert!(
                    seat > 0.0 && seat < authored,
                    "the {} note reported a {seat} px seat against a {authored} px authored width",
                    policy.canonical_name()
                );
            }
        }

        // And a page with no row specimens makes no such claim.
        let colors = painted_text(gallery_window(), ComponentGalleryPage::Colors);
        assert!(
            !colors.iter().any(|run| run.contains("specimen seat is")),
            "a page with no row specimen still reported a seat shortfall"
        );
    }

    /// Reads the leading `<number> px` off `text`.
    fn read_px(text: &str) -> Option<f32> {
        let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    }

    /// The note is conditional, not unconditional: a seat at or above the
    /// authored width paints nothing.
    ///
    /// Without this the note would be indistinguishable from a banner the page
    /// always carries, which would tell an operator nothing about this seat.
    #[test]
    fn a_row_specimen_seat_at_the_authored_width_reports_no_shortfall() {
        let context = egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the vendored faces are present")
                .font_definitions(),
        );
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 900.0))),
            ..RawInput::default()
        };
        for (seat_px, expected) in [(279.0_f32, true), (280.0, false), (999.0, false)] {
            let mut runs = Vec::new();
            let _ = context.run(input.clone(), |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    runs = painted_runs(ui, |ui| {
                        let painter = ui.painter().clone();
                        let families = ui.ctx().fonts(eframe::egui::text::Fonts::families);
                        let mut ledger = GalleryPaintLedger::default();
                        let area = ui.max_rect();
                        let mut specimen =
                            SpecimenPainter::new(&painter, ui.ctx(), &families, &mut ledger, area);
                        let mut stack = Stack::new(area);
                        paint_row_seat_note(
                            &mut specimen,
                            &mut stack,
                            ViewportDensityPolicy::SteamDeck,
                            seat_px,
                        );
                    });
                });
            });
            let said = runs.iter().any(|run| run.text.contains("specimen seat is"));
            assert_eq!(
                said, expected,
                "a {seat_px} px seat against an authored 280 px width reported {said}"
            );
        }
    }

    /// Two rects that merely touch are not overlapping; one inside another is.
    #[test]
    fn the_overlap_test_separates_touching_from_overlapping() {
        let left = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        let touching = Rect::from_min_max(pos2(10.0, 0.0), pos2(20.0, 10.0));
        let crossing = Rect::from_min_max(pos2(9.0, 0.0), pos2(20.0, 10.0));
        assert!(!overlaps(left, touching));
        assert!(overlaps(left, crossing));
        assert!(overlaps(left, left));
    }
}
