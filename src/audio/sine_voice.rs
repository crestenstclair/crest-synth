use std::f64::consts::TAU;

use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;

/// An event emitted by [`SineVoice`].
#[derive(Debug, Clone, PartialEq)]
pub enum SineVoiceEvent {
    /// A voice began sounding at the given frequency.
    VoiceStarted { note_id: NoteId, frequency: f64 },
    /// A voice stopped sounding.
    VoiceStopped { note_id: NoteId },
}

/// Error type for [`SineVoice`] command failures.
#[derive(Debug, Clone, PartialEq)]
pub enum SineVoiceError {
    /// `NoteOn` was issued for a `NoteId` that is already active.
    DuplicateNoteId(NoteId),
    /// `NoteOff` was issued for a `NoteId` that is not active.
    UnknownNoteId(NoteId),
}

impl std::fmt::Display for SineVoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SineVoiceError::DuplicateNoteId(id) => {
                write!(f, "NoteOn for already-active note id {:?}", id)
            }
            SineVoiceError::UnknownNoteId(id) => {
                write!(f, "NoteOff for unknown note id {:?}", id)
            }
        }
    }
}

impl std::error::Error for SineVoiceError {}

/// Converts a MIDI note number to a frequency in Hz using equal temperament.
///
/// A4 (note 69) = 440 Hz.
fn midi_note_to_frequency(note: NoteNumber) -> f64 {
    440.0 * 2.0_f64.powf((note.value() as f64 - 69.0) / 12.0)
}

/// The internal state of a single active sine voice.
#[derive(Debug, Clone)]
struct VoiceState {
    note_id: NoteId,
    /// Stored as part of declared aggregate state; used for future per-note expression.
    #[allow(dead_code)]
    note_number: NoteNumber,
    frequency: f64,
    phase: f64,
    active: bool,
}

impl VoiceState {
    fn new(note_id: NoteId, note_number: NoteNumber, frequency: f64) -> Self {
        Self {
            note_id,
            note_number,
            frequency,
            phase: 0.0,
            active: true,
        }
    }

    /// Advance the phase by one sample at the given sample rate and return the sample.
    ///
    /// Phase is kept in [0, 2π) to satisfy the invariant that phase wraps at 2π.
    fn next_sample(&mut self, sample_rate: f64) -> f32 {
        let sample = self.phase.sin() as f32;
        self.phase += TAU * self.frequency / sample_rate;
        // Wrap phase to [0, TAU) — invariant: phase wraps at 2*PI.
        if self.phase >= TAU {
            self.phase -= TAU;
        }
        sample
    }
}

/// Aggregate root: plays a sine wave at a given pitch.
///
/// `SineVoice` manages up to one voice per `NoteId` (invariant: at most one
/// voice per noteId). Frequencies are always positive (invariant: frequency
/// must be positive). Phase is maintained in `[0, 2π)` and wraps at `2π`
/// (invariant: phase wraps at 2*PI).
///
/// # Commands
///
/// - [`SineVoice::note_on`] — start a voice for a note.
/// - [`SineVoice::note_off`] — stop the voice for a note.
///
/// # Audio rendering
///
/// Call [`SineVoice::render_sample`] on each active voice once per audio
/// sample to get the raw sine sample. The voice must still be active
/// (i.e., `note_off` has not been called for it) for this to produce sound.
pub struct SineVoice {
    voices: Vec<VoiceState>,
}

impl Default for SineVoice {
    fn default() -> Self {
        Self::new()
    }
}

impl SineVoice {
    /// Create a new `SineVoice` aggregate with no active voices.
    pub fn new() -> Self {
        Self { voices: Vec::new() }
    }

    /// **Command — NoteOn**: start a sine voice for the given note.
    ///
    /// Returns `VoiceStarted` on success, or `DuplicateNoteId` if a voice for
    /// `note_id` is already active (invariant: at most one voice per noteId).
    pub fn note_on(
        &mut self,
        note_id: NoteId,
        note_number: NoteNumber,
        _velocity: Velocity,
    ) -> Result<SineVoiceEvent, SineVoiceError> {
        // Invariant: at most one voice per noteId.
        if self.voices.iter().any(|v| v.note_id == note_id && v.active) {
            return Err(SineVoiceError::DuplicateNoteId(note_id));
        }

        let frequency = midi_note_to_frequency(note_number);
        // Invariant: frequency must be positive — guaranteed by the formula
        // (2^x is always positive; 440 > 0).
        debug_assert!(
            frequency > 0.0,
            "midi_note_to_frequency produced non-positive frequency"
        );

        let voice = VoiceState::new(note_id, note_number, frequency);
        let event = SineVoiceEvent::VoiceStarted { note_id, frequency };
        self.voices.push(voice);
        Ok(event)
    }

