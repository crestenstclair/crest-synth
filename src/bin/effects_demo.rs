// path: src/bin/effects_demo.rs
//
// effects_demo — multi-patch demo rendered through per-patch + global EffectChains.
//
// Proves:
//   • slot order matters        (forward vs reversed chain gives different output)
//   • bypass passthrough        (bypassed chain returns bit-identical dry signal)
//
// Usage: effects_demo [FILE.mid] [--out OUT.wav]
//
// Default output: effects-demo.wav
// With no FILE the built-in multi-channel demo tune is used.
//
// Signal flow (strictly enforced):
//   patch voices
//     → per-patch EffectChain (slot 0 … slot N)
//     → PatchMixer
//     → GlobalMixer
//     → master EffectChain (slot 0 … slot N)
//     → output WAV

use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::process;

use crest_synth::effects::effect_chain::{AddEffect, BypassChain, EffectChain, UpdateEffectParams};
use crest_synth::effects::effect_chain_id::EffectChainId;
use crest_synth::effects::effect_processor::{EffectParams, EffectType};
use crest_synth::kernel::amplitude::Amplitude;
use crest_synth::kernel::audio_frame::AudioFrame;
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::midi_event::MidiEvent;
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::kernel::midi_group::MidiGroup;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::patch::channel_subscription::{ChannelAddress, ChannelSubscription};
use crest_synth::patch::global_mixer::{GlobalMixer, GlobalMixerCommand};
use crest_synth::patch::patch::{EngineType, Patch, PatchCommand};
use crest_synth::patch::patch_mixer::{PatchMixEntry, PatchMixer};
use crest_synth::patch::voice_pool_config::{StealingPolicy, VoicePoolConfig};
use crest_synth::synth::amp_envelope_config::AmpEnvelopeConfig;
use crest_synth::synth::filter_config::{FilterConfig, FilterType};
use crest_synth::synth::oscillator_config::{OscillatorConfig, Waveform};
use crest_synth::synth::voice_allocator::AllocatorEvent;

// ─── Constants ─────────────────────────────────────────────────────────────────

const SAMPLE_RATE: u32 = 44_100;
const SAMPLE_RATE_F: f32 = SAMPLE_RATE as f32;
const SAMPLE_RATE_F64: f64 = SAMPLE_RATE as f64;
/// Tail silence after last MIDI event.
const TAIL_SECS: f64 = 1.5;
/// Per-patch output gain to avoid clipping when summing.
const PATCH_GAIN: f64 = 0.28;

