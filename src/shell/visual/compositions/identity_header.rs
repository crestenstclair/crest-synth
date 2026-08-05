//! The identity band: what is being edited.
//!
//! **Components paint. Views compose. The reducer decides.**
//!
//! This composition fills [`ShellRegion::IdentityHeader`]. It replaces
//! `paint_identity_header` in `src/adapter/eframe_graphical_window.rs`, which
//! stacked the projection's two identity labels vertically.
//!
//! # Why the two labels sit side by side
//!
//! The design file's `Patch Identity` band is one row: an identity at the
//! leading edge, a one-pixel `Spacer` filling the gap, and metadata anchored to
//! the trailing edge. The stacked pair the adapter painted was a Phase 4a
//! choice made to keep two lines of authored type inside the smaller band; one
//! row needs only the taller of the two line heights, so it fits the compact
//! band with more room, not less.
//!
//! The design file's band carries three text nodes — a short `Patch Label`
//! before the name, the name itself, and the trailing metadata. The projection
//! carries two: [`crate::control::ShellIdentityHeader`] has a primary and a
//! secondary label and no third value. The short leading label is therefore
//! omitted rather than filled in, which is `C-003`: designed structure with no
//! view data behind it is left out, never approximated.
//!
//! # Long labels
//!
//! **Raised, not resolved.** A Patch name can be longer than the band, and the
//! design file does not say what happens then: its identity nodes are sized at
//! the desktop viewport and carry no truncation, shrink, or marquee
//! annotation, and there is no compact frame for the band at all. What is not
//! ambiguous is the invariant — the gallery witness asserts zero clipped or
//! overlapping text, and `component_vocabulary`'s fixed-band check fails the
//! build on a run that escapes this band. So the composition takes the one
//! behaviour that cannot produce a cut glyph, and applies it to exactly one of
//! the two labels.
//!
//! The trailing metadata claims its space first at its natural width — it is a
//! generated `MIDI CH nn · engine` string, bounded by the vocabularies it is
//! built from — and the identity, which is a user-supplied Patch name and the
//! one unbounded value here, is laid out into what is left and ends in the
//! rendering stack's ellipsis when it does not fit. So the primary label is
//! painted through the band rhythm's `TruncatingHalf::fitted_text`
//! and the secondary through
//! [`crate::shell::visual::primitives::text::paint_text`] — not both, because
//! two truncating runs in one band give the whole band to whichever was laid
//! out first. That matches the design file's trailing-anchored `Patch Meta`,
//! and it is the asymmetry [`super::application_shell::BandPrecedence`]
//! requires. The types make truncating in the first-allocated half impossible;
//! which half that is remains a choice, and the one that holds it is
//! `an_overlong_label_neither_clips_nor_overlaps_at_either_viewport` below.
//!
//! # What it owns
//!
//! Nothing. The band height comes from [`ViewportDensityPolicy`], the inset
//! from its content rhythm, and both type styles from the token vocabulary.

use eframe::egui::Ui;

use crate::control::GraphicalShellProjection;
use crate::shell::visual::compositions::CompositionIntent;
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::text;
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, TypeStyle};

/// The type style the identity itself reads in.
pub const PRIMARY_STYLE: TypeStyle = TypeStyle::HeadingSection;

/// The type style the trailing metadata reads in.
pub const SECONDARY_STYLE: TypeStyle = TypeStyle::BodyCompact;

