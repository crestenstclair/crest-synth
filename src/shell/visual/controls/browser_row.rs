//! The browser row — a row naming an asset, which opens a browser rather than
//! adjusting.
//!
//! # Figma source
//!
//! `SCREEN · Sample Browser · 1920×1080` → the `Browser / …` component
//! instances (nodes `41:235` through `41:270`), which are the `CLI Browser
//! Line` component set. Four columns, left to right:
//!
//! | Column   | Role                                                  |
//! |----------|-------------------------------------------------------|
//! | `Prefix` | the reserved cursor column                            |
//! | `Kind`   | a bracketed type tag — `[..]`, `[d]`, `[f]`, `[x]`    |
//! | `Label`  | the entry's name                                      |
//! | `Meta`   | per-entry metadata, e.g. a duration                   |
//!
//! The bracketed tag is the specimen's own non-color signal: the row type is
//! legible as text before any accent is applied. This module keeps that
//! anatomy and resolves the tag from [`AssetKind`] — the *form* is authored,
//! the per-kind spelling is not, and that is raised in the handoff.
//!
//! # No waveform lives in the row
//!
//! `DESIGN.md:370` describes the Sample Browser as supporting metadata and
//! waveform preview, and the specimen shows where: the waveform is the
//! `Preview` region beside the list (node `41:278`), not part of a row. A row
//! carries a tag, a name, and a metadata column. This control therefore paints
//! no waveform at all rather than a decorative one.
//!
//! The metadata column has nothing to fill it — [`SemanticControlValue::Asset`]
//! carries a kind and a locator and no duration, format, or size — so it is
//! marked explicitly unavailable rather than left blank or filled with
//! something invented.
//!
//! # Not implemented here
//!
//! Hold-to-preview. That is Phase 7's Sample Browser workflow and the Start
//! control is reserved and unbound until then.
//!
//! [`AssetKind`]: crate::synth::AssetKind

use eframe::egui::{pos2, Painter, Rect, Ui};

use super::parameter_row::{
    allocate_row, intent_for, paint_row_chrome, status_detail, value_text, UNAVAILABLE_MARK,
};
use super::{ControlIntent, PresentationRole};
use crate::control::{SemanticControlValue, SemanticControlViewModel};
use crate::shell::visual::density::ViewportDensityPolicy;
use crate::shell::visual::primitives::{focus, status, text, value};
use crate::shell::visual::state::ComponentState;
use crate::shell::visual::token::{SemanticColor, SpacingStep, TypeStyle};
use crate::synth::AssetKind;

/// The tag a SoundFont asset row carries.
pub const SOUND_FONT_TAG: &str = "[sf]";

/// The tag a sample asset row carries.
pub const SAMPLE_TAG: &str = "[f]";

/// The tag an asset of any other kind carries.
pub const OTHER_TAG: &str = "[a]";

/// The tag a row whose value is not an asset carries.
///
/// Selection only ever routes an `Asset` kind here, so this is the honest
/// reading of a value that arrived from somewhere else rather than a silent
/// re-labelling of it as a file.
pub const NON_ASSET_TAG: &str = "[?]";

/// Paints a browser row and returns what the operator asked for.
///
/// A locked asset row — a SoundFont file row on PATCH — arrives as
/// [`ComponentState::Disabled`] and renders the authored `Locked` mark. Locked
/// is a state this row is handed, never one it decides.
pub fn render(
    ui: &mut Ui,
    view: &SemanticControlViewModel,
    state: ComponentState,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> ControlIntent {
    let response = allocate_row(ui, role, density);
    let painter = ui.painter_at(response.rect);
    let row = response.rect;
    paint_row_chrome(&painter, row, state);

    let tag_right = paint_kind_tag(&painter, row, kind_tag(view.value()), state);
    let name = value_text(view.value());
    paint_name(&painter, row, tag_right, &name, state);

    // The metadata column carries the row's lifecycle when it has one, and
    // otherwise says explicitly that no metadata arrived.
    let meta = metadata_region(row);
    let detail = status_detail(view);
    if status::status_mark(state, detail).is_some() {
        status::paint_status_mark(&painter, meta, state, detail);
    } else {
        paint_unavailable_metadata(&painter, meta, state);
    }

    intent_for(&response, ControlIntent::OptionListRequested)
}

/// The bracketed type tag for one value.
///
/// Exhaustive over the value union and over [`AssetKind`], with no wildcard, so
/// a new asset kind is a compile error here rather than a row silently tagged
/// as something it is not.
pub(super) fn kind_tag(value: &SemanticControlValue) -> &'static str {
    match value {
        SemanticControlValue::Asset(reference) => match reference.kind() {
            AssetKind::SoundFont => SOUND_FONT_TAG,
            AssetKind::Sample => SAMPLE_TAG,
            AssetKind::Other => OTHER_TAG,
        },
        SemanticControlValue::Scalar(_)
        | SemanticControlValue::Parameter(_)
        | SemanticControlValue::Identity(_)
        | SemanticControlValue::Summary(_) => NON_ASSET_TAG,
    }
}

