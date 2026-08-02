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
use crate::mixer::global_parameters::GlobalParameters;
use crate::shell::standalone_application::ApplicationConfig;
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
pub const GALLERY_PAGE_COUNT: usize = 8;

/// One named group of component specimens, selected by a locally bound digit.
///
/// The set is closed and has no catch-all, for the same reason
/// [`ComponentState`] is closed: a page added without a binding or a specimen
/// fails compilation or the declared coverage assertion rather than becoming a
/// dead key.
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
}

/// Every declared page, in digit order.
pub const ALL_GALLERY_PAGES: [ComponentGalleryPage; GALLERY_PAGE_COUNT] = [
    ComponentGalleryPage::Colors,
    ComponentGalleryPage::Type,
    ComponentGalleryPage::SpacingAndGeometry,
    ComponentGalleryPage::InteractionStates,
    ComponentGalleryPage::TextAndHairlines,
    ComponentGalleryPage::ValuesAndStatus,
    ComponentGalleryPage::ActionHints,
    ComponentGalleryPage::ShellBands,
];

impl ComponentGalleryPage {
    /// The digit that selects this page.
    ///
    /// The mapping is total in both directions: [`Self::for_digit`] inverts it,
    /// and the test in this module holds that no page lacks a key and no key
    /// lacks a page.
    pub const fn digit(self) -> WindowKey {
        match self {
            Self::Colors => WindowKey::Digit1,
            Self::Type => WindowKey::Digit2,
            Self::SpacingAndGeometry => WindowKey::Digit3,
            Self::InteractionStates => WindowKey::Digit4,
            Self::TextAndHairlines => WindowKey::Digit5,
            Self::ValuesAndStatus => WindowKey::Digit6,
            Self::ActionHints => WindowKey::Digit7,
            Self::ShellBands => WindowKey::Digit8,
        }
    }

    /// The page a digit selects, or `None` when the key binds no page.
    ///
    /// This is the whole of the scene's key binding. Nothing here produces a
    /// `SemanticAction`, and no unbound key reaches a default page.
    pub fn for_digit(key: WindowKey) -> Option<Self> {
        ALL_GALLERY_PAGES
            .into_iter()
            .find(|page| page.digit() == key)
    }

    /// The digit as it reads on screen.
    pub const fn digit_label(self) -> &'static str {
        match self {
            Self::Colors => "1",
            Self::Type => "2",
            Self::SpacingAndGeometry => "3",
            Self::InteractionStates => "4",
            Self::TextAndHairlines => "5",
            Self::ValuesAndStatus => "6",
            Self::ActionHints => "7",
            Self::ShellBands => "8",
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
        }
    }
}

/// What a consumed input did to the active page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageSelection {
    /// A bound digit selected this page.
    Changed(ComponentGalleryPage),
    /// The input bound no page, so the current one was kept.
    Retained(ComponentGalleryPage),
}

