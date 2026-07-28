## MODIFIED Requirements

### Requirement: Descriptor-owned instrument schema
Each installed instrument capability SHALL expose one immutable ordered descriptor containing its label, semantic accent, sections, parameter specifications, asset requirements, typed voice policy, and supported MIDI message kinds. `FixedPerPatch` SHALL declare an exact capacity newly owned by every prepared Patch; `EngineManaged` SHALL leave note allocation to one Patch-local engine instance while its preparer retains a finite measured real-time safety ceiling. Voice policy SHALL describe one Patch and SHALL NOT encode an engine-global voice budget or capability-specific Patch-count limit. Each parameter specification SHALL declare its stable identity, label, value kind, default, Scalar or Structural update class, applicable bounds or choices, adjustment steps, formatting metadata, and declarative dependencies. A descriptor SHALL contain at most the declared fixed real-time Scalar-parameter capacity.

#### Scenario: Current SoundFont descriptor is inspected
- **WHEN** the current application registry is constructed
- **THEN** it contains `instrument.soundfont.hidef` with ordered Structural bank, program, percussion, and `./sf2/HiDef.sf2` specifications plus `instrument.braids` with ordered Scalar Model, Timbre, and Color specifications; SoundFont reports `EngineManaged`, Braids reports `FixedPerPatch(16)`, and both report their exact supported MIDI kinds

#### Scenario: Descriptor metadata is invalid
- **WHEN** a descriptor repeats an identity, declares an invalid default or range, references an undeclared dependency, reports a zero `FixedPerPatch` capacity or malformed voice policy, repeats a supported MIDI kind, or exceeds the fixed Scalar capacity
- **THEN** registry construction fails with a typed error before any Patch or renderer is installed

### Requirement: Immutable validated capability registry
The accepted application state SHALL contain one immutable ordered registry of installed capability descriptors, and every installed Patch config SHALL resolve to exactly one descriptor and satisfy its declared parameters, values, assets, kinds, bounds, choices, update classes, and dependencies.

#### Scenario: Valid fixture Patches are installed
- **WHEN** alternating SoundFont and Braids fixture configs match their installed descriptors
- **THEN** installation succeeds atomically, the registry remains unchanged, and every Patch retains its validated generic config and capability identity

#### Scenario: Patch config is invalid
- **WHEN** a config uses an unknown capability or contains a missing, duplicate, undeclared, wrong-kind, dependency-invalid, or out-of-range assignment or asset reference
- **THEN** Patch installation fails atomically with a typed error and no descriptor, value, asset, preset, capability, or engine is substituted

### Requirement: Phase 2 increment boundary
This increment SHALL install HiDef SoundFont and Braids as the two production instrument capabilities and runtime preparers, SHALL alternate them in the fixed fixture scene, and SHALL permit descriptor-classified Scalar capability values plus common Patch ADSR to be edited through the generic reducer and latest snapshot. Structural capability values SHALL remain preparation-only. The increment SHALL NOT add engine selection, a PATCH page, SoundFont preset editing, modulation, layering, per-Patch effects, plugin hosting, or arbitrary graph edits.

#### Scenario: Application starts after the prepared-rack refactor
- **WHEN** the normal, smoke, headless-demo, or live-demo path is launched
- **THEN** the accepted registry and Patch projection contain both installed capabilities, the prepared rack contains exact alternating implementations, and no unavailable capability or deferred UI feature is presented

#### Scenario: Runtime polymorphism is verified
- **WHEN** simultaneous production SoundFont and Braids Patches receive targeted MIDI, scalar edits, and rendering
- **THEN** both share the same Patch, reducer, projection, preparation, rack, stem, mixer, and observation contracts without an engine-specific branch in those owners

## ADDED Requirements

### Requirement: Generic Scalar capability adjustment
The reducer SHALL resolve editable capability values from the selected Patch's installed descriptor and SHALL permit only parameters classified Scalar to change without graph preparation. It SHALL create and validate one canonical descriptor-ordered candidate config, commit it transactionally, and SHALL NOT match SoundFont or Braids identities to decide the parameter shape.

#### Scenario: Braids choice or continuous value is adjusted
- **WHEN** Model, Timbre, or Color is selected and receives a valid semantic adjustment
- **THEN** exactly that assignment changes to the descriptor-derived choice or numeric value and the new accepted generation is projected to audio

#### Scenario: Structural value is selected for live adjustment
- **WHEN** a SoundFont bank, program, percussion, or asset value is treated as a live Scalar control
- **THEN** the operation is unavailable or rejected without changing the config, preparing a graph, or selecting a fallback
