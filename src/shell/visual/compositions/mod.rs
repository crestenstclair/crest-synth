//! The reusable shell composition family.
//!
//! **Components paint. Views compose. The reducer decides.**
//!
//! A composition fills one named shell region by arranging primitives and
//! controls. It owns no value of its own: no color, no type style, no spacing
//! step, no geometry — those resolve through
//! [`SemanticVisualToken`](crate::shell::visual::token) and
//! [`ViewportDensityPolicy`] — and no Patch value, focus, navigation, reducer
//! state, or audio state either. It returns typed intent aggregated from the
//! controls it arranged, and dispatches nothing.
//!
//! # The region binding
//!
//! Each composition declares the [`ShellRegion`] it fills, and the five region
//! names `ShellFrameObservation` already emits — `contextLine`,
//! `identityHeader`, `mainWorkspace`, `persistentSideRegion`, `footer` — are
//! each claimed by at least one composition. That correspondence is asserted
//! here rather than assumed, because the production window's observation is
//! constructed from the rectangles those regions actually paint: if a region
//! name stopped matching a composition, the observation would keep reporting a
//! region nothing produces.
//!
//! [`ShellComposition::ApplicationShell`] is the one composition that fills no
//! observation region. It is the frame the other seven sit inside, so it binds to
//! [`ShellRegion::WholeFrame`] — declared explicitly rather than left as a
//! special case, so the mapping stays total.
//!
//! # What a composition is handed
//!
//! The whole immutable [`GraphicalShellProjection`], and the density policy.
//! Each composition narrows to the slice it arranges; narrowing is the
//! composition's business, and handing it the projection root rather than a
//! pre-sliced value is what keeps the adapter from deciding what a region is
//! allowed to see. Nothing here receives a mutable projection or an
//! application-state reference — there is no argument through which one could
//! arrive.
//!
//! Realizes `valueObject.Shell.ShellComposition` and
//! `requirement.reusable_shell_compositions`.

pub mod application_shell;
pub mod context_switch;
pub mod footer;
pub mod identity_header;
pub mod mixer_strip_bank;
pub mod patch_strip_row;
pub mod section;
pub mod utility_inspector_panel;

use eframe::egui::Ui;
use serde::Serialize;

use crate::control::{FocusPath, GraphicalShellProjection};
use crate::shell::visual::controls::ControlIntent;
use crate::shell::visual::density::ViewportDensityPolicy;

/// A named region of the shell.
///
/// The five that `ShellFrameObservation` emits, plus the whole frame that
/// contains them. The set is closed: a new region is a change to the structural
/// bands, which is a design decision authored in `DESIGN.md` and reflected in
/// the observation, never a layout a composition invents.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShellRegion {
    /// The whole viewport — every band, and the composition that arranges them.
    /// This is not an observation region; it is what contains all of them.
    WholeFrame,
    /// Product · context · status.
    ContextLine,
    /// Identity and header.
    IdentityHeader,
    /// The main surface half of the workspace band.
    MainWorkspace,
    /// The always-visible Utility/Inspector half of the workspace band.
    PersistentSideRegion,
    /// Current path and valid control hints.
    Footer,
}

/// The region names `ShellFrameObservation` emits, in the order it emits them.
///
/// Spelled exactly as the observation spells them, because the correspondence
/// between a composition and the rectangle reported for it is what makes the
/// observation evidence rather than decoration.
pub const OBSERVED_REGION_NAMES: [&str; 5] = [
    "contextLine",
    "identityHeader",
    "mainWorkspace",
    "persistentSideRegion",
    "footer",
];

impl ShellRegion {
    /// The name `ShellFrameObservation` reports this region under, if it
    /// reports it at all.
    ///
    /// [`ShellRegion::WholeFrame`] returns `None`: the frame is not observed as
    /// a region, it is the thing the observed regions tile.
    pub const fn observation_name(self) -> Option<&'static str> {
        match self {
            Self::WholeFrame => None,
            Self::ContextLine => Some("contextLine"),
            Self::IdentityHeader => Some("identityHeader"),
            Self::MainWorkspace => Some("mainWorkspace"),
            Self::PersistentSideRegion => Some("persistentSideRegion"),
            Self::Footer => Some("footer"),
        }
    }
}

