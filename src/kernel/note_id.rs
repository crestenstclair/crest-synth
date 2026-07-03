// path: src/kernel/note_id.rs

//! `NoteId` — a unique identifier assigned to each sounding note.
//!
//! A `NoteId` is issued by the non-real-time world (the voice allocator)
//! when a note begins sounding, and travels across the thread boundary via
//! the `EventRing` so the audio thread can correlate note-on/note-off pairs
//! and per-note modulation (e.g. MPE) without needing to re-derive identity
//! from channel/key alone. It carries no behavior of its own beyond identity
//! comparison — a plain newtype over `u32`.

/// A unique identifier for a sounding note.
///
/// `NoteId` values are opaque; callers should not assume any relationship
/// between the wrapped `u32` and MIDI channel, key, or velocity. Uniqueness
/// is scoped to "currently sounding notes" — an allocator may reuse a value
/// once the note it identified has fully released, but never while two
/// notes could be confused for one another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteId(u32);

impl NoteId {
    /// Constructs a `NoteId` from a raw `u32`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::kernel::note_id::NoteId;
    ///
    /// let id = NoteId::new(42);
    /// assert_eq!(id.value(), 42);
    /// ```
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the wrapped raw value.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl From<u32> for NoteId {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<NoteId> for u32 {
    fn from(id: NoteId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_the_raw_value() {
        let id = NoteId::new(7);
        assert_eq!(id.value(), 7);
    }

    #[test]
    fn from_u32_round_trips() {
        let id: NoteId = 99u32.into();
        assert_eq!(id.value(), 99);

        let raw: u32 = id.into();
        assert_eq!(raw, 99);
    }

    #[test]
    fn equality_is_by_value() {
        assert_eq!(NoteId::new(3), NoteId::new(3));
        assert_ne!(NoteId::new(3), NoteId::new(4));
    }

    #[test]
    fn ordering_matches_wrapped_value() {
        assert!(NoteId::new(1) < NoteId::new(2));
    }

    #[test]
    fn copy_semantics_allow_reuse_after_move() {
        let id = NoteId::new(5);
        let copied = id;
        assert_eq!(id, copied);
    }

    #[test]
    fn zero_and_max_are_valid_ids() {
        assert_eq!(NoteId::new(0).value(), 0);
        assert_eq!(NoteId::new(u32::MAX).value(), u32::MAX);
    }
}
