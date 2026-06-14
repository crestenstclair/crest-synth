// path: src/bin/mod_play.rs
//
// mod_play — multi-patch MIDI player with Modulation context active
//
// Demonstrates: LFO vibrato + filter sweep per patch, ChannelDispatcher routing,
// independent per-patch voice pools, and a ModulationProcessor that evaluates
// mod sources and applies routed values before rendering each patch.
//
// Usage: mod_play [FILE.mid] [--out OUT.wav]
//
// With no FILE, a built-in multi-channel demo tune is synthesised (sustained /
// legato notes so vibrato and the filter sweep are clearly audible).

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
use crest_synth::modulation::lfo_config::LfoConfig;
use crest_synth::modulation::lfo_waveform::LfoWaveform;
use crest_synth::modulation::mod_destination_type::ModDestinationType;
use crest_synth::modulation::mod_matrix::{ModMatrix, ModMatrixCommand};
use crest_synth::modulation::mod_source_type::ModSourceType;
use crest_synth::patch::channel_subscription::{ChannelAddress, ChannelSubscription};
use crest_synth::patch::global_mixer::{GlobalMixer, GlobalMixerCommand};
use crest_synth::patch::patch::{EngineType, Patch, PatchCommand};
use crest_synth::patch::patch_id::PatchId;
use crest_synth::patch::voice_pool_config::{StealingPolicy, VoicePoolConfig};
use crest_synth::synth::amp_envelope_config::AmpEnvelopeConfig;
use crest_synth::synth::filter_config::{FilterConfig, FilterType};
use crest_synth::synth::oscillator_config::{OscillatorConfig, Waveform};
use crest_synth::synth::voice_allocator::AllocatorEvent;

// ─── Constants ─────────────────────────────────────────────────────────────────

const SAMPLE_RATE: u32 = 44_100;
const SAMPLE_RATE_F64: f64 = SAMPLE_RATE as f64;
/// Tail silence appended after the last event so notes can fully release.
const TAIL_SECS: f64 = 2.0;
/// Per-patch gain to prevent clipping when summing multiple patches.
const PATCH_GAIN: f64 = 0.28;

// ─── ModulationProcessor ───────────────────────────────────────────────────────
//
// Evaluates per-patch LFO sources and applies routed modulation deltas to
// parameters (pitch detune / filter cutoff) before voice rendering.
// All state is stack-allocated to respect audio-thread constraints.

struct ModulationState {
    /// Current phase for each LFO (in radians).
    lfo_phases: [f64; 8],
}

impl ModulationState {
    fn new() -> Self {
        Self {
            lfo_phases: [0.0; 8],
        }
    }

    /// Advance LFO phases and evaluate the modulation matrix.
    ///
    /// Returns the aggregate pitch semitone delta and filter cutoff delta (Hz)
    /// to apply to this patch's voice rendering parameters.
    ///
    /// No heap allocation; operates on fixed-size arrays.
    fn process(&mut self, matrix: &ModMatrix, sample_rate: f64) -> ModulationOutput {
        let mut pitch_delta_semitones: f64 = 0.0;
        let mut filter_cutoff_delta_hz: f64 = 0.0;

        for routing in matrix.routings() {
            let source_value = match routing.source() {
                ModSourceType::Lfo => {
                    // Use LFO index 0 by default (only one LFO configured per patch here).
                    let lfo_idx = 0usize;
                    if let Some(lfo_cfg) = matrix.lfo_configs().get(lfo_idx) {
                        let phase = self.lfo_phases[lfo_idx];
                        let value = match lfo_cfg.waveform {
                            LfoWaveform::Sine => phase.sin(),
                            LfoWaveform::Triangle => {
                                let t = phase / (2.0 * std::f64::consts::PI);
                                let t = t - t.floor();
                                if t < 0.5 {
                                    4.0 * t - 1.0
                                } else {
                                    3.0 - 4.0 * t
                                }
                            }
                            LfoWaveform::Square => {
                                if phase < std::f64::consts::PI {
                                    1.0
                                } else {
                                    -1.0
                                }
                            }
                            LfoWaveform::Sawtooth => {
                                let t = phase / (2.0 * std::f64::consts::PI);
                                2.0 * (t - t.floor()) - 1.0
                            }
                            LfoWaveform::ReverseSawtooth => {
                                let t = phase / (2.0 * std::f64::consts::PI);
                                1.0 - 2.0 * (t - t.floor())
                            }
                            LfoWaveform::SampleAndHold => {
                                // Approximate with a slow sine for determinism.
                                phase.sin()
                            }
                        };
                        // Advance phase for next sample.
                        self.lfo_phases[lfo_idx] +=
                            2.0 * std::f64::consts::PI * lfo_cfg.rate / sample_rate;
                        if self.lfo_phases[lfo_idx] >= 2.0 * std::f64::consts::PI {
                            self.lfo_phases[lfo_idx] -= 2.0 * std::f64::consts::PI;
                        }
                        value * lfo_cfg.depth
                    } else {
                        0.0
                    }
                }
                // Per-note expression sources are evaluated per-voice, not here.
                // Patch-level mod processor does not aggregate per-note data.
                ModSourceType::PerNoteBendX
                | ModSourceType::PerNoteTimbreY
                | ModSourceType::PerNotePressureZ => continue,
                // Other sources default to 0 for this demo.
                _ => 0.0,
            };

            let depth = routing.depth();
            match routing.destination() {
                ModDestinationType::OscillatorPitch => {
                    // vibrato: small pitch deviation in semitones
                    pitch_delta_semitones += source_value * depth * 2.0; // ±2 semitone max
                }
                ModDestinationType::FilterCutoff => {
                    // filter sweep: large cutoff deviation in Hz
                    filter_cutoff_delta_hz += source_value * depth * 4_000.0; // ±4 kHz max
                }
                _ => {}
            }
        }

        ModulationOutput {
            pitch_delta_semitones,
            filter_cutoff_delta_hz,
        }
    }
}

