## MODIFIED Requirements

### Requirement: Canonical configurable Patch ADSR
Every Patch SHALL own one canonical Attack, Decay, Sustain, and Release value with stable semantic identities, finite typed bounds, defaults, and adjustment steps. The same values SHALL be serialized, projected and editable through the PATCH semantic adjustment path, and copied into the matching fixed real-time Patch projection without becoming capability-specific, mixer-track, or UI-owned fields. PATCH control identities, order, labels, bounds, units, and fine/coarse steps SHALL derive from the canonical envelope descriptor rather than a second ADSR list. MIXER SHALL expose only track and global controls and SHALL NOT expose or edit Patch envelopes. PATCH editing MUST reuse the already-conforming per-note envelope implementations in SoundFont and Braids; no PATCH adapter, projection, shared post-stem processor, mixer stage, or capability-specific control SHALL apply or cache another envelope. Acceptance SHALL connect all four PATCH controls through the production reducer and fixed snapshot to a measured target-only envelope consequence in both real engines.

#### Scenario: Envelope is inspected and edited
- **WHEN** the selected Patch envelope is inspected and one ADSR value is adjusted through PATCH
- **THEN** canonical state, selected text, PatchPageProjection, StateTree, and the graph-compatible real-time snapshot contain the same accepted value, every unrelated Patch and capability value remains unchanged, and MIXER retains only track and global controls

#### Scenario: Envelope is inspected and edited in PATCH
- **WHEN** the focused Patch's ADSR row is selected in PATCH and receives a fine or coarse adjustment
- **THEN** the same canonical envelope field, selected text, PatchPageProjection, StateTree, and graph-compatible real-time snapshot contain the accepted value and no second envelope, audio command, graph change, or track mutation exists

#### Scenario: MIXER and PATCH resolve the same field
- **WHEN** verification asks PATCH and MIXER to resolve the same Patch envelope parameter from identical state
- **THEN** PATCH resolves the canonical bounded field and fixed snapshot slot while MIXER exposes no Patch-envelope target, duplicate path, or alternate engine behavior

#### Scenario: MIXER is inspected
- **WHEN** the player navigates every MIXER track and global control
- **THEN** no Patch envelope identity appears in its resolver, projection, focus path, or adjustment events

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
