//! The projection transport: generation-gated push of the accepted
//! projection's embedded semantic model to the page.
//!
//! On each tick where the fetched [`GraphicalShellProjection`] carries a new
//! accepted generation, the channel serializes the projection's embedded
//! [`SemanticGraphicalViewModel`] with `serde_json` and emits the document on
//! the [`PROJECTION_EVENT`] named event through the tauri `Emitter`. The
//! payload is exactly the projector's serialization — one schema, no
//! page-facing struct, no trimmed or selected field (crest-spec
//! `requirement.serialized_projection_transport`). If the page ever needs a
//! field this document lacks, the fix is in the projector via the crest-spec,
//! never a wrapper here.
//!
//! # Generation gating
//!
//! Emission is gated on the projection's accepted `generation`, not on deep
//! comparison: the projector already stamps every accepted document with its
//! generation and state hash, so an unchanged generation means an unchanged
//! document. A tick that fetches the same generation emits nothing — the
//! ~85 KB serialization happens only when a reducer edit actually produced a
//! new accepted document. The gate advances only on a successful emit; a
//! failed emit leaves the gate unmoved so the document is not silently lost
//! while the window still lives.
//!
//! # Emit failures are typed, surfaced, and tolerated at teardown
//!
//! An emit failure is a typed [`ProjectionChannelError`] the window converts
//! onto its declared `WindowError` path — the same record-first-error-and-
//! close-once treatment the retired native adapter gave an invalid late frame. It is
//! never a panic; during teardown the window stops pushing once the webview
//! is destroyed, so a shutdown race cannot turn a clean close into a fatal
//! error (see `window.rs`).
//!
//! # Painted acks come back and become observations (WP02 T005)
//!
//! After the page paints a pushed document it echoes the document's semantic
//! identity plus its measured post-paint geometry on the [`PAINTED_EVENT`]
//! event. [`ProjectionChannel::forward_ack`] receives that ack, correlates
//! it against the bounded in-flight tracker this channel fills on each
//! successful push, and constructs exactly one [`ShellFrameObservation`] —
//! identity copied verbatim (the ack must match the pushed document field
//! for field), viewport and region rectangles taken from the ack's measured
//! geometry. An ack that matches no in-flight pushed document never becomes
//! an observation: expected region names or a pre-render layout plan alone
//! cannot construct a passing observation (crest-spec
//! `valueObject.Shell.ShellFrameObservation`). Which of the two ways that
//! can happen decides the outcome: an ack claiming a generation newer than
//! anything ever pushed is fabricated evidence and stays a typed
//! [`PaintedAckError`], while a late ack for an already-superseded document
//! (dropped from the bounded tracker under sustained generation churn —
//! observed on the first webview-hosted live runs, mission
//! webview-shell-cutover WP03) is a lost frame consumed as
//! [`ForwardedAck::SupersededLate`], degrading observation only.
//!
//! A superseded document whose ack never arrives is dropped from tracking
//! when a successor acks (or when the bounded tracker is full) — lost frames
//! degrade observation only, exactly like the retired native adapter dropping an
//! unpainted frame.
//!
//! # Nothing here touches the real-time callback
//!
//! The channel runs on the window's event thread and reads only the immutable
//! control-side projection that the `ProjectionCallback` already hands every
//! `AppWindow` — the same value the retired native adapter painted. It installs
//! nothing into the audio callback, allocates nothing inside it, and shares
//! no lock the callback could contend on; the render path's measured bounds
//! are unchanged from the pre-cutover shell baseline (crest-spec
//! `requirement.serialized_projection_transport`; measured in WP06).

use crate::control::{GraphicalShellProjection, SemanticGraphicalViewModel};
use crate::shell::app_window::WindowError;
use crate::shell::{
    ShellFrameObservation, ShellFrameObservationError, ShellRegionId, ShellRegionObservation,
    ShellRegionRect,
};
use core::fmt;
use serde_json::Value;
use std::collections::VecDeque;

/// The named tauri event carrying each newly accepted projection's
/// serialized [`SemanticGraphicalViewModel`] to the page.
pub const PROJECTION_EVENT: &str = "crest://projection";

/// The named tauri event on which the page acknowledges each painted
/// document: the document's semantic identity copied verbatim plus the
/// measured viewport and region geometry (the page's `paintedEvidence`).
pub const PAINTED_EVENT: &str = "crest://painted";