/// Paints the type tag after the reserved cursor column and returns the x it
/// ends at.
fn paint_kind_tag(painter: &Painter, row: Rect, tag: &str, state: ComponentState) -> f32 {
    let galley = text::layout(
        painter,
        tag,
        TypeStyle::LabelControl,
        SemanticColor::TextMuted,
        state,
    );
    let size = galley.size();
    let left = row.min.x + focus::LABEL_START_X_PX;
    painter.galley(
        pos2(left, row.center().y - size.y / 2.0),
        galley,
        text::resolved_color(SemanticColor::TextMuted, state).resolve(),
    );
    left + size.x
}

/// Paints the entry's name after the type tag.
fn paint_name(painter: &Painter, row: Rect, tag_right: f32, name: &str, state: ComponentState) {
    let galley = text::layout(
        painter,
        name,
        TypeStyle::BodyCompact,
        SemanticColor::TextPrimary,
        state,
    );
    let size = galley.size();
    painter.galley(
        pos2(
            tag_right + SpacingStep::S16.resolve(),
            row.center().y - size.y / 2.0,
        ),
        galley,
        text::resolved_color(SemanticColor::TextPrimary, state).resolve(),
    );
}

/// The metadata column, at the trailing edge of the row.
fn metadata_region(row: Rect) -> Rect {
    let right = row.max.x - SpacingStep::S12.resolve();
    let left = right - SpacingStep::S32.resolve();
    Rect::from_min_max(pos2(left.min(right), row.min.y), pos2(right, row.max.y))
}

/// Says explicitly that no metadata arrived for this row.
fn paint_unavailable_metadata(painter: &Painter, region: Rect, state: ComponentState) {
    value::paint_value(
        painter,
        region.max.x,
        region.center().y,
        UNAVAILABLE_MARK,
        state,
    );
}

#[cfg(test)]
mod tests {
    use super::super::parameter_row::probe::{paint, projected_control, Painted};
    use super::*;
    use crate::control::TopLevelContext;
    use crate::shell::visual::controls::ComponentControl;
    use crate::shell::visual::primitives::status::{LoadingPhase, GENERIC_FAILURE_WORD};
    use crate::shell::visual::state::{NonColorSignal, LOADING_PROGRESS_WORDS};
    use crate::synth::{AssetReference, ParameterValue};

    const CONTROL: ComponentControl = ComponentControl::BrowserRow;

    fn painted(state: ComponentState, density: ViewportDensityPolicy) -> Painted {
        paint(
            CONTROL,
            &projected_control(TopLevelContext::Patch),
            state,
            PresentationRole::ListedRow,
            density,
        )
    }

    /// Every applicable state announces itself without color.
    #[test]
    fn every_applicable_state_renders_evidence_that_is_not_a_color() {
        let resting = painted(ComponentState::Resting, ViewportDensityPolicy::Desktop);
        for state in CONTROL.applicable_states() {
            let shown = painted(*state, ViewportDensityPolicy::Desktop);
            match state.appearance().signal {
                NonColorSignal::Word(word) => assert!(
                    shown.says(word),
                    "{} does not paint its authored word",
                    state.canonical_name()
                ),
                NonColorSignal::ProgressWord => assert!(
                    shown.says(LoadingPhase::Preparing.word()),
                    "{} does not paint a progress word",
                    state.canonical_name()
                ),
                NonColorSignal::TypedFailure => assert!(
                    shown.says(GENERIC_FAILURE_WORD) || shown.texts.len() > resting.texts.len(),
                    "{} does not paint failure text",
                    state.canonical_name()
                ),
                NonColorSignal::Shape => {
                    if *state == ComponentState::Resting {
                        continue;
                    }
                    assert_ne!(
                        shown.colorless_signature(),
                        resting.colorless_signature(),
                        "{} is distinguishable from Resting by color alone",
                        state.canonical_name()
                    );
                }
            }
        }
    }

    /// A locked asset row is disabled and says so.
    ///
    /// `DESIGN.md:454` — the SoundFont file row stays visible and locked on
    /// PATCH. The row is handed that state; it does not decide it.
    #[test]
    fn a_locked_asset_row_says_it_is_locked() {
        let shown = painted(ComponentState::Disabled, ViewportDensityPolicy::Desktop);
        assert!(
            shown.says("Locked"),
            "a locked asset row does not announce itself"
        );
    }

