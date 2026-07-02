// path: src/effects/effect_slot.rs

//! EffectSlot: one position in an effect chain holding a processor and its bypass flag.
//!
//! Real-time invariant: `EffectSlot::process_sample` runs on the audio thread
//! and must never allocate, lock, or block. It contains no cross-thread
//! synchronization of its own — the bypass flag and processor parameters are
//! plain state that must be updated only via values already delivered across
//! the RT boundary (through the EventRing or ParameterBridge), never by this
//! type reaching across threads itself.

/// A single-sample audio effect processor that can occupy an [`EffectSlot`].
///
/// Implementors must be safe to call from the real-time audio thread:
/// `process_sample` must not allocate, lock, or perform blocking I/O.
pub trait Processor: Send {
    /// Process one sample in place and return the result. Must not allocate,
    /// lock, or block — this is called from the audio thread's inner loop.
    fn process_sample(&mut self, sample: f32) -> f32;

    /// Human-readable name for UI display. Not called from the audio thread.
    fn name(&self) -> &str;
}

/// One position in an effect chain: a processor plus its bypass flag.
///
/// When bypassed, the slot passes its input through unchanged instead of
/// running it through the processor. Chains compose slots strictly in order
/// with no feedback within the chain (see `EffectChain`).
pub struct EffectSlot<P: Processor> {
    processor: P,
    bypassed: bool,
}

impl<P: Processor> EffectSlot<P> {
    /// Construct a new slot wrapping `processor`, starting bypassed or active
    /// per the `bypassed` flag.
    pub fn new(processor: P, bypassed: bool) -> Self {
        Self {
            processor,
            bypassed,
        }
    }

    /// True if this slot is currently bypassed (signal passes through
    /// unmodified rather than through the processor).
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Set the bypass flag. The caller is responsible for ensuring this value
    /// arrived via the EventRing or ParameterBridge rather than being computed
    /// on the audio thread from non-RT-safe sources.
    pub fn set_bypassed(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Immutable access to the wrapped processor, e.g. for UI display.
    pub fn processor(&self) -> &P {
        &self.processor
    }

    /// Mutable access to the wrapped processor, e.g. to apply a parameter
    /// update already delivered across the RT boundary.
    pub fn processor_mut(&mut self) -> &mut P {
        &mut self.processor
    }

    /// Process one sample through this slot: a bypassed slot passes the
    /// signal through unchanged; an active slot runs it through the wrapped
    /// processor.
    ///
    /// Real-time safe: performs no allocation, locking, or I/O of its own and
    /// delegates directly to the wrapped processor's `process_sample`.
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        if self.bypassed {
            sample
        } else {
            self.processor.process_sample(sample)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct GainProcessor {
        gain: f32,
    }

    impl Processor for GainProcessor {
        fn process_sample(&mut self, sample: f32) -> f32 {
            sample * self.gain
        }

        fn name(&self) -> &str {
            "gain"
        }
    }

    #[test]
    fn active_slot_applies_processor() {
        let mut slot = EffectSlot::new(GainProcessor { gain: 2.0 }, false);
        assert_eq!(slot.process_sample(1.0), 2.0);
    }

    #[test]
    fn bypassed_slot_passes_signal_through_unchanged() {
        let mut slot = EffectSlot::new(GainProcessor { gain: 2.0 }, true);
        assert_eq!(slot.process_sample(1.0), 1.0);
    }

    #[test]
    fn set_bypassed_toggles_behavior() {
        let mut slot = EffectSlot::new(GainProcessor { gain: 3.0 }, false);
        assert_eq!(slot.process_sample(1.0), 3.0);

        slot.set_bypassed(true);
        assert_eq!(slot.process_sample(1.0), 1.0);

        slot.set_bypassed(false);
        assert_eq!(slot.process_sample(1.0), 3.0);
    }

    #[test]
    fn is_bypassed_reflects_construction_flag() {
        let slot = EffectSlot::new(GainProcessor { gain: 1.0 }, true);
        assert!(slot.is_bypassed());
    }

    #[test]
    fn processor_accessors_expose_wrapped_processor() {
        let mut slot = EffectSlot::new(GainProcessor { gain: 1.0 }, false);
        assert_eq!(slot.processor().name(), "gain");

        slot.processor_mut().gain = 5.0;
        assert_eq!(slot.process_sample(2.0), 10.0);
    }
}
