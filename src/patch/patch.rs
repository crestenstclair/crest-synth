// path: src/patch/patch.rs

//! Patch aggregate: one playable instrument and its complete configuration.
//!
//! The `Patch` aggregate owns configuration state for a single instrument
//! slot: its MIDI channel mapping, optional MPE zone, modulation matrix,
//! sample set assignment, voice configuration, and which mixer strip
//! carries its audio. All command handling here runs off the real-time
//! audio thread; the resulting `PatchEvent`s are the only way this state
//! is meant to cross into the `ParameterBridge` / `EventRing` that the
//! audio thread observes.

use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;

/// Stable identity for a `Patch` aggregate instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PatchId(u32);

impl PatchId {
    /// Constructs a `PatchId` from a raw identifier.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw identifier.
    pub fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for PatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PatchId({})", self.0)
    }
}

/// Which MIDI channels (0-15) address this patch.
///
/// Represented as a 16-bit mask so a single patch can be layered across
/// multiple channels. Matching is what makes layering intentional per the
/// dispatch invariant: a `MidiEvent` is routed to exactly the set of
/// patches whose mapping matches its address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelMapping {
    mask: u16,
}

impl ChannelMapping {
    /// A mapping that matches no channel at all.
    pub fn none() -> Self {
        Self { mask: 0 }
    }

    /// A mapping that matches every MIDI channel (omni).
    pub fn omni() -> Self {
        Self { mask: 0xFFFF }
    }

    /// A mapping that matches exactly one MIDI channel (0-15).
    pub fn single(channel: u8) -> Result<Self, PatchError> {
        if channel > 15 {
            return Err(PatchError::InvalidChannel(channel));
        }
        Ok(Self {
            mask: 1u16 << channel,
        })
    }

    /// A mapping that matches any of `channels` (each must be 0-15).
    pub fn from_channels(channels: &[u8]) -> Result<Self, PatchError> {
        let mut mask = 0u16;
        for &channel in channels {
            if channel > 15 {
                return Err(PatchError::InvalidChannel(channel));
            }
            mask |= 1u16 << channel;
        }
        Ok(Self { mask })
    }

    /// True if this mapping matches `channel` (0-15).
    pub fn matches(&self, channel: u8) -> bool {
        if channel > 15 {
            return false;
        }
        self.mask & (1u16 << channel) != 0
    }

    /// The raw 16-bit channel mask, one bit per MIDI channel.
    pub fn mask(&self) -> u16 {
        self.mask
    }
}

impl Default for ChannelMapping {
    fn default() -> Self {
        Self::none()
    }
}

/// An MPE (MIDI Polyphonic Expression) zone: a master channel plus a
/// contiguous run of member channels that carry per-note expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpeZone {
    master_channel: u8,
    member_start: u8,
    member_count: u8,
}

impl MpeZone {
    /// Builds an MPE zone. `master_channel` and every member channel must
    /// fall within the 16 MIDI channels (0-15), and the zone must claim at
    /// least one member channel.
    pub fn try_new(
        master_channel: u8,
        member_start: u8,
        member_count: u8,
    ) -> Result<Self, PatchError> {
        if master_channel > 15 {
            return Err(PatchError::InvalidChannel(master_channel));
        }
        if member_start > 15 {
            return Err(PatchError::InvalidChannel(member_start));
        }
        if member_count == 0 {
            return Err(PatchError::EmptyMpeZone);
        }
        let last_member = u16::from(member_start) + u16::from(member_count) - 1;
        if last_member > 15 {
            return Err(PatchError::MpeZoneOutOfRange);
        }
        Ok(Self {
            master_channel,
            member_start,
            member_count,
        })
    }

    /// The zone's master channel, which carries zone-wide expression.
    pub fn master_channel(&self) -> u8 {
        self.master_channel
    }

    /// The inclusive range of member channels claimed by this zone.
    pub fn member_range(&self) -> RangeInclusive<u8> {
        self.member_start..=(self.member_start + self.member_count - 1)
    }

    /// True if this zone's occupied channels (master and members)
    /// intersect `other`'s occupied channels. Used to enforce the
    /// cross-patch invariant that MPE zones never overlap.
    pub fn overlaps(&self, other: &MpeZone) -> bool {
        self.occupied_mask() & other.occupied_mask() != 0
    }

    fn occupied_mask(&self) -> u16 {
        let mut mask = 1u16 << self.master_channel;
        for channel in self.member_range() {
            mask |= 1u16 << channel;
        }
        mask
    }
}

/// A modulation source: a signal that can drive a `ModDestination`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModSource {
    Lfo1,
    Lfo2,
    EnvelopeAmp,
    EnvelopeFilter,
    Aftertouch,
    ModWheel,
    Velocity,
}

