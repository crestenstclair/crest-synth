//! The context line: product, the PATCH / MIXER switch, and readiness.
//!
//! **Components paint. Views compose. The reducer decides.**
//!
//! This composition fills [`ShellRegion::ContextLine`]. It replaces
//! `paint_context_line` in `src/adapter/eframe_graphical_window.rs`, which
//! painted the product label, the *active* context label, the interaction mode,
//! and the readiness label across a horizontal strip split by two fractions the
//! adapter carried itself.
//!
//! # Why both contexts are painted, and not just the active one
//!
//! `DESIGN.md` names PATCH and MIXER as the only two top-level contexts, and the
//! design file's `Context Switch` component set is a *switch*: both entries are
//! on screen, one marked. The adapter showed only the active label, which reads
//! as a title rather than as a control an operator can move between. The switch
//! is the authored shape, so the match below is over
//! [`TopLevelContext`] exhaustively — a third context would be a compile error
//! here, which is the correct outcome for a product invariant.
//!
//! # Distinguishing the active entry without color
//!
//! The design file's `Context Switch` carries three states — default, selected,
//! and focused — and the selected one is marked with a leading `*`
//! ([`ACTIVE_CONTEXT_MARK`]), not with color alone. That mark is what this
//! composition paints. The inactive entry reserves the same column
//! ([`INACTIVE_CONTEXT_MARK`]) so that changing context does not shift either
//! label sideways — the same reason [`crate::shell::visual::primitives::focus`]
//! reserves its cursor column.
//!
//! The mark and the label are **two runs**, not one `"* PATCH"` string. That is
//! forced by two frozen consumers of the frame observation which disagree about
//! what the context line's label may be; see [`entry_mark`] for both and for
//! why splitting satisfies them together.
//!
//! The design file's third state, *focused*, is deliberately not painted. Focus
//! lives on controls inside a surface; the immutable projection carries no focus
//! for the context switch itself, and painting a focused entry with no view data
//! behind it is the placeholder `C-003` forbids.
//!
//! # Raised, not resolved
//!
//! The design file's `Context Line` is authored at the desktop viewport only —
//! there is no compact frame for this band, as there is none for `Patch
//! Identity` or `Controls` — so it does not say what the band does when its
//! content stops fitting. The switch, the mode, and readiness are what the band
//! is *for*, so they are laid out first and keep their space; the product name
//! is laid out into what is left and is the run that ends in an ellipsis. That
//! is `BandPrecedence::Trailing` on the shared band rhythm, and it is a
//! choice this composition records rather than one the design file made. A
//! compact rule — a shorter product mark, a dropped readiness label — is a
//! design decision, not a layout to invent here.
//!
//! # What it owns
//!
//! Nothing. Every extent comes from [`ViewportDensityPolicy`], every color and
//! type style from the token vocabulary, and every string from the immutable
//! projection or from the closed context and mode vocabularies.

use eframe::egui::Ui;

use crate::control::{GraphicalShellProjection, InteractionMode, TopLevelContext};
use crate::shell::visual::compositions::CompositionIntent;
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::text;
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};

/// The mark the design file places before the selected context.
///
/// A glyph, not a color: an operator who cannot separate two accents still sees
/// which context is active.
pub const ACTIVE_CONTEXT_MARK: &str = "*";

/// What an unselected context carries in the mark column.
///
/// One space in the authored monospace family is exactly one mark wide, so both
/// entries' labels start on the same column and the switch does not jump when
/// the context changes.
pub const INACTIVE_CONTEXT_MARK: &str = " ";

/// What one context entry carries in the mark column.
///
/// # Why the mark is a separate run from the label
///
/// The mark and the label are painted as two runs rather than as one
/// `"* PATCH"` string, and that is a hard constraint rather than a stylistic
/// choice. Two frozen consumers read the context line's observed label and they
/// disagree about what it may be:
///
/// - `tests/graphical_application_shell.rs` requires the observation's label to
///   equal some galley the frame **painted**, whole-string.
/// - `src/testing/live_demo_runner.rs` requires it to equal
///   `context_line().context_label()` — the bare `"PATCH"` — and returns
///   `LiveDemoError::ShellFrameMismatch` otherwise.
///
/// A concatenated `"* PATCH"` can satisfy the first or the second, never both.
/// Splitting the runs satisfies both at once: the band paints a galley that is
/// exactly `"PATCH"`, and it also paints the mark. Nothing about the switch is
/// lost, because what distinguishes the active entry without colour is the mark
/// *existing*, not the mark sharing a galley with the label.
///
/// The inactive entry carries one space, which in the authored monospace family
/// is exactly one mark wide, so both labels start on the same column and the
/// switch does not jump when the context changes — the same reason
/// [`crate::shell::visual::primitives::focus`] reserves its cursor column.
pub const fn entry_mark(active: bool) -> &'static str {
    if active {
        ACTIVE_CONTEXT_MARK
    } else {
        INACTIVE_CONTEXT_MARK
    }
}

