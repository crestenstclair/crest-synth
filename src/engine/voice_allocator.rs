// path: src/engine/voice_allocator.rs

//! Assigns incoming notes to voices, stealing per the configured policy when polyphony is
//! exhausted. This domain service owns *which* `Voice` slot handles a given note; it never
//! bypasses `Voice`'s own state machine invariants (a voice can only be `trigger`ed while
//! idle). Stealing therefore works in two steps that respect that invariant:
//!
//! 1. When polyphony is exhausted, the allocator picks a victim slot per `StealPolicy`,
//!    begins releasing it (if it is not already releasing), and remembers the incoming note
//!    as a *pending* trigger for that slot.
//! 2. On a later call to `advance_all`, once the victim's amp envelope reaches `Idle`
//!    (`Voice::advance` returns `VoiceEvent::BecameIdle`), the allocator immediately
//!    triggers the pending note on that now-reclaimable slot.
//!
//! `allocate`, `release`, `apply_expression` and `advance_all` never allocate heap memory:
//! the voice pool is preallocated once in `VoiceAllocator::new` and never grows or shrinks
//! afterward, so these methods are safe to call from the audio thread's real-time callback.
//!
//! `StealPolicy` is the Engine context's canonical steal policy value object, defined once in
//! `crate::engine::steal_policy` and imported here rather than duplicated privately. Every
//! documented variant (`Oldest`, `Quietest`, `LowestVelocity`, `Refuse`) is observably honored
//! through `allocate`'s public behavior: `Refuse` declines the allocation when the pool is
//! full, while `Oldest`/`Quietest`/`LowestVelocity` each pick their respective victim.

pub use crate::engine::steal_policy::StealPolicy;
use crate::engine::voice::{
    AmpEnvelopeStage, NoteId, NoteNumber, Velocity, Voice, VoiceConfig, VoiceEvent,
};

/// Errors returned when a `VoiceAllocator` command cannot be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceAllocatorError {
    /// `VoiceAllocator::new` was asked to manage zero voices.
    ZeroPolyphony,
    /// Every voice already has a pending steal queued, so none is eligible to be chosen as a
    /// new steal victim.
    NoStealCandidate,
    /// No currently-active voice matches the given `NoteId`.
    NoteIdNotFound,
    /// The underlying `Voice` command was rejected even though the allocator's own
    /// bookkeeping expected it to succeed (defensive; should not occur in practice).
    VoiceCommandRejected,
    /// `allocate` was called under `StealPolicy::Refuse` while every voice was active; the
    /// allocator declines the new note instead of stealing a voice for it.
    Refused,
}

/// Outcome of a successful `VoiceAllocator::allocate` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceAssignment {
    /// The note was assigned immediately to an idle, reclaimable voice at `index`.
    Assigned { index: usize },
    /// Polyphony was exhausted; the voice at `index` (currently sounding `stolen_note_id`)
    /// was chosen as the steal victim, its release was started, and the new note is queued
    /// to trigger on that slot once it becomes idle (see `advance_all`).
    Stolen {
        index: usize,
        stolen_note_id: NoteId,
    },
}

/// A note queued to trigger on a slot as soon as its voice becomes idle.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PendingTrigger {
    note: NoteNumber,
    note_id: NoteId,
    velocity: Velocity,
}

/// One managed voice plus the allocator-owned bookkeeping needed to implement stealing
/// without modifying `Voice` itself: a monotonic trigger sequence (for the `Oldest` policy)
/// and an optional pending trigger (for deferred steal completion).
struct VoiceSlot {
    voice: Voice,
    sequence: u64,
    pending: Option<PendingTrigger>,
}

/// Assigns incoming notes to a fixed pool of voices, stealing per the configured policy when
/// polyphony is exhausted.
pub struct VoiceAllocator {
    slots: Vec<VoiceSlot>,
    steal_policy: StealPolicy,
    next_sequence: u64,
}

