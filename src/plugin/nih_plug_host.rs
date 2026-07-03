// path: src/plugin/nih_plug_host.rs

//! `NihPlugHost` — the concrete adapter for `port.Plugin.PluginHost`.
//!
//! `NihPlugHost` is the seam a future nih-plug `Plugin` wrapper (living in
//! its own DAW-facing crate, outside this host-agnostic library) delegates
//! every audio/DAW call to. Nothing in this file touches an audio driver,
//! a window, or nih-plug's own `Plugin`/`Params` machinery — the library
//! stays host-agnostic and this adapter is the only piece a real nih-plug
//! wrapper would reach into.
//!
//! Parameter storage is a lock-free table (`AtomicParameterCell` per
//! `ParameterId`, backed by `AtomicU64` holding an `f64` bit pattern) so
//! `get_parameter`/`set_parameter` never take a lock, and `process_block`
//! reads the same cells with a plain atomic load — no mutex, no
//! allocation, no blocking I/O ever happens on that path. Synthesis is a
//! small fixed-size bank of sine voices driven by the raw MIDI events for
//! the block; the voice table lives inline in `NihPlugHost` (no `Vec`,
//! no heap) so triggering and releasing notes during `process_block`
//! never allocates.
//!
//! State save/load is an explicitly versioned byte format so a future
//! format revision can migrate an older payload on load rather than
//! silently misread it, and a failed load leaves the adapter's prior
//! parameter table completely untouched (the new table is built in full
//! before it ever replaces `self.parameters`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::plugin::plugin_host::{
    AudioBuffer, MidiEvents, ParameterId, PluginHost, RawMidiEvent, StateError,
};

/// Fixed voice-bank size for the adapter's built-in synthesis. A fixed
/// array (rather than a `Vec`) keeps note-on/note-off handling inside
/// `process_block` allocation-free.
const MAX_VOICES: usize = 16;

/// Sample rate used when a `NihPlugHost` is constructed without an
/// explicit rate (e.g. via `Default`). A real host wrapper overrides this
/// with `NihPlugHost::new` once it knows the DAW's configured rate.
const DEFAULT_SAMPLE_RATE_HZ: f32 = 44_100.0;

/// Explicit format version for `NihPlugHost` saved state. Future
/// revisions add a migration branch in `decode_state` rather than
/// replacing this constant's meaning.
const STATE_FORMAT_VERSION: u32 = 1;

/// Stable parameter ID for the adapter's built-in master gain, applied to
/// the synthesized signal before it is written to every output channel.
const MASTER_GAIN_PARAMETER_VALUE: u32 = 0;

/// Byte width of one encoded parameter entry: a 4-byte `ParameterId`
/// value followed by an 8-byte little-endian `f64`.
const ENCODED_PARAMETER_LEN: usize = 12;

/// Byte width of the state header: a 4-byte version followed by a 4-byte
/// parameter count.
const STATE_HEADER_LEN: usize = 8;

/// Real-time-safe storage for one parameter's current value.
///
/// Backed by an `AtomicU64` holding the bit pattern of an `f64`, so the
/// control thread (host automation, UI edits) can publish a new value and
/// the audio thread can read the latest one with a plain atomic load —
/// never a mutex, never a block. This is the parameter-bridge seam
/// applied to the plugin adapter: every parameter change crosses the
/// thread boundary through this lock-free cell alone.
struct AtomicParameterCell {
    bits: AtomicU64,
}

impl AtomicParameterCell {
    fn new(value: f64) -> Self {
        Self {
            bits: AtomicU64::new(value.to_bits()),
        }
    }

    fn load(&self) -> f64 {
        f64::from_bits(self.bits.load(Ordering::Acquire))
    }

    fn store(&self, value: f64) {
        self.bits.store(value.to_bits(), Ordering::Release);
    }
}

/// One slot in the fixed-size voice bank. `note` is `None` when the slot
/// is idle; `phase` tracks the oscillator's position in `[0, 1)`.
#[derive(Debug, Clone, Copy)]
struct Voice {
    note: Option<u8>,
    phase: f32,
}

impl Voice {
    const fn silent() -> Self {
        Self {
            note: None,
            phase: 0.0,
        }
    }
}

/// Convert a MIDI note number to a frequency in Hz, using equal
/// temperament with A4 (note 69) at 440 Hz.
fn note_number_to_frequency_hz(note: u8) -> f32 {
    440.0 * 2f32.powf((f32::from(note) - 69.0) / 12.0)
}

