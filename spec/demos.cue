package crestsynth

// Demo / proof binaries — the project's validation harness, ported verbatim
// from the original spec (main branch) onto the clean base; only resource
// IDs were remapped to the new context names. Each demo PROVES a behavior
// with a measured value, per the de-theatered validation methodology.
// MixerDemoMain and GamepadNavDemoMain return with the editor increment
// (they exercise the MixerView/GamepadNavigator resources).

project: assets: VoiceDemoMain: {
	kind:        "rust-bin-target"
	description: "src/bin/voice_demo.rs: over-polyphonic passage through SynthEngine/VoiceAllocator that forces voice stealing, renders to WAV"
	uses: ["aggregate.Engine.Voice", "domainService.Engine.VoiceAllocator", "domainService.Engine.EngineRenderer"]
	prompts: [
		"File path: src/bin/voice_demo.rs",
		"CLI: `voice_demo [--out OUT.wav]`. Default output path voice-demo.wav.",
		"Build a VoiceAllocator with a DELIBERATELY SMALL polyphony limit (e.g. maxVoices = 4) and feed it a built-in passage that holds MORE simultaneous notes than the limit (e.g. a rolling cluster of 8-12 overlapping sustained notes), so the allocator is FORCED to steal voices to service new note-ons. The passage must guarantee stealing actually occurs for the chosen limit.",
		"Drive each note through the real engine: trigger note_on/note_off via the VoiceAllocator, render every active Voice through the SynthEngine port (oscillator → filter → amp envelope), and mix via the AudioRenderer in fixed sample blocks.",
		"Track and count every voice steal (each time the allocator reclaims an active voice to service a new note). Maintain a running total across the whole passage.",
		"Print per-stage envelope markers so the envelope progression is observable: at minimum print a line for each envelope stage transition observed (Attack/Decay/Sustain/Release) at least once, and a per-section summary.",
		#"ASSERT IN CODE that the measured total steal count is > 0: if it is 0, panic with a clear message (e.g. `panic!("voice_demo FAILED: passage forced no voice steals")`) so the process exits non-zero and the validation FAILS. Printing `steals=0` and exiting 0 is not acceptable — the in-code assertion on the measured count is what makes a zero-steal regression fail (the `steals=` token alone matches `steals=0`)."#,
		#"Then print EXACTLY a line containing the token `steals=` followed by the integer total (verbatim, lowercase, no spaces around `=`, e.g. `steals=37`) so a human and the validation can see the measured count."#,
		#"WITNESS CLI: also accept `--observe` and optional `--degenerate-stub`. In observe mode, measure peak absolute output, clipping (any abs sample > 1.0), total steals, and the non-zero rendered difference caused by applying per-note pitch/pressure expression to a matching NoteId. Emit exactly one line `CREST_OBSERVATION {\"peak\":<number>,\"clipped\":<bool>,\"steals\":<number>,\"expressive_delta\":<number>}`. The real case MUST compute these from the actual Voice/VoiceAllocator/EngineRenderer output. The degenerate case must run the same harness with an explicitly silent/non-expressive stub so the JSON is schema-valid but the behavioral predicates fail; it exits 0 so crest-spec, not the binary, classifies falsification."#,
		"Write 16-bit mono WAV (default voice-demo.wav, or --out) using a pure-Rust WAV writer (no external WAV crate).",
		"Exit 0 on success.",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "voice demo builds"},
		{kind: "integration", command: ["make", "demo-voices"], description: "over-polyphonic passage renders to WAV and forces voice stealing", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "file_exists", path: "voice-demo.wav"},
			{kind: "stdout_contains", pattern: "steals="},
		]},
	]
	contributesTo: [{capability: "capability.render_expressive_sound", contribution: "provides the falsification-gated expressive polyphony and voice-stealing witness"}]
}

