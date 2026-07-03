// path: src/bin/patch_play.rs
//
// Multi-patch MIDI player: proves the dispatcher -> per-patch voice pools ->
// global mix integration end to end.
//
// Three `Patch` aggregates are configured with distinct engine settings
// (oscillator waveform, filter, amp envelope, gain, and pan), each
// subscribed to its own MIDI channel via a `ChannelMapping`. Every event in
// the timeline is routed through the `MidiDispatcher` to the patches whose
// mapping matches its channel; each matched patch drives its own
// `VoiceAllocator`, so one patch's polyphony/stealing can never exhaust
// another's voice pool. Each patch's rendered audio is scaled by its own
// gain and pan (the "PatchMixer" stage) and summed into a global bus, which
// is then scaled by a single master gain (the "GlobalMixer" stage) before
// being limited and quantized to 16-bit mono WAV.
//
// This binary is a batch/offline tool: it has no real-time audio callback,
// so it is not the "audio thread" the project's real-time invariants
// govern. It is free to allocate and perform blocking file I/O.
//
// Usage:
//   patch_play [FILE.mid] [--out OUT.wav]
//
// If FILE is omitted, a built-in multi-channel demo tune is rendered
// instead, with events spread across every channel the patches subscribe
// to, so no .mid asset needs to live in the repository.

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::exit;

use crest_synth::engine::filter::{Filter, FilterConfig, FilterKind, StateVariableFilter};
use crest_synth::engine::oscillator::{
    Amplitude as OscAmplitude, Frequency, Oscillator, OscillatorConfig, SampleRate,
    StandardOscillator, Waveform,
};
use crest_synth::engine::voice::{
    EnvelopeTiming, NoteId as EngineNoteId, NoteNumber as EngineNoteNumber,
    Velocity as EngineVelocity, VoiceConfig as EngineVoiceConfig, VoiceEvent,
};
use crest_synth::engine::voice_allocator::{StealPolicy, VoiceAllocator, VoiceAssignment};
use crest_synth::kernel::audio_frame::AudioFrame;
use crest_synth::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
use crest_synth::kernel::midi_event::MidiEvent;
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::kernel::note_id::NoteId as KernelNoteId;
use crest_synth::kernel::note_number::NoteNumber as KernelNoteNumber;
use crest_synth::kernel::velocity::Velocity as KernelVelocity;
use crest_synth::midi_file::midi_file_reader::{MidiFileReader, TimedMidiEvent};
use crest_synth::midi_file::midly_midi_file_reader::MidlyMidiFileReader;
use crest_synth::patch::midi_dispatcher::{MidiAddress, MidiDispatcher, RoutablePatch};
use crest_synth::patch::patch::{ChannelMapping, Patch, PatchId, VoiceConfig as PatchVoiceConfig};

/// Fixed sample rate used for the offline render.
const SAMPLE_RATE_HZ: f64 = 44_100.0;
/// Extra tail rendered after the last event so releases are not cut off.
const TAIL_SECONDS: f64 = 1.0;
/// Master ("GlobalMixer") gain applied after summing every patch.
const MASTER_GAIN: f32 = 0.8;
/// Default WAV output path when `--out` is not supplied.
const DEFAULT_OUT_PATH: &str = "patch-play.wav";

fn main() {
    match run() {
        Ok(()) => exit(0),
        Err(message) => {
            eprintln!("patch_play: error: {message}");
            exit(1);
        }
    }
}

// ---------------------------------------------------------------------
// Per-patch engine configuration: distinct oscillator/filter/envelope
// settings and distinct gain/pan, one entry per `Patch`.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct EngineSettings {
    name: &'static str,
    channel: u8,
    waveform: Waveform,
    filter_kind: FilterKind,
    filter_cutoff_hz: f64,
    filter_resonance: f64,
    attack_seconds: f64,
    decay_seconds: f64,
    sustain_level: f64,
    release_seconds: f64,
    polyphony: usize,
    steal_policy: StealPolicy,
    gain: f32,
    pan: f32,
}

