//! The persistent Utility/Inspector panel.
//!
//! The side region of the workspace band: Utility on PATCH, Inspector on MIXER
//! (`DESIGN.md:444`). It is always visible — narrowing it is a density
//! decision, hiding it is not one this composition can express (`DESIGN.md:450`).
//!
//! # Figma source
//!
//! - `SCREEN · Patch Strip · 1920×1080` → `Utility Panel` (`36:49`): the panel
//!   title, a patch identity line, a hint line, and five 380 × 48 entries on a
//!   60 px pitch — `MASTER VOLUME`, `PATCH VOLUME`, `MIDI INPUT`,
//!   `OUTPUT CHANNEL`, `VOICE LIMIT`.
//! - `SCREEN · Mixer · 1920×1080` → `Inspector` (`42:191`): `CURSOR`, the
//!   cursor identity, its value, its range, a hairline, `MUTE`/`SOLO`, the
//!   route and sends, and a help block.
//!
//! Both panels are a 20 px-inset column with a left keyline. The inset is the
//! authored `ContentRhythm::inset_px` here — 24 px on desktop — because that is
//! the token nearest the authored 20 px and this composition declares no
//! spacing of its own. The 4 px divergence is raised in the handoff.
//!
//! # This is where the placeholder temptation is strongest
//!
//! The panel is dense and the projection is sparse. `DESIGN.md:454` names five
//! Utility entries; the projection carries two of them, both as
//! `PatchControlId::Output` rows. The other three are drawn by the design file
//! and driven by nothing.
//!
//! They are therefore **marked explicitly unavailable rather than omitted**.
//! Omitting them would hide that the panel is incomplete, and this is the one
//! surface where the operator can see the whole declared Utility contract at
//! once. Each designed entry names the projected parameter that drives it and
//! is looked up by that identity, so the marker disappears on its own the moment
//! Phase 5 projects the field — the difference comes from the view data, never
//! from a flag inside this composition.
//!
//! The Inspector's three-line help block is **omitted** instead. Action hints
//! belong to the footer, which projects `validActions`; a second copy here would
//! be a product decision, and one of the three authored lines
//! (`SELECT enters multi-select`) describes an interaction the reducer does not
//! have. Recorded in the handoff.
//!
//! # What this module owns
//!
//! Nothing. Every value comes from the projection or from the authored token
//! and density vocabularies, and the panel returns aggregated
//! [`CompositionIntent`].

use eframe::egui::{ScrollArea, Ui};

use super::{patch_strip_row, section, CompositionIntent};
use crate::control::semantic_graphical_view_model::SemanticRoutedPatch;
use crate::control::{
    GraphicalShellProjection, InteractionMode, MixerControlId, PatchControlId, SemanticControlId,
    SemanticControlViewModel, SemanticGraphicalViewModel, SemanticSurfaceSummary,
    SemanticSurfaceViewModel, SurfaceId,
};
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::MixerTrackParameter;
use crate::mixer::patch_output::PatchOutputParameter;
use crate::shell::visual::controls::PresentationRole;
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::hint::HINT_SEPARATOR;
use crate::shell::visual::primitives::{rules, text};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};

/// Names the Inspector's cursor readout (`42:192`).
/// Introduces the track identity the mixer cursor is standing on.
///
/// A static word, not a value: what follows it is the track the projection's
/// own summary reports as focused.
pub(super) const SELECTED_LABEL: &str = "SELECTED";

/// Names a Patch on the Inspector's route list.
///
/// The same noun the identity band uses, so one Patch is not called two things
/// in one frame.
pub(super) const PATCH_LABEL: &str = "PATCH";

pub(super) const CURSOR_LABEL: &str = "CURSOR";

/// Names the Inspector's routing readout (`42:198`).
pub(super) const ROUTE_LABEL: &str = "ROUTE";

/// Names the Inspector's send list, and the structure marked when the
/// projection supplies no inspector entry at all.
pub(super) const SENDS_LABEL: &str = "SENDS";

/// Names the panel's own summary, marked when the side region is handed a
/// main-surface summary.
pub(super) const SUMMARY_LABEL: &str = "PANEL SUMMARY";

