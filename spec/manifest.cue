package crestsynth

// Manifest — the crate manifest and the demo binary. This file is the ONLY
// place in the spec where crate dependencies are named; everywhere else the
// spec is language-profile-clean (adapters carry a framework name only).

project: assets: RootCargoToml: {
	kind:        "cargo-manifest"
	description: "Cargo.toml for the crest-synth crate"
	prompts: [
		"File path: Cargo.toml",
		"Package name crest-synth, edition 2021, a lib target plus binary targets under src/bin/.",
		"Dependencies and why each exists: cpal (audio output), midir (MIDI input), eframe + egui (GUI), gilrs (gamepad input), rtrb (lock-free SPSC ring buffer), triple_buffer (lock-free latest-wins parameter sharing), basedrop (deferred deallocation for real-time), serde + serde_json (preset/session serialization), symphonia (audio file decoding), midly (Standard MIDI File parsing).",
		#"CRITICAL eframe/egui version pin: depend on a CURRENT eframe/egui release — 0.28 or newer (prefer the latest 0.x line) — that transitively uses `objc2` 0.5+ and `winit` 0.30+. Do NOT use the eframe/egui 0.27 line: it pulls `winit` 0.29 → `objc2` 0.3-beta + `icrate` 0.0.4, which on current macOS aborts at window creation with a non-unwinding panic inside winit's `did_finish_launching` ("invalid message send to NSScreen countByEnumeratingWithState…: expected 'q', found 'Q'"). The crate builds fine and `ui-smoke` passes regardless (it opens no window), so this MUST be pinned here — the validation loop cannot catch a window-creation runtime panic."#,
		"Choose current stable versions; the whole-tree gate (build/clippy/test) proves the resolution works.",
	]
}

project: assets: ToneTestMain: {
	kind:        "rust-bin-target"
	description: "src/bin/tone_test.rs: renders one second of A440 through the engine and asserts the output is audible"
	uses: ["domainService.Engine.EngineRenderer", "valueObject.Kernel.Frequency", "valueObject.Kernel.AudioFrame"]
	prompts: [
		"File path: src/bin/tone_test.rs",
		"Trigger a single A440 note through the engine, render one second of audio into a buffer, and MEASURE the peak absolute sample value of the rendered buffer.",
		#"Print exactly one line `peak=<value>` with the measured peak, then exit non-zero unless 0.1 < peak <= 1.0 — a silent or clipping render must fail the run."#,
	]
	validations: [
		{kind: "integration", command: ["cargo", "run", "--bin", "tone_test"], description: "renders an audible, non-clipping tone", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "peak="},
		]},
	]
}

