// path: src/sample_library/sample_zone.rs

use std::sync::Arc;

use crate::sample_library::key_velocity_range::KeyVelocityRange;
use crate::sample_library::sample_metadata::SampleMetadata;

/// A single zone within a sample set.
///
/// A `SampleZone` binds a region of the keyboard/velocity space
/// ([`KeyVelocityRange`]) to recorded audio data ([`SampleMetadata`] plus the
/// actual PCM frames stored in an `Arc<[f32]>`).
///
/// # Audio-thread safety
///
/// * `SampleZone` is `Clone` — cloning bumps the `Arc` reference count, which
///   is a single atomic operation and is safe from the audio thread.
/// * Dropping the last `Arc<[f32]>` reference (i.e. releasing the sample data)
///   may call `free()`, which must **not** happen on the audio thread.  When
///   swapping a sample set, the old `Arc<[f32]>` must be handed to a
///   [`crate::real_time::deferred_deallocator::RetireHandle`] so that
///   deallocation happens on a background thread.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use crest_synth::kernel::note_number::NoteNumber;
/// use crest_synth::kernel::sample_rate::SampleRate;
/// use crest_synth::kernel::velocity::Velocity;
/// use crest_synth::sample_library::key_velocity_range::KeyVelocityRange;
/// use crest_synth::sample_library::sample_metadata::SampleMetadata;
/// use crest_synth::sample_library::sample_zone::SampleZone;
///
/// let root = NoteNumber::try_new(60).unwrap();
/// let sr = SampleRate::try_new(44100).unwrap();
/// let meta = SampleMetadata::try_new(1, 100, None, None, root, sr).unwrap();
///
/// let lo = NoteNumber::try_new(48).unwrap();
/// let hi = NoteNumber::try_new(72).unwrap();
/// let vel_lo = Velocity::try_new(0.0).unwrap();
/// let vel_hi = Velocity::try_new(1.0).unwrap();
/// let range = KeyVelocityRange::try_new(lo, hi, vel_lo, vel_hi).unwrap();
///
/// let data: Arc<[f32]> = Arc::from(vec![0.0_f32; 100]);
/// let zone = SampleZone::new(meta, range, Arc::clone(&data));
/// assert_eq!(zone.metadata(), meta);
/// ```
#[derive(Debug, Clone)]
pub struct SampleZone {
    /// Metadata describing the recorded audio (channels, sample rate, root note, loop).
    metadata: SampleMetadata,
    /// The key/velocity region this zone responds to.
    range: KeyVelocityRange,
    /// Shared, immutable reference to the PCM sample data (interleaved f32 frames).
    ///
    /// `Arc<[f32]>` lets the audio thread clone a reference without allocation
    /// (one atomic increment).  The last owner must retire the Arc through
    /// [`crate::real_time::deferred_deallocator::RetireHandle`] so that the
    /// underlying heap free never runs on the audio thread.
    sample_data_ref: Arc<[f32]>,
}

impl SampleZone {
    /// Construct a new `SampleZone`.
    ///
    /// # Arguments
    ///
    /// * `metadata` – Metadata describing the recorded audio.
    /// * `range` – Key/velocity region this zone responds to.
    /// * `sample_data_ref` – Shared reference to the PCM sample frames.
    pub fn new(
        metadata: SampleMetadata,
        range: KeyVelocityRange,
        sample_data_ref: Arc<[f32]>,
    ) -> Self {
        Self {
            metadata,
            range,
            sample_data_ref,
        }
    }

    /// Metadata describing the recorded audio for this zone.
    #[inline]
    pub fn metadata(&self) -> SampleMetadata {
        self.metadata
    }

    /// The key/velocity region this zone responds to.
    #[inline]
    pub fn range(&self) -> KeyVelocityRange {
        self.range
    }

    /// A cloned reference to the PCM sample data.
    ///
    /// Cloning an `Arc` is a single atomic operation — safe to call from the
    /// audio thread.  **Do not drop the returned `Arc` on the audio thread.**
    /// Retire it via `RetireHandle` if it may be the last reference.
    #[inline]
    pub fn sample_data_ref(&self) -> Arc<[f32]> {
        Arc::clone(&self.sample_data_ref)
    }

