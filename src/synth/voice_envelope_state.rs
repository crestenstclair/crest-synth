use crate::synth::voice_envelope::VoiceEnvelope;

/// One bounded sample-domain stage for an engine-owned note voice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceEnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Destructor-free ADSR state embedded in one prepared note voice.
///
/// Attack, Decay, and Sustain are latched at note-on. Release is latched at
/// note-off. Stage transitions contain no loops and zero-time stages collapse
/// immediately in bounded work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceEnvelopeState {
    stage: VoiceEnvelopeStage,
    level: f32,
    remaining_samples: u32,
    increment: f32,
    latched_decay_milliseconds: f32,
    latched_sustain: f32,
}

impl VoiceEnvelopeState {
    pub const IDLE: Self = Self {
        stage: VoiceEnvelopeStage::Idle,
        level: 0.0,
        remaining_samples: 0,
        increment: 0.0,
        latched_decay_milliseconds: 0.0,
        latched_sustain: 0.0,
    };

    pub const fn new() -> Self {
        Self::IDLE
    }

    pub const fn stage(&self) -> VoiceEnvelopeStage {
        self.stage
    }

    pub const fn level(&self) -> f32 {
        self.level
    }

    pub const fn is_idle(&self) -> bool {
        matches!(self.stage, VoiceEnvelopeStage::Idle)
    }

    pub const fn is_releasing(&self) -> bool {
        matches!(self.stage, VoiceEnvelopeStage::Release)
    }

    /// Restarts the voice and latches Attack/Decay/Sustain from this snapshot.
    pub fn note_on(&mut self, envelope: VoiceEnvelope, sample_rate: f32) {
        self.level = 0.0;
        self.latched_decay_milliseconds = envelope.decay_milliseconds();
        self.latched_sustain = envelope.sustain();
        let attack_samples = milliseconds_to_samples(envelope.attack_milliseconds(), sample_rate);
        if attack_samples == 0 {
            self.begin_decay(sample_rate);
        } else {
            self.stage = VoiceEnvelopeStage::Attack;
            self.remaining_samples = attack_samples;
            self.increment = 1.0 / attack_samples as f32;
        }
    }

    /// Begins release from the exact current level and latches this duration.
    pub fn note_off(&mut self, release_milliseconds: f32, sample_rate: f32) {
        if self.is_idle() {
            return;
        }
        let release_samples = milliseconds_to_samples(release_milliseconds, sample_rate);
        if release_samples == 0 || self.level <= 0.0 {
            self.clear();
        } else {
            self.stage = VoiceEnvelopeStage::Release;
            self.remaining_samples = release_samples;
            self.increment = -self.level / release_samples as f32;
        }
    }

    /// Advances exactly one output sample and returns a finite bounded gain.
    pub fn next_gain(&mut self, sample_rate: f32) -> f32 {
        match self.stage {
            VoiceEnvelopeStage::Idle => 0.0,
            VoiceEnvelopeStage::Sustain => self.level,
            VoiceEnvelopeStage::Attack
            | VoiceEnvelopeStage::Decay
            | VoiceEnvelopeStage::Release => {
                self.level = (self.level + self.increment).clamp(0.0, 1.0);
                self.remaining_samples = self.remaining_samples.saturating_sub(1);
                if self.remaining_samples == 0 {
                    match self.stage {
                        VoiceEnvelopeStage::Attack => self.begin_decay(sample_rate),
                        VoiceEnvelopeStage::Decay => self.begin_sustain(),
                        VoiceEnvelopeStage::Release => self.clear(),
                        VoiceEnvelopeStage::Idle | VoiceEnvelopeStage::Sustain => {}
                    }
                }
                self.level
            }
        }
    }

    /// Immediately returns the slot to its reusable idle state.
    pub fn clear(&mut self) {
        *self = Self::IDLE;
    }

    fn begin_decay(&mut self, sample_rate: f32) {
        self.level = 1.0;
        let decay_samples = milliseconds_to_samples(self.latched_decay_milliseconds, sample_rate);
        if decay_samples == 0 {
            self.begin_sustain();
        } else {
            self.stage = VoiceEnvelopeStage::Decay;
            self.remaining_samples = decay_samples;
            self.increment = (self.latched_sustain - 1.0) / decay_samples as f32;
        }
    }

