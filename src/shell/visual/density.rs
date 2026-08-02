//! Viewport density policies.
//!
//! The interface is authored at two sizes and only two. Rather than let each
//! surface carry its own resolution-specific constants, each authored size is
//! declared once here as a policy carrying band heights, the main/side split,
//! the content rhythm, and control geometry.
//!
//! A surface asks the policy. It does not read a viewport size to decide a
//! layout: [`ViewportDensityPolicy::resolve`] is the single place a raw width
//! is consumed, and everything downstream of it is a named accessor. The
//! intended first consumer is `src/adapter/eframe_graphical_window.rs`, whose
//! band constants and ad-hoc `desired_side_width` proportional rule this
//! module replaces — that swap is WP04's work, not this module's.
//!
//! Realizes `valueObject.Shell.ViewportDensityPolicy`.

/// Which authored viewport a policy resolves.
///
/// The set is closed at two. A third size is a design decision, not a code
/// decision: it is authored in `DESIGN.md` and added here, never synthesized
/// by scaling one of these at a call site.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ViewportDensityPolicy {
    /// The authored 1920×1080 layout, measured from the design file.
    Desktop,
    /// The 1280×800 Steam Deck layout, authored from the desktop frames.
    SteamDeck,
}

/// Every declared policy, largest viewport first.
pub const ALL_DENSITY_POLICIES: [ViewportDensityPolicy; 2] = [
    ViewportDensityPolicy::Desktop,
    ViewportDensityPolicy::SteamDeck,
];

/// The widest viewport that resolves to [`ViewportDensityPolicy::SteamDeck`].
///
/// This is the authored Steam Deck width. The declared resolution rule is a
/// single threshold: at or below this width the Steam Deck policy applies,
/// above it the Desktop policy does. A viewport between or below the two
/// authored sizes therefore resolves to an authored policy rather than to a
/// third, interpolated layout.
pub const STEAM_DECK_MAX_WIDTH_PX: f32 = 1_280.0;

/// Where a policy's numbers came from.
///
/// Recorded rather than blurred: one policy is transcription and the other is
/// authorship, and a reader deciding whether to trust a value needs to know
/// which.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PolicyProvenance {
    /// Measured from the authored design file. Changing one of these values
    /// changes what the product looks like away from what was designed.
    MeasuredFromAuthoredDesign,
    /// Authored from the desktop frames and the declared minimums, because no
    /// design exists at this size. Subject to visual review.
    AuthoredFromDesktopFrames,
}

/// The authored size a policy resolves.
///
/// This exists so a window or a specimen frame can be opened at the size the
/// policy was authored for. It is deliberately not a pair of loose numbers: no
/// surface branches on these to pick a layout — that is what the policy itself
/// is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuthoredViewport {
    /// Authored width in pixels.
    pub width_px: f32,
    /// Authored height in pixels.
    pub height_px: f32,
}

/// The heights of the four vertical bands, top to bottom.
///
/// The workspace band is the one that splits into a main surface and the
/// persistent side region; see [`SurfaceSplit`]. Together they are the five
/// structural regions the layout must always retain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StructuralBands {
    /// Product · context · status.
    pub context_line_px: f32,
    /// Identity and header.
    pub identity_header_px: f32,
    /// The workspace that holds the main surface and the side region.
    pub workspace_px: f32,
    /// Current path and valid control hints.
    pub footer_px: f32,
}

impl StructuralBands {
    /// The total height the four bands occupy.
    ///
    /// This equals the authored viewport height exactly. The bands tile the
    /// viewport; nothing floats above or below them.
    pub const fn total_height_px(self) -> f32 {
        self.context_line_px + self.identity_header_px + self.workspace_px + self.footer_px
    }
}

/// How the workspace band divides horizontally.
///
/// The side region is the Utility/Inspector. It is always visible — narrowing
/// it is a density decision, hiding it is not one this vocabulary can express.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceSplit {
    /// The main surface width.
    pub main_px: f32,
    /// The persistent Utility/Inspector width.
    pub side_px: f32,
}

impl SurfaceSplit {
    /// The total width the split occupies.
    ///
    /// This equals the authored viewport width exactly.
    pub const fn total_width_px(self) -> f32 {
        self.main_px + self.side_px
    }
}

