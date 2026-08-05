//! The authored visual vocabulary every rendered surface resolves through.
//!
//! Colors, type styles, spacing steps, and geometry are declared once in
//! [`token`]; the typeface that makes those styles renderable is installed by
//! [`typeface`]. Density policies, the component state vocabulary, and the
//! reusable primitives complete the painting vocabulary.
//!
//! On top of that vocabulary sit the two component families: [`controls`], the
//! configurable controls a surface asks for by control kind and presentation
//! role, and [`compositions`], the reusable arrangements that fill the named
//! shell regions. Surfaces assemble from these two rather than re-deriving how
//! a parameter is drawn or how a band is laid out.
//!
//! Raw values stay private to their declaring module. Nothing outside this
//! module constructs a color, a type size, or a spacing constant.
//!
//! The re-export surface below is deliberately narrow: the families, the roles,
//! the states, and the intents, and no internal helper of any of them. A wide
//! surface is how a literal escapes the module.

pub mod compositions;
pub mod controls;
pub mod density; // WP02
pub mod primitives; // WP03
pub mod state; // WP02
pub mod token;
pub mod typeface;

pub use compositions::{
    CompositionIntent, CompositionRenderFn, ControlRequest, ShellComposition, ShellRegion,
    ALL_SHELL_COMPOSITIONS, OBSERVED_REGION_NAMES, SHELL_COMPOSITION_COUNT,
};
pub use controls::{
    control_for, AdjustDirection, AdjustGranularity, ComponentControl, ControlIntent,
    ControlRenderFn, ControlSelection, PresentationRole, ALL_COMPONENT_CONTROLS,
    ALL_PRESENTATION_ROLES, ALL_SEMANTIC_CONTROL_KINDS, COMPONENT_CONTROL_COUNT,
    PRESENTATION_ROLE_COUNT, SEMANTIC_CONTROL_KIND_COUNT,
};
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