/// The closed family of reusable compositions.
///
/// Each fills one shell region by arranging primitives and controls. The set is
/// closed at eight, and adding one is a compile error at
/// [`ShellComposition::region`] rather than a composition bound to no region.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShellComposition {
    /// The four structural bands and the workspace split that together tile the
    /// viewport.
    ApplicationShell,
    /// The PATCH / MIXER context indicator and status.
    ContextSwitch,
    /// The identity and header band.
    IdentityHeader,
    /// A titled group of rows within a surface.
    Section,
    /// One row of the PATCH strip.
    PatchStripRow,
    /// The sixteen fixed mixer track columns, side by side.
    MixerStripBank,
    /// The always-visible Utility/Inspector panel.
    UtilityInspectorPanel,
    /// The current path and the valid control hints.
    Footer,
}

/// Every declared composition, in declaration order.
pub const ALL_SHELL_COMPOSITIONS: [ShellComposition; SHELL_COMPOSITION_COUNT] = [
    ShellComposition::ApplicationShell,
    ShellComposition::ContextSwitch,
    ShellComposition::IdentityHeader,
    ShellComposition::Section,
    ShellComposition::PatchStripRow,
    ShellComposition::MixerStripBank,
    ShellComposition::UtilityInspectorPanel,
    ShellComposition::Footer,
];

/// How many compositions the family declares.
pub const SHELL_COMPOSITION_COUNT: usize = 8;

impl ShellComposition {
    /// Returns the canonical name of this composition.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::ApplicationShell => "ApplicationShell",
            Self::ContextSwitch => "ContextSwitch",
            Self::IdentityHeader => "IdentityHeader",
            Self::Section => "Section",
            Self::PatchStripRow => "PatchStripRow",
            Self::MixerStripBank => "MixerStripBank",
            Self::UtilityInspectorPanel => "UtilityInspectorPanel",
            Self::Footer => "Footer",
        }
    }

    /// The shell region this composition fills.
    ///
    /// `Section`, `PatchStripRow`, and `MixerStripBank` all fill the main
    /// workspace: a section groups rows, a strip row is one of them, and the
    /// bank arranges the mixer's sixteen track columns across it. All three are
    /// main-surface structure. The binding is many-to-one by design and remains
    /// so — what must hold is that every observed region has a composition, not
    /// that every composition has a region to itself.
    pub const fn region(self) -> ShellRegion {
        match self {
            Self::ApplicationShell => ShellRegion::WholeFrame,
            Self::ContextSwitch => ShellRegion::ContextLine,
            Self::IdentityHeader => ShellRegion::IdentityHeader,
            Self::Section | Self::PatchStripRow | Self::MixerStripBank => {
                ShellRegion::MainWorkspace
            }
            Self::UtilityInspectorPanel => ShellRegion::PersistentSideRegion,
            Self::Footer => ShellRegion::Footer,
        }
    }
}

/// One control's request, carried with the path that identifies which control
/// made it.
///
/// The intent alone is not actionable — "increase by one fine step" means
/// nothing without knowing what asked — so a composition pairs each intent with
/// the focus path of the view data it handed the control. The path is identity,
/// not state: the composition read it from the immutable projection and passes
/// it back unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlRequest {
    path: FocusPath,
    intent: ControlIntent,
}

impl ControlRequest {
    /// Pairs an intent with the control that asked.
    pub const fn new(path: FocusPath, intent: ControlIntent) -> Self {
        Self { path, intent }
    }

    /// Which control asked.
    pub const fn path(&self) -> &FocusPath {
        &self.path
    }

    /// What it asked for.
    pub const fn intent(&self) -> ControlIntent {
        self.intent
    }
}

/// What a composition asks for, aggregated from the controls it arranged.
///
/// A composition collects rather than decides: it does not rank competing
/// requests, resolve them against what is currently valid, or turn any of them
/// into an action. All of that needs focus and the reducer, which live on the
/// other side of this boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompositionIntent {
    requests: Vec<ControlRequest>,
}