project: assets: SamplePlayDemoMain: {
	kind:        "rust-bin-target"
	description: "src/bin/sample_demo.rs: hermetic SampleLibrary prover — synthesizes a sample, loads it, maps key/velocity zones, interpolates, renders to WAV"
	uses: ["port.MidiFile.MidiFileReader", "aggregate.Sample.SampleSet", "port.Sample.SampleLoader", "domainService.Sample.SamplePlayer"]
	prompts: [
		"File path: src/bin/sample_demo.rs",
		"CLI: `sample_demo [--out OUT.wav]`. Default output path sample-demo.wav.",
		"HERMETIC: at startup, SYNTHESIZE a tiny mono 16-bit WAV sample in code (e.g. a short decaying sine ~0.3s at a known root note) and write it to a TEMP file (std::env::temp_dir() + a unique name). No sample/SF2 file may ship in the repo. Clean the temp file up at the end.",
		"Load that temp WAV through the SampleLoader (applicationService.SampleLibrary.SampleLoader) into a SampleSet aggregate (LoadSampleSet). Build a SampleSet with at least TWO non-overlapping zones differing in KeyVelocityRange (e.g. a low-key zone and a high-key zone, or two velocity layers) sharing the synthesized SampleData via Arc.",
		"Drive a short built-in passage of note-ons at DIFFERENT (note, velocity) pairs chosen so they land in DIFFERENT zones; for each note, look up the matching SampleZone by key+velocity, then read the sample pitch-shifted to the note's frequency through the SampleInterpolator (use Linear interpolation at minimum). Mix the rendered output in fixed sample blocks.",
		"Write 16-bit mono WAV (default sample-demo.wav, or --out) with a pure-Rust WAV writer.",
		#"Print verbatim behavior markers: a line `zones loaded=N` with the zone count, and for each played note a line containing the token `zone hit:` naming which zone matched the (key, velocity) lookup (e.g. `zone hit: low-key (note=48 vel=0.3)`)."#,
		#"ASSERT IN CODE (panic → non-zero exit on any failure) the MEASURED properties — printing the markers is not proof: (1) the loaded zone count is >= 2 (panic if fewer); (2) the played notes hit at least TWO DISTINCT zones — track the set of matched zone identities and panic if fewer than 2 distinct zones were hit, which proves key/velocity routing actually selects different zones rather than the same one every time; (3) interpolation is NOT a no-op — for at least one note played at a different pitch than its zone's root, assert the pitch-shifted interpolated render differs from a same-length read at root pitch (compare sample values / rendered length / peak; panic if identical). Print a line `distinct zones hit=K` with the measured K. Exit 0 ONLY if all three in-code assertions hold — a silent no-op interpolation or single-zone routing MUST fail."#,
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "sample demo builds"},
		{kind: "integration", command: ["make", "demo-samples"], description: "synthesized sample loads, >=2 zones resolve by DISTINCT key/velocity lookup, interpolation provably changes the render (all asserted in-code; a no-op or single-zone run exits non-zero)", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "file_exists", path: "sample-demo.wav"},
			{kind: "stdout_contains", pattern: "zones loaded="},
			{kind: "stdout_contains", pattern: "zone hit:"},
			{kind: "stdout_contains", pattern: "distinct zones hit="},
		]},
	]
}

