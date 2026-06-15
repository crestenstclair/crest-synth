package crestsynth

// ── Editor ─────────────────────────────────────────────
// Keyboard/gamepad-driven parameter editor: a one-way (Elm/Flux) event loop
// over a single store (EditorState) that edits live engine parameters. The
// standalone editor app hosts the live engine (external MIDI in via MidirInput,
// audio out via cpal) and is hermetically smoke-testable with no window/device.
//
// EditorState is the single store. The egui shell and the gamepad adapter both
// emit the SAME EditorEvents into it, so keyboard and gamepad are interchangeable
// and the whole control plane is hermetically testable: feed an event sequence,
// assert focus / edit-mode / field values — no window, no device.

project: contexts: Editor: purpose: "keyboard/gamepad-driven parameter editor: a one-way event loop over a single store that edits live engine parameters"
project: contexts: Editor: ubiquitousLanguage: {
	EditorEvent: "a semantic input event (navigate or edit-mode change) emitted by the keyboard/gamepad adapter — the only thing that mutates editor state"
	EditorState: "the single store: focus position, edit-mode flag, and the list of editable parameter fields"
	ParamField:  "one editable parameter row: label, current value, bounds, and fine step"
	EditMode:    "active only while the edit modifier (J / a gamepad button) is held; directional input then adjusts the focused field's value instead of moving focus"
}

project: contexts: Editor: valueObjects: EditorEvent: {
	from:        "enum"
	description: "NavUp, NavDown, NavLeft, NavRight, EnterEditMode, ExitEditMode — the semantic input vocabulary; keyboard and gamepad adapters both emit these and nothing else"
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with EditorEvent"}]
}

project: contexts: Editor: valueObjects: ParamField: {
	state:       {id: "string", label: "string", value: "f64", min: "f64", max: "f64", step: "f64"}
	description: "one editable parameter row: a label, current value, inclusive bounds, and the fine adjustment step (coarse = 10x step)"
	invariants: ["min <= max", "value is always within [min, max]", "step > 0"]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with ParamField"},
		{kind: "test", command: ["cargo", "test", "param_field"], description: "ParamField clamp/bounds unit tests pass"},
	]
}

project: contexts: Editor: aggregates: EditorState: {
	root:    true
	purpose: "the single editor store: owns focus, edit-mode, and the editable parameter fields; the one entry point that reacts to EditorEvents"
	state: {fields: "Vec<ParamField>", focus: "usize", editMode: "bool"}
	invariants: [
		"apply(EditorEvent) is the ONLY way to mutate editor state; no setters, pure and allocation-free (no I/O, rendering, or audio)",
		"focus always stays within the fields range; navigate-mode directional events move focus by one, saturating at the ends (no wrap)",
		"in navigate mode directional events move focus; in edit mode they adjust the focused field's value instead",
		"in edit mode NavRight = +fine and NavLeft = -fine (one unit); NavUp = +coarse and NavDown = -coarse (ten units = 10x fine)",
		"every value adjustment clamps to the focused field's [min, max]",
	]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with EditorState"},
		{kind: "test", command: ["cargo", "test", "editor_state"], description: "EditorState event-reducer unit tests pass (nav, edit-mode, fine/coarse, clamping)"},
	]
}

// ── Standalone editor app (hosts the live engine) ──────

