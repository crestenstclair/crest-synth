// path: src/effects/chain_renderer.rs

//! [`ChainRenderer`]: the domain service that runs an audio buffer through
//! every non-bypassed slot of an [`EffectChain`], in order.
//!
//! `EffectChain` only records *which* slot exists and whether it is
//! bypassed (see [`crate::effects::effect_slot::EffectSlot`]); it does not
//! own the [`EffectProcessor`] instances that actually transform samples.
//! `ChainRenderer` is handed those instances by its caller (one per slot,
//! positionally aligned with `chain.slots()`) rather than constructing or
//! owning them itself — the processors are a caller-supplied dependency,
//! not something this service instantiates.

use crate::effects::effect_chain::EffectChain;
use crate::effects::effect_processor::{AudioFrame, EffectProcessor};

/// Errors that prevent [`ChainRenderer::render`] from running a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainRenderError {
    /// The number of processors supplied does not match the number of
    /// slots in the chain, so slots cannot be aligned with processors.
    ProcessorCountMismatch {
        /// Number of slots in the chain.
        slots: usize,
        /// Number of processors supplied.
        processors: usize,
    },
}

impl std::fmt::Display for ChainRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainRenderError::ProcessorCountMismatch { slots, processors } => write!(
                f,
                "chain has {slots} slot(s) but {processors} processor(s) were supplied"
            ),
        }
    }
}

impl std::error::Error for ChainRenderError {}

/// Renders an [`EffectChain`] by processing a buffer through every
/// non-bypassed slot in top-to-bottom order.
///
/// Invariants upheld by this type:
/// - signal flows through slots strictly top-to-bottom in slot order
///   (`render` walks `chain.slots()` front to back, in lockstep with the
///   supplied processors)
/// - a bypassed slot passes its input through unchanged (a bypassed slot's
///   processor is skipped entirely; the buffer flows to the next slot
///   untouched)
/// - insert chains process slots strictly in order with no feedback loops
///   within a chain (each slot consumes exactly the previous slot's output
///   and nothing is fed back upstream)
///
/// This service holds no state of its own and instantiates no processors:
/// the [`EffectProcessor`] for each slot is supplied by the caller,
/// keeping `ChainRenderer` decoupled from how effects are constructed or
/// looked up (dependency inversion — it depends on the `EffectProcessor`
/// abstraction, never on a concrete DSP implementation).
#[derive(Debug, Default, Clone, Copy)]
pub struct ChainRenderer;

impl ChainRenderer {
    /// Constructs a new, stateless chain renderer.
    pub fn new() -> Self {
        Self
    }

