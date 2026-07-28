## MODIFIED Requirements

### Requirement: Exact current editable-surface coverage
Before dispatching its first scene action, the live demo SHALL derive and freeze one ordered unique expected set containing the focused first Patch's two Patch-output fields, every editable target from that Patch's canonical resolver, every enabled ScalarEdit parameter from its configured effects, every one of the six track parameters for all sixteen mixer tracks, and every editable global parameter. Patch-owned identifiers SHALL be navigated and edited through PATCH Main or PATCH Utility, while track-owned identifiers SHALL be navigated and edited through MIXER Main or Mixer Inspector; no identifier SHALL receive duplicate credit. Every expected parameter instance SHALL receive at least one accepted value change, SHALL remain visibly projected for at least 500 milliseconds, and SHALL receive a generation-correlated audible observation.

#### Scenario: Focused Patch controls are exercised through PATCH
- **WHEN** the scene reaches the focused first Patch's frozen output, envelope, editable instrument, or effect identifiers
- **THEN** semantic PATCH navigation visits the owning Main or Utility control in canonical order, each identifier receives one accepted bounded adjustment and visible/audio checkpoint, exactly one control is focused, and no MIXER exercise is credited

#### Scenario: Focused Patch envelope is exercised through PATCH
- **WHEN** the scene reaches the focused first Patch's four frozen envelope identifiers
- **THEN** semantic PATCH navigation visits Attack, Decay, Sustain, and Release in canonical order, each identifier receives one accepted bounded adjustment and visible/audio checkpoint, exactly one control is focused, and no MIXER exercise is credited

#### Scenario: Sixteen-track surface is exercised through MIXER
- **WHEN** the scene reaches its frozen mixer-track identifiers
- **THEN** Level, Pan, Mute, Solo, Reverb Send, and Delay Send are each exercised once for T00 through T0F through their canonical Main or Inspector path, including empty tracks, independently of routed Patch count

#### Scenario: Shared-track and reroute checkpoints run
- **WHEN** two sounding Patches are routed to one track and one Patch is subsequently rerouted
- **THEN** the live observations prove the shared track controls both contributions before rerouting, the next compatible generation moves only the selected Patch, all sixteen meters retain track identity, and no graph replacement is credited

#### Scenario: Current parameter surface is exercised
- **WHEN** the scene reaches its coverage-comparison step
- **THEN** exercised identifiers exactly equal the frozen resolver- and descriptor-derived identifiers, include both output fields, all four ADSR instances, and configured Chorus Amount and Depth once for the focused first Patch, include every track-parameter pair across sixteen tracks, and leave both missing and unexpected sets empty

#### Scenario: Parameter is omitted or only visited
- **WHEN** an expected Patch output, envelope, engine, effect, track, or global parameter is missing, duplicated, selected without changing, not projected, lacks its dwell, or lacks a qualifying audio observation
- **THEN** the live report is incomplete and identifies the coverage or checkpoint failure

#### Scenario: PATCH focus or value disagrees
- **WHEN** a PATCH or MIXER checkpoint marks another control, changes another Patch, output, track, or field, emits an undeclared structural effect, or reports a value different from canonical state and the fixed snapshot
- **THEN** the checkpoint earns no coverage and the live report cannot complete