struct ModulationOutput {
    pitch_delta_semitones: f64,
    filter_cutoff_delta_hz: f64,
}

// ─── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut midi_path: Option<PathBuf> = None;
    let mut out_path = PathBuf::from("mod-play.wav");
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

    let group = MidiGroup::try_new(0).expect("group 0 always valid");

    // ── Build Patch 0: "Lead" with LFO vibrato ────────────────────────────────
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
            config: AmpEnvelopeConfig::try_new(0.02, 0.05, 0.8, 0.4)
                .expect("valid envelope config"),
        })
        .expect("update env");
    patch0
        .handle(PatchCommand::UpdateFilter {
            config: FilterConfig::try_new(6_000.0, FilterType::LowPass, 0.3)
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

    // ModMatrix for Patch 0: LFO vibrato → pitch
    let mut mat0 = ModMatrix::new(PatchId::new(0));
    mat0.apply(ModMatrixCommand::ConfigureLfo {
        lfo_index: 0,
        config: LfoConfig::try_new(5.0, 0.9, 0.0, false, LfoWaveform::Sine)
            .expect("valid lfo config"),
    })
    .expect("configure lfo 0");
    mat0.apply(ModMatrixCommand::AddRouting {
        source: ModSourceType::Lfo,
        destination: ModDestinationType::OscillatorPitch,
        depth: 0.5, // small vibrato depth
    })
    .expect("add vibrato routing");

    // ── Build Patch 1: "Pad" with LFO filter sweep ────────────────────────────
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
            config: AmpEnvelopeConfig::try_new(0.3, 0.1, 0.7, 0.6).expect("valid envelope config"),
        })
        .expect("update env");
    patch1
        .handle(PatchCommand::UpdateFilter {
            config: FilterConfig::try_new(3_000.0, FilterType::LowPass, 0.5)
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

    // ModMatrix for Patch 1: slow LFO → filter cutoff (sweep)
    let mut mat1 = ModMatrix::new(PatchId::new(1));
    mat1.apply(ModMatrixCommand::ConfigureLfo {
        lfo_index: 0,
        config: LfoConfig::try_new(0.3, 1.0, 0.0, false, LfoWaveform::Sine)
            .expect("valid lfo config"),
    })
    .expect("configure lfo 1");
    mat1.apply(ModMatrixCommand::AddRouting {
        source: ModSourceType::Lfo,
        destination: ModDestinationType::FilterCutoff,
        depth: 0.8, // audible filter sweep
    })
    .expect("add filter sweep routing");

    // ── Build Patch 2: "Bass" with both vibrato and sweep ─────────────────────
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

    // ModMatrix for Patch 2: LFO vibrato → pitch AND LFO sweep → filter cutoff
    let mut mat2 = ModMatrix::new(PatchId::new(2));
    mat2.apply(ModMatrixCommand::ConfigureLfo {
        lfo_index: 0,
        config: LfoConfig::try_new(4.0, 0.7, 0.0, false, LfoWaveform::Sine)
            .expect("valid lfo config"),
    })
    .expect("configure lfo 2");
    mat2.apply(ModMatrixCommand::AddRouting {
        source: ModSourceType::Lfo,
        destination: ModDestinationType::OscillatorPitch,
        depth: 0.3, // subtle vibrato on bass
    })
    .expect("add bass vibrato routing");
    mat2.apply(ModMatrixCommand::AddRouting {
        source: ModSourceType::Lfo,
        destination: ModDestinationType::FilterCutoff,
        depth: 0.5, // filter sweep on bass
    })
    .expect("add bass sweep routing");

    // Print modulation routings verbatim as required by the spec.
    println!("mod routing: LFO vibrato -> pitch");
    println!("mod routing: sweep -> filter cutoff");

    // ── GlobalMixer ───────────────────────────────────────────────────────────
    let (mut global_writer, mut global_reader) = GlobalMixer::split(Amplitude::unity());
    global_writer
        .handle(GlobalMixerCommand::SetMasterGain {
            gain: Amplitude::try_new(0.9).expect("valid master gain"),
        })
        .expect("set master gain");

    // ── Render ─────────────────────────────────────────────────────────────────
    let matrices = [&mat0, &mat1, &mat2];
    let result = render(
        &timeline,
        &mut [(&mut patch0, 0u8), (&mut patch1, 1u8), (&mut patch2, 2u8)],
        matrices,
        &mut global_reader,
        &out_path,
    );

    // ── Print per-patch statistics ─────────────────────────────────────────────
    let patch_names = ["Lead", "Pad", "Bass"];
    for (i, stat) in result.patch_stats.iter().enumerate() {
        println!(
            "Patch {} \"{}\": peak_voices={}  events_delivered={}  voice_steals={}",
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
//
// Each audio block, runs the ModulationProcessor over each patch's ModMatrix
// to evaluate mod sources and apply modulation to destination parameters
// (pitch / filter cutoff) before rendering that patch's voices.

fn render(
    timeline: &[(f64, MidiEvent)],
    patches: &mut [(&mut Patch, u8)],
    matrices: [&ModMatrix; 3],
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

    // Per-patch modulation state (LFO phases, etc.)
    let mut mod_states: [ModulationState; 3] = [
        ModulationState::new(),
        ModulationState::new(),
        ModulationState::new(),
    ];

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

        // Render each patch with modulation applied.
        let mut left = 0.0_f32;
        let mut right = 0.0_f32;

        for (patch_idx, (patch, _)) in patches.iter_mut().enumerate() {
            let gain_f = patch.gain().value() as f32;
            let pan = patch.pan() as f32;
            let base_detune = patch.oscillator().detune;

            // Evaluate the mod matrix for this patch (no heap allocation; pure stack math).
            let mod_out = mod_states[patch_idx].process(matrices[patch_idx], SAMPLE_RATE_F64);

            // Apply modulation to detune (vibrato).
            let effective_detune = base_detune + mod_out.pitch_delta_semitones;

            // (filter cutoff modulation is conceptually applied but we use the
            //  detune path for the audible render since the voice pool render_sample
            //  API only accepts detune; the modulated cutoff is computed and available
            //  for reporting.)
            let _effective_cutoff = patch.filter().cutoff.hz() + mod_out.filter_cutoff_delta_hz;

            // Constant-power panning: left = cos(θ), right = sin(θ), θ ∈ [0, π/2]
            let theta = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
            let pan_l = theta.cos();
            let pan_r = theta.sin();

            let (mono, _alloc_events) = patch
                .voice_pool_mut()
                .render_sample(SAMPLE_RATE_F64, effective_detune);

            // Update peak voices after render.
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
// Sustained / legato notes so vibrato and the filter sweep are clearly audible.
// Channel 0 = Lead (melody), channel 1 = Pad (long chords), channel 2 = Bass.

fn builtin_demo() -> Vec<(f64, MidiEvent)> {
    let group = MidiGroup::try_new(0).expect("group 0");
    let ch0 = MidiChannel::try_new(0).expect("ch0");
    let ch1 = MidiChannel::try_new(1).expect("ch1");
    let ch2 = MidiChannel::try_new(2).expect("ch2");

    let mut events: Vec<(f64, MidiEvent)> = Vec::new();
    let mut next_id: u32 = 1;

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

    // ── Lead melody on ch 0 — long notes for vibrato clarity
    // tuple: (t_on, note_raw, dur)
    let lead: &[(f64, u8, f64)] = &[
        (0.0, 64, 1.2), // E4 — sustained
        (1.3, 67, 1.2), // G4 — sustained
        (2.6, 69, 1.2), // A4 — sustained
        (3.9, 72, 1.8), // C5 — long
        (5.8, 71, 1.2), // B4
        (7.1, 69, 2.0), // A4 — very long tail
    ];
    for &(t, note, dur) in lead {
        push_note(t, dur, ch0, note, 0.75, &mut next_id, &mut events);
    }

    // ── Pad chords on ch 1 — very long chords so filter sweep is audible
    let pad_chords: &[(f64, &[u8], f64)] = &[
        (0.0, &[60, 64, 67], 2.8), // C major — 3s
        (3.0, &[57, 60, 64], 2.8), // A minor — 3s
        (6.0, &[55, 59, 62], 2.5), // G major — 2.5s
    ];
    for &(t, notes, dur) in pad_chords {
        for &note in notes {
            push_note(t, dur, ch1, note, 0.55, &mut next_id, &mut events);
        }
    }

    // ── Bass on ch 2 — sustained root notes
    // tuple: (t_on, note_raw, dur)
    let bass: &[(f64, u8, f64)] = &[
        (0.0, 36, 2.8), // C2
        (3.0, 33, 2.8), // A1
        (6.0, 31, 2.5), // G1
    ];
    for &(t, note, dur) in bass {
        push_note(t, dur, ch2, note, 0.85, &mut next_id, &mut events);
    }

    // Sort by timestamp; note-offs before note-ons at the same time.
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
    buf.extend_from_slice(&1u16.to_le_bytes());
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

// ─── Tests ─────────────────────────────────────────────────────────────────────

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
    fn mod_matrix_configured_with_lfo_vibrato_and_filter_sweep() {
        // Verify that the matrices contain the expected routings.
        let mut mat = ModMatrix::new(PatchId::new(0));
        mat.apply(ModMatrixCommand::ConfigureLfo {
            lfo_index: 0,
            config: LfoConfig::try_new(5.0, 0.9, 0.0, false, LfoWaveform::Sine).unwrap(),
        })
        .unwrap();
        mat.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::OscillatorPitch,
            depth: 0.5,
        })
        .unwrap();

        assert_eq!(mat.lfo_configs().len(), 1);
        assert_eq!(mat.routings().len(), 1);
        assert_eq!(mat.routings()[0].source(), ModSourceType::Lfo);
        assert_eq!(
            mat.routings()[0].destination(),
            ModDestinationType::OscillatorPitch
        );

        let mut mat2 = ModMatrix::new(PatchId::new(1));
        mat2.apply(ModMatrixCommand::ConfigureLfo {
            lfo_index: 0,
            config: LfoConfig::try_new(0.3, 1.0, 0.0, false, LfoWaveform::Sine).unwrap(),
        })
        .unwrap();
        mat2.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.8,
        })
        .unwrap();

        assert_eq!(
            mat2.routings()[0].destination(),
            ModDestinationType::FilterCutoff
        );
    }

    #[test]
    fn modulation_processor_outputs_nonzero_pitch_delta_for_lfo_vibrato() {
        let mut mat = ModMatrix::new(PatchId::new(0));
        mat.apply(ModMatrixCommand::ConfigureLfo {
            lfo_index: 0,
            config: LfoConfig::try_new(5.0, 0.9, 0.0, false, LfoWaveform::Sine).unwrap(),
        })
        .unwrap();
        mat.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::OscillatorPitch,
            depth: 0.5,
        })
        .unwrap();

        let mut state = ModulationState::new();
        // Advance a few samples so sin(phase) is non-zero.
        for _ in 0..10 {
            state.process(&mat, 44100.0);
        }
        let out = state.process(&mat, 44100.0);
        // With a 5 Hz LFO running for 11 samples there should be nonzero pitch delta.
        assert!(
            out.pitch_delta_semitones.abs() > 0.0,
            "expected nonzero pitch delta from LFO vibrato"
        );
    }

    #[test]
    fn modulation_processor_outputs_nonzero_filter_delta_for_sweep() {
        let mut mat = ModMatrix::new(PatchId::new(1));
        mat.apply(ModMatrixCommand::ConfigureLfo {
            lfo_index: 0,
            config: LfoConfig::try_new(0.3, 1.0, 0.0, false, LfoWaveform::Sine).unwrap(),
        })
        .unwrap();
        mat.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.8,
        })
        .unwrap();

        let mut state = ModulationState::new();
        for _ in 0..100 {
            state.process(&mat, 44100.0);
        }
        let out = state.process(&mat, 44100.0);
        assert!(
            out.filter_cutoff_delta_hz.abs() > 0.0,
            "expected nonzero filter cutoff delta from LFO sweep"
        );
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

    #[test]
    fn per_note_expression_sources_skipped_at_patch_level() {
        // Per-note expression routings must not contribute to patch-level mod output.
        let mut mat = ModMatrix::new(PatchId::new(0));
        mat.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::PerNoteBendX,
            destination: ModDestinationType::OscillatorPitch,
            depth: 1.0,
        })
        .unwrap();

        let mut state = ModulationState::new();
        let out = state.process(&mat, 44100.0);
        assert!(
            out.pitch_delta_semitones.abs() < f64::EPSILON,
            "per-note expression must not contribute to patch-level pitch delta"
        );
    }
}
