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
//! close-once treatment the eframe adapter gives an invalid late frame. It is
//! never a panic; during teardown the window stops pushing once the webview
//! is destroyed, so a shutdown race cannot turn a clean close into a fatal
//! error (see `window.rs`).
//!
//! # Nothing here touches the real-time callback
//!
//! The channel runs on the window's event thread and reads only the immutable
//! control-side projection that the `ProjectionCallback` already hands every
//! `AppWindow` — the same value the eframe adapter paints. It installs
//! nothing into the audio callback, allocates nothing inside it, and shares
//! no lock the callback could contend on; the render path's measured bounds
//! are unchanged from the egui-shell baseline (crest-spec
//! `requirement.serialized_projection_transport`; measured in WP06).

use crate::control::GraphicalShellProjection;
use crate::shell::app_window::WindowError;
use core::fmt;

/// The named tauri event carrying each newly accepted projection's
/// serialized [`SemanticGraphicalViewModel`] to the page.
///
/// [`SemanticGraphicalViewModel`]: crate::control::SemanticGraphicalViewModel
pub const PROJECTION_EVENT: &str = "crest://projection";

/// What one [`ProjectionChannel::push`] did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionPush {
    /// A new accepted generation was serialized and emitted.
    Emitted,
    /// The projection's generation matched the last emitted one; nothing was
    /// serialized and nothing was emitted.
    Unchanged,
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

/// Generation-gated push channel from accepted projections to the page.
///
/// The channel owns exactly one piece of state — the last successfully
/// emitted generation — and performs no buffering: a push either emits the
/// current document or does nothing.
#[derive(Debug, Default)]
pub struct ProjectionChannel {
    last_emitted_generation: Option<u64>,
}

impl ProjectionChannel {
    /// Creates a channel that has emitted nothing yet, so the first accepted
    /// projection always emits.
    pub const fn new() -> Self {
        Self {
            last_emitted_generation: None,
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
        emit(document).map_err(|source| ProjectionChannelError::Emit { generation, source })?;
        self.last_emitted_generation = Some(generation);
        Ok(ProjectionPush::Emitted)
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectionChannel, ProjectionChannelError, ProjectionPush, PROJECTION_EVENT};
    use crate::control::{
        GraphicalShellProjection, SemanticGraphicalViewModel, ShellContextLine, ShellFooter,
        ShellIdentityHeader, TextProjection, TopLevelContext,
    };
    use serde_json::Value;
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
}
