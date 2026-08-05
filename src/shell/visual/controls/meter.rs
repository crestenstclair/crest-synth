//! The mixer meter.
//!
//! The read-only level readout of one mixer track column.
//!
//! # Specimen — and the gap in it
//!
//! **The design file carries no meter.** Inside frame `SCREEN · Mixer ·
//! 1920×1080` (`42:3`) it is the `16 Fader Grid` (`42:25`) that holds the
//! sixteen `Fader / T00…T0F` instances and nothing else; the frame's other
//! children — the context line, the mixer header, the legend, the adjustment
//! line, the inspector, and the controls — carry no level readout either, and
//! nor does any other reachable page. A column is a track name, a level track
//! with its cap, a value, a pan pair, and a mute/solo line. There is no separate
//! level readout beside the fader, no segment ladder, and no peak mark, in the
//! screen frames or in the component specimens.
//!
//! That is recorded as a finding rather than filled in. What is built here is
//! the one thing the mixer strip's own specimen already settles: a meter is the
//! read-only twin of the column it sits in, so it takes the column geometry,
//! the band stack, the centered text alignment, and the track and fill
//! proportions from [`super::fader`] rather than inventing a second look for the
//! same strip. It paints no cap — the cap is the grab affordance of a control
//! that is grabbed, and this one is not. Segmentation and peak indication are
//! deliberately absent: both would be invented, and an invented meter reads as
//! authoritative.
//!
//! # It reads no signal
//!
//! C-001 puts audio out of scope for this slice, and a meter is the one control
//! that would normally carry a live one. It does not carry one here, and it must
//! not acquire one to look convincing. It paints the level its immutable
//! [`SemanticControlViewModel`] reports and nothing else — no snapshot, no
//! shared cell, no observation, no engine. **A resting meter is a correct meter
//! here.** The test at the bottom of this file scans this source for every
//! spelling of that boundary, and a second test proves the render is a pure
//! function of its input by running it twice and comparing what reached the
//! screen.
//!
//! # Muted and soloed
//!
//! Both are applicable. `DESIGN.md:420` — meters observe each track after level
//! and pan but **before** the mute/solo audibility gate, so a muted track stays
//! diagnosable. A muted meter therefore still shows its level; its mute is
//! signalled separately, by the authored `M ON` in the status band, and never by
//! zeroing what it displays.
//!
//! # Painted, not assembled
//!
//! `research.md` R-02. No `egui::ProgressBar`: its rounding, fill, and animation
//! come out of `egui::Style`, a second visual vocabulary the literal guard
//! cannot see.

use eframe::egui::{pos2, Rect, Sense, Ui, Vec2};

use super::compact_slider::value_fraction;
use super::fader::{bands, column_height_px, column_width_px, fill_span, level_color, track_bar};
use super::{ControlIntent, PresentationRole};
use crate::control::SemanticControlViewModel;
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::{focus, status};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{Radius, SemanticColor, MIN_INTERACTIVE_TARGET_PX};

