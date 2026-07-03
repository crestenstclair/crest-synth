// path: src/plugin/plugin_host.rs

//! Host-agnostic port for a plugin instance's audio/DAW-facing behavior.
//!
//! `PluginHost` is the abstraction a concrete adapter (e.g. an nih-plug
//! `Plugin` implementation) depends on and delegates to. Nothing in this
//! file knows about nih-plug, an audio driver, or a window — the engine
//! library stays host-agnostic and this trait is the only seam a host
//! wrapper crosses to reach it.
//!
//! `process_block` is the real-time audio path: it must never allocate,
//! lock, or block. `AudioBuffer` and `MidiEvents` therefore borrow their
//! backing storage from the caller (the host) rather than owning a `Vec`,
//! so a conforming implementation can process a block without touching
//! the heap. `save_state` / `load_state` run on the control (non-real-time)
//! thread and may allocate freely; a conforming adapter backs them with
//! the same `PresetCodec` the standalone app uses so presets are portable
//! between the plugin and the standalone shell.

/// Stable numeric identifier for a plugin parameter.
///
/// IDs must stay stable across versions: a DAW's saved automation lane
/// references a parameter by this ID, and changing it would silently
/// detach existing automation from the parameter it used to control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterId(u32);

impl ParameterId {
    /// Construct a `ParameterId` from its raw numeric value.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// The raw numeric value, as sent to and from the host.
    pub fn value(self) -> u32 {
        self.0
    }
}

impl From<u32> for ParameterId {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

/// A single raw MIDI event delivered to the plugin for the current block,
/// timestamped by its sample offset within that block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawMidiEvent {
    /// Offset, in samples, from the start of the current block.
    pub sample_offset: u32,
    /// MIDI status byte (includes the channel nibble).
    pub status: u8,
    /// First MIDI data byte.
    pub data1: u8,
    /// Second MIDI data byte.
    pub data2: u8,
}

/// A borrowed batch of MIDI events for the block currently being
/// processed. Borrowing a caller-owned slice, rather than owning a `Vec`,
/// keeps the real-time `process_block` path allocation-free.
#[derive(Debug, Clone, Copy)]
pub struct MidiEvents<'a> {
    events: &'a [RawMidiEvent],
}

impl<'a> MidiEvents<'a> {
    /// Wrap a caller-owned slice of MIDI events for one block.
    pub fn new(events: &'a [RawMidiEvent]) -> Self {
        Self { events }
    }

    /// The events in this block, in ascending sample-offset order.
    pub fn events(&self) -> &'a [RawMidiEvent] {
        self.events
    }
}

/// A borrowed, per-channel audio buffer for one processing block.
///
/// `channels` borrows its sample storage from the host's own buffer, so
/// constructing and passing an `AudioBuffer` through `process_block`
/// never allocates.
pub struct AudioBuffer<'a> {
    channels: &'a mut [&'a mut [f32]],
}

impl<'a> AudioBuffer<'a> {
    /// Wrap host-owned per-channel sample slices for one block.
    pub fn new(channels: &'a mut [&'a mut [f32]]) -> Self {
        Self { channels }
    }

    /// Number of channels in this buffer.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Number of samples in each channel of this buffer.
    pub fn frame_count(&self) -> usize {
        self.channels.first().map_or(0, |channel| channel.len())
    }

    /// Immutable access to one channel's samples.
    pub fn channel(&self, index: usize) -> &[f32] {
        self.channels[index]
    }

    /// Mutable access to one channel's samples.
    pub fn channel_mut(&mut self, index: usize) -> &mut [f32] {
        self.channels[index]
    }
}

/// Failure loading previously saved plugin state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    /// The bytes did not decode as valid saved state.
    Malformed(String),
    /// The state was encoded with a format version this build does not
    /// know how to migrate.
    UnsupportedVersion(u32),
}

/// Host-agnostic port a plugin adapter (e.g. an nih-plug `Plugin` impl)
/// delegates to. This is the only seam between the engine and a DAW
/// host: no audio driver, window, or controller code belongs on either
/// side of it.
///
/// Implementations must uphold:
/// - `process_block` never allocates, locks, or blocks (the real-time
///   audio thread invariant);
/// - parameter IDs are stable across versions (host automation
///   compatibility);
/// - `save_state` / `load_state` round-trip through the same versioned
///   codec the standalone preset system uses, so presets are portable
///   between shells;
/// - a failed `load_state` leaves prior state untouched (no partial
///   restores).
pub trait PluginHost {
    /// Read the current value of a parameter.
    fn get_parameter(&self, id: ParameterId) -> f64;

