// path: src/engine/voice.rs

//! One sounding note: oscillator through filter through amp/filter/pitch envelopes, with
//! per-note expression. This module owns the amp-envelope state machine and per-note
//! expression state for a single `Voice`. Oscillator/filter rendering itself is delegated
//! to ports defined elsewhere (Oscillator, Filter, EnvelopeGenerator) and driven by a
//! domain service -- this aggregate never instantiates those dependencies itself.
//!
//! The value objects `NoteId`, `NoteNumber`, `Velocity` and `VoiceConfig` referenced by the
//! resource declaration are not yet available as shared kernel/engine modules in this
//! project, so they are defined locally here. When the shared `valueObject.Kernel.NoteId`,
//! `valueObject.Kernel.NoteNumber`, `valueObject.Kernel.Velocity` and
//! `valueObject.Engine.VoiceConfig` resources are generated, these local definitions should
//! be replaced by `use` imports of the shared types.

/// Unique identifier correlating commands and events to one sounding note instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoteId(u64);

impl NoteId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Error returned when a `NoteNumber` is constructed outside the valid MIDI range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteNumberError;

/// A MIDI note number in `0..=127`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteNumber(u8);

impl NoteNumber {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 127;

    pub fn try_new(value: u8) -> Result<Self, NoteNumberError> {
        if value > Self::MAX {
            Err(NoteNumberError)
        } else {
            Ok(Self(value))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Error returned when a `Velocity` is constructed outside the valid normalized range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityError;

/// A normalized note-on velocity in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity(f64);

impl Velocity {
    pub fn try_new(value: f64) -> Result<Self, VelocityError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            Err(VelocityError)
        } else {
            Ok(Self(value))
        }
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Timing/shape configuration for the amp envelope's Attack/Decay/Sustain/Release stages.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopeTiming {
    pub attack_seconds: f64,
    pub decay_seconds: f64,
    pub sustain_level: f64,
    pub release_seconds: f64,
}

impl EnvelopeTiming {
    pub fn new(
        attack_seconds: f64,
        decay_seconds: f64,
        sustain_level: f64,
        release_seconds: f64,
    ) -> Self {
        Self {
            attack_seconds: attack_seconds.max(0.0),
            decay_seconds: decay_seconds.max(0.0),
            sustain_level: sustain_level.clamp(0.0, 1.0),
            release_seconds: release_seconds.max(0.0),
        }
    }
}

/// Static per-voice configuration. Only the amp envelope timing is modeled here because it
/// is the piece this aggregate's invariants govern directly; oscillator/filter/pitch
/// envelope configuration belongs to the ports and value objects that render a `Voice`, not
/// to the aggregate's own state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceConfig {
    pub amp_envelope: EnvelopeTiming,
}

impl VoiceConfig {
    pub fn new(amp_envelope: EnvelopeTiming) -> Self {
        Self { amp_envelope }
    }
}

/// Stage of the amp envelope state machine. Progression is strictly
/// `Idle -> Attack -> Decay -> Sustain -> Release -> Idle` with no other transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmpEnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Events emitted by a `Voice` in response to commands or time advancement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceEvent {
    Triggered { note_id: NoteId },
    Released { note_id: NoteId },
    BecameIdle { note_id: NoteId },
}

/// Errors returned when a command cannot be applied to a `Voice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceCommandError {
    /// The command's `NoteId` does not match this voice's current note. Per-note commands
    /// (release, expression) affect only the voice with the matching `NoteId`; a mismatch is
    /// rejected rather than silently ignored or misapplied.
    NoteIdMismatch,
    /// `Trigger` was issued against a voice whose amp envelope has not reached `Idle`. A
    /// voice is reclaimable -- and therefore triggerable -- only when idle.
    VoiceNotIdle,
    /// `Release` was issued against a voice that is already `Idle`.
    VoiceAlreadyIdle,
}

/// One sounding note: oscillator through filter through amp/filter/pitch envelopes, with
/// per-note expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Voice {
    config: VoiceConfig,
    note: NoteNumber,
    note_id: NoteId,
    phase: f64,
    velocity: Velocity,
    amp_stage: AmpEnvelopeStage,
    stage_elapsed_seconds: f64,
    pitch_bend: f64,
    pressure: f64,
    slide: f64,
}

