//! The configurable control family.
//!
//! **Components paint. Views compose. The reducer decides.**
//!
//! A control here is handed immutable view data, one explicit
//! [`ComponentState`], and one [`PresentationRole`], and it paints. It owns,
//! caches, and derives no Patch value, focus, navigation, reducer state, or
//! audio state, and it never reaches `AppState`. It returns a
//! [`ControlIntent`] — what the operator *asked for* — and dispatches nothing.
//! Mapping an intent onto a semantic action is the caller's job and happens
//! outside this module, which is why no type here is convertible into one.
//!
//! # Why a role, and not just a kind
//!
//! `SemanticControlKind` has seven values and the product names eight control
//! shapes, so kind cannot be the selector: a continuous parameter is a
//! labelled row on the PATCH surface and a fader in a mixer strip. The missing
//! half is [`PresentationRole`], and it is *supplied by the requesting
//! composition* — never inferred from the value, the surface identity, or the
//! viewport. A control does not decide what it is.
//!
//! Selection is therefore total over the pair, and [`control_for`] is written
//! as a single `match` on a tuple so that adding a kind *or* a role is a
//! compile error naming this function. A nested match with a `_ =>` arm would
//! compile, pass every test, and silently answer for a pair nobody ever
//! considered — which is the exact drift the closed unions exist to catch. A
//! pair that is genuinely not askable resolves to
//! [`ControlSelection::NotAskableInRole`], which is a decision; falling through
//! to a generic label-and-value row is not.
//!
//! Realizes `valueObject.Shell.ComponentControl` and
//! `requirement.configurable_control_family`.

pub mod browser_row;
pub mod choice_row;
pub mod compact_slider;
pub mod fader;
pub mod meter;
pub mod modal_option;
pub mod parameter_row;
pub mod toggle;

use eframe::egui::Ui;
use serde::Serialize;

use crate::control::{SemanticControlKind, SemanticControlViewModel};
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::state::{ComponentState, ALL_COMPONENT_STATES, COMPONENT_STATE_COUNT};

/// The role a requesting composition asks a control in.
///
/// The same control kind is a parameter row on a listed surface and a fader in
/// a mixer strip, so kind alone cannot select a shape. The set is closed at
/// four: a fifth role is a design decision about a surface the product does not
/// yet name, and adding one fails compilation at [`control_for`] rather than
/// quietly reusing an existing role's shapes.
///
/// The roles are the surfaces `DESIGN.md` already declares, not an invented
/// taxonomy — the PATCH strip and its rows (`DESIGN.md:454`), the sixteen
/// compact mixer columns (`:462`), the persistent Utility/Inspector (`:444`,
/// `:466`), and the nested option modal with trapped focus (`:458`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PresentationRole {
    /// PATCH main surface, MIXER inspector, Utility panel — stacked labelled
    /// rows that occupy the full width they are given.
    ListedRow,
    /// MIXER track columns — the sixteen fixed compact columns with hairline
    /// separators.
    VerticalStrip,
    /// Utility/Inspector panel entries that are not full rows.
    PanelEntry,
    /// Focus-trapped option modals and the later Sample Browser.
    ModalEntry,
}

/// Every declared role, in declaration order.
pub const ALL_PRESENTATION_ROLES: [PresentationRole; PRESENTATION_ROLE_COUNT] = [
    PresentationRole::ListedRow,
    PresentationRole::VerticalStrip,
    PresentationRole::PanelEntry,
    PresentationRole::ModalEntry,
];

/// How many roles the vocabulary declares.
pub const PRESENTATION_ROLE_COUNT: usize = 4;

impl PresentationRole {
    /// Returns the canonical name of this role.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::ListedRow => "ListedRow",
            Self::VerticalStrip => "VerticalStrip",
            Self::PanelEntry => "PanelEntry",
            Self::ModalEntry => "ModalEntry",
        }
    }
}