project: assets: StandaloneUiMain: {
	kind:        "rust-bin-target"
	description: "src/bin/synth_ui.rs: standalone eframe/egui MIXER VIEW, keyboard+gamepad driven (no mouse/touch, no note triggering); hosts the live engine (external MIDI in via MidirInput, audio out via cpal); one-way MixerViewEvent -> MixerView -> view loop; live per-channel metering; hermetic --smoke headless mode"
	uses: [
		"aggregate.Mixer.MixerView",
		"valueObject.Mixer.MixerViewEvent",
		"valueObject.Mixer.MixerParam",
		"aggregate.Patch.ChannelMixer",
		"valueObject.Patch.ChannelStrip",
		"port.DesignSystem.Theme",
		"valueObject.DesignSystem.SemanticToken",
		"valueObject.DesignSystem.Rgba",
		"domainService.DesignSystem.DefaultTheme",
		"asset.MidiFileLoader",
		"adapter.EguiRenderer",
		"adapter.EframeWindow",
		"adapter.GilrsGamepad",
		"port.Shell.GamepadInput",
		"adapter.MidirInput",
		"port.Shell.MidiInput",
		"adapter.CpalAudioOutput",
		"port.Shell.AudioOutput",
		"aggregate.Patch.GlobalMixer",
		"domainService.Patch.PatchMixer",
		"port.Synth.SynthEngine",
		"domainService.Synth.VoiceAllocator",
	]
	prompts: [
		"File path: src/bin/synth_ui.rs",
		"This is the STANDALONE EDITOR app. It opens a real eframe/egui window AND hosts the live synth engine. It must build and run on macOS using eframe's default backend.",
		"Target the CURRENT eframe/egui API (0.28 or newer — the version the manifest pins). Use that line's APIs: eframe::run_native taking a NativeOptions and an app-creator closure receiving &CreationContext returning Result<Box<dyn App>, _>, and the current egui Context/Ui calls. Do NOT use removed/renamed 0.27-era signatures. (The 0.27 line is forbidden because it aborts at window creation on current macOS via icrate 0.0.4.)",
		"SCOPE: this app is the MIXER VIEW (the first real GUI view) and nothing else — do NOT add view-switching or other screens yet. It REPLACES the previous parameter-list editor that used to live here. The goal is to prove the window, the keyboard/gamepad input, the one-way event loop, the mixer view (6-of-16 channel strips with edge scrolling, edit-mode, fine/coarse, double-tap toggles), live per-channel metering, and the live-engine hosting all work end to end.",
		"INPUT IS KEYBOARD + GAMEPAD ONLY. Do NOT implement any mouse or touch interaction — no clickable widgets, no draggable sliders, no hover behavior. (Mouse/touch may be added later; not now.)",
		"This is a MIXER, not a performance surface: there is NO on-screen keyboard and NO note triggering of any kind from the UI. All note performance comes from EXTERNAL MIDI hardware.",
		#"Key bindings (keyboard): W = up, S = down, A = left, D = right. Holding J = edit mode (momentary: edit mode is active only while J is held; releasing J returns to navigate mode). A DOUBLE-TAP of J (two presses within a short window) emits ToggleFocusedParam. The input layer reads raw egui key state each frame and translates it into semantic MixerViewEvents (NavUp/NavDown/NavLeft/NavRight on key-press edges; EnterEditMode/ExitEditMode on the J hold transitions; ToggleFocusedParam on a J double-tap). The double-tap/hold timing lives ONLY in this input layer — never in MixerView. The gamepad adapter (GilrsGamepad / GamepadInput) maps the D-pad to the same Nav events, an Edit/face button to EnterEditMode/ExitEditMode, and a double-tap of that button to ToggleFocusedParam, so keyboard and gamepad emit IDENTICAL MixerViewEvents."#,
		"ONE-WAY EVENT LOOP: the only way UI input changes state is by emitting MixerViewEvents and calling MixerView::apply on them. The egui draw code is a PURE VIEW over MixerView — it never mutates state directly and never reads or writes a channel parameter except through MixerView / its ChannelMixer.",
		#"RENDER THE MIXER VIEW: draw all 6 currently-visible channel strips (the window MixerView exposes via its viewportOffset) SIDE BY SIDE in a single horizontal row — all six must be visible at once, not just the first. Each strip is vertical with these rows top-to-bottom: Volume, Reverb send, Echo send, Pan, Mute, Solo. The VOLUME control IS the level strip: a vertical strip, dark at rest, that animates to show that channel's live peak level (read each channel's PeakLevel from the ChannelMixer) — it both sets volume and meters. Every channel meters independently and metering is UNAFFECTED by solo (a channel silenced by another's solo still shows its own level). Highlight the cursor's (channel, parameter) cell. While Edit (J) is held, the cursor changes color, and when the focused row is Volume the full box containing the level strip is highlighted. Show Mute/Solo as toggle indicators."#,
		#"STRIP LAYOUT — each visible channel strip MUST occupy a FIXED width (e.g. a STRIP_WIDTH constant around 120 px), laid out left-to-right inside one horizontal container, so all 6 strips fit on screen at once. NEVER size a strip, a row, or a separator by `ui.available_width()` — inside the per-strip vertical that returns the whole remaining window width, which makes the FIRST strip consume all horizontal space and pushes strips 2–6 off-screen (this is the single-channel bug; do not reintroduce it). Give each strip its own fixed-width sub-region (e.g. `ui.allocate_ui_with_layout(vec2(STRIP_WIDTH, ...), ...)` or a child ui with `set_width(STRIP_WIDTH)`); the vertical separator between strips is 1 px wide and STRIP-tall, never available_width-wide. Set the window's default inner size wide enough for all 6 strips plus the row labels (at least ~6*STRIP_WIDTH + label gutter, e.g. 820x520) via NativeOptions/viewport so they are visible without resizing."#,
		#"STYLE THROUGH THE DESIGN SYSTEM: the draw code is a SKIN. Construct one DefaultTheme (the DesignSystem::Theme port) once at app construction and resolve EVERY color through `theme.color(SemanticToken::…)`, converting the returned Rgba to egui Color32 only at the point of use (never a hand-written Color32::from_rgb). Use each SemanticToken for the intent its own type defines — the variant set and per-variant meanings live in the SemanticToken value object (FocusRing=focused cell, EditActive=focused-and-edit-mode, ValueFill=continuous value bar, MeterPeak=live peak overlay, ToggleOn/ToggleOff=toggle state, TextDefault/TextMuted=text, PanelBg/Separator=chrome), not restated here. The 'no literal color in draw code; swap the Theme to restyle' rule is the standaloneEditor and DesignSystem invariants."#,
		"Seed a ChannelMixer (16 channels) wired into the live engine and wrap it in a MixerView. Map the 16 mixer channels to the engine's 16 MIDI channels: editing a channel's Volume/Reverb send/Echo send/Pan/Mute/Solo updates the ChannelMixer via MixerView; after each event-loop tick, publish the current per-channel mixer values to the audio engine as a ParameterSnapshot across the phase-3 lock-free real-time seam (no locks/alloc/blocking on the audio callback), and read back each channel's live peak level for the meters.",
		"HOST THE LIVE ENGINE — THE AUDIO PATH MUST ACTUALLY PRODUCE SOUND, not a stub. Open external MIDI input via the MidirInput adapter (Shell::MidiInput) and an audio output stream via the CpalAudioOutput adapter (Shell::AudioOutput). CpalAudioOutput::open_stream starts a cpal callback (on cpal's own internal audio thread) that drains the adapter's internal ring buffer; you MUST feed that buffer with rendered engine audio by calling AudioOutput::write_buffer(&[AudioFrame]) — opening the stream alone yields SILENCE.",
		#"THREADING — a cpal Stream / CpalAudioOutput is NOT Send on macOS (CoreAudio); you CANNOT move it into a thread::spawn closure (that fails to compile with "*mut () cannot be sent between threads safely"). So keep CpalAudioOutput on the thread that created it (the main/UI thread) and render+feed there. Own the VoiceAllocator/SynthEngine/PatchMixer/ChannelMixer/GlobalMixer and the CpalAudioOutput on the main thread; in the eframe update tick drain pending MIDI events and apply them to the VoiceAllocator (note-on allocates/steals; note-off releases), then render AudioFrames via VoiceAllocator -> SynthEngine -> PatchMixer -> ChannelMixer (per-channel volume/pan/solo/mute + peak metering) -> GlobalMixer applying the current mixer values, and read back each channel's PeakLevel from the ChannelMixer for the UI meters. These engine objects must be OWNED and DRIVEN here — never unused/`_`-prefixed. Do NOT spawn a thread that holds the stream or the engine. Call ctx.request_repaint() each update so the loop keeps running."#,
		#"AUDIO FEED RATE — CRITICAL. Feed the cpal ring buffer by its FREE SPACE, not by a guessed rate. Each eframe update tick: let free = cpal_out.available_frames(); render exactly `free` AudioFrames (in BLOCK_SIZE chunks) and write_buffer them. This is self-regulating: the callback consumes at the real sample rate, so `free` each tick equals exactly what was drained since the last tick — you refill the buffer to full using only that much, so the engine advances at exactly real time. This avoids BOTH failure modes: feeding a fixed tiny block (e.g. 64 frames) underfeeds → the callback zero-fills gaps → a ~60 Hz repaint-rate gating buzz (heard as an "incredibly low" tone); and feeding a guessed/elapsed-time amount overshoots → write_buffer overflows and drops frames (console spam + glitches). Do NOT pace by wall-clock elapsed time, do NOT prime the buffer to completely full and then keep pushing, and do NOT push a constant block size. Just: render-and-push exactly available_frames() every tick (it returns the full capacity on the first tick, which primes it). Call ctx.request_repaint() each tick so ticks keep coming faster than the buffer can drain."#,
		#"MIDI sources feed the main-thread render via Send channels only: the external MidirInput callback and the optional --play sequencer thread each SEND MidiEvents (which are Send) over a channel (e.g. std::sync::mpsc, or an rtrb whose Producer is Send) to the main thread, which drains them every update tick. Only Send data (MidiEvents) crosses a thread boundary — never the cpal Stream, never the engine."#,
		"CLI: `synth_ui [--smoke] [--autopilot] [--seconds <N>] [--play <FILE.mid>]`. Default mode opens the window and runs the loop. Parse args yourself; treat any unknown flag as a clear stderr error with non-zero exit. `--smoke` and `--autopilot` are mutually exclusive (error if both given).",
		#"`--autopilot [--seconds <N>]` is the REAL end-to-end run (N defaults to 4) and it must PROVE the app works, not just exit cleanly. It is NOT hermetic: it opens the SAME real eframe window and the SAME cpal audio output and runs the EXACT same update/render/audio path as the default window mode — only input, a built-in note source, and termination are automated. It must be self-contained: it does NOT depend on --play or any external MIDI or file (if --play is also given, honor it, but never depend on it)."#,
		#"AUTOPILOT — BUILT-IN NOTES + REAL AUDIO ASSERTION (catches a silent engine for real, unlike the in-memory smoke check): autopilot drives its OWN deterministic note sequence — each tick (or on a fixed schedule) inject synthetic MidiEvents (note-on/note-off across a few pitches on a couple of MIDI channels) into the SAME VoiceAllocator the live path uses, so the real engine→PatchMixer→ChannelMixer→GlobalMixer→cpal path produces sound. Track the PEAK absolute sample of the AudioFrames ACTUALLY written to the cpal ring buffer (in render_and_feed_audio / write_buffer) across the whole run — the real device-bound audio, not a separate in-memory render. Just before closing, print EXACTLY `autopilot audio peak: <peak>` (the measured peak) and ASSERT IN CODE that peak > 0.0; if it is 0.0 (silent real output) the process MUST exit non-zero (panic with a clear message). This is the assertion that fails when nothing plays."#,
		#"AUTOPILOT — DRIVE THE CONTROL PLANE + ASSERT THE LAYOUT (catches the single-channel bug for real): drive a deterministic scripted sequence of MixerViewEvents through MixerView::apply over the run — navigate across ALL 16 channels (enough NavRight then NavLeft to force viewport edge-scrolling both directions and visit every channel), enter edit mode and nudge a continuous row by fine and coarse steps, and double-tap to toggle a Mute and a Solo — so the live skin renders every state through the DefaultTheme. The draw code must COUNT how many channel strips it actually lays out fully within the window's visible width on a frame (a strip whose allocated rect's right edge exceeds the panel width is NOT visible) and expose that count; just before closing, print EXACTLY `autopilot strips visible: <n>` and ASSERT IN CODE that n == 6 (VISIBLE_CHANNELS). If fewer than 6 strips fit on screen (the off-screen-strip bug), the process MUST exit non-zero. Also save a screenshot of a rendered frame to `autopilot.png` (request it via eframe's screenshot API, e.g. `ViewportCommand::Screenshot` / `frame.screenshot()`, and write the PNG) so the layout has a real artifact."#,
		#"AUTOPILOT — SELF-TERMINATE: after N seconds of wall-clock and after the audio-peak and strips-visible assertions have been evaluated and the screenshot written, CLOSE THE WINDOW ITSELF via `ctx.send_viewport_cmd(egui::ViewportCommand::Close)` so eframe::run_native returns and the process exits 0. It must terminate on its own within roughly N seconds; a non-self-terminating autopilot is a bug. Print `autopilot complete: <K> events` (K = scripted MixerViewEvents applied) as the final line. Exit 0 ONLY if every in-code assertion passed (real audio peak > 0 AND strips visible == 6); any failure or panic is a non-zero exit. `--autopilot` requires a display and a default audio output device (run locally, not headless CI)."#,
		#"Optional `--play <FILE.mid>` flag (dev/audition convenience, NOT a product feature): on launch, load the given Standard MIDI File via the phase-1 MidiFileLoader and run a simple internal sequencer (background thread/timer) that feeds the file's note-on/note-off and other MidiEvents into the SAME VoiceAllocator the external-MIDI path drives, at their scheduled times (honor the file's tempo), looping when the file ends — so you hear the synth while editing without external gear. CRITICAL wiring: the sequencer's events MUST actually reach the audio-producer loop and the VoiceAllocator. If you use a ring buffer between sequencer and audio loop, the CONSUMER half must be drained by the audio-producer loop every block — do NOT discard it (a `_consumer` that is dropped means the notes never sound, which is the exact bug this requirement exists to prevent). This does NOT change the architecture: external MIDI (MidirInput) is still opened and remains the primary note source; --play is just an additional internal MIDI source for monitoring. Without --play, behave exactly as before (external MIDI only). If the file is missing/unparseable, print a clear stderr warning and continue running the editor (do not crash)."#,
		"In --smoke mode, ignore any --play file entirely: do NOT read the file, do NOT start the sequencer, do NOT spawn a player thread — --smoke must stay hermetic (parsing the flag value is fine; touching the file or audio is not).",
		#"`--smoke` mode is HERMETIC and HEADLESS: construct the ENTIRE app state exactly as the window path would — the MixerView wrapping its seeded 16-channel ChannelMixer, the engine objects (VoiceAllocator/SynthEngine/PatchMixer/ChannelMixer/GlobalMixer), and the cpal stream-CONFIG value (sample rate / channels / buffer) — but do NOT call eframe::run_native, do NOT open a window, do NOT open or start any cpal stream or audio device, and do NOT open any MIDI device. Then drive a few MixerViewEvents through MixerView::apply (e.g. navigate channels, enter edit mode, nudge a volume) to confirm the loop is wired, print EXACTLY `ui smoke ok: app constructed`, and continue to the audio-render self-check below."#,
		#"`--smoke` AUDIO SELF-CHECK (this is what catches a silent engine path without any audio device): after constructing state, apply a synthetic note-on (e.g. middle C at full velocity) to the VoiceAllocator and render one block through the EXACT SAME render function the live audio-producer loop uses (VoiceAllocator -> SynthEngine -> PatchMixer -> ChannelMixer -> GlobalMixer). Compute the block's peak absolute sample. If peak > 0 (audible) print EXACTLY `render non-silent: true`; otherwise print `render non-silent: false`. Because the render path runs through the ChannelMixer, the channel carrying that note must also record a non-zero PeakLevel: if any channel's metered peak is > 0 print EXACTLY `channel metered: true`, otherwise `channel metered: false`. Then exit 0. This must call the real render path (NOT a hardcoded constant) so that if the engine graph or the per-channel metering is not actually wired, the check prints false and the validation fails. Use the same render routine the live path feeds to write_buffer — do not duplicate a fake one."#,
		#"`--smoke` THEME SELF-CHECK (proves the design-system seam is wired and exhaustive without a window): build the SAME DefaultTheme the draw path uses, resolve EVERY SemanticToken variant through it (iterate the full variant set — FocusRing, EditActive, ValueFill, MeterPeak, ToggleOn, ToggleOff, TextDefault, TextMuted, PanelBg, Separator — calling theme.color(t) on each), and count how many resolved to an Rgba with no panic and no fallback. Print EXACTLY `theme tokens resolved: N` where N is that count; N MUST equal the number of SemanticToken variants (10). This must drive the real Theme port (not a hardcoded N) so that an unwired or non-exhaustive theme prints the wrong count and fails the validation."#,
		"In --smoke mode never touch cpal/midir device-opening APIs and never enter the eframe event loop; it must return 0 quickly and deterministically on any machine (including CI with no display, no audio, no MIDI). Building config/value objects and rendering audio blocks in-memory is allowed; opening devices or windows is NOT. The tokens `ui smoke ok`, `render non-silent: true`, `channel metered: true`, and `theme tokens resolved: 10` must all appear verbatim in stdout on success.",
	]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with the standalone mixer-view app"},
		{kind: "integration", command: ["make", "ui-smoke"], description: "mixer-view app constructs headlessly AND renders a non-silent audio block through the ChannelMixer with live per-channel metering (catches a stubbed/silent engine path or an unwired meter without a device)", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "ui smoke ok"},
			{kind: "stdout_contains", pattern: "render non-silent: true"},
			{kind: "stdout_contains", pattern: "channel metered: true"},
			{kind: "stdout_contains", pattern: "theme tokens resolved: 10"},
		]},
		{kind: "integration", command: ["make", "autopilot"], description: "REAL end-to-end autopilot run: opens the actual window + cpal audio, drives built-in notes through the live engine and a scripted MixerViewEvent session through the skin, then ASSERTS IN CODE that real device-bound audio was non-silent (peak > 0) and that all 6 channel strips fit on screen — exiting non-zero if the app is silent or only one channel renders. Self-terminates. (macOS-local, needs a display + audio device.)", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "autopilot audio peak:"},
			{kind: "stdout_contains", pattern: "autopilot strips visible: 6"},
			{kind: "stdout_contains", pattern: "autopilot complete:"},
			{kind: "file_exists", path: "autopilot.png"},
		]},
	]
}