impl CompositionIntent {
    /// A composition that asked for nothing.
    pub const fn none() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// Records what one arranged control asked for, dropping the empty asks so
    /// that a frame in which nothing happened aggregates to nothing.
    pub fn record(&mut self, path: &FocusPath, intent: ControlIntent) {
        if intent != ControlIntent::None {
            self.requests
                .push(ControlRequest::new(path.clone(), intent));
        }
    }

    /// Folds a nested composition's requests into this one.
    pub fn absorb(&mut self, other: Self) {
        self.requests.extend(other.requests);
    }

    /// Everything the arranged controls asked for, in arrangement order.
    pub fn requests(&self) -> &[ControlRequest] {
        &self.requests
    }

    /// Whether anything was asked for at all.
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
}

/// The signature every composition in the family satisfies.
///
/// A composition arranges. It reads only the immutable projection it is handed,
/// resolves every extent through the density policy, and returns aggregated
/// intent. It owns nothing, caches nothing, dispatches nothing, and never
/// reaches `AppState`.
///
/// Declared as a function type rather than as prose so that each composition's
/// `render` is coerced to it in [`ShellComposition::renderer`], and a
/// composition whose signature drifts fails to compile.
pub type CompositionRenderFn =
    fn(&mut Ui, &GraphicalShellProjection, &ViewportDensityPolicy) -> CompositionIntent;

impl ShellComposition {
    /// The function that arranges this composition.
    ///
    /// Exhaustive, so adding a composition is a compile error here rather than
    /// a variant nothing can paint. Every arm now resolves to its own module's
    /// `arrange`; the shared do-nothing stub WP01 scaffolded was deleted once
    /// the last composition landed, since an unreachable stub is the dead code
    /// the closed family exists to make impossible.
    const fn renderer(self) -> CompositionRenderFn {
        match self {
            // WP04 — the frame and its bands.
            Self::ApplicationShell => application_shell::render,
            Self::ContextSwitch => context_switch::render,
            Self::IdentityHeader => identity_header::render,
            Self::Footer => footer::render,
            // WP05 — the workspace contents.
            Self::Section => section::render,
            Self::PatchStripRow => patch_strip_row::render,
            Self::UtilityInspectorPanel => utility_inspector_panel::render,
            // WP09 — the eighth composition, which arranges groups where the
            // section arranges controls.
            Self::MixerStripBank => mixer_strip_bank::render,
        }
    }