/// The closed family of configurable controls.
///
/// Each presents one `SemanticControlViewModel` in one [`ComponentState`] and
/// returns typed intent rather than acting. The set is closed at eight, and
/// every one of the eight is reachable from [`control_for`] by at least one
/// kind × role pair — a control nothing can ask for is dead code that would
/// otherwise pass every other check in this module.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ComponentControl {
    /// A labelled row carrying a numeric value and its unit.
    ParameterRow,
    /// A labelled row carrying one selected option from a set.
    ChoiceRow,
    /// A labelled row carrying a binary value.
    Toggle,
    /// A short horizontal slider that is not a full row.
    CompactSlider,
    /// The tall vertical level control of a mixer strip.
    Fader,
    /// A read-only level readout in a mixer strip.
    Meter,
    /// A row naming an asset, which opens a browser rather than adjusting.
    BrowserRow,
    /// One selectable entry inside a focus-trapped option modal.
    ModalOption,
}

/// Every declared control, in declaration order.
pub const ALL_COMPONENT_CONTROLS: [ComponentControl; COMPONENT_CONTROL_COUNT] = [
    ComponentControl::ParameterRow,
    ComponentControl::ChoiceRow,
    ComponentControl::Toggle,
    ComponentControl::CompactSlider,
    ComponentControl::Fader,
    ComponentControl::Meter,
    ComponentControl::BrowserRow,
    ComponentControl::ModalOption,
];

/// How many controls the family declares.
pub const COMPONENT_CONTROL_COUNT: usize = 8;

/// The states a control living in a mixer track column can be handed.
///
/// This is every declared state. `Muted` and `Soloed` describe the track a
/// strip represents, so only the controls that represent a track can receive
/// them.
const MIXER_STRIP_STATES: [ComponentState; COMPONENT_STATE_COUNT] = ALL_COMPONENT_STATES;

/// The states a control outside a mixer track column can be handed.
///
/// Every declared state except `Muted` and `Soloed`. Mute and solo are
/// mixer-track concepts: a Utility panel row has no track to be muted, and a
/// control that declares them would owe the gallery a specimen it cannot
/// meaningfully paint.
///
/// A mixer strip's own mute and solo controls are deliberately *not* an
/// exception. Their on-ness is a value they already paint from their view data
/// (`SemanticControlValue::Parameter(ParameterValue::Toggle)`); handing them
/// `Muted` as a state as well would give one fact two representations that can
/// disagree.
const NON_TRACK_STATES: [ComponentState; 7] = [
    ComponentState::Resting,
    ComponentState::Focused,
    ComponentState::Adjusting,
    ComponentState::Disabled,
    ComponentState::Loading,
    ComponentState::Error,
    ComponentState::Selected,
];

impl ComponentControl {
    /// Returns the canonical name of this control.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::ParameterRow => "ParameterRow",
            Self::ChoiceRow => "ChoiceRow",
            Self::Toggle => "Toggle",
            Self::CompactSlider => "CompactSlider",
            Self::Fader => "Fader",
            Self::Meter => "Meter",
            Self::BrowserRow => "BrowserRow",
            Self::ModalOption => "ModalOption",
        }
    }

    /// The states this control can be handed.
    ///
    /// A control that can never be muted or soloed declares that here rather
    /// than silently omitting a specimen from the gallery. Every control
    /// declares at least `Resting`, `Focused`, and `Disabled`, and the union
    /// across the family covers all nine declared states, so no state is
    /// applicable to nothing.
    pub const fn applicable_states(self) -> &'static [ComponentState] {
        match self {
            Self::Fader | Self::Meter => &MIXER_STRIP_STATES,
            Self::ParameterRow
            | Self::ChoiceRow
            | Self::Toggle
            | Self::CompactSlider
            | Self::BrowserRow
            | Self::ModalOption => &NON_TRACK_STATES,
        }
    }

    /// Whether this control can be handed `state`.
    pub fn accepts(self, state: ComponentState) -> bool {
        self.applicable_states().contains(&state)
    }
}