project: assets: EffectsDemoMain: {
	kind:        "rust-bin-target"
	description: "src/bin/effects_demo.rs: renders the multi-patch demo through per-patch + global EffectChains, proving slot-order and bypass-passthrough to WAV"
	uses: ["port.MidiFile.MidiFileReader", "aggregate.Patch.Patch", "aggregate.Mixer.MixBus", "domainService.Patch.MidiDispatcher", "domainService.Mixer.MixEngine", "aggregate.Effects.EffectChain", "port.Effects.EffectProcessor"]
	prompts: [
		"File path: src/bin/effects_demo.rs",
		"CLI: `effects_demo [FILE.mid] [--out OUT.wav]`. Default output path effects-demo.wav. With no FILE, use the built-in multi-channel demo tune (sustained notes so the effect is audible).",
		"Start from the patch_play setup: 2-3 Patches subscribed to different channels via the ChannelDispatcher into per-patch voice pools, summed via PatchMixer then GlobalMixer.",
		"Provide a tiny in-crate implementation of the EffectProcessor port (port.Effects.EffectProcessor) — a couple of simple effects are enough (e.g. a gain/trim and a single-tap feedback delay). fundsp is NOT a dependency at this phase; do not import it.",
		"Build a per-patch EffectChain for at least one patch with at least TWO EffectSlots, and a global (master) EffectChain on the mix bus. Process signal flow STRICTLY in order: patch voices -> per-patch EffectChain (slot 0 then slot 1 ...) -> PatchMixer -> GlobalMixer -> master EffectChain -> output. Render the whole passage to WAV.",
		#"MECHANICALLY prove slot order: process one short test block through the chain in its declared slot order AND through the reversed slot order, and assert in code that the two outputs DIFFER (panic with a clear message if they are identical). Print a verbatim line `slot order matters: true`."#,
		#"MECHANICALLY prove bypass passthrough: take a short test block, run it through a BYPASSED EffectChain, and assert in code the output is BIT-IDENTICAL to the dry input (panic if not). Print a verbatim line `bypass passthrough: true`."#,
		"Write 16-bit mono WAV (default effects-demo.wav, or --out) with a pure-Rust WAV writer.",
		"Print per-patch/per-chain stats. The `slot order matters: true` and `bypass passthrough: true` tokens MUST appear verbatim so a validation can assert both EffectChain invariants held.",
		"Exit 0 on success (the two in-code assertions must pass for a normal run).",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "effects demo builds"},
		{kind: "integration", command: ["make", "demo-effects"], description: "multi-patch demo renders through effect chains; slot-order and bypass invariants hold", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "file_exists", path: "effects-demo.wav"},
			{kind: "stdout_contains", pattern: "slot order matters: true"},
			{kind: "stdout_contains", pattern: "bypass passthrough: true"},
		]},
	]
}

project: assets: ModPlayMain: {
	kind:        "rust-bin-target"
	description: "src/bin/mod_play.rs: multi-patch MIDI player with the Modulation context active — audible LFO vibrato + filter sweep"
	uses: ["port.MidiFile.MidiFileReader", "aggregate.Patch.Patch", "aggregate.Mixer.MixBus", "domainService.Patch.MidiDispatcher", "domainService.Mixer.MixEngine", "aggregate.Modulation.ModMatrix", "domainService.Modulation.ModProcessor"]
	prompts: [
		"File path: src/bin/mod_play.rs",
		"Start from the patch_play setup: 2-3 Patches with distinct engine settings, each subscribed to a different MIDI channel, fed by the ChannelDispatcher into per-patch voice pools, summed via PatchMixer / GlobalMixer.",
		"For each patch build a ModMatrix (aggregate.Modulation.ModMatrix). Configure at least one LfoConfig (ConfigureLfo) and add routings via AddRouting: (1) an LFO vibrato — ModSourceType::Lfo routed to the pitch ModDestinationType with a small depth; (2) a filter sweep — a ModSourceType (Lfo or an Envelope from a ModEnvelopeConfig) routed to the filter-cutoff ModDestinationType with a clearly audible depth.",
		"Each audio block, run the ModulationProcessor over each patch's ModMatrix to evaluate the mod sources and apply the routed modulation to the destination parameters (pitch / filter cutoff) before rendering that patch's voices.",
		"CLI: `mod_play [FILE.mid] [--out OUT.wav]`. With no FILE, use the built-in multi-channel demo tune (sustained/legato notes so the vibrato and sweep are clearly audible).",
		"Load FILE (when given) with the MidiFileLoader module; otherwise use the built-in timeline.",
		"Write 16-bit mono WAV (default mod-play.wav, or --out) with a pure-Rust WAV writer.",
		#"Print stats: events per patch and peak voices per patch. For the active modulation print a verbatim line per routing tagged with the token `mod routing:` — e.g. `mod routing: LFO vibrato -> pitch` and `mod routing: sweep -> filter cutoff` — so a validation can assert the ModMatrix routings were actually configured and applied. The `mod routing:` token must appear verbatim."#,
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "mod player builds"},
		{kind: "integration", command: ["make", "demo-mod"], description: "modulated demo renders to WAV with active routings", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "file_exists", path: "mod-play.wav"},
			{kind: "stdout_contains", pattern: "mod routing:"},
		]},
	]
}

