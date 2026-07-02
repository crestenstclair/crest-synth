// path: src/sample/sample_loader.rs

//! Port: `SampleLoader`.
//!
//! Defines the boundary between the sample-library domain and the outside
//! world (filesystem, sample-format parsers). Implementations live in the
//! adapter layer (e.g. an SF2 bank parser, a WAV reader) and are injected
//! into callers that need sample data — callers depend on this trait, not
//! on any concrete file-format parser (Dependency Inversion).
//!
//! This is a non-real-time boundary: loading involves blocking file I/O and
//! heap allocation, so implementations must run on the UI/loader thread,
//! never inside the audio callback. Loaded results reach the audio thread
//! only via the ParameterBridge/EventRing handoff performed by the caller,
//! never directly from this trait.

use std::error::Error;
use std::fmt;
use std::path::Path;

/// A single audio sample: interleaved PCM frames plus the metadata needed to
/// play them back correctly (sample rate, channel count, and the MIDI
/// note/velocity range and root key it was recorded at).
#[derive(Debug, Clone, PartialEq)]
pub struct SampleData {
    /// Sample rate in Hz, e.g. `44_100`.
    pub sample_rate_hz: u32,
    /// Number of interleaved channels (1 = mono, 2 = stereo).
    pub channel_count: u16,
    /// Interleaved f32 PCM frames.
    pub frames: Vec<f32>,
    /// MIDI note this sample was recorded at (0-127).
    pub root_key: u8,
    /// Inclusive MIDI note range this sample covers (low, high), both 0-127.
    pub key_range: (u8, u8),
    /// Inclusive MIDI velocity range this sample covers (low, high), both
    /// 0-127.
    pub velocity_range: (u8, u8),
}

/// A named collection of samples that together form a playable instrument
/// (e.g. one preset extracted from an SF2 bank, or a single loaded WAV
/// treated as a one-sample instrument).
#[derive(Debug, Clone, PartialEq)]
pub struct SampleSet {
    pub name: String,
    pub samples: Vec<SampleData>,
}

impl SampleSet {
    /// Constructs a `SampleSet` from a name and its samples. An empty
    /// `samples` list is a legal (if unplayable) starting point that
    /// adapters may populate incrementally while parsing.
    pub fn new(name: impl Into<String>, samples: Vec<SampleData>) -> Self {
        Self {
            name: name.into(),
            samples,
        }
    }
}

/// Failure modes when loading sample data from a file.
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be opened or read (permissions, missing file, I/O
    /// error). Carries the OS-provided error for diagnostics.
    Io(std::io::Error),
    /// The file was read but its contents do not conform to the expected
    /// format (corrupt or unsupported SF2/WAV structure).
    Format(String),
    /// The file extension/content did not match the loader that was asked
    /// to parse it (e.g. `load_wav` given an SF2 file).
    UnsupportedFormat(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(err) => write!(f, "sample load I/O error: {err}"),
            LoadError::Format(msg) => write!(f, "malformed sample file: {msg}"),
            LoadError::UnsupportedFormat(msg) => write!(f, "unsupported sample format: {msg}"),
        }
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LoadError::Io(err) => Some(err),
            LoadError::Format(_) | LoadError::UnsupportedFormat(_) => None,
        }
    }
}

impl From<std::io::Error> for LoadError {
    fn from(err: std::io::Error) -> Self {
        LoadError::Io(err)
    }
}

/// Port: loads sample data from files on disk.
///
/// This trait is the single seam between the sample-library domain and
/// concrete file formats. Implementations (SF2 bank parsers, WAV readers,
/// future formats) live behind this interface so callers depend on the
/// abstraction rather than any one format's parser. Runs on the
/// non-real-time loader thread only — never call from the audio callback.
pub trait SampleLoader {
    /// Loads every preset/instrument found in an SF2 sound bank as a list of
    /// `SampleSet`s, one per preset.
    fn load_sf2(&self, path: &Path) -> Result<Vec<SampleSet>, LoadError>;

    /// Loads a single WAV file as one `SampleSet` containing exactly one
    /// sample spanning the full key/velocity range unless the WAV's own
    /// metadata narrows it.
    fn load_wav(&self, path: &Path) -> Result<SampleSet, LoadError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    struct StubLoader {
        sf2_result: fn() -> Result<Vec<SampleSet>, LoadError>,
        wav_result: fn() -> Result<SampleSet, LoadError>,
    }

    impl SampleLoader for StubLoader {
        fn load_sf2(&self, _path: &Path) -> Result<Vec<SampleSet>, LoadError> {
            (self.sf2_result)()
        }

        fn load_wav(&self, _path: &Path) -> Result<SampleSet, LoadError> {
            (self.wav_result)()
        }
    }

    fn sample_data(root_key: u8) -> SampleData {
        SampleData {
            sample_rate_hz: 44_100,
            channel_count: 1,
            frames: vec![0.0, 0.25, -0.25, 0.0],
            root_key,
            key_range: (0, 127),
            velocity_range: (0, 127),
        }
    }

    #[test]
    fn load_wav_returns_a_single_sample_set() {
        let loader = StubLoader {
            sf2_result: || Ok(vec![]),
            wav_result: || Ok(SampleSet::new("kick", vec![sample_data(36)])),
        };

        let result = loader.load_wav(Path::new("kick.wav"));

        let set = result.expect("expected a loaded sample set");
        assert_eq!(set.name, "kick");
        assert_eq!(set.samples.len(), 1);
        assert_eq!(set.samples[0].root_key, 36);
    }

    #[test]
    fn load_sf2_returns_one_sample_set_per_preset() {
        let loader = StubLoader {
            sf2_result: || {
                Ok(vec![
                    SampleSet::new("piano", vec![sample_data(60)]),
                    SampleSet::new("strings", vec![sample_data(60)]),
                ])
            },
            wav_result: || Ok(SampleSet::new("unused", vec![])),
        };

        let sets = loader
            .load_sf2(Path::new("bank.sf2"))
            .expect("expected sf2 presets to load");

        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].name, "piano");
        assert_eq!(sets[1].name, "strings");
    }

    #[test]
    fn load_wav_propagates_io_errors() {
        let loader = StubLoader {
            sf2_result: || Ok(vec![]),
            wav_result: || {
                Err(LoadError::from(io::Error::new(
                    io::ErrorKind::NotFound,
                    "missing",
                )))
            },
        };

        let result = loader.load_wav(Path::new("missing.wav"));

        assert!(matches!(result, Err(LoadError::Io(_))));
    }

    #[test]
    fn load_sf2_reports_format_errors_for_corrupt_banks() {
        let loader = StubLoader {
            sf2_result: || Err(LoadError::Format("bad RIFF header".to_string())),
            wav_result: || Ok(SampleSet::new("unused", vec![])),
        };

        let result = loader.load_sf2(Path::new("corrupt.sf2"));

        assert!(matches!(result, Err(LoadError::Format(_))));
    }

    #[test]
    fn load_error_display_messages_are_distinct_per_variant() {
        let io_err = LoadError::from(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        let format_err = LoadError::Format("truncated chunk".to_string());
        let unsupported_err = LoadError::UnsupportedFormat("ogg".to_string());

        assert!(io_err.to_string().contains("I/O error"));
        assert!(format_err.to_string().contains("malformed"));
        assert!(unsupported_err.to_string().contains("unsupported"));
    }
}
