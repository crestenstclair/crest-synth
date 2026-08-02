//! The authored visual vocabulary every rendered surface resolves through.
//!
//! Colors, type styles, spacing steps, and geometry are declared once in
//! [`token`]; the typeface that makes those styles renderable is installed by
//! [`typeface`]. Density policies, the component state vocabulary, and the
//! reusable primitives complete the module as later work packages land.
//!
//! Raw values stay private to their declaring module. Nothing outside this
//! module constructs a color, a type size, or a spacing constant.

pub mod density; // WP02
pub mod primitives; // WP03
pub mod state; // WP02
pub mod token;
pub mod typeface;

pub use density::{
    AuthoredViewport, ContentRhythm, ControlGeometry, PolicyProvenance, StructuralBands,
    SurfaceSplit, ViewportDensityPolicy, ALL_DENSITY_POLICIES, STEAM_DECK_MAX_WIDTH_PX,
};
pub use state::{
    ComponentState, NonColorSignal, StateAppearance, ALL_COMPONENT_STATES, COMPONENT_STATE_COUNT,
    LOADING_PROGRESS_WORDS,
};
pub use token::{
    FontWeight, Radius, SemanticColor, SpacingStep, TypeStyle, TypeStyleMetrics, ALL_COLORS,
    ALL_RADII, ALL_SPACING_STEPS, ALL_TYPE_STYLES, ALL_WEIGHTS, FOCUS_HALO_OPACITY,
    FOCUS_HALO_RADIUS_PX, FOCUS_HALO_SPREAD_PX, KEYLINE_EMPHASIS_PX, KEYLINE_RESTING_PX,
    MIN_INTERACTIVE_TARGET_PX,
};
pub use typeface::{AuthoredTypeface, TypefaceError, AUTHORED_FAMILY};