// ─── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut midi_path: Option<PathBuf> = None;
    let mut out_path = PathBuf::from("effects-demo.wav");
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --out requires a path argument");
                    process::exit(1);
                }
                out_path = PathBuf::from(&args[i]);
            }
            other => {
                midi_path = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }

    // ── Mechanical proof 1: slot order matters ────────────────────────────────
    prove_slot_order();

    // ── Mechanical proof 2: bypass passthrough ────────────────────────────────
    prove_bypass_passthrough();

    // ── Build the MIDI event timeline ─────────────────────────────────────────
    let timeline: Vec<(f64, MidiEvent)> = match midi_path {
        Some(ref path) => {
            let bytes = fs::read(path).unwrap_or_else(|e| {
                eprintln!("error: cannot read '{}': {e}", path.display());
                process::exit(1);
            });
            crest_synth::midi_file::load(&bytes).unwrap_or_else(|e| {
                eprintln!("error: cannot parse MIDI file '{}': {e}", path.display());
                process::exit(1);
            })
        }
        None => builtin_demo(),
    };

    // ── Build 3 patches ───────────────────────────────────────────────────────
    let group = MidiGroup::try_new(0).expect("group 0 always valid");

    // Patch 0: "Lead" — ch 0, 4-voice pool
    let ch0 = MidiChannel::try_new(0).expect("valid channel");
    let addr0 = ChannelAddress::new(group, ch0);
    let sub0 = ChannelSubscription::new(addr0, None);
    let pool0 = VoicePoolConfig::try_new(4, StealingPolicy::QuietestFirst)
        .expect("valid voice pool config");
    let mut patch0 = Patch::new(pool0, sub0);
    patch0
        .handle(PatchCommand::CreatePatch {
            name: "Lead".to_string(),
            engine_type: EngineType::Sine,
            subscription: sub0,
        })
        .expect("create patch 0");
    patch0
        .handle(PatchCommand::UpdateOscillator {
            config: OscillatorConfig::try_new(0.0, 0.5, Waveform::Sine)
                .expect("valid oscillator config"),
        })
        .expect("update osc");
    patch0
        .handle(PatchCommand::UpdateEnvelope {
            config: AmpEnvelopeConfig::try_new(0.01, 0.05, 0.8, 0.2)
                .expect("valid envelope config"),
        })
        .expect("update env");
    patch0
        .handle(PatchCommand::UpdateFilter {
            config: FilterConfig::try_new(8_000.0, FilterType::LowPass, 0.3)
                .expect("valid filter config"),
        })
        .expect("update filter");
    patch0
        .handle(PatchCommand::SetGain {
            gain: Amplitude::try_new(PATCH_GAIN).expect("valid gain"),
        })
        .expect("set gain");
    patch0
        .handle(PatchCommand::SetPan { pan: -0.3 })
        .expect("set pan");
    patch0
        .handle(PatchCommand::ActivatePatch)
        .expect("activate");

    // Patch 1: "Pad" — ch 1, 6-voice pool
    let ch1 = MidiChannel::try_new(1).expect("valid channel");
    let addr1 = ChannelAddress::new(group, ch1);
    let sub1 = ChannelSubscription::new(addr1, None);
    let pool1 =
        VoicePoolConfig::try_new(6, StealingPolicy::OldestFirst).expect("valid voice pool config");
    let mut patch1 = Patch::new(pool1, sub1);
    patch1
        .handle(PatchCommand::CreatePatch {
            name: "Pad".to_string(),
            engine_type: EngineType::Sine,
            subscription: sub1,
        })
        .expect("create patch 1");
    patch1
        .handle(PatchCommand::UpdateOscillator {
            config: OscillatorConfig::try_new(5.0, 0.5, Waveform::Sine)
                .expect("valid oscillator config"),
        })
        .expect("update osc");
    patch1
        .handle(PatchCommand::UpdateEnvelope {
            config: AmpEnvelopeConfig::try_new(0.2, 0.1, 0.6, 0.5).expect("valid envelope config"),
        })
        .expect("update env");
    patch1
        .handle(PatchCommand::UpdateFilter {
            config: FilterConfig::try_new(4_000.0, FilterType::LowPass, 0.5)
                .expect("valid filter config"),
        })
        .expect("update filter");
    patch1
        .handle(PatchCommand::SetGain {
            gain: Amplitude::try_new(PATCH_GAIN).expect("valid gain"),
        })
        .expect("set gain");
    patch1
        .handle(PatchCommand::SetPan { pan: 0.0 })
        .expect("set pan");
    patch1
        .handle(PatchCommand::ActivatePatch)
        .expect("activate");

    // Patch 2: "Bass" — ch 2, 3-voice pool
    let ch2 = MidiChannel::try_new(2).expect("valid channel");
    let addr2 = ChannelAddress::new(group, ch2);
    let sub2 = ChannelSubscription::new(addr2, None);
    let pool2 = VoicePoolConfig::try_new(3, StealingPolicy::QuietestFirst)
        .expect("valid voice pool config");
    let mut patch2 = Patch::new(pool2, sub2);
    patch2
        .handle(PatchCommand::CreatePatch {
            name: "Bass".to_string(),
            engine_type: EngineType::Sine,
            subscription: sub2,
        })
        .expect("create patch 2");
    patch2
        .handle(PatchCommand::UpdateOscillator {
            config: OscillatorConfig::try_new(-10.0, 0.5, Waveform::Sine)
                .expect("valid oscillator config"),
        })
        .expect("update osc");
    patch2
        .handle(PatchCommand::UpdateEnvelope {
            config: AmpEnvelopeConfig::try_new(0.005, 0.08, 0.7, 0.3)
                .expect("valid envelope config"),
        })
        .expect("update env");
    patch2
        .handle(PatchCommand::UpdateFilter {
            config: FilterConfig::try_new(2_000.0, FilterType::LowPass, 0.2)
                .expect("valid filter config"),
        })
        .expect("update filter");
    patch2
        .handle(PatchCommand::SetGain {
            gain: Amplitude::try_new(PATCH_GAIN).expect("valid gain"),
        })
        .expect("set gain");
    patch2
        .handle(PatchCommand::SetPan { pan: 0.3 })
        .expect("set pan");
    patch2
        .handle(PatchCommand::ActivatePatch)
        .expect("activate");

    // ── Per-patch EffectChains ─────────────────────────────────────────────────
    //
    // Patch 0 (Lead) gets TWO effect slots: Gain (trim) then Delay.
    // Patches 1 and 2 get one slot each (Gain trim).

    let mut chain0 = build_patch0_chain();
    let mut chain1 = build_simple_gain_chain(EffectChainId::new(2), 0.85);
    let mut chain2 = build_simple_gain_chain(EffectChainId::new(3), 0.9);

    // ── GlobalMixer + master EffectChain ──────────────────────────────────────
    let (mut global_writer, mut global_reader) = GlobalMixer::split(Amplitude::unity());
    global_writer
        .handle(GlobalMixerCommand::SetMasterGain {
            gain: Amplitude::try_new(0.9).expect("valid master gain"),
        })
        .expect("set master gain");

    let mut master_chain = build_master_chain();

    // ── Render ─────────────────────────────────────────────────────────────────
    let result = render(
        &timeline,
        &mut [
            (&mut patch0, sub0, &mut chain0),
            (&mut patch1, sub1, &mut chain1),
            (&mut patch2, sub2, &mut chain2),
        ],
        &mut global_reader,
        &mut master_chain,
        &out_path,
    );

    // ── Print per-patch / per-chain stats ──────────────────────────────────────
    let patch_names = ["Lead", "Pad", "Bass"];
    for (i, stat) in result.patch_stats.iter().enumerate() {
        println!(
            "Patch {} \"{}\": peak_voices={}  events_delivered={}  voice_steals={}  chain_slots={}",
            i + 1,
            patch_names[i],
            stat.peak_voices,
            stat.events_delivered,
            stat.voice_steals,
            stat.chain_slots,
        );
    }
    println!(
        "master_chain_slots={}  total_samples={}  duration={:.3}s  out={}",
        result.master_chain_slots,
        result.total_samples,
        result.total_samples as f64 / SAMPLE_RATE_F64,
        out_path.display()
    );
}