impl Voice {
    /// Creates a freshly-allocated, idle voice. `note`, `note_id` and `velocity` are
    /// placeholders until the first `Trigger` assigns real values; the voice does not sound
    /// until triggered.
    pub fn new(config: VoiceConfig, note: NoteNumber, note_id: NoteId, velocity: Velocity) -> Self {
        Self {
            config,
            note,
            note_id,
            phase: 0.0,
            velocity,
            amp_stage: AmpEnvelopeStage::Idle,
            stage_elapsed_seconds: 0.0,
            pitch_bend: 0.0,
            pressure: 0.0,
            slide: 0.0,
        }
    }

    pub fn config(&self) -> VoiceConfig {
        self.config
    }

    pub fn note(&self) -> NoteNumber {
        self.note
    }

    pub fn note_id(&self) -> NoteId {
        self.note_id
    }

    pub fn phase(&self) -> f64 {
        self.phase
    }

    pub fn velocity(&self) -> Velocity {
        self.velocity
    }

    pub fn amp_stage(&self) -> AmpEnvelopeStage {
        self.amp_stage
    }

    pub fn pitch_bend(&self) -> f64 {
        self.pitch_bend
    }

    pub fn pressure(&self) -> f64 {
        self.pressure
    }

    pub fn slide(&self) -> f64 {
        self.slide
    }

    /// A voice is reclaimable only when its amp envelope has reached `Idle`.
    pub fn is_reclaimable(&self) -> bool {
        self.amp_stage == AmpEnvelopeStage::Idle
    }

    /// Starts a new note on this voice, moving the amp envelope from `Idle` to `Attack`.
    /// Rejected unless the voice is currently idle/reclaimable -- allocation policy for
    /// choosing *which* voice to trigger belongs to `domainService.Engine.VoiceAllocator`,
    /// not to this aggregate.
    pub fn trigger(
        &mut self,
        note: NoteNumber,
        note_id: NoteId,
        velocity: Velocity,
    ) -> Result<VoiceEvent, VoiceCommandError> {
        if self.amp_stage != AmpEnvelopeStage::Idle {
            return Err(VoiceCommandError::VoiceNotIdle);
        }

        self.note = note;
        self.note_id = note_id;
        self.velocity = velocity;
        self.phase = 0.0;
        self.pitch_bend = 0.0;
        self.pressure = 0.0;
        self.slide = 0.0;
        self.amp_stage = AmpEnvelopeStage::Attack;
        self.stage_elapsed_seconds = 0.0;

        Ok(VoiceEvent::Triggered { note_id })
    }

    /// Begins release, moving the amp envelope from `Sustain` (or any non-idle stage) to
    /// `Release`. Rejected if `note_id` does not match this voice's current note, or if the
    /// voice is already idle.
    pub fn release(&mut self, note_id: NoteId) -> Result<VoiceEvent, VoiceCommandError> {
        if note_id != self.note_id {
            return Err(VoiceCommandError::NoteIdMismatch);
        }
        if self.amp_stage == AmpEnvelopeStage::Idle {
            return Err(VoiceCommandError::VoiceAlreadyIdle);
        }

        self.amp_stage = AmpEnvelopeStage::Release;
        self.stage_elapsed_seconds = 0.0;

        Ok(VoiceEvent::Released { note_id })
    }

    /// Applies per-note expression (MPE-style pitch bend / pressure / slide). Rejected if
    /// `note_id` does not match this voice's current note -- per-note expression affects
    /// only the voice with the matching `NoteId`.
    pub fn apply_expression(
        &mut self,
        note_id: NoteId,
        pitch_bend: f64,
        pressure: f64,
        slide: f64,
    ) -> Result<(), VoiceCommandError> {
        if note_id != self.note_id {
            return Err(VoiceCommandError::NoteIdMismatch);
        }

        self.pitch_bend = pitch_bend;
        self.pressure = pressure;
        self.slide = slide;

        Ok(())
    }

    /// Advances the amp envelope by `dt_seconds` of wall-clock time, following the fixed
    /// progression `Idle -> Attack -> Decay -> Sustain -> Release -> Idle`. `Sustain` never
    /// advances on its own; only an explicit `release` moves it onward. Returns
    /// `VoiceEvent::BecameIdle` exactly when the `Release` stage completes.
    pub fn advance(&mut self, dt_seconds: f64) -> Option<VoiceEvent> {
        if dt_seconds <= 0.0 {
            return None;
        }

        match self.amp_stage {
            AmpEnvelopeStage::Idle | AmpEnvelopeStage::Sustain => None,
            AmpEnvelopeStage::Attack => {
                self.stage_elapsed_seconds += dt_seconds;
                if self.stage_elapsed_seconds >= self.config.amp_envelope.attack_seconds {
                    self.amp_stage = AmpEnvelopeStage::Decay;
                    self.stage_elapsed_seconds = 0.0;
                }
                None
            }
            AmpEnvelopeStage::Decay => {
                self.stage_elapsed_seconds += dt_seconds;
                if self.stage_elapsed_seconds >= self.config.amp_envelope.decay_seconds {
                    self.amp_stage = AmpEnvelopeStage::Sustain;
                    self.stage_elapsed_seconds = 0.0;
                }
                None
            }
            AmpEnvelopeStage::Release => {
                self.stage_elapsed_seconds += dt_seconds;
                if self.stage_elapsed_seconds >= self.config.amp_envelope.release_seconds {
                    self.amp_stage = AmpEnvelopeStage::Idle;
                    self.stage_elapsed_seconds = 0.0;
                    return Some(VoiceEvent::BecameIdle {
                        note_id: self.note_id,
                    });
                }
                None
            }
        }
    }