project: assets: SynthUiMain: {
	kind:        "rust-bin-target"
	description: "src/bin/synth_ui.rs: the standalone synthesizer application — window, GUI views, gamepad navigation, and MIDI playback through the full engine"
	uses: [
		"port.Shell.AppWindow", "port.Shell.GuiRenderer", "port.Shell.GamepadInput",
		"port.Shell.AudioOutput", "port.Shell.MidiInput",
		"domainService.Engine.EngineRenderer", "domainService.Mixer.MixEngine",
		"domainService.Patch.MidiDispatcher", "domainService.MidiFile.Sequencer",
		"aggregate.Mixer.MixerView", "valueObject.Mixer.MixerViewEvent", "valueObject.Mixer.MixerParam",
		"aggregate.Mixer.ChannelStrip", "aggregate.Mixer.MixBus",
		"port.DesignSystem.Theme", "valueObject.DesignSystem.SemanticToken", "valueObject.DesignSystem.Rgba",
		"domainService.DesignSystem.DefaultTheme",
		"domainService.Engine.VoiceAllocator",
		"port.RealTime.ParameterBridge", "valueObject.RealTime.ParameterSnapshot",
	]
	prompts: [
		"File path: src/bin/synth_ui.rs",
		"The standalone app: open the audio output and the window, render the GUI views, poll the gamepad for navigation, and play notes from connected MIDI inputs and/or a MIDI file through the full engine-to-mixer signal path.",
		#"--play <FILE.mid>: load the file via the MidiFileReader port and sequence it through the engine, looping until quit."#,
		#"--smoke: headless self-check with no window and no audio device — build the full stack (dispatcher, engine, mixer), sequence the first seconds of the --play file (or a synthetic note-on if none was given), render blocks through the SAME render path the live app uses, MEASURE the peak absolute sample and the count of dispatched events, print exactly one line `peak=<value>` and one line `events=<count>`, and exit non-zero unless 0.05 < peak <= 1.0 and events > 0."#,
		"MIXER VIEW SCOPE (editor increment): this app's GUI is the MIXER VIEW (aggregate.Mixer.MixerView over its 16 aggregate.Mixer.ChannelStrip channels) and nothing else — do NOT add view-switching or other screens yet. INPUT IS KEYBOARD + GAMEPAD ONLY: do NOT implement any mouse or touch interaction — no clickable widgets, no draggable sliders, no hover behavior (mouse/touch may be added later; not now). This is a MIXER, not a performance surface: there is NO on-screen keyboard and NO note triggering of any kind from the UI. All note performance comes from EXTERNAL MIDI hardware via port.Shell.MidiInput.",
		#"Key bindings (keyboard): W = up, S = down, A = left, D = right. Holding J = edit mode (momentary: edit mode is active only while J is held; releasing J returns to navigate mode). A DOUBLE-TAP of J (two presses within a short window) emits ToggleFocusedParam. The input layer reads raw egui key state each frame and translates it into semantic MixerViewEvents (NavUp/NavDown/NavLeft/NavRight on key-press edges; EnterEditMode/ExitEditMode on the J hold transitions; ToggleFocusedParam on a J double-tap). The double-tap/hold timing lives ONLY in this input layer — never in MixerView. The gamepad adapter behind port.Shell.GamepadInput maps its d-pad navigation action to the same Nav events, its select action to EnterEditMode/ExitEditMode, and a double-tap of that action to ToggleFocusedParam, so keyboard and gamepad emit IDENTICAL MixerViewEvents."#,
		"ONE-WAY EVENT LOOP: the only way UI input changes state is by emitting MixerViewEvents and calling MixerView::apply on them. The egui draw code is a PURE VIEW over MixerView — it never mutates state directly and never reads or writes a channel parameter except through MixerView / the ChannelStrip channels it wraps.",
		#"RENDER THE MIXER VIEW: draw all 6 currently-visible channel strips (the window MixerView exposes via its viewportOffset) SIDE BY SIDE in a single horizontal row — all six must be visible at once, not just the first. Each strip is vertical with these rows top-to-bottom: Volume, Reverb send, Echo send, Pan, Mute, Solo. The VOLUME control IS the level strip: a vertical strip, dark at rest, that animates to show that channel's live peak level (read each channel's `peak` field from its ChannelStrip) — it both sets volume and meters. Every channel meters independently and metering is UNAFFECTED by solo (a channel silenced by another's solo still shows its own level). Highlight the cursor's (channel, parameter) cell. While Edit (J) is held, the cursor changes color, and when the focused row is Volume the full box containing the level strip is highlighted. Show Mute/Solo as toggle indicators."#,
		#"STRIP LAYOUT — each visible channel strip MUST occupy a FIXED width (e.g. a STRIP_WIDTH constant around 120 px), laid out left-to-right inside one horizontal container, so all 6 strips fit on screen at once. NEVER size a strip, a row, or a separator by `ui.available_width()` — inside the per-strip vertical that returns the whole remaining window width, which makes the FIRST strip consume all horizontal space and pushes strips 2–6 off-screen (this is the single-channel bug; do not reintroduce it). Give each strip its own fixed-width sub-region (e.g. `ui.allocate_ui_with_layout(vec2(STRIP_WIDTH, ...), ...)` or a child ui with `set_width(STRIP_WIDTH)`); the vertical separator between strips is 1 px wide and STRIP-tall, never available_width-wide. Set the window's default inner size wide enough for all 6 strips plus the row labels (at least ~6*STRIP_WIDTH + label gutter, e.g. 820x520) via NativeOptions/viewport so they are visible without resizing."#,
		#"STYLE THROUGH THE DESIGN SYSTEM: the draw code is a SKIN. Construct one DefaultTheme (the DesignSystem::Theme port) once at app construction and resolve EVERY color through `theme.color(SemanticToken::…)`, converting the returned Rgba to egui Color32 only at the point of use (never a hand-written Color32::from_rgb). Use each SemanticToken for the intent its own type defines — the variant set and per-variant meanings live in the SemanticToken value object (FocusRing=focused cell, EditActive=focused-and-edit-mode, ValueFill=continuous value bar, MeterPeak=live peak overlay, ToggleOn/ToggleOff=toggle state, TextDefault/TextMuted=text, PanelBg/Separator=chrome), not restated here. The 'no literal color in draw code; swap the Theme to restyle' rule is the standaloneEditor and designSystem invariants."#,
		"Seed 16 aggregate.Mixer.ChannelStrip channels wired into the live engine and wrap them in a MixerView. Map the 16 mixer channels to the engine's 16 MIDI channels: editing a channel's Volume/Reverb send/Echo send/Pan/Mute/Solo updates the addressed ChannelStrip via MixerView; after each event-loop tick, publish the current per-channel mixer values to the audio engine as a ParameterSnapshot via port.RealTime.ParameterBridge (no locks/alloc/blocking on the audio callback), and read back each channel's live peak level for the meters.",
		"HOST THE LIVE ENGINE — THE AUDIO PATH MUST ACTUALLY PRODUCE SOUND, not a stub. Open external MIDI input via port.Shell.MidiInput and an audio output stream via port.Shell.AudioOutput. The RenderCallback you pass to AudioOutput.open runs on the audio thread (per the port's own contract note) and is invoked by the adapter every time it needs more samples — you MUST render real engine audio into the buffer it hands you on every invocation; returning silence, zeros, or a stub buffer fails this requirement.",
		"THREADING — the stream object behind port.Shell.AudioOutput is NOT Send on macOS (CoreAudio); you CANNOT move it into a thread::spawn closure (that fails to compile with \"*mut () cannot be sent between threads safely\"). So call AudioOutput.open from the thread that owns the engine state (the main/UI thread) and let its RenderCallback closure capture what it needs to render. Own the VoiceAllocator/EngineRenderer/MixEngine/16 ChannelStrip channels/master MixBus on the main thread; in the eframe update tick drain pending MIDI events and apply them to the VoiceAllocator (note-on allocates/steals; note-off releases). Inside the RenderCallback, render AudioFrames via VoiceAllocator -> EngineRenderer -> MixEngine over the 16 ChannelStrip channels (per-channel volume/pan/solo/mute + peak metering) -> master MixBus applying the current mixer values, and read back each channel's peak from its ChannelStrip for the UI meters. These engine objects must be OWNED and DRIVEN here — never unused/`_`-prefixed. Do NOT spawn a thread that holds the Stream or the engine. Call ctx.request_repaint() each update so the UI loop keeps running.",
		"AUDIO CALLBACK CORRECTNESS: the RenderCallback given to AudioOutput.open is invoked by the adapter on cpal's own audio-thread schedule and must synchronously render exactly the frame count it is asked for on every call — do NOT pace it by wall-clock elapsed time, do NOT prime a separate buffer and copy from it in fixed guessed chunks, and do NOT special-case a constant block size. Render precisely the requested span each call so the engine advances at exactly real time with no gaps (heard as silence or a gating buzz) and no overflow/drops.",
		"MIDI sources feed the main-thread render via Send channels only: the port.Shell.MidiInput connection's event callback and the optional --play sequencer (domainService.MidiFile.Sequencer) each SEND MidiEvents (which are Send) over a channel (e.g. std::sync::mpsc, or an rtrb whose Producer is Send) to the main thread, which drains them every update tick or inside the RenderCallback as appropriate. Only Send data (MidiEvents) crosses a thread boundary — never the Stream, never the engine.",
		"CLI: `synth_ui [--smoke] [--autopilot] [--seconds <N>] [--play <FILE.mid>] [--tour]`. Default mode opens the window and runs the loop. Parse args yourself; treat any unknown flag as a clear stderr error with non-zero exit. `--smoke` and `--autopilot` are mutually exclusive (error if both given).",
		#"`--autopilot [--seconds <N>]` is the REAL end-to-end run (N defaults to 4) and it must PROVE the app works, not just exit cleanly. It is NOT hermetic: it opens the SAME real window and the SAME audio output (via port.Shell.AppWindow / port.Shell.AudioOutput) and runs the EXACT same update/render/audio path as the default window mode — only input, a built-in note source, and termination are automated. It must be self-contained: it does NOT depend on --play or any external MIDI or file (if --play is also given, honor it, but never depend on it)."#,
		#"AUTOPILOT — BUILT-IN NOTES + REAL AUDIO ASSERTION (catches a silent engine for real, unlike the --smoke in-memory check): autopilot drives its OWN deterministic note sequence — each tick (or on a fixed schedule) inject synthetic MidiEvents (note-on/note-off across a few pitches on a couple of MIDI channels) into the SAME VoiceAllocator the live path uses, so the real engine->MixEngine->ChannelStrip->MixBus->audio-device path produces sound. Track the PEAK absolute sample of the AudioFrames ACTUALLY rendered into the RenderCallback's buffer across the whole run — the real device-bound audio, not a separate in-memory render. Just before closing, print EXACTLY `autopilot audio peak: <peak>` (the measured peak) and ASSERT IN CODE that peak > 0.0; if it is 0.0 (silent real output) the process MUST exit non-zero (panic with a clear message). This is the assertion that fails when nothing plays."#,
		#"AUTOPILOT — DRIVE THE CONTROL PLANE + ASSERT THE LAYOUT (catches the single-channel bug for real): drive a deterministic scripted sequence of MixerViewEvents through MixerView::apply over the run — navigate across ALL 16 channels (enough NavRight then NavLeft to force viewport edge-scrolling both directions and visit every channel), enter edit mode and nudge a continuous row by fine and coarse steps, and double-tap to toggle a Mute and a Solo — so the live skin renders every state through the DefaultTheme. The draw code must COUNT how many channel strips it actually lays out fully within the window's visible width on a frame (a strip whose allocated rect's right edge exceeds the panel width is NOT visible) and expose that count; just before closing, print EXACTLY `autopilot strips visible: <n>` and ASSERT IN CODE that n == 6. If fewer than 6 strips fit on screen (the off-screen-strip bug), the process MUST exit non-zero. Also save a screenshot of a rendered frame to `autopilot.png` (request it via eframe's screenshot API, e.g. `ViewportCommand::Screenshot` / `frame.screenshot()`, and write the PNG) so the layout has a real artifact."#,
		#"AUTOPILOT — SELF-TERMINATE: after N seconds of wall-clock and after the audio-peak and strips-visible assertions have been evaluated and the screenshot written, CLOSE THE WINDOW ITSELF via `ctx.send_viewport_cmd(egui::ViewportCommand::Close)` so the window's run loop returns and the process exits 0. It must terminate on its own within roughly N seconds; a non-self-terminating autopilot is a bug. Print `autopilot complete: <K> events` (K = scripted MixerViewEvents applied) as the final line. Exit 0 ONLY if every in-code assertion passed (real audio peak > 0 AND strips visible == 6); any failure or panic is a non-zero exit. `--autopilot` requires a display and a default audio output device (run locally, not headless CI)."#,
		#"`--ui-smoke` behavior — enrich the EXISTING `--smoke` hermetic self-check (do not add a second, separate flag) to ALSO cover the mixer view and design system in addition to the engine peak=/events= checks it already performs: construct the ENTIRE mixer-view app state exactly as the window path would — the MixerView wrapping its seeded 16 ChannelStrip channels, the engine objects (VoiceAllocator/EngineRenderer/MixEngine/master MixBus), and the audio stream-CONFIG value (sample rate / channels / buffer) — but do NOT open a window, do NOT open or start any audio stream or device, and do NOT open any MIDI device. Drive a few MixerViewEvents through MixerView::apply (e.g. navigate channels, enter edit mode, nudge a volume) to confirm the loop is wired, print EXACTLY `ui smoke ok: app constructed`, and continue to the audio-render self-check below."#,
		#"`--smoke` MIXER AUDIO SELF-CHECK (this is what catches a silent engine path without any audio device): after constructing state, apply a synthetic note-on (e.g. middle C at full velocity) to the VoiceAllocator and render one block through the EXACT SAME render function the live RenderCallback uses (VoiceAllocator -> EngineRenderer -> MixEngine -> ChannelStrip channels -> master MixBus). Compute the block's peak absolute sample. If peak > 0 (audible) print EXACTLY `render non-silent: true`; otherwise print `render non-silent: false`. Because the render path runs through the ChannelStrip channels, the channel carrying that note must also record a non-zero peak: if any channel's metered peak is > 0 print EXACTLY `channel metered: true`, otherwise `channel metered: false`. Then exit 0. This must call the real render path (NOT a hardcoded constant) so that if the engine graph or the per-channel metering is not actually wired, the check prints false and the validation fails."#,
		#"`--smoke` THEME SELF-CHECK (proves the design-system seam is wired and exhaustive without a window): build the SAME DefaultTheme the draw path uses, resolve EVERY SemanticToken variant through it (iterate the full variant set — FocusRing, EditActive, ValueFill, MeterPeak, ToggleOn, ToggleOff, TextDefault, TextMuted, PanelBg, Separator — calling theme.color(t) on each), and count how many resolved to an Rgba with no panic and no fallback. Print EXACTLY `theme tokens resolved: N` where N is that count; N MUST equal the number of SemanticToken variants (10). This must drive the real Theme port (not a hardcoded N) so that an unwired or non-exhaustive theme prints the wrong count and fails the validation."#,
		"In --smoke mode never touch audio/MIDI device-opening APIs and never enter the window event loop; it must return 0 quickly and deterministically on any machine (including CI with no display, no audio, no MIDI). Building config/value objects and rendering audio blocks in-memory is allowed; opening devices or windows is NOT. The tokens `ui smoke ok`, `render non-silent: true`, `channel metered: true`, and `theme tokens resolved: 10` must all appear verbatim in stdout on success, alongside the existing `peak=`/`events=` tokens.",
		#"OPTIONAL `--tour` flag (design goal carried forward from the original spec's UP-NEXT.md backlog rather than a previously-generated prompt — no prior implementation exists to port verbatim): a `--tour` flag, plus a runtime `T` key toggle, that runs a captioned auto-tour demonstrating every mixer-view feature (navigation, edit-mode, fine/coarse adjust, mute/solo double-tap toggle, viewport edge-scrolling) while looping the --play MIDI file if one was given, mirroring its captions to both an on-screen log panel and stdout. No validation is required for --tour in this increment."#,
	]
	validations: [
		{kind: "integration", command: ["cargo", "run", "--bin", "synth_ui", "--", "--smoke", "--play", "midi/Megalovania.mid"], description: "headless smoke: a real MIDI file drives audible, non-clipping output through the full engine", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "peak="},
			{kind: "stdout_contains", pattern: "events="},
		]},
		{kind: "integration", command: ["cargo", "run", "--bin", "synth_ui", "--", "--smoke", "--play", "midi/Corridors of Time - Chrono Trigger.mid"], description: "format-1 multi-track SMF: notes in non-first tracks must sound (regression: events=0 when only the conductor track was read)", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "peak="},
		]},
		{kind: "integration", command: ["make", "ui-smoke"], description: "mixer-view app constructs headlessly AND renders a non-silent audio block through the ChannelStrip channels with live per-channel metering and a fully-resolved design-system theme (catches a stubbed/silent engine path or an unwired meter/theme without a device)", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "ui smoke ok"},
			{kind: "stdout_contains", pattern: "render non-silent: true"},
			{kind: "stdout_contains", pattern: "channel metered: true"},
			{kind: "stdout_contains", pattern: "theme tokens resolved: 10"},
		]},
		{kind: "integration", command: ["make", "autopilot"], description: "REAL end-to-end autopilot run: opens the actual window + audio device, drives built-in notes through the live engine and a scripted MixerViewEvent session through the skin, then ASSERTS IN CODE that real device-bound audio was non-silent (peak > 0) and that all 6 channel strips fit on screen — exiting non-zero if the app is silent or only one channel renders. Self-terminates. (macOS-local, needs a display + audio device.)", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "autopilot audio peak:"},
			{kind: "stdout_contains", pattern: "autopilot strips visible: 6"},
			{kind: "stdout_contains", pattern: "autopilot complete:"},
			{kind: "file_exists", path: "autopilot.png"},
		]},
	]
}