// ── Invariants ─────────────────────────────────────────

project: invariants: standaloneEditor: [
	{text: "the standalone UI is keyboard/gamepad driven only — no mouse or touch input in this implementation", meta: rationale: "keeps the initial implementation clean; pointer input can be added later without changing the event-loop core"},
	{text: "the standalone UI is not a performance surface: it originates no notes; all note performance comes from external MIDI", meta: rationale: "the UI's job is mixing/editing, not playing"},
	{text: "the UI mutates state only by emitting MixerViewEvents applied to MixerView; egui draw code is a pure view that reads channel values and per-channel peak levels from the ChannelMixer", meta: rationale: "one-way data flow keeps state changes traceable and the control plane hermetically testable"},
	{text: "the audio model consumes external MIDI plus a published ParameterSnapshot and never observes MixerViewEvents", meta: rationale: "keeps the engine host-agnostic and the realtime path decoupled from the UI event loop"},
	{text: "the ui smoke path opens no window, no audio device, and no MIDI device; it only constructs state and drives the event loop", meta: rationale: "keeps the standalone app mechanically checkable with no display or hardware"},
	{text: "the mixer draw code is a skin: it holds no literal color and resolves every color through a DesignSystem Theme by SemanticToken; the only raw-color touch is converting the Theme's returned Rgba to egui Color32", meta: rationale: "the behavior÷skin÷token seam — swapping the Theme restyles the whole mixer with zero draw-code change (the DesignSystem invariant applied to its first consumer)"},
	{text: "--autopilot is a real end-to-end run that PROVES behavior, not just clean exit: it opens the actual window and audio device, injects its own built-in notes through the live engine, and asserts IN CODE that the real device-bound audio peak is > 0 and that all 6 channel strips fit on screen (exit non-zero on silence or off-screen strips) before self-terminating via a viewport Close", meta: rationale: "the validation must fail when the app is silent or renders one channel; asserting on a self-printed completion token alone is theater that lets real bugs pass"},
	{text: "no validation may assert only on a token the binary prints unconditionally; a behavioral claim (audio plays, N strips render) must be checked in code against the real observed value so a regression makes the process exit non-zero", meta: rationale: "the autopilot's `autopilot complete:` token passed while only one channel rendered and nothing played — observable behavior must be asserted, not narrated"},
	{text: "the window backend must use a current eframe/egui (0.28+ on objc2 0.5+/winit 0.30+), never the 0.27 line that pulls icrate 0.0.4", meta: rationale: "eframe 0.27/objc2-0.3-beta/icrate-0.0.4 aborts at window creation on current macOS (NSScreen enumeration ABI panic); this is invisible to ui-smoke (no window) so it can only be prevented by the dependency pin, not the validation loop"},
]
