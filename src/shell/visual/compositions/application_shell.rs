//! The application shell: the frame the other compositions sit inside.
//!
//! **Components paint. Views compose. The reducer decides.**
//!
//! This composition fills [`ShellRegion::WholeFrame`]. It replaces the layout
//! half of `paint_shell` in `src/adapter/eframe_graphical_window.rs` — which
//! band is which height, which side the persistent region sits on, which
//! surface each band is filled with, and in what order the bands claim space.
//!
//! # The shape of the contract with the render adapter
//!
//! `research.md` R-03 keeps panel construction in the adapter, because the
//! rectangles [`crate::shell::ShellFrameObservation`] is built from are the ones
//! the panels actually produced, and threading those back out of a composition
//! would buy nothing. But *constructing* a panel and *deciding* what the panel
//! is are different jobs, and only the first is plumbing.
//!
//! So the frame is handed over as a plan rather than as painting.
//! [`frame_plan`] returns the five bands in the order they claim space, each
//! carrying everything a panel needs: where it sits ([`BandPlacement`]), how far
//! it extends, which surface fills it, the id it is registered under, which
//! composition fills it, and which observed region it is. The adapter's whole
//! job becomes: for each band in claim order, build the panel the placement
//! names and call [`arrange_band`] inside it. It chooses nothing — not the
//! order, not the extent, not the fill, and not which projection value
//! identifies the region.
//!
//! # Two orders, and both are supplied
//!
//! Panels claim space in one order and the frame observation reports its
//! regions in another: [`ShellFrameObservation::try_new_semantic`] compares the
//! ids it is handed against [`ShellRegionId::surface_descriptor`] and rejects
//! any other sequence. Reconciling the two is not a layout decision, so it is
//! not the adapter's — [`frame_plan`] is in claim order, [`observed_bands`] is
//! in the order the observation demands, and neither has to be re-found.
//!
//! ```text
//! let mut rect_of = BTreeMap::new();
//! for band in application_shell::frame_plan(&policy) {
//!     let panel = <panel from band.placement()>
//!         .frame(<fill from band.surface()>)
//!         .show(context, |ui| frame.absorb(arrange_band(ui, band, …)));
//!     rect_of.insert(band.observed_region_id(), relative(panel.response.rect));
//! }
//! let regions = application_shell::observed_bands(&policy).map(|band| {
//!     ShellRegionObservation::new(
//!         band.observed_region_id(),
//!         rect_of[&band.observed_region_id()],
//!         band.observed_label(projection),
//!     )
//! });
//! ```
//!
//! [`ShellFrameObservation::try_new_semantic`]:
//!     crate::shell::ShellFrameObservation::try_new_semantic
//!
//! # The two paths, and why they cannot drift
//!
//! [`arrange`] tiles the whole frame inside a single [`Ui`], for a caller that
//! has no `egui::Context` to build panels in — the component gallery, and the
//! assertions at the bottom of these four files. It reads the same
//! [`frame_plan`], so a band that moved, resized, or changed surface moves in
//! both paths at once. What the two do not share is panel identity, which is
//! exactly the part the observation needs and the part a `Ui` cannot provide.
//!
//! # What it owns
//!
//! Nothing. Every extent resolves from [`ViewportDensityPolicy`] and every
//! surface from the token vocabulary. There is no band-height constant, no
//! split width, no inset, and no branch on a raw viewport size in this file.

use eframe::egui::{vec2, Align, Layout, Sense, Ui};
use egui_extras::{Size, StripBuilder};

use crate::control::{GraphicalShellProjection, TopLevelContext};
use crate::shell::visual::compositions::{
    footer, CompositionIntent, ShellComposition, ShellRegion,
};
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::{rules, text};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};
use crate::shell::ShellRegionId;

/// Where one band sits in the frame, and how far it extends.
///
/// The extent travels with the placement rather than beside it: a band that
/// spans the remainder has no extent to carry, and a separate `Option` would
/// let a caller build a top band with no height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BandPlacement {
    /// Claims a full-width strip from the top of what is left.
    TopEdge {
        /// How tall the strip is.
        height_px: f32,
    },
    /// Claims a full-width strip from the bottom of what is left.
    BottomEdge {
        /// How tall the strip is.
        height_px: f32,
    },
    /// Claims a full-height strip from the trailing edge of what is left.
    TrailingEdge {
        /// How wide the strip is.
        width_px: f32,
    },
    /// Claims everything the other bands did not.
    Remainder,
}

impl BandPlacement {
    /// This placement as a strip size, for the single-`Ui` tiling path.
    fn strip_size(self) -> Size {
        match self {
            Self::TopEdge { height_px } | Self::BottomEdge { height_px } => Size::exact(height_px),
            Self::TrailingEdge { width_px } => Size::exact(width_px),
            Self::Remainder => Size::remainder(),
        }
    }
}

/// One band of the frame, and everything a panel needs to be built for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBand {
    region: ShellRegion,
    observed: ShellRegionId,
    placement: BandPlacement,
    surface: SemanticColor,
    panel_id: &'static str,
    composition: ShellComposition,
}

impl FrameBand {
    /// The visual region this band is.
    pub const fn region(self) -> ShellRegion {
        self.region
    }

    /// The region this band is reported as in the frame observation.
    ///
    /// Carried as the observation's own type rather than as a name to look up,
    /// so wiring the observation is a move rather than a mapping the adapter
    /// has to get right.
    pub const fn observed_region_id(self) -> ShellRegionId {
        self.observed
    }

    /// Where this band sits and how far it extends.
    pub const fn placement(self) -> BandPlacement {
        self.placement
    }

    /// The surface role this band is filled with.
    pub const fn surface(self) -> SemanticColor {
        self.surface
    }

    /// The stable id the band's panel is registered under.
    pub const fn panel_id(self) -> &'static str {
        self.panel_id
    }

    /// The composition that fills this band.
    pub const fn composition(self) -> ShellComposition {
        self.composition
    }

    /// The text this region reports itself under in the frame observation.
    ///
    /// # Two frozen consumers, one value
    ///
    /// This value is read by two protected callers that constrain it from
    /// opposite directions, and it has to satisfy both:
    ///
    /// - `tests/graphical_application_shell.rs` asserts every region's label is
    ///   equal to some galley the frame **painted**, whole-string.
    /// - `src/testing/live_demo_runner.rs` compares every region's label
    ///   against the projection slice below, field for field, and returns
    ///   `LiveDemoError::ShellFrameMismatch` on any difference.
    ///
    /// So it is exactly the projection's own value — no decoration, no
    /// derivation — **and** the bands are composed so that value is what
    /// reaches the screen. The context line is where that took work: the switch
    /// paints a mark beside the active label, and the mark is a separate run
    /// precisely so this can stay the bare `context_label()`. See
    /// [`super::context_switch::entry_mark`].
    ///
    /// *Cycle 2 reported `"* PATCH"` here to satisfy the first consumer; that
    /// broke the second, which is unowned by any work package. Retracted —
    /// `every_band_reports_the_label_the_live_demo_runner_expects` now holds
    /// both at once.*
    ///
    /// Exhaustive, so a sixth observed region is a compile error here rather
    /// than a region reported under a borrowed label.
    pub fn observed_label(self, projection: &GraphicalShellProjection) -> String {
        match self.observed {
            ShellRegionId::ContextLine => projection.context_line().context_label().to_owned(),
            ShellRegionId::IdentityHeader => {
                projection.identity_header().primary_label().to_owned()
            }
            ShellRegionId::MainWorkspace => projection.workspace().main_label().to_owned(),
            ShellRegionId::PersistentSideRegion => projection.workspace().side_label().to_owned(),
            ShellRegionId::Footer => projection.footer().path_label().to_owned(),
        }
    }
}

/// How many bands tile the frame.
pub const FRAME_BAND_COUNT: usize = 5;

/// The frame, in the order the bands claim space.
///
/// The order is load-bearing and is the adapter's current order preserved
/// exactly: the two top strips, then the bottom strip, then the trailing
/// region, and the main surface takes what is left. A panel claims from what
/// its predecessors did not, so reordering this changes the layout even though
/// every extent stays the same.
///
/// Every number here comes from `density`. There is no constant to change.
pub fn frame_plan(density: &ViewportDensityPolicy) -> [FrameBand; FRAME_BAND_COUNT] {
    let bands = density.bands();
    [
        FrameBand {
            region: ShellRegion::ContextLine,
            observed: ShellRegionId::ContextLine,
            placement: BandPlacement::TopEdge {
                height_px: bands.context_line_px,
            },
            surface: SemanticColor::BgElevated,
            panel_id: "crest-context-line",
            composition: ShellComposition::ContextSwitch,
        },
        FrameBand {
            region: ShellRegion::IdentityHeader,
            observed: ShellRegionId::IdentityHeader,
            placement: BandPlacement::TopEdge {
                height_px: bands.identity_header_px,
            },
            surface: SemanticColor::BgPanel,
            panel_id: "crest-identity-header",
            composition: ShellComposition::IdentityHeader,
        },
        FrameBand {
            region: ShellRegion::Footer,
            observed: ShellRegionId::Footer,
            placement: BandPlacement::BottomEdge {
                height_px: bands.footer_px,
            },
            surface: SemanticColor::BgElevated,
            panel_id: "crest-footer",
            composition: ShellComposition::Footer,
        },
        FrameBand {
            region: ShellRegion::PersistentSideRegion,
            observed: ShellRegionId::PersistentSideRegion,
            placement: BandPlacement::TrailingEdge {
                width_px: density.split().side_px,
            },
            surface: SemanticColor::BgPanel,
            panel_id: "crest-persistent-side-region",
            composition: ShellComposition::UtilityInspectorPanel,
        },
        FrameBand {
            region: ShellRegion::MainWorkspace,
            observed: ShellRegionId::MainWorkspace,
            placement: BandPlacement::Remainder,
            surface: SemanticColor::BgCanvas,
            panel_id: "crest-main-workspace",
            composition: ShellComposition::Section,
        },
    ]
}

