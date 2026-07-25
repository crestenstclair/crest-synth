## MODIFIED Requirements

### Requirement: Single fixed SoundFont source
The application SHALL parse `./sf2/HiDef.sf2` exactly once before audio rendering, share that immutable parsed bank across independently prepared per-Patch SoundFont instruments, and register HiDef as the only installed production instrument capability and preparer in this increment. It SHALL NOT expose an alternate production synthesis engine, layering engine, engine-type selection, or fallback, while Patch state, preparation, and the running rack remain capability-polymorphic.

#### Scenario: Valid SoundFont prepares the rack
- **WHEN** the application starts with a valid `./sf2/HiDef.sf2`
- **THEN** it registers exactly `instrument.soundfont.hidef`, parses the bank once outside the callback, and prepares one private SoundFont instrument for every accepted Patch while sharing the parsed bank

#### Scenario: Missing or invalid SoundFont stops startup
- **WHEN** `./sf2/HiDef.sf2` is missing or invalid
- **THEN** startup fails with a clear SoundFont preparation error before graph publication or audio rendering begins and does not substitute another asset, preparer, or renderer

### Requirement: Instrument-aware Patch configuration
Each playable SoundFont Patch SHALL be prepared from a schema-valid generic `instrument.soundfont.hidef` config whose bank, program, percussion, and fixed asset values exactly represent its MIDI instrument identity, and SHALL retain private voice/render state and a stable Patch identity independent of other Patches while sharing only immutable parsed bank data.

#### Scenario: Multiple instrument parts are prepared
- **WHEN** notes for multiple discovered instrument parts are prepared
- **THEN** each part has one independent prepared instrument for the Patch whose generic config represents its own bank, program, percussion, and fixed SoundFont asset identity

#### Scenario: Non-SoundFont config reaches the HiDef preparer
- **WHEN** the HiDef preparation boundary is asked to prepare a Patch with another capability identity or an invalid SoundFont config
- **THEN** preparation fails with a typed error and no preset, asset, config, capability, instrument, or preparer is substituted

