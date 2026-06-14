package crestsynth

// ── Crate manifest & build automation ──────────────────
// The root Cargo.toml (dependencies + [[bin]] targets) and the Makefile that
// drives builds, demos, device-free checks, headless smokes, and human-only
// playback/UI targets.

project: assets: RootCargoToml: {
	kind:        "cargo-manifest"
	description: "Root Cargo.toml for the crest-synth project"
	prompts: [
		"Package name: crest-synth, version 0.1.0",
		#"Include [[bin]] section: name = "crest-synth", path = "src/main.rs""#,
		#"Include [[bin]] section: name = "midi_play", path = "src/bin/midi_play.rs""#,
		#"Include [[bin]] section: name = "voice_demo", path = "src/bin/voice_demo.rs""#,
		#"Include [[bin]] section: name = "midi_play_live", path = "src/bin/midi_play_live.rs""#,
		#"Include [[bin]] section: name = "patch_play", path = "src/bin/patch_play.rs""#,
		#"Include [[bin]] section: name = "mod_play", path = "src/bin/mod_play.rs""#,
		#"Include [[bin]] section: name = "sample_demo", path = "src/bin/sample_demo.rs""#,
		#"Include [[bin]] section: name = "effects_demo", path = "src/bin/effects_demo.rs""#,
		#"Include [[bin]] section: name = "preset_demo", path = "src/bin/preset_demo.rs""#,
		#"Include [[bin]] section: name = "gamepad_demo", path = "src/bin/gamepad_demo.rs""#,
		#"Include [[bin]] section: name = "synth_ui", path = "src/bin/synth_ui.rs""#,
		#"Dependencies: `midly` (0.5.x) for SMF parsing; `cpal` for audio output; the lock-free seam crates `rtrb`, `triple_buffer`, `basedrop`; `serde` (with `derive`) and `serde_json` for presets; and the phase-9 adapter crates: `midir` (MIDI I/O), `midi2` (MIDI 1.0 upconversion), `eframe`/`egui` (window + UI), `gilrs` (gamepad input), and `fundsp` (effects DSP)."#,
		#"CRITICAL eframe/egui version pin: depend on a CURRENT eframe/egui release — 0.28 or newer (prefer the latest 0.x line) — that transitively uses `objc2` 0.5+ and `winit` 0.30+. Do NOT use the eframe/egui 0.27 line: it pulls `winit` 0.29 → `objc2` 0.3-beta + `icrate` 0.0.4, which on current macOS aborts at window creation with a non-unwinding panic inside winit's `did_finish_launching` ("invalid message send to NSScreen countByEnumeratingWithState…: expected 'q', found 'Q'"). The crate builds fine and `ui-smoke` passes regardless (it opens no window), so this MUST be pinned here — the validation loop cannot catch a window-creation runtime panic."#,
		"Only include dependencies actually needed by the generated code. The standalone synth_ui binary is a new shell over the existing engine; it reuses the already-declared eframe/egui (window + UI) and cpal (audio) crates and pulls in NO new crate. Do NOT add nih-plug — that is the phase-10 plugin wrapper's dependency and phase 11 must not depend on it.",
	]
}