impl PageSelection {
    /// The page that is active after the input.
    pub const fn page(self) -> ComponentGalleryPage {
        match self {
            Self::Changed(page) | Self::Retained(page) => page,
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
    /// Only a key-*down* on a bound digit changes the page. A key-up, a focus
    /// loss, and any key with no page bound — including an unbound digit —
    /// retain the current page and change nothing else.
    pub fn apply(&mut self, input: WindowInput) -> PageSelection {
        if input.kind() != WindowInputKind::KeyDown {
            return PageSelection::Retained(self.active);
        }
        match ComponentGalleryPage::for_digit(input.key()) {
            Some(page) => {
                self.active = page;
                PageSelection::Changed(page)
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
    states_painted: [[Option<PaintedState>; COMPONENT_STATE_COUNT]; ALL_DENSITY_POLICIES.len()],
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
            states_painted: std::array::from_fn(|_| std::array::from_fn(|_| None)),
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

    /// Records one key press that bound no page.
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
    unbound_digit_retained_page: bool,
    states_declared: usize,
    states_painted: usize,
    states_distinguishable_without_color: bool,
    desktop_viewport_painted: bool,
    steam_deck_viewport_painted: bool,
    bands_retained_both_viewports: bool,
    clipped_or_overlapping_text: usize,
    token_source_exact: bool,
    typeface_resolved: bool,
    app_state_generation_delta: i64,
    window_closed: bool,
    viewport_width: f32,
    viewport_height: f32,
    active_page: &'static str,
    pages_visited: Vec<&'static str>,
    states_rendered: Vec<PaintedStateRecord>,
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
            unbound_digit_retained_page: ledger.unbound_key_presses > 0
                && ledger.unbound_key_page_changes == 0,
            states_declared: COMPONENT_STATE_COUNT,
            states_painted: ledger.painted_state_count(),
            states_distinguishable_without_color: ledger.states_distinguishable_without_color(),
            desktop_viewport_painted: ledger.viewport_painted(ViewportDensityPolicy::Desktop),
            steam_deck_viewport_painted: ledger.viewport_painted(ViewportDensityPolicy::SteamDeck),
            bands_retained_both_viewports: ledger.bands_retained_both_viewports(),
            clipped_or_overlapping_text: ledger.text_defects.len(),
            token_source_exact: ledger.token_source_exact(),
            typeface_resolved: ledger.typeface_resolved(),
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
            text_runs_painted: ledger.text_runs,
            unbound_key_presses: ledger.unbound_key_presses,
            text_defects: ledger.text_defects.iter().cloned().collect(),
        }
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
        let inputs: Vec<(WindowInput, bool)> = context.input(|input| {
            input
                .events
                .iter()
                .filter_map(normalize_gallery_event)
                .collect()
        });
        for (input, was_digit) in inputs {
            let before = self.selection.active();
            let selection = self.selection.apply(input);
            let mut ledger = self.ledger.borrow_mut();
            match selection {
                PageSelection::Changed(page) => ledger.record_digit_request(page),
                PageSelection::Retained(page) => {
                    if was_digit && input.kind() == WindowInputKind::KeyDown {
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

/// Normalizes one egui event into the window vocabulary, reporting whether the
/// physical key was a digit.
///
/// The digit flag is what lets the scene tell "the operator pressed 9" from "the
/// operator pressed a letter": [`WindowKey`] deliberately normalizes every key
/// beyond the eight bound digits to [`WindowKey::Other`], and the unbound-digit
/// retention the gallery must demonstrate needs that distinction. The adapter's
/// equivalent mapping is private to it and carries no such flag, so the two
/// stay separate rather than one growing a parameter it has no use for.
fn normalize_gallery_event(event: &egui::Event) -> Option<(WindowInput, bool)> {
    match event {
        egui::Event::Key { key, pressed, .. } => {
            let (normalized, is_digit) = normalize_gallery_key(*key);
            Some((
                if *pressed {
                    WindowInput::key_down(normalized)
                } else {
                    WindowInput::key_up(normalized)
                },
                is_digit,
            ))
        }
        egui::Event::WindowFocused(false) => Some((WindowInput::focus_lost(), false)),
        _ => None,
    }
}

/// Maps a physical key to the window vocabulary and reports whether it was a
/// digit.
fn normalize_gallery_key(key: egui::Key) -> (WindowKey, bool) {
    match key {
        egui::Key::Num1 => (WindowKey::Digit1, true),
        egui::Key::Num2 => (WindowKey::Digit2, true),
        egui::Key::Num3 => (WindowKey::Digit3, true),
        egui::Key::Num4 => (WindowKey::Digit4, true),
        egui::Key::Num5 => (WindowKey::Digit5, true),
        egui::Key::Num6 => (WindowKey::Digit6, true),
        egui::Key::Num7 => (WindowKey::Digit7, true),
        egui::Key::Num8 => (WindowKey::Digit8, true),
        egui::Key::Num9 | egui::Key::Num0 => (WindowKey::Other, true),
        _ => (WindowKey::Other, false),
    }
}

/// Paints one complete gallery frame into `ui`.
///
/// This is the whole render path. The tests below drive it in a headless
/// `egui::Context`, so what they measure is what the window paints.
fn paint_gallery(ui: &mut egui::Ui, active: ComponentGalleryPage, ledger: &mut GalleryPaintLedger) {
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
        desktop_column,
        ViewportDensityPolicy::Desktop,
        active,
    );
    paint_composition(
        &mut specimen,
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
        // The digit and the position are the same number, so the identity
        // names it once: `PAGE 3 / 8 · SPACING AND GEOMETRY` reads as both the
        // key that got here and where "here" is.
        &format!(
            "PAGE {} / {} · {}",
            active.digit_label(),
            GALLERY_PAGE_COUNT,
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

/// Paints the page index. Every binding is on screen, the active one accented.
fn paint_index_band(painter: &mut SpecimenPainter<'_>, band: Rect, active: ComponentGalleryPage) {
    painter.begin_region(band);
    painter.fill(band, SemanticColor::BgElevated, Radius::None);
    let hints: Vec<ActionHint<'static>> = ALL_GALLERY_PAGES
        .into_iter()
        .map(|page| {
            ActionHint::new(
                page.digit_label(),
                page.index_label(),
                if page == active {
                    HintTone::Focus
                } else {
                    HintTone::Neutral
                },
            )
        })
        .collect();
    let inset = ViewportDensityPolicy::Desktop.rhythm().inset_px;
    painter.hint_line_at(
        pos2(band.min.x + inset, band.center().y),
        Align2::LEFT_CENTER,
        &hints,
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
            ActionHint::new("1-8", "PAGE", HintTone::Focus),
            ActionHint::new("9", "UNBOUND DIGIT", HintTone::Adjust),
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
    }

    let painted_runs = painter.ledger.text_runs - runs_before;
    if painted_runs > 0 {
        painter.ledger.viewports_painted[policy_index(policy)] = true;
        painter.ledger.record_painted_page(active, painted_runs);
    }
    painter.finish_region();
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
/// past the stage and the pass reports it clipped; at 856 px all eight pages
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

/// The smallest window the gallery composes without clipping a specimen.
///
/// The width is the authored compact viewport width — a declared value, not a
/// guess, and 16 px above the 1264 px the pass actually needs — and the height
/// is [`MINIMUM_GALLERY_HEIGHT_PX`].
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
        width_px: ViewportDensityPolicy::SteamDeck
            .authored_viewport()
            .width_px,
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
    fn the_gallery_declares_exactly_eight_pages() {
        assert_eq!(GALLERY_PAGE_COUNT, 8);
        assert_eq!(ALL_GALLERY_PAGES.len(), GALLERY_PAGE_COUNT);
        let names: BTreeSet<&str> = ALL_GALLERY_PAGES
            .iter()
            .map(|page| page.canonical_name())
            .collect();
        assert_eq!(names.len(), GALLERY_PAGE_COUNT, "two pages share a name");
        let pages: BTreeSet<ComponentGalleryPage> = ALL_GALLERY_PAGES.into_iter().collect();
        assert_eq!(pages.len(), GALLERY_PAGE_COUNT, "a page appears twice");
    }

    /// No page without a key, no key without a page.
    #[test]
    fn every_page_has_exactly_one_digit_and_every_bound_digit_has_one_page() {
        let mut digits = std::collections::HashSet::new();
        for page in ALL_GALLERY_PAGES {
            assert!(
                digits.insert(page.digit()),
                "{} shares its digit with another page",
                page.canonical_name()
            );
            assert_eq!(
                ComponentGalleryPage::for_digit(page.digit()),
                Some(page),
                "{} is not reachable by its own digit",
                page.canonical_name()
            );
        }
        assert_eq!(digits.len(), GALLERY_PAGE_COUNT);
        for key in [
            WindowKey::Q,
            WindowKey::E,
            WindowKey::W,
            WindowKey::S,
            WindowKey::A,
            WindowKey::D,
            WindowKey::K,
            WindowKey::Other,
        ] {
            assert_eq!(
                ComponentGalleryPage::for_digit(key),
                None,
                "{key:?} binds a page it should not"
            );
        }
        // Indices and digit labels agree with declaration order, so the on-screen
        // identity cannot drift from the binding.
        for (index, page) in ALL_GALLERY_PAGES.into_iter().enumerate() {
            assert_eq!(page.index(), index);
            assert_eq!(page.digit_label(), (index + 1).to_string());
        }
    }

    #[test]
    fn a_bound_digit_changes_the_page_and_anything_else_retains_it() {
        let mut selection = GalleryPageSelection::default();
        assert_eq!(selection.active(), ComponentGalleryPage::Colors);

        for page in ALL_GALLERY_PAGES {
            assert_eq!(
                selection.apply(WindowInput::key_down(page.digit())),
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

    /// An unbound digit normalizes to the same value any unbound key does, and
    /// the scene keeps the page it was on.
    #[test]
    fn an_unbound_digit_retains_the_current_page() {
        for key in [egui::Key::Num9, egui::Key::Num0] {
            let (normalized, is_digit) = normalize_gallery_key(key);
            assert_eq!(normalized, WindowKey::Other);
            assert!(is_digit, "{key:?} is not recognized as a digit");
            assert_eq!(ComponentGalleryPage::for_digit(normalized), None);

            let mut selection = GalleryPageSelection::default();
            selection.apply(WindowInput::key_down(
                ComponentGalleryPage::ValuesAndStatus.digit(),
            ));
            let before = selection.active();
            assert_eq!(
                selection.apply(WindowInput::key_down(normalized)),
                PageSelection::Retained(before)
            );
            assert_eq!(selection.active(), before);
        }
        let (letter, is_digit) = normalize_gallery_key(egui::Key::Z);
        assert_eq!(letter, WindowKey::Other);
        assert!(
            !is_digit,
            "a letter must not be counted as an unbound digit"
        );
    }

    /// The `app_state_generation_delta = 0` predicate, measured.
    #[test]
    fn a_full_page_walk_never_advances_the_production_reducer() {
        let scene = ComponentGalleryScene::new().expect("the gallery reducer witness is buildable");
        let before = scene.app_state.generation();
        let mut selection = GalleryPageSelection::default();
        for page in ALL_GALLERY_PAGES {
            assert_eq!(
                selection.apply(WindowInput::key_down(page.digit())),
                PageSelection::Changed(page)
            );
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
        assert_eq!(
            smallest.width_px,
            ViewportDensityPolicy::SteamDeck
                .authored_viewport()
                .width_px,
            "the gallery minimum width drifted from the authored compact width"
        );
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
        assert_eq!(observation.pages_painted(), 0);
        assert_eq!(observation.pages_reachable_by_digit(), 0);
        assert_eq!(observation.states_painted(), 0);
        assert!(!observation.token_source_exact());
        assert!(!observation.typeface_resolved());
        assert!(!observation.desktop_viewport_painted());
        assert!(!observation.steam_deck_viewport_painted());
        assert!(!observation.bands_retained_both_viewports());
        assert!(!observation.unbound_digit_retained_page());
        assert!(!observation.window_closed());
        assert!(observation.states_rendered().is_empty());
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

    /// A complete browsing session satisfies every predicate the witness asserts.
    #[test]
    fn a_complete_session_satisfies_the_declared_witness_predicates() {
        let mut ledger = paint_pages(gallery_window(), &ALL_GALLERY_PAGES);
        for page in ALL_GALLERY_PAGES {
            ledger.record_digit_request(page);
            ledger.record_painted_page(page, 1);
        }
        ledger.record_unbound_key(false);
        let observation = ComponentGalleryObservation::from_paint(&ledger, 0, true);

        assert_eq!(observation.pages_declared(), 8);
        assert_eq!(observation.pages_painted(), 8);
        assert_eq!(observation.pages_reachable_by_digit(), 8);
        assert!(observation.unbound_digit_retained_page());
        assert_eq!(observation.states_declared(), 9);
        assert_eq!(observation.states_painted(), 9);
        assert!(observation.states_distinguishable_without_color());
        assert!(observation.desktop_viewport_painted());
        assert!(observation.steam_deck_viewport_painted());
        assert!(observation.bands_retained_both_viewports());
        assert_eq!(observation.clipped_or_overlapping_text(), 0);
        assert!(observation.token_source_exact());
        assert!(observation.typeface_resolved());
        assert_eq!(observation.app_state_generation_delta(), 0);
        assert!(observation.window_closed());
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
            "unbound_digit_retained_page",
            "states_declared",
            "states_painted",
            "states_distinguishable_without_color",
            "desktop_viewport_painted",
            "steam_deck_viewport_painted",
            "bands_retained_both_viewports",
            "clipped_or_overlapping_text",
            "token_source_exact",
            "typeface_resolved",
            "app_state_generation_delta",
            "window_closed",
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
