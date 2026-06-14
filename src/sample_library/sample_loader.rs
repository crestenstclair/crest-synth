// path: src/sample_library/sample_loader.rs
//
// SampleLoader — application service that decodes sample files from disk
// into SampleSet aggregates.
//
// Design notes
// ------------
//   • SampleLoader ONLY runs on the application (non-audio) thread. It performs
//     blocking I/O and allocates heap memory freely — both are forbidden on the
//     audio thread.
//   • WAV files are decoded via `hound`. Each WAV file becomes a single-zone
//     SampleSet covering the full key/velocity range unless metadata overrides
//     are supplied.
//   • SF2 (SoundFont 2) loading is represented by a typed stub. Full SF2 parsing
//     is complex; production code would use a dedicated SF2 parsing crate. The
//     stub returns a clear error so callers can handle the unsupported case.
//   • Decoded PCM data is stored as `Arc<[f32]>` — the audio thread reads via
//     the shared reference without any allocation or lock.
//   • The loader does NOT hold any state; it is a pure function-set. Call sites
//     that need to accumulate loaded sets use `SampleLibrary`.

use std::path::Path;
use std::sync::Arc;

use crate::kernel::note_number::NoteNumber;
use crate::kernel::sample_rate::SampleRate;
use crate::kernel::velocity::Velocity;
use crate::sample_library::key_velocity_range::KeyVelocityRange;
use crate::sample_library::sample_format::SampleFormat;
use crate::sample_library::sample_metadata::SampleMetadata;
use crate::sample_library::sample_set::{SampleSet, SampleSetCommand};
use crate::sample_library::sample_set_id::SampleSetId;
use crate::sample_library::sample_zone::SampleZone;

/// Errors that can occur during sample loading.
#[derive(Debug)]
pub enum SampleLoadError {
    /// The file could not be opened or read.
    Io(std::io::Error),
    /// WAV decoding failed.
    WavDecode(hound::Error),
    /// The WAV file has zero channels.
    InvalidChannelCount(u16),
    /// The WAV file reports zero frames.
    InvalidFrameCount,
    /// SF2 loading is not yet implemented.
    Sf2NotSupported,
    /// A path was provided with an unrecognised extension.
    UnknownFormat(String),
    /// A value derived from file metadata was out of the expected range.
    MetadataOutOfRange(String),
}

impl std::fmt::Display for SampleLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleLoadError::Io(e) => write!(f, "I/O error: {e}"),
            SampleLoadError::WavDecode(e) => write!(f, "WAV decode error: {e}"),
            SampleLoadError::InvalidChannelCount(c) => {
                write!(f, "WAV file has invalid channel count: {c}")
            }
            SampleLoadError::InvalidFrameCount => write!(f, "WAV file contains no audio frames"),
            SampleLoadError::Sf2NotSupported => write!(f, "SF2 loading is not yet supported"),
            SampleLoadError::UnknownFormat(ext) => {
                write!(f, "unrecognised sample format extension: {ext}")
            }
            SampleLoadError::MetadataOutOfRange(msg) => {
                write!(f, "metadata value out of range: {msg}")
            }
        }
    }
}

impl std::error::Error for SampleLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SampleLoadError::Io(e) => Some(e),
            SampleLoadError::WavDecode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SampleLoadError {
    fn from(e: std::io::Error) -> Self {
        SampleLoadError::Io(e)
    }
}

impl From<hound::Error> for SampleLoadError {
    fn from(e: hound::Error) -> Self {
        SampleLoadError::WavDecode(e)
    }
}