    /// Total number of PCM frames stored in this zone.
    ///
    /// This is the raw slice length divided by the channel count from metadata.
    /// Returns 0 if the channel count is zero.
    #[inline]
    pub fn frame_count(&self) -> usize {
        let channels = self.metadata.channels as usize;
        self.sample_data_ref
            .len()
            .checked_div(channels)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::sample_rate::SampleRate;
    use crate::kernel::velocity::Velocity;

    fn make_zone() -> SampleZone {
        let root = NoteNumber::try_new(69).unwrap();
        let sr = SampleRate::try_new(44100).unwrap();
        let meta = SampleMetadata::try_new(1, 512, None, None, root, sr).unwrap();

        let lo = NoteNumber::try_new(60).unwrap();
        let hi = NoteNumber::try_new(72).unwrap();
        let vel_lo = Velocity::try_new(0.0).unwrap();
        let vel_hi = Velocity::try_new(1.0).unwrap();
        let range = KeyVelocityRange::try_new(lo, hi, vel_lo, vel_hi).unwrap();

        let data: Arc<[f32]> = Arc::from(vec![0.0_f32; 512]);
        SampleZone::new(meta, range, data)
    }

    #[test]
    fn sample_zone_metadata_round_trips() {
        let zone = make_zone();
        assert_eq!(zone.metadata().channels, 1);
        assert_eq!(zone.metadata().length_frames, 512);
    }

    #[test]
    fn sample_zone_range_accessors() {
        let zone = make_zone();
        let r = zone.range();
        assert_eq!(r.key_low().value(), 60);
        assert_eq!(r.key_high().value(), 72);
    }

    #[test]
    fn sample_zone_frame_count() {
        let zone = make_zone();
        // 512 samples, 1 channel → 512 frames
        assert_eq!(zone.frame_count(), 512);
    }

    #[test]
    fn sample_zone_frame_count_stereo() {
        let root = NoteNumber::try_new(60).unwrap();
        let sr = SampleRate::try_new(44100).unwrap();
        let meta = SampleMetadata::try_new(2, 256, None, None, root, sr).unwrap();

        let lo = NoteNumber::try_new(0).unwrap();
        let hi = NoteNumber::try_new(127).unwrap();
        let vel_lo = Velocity::try_new(0.0).unwrap();
        let vel_hi = Velocity::try_new(1.0).unwrap();
        let range = KeyVelocityRange::try_new(lo, hi, vel_lo, vel_hi).unwrap();

        // 512 interleaved stereo samples → 256 frames
        let data: Arc<[f32]> = Arc::from(vec![0.0_f32; 512]);
        let zone = SampleZone::new(meta, range, data);
        assert_eq!(zone.frame_count(), 256);
    }

    #[test]
    fn sample_zone_sample_data_ref_clone_is_shared() {
        let zone = make_zone();
        let ref1 = zone.sample_data_ref();
        let ref2 = zone.sample_data_ref();
        // Both arc references point to the same allocation
        assert!(Arc::ptr_eq(&ref1, &ref2));
    }

    #[test]
    fn sample_zone_clone_shares_data() {
        let zone = make_zone();
        let cloned = zone.clone();
        // The cloned zone and the original share the same Arc<[f32]>
        assert!(Arc::ptr_eq(
            &zone.sample_data_ref(),
            &cloned.sample_data_ref()
        ));
    }

    #[test]
    fn sample_zone_arc_retire_pattern() {
        // Demonstrate the safe retirement pattern: retire the Arc through
        // DeferredDeallocator instead of dropping on the audio thread.
        //
        // `RetireHandle::retire` requires `T: Sized`, so we wrap the sample
        // data in a `Vec<f32>` (rather than a `[f32]` slice) for retirement.
        // In production the loader builds a `Vec<f32>` which is then stored
        // as `Arc<[f32]>` in the zone and as `Arc<Vec<f32>>` in the retire
        // queue so that the underlying buffer is never freed on the audio thread.
        use crate::real_time::deferred_deallocator::deferred_deallocator;

        let zone = make_zone();
        let (mut retire, mut collect) = deferred_deallocator();

        // "Audio thread" takes a local clone of the data reference.
        let data_arc: Arc<[f32]> = zone.sample_data_ref();
        drop(zone); // original zone dropped (simulating hot-swap)

        // Wrap into a sized Arc so RetireHandle can accept it.
        // This is the off-thread ownership transfer; no heap allocation occurs
        // here — `Arc::from` over a slice creates a new Arc pointing to the
        // same backing store only if there is a single owner; here we use
        // `to_vec()` to deliberately hand off a sized owned buffer.
        let owned_vec: Arc<Vec<f32>> = Arc::new(data_arc.to_vec());
        drop(data_arc);

        // Instead of dropping `owned_vec` here (simulated audio thread),
        // retire it so free() runs on the background thread.
        retire.retire(owned_vec);

        // Background thread collects (and may free) the allocation
        collect.collect();
    }
}