/// The same bands, in the order the frame observation reports them.
///
/// `ShellFrameObservation::try_new_semantic` compares the ids it is handed
/// against [`ShellRegionId::surface_descriptor`] and returns `RegionOrder` for
/// any other sequence — and that is not the order panels claim space in, so a
/// caller that iterated [`frame_plan`] straight into the observation would be
/// rejected at runtime. Supplying the second order here is what keeps the
/// reordering, and the per-id re-find it would otherwise need, out of the
/// adapter.
///
/// Every band in [`frame_plan`] appears here exactly once; the assertions below
/// hold that against `surface_descriptor()` itself rather than against a
/// restatement of it.
pub fn observed_bands(density: &ViewportDensityPolicy) -> [FrameBand; FRAME_BAND_COUNT] {
    let plan = frame_plan(density);
    ShellRegionId::ALL.map(|observed| band_reporting(&plan, observed))
}

/// The same bands, with the main workspace filled by the composition the
/// projected context calls for.
///
/// # Why this is not the adapter's branch
///
/// Three compositions bind to [`ShellRegion::MainWorkspace`] and the binding is
/// many-to-one by declaration, so *something* has to say which one fills it in a
/// given frame. It cannot be the render adapter: the `AppWindow` port invariant
/// says the window "decides no paint, layout, band height, or state
/// visualization", and choosing what a region is made of is the first of those.
/// It belongs to the composition that already decides what every other band is
/// made of, which is this one.
///
/// [`frame_plan`] keeps its context-free shape because the band *geometry* —
/// order, placement, extent, surface, panel id — does not depend on the
/// projection, and the assertions below hold it against the density policy
/// alone. Only the main workspace's composition is resolved here.
///
/// On MIXER the main surface is the sixteen fixed track columns, which is
/// [`ShellComposition::MixerStripBank`]. Routing MIXER through
/// [`ShellComposition::Section`] instead would paint all sixteen tracks' cells
/// as one flat vertical list — every track's controls present, no column, no
/// per-track title — which is the regression the bank exists to prevent.
pub fn frame_plan_for(
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> [FrameBand; FRAME_BAND_COUNT] {
    let main = main_workspace_composition(projection);
    frame_plan(density).map(|band| match band.region {
        ShellRegion::MainWorkspace => FrameBand {
            composition: main,
            ..band
        },
        ShellRegion::WholeFrame
        | ShellRegion::ContextLine
        | ShellRegion::IdentityHeader
        | ShellRegion::PersistentSideRegion
        | ShellRegion::Footer => band,
    })
}

/// The composition the main surface is made of in this frame.
///
/// Exhaustive over the top-level context with no wildcard, so a third context
/// is a compile error here rather than a surface silently painted as a PATCH
/// strip.
pub fn main_workspace_composition(projection: &GraphicalShellProjection) -> ShellComposition {
    match projection.semantic_model().context() {
        TopLevelContext::Patch => ShellComposition::Section,
        TopLevelContext::Mixer => ShellComposition::MixerStripBank,
    }
}

/// Resolves the chrome the rendering stack paints for itself to authored roles.
///
/// Most of the shell names its own colors at the call site. A handful are drawn
/// by the stack rather than by a composition — the rule between two panels, the
/// indent rule beside an expanded body, a disclosure triangle — and those come
/// from the stack's own default visuals, which are grays the vocabulary does not
/// declare. Naming them once here is what makes "every color the shell paints
/// resolves through the vocabulary" true of the whole frame rather than of the
/// part a composition happens to paint by hand.
///
/// This is the frame's business, not the render adapter's: it is a mapping from
/// the authored role vocabulary onto the rendering stack's own chrome slots,
/// which is a visual decision in exactly the sense the `AppWindow` port
/// invariant forbids the window to make. The adapter calls it; it does not
/// choose what it does.
///
/// Idempotent, and cheap on every frame but the first: the style is only
/// rewritten when it does not already carry the authored rule.
pub fn install_authored_chrome(context: &eframe::egui::Context) {
    let rule = SemanticColor::BorderDefault.resolve();
    if context
        .style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color
        == rule
    {
        return;
    }
    context.style_mut(|style| {
        let visuals = &mut style.visuals;
        // The rule between two panels, and beside an indented body.
        visuals.widgets.noninteractive.bg_stroke.color = rule;
        // Glyph chrome the stack draws itself, at rest and under interaction.
        visuals.widgets.noninteractive.fg_stroke.color = SemanticColor::TextSecondary.resolve();
        visuals.widgets.inactive.fg_stroke.color = SemanticColor::TextSecondary.resolve();
        visuals.widgets.open.fg_stroke.color = SemanticColor::TextSecondary.resolve();
        visuals.widgets.hovered.fg_stroke.color = SemanticColor::TextPrimary.resolve();
        visuals.widgets.active.fg_stroke.color = SemanticColor::AccentFocus.resolve();
        // The recessed background behind a meter's filled portion.
        visuals.extreme_bg_color = SemanticColor::BgCanvas.resolve();
    });
}

/// Which footer hint an operator addressed, carried with the size of the
/// collection the index was minted over.
///
/// # Why the count travels with the index
///
/// The footer paints `projection.footer().action_hints()`; the caller resolves
/// against `projection.semantic_model().valid_actions()`. Those are two
/// collections. The projector builds the first by mapping over the second in
/// order, so index `i` names the same action in both — *while they agree*. But
/// nothing enforces that they do: `ShellFooter::new` accepts any `Vec<String>`,
/// and none of `GraphicalShellProjection`'s coherence checks looks at the
/// footer at all.
///
/// The shipped `paint_footer` had no such gap, because it drew and emitted from
/// one collection and a mismatch was impossible by construction. Splitting the
/// paint from the resolution is what reintroduced it, so the count travels with
/// the index to close it from this side: [`AddressedHint::resolve`] answers only
/// when the collection it is handed is the same size as the one the index was
/// minted over. An index minted over four painted targets cannot select the
/// third element of a three-element list — the caller gets `None` to handle
/// rather than a neighbouring action dispatched silently.
///
/// The property this *cannot* restore is one collection carrying both, which is
/// the real fix and needs a `ShellFooter` that holds hint and action together.
/// That is a projection change outside this work package's files: recorded
/// here, and raised, rather than reached for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressedHint {
    index: usize,
    minted_over: usize,
}

impl AddressedHint {
    /// Mints an index over a collection of `minted_over` elements.
    ///
    /// `pub(super)` so that only the composition that painted the targets can
    /// mint one. An index arriving from anywhere else would carry a count
    /// nothing measured, which is the guarantee this type exists to make.
    pub(super) const fn minted(index: usize, minted_over: usize) -> Self {
        Self { index, minted_over }
    }

    /// How many elements that slice held.
    pub const fn minted_over(self) -> usize {
        self.minted_over
    }

    /// The element this hint addresses in `against`.
    ///
    /// `None` when `against` is not the size of the collection the index was
    /// minted over, because then it is not the collection this index names.
    pub fn resolve<T>(self, against: &[T]) -> Option<&T> {
        if against.len() != self.minted_over {
            return None;
        }
        against.get(self.index)
    }
}

/// What the frame asks for, aggregated across its bands.
///
/// The second field is the whole reason this is not a bare
/// [`CompositionIntent`]. The footer's valid-action targets are addressable,
/// and what an operator addressed is an *index into the projection slice the
/// footer was handed* — never an action. Resolving that index against
/// `semantic_model().valid_actions()` is a lookup with no choice in it, and it
/// happens outside the visual module, which is what keeps the reducer's
/// vocabulary out of a component. See [`AddressedHint`] for what the index
/// carries so that the lookup cannot answer with the wrong element.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShellFrameIntent {
    intent: CompositionIntent,
    activated_hint: Option<AddressedHint>,
}

impl ShellFrameIntent {
    /// A frame that asked for nothing.
    pub const fn none() -> Self {
        Self {
            intent: CompositionIntent::none(),
            activated_hint: None,
        }
    }

    /// A frame in which one footer hint was addressed, and nothing else was.
    pub const fn from_addressed_hint(activated_hint: Option<AddressedHint>) -> Self {
        Self {
            intent: CompositionIntent::none(),
            activated_hint,
        }
    }

    /// What the arranged controls asked for.
    pub const fn intent(&self) -> &CompositionIntent {
        &self.intent
    }

    /// Which footer hint the operator addressed.
    pub const fn activated_hint(&self) -> Option<AddressedHint> {
        self.activated_hint
    }

    /// Whether the frame asked for anything at all.
    pub fn is_empty(&self) -> bool {
        self.intent.is_empty() && self.activated_hint.is_none()
    }

    /// Folds one band's result into the frame's.
    ///
    /// The first activation wins, matching the adapter's current footer, which
    /// emits at most one action per frame.
    pub fn absorb(&mut self, other: Self) {
        self.intent.absorb(other.intent);
        if self.activated_hint.is_none() {
            self.activated_hint = other.activated_hint;
        }
    }
}