/// Paints the meter and returns what it asks for, which is always nothing.
///
/// The specimen shows no interactive element in a mixer column other than the
/// fader, so this control reports no request at all rather than a request the
/// design never authored.
pub fn render(
    ui: &mut Ui,
    view: &SemanticControlViewModel,
    state: ComponentState,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> ControlIntent {
    let width = match role {
        PresentationRole::VerticalStrip => column_width_px(density),
        PresentationRole::ListedRow
        | PresentationRole::PanelEntry
        | PresentationRole::ModalEntry => ui.available_width(),
    };
    let size = Vec2::new(
        width.max(MIN_INTERACTIVE_TARGET_PX),
        column_height_px(ui, density),
    );
    let (column, _response) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(column);
    let bands = bands(column);

    status::paint_row_fill(&painter, column, state);

    if bands.track.height() > 0.0 {
        let bar = track_bar(bands.track);
        let corner = Radius::Small.resolve();
        painter.rect_filled(bar, corner, SemanticColor::BgElevated.resolve());
        // Whatever the view data reports, including a resting level, and
        // nothing at all when it reports no position. The state is not
        // consulted here: a muted column still shows the level it was handed.
        if let Some(fraction) = value_fraction(view) {
            let span = fill_span(bar);
            let top = span.max.y - span.height() * fraction;
            painter.rect_filled(
                Rect::from_min_max(pos2(span.min.x, top), span.max),
                corner,
                level_color(state).resolve(),
            );
        }
    }

    super::fader::paint_column_text(&painter, &bands, view, state);
    focus::focus_frame(&painter, column, state);

    ControlIntent::None
}

#[cfg(test)]
mod tests {
    use super::super::compact_slider::harness::{
        paint, paint_states, ranged_control, unranged_control,
    };
    use super::super::compact_slider::value_text;
    use super::*;
    use crate::control::SemanticControlKind;
    use crate::shell::visual::controls::{control_for, ComponentControl, ControlSelection};
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::state::{NonColorSignal, ALL_COMPONENT_STATES};
    use crate::shell::visual::token::{SpacingStep, KEYLINE_RESTING_PX};

    const CONTROL: ComponentControl = ComponentControl::Meter;
    const ROLE: PresentationRole = PresentationRole::VerticalStrip;

    fn column(density: ViewportDensityPolicy) -> Rect {
        Rect::from_min_max(
            pos2(0.0, 0.0),
            pos2(column_width_px(&density), density.bands().workspace_px),
        )
    }

    #[test]
    fn the_family_asks_for_this_control_in_a_mixer_column() {
        assert_eq!(
            control_for(SemanticControlKind::Identity, ROLE),
            ControlSelection::Control(CONTROL),
            "a read-only mixer column no longer resolves to the meter"
        );
    }

    #[test]
    fn the_meter_declares_every_state_applicable() {
        assert_eq!(CONTROL.applicable_states(), ALL_COMPONENT_STATES);
    }

    #[test]
    fn every_applicable_state_paints_its_declared_non_color_signal() {
        let view = ranged_control();
        for density in ALL_DENSITY_POLICIES {
            let states = CONTROL.applicable_states();
            for (state, painted) in states
                .iter()
                .zip(paint_states(CONTROL, &view, states, ROLE, density))
            {
                assert!(
                    painted.painted_anything(),
                    "{} painted nothing at {}",
                    state.canonical_name(),
                    density.canonical_name()
                );
                match state.appearance().signal {
                    NonColorSignal::Word(word) => assert!(
                        painted.says(word),
                        "{} did not paint its authored word {word} at {}",
                        state.canonical_name(),
                        density.canonical_name()
                    ),
                    NonColorSignal::ProgressWord => assert!(
                        painted.says("Preparing") || painted.says("Activating"),
                        "{} painted no authored progress word at {}",
                        state.canonical_name(),
                        density.canonical_name()
                    ),
                    NonColorSignal::TypedFailure => assert!(
                        painted.says("Failed"),
                        "{} painted no failure text at {}",
                        state.canonical_name(),
                        density.canonical_name()
                    ),
                    NonColorSignal::Shape => assert!(
                        !painted.rects.is_empty(),
                        "{} painted no shape at {}",
                        state.canonical_name(),
                        density.canonical_name()
                    ),
                }
            }
        }
    }

    #[test]
    fn no_two_applicable_states_look_alike_without_color() {
        let view = ranged_control();
        let states = CONTROL.applicable_states();
        let painted = paint_states(CONTROL, &view, states, ROLE, ViewportDensityPolicy::Desktop);
        for (index, first) in states.iter().enumerate() {
            for (offset, second) in states.iter().enumerate().skip(index + 1) {
                assert_ne!(
                    painted[index].colorless_evidence(),
                    painted[offset].colorless_evidence(),
                    "{} and {} are distinguishable only by color",
                    first.canonical_name(),
                    second.canonical_name()
                );
            }
        }
    }

    /// The level the view data reports is what is painted, in every state — so
    /// a muted column stays diagnosable (`DESIGN.md:420`) and its mute is
    /// carried by the word beside it rather than by a zeroed bar.
    #[test]
    fn a_muted_meter_still_shows_its_level_and_says_so_separately() {
        let view = ranged_control();
        let density = ViewportDensityPolicy::Desktop;
        let fraction = value_fraction(&view).expect("a ranged specimen has a level");
        let span = fill_span(track_bar(bands(column(density)).track));
        let expected = span.height() * fraction;
        let states = [ComponentState::Resting, ComponentState::Muted];
        let painted = paint_states(CONTROL, &view, &states, ROLE, density);
        for (state, painted) in states.iter().zip(&painted) {
            assert!(
                painted.rects.iter().any(|painted| {
                    (painted.rect.height() - expected).abs() < KEYLINE_RESTING_PX
                        && (painted.rect.width() - span.width()).abs() < KEYLINE_RESTING_PX
                }),
                "{} stopped showing the level the view data reports",
                state.canonical_name()
            );
        }
        assert!(painted[1].says("M ON"), "a muted meter did not say M ON");
        assert!(
            !painted[0].says("M ON"),
            "a resting meter claimed a mute it was not handed"
        );
    }

    /// A meter with nothing to show is a correct meter in this slice. It paints
    /// its track, its name, and its value, and invents no level.
    #[test]
    fn a_meter_with_no_reported_position_invents_none() {
        let view = unranged_control();
        assert_eq!(value_fraction(&view), None);
        let density = ViewportDensityPolicy::Desktop;
        let span = fill_span(track_bar(bands(column(density)).track));
        let painted = paint(CONTROL, &view, ComponentState::Resting, ROLE, density);
        assert!(
            painted.painted_anything(),
            "a meter with no reported position painted nothing at all"
        );
        assert!(
            !painted.rects.iter().any(|painted| {
                (painted.rect.width() - span.width()).abs() < KEYLINE_RESTING_PX
                    && painted.rect.height() < span.height() - KEYLINE_RESTING_PX
            }),
            "a meter with no reported position invented a level"
        );
    }

    /// The meter is the fader's read-only twin, so it inherits the specimen's
    /// centered column rather than acquiring an alignment of its own. Asserted
    /// here as well as on the fader because "shared today" is not a guarantee
    /// the two stay one column tomorrow.
    #[test]
    fn the_column_runs_sit_on_the_column_axis_at_both_viewports() {
        let view = ranged_control();
        for density in ALL_DENSITY_POLICIES {
            let painted = paint(CONTROL, &view, ComponentState::Muted, ROLE, density);
            let axis = painted
                .rects
                .iter()
                .find(|painted| {
                    (painted.rect.width() - SpacingStep::S12.resolve()).abs() < KEYLINE_RESTING_PX
                })
                .expect("a meter paints its level groove")
                .rect
                .center()
                .x;
            for content in [
                view.label().to_owned(),
                value_text(view.value()),
                "M ON".to_owned(),
            ] {
                let run = painted.run(&content).unwrap_or_else(|| {
                    panic!(
                        "{} painted no run saying {content:?}",
                        density.canonical_name()
                    )
                });
                assert!(
                    (run.rect.center().x - axis).abs() < KEYLINE_RESTING_PX,
                    "{} paints {content:?} at {} rather than on the column axis {axis}",
                    density.canonical_name(),
                    run.rect.center().x
                );
            }
        }
    }

    /// The render is a pure function of the view data it is handed: the same
    /// input twice puts exactly the same thing on screen.
    #[test]
    fn the_same_view_data_paints_the_same_thing_twice() {
        let view = ranged_control();
        let painted = paint_states(
            CONTROL,
            &view,
            &[ComponentState::Resting, ComponentState::Resting],
            ROLE,
            ViewportDensityPolicy::Desktop,
        );
        assert_eq!(
            painted[0].signature, painted[1].signature,
            "the meter painted two different things from one input"
        );
        assert!(
            !painted[0].signature.is_empty(),
            "the determinism check compared two empty renders"
        );
    }

    #[test]
    fn it_paints_only_what_the_view_data_carries_and_asks_for_nothing() {
        let view = ranged_control();
        let painted = paint(
            CONTROL,
            &view,
            ComponentState::Resting,
            ROLE,
            ViewportDensityPolicy::Desktop,
        );
        let carried = [view.label().to_owned(), value_text(view.value())];
        for run in &painted.texts {
            assert!(
                carried.iter().any(|allowed| allowed == run),
                "the meter painted {run:?}, which is not in its view data"
            );
        }
        assert_eq!(
            painted.intent,
            ControlIntent::None,
            "the meter is presentational and asks for nothing"
        );
    }

    /// This source names no part of the audio boundary.
    ///
    /// The needles are assembled at runtime so this test does not find itself,
    /// and comment lines are stripped first, so prose that *names* the boundary
    /// is not mistaken for code that crosses it.
    #[test]
    fn the_meter_names_no_part_of_the_audio_boundary() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/shell/visual/controls/meter.rs");
        let source = std::fs::read_to_string(&path).expect("the meter source");
        let code: String = source
            .lines()
            .map(|line| match line.find("//") {
                Some(offset) => &line[..offset],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code.contains("fn render"),
            "the audio scan did not read the meter's own source"
        );
        let forbidden: Vec<String> = [
            ("Audio", "Observation"),
            ("ato", "mic"),
            ("Ato", "mic"),
            ("real", "_time"),
            ("Mix", "Engine"),
            ("triple", "_buffer"),
            ("rt", "rb"),
            ("cp", "al"),
            ("Prepared", "EngineRack"),
            ("Patch", "AudioBlock"),
            ("Voice", "Envelope"),
        ]
        .iter()
        .map(|(head, tail)| format!("{head}{tail}"))
        .collect();
        for needle in forbidden {
            assert!(
                !code.contains(needle.as_str()),
                "the meter names {needle}, which would give it a signal it must not have"
            );
        }
    }
}
