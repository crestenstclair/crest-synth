// path: src/bin/preset_demo.rs
//
// preset_demo — serializes a full Setup, reloads it, and proves round-trip
// fidelity by rendering identical audio before and after.
//
// Usage: preset_demo [--out OUT.wav]
//
// Default output: preset-demo.wav
//
// Signal flow (per sample):
//   patch voices → PatchMixer (gain + pan) → GlobalMixer (master gain) → WAV

use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process;

use crest_synth::kernel::amplitude::Amplitude;
use crest_synth::kernel::audio_frame::AudioFrame;
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::midi_group::MidiGroup;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::patch::channel_subscription::{ChannelAddress, ChannelSubscription};
use crest_synth::patch::global_mixer::GlobalMixer;
use crest_synth::patch::patch::{EngineType, Patch, PatchCommand};
use crest_synth::patch::patch_mixer::{PatchMixEntry, PatchMixer};
use crest_synth::patch::voice_pool_config::{StealingPolicy, VoicePoolConfig};
use crest_synth::presets::preset_codec::PresetCodec;
use crest_synth::presets::setup::{
    build_serialized_patch, SerializedEffectChain, SerializedEngineType, SerializedModMatrix, Setup,
};
use crest_synth::synth::amp_envelope_config::AmpEnvelopeConfig;
use crest_synth::synth::filter_config::{FilterConfig, FilterType};
use crest_synth::synth::oscillator_config::{OscillatorConfig, Waveform};
use crest_synth::synth::voice_allocator::VoiceAllocator;

// ─── Constants ────────────────────────────────────────────────────────────────

const SAMPLE_RATE: u32 = 44_100;
const SAMPLE_RATE_F64: f64 = SAMPLE_RATE as f64;
/// Render this many seconds of audio per round-trip comparison.
const RENDER_SECS: f64 = 0.5;
/// Total samples per render pass.
const RENDER_SAMPLES: usize = (RENDER_SECS * SAMPLE_RATE_F64) as usize;

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut out_path = PathBuf::from("preset-demo.wav");
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
                eprintln!("warning: unknown argument '{other}' ignored");
            }
        }
        i += 1;
    }

    // ── 1. Build the original Setup ───────────────────────────────────────────

    let (original_setup, patch_configs) = build_demo_setup();

    // ── 2. Round-trip the Setup via PresetCodec ───────────────────────────────

    let codec = PresetCodec::new();
    let setup_bytes = codec.serialize_setup(original_setup.clone());
    let reloaded_setup = codec.deserialize_setup(setup_bytes).unwrap_or_else(|e| {
        eprintln!("error: deserialize_setup failed: {e}");
        process::exit(1);
    });

    // Assert structural equality.
    assert!(
        original_setup == reloaded_setup,
        "setup roundtrip FAILED: original != reloaded"
    );

    println!("setup roundtrip: equal");

    // ── 3. Render with original Setup ─────────────────────────────────────────

    let buf_original = render_from_patch_configs(&patch_configs, original_setup.master_gain);

    // ── 4. Render with reloaded Setup ─────────────────────────────────────────

    // Reconstruct patch configs from the reloaded setup so rendering is driven
    // purely by deserialized state — this is the real fidelity proof.
    let reloaded_patch_configs = patch_configs_from_setup(&reloaded_setup);
    let buf_reloaded =
        render_from_patch_configs(&reloaded_patch_configs, reloaded_setup.master_gain);

    // ── 5. Assert bit-identical ───────────────────────────────────────────────

    assert_eq!(
        buf_original.len(),
        buf_reloaded.len(),
        "render lengths differ: {} vs {}",
        buf_original.len(),
        buf_reloaded.len()
    );

    for (idx, (orig, reload)) in buf_original.iter().zip(buf_reloaded.iter()).enumerate() {
        if orig != reload {
            panic!(
                "render identical: false — sample[{idx}] differs: original={orig}, reloaded={reload}"
            );
        }
    }

    println!("render identical: true");

    // ── 6. Write WAV ──────────────────────────────────────────────────────────

    write_wav(&out_path, &buf_original, SAMPLE_RATE);

    // ── 7. Print stats ────────────────────────────────────────────────────────

    let num_patches = patch_configs.len();
    println!(
        "patches={num_patches}  samples={}  duration={:.3}s  out={}",
        buf_original.len(),
        buf_original.len() as f64 / SAMPLE_RATE_F64,
        out_path.display()
    );
}

// ─── PatchConfig — plain data for rendering ────────────────────────────────────