/// Arranges one band's composition.
///
/// The adapter calls this inside the panel it built from `band`. The one
/// special case — the footer, which returns an addressed hint the family
/// signature has no room for — is decided here rather than by the caller.
pub fn arrange_band(
    ui: &mut Ui,
    band: FrameBand,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> ShellFrameIntent {
    match band.composition {
        ShellComposition::Footer => footer::arrange(ui, projection, density),
        ShellComposition::ApplicationShell
        | ShellComposition::ContextSwitch
        | ShellComposition::IdentityHeader
        | ShellComposition::Section
        | ShellComposition::PatchStripRow
        | ShellComposition::MixerStripBank
        | ShellComposition::UtilityInspectorPanel => ShellFrameIntent {
            intent: band.composition.render(ui, projection, density),
            activated_hint: None,
        },
    }
}

/// Tiles the whole frame inside one `Ui`.
///
/// For a caller with no `egui::Context` to build panels in. The bands are laid
/// out in reading order — the two top strips, the workspace split, then the
/// footer — while [`frame_plan`] keeps them in panel-claim order; both orders
/// are derived from the same plan, and the extents come from it unchanged.
pub fn arrange(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> ShellFrameIntent {
    let plan = frame_plan_for(projection, density);
    let context_line = band_for(&plan, ShellRegion::ContextLine);
    let identity = band_for(&plan, ShellRegion::IdentityHeader);
    let main = band_for(&plan, ShellRegion::MainWorkspace);
    let side = band_for(&plan, ShellRegion::PersistentSideRegion);
    let footer_band = band_for(&plan, ShellRegion::Footer);

    let mut frame = ShellFrameIntent::none();
    StripBuilder::new(ui)
        .size(context_line.placement.strip_size())
        .size(identity.placement.strip_size())
        .size(Size::remainder())
        .size(footer_band.placement.strip_size())
        .vertical(|mut strip| {
            strip.cell(|ui| fill_and_arrange(ui, context_line, projection, density, &mut frame));
            strip.cell(|ui| fill_and_arrange(ui, identity, projection, density, &mut frame));
            strip.cell(|ui| {
                StripBuilder::new(ui)
                    .size(main.placement.strip_size())
                    .size(side.placement.strip_size())
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            fill_and_arrange(ui, main, projection, density, &mut frame);
                        });
                        strip.cell(|ui| {
                            fill_and_arrange(ui, side, projection, density, &mut frame);
                        });
                    });
            });
            strip.cell(|ui| fill_and_arrange(ui, footer_band, projection, density, &mut frame));
        });
    frame
}

/// The family conformance for [`ShellComposition::ApplicationShell`].
///
/// Drops the addressed footer hint, which is correct for every caller that
/// reaches the frame through the family: the gallery shows the frame, it does
/// not operate it. The production path calls [`arrange`] or [`arrange_band`].
pub fn render(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    arrange(ui, projection, density).intent
}

/// Fills one band's surface and arranges its composition on top.
fn fill_and_arrange(
    ui: &mut Ui,
    band: FrameBand,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
    frame: &mut ShellFrameIntent,
) {
    ui.painter()
        .rect_filled(ui.max_rect(), 0.0, band.surface.resolve());
    frame.absorb(arrange_band(ui, band, projection, density));
}

/// The band filling one region.
///
/// Every region in [`ShellRegion`] except the whole frame appears exactly once
/// in the plan, which the assertions below hold; the fallback is unreachable
/// and returns the first band rather than panicking inside a paint path. The
/// debug assertion is what stops an unreachable fallback from becoming a
/// silently double-tiled band if it ever does become reachable.
fn band_for(plan: &[FrameBand; FRAME_BAND_COUNT], region: ShellRegion) -> FrameBand {
    match plan.iter().find(|band| band.region == region) {
        Some(band) => *band,
        None => {
            debug_assert!(false, "the frame plan does not fill {region:?}");
            plan[0]
        }
    }
}

/// The band reported under one observed region id. Same contract as
/// [`band_for`], keyed by the observation's own type.
fn band_reporting(plan: &[FrameBand; FRAME_BAND_COUNT], observed: ShellRegionId) -> FrameBand {
    match plan.iter().find(|band| band.observed == observed) {
        Some(band) => *band,
        None => {
            debug_assert!(false, "the frame plan reports no {observed:?}");
            plan[0]
        }
    }
}

// ===========================================================================
// The shared band rhythm
// ===========================================================================
//
// The three chrome bands read the same way — content inset from both edges,
// vertically centred, one leading run, a hairline leader, and a trailing
// group — because the design file draws them that way: `Context Line`,
// `Patch Identity`, and `Controls` each carry a leading text node, a one-pixel
// `Spacer` filling the gap, and trailing content. The frame owns that rhythm
// so the three bands cannot drift from each other.

/// Which half of a band keeps its space when the two cannot both have it.
///
/// A band is a fixed height with nothing to scroll, so when its content does
/// not fit, one half is cut short. Which one is a product decision — the
/// operator's location must survive in the footer, the Patch metadata must
/// survive in the identity band — so it is named at the call site rather than
/// falling out of the order two closures happen to be written in.
///
/// # The rule, and why it is a type rather than a comment
///
/// The half named here is laid out first and is given the whole band to take
/// what it needs from, so **it must ask for its natural width**. A truncating
/// run asks for everything available whenever its content does not fit, so a
/// band with a truncating run on both sides gives the whole band to whichever
/// was laid out first and leaves the other an ellipsis on top of it.
///
/// That rule used to be prose next to a runtime value, which meant a call site
/// could name one half and truncate in it and still compile. It no longer can:
/// [`band_row`] hands the first-allocated half a [`NaturalHalf`] and the other
/// a [`TruncatingHalf`], and the ellipsis helper exists only on the second.
///
/// What the types hold is *"the first-allocated half cannot truncate"*. They do
/// **not** pin which half that is: flipping the variant at a call site still
/// compiles, and what catches that is a forcing fixture, not the compiler —
/// `an_overlong_product_label_neither_clips_nor_overlaps_at_either_viewport`
/// and `an_overlong_label_neither_clips_nor_overlaps_at_either_viewport`. The
/// compiler holds one half of the rule and those two tests hold the other.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BandPrecedence {
    /// The leading run is laid out first and keeps what it needs.
    Leading,
    /// The trailing group is laid out first and keeps what it needs.
    Trailing,
}

/// The band half that is laid out first, out of the whole band.
///
/// It has no way to truncate against *what is left*, which is the point: a run
/// that asks for everything available whenever it does not fit would leave its
/// neighbour nothing. It can still be bounded — see [`Self::paint_reserving`],
/// which is how a half that keeps its space is stopped from taking all of it.
pub(super) struct NaturalHalf<'ui> {
    ui: &'ui mut Ui,
}

impl NaturalHalf<'_> {
    /// The `Ui` this half's content is added to.
    pub(super) fn ui(&mut self) -> &mut Ui {
        self.ui
    }

    /// Paints one run at its natural width, cut short only where continuing
    /// would leave the other half less than `reserved_px`.
    ///
    /// # Why keeping your space is not the same as taking all of it
    ///
    /// [`BandPrecedence`] says which half survives when the two cannot both
    /// fit. It does not say the survivor may erase its neighbour, and for a
    /// band whose halves are both required that distinction is the whole
    /// behaviour. The footer is the case: `DESIGN.md:514` asks it to echo the
    /// operator's path **and** the actions valid right now, so a path long
    /// enough to consume the band takes every action off screen and leaves no
    /// gesture that recovers them — the same annihilation the hint side was
    /// fixed for, entering from the other half.
    ///
    /// So the natural half is given the band minus the floor its neighbour is
    /// guaranteed. Content that fits is untouched and the leader rule still
    /// fills the gap behind it, because a truncating layout asks only for its
    /// natural width until it runs out of room. Content that does not fit stops
    /// at the floor, with the rendering stack's ellipsis.
    ///
    /// `reserved_px` is the caller's, because what the other half needs is a
    /// fact about that half — it must come from the density policy or the token
    /// vocabulary, never from a fraction chosen here.
    pub(super) fn paint_reserving(
        &mut self,
        reserved_px: f32,
        content: &str,
        style: TypeStyle,
        color: SemanticColor,
    ) -> eframe::egui::Response {
        let bound = (self.ui.available_width() - reserved_px).max(0.0);
        self.ui
            .scope(|ui| {
                ui.set_max_width(bound);
                ui.add(
                    eframe::egui::Label::new(text::text_run(
                        content,
                        style,
                        color,
                        ComponentState::Resting,
                    ))
                    .truncate(),
                )
            })
            .inner
    }
}

/// The band half that is laid out into whatever the other one left.
///
/// The only half that can cut a run short, because it is the only one whose
/// neighbour has already taken what it needs.
pub(super) struct TruncatingHalf<'ui> {
    ui: &'ui mut Ui,
}

impl TruncatingHalf<'_> {
    /// The `Ui` this half's content is added to.
    pub(super) fn ui(&mut self) -> &mut Ui {
        self.ui
    }

    /// Paints one chrome run, cut short with the rendering stack's ellipsis
    /// rather than allowed to leave the band it was given.
    ///
    /// A band is a fixed height with no scroll region inside it, so a run that
    /// does not fit has nowhere to go: it is cut, and there is no gesture that
    /// reveals the rest. The design file sizes its identity nodes at the desktop
    /// viewport and says nothing about what a longer name does — see the note on
    /// [`super::identity_header`] — so the one behaviour that cannot produce a
    /// clipped glyph is chosen and recorded, rather than a shrink factor being
    /// invented to avoid it.
    ///
    /// Reachable only through this type. That is what makes
    /// [`BandPrecedence`] a rule the compiler holds.
    pub(super) fn fitted_text(
        &mut self,
        content: &str,
        style: TypeStyle,
        color: SemanticColor,
    ) -> eframe::egui::Response {
        self.ui.add(
            eframe::egui::Label::new(text::text_run(
                content,
                style,
                color,
                ComponentState::Resting,
            ))
            .truncate(),
        )
    }
}

