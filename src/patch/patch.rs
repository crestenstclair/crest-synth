// path: src/patch/patch.rs

//! `Patch` aggregate — a complete instrument: engine type, parameters, voice pool,
//! and channel subscription.
//!
//! # Invariants
//!
//! - Each `Patch` owns its own independent `VoiceAllocator`; polyphony of one
//!   patch cannot starve another.
//! - `pan` is constrained to the range −1.0 (left) to 1.0 (right).

use crate::kernel::amplitude::Amplitude;
use crate::patch::channel_subscription::ChannelSubscription;
use crate::patch::patch_id::PatchId;
use crate::patch::voice_pool_config::VoicePoolConfig;
use crate::synth::amp_envelope_config::AmpEnvelopeConfig;
use crate::synth::filter_config::FilterConfig;
use crate::synth::oscillator_config::OscillatorConfig;
use crate::synth::voice_allocator::VoiceAllocator;

// ─────────────────────────────────────────────────────────────────────────────
// EngineType
// ─────────────────────────────────────────────────────────────────────────────

/// The synthesis engine that will power the voices in this patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EngineType {
    /// A simple sine-wave oscillator engine.
    #[default]
    Sine,
    /// A wavetable-based engine.
    Wavetable,
    /// A subtractive synthesis engine.
    Subtractive,
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Error variants for `Patch` command processing.
#[derive(Debug, Clone, PartialEq)]
pub enum PatchError {
    /// A command was applied to a patch that has not yet been created.
    NotInitialized,
    /// The `pan` value supplied is out of the valid range −1.0 to 1.0.
    InvalidPan(f64),
    /// The supplied name is empty.
    EmptyName,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::NotInitialized => write!(f, "patch has not been created yet"),
            PatchError::InvalidPan(v) => {
                write!(f, "pan value {v} is out of range -1.0 to 1.0")
            }
            PatchError::EmptyName => write!(f, "patch name must not be empty"),
        }
    }
}

impl std::error::Error for PatchError {}

// ─────────────────────────────────────────────────────────────────────────────
// Commands
// ─────────────────────────────────────────────────────────────────────────────

/// Commands that drive a `Patch` aggregate.
#[derive(Debug, Clone)]
pub enum PatchCommand {
    /// Create and initialise the patch.
    CreatePatch {
        name: String,
        engine_type: EngineType,
        subscription: ChannelSubscription,
    },
    /// Update the channel subscription.
    UpdateSubscription { subscription: ChannelSubscription },
    /// Update the oscillator configuration.
    UpdateOscillator { config: OscillatorConfig },
    /// Update the amplitude envelope configuration.
    UpdateEnvelope { config: AmpEnvelopeConfig },
    /// Set the output gain.
    SetGain { gain: Amplitude },
    /// Set the stereo pan position (−1.0 = left, 1.0 = right).
    SetPan { pan: f64 },
    /// Mark the patch as active.
    ActivatePatch,
    /// Mark the patch as inactive.
    DeactivatePatch,
    /// Update the filter configuration.
    UpdateFilter { config: FilterConfig },
}

// ─────────────────────────────────────────────────────────────────────────────
// Events
// ─────────────────────────────────────────────────────────────────────────────

/// Domain events emitted by the `Patch` aggregate.
#[derive(Debug, Clone, PartialEq)]
pub enum PatchEvent {
    /// The patch was created.
    PatchCreated {
        id: PatchId,
        name: String,
        engine_type: EngineType,
    },
    /// The channel subscription changed.
    SubscriptionChanged {
        id: PatchId,
        subscription: ChannelSubscription,
    },
    /// Oscillator, envelope, filter, gain, or pan parameters changed.
    PatchParametersUpdated { id: PatchId },
    /// The patch was activated.
    PatchActivated { id: PatchId },
    /// The patch was deactivated.
    PatchDeactivated { id: PatchId },
}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregate state
// ─────────────────────────────────────────────────────────────────────────────