    /// Current amp envelope output level in `0.0..=1.0`, derived purely from stage and
    /// elapsed time -- convenient for tests and simple rendering without a full
    /// `EnvelopeGenerator` port implementation.
    pub fn amp_level(&self) -> f64 {
        let timing = self.config.amp_envelope;
        match self.amp_stage {
            AmpEnvelopeStage::Idle => 0.0,
            AmpEnvelopeStage::Attack => {
                if timing.attack_seconds <= 0.0 {
                    1.0
                } else {
                    (self.stage_elapsed_seconds / timing.attack_seconds).clamp(0.0, 1.0)
                }
            }
            AmpEnvelopeStage::Decay => {
                if timing.decay_seconds <= 0.0 {
                    timing.sustain_level
                } else {
                    let t = (self.stage_elapsed_seconds / timing.decay_seconds).clamp(0.0, 1.0);
                    1.0 + (timing.sustain_level - 1.0) * t
                }
            }
            AmpEnvelopeStage::Sustain => timing.sustain_level,
            AmpEnvelopeStage::Release => {
                if timing.release_seconds <= 0.0 {
                    0.0
                } else {
                    let t = (self.stage_elapsed_seconds / timing.release_seconds).clamp(0.0, 1.0);
                    timing.sustain_level * (1.0 - t)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing() -> EnvelopeTiming {
        EnvelopeTiming::new(0.1, 0.2, 0.5, 0.3)
    }

    fn config() -> VoiceConfig {
        VoiceConfig::new(timing())
    }

    fn idle_voice() -> Voice {
        Voice::new(
            config(),
            NoteNumber::try_new(60).unwrap(),
            NoteId::new(1),
            Velocity::try_new(0.8).unwrap(),
        )
    }

    #[test]
    fn note_number_accepts_full_midi_range() {
        assert!(NoteNumber::try_new(0).is_ok());
        assert!(NoteNumber::try_new(127).is_ok());
    }

    #[test]
    fn note_number_rejects_out_of_range() {
        assert_eq!(NoteNumber::try_new(128), Err(NoteNumberError));
    }

    #[test]
    fn velocity_accepts_normalized_bounds() {
        assert!(Velocity::try_new(0.0).is_ok());
        assert!(Velocity::try_new(1.0).is_ok());
    }

    #[test]
    fn velocity_rejects_out_of_range_and_nan() {
        assert_eq!(Velocity::try_new(-0.01), Err(VelocityError));
        assert_eq!(Velocity::try_new(1.01), Err(VelocityError));
        assert_eq!(Velocity::try_new(f64::NAN), Err(VelocityError));
    }

    #[test]
    fn new_voice_starts_idle_and_reclaimable() {
        let voice = idle_voice();
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Idle);
        assert!(voice.is_reclaimable());
    }

    #[test]
    fn trigger_from_idle_moves_to_attack_and_emits_triggered() {
        let mut voice = idle_voice();
        let note_id = NoteId::new(42);
        let event = voice
            .trigger(
                NoteNumber::try_new(64).unwrap(),
                note_id,
                Velocity::try_new(1.0).unwrap(),
            )
            .unwrap();

        assert_eq!(event, VoiceEvent::Triggered { note_id });
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Attack);
        assert_eq!(voice.note_id(), note_id);
        assert!(!voice.is_reclaimable());
    }

    #[test]
    fn trigger_rejected_when_voice_not_idle() {
        let mut voice = idle_voice();
        voice
            .trigger(
                NoteNumber::try_new(60).unwrap(),
                NoteId::new(1),
                Velocity::try_new(0.5).unwrap(),
            )
            .unwrap();

        let result = voice.trigger(
            NoteNumber::try_new(61).unwrap(),
            NoteId::new(2),
            Velocity::try_new(0.5).unwrap(),
        );
        assert_eq!(result, Err(VoiceCommandError::VoiceNotIdle));
    }

    #[test]
    fn full_envelope_progression_follows_canonical_order() {
        let mut voice = idle_voice();
        let note_id = NoteId::new(7);
        voice
            .trigger(
                NoteNumber::try_new(60).unwrap(),
                note_id,
                Velocity::try_new(0.8).unwrap(),
            )
            .unwrap();
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Attack);

        // Attack -> Decay
        assert_eq!(voice.advance(0.1), None);
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Decay);

        // Decay -> Sustain
        assert_eq!(voice.advance(0.2), None);
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Sustain);

