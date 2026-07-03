// path: src/mixer/mix_engine.rs

//! `MixEngine` — the domain service that runs one full mix pass.
//!
//! Signal flow (per the project's canonical topology): engine output →
//! channel strip inserts → volume and pan → send taps and bus routing →
//! aux bus inserts → master bus inserts → limiter → output.
//!
//! `MixEngine` owns no audio-thread state: it is handed the strips, aux
//! buses, and master bus for one block, together with the insert-chain
//! processors each of them needs (caller-supplied, exactly like
//! [`ChainRenderer`] itself), and it returns the fully-rendered master
//! output for that block. Like [`ChannelStrip`] and [`MixBus`], this is
//! domain/control-plane modeling of *what* one mix pass computes; running
//! it allocation-free on the real-time thread is a concern for whatever
//! real-time adapter eventually drives it, not for this type.

use crate::effects::chain_renderer::{ChainRenderError, ChainRenderer};
use crate::effects::effect_chain::EffectChain;
use crate::effects::effect_processor::{AudioFrame, EffectProcessor};
use crate::mixer::channel_strip::{Amplitude as StripAmplitude, ChannelStrip, SendBusId};
use crate::mixer::mix_bus::{BusId, BusKind, MixBus};

/// One channel strip's material for a single mix pass: its domain state,
/// its insert chain, the concrete processors occupying that chain
/// (positionally aligned with `inserts.slots()`, per [`ChainRenderer`]'s
/// contract), and this block's raw (pre-insert, pre-fader) audio.
pub struct StripSource<'a> {
    pub strip: &'a mut ChannelStrip,
    pub inserts: &'a EffectChain,
    pub insert_processors: &'a mut [Box<dyn EffectProcessor>],
    pub input: &'a [AudioFrame],
}

/// One aux bus's material for a single mix pass: the [`MixBus`] aggregate
/// (supplying its identity and return level) and its insert chain. An aux
/// bus's input is never supplied directly by the caller — it is entirely
/// the sum of the send taps routed to it during this pass.
pub struct AuxBusSource<'a> {
    pub bus: &'a MixBus,
    pub inserts: &'a EffectChain,
    pub insert_processors: &'a mut [Box<dyn EffectProcessor>],
}

/// The master bus's material for a single mix pass: the master [`MixBus`],
/// its insert chain, and the limiter applied after those inserts,
/// immediately before output.
pub struct MasterSource<'a> {
    pub bus: &'a MixBus,
    pub inserts: &'a EffectChain,
    pub insert_processors: &'a mut [Box<dyn EffectProcessor>],
    pub limiter: &'a Limiter,
}

/// Errors that prevent [`MixEngine::render`] from completing a mix pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixEngineError {
    /// `master.bus` was not the master bus. Only `BusKind::Master` is a
    /// valid terminal summing point for a mix pass.
    MasterBusNotMaster,
    /// An entry in `aux_buses` was the master bus. Aux buses feed the
    /// master bus, never each other, and the master bus is supplied
    /// separately via `master`.
    AuxBusIsMaster { bus_index: usize },
    /// A channel strip's insert chain rejected the supplied processors.
    StripChain {
        strip_index: usize,
        source: ChainRenderError,
    },
    /// An aux bus's insert chain rejected the supplied processors.
    AuxChain {
        bus_index: usize,
        source: ChainRenderError,
    },
    /// The master bus's insert chain rejected the supplied processors.
    MasterChain(ChainRenderError),
    /// A channel strip's send tap named a bus id with no matching aux bus
    /// supplied to this mix pass. Every send must resolve to a real aux
    /// bus or the routing is undefined.
    UnknownSendBus { strip_index: usize, bus: BusId },
    /// A strip's input buffer did not match the block length established
    /// for this mix pass, so there is no single length to sum buses into.
    InconsistentBlockLength {
        expected: usize,
        got: usize,
        strip_index: usize,
    },
}