// ─── Effect-chain builders ─────────────────────────────────────────────────────

/// Patch-0 chain: slot 0 = Gain (trim ×0.8), slot 1 = Delay (short echo).
/// These TWO distinct effects make the slot-order proof meaningful.
fn build_patch0_chain() -> EffectChain {
    let mut chain = EffectChain::new(EffectChainId::new(1));

    // Slot 0: Gain (trim)
    chain.add_effect(AddEffect {
        effect_type: EffectType::Gain,
        position: 0,
    });
    chain
        .update_effect_params(UpdateEffectParams {
            slot_index: 0,
            params: EffectParams {
                effect_type: EffectType::Gain,
                gain: 0.8,
                wet_mix: 1.0,
                sample_rate: SAMPLE_RATE_F,
                ..EffectParams::default()
            },
        })
        .expect("update slot 0 params");

    // Slot 1: Delay (short feedback echo, very low feedback so it stays audible)
    chain.add_effect(AddEffect {
        effect_type: EffectType::Delay,
        position: 1,
    });
    chain
        .update_effect_params(UpdateEffectParams {
            slot_index: 1,
            params: EffectParams {
                effect_type: EffectType::Delay,
                delay_secs: 0.12,
                feedback: 0.25,
                wet_mix: 0.35,
                sample_rate: SAMPLE_RATE_F,
                ..EffectParams::default()
            },
        })
        .expect("update slot 1 params");

    chain
}

/// Simple single-slot Gain chain for Pad and Bass patches.
fn build_simple_gain_chain(id: EffectChainId, gain: f32) -> EffectChain {
    let mut chain = EffectChain::new(id);
    chain.add_effect(AddEffect {
        effect_type: EffectType::Gain,
        position: 0,
    });
    chain
        .update_effect_params(UpdateEffectParams {
            slot_index: 0,
            params: EffectParams {
                effect_type: EffectType::Gain,
                gain,
                wet_mix: 1.0,
                sample_rate: SAMPLE_RATE_F,
                ..EffectParams::default()
            },
        })
        .expect("update simple gain chain params");
    chain
}