    /// Processes `input` through every non-bypassed slot of `chain`, in
    /// order, returning the fully-rendered buffer.
    ///
    /// `processors` must have exactly one entry per slot in `chain`,
    /// positionally aligned with `chain.slots()`: `processors[i]` is the
    /// effect that occupies `chain.slots()[i]`. A bypassed slot's
    /// processor is not invoked; the buffer simply passes through
    /// unchanged for that slot.
    ///
    /// # Errors
    ///
    /// Returns [`ChainRenderError::ProcessorCountMismatch`] if
    /// `processors.len() != chain.slots().len()`.
    pub fn render(
        &self,
        chain: &EffectChain,
        processors: &mut [Box<dyn EffectProcessor>],
        input: &[AudioFrame],
    ) -> Result<Vec<AudioFrame>, ChainRenderError> {
        let slots = chain.slots();
        if processors.len() != slots.len() {
            return Err(ChainRenderError::ProcessorCountMismatch {
                slots: slots.len(),
                processors: processors.len(),
            });
        }

        let mut buffer = input.to_vec();
        for (slot, processor) in slots.iter().zip(processors.iter_mut()) {
            if slot.is_bypassed() {
                continue;
            }
            buffer = processor.process(&buffer);
        }
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::effect_chain::EffectChainCommand;
    use crate::effects::effect_processor::{GainEffect, OneSampleDelay, PassthroughEffect};

    fn frames(samples: &[(f32, f32)]) -> Vec<AudioFrame> {
        samples
            .iter()
            .map(|&(l, r)| AudioFrame::new(l, r))
            .collect()
    }

    fn chain_with_slots(count: u32) -> EffectChain {
        let mut chain = EffectChain::new(count as u8);
        for index in 0..count {
            chain
                .apply(EffectChainCommand::InsertSlot { index })
                .unwrap();
        }
        chain
    }

    #[test]
    fn empty_chain_returns_input_unchanged() {
        let chain = chain_with_slots(0);
        let renderer = ChainRenderer::new();
        let input = frames(&[(0.1, -0.1), (0.2, -0.2)]);

        let output = renderer.render(&chain, &mut [], &input).unwrap();

        assert_eq!(output, input);
    }

    #[test]
    fn active_slots_apply_in_top_to_bottom_order() {
        let chain = chain_with_slots(2);
        let renderer = ChainRenderer::new();
        let mut processors: Vec<Box<dyn EffectProcessor>> = vec![
            Box::new(GainEffect::new(2.0)),
            Box::new(GainEffect::new(3.0)),
        ];
        let input = frames(&[(1.0, 1.0)]);

        let output = renderer.render(&chain, &mut processors, &input).unwrap();

        // 1.0 * 2.0 * 3.0 == 6.0; the non-commutative case below (delay
        // ordering) proves order more strongly than this commutative one.
        assert_eq!(output, frames(&[(6.0, 6.0)]));
    }

    #[test]
    fn bypassed_slot_passes_input_through_unchanged() {
        let mut chain = chain_with_slots(2);
        chain
            .apply(EffectChainCommand::SetBypass {
                index: 0,
                bypassed: true,
            })
            .unwrap();
        let renderer = ChainRenderer::new();
        let mut processors: Vec<Box<dyn EffectProcessor>> = vec![
            Box::new(GainEffect::new(100.0)),
            Box::new(GainEffect::new(2.0)),
        ];
        let input = frames(&[(1.0, 1.0)]);

        let output = renderer.render(&chain, &mut processors, &input).unwrap();

        // The bypassed 100x gain must be skipped; only the active 2x gain applies.
        assert_eq!(output, frames(&[(2.0, 2.0)]));
    }

    #[test]
    fn all_slots_bypassed_returns_input_unchanged() {
        let mut chain = chain_with_slots(1);
        chain
            .apply(EffectChainCommand::SetBypass {
                index: 0,
                bypassed: true,
            })
            .unwrap();
        let renderer = ChainRenderer::new();
        let mut processors: Vec<Box<dyn EffectProcessor>> = vec![Box::new(GainEffect::new(9.0))];
        let input = frames(&[(0.4, -0.4)]);

        let output = renderer.render(&chain, &mut processors, &input).unwrap();

        assert_eq!(output, input);
    }

    #[test]
    fn slot_order_is_respected_for_non_commutative_effects() {
        let chain = chain_with_slots(2);
        let renderer = ChainRenderer::new();
        // Delay-then-gain differs from gain-then-delay because the delay
        // carries whatever sample preceded it; this proves slots run
        // top-to-bottom rather than in some other order.
        let mut processors: Vec<Box<dyn EffectProcessor>> = vec![
            Box::new(OneSampleDelay::new()),
            Box::new(GainEffect::new(10.0)),
        ];
        let input = frames(&[(1.0, 1.0), (2.0, 2.0)]);

        let output = renderer.render(&chain, &mut processors, &input).unwrap();

        // OneSampleDelay first: [0.0, 1.0]; then Gain(10): [0.0, 10.0].
        assert_eq!(output, frames(&[(0.0, 0.0), (10.0, 10.0)]));
    }

    #[test]
    fn processor_count_mismatch_is_rejected() {
        let chain = chain_with_slots(2);
        let renderer = ChainRenderer::new();
        let mut processors: Vec<Box<dyn EffectProcessor>> =
            vec![Box::new(PassthroughEffect::new())];
        let input = frames(&[(1.0, 1.0)]);

        let result = renderer.render(&chain, &mut processors, &input);

        assert_eq!(
            result,
            Err(ChainRenderError::ProcessorCountMismatch {
                slots: 2,
                processors: 1,
            })
        );
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let chain = chain_with_slots(1);
        let renderer = ChainRenderer::new();
        let mut processors: Vec<Box<dyn EffectProcessor>> =
            vec![Box::new(PassthroughEffect::new())];

        let output = renderer.render(&chain, &mut processors, &[]).unwrap();

        assert!(output.is_empty());
    }

    #[test]
    fn chain_render_error_display_is_human_readable() {
        let err = ChainRenderError::ProcessorCountMismatch {
            slots: 2,
            processors: 1,
        };
        assert_eq!(
            err.to_string(),
            "chain has 2 slot(s) but 1 processor(s) were supplied"
        );
    }
}