/// Three patches, each on a different MIDI channel, each with visibly
/// different engine settings and a deliberately small polyphony so the
/// built-in demo tune exercises voice stealing independently per patch.
fn patch_settings() -> Vec<EngineSettings> {
    vec![
        EngineSettings {
            name: "Bass",
            channel: 0,
            waveform: Waveform::Saw,
            filter_kind: FilterKind::LowPass,
            filter_cutoff_hz: 900.0,
            filter_resonance: 0.15,
            attack_seconds: 0.01,
            decay_seconds: 0.08,
            sustain_level: 0.8,
            release_seconds: 0.2,
            polyphony: 2,
            steal_policy: StealPolicy::Oldest,
            gain: 0.9,
            pan: -0.4,
        },
        EngineSettings {
            name: "Lead",
            channel: 1,
            waveform: Waveform::Square,
            filter_kind: FilterKind::HighPass,
            filter_cutoff_hz: 1800.0,
            filter_resonance: 0.1,
            attack_seconds: 0.005,
            decay_seconds: 0.05,
            sustain_level: 0.6,
            release_seconds: 0.08,
            polyphony: 4,
            steal_policy: StealPolicy::Quietest,
            gain: 0.6,
            pan: 0.4,
        },
        EngineSettings {
            name: "Pad",
            channel: 2,
            waveform: Waveform::Triangle,
            filter_kind: FilterKind::BandPass,
            filter_cutoff_hz: 1200.0,
            filter_resonance: 0.05,
            attack_seconds: 0.3,
            decay_seconds: 0.3,
            sustain_level: 0.9,
            release_seconds: 0.6,
            polyphony: 3,
            steal_policy: StealPolicy::Oldest,
            gain: 0.5,
            pan: 0.0,
        },
    ]
}

// ---------------------------------------------------------------------
// Runtime state: one `Patch` aggregate, one independent `VoiceAllocator`
// (voice pool), and per-slot oscillator phase / filter state, per patch.
// ---------------------------------------------------------------------

struct PatchRuntime {
    aggregate: Patch,
    settings: EngineSettings,
    allocator: VoiceAllocator,
    phases: Vec<f64>,
    filters: Vec<StateVariableFilter>,
    peak_voices: usize,
    events_delivered: usize,
    voice_steals: usize,
}

/// Narrow routing view handed to the `MidiDispatcher`: just a patch's
/// identity and its channel mapping, decoupled from the full `Patch`
/// aggregate and from `PatchRuntime`'s render-only state.
struct PatchRoute {
    id: PatchId,
    mapping: ChannelMapping,
}

impl RoutablePatch for PatchRoute {
    fn id(&self) -> PatchId {
        self.id
    }

    fn mapping(&self) -> ChannelMapping {
        self.mapping
    }
}

fn build_patches(
    all_settings: &[EngineSettings],
) -> Result<(Vec<PatchRuntime>, Vec<PatchRoute>), String> {
    let mut patches = Vec::with_capacity(all_settings.len());
    let mut routes = Vec::with_capacity(all_settings.len());

    for (index, settings) in all_settings.iter().enumerate() {
        let id = PatchId::new(index as u32);
        let mapping = ChannelMapping::single(settings.channel)
            .map_err(|e| format!("invalid channel for patch '{}': {e}", settings.name))?;

        let patch_voice = PatchVoiceConfig::try_new(
            settings.polyphony as u8,
            (settings.attack_seconds * 1000.0) as f32,
            (settings.decay_seconds * 1000.0) as f32,
            settings.sustain_level as f32,
            (settings.release_seconds * 1000.0) as f32,
        )
        .map_err(|e| format!("invalid voice config for patch '{}': {e}", settings.name))?;

        let mut aggregate = Patch::new(id, index as u32, patch_voice);
        aggregate.set_mapping(mapping);

        let engine_voice = EngineVoiceConfig::new(EnvelopeTiming::new(
            settings.attack_seconds,
            settings.decay_seconds,
            settings.sustain_level,
            settings.release_seconds,
        ));
        let allocator =
            VoiceAllocator::new(engine_voice, settings.polyphony, settings.steal_policy).map_err(
                |e| {
                    format!(
                        "invalid voice allocator for patch '{}': {e:?}",
                        settings.name
                    )
                },
            )?;

        routes.push(PatchRoute { id, mapping });
        patches.push(PatchRuntime {
            aggregate,
            settings: *settings,
            allocator,
            phases: vec![0.0; settings.polyphony],
            filters: vec![StateVariableFilter::new(); settings.polyphony],
            peak_voices: 0,
            events_delivered: 0,
            voice_steals: 0,
        });
    }

    Ok((patches, routes))
}

