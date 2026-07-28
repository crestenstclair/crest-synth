# per-voice-envelope Specification

## Purpose
TBD - created by archiving change phase-2-polymorphic-envelope-proof. Update Purpose after archive.
## Requirements
### Requirement: Canonical configurable Patch ADSR
Every Patch SHALL own one canonical Attack, Decay, Sustain, and Release value with stable semantic identities, finite typed bounds, defaults, and adjustment steps. The same values SHALL be serialized, projected and editable through both MIXER and PATCH semantic adjustment paths, and copied into the matching fixed real-time Patch projection without becoming capability-specific or UI-owned fields. PATCH control identities, order, labels, bounds, units, and fine/coarse steps SHALL derive from the canonical envelope descriptor rather than a second ADSR list. PATCH editing MUST reuse the already-conforming per-note envelope implementations in SoundFont and Braids; no PATCH adapter, projection, shared post-stem processor, or capability-specific control SHALL apply or cache another envelope. Acceptance SHALL connect all four PATCH controls through the production reducer and fixed snapshot to a measured target-only envelope consequence in both real engines.

#### Scenario: Envelope is inspected and edited
- **WHEN** the selected Patch envelope is projected in MIXER and one ADSR value is adjusted
- **THEN** canonical state, selected text, StateTree, and the graph-compatible real-time snapshot contain the same accepted value and every unrelated Patch and capability value remains unchanged

#### Scenario: Envelope is inspected and edited in PATCH
- **WHEN** the focused Patch's ADSR row is selected in PATCH and receives a fine or coarse adjustment
- **THEN** the same canonical envelope field, text, PatchPageProjection, StateTree, and graph-compatible real-time snapshot contain the accepted value and no second envelope, audio command, or graph change exists

#### Scenario: MIXER and PATCH resolve the same field
- **WHEN** the same Patch envelope parameter is edited once through MIXER and once through PATCH from identical state
- **THEN** both paths use the same bounds, step, numeric result, canonical field, fixed snapshot slot, and engine behavior

#### Scenario: PATCH edit reaches overlapping SoundFont voices
- **WHEN** a SoundFont Patch ADSR value is changed through PATCH and controlled overlapping notes are rendered
- **THEN** the one Patch-local synthesizer consumes the accepted value through its independent native voices, the declared amplitude/time measurement changes, and untargeted Patch stems remain exact

#### Scenario: PATCH edit reaches overlapping Braids voices
- **WHEN** a Braids Patch ADSR value is changed through PATCH and controlled overlapping notes are rendered
- **THEN** the targeted Patch's oscillator envelopes consume the accepted value independently, the declared amplitude/time measurement changes, and every untargeted bank and stem remains exact

#### Scenario: UI-only or post-stem implementation is attempted
- **WHEN** the projected PATCH value changes without the canonical fixed snapshot and per-voice renderer consequence, or one gain envelope is applied after a mixed Patch stem
- **THEN** the structured envelope proof fails and the increment cannot report acceptance

#### Scenario: Envelope value is invalid
- **WHEN** an ADSR adjustment or candidate value is non-finite or outside its declared bound
- **THEN** the reducer rejects it transactionally and publishes no new state, parameter generation, command, structural effect, or fallback value

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

