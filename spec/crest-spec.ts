import {
  project,
  command,
  event,
  layer,
} from "../../krusty-spec/.worktrees/feat-subagent-cli/src/index.js";

// ════════════════════════════════════════════════════════════════
// crest-synth — Phase 1: Plumbing that makes noise
// ════════════════════════════════════════════════════════════════
//
// cpal output + midir input + sine voice.
// On note-on play a sine; on note-off stop it.
// Normalize MIDI through midi2 into (group, channel) + high-res + note-id.

const app = project("crest-synth", {
  layers: ["domain", "application", "infrastructure"],
  rules: [
    layer("domain").dependsOn([]),
    layer("application").dependsOn(["domain"]),
    layer("infrastructure").dependsOn(["domain", "application"]),
  ],
  meta: {
    language: "rust",
    style: "idiomatic Rust; lock-free audio thread; gamepad-driven UI",
    avoid: [
      "heap allocation on audio thread",
      "mutex locks on audio thread",
      "blocking I/O on audio thread",
    ],
  },
});

// ── Kernel ──────────────────────────────────────────────

const kernel = app.context("Kernel", {
  purpose: "shared value types for MIDI addressing, audio primitives, and note identity",
  ubiquitousLanguage: {
    MidiEvent: "normalized internal event addressed by (group, channel) with high-res values and note-id",
    NoteId: "unique identifier for a sounding note, enabling per-note expression",
  },
});

kernel.valueObject("MidiGroup", {
  from: "u8",
  description: "MIDI 2.0 group index (0-15)",
  invariants: ["must be 0-15"],
});

kernel.valueObject("MidiChannel", {
  from: "u8",
  description: "MIDI channel (0-15 within a group)",
  invariants: ["must be 0-15"],
});

kernel.valueObject("NoteId", {
  from: "u32",
  description: "unique identifier for a sounding note, enabling per-note expression",
});

kernel.valueObject("NoteNumber", {
  from: "u8",
  description: "MIDI note number (0-127)",
  invariants: ["must be 0-127"],
});

kernel.valueObject("Velocity", {
  from: "f64",
  description: "normalized note velocity (0.0-1.0), upconverted from MIDI 1.0 7-bit to high-res",
  invariants: ["must be 0.0-1.0"],
});

kernel.valueObject("MidiEvent", {
  state: {
    group: "MidiGroup",
    channel: "MidiChannel",
    noteId: "NoteId",
    kind: "MidiEventKind",
    noteNumber: "NoteNumber",
    velocity: "Velocity",
    value: "f64",
  },
  description: "normalized internal event: (group, channel) addressed, high-res values, note-id tagged",
});

kernel.valueObject("SampleRate", {
  from: "u32",
  description: "audio sample rate in Hz (e.g. 44100, 48000)",
  invariants: ["must be positive"],
});

kernel.valueObject("AudioFrame", {
  state: { left: "f32", right: "f32" },
  description: "one stereo sample pair",
});

// ── Shell ───────────────────────────────────────────────

const shell = app.context("Shell", {
  purpose: "application shell: wires audio output, MIDI input, and the window to the engine",
});

shell.port("AudioOutput", {
  contract: {
    openStream: "SampleRate -> AudioStream",
    writeBuffer: "[AudioFrame] -> ()",
  },
});

shell.port("MidiInput", {
  contract: {
    listPorts: "() -> Vec<MidiPortInfo>",
    connect: "MidiPortId -> MidiConnection",
    nextEvent: "() -> Option<RawMidiMessage>",
  },
});

shell.port("MidiNormalizer", {
  contract: {
    normalize: "RawMidiMessage -> MidiEvent",
  },
});

shell.port("AppWindow", {
  contract: {
    create: "WindowConfig -> Window",
    runLoop: "FrameCallback -> ()",
  },
});

// ── Audio ───────────────────────────────────────────────
// Throwaway sine voice — just enough to hear MIDI.
// Replaced in a later phase by a real polyphonic engine.

const audio = app.context("Audio", {
  purpose: "minimal audio rendering: sine voice to prove MIDI-in-to-sound-out path works",
});

