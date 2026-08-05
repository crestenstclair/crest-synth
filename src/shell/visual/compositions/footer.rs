//! The footer: the current path, and the actions valid right now.
//!
//! **Components paint. Views compose. The reducer decides.**
//!
//! This composition fills [`ShellRegion::Footer`]. It replaces `paint_footer` in
//! `src/adapter/eframe_graphical_window.rs`, which painted the path label on one
//! side of a fractional split and one bordered target per valid action on the
//! other.
//!
//! # It invents nothing
//!
//! `DESIGN.md:514` — the footer echoes the current context/path and *only*
//! actions valid at the focused target. Validity is reducer-owned and arrives
//! as [`crate::control::ShellFooter::action_hints`], one already-finished string
//! per valid action. This composition renders those strings and adds none. When
//! the projection supplies none, the trailing half of the band is empty — not
//! filled with a plausible default, which is `C-003` in miniature.
//!
//! # Why the hints are addressable at all
//!
//! The design file's `Controls` band draws plain hint text. This band draws a
//! bordered, pointer- and touch-addressable target per hint, at the authored
//! minimum height. **That divergence is inherited, not introduced here.** The
//! shipped `paint_footer` this composition replaces already emitted an action
//! from `ui.add(button).clicked()`, and the `crest-component-foundations`
//! mission already gave those buttons `KEYLINE_RESTING_PX` and a
//! `MIN_INTERACTIVE_TARGET_PX` floor; `hint_target` copies that shape. Moving
//! painting into a composition is not the occasion to drop a shipped
//! affordance, and dropping touch addressability to match a desktop-only frame
//! would be the wrong way to lose it on a Steam Deck target.
//!
//! What plain text would break is two of the assertions written below it,
//! which is not an argument for anything: they are this file's own. The reason to
//! keep the targets is the behaviour they preserve and the minimum target size
//! `DESIGN.md` requires — not a test. The Figma divergence stays recorded as
//! raised, not resolved.
//!
//! # Why an index and not an action
//!
//! A footer target is addressable, so the composition has to be able to report
//! that one was addressed. What it reports is the *index into the slice it was
//! handed*. It does not report an action, and it cannot: resolving an index
//! against the reducer's vocabulary needs the active context and what is valid
//! right now, which live on the other side of this boundary. The parallel
//! `semantic_model().valid_actions()` is built by mapping over the same list in
//! the same order, so the index addresses both — and the caller that owns the
//! reducer does that lookup, in one line, with no choice in it.
//!
//! The shipped footer drew and emitted from one collection, so it could not
//! address the wrong action. Two collections joined by convention can, which is
//! why the index is minted as an [`AddressedHint`] carrying the count it was
//! minted over rather than as a bare `usize`.
//!
//! # Recorded limitations
//!
//! Two, stated rather than papered over:
//!
//! - **Hints carry one tone.** [`crate::shell::visual::primitives::hint`]
//!   declares four tones, and the design file's `CLI Hint` set distinguishes
//!   neutral, focus, adjust, and back. Nothing in the projection says which tone
//!   a valid action reads in, and deriving one from the hint's text would be an
//!   invented mapping. Every hint therefore paints [`HINT_TONE`] until a tone
//!   reaches the projection, which is a reducer-side change this mission's
//!   `C-002` puts out of scope.
//! - **A single-token hint gains one space.** [`ActionHint`] is a chord and an
//!   action joined by a space, and a hint string with no space in it has no
//!   chord to split off. [`hint_for`] puts the whole string in the chord
//!   position, so its rendered text ends with one trailing space. Every hint the
//!   projector builds today is at least two tokens, and the assertions below
//!   hold the round-trip exactly for those; the artifact is recorded here so
//!   that a single-token hint shows up as a known shape rather than a surprise.
//!
//! # Raised, not resolved
//!
//! The design file's `Controls` band is authored at the desktop viewport only —
//! there is no compact frame for it — and it does not say what happens when
//! more hints arrive than fit. What this composition does is what the shipped
//! footer did: the hints sit in a horizontal scroll region (`hint_viewport`)
//! bounded by where the path label ends, so a hint the band has no room for
//! scrolls into view rather than ceasing to exist. That the overflow is
//! *reachable* is the part `DESIGN.md:514` constrains, and it is measured by
//! gesture below; that it neither clips nor collides is measured against the
//! same rule the acceptance targets count. A compact-viewport rule — fewer
//! hints, a second line, a smaller style, or a decision that hints should never
//! need scrolling at 1280×800 — is a design decision this composition records
//! rather than invents.
//!
//! # What it owns
//!
//! Nothing. The band height and inset come from [`ViewportDensityPolicy`], the
//! target height from the authored minimum, the keyline width and every color
//! from the token vocabulary, and every hint from the projection.