project: assets: PatchPlayMain: {
	kind:        "rust-bin-target"
	description: "src/bin/patch_play.rs: multi-patch MIDI player — proves dispatcher → per-patch voice pools → global mix end to end"
	uses: ["port.MidiFile.MidiFileReader", "aggregate.Patch.Patch", "aggregate.Mixer.MixBus", "domainService.Patch.MidiDispatcher", "domainService.Mixer.MixEngine", "domainService.Engine.VoiceAllocator"]
	prompts: [
		"File path: src/bin/patch_play.rs",
		"Configure 2-3 Patch aggregates with DISTINCT engine settings (different OscillatorConfig / FilterConfig / AmpEnvelopeConfig and gain/pan), each with its own VoicePoolConfig, and each subscribed (ChannelSubscription) to a DIFFERENT MIDI channel via its ChannelAddress.",
		"CLI: `patch_play [FILE.mid] [--out OUT.wav]`. With no FILE, build a BUILT-IN multi-channel demo tune in code: events spread across the channels the patches subscribe to (so every patch sounds), spanning a few bars.",
		"Load FILE (when given) with the MidiFileLoader module; otherwise use the built-in multi-channel timeline.",
		"Route EVERY event through the ChannelDispatcher to all subscribed patches; each patch drives its OWN VoiceAllocator / voice pool (independent polyphony + stealing), proving one patch cannot exhaust another's voices.",
		"Sum each patch's rendered audio through the PatchMixer (per-patch gain + pan), then the GlobalMixer (master gain), into one output buffer.",
		"Write 16-bit mono WAV (default patch-play.wav, or --out) with a pure-Rust WAV writer.",
		#"Print per-channel / per-patch statistics to stdout. For EACH patch print a line containing the verbatim token `Peak Voices` followed by that patch's peak simultaneous voice count (e.g. `Patch 1 \"Bass\": Peak Voices = 3`). Also print events delivered per patch and voice-steal counts per patch. The `Peak Voices` token must appear verbatim so a validation can assert the per-patch voice accounting ran."#,
		"Purpose: this binary proves the dispatcher → per-patch-pools → global-mix integration works end to end.",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "patch player builds"},
		{kind: "integration", command: ["make", "demo-patches"], description: "multi-channel demo renders through all patches to WAV", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "file_exists", path: "patch-play.wav"},
			{kind: "stdout_contains", pattern: "Peak Voices"},
		]},
	]
}

