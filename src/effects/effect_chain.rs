// path: src/effects/effect_chain.rs

use crate::effects::effect_chain_id::EffectChainId;
use crate::effects::effect_processor::{EffectParams, EffectProcessor, EffectType};
use crate::effects::effect_slot::EffectSlot;
use crate::kernel::audio_frame::AudioFrame;

// ─── Commands ──────────────────────────────────────────────────────────────────────────────

/// Insert a new effect at the given position (0-based).
///
/// If `position` is greater than the current slot count the effect is
/// appended at the end.
pub struct AddEffect {
    pub effect_type: EffectType,
    pub position: u8,
}

/// Remove the effect at `slot_index`.
pub struct RemoveEffect {
    pub slot_index: u8,
}

/// Move the effect at `from_index` to `to_index`, shifting other effects
/// to fill the gap.
pub struct ReorderEffect {
    pub from_index: u8,
    pub to_index: u8,
}

/// Replace the parameters on the slot at `slot_index`.
pub struct UpdateEffectParams {
    pub slot_index: u8,
    pub params: EffectParams,
}

/// Bypass the entire chain (audio passes through unmodified).
pub struct BypassChain;

/// Re-enable the chain after bypass.
pub struct EnableChain;

// ─── Events ───────────────────────────────────────────────────────────────────────────────

/// Emitted when a new effect is added to the chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectAdded {
    pub effect_type: EffectType,
    pub position: u8,
}

/// Emitted when an effect is removed from the chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectRemoved {
    pub slot_index: u8,
}

/// Emitted when an effect is moved within the chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectReordered {
    pub from_index: u8,
    pub to_index: u8,
}

/// Emitted when an effect's parameters are changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectParamsUpdated {
    pub slot_index: u8,
}

/// Emitted when the whole chain is bypassed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChainBypassed {
    pub id: EffectChainId,
}

/// Emitted when the whole chain is re-enabled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChainEnabled {
    pub id: EffectChainId,
}

// ─── Errors ──────────────────────────────────────────────────────────────────────────────

/// Errors produced when a command cannot be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectChainError {
    /// The requested slot index is out of range.
    SlotIndexOutOfRange { index: u8, len: u8 },
}

impl std::fmt::Display for EffectChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EffectChainError::SlotIndexOutOfRange { index, len } => {
                write!(
                    f,
                    "slot index {index} is out of range (chain has {len} slots)"
                )
            }
        }
    }
}

// ─── Aggregate ────────────────────────────────────────────────────────────────────────────

/// An ordered list of effect slots processed in series.
///
/// # Signal flow
///
/// Audio passes through slot 0 first and exits slot N last:
///
/// ```text
/// input → slot[0] → slot[1] → … → slot[N] → output
/// ```
///
/// When `bypass` is `true` the input audio is returned unchanged — no
/// slots are executed.
///
/// # Audio-thread constraints
///
/// `process` performs no heap allocation after warm-up and acquires no locks.
/// Each `EffectProcessor` pre-allocates its delay buffer at the time the
/// slot is added; scratch buffers grow only when the block size increases.
#[derive(Debug)]
pub struct EffectChain {
    id: EffectChainId,
    bypass: bool,
    /// Configuration for each slot (parallel-indexed with `processors`).
    slots: Vec<EffectSlot>,
    /// Live DSP state for each slot (parallel-indexed with `slots`).
    processors: Vec<EffectProcessor>,
    /// Pre-allocated scratch buffer A for ping-pong processing across slots.
    scratch_a: Vec<AudioFrame>,
    /// Pre-allocated scratch buffer B for ping-pong processing across slots.
    scratch_b: Vec<AudioFrame>,
}

impl EffectChain {
    /// Creates a new, empty, non-bypassed `EffectChain`.
    pub fn new(id: EffectChainId) -> Self {
        Self {
            id,
            bypass: false,
            slots: Vec::new(),
            processors: Vec::new(),
            scratch_a: Vec::new(),
            scratch_b: Vec::new(),
        }
    }

    // ── Queries ──────────────────────────────────────────────────────────────────────

