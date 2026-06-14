// path: src/sample_library/sample_metadata.rs

use crate::kernel::note_number::NoteNumber;
use crate::kernel::sample_rate::SampleRate;

/// Metadata describing a single audio sample stored in a sample library.
///
/// `SampleMetadata` is a pure value object — it carries no heap-allocated data
/// and performs no I/O. Audio threads may read it freely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleMetadata {
    /// Number of audio channels (1 = mono, 2 = stereo, …).
    pub channels: u8,
    /// Total length of the sample in audio frames.
    pub length_frames: u64,
    /// Optional loop region: start frame index (inclusive).
    pub loop_start: Option<u64>,
    /// Optional loop region: end frame index (inclusive).
    pub loop_end: Option<u64>,
    /// MIDI root note of this sample (used for pitch-shifting).
    pub root_note: NoteNumber,
    /// Native sample rate of the audio data.
    pub sample_rate: SampleRate,
}

/// Error returned when `SampleMetadata` is constructed with invalid values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleMetadataError {
    /// `channels` must be at least 1.
    ZeroChannels,
    /// `length_frames` must be at least 1.
    ZeroLength,
    /// A loop region was partially specified (both endpoints must be `Some` or `None`).
    InconsistentLoop,
    /// `loop_start` must be less than `loop_end`.
    LoopStartNotBeforeEnd,
    /// Loop endpoints must lie within `[0, length_frames)`.
    LoopOutOfBounds,
}

impl std::fmt::Display for SampleMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroChannels => write!(f, "channels must be >= 1"),
            Self::ZeroLength => write!(f, "length_frames must be >= 1"),
            Self::InconsistentLoop => {
                write!(
                    f,
                    "loop_start and loop_end must both be Some or both be None"
                )
            }
            Self::LoopStartNotBeforeEnd => write!(f, "loop_start must be < loop_end"),
            Self::LoopOutOfBounds => {
                write!(f, "loop region must lie within [0, length_frames)")
            }
        }
    }
}

impl std::error::Error for SampleMetadataError {}

impl SampleMetadata {
    /// Construct and validate a `SampleMetadata`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - `channels == 0`
    /// - `length_frames == 0`
    /// - Exactly one of `loop_start`/`loop_end` is `Some`
    /// - `loop_start >= loop_end`
    /// - Either loop endpoint is `>= length_frames`
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::kernel::note_number::NoteNumber;
    /// use crest_synth::kernel::sample_rate::SampleRate;
    /// use crest_synth::sample_library::sample_metadata::SampleMetadata;
    ///
    /// let meta = SampleMetadata::try_new(
    ///     2,
    ///     44100,
    ///     None,
    ///     None,
    ///     NoteNumber::try_new(60).unwrap(),
    ///     SampleRate::try_new(44100).unwrap(),
    /// )
    /// .unwrap();
    /// assert_eq!(meta.channels, 2);
    /// assert_eq!(meta.length_frames, 44100);
    /// ```
    pub fn try_new(
        channels: u8,
        length_frames: u64,
        loop_start: Option<u64>,
        loop_end: Option<u64>,
        root_note: NoteNumber,
        sample_rate: SampleRate,
    ) -> Result<Self, SampleMetadataError> {
        if channels == 0 {
            return Err(SampleMetadataError::ZeroChannels);
        }
        if length_frames == 0 {
            return Err(SampleMetadataError::ZeroLength);
        }
        match (loop_start, loop_end) {
            (Some(start), Some(end)) => {
                if start >= end {
                    return Err(SampleMetadataError::LoopStartNotBeforeEnd);
                }
                if end >= length_frames {
                    return Err(SampleMetadataError::LoopOutOfBounds);
                }
            }
            (None, None) => {}
            _ => return Err(SampleMetadataError::InconsistentLoop),
        }
        Ok(Self {
            channels,
            length_frames,
            loop_start,
            loop_end,
            root_note,
            sample_rate,
        })
    }

    /// Duration in seconds, computed from `length_frames` and `sample_rate`.
    ///
    /// This calculation is stack-only and safe to call from the audio thread.
    #[inline]
    pub fn duration_secs(self) -> f64 {
        self.length_frames as f64 / self.sample_rate.value() as f64
    }