/// The named tauri event on which the page reports a thrown render failure
/// for a received document. The window converts it to the typed
/// `WebviewShellError::PageRenderFailed` and takes the fatal runtime path —
/// a page that loads then fails to render never leaves a frozen window
/// (WP02 T008).
pub const RENDER_ERROR_EVENT: &str = "crest://render-error";

/// How many pushed documents may await their painted ack at once.
///
/// The live cadence keeps this at one or two (a push happens only on a new
/// accepted generation and the page acks within a frame); the bound exists
/// so a page that stops acking can never grow an unbounded queue. When the
/// tracker is full the oldest unacked document is dropped — a lost frame
/// degrades observation only.
pub const MAX_IN_FLIGHT_DOCUMENTS: usize = 8;

/// The serialized identity fields a painted ack must copy verbatim from its
/// document, in the order they are tracked (crest-spec
/// `valueObject.Shell.ShellFrameObservation`: "copies semantic identity
/// exactly").
const ACK_IDENTITY_FIELDS: [&str; 6] = [
    "generation",
    "stateHash",
    "context",
    "activeSurface",
    "focusPath",
    "interactionMode",
];

/// What one [`ProjectionChannel::push`] did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionPush {
    /// A new accepted generation was serialized and emitted.
    Emitted,
    /// The projection's generation matched the last emitted one; nothing was
    /// serialized and nothing was emitted.
    Unchanged,
}

/// What one received painted ack became.
#[derive(Clone, Debug, PartialEq)]
// The observation is carried inline: this value lives for exactly one match
// arm on the window's event thread, so a second heap ownership step would
// buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum ForwardedAck {
    /// The ack correlated with an in-flight pushed document and copied its
    /// identity verbatim: exactly one honest post-paint observation.
    Observation(ShellFrameObservation),
    /// A late ack for a document this channel pushed but had already
    /// dropped from its bounded tracker (superseded under sustained
    /// generation churn before the ack arrived). The paint happened, but
    /// the document it proves is gone — a lost frame, consumed without an
    /// observation and without an error (display evidence degrades only).
    SupersededLate {
        /// The already-superseded generation the late ack named.
        generation: u64,
    },
}

/// A typed projection transport failure, surfaced through the window's
/// declared `WindowError` path.
#[derive(Debug)]
pub enum ProjectionChannelError {
    /// The embedded semantic model failed to serialize. The projector's
    /// derives make this structurally unreachable; if it ever fires it is a
    /// projector defect, not a page concern.
    Serialization {
        /// The accepted generation that failed to serialize.
        generation: u64,
        /// The underlying serde failure.
        source: serde_json::Error,
    },
    /// The tauri emit itself failed (typically the window going away).
    Emit {
        /// The accepted generation whose document failed to emit.
        generation: u64,
        /// The underlying tauri failure.
        source: tauri::Error,
    },
}

impl fmt::Display for ProjectionChannelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization { generation, source } => write!(
                formatter,
                "projection generation {generation} failed to serialize: {source}"
            ),
            Self::Emit { generation, source } => write!(
                formatter,
                "projection generation {generation} failed to emit: {source}"
            ),
        }
    }
}

impl std::error::Error for ProjectionChannelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serialization { source, .. } => Some(source),
            Self::Emit { source, .. } => Some(source),
        }
    }
}

/// Joins the typed transport failure onto the window's declared error path,
/// preserving the cause text.
impl From<ProjectionChannelError> for WindowError {
    fn from(error: ProjectionChannelError) -> Self {
        WindowError::new(error.to_string())
    }
}

/// A typed painted-ack rejection: the ack could not become a
/// [`ShellFrameObservation`]. Never a panic; the window gives it the same
/// record-first-error-and-close-once treatment the retired native adapter gave an
/// invalid late frame.
#[derive(Debug)]
pub enum PaintedAckError {
    /// The ack payload could not be understood as post-paint evidence.
    Malformed {
        /// What was wrong with the payload.
        detail: String,
    },
    /// The ack names a generation newer than anything this channel ever
    /// pushed — the observation the ack describes cannot have been painted
    /// from a pushed document, so it must not be constructible. (A late ack
    /// for a pushed-then-dropped document is not this error; it is consumed
    /// as [`ForwardedAck::SupersededLate`].)
    UnknownDocument {
        /// The generation the ack claimed.
        generation: u64,
    },
    /// The ack correlated with a pushed document but one of its identity
    /// fields is not a verbatim copy of that document's.
    IdentityMismatch {
        /// The correlated generation.
        generation: u64,
        /// The serialized identity field that differed.
        field: &'static str,
    },
    /// The ack correlated and its identity matched, but its measured
    /// geometry or labels fail the observation's own post-paint invariants.
    InvalidObservation {
        /// The correlated generation.
        generation: u64,
        /// The observation invariant that rejected the evidence.
        source: ShellFrameObservationError,
    },
}