/// A modulation destination: a parameter a `ModSource` can drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModDestination {
    Pitch,
    FilterCutoff,
    FilterResonance,
    Amplitude,
    Pan,
}

/// One route in the modulation matrix: a source driving a destination by
/// some signed amount.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModRoute {
    pub source: ModSource,
    pub destination: ModDestination,
    pub amount: f32,
}

/// The modulation matrix: an ordered set of source-to-destination routes.
///
/// This is configuration data owned by the non-real-time side; the audio
/// thread only ever sees a snapshot delivered through the
/// `ParameterBridge`, never this owning collection.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModMatrix {
    routes: Vec<ModRoute>,
}

impl ModMatrix {
    /// A modulation matrix with no routes.
    pub fn empty() -> Self {
        Self { routes: Vec::new() }
    }

    /// Builds a modulation matrix from an explicit set of routes.
    pub fn with_routes(routes: Vec<ModRoute>) -> Self {
        Self { routes }
    }

    /// The routes currently configured, in evaluation order.
    pub fn routes(&self) -> &[ModRoute] {
        &self.routes
    }
}

/// Per-voice synthesis configuration: polyphony and amplitude envelope
/// timing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceConfig {
    polyphony: u8,
    attack_ms: f32,
    decay_ms: f32,
    sustain_level: f32,
    release_ms: f32,
}

impl VoiceConfig {
    /// Builds a voice configuration. `polyphony` must be at least 1;
    /// `sustain_level` must lie in `0.0..=1.0` and not be NaN; the envelope
    /// timings must be finite and non-negative.
    pub fn try_new(
        polyphony: u8,
        attack_ms: f32,
        decay_ms: f32,
        sustain_level: f32,
        release_ms: f32,
    ) -> Result<Self, PatchError> {
        if polyphony == 0 {
            return Err(PatchError::InvalidVoiceConfig(
                "polyphony must be at least 1",
            ));
        }
        if sustain_level.is_nan() || !(0.0..=1.0).contains(&sustain_level) {
            return Err(PatchError::InvalidVoiceConfig(
                "sustain_level must be within 0.0..=1.0",
            ));
        }
        if !attack_ms.is_finite() || attack_ms < 0.0 {
            return Err(PatchError::InvalidVoiceConfig(
                "attack_ms must be finite and non-negative",
            ));
        }
        if !decay_ms.is_finite() || decay_ms < 0.0 {
            return Err(PatchError::InvalidVoiceConfig(
                "decay_ms must be finite and non-negative",
            ));
        }
        if !release_ms.is_finite() || release_ms < 0.0 {
            return Err(PatchError::InvalidVoiceConfig(
                "release_ms must be finite and non-negative",
            ));
        }
        Ok(Self {
            polyphony,
            attack_ms,
            decay_ms,
            sustain_level,
            release_ms,
        })
    }

    pub fn polyphony(&self) -> u8 {
        self.polyphony
    }

    pub fn attack_ms(&self) -> f32 {
        self.attack_ms
    }

    pub fn decay_ms(&self) -> f32 {
        self.decay_ms
    }

    pub fn sustain_level(&self) -> f32 {
        self.sustain_level
    }

    pub fn release_ms(&self) -> f32 {
        self.release_ms
    }
}

/// Errors raised while constructing patch value objects or applying
/// commands to a `Patch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
    InvalidChannel(u8),
    EmptyMpeZone,
    MpeZoneOutOfRange,
    InvalidVoiceConfig(&'static str),
    OverlappingMpeZone,
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatchError::InvalidChannel(channel) => {
                write!(f, "invalid MIDI channel: {channel} (must be 0..=15)")
            }
            PatchError::EmptyMpeZone => {
                write!(f, "MPE zone must claim at least one member channel")
            }
            PatchError::MpeZoneOutOfRange => {
                write!(f, "MPE zone member range exceeds the 16 MIDI channels")
            }
            PatchError::InvalidVoiceConfig(reason) => {
                write!(f, "invalid voice configuration: {reason}")
            }
            PatchError::OverlappingMpeZone => {
                write!(f, "MPE zone overlaps another patch's zone")
            }
        }
    }
}

impl Error for PatchError {}

/// Domain events raised by `Patch` commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchEvent {
    ConfigChanged { id: PatchId },
    MappingChanged { id: PatchId },
}

/// One playable instrument and its complete configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Patch {
    id: PatchId,
    mapping: ChannelMapping,
    mixer_strip: u32,
    mod_matrix: ModMatrix,
    mpe_zone: Option<MpeZone>,
    sample_set: Option<u32>,
    voice: VoiceConfig,
}

