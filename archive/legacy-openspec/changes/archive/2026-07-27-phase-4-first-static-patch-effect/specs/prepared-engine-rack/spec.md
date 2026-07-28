## ADDED Requirements

### Requirement: Complete graphs contain aligned Patch-local prepared effect ownership
Every `PreparedGraph` SHALL contain both the existing `PreparedEngineRack` and a fixed-capacity `PreparedPostEffectRack` aligned by ordered PatchId, plus a compatible initial snapshot containing the same effect slot/layout identities. `PreparedPostEffectRackBuilder` SHALL resolve every accepted config through exactly one effect descriptor and identity-matched preparer outside the callback and SHALL fail atomically for any registration, identity, rate, config, capacity, allocation, or returned-slot mismatch.

#### Scenario: Mixed graph is built
- **WHEN** one configured Patch and one unconfigured Patch are prepared
- **THEN** the engine rack contains both instruments, the effect rack contains exactly the configured Patch/slot processor, both racks and the snapshot share Patch order, and only the complete graph may be published

#### Scenario: Effect preparation fails
- **WHEN** a preparer is missing, duplicated, identity-mismatched, returns the wrong Patch/slot, rejects the rate, or cannot allocate its bounded state
- **THEN** graph construction returns one typed error, destroys all partial ownership off callback, and publishes neither a partial rack nor a dry/bypass substitute

**Contract facet — Patch-local independently stateful instances.**
Every configured Patch slot SHALL own one distinct `PreparedPostEffect` instance and its private delay/LFO/tail state. The rack SHALL target only the exact aligned Patch stem and SHALL never share, pool, broadcast, or cross-route native effect state.

#### Scenario: Two-instance focused graph renders
- **WHEN** a focused test prepares Chorus on two Patches, excites them differently, and renders both over time
- **THEN** each instance's output and tail follow only its own input and scalar slot while the other instance's state and untargeted stems remain exact

#### Scenario: Slot layout disagrees at render time
- **WHEN** a fixed scalar slot id, count, or Patch identity does not match the prepared rack layout
- **THEN** processing reports the bounded routing/layout failure and does not process another Patch, broadcast values, or silently bypass the configured slot