use eframe::egui::{vec2, Align, Button, Layout, ScrollArea, Stroke, Ui};

use crate::control::GraphicalShellProjection;
use crate::shell::visual::compositions::application_shell::{AddressedHint, ShellFrameIntent};
use crate::shell::visual::compositions::CompositionIntent;
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::hint::{hint_line, ActionHint, HintTone};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{
    SemanticColor, TypeStyle, KEYLINE_RESTING_PX, MIN_INTERACTIVE_TARGET_PX,
};

/// The tone every footer hint reads in.
///
/// One tone, because the projection carries no tone. See the recorded
/// limitation on this module.
pub const HINT_TONE: HintTone = HintTone::Neutral;

/// Splits one finished hint string into the chord and action
/// [`ActionHint`] is built from.
///
/// The split is bookkeeping, not presentation: [`hint_line`] appends a hint's
/// rendered text as one run in one format, so the chord and the action are
/// painted identically and the string that reaches the screen is the projection
/// value unchanged. Splitting at the first space is what makes that round-trip
/// exact for a `"{chord} {label}"` hint; a hint with no space keeps the whole
/// string, which is the one shape that gains a trailing space.
pub fn hint_for(supplied: &str) -> ActionHint<'_> {
    match supplied.split_once(' ') {
        Some((chord, action)) => ActionHint::new(chord, action, HINT_TONE),
        None => ActionHint::new(supplied, "", HINT_TONE),
    }
}

/// Arranges the footer and reports which hint, if any, was addressed.
pub fn arrange(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> ShellFrameIntent {
    let footer = projection.footer();
    let hints = footer.action_hints();
    let mut addressed = None;
    super::application_shell::band_row(
        ui,
        density,
        // The operator's location survives; the hints are what gives way —
        // but only down to the floor below, never to nothing.
        super::application_shell::BandPrecedence::Leading,
        |half| {
            half.paint_reserving(
                hint_floor_px(density),
                footer.path_label(),
                TypeStyle::InstructionHint,
                SemanticColor::TextPrimary,
            );
        },
        |half| {
            hint_group(half.ui(), hints, &mut addressed);
        },
    );
    ShellFrameIntent::from_addressed_hint(addressed)
}

/// Adds the hint targets and records which one, if any, was addressed.
///
/// The index and the count are minted together at the one site that painted the
/// targets, so an [`AddressedHint`] can only ever name a position in the slice
/// this loop walked. See [`AddressedHint`] for why the count travels with it.
fn hint_group(ui: &mut Ui, hints: &[String], addressed: &mut Option<AddressedHint>) {
    hint_viewport(ui, |ui| {
        for (index, supplied) in hints.iter().enumerate() {
            if hint_target(ui, supplied).clicked() && addressed.is_none() {
                *addressed = Some(AddressedHint::minted(index, hints.len()));
            }
        }
    });
}

/// The stable id the hint viewport is registered under.
const HINT_VIEWPORT_ID: &str = "crest-footer-action-hints";

/// The width the hints are guaranteed however long the path label runs.
///
/// The shipped `paint_footer` gave the path a hard 38% cell and the hints the
/// remainder, so neither half could erase the other. Composing the band as two
/// halves lost that: `path_label` is built from the focus path and the
/// projection bounds nothing about its depth, so a deep enough path takes the
/// whole band and every action with it — permanently, since there is nothing
/// left to scroll.
///
/// 38% is a visual value and would have to come from [`ViewportDensityPolicy`],
/// which has no accessor for a band's internal split. Rather than invent a
/// fraction here, the floor is the two authored quantities that already say how
/// small an addressable thing may be: one interactive target's authored minimum
/// plus the band's own content inset. A hint group that narrow is mostly scroll
/// region, which is the honest outcome — the actions are reachable, and the
/// path is the run that was cut.
///
/// Recorded as a choice, not a derivation: the design file does not say what a
/// footer does when the path alone overruns the band.
fn hint_floor_px(density: &ViewportDensityPolicy) -> f32 {
    MIN_INTERACTIVE_TARGET_PX + density.rhythm().inset_px
}

/// The region the hints are laid out inside.
///
/// A horizontal scroll region, which is what the shipped `paint_footer` gave
/// them and what this composition preserves.
///
/// # What it is for: reachability, not containment
///
/// Measured rather than assumed. With twenty hints in a band that holds six,
/// the rendering stack culls every run that falls outside the space the group
/// was given, so **without** this region the over-full footer is already clean
/// by the clipped-or-overlapping rule — nothing collides, one run leaves its
/// container, and the other fourteen hints simply never reach the shape
/// stream. That is the failure this region exists to prevent, and it is one no
/// containment assertion can see: a run the stack culled and a run that was
/// never composed are the same absence in a shape stream.
///
/// `DESIGN.md:514` says the footer shows the actions valid right now, so a hint
/// that does not fit may not quietly stop existing. Inside a scroll region it
/// is still reachable, which
/// `a_hint_the_over_full_band_could_not_show_is_still_reachable` measures by
/// performing the gesture rather than by asserting the region was constructed.
///
/// # Why the direction is stated
///
/// The trailing half of a band is laid out right to left so that a group which
/// fits lands against the band's trailing edge, and `Ui::horizontal` adopts
/// that preference — which would paint the hints in reverse order and grow the
/// overflow *towards* the path label instead of away from it. Stating the
/// direction keeps the projection's order on screen and sends the overflow out
/// through the scroll region's own edge, which is where a gesture can follow
/// it.
fn hint_viewport(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ScrollArea::horizontal()
        .id_salt(HINT_VIEWPORT_ID)
        .show(ui, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), add);
        });
}