const sineVoice = audio.aggregate("SineVoice", {
  root: true,
  purpose: "plays a sine wave at a given pitch; placeholder for real synthesis",
  state: {
    noteId: "NoteId",
    noteNumber: "NoteNumber",
    frequency: "f64",
    phase: "f64",
    active: "bool",
  },
  commands: [
    command("NoteOn", { noteId: "NoteId", noteNumber: "NoteNumber", velocity: "Velocity" }),
    command("NoteOff", { noteId: "NoteId" }),
  ],
  events: [
    event("VoiceStarted", { noteId: "NoteId", frequency: "f64" }),
    event("VoiceStopped", { noteId: "NoteId" }),
  ],
  invariants: [
    "frequency must be positive",
    "phase wraps at 2*PI",
    "at most one voice per noteId",
  ],
});

audio.domainService("AudioRenderer", {
  purpose: "mixes all active SineVoices into an output buffer each audio callback",
  uses: [sineVoice],
});

// ── Asset Kinds ─────────────────────────────────────────

app.assetKind("cargo-manifest", {
  description: "Rust Cargo.toml project manifest",
  filePattern: "Cargo.toml",
  prompts: [
    "Use edition 2021",
    "Only include dependencies actually needed by the generated code",
    "Include [lib] section with path = \"src/lib.rs\"",
    "Include [[bin]] section: name = \"crest-synth\", path = \"src/main.rs\"",
  ],
});

app.assetKind("makefile", {
  description: "GNU Makefile for build automation",
  filePattern: "Makefile",
  prompts: [
    "Include targets: build, test, clean, check, run",
    "Use cargo for all Rust operations",
  ],
});

app.assetKind("rust-binary", {
  description: "Rust main.rs binary entry point",
  filePattern: "src/main.rs",
  prompts: [
    "This is a runnable binary — it must compile and execute with `cargo run`",
    "Use only types from the crate's own lib (no external dependencies)",
    "Include a write_wav helper that writes 16-bit stereo PCM WAV files using std::io only",
  ],
});

app.assetKind("rust-module-declaration", {
  description: "Rust mod.rs or lib.rs module declaration file",
  prompts: [
    "Only output module declarations (pub mod) and re-exports",
    "Add #![allow(non_snake_case)] if any module name uses PascalCase",
    "Do not add any implementation code",
  ],
});

// ── Project Assets ──────────────────────────────────────

app.asset("RootCargoToml", {
  kind: "cargo-manifest",
  description: "Root Cargo.toml for the crest-synth project",
  prompts: [
    "Package name: crest-synth, version 0.1.0",
    "No external dependencies needed (pure Rust for phase 1)",
  ],
});

app.asset("BuildMakefile", {
  kind: "makefile",
  description: "Build automation for the crest-synth project",
  prompts: [
    "Default target: build",
    "test: cargo test",
    "check: cargo check",
    "clean: cargo clean",
    "run: cargo run",
  ],
});

app.asset("ToneTestMain", {
  kind: "rust-binary",
  description: "src/main.rs tone test: exercises AudioRenderer to prove MIDI-to-sound path",
  prompts: [
    "File path: src/main.rs",
    "Import kernel types and Audio::AudioRenderer from the crate lib",
    "Create an AudioRenderer at 44100 Hz",
    "Play a 3-second C4-E4-G4 arpeggio (notes at 0.0s, 0.5s, 1.0s; each ~0.4s duration)",
    "Render in 256-sample blocks, triggering note_on/note_off at the correct sample offsets",
    "Write output to tone-test.wav using a pure-Rust WAV writer (no external crates)",
    "Print the output filename and a short description to stdout",
  ],
});

// ── Module Assets ───────────────────────────────────────

app.asset("LibRs", {
  kind: "rust-module-declaration",
  description: "Root src/lib.rs module declarations",
  prompts: [
    "File path: src/lib.rs",
    "Declare modules: kernel, Shell, Audio",
  ],
});

kernel.asset("KernelMod", {
  kind: "rust-module-declaration",
  description: "src/kernel/mod.rs module declarations for Kernel context",
  prompts: [
    "File path: src/kernel/mod.rs",
    "Declare modules for: MidiGroup, MidiChannel, NoteId, NoteNumber, Velocity, MidiEvent, SampleRate, AudioFrame",
  ],
});

shell.asset("ShellMod", {
  kind: "rust-module-declaration",
  description: "src/Shell/mod.rs module declarations for Shell context",
  prompts: [
    "File path: src/Shell/mod.rs",
    "Declare modules for: AudioOutput, MidiInput, MidiNormalizer, AppWindow",
  ],
});

audio.asset("AudioMod", {
  kind: "rust-module-declaration",
  description: "src/Audio/mod.rs module declarations for Audio context",
  prompts: [
    "File path: src/Audio/mod.rs",
    "Declare modules for: SineVoice, AudioRenderer",
  ],
});
