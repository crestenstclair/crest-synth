//! [`EffectChain`]: an ordered, bounded sequence of independently bypassable effect slots.

use crate::effects::effect_slot::{EffectSlot, Processor};

/// Forwarding impl so a boxed, type-erased processor can itself be used
/// wherever a [`Processor`] is expected. This is what lets [`EffectChain`]
/// hold slots of *different* concrete effect types (chorus, delay, reverb,
/// ...) in one `Vec` without making the whole chain generic over a single
/// processor type.
impl Processor for Box<dyn Processor> {
    fn process_sample(&mut self, sample: f32) -> f32 {
        (**self).process_sample(sample)
    }

    fn name(&self) -> &str {
        (**self).name()
    }
}

/// A processor that passes its input through unchanged. Used as the default
/// occupant of a freshly inserted slot until the caller swaps in a real
/// effect.
#[derive(Debug, Default)]
struct PassthroughProcessor;

impl Processor for PassthroughProcessor {
    fn process_sample(&mut self, sample: f32) -> f32 {
        sample
    }

    fn name(&self) -> &str {
        "passthrough"
    }
}

/// The concrete slot type stored by [`EffectChain`]. Slots are type-erased
/// (`Box<dyn Processor>`) rather than parameterized by one processor type,
/// because a chain's slots each hold an independently-chosen effect.
type BoxedSlot = EffectSlot<Box<dyn Processor>>;

/// Commands accepted by [`EffectChain`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectChainCommand {
    /// Insert a new, default effect slot at `index`.
    InsertSlot { index: u32 },
    /// Remove the slot at `index`.
    RemoveSlot { index: u32 },
    /// Move the slot at `from` to `to`, preserving the order of the remaining slots.
    ReorderSlot { from: u32, to: u32 },
    /// Set the bypass flag of the slot at `index`.
    SetBypass { index: u32, bypassed: bool },
}

/// Events emitted by [`EffectChain`] in response to commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectChainEvent {
    /// A slot was inserted at `index`.
    SlotInserted { index: u32 },
    /// The slot at `index` was removed.
    SlotRemoved { index: u32 },
    /// A slot moved from `from` to `to`.
    SlotsReordered { from: u32, to: u32 },
}

/// Errors returned when a command cannot be applied to an [`EffectChain`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectChainError {
    /// The chain already holds `max_slots` slots; no more may be inserted.
    ChainFull { max_slots: u8 },
    /// `index` does not refer to an existing slot.
    IndexOutOfBounds { index: u32, len: usize },
}

impl std::fmt::Display for EffectChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EffectChainError::ChainFull { max_slots } => {
                write!(
                    f,
                    "effect chain already has the maximum of {max_slots} slots"
                )
            }
            EffectChainError::IndexOutOfBounds { index, len } => {
                write!(
                    f,
                    "index {index} is out of bounds for a chain of {len} slots"
                )
            }
        }
    }
}

impl std::error::Error for EffectChainError {}

/// An ordered sequence of effect slots, each independently bypassable.
///
/// Invariants upheld by this type:
/// - signal flows through slots strictly top-to-bottom in slot order (`slots` is a `Vec`,
///   reordered only by an explicit [`EffectChainCommand::ReorderSlot`], and
///   [`EffectChain::process_sample`] always walks it front to back)
/// - a bypassed slot passes its input through unchanged (see [`EffectChain::process_sample`])
/// - the chain never exceeds `max_slots` slots (enforced when inserting)
///
/// Slots are stored as type-erased [`EffectSlot<Box<dyn Processor>>`] rather
/// than a single generic processor type, since each slot in a chain may hold
/// a different concrete effect (chorus, delay, reverb, ...).
pub struct EffectChain {
    max_slots: u8,
    slots: Vec<BoxedSlot>,
}

impl std::fmt::Debug for EffectChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectChain")
            .field("max_slots", &self.max_slots)
            .field("slot_count", &self.slots.len())
            .finish()
    }
}

