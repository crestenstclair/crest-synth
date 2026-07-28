## ADDED Requirements

### Requirement: Structural replacement preserves Patch effects
Every engine- or instrument-structural-choice request, immutable candidate, canonical commit, compatible initial snapshot, and prepared complete graph SHALL preserve the targeted and untargeted Patches' ordered effect slot ids, capability ids, configs, parameter identities, and scalar layouts exactly. Only the requested instrument config assignment/layout may differ. A complete activation MAY reset effect tails but SHALL NOT remove, bypass, substitute, reorder, or re-default an effect.

#### Scenario: SoundFont preset changes with Chorus configured
- **WHEN** the first Patch selects an adjacent authored SoundFont preset
- **THEN** the candidate and activated graph contain the exact same Chorus slot/config/layout and the newer prepared effect instance processes the replacement instrument stem after activation

#### Scenario: Engine changes in both directions
- **WHEN** the first Patch selects SoundFont to Braids and later descriptor-default SoundFont
- **THEN** each complete candidate preserves the exact Chorus config while only the instrument config/layout changes and the final Patch remains Ready with Chorus configured

#### Scenario: Effect scalar changes while replacement is pending
- **WHEN** Amount or Depth is accepted during Preparing or Activating
- **THEN** the audible source or target-compatible snapshot receives the committed value according to graph revision, candidate initial parameters are refreshed before publication as applicable, and activation does not revert the edit

#### Scenario: Candidate changes effect state structurally
- **WHEN** candidate validation sees a different effect slot, capability, config assignment, parameter order, or effect scalar layout
- **THEN** it rejects and destroys the candidate off callback, leaves the active graph/config exact, and selects no fallback