    /// Set a parameter to a new value. Called from the control thread in
    /// response to host automation or UI edits, never from the audio
    /// callback.
    fn set_parameter(&mut self, id: ParameterId, value: f64);

    /// Process one real-time audio block against a batch of MIDI events,
    /// returning the buffer written with output.
    ///
    /// This is the audio-thread hot path: implementations must not
    /// allocate heap memory, acquire a lock, or perform I/O here.
    fn process_block<'a>(
        &mut self,
        audio: AudioBuffer<'a>,
        midi: MidiEvents<'a>,
    ) -> AudioBuffer<'a>;

    /// Serialize current state (parameters, patch, etc.) for the host to
    /// persist alongside its project. Not real-time: may allocate.
    fn save_state(&self) -> Vec<u8>;

    /// Restore state previously produced by `save_state`. A failed load
    /// must leave the plugin's prior state untouched — no partial
    /// restores.
    fn load_state(&mut self, data: Vec<u8>) -> Result<(), StateError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A minimal in-memory `PluginHost` used only to prove the trait's
    /// contract is exercisable and substitutable (LSP): any conforming
    /// implementation, including this test double, behaves identically
    /// from the caller's point of view.
    struct FakePluginHost {
        params: HashMap<ParameterId, f64>,
        saved: Vec<u8>,
    }

    impl FakePluginHost {
        fn new() -> Self {
            Self {
                params: HashMap::new(),
                saved: Vec::new(),
            }
        }
    }

    impl PluginHost for FakePluginHost {
        fn get_parameter(&self, id: ParameterId) -> f64 {
            *self.params.get(&id).unwrap_or(&0.0)
        }

        fn set_parameter(&mut self, id: ParameterId, value: f64) {
            self.params.insert(id, value);
        }

        fn process_block<'a>(
            &mut self,
            mut audio: AudioBuffer<'a>,
            midi: MidiEvents<'a>,
        ) -> AudioBuffer<'a> {
            // Silence-plus-bump fake: adds the event count to every
            // sample so the test can observe processing happened,
            // without allocating.
            let bump = midi.events().len() as f32;
            for channel_index in 0..audio.channel_count() {
                for sample in audio.channel_mut(channel_index).iter_mut() {
                    *sample += bump;
                }
            }
            audio
        }

        fn save_state(&self) -> Vec<u8> {
            self.saved.clone()
        }

        fn load_state(&mut self, data: Vec<u8>) -> Result<(), StateError> {
            if data.is_empty() {
                return Err(StateError::Malformed("empty state".to_string()));
            }
            self.saved = data;
            Ok(())
        }
    }

    #[test]
    fn parameter_id_round_trips_its_raw_value() {
        let id = ParameterId::new(42);
        assert_eq!(id.value(), 42);
        assert_eq!(ParameterId::from(7), ParameterId::new(7));
    }

    #[test]
    fn get_parameter_defaults_to_zero_until_set() {
        let host = FakePluginHost::new();
        assert_eq!(host.get_parameter(ParameterId::new(1)), 0.0);
    }

    #[test]
    fn set_parameter_then_get_parameter_round_trips() {
        let mut host = FakePluginHost::new();
        let id = ParameterId::new(3);
        host.set_parameter(id, 0.75);
        assert_eq!(host.get_parameter(id), 0.75);
    }

    #[test]
    fn process_block_writes_every_channel_without_allocating_a_new_buffer() {
        let mut host = FakePluginHost::new();
        let mut left = [0.0_f32; 4];
        let mut right = [0.0_f32; 4];
        let mut channels: [&mut [f32]; 2] = [&mut left, &mut right];
        let audio = AudioBuffer::new(&mut channels);
        let events = [RawMidiEvent {
            sample_offset: 0,
            status: 0x90,
            data1: 60,
            data2: 100,
        }];
        let midi = MidiEvents::new(&events);

        let processed = host.process_block(audio, midi);

        assert_eq!(processed.frame_count(), 4);
        assert_eq!(processed.channel(0)[0], 1.0);
        assert_eq!(processed.channel(1)[0], 1.0);
    }

    #[test]
    fn load_state_rejects_empty_payload_and_leaves_prior_state_untouched() {
        let mut host = FakePluginHost::new();
        host.saved = vec![1, 2, 3];

        let result = host.load_state(Vec::new());

        assert!(result.is_err());
        assert_eq!(host.save_state(), vec![1, 2, 3]);
    }

    #[test]
    fn load_state_then_save_state_round_trips_bytes() {
        let mut host = FakePluginHost::new();
        host.load_state(vec![9, 9, 9]).unwrap();
        assert_eq!(host.save_state(), vec![9, 9, 9]);
    }
}