/// The color role one context entry paints in.
///
/// Colour is the *second* signal here, not the only one; see
/// [`ACTIVE_CONTEXT_MARK`].
pub const fn entry_color(active: bool) -> SemanticColor {
    if active {
        SemanticColor::TextPrimary
    } else {
        SemanticColor::TextMuted
    }
}

/// The color role the interaction mode reads in.
///
/// Moved verbatim from the adapter's `paint_context_line`: adjustment is the
/// one mode with an accent of its own, and the two modes this phase cannot
/// reach are muted rather than dropped. Exhaustive, so a fifth mode is a
/// compile error here rather than a mode that paints as `Navigate`.
pub const fn mode_color(mode: InteractionMode) -> SemanticColor {
    match mode {
        InteractionMode::Navigate => SemanticColor::TextPrimary,
        InteractionMode::Adjust => SemanticColor::AccentAdjust,
        InteractionMode::Modal | InteractionMode::MultiSelect => SemanticColor::TextMuted,
    }
}

/// Arranges the context line.
///
/// Returns no intent: the band carries chrome and a context indicator, and no
/// focusable control lives in it. An empty aggregate is the honest answer, not
/// a missing one.
pub fn render(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let line = projection.context_line();
    let active = projection.context();
    let mode = projection.semantic_model().interaction_mode();

    super::application_shell::band_row(
        ui,
        density,
        // The switch, the mode, and readiness are the functional half of the
        // band; the product name is what gives way.
        super::application_shell::BandPrecedence::Trailing,
        |half| {
            // The half that keeps its space. A right-to-left layout places the
            // first widget added at the right edge, so this group is added in
            // reverse reading order and lands as: PATCH, MIXER, mode, status.
            let ui = half.ui();
            text::paint_text(
                ui,
                line.status_label(),
                TypeStyle::InstructionHint,
                SemanticColor::TextMuted,
                ComponentState::Resting,
            );
            text::paint_text(
                ui,
                mode.label(),
                TypeStyle::LabelControl,
                mode_color(mode),
                ComponentState::Resting,
            );
            for context in TopLevelContext::ALL.into_iter().rev() {
                let selected = context == active;
                // The label first, so that in this right-to-left group it lands
                // to the *right* of its own mark and the entry reads `* PATCH`.
                // Two runs, not one string — see [`entry_mark`].
                text::paint_text(
                    ui,
                    context.label(),
                    TypeStyle::LabelControl,
                    entry_color(selected),
                    ComponentState::Resting,
                );
                text::paint_text(
                    ui,
                    entry_mark(selected),
                    TypeStyle::LabelControl,
                    entry_color(selected),
                    ComponentState::Resting,
                );
            }
        },
        |half| {
            // Laid out second, into what the switch left, so this is the run
            // that gives way — and the only one the band's type allows to.
            half.fitted_text(
                line.product_label(),
                TypeStyle::LabelControl,
                SemanticColor::TextPrimary,
            );
        },
    );
    CompositionIntent::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ShellContextLine;
    use crate::shell::visual::compositions::application_shell::testing_support::{
        band_rect, band_row_runs, band_runs, containment_defects, painted_text, projection_fixture,
        projection_with_context_line, unbounded_text_width,
    };
    use crate::shell::visual::compositions::{ShellComposition, ShellRegion};
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::ShellRegionId;

    #[test]
    fn the_context_line_is_bound_to_its_observed_region() {
        assert_eq!(
            ShellComposition::ContextSwitch.region(),
            ShellRegion::ContextLine
        );
    }

    /// Both top-level contexts are on screen at once, each as a galley equal to
    /// its own bare label, and exactly one mark is painted.
    ///
    /// The bare-label half is load-bearing beyond the switch: the frame
    /// observation reports the context line under `context_label()`, and
    /// `tests/graphical_application_shell.rs` requires that value to equal a
    /// painted galley whole-string. A concatenated `"* PATCH"` satisfies the
    /// switch and fails that, which is why the mark is its own run.
    #[test]
    fn both_top_level_contexts_are_painted_and_exactly_one_is_marked() {
        for policy in ALL_DENSITY_POLICIES {
            for active in TopLevelContext::ALL {
                let projection = projection_fixture(active);
                let runs = band_runs(&projection, &policy, ShellRegionId::ContextLine);
                let text = painted_text(&runs);
                for context in TopLevelContext::ALL {
                    assert!(
                        text.iter().any(|run| run == context.label()),
                        "{} at {} does not paint {:?} as a run of its own; it painted {text:?}",
                        active.label(),
                        policy.canonical_name(),
                        context.label()
                    );
                }
                let marks = text
                    .iter()
                    .filter(|run| run.as_str() == ACTIVE_CONTEXT_MARK)
                    .count();
                assert_eq!(
                    marks,
                    1,
                    "{} at {} painted {marks} marks; it painted {text:?}",
                    active.label(),
                    policy.canonical_name()
                );

                // ...and it is the *active* entry that carries it.
                //
                // Counting marks says the switch marks something; it does not
                // say it marks the right thing. Until the mark became a run of
                // its own this was `marked[0].contains(active.label())`, which
                // a split run cannot express — so the invariant is measured
                // where it now lives, in geometry: the mark is painted
                // immediately before its own entry's label, so the label it
                // sits nearest must be the active one. Painting the mark beside
                // the inactive entry inverts what an operator reads off the
                // switch and is caught here.
                let mark = runs
                    .iter()
                    .find(|run| run.content == ACTIVE_CONTEXT_MARK)
                    .expect("the active mark is painted");
                let label_of = |context: TopLevelContext| {
                    runs.iter()
                        .find(|run| run.content == context.label())
                        .unwrap_or_else(|| panic!("{} is painted as its own run", context.label()))
                };
                let inactive = TopLevelContext::ALL
                    .into_iter()
                    .find(|context| *context != active)
                    .expect("there are exactly two top-level contexts");
                let gap_to = |context: TopLevelContext| {
                    (label_of(context).rect.min.x - mark.rect.max.x).abs()
                };
                assert!(
                    gap_to(active) < gap_to(inactive),
                    "{} at {}: the mark sits {} px from {} and {} px from {}, so it marks the \
                     inactive entry",
                    active.label(),
                    policy.canonical_name(),
                    gap_to(active),
                    active.label(),
                    gap_to(inactive),
                    inactive.label()
                );
            }
        }
    }

    /// The mark, not the color, is what separates the two entries, and the
    /// entry that is not marked still reserves the column.
    ///
    /// Measured off the frame rather than asserted about strings: both labels
    /// must land at the same x whichever context is active, or the switch jumps
    /// sideways when the operator changes context.
    #[test]
    fn the_active_entry_is_distinguishable_from_the_inactive_one_without_color() {
        assert_ne!(entry_mark(true), entry_mark(false));
        assert_eq!(
            entry_mark(true).chars().count(),
            entry_mark(false).chars().count(),
            "the two marks are different widths, so the switch shifts when it changes"
        );
        assert_ne!(entry_color(true), entry_color(false));

        for policy in ALL_DENSITY_POLICIES {
            let mut placements: Vec<Vec<(String, f32)>> = Vec::new();
            for active in TopLevelContext::ALL {
                let projection = projection_fixture(active);
                let runs = band_runs(&projection, &policy, ShellRegionId::ContextLine);
                placements.push(
                    TopLevelContext::ALL
                        .into_iter()
                        .map(|context| {
                            let run = runs
                                .iter()
                                .find(|run| run.content == context.label())
                                .expect("each context label is painted as its own run");
                            (run.content.clone(), run.rect.min.x)
                        })
                        .collect(),
                );
            }
            assert_eq!(
                placements[0],
                placements[1],
                "{} moves the context labels when the active context changes",
                policy.canonical_name()
            );
        }
    }

    /// Every run the band paints is text the projection or a closed vocabulary
    /// supplied. Nothing is invented for the band to have something to show.
    #[test]
    fn the_band_paints_only_text_the_projection_supplied() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let line = projection.context_line();
                let mode = projection.semantic_model().interaction_mode();
                let permitted: Vec<String> = TopLevelContext::ALL
                    .into_iter()
                    .map(|entry| entry.label().to_owned())
                    .chain([
                        entry_mark(true).to_owned(),
                        entry_mark(false).to_owned(),
                        line.product_label().to_owned(),
                        line.status_label().to_owned(),
                        mode.label().to_owned(),
                    ])
                    .collect();
                for run in
                    painted_text(&band_runs(&projection, &policy, ShellRegionId::ContextLine))
                {
                    assert!(
                        permitted.contains(&run),
                        "the context line invented {run:?} at {}",
                        policy.canonical_name()
                    );
                }
            }
        }
    }

    #[test]
    fn every_declared_interaction_mode_resolves_to_a_declared_color() {
        for mode in InteractionMode::ALL {
            assert!(
                crate::shell::visual::token::ALL_COLORS.contains(&mode_color(mode)),
                "{} resolved outside the vocabulary",
                mode.label()
            );
        }
        assert_eq!(
            mode_color(InteractionMode::Adjust),
            SemanticColor::AccentAdjust
        );
    }

    /// Nothing in the context line clips or overlaps at either viewport. This
    /// band is one of the two the delivered suite asserts containment for
    /// strictly, because it has no scroll region to hide an overflow in.
    #[test]
    fn no_context_line_run_clips_or_overlaps_at_either_viewport() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let defects = containment_defects(
                    &band_runs(&projection, &policy, ShellRegionId::ContextLine),
                    band_rect(&policy, ShellRegionId::ContextLine),
                );
                assert!(
                    defects.is_empty(),
                    "the context line at {} in {}: {defects:#?}",
                    policy.canonical_name(),
                    context.label()
                );
            }
        }
    }

    /// A product label far longer than the band still neither clips nor
    /// overlaps, and the switch is what keeps its space.
    ///
    /// The band's precedence rule — the switch, the mode, and readiness are
    /// laid out first; the product name gives way — was applied and documented
    /// but never forced: `projection_fixture`'s `"CREST SYNTH"` fits at both
    /// viewports with room to spare, so inverting the rule left every assertion
    /// in this file green. This is the fixture that does not.
    ///
    /// The band is one of the two the acceptance targets treat as fixed, where
    /// a run leaving the container is a hard defect rather than a count, so
    /// containment is asserted strictly here.
    #[test]
    fn an_overlong_product_label_neither_clips_nor_overlaps_at_either_viewport() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = overlong_product_projection(context);
                let band = band_rect(&policy, ShellRegionId::ContextLine);
                let runs = band_row_runs(&projection, &policy, ShellRegionId::ContextLine);
                assert!(
                    !runs.is_empty(),
                    "{} painted nothing for an overlong product label",
                    policy.canonical_name()
                );
                // The rendering stack keeps the original string on a truncated
                // layout job, so the proof that the fixture forces the rule is
                // geometry: this label needs more width than the whole band.
                let unbounded = unbounded_text_width(OVERLONG_PRODUCT, TypeStyle::LabelControl);
                assert!(
                    unbounded > band.width(),
                    "the overlong product fixture fits inside the {} band, so it tests nothing",
                    policy.canonical_name()
                );
                let defects = containment_defects(&runs, band);
                assert!(
                    defects.is_empty(),
                    "an overlong product label at {} in {}: {defects:#?}",
                    policy.canonical_name(),
                    context.label()
                );

                // The switch, the mode, and readiness are what the band is for,
                // so they survive whole; the product name is the run that was
                // cut. Both halves truncating would give the band to whichever
                // was laid out first and paint the other on top of it.
                let painted = painted_text(&runs);
                for expected in [
                    context.label().to_owned(),
                    entry_mark(true).to_owned(),
                    projection.context_line().status_label().to_owned(),
                    projection
                        .semantic_model()
                        .interaction_mode()
                        .label()
                        .to_owned(),
                ] {
                    assert!(
                        painted.iter().any(|run| run == &expected),
                        "{} cut {expected:?} instead of the product label; it painted {painted:?}",
                        policy.canonical_name()
                    );
                }
                for run in &runs {
                    assert!(
                        run.rect.width() < unbounded,
                        "{} painted {:?} at its full {unbounded} px width",
                        policy.canonical_name(),
                        run.content
                    );
                }
            }
        }
    }

    /// A product name far wider than either authored band, in the style the
    /// band sets — the assertion above refuses to run against one that fits.
    const OVERLONG_PRODUCT: &str = "CREST SYNTH BUILT FOR A PRODUCT NAME LONG ENOUGH THAT IT \
                                    CANNOT POSSIBLY FIT INSIDE EITHER AUTHORED CONTEXT LINE \
                                    ALONGSIDE THE SWITCH, THE INTERACTION MODE, AND THE \
                                    READINESS LABEL, AT EITHER OF THE TWO AUTHORED VIEWPORT \
                                    WIDTHS, WITH ENOUGH WORDS LEFT OVER AFTERWARDS THAT NOBODY \
                                    COULD MISTAKE IT FOR ONE THAT MIGHT HAVE FITTED";

    fn overlong_product_projection(context: TopLevelContext) -> GraphicalShellProjection {
        projection_with_context_line(
            context,
            ShellContextLine::new(OVERLONG_PRODUCT, context.label(), "READY"),
        )
    }
}