// ---------------------------------------------------------------------
// Built-in multi-channel demo tune (no external .mid asset required).
// ---------------------------------------------------------------------

fn demo_address(channel: u8) -> ChannelAddress {
    ChannelAddress::new(
        MidiChannel::try_new(channel).expect("demo channel is within 0..=15"),
        MidiGroup::try_new(0).expect("group 0 is always valid"),
    )
}

/// Appends a matched NoteOn/NoteOff pair (sharing one freshly minted
/// `NoteId`, as a real `MidiFileReader` would) to `events`.
#[allow(clippy::too_many_arguments)]
fn push_demo_note(
    events: &mut Vec<TimedMidiEvent>,
    next_note_id: &mut u32,
    channel: u8,
    note: u8,
    velocity: u8,
    start_seconds: f64,
    end_seconds: f64,
) {
    let note_number = KernelNoteNumber::try_new(note).expect("demo note number is in 0..=127");
    let note_id = KernelNoteId::new(*next_note_id);
    *next_note_id += 1;
    let velocity = KernelVelocity::from_midi7(velocity);
    let address = demo_address(channel);

    events.push(TimedMidiEvent::new(
        start_seconds,
        MidiEvent::new(
            address,
            MidiEventKind::NoteOn,
            note_number,
            note_id,
            velocity,
        ),
    ));
    events.push(TimedMidiEvent::new(
        end_seconds,
        MidiEvent::new(
            address,
            MidiEventKind::NoteOff,
            note_number,
            note_id,
            velocity,
        ),
    ));
}

/// Builds a short multi-channel demo tune spanning a few bars at 120 BPM
/// (0.5s/beat, 2.0s/bar), with events on every channel the three demo
/// patches subscribe to. Each part is deliberately dense enough relative to
/// its patch's polyphony to force at least one voice steal, proving the
/// per-patch voice accounting actually runs.
fn build_demo_timeline() -> Vec<TimedMidiEvent> {
    let mut events = Vec::new();
    let mut next_note_id = 0u32;

    // Bass (channel 0, polyphony 2): a walking bass line with one
    // deliberate triple-overlap early on to force a steal.
    push_demo_note(&mut events, &mut next_note_id, 0, 36, 100, 0.0, 1.0);
    push_demo_note(&mut events, &mut next_note_id, 0, 38, 100, 0.5, 1.5);
    push_demo_note(&mut events, &mut next_note_id, 0, 41, 100, 0.9, 1.9);
    for (i, note) in [36u8, 38, 41, 36, 38, 41, 36].iter().enumerate() {
        let start = 2.0 + i as f64 * 2.0;
        push_demo_note(
            &mut events,
            &mut next_note_id,
            0,
            *note,
            95,
            start,
            start + 1.6,
        );
    }

    // Lead (channel 1, polyphony 4): a fast, mostly non-overlapping
    // arpeggio, repeated across the tune, plus one 5-note chord stab that
    // exceeds polyphony and forces a steal.
    const ARPEGGIO: [u8; 16] = [
        60, 64, 67, 72, 67, 64, 60, 64, 67, 72, 67, 64, 60, 64, 67, 72,
    ];
    for rep in 0..4 {
        let base = rep as f64 * 4.0;
        for (i, note) in ARPEGGIO.iter().enumerate() {
            let start = base + i as f64 * 0.25;
            push_demo_note(
                &mut events,
                &mut next_note_id,
                1,
                *note,
                100,
                start,
                start + 0.22,
            );
        }
    }
    for note in [60u8, 64, 67, 71, 74] {
        push_demo_note(&mut events, &mut next_note_id, 1, note, 110, 8.0, 8.3);
    }

    // Pad (channel 2, polyphony 3): sustained four-note chords, each
    // exceeding polyphony by one note and so forcing a steal every time.
    for (i, chord) in [
        [60u8, 64, 67, 72],
        [57, 60, 64, 69],
        [55, 59, 62, 67],
        [60, 64, 67, 71],
    ]
    .iter()
    .enumerate()
    {
        let start = i as f64 * 4.0;
        let end = start + 3.8;
        for note in chord {
            push_demo_note(&mut events, &mut next_note_id, 2, *note, 90, start, end);
        }
    }

    events.sort_by(|a, b| a.at_seconds().partial_cmp(&b.at_seconds()).unwrap());
    events
}