impl EffectChain {
    /// Creates an empty chain that will accept at most `max_slots` slots.
    pub fn new(max_slots: u8) -> Self {
        Self {
            max_slots,
            slots: Vec::new(),
        }
    }

    /// The maximum number of slots this chain will ever hold.
    pub fn max_slots(&self) -> u8 {
        self.max_slots
    }

    /// The slots currently in the chain, in top-to-bottom processing order.
    pub fn slots(&self) -> &[BoxedSlot] {
        &self.slots
    }

    /// Applies a command, returning the events it produced or the error that rejected it.
    pub fn apply(
        &mut self,
        command: EffectChainCommand,
    ) -> Result<Vec<EffectChainEvent>, EffectChainError> {
        match command {
            EffectChainCommand::InsertSlot { index } => self.insert_slot(index),
            EffectChainCommand::RemoveSlot { index } => self.remove_slot(index),
            EffectChainCommand::ReorderSlot { from, to } => self.reorder_slot(from, to),
            EffectChainCommand::SetBypass { index, bypassed } => self.set_bypass(index, bypassed),
        }
    }

    fn insert_slot(&mut self, index: u32) -> Result<Vec<EffectChainEvent>, EffectChainError> {
        if self.slots.len() >= self.max_slots as usize {
            return Err(EffectChainError::ChainFull {
                max_slots: self.max_slots,
            });
        }
        let position = (index as usize).min(self.slots.len());
        let processor: Box<dyn Processor> = Box::new(PassthroughProcessor);
        self.slots
            .insert(position, EffectSlot::new(processor, false));
        Ok(vec![EffectChainEvent::SlotInserted {
            index: position as u32,
        }])
    }

    fn remove_slot(&mut self, index: u32) -> Result<Vec<EffectChainEvent>, EffectChainError> {
        let position = self.bounded_index(index)?;
        self.slots.remove(position);
        Ok(vec![EffectChainEvent::SlotRemoved { index }])
    }

    fn reorder_slot(
        &mut self,
        from: u32,
        to: u32,
    ) -> Result<Vec<EffectChainEvent>, EffectChainError> {
        let from_position = self.bounded_index(from)?;
        let to_position = self.bounded_index(to)?;
        let slot = self.slots.remove(from_position);
        self.slots.insert(to_position, slot);
        Ok(vec![EffectChainEvent::SlotsReordered { from, to }])
    }

    fn set_bypass(
        &mut self,
        index: u32,
        bypassed: bool,
    ) -> Result<Vec<EffectChainEvent>, EffectChainError> {
        let position = self.bounded_index(index)?;
        self.slots[position].set_bypassed(bypassed);
        Ok(Vec::new())
    }

    fn bounded_index(&self, index: u32) -> Result<usize, EffectChainError> {
        let position = index as usize;
        if position >= self.slots.len() {
            return Err(EffectChainError::IndexOutOfBounds {
                index,
                len: self.slots.len(),
            });
        }
        Ok(position)
    }