/// Every declared semantic control kind, in the order
/// `crate::control::SemanticControlKind` declares them.
///
/// The kind vocabulary is reducer-adjacent and owned elsewhere; this module
/// only enumerates it so selection can be shown total over it. Adding a kind
/// there fails compilation here — in [`control_for`] and in this module's
/// iteration test — rather than leaving a pair nobody selects for.
pub const ALL_SEMANTIC_CONTROL_KINDS: [SemanticControlKind; SEMANTIC_CONTROL_KIND_COUNT] = [
    SemanticControlKind::Continuous,
    SemanticControlKind::Stepped,
    SemanticControlKind::Choice,
    SemanticControlKind::Toggle,
    SemanticControlKind::Asset,
    SemanticControlKind::Identity,
    SemanticControlKind::Surface,
];

/// How many semantic control kinds selection covers.
pub const SEMANTIC_CONTROL_KIND_COUNT: usize = 7;

/// What one `(SemanticControlKind, PresentationRole)` pair resolves to.
///
/// The second variant is the point of the type. A pair that no composition can
/// ask — a choice in a mixer track column, say — resolves to an explicit
/// refusal rather than to a generic row, so "we decided this cannot be asked"
/// and "we never thought about it" stay distinguishable.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ControlSelection {
    /// The pair resolves to exactly this control.
    Control(ComponentControl),
    /// This kind is not askable in this role.
    NotAskableInRole,
}

impl ControlSelection {
    /// The control this selection resolves to, if the pair is askable.
    pub const fn control(self) -> Option<ComponentControl> {
        match self {
            Self::Control(control) => Some(control),
            Self::NotAskableInRole => None,
        }
    }

    /// Whether a composition may ask this pair.
    pub const fn is_askable(self) -> bool {
        matches!(self, Self::Control(_))
    }
}

/// Resolves a control kind and the role it is asked in to exactly one control.
///
/// This is a single `match` on the tuple, deliberately. Rust checks tuple-match
/// exhaustiveness, so adding a `SemanticControlKind` or a [`PresentationRole`]
/// is a compile error naming this function. Nested matches with a `_ =>` arm
/// would not be, and a defaulted mapping is behaviorally indistinguishable from
/// a considered one.
pub const fn control_for(kind: SemanticControlKind, role: PresentationRole) -> ControlSelection {
    use ComponentControl as Control;
    use ControlSelection::{Control as Asks, NotAskableInRole};
    use PresentationRole as Role;
    use SemanticControlKind as Kind;

    match (kind, role) {
        // A numeric value. Stepped differs from Continuous only in how it
        // moves, which is the control's adjustment behaviour and not its
        // shape, so the two select identically.
        (Kind::Continuous | Kind::Stepped, Role::ListedRow) => Asks(Control::ParameterRow),
        (Kind::Continuous | Kind::Stepped, Role::VerticalStrip) => Asks(Control::Fader),
        (Kind::Continuous | Kind::Stepped, Role::PanelEntry) => Asks(Control::CompactSlider),
        (Kind::Continuous | Kind::Stepped, Role::ModalEntry) => Asks(Control::ModalOption),

        // One selected option from a set. A mixer column carries a level, a
        // pan, and the two track toggles — never a choice.
        (Kind::Choice, Role::ListedRow | Role::PanelEntry) => Asks(Control::ChoiceRow),
        (Kind::Choice, Role::VerticalStrip) => NotAskableInRole,
        (Kind::Choice, Role::ModalEntry) => Asks(Control::ModalOption),

        // A binary value. The mixer column's mute and solo are toggles in a
        // strip, so this is the one kind every listed role shares a shape for.
        (Kind::Toggle, Role::ListedRow | Role::VerticalStrip | Role::PanelEntry) => {
            Asks(Control::Toggle)
        }
        (Kind::Toggle, Role::ModalEntry) => Asks(Control::ModalOption),

        // A named asset. It opens a browser wherever it is asked, including
        // inside a modal, so the browser row is its shape in three roles and
        // is not askable in a mixer column.
        (Kind::Asset, Role::ListedRow | Role::PanelEntry | Role::ModalEntry) => {
            Asks(Control::BrowserRow)
        }
        (Kind::Asset, Role::VerticalStrip) => NotAskableInRole,

        // A read-only value. In a listed row or a panel it reads as a labelled
        // row; in a mixer column a read-only level *is* the meter. This pair is
        // the whole argument for selecting on the role: one kind, two shapes,
        // decided by where it was asked and not by what it holds.
        (Kind::Identity, Role::ListedRow | Role::PanelEntry) => Asks(Control::ParameterRow),
        (Kind::Identity, Role::VerticalStrip) => Asks(Control::Meter),
        (Kind::Identity, Role::ModalEntry) => Asks(Control::ModalOption),

        // A surface summary — the read-only root the Utility and Inspector
        // panels carry (`DESIGN.md:466`). It is a row, and a mixer column has
        // no surface to summarize.
        (Kind::Surface, Role::ListedRow | Role::PanelEntry) => Asks(Control::ParameterRow),
        (Kind::Surface, Role::VerticalStrip) => NotAskableInRole,
        (Kind::Surface, Role::ModalEntry) => Asks(Control::ModalOption),
    }
}