impl Patch {
    /// Constructs a new patch bound to `mixer_strip`, with no channel
    /// mapping, no MPE zone, no sample set, and an empty modulation
    /// matrix. `voice` has no universally safe default, so callers must
    /// supply one explicitly.
    pub fn new(id: PatchId, mixer_strip: u32, voice: VoiceConfig) -> Self {
        Self {
            id,
            mapping: ChannelMapping::none(),
            mixer_strip,
            mod_matrix: ModMatrix::empty(),
            mpe_zone: None,
            sample_set: None,
            voice,
        }
    }

    pub fn id(&self) -> PatchId {
        self.id
    }

    pub fn mapping(&self) -> ChannelMapping {
        self.mapping
    }

    pub fn mixer_strip(&self) -> u32 {
        self.mixer_strip
    }

    pub fn mod_matrix(&self) -> &ModMatrix {
        &self.mod_matrix
    }

    pub fn mpe_zone(&self) -> Option<MpeZone> {
        self.mpe_zone
    }

    pub fn sample_set(&self) -> Option<u32> {
        self.sample_set
    }

    pub fn voice(&self) -> VoiceConfig {
        self.voice
    }

    /// Command: `SetMapping`. Always succeeds; a mapping that matches no
    /// channels is valid — the patch simply receives no `MidiEvent`s.
    pub fn set_mapping(&mut self, mapping: ChannelMapping) -> PatchEvent {
        self.mapping = mapping;
        PatchEvent::MappingChanged { id: self.id }
    }

    /// Command: `SetMpeZone`. `other_zones` must contain every MPE zone
    /// currently claimed by *other* patches. The invariant that MPE zones
    /// never overlap across patches is a cross-aggregate rule, so this
    /// method checks the proposed zone (if any) against that set before
    /// committing the change; on rejection this patch's state is left
    /// untouched.
    pub fn set_mpe_zone(
        &mut self,
        zone: Option<MpeZone>,
        other_zones: &[MpeZone],
    ) -> Result<PatchEvent, PatchError> {
        if let Some(candidate) = zone {
            if other_zones
                .iter()
                .any(|existing| existing.overlaps(&candidate))
            {
                return Err(PatchError::OverlappingMpeZone);
            }
        }
        self.mpe_zone = zone;
        Ok(PatchEvent::ConfigChanged { id: self.id })
    }

    /// Command: `AssignSampleSet`. Always succeeds; `None` unassigns any
    /// previously configured sample set.
    pub fn assign_sample_set(&mut self, sample_set: Option<u32>) -> PatchEvent {
        self.sample_set = sample_set;
        PatchEvent::ConfigChanged { id: self.id }
    }