/// All the data needed to render one patch deterministically.
///
/// Derived from the live `Patch` struct (original) or reconstructed from the
/// deserialized `SerializedPatch` (reloaded). Using the same struct for both
/// passes keeps the render function identical.
#[derive(Clone)]
struct PatchConfig {
    oscillator: OscillatorConfig,
    /// Stored for round-trip equality comparison in tests.
    #[allow(dead_code)]
    amp_envelope: AmpEnvelopeConfig,
    /// Stored for round-trip equality comparison in tests.
    #[allow(dead_code)]
    filter: FilterConfig,
    gain: f64,
    pan: f64,
    /// Which MIDI channel this patch listens on (used for note dispatch).
    channel: u8,
    max_voices: usize,
    #[allow(dead_code)]
    stealing_policy: StealingPolicy,
}

// ─── Build demo Setup ─────────────────────────────────────────────────────────

/// Construct a demo setup with three distinct patches and return a matching
/// `Vec<PatchConfig>` for deterministic rendering.
fn build_demo_setup() -> (Setup, Vec<PatchConfig>) {
    let group = MidiGroup::try_new(0).expect("group 0 valid");
    let mut setup = Setup::new("preset-demo");
    setup.master_gain = 0.85;

    // ── Patch 0: Lead (Saw, LPF, fast attack) ─────────────────────────────
    let ch0 = MidiChannel::try_new(0).expect("ch0 valid");
    let addr0 = ChannelAddress::new(group, ch0);
    let sub0 = ChannelSubscription::new(addr0, None);
    let pool0 = VoicePoolConfig::try_new(4, StealingPolicy::OldestFirst).expect("pool0 valid");

    let osc0 = OscillatorConfig::try_new(0.0, 0.5, Waveform::Saw).expect("osc0 valid");
    let env0 = AmpEnvelopeConfig::try_new(0.005, 0.08, 0.75, 0.15).expect("env0 valid");
    let flt0 = FilterConfig::try_new(6_000.0, FilterType::LowPass, 0.3).expect("flt0 valid");
    let gain0 = Amplitude::try_new(0.28).expect("gain0 valid");

    let mut patch0 = Patch::new(pool0, sub0);
    patch0
        .handle(PatchCommand::CreatePatch {
            name: "Lead".to_string(),
            engine_type: EngineType::Sine,
            subscription: sub0,
        })
        .expect("create patch0");
    patch0
        .handle(PatchCommand::UpdateOscillator { config: osc0 })
        .expect("osc0");
    patch0
        .handle(PatchCommand::UpdateEnvelope { config: env0 })
        .expect("env0");
    patch0
        .handle(PatchCommand::UpdateFilter { config: flt0 })
        .expect("flt0");
    patch0
        .handle(PatchCommand::SetGain { gain: gain0 })
        .expect("gain0");
    patch0
        .handle(PatchCommand::SetPan { pan: -0.3 })
        .expect("pan0");
    patch0
        .handle(PatchCommand::ActivatePatch)
        .expect("activate0");

    setup.patches.push(build_serialized_patch(
        0,
        "Lead",
        true,
        SerializedEngineType::Sine,
        osc0,
        env0,
        flt0,
        gain0,
        -0.3,
        sub0,
        pool0,
        SerializedModMatrix::default(),
        SerializedEffectChain::default(),
    ));

    // ── Patch 1: Pad (Sine, slow attack/release, centre pan) ──────────────
    let ch1 = MidiChannel::try_new(1).expect("ch1 valid");
    let addr1 = ChannelAddress::new(group, ch1);
    let sub1 = ChannelSubscription::new(addr1, None);
    let pool1 = VoicePoolConfig::try_new(6, StealingPolicy::QuietestFirst).expect("pool1 valid");

    let osc1 = OscillatorConfig::try_new(5.0, 0.5, Waveform::Sine).expect("osc1 valid");
    let env1 = AmpEnvelopeConfig::try_new(0.25, 0.1, 0.6, 0.5).expect("env1 valid");
    let flt1 = FilterConfig::try_new(4_000.0, FilterType::LowPass, 0.5).expect("flt1 valid");
    let gain1 = Amplitude::try_new(0.22).expect("gain1 valid");

    let mut patch1 = Patch::new(pool1, sub1);
    patch1
        .handle(PatchCommand::CreatePatch {
            name: "Pad".to_string(),
            engine_type: EngineType::Sine,
            subscription: sub1,
        })
        .expect("create patch1");
    patch1
        .handle(PatchCommand::UpdateOscillator { config: osc1 })
        .expect("osc1");
    patch1
        .handle(PatchCommand::UpdateEnvelope { config: env1 })
        .expect("env1");
    patch1
        .handle(PatchCommand::UpdateFilter { config: flt1 })
        .expect("flt1");
    patch1
        .handle(PatchCommand::SetGain { gain: gain1 })
        .expect("gain1");
    patch1
        .handle(PatchCommand::SetPan { pan: 0.0 })
        .expect("pan1");
    patch1
        .handle(PatchCommand::ActivatePatch)
        .expect("activate1");

    setup.patches.push(build_serialized_patch(
        1,
        "Pad",
        true,
        SerializedEngineType::Sine,
        osc1,
        env1,
        flt1,
        gain1,
        0.0,
        sub1,
        pool1,
        SerializedModMatrix::default(),
        SerializedEffectChain::default(),
    ));

    // ── Patch 2: Bass (Triangle, sub-bass filter, slight right pan) ───────
    let ch2 = MidiChannel::try_new(2).expect("ch2 valid");
    let addr2 = ChannelAddress::new(group, ch2);
    let sub2 = ChannelSubscription::new(addr2, None);
    let pool2 = VoicePoolConfig::try_new(3, StealingPolicy::OldestFirst).expect("pool2 valid");

    let osc2 = OscillatorConfig::try_new(-12.0, 0.5, Waveform::Triangle).expect("osc2 valid");
    let env2 = AmpEnvelopeConfig::try_new(0.003, 0.05, 0.8, 0.25).expect("env2 valid");
    let flt2 = FilterConfig::try_new(2_000.0, FilterType::LowPass, 0.2).expect("flt2 valid");
    let gain2 = Amplitude::try_new(0.30).expect("gain2 valid");

    let mut patch2 = Patch::new(pool2, sub2);
    patch2
        .handle(PatchCommand::CreatePatch {
            name: "Bass".to_string(),
            engine_type: EngineType::Sine,
            subscription: sub2,
        })
        .expect("create patch2");
    patch2
        .handle(PatchCommand::UpdateOscillator { config: osc2 })
        .expect("osc2");
    patch2
        .handle(PatchCommand::UpdateEnvelope { config: env2 })
        .expect("env2");
    patch2
        .handle(PatchCommand::UpdateFilter { config: flt2 })
        .expect("flt2");
    patch2
        .handle(PatchCommand::SetGain { gain: gain2 })
        .expect("gain2");
    patch2
        .handle(PatchCommand::SetPan { pan: 0.25 })
        .expect("pan2");
    patch2
        .handle(PatchCommand::ActivatePatch)
        .expect("activate2");

    setup.patches.push(build_serialized_patch(
        2,
        "Bass",
        true,
        SerializedEngineType::Sine,
        osc2,
        env2,
        flt2,
        gain2,
        0.25,
        sub2,
        pool2,
        SerializedModMatrix::default(),
        SerializedEffectChain::default(),
    ));

    // Keep patch variables alive until here so they outlast the build phase.
    let _ = (patch0, patch1, patch2);

    let patch_configs = vec![
        PatchConfig {
            oscillator: osc0,
            amp_envelope: env0,
            filter: flt0,
            gain: gain0.value(),
            pan: -0.3,
            channel: 0,
            max_voices: 4,
            stealing_policy: StealingPolicy::OldestFirst,
        },
        PatchConfig {
            oscillator: osc1,
            amp_envelope: env1,
            filter: flt1,
            gain: gain1.value(),
            pan: 0.0,
            channel: 1,
            max_voices: 6,
            stealing_policy: StealingPolicy::QuietestFirst,
        },
        PatchConfig {
            oscillator: osc2,
            amp_envelope: env2,
            filter: flt2,
            gain: gain2.value(),
            pan: 0.25,
            channel: 2,
            max_voices: 3,
            stealing_policy: StealingPolicy::OldestFirst,
        },
    ];

    (setup, patch_configs)
}

