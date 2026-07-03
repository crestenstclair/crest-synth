// path: src/sample/loop_mode.rs

//! Playback looping behavior for a sample.
//!
//! `LoopMode` describes how a sample player advances through its buffer once
//! it reaches the loop boundaries. It is a pure value object: no allocation,
//! no I/O, safe to read and copy on the audio thread.

/// How a sample loops during playback.
///
/// - `NoLoop`: play once from start to end, then stop.
/// - `Forward`: loop repeatedly from the loop start to the loop end, moving
///   forward each pass.
/// - `PingPong`: loop between the loop start and loop end, reversing
///   direction at each boundary.
/// - `Release`: loop like `Forward` until note-off, then play through to the
///   end of the sample instead of looping further.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LoopMode {
    /// Play once, no looping.
    #[default]
    NoLoop,
    /// Loop forward repeatedly between loop start and loop end.
    Forward,
    /// Loop back and forth between loop start and loop end.
    PingPong,
    /// Loop until note-off, then play to the end of the sample.
    Release,
}

impl LoopMode {
    /// All variants, in a stable, deliberate order.
    pub const ALL: [LoopMode; 4] = [
        LoopMode::NoLoop,
        LoopMode::Forward,
        LoopMode::PingPong,
        LoopMode::Release,
    ];

    /// Whether this mode ever loops (as opposed to playing straight through).
    pub const fn loops(self) -> bool {
        !matches!(self, LoopMode::NoLoop)
    }

    /// Whether the looping behavior for this mode depends on note-off having
    /// occurred (i.e. it keeps looping only while the note is held).
    pub const fn depends_on_note_off(self) -> bool {
        matches!(self, LoopMode::Release)
    }

    /// Whether this mode reverses direction at the loop boundaries.
    pub const fn is_bidirectional(self) -> bool {
        matches!(self, LoopMode::PingPong)
    }

    /// A short, stable machine name (used for preset serialization keys and
    /// UI labels that must round-trip across versions).
    pub const fn name(self) -> &'static str {
        match self {
            LoopMode::NoLoop => "no_loop",
            LoopMode::Forward => "forward",
            LoopMode::PingPong => "ping_pong",
            LoopMode::Release => "release",
        }
    }

    /// Parse a `LoopMode` from its stable machine name.
    ///
    /// Returns `None` for any string that does not match one of the known
    /// names, so callers (e.g. preset migration code) can decide how to
    /// handle unknown/future values explicitly rather than defaulting
    /// silently.
    pub fn from_name(name: &str) -> Option<LoopMode> {
        LoopMode::ALL.into_iter().find(|mode| mode.name() == name)
    }
}

impl std::fmt::Display for LoopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::LoopMode;

    #[test]
    fn default_is_no_loop() {
        assert_eq!(LoopMode::default(), LoopMode::NoLoop);
    }

    #[test]
    fn no_loop_does_not_loop() {
        assert!(!LoopMode::NoLoop.loops());
        assert!(LoopMode::Forward.loops());
        assert!(LoopMode::PingPong.loops());
        assert!(LoopMode::Release.loops());
    }

    #[test]
    fn only_release_depends_on_note_off() {
        for mode in LoopMode::ALL {
            assert_eq!(mode.depends_on_note_off(), mode == LoopMode::Release);
        }
    }

    #[test]
    fn only_ping_pong_is_bidirectional() {
        for mode in LoopMode::ALL {
            assert_eq!(mode.is_bidirectional(), mode == LoopMode::PingPong);
        }
    }

    #[test]
    fn name_round_trips_through_from_name() {
        for mode in LoopMode::ALL {
            assert_eq!(LoopMode::from_name(mode.name()), Some(mode));
        }
    }

    #[test]
    fn from_name_rejects_unknown_strings() {
        assert_eq!(LoopMode::from_name("bogus"), None);
        assert_eq!(LoopMode::from_name(""), None);
    }

    #[test]
    fn display_matches_name() {
        for mode in LoopMode::ALL {
            assert_eq!(mode.to_string(), mode.name());
        }
    }

    #[test]
    fn all_names_are_distinct() {
        let mut names: Vec<&str> = LoopMode::ALL.iter().map(|m| m.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), LoopMode::ALL.len());
    }
}
