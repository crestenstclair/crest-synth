// path: src/sample/sample_player.rs
//
// SamplePlayer — domain service for the Sample context.
//
// Plays a zone's sample at the correct pitch with the configured
// interpolation and loop mode. Zone *selection* ("correct pitch" in the
// sense of matching the right zone(s) to a note+velocity address) is
// `ZoneResolver`'s job, delegated to here; `SamplePlayer`'s own
// responsibility is to attach playback configuration — the `SampleSet`'s
// configured interpolation mode, and the caller-supplied loop mode — to
// each matched zone, producing a `PlaybackInstruction` ready to hand to
// whatever audio-rate voice actually decodes and mixes the sample.
//
// `SamplePlayer` owns no audio state itself (no buffers, no playback
// position, no sample data) — it is a stateless orchestrator that turns
// "note + velocity + loop mode" into "which sample(s), with which
// interpolation, looped how". It depends on `ZoneResolver` via constructor
// injection rather than instantiating one itself, so a test can substitute
// a stub resolver without touching this type's implementation.

use crate::sample::sample_set::{InterpolationMode, SampleSet};
use crate::sample::zone_resolver::ZoneResolver;

/// How a matched zone's sample should loop once played.
///
/// This is playback-time configuration, not a property of the `SampleSet`
/// aggregate itself — the same zone can be triggered with different loop
/// modes by different callers (e.g. a one-shot preview vs. sustained
/// playback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    /// Play the sample once from start to end, then stop.
    #[default]
    OneShot,
    /// Loop the sample forward from start to end indefinitely.
    Forward,
    /// Loop the sample back and forth between start and end indefinitely.
    PingPong,
}

/// A request to play a note: the address to resolve zones against, plus the
/// loop mode to apply to whatever zones match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackRequest {
    key: u8,
    velocity: u8,
    loop_mode: LoopMode,
}

impl PlaybackRequest {
    pub fn new(key: u8, velocity: u8, loop_mode: LoopMode) -> Self {
        Self {
            key,
            velocity,
            loop_mode,
        }
    }

    pub fn key(&self) -> u8 {
        self.key
    }

    pub fn velocity(&self) -> u8 {
        self.velocity
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }
}

/// Everything an audio-rate voice needs to sound a matched zone: which
/// sample to play, how to resample it, and how to loop it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackInstruction {
    sample_ref: String,
    key: u8,
    interpolation: InterpolationMode,
    loop_mode: LoopMode,
}

impl PlaybackInstruction {
    pub fn sample_ref(&self) -> &str {
        &self.sample_ref
    }

    pub fn key(&self) -> u8 {
        self.key
    }

    pub fn interpolation(&self) -> InterpolationMode {
        self.interpolation
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }
}

/// Domain service that turns a [`PlaybackRequest`] into zero or more
/// [`PlaybackInstruction`]s: one per zone in a [`SampleSet`] whose key range
/// and velocity range both match the request.
///
/// Depends on [`ZoneResolver`] via constructor injection (never constructs
/// its own), so a test can substitute a stub resolver without touching this
/// type's implementation.
#[derive(Debug, Clone)]
pub struct SamplePlayer {
    resolver: ZoneResolver,
}

impl SamplePlayer {
    /// Convenience constructor for callers that don't care which resolver
    /// implementation is used; wires up the default `ZoneResolver`.
    pub fn new() -> Self {
        Self::with_resolver(ZoneResolver::new())
    }

    /// Full constructor: inject the `ZoneResolver` collaborator explicitly.
    pub fn with_resolver(resolver: ZoneResolver) -> Self {
        Self { resolver }
    }