project: assets: BuildMakefile: {
	kind:        "makefile"
	description: "Makefile: the human entry points for building, testing, and hearing the synth"
	uses: ["asset.SynthUiMain", "asset.ToneTestMain", "asset.VoiceDemoMain", "asset.SamplePlayDemoMain", "asset.EffectsDemoMain", "asset.ModPlayMain", "asset.PatchPlayMain", "asset.PresetRoundtripDemoMain", "asset.MidiPlayMain", "asset.MidiPlayLiveMain", "asset.MixerDemoMain", "asset.GamepadNavDemoMain"]
	prompts: [
		"File path: Makefile",
		"Targets, each with a one-line ## comment shown by a default `help` target: build (cargo build), test (cargo test), lint (cargo clippy --all-targets -- -D warnings), fmt (cargo fmt), tone (run the tone_test proof), smoke (run synth_ui --smoke --play midi/Megalovania.mid), play (run synth_ui --play $(FILE), FILE defaulting to midi/Megalovania.mid), ui (launch the synth_ui app windowed, no --play unless FILE is set), plus demo-scenes (run scenes/check.sh) and scene (run scene_run --scene \"$(FILE)\" --dump-every-step), and one target per proof binary, named EXACTLY as the demo validations invoke them: demo-voices (voice_demo), demo-samples (sample_demo), demo-effects (effects_demo), demo-mod (mod_play), demo-patches (patch_play), demo-presets (preset_demo), demo-midi (midi_play, offline WAV render), check-live (midi_play_live) — each simply cargo-runs its binary with the arguments its validation expects.",
		"Additional targets (editor increment, original names ported from the source spec): ui-smoke (cargo run --bin synth_ui -- --smoke --play midi/Megalovania.mid — the enriched hermetic self-check covering the mixer view + design system, asserting `ui smoke ok`, `render non-silent: true`, `channel metered: true`, `theme tokens resolved: 10`, in addition to the existing peak=/events= tokens); autopilot (cargo run --bin synth_ui -- --autopilot --seconds 4 — the real end-to-end window+audio run that self-drives a scripted MixerViewEvent session, asserts real audio + 6 visible strips in code, writes autopilot.png, and self-terminates; opens a real window/device, no afplay, but IS used by a validation because it is self-driving and self-terminating); demo-mixer (cargo run --bin mixer_demo — headless prover for MixerView + its 16 ChannelStrip channels; opens no device or window); check-gamepad (cargo run --bin gamepad_demo — headless prover for GamepadNavigator/GlyphResolver; opens no device or window).",
		"Plain portable Makefile: .PHONY where appropriate, no shell-specific tricks. Always quote \"$(FILE)\" and any path variable in recipes — MIDI file paths contain spaces.",
	]
	validations: [
		{kind: "custom", command: ["make", "-n", "ui"], description: "ui target exists"},
		{kind: "custom", command: ["make", "-n", "ui-smoke"], description: "ui-smoke target exists"},
		{kind: "custom", command: ["make", "-n", "autopilot"], description: "autopilot target exists"},
		{kind: "custom", command: ["make", "-n", "demo-mixer"], description: "demo-mixer target exists"},
		{kind: "custom", command: ["make", "-n", "check-gamepad"], description: "check-gamepad target exists"},
		{kind: "integration", command: ["make", "smoke"], description: "make smoke runs the audible self-check", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "peak="},
		]},
	]
}

