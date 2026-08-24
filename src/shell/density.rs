//! Bounded responsive shell composition.
//!
//! Rust owns the named bounds and structural thresholds exported to CSS. The
//! browser computes the current composition from its containing block; no
//! layout mode enters product state or the semantic projection. Reference
//! viewport sizes exist only as deterministic witness inputs.

use crate::shell::tokens::MIN_INTERACTIVE_TARGET_PX;

/// Presentation modes reported by painted-DOM observations.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResponsiveLayoutMode {
    Wide,
    Standard,
    Compact,
}

pub const ALL_LAYOUT_MODES: [ResponsiveLayoutMode; 3] = [
    ResponsiveLayoutMode::Wide,
    ResponsiveLayoutMode::Standard,
    ResponsiveLayoutMode::Compact,
];

/// Compact stacks the main and persistent side regions at or below this width.
pub const COMPACT_MAX_WIDTH_PX: f32 = 1_023.0;
/// Standard retains two tracks through this width; larger widths are Wide.
pub const STANDARD_MAX_WIDTH_PX: f32 = 1_599.0;

impl ResponsiveLayoutMode {
    /// Mirrors the generated CSS media-query boundaries for observations only.
    pub fn resolve(viewport_width_px: f32) -> Self {
        if viewport_width_px <= COMPACT_MAX_WIDTH_PX {
            Self::Compact
        } else if viewport_width_px <= STANDARD_MAX_WIDTH_PX {
            Self::Standard
        } else {
            Self::Wide
        }
    }

    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Wide => "Wide",
            Self::Standard => "Standard",
            Self::Compact => "Compact",
        }
    }
}

/// A CSS `clamp()` contract expressed in one source vocabulary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutBounds {
    pub minimum_px: f32,
    pub preferred_px: f32,
    pub maximum_px: f32,
}

impl LayoutBounds {
    pub const fn new(minimum_px: f32, preferred_px: f32, maximum_px: f32) -> Self {
        Self {
            minimum_px,
            preferred_px,
            maximum_px,
        }
    }

    pub const fn is_ordered(self) -> bool {
        self.minimum_px <= self.preferred_px && self.preferred_px <= self.maximum_px
    }
}

/// The responsive layout vocabulary exported to the webview.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResponsiveShellContract {
    pub context_line: LayoutBounds,
    pub identity_header: LayoutBounds,
    pub footer: LayoutBounds,
    pub content_inset: LayoutBounds,
    pub region_gap: LayoutBounds,
    pub row_height: LayoutBounds,
    pub side_track: LayoutBounds,
    pub mixer_column: LayoutBounds,
    pub utility_bar_thickness_px: f32,
}

impl ResponsiveShellContract {
    pub const fn get() -> Self {
        Self {
            context_line: LayoutBounds::new(40.0, 48.0, 56.0),
            identity_header: LayoutBounds::new(60.0, 72.0, 88.0),
            footer: LayoutBounds::new(56.0, 64.0, 80.0),
            content_inset: LayoutBounds::new(16.0, 24.0, 32.0),
            region_gap: LayoutBounds::new(8.0, 16.0, 24.0),
            row_height: LayoutBounds::new(MIN_INTERACTIVE_TARGET_PX, 52.0, 60.0),
            side_track: LayoutBounds::new(320.0, 360.0, 420.0),
            mixer_column: LayoutBounds::new(MIN_INTERACTIVE_TARGET_PX, 72.0, 82.0),
            utility_bar_thickness_px: 5.0,
        }
    }

