// path: src/patch/voice_pool_config.rs

//! Per-patch voice pool sizing configuration.
//!
//! [`VoicePoolConfig`] specifies how many voices a patch may use simultaneously
//! and what stealing policy applies when the pool is exhausted.
//!
//! Each patch owns an independent pool so that one busy patch cannot starve
//! another.

// ─────────────────────────────────────────────────────────────────────────────
// StealingPolicy
// ─────────────────────────────────────────────────────────────────────────────

/// Determines which voice is reclaimed when the voice pool is exhausted.
///
/// Voice stealing is a real-time, lock-free operation: no allocation or
/// mutex acquisition is permitted during the steal decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StealingPolicy {
    /// Steal the voice that started playing the longest ago (oldest active note).
    OldestFirst,
    /// Steal the voice with the lowest current amplitude (quietest, least audible).
    QuietestFirst,
    /// Never steal a voice; if the pool is full, the new note is silently dropped.
    NoStealing,
}

// ─────────────────────────────────────────────────────────────────────────────
// VoicePoolConfig
// ─────────────────────────────────────────────────────────────────────────────

/// Per-patch voice pool sizing.
///
/// Controls how many simultaneous voices a patch may produce and what happens
/// when that limit is reached.
///
/// # Invariants
///
/// - `max_voices` must be at least 1.
///
/// # Examples
///
/// ```
/// use crest_synth::patch::voice_pool_config::{VoicePoolConfig, StealingPolicy};
///
/// let cfg = VoicePoolConfig::try_new(8, StealingPolicy::QuietestFirst).unwrap();
/// assert_eq!(cfg.max_voices(), 8);
/// assert_eq!(cfg.stealing_policy(), StealingPolicy::QuietestFirst);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoicePoolConfig {
    max_voices: u8,
    stealing_policy: StealingPolicy,
}

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Error returned when a [`VoicePoolConfig`] field violates an invariant.
#[derive(Debug, Clone, PartialEq)]
pub struct VoicePoolConfigError(String);

impl std::fmt::Display for VoicePoolConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VoicePoolConfig validation error: {}", self.0)
    }
}

impl std::error::Error for VoicePoolConfigError {}

// ─────────────────────────────────────────────────────────────────────────────
// VoicePoolConfig impl
// ─────────────────────────────────────────────────────────────────────────────

impl VoicePoolConfig {
    /// Construct a [`VoicePoolConfig`] with validation.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `max_voices` is `0`.
    ///
    /// ```
    /// use crest_synth::patch::voice_pool_config::{VoicePoolConfig, StealingPolicy};
    ///
    /// assert!(VoicePoolConfig::try_new(1, StealingPolicy::OldestFirst).is_ok());
    /// assert!(VoicePoolConfig::try_new(0, StealingPolicy::OldestFirst).is_err());
    /// ```
    pub fn try_new(
        max_voices: u8,
        stealing_policy: StealingPolicy,
    ) -> Result<Self, VoicePoolConfigError> {
        if max_voices == 0 {
            return Err(VoicePoolConfigError(
                "max_voices must be at least 1".to_owned(),
            ));
        }
        Ok(Self {
            max_voices,
            stealing_policy,
        })
    }

    /// Maximum number of simultaneous voices this patch may use.
    #[inline]
    pub fn max_voices(self) -> u8 {
        self.max_voices
    }

    /// The stealing policy applied when the pool is exhausted.
    #[inline]
    pub fn stealing_policy(self) -> StealingPolicy {
        self.stealing_policy
    }
}

impl Default for VoicePoolConfig {
    /// Returns an 8-voice pool with [`StealingPolicy::QuietestFirst`].
    fn default() -> Self {
        Self {
            max_voices: 8,
            stealing_policy: StealingPolicy::QuietestFirst,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn valid_config_one_voice_is_accepted() {
        let cfg = VoicePoolConfig::try_new(1, StealingPolicy::OldestFirst).unwrap();
        assert_eq!(cfg.max_voices(), 1);
        assert_eq!(cfg.stealing_policy(), StealingPolicy::OldestFirst);
    }

    #[test]
    fn valid_config_max_u8_voices_is_accepted() {
        let cfg = VoicePoolConfig::try_new(u8::MAX, StealingPolicy::NoStealing).unwrap();
        assert_eq!(cfg.max_voices(), u8::MAX);
    }

    #[test]
    fn zero_max_voices_is_rejected() {
        let err = VoicePoolConfig::try_new(0, StealingPolicy::QuietestFirst).unwrap_err();
        assert!(err.to_string().contains("max_voices must be at least 1"));
    }

    // ── Stealing policies ─────────────────────────────────────────────────────

    #[test]
    fn oldest_first_policy_is_stored() {
        let cfg = VoicePoolConfig::try_new(4, StealingPolicy::OldestFirst).unwrap();
        assert_eq!(cfg.stealing_policy(), StealingPolicy::OldestFirst);
    }

    #[test]
    fn quietest_first_policy_is_stored() {
        let cfg = VoicePoolConfig::try_new(4, StealingPolicy::QuietestFirst).unwrap();
        assert_eq!(cfg.stealing_policy(), StealingPolicy::QuietestFirst);
    }

    #[test]
    fn no_stealing_policy_is_stored() {
        let cfg = VoicePoolConfig::try_new(4, StealingPolicy::NoStealing).unwrap();
        assert_eq!(cfg.stealing_policy(), StealingPolicy::NoStealing);
    }

    // ── Default ───────────────────────────────────────────────────────────────

    #[test]
    fn default_is_valid() {
        let cfg = VoicePoolConfig::default();
        assert!(cfg.max_voices() >= 1);
        assert_eq!(cfg.max_voices(), 8);
        assert_eq!(cfg.stealing_policy(), StealingPolicy::QuietestFirst);
    }

    // ── Copy/Eq semantics ─────────────────────────────────────────────────────

    #[test]
    fn copy_semantics() {
        let a = VoicePoolConfig::default();
        let b = a; // Copy — a is still usable
        assert_eq!(a, b);
    }

    #[test]
    fn equality() {
        let a = VoicePoolConfig::try_new(4, StealingPolicy::OldestFirst).unwrap();
        let b = VoicePoolConfig::try_new(4, StealingPolicy::OldestFirst).unwrap();
        let c = VoicePoolConfig::try_new(8, StealingPolicy::OldestFirst).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ── Error Display ─────────────────────────────────────────────────────────

    #[test]
    fn error_display_contains_context() {
        let err = VoicePoolConfig::try_new(0, StealingPolicy::QuietestFirst).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("VoicePoolConfig validation error"));
        assert!(msg.contains("max_voices"));
    }
}