/// Reconstruct patch configs from a deserialized Setup.
///
/// Parameters come ONLY from the deserialized bytes, proving that the loaded
/// state exactly reproduces the saved state.
fn patch_configs_from_setup(setup: &Setup) -> Vec<PatchConfig> {
    setup
        .patches
        .iter()
        .map(|sp| {
            let oscillator: OscillatorConfig = sp.oscillator.into();
            let amp_envelope: AmpEnvelopeConfig = sp.amp_envelope.into();
            let filter = FilterConfig::try_new(
                sp.filter.cutoff_hz,
                sp.filter.filter_type,
                sp.filter.resonance,
            )
            .unwrap_or_default();
            let stealing_policy: StealingPolicy = sp.stealing_policy.into();
            PatchConfig {
                oscillator,
                amp_envelope,
                filter,
                gain: sp.gain,
                pan: sp.pan,
                channel: sp.midi_channel,
                max_voices: sp.max_voices as usize,
                stealing_policy,
            }
        })
        .collect()
}

// ─── Fixed demo passage ────────────────────────────────────────────────────────

/// A fixed built-in note passage: (time_secs, channel, note, velocity, dur_secs).
///
/// All values are deterministic constants so the same passage renders the same
/// PCM buffer every time.
fn demo_passage() -> Vec<(f64, u8, u8, f64, f64)> {
    vec![
        // Lead melody (ch 0)
        (0.00, 0, 60, 0.75, 0.18),
        (0.22, 0, 62, 0.72, 0.18),
        (0.44, 0, 64, 0.70, 0.18),
        // Pad chord (ch 1)
        (0.00, 1, 60, 0.55, 0.45),
        (0.00, 1, 64, 0.55, 0.45),
        (0.00, 1, 67, 0.55, 0.45),
        // Bass (ch 2)
        (0.00, 2, 36, 0.85, 0.45),
    ]
}

