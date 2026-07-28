# SoundFont Audio

## Purpose

Specify the single shared SoundFont source, instrument-aware Patch configuration, and independent bounded stereo stems used by the audio engine.
## Requirements
### Requirement: Single fixed SoundFont source
The application SHALL parse `./sf2/HiDef.sf2` exactly once before audio rendering, share that immutable parsed bank across independently prepared per-Patch SoundFont synthesizers, and register HiDef beside Braids as one of exactly two installed production instrument capabilities and preparers in this increment. Each SoundFont Patch SHALL own exactly one synthesizer instance with engine-managed polyphony. It SHALL NOT expose layering, engine selection, alternate SoundFont assets, or fallback, while Patch state, preparation, and the running rack remain capability-polymorphic.

#### Scenario: Valid SoundFont prepares the rack
- **WHEN** the application starts with a valid `./sf2/HiDef.sf2`
- **THEN** it registers `instrument.soundfont.hidef` and `instrument.braids`, parses the bank once outside the callback, and prepares exactly one private engine-managed synthesizer for each accepted SoundFont Patch while sharing the parsed immutable bank

#### Scenario: Missing or invalid SoundFont stops startup
- **WHEN** `./sf2/HiDef.sf2` is missing or invalid while any accepted Patch selects SoundFont
- **THEN** startup fails with a clear SoundFont preparation error before graph publication or audio rendering begins and does not substitute Braids, another asset, preparer, or renderer

### Requirement: Instrument-aware Patch configuration
Each playable SoundFont Patch SHALL be prepared from a schema-valid generic `instrument.soundfont.hidef` config whose bank, program, percussion, and fixed asset values exactly represent its MIDI instrument identity, and SHALL retain private voice/render state and a stable Patch identity independent of other Patches while sharing only immutable parsed bank data.

#### Scenario: Multiple instrument parts are prepared
- **WHEN** notes for multiple discovered instrument parts are prepared
- **THEN** each part has one independent prepared instrument for the Patch whose generic config represents its own bank, program, percussion, and fixed SoundFont asset identity

#### Scenario: Non-SoundFont config reaches the HiDef preparer
- **WHEN** the HiDef preparation boundary is asked to prepare a Patch with another capability identity or an invalid SoundFont config
- **THEN** preparation fails with a typed error and no preset, asset, config, capability, instrument, or preparer is substituted

### Requirement: Independent bounded stereo output
The SoundFont path SHALL expose a distinct stereo stem for each active Patch and every rendered sample SHALL be finite and bounded to the valid output range.

#### Scenario: Multiple Patches produce sound
- **WHEN** notes belonging to multiple configured Patches are rendered
- **THEN** each target Patch produces a distinct nonzero stereo stem whose samples are finite and bounded

### Requirement: Engine-managed SoundFont polyphony and envelope isolation
Each prepared SoundFont Patch SHALL own exactly one synthesizer instance and SHALL delegate note allocation to that engine under a finite prepared real-time safety ceiling. SoundFont voice capacity remains engine-managed and is not a numeric product acceptance criterion. Crest SHALL NOT instantiate one synthesizer per note or share mutable synthesizer state between Patches. The common Patch envelope SHALL reach independent native note voices through a conforming backend seam before their audio contributes to the Patch stem.

#### Scenario: Two SoundFont notes overlap
- **WHEN** two notes sound and only the first is released
- **THEN** the first follows its own Release while the second retains its own held envelope and both contribute independently to the Patch stem

#### Scenario: SoundFont backend lacks per-note ADSR control
- **WHEN** the selected SoundFont backend cannot audibly apply all four common ADSR values independently to overlapping native voices in its single Patch-local synthesizer
- **THEN** SoundFont envelope conformance fails and the adapter must be extended or replaced; it does not create sixteen synthesizers, apply a post-stem envelope, ignore the control, or claim acceptance