    /// Every asset kind resolves to its own tag, and the tags read as text.
    #[test]
    fn every_asset_kind_carries_its_own_readable_tag() {
        let tags = [
            (AssetKind::SoundFont, SOUND_FONT_TAG),
            (AssetKind::Sample, SAMPLE_TAG),
            (AssetKind::Other, OTHER_TAG),
        ];
        let mut seen = std::collections::HashSet::new();
        for (kind, expected) in tags {
            let value =
                SemanticControlValue::Asset(AssetReference::new(kind, "entry").expect("locator"));
            assert_eq!(kind_tag(&value), expected);
            assert!(seen.insert(expected), "{expected} tags two kinds");
            assert!(!expected.is_empty());
        }
        // A value that is not an asset is tagged as such rather than being
        // silently re-labelled a file.
        for value in [
            SemanticControlValue::Scalar(1.0),
            SemanticControlValue::Parameter(ParameterValue::Toggle(true)),
            SemanticControlValue::Identity("PLAITS".to_owned()),
            SemanticControlValue::Summary("Read-only".to_owned()),
        ] {
            assert_eq!(kind_tag(&value), NON_ASSET_TAG);
        }
    }

    /// The row renders the locator the projection supplied and nothing else.
    #[test]
    fn the_row_names_the_asset_the_projection_supplied() {
        let reference =
            AssetReference::new(AssetKind::Sample, "pad-cloud-C3.wav").expect("locator");
        let value = SemanticControlValue::Asset(reference.clone());
        assert_eq!(value_text(&value), reference.locator());
    }

    /// The metadata column is marked unavailable rather than decorated.
    ///
    /// The view data carries no duration, format, or size, so a row that
    /// painted one would be inventing it.
    #[test]
    fn the_metadata_column_is_explicitly_unavailable() {
        let shown = painted(ComponentState::Resting, ViewportDensityPolicy::Desktop);
        assert!(
            shown.says(UNAVAILABLE_MARK),
            "the metadata column neither carries data nor says it has none"
        );
    }

    /// No waveform is painted. The specimen puts it in the Preview region
    /// beside the list, not in a row.
    #[test]
    fn a_row_paints_no_waveform() {
        // A waveform would be dozens of separate bars. A browser row's whole
        // shape budget is its chrome, so an order-of-magnitude jump here is
        // what a decorative waveform would look like.
        let shown = painted(ComponentState::Resting, ViewportDensityPolicy::Desktop);
        assert!(
            shown.shapes < 16,
            "the row emitted {} shapes, which is more than its authored anatomy",
            shown.shapes
        );
    }

    #[test]
    fn focus_and_adjustment_paint_the_authored_cursor() {
        for state in [ComponentState::Focused, ComponentState::Adjusting] {
            assert!(
                painted(state, ViewportDensityPolicy::Desktop).says(focus::CURSOR_GLYPH),
                "{} does not paint the cursor",
                state.canonical_name()
            );
        }
    }

    #[test]
    fn both_viewports_paint_the_same_content() {
        for state in CONTROL.applicable_states() {
            assert_eq!(
                painted(*state, ViewportDensityPolicy::Desktop).texts,
                painted(*state, ViewportDensityPolicy::SteamDeck).texts,
                "{} paints different content at the two viewports",
                state.canonical_name()
            );
        }
    }

    /// The row paints nothing it was not handed.
    #[test]
    fn the_row_paints_no_text_that_did_not_arrive_in_its_view_model() {
        let view = projected_control(TopLevelContext::Patch);
        let name = value_text(view.value());
        let tag = kind_tag(view.value());
        for state in CONTROL.applicable_states() {
            let shown = paint(
                CONTROL,
                &view,
                *state,
                PresentationRole::ListedRow,
                ViewportDensityPolicy::Desktop,
            );
            for run in &shown.texts {
                let from_view_model = name.contains(run.as_str()) || run.is_empty();
                let from_vocabulary = run == focus::CURSOR_GLYPH
                    || run == tag
                    || run == UNAVAILABLE_MARK
                    || run == GENERIC_FAILURE_WORD
                    || LOADING_PROGRESS_WORDS.contains(&run.as_str())
                    || matches!(
                        state.appearance().signal,
                        NonColorSignal::Word(word) if word == run
                    );
                assert!(
                    from_view_model || from_vocabulary,
                    "{} painted {run:?}, which is neither view data nor authored vocabulary",
                    state.canonical_name()
                );
            }
        }
    }
}
