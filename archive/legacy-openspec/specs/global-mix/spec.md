# Global Mix

## Purpose

Specify independent per-Patch mixing, the two shared effects, global mix controls, and the behavioral evidence required to prove wet-signal behavior faithfully.
## Requirements
### Requirement: Independent per-Patch mixing
The mixer SHALL keep every Patch as a separate stereo stem and SHALL apply that Patch's gain, pan, reverb send, and delay send without changing any other Patch's contribution.

#### Scenario: Non-first Patch parameter changes
- **WHEN** a gain, pan, reverb-send, or delay-send value changes on a non-first Patch
- **THEN** only that Patch's dry or send contribution changes and every untargeted Patch contribution remains sample-identical

### Requirement: Exactly two shared effects
The final signal path SHALL contain exactly one shared reverb and one shared delay, fed only by the corresponding per-Patch sends, and SHALL contain no channel inserts or additional effect processors.

#### Scenario: Several active Patches feed effects
- **WHEN** two or more Patches render with nonzero reverb and delay sends
- **THEN** their send signals are accumulated into the one shared reverb and one shared delay before the final stereo output

#### Scenario: All effect sends are zero
- **WHEN** every Patch supplies zero reverb and delay input
- **THEN** dry audio does not create new wet excitation in either shared effect

### Requirement: Complete global parameter control
The shared mix SHALL expose master gain, reverb room size, reverb damping, reverb return, delay time, delay feedback, and delay return, with each value bounded by its declared parameter range.

#### Scenario: Global effect parameter changes
- **WHEN** one bounded global parameter is changed and a new complete parameter generation is published
- **THEN** the next render reflects that exact value while all unrelated Patch and global values remain unchanged

### Requirement: Faithful effect comparison
Behavioral proof of a wet parameter SHALL establish nonzero routed sends, compare renders from identical effect state, measure the supplied effect inputs and resulting output, and restore all edited values and sends exactly to their captured baseline.

#### Scenario: Wet controls are observed
- **WHEN** reverb or delay controls are compared in the deterministic observation path
- **THEN** both comparisons use nonzero corresponding inputs and identical starting effect state, produce measured output differences, and leave the final parameters and sends equal to baseline

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