impl fmt::Display for PaintedAckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { detail } => write!(formatter, "painted ack rejected: {detail}"),
            Self::UnknownDocument { generation } => write!(
                formatter,
                "painted ack for generation {generation} matches no in-flight pushed document"
            ),
            Self::IdentityMismatch { generation, field } => write!(
                formatter,
                "painted ack for generation {generation} does not copy {field} verbatim from its document"
            ),
            Self::InvalidObservation { generation, source } => write!(
                formatter,
                "painted ack for generation {generation} carries invalid painted evidence: {source}"
            ),
        }
    }
}

impl std::error::Error for PaintedAckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidObservation { source, .. } => Some(source),
            Self::Malformed { .. }
            | Self::UnknownDocument { .. }
            | Self::IdentityMismatch { .. } => None,
        }
    }
}

/// Joins the typed ack rejection onto the window's declared error path,
/// preserving the cause text.
impl From<PaintedAckError> for WindowError {
    fn from(error: PaintedAckError) -> Self {
        WindowError::new(error.to_string())
    }
}

/// One pushed document awaiting its painted ack: the correlation key, the
/// serialized identity the ack must copy verbatim, and the semantic model
/// the resulting observation is constructed against.
#[derive(Debug)]
struct InFlightDocument {
    generation: u64,
    identity: [Value; ACK_IDENTITY_FIELDS.len()],
    semantic: SemanticGraphicalViewModel,
}

/// Generation-gated push channel from accepted projections to the page,
/// plus the receive side of the page's painted acks.
///
/// Push state is exactly one value — the last successfully emitted
/// generation; a push either emits the current document or does nothing.
/// The ack side tracks at most [`MAX_IN_FLIGHT_DOCUMENTS`] pushed documents
/// so each ack can be correlated with the document it claims to have
/// painted.
#[derive(Debug, Default)]
pub struct ProjectionChannel {
    last_emitted_generation: Option<u64>,
    in_flight: VecDeque<InFlightDocument>,
}

impl ProjectionChannel {
    /// Creates a channel that has emitted nothing yet, so the first accepted
    /// projection always emits.
    pub const fn new() -> Self {
        Self {
            last_emitted_generation: None,
            in_flight: VecDeque::new(),
        }
    }

    /// Pushes the projection's embedded semantic model through `emit` if its
    /// accepted generation differs from the last successfully emitted one.
    ///
    /// `emit` receives the complete `serde_json` document of the embedded
    /// [`SemanticGraphicalViewModel`] and performs the actual tauri emit on
    /// [`PROJECTION_EVENT`]; injecting it keeps the gating logic provable
    /// without a tauri runtime (the `Emitter` trait is sealed).
    ///
    /// [`SemanticGraphicalViewModel`]: crate::control::SemanticGraphicalViewModel
    pub fn push<E>(
        &mut self,
        projection: &GraphicalShellProjection,
        emit: E,
    ) -> Result<ProjectionPush, ProjectionChannelError>
    where
        E: FnOnce(serde_json::Value) -> tauri::Result<()>,
    {
        let generation = projection.generation();
        if self.last_emitted_generation == Some(generation) {
            return Ok(ProjectionPush::Unchanged);
        }
        let document = serde_json::to_value(projection.semantic_model())
            .map_err(|source| ProjectionChannelError::Serialization { generation, source })?;
        let identity = ack_identity_of(&document);
        emit(document).map_err(|source| ProjectionChannelError::Emit { generation, source })?;
        self.last_emitted_generation = Some(generation);
        // Track the emitted document so its painted ack can correlate. The
        // tracker is bounded: when full, the oldest unacked document is a
        // lost frame — dropped, degrading observation only.
        if self.in_flight.len() == MAX_IN_FLIGHT_DOCUMENTS {
            self.in_flight.pop_front();
        }
        self.in_flight.push_back(InFlightDocument {
            generation,
            identity,
            semantic: projection.semantic_model().clone(),
        });
        Ok(ProjectionPush::Emitted)
    }

