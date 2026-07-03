// path: src/midi_file/sequencer.rs

//! Feeds a [`Song`]'s events into the engine in real time, one audio block
//! at a time.
//!
//! `Sequencer` owns a playback cursor over an immutable `Song`. On each
//! call to [`Sequencer::advance`] it computes the time window covered by
//! the block just rendered and hands, in order, every event whose
//! `at_seconds` falls within that window to a [`MidiEventSink`].
//!
//! # Why a sink, not a direct call into `MidiDispatcher`
//!
//! `domainService.Patch.MidiDispatcher::dispatch` allocates the `Vec` of
//! matching patch ids it returns, and needs the full slice of patches to
//! test against — neither of which is available, or safe, on the
//! real-time audio thread that drives `advance`. `Sequencer` therefore
//! depends on the narrow [`MidiEventSink`] abstraction (Dependency
//! Inversion / Interface Segregation) rather than on the concrete
//! dispatcher: a non-real-time collaborator is expected to receive each
//! forwarded event from the sink (e.g. via the `EventRing`) and, off the
//! audio thread, normalize it and hand it to `MidiDispatcher` exactly as
//! that service's contract describes. This keeps the one auditable seam
//! for crossing the real-time boundary intact.
//!
//! `advance` itself performs no heap allocation, no locking, and no I/O,
//! so it is safe to call from the audio callback's hard-deadline path.
//! `Sequencer` is generic over its sink (`S: MidiEventSink`) rather than
//! storing a trait object, so dispatch from the inner loop is statically
//! resolved rather than dynamic.

use crate::midi_file::song::Song;
use crate::midi_file::timed_event::TimedEvent;

/// Destination for events emitted by a [`Sequencer`] during playback.
///
/// Deliberately narrow (Interface Segregation): a `Sequencer` only ever
/// needs to hand off one event at a time, in order. Concrete
/// implementations (e.g. one that pushes a normalized representation onto
/// an `EventRing`, or a test recorder) live outside this module.
pub trait MidiEventSink {
    /// Receives a single event that falls within the block just advanced.
    /// Called in ascending `at_seconds` order for a given `advance` call,
    /// and never for an event already delivered by a previous call.
    fn dispatch(&mut self, event: &TimedEvent);
}

/// Whether a [`Sequencer`] restarts from the beginning of the song after
/// reaching the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    /// Playback stops emitting further events once the song's duration
    /// has elapsed.
    Once,
    /// Playback wraps back to the start of the song once its duration has
    /// elapsed, continuing to emit events indefinitely.
    Loop,
}

/// Feeds a [`Song`]'s events into the engine in real time, block by block.
///
/// `Sequencer` owns its own playback cursor (`position_seconds` and the
/// index of the next event to emit), so successive calls to `advance` are
/// cheap, allocation-free, and lock-free: nothing needed to make progress
/// lives anywhere but in this struct's fields and the immutable `Song` it
/// was constructed with.
pub struct Sequencer<S: MidiEventSink> {
    song: Song,
    loop_mode: LoopMode,
    position_seconds: f64,
    next_event_index: usize,
    sink: S,
}

impl<S: MidiEventSink> Sequencer<S> {
    /// Constructs a new `Sequencer` starting at the beginning of `song`,
    /// forwarding emitted events to `sink`.
    ///
    /// `sink` is accepted by the constructor (Dependency Injection) so
    /// tests can supply a recording fake without touching production
    /// wiring.
    pub fn new(song: Song, loop_mode: LoopMode, sink: S) -> Self {
        Self {
            song,
            loop_mode,
            position_seconds: 0.0,
            next_event_index: 0,
            sink,
        }
    }

    /// The song this sequencer is playing.
    pub fn song(&self) -> &Song {
        &self.song
    }

    /// The current playback position, in seconds from the start of the
    /// song.
    pub fn position_seconds(&self) -> f64 {
        self.position_seconds
    }

