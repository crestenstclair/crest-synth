// path: src/engine/steal_policy.rs

//! `StealPolicy` decides which active voice yields when polyphony is
//! exhausted and a new note demands a voice slot.
//!
//! This is a pure value object: no I/O, no allocation, no locking. It is
//! safe to read and copy on the real-time audio thread because it is a
//! plain `Copy` enum with no heap-backed state.

/// Which voice to reclaim when the voice pool has no free slots.
///
/// - `Oldest`: steal the voice that has been sounding the longest.
/// - `Quietest`: steal the voice with the lowest current output level.
/// - `LowestVelocity`: steal the voice whose note-on velocity was lowest.
/// - `Refuse`: do not steal; the incoming note is dropped instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StealPolicy {
    Oldest,
    Quietest,
    LowestVelocity,
    Refuse,
}

impl StealPolicy {
    /// All policy variants, in a stable order suitable for UI enumeration
    /// (e.g. cycling through choices with a gamepad D-pad).
    pub const ALL: [StealPolicy; 4] = [
        StealPolicy::Oldest,
        StealPolicy::Quietest,
        StealPolicy::LowestVelocity,
        StealPolicy::Refuse,
    ];

    /// Whether this policy permits stealing a voice at all.
    ///
    /// `Refuse` is the only variant that forbids stealing; every other
    /// variant just differs in *which* voice it picks.
    pub fn allows_stealing(self) -> bool {
        !matches!(self, StealPolicy::Refuse)
    }

    /// A short, human-readable label for the policy, suitable for display
    /// in a settings list.
    pub fn label(self) -> &'static str {
        match self {
            StealPolicy::Oldest => "Oldest",
            StealPolicy::Quietest => "Quietest",
            StealPolicy::LowestVelocity => "Lowest Velocity",
            StealPolicy::Refuse => "Refuse",
        }
    }

    /// Returns the next policy in `ALL`, wrapping around. Useful for
    /// gamepad-driven cycling through the available choices.
    pub fn next(self) -> StealPolicy {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// Returns the previous policy in `ALL`, wrapping around.
    pub fn previous(self) -> StealPolicy {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

impl Default for StealPolicy {
    /// `Oldest` is the conventional default voice-stealing behavior found
    /// on most polyphonic synthesizers.
    fn default() -> Self {
        StealPolicy::Oldest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_oldest() {
        assert_eq!(StealPolicy::default(), StealPolicy::Oldest);
    }

    #[test]
    fn refuse_does_not_allow_stealing() {
        assert!(!StealPolicy::Refuse.allows_stealing());
    }

    #[test]
    fn non_refuse_variants_allow_stealing() {
        assert!(StealPolicy::Oldest.allows_stealing());
        assert!(StealPolicy::Quietest.allows_stealing());
        assert!(StealPolicy::LowestVelocity.allows_stealing());
    }

    #[test]
    fn labels_are_distinct_and_non_empty() {
        let labels: Vec<&str> = StealPolicy::ALL.iter().map(|p| p.label()).collect();
        for label in &labels {
            assert!(!label.is_empty());
        }
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), labels.len());
    }

    #[test]
    fn next_cycles_through_all_variants_and_wraps() {
        let mut p = StealPolicy::Oldest;
        let mut seen = vec![p];
        for _ in 0..StealPolicy::ALL.len() - 1 {
            p = p.next();
            seen.push(p);
        }
        assert_eq!(seen, StealPolicy::ALL.to_vec());
        assert_eq!(p.next(), StealPolicy::Oldest);
    }

    #[test]
    fn previous_is_inverse_of_next() {
        for policy in StealPolicy::ALL {
            assert_eq!(policy.next().previous(), policy);
            assert_eq!(policy.previous().next(), policy);
        }
    }

    #[test]
    fn is_copy_and_plain_value_type() {
        let a = StealPolicy::Quietest;
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }
}