    /// How many pushed documents currently await their painted ack.
    pub fn in_flight_documents(&self) -> usize {
        self.in_flight.len()
    }

    /// Receives one painted ack and constructs exactly one
    /// [`ShellFrameObservation`] for it, consumes it as a lost late frame
    /// ([`ForwardedAck::SupersededLate`]), or rejects it with a typed
    /// [`PaintedAckError`].
    ///
    /// `payload` is the raw JSON the page emitted on [`PAINTED_EVENT`]:
    /// the document's semantic identity — generation, stateHash, context,
    /// activeSurface, focusPath, interactionMode — copied verbatim, plus
    /// the measured viewport (`viewport.widthPx`/`heightPx`) and the five
    /// painted region rectangles with their visible labels (`regions[]`,
    /// ids spelled as `ShellRegionId` serialized names).
    ///
    /// The ack must correlate with an in-flight pushed document and copy its
    /// identity exactly; the observation is then constructed against that
    /// document's own semantic model with the ack's measured geometry, so an
    /// observation can never be built from expectations alone. A successful
    /// ack consumes its document and every older unacked one (superseded —
    /// lost frames degrade observation only).
    pub fn forward_ack(&mut self, payload: &str) -> Result<ForwardedAck, PaintedAckError> {
        let ack: Value =
            serde_json::from_str(payload).map_err(|error| PaintedAckError::Malformed {
                detail: format!("payload is not JSON: {error}"),
            })?;
        let generation = ack
            .get("generation")
            .and_then(Value::as_u64)
            .ok_or_else(|| PaintedAckError::Malformed {
                detail: "ack carries no numeric generation".to_owned(),
            })?;
        let Some(index) = self
            .in_flight
            .iter()
            .position(|document| document.generation == generation)
        else {
            // Generations are monotone, so an ack at or below the newest
            // emitted generation names a document this channel pushed and
            // has since dropped (superseded before its ack arrived): a lost
            // frame, not fabricated evidence. Anything newer than every
            // emission cannot have been painted from a pushed document.
            return if self
                .last_emitted_generation
                .is_some_and(|newest| generation <= newest)
            {
                Ok(ForwardedAck::SupersededLate { generation })
            } else {
                Err(PaintedAckError::UnknownDocument { generation })
            };
        };

        let document = &self.in_flight[index];
        for (field, expected) in ACK_IDENTITY_FIELDS.iter().zip(&document.identity) {
            let supplied = ack.get(*field).unwrap_or(&Value::Null);
            if supplied != expected {
                return Err(PaintedAckError::IdentityMismatch { generation, field });
            }
        }

        let viewport = ack
            .get("viewport")
            .ok_or_else(|| PaintedAckError::Malformed {
                detail: "ack carries no measured viewport".to_owned(),
            })?;
        let viewport_width = measured_px(viewport, "widthPx")?;
        let viewport_height = measured_px(viewport, "heightPx")?;
        let supplied_regions = ack
            .get("regions")
            .and_then(Value::as_array)
            .ok_or_else(|| PaintedAckError::Malformed {
                detail: "ack carries no measured regions".to_owned(),
            })?;
        if supplied_regions.len() != ShellRegionId::ALL.len() {
            return Err(PaintedAckError::Malformed {
                detail: format!(
                    "ack carries {} measured regions, the shell declares {}",
                    supplied_regions.len(),
                    ShellRegionId::ALL.len()
                ),
            });
        }
        let regions = supplied_regions
            .iter()
            .map(measured_region)
            .collect::<Result<Vec<_>, _>>()?;
        let regions: [ShellRegionObservation; 5] = regions
            .try_into()
            .expect("the region count was checked against the declared shell regions");

        let observation = ShellFrameObservation::try_new_semantic(
            viewport_width,
            viewport_height,
            &document.semantic,
            regions,
        )
        .map_err(|source| PaintedAckError::InvalidObservation { generation, source })?;

        // The ack consumed its document. Everything older was superseded
        // without ever acking — dropped, degrading observation only.
        self.in_flight.drain(..=index);
        Ok(ForwardedAck::Observation(observation))
    }
}

/// Copies the serialized identity fields the painted ack must echo out of a
/// pushed document.
fn ack_identity_of(document: &Value) -> [Value; ACK_IDENTITY_FIELDS.len()] {
    ACK_IDENTITY_FIELDS.map(|field| document.get(field).cloned().unwrap_or(Value::Null))
}