        // Sustain never advances on its own.
        assert_eq!(voice.advance(10.0), None);
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Sustain);

        // Sustain -> Release only via explicit release.
        let released = voice.release(note_id).unwrap();
        assert_eq!(released, VoiceEvent::Released { note_id });
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Release);

        // Release -> Idle
        let became_idle = voice.advance(0.3);
        assert_eq!(became_idle, Some(VoiceEvent::BecameIdle { note_id }));
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Idle);
        assert!(voice.is_reclaimable());
    }

    #[test]
    fn release_rejected_for_mismatched_note_id() {
        let mut voice = idle_voice();
        let note_id = NoteId::new(1);
        voice
            .trigger(
                NoteNumber::try_new(60).unwrap(),
                note_id,
                Velocity::try_new(0.8).unwrap(),
            )
            .unwrap();

        let result = voice.release(NoteId::new(999));
        assert_eq!(result, Err(VoiceCommandError::NoteIdMismatch));
        assert_eq!(voice.amp_stage(), AmpEnvelopeStage::Attack);
    }

    #[test]
    fn release_rejected_when_already_idle() {
        let mut voice = idle_voice();
        let result = voice.release(voice.note_id());
        assert_eq!(result, Err(VoiceCommandError::VoiceAlreadyIdle));
    }

    #[test]
    fn apply_expression_updates_only_matching_note_id() {
        let mut voice = idle_voice();
        let note_id = NoteId::new(3);
        voice
            .trigger(
                NoteNumber::try_new(60).unwrap(),
                note_id,
                Velocity::try_new(0.8).unwrap(),
            )
            .unwrap();

        voice.apply_expression(note_id, 0.25, 0.6, -0.1).unwrap();
        assert_eq!(voice.pitch_bend(), 0.25);
        assert_eq!(voice.pressure(), 0.6);
        assert_eq!(voice.slide(), -0.1);
    }

    #[test]
    fn apply_expression_rejected_for_mismatched_note_id() {
        let mut voice = idle_voice();
        let note_id = NoteId::new(3);
        voice
            .trigger(
                NoteNumber::try_new(60).unwrap(),
                note_id,
                Velocity::try_new(0.8).unwrap(),
            )
            .unwrap();

        let result = voice.apply_expression(NoteId::new(4), 0.25, 0.6, -0.1);
        assert_eq!(result, Err(VoiceCommandError::NoteIdMismatch));
        // Unmodified because the command targeted a different voice.
        assert_eq!(voice.pitch_bend(), 0.0);
        assert_eq!(voice.pressure(), 0.0);
        assert_eq!(voice.slide(), 0.0);
    }

    #[test]
    fn is_reclaimable_only_true_at_idle_stage() {
        let mut voice = idle_voice();
        let note_id = voice.note_id();
        assert!(voice.is_reclaimable());

        voice
            .trigger(
                NoteNumber::try_new(60).unwrap(),
                note_id,
                Velocity::try_new(0.8).unwrap(),
            )
            .unwrap();
        assert!(!voice.is_reclaimable());

        voice.advance(0.1); // -> Decay
        assert!(!voice.is_reclaimable());
        voice.advance(0.2); // -> Sustain
        assert!(!voice.is_reclaimable());

        voice.release(note_id).unwrap();
        assert!(!voice.is_reclaimable());

        voice.advance(0.3); // -> Idle
        assert!(voice.is_reclaimable());
    }

    #[test]
    fn amp_level_ramps_from_zero_to_one_during_attack() {
        let mut voice = idle_voice();
        let note_id = voice.note_id();
        voice
            .trigger(
                NoteNumber::try_new(60).unwrap(),
                note_id,
                Velocity::try_new(0.8).unwrap(),
            )
            .unwrap();

        assert_eq!(voice.amp_level(), 0.0);
        voice.advance(0.05);
        assert!(voice.amp_level() > 0.0 && voice.amp_level() < 1.0);
    }
}