    /// Returns the chain's unique identifier.
    pub fn id(&self) -> EffectChainId {
        self.id
    }

    /// Returns `true` when the chain is bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    /// Returns an ordered slice of all effect slot configurations.
    ///
    /// Slot 0 is processed first; slot `len - 1` last.
    pub fn slots(&self) -> &[EffectSlot] {
        &self.slots
    }

    /// Returns the number of slots in the chain.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns `true` when the chain has no slots.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    // ── Commands ─────────────────────────────────────────────────────────────────────

    /// Insert a new effect at `position`.
    ///
    /// If `position` exceeds the current slot count the slot is appended.
    ///
    /// # Returns
    /// `EffectAdded` event on success.
    pub fn add_effect(&mut self, cmd: AddEffect) -> EffectAdded {
        let pos = (cmd.position as usize).min(self.slots.len());
        self.slots.insert(pos, EffectSlot::new(cmd.effect_type));
        self.processors.insert(pos, EffectProcessor::new());
        EffectAdded {
            effect_type: cmd.effect_type,
            position: pos as u8,
        }
    }

    /// Remove the slot at `slot_index`.
    ///
    /// # Errors
    /// Returns `SlotIndexOutOfRange` if the index is out of range.
    pub fn remove_effect(&mut self, cmd: RemoveEffect) -> Result<EffectRemoved, EffectChainError> {
        let idx = cmd.slot_index as usize;
        if idx >= self.slots.len() {
            return Err(EffectChainError::SlotIndexOutOfRange {
                index: cmd.slot_index,
                len: self.slots.len() as u8,
            });
        }
        self.slots.remove(idx);
        self.processors.remove(idx);
        Ok(EffectRemoved {
            slot_index: cmd.slot_index,
        })
    }

    /// Move the effect at `from_index` to `to_index`.
    ///
    /// # Errors
    /// Returns `SlotIndexOutOfRange` if either index is out of range.
    pub fn reorder_effect(
        &mut self,
        cmd: ReorderEffect,
    ) -> Result<EffectReordered, EffectChainError> {
        let from = cmd.from_index as usize;
        let to = cmd.to_index as usize;
        let len = self.slots.len();

        if from >= len {
            return Err(EffectChainError::SlotIndexOutOfRange {
                index: cmd.from_index,
                len: len as u8,
            });
        }
        if to >= len {
            return Err(EffectChainError::SlotIndexOutOfRange {
                index: cmd.to_index,
                len: len as u8,
            });
        }

        let slot = self.slots.remove(from);
        self.slots.insert(to, slot);
        let proc = self.processors.remove(from);
        self.processors.insert(to, proc);

        Ok(EffectReordered {
            from_index: cmd.from_index,
            to_index: cmd.to_index,
        })
    }

    /// Replace the parameters on the slot at `slot_index`.
    ///
    /// # Errors
    /// Returns `SlotIndexOutOfRange` if the index is out of range.
    pub fn update_effect_params(
        &mut self,
        cmd: UpdateEffectParams,
    ) -> Result<EffectParamsUpdated, EffectChainError> {
        let idx = cmd.slot_index as usize;
        if idx >= self.slots.len() {
            return Err(EffectChainError::SlotIndexOutOfRange {
                index: cmd.slot_index,
                len: self.slots.len() as u8,
            });
        }
        self.slots[idx].params = cmd.params;
        Ok(EffectParamsUpdated {
            slot_index: cmd.slot_index,
        })
    }

    /// Bypass the entire chain.
    ///
    /// While bypassed, `process` returns each input frame unchanged.
    pub fn bypass_chain(&mut self, _cmd: BypassChain) -> ChainBypassed {
        self.bypass = true;
        ChainBypassed { id: self.id }
    }

    /// Re-enable the chain after bypass.
    pub fn enable_chain(&mut self, _cmd: EnableChain) -> ChainEnabled {
        self.bypass = false;
        ChainEnabled { id: self.id }
    }

    // ── Audio processing ───────────────────────────────────────────────────────────────────

