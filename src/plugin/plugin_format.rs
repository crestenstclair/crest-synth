// path: src/plugin/plugin_format.rs

//! `PluginFormat` identifies the plugin API a build targets: CLAP or VST3.
//!
//! This is a pure value object — no I/O, no allocation beyond the
//! `'static str`s it hands back, and no dependency on any host, audio
//! driver, or window. It exists purely to name a plugin format so the
//! plugin wrapper layer can branch on it without stringly-typed comparisons.

use std::fmt;
use std::str::FromStr;

/// The plugin format a build target implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginFormat {
    Clap,
    Vst3,
}

impl PluginFormat {
    /// All plugin formats this project targets, in a stable order.
    pub fn all() -> [PluginFormat; 2] {
        [PluginFormat::Clap, PluginFormat::Vst3]
    }

    /// Human-readable canonical name for the format (also its `Display`).
    pub fn name(self) -> &'static str {
        match self {
            PluginFormat::Clap => "CLAP",
            PluginFormat::Vst3 => "VST3",
        }
    }

    /// Conventional bundle/file extension for a build of this format.
    pub fn file_extension(self) -> &'static str {
        match self {
            PluginFormat::Clap => "clap",
            PluginFormat::Vst3 => "vst3",
        }
    }
}

impl fmt::Display for PluginFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Error returned when a string does not name a known [`PluginFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsePluginFormatError(());

impl fmt::Display for ParsePluginFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unrecognized plugin format")
    }
}

impl std::error::Error for ParsePluginFormatError {}

impl FromStr for PluginFormat {
    type Err = ParsePluginFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "CLAP" => Ok(PluginFormat::Clap),
            "VST3" => Ok(PluginFormat::Vst3),
            _ => Err(ParsePluginFormatError(())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_extension_matches_format() {
        assert_eq!(PluginFormat::Clap.file_extension(), "clap");
        assert_eq!(PluginFormat::Vst3.file_extension(), "vst3");
    }

    #[test]
    fn display_matches_name() {
        assert_eq!(PluginFormat::Clap.to_string(), "CLAP");
        assert_eq!(PluginFormat::Vst3.to_string(), "VST3");
    }

    #[test]
    fn from_str_round_trips_display() {
        for format in PluginFormat::all() {
            assert_eq!(PluginFormat::from_str(&format.to_string()).unwrap(), format);
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(PluginFormat::from_str("clap").unwrap(), PluginFormat::Clap);
        assert_eq!(PluginFormat::from_str("vst3").unwrap(), PluginFormat::Vst3);
    }

    #[test]
    fn from_str_rejects_unknown_format() {
        assert!(PluginFormat::from_str("au").is_err());
        assert!(PluginFormat::from_str("").is_err());
    }

    #[test]
    fn all_contains_both_variants_exactly_once() {
        let all = PluginFormat::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&PluginFormat::Clap));
        assert!(all.contains(&PluginFormat::Vst3));
    }
}