    pub const fn all_bounds(self) -> [(&'static str, LayoutBounds); 8] {
        [
            ("context line", self.context_line),
            ("identity header", self.identity_header),
            ("footer", self.footer),
            ("content inset", self.content_inset),
            ("region gap", self.region_gap),
            ("row height", self.row_height),
            ("side track", self.side_track),
            ("mixer column", self.mixer_column),
        ]
    }

    /// Deterministic geometry for transport-only synthetic paint witnesses.
    /// Production layout remains CSS-computed.
    pub fn witness_geometry(self, width_px: f32, height_px: f32) -> WitnessShellGeometry {
        let layout_mode = ResponsiveLayoutMode::resolve(width_px);
        let context_line_px = self.context_line.preferred_px;
        let identity_header_px = self.identity_header.preferred_px;
        let footer_px = self.footer.preferred_px;
        let workspace_y_px = context_line_px + identity_header_px;
        let workspace_height_px = height_px - workspace_y_px - footer_px;
        let side_track_px = self
            .side_track
            .preferred_px
            .clamp(self.side_track.minimum_px, self.side_track.maximum_px);
        let (main_width_px, main_height_px, side_x_px, side_y_px, side_width_px, side_height_px) =
            match layout_mode {
                ResponsiveLayoutMode::Compact => (
                    width_px,
                    workspace_height_px / 2.0,
                    0.0,
                    workspace_y_px + workspace_height_px / 2.0,
                    width_px,
                    workspace_height_px / 2.0,
                ),
                ResponsiveLayoutMode::Wide | ResponsiveLayoutMode::Standard => (
                    width_px - side_track_px,
                    workspace_height_px,
                    width_px - side_track_px,
                    workspace_y_px,
                    side_track_px,
                    workspace_height_px,
                ),
            };
        WitnessShellGeometry {
            layout_mode,
            context_line_px,
            identity_header_px,
            footer_px,
            workspace_y_px,
            main_width_px,
            main_height_px,
            side_x_px,
            side_y_px,
            side_width_px,
            side_height_px,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WitnessShellGeometry {
    pub layout_mode: ResponsiveLayoutMode,
    pub context_line_px: f32,
    pub identity_header_px: f32,
    pub footer_px: f32,
    pub workspace_y_px: f32,
    pub main_width_px: f32,
    pub main_height_px: f32,
    pub side_x_px: f32,
    pub side_y_px: f32,
    pub side_width_px: f32,
    pub side_height_px: f32,
}

/// Deterministic viewport inputs for automated and native witnesses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RepresentativeViewport {
    WideReference,
    StandardReference,
    Intermediate,
    Compact,
    ScaledText,
}

pub const ALL_REPRESENTATIVE_VIEWPORTS: [RepresentativeViewport; 5] = [
    RepresentativeViewport::WideReference,
    RepresentativeViewport::StandardReference,
    RepresentativeViewport::Intermediate,
    RepresentativeViewport::Compact,
    RepresentativeViewport::ScaledText,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportFixture {
    pub width_px: f32,
    pub height_px: f32,
    pub text_scale: f32,
}

impl RepresentativeViewport {
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::WideReference => "WideReference",
            Self::StandardReference => "StandardReference",
            Self::Intermediate => "Intermediate",
            Self::Compact => "Compact",
            Self::ScaledText => "ScaledText",
        }
    }

    pub const fn fixture(self) -> ViewportFixture {
        let (width_px, height_px, text_scale) = match self {
            Self::WideReference => (1_920.0, 1_080.0, 1.0),
            Self::StandardReference => (1_280.0, 800.0, 1.0),
            Self::Intermediate => (1_440.0, 900.0, 1.0),
            Self::Compact => (900.0, 800.0, 1.0),
            Self::ScaledText => (1_280.0, 800.0, 1.25),
        };
        ViewportFixture {
            width_px,
            height_px,
            text_scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_are_ordered_and_interactive_floors_are_preserved() {
        let contract = ResponsiveShellContract::get();
        for (name, bounds) in contract.all_bounds() {
            assert!(bounds.is_ordered(), "{name} bounds are not ordered");
            assert!(bounds.minimum_px >= 0.0, "{name} has a negative floor");
        }
        assert!(contract.row_height.minimum_px >= MIN_INTERACTIVE_TARGET_PX);
        assert!(contract.mixer_column.minimum_px >= MIN_INTERACTIVE_TARGET_PX);
    }

    #[test]
    fn layout_thresholds_are_ordered_and_cover_intermediate_widths() {
        const {
            assert!(COMPACT_MAX_WIDTH_PX < STANDARD_MAX_WIDTH_PX);
        }
        assert_eq!(
            ResponsiveLayoutMode::resolve(900.0),
            ResponsiveLayoutMode::Compact
        );
        assert_eq!(
            ResponsiveLayoutMode::resolve(1_280.0),
            ResponsiveLayoutMode::Standard
        );
        assert_eq!(
            ResponsiveLayoutMode::resolve(1_440.0),
            ResponsiveLayoutMode::Standard
        );
        assert_eq!(
            ResponsiveLayoutMode::resolve(1_920.0),
            ResponsiveLayoutMode::Wide
        );
    }

    #[test]
    fn representative_viewports_are_witness_inputs_not_geometry_policies() {
        for viewport in ALL_REPRESENTATIVE_VIEWPORTS {
            let fixture = viewport.fixture();
            assert!(fixture.width_px > 0.0);
            assert!(fixture.height_px > 0.0);
            assert!(fixture.text_scale >= 1.0);
        }
        assert_eq!(
            ResponsiveLayoutMode::resolve(
                RepresentativeViewport::StandardReference.fixture().width_px
            ),
            ResponsiveLayoutMode::Standard
        );
    }
}
