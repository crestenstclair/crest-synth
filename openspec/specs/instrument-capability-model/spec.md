# Instrument Capability Model

## Purpose

Define stable, schema-driven instrument capabilities and generic Patch-owned configurations without coupling canonical state or projections to one sound engine.
## Requirements
### Requirement: Canonical instrument capability identities
Every installed instrument implementation and each of its parameters SHALL have stable namespaced identifiers that are independent of display labels, registry positions, filesystem paths, engine instances, and UI widgets.

#### Scenario: Capability metadata is serialized
- **WHEN** the installed instrument registry is projected or serialized
- **THEN** capability and parameter identities remain stable across runs and can be joined to Patch configs without positional or label matching

### Requirement: Descriptor-owned instrument schema
Each installed instrument capability SHALL expose one immutable ordered descriptor containing its label, semantic accent, sections, parameter specifications, asset requirements, typed voice policy, and supported MIDI message kinds. `FixedPerPatch` SHALL declare an exact capacity newly owned by every prepared Patch; `EngineManaged` SHALL leave note allocation to one Patch-local engine instance while its preparer retains a finite measured real-time safety ceiling. Voice policy SHALL describe one Patch and SHALL NOT encode an engine-global voice budget or capability-specific Patch-count limit. Each parameter specification SHALL declare its stable identity, label, value kind, default, Scalar or Structural update class, applicable bounds or choices, adjustment steps, formatting metadata, and declarative dependencies. A descriptor SHALL contain at most the declared fixed real-time Scalar-parameter capacity.

#### Scenario: Current SoundFont descriptor is inspected
- **WHEN** the current application registry is constructed
- **THEN** it contains `instrument.soundfont.hidef` with ordered Structural bank, program, percussion, and `./sf2/HiDef.sf2` specifications plus `instrument.braids` with ordered Scalar Model, Timbre, and Color specifications; SoundFont reports `EngineManaged`, Braids reports `FixedPerPatch(16)`, and both report their exact supported MIDI kinds

#### Scenario: Descriptor metadata is invalid
- **WHEN** a descriptor repeats an identity, declares an invalid default or range, references an undeclared dependency, reports a zero `FixedPerPatch` capacity or malformed voice policy, repeats a supported MIDI kind, or exceeds the fixed Scalar capacity
- **THEN** registry construction fails with a typed error before any Patch or renderer is installed

### Requirement: Generic Patch-owned instrument config
Each Patch SHALL own one generic instrument config containing a capability identity, ordered typed parameter assignments, and stable asset references. The Patch SHALL NOT contain capability-specific fields, an engine instance, a descriptor copy, decoded asset data, prepared renderer state, UI state, or a fallback configuration.

#### Scenario: SoundFont fixture Patch is created
- **WHEN** a discovered SoundFont instrument identity is converted for Patch installation
- **THEN** the resulting Patch owns a generic config for `instrument.soundfont.hidef` whose bank, program, percussion, and file values exactly represent that identity

#### Scenario: A second descriptor shape is considered
- **WHEN** a conforming capability declares a different ordered parameter set
- **THEN** the generic config contract can represent its declared values and assets without adding capability-specific fields to Patch

### Requirement: Immutable validated capability registry
The accepted application state SHALL contain one immutable ordered registry of installed capability descriptors, and every installed Patch config SHALL resolve to exactly one descriptor and satisfy its declared parameters, values, assets, kinds, bounds, choices, update classes, and dependencies.

#### Scenario: Valid fixture Patches are installed
- **WHEN** alternating SoundFont and Braids fixture configs match their installed descriptors
- **THEN** installation succeeds atomically, the registry remains unchanged, and every Patch retains its validated generic config and capability identity

#### Scenario: Patch config is invalid
- **WHEN** a config uses an unknown capability or contains a missing, duplicate, undeclared, wrong-kind, dependency-invalid, or out-of-range assignment or asset reference
- **THEN** Patch installation fails atomically with a typed error and no descriptor, value, asset, preset, capability, or engine is substituted