    /// Arranges this composition's region and returns what its controls asked
    /// for.
    pub fn render(
        self,
        ui: &mut Ui,
        projection: &GraphicalShellProjection,
        density: &ViewportDensityPolicy,
    ) -> CompositionIntent {
        self.renderer()(ui, projection, density)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_composition_family_holds_exactly_eight_compositions() {
        assert_eq!(SHELL_COMPOSITION_COUNT, 8);
        assert_eq!(ALL_SHELL_COMPOSITIONS.len(), SHELL_COMPOSITION_COUNT);
    }

    /// Adding a composition without adding it to [`ALL_SHELL_COMPOSITIONS`]
    /// fails here: the `match` is exhaustive, so the new composition must be
    /// named, and naming it makes the iteration check fail until the array is
    /// updated.
    #[test]
    fn iteration_yields_every_declared_composition() {
        for composition in ALL_SHELL_COMPOSITIONS {
            let named = match composition {
                ShellComposition::ApplicationShell => "ApplicationShell",
                ShellComposition::ContextSwitch => "ContextSwitch",
                ShellComposition::IdentityHeader => "IdentityHeader",
                ShellComposition::Section => "Section",
                ShellComposition::PatchStripRow => "PatchStripRow",
                ShellComposition::MixerStripBank => "MixerStripBank",
                ShellComposition::UtilityInspectorPanel => "UtilityInspectorPanel",
                ShellComposition::Footer => "Footer",
            };
            assert_eq!(named, composition.canonical_name());
        }
        let seen: HashSet<_> = ALL_SHELL_COMPOSITIONS.into_iter().collect();
        assert_eq!(
            seen.len(),
            SHELL_COMPOSITION_COUNT,
            "a composition appears twice in ALL_SHELL_COMPOSITIONS"
        );
    }

    /// Every region name the observation emits is filled by a composition.
    ///
    /// WP06 replaces the adapter's painting with these compositions while the
    /// observation keeps its current shape; if a region name here stopped
    /// matching one, the observation would report a rectangle nothing produced.
    #[test]
    fn every_observed_region_is_filled_by_a_composition() {
        let filled: HashSet<&str> = ALL_SHELL_COMPOSITIONS
            .into_iter()
            .filter_map(|composition| composition.region().observation_name())
            .collect();
        for name in OBSERVED_REGION_NAMES {
            assert!(
                filled.contains(name),
                "no composition fills the {name} region"
            );
        }
        assert_eq!(
            filled.len(),
            OBSERVED_REGION_NAMES.len(),
            "a composition fills a region the observation does not report"
        );
    }

    /// The application shell is the only composition that fills no observed
    /// region, and every other composition fills one.
    #[test]
    fn only_the_application_shell_fills_the_whole_frame() {
        for composition in ALL_SHELL_COMPOSITIONS {
            let whole_frame = composition.region() == ShellRegion::WholeFrame;
            assert_eq!(
                whole_frame,
                composition == ShellComposition::ApplicationShell,
                "{} and the whole frame",
                composition.canonical_name()
            );
            assert_eq!(
                composition.region().observation_name().is_none(),
                whole_frame,
                "{} region reporting",
                composition.canonical_name()
            );
        }
    }

    #[test]
    fn an_empty_composition_intent_records_nothing() {
        let intent = CompositionIntent::none();
        assert!(intent.is_empty());
        assert!(intent.requests().is_empty());
        assert_eq!(intent, CompositionIntent::default());
    }

    /// The composition family's own sources, including the leaf modules later
    /// work packages add.
    fn composition_sources() -> Vec<std::path::PathBuf> {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell/visual/compositions");
        let mut sources: Vec<std::path::PathBuf> = std::fs::read_dir(&directory)
            .expect("the compositions module directory")
            .map(|entry| entry.expect("directory entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
            .collect();
        sources.sort();
        sources
    }

    /// Strips line comments, so prose that names the boundary is not mistaken
    /// for code that crosses it.
    fn code_only(source: &str) -> String {
        source
            .lines()
            .map(|line| match line.find("//") {
                Some(offset) => &line[..offset],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A scan that reads nothing passes vacuously.
    #[test]
    fn the_scan_reads_the_composition_sources() {
        let sources = composition_sources();
        assert!(
            sources
                .iter()
                .any(|path| path.file_name().is_some_and(|name| name == "mod.rs")),
            "the composition scan did not find the module root"
        );
        for path in &sources {
            assert!(
                !std::fs::read_to_string(path)
                    .expect("composition source")
                    .is_empty(),
                "{} is empty",
                path.display()
            );
        }
    }

    /// No composition names a semantic action. It aggregates intent and returns
    /// it; deciding what that becomes is the caller's job. The needle is
    /// assembled at runtime so this test does not find itself.
    #[test]
    fn no_composition_source_names_a_semantic_action() {
        let forbidden = format!("{}{}", "Semantic", "Action");
        for path in composition_sources() {
            let source = std::fs::read_to_string(&path).expect("composition source");
            assert!(
                !code_only(&source).contains(&forbidden),
                "{} names {forbidden}, which a composition must never build or dispatch",
                path.display()
            );
        }
    }

    /// No `match` in the family falls through a wildcard, so adding a
    /// composition or a region names every site that must change.
    #[test]
    fn no_match_arm_in_the_composition_family_falls_through_a_wildcard() {
        let wildcard_arm = format!("{}{}", "_ =", ">");
        let binding_arm = format!("{}{}", ".. =", ">");
        for path in composition_sources() {
            let source = code_only(&std::fs::read_to_string(&path).expect("composition source"));
            assert!(
                !source.contains(&wildcard_arm),
                "{} has a wildcard match arm",
                path.display()
            );
            assert!(
                !source.contains(&binding_arm),
                "{} has a catch-all match arm",
                path.display()
            );
        }
    }
}