// ---------------------------------------------------------------------
// Conversions between the kernel's normalized MIDI-event value objects and
// the engine's per-voice value objects.
// ---------------------------------------------------------------------

fn to_engine_note(note: &KernelNoteNumber) -> EngineNoteNumber {
    EngineNoteNumber::try_new(note.value())
        .expect("kernel and engine NoteNumber share the same 0..=127 range")
}

fn to_engine_velocity(velocity: &KernelVelocity) -> EngineVelocity {
    EngineVelocity::try_new(velocity.value())
        .expect("kernel and engine Velocity share the same 0.0..=1.0 range")
}

fn to_engine_note_id(note_id: &KernelNoteId) -> EngineNoteId {
    EngineNoteId::new(u64::from(note_id.value()))
}

fn note_to_frequency(note_value: u8) -> Frequency {
    let hertz = 440.0 * 2f64.powf((f64::from(note_value) - 69.0) / 12.0);
    Frequency::try_new(hertz).expect("MIDI note numbers always yield a valid frequency")
}

// ---------------------------------------------------------------------
// Dispatch: routes every timeline event through the `MidiDispatcher` to
// exactly the patches whose `ChannelMapping` matches its channel, then
// drives that patch's own `VoiceAllocator`.
// ---------------------------------------------------------------------