/// Arranges the identity band.
///
/// Returns no intent: the band names what is being edited and holds no
/// focusable control.
pub fn render(
    ui: &mut Ui,
    projection: &GraphicalShellProjection,
    density: &ViewportDensityPolicy,
) -> CompositionIntent {
    let identity = projection.identity_header();
    super::application_shell::band_row(
        ui,
        density,
        // The Patch metadata is anchored to the trailing edge in the design
        // file and keeps its space; an overlong identity is what gives way.
        super::application_shell::BandPrecedence::Trailing,
        |half| {
            // Laid out first, so it asks for its natural width: the metadata is
            // a generated `MIDI CH nn · engine` string, bounded by the
            // vocabularies it is built from.
            text::paint_text(
                half.ui(),
                identity.secondary_label(),
                SECONDARY_STYLE,
                SemanticColor::TextSecondary,
                ComponentState::Resting,
            );
        },
        |half| {
            // The Patch name is operator-supplied and unbounded, so it is the
            // run that gives way — and the band's types allow it to give way
            // here and nowhere else.
            half.fitted_text(
                identity.primary_label(),
                PRIMARY_STYLE,
                SemanticColor::TextPrimary,
            );
        },
    );
    CompositionIntent::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{ShellIdentityHeader, TopLevelContext};
    use crate::shell::visual::compositions::application_shell::testing_support::{
        band_rect, band_row_runs, band_runs, containment_defects, painted_text, projection_fixture,
        projection_with_identity, unbounded_text_width,
    };
    use crate::shell::visual::compositions::{ShellComposition, ShellRegion};
    use crate::shell::visual::density::ALL_DENSITY_POLICIES;
    use crate::shell::visual::token::ALL_TYPE_STYLES;
    use crate::shell::ShellRegionId;

    /// A name far longer than either band is wide, in either of the two styles
    /// the band sets — the assertion below refuses to run against a fixture
    /// that fits.
    const OVERLONG: &str = "PATCH 01 · A PATCH NAME LONG ENOUGH THAT IT CANNOT POSSIBLY FIT \
                            INSIDE EITHER AUTHORED IDENTITY BAND WITHOUT BEING CUT SHORT BY \
                            SOMETHING, AT EITHER OF THE TWO AUTHORED VIEWPORT WIDTHS, IN \
                            EITHER OF THE TWO TYPE STYLES THE BAND SETS, WITH ENOUGH WORDS \
                            LEFT OVER AFTERWARDS THAT NOBODY READING THIS COULD MISTAKE IT \
                            FOR A NAME THAT MIGHT HAVE FITTED ON A WIDER DISPLAY SOMEWHERE";

    #[test]
    fn the_identity_band_is_bound_to_its_observed_region() {
        assert_eq!(
            ShellComposition::IdentityHeader.region(),
            ShellRegion::IdentityHeader
        );
    }

    /// Both labels reach the screen at both viewports.
    #[test]
    fn both_identity_labels_are_painted_at_both_viewports() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let identity = projection.identity_header();
                let text = painted_text(&band_runs(
                    &projection,
                    &policy,
                    ShellRegionId::IdentityHeader,
                ));
                for expected in [identity.primary_label(), identity.secondary_label()] {
                    assert!(
                        text.iter().any(|run| run == expected),
                        "{} did not paint {expected:?}; it painted {text:?}",
                        policy.canonical_name()
                    );
                }
                assert_eq!(
                    text.len(),
                    2,
                    "{} painted {text:?}",
                    policy.canonical_name()
                );
            }
        }
    }

    /// The band paints only what the projection slice holds.
    #[test]
    fn the_band_paints_only_text_the_projection_supplied() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = projection_fixture(context);
                let identity = projection.identity_header();
                for run in painted_text(&band_runs(
                    &projection,
                    &policy,
                    ShellRegionId::IdentityHeader,
                )) {
                    assert!(
                        run == identity.primary_label() || run == identity.secondary_label(),
                        "the identity band invented {run:?} at {}",
                        policy.canonical_name()
                    );
                }
            }
        }
    }

    /// A label longer than the band neither clips nor overlaps, at either
    /// viewport. This is the assertion the raised Figma ambiguity is settled
    /// against: whatever the design file eventually says, it may not produce a
    /// cut glyph.
    #[test]
    fn an_overlong_label_neither_clips_nor_overlaps_at_either_viewport() {
        for policy in ALL_DENSITY_POLICIES {
            for context in TopLevelContext::ALL {
                let projection = overlong_projection(context);
                let band = band_rect(&policy, ShellRegionId::IdentityHeader);
                let runs = band_row_runs(&projection, &policy, ShellRegionId::IdentityHeader);
                assert!(
                    !runs.is_empty(),
                    "{} painted nothing for an overlong identity",
                    policy.canonical_name()
                );
                let defects = containment_defects(&runs, band);
                assert!(
                    defects.is_empty(),
                    "an overlong identity at {} in {}: {defects:#?}",
                    policy.canonical_name(),
                    context.label()
                );
                // The rendering stack keeps the original string on the layout
                // job and cuts the rows, so what proves the cut is geometry
                // rather than text: this label needs more width than the band
                // has, and every run it produced still fits inside it.
                let unbounded = unbounded_text_width(OVERLONG, PRIMARY_STYLE);
                assert!(
                    unbounded > band.width(),
                    "the overlong fixture fits inside the {} band, so it tests nothing",
                    policy.canonical_name()
                );
                // The bounded metadata survives whole; the unbounded identity
                // is the run that gave way.
                assert!(
                    painted_text(&runs)
                        .iter()
                        .any(|run| run == projection.identity_header().secondary_label()),
                    "{} cut the bounded metadata instead of the identity",
                    policy.canonical_name()
                );
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

    /// The two styles are authored ones, and they differ — an identity and its
    /// metadata that read identically would be one run of text in two pieces.
    #[test]
    fn both_labels_take_authored_styles_and_do_not_read_alike() {
        assert!(ALL_TYPE_STYLES.contains(&PRIMARY_STYLE));
        assert!(ALL_TYPE_STYLES.contains(&SECONDARY_STYLE));
        assert_ne!(PRIMARY_STYLE, SECONDARY_STYLE);
        assert_ne!(
            PRIMARY_STYLE.metrics().size_px,
            SECONDARY_STYLE.metrics().size_px
        );
    }

    /// Both authored bands are taller than the type they carry, so neither
    /// label is squeezed vertically by the compact policy.
    #[test]
    fn both_authored_bands_hold_the_type_they_carry() {
        for policy in ALL_DENSITY_POLICIES {
            let tallest = PRIMARY_STYLE
                .metrics()
                .line_height_px
                .max(SECONDARY_STYLE.metrics().line_height_px);
            assert!(
                policy.bands().identity_header_px > tallest,
                "{} identity band is {} px for {} px of type",
                policy.canonical_name(),
                policy.bands().identity_header_px,
                tallest
            );
        }
    }

    /// A projection whose Patch name is far too long for either band, with the
    /// metadata the projector actually generates beside it. That is the shape
    /// the risk has: the name is operator-supplied and unbounded, the metadata
    /// is generated and short.
    fn overlong_projection(context: TopLevelContext) -> GraphicalShellProjection {
        projection_with_identity(
            context,
            ShellIdentityHeader::new(OVERLONG, format!("MIDI CH 01 · {} engine", context.label())),
        )
    }
}