/// The vertical and horizontal rhythm content is laid out on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContentRhythm {
    /// How far content insets from the screen edge.
    pub inset_px: f32,
    /// The height of one row.
    pub row_height_px: f32,
    /// The distance between the tops of two consecutive rows.
    pub row_pitch_px: f32,
}

impl ContentRhythm {
    /// The gap between the bottom of one row and the top of the next.
    pub const fn row_gap_px(self) -> f32 {
        self.row_pitch_px - self.row_height_px
    }
}

/// The geometry of one utility control in the side region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlGeometry {
    /// Control width.
    pub width_px: f32,
    /// Control height. This is an interactive target, so it never drops below
    /// [`super::token::MIN_INTERACTIVE_TARGET_PX`].
    pub height_px: f32,
    /// The distance between the tops of two consecutive controls.
    pub pitch_px: f32,
    /// The thickness of the control's value bar.
    pub bar_thickness_px: f32,
}

impl ViewportDensityPolicy {
    /// Resolves a viewport width to the policy that governs it.
    ///
    /// This is the one function in the codebase that reads a raw viewport size.
    /// Everything else asks the resolved policy. A width at or below
    /// [`STEAM_DECK_MAX_WIDTH_PX`] — including one below the authored Steam
    /// Deck size — resolves to [`Self::SteamDeck`]; anything wider resolves to
    /// [`Self::Desktop`].
    pub fn resolve(viewport_width_px: f32) -> Self {
        if viewport_width_px <= STEAM_DECK_MAX_WIDTH_PX {
            Self::SteamDeck
        } else {
            Self::Desktop
        }
    }

    /// Returns the size this policy was authored for.
    pub const fn authored_viewport(self) -> AuthoredViewport {
        let (width_px, height_px) = match self {
            Self::Desktop => (1_920.0, 1_080.0),
            Self::SteamDeck => (1_280.0, 800.0),
        };
        AuthoredViewport {
            width_px,
            height_px,
        }
    }

    /// Returns the four vertical band heights.
    ///
    /// Desktop reproduces `DESIGN.md:440-445` and the constants the adapter
    /// already carries. The Steam Deck bands shrink less than proportionally:
    /// they carry text at fixed authored sizes, so uniform scaling would
    /// squeeze the type before it squeezed the space.
    pub const fn bands(self) -> StructuralBands {
        let (context_line_px, identity_header_px, workspace_px, footer_px) = match self {
            Self::Desktop => (48.0, 72.0, 896.0, 64.0),
            Self::SteamDeck => (40.0, 60.0, 644.0, 56.0),
        };
        StructuralBands {
            context_line_px,
            identity_header_px,
            workspace_px,
            footer_px,
        }
    }

    /// Returns the main-surface and side-region widths.
    ///
    /// The Steam Deck side region is 320 px — the same floor the adapter
    /// already treats as its minimum. It is narrowed, never hidden.
    pub const fn split(self) -> SurfaceSplit {
        let (main_px, side_px) = match self {
            Self::Desktop => (1_500.0, 420.0),
            Self::SteamDeck => (960.0, 320.0),
        };
        SurfaceSplit { main_px, side_px }
    }

    /// Returns the content inset and row rhythm.
    ///
    /// Desktop row height is 52 px, above the 48 px minimum target rather than
    /// at it. The Steam Deck row height sits exactly on the floor: that is the
    /// density budget spent, not a value with room left in it.
    pub const fn rhythm(self) -> ContentRhythm {
        let (inset_px, row_height_px, row_pitch_px) = match self {
            Self::Desktop => (24.0, 52.0, 66.0),
            Self::SteamDeck => (16.0, 48.0, 56.0),
        };
        ContentRhythm {
            inset_px,
            row_height_px,
            row_pitch_px,
        }
    }