fn process_event(
    timed: &TimedMidiEvent,
    patches: &mut [PatchRuntime],
    routes: &[PatchRoute],
    dispatcher: &MidiDispatcher,
) {
    let event = timed.event();
    let channel = event.address().channel().value();
    let address = match MidiAddress::try_new(channel) {
        Ok(address) => address,
        Err(_) => return,
    };

    for id in dispatcher.dispatch(address, routes) {
        let index = id.value() as usize;
        let Some(patch) = patches.get_mut(index) else {
            continue;
        };
        patch.events_delivered += 1;

        match event.kind() {
            MidiEventKind::NoteOn => {
                let note = to_engine_note(event.note());
                let note_id = to_engine_note_id(event.note_id());
                let velocity = to_engine_velocity(event.velocity());

                if let Ok(assignment) = patch.allocator.allocate(note, note_id, velocity) {
                    match assignment {
                        VoiceAssignment::Assigned { index } => {
                            patch.phases[index] = 0.0;
                            patch.filters[index].reset();
                        }
                        VoiceAssignment::Stolen { .. } => {
                            patch.voice_steals += 1;
                        }
                    }
                }
            }
            MidiEventKind::NoteOff => {
                let note_id = to_engine_note_id(event.note_id());
                // A note that was already stolen away (and so is no longer
                // active under this note id) has nothing to release; that
                // is expected, not an error.
                let _ = patch.allocator.release(note_id);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------
// Rendering: per-sample simulation. Each active voice's amp-envelope
// state machine (owned by `Voice`, advanced via the patch's own
// `VoiceAllocator`) drives its output level; the oscillator and filter
// are rendered per patch-owned slot. Each patch's mono sum is scaled and
// panned (PatchMixer), then summed into the master bus, which is scaled
// by a single master gain (GlobalMixer) before limiting and quantizing.
// ---------------------------------------------------------------------

fn render(
    patches: &mut [PatchRuntime],
    routes: &[PatchRoute],
    dispatcher: &MidiDispatcher,
    timeline: &[TimedMidiEvent],
) -> Vec<i16> {
    let sample_rate = SampleRate::try_new(SAMPLE_RATE_HZ).expect("44.1kHz is a valid sample rate");
    let oscillator = StandardOscillator::new();
    let dt = 1.0 / SAMPLE_RATE_HZ;

    let last_time = timeline
        .iter()
        .map(TimedMidiEvent::at_seconds)
        .fold(0.0_f64, f64::max);
    let duration_seconds = last_time + TAIL_SECONDS;
    let total_samples = ((duration_seconds * SAMPLE_RATE_HZ).ceil() as usize).max(1);

    let mut event_index = 0usize;
    let mut output = Vec::with_capacity(total_samples);

    for sample_index in 0..total_samples {
        let t = sample_index as f64 * dt;

        while event_index < timeline.len() && timeline[event_index].at_seconds() <= t {
            process_event(&timeline[event_index], patches, routes, dispatcher);
            event_index += 1;
        }

        let mut global = AudioFrame::silence();

        for patch in patches.iter_mut() {
            let PatchRuntime {
                allocator,
                phases,
                filters,
                settings,
                peak_voices,
                ..
            } = patch;

            allocator.advance_all(dt, |index, event| {
                // A steal completes here, once the victim's own envelope
                // reaches Idle: start the freshly (re)triggered voice from
                // a clean oscillator phase and filter state.
                if let VoiceEvent::Triggered { .. } = event {
                    phases[index] = 0.0;
                    filters[index].reset();
                }
            });

            let osc_config = OscillatorConfig::new(
                settings.waveform,
                OscAmplitude::try_new(1.0).expect("1.0 is a valid amplitude"),
            );
            let filter_config = FilterConfig::new(
                settings.filter_kind,
                settings.filter_cutoff_hz,
                settings.filter_resonance,
                SAMPLE_RATE_HZ,
            );

            let mut patch_mono = 0.0_f64;
            for index in 0..allocator.polyphony() {
                let Some(voice) = allocator.voice(index) else {
                    continue;
                };
                if voice.is_reclaimable() {
                    continue;
                }

                let frequency = note_to_frequency(voice.note().value());
                let raw = oscillator.render(phases[index], osc_config);
                let filtered = filters[index].process(raw, filter_config);
                let level = voice.amp_level();
                patch_mono += filtered * level * voice.velocity().value();
                phases[index] = oscillator.advance(phases[index], frequency, sample_rate);
            }

            *peak_voices = (*peak_voices).max(allocator.active_count());

            // PatchMixer: per-patch gain, then pan.
            let patch_frame = AudioFrame::from_mono(patch_mono as f32)
                .scaled(settings.gain)
                .panned(settings.pan);
            global += patch_frame;
        }

        // GlobalMixer: single master gain, then a simple limiter (clamp)
        // before quantization, matching the canonical signal path's final
        // stages.
        let mastered = global.scaled(MASTER_GAIN);
        output.push(clamp_to_i16(mastered.mono()));
    }

    output
}

fn clamp_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

// ---------------------------------------------------------------------
// Pure-Rust 16-bit mono WAV writer (no external WAV crate).
// ---------------------------------------------------------------------

fn write_wav(path: &str, samples: &[i16]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);

    const BITS_PER_SAMPLE: u16 = 16;
    const NUM_CHANNELS: u16 = 1;
    let sample_rate = SAMPLE_RATE_HZ as u32;
    let byte_rate = sample_rate * u32::from(NUM_CHANNELS) * (u32::from(BITS_PER_SAMPLE) / 8);
    let block_align = NUM_CHANNELS * (BITS_PER_SAMPLE / 8);
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    writer.write_all(b"RIFF").map_err(|e| e.to_string())?;
    writer
        .write_all(&riff_len.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(b"WAVE").map_err(|e| e.to_string())?;

    writer.write_all(b"fmt ").map_err(|e| e.to_string())?;
    writer
        .write_all(&16u32.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&1u16.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&NUM_CHANNELS.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&sample_rate.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&byte_rate.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&block_align.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&BITS_PER_SAMPLE.to_le_bytes())
        .map_err(|e| e.to_string())?;

    writer.write_all(b"data").map_err(|e| e.to_string())?;
    writer
        .write_all(&data_len.to_le_bytes())
        .map_err(|e| e.to_string())?;
    for sample in samples {
        writer
            .write_all(&sample.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }

    writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------

fn parse_args(args: &[String]) -> Result<(Option<String>, String), String> {
    let mut file_arg: Option<String> = None;
    let mut out_path = DEFAULT_OUT_PATH.to_string();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--out" {
            let value = args
                .get(i + 1)
                .ok_or_else(|| "--out requires a path argument".to_string())?;
            out_path = value.clone();
            i += 2;
        } else if file_arg.is_none() {
            file_arg = Some(arg.clone());
            i += 1;
        } else {
            return Err(format!("unexpected argument: {arg}"));
        }
    }

    Ok((file_arg, out_path))
}

fn load_file_timeline(path: &str) -> Result<Vec<TimedMidiEvent>, String> {
    let reader = MidlyMidiFileReader::new();
    let song = reader
        .load(Path::new(path))
        .map_err(|e| format!("failed to parse MIDI file '{path}': {e}"))?;
    Ok(song.events().to_vec())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (file_arg, out_path) = parse_args(&args)?;

    let (timeline, source_label) = match &file_arg {
        Some(path) => (load_file_timeline(path)?, path.clone()),
        None => (
            build_demo_timeline(),
            "built-in multi-channel demo tune".to_string(),
        ),
    };

    let all_settings = patch_settings();
    let (mut patches, routes) = build_patches(&all_settings)?;
    let dispatcher = MidiDispatcher::new();

    println!("patch_play: source={source_label}");
    println!("patch_play: patches={}", patches.len());

    let samples = render(&mut patches, &routes, &dispatcher, &timeline);

    write_wav(&out_path, &samples)
        .map_err(|e| format!("failed to write WAV file '{out_path}': {e}"))?;

    for (index, patch) in patches.iter().enumerate() {
        let mapping = patch.aggregate.mapping();
        println!(
            "Patch {} \"{}\": Peak Voices = {}, Events Delivered = {}, Voice Steals = {}, Channel = {}, Mapped = {}",
            index + 1,
            patch.settings.name,
            patch.peak_voices,
            patch.events_delivered,
            patch.voice_steals,
            patch.settings.channel,
            mapping.matches(patch.settings.channel),
        );
    }

    let rendered_seconds = samples.len() as f64 / SAMPLE_RATE_HZ;
    println!("patch_play: rendered seconds={rendered_seconds:.2}");
    println!("patch_play: output={out_path}");

    Ok(())
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_settings_cover_three_distinct_channels() {
        let settings = patch_settings();
        assert_eq!(settings.len(), 3);
        let mut channels: Vec<u8> = settings.iter().map(|s| s.channel).collect();
        channels.sort_unstable();
        channels.dedup();
        assert_eq!(channels.len(), 3);
    }

    #[test]
    fn patch_settings_have_distinct_waveforms() {
        let settings = patch_settings();
        assert_ne!(settings[0].waveform, settings[1].waveform);
        assert_ne!(settings[1].waveform, settings[2].waveform);
    }

    #[test]
    fn build_patches_maps_each_patch_to_its_own_channel() {
        let settings = patch_settings();
        let (patches, routes) = build_patches(&settings).expect("valid settings");
        assert_eq!(patches.len(), 3);
        assert_eq!(routes.len(), 3);
        for (patch, settings) in patches.iter().zip(settings.iter()) {
            assert!(patch.aggregate.mapping().matches(settings.channel));
        }
    }

    #[test]
    fn demo_timeline_is_time_ordered_and_nonempty() {
        let events = build_demo_timeline();
        assert!(!events.is_empty());
        for pair in events.windows(2) {
            assert!(pair[0].at_seconds() <= pair[1].at_seconds());
        }
    }

    #[test]
    fn demo_timeline_covers_every_patch_channel() {
        let events = build_demo_timeline();
        let mut channels: Vec<u8> = events
            .iter()
            .map(|e| e.event().address().channel().value())
            .collect();
        channels.sort_unstable();
        channels.dedup();
        assert_eq!(channels, vec![0, 1, 2]);
    }

    #[test]
    fn dispatch_routes_event_only_to_the_matching_patch() {
        let settings = patch_settings();
        let (_, routes) = build_patches(&settings).expect("valid settings");
        let dispatcher = MidiDispatcher::new();

        let address = MidiAddress::try_new(1).expect("channel 1 is valid");
        let matched = dispatcher.dispatch(address, &routes);

        assert_eq!(matched, vec![PatchId::new(1)]);
    }

    #[test]
    fn render_produces_one_sample_per_output_frame_covering_the_tail() {
        let settings = patch_settings();
        let (mut patches, routes) = build_patches(&settings).expect("valid settings");
        let dispatcher = MidiDispatcher::new();
        let timeline = build_demo_timeline();

        let samples = render(&mut patches, &routes, &dispatcher, &timeline);

        let last_time = timeline
            .iter()
            .map(TimedMidiEvent::at_seconds)
            .fold(0.0_f64, f64::max);
        let expected = (((last_time + TAIL_SECONDS) * SAMPLE_RATE_HZ).ceil() as usize).max(1);
        assert_eq!(samples.len(), expected);
    }

    #[test]
    fn render_exercises_independent_polyphony_per_patch() {
        let settings = patch_settings();
        let (mut patches, routes) = build_patches(&settings).expect("valid settings");
        let dispatcher = MidiDispatcher::new();
        let timeline = build_demo_timeline();

        let _ = render(&mut patches, &routes, &dispatcher, &timeline);

        for patch in &patches {
            assert!(
                patch.peak_voices > 0,
                "{} never sounded a voice",
                patch.settings.name
            );
            assert!(patch.peak_voices <= patch.settings.polyphony);
            assert!(patch.events_delivered > 0);
        }
    }

    #[test]
    fn render_forces_at_least_one_steal_on_the_densely_packed_pad_patch() {
        let settings = patch_settings();
        let (mut patches, routes) = build_patches(&settings).expect("valid settings");
        let dispatcher = MidiDispatcher::new();
        let timeline = build_demo_timeline();

        let _ = render(&mut patches, &routes, &dispatcher, &timeline);

        let pad = patches
            .iter()
            .find(|p| p.settings.name == "Pad")
            .expect("Pad exists");
        assert!(
            pad.voice_steals > 0,
            "expected the four-note pad chords to force stealing"
        );
    }

    #[test]
    fn clamp_to_i16_never_exceeds_full_scale() {
        assert_eq!(clamp_to_i16(2.0), i16::MAX);
        assert!(clamp_to_i16(-2.0) < 0);
        assert_eq!(clamp_to_i16(0.0), 0);
    }

    #[test]
    fn parse_args_defaults_to_demo_and_default_out_path() {
        let (file_arg, out_path) = parse_args(&[]).unwrap();
        assert_eq!(file_arg, None);
        assert_eq!(out_path, DEFAULT_OUT_PATH);
    }

    #[test]
    fn parse_args_reads_file_and_out_flag() {
        let args: Vec<String> = vec![
            "song.mid".to_string(),
            "--out".to_string(),
            "out.wav".to_string(),
        ];
        let (file_arg, out_path) = parse_args(&args).unwrap();
        assert_eq!(file_arg, Some("song.mid".to_string()));
        assert_eq!(out_path, "out.wav");
    }

    #[test]
    fn parse_args_rejects_out_flag_without_value() {
        let args: Vec<String> = vec!["--out".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn load_file_timeline_reports_a_clear_error_for_a_missing_file() {
        let result = load_file_timeline("/definitely/does/not/exist.mid");
        assert!(result.is_err());
    }

    #[test]
    fn note_to_frequency_a4_is_440_hertz() {
        let frequency = note_to_frequency(69);
        assert!((frequency.hertz() - 440.0).abs() < 1e-9);
    }
}