/// The nih-plug-facing adapter for `port.Plugin.PluginHost`.
///
/// Owns a lock-free parameter table keyed by stable `ParameterId`s and a
/// fixed-size bank of sine voices driven by incoming MIDI. No audio
/// driver, window, or controller code lives here — those belong to the
/// standalone shell, not this library.
pub struct NihPlugHost {
    parameters: HashMap<ParameterId, AtomicParameterCell>,
    voices: [Voice; MAX_VOICES],
    sample_rate_hz: f32,
}

impl NihPlugHost {
    /// Full constructor: build a `NihPlugHost` for a host running at
    /// `sample_rate_hz`. The master-gain parameter is seeded to unity
    /// gain so a host that never calls `set_parameter` still hears the
    /// synthesized signal at full volume.
    pub fn new(sample_rate_hz: f32) -> Self {
        let mut parameters = HashMap::new();
        parameters.insert(
            Self::master_gain_parameter_id(),
            AtomicParameterCell::new(1.0),
        );
        Self {
            parameters,
            voices: [Voice::silent(); MAX_VOICES],
            sample_rate_hz,
        }
    }

    /// The stable `ParameterId` for the adapter's built-in master gain.
    pub fn master_gain_parameter_id() -> ParameterId {
        ParameterId::new(MASTER_GAIN_PARAMETER_VALUE)
    }

    fn parameter_value_or(&self, id: ParameterId, default: f64) -> f64 {
        self.parameters
            .get(&id)
            .map_or(default, AtomicParameterCell::load)
    }

    /// Apply one raw MIDI event to the voice bank: note-on claims the
    /// first idle voice (dropping the note if the bank is full — no
    /// stealing policy is wired here), note-on with zero velocity and
    /// note-off both release the matching voice.
    fn apply_midi_event(&mut self, event: RawMidiEvent) {
        let status_nibble = event.status & 0xF0;
        match status_nibble {
            0x90 if event.data2 > 0 => self.note_on(event.data1),
            0x90 | 0x80 => self.note_off(event.data1),
            _ => {}
        }
    }

    fn note_on(&mut self, note: u8) {
        if let Some(voice) = self.voices.iter_mut().find(|voice| voice.note.is_none()) {
            voice.note = Some(note);
            voice.phase = 0.0;
        }
    }

    fn note_off(&mut self, note: u8) {
        if let Some(voice) = self
            .voices
            .iter_mut()
            .find(|voice| voice.note == Some(note))
        {
            voice.note = None;
        }
    }

    /// Advance every active voice by one sample and return the mixed,
    /// averaged output for that sample. Allocation-free: only the fixed
    /// `voices` array is touched.
    fn advance_and_mix_one_sample(&mut self) -> f32 {
        let mut mixed = 0.0_f32;
        let mut active_voice_count = 0_u32;

        for voice in self.voices.iter_mut() {
            if let Some(note) = voice.note {
                mixed += (voice.phase * std::f32::consts::TAU).sin();
                let phase_increment = note_number_to_frequency_hz(note) / self.sample_rate_hz;
                voice.phase += phase_increment;
                if voice.phase >= 1.0 {
                    voice.phase -= 1.0;
                }
                active_voice_count += 1;
            }
        }

        if active_voice_count == 0 {
            0.0
        } else {
            mixed / active_voice_count as f32
        }
    }

    /// Encode current parameters into the versioned state format.
    fn encode_state(&self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity(STATE_HEADER_LEN + self.parameters.len() * ENCODED_PARAMETER_LEN);
        bytes.extend_from_slice(&STATE_FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&(self.parameters.len() as u32).to_le_bytes());
        for (id, cell) in &self.parameters {
            bytes.extend_from_slice(&id.value().to_le_bytes());
            bytes.extend_from_slice(&cell.load().to_le_bytes());
        }
        bytes
    }

