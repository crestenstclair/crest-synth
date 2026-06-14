// path: src/bin/patch_play.rs
//
// patch_play — multi-patch MIDI player
//
// Proves: dispatcher → per-patch voice pools → global mix end to end.
//
// Usage: patch_play [FILE.mid] [--out OUT.wav]
//
// With no FILE, a built-in multi-channel demo tune is synthesised.
// Events are routed through a ChannelDispatcher to 3 patches, each on a
// different MIDI channel with its own independent VoiceAllocator pool.

use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::process;

use crest_synth::kernel::amplitude::Amplitude;
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
use crest_synth::patch::voice_pool_config::{StealingPolicy, VoicePoolConfig};
use crest_synth::synth::amp_envelope_config::AmpEnvelopeConfig;
use crest_synth::synth::filter_config::{FilterConfig, FilterType};
use crest_synth::synth::oscillator_config::{OscillatorConfig, Waveform};
use crest_synth::synth::voice_allocator::AllocatorEvent;

// ─── Constants ─────────────────────────────────────────────────────────────────

const SAMPLE_RATE: u32 = 44_100;
const SAMPLE_RATE_F64: f64 = SAMPLE_RATE as f64;
/// Tail silence appended after the last event so notes can fully release.
const TAIL_SECS: f64 = 1.5;
/// Per-patch gain to prevent clipping when summing multiple patches.
const PATCH_GAIN: f64 = 0.28;

// ─── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut midi_path: Option<PathBuf> = None;
    let mut out_path = PathBuf::from("patch-play.wav");
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

    // ── Build 3 patches with distinct settings on distinct channels ───────────
    let group = MidiGroup::try_new(0).expect("group 0 always valid");

    // Patch 1: "Lead" — sine, ch 0, 4-voice pool
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

    // Patch 2: "Pad" — sine (wider envelope), ch 1, 6-voice pool
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

    // Patch 3: "Bass" — sine (deeper envelope), ch 2, 3-voice pool
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

    // ── GlobalMixer ───────────────────────────────────────────────────────────
    let (mut global_writer, mut global_reader) = GlobalMixer::split(Amplitude::unity());
    global_writer
        .handle(GlobalMixerCommand::SetMasterGain {
            gain: Amplitude::try_new(0.9).expect("valid master gain"),
        })
        .expect("set master gain");

    // ── Render ─────────────────────────────────────────────────────────────────
    let result = render(
        &timeline,
        &mut [(&mut patch0, 0u8), (&mut patch1, 1u8), (&mut patch2, 2u8)],
        &mut global_reader,
        &out_path,
    );

    // ── Print per-patch statistics ─────────────────────────────────────────────
    let patch_names = ["Lead", "Pad", "Bass"];
    for (i, stat) in result.patch_stats.iter().enumerate() {
        println!(
            "Patch {} \"{}\": Peak Voices = {}  events_delivered={}  voice_steals={}",
            i + 1,
            patch_names[i],
            stat.peak_voices,
            stat.events_delivered,
            stat.voice_steals,
        );
    }
    println!(
        "total_samples={}  duration={:.3}s  out={}",
        result.total_samples,
        result.total_samples as f64 / SAMPLE_RATE_F64,
        out_path.display()
    );
}

// ─── Per-patch statistics ───────────────────────────────────────────────────────

#[derive(Default)]
struct PatchStats {
    peak_voices: usize,
    events_delivered: usize,
    voice_steals: usize,
}

struct RenderResult {
    patch_stats: Vec<PatchStats>,
    total_samples: usize,
}

// ─── Channel dispatcher ─────────────────────────────────────────────────────────
//
// Delivers every event to ALL patches whose subscription matches the event's
// (group, channel).  Proves: channel dispatch goes to all subscribers, not
// just the first match.

