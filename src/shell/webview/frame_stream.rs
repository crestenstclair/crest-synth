//! The qualifying-frame stream: the control-side seam scenes block on
//! instead of sleeping (mission webview-shell-cutover WP02, T006).
//!
//! The window's painted-ack forwarding records every forwarded
//! [`ShellFrameObservation`] here, right where it also hands the observation
//! to the `AppWindow` port's frame callback. Control-side consumers (the
//! WP03 scene hosting) hold a [`QualifyingFrameStream`] handle cloned from
//! [`TauriWebviewWindow::frame_stream`] before the window runs, and ask it
//! "has a qualifying frame for this accepted identity been painted?" —
//! either as a non-blocking [`poll`] from inside the tick loop, or as a
//! blocking [`await_qualifying`] with a caller-supplied timeout from a
//! harness thread.
//!
//! # What qualifies
//!
//! A recorded observation qualifies for a [`FrameExpectation`] when its
//! `generation` and `stateHash` equal the awaited accepted generation's and
//! its `context` and `activeSurface` equal the expectation's — the same
//! identity fields the retired live path's crediting started from
//! (`src/testing/live_demo_runner.rs`, `shell_frame_qualifies`). The stream
//! never fabricates or relaxes anything: it only replays observations the
//! forwarding actually constructed from painted acks.
//!
//! # No sleeps, bounded memory
//!
//! [`await_qualifying`] parks on a condition variable and wakes when a frame
//! is recorded or the deadline passes — there is no polling loop and no
//! sleep anywhere in this module. The stream retains only the most recent
//! [`RECENT_OBSERVATION_CAPACITY`] observations; an observation superseded
//! past that window is display evidence already consumed, and losing it
//! degrades observation only.
//!
//! [`poll`]: QualifyingFrameStream::poll
//! [`await_qualifying`]: QualifyingFrameStream::await_qualifying
//! [`TauriWebviewWindow::frame_stream`]: crate::shell::webview::TauriWebviewWindow::frame_stream

use crate::control::{SurfaceId, TopLevelContext};
use crate::shell::ShellFrameObservation;
use core::fmt;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// How many recent forwarded observations the stream retains for polling.
///
/// The live cadence forwards one observation per accepted generation, so a
/// consumer that polls every tick is at most one generation behind; eight
/// leaves generous room for a harness thread that wakes late.
pub const RECENT_OBSERVATION_CAPACITY: usize = 8;

/// The accepted-frame identity a consumer awaits: the accepted generation
/// and state hash, plus the context and active surface the projection
/// declared for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameExpectation {
    generation: u64,
    state_hash: String,
    context: TopLevelContext,
    active_surface: SurfaceId,
}

impl FrameExpectation {
    /// Describes the accepted generation a qualifying frame must have
    /// painted.
    pub fn new(
        generation: u64,
        state_hash: impl Into<String>,
        context: TopLevelContext,
        active_surface: SurfaceId,
    ) -> Self {
        Self {
            generation,
            state_hash: state_hash.into(),
            context,
            active_surface,
        }
    }

    /// The awaited accepted generation.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// The awaited accepted state hash.
    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }

    /// The awaited top-level context.
    pub const fn context(&self) -> TopLevelContext {
        self.context
    }

    /// The awaited active surface.
    pub const fn active_surface(&self) -> SurfaceId {
        self.active_surface
    }

    /// True when the forwarded observation's identity satisfies this
    /// expectation (generation, state hash, context, and active surface all
    /// equal — mirroring the identity gate the retired live crediting applied).
    pub fn matches(&self, observation: &ShellFrameObservation) -> bool {
        observation.generation() == self.generation
            && observation.state_hash() == self.state_hash
            && observation.context() == self.context
            && observation.active_surface() == self.active_surface
    }
}

impl fmt::Display for FrameExpectation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "generation {} state {} context {:?} surface {:?}",
            self.generation, self.state_hash, self.context, self.active_surface
        )
    }
}

/// A typed await failure naming the identity that never painted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameAwaitError {
    /// No qualifying observation was forwarded within the caller's timeout.
    Timeout {
        /// The awaited identity that never received a qualifying frame.
        expectation: FrameExpectation,
        /// How long the caller waited.
        waited: Duration,
    },
}