/// `DESIGN.md:454` — master volume.
pub(super) const MASTER_VOLUME_LABEL: &str = "MASTER VOLUME";

/// `DESIGN.md:454` — patch volume, the Patch-local trim before track
/// accumulation.
pub(super) const PATCH_VOLUME_LABEL: &str = "PATCH VOLUME";

/// `DESIGN.md:454` — MIDI input.
pub(super) const MIDI_INPUT_LABEL: &str = "MIDI INPUT";

/// `DESIGN.md:454` — output track.
pub(super) const OUTPUT_TRACK_LABEL: &str = "OUTPUT TRACK";

/// `DESIGN.md:454` — voice limit.
pub(super) const VOICE_LIMIT_LABEL: &str = "VOICE LIMIT";

/// One entry the design file draws in the PATCH Utility panel, and the
/// projected parameter that drives it.
///
/// `driver: None` means the projection carries no identity for this entry at
/// all — not that the value happens to be missing right now. That is the finding
/// this work package records; closing it is Phase 5's job.
struct DesignedUtilityEntry {
    /// The name the design file and `DESIGN.md:454` give this entry.
    label: &'static str,
    /// The projected Patch output parameter behind it, when one exists.
    driver: Option<PatchOutputParameter>,
}

/// The five entries the Utility panel is designed to carry, in authored order.
const DESIGNED_UTILITY_ENTRIES: [DesignedUtilityEntry; 5] = [
    DesignedUtilityEntry {
        label: MASTER_VOLUME_LABEL,
        driver: None,
    },
    DesignedUtilityEntry {
        label: PATCH_VOLUME_LABEL,
        driver: Some(PatchOutputParameter::TrimGain),
    },
    DesignedUtilityEntry {
        label: MIDI_INPUT_LABEL,
        driver: None,
    },
    DesignedUtilityEntry {
        label: OUTPUT_TRACK_LABEL,
        driver: Some(PatchOutputParameter::OutputTrack),
    },
    DesignedUtilityEntry {
        label: VOICE_LIMIT_LABEL,
        driver: None,
    },
];

/// Arranges the persistent side region and returns what its entries asked for.
pub fn render(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let model = projection.semantic_model();
    let title = projection.workspace().side_label();
    let surface = model.surface(SurfaceId::side_for(model.context()));
    section::inset_scope(ui, density, |ui| {
        render_title(ui, title, density);
        entry_viewport(ui, |ui| {
            let Some(surface) = surface else {
                section::mark_unavailable(ui, title, density);
                return CompositionIntent::none();
            };
            let mode = model.interaction_mode();
            match surface.summary() {
                SemanticSurfaceSummary::PatchUtility {
                    patch_id,
                    capability_id,
                    ..
                } => {
                    // The authored identity line (`36:51`). Both halves are
                    // projected by the summary, so both are painted.
                    section::caption(
                        ui,
                        format!("{patch_id}{HINT_SEPARATOR}{capability_id}"),
                        SemanticColor::AccentFocus,
                    );
                    render_patch_utility(ui, surface, mode, density)
                }
                SemanticSurfaceSummary::MixerInspector {
                    focused_control,
                    focused_track,
                    routed_patches,
                    ..
                } => render_mixer_inspector(
                    ui,
                    InspectorSlice {
                        model,
                        surface,
                        focused_control,
                        focused_track: *focused_track,
                        routed_patches,
                    },
                    mode,
                    density,
                ),
                // A main-surface summary in the side region is incoherent: the
                // panel has no reading for it and will not invent one.
                SemanticSurfaceSummary::Patch { .. } | SemanticSurfaceSummary::Mixer { .. } => {
                    section::mark_unavailable(ui, SUMMARY_LABEL, density);
                    CompositionIntent::none()
                }
            }
        })
    })
}

/// The stable id the entry viewport is registered under.
const ENTRY_VIEWPORT_ID: &str = "crest-side-region-entries";