    /// Decode a versioned state payload into a list of parameter
    /// values, without mutating `self`. Keeping decode side-effect-free
    /// is what lets `load_state` build the replacement table fully
    /// before ever touching prior state, so a failed decode leaves the
    /// adapter untouched.
    fn decode_state(data: &[u8]) -> Result<Vec<(ParameterId, f64)>, StateError> {
        if data.len() < STATE_HEADER_LEN {
            return Err(StateError::Malformed(format!(
                "state payload of {} bytes is shorter than the {}-byte header",
                data.len(),
                STATE_HEADER_LEN
            )));
        }

        let version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if version != STATE_FORMAT_VERSION {
            return Err(StateError::UnsupportedVersion(version));
        }

        let parameter_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let expected_len = STATE_HEADER_LEN + parameter_count * ENCODED_PARAMETER_LEN;
        if data.len() != expected_len {
            return Err(StateError::Malformed(format!(
                "expected {expected_len} bytes for {parameter_count} parameters, found {}",
                data.len()
            )));
        }

        let mut parameters = Vec::with_capacity(parameter_count);
        let mut offset = STATE_HEADER_LEN;
        for _ in 0..parameter_count {
            let id_bytes: [u8; 4] = match data[offset..offset + 4].try_into() {
                Ok(bytes) => bytes,
                Err(_) => return Err(StateError::Malformed("truncated parameter id".to_string())),
            };
            let value_bytes: [u8; 8] =
                match data[offset + 4..offset + ENCODED_PARAMETER_LEN].try_into() {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        return Err(StateError::Malformed(
                            "truncated parameter value".to_string(),
                        ))
                    }
                };
            parameters.push((
                ParameterId::new(u32::from_le_bytes(id_bytes)),
                f64::from_le_bytes(value_bytes),
            ));
            offset += ENCODED_PARAMETER_LEN;
        }

        Ok(parameters)
    }
}

impl Default for NihPlugHost {
    /// Convenience constructor for callers who don't yet know the host's
    /// sample rate; a real host wrapper should use `new` once it does.
    fn default() -> Self {
        Self::new(DEFAULT_SAMPLE_RATE_HZ)
    }
}

impl PluginHost for NihPlugHost {
    fn get_parameter(&self, id: ParameterId) -> f64 {
        self.parameter_value_or(id, 0.0)
    }

    fn set_parameter(&mut self, id: ParameterId, value: f64) {
        match self.parameters.get(&id) {
            Some(cell) => cell.store(value),
            None => {
                self.parameters.insert(id, AtomicParameterCell::new(value));
            }
        }
    }