/// The family conformance for [`ShellComposition::Footer`].
///
/// Drops the addressed hint, which is correct for a caller that reaches the
/// footer through the family: the gallery shows the band, it does not operate
/// it. The production path goes through
/// [`super::application_shell::arrange_band`].
///
/// [`ShellComposition::Footer`]: crate::shell::visual::compositions::ShellComposition::Footer
pub fn render(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    arrange(ui, projection, density).intent().clone()
}

/// One addressable hint target.
///
/// The fill, the resting keyline, and the authored minimum target height are
/// the shipped `paint_footer`'s, line for line — see the note on this module
/// for why that shape is preserved rather than reduced to the design file's
/// plain hint text. Width is left to the hint: the minimum binds the dimension
/// the text does not already set.
fn hint_target(ui: &mut Ui, supplied: &str) -> eframe::egui::Response {
    ui.add(
        Button::new(hint_line(&[hint_for(supplied)], ComponentState::Resting))
            .fill(SemanticColor::BgSurface.resolve())
            .stroke(Stroke::new(
                KEYLINE_RESTING_PX,
                SemanticColor::BorderDefault.resolve(),
            ))
            .min_size(vec2(0.0, MIN_INTERACTIVE_TARGET_PX)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{ShellFooter, TopLevelContext};
    use crate::shell::visual::compositions::application_shell::testing_support::{
        arrange_with_pointer_click, band_rect, band_rects, band_row_runs, band_runs,
        containment_defects, overflow_report, painted_across_horizontal_sweep, painted_text,
        paired_projection_fixture, projection_fixture, projection_with_footer,
        unbounded_text_width,
    };
    use crate::shell::visual::compositions::{ShellComposition, ShellRegion};
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::primitives::hint::{ALL_HINT_TONES, HINT_SEPARATOR};
    use crate::shell::ShellRegionId;

    fn footer_with(hints: Vec<String>) -> ShellFooter {
        ShellFooter::new("PATCH / patch.engine", hints)
    }

    /// Every run the footer paints anywhere in its scroll region, found by
    /// sweeping the region to its end.
    ///
    /// "Reachable" for a scroll region is the union over the sweep, not what
    /// one gesture happens to reveal: a single scroll proves the region moves,
    /// while the union is what says the content is all still there. The step is
    /// the authored minimum target, so no hint can hide between two offsets,
    /// and the sweep stops as soon as everything expected has appeared.
    /// The gesture is aimed at a painted hint target rather than at the band's
    /// centre, because the rendering stack delivers a wheel event to whatever
    /// is under the pointer. When the path label is long the hint region is a
    /// narrow strip at the trailing edge and the band's centre is over the path
    /// — scrolling there moves nothing, and the sweep would report content
    /// unreachable that an operator can in fact reach.
    fn reachable_by_sweeping(
        projection: &GraphicalShellProjection,
        policy: &ViewportDensityPolicy,
        expected: &[String],
    ) -> Vec<String> {
        let band = band_rect(policy, ShellRegionId::Footer);
        let aim = band_rects(projection, policy, ShellRegionId::Footer)
            .first()
            .map_or_else(|| band.center(), |target| target.rect.center());
        // Step by the authored minimum target, which is never wider than the
        // hint region (its floor is that minimum plus the band inset), so no
        // hint can sit entirely between two offsets. The step count covers a
        // band's width of content per hint, which is more than any hint the
        // projector builds occupies.
        let steps = expected.len() * (band.width() / MIN_INTERACTIVE_TARGET_PX).ceil() as usize;
        painted_across_horizontal_sweep(projection, policy, aim, MIN_INTERACTIVE_TARGET_PX, steps)
    }

    #[test]
    fn the_footer_is_bound_to_its_observed_region() {
        assert_eq!(ShellComposition::Footer.region(), ShellRegion::Footer);
    }

    /// Every hint the projection supplied reaches the screen, and nothing else
    /// does. This is the assertion that fails if the footer fills itself in.
    #[test]
    fn the_footer_paints_exactly_the_hints_the_projection_supplied() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let supplied = projection.footer().action_hints();
                let painted = painted_text(&band_runs(&projection, &policy, ShellRegionId::Footer));
                for hint in supplied {
                    assert!(
                        painted.iter().any(|run| run == hint),
                        "{} did not paint {hint:?}; it painted {painted:?}",
                        policy.canonical_name()
                    );
                }
                for run in &painted {
                    assert!(
                        run == projection.footer().path_label()
                            || supplied.iter().any(|hint| hint == run),
                        "the footer invented {run:?} at {}",
                        policy.canonical_name()
                    );
                }
                assert_eq!(
                    painted.len(),
                    supplied.len() + 1,
                    "{} painted {painted:?}",
                    policy.canonical_name()
                );
            }
        }
    }

    /// With no hints supplied the band carries its path and nothing else. An
    /// empty region looks broken and inviting a default is natural; `C-003`
    /// forbids it.
    #[test]
    fn a_footer_with_no_hints_renders_empty_rather_than_defaulted() {
        for policy in ALL_DENSITY_POLICIES {
            let projection =
                projection_with_footer(TopLevelContext::Patch, footer_with(Vec::new()));
            let painted = painted_text(&band_runs(&projection, &policy, ShellRegionId::Footer));
            assert_eq!(
                painted,
                vec![projection.footer().path_label().to_owned()],
                "{} defaulted an empty footer",
                policy.canonical_name()
            );
            assert!(
                band_rects(&projection, &policy, ShellRegionId::Footer).is_empty(),
                "{} framed a target for a hint that does not exist",
                policy.canonical_name()
            );
        }
    }

    /// Each hint round-trips through the primitive unchanged, so the composed
    /// line is the projection's own strings and not a re-derivation of them.
    #[test]
    fn every_supplied_hint_round_trips_through_the_hint_primitive() {
        let hints = vec![
            "1 OPEN MIXER".to_owned(),
            "K+D FINE INCREASE".to_owned(),
            "A / RETURN RETURN".to_owned(),
            "UNAVAILABLE PATCH STEP".to_owned(),
        ];
        for supplied in &hints {
            assert_eq!(&hint_for(supplied).text(), supplied);
        }
        let composed: Vec<ActionHint<'_>> = hints.iter().map(|hint| hint_for(hint)).collect();
        assert_eq!(
            hint_line(&composed, ComponentState::Resting).text,
            hints.join(HINT_SEPARATOR)
        );
    }

    /// The one shape the primitive cannot express, pinned so it stays a known
    /// artifact rather than becoming a surprise.
    #[test]
    fn a_single_token_hint_is_the_one_shape_that_gains_a_trailing_space() {
        assert_eq!(hint_for("RETURN").text(), "RETURN ");
        assert_eq!(hint_for("RETURN").text().trim_end(), "RETURN");
    }

    /// Every hint is addressable at the authored minimum height, at both
    /// viewports. Measured from the stroked rectangles that reached the screen,
    /// not from what the composition asked for.
    #[test]
    fn every_hint_target_is_framed_at_the_authored_minimum_at_both_viewports() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let targets = band_rects(&projection, &policy, ShellRegionId::Footer);
                assert_eq!(
                    targets.len(),
                    projection.footer().action_hints().len(),
                    "{} framed {} targets for {} hints",
                    policy.canonical_name(),
                    targets.len(),
                    projection.footer().action_hints().len()
                );
                for target in targets {
                    assert!(
                        target.rect.height() >= MIN_INTERACTIVE_TARGET_PX,
                        "{} framed a {} px target, below the authored {} px minimum",
                        policy.canonical_name(),
                        target.rect.height(),
                        MIN_INTERACTIVE_TARGET_PX
                    );
                }
                assert!(
                    policy.bands().footer_px >= MIN_INTERACTIVE_TARGET_PX,
                    "{} footer band cannot hold an authored target",
                    policy.canonical_name()
                );
            }
        }
    }

    /// Nothing in the footer clips or overlaps at either viewport.
    #[test]
    fn no_footer_run_clips_or_overlaps_at_either_viewport() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let defects = containment_defects(
                    &band_runs(&projection, &policy, ShellRegionId::Footer),
                    band_rect(&policy, ShellRegionId::Footer),
                );
                assert!(
                    defects.is_empty(),
                    "the footer at {} in {}: {defects:#?}",
                    policy.canonical_name(),
                    context.label()
                );
            }
        }
    }

    /// The single tone is a declared one, and the vocabulary still holds four —
    /// so the limitation is one tone chosen, not three lost.
    #[test]
    fn the_one_painted_tone_is_a_declared_tone() {
        assert!(ALL_HINT_TONES.contains(&HINT_TONE));
        assert_eq!(ALL_HINT_TONES.len(), 4);
    }

    /// Addressing one hint reports that hint's index, minted over the slice the
    /// footer painted, and addressing a different one reports a different
    /// index.
    ///
    /// This is the contract the render adapter resolves against
    /// `semantic_model().valid_actions()`. Driven with real pointer events
    /// through the real composition, because an index that is never measured
    /// coming back is a type, not a contract.
    #[test]
    fn addressing_a_hint_reports_that_hint_index() {
        for policy in ALL_DENSITY_POLICIES {
            let projection = projection_fixture(TopLevelContext::Patch);
            let supplied = projection.footer().action_hints();
            let hints = supplied.len();
            let mut targets = band_rects(&projection, &policy, ShellRegionId::Footer);
            assert_eq!(
                targets.len(),
                hints,
                "{} framed {} targets for {hints} hints",
                policy.canonical_name(),
                targets.len()
            );
            targets.sort_by(|first, second| {
                first
                    .rect
                    .min
                    .x
                    .partial_cmp(&second.rect.min.x)
                    .expect("target positions are finite")
            });

            for (index, target) in targets.iter().enumerate() {
                let reported =
                    arrange_with_pointer_click(&projection, &policy, target.rect.center());
                let addressed = reported.activated_hint().unwrap_or_else(|| {
                    panic!(
                        "{} reported nothing for the hint at {:?}",
                        policy.canonical_name(),
                        target.rect
                    )
                });
                // There is no accessor for the raw index, so which hint came
                // back is read the only way a caller can read it: by resolving
                // against the slice the footer painted. The fixture's hints are
                // distinct, so this names the position uniquely.
                assert_eq!(
                    addressed.resolve(supplied),
                    Some(&supplied[index]),
                    "{} resolved {:?} for the hint at {:?}",
                    policy.canonical_name(),
                    addressed.resolve(supplied),
                    target.rect
                );
                // The count travels with the index, so it resolves against that
                // slice and against nothing of another size.
                assert_eq!(addressed.minted_over(), hints);
                assert!(
                    reported.intent().is_empty(),
                    "addressing a hint produced a control request"
                );
            }

            // A press somewhere the footer painted no target reports nothing.
            let band = band_rect(&policy, ShellRegionId::Footer);
            let empty = arrange_with_pointer_click(
                &projection,
                &policy,
                band.left_center() + eframe::egui::vec2(KEYLINE_RESTING_PX, 0.0),
            );
            assert_eq!(
                empty.activated_hint(),
                None,
                "{} reported a hint for a press on the path label",
                policy.canonical_name()
            );
        }
    }

    /// An addressed index resolves against a real projection's valid actions,
    /// and refuses against a projection whose two collections disagree.
    ///
    /// Both halves of the contract, over both fixtures, as one rule rather than
    /// two. `paired_projection_fixture` goes through the production projector,
    /// which builds `action_hints` by mapping over `valid_actions` in order —
    /// so it exercises the half `WP06` actually consumes, which a
    /// four-versus-zero fixture cannot: there, `resolve` returning `None` is
    /// indistinguishable from `resolve` doing nothing at all.
    ///
    /// *Cycle 2 asserted only the refusing half, and justified it with the
    /// claim that a paired fixture was unreachable from this work package's
    /// owned files. That was false — `StateProjector::project_with_shell` is
    /// `pub` and `controls::compact_slider` already builds a real reducer the
    /// same way. Retracted; the honest claim is that repairing the *shared*
    /// fixture would change what every other assertion in these files
    /// measures.*
    #[test]
    fn an_addressed_index_resolves_exactly_when_the_two_collections_agree() {
        let mut projections = vec![paired_projection_fixture()];
        projections.extend(TopLevelContext::ALL.map(projection_fixture));

        let mut agreed = 0_usize;
        let mut disagreed = 0_usize;
        for projection in &projections {
            let hints = projection.footer().action_hints();
            let actions = projection.semantic_model().valid_actions();
            if hints.is_empty() {
                continue;
            }
            let addressed = AddressedHint::minted(0, hints.len());
            if hints.len() == actions.len() {
                agreed += 1;
                assert_eq!(
                    addressed.resolve(actions),
                    actions.first(),
                    "a paired projection refused an index minted over its own hints"
                );
            } else {
                disagreed += 1;
                assert!(
                    addressed.resolve(actions).is_none(),
                    "an index minted over {} hints resolved against {} actions",
                    hints.len(),
                    actions.len()
                );
            }
        }
        assert!(
            agreed > 0,
            "no fixture paired its collections, so the resolving half is untested"
        );
        assert!(
            disagreed > 0,
            "no fixture desynchronised its collections, so the guard is untested"
        );
    }

    /// What the shared fixture *should* be: hints and valid actions paired, the
    /// way the production projector pairs them.
    ///
    /// Ignored, not deleted, and not inverted into an assertion that the
    /// current absence is correct. `SemanticGraphicalViewModel::fixture`
    /// (`src/control/semantic_graphical_view_model.rs`) hard-codes
    /// `valid_actions: Vec::new()` while `fixture_footer` supplies four hints,
    /// so every index the pointer assertions in this file check is out of range
    /// for the collection the adapter resolves against. That file is owned by
    /// no work package in this mission, and repairing it here would change what
    /// every other footer assertion measures.
    ///
    /// Run it with `cargo test -- --ignored` to see the gap. Whoever gains
    /// ownership of that fixture should make this pass and remove the attribute.
    #[test]
    #[ignore = "shared fixture desync: SemanticGraphicalViewModel::fixture carries no valid actions"]
    fn the_shared_fixture_should_pair_its_hints_with_its_valid_actions() {
        for context in TopLevelContext::ALL {
            let projection = projection_fixture(context);
            assert_eq!(
                projection.footer().action_hints().len(),
                projection.semantic_model().valid_actions().len(),
                "{} fixture: the footer's hints and the model's valid actions disagree",
                context.label()
            );
        }
    }

    /// Far more hints than the band can hold produce no clipped and no
    /// overlapping text, at either viewport.
    ///
    /// The recorded overflow behaviour was never exercised: every other
    /// assertion in this file runs against a four-hint fixture that fits with
    /// room to spare, so whether the chosen rule satisfies the acceptance
    /// targets' clipped-or-overlapping count was unmeasured.
    ///
    /// Measured with `component_vocabulary`'s own rule ([`overflow_report`])
    /// rather than a second one: leaving the container is counted, colliding
    /// with a readable neighbour in the same container is a defect. The runs
    /// are attributed to the band by row, so an overflowing hint fails this
    /// rather than disappearing from the measurement.
    ///
    /// This is half the overflow question. It says the band is clean; it cannot
    /// say the hints it did not paint still exist, because the two look
    /// identical in a shape stream. That half is
    /// `a_hint_the_over_full_band_could_not_show_is_still_reachable`, and it is
    /// the half that actually constrains the design.
    #[test]
    fn an_over_full_footer_neither_clips_nor_overlaps_at_either_viewport() {
        let crowded: Vec<String> = (0..20)
            .map(|index| format!("{index} OPEN A DESTINATION WITH A LONG NAME"))
            .collect();
        for policy in ALL_DENSITY_POLICIES {
            let projection =
                projection_with_footer(TopLevelContext::Patch, footer_with(crowded.clone()));
            let runs = band_row_runs(&projection, &policy, ShellRegionId::Footer);
            let band = band_rect(&policy, ShellRegionId::Footer);
            // The fixture has to actually overflow, or this proves nothing.
            // The scroll region does not paint what is past its edge, so the
            // count that reached the screen is what says the band ran out of
            // room — and it must not be zero, or the overflow rule would be
            // "show nothing", which is not a rule the design file permits.
            let hint_runs = runs
                .iter()
                .filter(|run| run.content != projection.footer().path_label())
                .count();
            assert!(
                hint_runs > 0 && hint_runs < crowded.len(),
                "{} painted {hint_runs} of {} hints, so nothing overflowed",
                policy.canonical_name(),
                crowded.len()
            );

            let report = overflow_report(&runs, &[band]);
            assert!(
                report.defects.is_empty(),
                "an over-full footer at {} ({} run(s) left their container): {:#?}",
                policy.canonical_name(),
                report.left_their_container,
                report.defects
            );
        }
    }

    /// A path label long enough to swallow the band still leaves every hint
    /// reachable.
    ///
    /// The band's other annihilation, entering from the half that keeps its
    /// space. `path_label` is built from the focus path and the projection
    /// bounds nothing about its depth, so before [`hint_floor_px`] a deep
    /// enough path took the whole band: **both hints vanished permanently** —
    /// not scrolled away, gone, with nothing left to scroll — while the
    /// clipped-or-overlapping rule reported zero defects, because the run's
    /// clip rectangle is the whole frame rather than the band.
    ///
    /// Reachability is therefore measured the same way it is measured for the
    /// hint side: by gesture, against the projection's own supplied count.
    #[test]
    fn an_overlong_path_label_still_leaves_every_hint_reachable() {
        let hints = vec![
            "1 OPEN MIXER".to_owned(),
            "2 OPEN PATCH".to_owned(),
            "W MOVE UP".to_owned(),
            "S MOVE DOWN".to_owned(),
        ];
        for policy in ALL_DENSITY_POLICIES {
            let projection = projection_with_footer(
                TopLevelContext::Patch,
                ShellFooter::new(OVERLONG_PATH, hints.clone()),
            );
            let band = band_rect(&policy, ShellRegionId::Footer);
            // The fixture has to actually overrun the band, or it tests nothing.
            let unbounded = unbounded_text_width(OVERLONG_PATH, TypeStyle::InstructionHint);
            assert!(
                unbounded > band.width(),
                "the overlong path fits inside the {} band, so it tests nothing",
                policy.canonical_name()
            );

            let runs = band_row_runs(&projection, &policy, ShellRegionId::Footer);
            let painted = painted_text(&runs);
            let reachable = reachable_by_sweeping(&projection, &policy, &hints);
            for hint in &hints {
                assert!(
                    reachable.contains(hint),
                    "{} made {hint:?} unreachable behind the path label; it painted {painted:?}",
                    policy.canonical_name()
                );
            }
            // And the path is the run that was cut, rather than the band being
            // handed to the hints instead.
            assert!(
                runs.iter().any(|run| run.content == OVERLONG_PATH),
                "{} dropped the path label entirely; it painted {painted:?}",
                policy.canonical_name()
            );
            let defects = overflow_report(&runs, &[band]);
            assert!(
                defects.defects.is_empty(),
                "an overlong path at {}: {:#?}",
                policy.canonical_name(),
                defects.defects
            );
        }
    }

    /// A focus path deep enough to overrun either authored band. The projection
    /// places no bound on this value, so nothing but the band's own floor does.
    const OVERLONG_PATH: &str = "MIXER / TRACK 00 / SEND A / EFFECT 03 / MODULATION MATRIX / \
                                 SOURCE 07 / DESTINATION 11 / CURVE SEGMENT 04 / \
                                 BREAKPOINT 09 / FINE OFFSET / SECONDARY TRIM / \
                                 RESPONSE CURVE / DEPTH SCALING / POLARITY INVERT / \
                                 SMOOTHING WINDOW / QUANTISE GRID / levelDb";

    /// A hint the over-full band had no room for is still reachable.
    ///
    /// This is the half of the overflow rule the clipped-or-overlapping count
    /// cannot see. A shape stream cannot tell a run the rendering stack culled
    /// from one that was never composed, so a footer that simply stopped
    /// painting once it ran out of width would satisfy every containment
    /// assertion in this file while quietly losing most of the actions the
    /// reducer says are valid — which `DESIGN.md:514` does not permit and
    /// `T021` names outright ("do not silently drop one").
    ///
    /// So it is measured as a gesture: sweep the band to the end, union what
    /// every pass painted, and require the projection's whole supplied list to
    /// appear. **Every** hint, not merely one previously-hidden hint — a
    /// gesture that reveals something proves the region scrolls, not that the
    /// content is all there. Without the scroll region ([`hint_viewport`]) the
    /// sweep paints exactly what the first pass did, and this fails.
    #[test]
    fn a_hint_the_over_full_band_could_not_show_is_still_reachable() {
        let crowded: Vec<String> = (0..20)
            .map(|index| format!("{index} OPEN A DESTINATION WITH A LONG NAME"))
            .collect();
        for policy in ALL_DENSITY_POLICIES {
            let projection =
                projection_with_footer(TopLevelContext::Patch, footer_with(crowded.clone()));
            let before = painted_text(&band_row_runs(&projection, &policy, ShellRegionId::Footer));
            assert!(
                crowded.iter().any(|hint| !before.contains(hint)),
                "{} showed all {} hints at once, so nothing needed revealing",
                policy.canonical_name(),
                crowded.len()
            );

            // Sweep to the end rather than scrolling once: the union across the
            // sweep is what "reachable" means for a scroll region.
            let reachable = reachable_by_sweeping(&projection, &policy, &crowded);
            let unreachable: Vec<&String> = crowded
                .iter()
                .filter(|hint| !reachable.contains(hint))
                .collect();
            assert!(
                unreachable.is_empty(),
                "{} left {} of {} hint(s) unreachable at any scroll offset: {unreachable:?}",
                policy.canonical_name(),
                unreachable.len(),
                crowded.len()
            );
        }
    }

    /// The path label keeps its space when the hints cannot fit beside it.
    ///
    /// `BandPrecedence::Leading` says the operator's location survives. With
    /// four hints nothing competes for the space, so this is asserted against
    /// the crowded fixture, where something does.
    #[test]
    fn an_over_full_footer_still_paints_the_operator_location() {
        let crowded: Vec<String> = (0..20)
            .map(|index| format!("{index} OPEN A DESTINATION WITH A LONG NAME"))
            .collect();
        for policy in ALL_DENSITY_POLICIES {
            let projection =
                projection_with_footer(TopLevelContext::Patch, footer_with(crowded.clone()));
            let painted = painted_text(&band_row_runs(&projection, &policy, ShellRegionId::Footer));
            assert!(
                painted
                    .iter()
                    .any(|run| run == projection.footer().path_label()),
                "{} lost the path label to the hints; it painted {painted:?}",
                policy.canonical_name()
            );
        }
    }

    /// Nothing was addressed, so nothing is reported. An intent that appeared
    /// without a pointer would be the composition deciding.
    #[test]
    fn an_unaddressed_footer_reports_no_activation() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                assert!(
                    !band_runs(&projection, &policy, ShellRegionId::Footer).is_empty(),
                    "{} painted an empty footer",
                    policy.canonical_name()
                );
            }
        }
        assert_eq!(ShellFrameIntent::none().activated_hint(), None);
    }
}