    /// Process a slice of `AudioFrame`s through the entire chain.
    ///
    /// Slots are applied in index order (slot 0 first, slot N last),
    /// satisfying the invariant *"effects process in slot order"*.
    ///
    /// When the chain is bypassed (`bypass_chain` was called) the input
    /// slice is copied directly into `output` unmodified, satisfying
    /// *"bypassed chain passes audio through unmodified"*.
    ///
    /// # Arguments
    ///
    /// * `frames` — input audio frames (not mutated).
    /// * `output` — caller-provided scratch buffer; must have at least
    ///   `frames.len()` elements. On return the first `frames.len()`
    ///   elements hold the processed audio.
    ///
    /// # Audio-thread safety
    ///
    /// - Zero heap allocation after warm-up (scratch buffers pre-grow on first large block).
    /// - Zero mutex / lock acquisition.
    /// - Zero blocking I/O.
    pub fn process(&mut self, frames: &[AudioFrame], output: &mut Vec<AudioFrame>) {
        let n = frames.len();

        // Ensure caller's output buffer is large enough (grows only during warm-up).
        if output.len() < n {
            output.resize(n, AudioFrame::silence());
        }

        if self.bypass || self.slots.is_empty() {
            output[..n].copy_from_slice(frames);
            return;
        }

        // Grow the internal scratch buffers if the block size increased.
        // After warm-up this branch is never taken.
        if self.scratch_a.len() < n {
            self.scratch_a.resize(n, AudioFrame::silence());
        }
        if self.scratch_b.len() < n {
            self.scratch_b.resize(n, AudioFrame::silence());
        }

        // Ping-pong between scratch_a and scratch_b.
        // scratch_a starts as input; each active slot reads from the current
        // source and writes into the other buffer via the processor's own
        // pre-allocated output buffer.  After the slot completes we copy
        // the processor output into the destination scratch buffer.
        self.scratch_a[..n].copy_from_slice(frames);

        // `use_a_as_input` tracks which scratch buffer holds the current signal.
        let mut use_a_as_input = true;

        for (slot, proc) in self.slots.iter().zip(self.processors.iter_mut()) {
            if slot.bypass {
                continue;
            }
            // proc.process() returns a reference into proc's internal Vec,
            // which is disjoint from both scratch buffers.
            let slot_out = if use_a_as_input {
                proc.process(&self.scratch_a[..n], slot.params)
            } else {
                proc.process(&self.scratch_b[..n], slot.params)
            };
            let len = slot_out.len().min(n);

            // Copy the result into the other scratch buffer.
            if use_a_as_input {
                self.scratch_b[..len].copy_from_slice(&slot_out[..len]);
            } else {
                self.scratch_a[..len].copy_from_slice(&slot_out[..len]);
            }
            use_a_as_input = !use_a_as_input;
        }

        // The final result is in whichever buffer was last written to.
        if use_a_as_input {
            // Last write was into scratch_a (no active slots flipped the flag).
            output[..n].copy_from_slice(&self.scratch_a[..n]);
        } else {
            output[..n].copy_from_slice(&self.scratch_b[..n]);
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chain() -> EffectChain {
        EffectChain::new(EffectChainId::new(1))
    }

    fn make_output(n: usize) -> Vec<AudioFrame> {
        vec![AudioFrame::silence(); n]
    }

    // ── add_effect ──────────────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_add_effect_appends_when_empty() {
        let mut chain = make_chain();
        let evt = chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        assert_eq!(chain.len(), 1);
        assert_eq!(evt.effect_type, EffectType::Gain);
        assert_eq!(evt.position, 0);
    }

    #[test]
    fn effect_chain_add_effect_inserts_at_position() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        chain.add_effect(AddEffect {
            effect_type: EffectType::LowPassFilter,
            position: 0,
        });
        // LPF now at 0, Gain at 1
        assert_eq!(chain.slots()[0].effect_type, EffectType::LowPassFilter);
        assert_eq!(chain.slots()[1].effect_type, EffectType::Gain);
    }

