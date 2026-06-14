// path: src/synth/voice.rs

//! Single-voice aggregate: one sounding note with oscillator, filter, and amp envelope.
//!
//! # Invariants
//!
//! 1. Frequency is always derived from the active `NoteNumber` via equal-temperament
//!    (A4 = 440 Hz). Any pitch modulation is expressed as an additive semitone offset
//!    supplied at render time — the base frequency stored in the voice is never changed
//!    outside of a `NoteOn` command.
//! 2. The envelope stage progresses strictly `Idle → Attack → Decay → Sustain →
//!    Release → Idle`. No stage can be skipped.
//! 3. A voice is reclaimable (stealable) only when `envelope_stage` is `Idle`.

use std::f64::consts::TAU;

use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;
use crate::synth::amp_envelope_config::AmpEnvelopeConfig;
use crate::synth::envelope_stage::EnvelopeStage;

// ─────────────────────────────────────────────────────────────────────────────
// Supporting value types
// ─────────────────────────────────────────────────────────────────────────────

/// Amplitude in the range [0.0, 1.0].
///
/// Used for per-voice envelope levels. NaN and out-of-range values are
/// rejected at construction time.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Amplitude(f64);

/// Error returned when an [`Amplitude`] value is out of range or NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmplitudeError(f64);

impl std::fmt::Display for AmplitudeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Amplitude {} is out of range 0.0-1.0", self.0)
    }
}

impl std::error::Error for AmplitudeError {}

impl Amplitude {
    /// Construct an [`Amplitude`] from a raw `f64`.
    ///
    /// Returns `Err` if the value is NaN or not in `[0.0, 1.0]`.
    ///
    /// ```
    /// use crest_synth::synth::voice::Amplitude;
    /// assert!(Amplitude::try_new(0.0).is_ok());
    /// assert!(Amplitude::try_new(1.0).is_ok());
    /// assert!(Amplitude::try_new(1.1).is_err());
    /// ```
    pub fn try_new(value: f64) -> Result<Self, AmplitudeError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(AmplitudeError(value));
        }
        Ok(Self(value))
    }

    /// Return the underlying value.
    #[inline]
    pub fn value(self) -> f64 {
        self.0
    }

    /// Silence (0.0).
    pub fn zero() -> Self {
        Self(0.0)
    }
}

impl Default for Amplitude {
    fn default() -> Self {
        Self(0.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Audio frequency in Hz (must be positive and finite).
///
/// Defined locally for the voice aggregate because MIDI note 0 (≈ 8.18 Hz) is
/// below the audible 20–20 000 Hz range used by `filter_config::Frequency`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Frequency(f64);

/// Error returned when a [`Frequency`] is non-positive or non-finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyError(f64);

impl std::fmt::Display for FrequencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Frequency {} Hz must be positive and finite", self.0)
    }
}

impl std::error::Error for FrequencyError {}

impl Frequency {
    /// Construct a [`Frequency`] from a raw `f64` (Hz).
    ///
    /// Returns `Err` if the value is not positive and finite.
    ///
    /// ```
    /// use crest_synth::synth::voice::Frequency;
    /// assert!(Frequency::try_new(440.0).is_ok());
    /// assert!(Frequency::try_new(8.0).is_ok());
    /// assert!(Frequency::try_new(0.0).is_err());
    /// assert!(Frequency::try_new(-1.0).is_err());
    /// ```
    pub fn try_new(hz: f64) -> Result<Self, FrequencyError> {
        if !hz.is_finite() || hz <= 0.0 {
            return Err(FrequencyError(hz));
        }
        Ok(Self(hz))
    }