project: assets: PresetRoundtripDemoMain: {
	kind:        "rust-bin-target"
	description: "src/bin/preset_demo.rs: serializes a full Setup, reloads it, and proves round-trip fidelity by rendering identical audio before/after"
	uses: ["port.MidiFile.MidiFileReader", "aggregate.Patch.Patch", "aggregate.Mixer.MixBus", "domainService.Patch.MidiDispatcher", "domainService.Mixer.MixEngine", "valueObject.Preset.Preset", "aggregate.Preset.Session", "port.Preset.PresetCodec"]
	prompts: [
		"File path: src/bin/preset_demo.rs",
		"CLI: `preset_demo [--out OUT.wav]`. Default output path preset-demo.wav.",
		"Build a full Setup: 2-3 distinct Patches (different OscillatorConfig/FilterConfig/AmpEnvelopeConfig, gain/pan, channel subscriptions) plus master gain. Each Patch's complete state must be captured as a Preset.",
		"Implement the PresetCodec port (port.Presets.PresetCodec) inline using serde + serde_json (derive Serialize/Deserialize on the serialized preset/setup value objects). serialize/deserialize a single Preset and serializeSetup/deserializeSetup for the whole Setup.",
		#"Round-trip the Setup: serializeSetup -> Vec<u8> -> deserializeSetup -> Setup'. Assert in code that Setup' EQUALS the original Setup (derive PartialEq; panic with a clear message on mismatch). Print a verbatim line `setup roundtrip: equal`."#,
		"Render a fixed built-in demo passage through the ORIGINAL Setup to an in-memory buffer, and the SAME passage through the RELOADED Setup' to a second buffer (same dispatcher -> per-patch pools -> PatchMixer -> GlobalMixer path, deterministic, fixed sample blocks).",
		#"Assert in code the two rendered buffers are BIT-IDENTICAL sample-for-sample (panic if any sample differs) — this is the real proof that the preset reproduces the saved sound exactly. Print a verbatim line `render identical: true`."#,
		#"ATOMIC FAILURE PROOF: attempt to restore malformed or unsupported-version session bytes and assert the previously active complete session is byte-for-byte unchanged after the error."#,
		#"WITNESS CLI: also accept `--observe` and optional `--degenerate-stub`. Emit exactly one line `CREST_OBSERVATION {\"roundtrip_equal\":<bool>,\"render_identical\":<bool>,\"failed_restore_atomic\":<bool>}` from the measured comparisons. The degenerate case uses an explicit lossy/non-atomic codec stub through the same harness so at least one field is false while the schema remains valid; it exits 0 for crest-spec classification."#,
		"Write the (identical) rendered audio to 16-bit mono WAV (default preset-demo.wav, or --out) with a pure-Rust WAV writer.",
		"Print stats. The `setup roundtrip: equal` and `render identical: true` tokens MUST appear verbatim so a validation can assert both presetIntegrity invariants held.",
		"Exit 0 on success (both in-code assertions must pass).",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "preset demo builds"},
		{kind: "integration", command: ["make", "demo-presets"], description: "Setup round-trips through the codec and re-renders bit-identical audio", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "file_exists", path: "preset-demo.wav"},
			{kind: "stdout_contains", pattern: "setup roundtrip: equal"},
			{kind: "stdout_contains", pattern: "render identical: true"},
		]},
	]
	contributesTo: [{capability: "capability.save_and_restore_sound_library", contribution: "provides the falsification-gated versioned and atomic persistence witness"}]
}

project: assets: MidiPlayMain: {
	kind:        "rust-bin-target"
	description: "src/bin/midi_play.rs: offline MIDI-file player — renders a .mid (or a built-in demo tune) to WAV through the phase-1 engine"
	uses: ["port.MidiFile.MidiFileReader", "domainService.Engine.EngineRenderer"]
	prompts: [
		"File path: src/bin/midi_play.rs",
		"CLI: `midi_play [FILE.mid] [--out OUT.wav]`. If FILE is omitted, play a BUILT-IN demo melody constructed in code as a short multi-bar tune (a recognizable arpeggio/melody spanning a few seconds) — so no .mid asset file must live in the repo.",
		"When FILE is given, load it with the MidiFileLoader module into the time-ordered (seconds, MidiEvent) timeline.",
		"Render the timeline OFFLINE through the phase-1 engine (SineVoice + AudioRenderer): step in fixed sample blocks, trigger note_on/note_off at the correct sample offsets, and render what the current engine supports — sum simultaneous notes (basic polyphony by summing active voices).",
		"Write 16-bit mono WAV (default path midi-play.wav, or the --out path) using a pure-Rust WAV writer (no external WAV crate).",
		#"Print a one-line-per-section summary to stdout. Include a verbatim line with the token `rendered seconds=` followed by the rendered duration in seconds (e.g. `rendered seconds=4.0`), plus total events, peak simultaneous voices, and the output path. The `rendered seconds=` token must appear verbatim so a validation can assert the offline render actually ran."#,
		"Exit 0 on success; exit non-zero with a clear stderr message if the FILE cannot be parsed.",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "project builds cleanly"},
		{kind: "integration", command: ["make", "demo-midi"], description: "built-in demo tune renders to WAV", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "file_exists", path: "midi-play.wav"},
			{kind: "stdout_contains", pattern: "rendered seconds="},
		]},
	]
}

