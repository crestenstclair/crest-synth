// path: src/sample_library/sample_format.rs

/// Audio sample file format.
///
/// Identifies the encoded format of sample data on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SampleFormat {
    /// Waveform Audio File Format (.wav).
    #[default]
    Wav,

    /// Audio Interchange File Format (.aiff / .aif).
    Aiff,

    /// Free Lossless Audio Codec (.flac).
    Flac,

    /// MPEG-1 Audio Layer III (.mp3).
    Mp3,

    /// Ogg Vorbis (.ogg).
    Ogg,

    /// SoundFont 2 (.sf2) — a multi-sample instrument container format.
    Sf2,
}

impl std::fmt::Display for SampleFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleFormat::Wav => write!(f, "WAV"),
            SampleFormat::Aiff => write!(f, "AIFF"),
            SampleFormat::Flac => write!(f, "FLAC"),
            SampleFormat::Mp3 => write!(f, "MP3"),
            SampleFormat::Ogg => write!(f, "OGG"),
            SampleFormat::Sf2 => write!(f, "SF2"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_wav() {
        assert_eq!(SampleFormat::default(), SampleFormat::Wav);
    }

    #[test]
    fn display_names() {
        assert_eq!(SampleFormat::Wav.to_string(), "WAV");
        assert_eq!(SampleFormat::Aiff.to_string(), "AIFF");
        assert_eq!(SampleFormat::Flac.to_string(), "FLAC");
        assert_eq!(SampleFormat::Mp3.to_string(), "MP3");
        assert_eq!(SampleFormat::Ogg.to_string(), "OGG");
    }

    #[test]
    fn equality_and_copy() {
        let a = SampleFormat::Wav;
        let b = a;
        assert_eq!(a, b);
        assert_ne!(SampleFormat::Wav, SampleFormat::Flac);
    }

    #[test]
    fn hash_in_hashset() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SampleFormat::Wav);
        set.insert(SampleFormat::Wav);
        assert_eq!(set.len(), 1);
        set.insert(SampleFormat::Flac);
        assert_eq!(set.len(), 2);
    }
}