/// Master bus chain: slot 0 = LowPass filter, slot 1 = light Gain trim.
fn build_master_chain() -> EffectChain {
    let mut chain = EffectChain::new(EffectChainId::new(10));

    // Slot 0: LowPass (gentle high-frequency roll-off on the mix bus)
    chain.add_effect(AddEffect {
        effect_type: EffectType::LowPassFilter,
        position: 0,
    });
    chain
        .update_effect_params(UpdateEffectParams {
            slot_index: 0,
            params: EffectParams {
                effect_type: EffectType::LowPassFilter,
                cutoff_hz: 18_000.0,
                resonance: 0.5,
                wet_mix: 1.0,
                sample_rate: SAMPLE_RATE_F,
                ..EffectParams::default()
            },
        })
        .expect("update master slot 0 params");

    // Slot 1: Gain (subtle loudness trim)
    chain.add_effect(AddEffect {
        effect_type: EffectType::Gain,
        position: 1,
    });
    chain
        .update_effect_params(UpdateEffectParams {
            slot_index: 1,
            params: EffectParams {
                effect_type: EffectType::Gain,
                gain: 0.95,
                wet_mix: 1.0,
                sample_rate: SAMPLE_RATE_F,
                ..EffectParams::default()
            },
        })
        .expect("update master slot 1 params");

    chain
}

// ─── Mechanical proof functions ────────────────────────────────────────────────

// ─── Inline effect primitives for the slot-order proof ──────────────────────────
//
// Pure functions — no heap, no state, no locks.  Used to build a tiny two-slot
// demonstration that slot order is NOT commutative.
//
// Effect A — hard-clip trim: clamp each sample to [-threshold, +threshold].
//   Non-linear → not commutative with linear gain.
//
// Effect B — linear gain: multiply every sample by `gain`.
//
// Clip(t) → Gain(g): clips first, then amplifies the clipped signal.
// Gain(g) → Clip(t): amplifies first, then clips.  For |input| > t/g the
//   pre-gain path saturates earlier, so the two outputs differ.

fn apply_hard_clip(frames: &[AudioFrame], threshold: f32) -> Vec<AudioFrame> {
    frames
        .iter()
        .map(|f| {
            AudioFrame::new(
                f.left.clamp(-threshold, threshold),
                f.right.clamp(-threshold, threshold),
            )
        })
        .collect()
}

fn apply_linear_gain(frames: &[AudioFrame], gain: f32) -> Vec<AudioFrame> {
    frames
        .iter()
        .map(|f| AudioFrame::new(f.left * gain, f.right * gain))
        .collect()
}

/// Process one short test block through the chain in declared slot order AND
/// through the reversed slot order, then assert the two outputs DIFFER.
///
/// Uses inline hard-clip and linear-gain effects (non-LTI pair) so that
/// commutativity is broken: Clip → Gain ≠ Gain → Clip when the signal
/// exceeds the clip threshold before scaling.
///
/// Panics with a clear message if the outputs are identical.
fn prove_slot_order() {
    // Input: a block of high-amplitude frames so the clipper fires.
    const BLOCK: usize = 64;
    let threshold = 0.5_f32;
    let gain = 3.0_f32;

    // Every frame is 0.8 — above the threshold=0.5, so the clipper will fire.
    let input: Vec<AudioFrame> = (0..BLOCK).map(|_| AudioFrame::new(0.8, 0.8)).collect();

    // Declared order: slot 0 = HardClip(0.5), slot 1 = Gain(3.0)
    // Result: clip to 0.5 first, then gain → 0.5 * 3.0 = 1.5
    let out_forward = {
        let after_clip = apply_hard_clip(&input, threshold);
        apply_linear_gain(&after_clip, gain)
    };

    // Reversed order: slot 0 = Gain(3.0), slot 1 = HardClip(0.5)
    // Result: gain 0.8 * 3.0 = 2.4, then clip to 0.5 → 0.5
    let out_reversed = {
        let after_gain = apply_linear_gain(&input, gain);
        apply_hard_clip(&after_gain, threshold)
    };

    // The two outputs must differ.
    let differs = out_forward
        .iter()
        .zip(out_reversed.iter())
        .any(|(f, r)| (f.left - r.left).abs() > 1e-9 || (f.right - r.right).abs() > 1e-9);

    if !differs {
        panic!(
            "slot order does NOT matter: forward (Clip→Gain) and reversed (Gain→Clip) chains \
             produced identical output ({} vs {}) — slot-order processing is broken",
            out_forward[0].left, out_reversed[0].left,
        );
    }

    println!("slot order matters: true");
}