project: assets: MidiPlayLiveMain: {
	kind:        "rust-bin-target"
	description: "src/bin/midi_play_live.rs: live MIDI-file player — streams a .mid (or built-in demo tune) through the default output device via cpal"
	uses: ["port.MidiFile.MidiFileReader", "adapter.CpalAudioOutput", "domainService.Engine.EngineRenderer", "aggregate.Engine.Voice", "adapter.RtrbEventRing", "adapter.TripleBufferParameterBridge", "adapter.BasedropDeferredDeallocator"]
	prompts: [
		"File path: src/bin/midi_play_live.rs",
		"CLI: `midi_play_live [FILE.mid] [--seconds N]`. If FILE is omitted, play the same built-in demo melody as midi_play. `--seconds N` optionally caps playback duration.",
		"Load FILE (when given) with the MidiFileLoader module into the time-ordered (seconds, MidiEvent) timeline; otherwise use the built-in demo timeline.",
		"Open the default output device through the CpalAudioOutput adapter (the Shell::AudioOutput port). Render the timeline through the phase-2/3 engine (Voice + AudioRenderer) in real time, writing rendered AudioFrames to the output stream as the wall clock advances; respect --seconds if set.",
		"If NO output device is available, exit with a clear non-zero status and a human-readable stderr message (e.g. \"no default output device\") — never panic.",
		"Print a startup line (device name, event count, duration) before streaming. Do NOT write a WAV file — this binary is for live audio only.",
		#"Support a `--no-device-dry-run` flag (mutually exclusive with live playback). In dry-run mode, parse the args and the timeline, and CONSTRUCT the full real-time pipeline objects — the rtrb event ring buffer, the triple_buffer ParameterBridge, and the basedrop DeferredDeallocator plumbing that the live path would use — WITHOUT opening any audio device. Then print EXACTLY a line containing the token `dry-run ok: pipeline constructed` and exit 0. This makes the realtime wiring mechanically checkable with no audio device present."#,
		#"WITNESS CLI: dry-run also accepts `--observe` and optional `--degenerate-stub`. The real observation must push then pop a concrete event, publish two distinct snapshots then read the newest, and retire/collect a tracked value while recording that its destructor did not run on the simulated audio thread. Emit exactly one line `CREST_OBSERVATION {\"event_delivered\":<bool>,\"latest_snapshot_read\":<bool>,\"reclaimed_off_audio_thread\":<bool>}` from those measured facts. The degenerate case runs the same harness with an explicit broken boundary implementation so at least one fact is false, emits valid JSON, and exits 0."#,
		"In dry-run mode never touch cpal's host/device APIs and never block on the wall clock; it must return 0 quickly and deterministically on any machine, including CI.",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "live player compiles"},
		{kind: "integration", command: ["make", "check-live"], description: "realtime pipeline constructs without an audio device", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "dry-run ok"},
		]},
	]
	contributesTo: [
		{capability: "capability.operate_audio_and_midi_devices", contribution: "constructs the same standalone device pipeline used for live playback"},
		{capability: "capability.preserve_realtime_safety", contribution: "provides the falsification-gated lock-free boundary witness"},
	]
}

// Demo / proof binaries — the editor increment's headless provers, ported
// verbatim from the original spec (main branch) onto the clean base; only
// resource IDs were remapped to the new context names (see NOTES.md). These
// are the two assets spec/demos.cue's header comment already anticipates:
// "MixerDemoMain and GamepadNavDemoMain return with the editor increment
// (they exercise the MixerView/GamepadNavigator resources)."

// ── Headless mixer-view prover ─────────────────────────
// mixer_demo proves the mixer view's control logic AND the ChannelStrip's
// solo/metering semantics WITHOUT any device or window: it scripts MixerViewEvent
// sequences through MixerView and asserts the cursor / viewport / values, then
// mixes per-channel buffers through the 16 ChannelStrip channels to prove solo
// silences other channels' AUDIO while every channel still meters its own
// level. The egui/gilrs adapters open real windows/devices, so they are NEVER
// invoked here.