    /// Return the frequency in Hz.
    #[inline]
    pub fn hz(self) -> f64 {
        self.0
    }
}

impl Default for Frequency {
    /// A4 = 440 Hz.
    fn default() -> Self {
        Self(440.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Runtime state for a one-pole IIR low-pass filter.
///
/// Stores only the previous output sample; parameters (cutoff coefficient)
/// are supplied per-sample by the caller. No heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FilterState {
    /// Most recent filter output (initialised to 0.0).
    pub z1: f64,
}

impl FilterState {
    /// Create a zeroed [`FilterState`].
    pub fn new() -> Self {
        Self { z1: 0.0 }
    }

    /// Process one input sample through a one-pole low-pass filter.
    ///
    /// `cutoff_coeff` ∈ (0.0, 1.0]: 1.0 = no filtering, smaller = more filtering.
    #[inline]
    pub fn process(&mut self, input: f64, cutoff_coeff: f64) -> f64 {
        let output = cutoff_coeff * input + (1.0 - cutoff_coeff) * self.z1;
        self.z1 = output;
        output
    }

    /// Reset the filter state to zero.
    pub fn reset(&mut self) {
        self.z1 = 0.0;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Commands
// ─────────────────────────────────────────────────────────────────────────────

/// Command: start sounding a note.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteOn {
    pub note_id: NoteId,
    pub note_number: NoteNumber,
    pub velocity: Velocity,
}

/// Command: stop sounding a note (begins release phase).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteOff {
    pub note_id: NoteId,
}

// ─────────────────────────────────────────────────────────────────────────────
// Events
// ─────────────────────────────────────────────────────────────────────────────

/// Domain events emitted by [`Voice`] in response to commands.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceEvent {
    /// The voice began sounding.
    VoiceActivated {
        note_id: NoteId,
        note_number: NoteNumber,
        frequency: Frequency,
    },
    /// The voice entered the Release stage (key released, still sounding).
    VoiceReleased { note_id: NoteId },
    /// The envelope reached `Idle` — voice is fully silent and reclaimable.
    VoiceFinished { note_id: NoteId },
    /// The voice was stolen to play a new note while still active.
    VoiceStolen {
        old_note_id: NoteId,
        new_note_id: NoteId,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors returned by [`Voice`] command handlers.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceError {
    /// `NoteOff` was received for a `NoteId` different from the active one.
    WrongNoteId { expected: NoteId, received: NoteId },
    /// `NoteOff` was received but the voice is already idle.
    VoiceNotActive,
}

impl std::fmt::Display for VoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceError::WrongNoteId { expected, received } => write!(
                f,
                "NoteOff for wrong note id: expected {expected}, received {received}"
            ),
            VoiceError::VoiceNotActive => {
                write!(f, "NoteOff received but voice is not active")
            }
        }
    }
}

impl std::error::Error for VoiceError {}

// ─────────────────────────────────────────────────────────────────────────────
// Voice aggregate
// ─────────────────────────────────────────────────────────────────────────────

/// Aggregate root — a single sounding note.
///
/// Models one voice in a polyphonic synthesizer: a sine oscillator shaped
/// by a configurable ADSR amp envelope and a one-pole low-pass filter.
///
/// # Audio-thread safety
///
/// `render_sample` is lock-free and allocation-free: safe to call on the
/// audio thread.
///
/// # Invariants
///
/// 1. `frequency` is always derived from `note_number` (equal temperament,
///    A4 = 440 Hz). Pitch modulation is an additive semitone offset at render
///    time — the stored base frequency is set only by `note_on`.
/// 2. Envelope progresses `Idle → Attack → Decay → Sustain → Release → Idle`.
/// 3. `is_reclaimable()` returns `true` only when `envelope_stage == Idle`.
pub struct Voice {
    active: bool,
    envelope_config: AmpEnvelopeConfig,
    envelope_level: Amplitude,
    envelope_stage: EnvelopeStage,
    filter_state: FilterState,
    frequency: Frequency,
    note_id: NoteId,
    note_number: NoteNumber,
    oscillator_phase: f64,
    velocity: Velocity,
}

impl Voice {
    /// Create a new, idle [`Voice`] with default ADSR parameters.
    pub fn new() -> Self {
        Self::with_config(AmpEnvelopeConfig::default())
    }