    #[test]
    fn effect_chain_add_effect_clamps_position_to_end() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 99,
        });
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.slots()[0].effect_type, EffectType::Gain);
    }

    // ── remove_effect ────────────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_remove_effect_removes_correct_slot() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        chain.add_effect(AddEffect {
            effect_type: EffectType::LowPassFilter,
            position: 1,
        });
        let evt = chain.remove_effect(RemoveEffect { slot_index: 0 }).unwrap();
        assert_eq!(evt.slot_index, 0);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.slots()[0].effect_type, EffectType::LowPassFilter);
    }

    #[test]
    fn effect_chain_remove_effect_out_of_range_returns_error() {
        let mut chain = make_chain();
        let err = chain
            .remove_effect(RemoveEffect { slot_index: 5 })
            .unwrap_err();
        assert_eq!(
            err,
            EffectChainError::SlotIndexOutOfRange { index: 5, len: 0 }
        );
    }

    // ── reorder_effect ───────────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_reorder_effect_moves_slot() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        chain.add_effect(AddEffect {
            effect_type: EffectType::LowPassFilter,
            position: 1,
        });
        chain.add_effect(AddEffect {
            effect_type: EffectType::Delay,
            position: 2,
        });
        // Move Gain (0) to position 2
        chain
            .reorder_effect(ReorderEffect {
                from_index: 0,
                to_index: 2,
            })
            .unwrap();
        assert_eq!(chain.slots()[0].effect_type, EffectType::LowPassFilter);
        assert_eq!(chain.slots()[1].effect_type, EffectType::Delay);
        assert_eq!(chain.slots()[2].effect_type, EffectType::Gain);
    }

    #[test]
    fn effect_chain_reorder_effect_out_of_range_returns_error() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        let err = chain
            .reorder_effect(ReorderEffect {
                from_index: 0,
                to_index: 5,
            })
            .unwrap_err();
        assert_eq!(
            err,
            EffectChainError::SlotIndexOutOfRange { index: 5, len: 1 }
        );
    }

    // ── update_effect_params ─────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_update_effect_params_updates_slot() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        let new_params = EffectParams {
            effect_type: EffectType::Gain,
            gain: 2.0,
            wet_mix: 1.0,
            ..EffectParams::default()
        };
        let evt = chain
            .update_effect_params(UpdateEffectParams {
                slot_index: 0,
                params: new_params,
            })
            .unwrap();
        assert_eq!(evt.slot_index, 0);
        assert_eq!(chain.slots()[0].params, new_params);
    }

    #[test]
    fn effect_chain_update_effect_params_out_of_range_returns_error() {
        let mut chain = make_chain();
        let err = chain
            .update_effect_params(UpdateEffectParams {
                slot_index: 0,
                params: EffectParams::default(),
            })
            .unwrap_err();
        assert_eq!(
            err,
            EffectChainError::SlotIndexOutOfRange { index: 0, len: 0 }
        );
    }

    // ── bypass / enable ───────────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_bypass_chain_sets_bypass_flag() {
        let mut chain = make_chain();
        assert!(!chain.is_bypassed());
        let evt = chain.bypass_chain(BypassChain);
        assert!(chain.is_bypassed());
        assert_eq!(evt.id, EffectChainId::new(1));
    }

    #[test]
    fn effect_chain_enable_chain_clears_bypass_flag() {
        let mut chain = make_chain();
        chain.bypass_chain(BypassChain);
        let evt = chain.enable_chain(EnableChain);
        assert!(!chain.is_bypassed());
        assert_eq!(evt.id, EffectChainId::new(1));
    }

    // ── process – slot order invariant ─────────────────────────────────────────────────────────────────

    /// Verify that slots are applied in order: slot 0 first, slot N last.
    ///
    /// Two Gain slots: each with gain=2.0 (linear) and wet_mix=1.0.
    /// In series the overall gain should be 2.0 × 2.0 = 4.0.
    #[test]
    fn effect_chain_process_slots_in_order() {
        let mut chain = make_chain();

        // Slot 0: gain ×2.0
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        chain
            .update_effect_params(UpdateEffectParams {
                slot_index: 0,
                params: EffectParams {
                    effect_type: EffectType::Gain,
                    gain: 2.0,
                    wet_mix: 1.0,
                    ..EffectParams::default()
                },
            })
            .unwrap();

        // Slot 1: gain ×2.0
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 1,
        });
        chain
            .update_effect_params(UpdateEffectParams {
                slot_index: 1,
                params: EffectParams {
                    effect_type: EffectType::Gain,
                    gain: 2.0,
                    wet_mix: 1.0,
                    ..EffectParams::default()
                },
            })
            .unwrap();

        let input = vec![AudioFrame::new(1.0, 1.0)];
        let mut output = make_output(1);
        chain.process(&input, &mut output);

        // Two ×2 gains applied in series → ×4
        assert!(
            (output[0].left - 4.0).abs() < 0.05,
            "left={}",
            output[0].left
        );
        assert!(
            (output[0].right - 4.0).abs() < 0.05,
            "right={}",
            output[0].right
        );
    }

    // ── process – bypassed chain passes through unmodified ───────────────────────────────────────

    #[test]
    fn effect_chain_bypass_passes_audio_through_unmodified() {
        let mut chain = make_chain();

        // Add a ×4 gain that would clearly amplify output if applied.
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        chain
            .update_effect_params(UpdateEffectParams {
                slot_index: 0,
                params: EffectParams {
                    effect_type: EffectType::Gain,
                    gain: 4.0,
                    wet_mix: 1.0,
                    ..EffectParams::default()
                },
            })
            .unwrap();

        chain.bypass_chain(BypassChain);

        let input = vec![AudioFrame::new(0.5, -0.5)];
        let mut output = make_output(1);
        chain.process(&input, &mut output);

        assert_eq!(output[0].left, 0.5);
        assert_eq!(output[0].right, -0.5);
    }

    #[test]
    fn effect_chain_process_active_after_enable() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        chain
            .update_effect_params(UpdateEffectParams {
                slot_index: 0,
                params: EffectParams {
                    effect_type: EffectType::Gain,
                    gain: 2.0,
                    wet_mix: 1.0,
                    ..EffectParams::default()
                },
            })
            .unwrap();

        chain.bypass_chain(BypassChain);
        chain.enable_chain(EnableChain);

        let input = vec![AudioFrame::new(1.0, 1.0)];
        let mut output = make_output(1);
        chain.process(&input, &mut output);
        assert!(
            (output[0].left - 2.0).abs() < 0.05,
            "left={}",
            output[0].left
        );
    }

    // ── process – empty chain ─────────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_empty_chain_passes_through() {
        let mut chain = make_chain();
        let input = vec![AudioFrame::new(0.3, -0.7)];
        let mut output = make_output(1);
        chain.process(&input, &mut output);
        assert_eq!(output[0].left, 0.3);
        assert_eq!(output[0].right, -0.7);
    }

    // ── per-slot bypass ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_per_slot_bypass_skips_slot() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        chain
            .update_effect_params(UpdateEffectParams {
                slot_index: 0,
                params: EffectParams {
                    effect_type: EffectType::Gain,
                    gain: 4.0,
                    wet_mix: 1.0,
                    ..EffectParams::default()
                },
            })
            .unwrap();
        // Bypass just the slot
        chain.slots[0].bypass = true;

        let input = vec![AudioFrame::new(0.5, 0.5)];
        let mut output = make_output(1);
        chain.process(&input, &mut output);
        // Slot bypassed → gain not applied
        assert_eq!(output[0].left, 0.5);
        assert_eq!(output[0].right, 0.5);
    }

    // ── multiple frames ─────────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_process_multiple_frames() {
        let mut chain = make_chain();
        chain.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        chain
            .update_effect_params(UpdateEffectParams {
                slot_index: 0,
                params: EffectParams {
                    effect_type: EffectType::Gain,
                    gain: 2.0,
                    wet_mix: 1.0,
                    ..EffectParams::default()
                },
            })
            .unwrap();

        let input = vec![
            AudioFrame::new(0.1, 0.2),
            AudioFrame::new(0.3, 0.4),
            AudioFrame::new(0.5, 0.6),
        ];
        let mut output = make_output(3);
        chain.process(&input, &mut output);

        assert!((output[0].left - 0.2).abs() < 1e-5);
        assert!((output[1].left - 0.6).abs() < 1e-5);
        assert!((output[2].left - 1.0).abs() < 1e-5);
    }
}