project: assets: MixerDemoMain: {
	kind:        "rust-bin-target"
	description: "src/bin/mixer_demo.rs: headless prover for MixerView + its 16 ChannelStrip channels — scripted events drive cursor/viewport/edit, and a mixdown proves solo-vs-metering independence"
	uses: ["aggregate.Mixer.MixerView", "valueObject.Mixer.MixerViewEvent", "valueObject.Mixer.MixerParam", "aggregate.Mixer.ChannelStrip", "aggregate.Mixer.MixBus", "domainService.Mixer.MixEngine"]
	prompts: [
		"File path: src/bin/mixer_demo.rs",
		"CLI: `mixer_demo`. Takes no arguments and opens NO device and NO window — it is a headless harness over the Mixer view store and the ChannelStrip domain. Do NOT import gilrs, egui, eframe, cpal, or midir.",
		#"EDGE-SCROLL PROOF: build a MixerView (16 channels, 6 visible). Put the cursor on the trailing visible channel (channel index 5) with viewportOffset 0, then feed NavRight and assert IN CODE that viewportOffset becomes 1 AND cursorChannel stays 5 (panic with a clear message on mismatch). Feed a few more NavRights and assert the viewport keeps scrolling while the cursor never leaves the visible window. Then mirror at the left edge (NavLeft scrolls the viewport back, cursor stays). Print a verbatim line `edge scroll ok`."#,
		#"FINE/COARSE PROOF: navigate to a continuous row (e.g. Volume) on some channel, EnterEditMode, then assert NavRight raises that channel's volume by the fine step and NavUp raises it by the coarse step (10x fine), and that values clamp at their upper and lower bounds (repeated NavUp never exceeds the max). Read the values back from the addressed ChannelStrip. Panic on mismatch. Print a verbatim line `fine/coarse ok`."#,
		#"TOGGLE PROOF: navigate to the Mute row of a channel, feed ToggleFocusedParam and assert the channel's mute flips true; feed it again and assert it flips back. Then EnterEditMode on the Mute row and feed NavRight/NavUp and assert the mute flag is UNCHANGED (directional input is a no-op on a toggle). Panic on mismatch. Print a verbatim line `toggle ok`."#,
		#"SOLO-VS-METERING PROOF (this is the one that catches a wrong gating implementation): on a fresh set of 16 ChannelStrip channels (as MixerView wraps them), build 16 small per-channel audio buffers each with a clearly non-zero signal. Solo exactly one channel (e.g. channel 3) via the view (navigate to its Solo row + ToggleFocusedParam). Run the mix pass over the 16 channels' buffers (e.g. via MixEngine, or an equivalent direct mixdown over the ChannelStrip list). Assert IN CODE: (a) the stereo mix equals ONLY channel 3's contribution — every other channel added zero (compare against a reference mix of channel 3 alone); (b) EVERY channel's peak level is still > 0, including the solo-silenced ones (so a channel inaudible due to another's solo still meters its own level). Panic with a clear message on either failure. Print the verbatim lines `solo mutes others: true` and `metering independent of solo: true`."#,
		#"WITNESS CLI: also accept `--observe` and optional `--degenerate-stub`. Emit exactly one line `CREST_OBSERVATION {\"peak\":<number>,\"bounded\":<bool>,\"solo_isolation\":<bool>,\"all_channels_metered\":<bool>}` from the actual MixEngine output and per-strip meters. The degenerate case routes the same inputs through an explicit bypass/summing stub so isolation or metering fails while JSON remains valid; it exits 0 for crest-spec classification."#,
		"Print a short summary. The tokens `edge scroll ok`, `fine/coarse ok`, `toggle ok`, `solo mutes others: true`, and `metering independent of solo: true` MUST appear verbatim so a validation can assert each behavior ran correctly with no device.",
		"Exit 0 on success (all in-code assertions must pass).",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "mixer demo builds"},
		{kind: "integration", command: ["make", "demo-mixer"], description: "scripted mixer-view events drive cursor/viewport/edit/toggle, and the mixdown proves solo silences other channels' audio but not their metering — no device", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "edge scroll ok"},
			{kind: "stdout_contains", pattern: "fine/coarse ok"},
			{kind: "stdout_contains", pattern: "toggle ok"},
			{kind: "stdout_contains", pattern: "solo mutes others: true"},
			{kind: "stdout_contains", pattern: "metering independent of solo: true"},
		]},
	]
	contributesTo: [
		{capability: "capability.mix_to_stereo", contribution: "provides the falsification-gated strip-to-master mixing witness"},
		{capability: "capability.edit_without_pointer", contribution: "proves the complete headless mixer editing event journey"},
	]
}