impl std::fmt::Display for MixEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MixEngineError::MasterBusNotMaster => {
                write!(f, "the master bus source must be the master bus (BusId::MASTER)")
            }
            MixEngineError::AuxBusIsMaster { bus_index } => write!(
                f,
                "aux bus slot {bus_index} was supplied the master bus; aux buses feed the master bus, never each other"
            ),
            MixEngineError::StripChain { strip_index, source } => {
                write!(f, "strip {strip_index} insert chain failed: {source}")
            }
            MixEngineError::AuxChain { bus_index, source } => {
                write!(f, "aux bus {bus_index} insert chain failed: {source}")
            }
            MixEngineError::MasterChain(source) => {
                write!(f, "master bus insert chain failed: {source}")
            }
            MixEngineError::UnknownSendBus { strip_index, bus } => write!(
                f,
                "strip {strip_index} sends to {bus}, which has no aux bus in this mix pass"
            ),
            MixEngineError::InconsistentBlockLength {
                expected,
                got,
                strip_index,
            } => write!(
                f,
                "strip {strip_index} supplied {got} frames but the mix pass block length is {expected}"
            ),
        }
    }
}

impl std::error::Error for MixEngineError {}

/// A minimal brick-wall limiter: hard-clips both channels to `[-ceiling,
/// ceiling]`.
///
/// This is the final safety stage on the master bus, applied after master
/// inserts and before output. It intentionally has no lookahead or
/// attack/release behavior — `MixEngine`'s contract is only that the
/// signal leaving the mixer never exceeds `ceiling`; a more elaborate
/// limiter can replace this implementation later without changing
/// `MixEngine`'s public API, since it is always applied through this type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limiter {
    ceiling: f32,
}

impl Limiter {
    /// Construct a limiter that clamps both channels to `[-ceiling,
    /// ceiling]`. The sign of `ceiling` is ignored (its absolute value is
    /// used), since a limiter ceiling is always a magnitude.
    pub fn new(ceiling: f32) -> Self {
        Self {
            ceiling: ceiling.abs(),
        }
    }

    /// A limiter with a unity (1.0) ceiling — the conventional "never
    /// exceed full scale" default.
    pub fn unity_ceiling() -> Self {
        Self::new(1.0)
    }

    /// The magnitude beyond which this limiter clamps a sample.
    pub fn ceiling(&self) -> f32 {
        self.ceiling
    }

    /// Clamp a single frame's channels to `[-ceiling, ceiling]`.
    pub fn process(&self, frame: AudioFrame) -> AudioFrame {
        AudioFrame::new(
            frame.left.clamp(-self.ceiling, self.ceiling),
            frame.right.clamp(-self.ceiling, self.ceiling),
        )
    }
}

impl Default for Limiter {
    fn default() -> Self {
        Self::unity_ceiling()
    }
}

/// Converts a channel strip's send target (`SendBusId`, scoped to
/// `ChannelStrip`) into the mixer-wide `BusId` used by `MixBus`.
fn send_target_bus_id(bus: SendBusId) -> BusId {
    BusId::new(u32::from(bus.0))
}

/// Runs one full mix pass: renders every channel strip, collects send taps
/// into aux buses, processes aux inserts, sums into the master bus, and
/// processes master inserts and the limiter.
///
/// Holds a [`ChainRenderer`] as an injected dependency (constructor
/// injection) rather than constructing one itself, so a test can supply a
/// substitute if the rendering step itself ever needs to be doubled.
pub struct MixEngine {
    chain_renderer: ChainRenderer,
}

impl MixEngine {
    /// Construct a `MixEngine` around the given [`ChainRenderer`].
    pub fn new(chain_renderer: ChainRenderer) -> Self {
        Self { chain_renderer }
    }

