// path: src/bin/voice_demo.rs
//
// voice_demo: over-polyphonic passage that forces voice stealing.
//
// Builds a VoiceAllocator with MAX_VOICES=4 and feeds it a rolling cluster
// of 8-12 overlapping sustained notes, guaranteeing stealing.  Each stolen
// voice increments a counter printed at the end as `steals=N`.
//
// Renders audio through the real Voice aggregate (oscillator → filter → amp
// envelope) and writes 16-bit mono WAV.

use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};

use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::synth::amp_envelope_config::AmpEnvelopeConfig;
use crest_synth::synth::envelope_stage::EnvelopeStage;
use crest_synth::synth::voice::{NoteOff, NoteOn, Voice, VoiceEvent};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum simultaneous voices (deliberately small to force stealing).
const MAX_VOICES: usize = 4;

/// Audio sample rate.
const SAMPLE_RATE: f64 = 44_100.0;

/// Render block size (samples per tick).
const BLOCK_SIZE: usize = 256;

// ─────────────────────────────────────────────────────────────────────────────
// VoiceAllocator
// ─────────────────────────────────────────────────────────────────────────────

/// Simple round-robin voice allocator with stealing.
///
/// Maintains a pool of `MAX_VOICES` voices.  When all voices are active,
/// the oldest voice (index 0 after rotation) is stolen.
struct VoiceAllocator {
    voices: Vec<Voice>,
    steal_count: usize,
    next_steal_index: usize,
}

impl VoiceAllocator {
    fn new(envelope_config: AmpEnvelopeConfig) -> Self {
        let voices = (0..MAX_VOICES)
            .map(|_| Voice::with_config(envelope_config))
            .collect();
        Self {
            voices,
            steal_count: 0,
            next_steal_index: 0,
        }
    }

    /// Trigger a note-on.  Returns any `VoiceEvent`s emitted (including steals).
    fn note_on(&mut self, cmd: NoteOn) -> Vec<VoiceEvent> {
        // Find a free (idle/reclaimable) voice.
        let target = self
            .voices
            .iter()
            .position(|v| v.is_reclaimable())
            .unwrap_or_else(|| {
                // No free voice — steal the oldest.
                let idx = self.next_steal_index % MAX_VOICES;
                self.next_steal_index += 1;
                idx
            });

        let events = self.voices[target].note_on(cmd);

        // Count steals.
        for ev in &events {
            if matches!(ev, VoiceEvent::VoiceStolen { .. }) {
                self.steal_count += 1;
            }
        }

        events
    }

    /// Trigger a note-off on the voice currently playing `note_id`.
    fn note_off(&mut self, cmd: NoteOff) {
        for voice in &mut self.voices {
            if voice.is_active() && voice.note_id() == cmd.note_id {
                let _ = voice.note_off(cmd);
                return;
            }
        }
    }

    /// Render one sample by summing all active voices.
    ///
    /// Also collects `VoiceFinished` events (to observe envelope completion).
    fn render_sample(&mut self, sample_rate: f64) -> (f32, Vec<VoiceEvent>) {
        let mut sum = 0.0_f32;
        let mut events = Vec::new();
        for voice in &mut self.voices {
            let (s, ev) = voice.render_sample(sample_rate, 0.0);
            sum += s;
            if let Some(e) = ev {
                events.push(e);
            }
        }
        (sum, events)
    }