    fn begin_sustain(&mut self) {
        self.stage = VoiceEnvelopeStage::Sustain;
        self.level = self.latched_sustain;
        self.remaining_samples = 0;
        self.increment = 0.0;
    }
}

impl Default for VoiceEnvelopeState {
    fn default() -> Self {
        Self::IDLE
    }
}

fn milliseconds_to_samples(milliseconds: f32, sample_rate: f32) -> u32 {
    if !milliseconds.is_finite()
        || !sample_rate.is_finite()
        || milliseconds <= 0.0
        || sample_rate <= 0.0
    {
        return 0;
    }
    let samples = f64::from(milliseconds) * f64::from(sample_rate) / 1_000.0;
    samples.round().clamp(1.0, f64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    use super::{VoiceEnvelopeStage, VoiceEnvelopeState};
    use crate::synth::voice_envelope::VoiceEnvelope;

    #[test]
    fn zero_time_stages_collapse_without_a_loop() {
        let mut state = VoiceEnvelopeState::new();
        state.note_on(VoiceEnvelope::DEFAULT, 48_000.0);
        assert_eq!(state.stage(), VoiceEnvelopeStage::Sustain);
        assert_eq!(state.next_gain(48_000.0), 1.0);

        state.note_off(0.0, 48_000.0);
        assert!(state.is_idle());
        assert_eq!(state.next_gain(48_000.0), 0.0);
    }

    #[test]
    fn attack_decay_and_release_advance_exactly_one_sample_at_a_time() {
        let envelope = VoiceEnvelope::new(2.0, 2.0, 0.5, 4.0).unwrap();
        let mut state = VoiceEnvelopeState::new();
        state.note_on(envelope, 1_000.0);

        assert_eq!(state.next_gain(1_000.0), 0.5);
        assert_eq!(state.next_gain(1_000.0), 1.0);
        assert_eq!(state.next_gain(1_000.0), 0.75);
        assert_eq!(state.next_gain(1_000.0), 0.5);
        assert_eq!(state.stage(), VoiceEnvelopeStage::Sustain);

        state.note_off(envelope.release_milliseconds(), 1_000.0);
        assert_eq!(state.next_gain(1_000.0), 0.375);
        assert_eq!(state.next_gain(1_000.0), 0.25);
        assert_eq!(state.next_gain(1_000.0), 0.125);
        assert_eq!(state.next_gain(1_000.0), 0.0);
        assert!(state.is_idle());
    }

    #[test]
    fn note_on_and_note_off_latch_independent_snapshot_fields() {
        let note_on = VoiceEnvelope::new(2.0, 2.0, 0.5, 1.0).unwrap();
        let note_off = VoiceEnvelope::new(0.0, 0.0, 1.0, 4.0).unwrap();
        let mut first = VoiceEnvelopeState::new();
        let mut second = VoiceEnvelopeState::new();
        first.note_on(note_on, 1_000.0);
        second.note_on(VoiceEnvelope::DEFAULT, 1_000.0);

        assert_eq!(first.next_gain(1_000.0), 0.5);
        assert_eq!(second.next_gain(1_000.0), 1.0);
        first.note_off(note_off.release_milliseconds(), 1_000.0);
        assert_eq!(first.next_gain(1_000.0), 0.375);
        assert_eq!(second.next_gain(1_000.0), 1.0);
    }

    #[test]
    fn extreme_finite_times_and_rates_remain_finite_and_destructor_free() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<VoiceEnvelopeState>();
        assert!(!core::mem::needs_drop::<VoiceEnvelopeState>());

        let mut state = VoiceEnvelopeState::new();
        state.note_on(
            VoiceEnvelope::new(10_000.0, 10_000.0, 0.0, 10_000.0).unwrap(),
            f32::MAX,
        );
        for _ in 0..32 {
            assert!(state.next_gain(f32::MAX).is_finite());
        }
    }
}
