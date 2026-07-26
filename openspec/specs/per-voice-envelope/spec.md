# per-voice-envelope Specification

## Purpose
TBD - created by archiving change phase-2-polymorphic-envelope-proof. Update Purpose after archive.
## Requirements
### Requirement: Canonical configurable Patch ADSR
Every Patch SHALL own one canonical Attack, Decay, Sustain, and Release value with stable semantic identities, finite typed bounds, defaults, and adjustment steps. The values SHALL be serialized, text-projected, reducible through the normal semantic adjustment path, and copied into the matching fixed real-time Patch projection without becoming capability-specific fields.

#### Scenario: Envelope is inspected and edited
- **WHEN** the selected Patch envelope is projected and one ADSR value is adjusted
- **THEN** canonical state, selected text, StateTree, and the graph-compatible real-time snapshot contain the same accepted value and every unrelated Patch and capability value remains unchanged

#### Scenario: Envelope value is invalid
- **WHEN** an ADSR adjustment or candidate value is non-finite or outside its declared bound
- **THEN** the reducer rejects it transactionally and publishes no new state, parameter generation, command, or fallback value

### Requirement: Independent per-note envelope application
Every admitted production engine SHALL apply the Patch ADSR independently inside each note voice. Attack, Decay, and Sustain SHALL be latched for a note when it starts, Release SHALL be latched when that note is released, zero-time stages SHALL complete safely, and a Patch-level post-stem gain envelope SHALL NOT satisfy this requirement.

#### Scenario: Notes overlap during release
- **WHEN** two notes on the same Patch overlap and the first receives note-off while the second remains held
- **THEN** only the first voice enters its release stage while the second continues its own envelope and remains audible

#### Scenario: Envelope differs between notes
- **WHEN** an ADSR value changes between two otherwise identical note lifecycles
- **THEN** the later voice uses the accepted value and the production render produces a measured finite amplitude/time difference

### Requirement: SoundFont and Braids envelope conformance
Both `instrument.soundfont.hidef` and `instrument.braids` SHALL implement the canonical ADSR according to their distinct voice policies. SoundFont SHALL apply it to the backend-managed native voices inside one synthesizer instance per SoundFont Patch; Braids SHALL own one envelope per oscillator in each Patch's sixteen-voice bank. Neither engine SHALL expose ADSR until all four fields have an audible production-path proof, and neither a post-stem SoundFont envelope nor a synthesizer-per-note split SHALL conform.

#### Scenario: SoundFont overlapping notes render
- **WHEN** two SoundFont notes with different release timing are rendered through one Patch
- **THEN** the one Patch-local SoundFont synthesizer's native voices exhibit independent envelopes before contributing to the Patch stem

#### Scenario: Braids overlapping notes render
- **WHEN** two Braids notes with different release timing are rendered through one Patch
- **THEN** their distinct oscillator slots exhibit independent envelopes before contributing to the Patch stem

### Requirement: Bounded envelope lifecycle
Envelope dispatch, sample advancement, voice completion, capability-owned stealing, and all-notes-off SHALL use fixed preallocated state and work bounded by the prepared engine policy: sixteen voices for each Braids Patch and the finite engine-managed safety ceiling for each SoundFont Patch. They SHALL perform no allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwind, or destruction in the callback.

#### Scenario: Envelope extremes are rendered
- **WHEN** minimum and maximum ADSR values, the declared worst-case Braids banks, the SoundFont prepared polyphony ceiling, capability-owned stealing, and all-notes-off are exercised
- **THEN** every sample and stage remains finite, each Patch's work stays within its own prepared bound, cleanup reaches silence, and callback allocation/destruction counts remain zero