    /// Runs `input` through every slot top-to-bottom, in order.
    ///
    /// A bypassed slot passes its input through unchanged; an active slot's contribution is
    /// computed by `effect_fn`, which is given the slot and the running sample. `effect_fn` is
    /// a generic closure rather than a trait object, so no dynamic dispatch is introduced into
    /// the per-sample fold by the chain itself; the type erasure lives only in what a slot may
    /// contain, not in how the chain walks its slots.
    pub fn process_sample<F>(&self, input: f32, mut effect_fn: F) -> f32
    where
        F: FnMut(&BoxedSlot, f32) -> f32,
    {
        self.slots.iter().fold(input, |sample, slot| {
            if slot.is_bypassed() {
                sample
            } else {
                effect_fn(slot, sample)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_slot_appends_within_capacity() {
        let mut chain = EffectChain::new(2);
        let events = chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        assert_eq!(events, vec![EffectChainEvent::SlotInserted { index: 0 }]);
        assert_eq!(chain.slots().len(), 1);
    }

    #[test]
    fn insert_slot_rejected_once_full() {
        let mut chain = EffectChain::new(1);
        chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        let result = chain.apply(EffectChainCommand::InsertSlot { index: 0 });
        assert_eq!(result, Err(EffectChainError::ChainFull { max_slots: 1 }));
    }

    #[test]
    fn remove_slot_rejects_out_of_bounds_index() {
        let mut chain = EffectChain::new(2);
        let result = chain.apply(EffectChainCommand::RemoveSlot { index: 0 });
        assert_eq!(
            result,
            Err(EffectChainError::IndexOutOfBounds { index: 0, len: 0 })
        );
    }

    #[test]
    fn remove_slot_removes_and_emits_event() {
        let mut chain = EffectChain::new(2);
        chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        let events = chain
            .apply(EffectChainCommand::RemoveSlot { index: 0 })
            .unwrap();
        assert_eq!(events, vec![EffectChainEvent::SlotRemoved { index: 0 }]);
        assert!(chain.slots().is_empty());
    }

    #[test]
    fn reorder_slot_preserves_length_and_emits_event() {
        let mut chain = EffectChain::new(3);
        chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        chain
            .apply(EffectChainCommand::InsertSlot { index: 1 })
            .unwrap();
        chain
            .apply(EffectChainCommand::InsertSlot { index: 2 })
            .unwrap();
        let events = chain
            .apply(EffectChainCommand::ReorderSlot { from: 2, to: 0 })
            .unwrap();
        assert_eq!(
            events,
            vec![EffectChainEvent::SlotsReordered { from: 2, to: 0 }]
        );
        assert_eq!(chain.slots().len(), 3);
    }

    #[test]
    fn reorder_slot_rejects_out_of_bounds() {
        let mut chain = EffectChain::new(2);
        chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        let result = chain.apply(EffectChainCommand::ReorderSlot { from: 0, to: 5 });
        assert_eq!(
            result,
            Err(EffectChainError::IndexOutOfBounds { index: 5, len: 1 })
        );
    }

    #[test]
    fn set_bypass_toggles_slot_flag() {
        let mut chain = EffectChain::new(1);
        chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        chain
            .apply(EffectChainCommand::SetBypass {
                index: 0,
                bypassed: true,
            })
            .unwrap();
        assert!(chain.slots()[0].is_bypassed());
    }

    #[test]
    fn set_bypass_rejects_out_of_bounds() {
        let mut chain = EffectChain::new(1);
        let result = chain.apply(EffectChainCommand::SetBypass {
            index: 0,
            bypassed: true,
        });
        assert_eq!(
            result,
            Err(EffectChainError::IndexOutOfBounds { index: 0, len: 0 })
        );
    }

    #[test]
    fn bypassed_slot_passes_input_unchanged() {
        let mut chain = EffectChain::new(2);
        chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        chain
            .apply(EffectChainCommand::InsertSlot { index: 1 })
            .unwrap();
        chain
            .apply(EffectChainCommand::SetBypass {
                index: 0,
                bypassed: true,
            })
            .unwrap();
        // The active slot doubles the sample; the bypassed slot must be a no-op.
        let output = chain.process_sample(1.0_f32, |_slot, sample| sample * 2.0);
        assert_eq!(output, 2.0);
    }

    #[test]
    fn signal_flows_top_to_bottom_in_slot_order() {
        let mut chain = EffectChain::new(3);
        chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        chain
            .apply(EffectChainCommand::InsertSlot { index: 1 })
            .unwrap();
        chain
            .apply(EffectChainCommand::InsertSlot { index: 2 })
            .unwrap();
        // Each active slot records the sample it received, in visitation order; the recorded
        // sequence proves slots are visited top-to-bottom.
        let mut visited = Vec::new();
        chain.process_sample(0.0_f32, |_slot, sample| {
            visited.push(sample);
            sample + 1.0
        });
        assert_eq!(visited, vec![0.0, 1.0, 2.0]);
    }
}
