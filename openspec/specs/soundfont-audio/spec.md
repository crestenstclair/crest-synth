# SoundFont Audio

## Purpose

Specify the single shared SoundFont source, instrument-aware Patch configuration, and independent bounded stereo stems used by the audio engine.

## Requirements

### Requirement: Single fixed SoundFont source
The application SHALL load `./sf2/HiDef.sf2` into exactly one shared SoundFont engine before audio rendering and SHALL NOT expose a per-Patch engine, alternate synthesis engine, layering engine, or engine-type selection.

#### Scenario: Valid SoundFont starts the engine
- **WHEN** the application starts with a valid `./sf2/HiDef.sf2`
- **THEN** it loads the bank once before rendering and reports exactly one SoundFont engine instance

#### Scenario: Missing or invalid SoundFont stops startup
- **WHEN** `./sf2/HiDef.sf2` is missing or invalid
- **THEN** startup fails with a clear SoundFont error before audio rendering begins

### Requirement: Instrument-aware Patch configuration
Each playable Patch SHALL be configured from its MIDI bank, program, or percussion identity and SHALL retain a stable Patch identity independent of other Patches.

#### Scenario: Multiple instrument parts are configured
- **WHEN** notes for multiple discovered instrument parts are prepared
- **THEN** each part is routed to the Patch configured for its own bank, program, or percussion identity

### Requirement: Independent bounded stereo output
The SoundFont path SHALL expose a distinct stereo stem for each active Patch and every rendered sample SHALL be finite and bounded to the valid output range.

#### Scenario: Multiple Patches produce sound
- **WHEN** notes belonging to multiple configured Patches are rendered
- **THEN** each target Patch produces a distinct nonzero stereo stem whose samples are finite and bounded