    /// Whether playback has reached the end of a non-looping song with no
    /// further events left to emit.
    pub fn is_finished(&self) -> bool {
        self.loop_mode == LoopMode::Once
            && self.position_seconds >= self.song.duration_seconds()
            && self.next_event_index >= self.song.events().len()
    }

    /// Resets playback to the beginning of the song without changing the
    /// song, loop mode, or sink.
    pub fn reset(&mut self) {
        self.position_seconds = 0.0;
        self.next_event_index = 0;
    }

    /// Advances playback by `block_seconds` and emits, in order, every
    /// event whose `at_seconds` falls within the resulting time window to
    /// the sink supplied at construction.
    ///
    /// This is the real-time hot path: it performs no heap allocation, no
    /// locking, and no blocking I/O. Non-finite or non-positive
    /// `block_seconds` values are ignored (no time is considered to have
    /// passed).
    pub fn advance(&mut self, block_seconds: f64) {
        if !block_seconds.is_finite() || block_seconds <= 0.0 {
            return;
        }

        let duration = self.song.duration_seconds();

        // A zero-duration song has no time to loop across. By the `Song`
        // invariant (`duration_seconds` is at least the last event's
        // `at_seconds`, and every `at_seconds` is non-negative) the only
        // events such a song can hold sit at exactly `0.0`. Emit them
        // once and stop — looping here would mean "no time passes but we
        // dispatch forever", which would violate the audio thread's hard
        // deadline.
        if duration <= 0.0 {
            self.drain_ready(f64::INFINITY);
            return;
        }

        let mut remaining = block_seconds;
        while remaining > 0.0 {
            let time_to_end = duration - self.position_seconds;
            if remaining < time_to_end {
                self.position_seconds += remaining;
                let horizon = self.position_seconds;
                self.drain_ready(horizon);
                remaining = 0.0;
            } else {
                // This step reaches (or exactly lands on) the end of the
                // song. Flush every remaining event up to and including
                // `duration` before deciding whether to wrap or stop.
                self.drain_ready(duration);
                remaining -= time_to_end;
                match self.loop_mode {
                    LoopMode::Loop => {
                        self.position_seconds = 0.0;
                        self.next_event_index = 0;
                    }
                    LoopMode::Once => {
                        self.position_seconds = duration;
                        remaining = 0.0;
                    }
                }
            }
        }
    }

