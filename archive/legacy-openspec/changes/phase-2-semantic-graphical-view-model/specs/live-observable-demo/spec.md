## MODIFIED Requirements

### Requirement: Dedicated live standalone entry point
The application SHALL provide retained `make demo-live-graphical-shell` and new cumulative `make demo-live-semantic-view-model` commands backed by distinct interactive CLI modes. `make demo-live` and `--demo-live` SHALL remain compatibility aliases for the newest semantic-view-model scene. The new scene SHALL open the normal Crest Synth window, physical audio output, the two installed SoundFont and Braids capabilities, the configured Chorus effect, and the existing Corridors of Time MIDI fixture with alternating Patch configs; it SHALL retain all prior scalar, structural, graphical-shell, physical-audio, cleanup, and teardown obligations. Every live mode SHALL be mutually exclusive with headless, observation, exhaustive-demo, and controlled-negative modes and SHALL NOT substitute a fake window, null device, offline-only renderer, silent engine, fallback capability, or injected failure.

#### Scenario: User launches the live demo
- **WHEN** the user runs `make demo-live-semantic-view-model` or its `make demo-live` alias with repository fixtures and a usable 48 kHz physical output device
- **THEN** the production window and audio stream open, the fixed fixture begins through the normal input path, alternating SoundFont and Braids Patch identities plus configured Chorus remain visible, semantic traversal runs cumulatively, and both engines produce audible output

#### Scenario: Required live resource is unavailable
- **WHEN** the SoundFont, pinned Braids build, Chorus capability, MIDI fixture, window runtime, or supported physical audio device cannot be opened
- **THEN** live startup fails with a typed visible error and does not silently run a headless, silent, resampled, effect-free, or single-engine substitute

#### Scenario: Live and headless flags are mixed
- **WHEN** `--demo-live`, `--demo-live-graphical-shell`, or `--demo-live-semantic-view-model` is combined with any headless, observation, exhaustive-demo, controlled-negative, or other live-scene flag
- **THEN** argument validation rejects the invocation before application startup

#### Scenario: Retained graphical-shell scene is launched
- **WHEN** the user explicitly runs `make demo-live-graphical-shell`
- **THEN** the retained Phase 1 graphical-shell witness runs with its established CLI mode and obligations rather than being silently redirected to the Phase 2 scene