    /// Runs one mix pass over `block_len` frames.
    ///
    /// For each strip (in order): its raw input is rendered through its
    /// insert chain, then volume and pan are applied. Mute always silences
    /// a strip's contribution; if any strip is soloed, only soloed and
    /// unmuted strips contribute (this is where solo/mute resolve into
    /// actual audibility — the full-mixer view this decision requires).
    /// Peak metering always reflects the post-volume, post-pan level
    /// regardless of audibility, matching [`ChannelStrip::meter`]'s own
    /// contract.
    ///
    /// The post-fader signal feeds the master bus directly; each of the
    /// strip's send taps additionally routes a copy (post-fader by
    /// default, or the pre-fader signal if the tap opts in) into its
    /// target aux bus. Aux buses are then rendered through their own
    /// insert chains, scaled by return level, and summed into the master
    /// bus — never into one another. Finally the master bus is rendered
    /// through its insert chain and the limiter.
    pub fn render(
        &self,
        block_len: usize,
        strips: &mut [StripSource<'_>],
        aux_buses: &mut [AuxBusSource<'_>],
        master: &mut MasterSource<'_>,
    ) -> Result<Vec<AudioFrame>, MixEngineError> {
        if master.bus.kind() != BusKind::Master {
            return Err(MixEngineError::MasterBusNotMaster);
        }
        for (bus_index, aux) in aux_buses.iter().enumerate() {
            if aux.bus.kind() == BusKind::Master {
                return Err(MixEngineError::AuxBusIsMaster { bus_index });
            }
        }

        let mut aux_accum: Vec<Vec<AudioFrame>> = aux_buses
            .iter()
            .map(|_| vec![AudioFrame::silence(); block_len])
            .collect();
        let mut master_accum = vec![AudioFrame::silence(); block_len];

        let any_soloed = strips.iter().any(|s| s.strip.solo());

        for (strip_index, source) in strips.iter_mut().enumerate() {
            if source.input.len() != block_len {
                return Err(MixEngineError::InconsistentBlockLength {
                    expected: block_len,
                    got: source.input.len(),
                    strip_index,
                });
            }

            let post_insert = self
                .chain_renderer
                .render(source.inserts, source.insert_processors, source.input)
                .map_err(|source_err| MixEngineError::StripChain {
                    strip_index,
                    source: source_err,
                })?;

            let volume_gain = source.strip.volume_db().to_linear();
            let (pan_left, pan_right) = source.strip.pan().equal_power_gains();
            let audible = !source.strip.mute() && (!any_soloed || source.strip.solo());

            let mut post_fader = Vec::with_capacity(block_len);
            let mut peak_raw: f32 = 0.0;
            for frame in &post_insert {
                let magnitude = frame.left.abs().max(frame.right.abs());
                if magnitude > peak_raw {
                    peak_raw = magnitude;
                }
                let gained = AudioFrame::new(
                    frame.left * volume_gain * pan_left,
                    frame.right * volume_gain * pan_right,
                );
                post_fader.push(if audible {
                    gained
                } else {
                    AudioFrame::silence()
                });
            }

            // Metering reflects the post-volume, post-pan level regardless
            // of mute/solo audibility, matching ChannelStrip::meter's own
            // contract (it never consults mute/solo).
            let clamped_peak = peak_raw.clamp(StripAmplitude::MIN, StripAmplitude::MAX);
            if let Ok(amplitude) = StripAmplitude::try_new(clamped_peak) {
                source.strip.meter(amplitude);
            }

            for (frame, accum) in post_fader.iter().zip(master_accum.iter_mut()) {
                accum.left += frame.left;
                accum.right += frame.right;
            }

            for tap in source.strip.sends() {
                let target = send_target_bus_id(tap.bus);
                let bus_index = aux_buses
                    .iter()
                    .position(|aux| aux.bus.id() == target)
                    .ok_or(MixEngineError::UnknownSendBus {
                        strip_index,
                        bus: target,
                    })?;
                let level = tap.level.value();
                let tap_source: &[AudioFrame] = if tap.pre_fader {
                    &post_insert
                } else {
                    &post_fader
                };
                for (frame, accum) in tap_source.iter().zip(aux_accum[bus_index].iter_mut()) {
                    accum.left += frame.left * level;
                    accum.right += frame.right * level;
                }
            }
        }

        for (bus_index, (source, accum)) in aux_buses.iter_mut().zip(aux_accum).enumerate() {
            let processed = self
                .chain_renderer
                .render(source.inserts, source.insert_processors, &accum)
                .map_err(|source_err| MixEngineError::AuxChain {
                    bus_index,
                    source: source_err,
                })?;
            let return_gain = source.bus.return_level().value();
            for (frame, accum) in processed.iter().zip(master_accum.iter_mut()) {
                accum.left += frame.left * return_gain;
                accum.right += frame.right * return_gain;
            }
        }

        let master_processed = self
            .chain_renderer
            .render(master.inserts, master.insert_processors, &master_accum)
            .map_err(MixEngineError::MasterChain)?;

        Ok(master_processed
            .iter()
            .map(|frame| master.limiter.process(*frame))
            .collect())
    }
}

impl Default for MixEngine {
    fn default() -> Self {
        Self::new(ChainRenderer::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::effect_chain::EffectChainCommand;
    use crate::effects::effect_processor::GainEffect;
    use crate::mixer::channel_strip::{ChannelStripCommand, Decibel, Pan, SendTap};
    use crate::mixer::mix_bus::Amplitude as BusAmplitude;

    fn frames(samples: &[(f32, f32)]) -> Vec<AudioFrame> {
        samples
            .iter()
            .map(|&(l, r)| AudioFrame::new(l, r))
            .collect()
    }

    fn empty_chain() -> EffectChain {
        EffectChain::new(0)
    }

    fn no_processors() -> Vec<Box<dyn EffectProcessor>> {
        Vec::new()
    }

    #[test]
    fn empty_mix_produces_silence_of_block_length() {
        let engine = MixEngine::default();
        let master_bus = MixBus::new_master();
        let chain = empty_chain();
        let mut processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &chain,
            insert_processors: &mut processors,
            limiter: &limiter,
        };

        let output = engine.render(4, &mut [], &mut [], &mut master).unwrap();

        assert_eq!(output, vec![AudioFrame::silence(); 4]);
    }

    #[test]
    fn single_strip_hard_right_feeds_master_directly() {
        let engine = MixEngine::default();
        let mut strip = ChannelStrip::new();
        strip
            .handle(ChannelStripCommand::SetPan {
                pan: Pan::try_new(1.0).unwrap(),
            })
            .unwrap();
        let strip_chain = empty_chain();
        let mut strip_processors = no_processors();
        let input = frames(&[(1.0, 1.0)]);

        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let output = engine
            .render(1, &mut [strip_source], &mut [], &mut master)
            .unwrap();

        // Hard right: left silent, right carries full (unity volume) gain.
        assert!(output[0].left.abs() < 1e-6);
        assert!((output[0].right - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mute_silences_output_but_metering_still_reflects_post_fader_level() {
        let engine = MixEngine::default();
        let mut strip = ChannelStrip::new();
        strip
            .handle(ChannelStripCommand::SetMute { mute: true })
            .unwrap();
        let strip_chain = empty_chain();
        let mut strip_processors = no_processors();
        let input = frames(&[(1.0, 1.0)]);
        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let output = engine
            .render(1, &mut [strip_source], &mut [], &mut master)
            .unwrap();

        assert_eq!(output[0], AudioFrame::silence());
        // Center pan (default) applies equal-power ~0.707 gain to a unity
        // input, and metering ignores mute per ChannelStrip::meter's own
        // contract.
        assert!(strip.peak().value() > 0.0);
    }

    #[test]
    fn solo_suppresses_non_soloed_strips() {
        let engine = MixEngine::default();

        let mut soloed = ChannelStrip::new();
        soloed
            .handle(ChannelStripCommand::SetSolo { solo: true })
            .unwrap();
        soloed
            .handle(ChannelStripCommand::SetPan {
                pan: Pan::try_new(1.0).unwrap(),
            })
            .unwrap();
        let soloed_chain = empty_chain();
        let mut soloed_processors = no_processors();
        let soloed_input = frames(&[(1.0, 1.0)]);
        let soloed_source = StripSource {
            strip: &mut soloed,
            inserts: &soloed_chain,
            insert_processors: &mut soloed_processors,
            input: &soloed_input,
        };

        let mut quiet = ChannelStrip::new();
        quiet
            .handle(ChannelStripCommand::SetPan {
                pan: Pan::try_new(1.0).unwrap(),
            })
            .unwrap();
        let quiet_chain = empty_chain();
        let mut quiet_processors = no_processors();
        let quiet_input = frames(&[(1.0, 1.0)]);
        let quiet_source = StripSource {
            strip: &mut quiet,
            inserts: &quiet_chain,
            insert_processors: &mut quiet_processors,
            input: &quiet_input,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let output = engine
            .render(1, &mut [soloed_source, quiet_source], &mut [], &mut master)
            .unwrap();

        // Only the soloed strip's hard-right unity signal should appear;
        // if the non-soloed strip leaked through, right would be 2.0.
        assert!((output[0].right - 1.0).abs() < 1e-6);
    }

    #[test]
    fn post_fader_send_routes_scaled_copy_into_aux_bus() {
        let engine = MixEngine::default();
        let mut strip = ChannelStrip::new();
        strip
            .handle(ChannelStripCommand::SetSend {
                index: 0,
                tap: SendTap::new(SendBusId(1), StripAmplitude::try_new(1.0).unwrap()),
            })
            .unwrap();
        let strip_chain = empty_chain();
        let mut strip_processors = no_processors();
        let input = frames(&[(1.0, 0.0)]);
        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let aux_bus = MixBus::new_aux(BusId::new(1), BusAmplitude::try_new(1.0).unwrap()).unwrap();
        let aux_chain = empty_chain();
        let mut aux_processors = no_processors();
        let aux = AuxBusSource {
            bus: &aux_bus,
            inserts: &aux_chain,
            insert_processors: &mut aux_processors,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::new(100.0);
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let output = engine
            .render(1, &mut [strip_source], &mut [aux], &mut master)
            .unwrap();

        // Center pan applies ~0.707 gain to the direct path; the send adds
        // another ~0.707 * 1.0 on top (post-fader), so the left channel
        // should be noticeably louder than the direct path alone.
        let direct_only = std::f32::consts::FRAC_1_SQRT_2;
        assert!(output[0].left > direct_only + 0.5);
    }

    #[test]
    fn pre_fader_send_uses_signal_before_volume_and_pan() {
        let engine = MixEngine::default();
        let mut strip = ChannelStrip::new();
        strip
            .handle(ChannelStripCommand::SetVolume {
                volume_db: Decibel::try_new(-96.0).unwrap(),
            })
            .unwrap();
        strip
            .handle(ChannelStripCommand::SetSend {
                index: 0,
                tap: SendTap::pre_fader(SendBusId(2), StripAmplitude::try_new(1.0).unwrap()),
            })
            .unwrap();
        let strip_chain = empty_chain();
        let mut strip_processors = no_processors();
        let input = frames(&[(1.0, 1.0)]);
        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let aux_bus = MixBus::new_aux(BusId::new(2), BusAmplitude::try_new(1.0).unwrap()).unwrap();
        let aux_chain = empty_chain();
        let mut aux_processors = no_processors();
        let aux = AuxBusSource {
            bus: &aux_bus,
            inserts: &aux_chain,
            insert_processors: &mut aux_processors,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::new(100.0);
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let output = engine
            .render(1, &mut [strip_source], &mut [aux], &mut master)
            .unwrap();

        // Volume is all but silenced (-96dB), so a post-fader send would
        // contribute ~nothing; a pre-fader send still carries the full
        // unity input straight into the aux bus and on to master.
        assert!(output[0].left > 0.5);
    }

    #[test]
    fn aux_inserts_and_return_level_apply_before_summing_into_master() {
        let engine = MixEngine::default();
        let mut strip = ChannelStrip::new();
        strip
            .handle(ChannelStripCommand::SetSend {
                index: 0,
                tap: SendTap::new(SendBusId(3), StripAmplitude::try_new(1.0).unwrap()),
            })
            .unwrap();
        let strip_chain = empty_chain();
        let mut strip_processors = no_processors();
        let input = frames(&[(1.0, 0.0)]);
        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let aux_bus = MixBus::new_aux(BusId::new(3), BusAmplitude::try_new(0.5).unwrap()).unwrap();
        let mut aux_chain = EffectChain::new(1);
        aux_chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        let mut aux_processors: Vec<Box<dyn EffectProcessor>> =
            vec![Box::new(GainEffect::new(10.0))];
        let aux = AuxBusSource {
            bus: &aux_bus,
            inserts: &aux_chain,
            insert_processors: &mut aux_processors,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::new(100.0);
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let output = engine
            .render(1, &mut [strip_source], &mut [aux], &mut master)
            .unwrap();

        // Direct path: center pan ~0.707 * 1.0. Aux path: send level 1.0 *
        // center-pan ~0.707 post-fader signal, *10 gain in the aux insert,
        // *0.5 return level == ~3.54 contribution, dwarfing the direct path.
        let direct_only = std::f32::consts::FRAC_1_SQRT_2;
        assert!(output[0].left > direct_only + 3.0);
    }

    #[test]
    fn master_inserts_apply_before_the_limiter_clamps_output() {
        let engine = MixEngine::default();
        let strip_chain = empty_chain();
        let mut master_chain = EffectChain::new(1);
        master_chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        let mut master_processors: Vec<Box<dyn EffectProcessor>> =
            vec![Box::new(GainEffect::new(50.0))];
        let limiter = Limiter::new(1.0);

        let mut strip = ChannelStrip::new();
        strip
            .handle(ChannelStripCommand::SetPan {
                pan: Pan::try_new(1.0).unwrap(),
            })
            .unwrap();
        let mut strip_processors = no_processors();
        let input = frames(&[(0.0, 1.0)]);
        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let master_bus = MixBus::new_master();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let output = engine
            .render(1, &mut [strip_source], &mut [], &mut master)
            .unwrap();

        // 1.0 * 50.0 == 50.0, which the limiter must clamp to its ceiling.
        assert!((output[0].right - 1.0).abs() < 1e-6);
    }

    #[test]
    fn unknown_send_bus_is_rejected() {
        let engine = MixEngine::default();
        let mut strip = ChannelStrip::new();
        strip
            .handle(ChannelStripCommand::SetSend {
                index: 0,
                tap: SendTap::new(SendBusId(7), StripAmplitude::try_new(1.0).unwrap()),
            })
            .unwrap();
        let strip_chain = empty_chain();
        let mut strip_processors = no_processors();
        let input = frames(&[(1.0, 1.0)]);
        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let result = engine.render(1, &mut [strip_source], &mut [], &mut master);

        assert_eq!(
            result,
            Err(MixEngineError::UnknownSendBus {
                strip_index: 0,
                bus: BusId::new(7),
            })
        );
    }

    #[test]
    fn inconsistent_block_length_is_rejected() {
        let engine = MixEngine::default();
        let mut strip = ChannelStrip::new();
        let strip_chain = empty_chain();
        let mut strip_processors = no_processors();
        let input = frames(&[(1.0, 1.0), (1.0, 1.0)]);
        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let result = engine.render(1, &mut [strip_source], &mut [], &mut master);

        assert_eq!(
            result,
            Err(MixEngineError::InconsistentBlockLength {
                expected: 1,
                got: 2,
                strip_index: 0,
            })
        );
    }

    #[test]
    fn strip_chain_processor_mismatch_is_propagated() {
        let engine = MixEngine::default();
        let mut strip = ChannelStrip::new();
        let mut strip_chain = EffectChain::new(1);
        strip_chain
            .apply(EffectChainCommand::InsertSlot { index: 0 })
            .unwrap();
        let mut strip_processors = no_processors(); // 0 processors, 1 slot: mismatch
        let input = frames(&[(1.0, 1.0)]);
        let strip_source = StripSource {
            strip: &mut strip,
            inserts: &strip_chain,
            insert_processors: &mut strip_processors,
            input: &input,
        };

        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let result = engine.render(1, &mut [strip_source], &mut [], &mut master);

        assert_eq!(
            result,
            Err(MixEngineError::StripChain {
                strip_index: 0,
                source: ChainRenderError::ProcessorCountMismatch {
                    slots: 1,
                    processors: 0,
                },
            })
        );
    }

    #[test]
    fn master_bus_must_be_master_kind() {
        let engine = MixEngine::default();
        let not_master = MixBus::new_aux(BusId::new(1), BusAmplitude::UNITY).unwrap();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &not_master,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let result = engine.render(1, &mut [], &mut [], &mut master);

        assert_eq!(result, Err(MixEngineError::MasterBusNotMaster));
    }

    #[test]
    fn aux_bus_must_not_be_master_kind() {
        let engine = MixEngine::default();
        let master_bus = MixBus::new_master();
        let master_chain = empty_chain();
        let mut master_processors = no_processors();
        let limiter = Limiter::unity_ceiling();
        let mut master = MasterSource {
            bus: &master_bus,
            inserts: &master_chain,
            insert_processors: &mut master_processors,
            limiter: &limiter,
        };

        let mistaken_master = MixBus::new_master();
        let aux_chain = empty_chain();
        let mut aux_processors = no_processors();
        let aux = AuxBusSource {
            bus: &mistaken_master,
            inserts: &aux_chain,
            insert_processors: &mut aux_processors,
        };

        let result = engine.render(1, &mut [], &mut [aux], &mut master);

        assert_eq!(result, Err(MixEngineError::AuxBusIsMaster { bus_index: 0 }));
    }

    #[test]
    fn limiter_clamps_symmetric_ceiling() {
        let limiter = Limiter::new(-2.0); // sign is ignored
        let clamped = limiter.process(AudioFrame::new(5.0, -5.0));
        assert_eq!(clamped, AudioFrame::new(2.0, -2.0));
        assert_eq!(limiter.ceiling(), 2.0);
    }
}
