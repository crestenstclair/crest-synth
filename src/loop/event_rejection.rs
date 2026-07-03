// path: src/loop/event_rejection.rs

//! EventRejection: why an event was not applicable in the current state.
//!
//! Rejections are values, never panics. Emitting one is how the Loop
//! reducer reports "this event does not apply" (unknown target,
//! out-of-range value) without ever unwinding the control-plane state
//! machine.

/// A value describing why an event was rejected by the Loop reducer.
///
/// Rejections carry a human-readable reason and are returned to callers
/// instead of panicking, so an invalid event never crashes the control
/// plane -- it produces an explicit, inspectable value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventRejection {
    reason: String,
}

impl EventRejection {
    /// Creates a rejection with the given human-readable reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    /// The human-readable reason the event was rejected.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for EventRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_reason() {
        let rejection = EventRejection::new("unknown target");
        assert_eq!(rejection.reason(), "unknown target");
    }

    #[test]
    fn new_accepts_owned_string() {
        let rejection = EventRejection::new(String::from("out-of-range value"));
        assert_eq!(rejection.reason(), "out-of-range value");
    }

    #[test]
    fn display_renders_reason() {
        let rejection = EventRejection::new("unknown target");
        assert_eq!(rejection.to_string(), "unknown target");
    }

    #[test]
    fn equality_compares_by_reason() {
        let a = EventRejection::new("unknown target");
        let b = EventRejection::new("unknown target");
        let c = EventRejection::new("out-of-range value");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
