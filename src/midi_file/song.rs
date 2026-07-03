// path: src/midi_file/song.rs

//! `Song` is a fully parsed MIDI file: an ordered sequence of
//! [`TimedEvent`]s plus the total duration of the piece. It is produced by
//! a MIDI file loader (a non real-time path) and consumed by playback
//! logic that walks the events in order, translating each one, at its own
//! pace, across the real-time boundary.

use crate::midi_file::timed_event::TimedEvent;

/// Errors that can occur while constructing a [`Song`].
#[derive(Debug, Clone, PartialEq)]
pub enum SongError {
    /// `duration_seconds` was negative, NaN, or otherwise not a valid
    /// duration.
    InvalidDuration(f64),
    /// Two consecutive events were not in non-decreasing `at_seconds`
    /// order. `index` is the position of the first offending event.
    EventsNotOrdered { index: usize },
    /// `duration_seconds` was shorter than the last event's `at_seconds`.
    DurationShorterThanLastEvent {
        duration_seconds: f64,
        last_event_at_seconds: f64,
    },
}

impl std::fmt::Display for SongError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SongError::InvalidDuration(value) => {
                write!(f, "invalid song duration: {value}")
            }
            SongError::EventsNotOrdered { index } => {
                write!(
                    f,
                    "events are not ordered by at_seconds ascending at index {index}"
                )
            }
            SongError::DurationShorterThanLastEvent {
                duration_seconds,
                last_event_at_seconds,
            } => write!(
                f,
                "duration_seconds ({duration_seconds}) is shorter than the last event's at_seconds ({last_event_at_seconds})"
            ),
        }
    }
}

impl std::error::Error for SongError {}

/// A fully parsed MIDI file: an ordered list of [`TimedEvent`]s and the
/// total duration, in seconds, of the piece.
///
/// # Invariants
/// - `events` are ordered by `at_seconds` ascending (non-decreasing).
/// - `duration_seconds` is at least the last event's `at_seconds`.
#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    duration_seconds: f64,
    events: Vec<TimedEvent>,
}

impl Song {
    /// Constructs a new `Song`, validating both invariants: events must be
    /// ordered by `at_seconds` ascending, and `duration_seconds` must be at
    /// least the last event's `at_seconds`.
    pub fn try_new(duration_seconds: f64, events: Vec<TimedEvent>) -> Result<Self, SongError> {
        if duration_seconds.is_nan() || duration_seconds.is_infinite() || duration_seconds < 0.0 {
            return Err(SongError::InvalidDuration(duration_seconds));
        }

        for (index, pair) in events.windows(2).enumerate() {
            if pair[0].at_seconds() > pair[1].at_seconds() {
                // `index` is the position of the earlier event in the
                // offending pair, i.e. the first event that breaks order.
                return Err(SongError::EventsNotOrdered { index });
            }
        }

        if let Some(last_event) = events.last() {
            if duration_seconds < last_event.at_seconds() {
                return Err(SongError::DurationShorterThanLastEvent {
                    duration_seconds,
                    last_event_at_seconds: last_event.at_seconds(),
                });
            }
        }

        Ok(Self {
            duration_seconds,
            events,
        })
    }

    /// The total duration of the song, in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.duration_seconds
    }

    /// The song's events, ordered by `at_seconds` ascending.
    pub fn events(&self) -> &[TimedEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(at_seconds: f64) -> TimedEvent {
        TimedEvent::try_new(at_seconds, vec![0x90, 60, 100]).expect("valid timed event")
    }

    #[test]
    fn try_new_accepts_ordered_events_with_sufficient_duration() {
        let events = vec![event(0.0), event(1.0), event(2.0)];
        let song = Song::try_new(3.0, events.clone()).expect("should construct");
        assert_eq!(song.duration_seconds(), 3.0);
        assert_eq!(song.events(), events.as_slice());
    }

    #[test]
    fn try_new_accepts_duration_exactly_matching_last_event() {
        let events = vec![event(0.0), event(2.5)];
        let song = Song::try_new(2.5, events).expect("should construct");
        assert_eq!(song.duration_seconds(), 2.5);
    }

    #[test]
    fn try_new_accepts_no_events() {
        let song = Song::try_new(0.0, vec![]).expect("should construct");
        assert!(song.events().is_empty());
    }

    #[test]
    fn try_new_accepts_simultaneous_events_with_equal_timestamps() {
        let events = vec![event(1.0), event(1.0), event(1.0)];
        let song = Song::try_new(1.0, events).expect("should construct");
        assert_eq!(song.events().len(), 3);
    }

    #[test]
    fn try_new_rejects_unordered_events() {
        let events = vec![event(1.0), event(0.5)];
        let result = Song::try_new(2.0, events);
        assert_eq!(result, Err(SongError::EventsNotOrdered { index: 0 }));
    }

    #[test]
    fn try_new_rejects_duration_shorter_than_last_event() {
        let events = vec![event(0.0), event(5.0)];
        let result = Song::try_new(4.0, events);
        assert_eq!(
            result,
            Err(SongError::DurationShorterThanLastEvent {
                duration_seconds: 4.0,
                last_event_at_seconds: 5.0,
            })
        );
    }

    #[test]
    fn try_new_rejects_negative_duration() {
        let result = Song::try_new(-1.0, vec![]);
        assert_eq!(result, Err(SongError::InvalidDuration(-1.0)));
    }

    #[test]
    fn try_new_rejects_nan_duration() {
        let result = Song::try_new(f64::NAN, vec![]);
        assert!(matches!(result, Err(SongError::InvalidDuration(v)) if v.is_nan()));
    }
}