/// Which way an adjustment was asked to move.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AdjustDirection {
    /// Toward the minimum, or the previous option.
    Decrease,
    /// Toward the maximum, or the next option.
    Increase,
}

/// How far one adjustment step moves.
///
/// Which number that is belongs to the parameter descriptor the view data
/// carries, not to the control: a control reports that a fine or a coarse step
/// was asked for and never resolves it to a value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AdjustGranularity {
    /// One fine step.
    Fine,
    /// One coarse step.
    Coarse,
}

/// What a control asks for.
///
/// This is a request, never a decision. It carries no `SemanticAction`, and
/// nothing in this module turns it into one: mapping an intent onto an action
/// requires focus, the active context, and the reducer's notion of what is
/// valid right now, all of which live on the other side of this boundary. A
/// control that could build an action could dispatch one, and then the loop
/// physical input → semantic action → `AppState::apply` → projection would no
/// longer be true of the render path.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ControlIntent {
    /// The control asks for nothing. Painting produced no request.
    None,
    /// The operator addressed this control and it asks to become the focus
    /// target. It does not know whether that is allowed.
    FocusRequested,
    /// The operator asked to move this control's value one step.
    AdjustRequested {
        /// Which way to move.
        direction: AdjustDirection,
        /// How far one step is.
        granularity: AdjustGranularity,
    },
    /// The operator asked to see this control's options — the nested modal, or
    /// the asset browser.
    OptionListRequested,
    /// The operator asked to commit this control — flip a toggle, choose a
    /// modal entry.
    ActivateRequested,
}

/// The signature every control in the family satisfies.
///
/// A control paints. It reads only what it is handed, and it returns intent.
/// It owns nothing, caches nothing, dispatches nothing, resolves no Patch
/// value, and never reaches `AppState`.
///
/// The density policy is handed in rather than looked up because
/// `ViewportDensityPolicy` is the single place a raw viewport width is
/// consumed; a control that read a viewport size would be deciding its own
/// layout from a number the vocabulary exists to own. For the same reason no
/// control receives a mutable projection or an application-state reference —
/// there is no argument here through which one could arrive.
///
/// Declaring this as a function type rather than as prose is what makes it
/// binding: each control's `render` is coerced to it in [`ComponentControl::renderer`],
/// so a control whose signature drifts fails to compile rather than diverging
/// quietly across eight parallel work streams.
pub type ControlRenderFn = fn(
    &mut Ui,
    &SemanticControlViewModel,
    ComponentState,
    PresentationRole,
    &ViewportDensityPolicy,
) -> ControlIntent;

impl ComponentControl {
    /// The function that paints this control.
    ///
    /// Exhaustive, so adding a control to the family is a compile error here
    /// rather than a variant nothing can paint. Every arm now resolves to its
    /// leaf module's `render`; the shared do-nothing stub WP01 scaffolded was
    /// deleted once WP02 and WP03 landed, since an unreachable stub is the
    /// dead code the family exists to make impossible.
    const fn renderer(self) -> ControlRenderFn {
        match self {
            // WP02 — listed rows.
            Self::ParameterRow => parameter_row::render,
            Self::ChoiceRow => choice_row::render,
            Self::Toggle => toggle::render,
            Self::BrowserRow => browser_row::render,
            // WP03 — sliders, faders, meters, and modal entries.
            Self::CompactSlider => compact_slider::render,
            Self::Fader => fader::render,
            Self::Meter => meter::render,
            Self::ModalOption => modal_option::render,
        }
    }