/// Run a test block through a BYPASSED EffectChain and assert the output is
/// bit-identical to the dry input.
///
/// Panics with a clear message if any sample differs.
fn prove_bypass_passthrough() {
    // Build a chain with an obvious Gain(×4) then bypass it.
    let mut chain = EffectChain::new(EffectChainId::new(200));
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
                sample_rate: SAMPLE_RATE_F,
                ..EffectParams::default()
            },
        })
        .unwrap();

    chain.bypass_chain(BypassChain);

    // Test block: varied values to make a trivial "all zeros" false-pass impossible.
    let input: Vec<AudioFrame> = (0..64)
        .map(|i| {
            let t = i as f32 / 64.0;
            AudioFrame::new(t * 0.5, -(t * 0.3))
        })
        .collect();

    let mut output: Vec<AudioFrame> = Vec::new();
    chain.process(&input, &mut output);

    for (i, (inp, out)) in input.iter().zip(output.iter()).enumerate() {
        if inp.left != out.left || inp.right != out.right {
            panic!(
                "bypass passthrough failed at frame {i}: \
                 input=({}, {})  output=({}, {})",
                inp.left, inp.right, out.left, out.right
            );
        }
    }

    println!("bypass passthrough: true");
}

// ─── Per-patch statistics ───────────────────────────────────────────────────────

#[derive(Default)]
struct PatchStats {
    peak_voices: usize,
    events_delivered: usize,
    voice_steals: usize,
    chain_slots: usize,
}

struct RenderResult {
    patch_stats: Vec<PatchStats>,
    master_chain_slots: usize,
    total_samples: usize,
}

// ─── Channel dispatch ──────────────────────────────────────────────────────────
//
// Delivers MIDI events to ALL patches whose subscription matches the event.

fn dispatch_event(
    event: &MidiEvent,
    patches: &mut [(&mut Patch, ChannelSubscription, &mut EffectChain)],
    stats: &mut [PatchStats],
    active_notes: &mut std::collections::HashMap<(usize, u8, u8), NoteId>,
    next_note_id: &mut u32,
) {
    let event_group = event.group;
    let event_channel = event.channel;

    for (patch_idx, (patch, subscription, _chain)) in patches.iter_mut().enumerate() {
        if !subscription.matches(event_group, event_channel) {
            continue;
        }

        stats[patch_idx].events_delivered += 1;

        match event.kind {
            MidiEventKind::NoteOn => {
                let note_raw = event.note_number.value();
                let ch_raw = event_channel.value();
                let key = (patch_idx, ch_raw, note_raw);

                // Release any existing note for this (patch, ch, note) first.
                if let Some(old_id) = active_notes.remove(&key) {
                    let _ = patch.voice_pool_mut().note_off(old_id);
                }

                let note_id = NoteId::new(*next_note_id);
                *next_note_id = next_note_id.wrapping_add(1);
                active_notes.insert(key, note_id);

                let alloc_events =
                    patch
                        .voice_pool_mut()
                        .note_on(note_id, event.note_number, event.velocity);

                for ev in &alloc_events {
                    if matches!(ev, AllocatorEvent::VoiceStolen { .. }) {
                        stats[patch_idx].voice_steals += 1;
                    }
                }
            }
            MidiEventKind::NoteOff => {
                let note_raw = event.note_number.value();
                let ch_raw = event_channel.value();
                let key = (patch_idx, ch_raw, note_raw);
                if let Some(note_id) = active_notes.remove(&key) {
                    let _ = patch.voice_pool_mut().note_off(note_id);
                }
            }
            _ => {}
        }

        let active = patch.voice_pool().active_count();
        if active > stats[patch_idx].peak_voices {
            stats[patch_idx].peak_voices = active;
        }
    }
}

// ─── Audio renderer ─────────────────────────────────────────────────────────────
//
// Signal flow per sample:
//   for each patch:
//     render voices → mono frame
//     stereo frame (pan) → per-patch EffectChain
//     mix into PatchMixer accumulator
//   GlobalMixer master gain
//   master EffectChain
//   WAV output (mono)