/// Lays one band's content out on the shared band rhythm.
///
/// `keeps_its_space` names the half that is allocated first and therefore gets
/// the [`NaturalHalf`]; the other is given only what is left, gets the
/// [`TruncatingHalf`], and is cut short there rather than painted over its
/// neighbour. Two chrome runs on top of each other are unreadable; one that
/// stops early is merely short.
///
/// The trailing half is added inside a right-to-left layout, so it is written
/// in reverse reading order and lands right-aligned. The leader rule fills
/// whatever is left between the two halves, and disappears when nothing is.
pub(super) fn band_row<Natural, Truncating>(
    ui: &mut Ui,
    density: &ViewportDensityPolicy,
    keeps_its_space: BandPrecedence,
    natural: Natural,
    truncating: Truncating,
) where
    Natural: FnOnce(&mut NaturalHalf<'_>),
    Truncating: FnOnce(&mut TruncatingHalf<'_>),
{
    let inset = density.rhythm().inset_px;
    ui.horizontal_centered(|ui| match keeps_its_space {
        BandPrecedence::Leading => {
            ui.add_space(inset);
            natural(&mut NaturalHalf { ui: &mut *ui });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(inset);
                truncating(&mut TruncatingHalf { ui: &mut *ui });
                leader_rule(ui);
            });
        }
        BandPrecedence::Trailing => {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(inset);
                natural(&mut NaturalHalf { ui: &mut *ui });
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add_space(inset);
                    truncating(&mut TruncatingHalf { ui: &mut *ui });
                    leader_rule(ui);
                });
            });
        }
    });
}

/// The design file's `Spacer`: a hairline filling the gap a band's content
/// leaves between its leading and trailing runs.
pub(super) fn leader_rule(ui: &mut Ui) {
    let width = ui.available_width();
    let height = ui.available_height();
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    rules::hairline(
        ui.painter(),
        rules::RuleSpan::Horizontal {
            y_px: rect.center().y,
            from_x_px: rect.min.x,
            to_x_px: rect.max.x,
        },
    );
}

#[cfg(test)]
pub(crate) mod testing_support {
    //! The shared harness the four frame compositions assert through.
    //!
    //! Every assertion in these files reads glyphs off a real `egui` frame
    //! driven through the real composition, because a composition that is
    //! constructed and never rendered proves nothing about what reaches the
    //! screen.

    use super::*;
    use crate::control::{
        SemanticGraphicalViewModel, ShellContextLine, ShellFooter, ShellIdentityHeader,
        TextProjection, TopLevelContext,
    };
    use crate::shell::visual::typeface::AuthoredTypeface;
    use eframe::egui::{self, pos2, Rect};

    /// One glyph run that reached the screen.
    #[derive(Clone, Debug)]
    pub(crate) struct PaintedRun {
        /// What it says.
        pub content: String,
        /// Where it landed.
        pub rect: Rect,
        /// The container the rendering stack clipped it to.
        pub clip: Rect,
    }