impl fmt::Display for FrameAwaitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout {
                expectation,
                waited,
            } => write!(
                formatter,
                "no qualifying webview frame for {expectation} within {waited:?}"
            ),
        }
    }
}

impl std::error::Error for FrameAwaitError {}

#[derive(Debug, Default)]
struct StreamShared {
    recent: Mutex<VecDeque<ShellFrameObservation>>,
    arrived: Condvar,
}

/// Shared handle onto the window's forwarded-observation stream.
///
/// Clones share one underlying stream: the window records into it, any
/// number of control-side consumers poll or await on it.
#[derive(Clone, Debug, Default)]
pub struct QualifyingFrameStream {
    shared: Arc<StreamShared>,
}

impl QualifyingFrameStream {
    /// Creates an empty stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one forwarded observation and wakes every waiting consumer.
    ///
    /// Called by the window's painted-ack forwarding, next to the frame
    /// callback — the stream only ever sees observations that were
    /// constructed from a real painted ack.
    pub fn record(&self, observation: ShellFrameObservation) {
        let mut recent = self
            .shared
            .recent
            .lock()
            .expect("the frame stream lock is never poisoned");
        if recent.len() == RECENT_OBSERVATION_CAPACITY {
            recent.pop_front();
        }
        recent.push_back(observation);
        drop(recent);
        self.shared.arrived.notify_all();
    }

    /// Non-blocking: returns the most recent retained observation satisfying
    /// `expectation`, if one has been forwarded.
    pub fn poll(&self, expectation: &FrameExpectation) -> Option<ShellFrameObservation> {
        let recent = self
            .shared
            .recent
            .lock()
            .expect("the frame stream lock is never poisoned");
        recent
            .iter()
            .rev()
            .find(|observation| expectation.matches(observation))
            .cloned()
    }