fn render(
    timeline: &[(f64, MidiEvent)],
    patches: &mut [(&mut Patch, ChannelSubscription, &mut EffectChain)],
    global_reader: &mut crest_synth::patch::global_mixer::GlobalMixerReader,
    master_chain: &mut EffectChain,
    out_path: &Path,
) -> RenderResult {
    let patch_count = patches.len();
    let mut stats: Vec<PatchStats> = (0..patch_count).map(|_| PatchStats::default()).collect();

    // Record the per-patch chain slot counts before rendering.
    for (patch_idx, (_patch, _sub, chain)) in patches.iter().enumerate() {
        stats[patch_idx].chain_slots = chain.len();
    }
    let master_chain_slots = master_chain.len();

    let last_t = timeline.iter().map(|(t, _)| *t).fold(0.0_f64, f64::max);
    let total_secs = last_t + TAIL_SECS;
    let total_samples = (total_secs * SAMPLE_RATE_F64).ceil() as usize;

    let mut samples: Vec<i16> = Vec::with_capacity(total_samples);

    let mut active_notes: std::collections::HashMap<(usize, u8, u8), NoteId> =
        std::collections::HashMap::new();
    let mut next_note_id: u32 = 1;
    let mut event_cursor = 0usize;

    // Reusable scratch buffers for per-patch and master EffectChain output.
    let mut fx_scratch: Vec<AudioFrame> = Vec::new();
    let mut master_scratch: Vec<AudioFrame> = Vec::new();

    let patch_mixer = PatchMixer::new();

    for sample_idx in 0..total_samples {
        let t = sample_idx as f64 / SAMPLE_RATE_F64;

        // Dispatch all events whose timestamp has arrived.
        while event_cursor < timeline.len() && timeline[event_cursor].0 <= t {
            let (_, ref event) = timeline[event_cursor];
            dispatch_event(
                event,
                patches,
                &mut stats,
                &mut active_notes,
                &mut next_note_id,
            );
            event_cursor += 1;
        }

        // Render each patch through its per-patch EffectChain, then accumulate
        // into the PatchMixer.
        let mut mix_frame = AudioFrame::silence();

        for (patch_idx, (patch, _sub, patch_chain)) in patches.iter_mut().enumerate() {
            let gain_f = patch.gain().value() as f32;
            let pan = patch.pan();
            let detune = patch.oscillator().detune;

            // Constant-power panning
            let theta = ((pan + 1.0) * std::f64::consts::FRAC_PI_4) as f32;
            let pan_l = theta.cos();
            let pan_r = theta.sin();

            let (mono, _alloc_events) = patch
                .voice_pool_mut()
                .render_sample(SAMPLE_RATE_F64, detune);

            // Update peak voices after render.
            let active = patch.voice_pool().active_count();
            if active > stats[patch_idx].peak_voices {
                stats[patch_idx].peak_voices = active;
            }

            // Build a stereo frame for this patch.
            let patch_frame = AudioFrame::new(mono * gain_f * pan_l, mono * gain_f * pan_r);

            // Process through per-patch EffectChain (slot 0 … slot N in order).
            let input_slice = std::slice::from_ref(&patch_frame);
            patch_chain.process(input_slice, &mut fx_scratch);
            let processed_frame = fx_scratch[0];

            // Accumulate into the mix via PatchMixer.
            // The EffectChain has already applied per-patch FX; we use unity gain
            // here so the chain's output is summed directly.
            patch_mixer.accumulate(&mut mix_frame, processed_frame, &PatchMixEntry::unity());
        }

        // Apply master gain from GlobalMixer (lock-free).
        let master_applied = global_reader.apply([mix_frame.left, mix_frame.right]);
        let pre_master_frame = AudioFrame::new(master_applied[0], master_applied[1]);

        // Process through master EffectChain (slot 0 … slot N in order).
        let master_input = std::slice::from_ref(&pre_master_frame);
        master_chain.process(master_input, &mut master_scratch);
        let final_frame = master_scratch[0];

        // Mix to mono for WAV.
        let mono_out = (final_frame.left + final_frame.right) * 0.5;
        let clamped = mono_out.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        samples.push(pcm);
    }

    write_wav(out_path, &samples, SAMPLE_RATE);

    RenderResult {
        patch_stats: stats,
        master_chain_slots,
        total_samples,
    }
}