### Requirement: Capability provider boundary
Capability-specific descriptor and generic config construction SHALL be supplied through a control-side provider boundary that accepts only typed parameter assignments and asset references, performs no rendering, device access, audio-buffer access, UI behavior, or file loading, and returns typed errors without fallback. Runtime preparation SHALL use a separate control/worker-side boundary selected by the same stable capability identity; it MAY resolve and prepare assets but SHALL NOT own descriptor projection or canonical Patch state. Neither polymorphic boundary SHALL expose a SoundFont-specific construction method.

#### Scenario: Fixture identity is converted
- **WHEN** automatic MIDI initialization translates a valid SoundFont instrument identity into the stable assignments declared by the installed descriptor
- **THEN** the provider deterministically validates and returns a config matching its own descriptor without reading, decoding, or preparing the SoundFont file

#### Scenario: Provider, registry, and preparer agree
- **WHEN** the accepted SoundFont Patches are prepared for audio
- **THEN** the descriptor provider and the runtime preparer match `instrument.soundfont.hidef` by stable identity while remaining separate owners

#### Scenario: Provider, registry, or preparer disagrees
- **WHEN** the provider descriptor is missing or differs from the registry, or a Patch has zero or multiple capability-matched preparers
- **THEN** startup fails before graph publication and no alternate provider, preparer, capability, asset, or renderer is selected

### Requirement: Generic capability projection
Canonical serialized state and the current text projection SHALL include the installed descriptor registry plus every Patch capability identity, parameter assignment, and asset reference in descriptor order. Projection SHALL derive from descriptors and configs without capability-identity branches or duplicate engine-specific field lists.

#### Scenario: Maintainer inspects current state
- **WHEN** the current SoundFont fixture has been installed
- **THEN** StateTree and the text view show the exact registered descriptor and every Patch config value and asset from the same accepted generation

#### Scenario: Descriptor order changes
- **WHEN** the production descriptor order differs from a duplicated expected order
- **THEN** serialization, text projection, and schema-derived coverage follow the production descriptor and the duplicated expectation fails verification

### Requirement: Production capability ports are composed explicitly
The production composition root SHALL construct and inject every installed instrument capability provider and its separate preparer into the standalone application. Before fixture initialization or graph publication, the application SHALL build the immutable registry from the injected provider descriptors and SHALL require exactly one provider and exactly one preparer for every installed capability identity. Missing, duplicate, unknown, or mismatched registrations SHALL return a typed startup failure without selecting a fallback.

#### Scenario: Matching production registrations start
- **WHEN** the production constructor receives the declared providers and one identity-matched preparer for each provider
- **THEN** it accepts the composition and later graph preparation uses only those injected ports

#### Scenario: Provider or preparer registration is invalid
- **WHEN** a provider or preparer is missing, duplicated, unknown to the opposite collection, or identity-mismatched
- **THEN** construction fails with the corresponding typed error before a graph is prepared or published

#### Scenario: Concrete selection is replaced in a witness
- **WHEN** a deterministic witness supplies conforming provider and preparer fixtures through the production constructor
- **THEN** the standalone application uses those fixtures without constructing a built-in capability or preparer internally

### Requirement: Generic Scalar capability adjustment
The reducer SHALL resolve editable capability values from the selected Patch's installed descriptor and SHALL permit only parameters classified Scalar to change without graph preparation. It SHALL create and validate one canonical descriptor-ordered candidate config, commit it transactionally, and SHALL NOT match SoundFont or Braids identities to decide the parameter shape.

#### Scenario: Braids choice or continuous value is adjusted
- **WHEN** Model, Timbre, or Color is selected and receives a valid semantic adjustment
- **THEN** exactly that assignment changes to the descriptor-derived choice or numeric value and the new accepted generation is projected to audio

#### Scenario: Structural value is selected for live adjustment
- **WHEN** a SoundFont bank, program, percussion, or asset value is treated as a live Scalar control
- **THEN** the operation is unavailable or rejected without changing the config, preparing a graph, or selecting a fallback

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