    /// Blocks until a qualifying observation has been forwarded or `timeout`
    /// passes, whichever comes first. The timeout path is a typed
    /// [`FrameAwaitError::Timeout`] naming the awaited identity.
    ///
    /// The wait parks on a condition variable — no sleeping, no poll loop.
    /// Never call this on the window's event thread: the forwarding that
    /// would satisfy it runs there. It is for harness/control threads that
    /// hold a cloned handle.
    pub fn await_qualifying(
        &self,
        expectation: &FrameExpectation,
        timeout: Duration,
    ) -> Result<ShellFrameObservation, FrameAwaitError> {
        let started = Instant::now();
        let mut recent = self
            .shared
            .recent
            .lock()
            .expect("the frame stream lock is never poisoned");
        loop {
            if let Some(observation) = recent
                .iter()
                .rev()
                .find(|observation| expectation.matches(observation))
            {
                return Ok(observation.clone());
            }
            let waited = started.elapsed();
            let Some(remaining) = timeout.checked_sub(waited) else {
                return Err(FrameAwaitError::Timeout {
                    expectation: expectation.clone(),
                    waited,
                });
            };
            if remaining.is_zero() {
                return Err(FrameAwaitError::Timeout {
                    expectation: expectation.clone(),
                    waited,
                });
            }
            let (guard, _timed_out) = self
                .shared
                .arrived
                .wait_timeout(recent, remaining)
                .expect("the frame stream lock is never poisoned");
            recent = guard;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FrameAwaitError, FrameExpectation, QualifyingFrameStream, RECENT_OBSERVATION_CAPACITY,
    };
    use crate::control::TopLevelContext;
    use crate::shell::{
        ShellFrameObservation, ShellRegionId, ShellRegionObservation, ShellRegionRect,
    };
    use std::time::Duration;

    fn observation(generation: u64, state_hash: &str) -> ShellFrameObservation {
        ShellFrameObservation::try_new(
            1920.0,
            1080.0,
            generation,
            state_hash,
            TopLevelContext::Mixer,
            [
                ShellRegionObservation::new(
                    ShellRegionId::ContextLine,
                    ShellRegionRect::new(0.0, 0.0, 1920.0, 48.0),
                    "CREST SYNTH",
                ),
                ShellRegionObservation::new(
                    ShellRegionId::IdentityHeader,
                    ShellRegionRect::new(0.0, 48.0, 1920.0, 120.0),
                    "MIXER",
                ),
                ShellRegionObservation::new(
                    ShellRegionId::MainWorkspace,
                    ShellRegionRect::new(0.0, 120.0, 1500.0, 1016.0),
                    "MIXER WORKSPACE",
                ),
                ShellRegionObservation::new(
                    ShellRegionId::PersistentSideRegion,
                    ShellRegionRect::new(1500.0, 120.0, 1920.0, 1016.0),
                    "INSPECTOR",
                ),
                ShellRegionObservation::new(
                    ShellRegionId::Footer,
                    ShellRegionRect::new(0.0, 1016.0, 1920.0, 1080.0),
                    "MIXER",
                ),
            ],
        )
        .expect("the stream test fixture observation is coherent")
    }

    fn expectation_for(observation: &ShellFrameObservation) -> FrameExpectation {
        FrameExpectation::new(
            observation.generation(),
            observation.state_hash(),
            observation.context(),
            observation.active_surface(),
        )
    }

    #[test]
    fn a_recorded_qualifying_observation_satisfies_poll_and_await() {
        let stream = QualifyingFrameStream::new();
        let painted = observation(7, "state-7");
        let expectation = expectation_for(&painted);

        assert!(stream.poll(&expectation).is_none(), "nothing painted yet");

        stream.record(painted.clone());

        let polled = stream.poll(&expectation).expect("poll sees the frame");
        assert_eq!(polled, painted);

        // Already satisfied: the await returns immediately without waiting
        // out the timeout.
        let awaited = stream
            .await_qualifying(&expectation, Duration::from_secs(5))
            .expect("an already-painted frame satisfies the await");
        assert_eq!(awaited, painted);
    }

    #[test]
    fn a_non_matching_observation_does_not_qualify() {
        let stream = QualifyingFrameStream::new();
        stream.record(observation(7, "state-7"));

        // Wrong generation and wrong state hash each fail the identity gate.
        let stale_generation = FrameExpectation::new(
            8,
            "state-7",
            TopLevelContext::Mixer,
            observation(7, "state-7").active_surface(),
        );
        assert!(stream.poll(&stale_generation).is_none());

        let stale_hash = FrameExpectation::new(
            7,
            "state-8",
            TopLevelContext::Mixer,
            observation(7, "state-7").active_surface(),
        );
        assert!(stream.poll(&stale_hash).is_none());
    }

    #[test]
    fn a_frame_recorded_from_another_thread_wakes_the_await() {
        let stream = QualifyingFrameStream::new();
        let painted = observation(9, "state-9");
        let expectation = expectation_for(&painted);

        let recorder = {
            let stream = stream.clone();
            let painted = painted.clone();
            std::thread::spawn(move || stream.record(painted))
        };

        let awaited = stream
            .await_qualifying(&expectation, Duration::from_secs(10))
            .expect("the recorded frame satisfies the await");
        assert_eq!(awaited, painted);
        recorder.join().expect("the recorder thread completes");
    }

    #[test]
    fn the_timeout_path_is_a_typed_error_naming_the_awaited_identity() {
        let stream = QualifyingFrameStream::new();
        let expectation = expectation_for(&observation(11, "state-11"));

        let error = stream
            .await_qualifying(&expectation, Duration::from_millis(20))
            .expect_err("an empty stream must time out");
        match &error {
            FrameAwaitError::Timeout {
                expectation: named,
                waited,
            } => {
                assert_eq!(named, &expectation);
                assert!(*waited >= Duration::from_millis(20));
            }
        }
        let text = error.to_string();
        assert!(text.contains("generation 11"), "{text}");
        assert!(text.contains("state-11"), "{text}");
    }

    #[test]
    fn the_stream_retains_only_the_declared_recent_window() {
        let stream = QualifyingFrameStream::new();
        let first = observation(1, "state-1");
        stream.record(first.clone());
        for generation in 2..=(RECENT_OBSERVATION_CAPACITY as u64 + 1) {
            stream.record(observation(generation, &format!("state-{generation}")));
        }

        // The oldest observation fell out of the bounded window; the newest
        // is still visible. Losing a superseded frame degrades observation
        // only.
        assert!(stream.poll(&expectation_for(&first)).is_none());
        let newest = observation(
            RECENT_OBSERVATION_CAPACITY as u64 + 1,
            &format!("state-{}", RECENT_OBSERVATION_CAPACITY as u64 + 1),
        );
        assert!(stream.poll(&expectation_for(&newest)).is_some());
    }
}