    /// One stroked rectangle that reached the screen.
    ///
    /// A stroked rectangle is how this shell draws an addressable target, so
    /// measuring these is how target height is measured from the pixels rather
    /// than from what the composition says it asked for.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct PaintedRect {
        /// Where it landed.
        pub rect: Rect,
        /// How wide its stroke is.
        pub stroke_width: f32,
    }

    /// Everything one rendered frame put on screen.
    #[derive(Clone, Debug, Default)]
    pub(crate) struct PaintedFrame {
        /// Every glyph run.
        pub runs: Vec<PaintedRun>,
        /// Every stroked rectangle.
        pub rects: Vec<PaintedRect>,
    }

    /// The identity the fixtures carry, shaped like the one the projector
    /// builds: `PATCH nn · name` over `MIDI CH nn · engine`.
    pub(crate) fn fixture_identity(context: TopLevelContext) -> ShellIdentityHeader {
        let label = context.label();
        ShellIdentityHeader::new(
            format!("PATCH 01 · {label} identity"),
            format!("MIDI CH 01 · {label} engine"),
        )
    }

    /// The footer the fixtures carry, shaped like the projector's: one
    /// already-uppercased `"{chord} {label}"` string per valid action.
    pub(crate) fn fixture_footer(context: TopLevelContext) -> ShellFooter {
        ShellFooter::new(
            format!("{} / patch.engine", context.label()),
            vec![
                "1 OPEN MIXER".to_owned(),
                "2 OPEN PATCH".to_owned(),
                "W MOVE UP".to_owned(),
                "S MOVE DOWN".to_owned(),
            ],
        )
    }

    /// The context line the fixtures carry, shaped like the projector's.
    pub(crate) fn fixture_context_line(context: TopLevelContext) -> ShellContextLine {
        ShellContextLine::new("CREST SYNTH", context.label(), "READY")
    }

    /// A projection the production projector built, so its footer hints and its
    /// valid actions are paired **by construction**.
    ///
    /// [`projection_fixture`] cannot be: it composes a hand-built `ShellFooter`
    /// with `SemanticGraphicalViewModel::fixture`, which carries no valid
    /// actions, so every index minted against its hints is out of range for the
    /// collection the adapter resolves against. That is a real desync and it is
    /// recorded, but pinning it is not the same as covering the contract:
    /// against a four-versus-zero fixture, an [`AddressedHint`] that refuses to
    /// resolve is indistinguishable from one that cannot resolve at all. The
    /// half `WP06` actually consumes needs a projection where the two agree.
    ///
    /// So this one goes through `StateProjector::project_with_shell`, which
    /// builds `action_hints` by mapping over `valid_actions` in order — the
    /// pairing the whole index contract rests on, taken from the code that
    /// establishes it rather than restated. The Braids capability is used
    /// because it needs no SoundFont on disk, the same reason and the same
    /// shape as `controls::compact_slider`'s specimen projection.
    pub(crate) fn paired_projection_fixture() -> GraphicalShellProjection {
        use crate::adapter::braids_capability::BraidsCapability;
        use crate::control::{AppState, StateProjector};
        use crate::mixer::global_parameters::GlobalParameters;
        use crate::synth::{CapabilityRegistry, InstrumentCapabilityProvider};

        let braids =
            BraidsCapability::new().expect("the Braids capability builds without an asset");
        let registry = CapabilityRegistry::new(vec![braids.descriptor()])
            .expect("the fixture registry is valid");
        let state = AppState::new(
            registry,
            GlobalParameters::new(0.0).expect("the fixture global parameters are valid"),
        );
        let (_, _, _, shell, _) = StateProjector::new()
            .project_with_shell(&state)
            .expect("the production projection resolves for the paired fixture");
        shell
    }

    /// A coherent projection for `context`.
    pub(crate) fn projection_fixture(context: TopLevelContext) -> GraphicalShellProjection {
        projection_with(
            context,
            fixture_context_line(context),
            fixture_identity(context),
            fixture_footer(context),
        )
    }

    /// The same projection with a caller-chosen footer, for the hint
    /// assertions.
    pub(crate) fn projection_with_footer(
        context: TopLevelContext,
        footer: ShellFooter,
    ) -> GraphicalShellProjection {
        projection_with(
            context,
            fixture_context_line(context),
            fixture_identity(context),
            footer,
        )
    }

    /// The same projection with a caller-chosen identity, for the long-label
    /// assertions.
    pub(crate) fn projection_with_identity(
        context: TopLevelContext,
        identity: ShellIdentityHeader,
    ) -> GraphicalShellProjection {
        projection_with(
            context,
            fixture_context_line(context),
            identity,
            fixture_footer(context),
        )
    }

    /// The same projection with a caller-chosen context line, for the
    /// overlong-product-label assertions.
    pub(crate) fn projection_with_context_line(
        context: TopLevelContext,
        line: ShellContextLine,
    ) -> GraphicalShellProjection {
        projection_with(
            context,
            line,
            fixture_identity(context),
            fixture_footer(context),
        )
    }

    fn projection_with(
        context: TopLevelContext,
        line: ShellContextLine,
        identity: ShellIdentityHeader,
        footer: ShellFooter,
    ) -> GraphicalShellProjection {
        let label = context.label();
        GraphicalShellProjection::new(
            7,
            "state-7",
            SemanticGraphicalViewModel::fixture(7, "state-7", context),
            line,
            identity,
            format!("{label} WORKSPACE"),
            match context {
                TopLevelContext::Patch => "UTILITY",
                TopLevelContext::Mixer => "INSPECTOR",
            },
            TextProjection::for_context(
                context,
                format!("{label} diagnostic"),
                0,
                "state-7".to_owned(),
            ),
            footer,
        )
        .expect("the composition fixture projection is coherent")
    }

    /// Renders the whole frame at `policy`'s authored viewport and returns the
    /// runs that reached the screen.
    pub(crate) fn painted_frame(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
    ) -> Vec<PaintedRun> {
        painted(projection, policy).runs
    }

    /// Drives the real composition through a real `egui` pass and returns
    /// everything it painted.
    pub(crate) fn painted(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
    ) -> PaintedFrame {
        painted_via(projection, policy, |ui, projection, density| {
            arrange(ui, projection, density).intent
        })
    }

    /// The same, through a caller-chosen entry point, so that what the family
    /// dispatch in `compositions/mod.rs` actually reaches can be compared
    /// against what this module calls directly.
    pub(crate) fn painted_via(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
        via: fn(&mut Ui, &GraphicalShellProjection, &ViewportDensityPolicy) -> CompositionIntent,
    ) -> PaintedFrame {
        let viewport = policy.authored_viewport();
        let context = egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the authored typeface loads for the composition assertions")
                .font_definitions(),
        );

        let screen =
            Rect::from_min_size(pos2(0.0, 0.0), vec2(viewport.width_px, viewport.height_px));
        context.begin_pass(egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .inner_margin(egui::Margin::ZERO)
                    .outer_margin(egui::Margin::ZERO),
            )
            .show(&context, |ui| {
                via(ui, projection, policy);
            });
        let output = context.end_pass();

        let mut frame = PaintedFrame::default();
        for clipped in &output.shapes {
            collect(&clipped.shape, clipped.clip_rect, &mut frame);
        }
        frame
    }

    /// Renders the frame twice with a pointer press and release at `at`, and
    /// returns what the second pass reported.
    ///
    /// A press followed by a release on the same widget is what a click is, and
    /// that is a fact about the rendering stack rather than about this
    /// composition. Without this, nothing would ever exercise the path that
    /// carries an addressed hint back out, and the contract WP06 depends on
    /// would be a type with no measurement behind it.
    pub(crate) fn arrange_with_pointer_click(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
        at: egui::Pos2,
    ) -> ShellFrameIntent {
        let viewport = policy.authored_viewport();
        let context = egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the authored typeface loads for the composition assertions")
                .font_definitions(),
        );
        let screen =
            Rect::from_min_size(pos2(0.0, 0.0), vec2(viewport.width_px, viewport.height_px));

        // A warm-up pass first: the rendering stack resolves interaction
        // against the rectangles the previous pass registered, so a press in
        // the very first pass lands on a widget that does not exist yet.
        let mut reported = ShellFrameIntent::none();
        for button in [None, Some(true), Some(false)] {
            let mut events = vec![egui::Event::PointerMoved(at)];
            if let Some(pressed) = button {
                events.push(egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                });
            }
            context.begin_pass(egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .inner_margin(egui::Margin::ZERO)
                        .outer_margin(egui::Margin::ZERO),
                )
                .show(&context, |ui| {
                    reported = arrange(ui, projection, policy);
                });
            let _ = context.end_pass();
        }
        reported
    }

    /// Scrolls horizontally at `at` in `steps` increments of `step_px`, and
    /// returns every distinct run painted anywhere along the way.
    ///
    /// One `egui::Context` for the whole sweep, so the scroll offset
    /// accumulates the way it does under a real operator's hand rather than
    /// being re-applied from zero each time — and so the authored typeface is
    /// loaded once rather than once per step.
    ///
    /// The union across a sweep is what "reachable" means for a scroll region:
    /// a single gesture only shows that the region moves.
    pub(crate) fn painted_across_horizontal_sweep(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
        at: egui::Pos2,
        step_px: f32,
        steps: usize,
    ) -> Vec<String> {
        let viewport = policy.authored_viewport();
        let context = egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the authored typeface loads for the composition assertions")
                .font_definitions(),
        );
        let screen =
            Rect::from_min_size(pos2(0.0, 0.0), vec2(viewport.width_px, viewport.height_px));

        let mut seen: Vec<String> = Vec::new();
        // One pass with no wheel event first: the rendering stack resolves a
        // scroll against the regions the previous pass registered.
        for step in 0..=steps {
            let mut events = vec![egui::Event::PointerMoved(at)];
            if step > 0 {
                events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(-step_px, 0.0),
                    modifiers: egui::Modifiers::default(),
                });
            }
            context.begin_pass(egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .inner_margin(egui::Margin::ZERO)
                        .outer_margin(egui::Margin::ZERO),
                )
                .show(&context, |ui| {
                    arrange(ui, projection, policy);
                });
            let output = context.end_pass();

            let mut frame = PaintedFrame::default();
            for clipped in &output.shapes {
                collect(&clipped.shape, clipped.clip_rect, &mut frame);
            }
            for run in frame.runs {
                if !run.content.trim().is_empty() && !seen.contains(&run.content) {
                    seen.push(run.content);
                }
            }
        }
        seen
    }

    fn collect(shape: &egui::Shape, clip: Rect, frame: &mut PaintedFrame) {
        match shape {
            egui::Shape::Text(text) => frame.runs.push(PaintedRun {
                content: text.galley.job.text.clone(),
                rect: text.galley.rect.translate(text.pos.to_vec2()),
                clip,
            }),
            egui::Shape::Rect(rect) => frame.rects.push(PaintedRect {
                rect: rect.rect,
                stroke_width: rect.stroke.width,
            }),
            egui::Shape::Vec(children) => {
                for child in children {
                    collect(child, clip, frame);
                }
            }
            egui::Shape::Noop
            | egui::Shape::Circle(_)
            | egui::Shape::Ellipse(_)
            | egui::Shape::LineSegment { .. }
            | egui::Shape::Path(_)
            | egui::Shape::QuadraticBezier(_)
            | egui::Shape::CubicBezier(_)
            | egui::Shape::Mesh(_)
            | egui::Shape::Callback(_) => {}
        }
    }

    /// The rectangle one band occupies, derived from the policy rather than
    /// read back from the composition.
    ///
    /// This is the independent expectation: the bands stack top to bottom in
    /// reading order and the workspace splits main then side, which is what
    /// `DESIGN.md:440` draws. If the composition tiled differently, the runs
    /// would land outside the band this returns.
    pub(crate) fn band_rect(policy: &ViewportDensityPolicy, region: ShellRegionId) -> Rect {
        let viewport = policy.authored_viewport();
        let bands = policy.bands();
        let split = policy.split();
        let identity_top = bands.context_line_px;
        let workspace_top = identity_top + bands.identity_header_px;
        let footer_top = viewport.height_px - bands.footer_px;
        let side_left = viewport.width_px - split.side_px;
        match region {
            ShellRegionId::ContextLine => {
                Rect::from_min_max(pos2(0.0, 0.0), pos2(viewport.width_px, identity_top))
            }
            ShellRegionId::IdentityHeader => Rect::from_min_max(
                pos2(0.0, identity_top),
                pos2(viewport.width_px, workspace_top),
            ),
            ShellRegionId::MainWorkspace => {
                Rect::from_min_max(pos2(0.0, workspace_top), pos2(side_left, footer_top))
            }
            ShellRegionId::PersistentSideRegion => Rect::from_min_max(
                pos2(side_left, workspace_top),
                pos2(viewport.width_px, footer_top),
            ),
            ShellRegionId::Footer => Rect::from_min_max(
                pos2(0.0, footer_top),
                pos2(viewport.width_px, viewport.height_px),
            ),
        }
    }

    /// The runs one band painted, attributed by where they landed.
    pub(crate) fn band_runs(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
        region: ShellRegionId,
    ) -> Vec<PaintedRun> {
        let band = band_rect(policy, region);
        painted_frame(projection, policy)
            .into_iter()
            .filter(|run| !run.content.trim().is_empty() && band.contains(run.rect.center()))
            .collect()
    }

    /// The runs one band painted, attributed by the row they landed in rather
    /// than by containment.
    ///
    /// [`band_runs`] keeps a run when its centre is inside the band, which is
    /// the right attribution while the band's content fits and exactly the
    /// wrong one for measuring what happens when it does not: a run pushed past
    /// the band's leading edge has its centre outside the band and would
    /// silently leave the measurement rather than fail it. The three chrome
    /// bands span the full viewport width, so a run belongs to whichever band's
    /// vertical extent contains it — which is true of an overflowing run too.
    pub(crate) fn band_row_runs(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
        region: ShellRegionId,
    ) -> Vec<PaintedRun> {
        let band = band_rect(policy, region);
        painted_frame(projection, policy)
            .into_iter()
            .filter(|run| {
                !run.content.trim().is_empty()
                    && run.rect.center().y >= band.min.y
                    && run.rect.center().y <= band.max.y
            })
            .collect()
    }

    /// What `tests/component_vocabulary.rs` counts as clipped or overlapping,
    /// measured here at the source.
    ///
    /// The rule is that target's, not a second one: a run that leaves the
    /// container the rendering stack clipped it to is a **defect** only when
    /// that container is a band with nothing to scroll inside it, and is
    /// otherwise counted and reported; overlap is compared only between runs
    /// that are both fully visible, and only within one container, because two
    /// runs the stack clipped to different regions cannot collide. `T044`
    /// asserts the same quantity over the production shell, so measuring it
    /// with a different rule here would prove something else.
    pub(crate) struct OverflowReport {
        /// Runs that clipped, and pairs that collided.
        pub defects: Vec<String>,
        /// Runs that left their container without that being a defect — the
        /// quantity the acceptance target reports as scrolled out of view.
        pub left_their_container: usize,
    }

    /// Applies the rule above to one set of runs.
    ///
    /// `fixed_containers` are the clip rectangles with no scroll region inside
    /// them, for which leaving the container is a defect rather than a count.
    pub(crate) fn overflow_report(
        runs: &[PaintedRun],
        fixed_containers: &[Rect],
    ) -> OverflowReport {
        let mut report = OverflowReport {
            defects: Vec::new(),
            left_their_container: 0,
        };
        let mut visible: Vec<&PaintedRun> = Vec::new();
        for run in runs {
            if !run.clip.contains_rect(run.rect) {
                report.left_their_container += 1;
                if fixed_containers.contains(&run.clip) {
                    report.defects.push(format!(
                        "clipped — {:?} at {:?} escapes the fixed container {:?}",
                        run.content, run.rect, run.clip
                    ));
                }
                continue;
            }
            visible.push(run);
        }
        for (index, first) in visible.iter().enumerate() {
            for second in visible.iter().skip(index + 1) {
                if first.clip != second.clip {
                    continue;
                }
                let overlap = first.rect.intersect(second.rect);
                if overlap.width() > 0.0 && overlap.height() > 0.0 {
                    report.defects.push(format!(
                        "overlapping — {:?} at {:?} and {:?} at {:?} inside {:?}",
                        first.content, first.rect, second.content, second.rect, first.clip
                    ));
                }
            }
        }
        report
    }

    /// The stroked rectangles one band painted, attributed by where they
    /// landed. A rule is not a target, so zero-stroke fills are excluded.
    pub(crate) fn band_rects(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
        region: ShellRegionId,
    ) -> Vec<PaintedRect> {
        let band = band_rect(policy, region);
        painted(projection, policy)
            .rects
            .into_iter()
            .filter(|rect| rect.stroke_width > 0.0 && band.contains(rect.rect.center()))
            .collect()
    }

    /// The text of a set of runs, in paint order.
    pub(crate) fn painted_text(runs: &[PaintedRun]) -> Vec<String> {
        runs.iter().map(|run| run.content.clone()).collect()
    }

    /// How wide one run would be with nothing bounding it.
    ///
    /// The rendering stack keeps the original string on a truncated layout
    /// job, so "was this cut short?" is a question about geometry. This is the
    /// width to compare a painted run against.
    pub(crate) fn unbounded_text_width(content: &str, style: TypeStyle) -> f32 {
        let context = egui::Context::default();
        context.set_fonts(
            AuthoredTypeface::load()
                .expect("the authored typeface loads for the composition assertions")
                .font_definitions(),
        );
        context.begin_pass(egui::RawInput::default());
        let job = text::text_run(
            content,
            style,
            SemanticColor::TextPrimary,
            ComponentState::Resting,
        );
        let width = context.fonts(|fonts| fonts.layout_job(job)).rect.width();
        let _ = context.end_pass();
        width
    }

    /// Every run in `runs` sits inside `band`, and no two of them overlap.
    ///
    /// Returned as a list of defects rather than asserted here so each
    /// composition names itself in the failure.
    pub(crate) fn containment_defects(runs: &[PaintedRun], band: Rect) -> Vec<String> {
        let mut defects = Vec::new();
        for run in runs {
            if !band.contains_rect(run.rect) {
                defects.push(format!(
                    "{:?} at {:?} escapes the band {band:?} (clipped to {:?})",
                    run.content, run.rect, run.clip
                ));
            }
        }
        for (index, first) in runs.iter().enumerate() {
            for second in runs.iter().skip(index + 1) {
                let overlap = first.rect.intersect(second.rect);
                if overlap.width() > 0.0 && overlap.height() > 0.0 {
                    defects.push(format!(
                        "{:?} at {:?} overlaps {:?} at {:?}",
                        first.content, first.rect, second.content, second.rect
                    ));
                }
            }
        }
        defects
    }
}