    /// Resolve `request` against `sample_set`, returning one
    /// `PlaybackInstruction` per matching zone, each carrying the sample
    /// set's configured interpolation mode and the request's loop mode.
    ///
    /// Delegates zone matching to `ZoneResolver` rather than reimplementing
    /// it. Multiple matches are returned (layered zones are intentional,
    /// per `SampleSet`'s own invariant); a request that matches no zone
    /// returns an empty list rather than an error.
    pub fn play(
        &self,
        sample_set: &SampleSet,
        request: PlaybackRequest,
    ) -> Vec<PlaybackInstruction> {
        let interpolation = sample_set.interpolation();
        self.resolver
            .resolve(sample_set, request.key(), request.velocity())
            .into_iter()
            .map(|zone| PlaybackInstruction {
                sample_ref: zone.sample_ref().to_string(),
                key: request.key(),
                interpolation,
                loop_mode: request.loop_mode(),
            })
            .collect()
    }
}

impl Default for SamplePlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::sample_set::{KeyRange, VelocityRange, Zone};

    fn zone(key_lo: u8, key_hi: u8, vel_lo: u8, vel_hi: u8, sample_ref: &str) -> Zone {
        Zone::new(
            KeyRange::try_new(key_lo, key_hi).unwrap(),
            VelocityRange::try_new(vel_lo, vel_hi).unwrap(),
            sample_ref.to_string(),
        )
    }

    #[test]
    fn play_returns_one_instruction_per_matching_zone() {
        let mut set = SampleSet::new(InterpolationMode::Cubic);
        set.apply_add_zone(zone(60, 72, 0, 127, "layer-a"));
        set.apply_add_zone(zone(60, 72, 0, 127, "layer-b"));
        set.apply_add_zone(zone(73, 84, 0, 127, "layer-c"));

        let player = SamplePlayer::new();
        let request = PlaybackRequest::new(65, 100, LoopMode::Forward);
        let instructions = player.play(&set, request);

        assert_eq!(instructions.len(), 2);
        let refs: Vec<&str> = instructions.iter().map(|i| i.sample_ref()).collect();
        assert!(refs.contains(&"layer-a"));
        assert!(refs.contains(&"layer-b"));
    }

    #[test]
    fn play_carries_the_sample_sets_configured_interpolation_mode() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(0, 127, 0, 127, "only"));

        let player = SamplePlayer::new();
        let instructions = player.play(&set, PlaybackRequest::new(60, 64, LoopMode::OneShot));

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].interpolation(), InterpolationMode::Linear);
    }

    #[test]
    fn play_carries_the_requested_loop_mode() {
        let mut set = SampleSet::new(InterpolationMode::None);
        set.apply_add_zone(zone(0, 127, 0, 127, "only"));

        let player = SamplePlayer::new();
        let instructions = player.play(&set, PlaybackRequest::new(60, 64, LoopMode::PingPong));

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].loop_mode(), LoopMode::PingPong);
    }

    #[test]
    fn play_returns_empty_when_no_zone_matches() {
        let set = SampleSet::new(InterpolationMode::Linear);
        let player = SamplePlayer::new();
        let instructions = player.play(&set, PlaybackRequest::new(60, 64, LoopMode::OneShot));
        assert!(instructions.is_empty());
    }

    #[test]
    fn play_excludes_zones_whose_velocity_range_does_not_match() {
        let mut set = SampleSet::new(InterpolationMode::Linear);
        set.apply_add_zone(zone(60, 72, 0, 50, "quiet-only"));

        let player = SamplePlayer::new();
        let instructions = player.play(&set, PlaybackRequest::new(65, 100, LoopMode::OneShot));
        assert!(instructions.is_empty());
    }

    #[test]
    fn with_resolver_allows_injecting_an_explicit_resolver() {
        let mut set = SampleSet::new(InterpolationMode::None);
        set.apply_add_zone(zone(0, 127, 0, 127, "only"));

        let player = SamplePlayer::with_resolver(ZoneResolver::new());
        let instructions = player.play(&set, PlaybackRequest::new(10, 10, LoopMode::OneShot));
        assert_eq!(instructions.len(), 1);
    }

    #[test]
    fn default_constructs_equivalent_player() {
        let set = SampleSet::new(InterpolationMode::None);
        let player = SamplePlayer::default();
        assert!(player
            .play(&set, PlaybackRequest::new(0, 0, LoopMode::OneShot))
            .is_empty());
    }
}