/// The scrollable extent the panel's entries are arranged in, below the title.
///
/// # Why the side region scrolls and the mixer bank does not
///
/// The bank's authored rule is uniform narrowing, never scrolling and never
/// elision, because its cardinality is fixed at sixteen and the design seats all
/// sixteen inside the authored content width. The Inspector is the opposite
/// shape: its extent is vertical and driven by how many return buses, sends and
/// routed patches the projection carries, so there is no width budget to
/// reclaim and no authored count to fit. Content that does not fit has to stay
/// reachable by a gesture.
///
/// The title is arranged outside this viewport so the surface stays named while
/// its entries scroll, which is the behaviour the region had before the shell
/// was recomposed — the adapter wrapped the same content in a vertical scroll
/// region and that wrapping had no composition to land in.
fn entry_viewport<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ScrollArea::vertical()
        .id_salt(ENTRY_VIEWPORT_ID)
        .auto_shrink([false, false])
        .show(ui, add)
        .inner
}

/// The immutable slice the Inspector reads.
struct InspectorSlice<'a> {
    model: &'a SemanticGraphicalViewModel,
    surface: &'a SemanticSurfaceViewModel,
    focused_control: &'a MixerControlId,
    focused_track: MixerTrackId,
    routed_patches: &'a [SemanticRoutedPatch],
}

/// PATCH Utility: the five designed entries in authored order, then anything
/// else the projection supplies that the designed set does not name.
fn render_patch_utility(
    ui: &mut Ui,
    surface: &SemanticSurfaceViewModel,
    mode: InteractionMode,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let mut intent = CompositionIntent::none();
    let gap = section::entry_gap_px(PresentationRole::PanelEntry, density);
    for entry in &DESIGNED_UTILITY_ENTRIES {
        ui.add_space(gap);
        match entry.driver.and_then(|driver| output_row(surface, driver)) {
            Some(view) => intent.absorb(patch_strip_row::render_row(
                ui,
                view,
                mode,
                PresentationRole::PanelEntry,
                density,
            )),
            None => section::mark_unavailable(ui, entry.label, density),
        }
    }
    let remaining: Vec<SemanticControlViewModel> = surface
        .controls()
        .iter()
        .filter(|control| !is_designed(control))
        .cloned()
        .collect();
    if !remaining.is_empty() {
        ui.add_space(gap);
        intent.absorb(section::render_entries(
            ui,
            &remaining,
            mode,
            PresentationRole::PanelEntry,
            density,
            SUMMARY_LABEL,
        ));
    }
    intent
}