project: assets: SceneRunMain: {
	kind:        "rust-bin-target"
	description: "src/bin/scene_run.rs: execute a scene file and emit snapshots for evaluation"
	uses: ["domainService.Loop.SceneRunner", "port.Loop.SnapshotCodec"]
	prompts: [
		"File path: src/bin/scene_run.rs",
		"scene_run --scene <FILE> [--dump-every-step] [--out <FILE>]: load the scene, run it through SceneRunner, print the FINAL StateSnapshot to stdout as one JSON document; with --dump-every-step print one snapshot JSON per step first.",
		#"After the snapshot, print exactly one summary line: `events_applied=<N> rejections=<M> frames=<F> peak=<final rendered peak>` — measured from the run, and exit non-zero if any event was rejected (a scene that doesn't fully apply is a failed scene)."#,
	]
	validations: [
		{kind: "integration", command: ["make", "demo-scenes"], description: "the starter scene library applies cleanly and asserts state facts", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "events_applied="},
		]},
	]
}

project: assets: SceneLibrary: {
	kind:        "scene-library"
	description: "scenes/: starter scenes proving one behavior each, plus scenes/check.sh asserting state facts from the snapshots"
	uses: ["asset.SceneRunMain"]
	prompts: [
		"Directory: scenes/. Author FOUR scene files in the SnapshotCodec format plus a scenes/check.sh that runs each through scene_run and asserts snapshot facts with jq.",
		"mixer-solo: solo one of three strips, assert the snapshot shows the other two muted=true and the soloed one muted=false (solo exclusivity).",
		"volume-edit: navigate to a strip, enter edit mode, adjust volume down 6 dB, assert the snapshot volume field equals the expected value exactly.",
		"voice-steal: configure polyphony 2 with oldest-steal, fire 3 note-ons with renders between, assert active voice count is 2 and the frame clock equals the step count.",
		"preset-roundtrip: edit a patch, save preset, mutate again, load the preset, assert the snapshot's patch state equals the post-save snapshot's patch state.",
		"Every assertion reads a MEASURED value from the snapshot JSON — never a token the binary prints unconditionally.",
	]
}
