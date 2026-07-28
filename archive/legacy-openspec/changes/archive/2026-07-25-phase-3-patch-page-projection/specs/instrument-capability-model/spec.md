## MODIFIED Requirements

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