    /// Returns `true` if this sample has a loop region defined.
    #[inline]
    pub fn has_loop(self) -> bool {
        self.loop_start.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::sample_rate::SampleRate;

    fn root() -> NoteNumber {
        NoteNumber::try_new(60).unwrap()
    }

    fn sr() -> SampleRate {
        SampleRate::try_new(44100).unwrap()
    }

    fn make(
        channels: u8,
        length_frames: u64,
        loop_start: Option<u64>,
        loop_end: Option<u64>,
    ) -> Result<SampleMetadata, SampleMetadataError> {
        SampleMetadata::try_new(channels, length_frames, loop_start, loop_end, root(), sr())
    }

    #[test]
    fn sample_metadata_basic_stereo_no_loop() {
        let m = make(2, 88200, None, None).unwrap();
        assert_eq!(m.channels, 2);
        assert_eq!(m.length_frames, 88200);
        assert!(!m.has_loop());
        assert_eq!(m.root_note, root());
        assert_eq!(m.sample_rate, sr());
    }

    #[test]
    fn sample_metadata_mono_no_loop() {
        let m = make(1, 1000, None, None).unwrap();
        assert_eq!(m.channels, 1);
    }

    #[test]
    fn sample_metadata_with_valid_loop() {
        let m = make(1, 1000, Some(100), Some(900)).unwrap();
        assert!(m.has_loop());
        assert_eq!(m.loop_start, Some(100));
        assert_eq!(m.loop_end, Some(900));
    }

    #[test]
    fn sample_metadata_zero_channels_rejected() {
        assert_eq!(
            make(0, 1000, None, None),
            Err(SampleMetadataError::ZeroChannels)
        );
    }

    #[test]
    fn sample_metadata_zero_length_rejected() {
        assert_eq!(make(1, 0, None, None), Err(SampleMetadataError::ZeroLength));
    }

    #[test]
    fn sample_metadata_inconsistent_loop_start_only() {
        assert_eq!(
            make(1, 1000, Some(100), None),
            Err(SampleMetadataError::InconsistentLoop)
        );
    }

    #[test]
    fn sample_metadata_inconsistent_loop_end_only() {
        assert_eq!(
            make(1, 1000, None, Some(900)),
            Err(SampleMetadataError::InconsistentLoop)
        );
    }

    #[test]
    fn sample_metadata_loop_start_equals_end_rejected() {
        assert_eq!(
            make(1, 1000, Some(500), Some(500)),
            Err(SampleMetadataError::LoopStartNotBeforeEnd)
        );
    }

    #[test]
    fn sample_metadata_loop_start_after_end_rejected() {
        assert_eq!(
            make(1, 1000, Some(900), Some(100)),
            Err(SampleMetadataError::LoopStartNotBeforeEnd)
        );
    }

    #[test]
    fn sample_metadata_loop_end_equals_length_rejected() {
        // loop_end must be < length_frames (i.e., a valid frame index)
        assert_eq!(
            make(1, 1000, Some(0), Some(1000)),
            Err(SampleMetadataError::LoopOutOfBounds)
        );
    }

    #[test]
    fn sample_metadata_loop_end_beyond_length_rejected() {
        assert_eq!(
            make(1, 1000, Some(0), Some(9999)),
            Err(SampleMetadataError::LoopOutOfBounds)
        );
    }

    #[test]
    fn sample_metadata_duration_secs() {
        let m = make(1, 44100, None, None).unwrap();
        let dur = m.duration_secs();
        assert!((dur - 1.0_f64).abs() < 1e-9, "expected 1.0 s, got {dur}");
    }

    #[test]
    fn sample_metadata_error_messages_non_empty() {
        assert!(!SampleMetadataError::ZeroChannels.to_string().is_empty());
        assert!(!SampleMetadataError::ZeroLength.to_string().is_empty());
        assert!(!SampleMetadataError::InconsistentLoop.to_string().is_empty());
        assert!(!SampleMetadataError::LoopStartNotBeforeEnd
            .to_string()
            .is_empty());
        assert!(!SampleMetadataError::LoopOutOfBounds.to_string().is_empty());
    }

    #[test]
    fn sample_metadata_copy_semantics() {
        let m = make(2, 1000, None, None).unwrap();
        let m2 = m; // Copy
        assert_eq!(m, m2);
    }
}
