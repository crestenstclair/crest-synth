## ADDED Requirements

### Requirement: Descriptor-default engine-selection boundary
Selecting another installed engine SHALL construct exactly one candidate from descriptor defaults and required default assets, pass it through the identity-matched provider, and revalidate it against the immutable registry. The generic path SHALL preserve the two-engine fixture and SHALL NOT branch on engine identity, cache inactive configs, copy incompatible values, expose other Structural edits, or select a fallback.

#### Scenario: SoundFont Patch requests Braids
- **WHEN** a Patch using `instrument.soundfont.hidef` selects `instrument.braids`
- **THEN** the candidate contains exactly the default Model, Timbre, and Color assignments in descriptor order and no SoundFont value or asset

#### Scenario: Braids Patch requests SoundFont
- **WHEN** a Patch using `instrument.braids` selects `instrument.soundfont.hidef`
- **THEN** the candidate contains exactly default bank, program, and percussion plus the required `./sf2/HiDef.sf2` reference in descriptor order and no Braids assignment

#### Scenario: Selected capability cannot produce its declared default
- **WHEN** the provider is missing or mismatched, a default or asset is invalid, or its result differs from the requested installed descriptor
- **THEN** preparation fails before canonical Patch mutation or graph publication and no config, capability, asset, preset, or engine is substituted

#### Scenario: Player returns to a previously used capability
- **WHEN** the Patch selects a capability it used earlier in the session
- **THEN** the candidate is reconstructed from current descriptor defaults rather than an inactive-engine settings cache

#### Scenario: Application starts before any selection
- **WHEN** the normal, smoke, headless-demo, or live-demo path is launched
- **THEN** the registry, projections, and prepared rack retain the exact alternating HiDef SoundFont and Braids fixture and present no unavailable capability

#### Scenario: Runtime selection completes generically
- **WHEN** a focused Patch changes from one installed capability to the other
- **THEN** the same Patch, registry, provider/preparer matching, reducer, serialization, projection, rack, stem, mixer, and observation contracts represent the new config without an engine-specific owner or fallback

#### Scenario: Deferred Phase 3 controls remain unavailable
- **WHEN** PATCH receives an ADSR or non-engine Structural edit in this increment
- **THEN** the edit is rejected without adding preset discovery, modulation, layering, per-Patch effects, plugin hosting, arbitrary graph editing, or another engine

## REMOVED Requirements

### Requirement: Phase 2 increment boundary
The completed Phase 2 foundation SHALL retain HiDef SoundFont and Braids as the two production instrument capabilities and runtime preparers, SHALL alternate them in the fixed fixture scene, and SHALL permit descriptor-classified Scalar capability values plus common Patch ADSR to be edited through the existing generic MIXER reducer and latest snapshot. Phase 3 MAY project both installed descriptors, their generic configs, Structural fields, and complete registry choices through a read-only PATCH page. Structural capability values SHALL remain preparation-only. This increment SHALL NOT add runtime engine selection, PATCH editing, SoundFont preset discovery or editing, modulation, layering, per-Patch effects, plugin hosting, or arbitrary graph edits.

#### Scenario: Application starts after the prepared-rack refactor
- **WHEN** the normal, smoke, headless-demo, or live-demo path is launched
- **THEN** the accepted registry and projections contain both installed capabilities, the prepared rack contains exact alternating implementations, PATCH can display their real schemas without placeholders, and no unavailable capability or later structural workflow is presented

#### Scenario: Runtime polymorphism is verified
- **WHEN** SoundFont and Braids Patches are inspected in PATCH and receive targeted MIDI, Scalar edits in MIXER, and rendering
- **THEN** both share the same Patch, reducer, descriptor/config projection, preparation, rack, stem, mixer, and observation contracts without an engine-specific branch in those owners

#### Scenario: Engine choice is invoked during the read-only increment
- **WHEN** an installed engine choice is treated as an editable PATCH control
- **THEN** the action is unavailable and no config, preparer, prepared graph, active instrument, or fallback changes

**Reason**: Phase 2 and the read-only Phase 3 projection are complete; this increment admits one constrained engine-selection workflow.

**Migration**: Preserve the two-engine and no-fallback contracts, replace only the runtime-selection prohibition with descriptor-default candidate construction, and keep all other Structural editing deferred.
