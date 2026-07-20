use crate::real_time::audio_observation::{
    AudioObservation, CallbackAudioObservation, ControlAudioObservation,
};
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// Prepared coherent latest-value storage for callback audio observations.
pub struct AtomicAudioObservation {
    shared: Arc<AtomicObservationFields>,
}

impl AtomicAudioObservation {
    #[must_use]
    pub fn new(initial: AudioObservationSnapshot) -> Self {
        Self {
            shared: Arc::new(AtomicObservationFields::new(initial)),
        }
    }
}

impl Default for AtomicAudioObservation {
    fn default() -> Self {
        Self::new(AudioObservationSnapshot::default())
    }
}

impl AudioObservation for AtomicAudioObservation {
    type CallbackHandle = AtomicAudioObservationWriter;
    type ControlHandle = AtomicAudioObservationReader;

    fn into_handles(self) -> (Self::CallbackHandle, Self::ControlHandle) {
        (
            AtomicAudioObservationWriter {
                shared: Arc::clone(&self.shared),
            },
            AtomicAudioObservationReader {
                shared: self.shared,
            },
        )
    }
}

/// Narrow callback-side publisher.
pub struct AtomicAudioObservationWriter {
    shared: Arc<AtomicObservationFields>,
}

impl CallbackAudioObservation for AtomicAudioObservationWriter {
    fn publish_from_callback(&mut self, snapshot: AudioObservationSnapshot) {
        self.shared.publish(snapshot);
    }
}

/// Narrow control-side latest-value reader.
#[derive(Clone)]
pub struct AtomicAudioObservationReader {
    shared: Arc<AtomicObservationFields>,
}

impl ControlAudioObservation for AtomicAudioObservationReader {
    fn read_latest_on_control(&self) -> AudioObservationSnapshot {
        self.shared.read()
    }
}

struct AtomicObservationFields {
    version: AtomicU64,
    sequence: AtomicU64,
    rendered_blocks: AtomicU64,
    rendered_frames: AtomicU64,
    parameter_generation: AtomicU64,
    commands_consumed: AtomicU64,
    active_notes: AtomicU32,
    left_peak: AtomicU32,
    right_peak: AtomicU32,
    output_rms: AtomicU32,
    reverb_input_rms: AtomicU32,
    delay_input_rms: AtomicU32,
    wet_output_rms: AtomicU32,
    non_finite_samples: AtomicU64,
    clipped_samples: AtomicU64,
}

impl AtomicObservationFields {
    fn new(initial: AudioObservationSnapshot) -> Self {
        Self {
            version: AtomicU64::new(0),
            sequence: AtomicU64::new(initial.sequence()),
            rendered_blocks: AtomicU64::new(initial.rendered_blocks()),
            rendered_frames: AtomicU64::new(initial.rendered_frames()),
            parameter_generation: AtomicU64::new(initial.parameter_generation()),
            commands_consumed: AtomicU64::new(initial.commands_consumed()),
            active_notes: AtomicU32::new(initial.active_notes()),
            left_peak: AtomicU32::new(initial.left_peak().to_bits()),
            right_peak: AtomicU32::new(initial.right_peak().to_bits()),
            output_rms: AtomicU32::new(initial.output_rms().to_bits()),
            reverb_input_rms: AtomicU32::new(initial.reverb_input_rms().to_bits()),
            delay_input_rms: AtomicU32::new(initial.delay_input_rms().to_bits()),
            wet_output_rms: AtomicU32::new(initial.wet_output_rms().to_bits()),
            non_finite_samples: AtomicU64::new(initial.non_finite_samples()),
            clipped_samples: AtomicU64::new(initial.clipped_samples()),
        }
    }