    fn steal_count(&self) -> usize {
        self.steal_count
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// WAV writer (pure Rust, no external crate)
// ─────────────────────────────────────────────────────────────────────────────

fn write_wav_header(writer: &mut impl Write, num_samples: u32, sample_rate: u32) -> io::Result<()> {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate: u32 = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
    let block_align: u16 = num_channels * bits_per_sample / 8;
    let data_size: u32 = num_samples * u32::from(block_align);
    let riff_size: u32 = 36 + data_size;

    // RIFF header
    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;

    // fmt chunk
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?; // chunk size
    writer.write_all(&1u16.to_le_bytes())?; // PCM
    writer.write_all(&num_channels.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk header
    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;

    Ok(())
}

fn f32_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

// ─────────────────────────────────────────────────────────────────────────────
// Envelope stage tracker (for printing transitions)
// ─────────────────────────────────────────────────────────────────────────────

/// Tracks per-voice envelope stage transitions for logging.
struct EnvelopeTracker {
    stages: Vec<EnvelopeStage>,
    /// Set of (stage variant name) transitions already printed (de-duplicated).
    printed: std::collections::HashSet<String>,
}

impl EnvelopeTracker {
    fn new() -> Self {
        Self {
            stages: vec![EnvelopeStage::Idle; MAX_VOICES],
            printed: std::collections::HashSet::new(),
        }
    }

    fn observe(&mut self, allocator: &VoiceAllocator) {
        for (i, voice) in allocator.voices.iter().enumerate() {
            let current = voice.envelope_stage();
            let prev = self.stages[i];
            if current != prev {
                let key = format!("{prev:?}->{current:?}");
                if !self.printed.contains(&key) {
                    println!("  envelope transition [voice {i}]: {prev:?} -> {current:?}");
                    self.printed.insert(key);
                }
                self.stages[i] = current;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Passage definition
// ─────────────────────────────────────────────────────────────────────────────

/// A scheduled note event.
struct ScheduledNote {
    /// Sample offset at which to trigger note_on.
    on_at: usize,
    /// Sample offset at which to trigger note_off (or None to let it ring).
    off_at: Option<usize>,
    note_number: u8,
    velocity: f64,
}

/// Build a rolling cluster passage that forces voice stealing.
///
/// We start notes every 500 ms but hold each one for 2 seconds with MAX_VOICES=4.
/// After the 4th note there will always be more than 4 active notes → stealing.
fn build_passage() -> Vec<ScheduledNote> {
    // Each note-on is 500 ms apart, each note sustains for ~2.0 s.
    // With 4 voice slots and 2 s duration / 0.5 s onset = 4 notes in flight at
    // any moment after the first 2 s.  Notes 5+ steal voices.
    let on_interval_samples = (SAMPLE_RATE * 0.5) as usize; // 500 ms
    let sustain_samples = (SAMPLE_RATE * 2.0) as usize; // 2 s hold

    // A chromatic cluster: 12 notes, each on a successive semitone
    let note_numbers: [u8; 12] = [60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71];

    let mut notes = Vec::new();
    for (i, &nn) in note_numbers.iter().enumerate() {
        let on_at = i * on_interval_samples;
        let off_at = on_at + sustain_samples;
        notes.push(ScheduledNote {
            on_at,
            off_at: Some(off_at),
            note_number: nn,
            velocity: 0.8,
        });
    }

    notes
}

// ─────────────────────────────────────────────────────────────────────────────
// main
// ─────────────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    // ── Parse CLI args ────────────────────────────────────────────────────────
    let args: Vec<String> = env::args().collect();
    let mut out_path = String::from("voice-demo.wav");
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--out" && i + 1 < args.len() {
            out_path = args[i + 1].clone();
            i += 2;
        } else {
            i += 1;
        }
    }

    println!("voice_demo: MAX_VOICES={MAX_VOICES}, output={out_path}");

    // ── Build allocator ───────────────────────────────────────────────────────
    // Fast envelope so steals are observable quickly:
    //   attack=10 ms, decay=50 ms, sustain=0.7, release=100 ms
    let env_cfg = AmpEnvelopeConfig::try_new(0.01, 0.05, 0.7, 0.1).expect("valid envelope config");
    let mut allocator = VoiceAllocator::new(env_cfg);

    // ── Build passage ─────────────────────────────────────────────────────────
    let passage = build_passage();
    let total_samples = passage
        .iter()
        .map(|n| n.off_at.unwrap_or(n.on_at) + (SAMPLE_RATE * 1.5) as usize)
        .max()
        .unwrap_or(0);

    println!(
        "  passage: {} notes, {} s ({} samples)",
        passage.len(),
        total_samples as f64 / SAMPLE_RATE,
        total_samples
    );

    // ── Render ────────────────────────────────────────────────────────────────
    let mut samples: Vec<i16> = Vec::with_capacity(total_samples);
    let mut tracker = EnvelopeTracker::new();

    // Build sorted event list
    #[derive(Clone)]
    enum Event {
        NoteOn {
            sample: usize,
            note_id: u32,
            note_number: u8,
            velocity: f64,
        },
        NoteOff {
            sample: usize,
            note_id: u32,
        },
    }

    let mut events: Vec<Event> = Vec::new();
    for (idx, note) in passage.iter().enumerate() {
        events.push(Event::NoteOn {
            sample: note.on_at,
            note_id: idx as u32 + 1,
            note_number: note.note_number,
            velocity: note.velocity,
        });
        if let Some(off) = note.off_at {
            events.push(Event::NoteOff {
                sample: off,
                note_id: idx as u32 + 1,
            });
        }
    }
    events.sort_by_key(|e| match e {
        Event::NoteOn { sample, .. } => *sample,
        Event::NoteOff { sample, .. } => *sample,
    });

    let mut event_idx = 0;
    let mut section = 0usize;

    for s in 0..total_samples {
        // Dispatch scheduled events for this sample.
        while event_idx < events.len() {
            let ev_sample = match &events[event_idx] {
                Event::NoteOn { sample, .. } => *sample,
                Event::NoteOff { sample, .. } => *sample,
            };
            if ev_sample > s {
                break;
            }
            match events[event_idx].clone() {
                Event::NoteOn {
                    note_id,
                    note_number,
                    velocity,
                    ..
                } => {
                    let nn = NoteNumber::try_new(note_number).expect("note number in range");
                    let vel = Velocity::try_new(velocity).expect("velocity in range");
                    let cmd = NoteOn {
                        note_id: NoteId::new(note_id),
                        note_number: nn,
                        velocity: vel,
                    };
                    let voice_events = allocator.note_on(cmd);
                    for ve in &voice_events {
                        match ve {
                            VoiceEvent::VoiceStolen {
                                old_note_id,
                                new_note_id,
                            } => {
                                println!(
                                    "  [s={s}] STEAL: old={old_note_id} new={new_note_id} (total steals={})",
                                    allocator.steal_count()
                                );
                            }
                            VoiceEvent::VoiceActivated {
                                note_id,
                                note_number,
                                ..
                            } => {
                                println!(
                                    "  [s={s}] note_on: id={note_id} note={}",
                                    note_number.value()
                                );
                            }
                            _ => {}
                        }
                    }
                }
                Event::NoteOff { note_id, .. } => {
                    let cmd = NoteOff {
                        note_id: NoteId::new(note_id),
                    };
                    allocator.note_off(cmd);
                }
            }
            event_idx += 1;
        }

        // Per-section summary every 22050 samples (0.5 s).
        let new_section = s / 22050;
        if new_section != section {
            section = new_section;
            println!(
                "  [section {section}] steals so far: {}",
                allocator.steal_count()
            );
        }

        // Render one sample.
        let (raw, voice_events) = allocator.render_sample(SAMPLE_RATE);

        // Log VoiceFinished events.
        for ve in &voice_events {
            if let VoiceEvent::VoiceFinished { note_id } = ve {
                println!("  [s={s}] VoiceFinished: id={note_id}");
            }
        }

        // Track envelope transitions.
        if s % BLOCK_SIZE == 0 {
            tracker.observe(&allocator);
        }

        // Soft clip the mix to prevent clipping artefacts from polyphony.
        let clipped = (raw / MAX_VOICES as f32).clamp(-1.0, 1.0);
        samples.push(f32_to_i16(clipped));
    }

    // Final envelope observation.
    tracker.observe(&allocator);

    // ── Print steal total ─────────────────────────────────────────────────────
    // IMPORTANT: this exact format is required by the validation.
    println!("steals={}", allocator.steal_count());

    // Verify at least one steal occurred.
    assert!(
        allocator.steal_count() > 0,
        "passage must force at least one voice steal"
    );

    // ── Write WAV ─────────────────────────────────────────────────────────────
    let file = File::create(&out_path)?;
    let mut writer = BufWriter::new(file);
    write_wav_header(&mut writer, samples.len() as u32, SAMPLE_RATE as u32)?;
    for sample in &samples {
        writer.write_all(&sample.to_le_bytes())?;
    }
    writer.flush()?;

    println!("wrote {} samples to {out_path}", samples.len());
    Ok(())
}