/// MIXER Inspector: cursor, value and range, mute and solo, route and sends
/// (`DESIGN.md:466`).
fn render_mixer_inspector(
    ui: &mut Ui,
    slice: InspectorSlice<'_>,
    mode: InteractionMode,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let mut intent = CompositionIntent::none();
    let gap = section::entry_gap_px(PresentationRole::PanelEntry, density);

    // Which track the operator is on. `DESIGN.md:466` asks the Inspector to
    // identify the cursor, and a mixer cursor is a track *and* a parameter — the
    // rows below name the parameter, and without this nothing names the track.
    // The summary projects it, so it is read rather than derived from the focus
    // path: one fact resolved two ways can disagree.
    section::caption(
        ui,
        format!("{SELECTED_LABEL} {}", slice.focused_track),
        SemanticColor::TextPrimary,
    );

    // Cursor, and the value and range it carries. The summary names the control
    // the operator is on; the projection carries that control, so the panel
    // resolves the one by the other rather than re-deriving either.
    section::caption(ui, CURSOR_LABEL, SemanticColor::TextMuted);
    match mixer_row(slice.model, slice.focused_control) {
        Some(view) => intent.absorb(patch_strip_row::render_row(
            ui,
            view,
            mode,
            PresentationRole::PanelEntry,
            density,
        )),
        None => section::mark_unavailable(ui, CURSOR_LABEL, density),
    }

    render_rule(ui, density);

    // Mute and solo. Both are facts about the cursor's track, so both are read
    // from the track the summary names rather than from the state the fader was
    // handed: one fact with two representations can disagree.
    for parameter in [MixerTrackParameter::Mute, MixerTrackParameter::Solo] {
        ui.add_space(gap);
        let id = MixerControlId::Track {
            track_id: slice.focused_track,
            parameter,
        };
        match mixer_row(slice.model, &id) {
            Some(view) => intent.absorb(patch_strip_row::render_row(
                ui,
                view,
                mode,
                PresentationRole::PanelEntry,
                density,
            )),
            None => section::mark_unavailable(ui, parameter.descriptor().label(), density),
        }
    }

    render_rule(ui, density);

    // Route: the Patches the projection reports as routed to this track.
    if slice.routed_patches.is_empty() {
        section::mark_unavailable(ui, ROUTE_LABEL, density);
    } else {
        section::caption(ui, ROUTE_LABEL, SemanticColor::TextMuted);
        for patch in slice.routed_patches {
            // `PATCH 01 · name`, matching the identity band's own spelling of a
            // Patch rather than a second one. `PatchId`'s `Display` is the bare
            // number, so a route line built from it alone reads as `1 · name` —
            // a digit with no noun, and a different noun from the one the
            // header uses for the same thing two bands away.
            section::caption(
                ui,
                format!(
                    "{PATCH_LABEL} {:02}{HINT_SEPARATOR}{}",
                    patch.patch_id().value(),
                    patch.patch_name()
                ),
                SemanticColor::TextSecondary,
            );
        }
    }

    // Sends, returns, and the global rows the inspector surface carries. The
    // cursor is dropped from this list when it is one of them: it was already
    // painted at the top of the panel, and the same row twice reads as two.
    ui.add_space(gap);
    let remaining: Vec<SemanticControlViewModel> = slice
        .surface
        .controls()
        .iter()
        .filter(|control| !carries_mixer_id(control, slice.focused_control))
        .cloned()
        .collect();
    intent.absorb(section::render_entries(
        ui,
        &remaining,
        mode,
        PresentationRole::PanelEntry,
        density,
        SENDS_LABEL,
    ));
    intent
}

/// The projected Utility row driven by one Patch output parameter.
fn output_row(
    surface: &SemanticSurfaceViewModel,
    driver: PatchOutputParameter,
) -> Option<&SemanticControlViewModel> {
    surface
        .controls()
        .iter()
        .find(|control| control.visible() && output_parameter(control) == Some(driver))
}

/// Whether the designed entry list already names this projected row.
fn is_designed(control: &SemanticControlViewModel) -> bool {
    output_parameter(control).is_some_and(|parameter| {
        DESIGNED_UTILITY_ENTRIES
            .iter()
            .any(|entry| entry.driver == Some(parameter))
    })
}

/// The Patch output parameter one projected control carries, if it carries one.
fn output_parameter(control: &SemanticControlViewModel) -> Option<PatchOutputParameter> {
    match control.path().control_id() {
        SemanticControlId::Patch(PatchControlId::Output(parameter)) => Some(*parameter),
        SemanticControlId::Patch(_)
        | SemanticControlId::Mixer(_)
        | SemanticControlId::SurfaceRoot => None,
    }
}

/// The projected control carrying one mixer control identity, wherever the
/// projection put it.
///
/// The Inspector surface carries sends, returns, and globals; the cursor's own
/// track row lives on the main surface. Both are the same immutable projection,
/// and reading the control the summary names is reading view data, not
/// re-deriving it.
fn mixer_row<'a>(
    model: &'a SemanticGraphicalViewModel,
    id: &MixerControlId,
) -> Option<&'a SemanticControlViewModel> {
    model
        .surfaces()
        .iter()
        .flat_map(SemanticSurfaceViewModel::controls)
        .find(|control| control.visible() && carries_mixer_id(control, id))
}

/// Whether one projected control carries exactly this mixer control identity.
fn carries_mixer_id(control: &SemanticControlViewModel, id: &MixerControlId) -> bool {
    match control.path().control_id() {
        SemanticControlId::Mixer(found) => found == id,
        SemanticControlId::Patch(_) | SemanticControlId::SurfaceRoot => false,
    }
}