/// Optional per-zone metadata overrides provided by the caller.
///
/// When loading a WAV file without embedded metadata (e.g., a raw sample),
/// the caller can supply the root note and the key/velocity range the zone
/// should occupy. All fields are optional; sensible defaults are applied when
/// omitted (root = C4 / note 60, full key+velocity range).
#[derive(Debug, Clone, Default)]
pub struct WavLoadOptions {
    /// Root (tuning) note for the sample. Defaults to MIDI note 60 (C4).
    pub root_note: Option<NoteNumber>,
    /// Low end of the key range. Defaults to note 0.
    pub key_low: Option<NoteNumber>,
    /// High end of the key range. Defaults to note 127.
    pub key_high: Option<NoteNumber>,
    /// Low end of the velocity range. Defaults to 0.0.
    pub velocity_low: Option<Velocity>,
    /// High end of the velocity range. Defaults to 1.0.
    pub velocity_high: Option<Velocity>,
    /// Human-readable name for the resulting SampleSet. Defaults to the file
    /// stem of the path.
    pub name: Option<String>,
}

/// The `SampleLoader` application service.
///
/// Decodes sample files from disk into [`SampleSet`] aggregates. All methods
/// perform blocking I/O and heap allocation and **must only be called from the
/// application thread**, never from the audio thread.
///
/// # Examples
///
/// ```no_run
/// use crest_synth::sample_library::sample_loader::{SampleLoader, WavLoadOptions};
/// use crest_synth::sample_library::sample_set_id::SampleSetId;
///
/// let loader = SampleLoader::new();
/// let id = SampleSetId::new(1);
/// let result = loader.load_wav("piano_c4.wav", id, WavLoadOptions::default());
/// // result is Ok(SampleSet) on success, Err(SampleLoadError) on failure
/// ```
#[derive(Debug, Default)]
pub struct SampleLoader;

impl SampleLoader {
    /// Create a new `SampleLoader`.
    pub fn new() -> Self {
        Self
    }

    /// Load a WAV file from `path` and produce a [`SampleSet`] with `id`.
    ///
    /// The WAV file is decoded to interleaved `f32` samples. A single
    /// [`SampleZone`] is created that spans the key/velocity range supplied in
    /// `opts` (or the full range if not specified).
    ///
    /// # Errors
    ///
    /// Returns `Err` if the file cannot be opened, the WAV format is invalid,
    /// or any metadata value falls outside its legal range.
    pub fn load_wav(
        &self,
        path: impl AsRef<Path>,
        id: SampleSetId,
        opts: WavLoadOptions,
    ) -> Result<SampleSet, SampleLoadError> {
        let path = path.as_ref();

        let mut reader = hound::WavReader::open(path)?;
        let spec = reader.spec();

        let channels = spec.channels;
        if channels == 0 {
            return Err(SampleLoadError::InvalidChannelCount(channels));
        }

        // Decode all samples to f32, normalising from integer formats.
        let samples_f32: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(SampleLoadError::WavDecode)?,
            hound::SampleFormat::Int => {
                let bit_depth = spec.bits_per_sample;
                let scale = 1.0_f32 / (1u32 << (bit_depth - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|s| s.map(|v| v as f32 * scale))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(SampleLoadError::WavDecode)?
            }
        };

        let total_samples = samples_f32.len();
        let channels_usize = channels as usize;
        let frame_count = total_samples.checked_div(channels_usize).unwrap_or(0);

        if frame_count == 0 {
            return Err(SampleLoadError::InvalidFrameCount);
        }

        // Build SampleMetadata
        let sample_rate = SampleRate::try_new(spec.sample_rate)
            .map_err(|e| SampleLoadError::MetadataOutOfRange(e.to_string()))?;

        let channels_u8 =
            u8::try_from(channels).map_err(|_| SampleLoadError::InvalidChannelCount(channels))?;

        let length_frames = frame_count as u64;

        let root_note = opts
            .root_note
            .unwrap_or_else(|| NoteNumber::try_new(60).expect("60 is a valid MIDI note"));

        let metadata = SampleMetadata::try_new(
            channels_u8,
            length_frames,
            None,
            None,
            root_note,
            sample_rate,
        )
        .map_err(|e| SampleLoadError::MetadataOutOfRange(e.to_string()))?;

        // Build KeyVelocityRange
        let key_low = opts
            .key_low
            .unwrap_or_else(|| NoteNumber::try_new(0).expect("0 is valid"));
        let key_high = opts
            .key_high
            .unwrap_or_else(|| NoteNumber::try_new(127).expect("127 is valid"));
        let velocity_low = opts
            .velocity_low
            .unwrap_or_else(|| Velocity::try_new(0.0).expect("0.0 is valid"));
        let velocity_high = opts
            .velocity_high
            .unwrap_or_else(|| Velocity::try_new(1.0).expect("1.0 is valid"));

        let range = KeyVelocityRange::try_new(key_low, key_high, velocity_low, velocity_high)
            .map_err(|e| SampleLoadError::MetadataOutOfRange(e.to_string()))?;

        let sample_data: Arc<[f32]> = samples_f32.into();
        let zone = SampleZone::new(metadata, range, sample_data);

        // Determine set name
        let name = opts.name.unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

        let mut set = SampleSet::new(
            id,
            name,
            crate::sample_library::sample_format::SampleFormat::Wav,
        );
        set.add_zone(zone)
            .map_err(|e| SampleLoadError::MetadataOutOfRange(e.to_string()))?;

        Ok(set)
    }