#[cfg(test)]
mod tests {
    use super::testing_support::{
        band_rect, band_runs, containment_defects, painted_frame, painted_text, painted_via,
        paired_projection_fixture, projection_fixture,
    };
    use super::*;
    use crate::control::TopLevelContext;
    use crate::shell::visual::compositions::{
        context_switch, ALL_SHELL_COMPOSITIONS, OBSERVED_REGION_NAMES,
    };
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::token::ALL_COLORS;
    use std::collections::HashSet;

    #[test]
    fn the_application_shell_is_bound_to_the_whole_frame() {
        assert_eq!(
            ShellComposition::ApplicationShell.region(),
            ShellRegion::WholeFrame
        );
    }

    /// The plan covers every observed region exactly once, at both viewports.
    /// A frame that lost a band would lose it here first.
    #[test]
    fn the_plan_fills_every_observed_region_exactly_once() {
        for policy in ALL_DENSITY_POLICIES {
            let plan = frame_plan(&policy);
            assert_eq!(plan.len(), FRAME_BAND_COUNT);
            assert_eq!(FRAME_BAND_COUNT, OBSERVED_REGION_NAMES.len());

            let observed: HashSet<ShellRegionId> =
                plan.iter().map(|band| band.observed_region_id()).collect();
            assert_eq!(observed.len(), FRAME_BAND_COUNT, "a region appears twice");
            for id in ShellRegionId::ALL {
                assert!(observed.contains(&id), "no band fills {id:?}");
            }

            let panels: HashSet<&str> = plan.iter().map(|band| band.panel_id()).collect();
            assert_eq!(panels.len(), FRAME_BAND_COUNT, "two bands share a panel id");
        }
    }

    /// The visual region and the observed region agree, band by band. WP01
    /// bound the two by name; this holds that the plan uses that binding rather
    /// than a parallel one of its own.
    #[test]
    fn every_band_reports_the_region_it_fills() {
        for policy in ALL_DENSITY_POLICIES {
            for band in frame_plan(&policy) {
                assert_eq!(
                    band.region().observation_name(),
                    Some(observed_name(band.observed_region_id())),
                    "{:?} and {:?} disagree",
                    band.region(),
                    band.observed_region_id()
                );
                assert_eq!(
                    band.composition().region(),
                    band.region(),
                    "{} does not fill {:?}",
                    band.composition().canonical_name(),
                    band.region()
                );
            }
        }
    }