fn dispatch_event(
    event: &MidiEvent,
    patches: &mut [(&mut Patch, u8)],
    stats: &mut [PatchStats],
    // Per-patch note-id tracking: (patch_idx, channel, note_number) -> NoteId
    active_notes: &mut std::collections::HashMap<(usize, u8, u8), NoteId>,
    next_note_id: &mut u32,
) {
    let event_group = event.group;
    let event_channel = event.channel;

    for (patch_idx, (patch, _ch_raw)) in patches.iter_mut().enumerate() {
        let sub = patch.subscription();
        if !sub.matches(event_group, event_channel) {
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

        // Update peak-voice counter for this patch.
        let active = patch.voice_pool().active_count();
        if active > stats[patch_idx].peak_voices {
            stats[patch_idx].peak_voices = active;
        }
    }
}

// ─── Audio renderer ─────────────────────────────────────────────────────────────

fn render(
    timeline: &[(f64, MidiEvent)],
    patches: &mut [(&mut Patch, u8)],
    global_reader: &mut crest_synth::patch::global_mixer::GlobalMixerReader,
    out_path: &Path,
) -> RenderResult {
    let patch_count = patches.len();
    let mut stats: Vec<PatchStats> = (0..patch_count).map(|_| PatchStats::default()).collect();

    let last_t = timeline.iter().map(|(t, _)| *t).fold(0.0_f64, f64::max);
    let total_secs = last_t + TAIL_SECS;
    let total_samples = (total_secs * SAMPLE_RATE_F64).ceil() as usize;

    let mut samples: Vec<i16> = Vec::with_capacity(total_samples);

    // Per-patch active note tracking: (patch_idx, channel, note_number) -> NoteId.
    let mut active_notes: std::collections::HashMap<(usize, u8, u8), NoteId> =
        std::collections::HashMap::new();
    let mut next_note_id: u32 = 1;

    let mut event_cursor = 0usize;

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

        // Render each patch and accumulate into a stereo mix frame.
        let mut left = 0.0_f32;
        let mut right = 0.0_f32;

        for (patch_idx, (patch, _)) in patches.iter_mut().enumerate() {
            let gain_f = patch.gain().value() as f32;
            let pan = patch.pan() as f32;
            let detune = patch.oscillator().detune;
            // Constant-power panning: left = cos(θ), right = sin(θ), θ ∈ [0, π/2]
            let theta = (pan + 1.0) * std::f32::consts::FRAC_PI_4; // map [-1,1] → [0, π/2]
            let pan_l = theta.cos();
            let pan_r = theta.sin();

            let (mono, _alloc_events) = patch
                .voice_pool_mut()
                .render_sample(SAMPLE_RATE_F64, detune);

            // Update peak voices after render (some voices may have finished).
            let active = patch.voice_pool().active_count();
            if active > stats[patch_idx].peak_voices {
                stats[patch_idx].peak_voices = active;
            }

            left += mono * gain_f * pan_l;
            right += mono * gain_f * pan_r;
        }

        // Apply master gain (lock-free read).
        let master = global_reader.apply([left, right]);

        // Mix to mono for the WAV file.
        let mono_out = (master[0] + master[1]) * 0.5;
        let clamped = mono_out.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        samples.push(pcm);
    }

    write_wav(out_path, &samples, SAMPLE_RATE);

    RenderResult {
        patch_stats: stats,
        total_samples,
    }
}

// ─── Built-in multi-channel demo tune ──────────────────────────────────────────
//
// Spreads events across channels 0, 1, 2 so every patch sounds.
// Channel 0 = Lead (melody), channel 1 = Pad (chords), channel 2 = Bass.