    /// Create a new, idle [`Voice`] with the given ADSR configuration.
    pub fn with_config(envelope_config: AmpEnvelopeConfig) -> Self {
        Self {
            active: false,
            envelope_config,
            envelope_level: Amplitude::zero(),
            envelope_stage: EnvelopeStage::Idle,
            filter_state: FilterState::new(),
            frequency: Frequency::default(),
            note_id: NoteId::new(0),
            note_number: NoteNumber::try_new(69).expect("69 is a valid MIDI note"),
            oscillator_phase: 0.0,
            velocity: Velocity::default(),
        }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Returns `true` if the voice is currently active (not `Idle`).
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns `true` if the voice can be stolen for a new note.
    ///
    /// # Invariant
    ///
    /// A voice is reclaimable only when `envelope_stage == Idle`.
    pub fn is_reclaimable(&self) -> bool {
        self.envelope_stage == EnvelopeStage::Idle
    }

    /// The `NoteId` of the note currently (or most recently) playing.
    pub fn note_id(&self) -> NoteId {
        self.note_id
    }

    /// The current `EnvelopeStage`.
    pub fn envelope_stage(&self) -> EnvelopeStage {
        self.envelope_stage
    }

    /// The current envelope amplitude level.
    pub fn envelope_level(&self) -> Amplitude {
        self.envelope_level
    }

    /// The oscillator frequency (always derived from `note_number`).
    pub fn frequency(&self) -> Frequency {
        self.frequency
    }

    // ── Commands ──────────────────────────────────────────────────────────────

    /// **Command — NoteOn**: begin sounding a note.
    ///
    /// If the voice is currently active it is *stolen*: a `VoiceStolen` event
    /// is prepended to the returned list. Returns 1–2 events.
    ///
    /// # Invariant
    ///
    /// After this call `envelope_stage == Attack` and `frequency` is derived
    /// from `cmd.note_number`.
    pub fn note_on(&mut self, cmd: NoteOn) -> Vec<VoiceEvent> {
        // Invariant: frequency derived from note_number.
        let frequency = midi_note_to_frequency(cmd.note_number);
        let mut events: Vec<VoiceEvent> = Vec::with_capacity(2);

        if self.active {
            events.push(VoiceEvent::VoiceStolen {
                old_note_id: self.note_id,
                new_note_id: cmd.note_id,
            });
        }

        // Apply state.
        self.note_id = cmd.note_id;
        self.note_number = cmd.note_number;
        self.velocity = cmd.velocity;
        self.frequency = frequency;
        self.oscillator_phase = 0.0;
        self.envelope_stage = EnvelopeStage::Attack; // Invariant: Idle → Attack
        self.envelope_level = Amplitude::zero();
        self.filter_state.reset();
        self.active = true;

        events.push(VoiceEvent::VoiceActivated {
            note_id: cmd.note_id,
            note_number: cmd.note_number,
            frequency,
        });

        events
    }

    /// **Command — NoteOff**: begin the release phase.
    ///
    /// Returns `VoiceReleased` on success.
    ///
    /// # Errors
    ///
    /// - `VoiceNotActive` — the voice is already `Idle`.
    /// - `WrongNoteId` — the `NoteId` does not match the active note.
    pub fn note_off(&mut self, cmd: NoteOff) -> Result<VoiceEvent, VoiceError> {
        if !self.active || self.envelope_stage == EnvelopeStage::Idle {
            return Err(VoiceError::VoiceNotActive);
        }
        if self.note_id != cmd.note_id {
            return Err(VoiceError::WrongNoteId {
                expected: self.note_id,
                received: cmd.note_id,
            });
        }
        // Transition to Release (unless already releasing — idempotent).
        if self.envelope_stage != EnvelopeStage::Release {
            self.envelope_stage = EnvelopeStage::Release; // Invariant: → Release
        }
        Ok(VoiceEvent::VoiceReleased {
            note_id: self.note_id,
        })
    }

    // ── Audio rendering ───────────────────────────────────────────────────────

    /// Render one audio sample, advancing the oscillator phase and envelope.
    ///
    /// Returns 0.0 when the voice is idle. When the envelope completes its
    /// release stage this call emits a `VoiceFinished` event in the returned
    /// optional event.
    ///
    /// `semitone_detune` — additive pitch offset in fractional semitones (for
    /// vibrato / fine-tuning). The base `frequency` stored in the voice is
    /// never modified by this parameter.
    ///
    /// # Invariant
    ///
    /// This method is allocation-free and lock-free: safe on the audio thread.
    pub fn render_sample(
        &mut self,
        sample_rate: f64,
        semitone_detune: f64,
    ) -> (f32, Option<VoiceEvent>) {
        if !self.active {
            return (0.0, None);
        }

        // ── 1. Oscillator ──────────────────────────────────────────────────
        // Invariant: base frequency derived from note_number; detune is additive.
        let detuned_hz = self.frequency.hz() * 2.0_f64.powf(semitone_detune / 12.0);
        let raw_sample = self.oscillator_phase.sin();
        self.oscillator_phase += TAU * detuned_hz / sample_rate;
        if self.oscillator_phase >= TAU {
            self.oscillator_phase -= TAU;
        }

        // ── 2. Filter ──────────────────────────────────────────────────────
        // One-pole low-pass with a fixed warm coefficient.
        let filter_coeff = 0.3_f64;
        let filtered = self.filter_state.process(raw_sample, filter_coeff);

        // ── 3. Envelope ────────────────────────────────────────────────────
        let (envelope, finished) = self.advance_envelope(sample_rate);
        self.envelope_level = Amplitude::try_new(envelope).unwrap_or(Amplitude::zero());

        let vel_scale = self.velocity.value();
        let sample = (filtered * envelope * vel_scale) as f32;

        let event = if finished {
            Some(VoiceEvent::VoiceFinished {
                note_id: self.note_id,
            })
        } else {
            None
        };

        (sample, event)
    }

    /// Advance the ADSR envelope by one sample; returns `(level, finished)`.
    ///
    /// `finished` is `true` on the sample that transitions `Release → Idle`.
    ///
    /// # Invariant
    ///
    /// Stage order is strictly `Idle → Attack → Decay → Sustain → Release → Idle`.
    fn advance_envelope(&mut self, sample_rate: f64) -> (f64, bool) {
        let cfg = &self.envelope_config;
        // Compute per-sample increments from the config times (seconds).
        // Guard against zero time to avoid division by zero.
        let attack_inc = if cfg.attack > 0.0 {
            1.0 / (cfg.attack * sample_rate)
        } else {
            1.0 // Instantaneous attack
        };
        let decay_inc = if cfg.decay > 0.0 {
            (1.0 - cfg.sustain) / (cfg.decay * sample_rate)
        } else {
            1.0 // Instantaneous decay
        };
        let release_inc = if cfg.release > 0.0 {
            cfg.sustain / (cfg.release * sample_rate)
        } else {
            1.0 // Instantaneous release
        };

        let level = self.envelope_level.value();
        let mut finished = false;

        let new_level = match self.envelope_stage {
            EnvelopeStage::Idle => 0.0,
            EnvelopeStage::Attack => {
                let l = (level + attack_inc).min(1.0);
                if l >= 1.0 {
                    self.envelope_stage = EnvelopeStage::Decay; // Invariant: Attack → Decay
                }
                l
            }
            EnvelopeStage::Decay => {
                let l = (level - decay_inc).max(cfg.sustain);
                if l <= cfg.sustain {
                    self.envelope_stage = EnvelopeStage::Sustain; // Invariant: Decay → Sustain
                }
                l
            }
            EnvelopeStage::Sustain => cfg.sustain,
            EnvelopeStage::Release => {
                let l = (level - release_inc).max(0.0);
                if l <= 0.0 {
                    // Invariant: Release → Idle; voice is now reclaimable.
                    self.envelope_stage = EnvelopeStage::Idle;
                    self.active = false;
                    finished = true;
                }
                l
            }
        };

        (new_level, finished)
    }
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a MIDI note number to a [`Frequency`] using equal temperament.
///
/// A4 (note 69) = 440 Hz.
///
/// # Invariant
///
/// The returned frequency is always positive (guaranteed by the formula:
/// 440 * 2^x > 0 for all finite x).
fn midi_note_to_frequency(note: NoteNumber) -> Frequency {
    let hz = 440.0 * 2.0_f64.powf((note.value() as f64 - 69.0) / 12.0);
    Frequency::try_new(hz)
        .expect("midi_note_to_frequency always produces a positive finite frequency")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn note_id(v: u32) -> NoteId {
        NoteId::new(v)
    }

    fn note_number(v: u8) -> NoteNumber {
        NoteNumber::try_new(v).expect("valid note number")
    }

    fn velocity(v: f64) -> Velocity {
        Velocity::try_new(v).expect("valid velocity")
    }

    fn note_on_cmd(id: u32, nn: u8, vel: f64) -> NoteOn {
        NoteOn {
            note_id: note_id(id),
            note_number: note_number(nn),
            velocity: velocity(vel),
        }
    }

    fn note_off_cmd(id: u32) -> NoteOff {
        NoteOff {
            note_id: note_id(id),
        }
    }

    // ── Amplitude ─────────────────────────────────────────────────────────────

    #[test]
    fn amplitude_valid_range() {
        assert!(Amplitude::try_new(0.0).is_ok());
        assert!(Amplitude::try_new(1.0).is_ok());
        assert!(Amplitude::try_new(0.5).is_ok());
    }

    #[test]
    fn amplitude_out_of_range_rejected() {
        assert!(Amplitude::try_new(1.001).is_err());
        assert!(Amplitude::try_new(-0.001).is_err());
        assert!(Amplitude::try_new(f64::NAN).is_err());
    }

    // ── Frequency ─────────────────────────────────────────────────────────────

    #[test]
    fn frequency_positive_is_valid() {
        assert!(Frequency::try_new(440.0).is_ok());
        assert!(Frequency::try_new(8.0).is_ok()); // MIDI note 0 range
    }

    #[test]
    fn frequency_non_positive_rejected() {
        assert!(Frequency::try_new(0.0).is_err());
        assert!(Frequency::try_new(-1.0).is_err());
        assert!(Frequency::try_new(f64::NAN).is_err());
        assert!(Frequency::try_new(f64::INFINITY).is_err());
    }

    // ── FilterState ───────────────────────────────────────────────────────────

    #[test]
    fn filter_state_starts_at_zero() {
        let fs = FilterState::new();
        assert_eq!(fs.z1, 0.0);
    }

    #[test]
    fn filter_state_process_converges_toward_input() {
        let mut fs = FilterState::new();
        for _ in 0..1000 {
            fs.process(1.0, 0.1);
        }
        assert!((fs.z1 - 1.0).abs() < 0.01, "filter should converge to 1.0");
    }

    #[test]
    fn filter_state_reset_zeroes_state() {
        let mut fs = FilterState::new();
        fs.process(1.0, 0.5);
        fs.reset();
        assert_eq!(fs.z1, 0.0);
    }

    // ── midi_note_to_frequency invariant ─────────────────────────────────────

    #[test]
    fn a4_is_440hz() {
        let freq = midi_note_to_frequency(note_number(69));
        assert!((freq.hz() - 440.0).abs() < 0.01);
    }

    #[test]
    fn all_midi_note_numbers_produce_positive_frequency() {
        for n in [0u8, 60, 69, 127] {
            let freq = midi_note_to_frequency(note_number(n));
            assert!(
                freq.hz() > 0.0,
                "frequency must be positive for note {n}, got {}",
                freq.hz()
            );
        }
    }

    #[test]
    fn middle_c_is_approx_261hz() {
        let freq = midi_note_to_frequency(note_number(60));
        assert!(
            (freq.hz() - 261.63).abs() < 0.5,
            "C4 should be ~261.63 Hz, got {}",
            freq.hz()
        );
    }

    // ── NoteOn / VoiceActivated ───────────────────────────────────────────────

    #[test]
    fn note_on_emits_voice_activated() {
        let mut voice = Voice::new();
        let events = voice.note_on(note_on_cmd(1, 69, 0.8));
        assert_eq!(events.len(), 1);
        match &events[0] {
            VoiceEvent::VoiceActivated {
                note_id, frequency, ..
            } => {
                assert_eq!(*note_id, NoteId::new(1));
                assert!((frequency.hz() - 440.0).abs() < 0.01);
            }
            _ => panic!("expected VoiceActivated"),
        }
    }

    #[test]
    fn note_on_sets_voice_active() {
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 60, 0.5));
        assert!(voice.is_active());
    }

