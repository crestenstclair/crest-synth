// path: src/engine/envelope_generator.rs

//! Port: `EnvelopeGenerator`.
//!
//! This is a hexagonal *port* — an abstraction the real-time audio thread
//! depends on, not a concrete implementation. Concrete envelope shapes
//! (ADSR, AR, DAHDSR, ...) are adapters that implement this trait; this
//! module only defines the contract they must honor, so audio-thread
//! consumers depend on the abstraction (dependency inversion) rather than
//! any specific envelope shape.
//!
//! # Real-time safety contract
//!
//! Every implementation of this trait MUST uphold the project's
//! architectural invariants for anything invoked from the audio callback:
//!
//! - never allocate heap memory in `trigger`, `release`, or `tick`
//! - never acquire a mutex or other blocking lock in these methods
//! - never perform blocking I/O in these methods
//! - accept parameter changes only via the `ParameterBridge` or the
//!   `EventRing` — never by exposing setters a non-audio thread could call
//!   directly and race with the audio thread

/// A real-time-safe envelope generator.
///
/// `trigger` (re)starts the envelope from its onset stage, `release`
/// signals note-off and moves the envelope toward silence, and `tick`
/// advances the envelope by exactly one sample and reports the current
/// amplitude level.
///
/// Implementations are driven once per sample from the audio thread's
/// inner loop, so `tick` must be O(1) and allocation-free.
pub trait EnvelopeGenerator {
    /// (Re)starts the envelope from its onset stage.
    ///
    /// Calling `trigger` while the envelope is already running (e.g. a
    /// fast retrigger) restarts the envelope from its onset stage rather
    /// than requiring the caller to `release` first.
    fn trigger(&mut self);

    /// Signals note-off, moving the envelope toward its release stage.
    ///
    /// Calling `release` before `trigger` (or after the envelope has
    /// already reached silence) must be a safe no-op — it must never
    /// panic, allocate, or block.
    fn release(&mut self);

    /// Advances the envelope by exactly one sample and returns the
    /// resulting amplitude level, in the inclusive range `[0.0, 1.0]`.
    ///
    /// Called once per sample from the audio thread; must never allocate,
    /// lock, or block.
    fn tick(&mut self) -> f64;
}

#[cfg(test)]
mod tests {
    use super::EnvelopeGenerator;

    /// A minimal, deterministic test double used only to verify that
    /// consumers can depend on the `EnvelopeGenerator` trait (dependency
    /// inversion / Liskov substitution) rather than any concrete envelope
    /// shape. Private to this test module; intentionally does not model
    /// real ADSR timing.
    struct StubEnvelopeGenerator {
        triggered: bool,
        released: bool,
    }

    impl StubEnvelopeGenerator {
        fn new() -> Self {
            Self {
                triggered: false,
                released: false,
            }
        }
    }

    impl EnvelopeGenerator for StubEnvelopeGenerator {
        fn trigger(&mut self) {
            self.triggered = true;
            self.released = false;
        }

        fn release(&mut self) {
            self.released = true;
        }

        fn tick(&mut self) -> f64 {
            if self.released || !self.triggered {
                0.0
            } else {
                1.0
            }
        }
    }

    /// Exercises a generator purely through the trait, confirming the
    /// port can be consumed without knowledge of any concrete adapter.
    fn drive<G: EnvelopeGenerator>(generator: &mut G) -> (f64, f64, f64) {
        let before_trigger = generator.tick();
        generator.trigger();
        let during_attack = generator.tick();
        generator.release();
        let after_release = generator.tick();
        (before_trigger, during_attack, after_release)
    }

    #[test]
    fn tick_before_trigger_is_silent() {
        let mut env = StubEnvelopeGenerator::new();
        assert_eq!(env.tick(), 0.0);
    }

    #[test]
    fn trigger_then_tick_produces_nonzero_level() {
        let mut env = StubEnvelopeGenerator::new();
        env.trigger();
        assert_eq!(env.tick(), 1.0);
    }

    #[test]
    fn release_drives_level_toward_silence() {
        let mut env = StubEnvelopeGenerator::new();
        env.trigger();
        let _ = env.tick();
        env.release();
        assert_eq!(env.tick(), 0.0);
    }

    #[test]
    fn release_before_trigger_is_a_safe_no_op() {
        let mut env = StubEnvelopeGenerator::new();
        env.release();
        assert_eq!(env.tick(), 0.0);
    }

    #[test]
    fn retrigger_resets_from_onset() {
        let mut env = StubEnvelopeGenerator::new();
        env.trigger();
        env.release();
        let _ = env.tick();
        env.trigger();
        assert_eq!(env.tick(), 1.0);
    }

    #[test]
    fn generator_is_usable_purely_through_the_trait() {
        let mut env = StubEnvelopeGenerator::new();
        let (before, during, after) = drive(&mut env);
        assert_eq!(before, 0.0);
        assert_eq!(during, 1.0);
        assert_eq!(after, 0.0);
    }
}
