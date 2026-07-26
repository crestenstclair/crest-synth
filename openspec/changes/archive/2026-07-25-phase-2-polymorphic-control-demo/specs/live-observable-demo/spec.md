## MODIFIED Requirements

### Requirement: Dedicated live standalone entry point
The application SHALL provide `make demo-live`, backed by a dedicated interactive CLI mode, that opens the normal Crest Synth window, physical audio output, the two installed SoundFont and Braids capabilities, and existing Corridors of Time MIDI fixture with alternating Patch configs. The live mode SHALL be mutually exclusive with headless, observation, exhaustive-demo, and controlled-negative modes and SHALL NOT substitute a fake window, null device, offline-only renderer, silent engine, or fallback capability.

#### Scenario: User launches the live demo
- **WHEN** the user runs `make demo-live` with the repository fixtures and a usable 48 kHz physical output device
- **THEN** the production window and audio stream open, the fixed fixture begins through the normal input path, alternating SoundFont and Braids Patch identities are visible, and both engines produce audible output

#### Scenario: Required live resource is unavailable
- **WHEN** the SoundFont, pinned Braids build, MIDI fixture, window runtime, or supported physical audio device cannot be opened
- **THEN** live startup fails with a typed visible error and does not silently run a headless, silent, resampled, or single-engine substitute

#### Scenario: Live and headless flags are mixed
- **WHEN** `--demo-live` is combined with any headless, observation, exhaustive-demo, or controlled-negative flag
- **THEN** argument validation rejects the invocation before application startup

### Requirement: Exact current editable-surface coverage
Before dispatching its first scene action, the live demo SHALL derive and freeze an ordered unique expected set containing every Patch mixer and ADSR value for every installed Patch, every descriptor-classified Scalar parameter for each matching engine, and every editable global parameter. Every expected parameter instance SHALL receive at least one accepted value change, SHALL remain visibly projected for at least 500 milliseconds, and SHALL receive a generation-correlated audible observation.

#### Scenario: Current parameter surface is exercised
- **WHEN** the scene reaches its coverage-comparison step
- **THEN** exercised identifiers exactly equal the frozen descriptor-derived identifiers, include Model/Timbre/Color once for every Braids Patch and never for SoundFont Patches, and both missing and unexpected sets are empty

#### Scenario: Parameter is omitted or only visited
- **WHEN** an expected mixer, envelope, engine, or global parameter is missing, duplicated, selected without changing, not projected, lacks its dwell, or lacks a qualifying audio observation
- **THEN** the live report is incomplete and identifies the coverage or checkpoint failure

### Requirement: Bounded measured audio consequence
Each accepted parameter checkpoint SHALL use finite measurements from the actual physical mixed-engine render path and SHALL require an audio observation whose sequence advanced after dispatch and whose parameter generation equals the accepted generation. Gain/master edits SHALL observe output level, pan SHALL observe stereo balance, sends SHALL observe their corresponding effect input, shared effect controls SHALL observe wet output, and ADSR/Braids controls SHALL observe nonzero output plus their deterministic offline audible-difference proof.

#### Scenario: Audible observation follows an edit
- **WHEN** the callback renders nonzero SoundFont or Braids fixture audio using the accepted parameter generation
- **THEN** the checkpoint records the relevant finite mixer/output measurement and its declared parameter-specific predicate result

#### Scenario: Audio observation is stale or non-finite
- **WHEN** the latest observation predates the edit, carries another parameter generation, has no required nonzero signal, or reports non-finite output
- **THEN** the checkpoint remains pending or fails rather than crediting the parameter as audibly exercised

## ADDED Requirements

### Requirement: Alternating engine identity remains observable
The live final StateTree, text projection, coverage, and summary SHALL demonstrate that discovery-order Patches alternate between `instrument.soundfont.hidef` and `instrument.braids`, that at least one Patch of each type sounded, and that no Patch was layered, silently replaced, or routed to another capability.

#### Scenario: Live report completes
- **WHEN** the alternating scene reaches successful cleanup
- **THEN** final structured evidence contains both capability identities, exact alternating Patch assignments, nonzero mixed-engine render evidence, complete editable coverage, and zero active notes
