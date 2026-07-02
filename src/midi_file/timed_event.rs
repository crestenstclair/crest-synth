// path: src/midi_file/timed_event.rs

//! A single MIDI event bound to an absolute offset (in seconds) from the
//! start of a `Song`. `TimedEvent` is a plain value object: it owns raw
//! MIDI event bytes plus the timestamp at which the event occurs, and
//! guarantees the timestamp is a well-formed, non-negative instant.

/// Errors that can occur while constructing a [`TimedEvent`].
#[derive(Debug, Clone, PartialEq)]
pub enum TimedEventError {
    /// `at_seconds` was negative, NaN, or otherwise not a valid time offset.
    InvalidTimestamp(f64),
}

impl std::fmt::Display for TimedEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimedEventError::InvalidTimestamp(value) => {
                write!(f, "invalid event timestamp: {value}")
            }
        }
    }
}

impl std::error::Error for TimedEventError {}

/// A MIDI event that occurs at a specific offset (in seconds) from the
/// start of a parsed MIDI file.
///
/// `TimedEvent` stores the raw MIDI status/data bytes for the event rather
/// than a real-time engine type: it is produced by file parsing (a non
/// real-time path) and later translated, one at a time, into whatever
/// representation the real-time boundary (e.g. an `EventRing`) requires.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedEvent {
    at_seconds: f64,
    data: Vec<u8>,
}

impl TimedEvent {
    /// Constructs a new `TimedEvent`, validating that `at_seconds` is a
    /// finite, non-negative offset.
    pub fn try_new(at_seconds: f64, data: Vec<u8>) -> Result<Self, TimedEventError> {
        if at_seconds.is_nan() || at_seconds.is_infinite() || at_seconds < 0.0 {
            return Err(TimedEventError::InvalidTimestamp(at_seconds));
        }
        Ok(Self { at_seconds, data })
    }

    /// The offset, in seconds, from the start of the song at which this
    /// event occurs.
    pub fn at_seconds(&self) -> f64 {
        self.at_seconds
    }

    /// The raw MIDI status/data bytes for this event.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_a_non_negative_timestamp() {
        let event = TimedEvent::try_new(1.5, vec![0x90, 60, 100]).expect("should construct");
        assert_eq!(event.at_seconds(), 1.5);
        assert_eq!(event.data(), &[0x90, 60, 100]);
    }

    #[test]
    fn try_new_accepts_zero() {
        let event = TimedEvent::try_new(0.0, vec![]).expect("should construct");
        assert_eq!(event.at_seconds(), 0.0);
    }

    #[test]
    fn try_new_rejects_negative_timestamp() {
        let result = TimedEvent::try_new(-0.001, vec![0x80, 60, 0]);
        assert_eq!(result, Err(TimedEventError::InvalidTimestamp(-0.001)));
    }

    #[test]
    fn try_new_rejects_nan() {
        let result = TimedEvent::try_new(f64::NAN, vec![]);
        assert!(matches!(result, Err(TimedEventError::InvalidTimestamp(v)) if v.is_nan()));
    }

    #[test]
    fn try_new_rejects_infinite() {
        let result = TimedEvent::try_new(f64::INFINITY, vec![]);
        assert_eq!(
            result,
            Err(TimedEventError::InvalidTimestamp(f64::INFINITY))
        );
    }
}