    #[test]
    fn note_on_frequency_derived_from_note_number() {
        // Invariant: frequency derived from note_number.
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 60, 0.5));
        let freq = voice.frequency().hz();
        assert!(
            (freq - 261.63).abs() < 0.5,
            "C4 should be ~261.63 Hz, got {freq}"
        );
    }

    #[test]
    fn note_on_starts_in_attack_stage() {
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 69, 0.5));
        assert_eq!(voice.envelope_stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn note_on_frequency_positive_for_all_midi_notes() {
        let mut voice = Voice::new();
        for n in [0u8, 60, 69, 127] {
            voice.note_on(NoteOn {
                note_id: NoteId::new(n as u32),
                note_number: note_number(n),
                velocity: velocity(0.5),
            });
            assert!(voice.frequency().hz() > 0.0);
        }
    }

    // ── Voice steal ───────────────────────────────────────────────────────────

    #[test]
    fn note_on_while_active_emits_voice_stolen_then_activated() {
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 60, 0.5));
        let events = voice.note_on(note_on_cmd(2, 72, 0.7));
        assert_eq!(events.len(), 2);
        match &events[0] {
            VoiceEvent::VoiceStolen {
                old_note_id,
                new_note_id,
            } => {
                assert_eq!(*old_note_id, NoteId::new(1));
                assert_eq!(*new_note_id, NoteId::new(2));
            }
            _ => panic!("expected VoiceStolen as first event"),
        }
        match &events[1] {
            VoiceEvent::VoiceActivated { note_id, .. } => {
                assert_eq!(*note_id, NoteId::new(2));
            }
            _ => panic!("expected VoiceActivated as second event"),
        }
    }

    // ── NoteOff / VoiceReleased ───────────────────────────────────────────────

    #[test]
    fn note_off_emits_voice_released() {
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 69, 0.5));
        let result = voice.note_off(note_off_cmd(1));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            VoiceEvent::VoiceReleased {
                note_id: NoteId::new(1)
            }
        );
    }

    #[test]
    fn note_off_transitions_to_release_stage() {
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 69, 0.5));
        voice.note_off(note_off_cmd(1)).unwrap();
        assert_eq!(voice.envelope_stage(), EnvelopeStage::Release);
    }

    #[test]
    fn note_off_wrong_note_id_returns_error() {
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 69, 0.5));
        let err = voice.note_off(note_off_cmd(99));
        assert!(matches!(err, Err(VoiceError::WrongNoteId { .. })));
    }

    #[test]
    fn note_off_on_idle_voice_returns_error() {
        let mut voice = Voice::new();
        let err = voice.note_off(note_off_cmd(1));
        assert_eq!(err, Err(VoiceError::VoiceNotActive));
    }

    // ── Envelope progression Idle→Attack→Decay→Sustain→Release→Idle ──────────

    #[test]
    fn envelope_progresses_attack_to_decay() {
        // Use a fast attack to keep the test short.
        let cfg = AmpEnvelopeConfig::try_new(0.001, 0.1, 0.7, 0.2).unwrap();
        let mut voice = Voice::with_config(cfg);
        voice.note_on(note_on_cmd(1, 69, 1.0));
        assert_eq!(voice.envelope_stage(), EnvelopeStage::Attack);

        // Drive enough samples to complete the 1 ms attack at 44100 Hz ≈ 45 samples.
        for _ in 0..100 {
            voice.render_sample(44100.0, 0.0);
        }
        assert_eq!(voice.envelope_stage(), EnvelopeStage::Decay);
    }

    #[test]
    fn envelope_progresses_decay_to_sustain() {
        // attack=1ms, decay=10ms.
        let cfg = AmpEnvelopeConfig::try_new(0.001, 0.01, 0.7, 0.2).unwrap();
        let mut voice = Voice::with_config(cfg);
        voice.note_on(note_on_cmd(1, 69, 1.0));
        // attack ≈ 45 samples, decay ≈ 441 samples.
        for _ in 0..600 {
            voice.render_sample(44100.0, 0.0);
        }
        assert_eq!(voice.envelope_stage(), EnvelopeStage::Sustain);
    }

    #[test]
    fn envelope_progresses_release_to_idle() {
        // attack=1ms, decay=10ms, sustain=0.7, release=10ms.
        let cfg = AmpEnvelopeConfig::try_new(0.001, 0.01, 0.7, 0.01).unwrap();
        let mut voice = Voice::with_config(cfg);
        voice.note_on(note_on_cmd(1, 69, 1.0));
        // Complete attack + decay + a few sustain samples.
        for _ in 0..700 {
            voice.render_sample(44100.0, 0.0);
        }
        // Trigger release.
        voice.note_off(note_off_cmd(1)).unwrap();
        assert_eq!(voice.envelope_stage(), EnvelopeStage::Release);
        // Drive through release ≈ 441 samples.
        for _ in 0..500 {
            voice.render_sample(44100.0, 0.0);
        }
        assert_eq!(voice.envelope_stage(), EnvelopeStage::Idle);
    }

    #[test]
    fn voice_is_reclaimable_only_when_idle() {
        // Invariant: reclaimable only when Idle.
        let cfg = AmpEnvelopeConfig::try_new(0.001, 0.01, 0.7, 0.01).unwrap();
        let mut voice = Voice::with_config(cfg);

        // Initially idle → reclaimable.
        assert!(voice.is_reclaimable());

        voice.note_on(note_on_cmd(1, 69, 1.0));
        assert!(
            !voice.is_reclaimable(),
            "active voice must not be reclaimable"
        );

        voice.note_off(note_off_cmd(1)).unwrap();
        assert!(
            !voice.is_reclaimable(),
            "releasing voice must not be reclaimable"
        );

        // Drive to idle.
        for _ in 0..1200 {
            voice.render_sample(44100.0, 0.0);
        }
        assert!(voice.is_reclaimable(), "idle voice must be reclaimable");
    }

    // ── VoiceFinished emitted on transition to Idle ───────────────────────────

    #[test]
    fn render_sample_emits_voice_finished_when_release_completes() {
        let cfg = AmpEnvelopeConfig::try_new(0.001, 0.01, 0.7, 0.001).unwrap();
        let mut voice = Voice::with_config(cfg);
        voice.note_on(note_on_cmd(1, 69, 1.0));
        // Complete attack + decay + sustain.
        for _ in 0..700 {
            voice.render_sample(44100.0, 0.0);
        }
        voice.note_off(note_off_cmd(1)).unwrap();

        let mut finished_event = None;
        for _ in 0..200 {
            let (_, ev) = voice.render_sample(44100.0, 0.0);
            if let Some(e) = ev {
                finished_event = Some(e);
                break;
            }
        }
        assert!(
            matches!(
                finished_event,
                Some(VoiceEvent::VoiceFinished { note_id }) if note_id == NoteId::new(1)
            ),
            "expected VoiceFinished event"
        );
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    #[test]
    fn render_sample_is_zero_when_idle() {
        let mut voice = Voice::new();
        let (sample, ev) = voice.render_sample(44100.0, 0.0);
        assert_eq!(sample, 0.0_f32);
        assert!(ev.is_none());
    }

    #[test]
    fn render_sample_non_zero_after_note_on() {
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 69, 0.8));
        // Skip the first sample (phase=0 → sin(0)=0).
        voice.render_sample(44100.0, 0.0);
        let (sample, _) = voice.render_sample(44100.0, 0.0);
        assert!(
            sample.abs() > 1e-6,
            "expected non-zero sample after note_on, got {sample}"
        );
    }

    #[test]
    fn render_sample_accepts_detune_without_panic() {
        let mut voice = Voice::new();
        voice.note_on(note_on_cmd(1, 69, 0.8));
        let (_s, _ev) = voice.render_sample(44100.0, 1.0);
    }
}