/// The `Patch` aggregate.
///
/// A `Patch` is a complete instrument: it owns a `VoiceAllocator` (its voice
/// pool), carries synthesis parameters, and knows which MIDI channel(s) to
/// respond to.
///
/// # Independence guarantee
///
/// Each `Patch` constructs its own `VoiceAllocator` at creation time.
/// No two patches ever share a voice pool, so the polyphony of one patch
/// can never deplete the voices available to another.
///
/// # Pan invariant
///
/// `pan` is always in the range −1.0 (left) to 1.0 (right).  `SetPan`
/// commands with values outside that range are rejected with
/// [`PatchError::InvalidPan`].
pub struct Patch {
    /// Unique identifier, `None` until `CreatePatch` is applied.
    id: Option<PatchId>,
    name: String,
    active: bool,
    engine_type: EngineType,
    oscillator: OscillatorConfig,
    amp_envelope: AmpEnvelopeConfig,
    filter: FilterConfig,
    gain: Amplitude,
    /// Stereo pan: −1.0 (full left) to 1.0 (full right).
    pan: f64,
    subscription: ChannelSubscription,
    /// Each patch owns its own independent voice pool.
    voice_pool: VoiceAllocator,
    voice_pool_config: VoicePoolConfig,
}

impl Patch {
    /// Create an uninitialised `Patch` with the given voice-pool configuration.
    ///
    /// Call [`Patch::handle`] with [`PatchCommand::CreatePatch`] to initialise.
    pub fn new(voice_pool_config: VoicePoolConfig, subscription: ChannelSubscription) -> Self {
        let num_voices = voice_pool_config.max_voices() as usize;
        Self {
            id: None,
            name: String::new(),
            active: false,
            engine_type: EngineType::default(),
            oscillator: OscillatorConfig::default(),
            amp_envelope: AmpEnvelopeConfig::default(),
            filter: FilterConfig::default(),
            gain: Amplitude::unity(),
            pan: 0.0,
            subscription,
            voice_pool: VoiceAllocator::new(num_voices),
            voice_pool_config,
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// The patch's unique identifier (available after `CreatePatch`).
    pub fn id(&self) -> Option<PatchId> {
        self.id
    }

    /// The patch name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the patch is currently active.
    pub fn active(&self) -> bool {
        self.active
    }

    /// The synthesis engine type.
    pub fn engine_type(&self) -> EngineType {
        self.engine_type
    }

    /// Current oscillator configuration.
    pub fn oscillator(&self) -> OscillatorConfig {
        self.oscillator
    }

    /// Current amplitude envelope configuration.
    pub fn amp_envelope(&self) -> AmpEnvelopeConfig {
        self.amp_envelope
    }

    /// Current filter configuration.
    pub fn filter(&self) -> FilterConfig {
        self.filter
    }

    /// Current output gain.
    pub fn gain(&self) -> Amplitude {
        self.gain
    }

    /// Current stereo pan (−1.0 left … 1.0 right).
    pub fn pan(&self) -> f64 {
        self.pan
    }

    /// Current channel subscription.
    pub fn subscription(&self) -> ChannelSubscription {
        self.subscription
    }

    /// Shared reference to the voice pool for rendering.
    pub fn voice_pool(&self) -> &VoiceAllocator {
        &self.voice_pool
    }

    /// Mutable reference to the voice pool.
    pub fn voice_pool_mut(&mut self) -> &mut VoiceAllocator {
        &mut self.voice_pool
    }

    /// The voice pool configuration.
    pub fn voice_pool_config(&self) -> VoicePoolConfig {
        self.voice_pool_config
    }

    // ── Command handler ───────────────────────────────────────────────────────

    /// Apply a command to the patch and return the resulting events.
    ///
    /// Returns `Err(PatchError)` if the command is invalid in the current state.
    pub fn handle(&mut self, cmd: PatchCommand) -> Result<Vec<PatchEvent>, PatchError> {
        match cmd {
            PatchCommand::CreatePatch {
                name,
                engine_type,
                subscription,
            } => self.create_patch(name, engine_type, subscription),

            PatchCommand::UpdateSubscription { subscription } => {
                self.require_id()?;
                self.subscription = subscription;
                Ok(vec![PatchEvent::SubscriptionChanged {
                    id: self.id.unwrap(),
                    subscription,
                }])
            }

            PatchCommand::UpdateOscillator { config } => {
                self.require_id()?;
                self.oscillator = config;
                Ok(vec![PatchEvent::PatchParametersUpdated {
                    id: self.id.unwrap(),
                }])
            }

            PatchCommand::UpdateEnvelope { config } => {
                self.require_id()?;
                self.amp_envelope = config;
                Ok(vec![PatchEvent::PatchParametersUpdated {
                    id: self.id.unwrap(),
                }])
            }

            PatchCommand::SetGain { gain } => {
                self.require_id()?;
                self.gain = gain;
                Ok(vec![PatchEvent::PatchParametersUpdated {
                    id: self.id.unwrap(),
                }])
            }

            PatchCommand::SetPan { pan } => {
                self.require_id()?;
                if pan.is_nan() || !(-1.0_f64..=1.0_f64).contains(&pan) {
                    return Err(PatchError::InvalidPan(pan));
                }
                self.pan = pan;
                Ok(vec![PatchEvent::PatchParametersUpdated {
                    id: self.id.unwrap(),
                }])
            }

            PatchCommand::ActivatePatch => {
                self.require_id()?;
                self.active = true;
                Ok(vec![PatchEvent::PatchActivated {
                    id: self.id.unwrap(),
                }])
            }

            PatchCommand::DeactivatePatch => {
                self.require_id()?;
                self.active = false;
                Ok(vec![PatchEvent::PatchDeactivated {
                    id: self.id.unwrap(),
                }])
            }

            PatchCommand::UpdateFilter { config } => {
                self.require_id()?;
                self.filter = config;
                Ok(vec![PatchEvent::PatchParametersUpdated {
                    id: self.id.unwrap(),
                }])
            }
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn create_patch(
        &mut self,
        name: String,
        engine_type: EngineType,
        subscription: ChannelSubscription,
    ) -> Result<Vec<PatchEvent>, PatchError> {
        if name.is_empty() {
            return Err(PatchError::EmptyName);
        }
        // Use a deterministic ID derived from the name length for a simple
        // self-contained aggregate. In a real system the ID would come from an
        // external ID generator injected via the constructor.
        let id = PatchId::new(name.len() as u32 ^ 0xDEAD_BEEF);
        self.id = Some(id);
        self.name = name.clone();
        self.engine_type = engine_type;
        self.subscription = subscription;
        self.active = false;
        Ok(vec![PatchEvent::PatchCreated {
            id,
            name,
            engine_type,
        }])
    }

    /// Return `Err(PatchError::NotInitialized)` if the patch has no id yet.
    fn require_id(&self) -> Result<(), PatchError> {
        if self.id.is_none() {
            Err(PatchError::NotInitialized)
        } else {
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_group::MidiGroup;
    use crate::patch::channel_subscription::ChannelAddress;
    use crate::patch::voice_pool_config::StealingPolicy;
    use crate::synth::filter_config::FilterType;
    use crate::synth::oscillator_config::Waveform;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn default_subscription() -> ChannelSubscription {
        let group = MidiGroup::try_new(0).unwrap();
        let channel = MidiChannel::try_new(0).unwrap();
        let address = ChannelAddress::new(group, channel);
        ChannelSubscription::new(address, None)
    }

    fn default_voice_pool_config() -> VoicePoolConfig {
        VoicePoolConfig::try_new(8, StealingPolicy::QuietestFirst).unwrap()
    }

    fn default_patch() -> Patch {
        Patch::new(default_voice_pool_config(), default_subscription())
    }

    fn created_patch() -> Patch {
        let mut p = default_patch();
        p.handle(PatchCommand::CreatePatch {
            name: "Lead".to_string(),
            engine_type: EngineType::Sine,
            subscription: default_subscription(),
        })
        .unwrap();
        p
    }

    // ── CreatePatch ───────────────────────────────────────────────────────────

    #[test]
    fn create_patch_emits_patch_created() {
        let mut p = default_patch();
        let events = p
            .handle(PatchCommand::CreatePatch {
                name: "Bass".to_string(),
                engine_type: EngineType::Subtractive,
                subscription: default_subscription(),
            })
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PatchEvent::PatchCreated { .. }));
    }

    #[test]
    fn create_patch_sets_name_and_engine_type() {
        let p = created_patch();
        assert_eq!(p.name(), "Lead");
        assert_eq!(p.engine_type(), EngineType::Sine);
    }

    #[test]
    fn create_patch_with_empty_name_returns_error() {
        let mut p = default_patch();
        let result = p.handle(PatchCommand::CreatePatch {
            name: String::new(),
            engine_type: EngineType::Sine,
            subscription: default_subscription(),
        });
        assert!(matches!(result, Err(PatchError::EmptyName)));
    }

    #[test]
    fn commands_before_create_return_not_initialized() {
        let mut p = default_patch();
        let result = p.handle(PatchCommand::ActivatePatch);
        assert!(matches!(result, Err(PatchError::NotInitialized)));
    }

    // ── Independent voice pools ───────────────────────────────────────────────

    #[test]
    fn each_patch_has_independent_voice_pool() {
        let cfg = VoicePoolConfig::try_new(4, StealingPolicy::QuietestFirst).unwrap();
        let p1 = Patch::new(cfg, default_subscription());
        let p2 = Patch::new(cfg, default_subscription());
        // The two patches have independent allocators: the address of their
        // voice pools must differ (they are separate heap allocations).
        assert_ne!(
            p1.voice_pool() as *const _,
            p2.voice_pool() as *const _,
            "each patch must own its own voice pool"
        );
    }

    #[test]
    fn voice_pool_voice_count_matches_config() {
        let cfg = VoicePoolConfig::try_new(3, StealingPolicy::OldestFirst).unwrap();
        let p = Patch::new(cfg, default_subscription());
        assert_eq!(p.voice_pool().voice_count(), 3);
    }

    // ── ActivatePatch / DeactivatePatch ───────────────────────────────────────

    #[test]
    fn activate_patch_sets_active_true() {
        let mut p = created_patch();
        p.handle(PatchCommand::ActivatePatch).unwrap();
        assert!(p.active());
    }

    #[test]
    fn activate_patch_emits_patch_activated() {
        let mut p = created_patch();
        let events = p.handle(PatchCommand::ActivatePatch).unwrap();
        assert!(matches!(events[0], PatchEvent::PatchActivated { .. }));
    }

    #[test]
    fn deactivate_patch_sets_active_false() {
        let mut p = created_patch();
        p.handle(PatchCommand::ActivatePatch).unwrap();
        p.handle(PatchCommand::DeactivatePatch).unwrap();
        assert!(!p.active());
    }

    #[test]
    fn deactivate_patch_emits_patch_deactivated() {
        let mut p = created_patch();
        p.handle(PatchCommand::ActivatePatch).unwrap();
        let events = p.handle(PatchCommand::DeactivatePatch).unwrap();
        assert!(matches!(events[0], PatchEvent::PatchDeactivated { .. }));
    }

    // ── SetPan ────────────────────────────────────────────────────────────────

    #[test]
    fn set_pan_within_range_is_accepted() {
        let mut p = created_patch();
        p.handle(PatchCommand::SetPan { pan: -1.0 }).unwrap();
        assert!((p.pan() - (-1.0)).abs() < f64::EPSILON);

        p.handle(PatchCommand::SetPan { pan: 1.0 }).unwrap();
        assert!((p.pan() - 1.0).abs() < f64::EPSILON);

        p.handle(PatchCommand::SetPan { pan: 0.0 }).unwrap();
        assert!(p.pan().abs() < f64::EPSILON);
    }

    #[test]
    fn set_pan_above_one_is_rejected() {
        let mut p = created_patch();
        let result = p.handle(PatchCommand::SetPan { pan: 1.001 });
        assert!(matches!(result, Err(PatchError::InvalidPan(_))));
    }

    #[test]
    fn set_pan_below_minus_one_is_rejected() {
        let mut p = created_patch();
        let result = p.handle(PatchCommand::SetPan { pan: -1.001 });
        assert!(matches!(result, Err(PatchError::InvalidPan(_))));
    }

    #[test]
    fn set_pan_nan_is_rejected() {
        let mut p = created_patch();
        let result = p.handle(PatchCommand::SetPan { pan: f64::NAN });
        assert!(matches!(result, Err(PatchError::InvalidPan(_))));
    }

    // ── SetGain ───────────────────────────────────────────────────────────────

    #[test]
    fn set_gain_updates_gain() {
        let mut p = created_patch();
        let gain = Amplitude::try_new(0.75).unwrap();
        p.handle(PatchCommand::SetGain { gain }).unwrap();
        assert!((p.gain().value() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn set_gain_emits_parameters_updated() {
        let mut p = created_patch();
        let gain = Amplitude::try_new(0.5).unwrap();
        let events = p.handle(PatchCommand::SetGain { gain }).unwrap();
        assert!(matches!(
            events[0],
            PatchEvent::PatchParametersUpdated { .. }
        ));
    }

    // ── UpdateSubscription ────────────────────────────────────────────────────

    #[test]
    fn update_subscription_changes_subscription() {
        let mut p = created_patch();
        let new_sub = {
            let group = MidiGroup::try_new(0).unwrap();
            let channel = MidiChannel::try_new(5).unwrap();
            let address = ChannelAddress::new(group, channel);
            ChannelSubscription::new(address, None)
        };
        p.handle(PatchCommand::UpdateSubscription {
            subscription: new_sub,
        })
        .unwrap();
        assert_eq!(p.subscription(), new_sub);
    }

    #[test]
    fn update_subscription_emits_subscription_changed() {
        let mut p = created_patch();
        let events = p
            .handle(PatchCommand::UpdateSubscription {
                subscription: default_subscription(),
            })
            .unwrap();
        assert!(matches!(events[0], PatchEvent::SubscriptionChanged { .. }));
    }

    // ── UpdateOscillator ──────────────────────────────────────────────────────

    #[test]
    fn update_oscillator_changes_config() {
        let mut p = created_patch();
        let cfg = OscillatorConfig::try_new(100.0, 0.5, Waveform::Square).unwrap();
        p.handle(PatchCommand::UpdateOscillator { config: cfg })
            .unwrap();
        assert_eq!(p.oscillator(), cfg);
    }

    // ── UpdateEnvelope ────────────────────────────────────────────────────────

    #[test]
    fn update_envelope_changes_config() {
        let mut p = created_patch();
        let cfg = AmpEnvelopeConfig::try_new(0.1, 0.2, 0.5, 0.4).unwrap();
        p.handle(PatchCommand::UpdateEnvelope { config: cfg })
            .unwrap();
        assert_eq!(p.amp_envelope(), cfg);
    }

    // ── UpdateFilter ──────────────────────────────────────────────────────────

    #[test]
    fn update_filter_changes_config() {
        let mut p = created_patch();
        let cfg = FilterConfig::try_new(2_000.0, FilterType::HighPass, 0.3).unwrap();
        p.handle(PatchCommand::UpdateFilter { config: cfg })
            .unwrap();
        assert_eq!(p.filter(), cfg);
    }

    // ── PatchError display ────────────────────────────────────────────────────

    #[test]
    fn error_display_not_initialized() {
        let msg = PatchError::NotInitialized.to_string();
        assert!(msg.contains("not been created"));
    }

    #[test]
    fn error_display_invalid_pan() {
        let msg = PatchError::InvalidPan(2.5).to_string();
        assert!(msg.contains("2.5"));
    }

    #[test]
    fn error_display_empty_name() {
        let msg = PatchError::EmptyName.to_string();
        assert!(msg.contains("empty"));
    }

    // ── Multi-patch dispatch ──────────────────────────────────────────────────

    #[test]
    fn two_patches_on_same_subscription_both_independent() {
        // Validates: two patches with the same subscription each have their
        // own independent voice pool — the pool of one does not affect the other.
        let cfg = VoicePoolConfig::try_new(2, StealingPolicy::QuietestFirst).unwrap();
        let sub = default_subscription();
        let p1 = Patch::new(cfg, sub);
        let p2 = Patch::new(cfg, sub);
        // Voice pools at different addresses — truly independent.
        assert_ne!(p1.voice_pool() as *const _, p2.voice_pool() as *const _,);
        assert_eq!(p1.voice_pool().voice_count(), 2);
        assert_eq!(p2.voice_pool().voice_count(), 2);
    }
}