/// Paints the panel's own heading.
fn render_title(ui: &mut Ui, title: &str, density: &ViewportDensityPolicy) {
    let state = ComponentState::Resting;
    let height = TypeStyle::HeadingPanel.metrics().line_height_px + density.rhythm().row_gap_px();
    let response = ui.allocate_response(
        eframe::egui::Vec2::new(ui.available_width(), height),
        eframe::egui::Sense::hover(),
    );
    let painter = ui.painter_at(response.rect);
    let band = response.rect;
    let run = text::layout(
        &painter,
        title,
        TypeStyle::HeadingPanel,
        SemanticColor::TextPrimary,
        state,
    );
    let size = run.size();
    painter.galley(
        eframe::egui::pos2(band.min.x, band.center().y - size.y / 2.0),
        run,
        text::resolved_color(SemanticColor::TextPrimary, state).resolve(),
    );
}

/// Paints the authored hairline that divides the Inspector's blocks (`42:196`).
fn render_rule(ui: &mut Ui, density: &ViewportDensityPolicy) {
    let height = density.rhythm().row_gap_px();
    let response = ui.allocate_response(
        eframe::egui::Vec2::new(ui.available_width(), height),
        eframe::egui::Sense::hover(),
    );
    let band = response.rect;
    rules::hairline(
        &ui.painter_at(band),
        rules::RuleSpan::Horizontal {
            y_px: band.center().y,
            from_x_px: band.min.x,
            to_x_px: band.max.x,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::super::section::probe::{paint, projection, Painted};
    use super::super::section::tests_support::from_projection_or_vocabulary;
    use super::*;
    use crate::control::TopLevelContext;
    use crate::shell::visual::compositions::ShellComposition;
    use crate::shell::visual::controls::parameter_row::UNAVAILABLE_MARK;
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::token::MIN_INTERACTIVE_TARGET_PX;

    const COMPOSITION: ShellComposition = ShellComposition::UtilityInspectorPanel;

    fn painted(context: TopLevelContext, density: ViewportDensityPolicy) -> Painted {
        paint(COMPOSITION, &projection(context), density)
    }

    /// The panel is wired: asking the family for it reaches this module and
    /// paints.
    #[test]
    fn the_declared_panel_composition_renders_through_this_module() {
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let shown = painted(context, ViewportDensityPolicy::Desktop);
            assert!(
                !shown.texts.is_empty(),
                "{context:?} painted nothing; the panel is still the stub"
            );
        }
    }

    /// The panel carries its projected side-region title in both contexts.
    #[test]
    fn the_panel_paints_its_projected_side_label() {
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let projection = projection(context);
            let shown = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);
            assert!(
                shown.says(projection.workspace().side_label()),
                "{context:?} dropped its projected side label"
            );
        }
    }

    /// Every Utility entry the projection drives is painted from its own view
    /// data, and every designed entry it does not drive is marked.
    ///
    /// This is the C-003 case the panel exists to get right. The check is
    /// derived from the projection, not asserted against a hard-coded list of
    /// three: an entry with a projected driver must *not* be marked, and one
    /// without must be.
    #[test]
    fn every_designed_utility_entry_is_either_driven_or_marked() {
        let projection = projection(TopLevelContext::Patch);
        let surface = projection
            .semantic_model()
            .surface(SurfaceId::PatchUtility)
            .expect("the PATCH Utility surface projects");
        let shown = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);

        let mut driven = 0_usize;
        let mut marked = 0_usize;
        for entry in &DESIGNED_UTILITY_ENTRIES {
            match entry.driver.and_then(|driver| output_row(surface, driver)) {
                Some(view) => {
                    driven += 1;
                    assert!(
                        shown.says(view.label()),
                        "the driven entry {:?} did not reach the panel",
                        view.label()
                    );
                    assert!(
                        !shown.says(entry.label),
                        "{} is driven yet the panel still marked it",
                        entry.label
                    );
                }
                None => {
                    marked += 1;
                    assert!(
                        shown.says(entry.label),
                        "{} is undriven yet the panel did not name it",
                        entry.label
                    );
                }
            }
        }
        assert!(driven > 0, "the projection drives no Utility entry at all");
        assert!(
            marked > 0,
            "the production projection is expected to leave designed entries undriven"
        );
        assert_eq!(
            shown.count(UNAVAILABLE_MARK),
            marked,
            "the panel painted {} unavailable markers for {marked} undriven entries",
            shown.count(UNAVAILABLE_MARK)
        );
    }

    /// The Inspector identifies cursor, value/range, mute, solo, and route
    /// (`DESIGN.md:466`), each from the control the projection carries.
    #[test]
    fn the_inspector_identifies_cursor_mute_solo_and_route() {
        let projection = projection(TopLevelContext::Mixer);
        let model = projection.semantic_model();
        let surface = model
            .surface(SurfaceId::MixerInspector)
            .expect("the MIXER Inspector surface projects");
        let SemanticSurfaceSummary::MixerInspector {
            focused_control,
            focused_track,
            ..
        } = surface.summary()
        else {
            panic!("the Inspector surface carries a MixerInspector summary");
        };
        let shown = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);

        assert!(shown.says(CURSOR_LABEL), "the Inspector dropped its cursor");
        let cursor = mixer_row(model, focused_control)
            .expect("the projection carries the control its summary names");
        assert_eq!(
            shown.count(cursor.label()),
            1,
            "the cursor's projected label reached the Inspector {} times",
            shown.count(cursor.label())
        );
        for parameter in [MixerTrackParameter::Mute, MixerTrackParameter::Solo] {
            let id = MixerControlId::Track {
                track_id: *focused_track,
                parameter,
            };
            let row = mixer_row(model, &id)
                .unwrap_or_else(|| panic!("the projection carries {parameter:?} for the track"));
            assert!(
                shown.says(row.label()),
                "{parameter:?} did not reach the Inspector"
            );
        }
        assert!(shown.says(ROUTE_LABEL), "the Inspector dropped its route");
    }

    /// The panel stays visible at the compact viewport, with the same content
    /// and its interactive targets intact.
    #[test]
    fn the_panel_is_visible_at_the_compact_viewport_with_its_targets_intact() {
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let desktop = painted(context, ViewportDensityPolicy::Desktop);
            let deck = painted(context, ViewportDensityPolicy::SteamDeck);
            assert!(
                !deck.texts.is_empty(),
                "{context:?} painted nothing at the compact viewport"
            );
            assert_eq!(
                desktop.texts, deck.texts,
                "{context:?} paints different content at the two viewports"
            );
        }
        for policy in ALL_DENSITY_POLICIES {
            assert!(
                policy.split().side_px > 0.0,
                "{} hides the side region",
                policy.canonical_name()
            );
            assert!(
                policy.utility_control().height_px >= MIN_INTERACTIVE_TARGET_PX,
                "{} shrinks a panel entry below the authored minimum target",
                policy.canonical_name()
            );
        }
    }

    /// The panel paints nothing that did not arrive in the projection.
    #[test]
    fn the_panel_paints_no_text_that_did_not_arrive_in_the_projection() {
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let projection = projection(context);
            let shown = paint(COMPOSITION, &projection, ViewportDensityPolicy::Desktop);
            for run in &shown.texts {
                assert!(
                    from_projection_or_vocabulary(&projection, run),
                    "{context:?} painted {run:?}, which is neither projected nor declared"
                );
            }
        }
    }

    /// Every designed entry names a label the design file authors, and no two
    /// entries claim the same projected driver.
    #[test]
    fn the_designed_entry_list_is_coherent() {
        assert_eq!(DESIGNED_UTILITY_ENTRIES.len(), 5);
        let drivers: Vec<PatchOutputParameter> = DESIGNED_UTILITY_ENTRIES
            .iter()
            .filter_map(|entry| entry.driver)
            .collect();
        let unique: std::collections::HashSet<_> = drivers.iter().collect();
        assert_eq!(
            unique.len(),
            drivers.len(),
            "two designed entries claim the same projected driver"
        );
        for entry in &DESIGNED_UTILITY_ENTRIES {
            assert!(!entry.label.is_empty());
        }
    }
}