// ── Gamepad navigation prover ──────────────────────────
// gamepad_demo proves the controller-first navigation logic WITHOUT any device
// or window: it feeds a scripted sequence of raw GamepadEvents through the
// GamepadNavigator (translating them into GamepadActions that drive the app's
// own cursor/edit model) and resolves glyphs through the GlyphResolver for more
// than one controller type. The gilrs/egui/eframe adapters open real devices and
// windows, so they are NEVER invoked by a validation; this demo exercises the
// host-agnostic domain services (GamepadNavigator, GlyphResolver) that the
// adapters merely feed — which is exactly the "UI is a pure view / nav uses the
// app's own cursor model" invariant made checkable.

project: assets: GamepadNavDemoMain: {
	kind:        "rust-bin-target"
	description: "src/bin/gamepad_demo.rs: headless prover for GamepadNavigator + GlyphResolver — scripted events -> GamepadActions -> cursor model, glyph resolution per controller type"
	uses: ["domainService.Shell.GamepadNavigator", "domainService.Shell.GlyphResolver", "aggregate.Mixer.MixerView", "aggregate.Loop.AppState", "domainService.Loop.StateProjector"]
	prompts: [
		"File path: src/bin/gamepad_demo.rs",
		"CLI: `gamepad_demo`. Takes no arguments and opens NO device and NO window — it is a headless harness over the host-agnostic Shell domain services. Do NOT import gilrs, egui, or eframe.",
		"Build a small app cursor/edit model (the app's OWN navigation state, not egui focus). Feed a SCRIPTED, deterministic sequence of raw GamepadEvents through the GamepadNavigator, which must translate them into GamepadActions and drive the cursor/edit model accordingly.",
		#"Assert in code that the scripted events produce the EXPECTED GamepadActions and the EXPECTED final cursor position (panic with a clear message on mismatch). Print a verbatim line `nav actions ok: N` where N is the number of actions dispatched."#,
		#"Drive the GlyphResolver for at least TWO different ControllerTypes (e.g. an Xbox-style and a PlayStation-style controller) and assert each resolves to a DIFFERENT glyph for the same logical button (panic if identical). Print a verbatim line `glyphs resolved: per-controller`."#,
		#"WITNESS CLI: also accept `--observe` and optional `--degenerate-stub`. Drive translated gamepad actions through the real AppState/MixerView reducer, perform an out-of-range adjustment that must clamp, and publish the resulting state through StateProjector. Emit exactly one line `CREST_OBSERVATION {\"actions_dispatched\":<number>,\"state_changed\":<bool>,\"bounded_edit\":<bool>,\"state_published\":<bool>}` from measured state. The degenerate case deliberately bypasses the reducer or projector so a predicate fails while JSON remains valid; it exits 0."#,
		"Print a short summary. The `nav actions ok:` and `glyphs resolved: per-controller` tokens MUST appear verbatim so a validation can assert the navigation + glyph logic ran correctly with no device.",
		"Exit 0 on success (both in-code assertions must pass).",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "gamepad demo builds"},
		{kind: "integration", command: ["make", "check-gamepad"], description: "scripted gamepad events map to actions and glyphs resolve per controller, no device", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "nav actions ok:"},
			{kind: "stdout_contains", pattern: "glyphs resolved: per-controller"},
		]},
	]
	contributesTo: [{capability: "capability.edit_without_pointer", contribution: "provides the falsification-gated gamepad-to-state-to-audio projection journey"}]
}