impl VoiceAllocator {
    /// Preallocates `polyphony` idle voices sharing `voice_config`. The voice pool is fixed
    /// for the lifetime of the allocator: no method after construction grows or shrinks it,
    /// which keeps `allocate`/`release`/`apply_expression`/`advance_all` free of heap
    /// allocation.
    pub fn new(
        voice_config: VoiceConfig,
        polyphony: usize,
        steal_policy: StealPolicy,
    ) -> Result<Self, VoiceAllocatorError> {
        if polyphony == 0 {
            return Err(VoiceAllocatorError::ZeroPolyphony);
        }

        let mut slots = Vec::with_capacity(polyphony);
        for _ in 0..polyphony {
            let voice = Voice::new(
                voice_config,
                NoteNumber::try_new(0).expect("0 is a valid MIDI note number"),
                NoteId::new(0),
                Velocity::try_new(0.0).expect("0.0 is a valid velocity"),
            );
            slots.push(VoiceSlot {
                voice,
                sequence: 0,
                pending: None,
            });
        }

        Ok(Self {
            slots,
            steal_policy,
            next_sequence: 0,
        })
    }

    /// Number of voices this allocator manages.
    pub fn polyphony(&self) -> usize {
        self.slots.len()
    }

    /// Number of voices that are currently sounding (not reclaimable).
    pub fn active_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| !slot.voice.is_reclaimable())
            .count()
    }

    /// Read-only access to a managed voice by slot index.
    pub fn voice(&self, index: usize) -> Option<&Voice> {
        self.slots.get(index).map(|slot| &slot.voice)
    }

    /// Assigns `note`/`note_id`/`velocity` to a voice: an idle voice if one is available,
    /// otherwise the `steal_policy` victim, whose new note is queued to trigger once it
    /// becomes idle (see `advance_all`). Under `StealPolicy::Refuse`, declines with
    /// `VoiceAllocatorError::Refused` instead of stealing once the pool is full.
    pub fn allocate(
        &mut self,
        note: NoteNumber,
        note_id: NoteId,
        velocity: Velocity,
    ) -> Result<VoiceAssignment, VoiceAllocatorError> {
        if let Some(index) = self.find_reclaimable() {
            self.trigger_slot(index, note, note_id, velocity);
            return Ok(VoiceAssignment::Assigned { index });
        }

        if self.steal_policy == StealPolicy::Refuse {
            return Err(VoiceAllocatorError::Refused);
        }

        let index = self
            .choose_steal_candidate()
            .ok_or(VoiceAllocatorError::NoStealCandidate)?;
        let stolen_note_id = self.slots[index].voice.note_id();

        // Begin releasing the victim now unless it is already releasing -- calling
        // `release` again would reset its release timer, and `Voice` never lets its amp
        // envelope skip stages, so this is the only way to move it toward `Idle`.
        if self.slots[index].voice.amp_stage() != AmpEnvelopeStage::Release {
            let _ = self.slots[index].voice.release(stolen_note_id);
        }
        self.slots[index].pending = Some(PendingTrigger {
            note,
            note_id,
            velocity,
        });

        Ok(VoiceAssignment::Stolen {
            index,
            stolen_note_id,
        })
    }

    /// Releases the active voice matching `note_id`, if any.
    pub fn release(&mut self, note_id: NoteId) -> Result<VoiceEvent, VoiceAllocatorError> {
        let index = self.find_active_by_note_id(note_id)?;
        self.slots[index]
            .voice
            .release(note_id)
            .map_err(|_| VoiceAllocatorError::VoiceCommandRejected)
    }

    /// Forwards per-note expression to the active voice matching `note_id`, if any.
    pub fn apply_expression(
        &mut self,
        note_id: NoteId,
        pitch_bend: f64,
        pressure: f64,
        slide: f64,
    ) -> Result<(), VoiceAllocatorError> {
        let index = self.find_active_by_note_id(note_id)?;
        self.slots[index]
            .voice
            .apply_expression(note_id, pitch_bend, pressure, slide)
            .map_err(|_| VoiceAllocatorError::VoiceCommandRejected)
    }

    /// Advances every managed voice's amp envelope by `dt_seconds`. `on_event` is invoked
    /// with the slot index and each `VoiceEvent` a voice emits, including a synthetic
    /// `Triggered` event when a queued steal completes because its victim just reached
    /// `Idle`. Takes a caller-supplied callback rather than returning a `Vec` so this method
    /// never allocates on the (likely real-time) calling thread.
    pub fn advance_all(&mut self, dt_seconds: f64, mut on_event: impl FnMut(usize, VoiceEvent)) {
        for index in 0..self.slots.len() {
            let event = match self.slots[index].voice.advance(dt_seconds) {
                Some(event) => event,
                None => continue,
            };
            on_event(index, event);

            if let VoiceEvent::BecameIdle { .. } = event {
                if let Some(pending) = self.slots[index].pending.take() {
                    if let Ok(triggered) = self.slots[index].voice.trigger(
                        pending.note,
                        pending.note_id,
                        pending.velocity,
                    ) {
                        self.next_sequence += 1;
                        self.slots[index].sequence = self.next_sequence;
                        on_event(index, triggered);
                    }
                }
            }
        }
    }

    fn find_reclaimable(&self) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| slot.pending.is_none() && slot.voice.is_reclaimable())
    }

    fn find_active_by_note_id(&self, note_id: NoteId) -> Result<usize, VoiceAllocatorError> {
        self.slots
            .iter()
            .position(|slot| !slot.voice.is_reclaimable() && slot.voice.note_id() == note_id)
            .ok_or(VoiceAllocatorError::NoteIdNotFound)
    }

    fn choose_steal_candidate(&self) -> Option<usize> {
        let candidates = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.pending.is_none());

        match self.steal_policy {
            StealPolicy::Oldest => candidates
                .min_by_key(|(_, slot)| slot.sequence)
                .map(|(i, _)| i),
            StealPolicy::Quietest => candidates
                .min_by(|(_, a), (_, b)| {
                    a.voice
                        .amp_level()
                        .partial_cmp(&b.voice.amp_level())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i),
            StealPolicy::LowestVelocity => candidates
                .min_by(|(_, a), (_, b)| {
                    a.voice
                        .velocity()
                        .value()
                        .partial_cmp(&b.voice.velocity().value())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i),
            // `allocate` returns `VoiceAllocatorError::Refused` before ever calling this
            // method under `StealPolicy::Refuse`; this arm exists only for exhaustiveness.
            StealPolicy::Refuse => None,
        }
    }

    fn trigger_slot(
        &mut self,
        index: usize,
        note: NoteNumber,
        note_id: NoteId,
        velocity: Velocity,
    ) {
        // Reclaimable voices are always idle, so `trigger` cannot fail here.
        if self.slots[index]
            .voice
            .trigger(note, note_id, velocity)
            .is_ok()
        {
            self.next_sequence += 1;
            self.slots[index].sequence = self.next_sequence;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::voice::EnvelopeTiming;

    fn slow_config() -> VoiceConfig {
        VoiceConfig::new(EnvelopeTiming::new(1.0, 1.0, 0.5, 1.0))
    }

    fn fast_config() -> VoiceConfig {
        VoiceConfig::new(EnvelopeTiming::new(0.1, 0.1, 0.5, 0.2))
    }

    fn note(value: u8) -> NoteNumber {
        NoteNumber::try_new(value).unwrap()
    }

    fn velocity(value: f64) -> Velocity {
        Velocity::try_new(value).unwrap()
    }

    #[test]
    fn new_rejects_zero_polyphony() {
        let result = VoiceAllocator::new(fast_config(), 0, StealPolicy::Oldest);
        assert_eq!(result.err(), Some(VoiceAllocatorError::ZeroPolyphony));
    }

    #[test]
    fn allocate_assigns_first_idle_voice() {
        let mut allocator = VoiceAllocator::new(fast_config(), 2, StealPolicy::Oldest).unwrap();

        let assignment = allocator
            .allocate(note(60), NoteId::new(1), velocity(0.8))
            .unwrap();

        assert_eq!(assignment, VoiceAssignment::Assigned { index: 0 });
        assert_eq!(allocator.active_count(), 1);
        assert_eq!(allocator.voice(0).unwrap().note_id(), NoteId::new(1));
    }

    #[test]
    fn allocate_fills_all_voices_before_stealing() {
        let mut allocator = VoiceAllocator::new(fast_config(), 2, StealPolicy::Oldest).unwrap();

        let first = allocator
            .allocate(note(60), NoteId::new(1), velocity(0.8))
            .unwrap();
        let second = allocator
            .allocate(note(64), NoteId::new(2), velocity(0.8))
            .unwrap();

        assert_eq!(first, VoiceAssignment::Assigned { index: 0 });
        assert_eq!(second, VoiceAssignment::Assigned { index: 1 });
        assert_eq!(allocator.active_count(), 2);
    }

    #[test]
    fn allocate_when_full_steals_oldest_and_starts_releasing_it() {
        let mut allocator = VoiceAllocator::new(slow_config(), 2, StealPolicy::Oldest).unwrap();

        allocator
            .allocate(note(60), NoteId::new(1), velocity(0.8))
            .unwrap();
        allocator.advance_all(0.5, |_, _| {});
        allocator
            .allocate(note(64), NoteId::new(2), velocity(0.8))
            .unwrap();

        let assignment = allocator
            .allocate(note(67), NoteId::new(3), velocity(0.8))
            .unwrap();

        assert_eq!(
            assignment,
            VoiceAssignment::Stolen {
                index: 0,
                stolen_note_id: NoteId::new(1),
            }
        );
        assert_eq!(
            allocator.voice(0).unwrap().amp_stage(),
            AmpEnvelopeStage::Release
        );
    }

    #[test]
    fn allocate_when_full_steals_quietest_even_if_not_oldest() {
        let mut allocator = VoiceAllocator::new(slow_config(), 2, StealPolicy::Quietest).unwrap();

        // Voice 0: triggered, then advanced -- louder, and older.
        allocator
            .allocate(note(60), NoteId::new(1), velocity(0.8))
            .unwrap();
        allocator.advance_all(0.5, |_, _| {});
        // Voice 1: triggered just now -- quieter, but newer.
        allocator
            .allocate(note(64), NoteId::new(2), velocity(0.8))
            .unwrap();

        assert!(allocator.voice(0).unwrap().amp_level() > allocator.voice(1).unwrap().amp_level());

        let assignment = allocator
            .allocate(note(67), NoteId::new(3), velocity(0.8))
            .unwrap();

        assert_eq!(
            assignment,
            VoiceAssignment::Stolen {
                index: 1,
                stolen_note_id: NoteId::new(2),
            }
        );
    }

    #[test]
    fn allocate_when_full_steals_lowest_velocity_even_if_not_oldest_or_quietest() {
        let mut allocator =
            VoiceAllocator::new(fast_config(), 2, StealPolicy::LowestVelocity).unwrap();

        // Voice 0: triggered first (oldest) and loudest note-on velocity.
        allocator
            .allocate(note(60), NoteId::new(1), velocity(0.9))
            .unwrap();
        // Voice 1: triggered second (newer) but quietest note-on velocity.
        allocator
            .allocate(note(64), NoteId::new(2), velocity(0.2))
            .unwrap();

        let assignment = allocator
            .allocate(note(67), NoteId::new(3), velocity(0.8))
            .unwrap();

        assert_eq!(
            assignment,
            VoiceAssignment::Stolen {
                index: 1,
                stolen_note_id: NoteId::new(2),
            }
        );
    }

    #[test]
    fn allocate_with_refuse_policy_declines_when_full() {
        let mut allocator = VoiceAllocator::new(fast_config(), 1, StealPolicy::Refuse).unwrap();
        allocator
            .allocate(note(60), NoteId::new(1), velocity(0.8))
            .unwrap();

        let result = allocator.allocate(note(64), NoteId::new(2), velocity(0.8));

        assert_eq!(result.err(), Some(VoiceAllocatorError::Refused));
        assert_eq!(allocator.active_count(), 1);
        assert_eq!(allocator.voice(0).unwrap().note_id(), NoteId::new(1));
    }

    #[test]
    fn steal_victim_differs_across_policies_for_the_same_pool() {
        // A pool where age, current loudness, and note-on velocity each single out a
        // different voice, built identically except for the configured `StealPolicy`.
        let build = |policy: StealPolicy| {
            let mut allocator = VoiceAllocator::new(slow_config(), 3, policy).unwrap();
            // Voice 0: oldest, loudest once advanced, highest velocity.
            allocator
                .allocate(note(60), NoteId::new(1), velocity(0.9))
                .unwrap();
            allocator.advance_all(0.5, |_, _| {});
            // Voice 1: newer than 0, mid velocity.
            allocator
                .allocate(note(64), NoteId::new(2), velocity(0.5))
                .unwrap();
            // Voice 2: newest, lowest velocity.
            allocator
                .allocate(note(67), NoteId::new(3), velocity(0.1))
                .unwrap();
            allocator.advance_all(0.2, |_, _| {});
            allocator
        };

        let mut oldest = build(StealPolicy::Oldest);
        let mut quietest = build(StealPolicy::Quietest);
        let mut lowest_velocity = build(StealPolicy::LowestVelocity);

        let steal_victim = |allocator: &mut VoiceAllocator| match allocator
            .allocate(note(70), NoteId::new(4), velocity(0.8))
            .unwrap()
        {
            VoiceAssignment::Stolen { stolen_note_id, .. } => stolen_note_id,
            other => panic!("expected a steal, got {other:?}"),
        };

        let oldest_victim = steal_victim(&mut oldest);
        let quietest_victim = steal_victim(&mut quietest);
        let lowest_velocity_victim = steal_victim(&mut lowest_velocity);

        assert_eq!(oldest_victim, NoteId::new(1));
        assert_eq!(quietest_victim, NoteId::new(2));
        assert_eq!(lowest_velocity_victim, NoteId::new(3));
        assert_ne!(oldest_victim, quietest_victim);
        assert_ne!(quietest_victim, lowest_velocity_victim);
        assert_ne!(oldest_victim, lowest_velocity_victim);
    }

    #[test]
    fn advance_all_completes_pending_steal_once_victim_becomes_idle() {
        let mut allocator = VoiceAllocator::new(fast_config(), 1, StealPolicy::Oldest).unwrap();

        allocator
            .allocate(note(60), NoteId::new(1), velocity(0.8))
            .unwrap();
        let assignment = allocator
            .allocate(note(64), NoteId::new(2), velocity(0.8))
            .unwrap();
        assert_eq!(
            assignment,
            VoiceAssignment::Stolen {
                index: 0,
                stolen_note_id: NoteId::new(1),
            }
        );

        // release_seconds = 0.2 for fast_config; advance past it to reach Idle and complete
        // the queued steal.
        let mut events = Vec::new();
        allocator.advance_all(0.25, |index, event| events.push((index, event)));

        assert!(events.contains(&(
            0,
            VoiceEvent::BecameIdle {
                note_id: NoteId::new(1)
            }
        )));
        assert!(events.contains(&(
            0,
            VoiceEvent::Triggered {
                note_id: NoteId::new(2)
            }
        )));
        assert_eq!(allocator.voice(0).unwrap().note_id(), NoteId::new(2));
        assert_eq!(
            allocator.voice(0).unwrap().amp_stage(),
            AmpEnvelopeStage::Attack
        );
    }

    #[test]
    fn release_forwards_to_active_voice_matching_note_id() {
        let mut allocator = VoiceAllocator::new(fast_config(), 1, StealPolicy::Oldest).unwrap();
        let note_id = NoteId::new(1);
        allocator
            .allocate(note(60), note_id, velocity(0.8))
            .unwrap();

        let event = allocator.release(note_id).unwrap();

        assert_eq!(event, VoiceEvent::Released { note_id });
        assert_eq!(
            allocator.voice(0).unwrap().amp_stage(),
            AmpEnvelopeStage::Release
        );
    }

    #[test]
    fn release_errors_when_note_id_is_not_active() {
        let mut allocator = VoiceAllocator::new(fast_config(), 1, StealPolicy::Oldest).unwrap();
        allocator
            .allocate(note(60), NoteId::new(1), velocity(0.8))
            .unwrap();

        let result = allocator.release(NoteId::new(999));

        assert_eq!(result, Err(VoiceAllocatorError::NoteIdNotFound));
    }

    #[test]
    fn apply_expression_forwards_to_active_voice_matching_note_id() {
        let mut allocator = VoiceAllocator::new(fast_config(), 1, StealPolicy::Oldest).unwrap();
        let note_id = NoteId::new(1);
        allocator
            .allocate(note(60), note_id, velocity(0.8))
            .unwrap();

        allocator
            .apply_expression(note_id, 0.25, 0.6, -0.1)
            .unwrap();

        let voice = allocator.voice(0).unwrap();
        assert_eq!(voice.pitch_bend(), 0.25);
        assert_eq!(voice.pressure(), 0.6);
        assert_eq!(voice.slide(), -0.1);
    }

    #[test]
    fn apply_expression_errors_when_note_id_is_not_active() {
        let mut allocator = VoiceAllocator::new(fast_config(), 1, StealPolicy::Oldest).unwrap();
        allocator
            .allocate(note(60), NoteId::new(1), velocity(0.8))
            .unwrap();

        let result = allocator.apply_expression(NoteId::new(999), 0.1, 0.1, 0.1);

        assert_eq!(result, Err(VoiceAllocatorError::NoteIdNotFound));
    }
}