project: assets: BuildMakefile: {
	kind:        "makefile"
	description: "Build automation for the crest-synth project"
	prompts: [
		"Default target: build",
		"build: cargo build",
		"test: cargo test",
		"check: cargo check",
		"clean: cargo clean",
		"run: cargo run  (runs the default tone-test binary)",
		"lint: cargo clippy -- -D warnings",
		"fmt: cargo fmt -- --check",
		"demo-midi: cargo run --bin midi_play -- $(FILE)  — renders a MIDI file (or built-in demo tune) to midi-play.wav. `make demo-midi FILE=song.mid` forwards the path.",
		"demo-voices: cargo run --bin voice_demo  — renders the over-polyphonic voice-stealing prover to voice-demo.wav and prints a `steals=` count. Takes no FILE argument.",
		"check-live: cargo run --bin midi_play_live -- --no-device-dry-run  — constructs the real-time pipeline without opening an audio device, prints `dry-run ok: pipeline constructed`, exits 0. Validation-safe.",
		"demo-patches: cargo run --bin patch_play -- $(FILE)  — renders the multi-patch integration proof to patch-play.wav. `make demo-patches FILE=song.mid` forwards the path.",
		"demo-mod: cargo run --bin mod_play -- $(FILE)  — renders the modulated (LFO vibrato + filter sweep) demo to mod-play.wav. `make demo-mod FILE=song.mid` forwards the path.",
		"demo-samples: cargo run --bin sample_demo  — synthesizes a sample, loads it, resolves key/velocity zones, interpolates, renders to sample-demo.wav and prints `zones loaded=` / `zone hit:` markers. Hermetic; no FILE argument.",
		"demo-effects: cargo run --bin effects_demo -- $(FILE)  — renders the multi-patch demo through per-patch + global EffectChains to effects-demo.wav and prints `slot order matters: true` / `bypass passthrough: true`. `make demo-effects FILE=song.mid` forwards the path.",
		"demo-presets: cargo run --bin preset_demo  — serializes a full Setup, reloads it, proves bit-identical re-render, writes preset-demo.wav and prints `setup roundtrip: equal` / `render identical: true`. No FILE argument.",
		"check-gamepad: cargo run --bin gamepad_demo  — headless prover for the GamepadNavigator/GlyphResolver domain services; feeds scripted events, asserts action mapping + per-controller glyphs, prints `nav actions ok:` / `glyphs resolved: per-controller`, exits 0. Opens NO device or window; validation-safe.",
		"ui-smoke: cargo run --bin synth_ui -- --smoke  — hermetic headless self-check of the standalone window: constructs the full app state (patches, engine, mixer, cpal stream-config), prints `ui smoke ok: app constructed`, exits 0. Opens NO window and NO audio device; validation-safe.",
		"play-midi: depends on demo-midi, then `afplay midi-play.wav`. `make play-midi FILE=song.mid` plays that file.",
		"play-voices: depends on demo-voices, then `afplay voice-demo.wav`.",
		"play-tone: run the tone test to produce tone-test.wav (cargo run), then `afplay tone-test.wav`.",
		"play-midi-live: cargo run --bin midi_play_live -- $(FILE)  — streams live through the default output device. `make play-midi-live FILE=song.mid` plays that file. Opens an audio device; no afplay; never used by a validation.",
		"play-patches: depends on demo-patches, then `afplay patch-play.wav`.",
		"play-mod: depends on demo-mod, then `afplay mod-play.wav`.",
		"play-samples: depends on demo-samples, then `afplay sample-demo.wav`.",
		"play-effects: depends on demo-effects, then `afplay effects-demo.wav`.",
		"play-presets: depends on demo-presets, then `afplay preset-demo.wav`.",
		#"ui: cargo run --bin synth_ui -- --play "../../../midi/Corridors of Time - Chrono Trigger.mid"  — launches the standalone keyboard/gamepad parameter editor window over the engine + cpal audio, and auto-plays that MIDI file through the engine on launch so you hear the synth while editing (dev/audition convenience; external MIDI remains the primary note source). Quote the path — it contains spaces. The file lives in the repo's midi/ directory, which is ../../../midi/ relative to this workspace. Opens a real window and audio device; human-only; no afplay; NEVER used by a validation."#,
		"Use cargo for all Rust operations. Declare .PHONY for all targets. afplay must appear ONLY in play-midi, play-voices, play-tone, play-patches, play-mod, play-samples, play-effects, and play-presets. demo-*, check-*, and *-smoke targets never use afplay and never open a device or window. The `ui` target opens a real window/device and is human-only, never used by a validation.",
	]
}