    fn publish(&self, snapshot: AudioObservationSnapshot) {
        self.version.fetch_add(1, Ordering::AcqRel);
        self.sequence.store(snapshot.sequence(), Ordering::Relaxed);
        self.rendered_blocks
            .store(snapshot.rendered_blocks(), Ordering::Relaxed);
        self.rendered_frames
            .store(snapshot.rendered_frames(), Ordering::Relaxed);
        self.parameter_generation
            .store(snapshot.parameter_generation(), Ordering::Relaxed);
        self.commands_consumed
            .store(snapshot.commands_consumed(), Ordering::Relaxed);
        self.active_notes
            .store(snapshot.active_notes(), Ordering::Relaxed);
        self.left_peak
            .store(snapshot.left_peak().to_bits(), Ordering::Relaxed);
        self.right_peak
            .store(snapshot.right_peak().to_bits(), Ordering::Relaxed);
        self.output_rms
            .store(snapshot.output_rms().to_bits(), Ordering::Relaxed);
        self.reverb_input_rms
            .store(snapshot.reverb_input_rms().to_bits(), Ordering::Relaxed);
        self.delay_input_rms
            .store(snapshot.delay_input_rms().to_bits(), Ordering::Relaxed);
        self.wet_output_rms
            .store(snapshot.wet_output_rms().to_bits(), Ordering::Relaxed);
        self.non_finite_samples
            .store(snapshot.non_finite_samples(), Ordering::Relaxed);
        self.clipped_samples
            .store(snapshot.clipped_samples(), Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Release);
    }

    fn read(&self) -> AudioObservationSnapshot {
        loop {
            let before = self.version.load(Ordering::Acquire);
            if before & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }

            let snapshot = AudioObservationSnapshot::from_parts(
                self.sequence.load(Ordering::Relaxed),
                self.rendered_blocks.load(Ordering::Relaxed),
                self.rendered_frames.load(Ordering::Relaxed),
                self.parameter_generation.load(Ordering::Relaxed),
                self.commands_consumed.load(Ordering::Relaxed),
                self.active_notes.load(Ordering::Relaxed),
                f32::from_bits(self.left_peak.load(Ordering::Relaxed)),
                f32::from_bits(self.right_peak.load(Ordering::Relaxed)),
                f32::from_bits(self.output_rms.load(Ordering::Relaxed)),
                f32::from_bits(self.reverb_input_rms.load(Ordering::Relaxed)),
                f32::from_bits(self.delay_input_rms.load(Ordering::Relaxed)),
                f32::from_bits(self.wet_output_rms.load(Ordering::Relaxed)),
                self.non_finite_samples.load(Ordering::Relaxed),
                self.clipped_samples.load(Ordering::Relaxed),
            );
            let after = self.version.load(Ordering::Acquire);
            if before == after {
                return snapshot;
            }
            core::hint::spin_loop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AtomicAudioObservation;
    use crate::mixer::mix_observation::MixObservation;
    use crate::real_time::audio_observation::{
        AudioObservation, CallbackAudioObservation, ControlAudioObservation,
    };
    use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;

    fn snapshot(sequence: u64) -> AudioObservationSnapshot {
        let value = sequence as f32;
        AudioObservationSnapshot::from_mix(
            sequence,
            sequence,
            sequence * 64,
            sequence,
            sequence,
            sequence as u32,
            MixObservation::new(value, value, value, value, value, value, sequence, sequence),
        )
    }

    #[test]
    fn atomic_audio_observation_is_latest_wins_and_coherent() {
        let observation = AtomicAudioObservation::default();
        let (mut writer, reader) = observation.into_handles();

        writer.publish_from_callback(snapshot(1));
        writer.publish_from_callback(snapshot(2));

        let latest = reader.read_latest_on_control();
        assert_eq!(latest, snapshot(2));
        assert_eq!(latest.output_rms().to_bits(), 2.0_f32.to_bits());
    }

    #[test]
    fn atomic_audio_observation_never_tears_concurrent_snapshots() {
        let observation = AtomicAudioObservation::default();
        let (mut writer, reader) = observation.into_handles();
        let writer_thread = std::thread::spawn(move || {
            for sequence in 1..=20_000 {
                writer.publish_from_callback(snapshot(sequence));
            }
        });

        while reader.read_latest_on_control().sequence() < 20_000 {
            let latest = reader.read_latest_on_control();
            let sequence = latest.sequence();
            assert_eq!(latest.rendered_blocks(), sequence);
            assert_eq!(latest.parameter_generation(), sequence);
            assert_eq!(latest.output_rms().to_bits(), (sequence as f32).to_bits());
            assert_eq!(latest.non_finite_samples(), sequence);
        }
        writer_thread.join().expect("publisher thread completes");
    }

    #[test]
    fn concrete_handles_are_send_and_keep_narrow_capabilities() {
        fn assert_send<T: Send>() {}

        assert_send::<super::AtomicAudioObservationWriter>();
        assert_send::<super::AtomicAudioObservationReader>();
    }
}