    fn process_block<'a>(
        &mut self,
        mut audio: AudioBuffer<'a>,
        midi: MidiEvents<'a>,
    ) -> AudioBuffer<'a> {
        let master_gain = self.parameter_value_or(Self::master_gain_parameter_id(), 1.0) as f32;
        let events = midi.events();
        let mut next_event_index = 0_usize;
        let frame_count = audio.frame_count();

        for frame in 0..frame_count {
            while next_event_index < events.len()
                && events[next_event_index].sample_offset as usize == frame
            {
                self.apply_midi_event(events[next_event_index]);
                next_event_index += 1;
            }

            let sample = self.advance_and_mix_one_sample() * master_gain;
            for channel_index in 0..audio.channel_count() {
                audio.channel_mut(channel_index)[frame] = sample;
            }
        }

        audio
    }

    fn save_state(&self) -> Vec<u8> {
        self.encode_state()
    }

    fn load_state(&mut self, data: Vec<u8>) -> Result<(), StateError> {
        let restored = Self::decode_state(&data)?;

        // Build the replacement table fully before touching `self` so a
        // failed decode above never leaves the adapter half-restored.
        let mut parameters = HashMap::with_capacity(restored.len());
        for (id, value) in restored {
            parameters.insert(id, AtomicParameterCell::new(value));
        }
        self.parameters = parameters;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_on(sample_offset: u32, note: u8, velocity: u8) -> RawMidiEvent {
        RawMidiEvent {
            sample_offset,
            status: 0x90,
            data1: note,
            data2: velocity,
        }
    }

    fn note_off(sample_offset: u32, note: u8) -> RawMidiEvent {
        RawMidiEvent {
            sample_offset,
            status: 0x80,
            data1: note,
            data2: 0,
        }
    }

    fn silent_buffer(frame_count: usize) -> Vec<f32> {
        vec![0.0; frame_count]
    }

    #[test]
    fn get_parameter_defaults_to_zero_for_an_unset_id() {
        let host = NihPlugHost::default();
        assert_eq!(host.get_parameter(ParameterId::new(99)), 0.0);
    }

    #[test]
    fn master_gain_defaults_to_unity() {
        let host = NihPlugHost::default();
        assert_eq!(
            host.get_parameter(NihPlugHost::master_gain_parameter_id()),
            1.0
        );
    }

    #[test]
    fn set_parameter_then_get_parameter_round_trips() {
        let mut host = NihPlugHost::default();
        let id = ParameterId::new(5);
        host.set_parameter(id, 0.42);
        assert_eq!(host.get_parameter(id), 0.42);
    }

    #[test]
    fn process_block_is_silent_with_no_midi_events() {
        let mut host = NihPlugHost::default();
        let mut left = silent_buffer(8);
        let mut channels: [&mut [f32]; 1] = [&mut left];
        let audio = AudioBuffer::new(&mut channels);
        let midi = MidiEvents::new(&[]);

        let processed = host.process_block(audio, midi);

        assert!(processed.channel(0).iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn process_block_produces_nonzero_signal_after_a_note_on() {
        let mut host = NihPlugHost::default();
        let mut left = silent_buffer(64);
        let mut channels: [&mut [f32]; 1] = [&mut left];
        let audio = AudioBuffer::new(&mut channels);
        let events = [note_on(0, 69, 100)];
        let midi = MidiEvents::new(&events);

        let processed = host.process_block(audio, midi);

        let peak = processed
            .channel(0)
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        assert!(
            peak > 0.0,
            "expected nonzero output after a note-on, got peak {peak}"
        );
    }

    #[test]
    fn process_block_writes_every_channel_identically() {
        let mut host = NihPlugHost::default();
        let mut left = silent_buffer(16);
        let mut right = silent_buffer(16);
        let mut channels: [&mut [f32]; 2] = [&mut left, &mut right];
        let audio = AudioBuffer::new(&mut channels);
        let events = [note_on(0, 60, 100)];
        let midi = MidiEvents::new(&events);

        let processed = host.process_block(audio, midi);

        assert_eq!(processed.channel(0), processed.channel(1));
    }

    #[test]
    fn note_off_silences_the_voice_in_a_later_block() {
        let mut host = NihPlugHost::default();

        let mut warm_up = silent_buffer(8);
        let mut warm_up_channels: [&mut [f32]; 1] = [&mut warm_up];
        let warm_up_audio = AudioBuffer::new(&mut warm_up_channels);
        let warm_up_events = [note_on(0, 60, 100)];
        host.process_block(warm_up_audio, MidiEvents::new(&warm_up_events));

        let mut released = silent_buffer(8);
        let mut released_channels: [&mut [f32]; 1] = [&mut released];
        let released_audio = AudioBuffer::new(&mut released_channels);
        let released_events = [note_off(0, 60)];
        let processed = host.process_block(released_audio, MidiEvents::new(&released_events));

        assert!(processed.channel(0).iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn note_on_beyond_voice_capacity_is_dropped_without_panicking() {
        let mut host = NihPlugHost::default();
        let mut buffer = silent_buffer(1);
        let mut channels: [&mut [f32]; 1] = [&mut buffer];
        let audio = AudioBuffer::new(&mut channels);

        let events: Vec<RawMidiEvent> = (0..(MAX_VOICES as u8 + 4))
            .map(|note| note_on(0, note, 100))
            .collect();
        let midi = MidiEvents::new(&events);

        host.process_block(audio, midi);
    }

    #[test]
    fn save_state_then_load_state_round_trips_a_parameter_value() {
        let mut host = NihPlugHost::default();
        let id = ParameterId::new(7);
        host.set_parameter(id, 0.66);

        let saved = host.save_state();

        let mut restored = NihPlugHost::default();
        restored.load_state(saved).unwrap();

        assert_eq!(restored.get_parameter(id), 0.66);
    }

    #[test]
    fn load_state_rejects_a_payload_shorter_than_the_header() {
        let mut host = NihPlugHost::default();
        let result = host.load_state(vec![1, 2, 3]);
        assert!(matches!(result, Err(StateError::Malformed(_))));
    }

    #[test]
    fn load_state_rejects_an_unsupported_version_and_leaves_prior_state_untouched() {
        let mut host = NihPlugHost::default();
        let id = ParameterId::new(3);
        host.set_parameter(id, 0.9);

        let mut future_payload = Vec::new();
        future_payload.extend_from_slice(&99_u32.to_le_bytes());
        future_payload.extend_from_slice(&0_u32.to_le_bytes());

        let result = host.load_state(future_payload);

        assert!(matches!(result, Err(StateError::UnsupportedVersion(99))));
        assert_eq!(host.get_parameter(id), 0.9);
    }

    #[test]
    fn load_state_rejects_a_length_mismatch_and_leaves_prior_state_untouched() {
        let mut host = NihPlugHost::default();
        let id = ParameterId::new(4);
        host.set_parameter(id, 0.25);

        let mut malformed_payload = Vec::new();
        malformed_payload.extend_from_slice(&STATE_FORMAT_VERSION.to_le_bytes());
        malformed_payload.extend_from_slice(&1_u32.to_le_bytes());
        // Declares one parameter but supplies zero bytes for it.

        let result = host.load_state(malformed_payload);

        assert!(matches!(result, Err(StateError::Malformed(_))));
        assert_eq!(host.get_parameter(id), 0.25);
    }
}
