// path: src/plugin/plugin_format.rs

/// The plugin format a host loads the engine as.
///
/// # Examples
///
/// ```
/// use crest_synth::plugin::plugin_format::PluginFormat;
///
/// let fmt = PluginFormat::Clap;
/// assert_eq!(fmt.as_str(), "CLAP");
///
/// let fmt2 = PluginFormat::Vst3;
/// assert_eq!(fmt2.as_str(), "VST3");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginFormat {
    /// CLAP (CLever Audio Plugin) format.
    Clap,
    /// VST3 (Virtual Studio Technology 3) format.
    Vst3,
}

impl PluginFormat {
    /// Returns the canonical string representation of this format.
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginFormat::Clap => "CLAP",
            PluginFormat::Vst3 => "VST3",
        }
    }

    /// Returns an iterator over all known plugin formats.
    pub fn all() -> &'static [PluginFormat] {
        &[PluginFormat::Clap, PluginFormat::Vst3]
    }
}

impl std::fmt::Display for PluginFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_as_str() {
        assert_eq!(PluginFormat::Clap.as_str(), "CLAP");
    }

    #[test]
    fn vst3_as_str() {
        assert_eq!(PluginFormat::Vst3.as_str(), "VST3");
    }

    #[test]
    fn display_clap() {
        assert_eq!(format!("{}", PluginFormat::Clap), "CLAP");
    }

    #[test]
    fn display_vst3() {
        assert_eq!(format!("{}", PluginFormat::Vst3), "VST3");
    }

    #[test]
    fn all_contains_both_variants() {
        let all = PluginFormat::all();
        assert!(all.contains(&PluginFormat::Clap));
        assert!(all.contains(&PluginFormat::Vst3));
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn equality() {
        assert_eq!(PluginFormat::Clap, PluginFormat::Clap);
        assert_ne!(PluginFormat::Clap, PluginFormat::Vst3);
    }

    #[test]
    fn clone_and_copy() {
        let fmt = PluginFormat::Vst3;
        let cloned = fmt;
        assert_eq!(fmt, cloned);
    }
}