fn builtin_demo() -> Vec<(f64, MidiEvent)> {
    let group = MidiGroup::try_new(0).expect("group 0");
    let ch0 = MidiChannel::try_new(0).expect("ch0");
    let ch1 = MidiChannel::try_new(1).expect("ch1");
    let ch2 = MidiChannel::try_new(2).expect("ch2");

    let mut events: Vec<(f64, MidiEvent)> = Vec::new();
    let mut next_id: u32 = 1;

    // Helper to push a note-on/note-off pair.
    let push_note = |t_on: f64,
                     dur: f64,
                     ch: MidiChannel,
                     note_raw: u8,
                     vel_raw: f64,
                     id_counter: &mut u32,
                     out: &mut Vec<(f64, MidiEvent)>| {
        let note_number = NoteNumber::try_new(note_raw).expect("valid note");
        let velocity = Velocity::try_new(vel_raw).expect("valid velocity");
        let note_id = NoteId::new(*id_counter);
        *id_counter = id_counter.wrapping_add(1);
        out.push((
            t_on,
            MidiEvent::note_on(group, ch, note_id, note_number, velocity),
        ));
        out.push((
            t_on + dur,
            MidiEvent::note_off(group, ch, note_id, note_number),
        ));
    };

    // ── Lead melody on ch 0 (notes from C major: C4=60, D4=62, E4=64, G4=67, A4=69)
    let lead: &[(f64, u8, f64)] = &[
        (0.0, 60, 0.35),
        (0.35, 62, 0.35),
        (0.70, 64, 0.35),
        (1.05, 67, 0.5),
        (1.55, 69, 0.35),
        (1.90, 67, 0.35),
        (2.25, 64, 0.5),
        (2.75, 62, 0.35),
        (3.10, 60, 0.6),
    ];
    for &(t, note, dur) in lead {
        push_note(t, dur, ch0, note, 0.75, &mut next_id, &mut events);
    }

    // ── Pad chords on ch 1 (C major and G major)
    let pad_chords: &[(f64, &[u8], f64)] = &[
        (0.0, &[60, 64, 67], 1.4),
        (1.5, &[55, 59, 62], 1.4),
        (3.0, &[60, 64, 67], 0.9),
    ];
    for &(t, notes, dur) in pad_chords {
        for &note in notes {
            push_note(t, dur, ch1, note, 0.55, &mut next_id, &mut events);
        }
    }

    // ── Bass on ch 2 (root notes)
    let bass: &[(f64, u8, f64)] = &[
        (0.0, 36, 0.6),
        (0.7, 36, 0.6),
        (1.4, 43, 0.6),
        (2.1, 43, 0.6),
        (2.8, 36, 0.9),
    ];
    for &(t, note, dur) in bass {
        push_note(t, dur, ch2, note, 0.85, &mut next_id, &mut events);
    }

    // Sort by timestamp; note-offs before note-ons at the same time.
    events.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                // NoteOff < NoteOn at same timestamp
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
    let data_chunk_size = (samples.len() * 2) as u32; // 2 bytes per i16
    let riff_size = 4 + 24 + 8 + data_chunk_size;

    let mut buf: Vec<u8> = Vec::with_capacity((12 + 24 + 8 + data_chunk_size) as usize);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
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

#[cfg(test)]
mod tests {
    use super::*;
    use crest_synth::kernel::midi_group::MidiGroup;

    #[test]
    fn builtin_demo_covers_all_three_channels() {
        let events = builtin_demo();
        let group = MidiGroup::try_new(0).unwrap();
        let ch0 = MidiChannel::try_new(0).unwrap();
        let ch1 = MidiChannel::try_new(1).unwrap();
        let ch2 = MidiChannel::try_new(2).unwrap();

        let has_ch0 = events
            .iter()
            .any(|(_, e)| e.channel == ch0 && e.group == group);
        let has_ch1 = events
            .iter()
            .any(|(_, e)| e.channel == ch1 && e.group == group);
        let has_ch2 = events
            .iter()
            .any(|(_, e)| e.channel == ch2 && e.group == group);

        assert!(has_ch0, "demo must have events on ch 0 (Lead)");
        assert!(has_ch1, "demo must have events on ch 1 (Pad)");
        assert!(has_ch2, "demo must have events on ch 2 (Bass)");
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

        // Different pool sizes → definitely independent allocators.
        assert_eq!(p0.voice_pool().voice_count(), 4);
        assert_eq!(p1.voice_pool().voice_count(), 6);
        assert_ne!(
            p0.voice_pool() as *const _,
            p1.voice_pool() as *const _,
            "each patch must own its own voice pool"
        );
    }

    #[test]
    fn channel_dispatcher_delivers_to_all_matching_patches() {
        // Two patches on the same channel; both must receive the event.
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

        let mut patches: Vec<(&mut Patch, u8)> = vec![(&mut p0, 0), (&mut p1, 0)];
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

    #[test]
    fn builtin_demo_events_are_sorted() {
        let events = builtin_demo();
        for w in events.windows(2) {
            assert!(
                w[0].0 <= w[1].0,
                "events must be sorted by timestamp: {} > {}",
                w[0].0,
                w[1].0
            );
        }
    }
}