    fn observed_name(id: ShellRegionId) -> &'static str {
        match id {
            ShellRegionId::ContextLine => "contextLine",
            ShellRegionId::IdentityHeader => "identityHeader",
            ShellRegionId::MainWorkspace => "mainWorkspace",
            ShellRegionId::PersistentSideRegion => "persistentSideRegion",
            ShellRegionId::Footer => "footer",
        }
    }

    /// Every extent in the plan is the one the density policy declares, and no
    /// two policies produce the same frame. If a band height were a constant
    /// here, the two policies would agree and this would fail.
    #[test]
    fn every_band_extent_resolves_from_the_density_policy() {
        for policy in ALL_DENSITY_POLICIES {
            let bands = policy.bands();
            let plan = frame_plan(&policy);
            let extent = |region: ShellRegion| band_for(&plan, region).placement();
            assert_eq!(
                extent(ShellRegion::ContextLine),
                BandPlacement::TopEdge {
                    height_px: bands.context_line_px
                }
            );
            assert_eq!(
                extent(ShellRegion::IdentityHeader),
                BandPlacement::TopEdge {
                    height_px: bands.identity_header_px
                }
            );
            assert_eq!(
                extent(ShellRegion::Footer),
                BandPlacement::BottomEdge {
                    height_px: bands.footer_px
                }
            );
            assert_eq!(
                extent(ShellRegion::PersistentSideRegion),
                BandPlacement::TrailingEdge {
                    width_px: policy.split().side_px
                }
            );
            assert_eq!(extent(ShellRegion::MainWorkspace), BandPlacement::Remainder);
        }
        assert_ne!(
            frame_plan(&ViewportDensityPolicy::Desktop),
            frame_plan(&ViewportDensityPolicy::SteamDeck),
            "both viewports resolved to the same frame, so something is a constant"
        );
    }

    /// Every surface a band is filled with is a declared role. A band filled
    /// with a color built here would not be in the table.
    #[test]
    fn every_band_surface_is_a_declared_color_role() {
        for policy in ALL_DENSITY_POLICIES {
            for band in frame_plan(&policy) {
                assert!(
                    ALL_COLORS.contains(&band.surface()),
                    "{:?} is filled outside the vocabulary",
                    band.region()
                );
            }
        }
    }

    /// The observation label a band reports is that region's own projection
    /// value, and the five are distinct — a band reporting a borrowed label
    /// would make the observation name a region it did not paint.
    #[test]
    fn every_band_names_itself_from_its_own_projection_slice() {
        for context in TopLevelContext::ALL {
            let projection = projection_fixture(context);
            let plan = frame_plan(&ViewportDensityPolicy::Desktop);
            let labels: Vec<String> = plan
                .iter()
                .map(|band| band.observed_label(&projection))
                .collect();
            assert_eq!(
                labels,
                vec![
                    projection.context_line().context_label().to_owned(),
                    projection.identity_header().primary_label().to_owned(),
                    projection.footer().path_label().to_owned(),
                    projection.workspace().side_label().to_owned(),
                    projection.workspace().main_label().to_owned(),
                ]
            );
            assert_eq!(
                labels.iter().collect::<HashSet<_>>().len(),
                FRAME_BAND_COUNT,
                "two regions report the same label"
            );
        }
    }

    /// Every band reports itself under a string it actually put on screen.
    ///
    /// This is the assertion that closes the class the context line fell into:
    /// the projection carries `"PATCH"`, the switch paints `"* PATCH"`, and the
    /// shell's own frame-observation correlation compares a region's visible
    /// label against whole galley text for equality. A label nothing painted is
    /// a region naming a rectangle it did not produce, which the region binding
    /// exists to prevent — so it is measured off the shape stream rather than
    /// argued from the source.
    ///
    /// A band that paints nothing yet is exempt and cannot make this vacuous:
    /// the three chrome bands this work package fills are required to paint,
    /// and the two workspace bands are `WP05`'s and empty until it lands. When
    /// they do paint, they are held to the same rule with no change here.
    ///
    /// Non-vacuity is pinned by **naming** the bands that must paint rather
    /// than by counting them. A total would have been a lane block: `WP05`
    /// filling the workspace bands correctly would take the count from three to
    /// five and fail this test, in a file `WP05` cannot edit.
    #[test]
    fn every_band_reports_a_label_it_actually_paints() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let mut measured = Vec::new();
                for band in frame_plan(&policy) {
                    let region = band.observed_region_id();
                    let painted = painted_text(&band_runs(&projection, &policy, region));
                    if painted.is_empty() {
                        continue;
                    }
                    measured.push(region);
                    let reported = band.observed_label(&projection);
                    assert!(
                        painted.iter().any(|run| run == &reported),
                        "{region:?} at {} in {} reports {reported:?}, which it never painted; \
                         it painted {painted:?}",
                        policy.canonical_name(),
                        context.label()
                    );
                }
                for region in [
                    ShellRegionId::ContextLine,
                    ShellRegionId::IdentityHeader,
                    ShellRegionId::Footer,
                ] {
                    assert!(
                        measured.contains(&region),
                        "{} at {} painted nothing into {region:?}, so the label rule was never \
                         exercised there",
                        policy.canonical_name(),
                        context.label()
                    );
                }
            }
        }
    }

    /// The label every band reports is the exact projection value the live
    /// demo runner expects to find there.
    ///
    /// `src/testing/live_demo_runner.rs` builds its own expected frame from
    /// these five slices and returns `LiveDemoError::ShellFrameMismatch` on any
    /// difference, and no work package in this mission owns that file. Together
    /// with `every_band_reports_a_label_it_actually_paints` this pins the label
    /// from both sides at once: it must be the projection's own value, *and* it
    /// must be a value the band paints. Cycle 2 satisfied only the second and
    /// silently broke this one; the two tests exist so that cannot recur.
    ///
    /// The expectations below are transcribed from that file deliberately — a
    /// helper shared with it would move when it moved, which is the failure
    /// this is meant to catch.
    #[test]
    fn every_band_reports_the_label_the_live_demo_runner_expects() {
        for context in TopLevelContext::ALL {
            for projection in [projection_fixture(context), paired_projection_fixture()] {
                let shell = &projection;
                let expected = [
                    (
                        ShellRegionId::ContextLine,
                        shell.context_line().context_label(),
                    ),
                    (
                        ShellRegionId::IdentityHeader,
                        shell.identity_header().primary_label(),
                    ),
                    (ShellRegionId::MainWorkspace, shell.workspace().main_label()),
                    (
                        ShellRegionId::PersistentSideRegion,
                        shell.workspace().side_label(),
                    ),
                    (ShellRegionId::Footer, shell.footer().path_label()),
                ];
                for policy in ALL_DENSITY_POLICIES {
                    let reported: Vec<(ShellRegionId, String)> = observed_bands(&policy)
                        .iter()
                        .map(|band| (band.observed_region_id(), band.observed_label(&projection)))
                        .collect();
                    for (index, (id, label)) in expected.into_iter().enumerate() {
                        assert_eq!(
                            reported[index].0, id,
                            "region order left surface_descriptor"
                        );
                        assert_eq!(
                            reported[index].1,
                            label,
                            "{id:?} would fail live_demo_runner's ShellFrameMismatch check at {}",
                            policy.canonical_name()
                        );
                    }
                }
            }
        }
    }

    /// The bands come back in the order the frame observation demands, so the
    /// adapter neither reorders the plan nor re-finds a band per region id.
    ///
    /// `ShellFrameObservation::try_new_semantic` rejects any other sequence, so
    /// this is held against `surface_descriptor()` itself rather than against a
    /// restatement of it — and against `frame_plan`, which is deliberately a
    /// different order.
    #[test]
    fn the_observed_order_is_the_order_the_observation_demands() {
        for policy in ALL_DENSITY_POLICIES {
            let observed: Vec<ShellRegionId> = observed_bands(&policy)
                .iter()
                .map(|band| band.observed_region_id())
                .collect();
            assert_eq!(observed, ShellRegionId::surface_descriptor().to_vec());

            let claimed: Vec<ShellRegionId> = frame_plan(&policy)
                .iter()
                .map(|band| band.observed_region_id())
                .collect();
            assert_ne!(
                claimed, observed,
                "claim order and observation order coincide, so nothing here is being tested"
            );
            // Same five bands, reordered — not a second plan.
            let plan = frame_plan(&policy);
            for band in observed_bands(&policy) {
                assert!(
                    plan.contains(&band),
                    "{:?} is not the band the plan holds for it",
                    band.observed_region_id()
                );
            }
        }
    }

    /// The plan's sequence, pinned band by band.
    ///
    /// The order is load-bearing — a panel claims from what its predecessors
    /// left, so reordering changes the layout even though every extent stays
    /// the same — and until this assertion existed every test read the plan as
    /// a set or looked bands up by region, which cannot see a reordering at
    /// all.
    ///
    /// Three descriptions of this order exist: the array below, the strip order
    /// [`arrange`] tiles in, and the independent geometry `band_rect` derives
    /// from the policy. This pins the first;
    /// `the_independent_band_geometry_agrees_with_the_plan` ties the third to
    /// it; and the second is tied to the third by measurement, because every
    /// `band_runs` assertion in these four files attributes painted runs to a
    /// `band_rect` and would find the wrong band's text if `arrange` tiled in a
    /// different order.
    #[test]
    fn the_plan_claims_space_in_the_authored_order() {
        for policy in ALL_DENSITY_POLICIES {
            let bands = policy.bands();
            let plan = frame_plan(&policy);
            let expected = [
                (
                    ShellRegion::ContextLine,
                    BandPlacement::TopEdge {
                        height_px: bands.context_line_px,
                    },
                ),
                (
                    ShellRegion::IdentityHeader,
                    BandPlacement::TopEdge {
                        height_px: bands.identity_header_px,
                    },
                ),
                (
                    ShellRegion::Footer,
                    BandPlacement::BottomEdge {
                        height_px: bands.footer_px,
                    },
                ),
                (
                    ShellRegion::PersistentSideRegion,
                    BandPlacement::TrailingEdge {
                        width_px: policy.split().side_px,
                    },
                ),
                (ShellRegion::MainWorkspace, BandPlacement::Remainder),
            ];
            for (index, (region, placement)) in expected.into_iter().enumerate() {
                assert_eq!(
                    plan[index].region(),
                    region,
                    "{} claims space in a different order at position {index}",
                    policy.canonical_name()
                );
                assert_eq!(
                    plan[index].placement(),
                    placement,
                    "{} band {index} is placed differently",
                    policy.canonical_name()
                );
            }
        }
    }

    /// The independent band geometry the assertions attribute runs to is the
    /// plan's own extents, so the two cannot describe different frames.
    #[test]
    fn the_independent_band_geometry_agrees_with_the_plan() {
        for policy in ALL_DENSITY_POLICIES {
            for band in frame_plan(&policy) {
                let rect = band_rect(&policy, band.observed_region_id());
                match band.placement() {
                    BandPlacement::TopEdge { height_px }
                    | BandPlacement::BottomEdge { height_px } => assert_eq!(
                        rect.height(),
                        height_px,
                        "{:?} is {} px of geometry for a {height_px} px band at {}",
                        band.region(),
                        rect.height(),
                        policy.canonical_name()
                    ),
                    BandPlacement::TrailingEdge { width_px } => assert_eq!(
                        rect.width(),
                        width_px,
                        "{:?} is {} px of geometry for a {width_px} px band at {}",
                        band.region(),
                        rect.width(),
                        policy.canonical_name()
                    ),
                    BandPlacement::Remainder => assert_eq!(
                        rect.height(),
                        policy.bands().workspace_px,
                        "{:?} remainder disagrees with the declared workspace band at {}",
                        band.region(),
                        policy.canonical_name()
                    ),
                }
            }
        }
    }

    /// The overflow rule the band assertions are measured with detects both
    /// defects it claims to, and neither of the two non-defects.
    ///
    /// The rule is deliberately not "every run is inside its band": leaving a
    /// scroll region is how a scroll region works, and two runs the rendering
    /// stack clipped to different containers cannot collide however their
    /// rectangles fall. Those exemptions are what make the rule usable, and
    /// they are also what would make it silently vacuous if it were wrong — so
    /// it is exercised here against runs placed by hand rather than only
    /// against frames that happen to be clean.
    #[test]
    fn the_overflow_rule_detects_a_clipped_run_and_a_collision_and_nothing_else() {
        use super::testing_support::{overflow_report, PaintedRun};
        use eframe::egui::{pos2, Rect};

        let fixed = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 20.0));
        let scrolling = Rect::from_min_max(pos2(0.0, 40.0), pos2(100.0, 60.0));
        let run = |content: &str, min_x: f32, max_x: f32, top: f32, clip: Rect| PaintedRun {
            content: content.to_owned(),
            rect: Rect::from_min_max(pos2(min_x, top), pos2(max_x, top + 10.0)),
            clip,
        };

        // Inside a fixed container, leaving it is a defect.
        let clipped = overflow_report(&[run("cut", -20.0, 40.0, 5.0, fixed)], &[fixed]);
        assert_eq!(clipped.defects.len(), 1, "{:#?}", clipped.defects);
        assert_eq!(clipped.left_their_container, 1);

        // The same run in a scrolling container is counted, not faulted.
        let scrolled = overflow_report(&[run("cut", -20.0, 40.0, 45.0, scrolling)], &[fixed]);
        assert!(scrolled.defects.is_empty(), "{:#?}", scrolled.defects);
        assert_eq!(scrolled.left_their_container, 1);

        // Two readable runs in one container that collide is a defect.
        let collided = overflow_report(
            &[
                run("left", 0.0, 60.0, 5.0, fixed),
                run("right", 40.0, 100.0, 5.0, fixed),
            ],
            &[fixed],
        );
        assert_eq!(collided.defects.len(), 1, "{:#?}", collided.defects);
        assert_eq!(collided.left_their_container, 0);

        // The same two rectangles in different containers cannot collide.
        let separated = overflow_report(
            &[
                run("left", 0.0, 60.0, 5.0, fixed),
                run("right", 40.0, 100.0, 5.0, scrolling),
            ],
            &[fixed],
        );
        assert!(separated.defects.is_empty(), "{:#?}", separated.defects);

        // Two runs that merely touch do not.
        let adjacent = overflow_report(
            &[
                run("left", 0.0, 50.0, 5.0, fixed),
                run("right", 50.0, 100.0, 5.0, fixed),
            ],
            &[fixed],
        );
        assert!(adjacent.defects.is_empty(), "{:#?}", adjacent.defects);
    }

    /// An addressed hint answers only for the collection it was minted over.
    ///
    /// The footer paints one collection and the caller resolves against
    /// another. While they agree the index names the same element in both;
    /// when they do not, this is what stops a neighbouring element being
    /// returned as though it were the addressed one.
    #[test]
    fn an_addressed_hint_resolves_only_against_the_collection_it_was_minted_over() {
        let painted = ["1 OPEN MIXER", "2 OPEN PATCH", "W MOVE UP", "S MOVE DOWN"];
        let resolved = ["open-mixer", "open-patch", "move-up", "move-down"];
        for index in 0..painted.len() {
            let hint = AddressedHint::minted(index, painted.len());
            assert_eq!(hint.minted_over(), painted.len());
            // `resolve` is the only route from a hint to an element; the
            // painted slice is what names which one it was.
            assert_eq!(hint.resolve(&resolved), Some(&resolved[index]));
            assert_eq!(hint.resolve(&painted), Some(&painted[index]));
        }

        let minted_over_four = AddressedHint::minted(2, painted.len());
        // A shorter list is not the list this index names, even where the
        // index is in range for it — which is the case a bare `usize` would
        // have answered with the wrong element.
        assert_eq!(minted_over_four.resolve(&resolved[..3]), None);
        // An empty resolving collection is the shape the shared fixture has,
        // and it resolves to nothing rather than panicking or guessing.
        assert_eq!(minted_over_four.resolve::<&str>(&[]), None);
        // Same length, different contents: the count is a guard against a
        // desynchronised collection, not a proof of identity, and it is
        // documented as exactly that.
        let renamed = ["a", "b", "c", "d"];
        assert_eq!(minted_over_four.resolve(&renamed), Some(&"c"));
    }

    /// The frame renders at both authored viewports with every band and the
    /// persistent side region retained. This is the assertion that fails if a
    /// band is dropped to fit the compact viewport.
    #[test]
    fn both_authored_viewports_retain_every_band_and_the_side_region() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let viewport = policy.authored_viewport();
                for band in frame_plan(&policy) {
                    let rect = band_rect(&policy, band.observed_region_id());
                    assert!(
                        rect.width() > 0.0 && rect.height() > 0.0,
                        "{} dropped {:?}",
                        policy.canonical_name(),
                        band.region()
                    );
                    assert!(
                        rect.max.x <= viewport.width_px && rect.max.y <= viewport.height_px,
                        "{} pushed {:?} off the viewport",
                        policy.canonical_name(),
                        band.region()
                    );
                }
                // The workspace row is tiled as the remainder rather than
                // from a declared height, so this is where that arithmetic is
                // held against the policy's own workspace band.
                let workspace = band_rect(&policy, ShellRegionId::MainWorkspace);
                let side = band_rect(&policy, ShellRegionId::PersistentSideRegion);
                assert_eq!(
                    workspace.height(),
                    policy.bands().workspace_px,
                    "{} workspace remainder disagrees with the declared band",
                    policy.canonical_name()
                );
                assert_eq!(
                    workspace.width() + side.width(),
                    viewport.width_px,
                    "{} workspace split does not tile the viewport",
                    policy.canonical_name()
                );
                assert_eq!(
                    side.width(),
                    policy.split().side_px,
                    "{} narrowed or hid the persistent side region",
                    policy.canonical_name()
                );

                // The three chrome bands the frame itself fills must each have
                // put something on screen; the two workspace regions are
                // WP05's and are empty until it lands.
                for region in [
                    ShellRegionId::ContextLine,
                    ShellRegionId::IdentityHeader,
                    ShellRegionId::Footer,
                ] {
                    assert!(
                        !band_runs(&projection, &policy, region).is_empty(),
                        "{} painted nothing into {region:?} at {}",
                        context.label(),
                        policy.canonical_name()
                    );
                }
            }
        }
    }

    /// No run the frame paints leaves the band it was given, and no two runs in
    /// a band collide, at either viewport. The gallery witness asserts zero
    /// clipped or overlapping text, so this is that assertion at the source.
    #[test]
    fn no_chrome_run_clips_or_overlaps_at_either_viewport() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                for region in [
                    ShellRegionId::ContextLine,
                    ShellRegionId::IdentityHeader,
                    ShellRegionId::Footer,
                ] {
                    let defects = containment_defects(
                        &band_runs(&projection, &policy, region),
                        band_rect(&policy, region),
                    );
                    assert!(
                        defects.is_empty(),
                        "{region:?} at {} in {}: {defects:#?}",
                        policy.canonical_name(),
                        context.label()
                    );
                }
            }
        }
    }

    /// The three bands this frame fills paint only text the projection
    /// supplied. A run that is not a projection value or a closed-vocabulary
    /// label is invented, which is what `C-003` forbids in the shipped shell.
    ///
    /// Scoped to those three bands, not to the whole frame. The whitelist below
    /// is chrome, and a whole-frame loop makes it adjudicate every string
    /// anything paints — so `WP05`'s section titles, parameter names and values
    /// would each be reported as invented the moment the workspace bands fill,
    /// in a file `WP05` cannot edit. Widening the whitelist is not the fix
    /// either: the set of strings the workspace may legitimately paint is not
    /// knowable here. `C-003` for the workspace bands is `WP05`'s to assert in
    /// `WP05`'s own files, exactly as the band-scoped siblings of this test in
    /// `identity_header.rs` and `context_switch.rs` assert it for theirs.
    #[test]
    fn the_frame_paints_only_text_the_projection_supplied() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let mode = projection.semantic_model().interaction_mode();
                let mut permitted: Vec<String> = vec![
                    projection.context_line().product_label().to_owned(),
                    projection.context_line().status_label().to_owned(),
                    projection.identity_header().primary_label().to_owned(),
                    projection.identity_header().secondary_label().to_owned(),
                    projection.footer().path_label().to_owned(),
                    mode.label().to_owned(),
                ];
                // The switch paints each context's own label and, separately,
                // its mark. Both are closed vocabularies, neither is invented.
                permitted.extend(
                    TopLevelContext::ALL
                        .into_iter()
                        .map(|entry| entry.label().to_owned())
                        .collect::<Vec<_>>(),
                );
                permitted.extend([
                    context_switch::entry_mark(true).to_owned(),
                    context_switch::entry_mark(false).to_owned(),
                ]);
                permitted.extend(projection.footer().action_hints().iter().cloned());

                for region in [
                    ShellRegionId::ContextLine,
                    ShellRegionId::IdentityHeader,
                    ShellRegionId::Footer,
                ] {
                    let painted = painted_text(&band_runs(&projection, &policy, region));
                    assert!(
                        !painted.is_empty(),
                        "{region:?} painted nothing at {} in {}, so nothing was adjudicated",
                        policy.canonical_name(),
                        context.label()
                    );
                    for run in painted {
                        let trimmed = run.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        assert!(
                            permitted.iter().any(|allowed| allowed.trim() == trimmed),
                            "the {region:?} band invented {run:?} at {} in {}",
                            policy.canonical_name(),
                            context.label()
                        );
                    }
                }
            }
        }
    }

    /// Rendering the frame produces no intent of its own. The frame arranges;
    /// asking is the business of the controls inside it, and there are none in
    /// the chrome bands.
    #[test]
    fn an_arranged_frame_with_nothing_addressed_asks_for_nothing() {
        let projection = projection_fixture(TopLevelContext::Patch);
        let policy = ViewportDensityPolicy::Desktop;
        let mut frame = ShellFrameIntent::none();
        assert!(frame.is_empty());
        assert!(frame.intent().is_empty());
        assert_eq!(frame.activated_hint(), None);
        frame.absorb(ShellFrameIntent::none());
        assert!(frame.is_empty());
        // Driving the real render path leaves the aggregate empty because no
        // pointer addressed anything this pass.
        assert!(painted_frame(&projection, &policy)
            .iter()
            .any(|run| !run.content.trim().is_empty()));
    }

    /// The family dispatch reaches the same painting this module calls
    /// directly.
    ///
    /// `compositions/mod.rs` resolves each variant to a renderer, and a
    /// composition wired to the wrong one — or to the shared stub — would paint
    /// something else, or nothing. Comparing the two entry points is what makes
    /// "reachable through the family" a measurement rather than a claim about
    /// a line of source.
    #[test]
    fn the_family_dispatch_reaches_the_same_frame_this_module_arranges() {
        fn through_the_family(
            ui: &mut Ui,
            projection: &GraphicalShellProjection,
            density: &ViewportDensityPolicy,
        ) -> CompositionIntent {
            ShellComposition::ApplicationShell.render(ui, projection, density)
        }

        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let direct = painted_text(&painted_frame(&projection, &policy));
                let dispatched =
                    painted_text(&painted_via(&projection, &policy, through_the_family).runs);
                assert!(
                    !direct.is_empty(),
                    "{} painted nothing to compare",
                    policy.canonical_name()
                );
                assert_eq!(
                    dispatched,
                    direct,
                    "{} paints differently through the family at {}",
                    context.label(),
                    policy.canonical_name()
                );
            }
        }
    }

    /// Each band composition is reached through the family too, and each puts
    /// its own band's text on screen. A variant still wired to the shared stub
    /// would paint nothing and fail here.
    #[test]
    fn every_frame_composition_is_reachable_through_the_family_and_paints() {
        for (composition, region) in [
            (ShellComposition::ContextSwitch, ShellRegionId::ContextLine),
            (
                ShellComposition::IdentityHeader,
                ShellRegionId::IdentityHeader,
            ),
            (ShellComposition::Footer, ShellRegionId::Footer),
        ] {
            for policy in ALL_DENSITY_POLICIES {
                let projection = projection_fixture(TopLevelContext::Patch);
                assert_eq!(
                    frame_plan(&policy)
                        .iter()
                        .find(|band| band.observed_region_id() == region)
                        .map(|band| band.composition()),
                    Some(composition),
                    "{region:?} is not filled by {}",
                    composition.canonical_name()
                );
                assert!(
                    !band_runs(&projection, &policy, region).is_empty(),
                    "{} painted nothing into {region:?} at {}",
                    composition.canonical_name(),
                    policy.canonical_name()
                );
            }
        }
    }

    /// Every composition in the family is reachable through the plan or is
    /// workspace content one of the planned compositions arranges. This is what
    /// catches a band bound to a composition nothing else fills.
    #[test]
    fn every_planned_band_names_a_declared_composition() {
        let planned: HashSet<ShellComposition> = frame_plan(&ViewportDensityPolicy::Desktop)
            .iter()
            .map(|band| band.composition())
            .collect();
        for composition in planned {
            assert!(
                ALL_SHELL_COMPOSITIONS.contains(&composition),
                "{} is not a declared composition",
                composition.canonical_name()
            );
            assert_ne!(
                composition,
                ShellComposition::ApplicationShell,
                "the frame cannot be a band of itself"
            );
        }
    }
}