/// Reads one measured pixel value from the ack's geometry.
fn measured_px(value: &Value, field: &str) -> Result<f32, PaintedAckError> {
    value
        .get(field)
        .and_then(Value::as_f64)
        .map(|px| px as f32)
        .ok_or_else(|| PaintedAckError::Malformed {
            detail: format!("ack geometry carries no numeric {field}"),
        })
}

/// Reads one measured region — stable id, painted bounds, visible label —
/// from the ack's `regions[]` entry.
fn measured_region(value: &Value) -> Result<ShellRegionObservation, PaintedAckError> {
    let name =
        value
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| PaintedAckError::Malformed {
                detail: "ack region carries no id".to_owned(),
            })?;
    let id = ShellRegionId::ALL
        .into_iter()
        .find(|id| id.name() == name)
        .ok_or_else(|| PaintedAckError::Malformed {
            detail: format!("ack region id {name} names no declared shell region"),
        })?;
    let x = measured_px(value, "xPx")?;
    let y = measured_px(value, "yPx")?;
    let width = measured_px(value, "widthPx")?;
    let height = measured_px(value, "heightPx")?;
    let label = value.get("label").and_then(Value::as_str).unwrap_or("");
    Ok(ShellRegionObservation::new(
        id,
        ShellRegionRect::new(x, y, x + width, y + height),
        label,
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        ForwardedAck, PaintedAckError, ProjectionChannel, ProjectionChannelError, ProjectionPush,
        MAX_IN_FLIGHT_DOCUMENTS, PAINTED_EVENT, PROJECTION_EVENT, RENDER_ERROR_EVENT,
    };
    use crate::control::{
        GraphicalShellProjection, SemanticGraphicalViewModel, ShellContextLine, ShellFooter,
        ShellIdentityHeader, TextProjection, TopLevelContext,
    };
    use crate::shell::{ShellFrameObservationError, ShellRegionId};
    use serde_json::{json, Value};
    use std::cell::RefCell;

    fn projection(generation: u64, state_hash: &str) -> GraphicalShellProjection {
        GraphicalShellProjection::new(
            generation,
            state_hash,
            SemanticGraphicalViewModel::fixture(generation, state_hash, TopLevelContext::Mixer),
            ShellContextLine::new("CREST SYNTH", "MIXER", "READY"),
            ShellIdentityHeader::new("MIXER", "Transport fixture"),
            "MIXER WORKSPACE",
            "INSPECTOR",
            TextProjection::for_context(
                TopLevelContext::Mixer,
                "MIXER diagnostic".to_owned(),
                0,
                state_hash.to_owned(),
            ),
            ShellFooter::new("MIXER", vec!["1 MIXER".to_owned(), "2 PATCH".to_owned()]),
        )
        .expect("the transport test fixture is coherent")
    }

    #[test]
    fn projection_event_name_satisfies_the_tauri_event_charset() {
        assert!(PROJECTION_EVENT.chars().all(|c| c.is_alphanumeric()
            || c == '-'
            || c == '/'
            || c == ':'
            || c == '_'));
    }

    #[test]
    fn a_new_generation_emits_the_exact_projector_serialization_once() {
        let mut channel = ProjectionChannel::new();
        let projection = projection(7, "state-7");
        let emitted: RefCell<Vec<Value>> = RefCell::new(Vec::new());

        let outcome = channel
            .push(&projection, |document| {
                emitted.borrow_mut().push(document);
                Ok(())
            })
            .expect("a live emit succeeds");

        assert_eq!(outcome, ProjectionPush::Emitted);
        let emitted = emitted.into_inner();
        assert_eq!(emitted.len(), 1);
        // The payload is the projector's own serialization of the embedded
        // semantic model — the whole document, byte-for-byte equal, with the
        // declared top-level properties and nothing added or trimmed.
        let expected = serde_json::to_value(projection.semantic_model())
            .expect("the projector's model serializes");
        assert_eq!(emitted[0], expected);
        let Value::Object(document) = &emitted[0] else {
            panic!("the emitted payload must be the serialized document object");
        };
        let mut keys: Vec<&str> = document.keys().map(String::as_str).collect();
        keys.sort_unstable();
        let mut declared: Vec<&str> =
            SemanticGraphicalViewModel::SERIALIZED_PROPERTY_DESCRIPTOR.to_vec();
        declared.sort_unstable();
        assert_eq!(keys, declared);
        assert_eq!(document["generation"], Value::from(7));
        assert_eq!(document["stateHash"], Value::from("state-7"));
    }

    #[test]
    fn an_unchanged_generation_emits_nothing() {
        let mut channel = ProjectionChannel::new();
        let projection = projection(7, "state-7");
        let mut emits = 0_usize;

        for _ in 0..3 {
            let outcome = channel
                .push(&projection, |_| {
                    emits += 1;
                    Ok(())
                })
                .expect("pushing never fails with a succeeding emitter");
            if emits == 1 {
                continue;
            }
            assert_eq!(outcome, ProjectionPush::Unchanged);
        }

        assert_eq!(emits, 1, "same generation on every later tick: no emit");
    }

    #[test]
    fn a_changed_generation_emits_exactly_one_more_document() {
        let mut channel = ProjectionChannel::new();
        let first = projection(7, "state-7");
        let second = first
            .with_generation(
                8,
                "state-8".to_owned(),
                TextProjection::for_context(
                    TopLevelContext::Mixer,
                    "MIXER diagnostic".to_owned(),
                    0,
                    "state-8".to_owned(),
                ),
            )
            .expect("advancing the fixture generation stays coherent");
        let mut generations: Vec<u64> = Vec::new();

        for projection in [&first, &first, &second, &second] {
            channel
                .push(projection, |document| {
                    generations.push(document["generation"].as_u64().unwrap());
                    Ok(())
                })
                .expect("pushing never fails with a succeeding emitter");
        }

        assert_eq!(generations, [7, 8], "one emit per accepted generation");
    }

    #[test]
    fn a_failed_emit_is_typed_and_leaves_the_gate_unmoved() {
        let mut channel = ProjectionChannel::new();
        let projection = projection(7, "state-7");

        let error = channel
            .push(&projection, |_| {
                Err(tauri::Error::from(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "window gone",
                )))
            })
            .expect_err("a failing emitter must surface a typed error");
        match &error {
            ProjectionChannelError::Emit { generation, .. } => assert_eq!(*generation, 7),
            other => panic!("expected a typed emit failure, got {other:?}"),
        }
        assert!(error.to_string().contains("generation 7"));

        // The gate did not advance: the next tick with the same generation
        // retries instead of silently dropping the accepted document.
        let outcome = channel
            .push(&projection, |_| Ok(()))
            .expect("the retry emit succeeds");
        assert_eq!(outcome, ProjectionPush::Emitted);
    }

    // ------------------------------------------------------------------
    // Painted-ack forwarding (WP02 T005/T009)
    // ------------------------------------------------------------------

    fn advanced(
        projection: &GraphicalShellProjection,
        generation: u64,
    ) -> GraphicalShellProjection {
        let state_hash = format!("state-{generation}");
        projection
            .with_generation(
                generation,
                state_hash.clone(),
                TextProjection::for_context(
                    TopLevelContext::Mixer,
                    "MIXER diagnostic".to_owned(),
                    0,
                    state_hash,
                ),
            )
            .expect("advancing the fixture generation stays coherent")
    }

    fn push_ok(channel: &mut ProjectionChannel, projection: &GraphicalShellProjection) {
        let outcome = channel
            .push(projection, |_| Ok(()))
            .expect("the fixture emit succeeds");
        assert_eq!(outcome, ProjectionPush::Emitted);
    }

    /// The negotiated ack payload: the document's serialized identity copied
    /// verbatim, plus measured viewport and region geometry — exactly what
    /// the page's `paintedEvidence` emits after painting.
    fn ack_for(projection: &GraphicalShellProjection) -> Value {
        let document = serde_json::to_value(projection.semantic_model())
            .expect("the projector's model serializes");
        json!({
            "generation": document["generation"],
            "stateHash": document["stateHash"],
            "context": document["context"],
            "activeSurface": document["activeSurface"],
            "focusPath": document["focusPath"],
            "interactionMode": document["interactionMode"],
            "viewport": { "widthPx": 1920.0, "heightPx": 1080.0 },
            "regions": [
                { "id": "contextLine", "xPx": 0.0, "yPx": 0.0,
                  "widthPx": 1920.0, "heightPx": 48.0, "label": "CREST SYNTH" },
                { "id": "identityHeader", "xPx": 0.0, "yPx": 48.0,
                  "widthPx": 1920.0, "heightPx": 72.0, "label": "MIXER" },
                { "id": "mainWorkspace", "xPx": 0.0, "yPx": 120.0,
                  "widthPx": 1500.0, "heightPx": 896.0, "label": "MIXER WORKSPACE" },
                { "id": "persistentSideRegion", "xPx": 1500.0, "yPx": 120.0,
                  "widthPx": 420.0, "heightPx": 896.0, "label": "INSPECTOR" },
                { "id": "footer", "xPx": 0.0, "yPx": 1016.0,
                  "widthPx": 1920.0, "heightPx": 64.0, "label": "MIXER" },
            ],
        })
    }

    #[test]
    fn event_names_satisfy_the_tauri_event_charset() {
        for event in [PAINTED_EVENT, RENDER_ERROR_EVENT] {
            assert!(event.chars().all(|c| c.is_alphanumeric()
                || c == '-'
                || c == '/'
                || c == ':'
                || c == '_'));
        }
    }

    #[test]
    fn one_pushed_document_and_its_ack_become_exactly_one_verbatim_observation() {
        let mut channel = ProjectionChannel::new();
        let projection = projection(7, "state-7");
        push_ok(&mut channel, &projection);
        assert_eq!(channel.in_flight_documents(), 1);

        let payload = ack_for(&projection).to_string();
        let ForwardedAck::Observation(observation) = channel
            .forward_ack(&payload)
            .expect("a matching painted ack becomes an observation")
        else {
            panic!("a matching painted ack must construct the observation");
        };

        // Identity is copied verbatim from the pushed document.
        let model = projection.semantic_model();
        assert_eq!(observation.generation(), model.generation());
        assert_eq!(observation.state_hash(), model.state_hash());
        assert_eq!(observation.context(), model.context());
        assert_eq!(observation.active_surface(), model.active_surface());
        assert_eq!(observation.focus_path(), model.focus_path());
        assert_eq!(observation.interaction_mode(), model.interaction_mode());
        // Geometry and labels come from the ack's measured evidence.
        assert_eq!(observation.viewport_width(), 1920.0);
        assert_eq!(observation.viewport_height(), 1080.0);
        let footer = observation.region(ShellRegionId::Footer);
        assert_eq!(footer.rect().min_y(), 1016.0);
        assert_eq!(footer.rect().max_y(), 1080.0);
        assert_eq!(footer.visible_label(), "MIXER");
        assert!(observation.regions_are_non_overlapping());

        // Exactly one: the ack consumed its document, so replaying it can
        // construct nothing — it is consumed as a late superseded ack.
        assert_eq!(channel.in_flight_documents(), 0);
        let replay = channel
            .forward_ack(&payload)
            .expect("a replayed ack is consumed without constructing anything");
        assert_eq!(replay, ForwardedAck::SupersededLate { generation: 7 });
    }

    #[test]
    fn an_ack_with_no_pushed_document_is_a_typed_rejection() {
        let mut channel = ProjectionChannel::new();
        // Nothing was pushed: the ack describes a frame nobody painted from
        // a pushed document, so no observation is constructible from it.
        let payload = ack_for(&projection(7, "state-7")).to_string();

        let error = channel
            .forward_ack(&payload)
            .expect_err("an uncorrelated ack must be rejected");
        assert!(matches!(
            error,
            PaintedAckError::UnknownDocument { generation: 7 }
        ));
        assert!(error.to_string().contains("generation 7"));
        assert_eq!(channel.in_flight_documents(), 0);

        // With a document pushed, an ack claiming a generation newer than
        // every emission is still fabricated evidence — only late acks for
        // pushed-then-superseded documents are consumed quietly.
        let pushed = projection(7, "state-7");
        push_ok(&mut channel, &pushed);
        let future = channel
            .forward_ack(&ack_for(&advanced(&pushed, 9)).to_string())
            .expect_err("an ack newer than every emission must be rejected");
        assert!(matches!(
            future,
            PaintedAckError::UnknownDocument { generation: 9 }
        ));
    }

    #[test]
    fn an_ack_that_rewrites_identity_is_a_typed_mismatch() {
        let mut channel = ProjectionChannel::new();
        let projection = projection(7, "state-7");
        push_ok(&mut channel, &projection);

        let mut ack = ack_for(&projection);
        ack["stateHash"] = Value::from("state-invented");
        let error = channel
            .forward_ack(&ack.to_string())
            .expect_err("an identity-rewriting ack must be rejected");
        match error {
            PaintedAckError::IdentityMismatch { generation, field } => {
                assert_eq!(generation, 7);
                assert_eq!(field, "stateHash");
            }
            other => panic!("expected an identity mismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_superseded_document_that_never_acks_is_dropped_when_its_successor_acks() {
        let mut channel = ProjectionChannel::new();
        let first = projection(7, "state-7");
        let second = advanced(&first, 8);
        push_ok(&mut channel, &first);
        push_ok(&mut channel, &second);
        assert_eq!(channel.in_flight_documents(), 2);

        // The first document's ack never arrives; the successor's does.
        channel
            .forward_ack(&ack_for(&second).to_string())
            .expect("the successor's ack becomes its one observation");

        // The superseded document was dropped from tracking — a lost frame
        // degrades observation only, and its late ack constructs nothing:
        // it is consumed quietly rather than failing the window (WP03).
        assert_eq!(channel.in_flight_documents(), 0);
        let late = channel
            .forward_ack(&ack_for(&first).to_string())
            .expect("a superseded document's late ack is consumed quietly");
        assert_eq!(late, ForwardedAck::SupersededLate { generation: 7 });
    }

    #[test]
    fn the_in_flight_tracker_is_bounded() {
        let mut channel = ProjectionChannel::new();
        let base = projection(1, "state-1");
        push_ok(&mut channel, &base);
        let count = MAX_IN_FLIGHT_DOCUMENTS as u64 + 3;
        for generation in 2..=count {
            push_ok(&mut channel, &advanced(&base, generation));
        }

        // No ack ever arrived, yet tracking never grew past the bound; the
        // oldest documents were dropped, the newest still correlates. The
        // evicted document's late ack is a lost frame consumed quietly —
        // sustained generation churn must never turn paced late acks into
        // a fatal runtime failure (WP03).
        assert_eq!(channel.in_flight_documents(), MAX_IN_FLIGHT_DOCUMENTS);
        let oldest = channel
            .forward_ack(&ack_for(&base).to_string())
            .expect("the evicted oldest document's late ack is consumed quietly");
        assert_eq!(oldest, ForwardedAck::SupersededLate { generation: 1 });
        let newest = channel
            .forward_ack(&ack_for(&advanced(&base, count)).to_string())
            .expect("the newest document still correlates");
        assert!(matches!(newest, ForwardedAck::Observation(_)));
    }

    #[test]
    fn a_malformed_ack_is_a_typed_rejection() {
        let mut channel = ProjectionChannel::new();
        let projection = projection(7, "state-7");
        push_ok(&mut channel, &projection);

        for (payload, why) in [
            ("not json".to_owned(), "unparseable payload"),
            (json!({ "viewport": {} }).to_string(), "no generation"),
            (
                {
                    let mut ack = ack_for(&projection);
                    ack.as_object_mut().unwrap().remove("viewport");
                    ack.to_string()
                },
                "no measured viewport",
            ),
            (
                {
                    let mut ack = ack_for(&projection);
                    ack["regions"].as_array_mut().unwrap().pop();
                    ack.to_string()
                },
                "a missing region",
            ),
            (
                {
                    let mut ack = ack_for(&projection);
                    ack["regions"][0]["id"] = Value::from("inventedRegion");
                    ack.to_string()
                },
                "an undeclared region id",
            ),
        ] {
            let error = channel
                .forward_ack(&payload)
                .expect_err("a malformed ack must be rejected");
            assert!(
                matches!(error, PaintedAckError::Malformed { .. }),
                "{why}: got {error:?}"
            );
        }
        // Every rejection left the pushed document awaiting its honest ack.
        assert_eq!(channel.in_flight_documents(), 1);
    }

    #[test]
    fn measured_evidence_violating_observation_invariants_is_typed() {
        let mut channel = ProjectionChannel::new();
        let projection = projection(7, "state-7");
        push_ok(&mut channel, &projection);

        // The footer's measured bounds run past the measured viewport: the
        // observation's own post-paint invariants reject the evidence.
        let mut ack = ack_for(&projection);
        ack["regions"][4]["widthPx"] = Value::from(2000.0);
        let error = channel
            .forward_ack(&ack.to_string())
            .expect_err("out-of-viewport evidence must be rejected");
        match error {
            PaintedAckError::InvalidObservation { generation, source } => {
                assert_eq!(generation, 7);
                assert_eq!(
                    source,
                    ShellFrameObservationError::InvalidRegionBounds {
                        region: ShellRegionId::Footer
                    }
                );
            }
            other => panic!("expected invalid observation evidence, got {other:?}"),
        }
    }
}