// ─── Render from PatchConfig ──────────────────────────────────────────────────

/// Render RENDER_SAMPLES mono 16-bit PCM samples from the given patch
/// configurations using the fixed demo passage.
///
/// The same deterministic passage is used for both the original and reloaded
/// renders so the two buffers can be compared bit-for-bit.
fn render_from_patch_configs(configs: &[PatchConfig], master_gain: f64) -> Vec<i16> {
    // Build independent voice pools for each patch.
    let mut allocators: Vec<VoiceAllocator> = configs
        .iter()
        .map(|c| VoiceAllocator::new(c.max_voices))
        .collect();

    let patch_mixer = PatchMixer::new();
    let (_global_writer, mut global_reader) =
        GlobalMixer::split(Amplitude::try_new(master_gain).unwrap_or_else(|_| Amplitude::unity()));

    // Build the note-event list: (sample_idx, patch_idx, note_id, note_number,
    //                              velocity, is_on).
    let passage = demo_passage();
    let mut note_events: Vec<(usize, usize, NoteId, NoteNumber, Velocity, bool)> = Vec::new();

    for (raw_id, &(t, ch, note_raw, vel_raw, dur)) in (1_u32..).zip(passage.iter()) {
        let patch_idx = configs.iter().position(|c| c.channel == ch).unwrap_or(0);
        let on_sample = (t * SAMPLE_RATE_F64) as usize;
        let off_sample = ((t + dur) * SAMPLE_RATE_F64) as usize;
        let note_number = NoteNumber::try_new(note_raw).expect("note in range");
        let velocity = Velocity::try_new(vel_raw).expect("velocity in range");
        let note_id = NoteId::new(raw_id);
        note_events.push((on_sample, patch_idx, note_id, note_number, velocity, true));
        note_events.push((off_sample, patch_idx, note_id, note_number, velocity, false));
    }

    // Sort by sample index; note-offs before note-ons at the same position.
    note_events.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| {
            let rank_a = usize::from(a.5); // on=1, off=0
            let rank_b = usize::from(b.5);
            rank_a.cmp(&rank_b)
        })
    });

    let mut samples: Vec<i16> = Vec::with_capacity(RENDER_SAMPLES);
    let mut event_cursor = 0usize;

    for sample_idx in 0..RENDER_SAMPLES {
        // Dispatch due events.
        while event_cursor < note_events.len() && note_events[event_cursor].0 <= sample_idx {
            let (_, patch_idx, note_id, note_number, velocity, is_on) = note_events[event_cursor];
            if is_on {
                allocators[patch_idx].note_on(note_id, note_number, velocity);
            } else {
                let _ = allocators[patch_idx].note_off(note_id);
            }
            event_cursor += 1;
        }

        // Render each patch: mono sample → stereo pan frame.
        let mut mix_frame = AudioFrame::silence();

        for (patch_idx, config) in configs.iter().enumerate() {
            let (mono, _) =
                allocators[patch_idx].render_sample(SAMPLE_RATE_F64, config.oscillator.detune);

            let gain_f = config.gain as f32;
            let theta = ((config.pan + 1.0) * std::f64::consts::FRAC_PI_4) as f32;
            let pan_l = theta.cos();
            let pan_r = theta.sin();

            let patch_frame = AudioFrame::new(mono * gain_f * pan_l, mono * gain_f * pan_r);
            // PatchMixEntry::unity() — per-patch gain already applied above.
            patch_mixer.accumulate(&mut mix_frame, patch_frame, &PatchMixEntry::unity());
        }

        // Apply master gain.
        let master = global_reader.apply([mix_frame.left, mix_frame.right]);

        // Mix to mono.
        let mono_out = (master[0] + master[1]) * 0.5;
        let clamped = mono_out.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        samples.push(pcm);
    }

    samples
}