    /// Returns the geometry of one utility control in the side region.
    ///
    /// The bar thickness does not change between policies. It is a visual
    /// weight rather than a spatial one, and thinning it would cost legibility
    /// without buying room.
    pub const fn utility_control(self) -> ControlGeometry {
        let (width_px, height_px, pitch_px) = match self {
            Self::Desktop => (380.0, 48.0, 60.0),
            Self::SteamDeck => (280.0, 48.0, 56.0),
        };
        ControlGeometry {
            width_px,
            height_px,
            pitch_px,
            bar_thickness_px: 5.0,
        }
    }

    /// Returns whether this policy was measured or authored.
    pub const fn provenance(self) -> PolicyProvenance {
        match self {
            Self::Desktop => PolicyProvenance::MeasuredFromAuthoredDesign,
            Self::SteamDeck => PolicyProvenance::AuthoredFromDesktopFrames,
        }
    }

    /// Returns the canonical name of this policy.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Desktop => "Desktop",
            Self::SteamDeck => "SteamDeck",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::visual::token::MIN_INTERACTIVE_TARGET_PX;

    /// Every interactive target this policy declares, for the minimum-target
    /// assertions. Spelled out rather than derived so a new interactive
    /// dimension has to be added here deliberately.
    fn interactive_target_heights(policy: ViewportDensityPolicy) -> [(&'static str, f32); 2] {
        [
            ("row height", policy.rhythm().row_height_px),
            ("utility control height", policy.utility_control().height_px),
        ]
    }

    #[test]
    fn the_desktop_policy_reproduces_the_current_adapter_constants_exactly() {
        // These are the values `src/adapter/eframe_graphical_window.rs:17-25`
        // carries today. WP04 deletes those constants in favour of this
        // policy; if this test ever fails, that swap changed the geometry.
        let desktop = ViewportDensityPolicy::Desktop;
        assert_eq!(desktop.authored_viewport().width_px, 1_920.0);
        assert_eq!(desktop.authored_viewport().height_px, 1_080.0);
        assert_eq!(desktop.bands().context_line_px, 48.0);
        assert_eq!(desktop.bands().identity_header_px, 72.0);
        assert_eq!(desktop.bands().footer_px, 64.0);
        assert_eq!(desktop.split().side_px, 420.0);
    }

    #[test]
    fn the_desktop_policy_reproduces_the_authored_design_geometry() {
        // `DESIGN.md:440-445` plus the per-variant values measured from the
        // design file's Patch Strip frame.
        let desktop = ViewportDensityPolicy::Desktop;
        assert_eq!(desktop.bands().workspace_px, 896.0);
        assert_eq!(desktop.split().main_px, 1_500.0);
        assert_eq!(desktop.rhythm().inset_px, 24.0);
        assert_eq!(desktop.rhythm().row_height_px, 52.0);
        assert_eq!(desktop.rhythm().row_pitch_px, 66.0);
        assert_eq!(desktop.utility_control().width_px, 380.0);
        assert_eq!(desktop.utility_control().height_px, 48.0);
        assert_eq!(desktop.utility_control().pitch_px, 60.0);
        assert_eq!(desktop.utility_control().bar_thickness_px, 5.0);
    }

    #[test]
    fn the_steam_deck_policy_carries_its_authored_values() {
        let deck = ViewportDensityPolicy::SteamDeck;
        assert_eq!(deck.authored_viewport().width_px, 1_280.0);
        assert_eq!(deck.authored_viewport().height_px, 800.0);
        assert_eq!(deck.bands().context_line_px, 40.0);
        assert_eq!(deck.bands().identity_header_px, 60.0);
        assert_eq!(deck.bands().workspace_px, 644.0);
        assert_eq!(deck.bands().footer_px, 56.0);
        assert_eq!(deck.split().main_px, 960.0);
        assert_eq!(deck.split().side_px, 320.0);
        assert_eq!(deck.rhythm().inset_px, 16.0);
        assert_eq!(deck.rhythm().row_height_px, 48.0);
        assert_eq!(deck.rhythm().row_pitch_px, 56.0);
        assert_eq!(deck.utility_control().width_px, 280.0);
        assert_eq!(deck.utility_control().pitch_px, 56.0);
    }

    #[test]
    fn every_policy_retains_all_five_structural_regions() {
        for policy in ALL_DENSITY_POLICIES {
            let bands = policy.bands();
            let split = policy.split();
            let regions: [(&str, f32); 5] = [
                ("context line", bands.context_line_px),
                ("identity header", bands.identity_header_px),
                ("main surface", split.main_px),
                ("side region", split.side_px),
                ("footer", bands.footer_px),
            ];
            for (name, extent) in regions {
                assert!(
                    extent > 0.0,
                    "{} dropped the {} region",
                    policy.canonical_name(),
                    name
                );
            }
        }
    }

    #[test]
    fn every_policy_tiles_its_viewport_exactly() {
        for policy in ALL_DENSITY_POLICIES {
            let viewport = policy.authored_viewport();
            assert_eq!(
                policy.bands().total_height_px(),
                viewport.height_px,
                "{} bands do not sum to the viewport height",
                policy.canonical_name()
            );
            assert_eq!(
                policy.split().total_width_px(),
                viewport.width_px,
                "{} split does not sum to the viewport width",
                policy.canonical_name()
            );
        }
    }

    #[test]
    fn every_policy_holds_every_interactive_target_at_or_above_the_minimum() {
        for policy in ALL_DENSITY_POLICIES {
            for (name, height) in interactive_target_heights(policy) {
                assert!(
                    height >= MIN_INTERACTIVE_TARGET_PX,
                    "{} {} is {} px, below the {} px minimum",
                    policy.canonical_name(),
                    name,
                    height,
                    MIN_INTERACTIVE_TARGET_PX
                );
            }
        }
    }

    #[test]
    fn every_policy_leaves_a_positive_gap_between_rows_and_between_controls() {
        for policy in ALL_DENSITY_POLICIES {
            assert!(
                policy.rhythm().row_gap_px() > 0.0,
                "{} rows overlap",
                policy.canonical_name()
            );
            assert!(
                policy.utility_control().pitch_px > policy.utility_control().height_px,
                "{} utility controls overlap",
                policy.canonical_name()
            );
        }
    }

    #[test]
    fn every_policy_fits_its_utility_control_inside_its_side_region() {
        for policy in ALL_DENSITY_POLICIES {
            assert!(
                policy.utility_control().width_px < policy.split().side_px,
                "{} utility control is wider than its side region",
                policy.canonical_name()
            );
        }
    }

    #[test]
    fn viewport_widths_resolve_through_the_declared_threshold() {
        // At and below the Steam Deck width, including below the authored
        // size; above it, Desktop. No third layout exists to resolve to.
        assert_eq!(
            ViewportDensityPolicy::resolve(1_920.0),
            ViewportDensityPolicy::Desktop
        );
        assert_eq!(
            ViewportDensityPolicy::resolve(2_560.0),
            ViewportDensityPolicy::Desktop
        );
        assert_eq!(
            ViewportDensityPolicy::resolve(1_281.0),
            ViewportDensityPolicy::Desktop
        );
        assert_eq!(
            ViewportDensityPolicy::resolve(1_280.0),
            ViewportDensityPolicy::SteamDeck
        );
        assert_eq!(
            ViewportDensityPolicy::resolve(1_600.0),
            ViewportDensityPolicy::Desktop
        );
        assert_eq!(
            ViewportDensityPolicy::resolve(1_024.0),
            ViewportDensityPolicy::SteamDeck
        );
        assert_eq!(
            ViewportDensityPolicy::resolve(0.0),
            ViewportDensityPolicy::SteamDeck
        );
    }

    #[test]
    fn each_authored_size_resolves_to_its_own_policy() {
        for policy in ALL_DENSITY_POLICIES {
            assert_eq!(
                ViewportDensityPolicy::resolve(policy.authored_viewport().width_px),
                policy,
                "{} does not resolve to itself",
                policy.canonical_name()
            );
        }
    }

    #[test]
    fn the_measured_and_authored_policies_stay_distinguishable() {
        assert_eq!(
            ViewportDensityPolicy::Desktop.provenance(),
            PolicyProvenance::MeasuredFromAuthoredDesign
        );
        assert_eq!(
            ViewportDensityPolicy::SteamDeck.provenance(),
            PolicyProvenance::AuthoredFromDesktopFrames
        );
        assert_eq!(ALL_DENSITY_POLICIES.len(), 2);
    }
}
