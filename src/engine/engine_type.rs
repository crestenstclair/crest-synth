// path: src/engine/engine_type.rs

//! `EngineType` identifies which synthesis engine a `Patch` uses.
//!
//! This is a pure value object: it carries no runtime state of its own. The
//! `SamplePlayback` variant does not embed sample data — it is a discriminator
//! that tells patch-building code to delegate rendering to the `sample`
//! context (see `crate::sample::sample_set`). Keeping delegation as a matter
//! of dispatch (via this enum) rather than a compile-time dependency keeps
//! `engine` free of a hard dependency on `sample`'s internals.

use std::fmt;

/// Which synthesis technique produces samples for a patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineType {
    /// Analog-modeled subtractive synthesis (oscillators + filter + envelopes).
    VirtualAnalog,
    /// Wavetable synthesis: scanning through a table of single-cycle waveforms.
    Wavetable,
    /// Sample playback: rendering is delegated to the Sample context's
    /// sample-set machinery rather than an oscillator or operator model.
    SamplePlayback,
    /// Frequency modulation synthesis (operator-based).
    Fm,
}

impl EngineType {
    /// All engine types, in a stable declaration order.
    pub const ALL: [EngineType; 4] = [
        EngineType::VirtualAnalog,
        EngineType::Wavetable,
        EngineType::SamplePlayback,
        EngineType::Fm,
    ];

    /// True when this engine type delegates rendering to the Sample context
    /// instead of synthesizing from an oscillator/operator model.
    pub fn delegates_to_sample_context(self) -> bool {
        matches!(self, EngineType::SamplePlayback)
    }

    /// A short, stable machine-readable name (used by preset codecs).
    pub fn as_str(self) -> &'static str {
        match self {
            EngineType::VirtualAnalog => "virtual_analog",
            EngineType::Wavetable => "wavetable",
            EngineType::SamplePlayback => "sample_playback",
            EngineType::Fm => "fm",
        }
    }

    /// Parse the machine-readable name back into an `EngineType`.
    ///
    /// Returns `None` for any string that isn't one of the canonical names
    /// produced by [`EngineType::as_str`]. Callers that need version
    /// migration (e.g. a renamed engine in an older preset format) should
    /// translate the string before calling this.
    pub fn parse(s: &str) -> Option<EngineType> {
        match s {
            "virtual_analog" => Some(EngineType::VirtualAnalog),
            "wavetable" => Some(EngineType::Wavetable),
            "sample_playback" => Some(EngineType::SamplePlayback),
            "fm" => Some(EngineType::Fm),
            _ => None,
        }
    }
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for EngineType {
    /// Virtual analog is the default engine for a freshly created patch.
    fn default() -> Self {
        EngineType::VirtualAnalog
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_and_parse_round_trip_for_every_variant() {
        for engine in EngineType::ALL {
            let s = engine.as_str();
            assert_eq!(EngineType::parse(s), Some(engine));
        }
    }

    #[test]
    fn parse_rejects_unknown_strings() {
        assert_eq!(EngineType::parse("granular"), None);
        assert_eq!(EngineType::parse(""), None);
    }

    #[test]
    fn only_sample_playback_delegates_to_sample_context() {
        for engine in EngineType::ALL {
            let expected = matches!(engine, EngineType::SamplePlayback);
            assert_eq!(engine.delegates_to_sample_context(), expected);
        }
    }

    #[test]
    fn display_matches_as_str() {
        for engine in EngineType::ALL {
            assert_eq!(engine.to_string(), engine.as_str());
        }
    }

    #[test]
    fn default_is_virtual_analog() {
        assert_eq!(EngineType::default(), EngineType::VirtualAnalog);
    }
}