    /// Command: `SetVoiceConfig`. Always succeeds; `VoiceConfig` is
    /// pre-validated at construction via `VoiceConfig::try_new`.
    pub fn set_voice_config(&mut self, voice: VoiceConfig) -> PatchEvent {
        self.voice = voice;
        PatchEvent::ConfigChanged { id: self.id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice() -> VoiceConfig {
        VoiceConfig::try_new(8, 5.0, 50.0, 0.8, 200.0).expect("valid voice config")
    }

    #[test]
    fn channel_mapping_single_matches_only_that_channel() {
        let mapping = ChannelMapping::single(3).expect("valid channel");
        assert!(mapping.matches(3));
        assert!(!mapping.matches(4));
    }

    #[test]
    fn channel_mapping_single_rejects_out_of_range_channel() {
        assert_eq!(
            ChannelMapping::single(16),
            Err(PatchError::InvalidChannel(16))
        );
    }

    #[test]
    fn channel_mapping_omni_matches_every_channel() {
        let mapping = ChannelMapping::omni();
        for channel in 0..=15u8 {
            assert!(mapping.matches(channel));
        }
    }

    #[test]
    fn channel_mapping_from_channels_matches_each_listed_channel() {
        let mapping = ChannelMapping::from_channels(&[0, 5, 10]).expect("valid channels");
        assert!(mapping.matches(0));
        assert!(mapping.matches(5));
        assert!(mapping.matches(10));
        assert!(!mapping.matches(1));
    }

    #[test]
    fn mpe_zone_rejects_zero_member_count() {
        assert_eq!(MpeZone::try_new(0, 1, 0), Err(PatchError::EmptyMpeZone));
    }

    #[test]
    fn mpe_zone_rejects_member_range_beyond_sixteen_channels() {
        assert_eq!(
            MpeZone::try_new(0, 10, 10),
            Err(PatchError::MpeZoneOutOfRange)
        );
    }

    #[test]
    fn mpe_zone_overlap_detects_shared_member_channel() {
        let a = MpeZone::try_new(0, 1, 7).expect("valid zone");
        let b = MpeZone::try_new(8, 7, 8).expect("valid zone");
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn mpe_zone_no_overlap_for_disjoint_channels() {
        let a = MpeZone::try_new(0, 1, 6).expect("valid zone");
        let b = MpeZone::try_new(8, 9, 6).expect("valid zone");
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn voice_config_rejects_zero_polyphony() {
        assert_eq!(
            VoiceConfig::try_new(0, 1.0, 1.0, 0.5, 1.0),
            Err(PatchError::InvalidVoiceConfig(
                "polyphony must be at least 1"
            ))
        );
    }

    #[test]
    fn voice_config_rejects_out_of_range_sustain() {
        assert_eq!(
            VoiceConfig::try_new(1, 1.0, 1.0, 1.5, 1.0),
            Err(PatchError::InvalidVoiceConfig(
                "sustain_level must be within 0.0..=1.0"
            ))
        );
    }

    #[test]
    fn voice_config_rejects_nan_sustain() {
        assert_eq!(
            VoiceConfig::try_new(1, 1.0, 1.0, f32::NAN, 1.0),
            Err(PatchError::InvalidVoiceConfig(
                "sustain_level must be within 0.0..=1.0"
            ))
        );
    }

    #[test]
    fn voice_config_rejects_negative_attack() {
        assert_eq!(
            VoiceConfig::try_new(1, -1.0, 1.0, 0.5, 1.0),
            Err(PatchError::InvalidVoiceConfig(
                "attack_ms must be finite and non-negative"
            ))
        );
    }

    #[test]
    fn set_mapping_emits_mapping_changed_and_updates_state() {
        let mut patch = Patch::new(PatchId::new(1), 0, voice());
        let mapping = ChannelMapping::single(2).expect("valid channel");

        let event = patch.set_mapping(mapping);

        assert_eq!(
            event,
            PatchEvent::MappingChanged {
                id: PatchId::new(1)
            }
        );
        assert_eq!(patch.mapping(), mapping);
    }

    #[test]
    fn assign_sample_set_emits_config_changed_and_updates_state() {
        let mut patch = Patch::new(PatchId::new(1), 0, voice());

        let event = patch.assign_sample_set(Some(42));

        assert_eq!(
            event,
            PatchEvent::ConfigChanged {
                id: PatchId::new(1)
            }
        );
        assert_eq!(patch.sample_set(), Some(42));
    }

    #[test]
    fn assign_sample_set_none_clears_previous_assignment() {
        let mut patch = Patch::new(PatchId::new(1), 0, voice());
        patch.assign_sample_set(Some(7));

        patch.assign_sample_set(None);

        assert_eq!(patch.sample_set(), None);
    }

    #[test]
    fn set_voice_config_emits_config_changed_and_updates_state() {
        let mut patch = Patch::new(PatchId::new(1), 0, voice());
        let new_voice = VoiceConfig::try_new(4, 1.0, 1.0, 0.5, 1.0).expect("valid voice config");

        let event = patch.set_voice_config(new_voice);

        assert_eq!(
            event,
            PatchEvent::ConfigChanged {
                id: PatchId::new(1)
            }
        );
        assert_eq!(patch.voice(), new_voice);
    }

    #[test]
    fn set_mpe_zone_succeeds_when_no_overlap_with_other_zones() {
        let mut patch = Patch::new(PatchId::new(1), 0, voice());
        let zone = MpeZone::try_new(0, 1, 6).expect("valid zone");
        let others = [MpeZone::try_new(8, 9, 6).expect("valid zone")];

        let event = patch.set_mpe_zone(Some(zone), &others).expect("no overlap");

        assert_eq!(
            event,
            PatchEvent::ConfigChanged {
                id: PatchId::new(1)
            }
        );
        assert_eq!(patch.mpe_zone(), Some(zone));
    }

    #[test]
    fn set_mpe_zone_rejects_overlap_and_leaves_state_untouched() {
        let mut patch = Patch::new(PatchId::new(1), 0, voice());
        let original = MpeZone::try_new(0, 1, 6).expect("valid zone");
        patch
            .set_mpe_zone(Some(original), &[])
            .expect("initial zone has no conflicts");

        let overlapping = MpeZone::try_new(8, 5, 4).expect("valid zone");
        let others = [MpeZone::try_new(9, 6, 4).expect("valid zone")];

        let result = patch.set_mpe_zone(Some(overlapping), &others);

        assert_eq!(result, Err(PatchError::OverlappingMpeZone));
        assert_eq!(patch.mpe_zone(), Some(original));
    }

    #[test]
    fn set_mpe_zone_none_always_succeeds_regardless_of_other_zones() {
        let mut patch = Patch::new(PatchId::new(1), 0, voice());
        let others = [MpeZone::try_new(0, 0, 16).expect("valid zone")];

        let event = patch
            .set_mpe_zone(None, &others)
            .expect("clearing always ok");

        assert_eq!(
            event,
            PatchEvent::ConfigChanged {
                id: PatchId::new(1)
            }
        );
        assert_eq!(patch.mpe_zone(), None);
    }
}
