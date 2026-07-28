## MODIFIED Requirements

### Requirement: Independent per-Patch mixing
The mixer SHALL retain every Patch as a distinct stereo stem through its Patch-local effects and trim, route that stem to exactly one validated mixer track, and sum all stems sharing a destination before applying that track's Level, Pan, Mute, Solo, Reverb Send, and Delay Send. A Patch output edit SHALL affect only that Patch's contribution, while a track edit SHALL affect the combined contribution of every Patch currently routed to that track and no other track.

#### Scenario: Non-first Patch output changes
- **WHEN** a trim or output-track value changes on a non-first Patch
- **THEN** only that Patch's contribution changes or moves, every unrelated Patch stem remains sample-identical, and no mixer-track parameter changes

#### Scenario: Non-first Patch parameter changes
- **WHEN** Level, Pan, Reverb Send, or Delay Send is adjusted for the destination track carrying a non-first Patch
- **THEN** the track-owned operation changes the combined post-accumulation contribution on that track, every untargeted track remains sample-identical, and no Patch-owned output or instrument value changes

#### Scenario: Shared-track parameter changes
- **WHEN** two or more Patches share a track and its Level, Pan, Mute, Solo, Reverb Send, or Delay Send changes
- **THEN** the declared post-accumulation operation applies to their combined track signal and every untargeted track remains sample-identical

### Requirement: Exactly two shared effects
The final signal path SHALL contain exactly one shared reverb and one shared delay, fed only by the corresponding post-fader, post-mute/solo-gate mixer-track sends, and SHALL contain no channel inserts or additional shared effect processors.

#### Scenario: Several active Patches feed effects
- **WHEN** two or more audible tracks have nonzero reverb and delay sends
- **THEN** their post-gate send signals are accumulated into the one shared reverb and one shared delay before the final stereo output

#### Scenario: All effect sends are zero
- **WHEN** every mixer track supplies zero reverb and delay input
- **THEN** dry audio does not create new wet excitation in either shared effect

#### Scenario: Muted or excluded track has nonzero sends
- **WHEN** a track with nonzero send values is muted or excluded by an active Solo on another track
- **THEN** that track supplies zero input to both shared effects

### Requirement: MixEngine consumes post-effect Patch stems
`MixEngine` SHALL receive every Patch stem only after the aligned prepared Patch-local effect stage, apply Patch trim, accumulate the result into the Patch's validated destination among exactly sixteen preallocated tracks, and then apply each track's Level, Pan, pre-gate meter, Mute/Solo gate, Reverb Send, and Delay Send before global reverb, global delay, and master processing. MixEngine SHALL own no Patch-local Chorus or other insert, and its only shared wet processors SHALL remain one reverb and one delay.

#### Scenario: Chorus Patch enters the mixer
- **WHEN** a Patch renders through its configured Chorus
- **THEN** Patch trim and destination accumulation operate on the post-Chorus stem, track controls operate after accumulation, and pre/post-effect observations remain outside mixer ownership

#### Scenario: Multiple post-effect Patches share a track
- **WHEN** at least two post-effect Patch stems route to one track with nonzero track sends
- **THEN** the stems remain separately observable before routing, sum once into that track, and feed the single shared processors only after track Level, Pan, and the audibility gate

#### Scenario: Global effects are exercised
- **WHEN** at least two audible tracks have nonzero sends and global reverb or delay parameters are edited
- **THEN** the shared processors receive the declared post-gate track-send inputs, remain single mixer-owned instances, and no Patch-local effect is mistaken for or folded into a global wet path

#### Scenario: Patch effect is unavailable
- **WHEN** a configured effect cannot be prepared or its runtime layout mismatches
- **THEN** the system surfaces the typed effect failure rather than letting MixEngine accept an undeclared dry fallback as a valid configured stem