    /// Paints this control and returns what it asks for.
    ///
    /// `state` and `role` are handed in. The control derives neither: a control
    /// that inferred its own state would own a second copy of something the
    /// reducer already decided, and one that inferred its own role would decide
    /// what it is, which is the confusion [`PresentationRole`] exists to end.
    pub fn render(
        self,
        ui: &mut Ui,
        view: &SemanticControlViewModel,
        state: ComponentState,
        role: PresentationRole,
        density: &ViewportDensityPolicy,
    ) -> ControlIntent {
        debug_assert!(
            self.accepts(state),
            "{} was handed {}, which it does not declare applicable",
            self.canonical_name(),
            state.canonical_name()
        );
        self.renderer()(ui, view, state, role, density)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The pairs the model declares un-askable, and the only ones.
    ///
    /// Pinned as data rather than left implicit so that a later work package
    /// cannot quietly switch a pair off: turning an askable pair into a refusal
    /// removes a control from a surface, which is a product change, and it
    /// fails here.
    ///
    /// All three are mixer track columns. A column carries a level, a pan, and
    /// the two track toggles (`DESIGN.md:462-465`); it never carries a choice,
    /// an asset, or a surface summary.
    const NOT_ASKABLE_PAIRS: [(SemanticControlKind, PresentationRole); 3] = [
        (SemanticControlKind::Choice, PresentationRole::VerticalStrip),
        (SemanticControlKind::Asset, PresentationRole::VerticalStrip),
        (
            SemanticControlKind::Surface,
            PresentationRole::VerticalStrip,
        ),
    ];

    fn is_declared_not_askable(kind: SemanticControlKind, role: PresentationRole) -> bool {
        NOT_ASKABLE_PAIRS
            .iter()
            .any(|(declared_kind, declared_role)| *declared_kind == kind && *declared_role == role)
    }

    /// Every `(kind, role)` pair, in declaration order.
    fn every_pair() -> Vec<(SemanticControlKind, PresentationRole)> {
        ALL_SEMANTIC_CONTROL_KINDS
            .into_iter()
            .flat_map(|kind| {
                ALL_PRESENTATION_ROLES
                    .into_iter()
                    .map(move |role| (kind, role))
            })
            .collect()
    }

    fn kind_name(kind: SemanticControlKind) -> &'static str {
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

    #[test]
    fn the_role_vocabulary_holds_exactly_four_roles() {
        assert_eq!(PRESENTATION_ROLE_COUNT, 4);
        assert_eq!(ALL_PRESENTATION_ROLES.len(), PRESENTATION_ROLE_COUNT);
    }

    /// Adding a role without adding it to [`ALL_PRESENTATION_ROLES`] fails
    /// here: the `match` is exhaustive, so the new role must be named, and
    /// naming it makes the iteration check fail until the array is updated.
    #[test]
    fn iteration_yields_every_declared_role() {
        for role in ALL_PRESENTATION_ROLES {
            let named = match role {
                PresentationRole::ListedRow => "ListedRow",
                PresentationRole::VerticalStrip => "VerticalStrip",
                PresentationRole::PanelEntry => "PanelEntry",
                PresentationRole::ModalEntry => "ModalEntry",
            };
            assert_eq!(named, role.canonical_name());
        }
        let seen: HashSet<_> = ALL_PRESENTATION_ROLES.into_iter().collect();
        assert_eq!(
            seen.len(),
            PRESENTATION_ROLE_COUNT,
            "a role appears twice in ALL_PRESENTATION_ROLES"
        );
    }

    #[test]
    fn the_control_family_holds_exactly_eight_controls() {
        assert_eq!(COMPONENT_CONTROL_COUNT, 8);
        assert_eq!(ALL_COMPONENT_CONTROLS.len(), COMPONENT_CONTROL_COUNT);
    }

    #[test]
    fn iteration_yields_every_declared_control() {
        for control in ALL_COMPONENT_CONTROLS {
            let named = match control {
                ComponentControl::ParameterRow => "ParameterRow",
                ComponentControl::ChoiceRow => "ChoiceRow",
                ComponentControl::Toggle => "Toggle",
                ComponentControl::CompactSlider => "CompactSlider",
                ComponentControl::Fader => "Fader",
                ComponentControl::Meter => "Meter",
                ComponentControl::BrowserRow => "BrowserRow",
                ComponentControl::ModalOption => "ModalOption",
            };
            assert_eq!(named, control.canonical_name());
        }
        let seen: HashSet<_> = ALL_COMPONENT_CONTROLS.into_iter().collect();
        assert_eq!(
            seen.len(),
            COMPONENT_CONTROL_COUNT,
            "a control appears twice in ALL_COMPONENT_CONTROLS"
        );
    }

    /// The kind vocabulary is owned outside this module, so its enumeration
    /// here is checked the same way: the `match` in [`kind_name`] is
    /// exhaustive, so an added kind must be named, and naming it fails this
    /// count until [`ALL_SEMANTIC_CONTROL_KINDS`] is updated too.
    #[test]
    fn iteration_yields_every_declared_control_kind() {
        assert_eq!(SEMANTIC_CONTROL_KIND_COUNT, 7);
        assert_eq!(
            ALL_SEMANTIC_CONTROL_KINDS.len(),
            SEMANTIC_CONTROL_KIND_COUNT
        );
        let names: HashSet<&str> = ALL_SEMANTIC_CONTROL_KINDS
            .into_iter()
            .map(kind_name)
            .collect();
        assert_eq!(
            names.len(),
            SEMANTIC_CONTROL_KIND_COUNT,
            "a kind appears twice in ALL_SEMANTIC_CONTROL_KINDS"
        );
    }

    /// Selection is total over kind × role.
    ///
    /// Every declared pair resolves, every pair not declared un-askable
    /// resolves to exactly one control, and the un-askable set is exactly the
    /// three pinned pairs — so neither a missing mapping nor a silently
    /// widened refusal survives.
    #[test]
    fn selection_is_total_over_kind_and_role() {
        let pairs = every_pair();
        assert_eq!(
            pairs.len(),
            SEMANTIC_CONTROL_KIND_COUNT * PRESENTATION_ROLE_COUNT,
            "the pair enumeration lost a kind or a role"
        );
        for (kind, role) in pairs {
            let selection = control_for(kind, role);
            if is_declared_not_askable(kind, role) {
                assert_eq!(
                    selection,
                    ControlSelection::NotAskableInRole,
                    "{} in {} is declared un-askable but resolved to {selection:?}",
                    kind_name(kind),
                    role.canonical_name()
                );
            } else {
                assert!(
                    selection.is_askable(),
                    "{} in {} resolves to nothing; selection is not total",
                    kind_name(kind),
                    role.canonical_name()
                );
            }
        }
    }

    /// Every one of the eight controls is reachable by at least one pair.
    ///
    /// This catches a control that exists but nothing can ask for — dead code
    /// that passes every other check in this module.
    #[test]
    fn every_control_is_reachable_by_at_least_one_pair() {
        let reachable: HashSet<ComponentControl> = every_pair()
            .into_iter()
            .filter_map(|(kind, role)| control_for(kind, role).control())
            .collect();
        let declared: HashSet<ComponentControl> = ALL_COMPONENT_CONTROLS.into_iter().collect();
        let unreachable: Vec<&str> = ALL_COMPONENT_CONTROLS
            .into_iter()
            .filter(|control| !reachable.contains(control))
            .map(ComponentControl::canonical_name)
            .collect();
        assert!(
            unreachable.is_empty(),
            "no kind and role pair asks for: {}",
            unreachable.join(", ")
        );
        assert_eq!(reachable, declared);
    }

    #[test]
    fn every_control_declares_states_drawn_from_the_state_vocabulary() {
        for control in ALL_COMPONENT_CONTROLS {
            for state in control.applicable_states() {
                assert!(
                    ALL_COMPONENT_STATES.contains(state),
                    "{} declares {} which is not a declared state",
                    control.canonical_name(),
                    state.canonical_name()
                );
            }
        }
    }

    #[test]
    fn every_control_declares_resting_focused_and_disabled() {
        for control in ALL_COMPONENT_CONTROLS {
            for required in [
                ComponentState::Resting,
                ComponentState::Focused,
                ComponentState::Disabled,
            ] {
                assert!(
                    control.accepts(required),
                    "{} does not declare {}",
                    control.canonical_name(),
                    required.canonical_name()
                );
            }
        }
    }

    /// No state is applicable to nothing. A state the whole family refuses
    /// would be a state the gallery can never show.
    #[test]
    fn the_declared_states_cover_every_component_state() {
        for state in ALL_COMPONENT_STATES {
            assert!(
                ALL_COMPONENT_CONTROLS
                    .into_iter()
                    .any(|control| control.accepts(state)),
                "no control declares {}",
                state.canonical_name()
            );
        }
    }

    /// Mute and solo reach only the controls that represent a mixer track.
    #[test]
    fn only_mixer_strip_controls_declare_mute_and_solo() {
        for control in ALL_COMPONENT_CONTROLS {
            let in_strip = matches!(control, ComponentControl::Fader | ComponentControl::Meter);
            for state in [ComponentState::Muted, ComponentState::Soloed] {
                assert_eq!(
                    control.accepts(state),
                    in_strip,
                    "{} and {}",
                    control.canonical_name(),
                    state.canonical_name()
                );
            }
        }
    }

    /// The control family's own sources, including the leaf modules later work
    /// packages add. The guards below apply to whatever the directory holds, so
    /// a control added later is covered without amending them.
    fn control_sources() -> Vec<std::path::PathBuf> {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell/visual/controls");
        let mut sources: Vec<std::path::PathBuf> = std::fs::read_dir(&directory)
            .expect("the controls module directory")
            .map(|entry| entry.expect("directory entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
            .collect();
        sources.sort();
        sources
    }

    /// Strips line comments, so prose that *names* the boundary is not mistaken
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
    fn the_scan_reads_the_control_sources() {
        let sources = control_sources();
        assert!(
            sources
                .iter()
                .any(|path| path.file_name().is_some_and(|name| name == "mod.rs")),
            "the control scan did not find the module root"
        );
        for path in &sources {
            assert!(
                !std::fs::read_to_string(path)
                    .expect("control source")
                    .is_empty(),
                "{} is empty",
                path.display()
            );
        }
    }

    /// No control names a semantic action.
    ///
    /// A control returns [`ControlIntent`] and nothing else. If it could name
    /// an action it could build one, and a component that can build an action
    /// is one refactor away from dispatching it. The needle is assembled at
    /// runtime so this test does not find itself.
    #[test]
    fn no_control_source_names_a_semantic_action() {
        let forbidden = format!("{}{}", "Semantic", "Action");
        for path in control_sources() {
            let source = std::fs::read_to_string(&path).expect("control source");
            assert!(
                !code_only(&source).contains(&forbidden),
                "{} names {forbidden}, which a control must never build or dispatch",
                path.display()
            );
        }
    }

    /// No `match` in the family falls through a wildcard.
    ///
    /// The closed unions only close anything if every site is forced to name a
    /// new variant. One wildcard arm turns that compile error into a silent
    /// default — and in [`control_for`] specifically, into a mapping nobody
    /// decided. The needles are assembled at runtime for the same reason.
    #[test]
    fn no_match_arm_in_the_control_family_falls_through_a_wildcard() {
        let wildcard_arm = format!("{}{}", "_ =", ">");
        let binding_arm = format!("{}{}", ".. =", ">");
        for path in control_sources() {
            let source = code_only(&std::fs::read_to_string(&path).expect("control source"));
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