    /// Attempt to detect the sample format from a file extension and dispatch
    /// to the appropriate loader.
    ///
    /// Currently only WAV is supported; SF2 returns
    /// [`SampleLoadError::Sf2NotSupported`].
    pub fn load_auto(
        &self,
        path: impl AsRef<Path>,
        id: SampleSetId,
        opts: WavLoadOptions,
    ) -> Result<SampleSet, SampleLoadError> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match ext.as_deref() {
            Some("wav") | Some("wave") => self.load_wav(path, id, opts),
            Some("sf2") => Err(SampleLoadError::Sf2NotSupported),
            Some(other) => Err(SampleLoadError::UnknownFormat(other.to_string())),
            None => Err(SampleLoadError::UnknownFormat(String::new())),
        }
    }

    /// Process a [`SampleSetCommand::LoadSampleSet`] command by loading the
    /// referenced file and returning the constructed [`SampleSet`].
    ///
    /// Returns `None` for `UnloadSampleSet` commands (those are handled by
    /// [`SampleLibrary`](crate::sample_library::sample_set::SampleLibrary) directly).
    pub fn process_command(
        &self,
        command: &SampleSetCommand,
        id: SampleSetId,
    ) -> Option<Result<SampleSet, SampleLoadError>> {
        match command {
            SampleSetCommand::LoadSampleSet { path, format } => {
                let opts = WavLoadOptions::default();
                let result = match format {
                    SampleFormat::Wav => self.load_wav(path, id, opts),
                    SampleFormat::Sf2 => Err(SampleLoadError::Sf2NotSupported),
                    _ => Err(SampleLoadError::UnknownFormat(format.to_string())),
                };
                Some(result)
            }
            SampleSetCommand::UnloadSampleSet { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// Write a minimal valid 16-bit mono WAV file and return the temp file.
    fn write_test_wav(sample_rate: u32, channels: u16, frames: usize) -> NamedTempFile {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut tmp = NamedTempFile::new().expect("tempfile");
        {
            let mut writer = hound::WavWriter::new(&mut tmp, spec).unwrap();
            for _ in 0..(frames * channels as usize) {
                writer.write_sample(0i16).unwrap();
            }
            writer.finalize().unwrap();
        }
        tmp
    }

    /// Write a 32-bit float WAV file.
    fn write_float_wav(sample_rate: u32, channels: u16, frames: usize) -> NamedTempFile {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut tmp = NamedTempFile::new().expect("tempfile");
        {
            let mut writer = hound::WavWriter::new(&mut tmp, spec).unwrap();
            for _ in 0..(frames * channels as usize) {
                writer.write_sample(0.0f32).unwrap();
            }
            writer.finalize().unwrap();
        }
        tmp
    }

    #[test]
    fn sample_loader_load_wav_mono_succeeds() {
        let tmp = write_test_wav(44100, 1, 512);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(1);
        let set = loader
            .load_wav(tmp.path(), id, WavLoadOptions::default())
            .expect("load should succeed");

        assert_eq!(set.id, id);
        assert_eq!(set.zone_count(), 1);
        let zone = &set.zones()[0];
        assert_eq!(zone.metadata().channels, 1);
        assert_eq!(
            zone.metadata().sample_rate,
            SampleRate::try_new(44100).unwrap()
        );
        assert_eq!(zone.frame_count(), 512);
    }

    #[test]
    fn sample_loader_load_wav_stereo_succeeds() {
        let tmp = write_test_wav(48000, 2, 256);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(2);
        let set = loader
            .load_wav(tmp.path(), id, WavLoadOptions::default())
            .expect("stereo load should succeed");

        assert_eq!(set.zone_count(), 1);
        let zone = &set.zones()[0];
        assert_eq!(zone.metadata().channels, 2);
        assert_eq!(zone.frame_count(), 256);
    }

    #[test]
    fn sample_loader_load_wav_float_format_succeeds() {
        let tmp = write_float_wav(44100, 1, 128);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(3);
        let set = loader
            .load_wav(tmp.path(), id, WavLoadOptions::default())
            .expect("float WAV load should succeed");

        assert_eq!(set.zone_count(), 1);
        assert_eq!(set.zones()[0].frame_count(), 128);
    }

    #[test]
    fn sample_loader_load_wav_custom_root_note() {
        let tmp = write_test_wav(44100, 1, 64);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(4);
        let root = NoteNumber::try_new(69).unwrap(); // A4
        let opts = WavLoadOptions {
            root_note: Some(root),
            ..Default::default()
        };
        let set = loader
            .load_wav(tmp.path(), id, opts)
            .expect("load with custom root should succeed");

        assert_eq!(set.zones()[0].metadata().root_note, root);
    }

    #[test]
    fn sample_loader_load_wav_custom_key_range() {
        let tmp = write_test_wav(44100, 1, 64);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(5);
        let lo = NoteNumber::try_new(60).unwrap();
        let hi = NoteNumber::try_new(72).unwrap();
        let opts = WavLoadOptions {
            key_low: Some(lo),
            key_high: Some(hi),
            ..Default::default()
        };
        let set = loader
            .load_wav(tmp.path(), id, opts)
            .expect("load with key range should succeed");

        let range = set.zones()[0].range();
        assert_eq!(range.key_low(), lo);
        assert_eq!(range.key_high(), hi);
    }

    #[test]
    fn sample_loader_load_wav_custom_name() {
        let tmp = write_test_wav(44100, 1, 64);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(6);
        let opts = WavLoadOptions {
            name: Some("MyPiano".to_string()),
            ..Default::default()
        };
        let set = loader
            .load_wav(tmp.path(), id, opts)
            .expect("load with name should succeed");

        assert_eq!(set.name, "MyPiano");
    }

    #[test]
    fn sample_loader_load_wav_missing_file_returns_error() {
        let loader = SampleLoader::new();
        let id = SampleSetId::new(7);
        let result = loader.load_wav("/nonexistent/path/file.wav", id, WavLoadOptions::default());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SampleLoadError::Io(_) | SampleLoadError::WavDecode(_)),
            "unexpected error variant: {err}"
        );
    }

    #[test]
    fn sample_loader_load_auto_wav_succeeds() {
        let tmp = write_test_wav(44100, 1, 64);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(8);

        // Rename temp file to have .wav extension
        let wav_path = tmp.path().with_extension("wav");
        std::fs::copy(tmp.path(), &wav_path).unwrap();

        let set = loader
            .load_auto(&wav_path, id, WavLoadOptions::default())
            .expect("auto-load WAV should succeed");

        assert_eq!(set.zone_count(), 1);

        // Clean up the copied file
        let _ = std::fs::remove_file(&wav_path);
    }

    #[test]
    fn sample_loader_load_auto_sf2_returns_not_supported() {
        let loader = SampleLoader::new();
        let id = SampleSetId::new(9);
        let result = loader.load_auto("/some/file.sf2", id, WavLoadOptions::default());
        assert!(matches!(result, Err(SampleLoadError::Sf2NotSupported)));
    }

    #[test]
    fn sample_loader_load_auto_unknown_extension_returns_error() {
        let loader = SampleLoader::new();
        let id = SampleSetId::new(10);
        let result = loader.load_auto("/some/file.mp4", id, WavLoadOptions::default());
        assert!(matches!(result, Err(SampleLoadError::UnknownFormat(_))));
    }

    #[test]
    fn sample_loader_load_auto_no_extension_returns_error() {
        let loader = SampleLoader::new();
        let id = SampleSetId::new(11);
        let result = loader.load_auto("/some/file_without_ext", id, WavLoadOptions::default());
        assert!(matches!(result, Err(SampleLoadError::UnknownFormat(_))));
    }

    #[test]
    fn sample_loader_process_command_load_wav_succeeds() {
        let tmp = write_test_wav(44100, 1, 32);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(12);
        let cmd = SampleSetCommand::LoadSampleSet {
            path: tmp.path().to_string_lossy().to_string(),
            format: SampleFormat::Wav,
        };
        let result = loader.process_command(&cmd, id);
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
    }

    #[test]
    fn sample_loader_process_command_sf2_returns_not_supported() {
        let loader = SampleLoader::new();
        let id = SampleSetId::new(13);
        let cmd = SampleSetCommand::LoadSampleSet {
            path: "/some/font.sf2".to_string(),
            format: SampleFormat::Sf2,
        };
        let result = loader.process_command(&cmd, id);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            Err(SampleLoadError::Sf2NotSupported)
        ));
    }

    #[test]
    fn sample_loader_process_command_unload_returns_none() {
        let loader = SampleLoader::new();
        let id = SampleSetId::new(14);
        let cmd = SampleSetCommand::UnloadSampleSet {
            id: SampleSetId::new(99),
        };
        let result = loader.process_command(&cmd, id);
        assert!(result.is_none());
    }

    #[test]
    fn sample_loader_error_display_messages_non_empty() {
        let errs = [
            SampleLoadError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "x")),
            SampleLoadError::WavDecode(hound::Error::IoError(std::io::Error::other("y"))),
            SampleLoadError::InvalidChannelCount(0),
            SampleLoadError::InvalidFrameCount,
            SampleLoadError::Sf2NotSupported,
            SampleLoadError::UnknownFormat("xyz".to_string()),
            SampleLoadError::MetadataOutOfRange("too big".to_string()),
        ];
        for err in &errs {
            assert!(!err.to_string().is_empty(), "empty Display for {err:?}");
        }
    }

    #[test]
    fn sample_loader_default_construction() {
        let loader = SampleLoader;
        // Just verifying the default construction path compiles
        let _ = loader;
    }

    #[test]
    fn sample_loader_arc_data_is_shared_across_cloned_zones() {
        let tmp = write_test_wav(44100, 1, 64);
        let loader = SampleLoader::new();
        let id = SampleSetId::new(15);
        let set = loader
            .load_wav(tmp.path(), id, WavLoadOptions::default())
            .unwrap();

        // Clone the set: zones are cloned, but Arc<[f32]> is shared
        let set2 = set.clone();
        assert!(Arc::ptr_eq(
            &set.zones()[0].sample_data_ref(),
            &set2.zones()[0].sample_data_ref()
        ));
    }
}