    /// **Command — NoteOff**: stop the sine voice for the given note.
    ///
    /// Returns `VoiceStopped` on success, or `UnknownNoteId` if no active
    /// voice with `note_id` exists.
    pub fn note_off(&mut self, note_id: NoteId) -> Result<SineVoiceEvent, SineVoiceError> {
        let voice = self
            .voices
            .iter_mut()
            .find(|v| v.note_id == note_id && v.active)
            .ok_or(SineVoiceError::UnknownNoteId(note_id))?;

        voice.active = false;
        Ok(SineVoiceEvent::VoiceStopped { note_id })
    }

    /// Render one sample by summing all active voices.
    ///
    /// This method is safe to call on the audio thread: no heap allocation,
    /// no locking, no I/O.
    pub fn render_sample(&mut self, sample_rate: f64) -> f32 {
        let mut sum = 0.0_f32;
        for voice in self.voices.iter_mut().filter(|v| v.active) {
            sum += voice.next_sample(sample_rate);
        }
        sum
    }

    /// Returns `true` if there is at least one active voice.
    pub fn is_active(&self) -> bool {
        self.voices.iter().any(|v| v.active)
    }

    /// Remove inactive voices to keep the internal list compact.
    ///
    /// Call periodically (e.g., after each audio block) to avoid unbounded
    /// growth. This may allocate (it shrinks the `Vec`), so call it on a
    /// non-realtime thread if strict lock-free behaviour is required; in
    /// practice the allocation is tiny and infrequent.
    pub fn gc_voices(&mut self) {
        self.voices.retain(|v| v.active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note_id(v: u32) -> NoteId {
        NoteId::new(v)
    }

    fn make_note_number(v: u8) -> NoteNumber {
        NoteNumber::try_new(v).expect("valid note number")
    }

    fn make_velocity(v: f64) -> Velocity {
        Velocity::try_new(v).expect("valid velocity")
    }

    // ── NoteOn ───────────────────────────────────────────────────────────────────

    #[test]
    fn note_on_emits_voice_started() {
        let mut sv = SineVoice::new();
        let id = make_note_id(1);
        let note = make_note_number(69); // A4 → 440 Hz
        let vel = make_velocity(0.8);

        let event = sv.note_on(id, note, vel).expect("note_on should succeed");
        match event {
            SineVoiceEvent::VoiceStarted { note_id, frequency } => {
                assert_eq!(note_id, id);
                assert!((frequency - 440.0).abs() < 0.01, "A4 should be ~440 Hz");
            }
            _ => panic!("expected VoiceStarted"),
        }
    }

    #[test]
    fn note_on_frequency_is_positive() {
        let mut sv = SineVoice::new();
        // Test a range of note numbers — all should yield positive frequency.
        for n in [0u8, 60, 69, 127] {
            let id = make_note_id(n as u32);
            let note = make_note_number(n);
            let vel = make_velocity(0.5);
            let event = sv.note_on(id, note, vel).expect("note_on should succeed");
            if let SineVoiceEvent::VoiceStarted { frequency, .. } = event {
                assert!(frequency > 0.0, "frequency must be positive for note {n}");
            } else {
                panic!("expected VoiceStarted");
            }
        }
    }

    #[test]
    fn note_on_duplicate_note_id_returns_error() {
        let mut sv = SineVoice::new();
        let id = make_note_id(42);
        let note = make_note_number(60);
        let vel = make_velocity(0.5);

        sv.note_on(id, note, vel)
            .expect("first note_on should succeed");
        let err = sv
            .note_on(id, note, vel)
            .expect_err("duplicate note_on should fail");
        assert_eq!(err, SineVoiceError::DuplicateNoteId(id));
    }

    // ── NoteOff ──────────────────────────────────────────────────────────────────

    #[test]
    fn note_off_emits_voice_stopped() {
        let mut sv = SineVoice::new();
        let id = make_note_id(1);
        let note = make_note_number(69);
        let vel = make_velocity(0.8);

        sv.note_on(id, note, vel).expect("note_on should succeed");
        let event = sv.note_off(id).expect("note_off should succeed");
        assert_eq!(event, SineVoiceEvent::VoiceStopped { note_id: id });
    }

    #[test]
    fn note_off_unknown_note_id_returns_error() {
        let mut sv = SineVoice::new();
        let id = make_note_id(99);
        let err = sv
            .note_off(id)
            .expect_err("note_off on unknown id should fail");
        assert_eq!(err, SineVoiceError::UnknownNoteId(id));
    }

    #[test]
    fn note_off_after_note_off_returns_error() {
        let mut sv = SineVoice::new();
        let id = make_note_id(1);
        let note = make_note_number(60);
        let vel = make_velocity(0.5);

        sv.note_on(id, note, vel).unwrap();
        sv.note_off(id).unwrap();
        let err = sv.note_off(id).expect_err("second note_off should fail");
        assert_eq!(err, SineVoiceError::UnknownNoteId(id));
    }

    // ── Phase wrapping ───────────────────────────────────────────────────────────────

    #[test]
    fn phase_wraps_at_two_pi() {
        let mut sv = SineVoice::new();
        let id = make_note_id(1);
        let note = make_note_number(69); // 440 Hz
        let vel = make_velocity(0.5);

        sv.note_on(id, note, vel).unwrap();

        // Drive many samples through the voice state.
        let sample_rate = 44100.0;
        // Render enough samples for several complete cycles.
        for _ in 0..44100 {
            sv.render_sample(sample_rate);
        }

        // Inspect internal phase via the active voice.
        let voice = sv.voices.iter().find(|v| v.note_id == id).unwrap();
        assert!(
            voice.phase >= 0.0 && voice.phase < TAU,
            "phase must be in [0, 2π) after rendering; got {}",
            voice.phase
        );
    }

    // ── Polyphony / at-most-one-voice-per-noteId ────────────────────────────────────────

    #[test]
    fn multiple_distinct_note_ids_are_allowed() {
        let mut sv = SineVoice::new();
        let vel = make_velocity(0.5);

        for n in 0u32..4 {
            let id = make_note_id(n);
            let note = make_note_number((60 + n as u8).min(127));
            sv.note_on(id, note, vel)
                .expect("each unique NoteId should succeed");
        }

        assert_eq!(sv.voices.len(), 4);
    }

    #[test]
    fn note_on_after_note_off_on_same_id_is_allowed() {
        let mut sv = SineVoice::new();
        let id = make_note_id(1);
        let note = make_note_number(60);
        let vel = make_velocity(0.5);

        sv.note_on(id, note, vel).unwrap();
        sv.note_off(id).unwrap();
        // Re-using a previously released NoteId is allowed.
        sv.note_on(id, note, vel)
            .expect("reuse of released NoteId should succeed");
    }

    // ── Rendering ───────────────────────────────────────────────────────────────────

    #[test]
    fn render_sample_returns_zero_with_no_active_voices() {
        let mut sv = SineVoice::new();
        assert_eq!(sv.render_sample(44100.0), 0.0_f32);
    }

    #[test]
    fn render_sample_returns_nonzero_with_active_voice() {
        let mut sv = SineVoice::new();
        let id = make_note_id(1);
        let note = make_note_number(69);
        let vel = make_velocity(0.5);

        sv.note_on(id, note, vel).unwrap();

        // Advance past the initial zero-crossing (phase=0 gives sin(0)=0).
        sv.render_sample(44100.0); // phase = 0 → sample = 0
        let sample = sv.render_sample(44100.0); // phase advances → non-zero
                                                // At 440 Hz / 44100 sps the second sample's phase ≈ 0.0628 rad → sin ≈ 0.0628.
        assert!(
            sample.abs() > 1e-4,
            "expected non-zero sample, got {sample}"
        );
    }

    #[test]
    fn render_sample_is_zero_after_note_off() {
        let mut sv = SineVoice::new();
        let id = make_note_id(1);
        let note = make_note_number(69);
        let vel = make_velocity(0.5);

        sv.note_on(id, note, vel).unwrap();
        sv.render_sample(44100.0);
        sv.note_off(id).unwrap();
        let sample = sv.render_sample(44100.0);
        assert_eq!(sample, 0.0_f32, "silent after note_off");
    }

    // ── gc_voices ───────────────────────────────────────────────────────────────────

    #[test]
    fn gc_voices_removes_inactive() {
        let mut sv = SineVoice::new();
        let vel = make_velocity(0.5);

        for n in 0u32..4 {
            let id = make_note_id(n);
            let note = make_note_number(60 + n as u8);
            sv.note_on(id, note, vel).unwrap();
        }

        sv.note_off(make_note_id(0)).unwrap();
        sv.note_off(make_note_id(2)).unwrap();
        sv.gc_voices();

        assert_eq!(sv.voices.len(), 2);
        assert!(sv.voices.iter().all(|v| v.active));
    }

    // ── midi_note_to_frequency ───────────────────────────────────────────────────────────

    #[test]
    fn a4_is_440hz() {
        let note = make_note_number(69);
        let freq = midi_note_to_frequency(note);
        assert!((freq - 440.0).abs() < 0.01);
    }

    #[test]
    fn middle_c_is_approx_261hz() {
        let note = make_note_number(60);
        let freq = midi_note_to_frequency(note);
        assert!(
            (freq - 261.63).abs() < 0.5,
            "C4 should be ~261.63 Hz, got {freq}"
        );
    }
}