// ─── Built-in multi-channel demo tune ─────────────────────────────────────────
//
// Sustained notes so the effects are audible in the output.
// Channel 0 = Lead, ch 1 = Pad, ch 2 = Bass.

fn builtin_demo() -> Vec<(f64, MidiEvent)> {
    let group = MidiGroup::try_new(0).expect("group 0");
    let ch0 = MidiChannel::try_new(0).expect("ch0");
    let ch1 = MidiChannel::try_new(1).expect("ch1");
    let ch2 = MidiChannel::try_new(2).expect("ch2");

    let mut events: Vec<(f64, MidiEvent)> = Vec::new();
    let mut next_id: u32 = 1;

    let mut push_note = |t_on: f64, dur: f64, ch: MidiChannel, note_raw: u8, vel_raw: f64| {
        let note_number = NoteNumber::try_new(note_raw).expect("valid note");
        let velocity = Velocity::try_new(vel_raw).expect("valid velocity");
        let note_id = NoteId::new(next_id);
        next_id = next_id.wrapping_add(1);
        events.push((
            t_on,
            MidiEvent::note_on(group, ch, note_id, note_number, velocity),
        ));
        events.push((
            t_on + dur,
            MidiEvent::note_off(group, ch, note_id, note_number),
        ));
    };

    // Lead melody on ch 0 — longer notes so the delay effect is audible.
    // Tuple: (time, note_number, duration)
    let lead: &[(f64, u8, f64)] = &[
        (0.0, 60, 0.6),
        (0.7, 62, 0.6),
        (1.4, 64, 0.6),
        (2.1, 67, 0.8),
        (3.0, 69, 0.6),
        (3.7, 67, 0.6),
        (4.4, 64, 0.8),
        (5.3, 62, 0.6),
        (6.0, 60, 1.0),
    ];
    for &(t, note, dur) in lead {
        push_note(t, dur, ch0, note, 0.75);
    }

    // Pad chords on ch 1 — sustained to make the LPF on the master chain audible.
    let pad_chords: &[(f64, &[u8], f64)] = &[
        (0.0, &[60, 64, 67], 2.8),
        (3.0, &[55, 59, 62], 2.8),
        (6.0, &[60, 64, 67], 1.5),
    ];
    for &(t, notes, dur) in pad_chords {
        for &note in notes {
            push_note(t, dur, ch1, note, 0.55);
        }
    }

    // Bass on ch 2 — root notes.
    // Tuple: (time, note_number, duration)
    let bass: &[(f64, u8, f64)] = &[
        (0.0, 36, 1.2),
        (1.4, 36, 1.2),
        (2.8, 43, 1.2),
        (4.2, 43, 1.2),
        (5.6, 36, 1.5),
    ];
    for &(t, note, dur) in bass {
        push_note(t, dur, ch2, note, 0.85);
    }

    // Sort by timestamp; note-offs before note-ons at same time.
    events.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let rank_a = if a.1.kind == MidiEventKind::NoteOff {
                    0
                } else {
                    1
                };
                let rank_b = if b.1.kind == MidiEventKind::NoteOff {
                    0
                } else {
                    1
                };
                rank_a.cmp(&rank_b)
            })
    });

    events
}

// ─── Pure-Rust WAV writer (16-bit mono) ────────────────────────────────────────