// ─── Pure-Rust WAV writer (16-bit mono) ───────────────────────────────────────

fn write_wav(path: &std::path::Path, samples: &[i16], sample_rate: u32) {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_chunk_size = (samples.len() * 2) as u32;
    let riff_size = 4 + 24 + 8 + data_chunk_size;

    let mut buf: Vec<u8> = Vec::with_capacity(12 + 24 + 8 + data_chunk_size as usize);

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

    let mut file = std::fs::File::create(path).unwrap_or_else(|e| {
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
    fn build_demo_setup_produces_three_patches() {
        let (setup, configs) = build_demo_setup();
        assert_eq!(setup.patches.len(), 3, "demo setup must have 3 patches");
        assert_eq!(configs.len(), 3, "must have 3 patch configs");
    }

    #[test]
    fn setup_roundtrip_produces_equal_setup() {
        let (original, _) = build_demo_setup();
        let codec = PresetCodec::new();
        let bytes = codec.serialize_setup(original.clone());
        let restored = codec.deserialize_setup(bytes).unwrap();
        assert_eq!(original, restored, "setup must round-trip losslessly");
    }

    #[test]
    fn render_from_patch_configs_produces_nonsilent_output() {
        let (setup, configs) = build_demo_setup();
        let samples = render_from_patch_configs(&configs, setup.master_gain);
        assert_eq!(samples.len(), RENDER_SAMPLES);
        let any_nonzero = samples.iter().any(|&s| s != 0);
        assert!(any_nonzero, "render must produce non-silent output");
    }

    #[test]
    fn render_roundtrip_produces_bit_identical_buffers() {
        let (original_setup, configs) = build_demo_setup();
        let codec = PresetCodec::new();
        let bytes = codec.serialize_setup(original_setup.clone());
        let reloaded_setup = codec.deserialize_setup(bytes).unwrap();

        let buf_original = render_from_patch_configs(&configs, original_setup.master_gain);

        let reloaded_configs = patch_configs_from_setup(&reloaded_setup);
        let buf_reloaded = render_from_patch_configs(&reloaded_configs, reloaded_setup.master_gain);

        assert_eq!(buf_original.len(), buf_reloaded.len());
        for (idx, (orig, reload)) in buf_original.iter().zip(buf_reloaded.iter()).enumerate() {
            assert_eq!(
                orig, reload,
                "sample[{idx}] differs: original={orig}, reloaded={reload}"
            );
        }
    }

    #[test]
    fn demo_passage_has_notes_on_all_three_channels() {
        let passage = demo_passage();
        let channels: Vec<u8> = passage.iter().map(|&(_, ch, _, _, _)| ch).collect();
        assert!(channels.contains(&0), "must have notes on channel 0");
        assert!(channels.contains(&1), "must have notes on channel 1");
        assert!(channels.contains(&2), "must have notes on channel 2");
    }

    #[test]
    fn patch_configs_from_setup_restores_all_fields() {
        let (setup, original_configs) = build_demo_setup();
        let restored = patch_configs_from_setup(&setup);
        assert_eq!(restored.len(), original_configs.len());
        for (r, o) in restored.iter().zip(original_configs.iter()) {
            assert_eq!(r.oscillator, o.oscillator, "oscillator must match");
            assert_eq!(r.amp_envelope, o.amp_envelope, "amp_envelope must match");
            assert!(
                (r.gain - o.gain).abs() < f64::EPSILON,
                "gain must match: {} vs {}",
                r.gain,
                o.gain
            );
            assert!(
                (r.pan - o.pan).abs() < f64::EPSILON,
                "pan must match: {} vs {}",
                r.pan,
                o.pan
            );
        }
    }

    #[test]
    fn patches_have_distinct_oscillator_configs() {
        let (_, configs) = build_demo_setup();
        // All three patches must have different oscillator configs.
        assert_ne!(
            configs[0].oscillator, configs[1].oscillator,
            "patches 0 and 1 should differ"
        );
        assert_ne!(
            configs[0].oscillator, configs[2].oscillator,
            "patches 0 and 2 should differ"
        );
        assert_ne!(
            configs[1].oscillator, configs[2].oscillator,
            "patches 1 and 2 should differ"
        );
    }
}
