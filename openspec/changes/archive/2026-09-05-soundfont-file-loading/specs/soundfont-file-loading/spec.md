## Purpose

Allow users to select local SoundFont banks through the synth's shared controls, play their presets independently, and restore exact selections from saved sessions.

## ADDED Requirements

### Requirement: SoundFont uses the shared file page
SoundFont File SHALL open the existing in-app browser with SF2 filtering and the same navigation, confirm, and cancel controls as Sample. External files SHALL be validated and imported without overwriting existing assets. Cancellation and failed selection SHALL preserve the active assignment and exact return focus.

#### Scenario: Select a bank
- **WHEN** a user selects a supported local SF2
- **THEN** its first playable preset SHALL become the requested selection and become active only after complete audio graph activation

#### Scenario: Cancel or reject selection
- **WHEN** selection is cancelled, a bank is malformed/unavailable, or an import result is stale
- **THEN** the prior bank, preset, saved content, and usable audio graph SHALL remain intact with an explicit failure or cancellation state

### Requirement: Presets belong to the selected bank
Each Patch SHALL show the names and stable preset identities from its own selected bank. Selecting a preset SHALL prepare that exact preset without changing another Patch's bank or choices. Unsupported saved presets SHALL fail without fallback.

#### Scenario: Independent banks
- **WHEN** two Patches use banks with different preset catalogs
- **THEN** each menu SHALL match its own bank and both Patches SHALL render their selected presets independently

### Requirement: SoundFont loading remains off the audio callback
Bank reads, parsing, validation, and graph preparation SHALL happen off the audio callback and window tick. Runtime bank changes SHALL activate at a block boundary and retire old ownership off-thread. The existing test MIDI SHALL make a loaded melodic preset audible when routed to its channel.

#### Scenario: Play and replace
- **WHEN** a valid bank and preset replace a playing SoundFont
- **THEN** prepared audio SHALL produce finite nonzero output and callback allocation/destruction checks SHALL remain clear

### Requirement: Saved bank selections restore exactly
Saved sessions SHALL retain stable library-relative imported references and exact preset identities. Opening SHALL resolve each bank's catalog and prepare the complete candidate before replacing the current session. Existing bundled-bank sessions SHALL remain readable.

#### Scenario: Reopen without the original external file
- **WHEN** a session using an imported bank is saved, its original external file becomes unavailable, and the session is reopened
- **THEN** the imported bank and exact preset SHALL restore and render without relying on the original path