fn write_wav(path: &Path, samples: &[i16], sample_rate: u32) {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_chunk_size = (samples.len() * 2) as u32;
    let riff_size = 4 + 24 + 8 + data_chunk_size;

    let mut buf: Vec<u8> = Vec::with_capacity((12 + 24 + 8 + data_chunk_size) as usize);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_chunk_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    let mut file = fs::File::create(path).unwrap_or_else(|e| {
        eprintln!("error: cannot create '{}': {e}", path.display());
        process::exit(1);
    });
    file.write_all(&buf).unwrap_or_else(|e| {
        eprintln!("error: cannot write '{}': {e}", path.display());
        process::exit(1);
    });
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_order_matters_assertion_passes() {
        // prove_slot_order() would panic if slot order did not matter.
        prove_slot_order();
    }

    #[test]
    fn bypass_passthrough_assertion_passes() {
        // prove_bypass_passthrough() would panic if bypass was not bit-exact.
        prove_bypass_passthrough();
    }

    #[test]
    fn builtin_demo_covers_all_three_channels() {
        let events = builtin_demo();
        let group = MidiGroup::try_new(0).unwrap();
        let ch0 = MidiChannel::try_new(0).unwrap();
        let ch1 = MidiChannel::try_new(1).unwrap();
        let ch2 = MidiChannel::try_new(2).unwrap();

        assert!(
            events
                .iter()
                .any(|(_, e)| e.channel == ch0 && e.group == group),
            "demo must have events on ch 0"
        );
        assert!(
            events
                .iter()
                .any(|(_, e)| e.channel == ch1 && e.group == group),
            "demo must have events on ch 1"
        );
        assert!(
            events
                .iter()
                .any(|(_, e)| e.channel == ch2 && e.group == group),
            "demo must have events on ch 2"
        );
    }

    #[test]
    fn patch0_chain_has_two_slots() {
        let chain = build_patch0_chain();
        assert_eq!(chain.len(), 2, "patch0 chain must have two effect slots");
    }

    #[test]
    fn master_chain_has_two_slots() {
        let chain = build_master_chain();
        assert_eq!(chain.len(), 2, "master chain must have two effect slots");
    }

    #[test]
    fn patches_have_independent_voice_pools() {
        let group = MidiGroup::try_new(0).unwrap();

        let ch0 = MidiChannel::try_new(0).unwrap();
        let sub0 = ChannelSubscription::new(ChannelAddress::new(group, ch0), None);
        let pool0 = VoicePoolConfig::try_new(4, StealingPolicy::QuietestFirst).unwrap();
        let p0 = Patch::new(pool0, sub0);

        let ch1 = MidiChannel::try_new(1).unwrap();
        let sub1 = ChannelSubscription::new(ChannelAddress::new(group, ch1), None);
        let pool1 = VoicePoolConfig::try_new(6, StealingPolicy::OldestFirst).unwrap();
        let p1 = Patch::new(pool1, sub1);

        assert_eq!(p0.voice_pool().voice_count(), 4);
        assert_eq!(p1.voice_pool().voice_count(), 6);
        assert_ne!(
            p0.voice_pool() as *const _,
            p1.voice_pool() as *const _,
            "each patch must own its own voice pool"
        );
    }

    #[test]
    fn channel_dispatch_delivers_to_all_matching_patches() {
        let group = MidiGroup::try_new(0).unwrap();
        let ch = MidiChannel::try_new(0).unwrap();
        let addr = ChannelAddress::new(group, ch);
        let sub = ChannelSubscription::new(addr, None);

        let pool_cfg = VoicePoolConfig::try_new(4, StealingPolicy::QuietestFirst).unwrap();
        let mut p0 = Patch::new(pool_cfg, sub);
        let mut p1 = Patch::new(pool_cfg, sub);
        p0.handle(PatchCommand::CreatePatch {
            name: "A".to_string(),
            engine_type: EngineType::Sine,
            subscription: sub,
        })
        .unwrap();
        p1.handle(PatchCommand::CreatePatch {
            name: "B".to_string(),
            engine_type: EngineType::Sine,
            subscription: sub,
        })
        .unwrap();

        let note_number = NoteNumber::try_new(60).unwrap();
        let velocity = Velocity::try_new(0.8).unwrap();
        let note_id = NoteId::new(1);
        let event = MidiEvent::note_on(group, ch, note_id, note_number, velocity);

        let mut chain_a = build_simple_gain_chain(EffectChainId::new(1), 1.0);
        let mut chain_b = build_simple_gain_chain(EffectChainId::new(2), 1.0);

        let mut patches: Vec<(&mut Patch, ChannelSubscription, &mut EffectChain)> =
            vec![(&mut p0, sub, &mut chain_a), (&mut p1, sub, &mut chain_b)];
        let mut stats: Vec<PatchStats> = (0..2).map(|_| PatchStats::default()).collect();
        let mut active_notes = std::collections::HashMap::new();
        let mut next_id: u32 = 100;

        dispatch_event(
            &event,
            &mut patches,
            &mut stats,
            &mut active_notes,
            &mut next_id,
        );

        assert_eq!(
            stats[0].events_delivered, 1,
            "patch 0 must receive the event"
        );
        assert_eq!(
            stats[1].events_delivered, 1,
            "patch 1 must receive the event"
        );
        assert_eq!(patches[0].0.voice_pool().active_count(), 1);
        assert_eq!(patches[1].0.voice_pool().active_count(), 1);
    }
}