    /// Dispatches every not-yet-emitted event whose `at_seconds` is less
    /// than or equal to `horizon`, in ascending order, advancing
    /// `next_event_index` past each one.
    fn drain_ready(&mut self, horizon: f64) {
        let events = self.song.events();
        while self.next_event_index < events.len() {
            let event = &events[self.next_event_index];
            if event.at_seconds() > horizon {
                break;
            }
            self.sink.dispatch(event);
            self.next_event_index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingSink {
        received: Vec<TimedEvent>,
    }

    impl MidiEventSink for RecordingSink {
        fn dispatch(&mut self, event: &TimedEvent) {
            self.received.push(event.clone());
        }
    }

    fn event(at_seconds: f64) -> TimedEvent {
        TimedEvent::try_new(at_seconds, vec![0x90, 60, 100]).expect("valid timed event")
    }

    fn song(duration: f64, events: Vec<TimedEvent>) -> Song {
        Song::try_new(duration, events).expect("valid song")
    }

    #[test]
    fn emits_only_events_within_the_first_block() {
        let s = song(4.0, vec![event(0.5), event(1.5), event(3.5)]);
        let mut seq = Sequencer::new(s, LoopMode::Once, RecordingSink::default());

        seq.advance(1.0);

        assert_eq!(seq.sink.received, vec![event(0.5)]);
        assert_eq!(seq.position_seconds(), 1.0);
    }

    #[test]
    fn emits_events_in_order_across_successive_blocks() {
        let s = song(4.0, vec![event(0.5), event(1.5), event(3.5)]);
        let mut seq = Sequencer::new(s, LoopMode::Once, RecordingSink::default());

        seq.advance(1.0); // [0.0, 1.0) -> 0.5
        seq.advance(1.0); // [1.0, 2.0) -> 1.5
        seq.advance(1.0); // [2.0, 3.0) -> nothing
        seq.advance(1.0); // [3.0, 4.0) -> 3.5

        assert_eq!(seq.sink.received, vec![event(0.5), event(1.5), event(3.5)]);
    }

    #[test]
    fn does_not_reemit_an_event_already_delivered() {
        let s = song(2.0, vec![event(0.5)]);
        let mut seq = Sequencer::new(s, LoopMode::Once, RecordingSink::default());

        seq.advance(1.0);
        seq.advance(1.0);

        assert_eq!(seq.sink.received, vec![event(0.5)]);
    }

    #[test]
    fn once_mode_stops_emitting_after_the_song_ends() {
        let s = song(1.0, vec![event(0.5)]);
        let mut seq = Sequencer::new(s, LoopMode::Once, RecordingSink::default());

        seq.advance(1.5);
        assert!(seq.is_finished());

        seq.advance(1.0);
        assert_eq!(seq.sink.received, vec![event(0.5)]);
    }

    #[test]
    fn loop_mode_wraps_back_to_the_start() {
        let s = song(1.0, vec![event(0.25), event(0.75)]);
        let mut seq = Sequencer::new(s, LoopMode::Loop, RecordingSink::default());

        // One full block spans exactly one lap plus a bit into the next.
        seq.advance(1.5);

        assert_eq!(
            seq.sink.received,
            vec![event(0.25), event(0.75), event(0.25)]
        );
        assert_eq!(seq.position_seconds(), 0.5);
        assert!(!seq.is_finished());
    }

    #[test]
    fn loop_mode_handles_multiple_wraps_within_a_single_block() {
        let s = song(1.0, vec![event(0.5)]);
        let mut seq = Sequencer::new(s, LoopMode::Loop, RecordingSink::default());

        seq.advance(3.25);

        assert_eq!(seq.sink.received, vec![event(0.5), event(0.5), event(0.5)]);
        assert_eq!(seq.position_seconds(), 0.25);
    }

    #[test]
    fn reset_returns_to_the_beginning() {
        let s = song(2.0, vec![event(0.5), event(1.5)]);
        let mut seq = Sequencer::new(s, LoopMode::Once, RecordingSink::default());

        seq.advance(1.0);
        seq.reset();

        // `reset` alone (without a further `advance`) puts playback back
        // at the very start of the song.
        assert_eq!(seq.position_seconds(), 0.0);

        seq.advance(1.0);

        // Because `reset` rewound the event cursor as well as the clock,
        // the event at 0.5s is delivered again on this second pass, and
        // the position reflects the 1.0s that has now elapsed since the
        // reset.
        assert_eq!(seq.sink.received, vec![event(0.5), event(0.5)]);
        assert_eq!(seq.position_seconds(), 1.0);
    }

    #[test]
    fn non_positive_block_seconds_is_a_no_op() {
        let s = song(2.0, vec![event(0.5)]);
        let mut seq = Sequencer::new(s, LoopMode::Once, RecordingSink::default());

        seq.advance(0.0);
        seq.advance(-1.0);
        seq.advance(f64::NAN);

        assert!(seq.sink.received.is_empty());
        assert_eq!(seq.position_seconds(), 0.0);
    }

    #[test]
    fn empty_song_never_dispatches() {
        let s = song(3.0, vec![]);
        let mut seq = Sequencer::new(s, LoopMode::Loop, RecordingSink::default());

        seq.advance(10.0);

        assert!(seq.sink.received.is_empty());
    }

    #[test]
    fn zero_duration_song_emits_pinned_events_once_and_then_stops() {
        let s = song(0.0, vec![event(0.0), event(0.0)]);
        let mut seq = Sequencer::new(s, LoopMode::Loop, RecordingSink::default());

        seq.advance(1.0);
        seq.advance(1.0);

        assert_eq!(seq.sink.received, vec![event(0.0), event(0.0)]);
    }
}
