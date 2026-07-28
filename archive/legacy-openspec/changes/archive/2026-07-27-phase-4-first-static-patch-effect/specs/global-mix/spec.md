## ADDED Requirements

### Requirement: MixEngine consumes post-effect Patch stems
`MixEngine` SHALL receive each Patch stem only after the aligned prepared post-effect stage and SHALL then apply canonical Patch gain, pan, sends, mute/solo routing, global reverb, global delay, and master processing in the existing order. MixEngine SHALL own no Patch-local Chorus or other insert, and its only shared wet processors SHALL remain one reverb and one delay.

#### Scenario: Chorus Patch enters the mixer
- **WHEN** the first Patch renders through its configured Chorus
- **THEN** gain, pan, reverb send, delay send, mute/solo, and track accumulation operate on the post-Chorus stem while pre/post-effect observations remain outside mixer ownership

#### Scenario: Global effects are exercised
- **WHEN** at least two Patches have nonzero sends and global reverb/delay parameters are edited
- **THEN** the shared processors receive the declared post-effect send inputs, remain single mixer-owned instances, and no Patch-local effect is mistaken for or folded into a global wet path

#### Scenario: Patch effect is unavailable
- **WHEN** a configured effect cannot be prepared or its runtime layout mismatches
- **THEN** the system surfaces the typed effect failure rather than letting MixEngine accept an undeclared dry fallback as a valid configured stem

