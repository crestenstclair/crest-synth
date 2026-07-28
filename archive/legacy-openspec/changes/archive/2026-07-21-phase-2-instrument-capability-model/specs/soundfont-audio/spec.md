## MODIFIED Requirements

### Requirement: Single fixed SoundFont source
The application SHALL load `./sf2/HiDef.sf2` into exactly one shared SoundFont engine before audio rendering and SHALL register it as the only installed instrument capability and renderer in this increment. It SHALL NOT expose a per-Patch engine, alternate running synthesis engine, layering engine, engine-type selection, or fallback, while Patch and instrument config remain capability-polymorphic.

#### Scenario: Valid SoundFont starts the engine
- **WHEN** the application starts with a valid `./sf2/HiDef.sf2`
- **THEN** it registers exactly `instrument.soundfont.hidef`, loads the bank once before rendering, and reports exactly one SoundFont engine instance

#### Scenario: Missing or invalid SoundFont stops startup
- **WHEN** `./sf2/HiDef.sf2` is missing or invalid
- **THEN** startup fails with a clear SoundFont error before audio rendering begins and does not substitute another asset or renderer

### Requirement: Instrument-aware Patch configuration
Each playable SoundFont Patch SHALL be configured from a schema-valid generic `instrument.soundfont.hidef` config whose bank, program, percussion, and fixed asset values exactly represent its MIDI instrument identity, and SHALL retain a stable Patch identity independent of other Patches.

#### Scenario: Multiple instrument parts are configured
- **WHEN** notes for multiple discovered instrument parts are prepared
- **THEN** each part is routed to the Patch whose generic config represents its own bank, program, percussion, and fixed SoundFont asset identity

#### Scenario: Non-SoundFont config reaches the current renderer
- **WHEN** the SoundFont renderer is asked to configure a Patch with another capability identity or an invalid SoundFont config
- **THEN** configuration fails with a typed error and no preset, asset, config, or renderer is substituted

