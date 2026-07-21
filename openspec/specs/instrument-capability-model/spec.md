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
Each installed instrument capability SHALL expose one immutable ordered descriptor containing its label, semantic accent, sections, parameter specifications, asset requirements, bounded voice capacity, and supported MIDI message kinds. Each parameter specification SHALL declare its stable identity, label, value kind, default, Scalar or Structural update class, applicable bounds or choices, adjustment steps, formatting metadata, and declarative dependencies.

#### Scenario: Current SoundFont descriptor is inspected
- **WHEN** the current application registry is constructed
- **THEN** it contains exactly `instrument.soundfont.hidef` with ordered specifications for bank, program, percussion, and `./sf2/HiDef.sf2`, all classified as Structural

#### Scenario: Descriptor metadata is invalid
- **WHEN** a descriptor repeats an identity, declares an invalid default or range, references an undeclared dependency, reports zero voice capacity, or repeats a supported MIDI kind
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
The accepted application state SHALL contain one immutable ordered registry of installed capability descriptors, and every installed Patch config SHALL resolve to exactly one descriptor and satisfy its declared parameters, values, assets, kinds, bounds, choices, and dependencies.

#### Scenario: Valid fixture Patches are installed
- **WHEN** all fixture configs match the installed SoundFont descriptor
- **THEN** installation succeeds atomically, the registry remains unchanged, and every Patch retains its validated generic config

#### Scenario: Patch config is invalid
- **WHEN** a config uses an unknown capability or contains a missing, duplicate, undeclared, wrong-kind, dependency-invalid, or out-of-range assignment or asset reference
- **THEN** Patch installation fails atomically with a typed error and no descriptor, value, asset, preset, capability, or engine is substituted

### Requirement: Capability provider boundary
Capability-specific descriptor and generic config construction SHALL be supplied through a control-side provider boundary that accepts only typed parameter assignments and asset references, performs no rendering, device access, audio-buffer access, UI behavior, or file loading, and returns typed errors without fallback. The polymorphic boundary SHALL NOT expose a SoundFont-specific construction method.

#### Scenario: Fixture identity is converted
- **WHEN** automatic MIDI initialization translates a valid SoundFont instrument identity into the stable assignments declared by the installed descriptor
- **THEN** the provider deterministically validates and returns a config matching its own descriptor without reading or decoding the SoundFont file

#### Scenario: Provider and registry disagree
- **WHEN** the provider descriptor is missing from the registry or differs from the registered descriptor
- **THEN** startup fails before Patch installation and no alternate provider or renderer is selected

### Requirement: Generic capability projection
Canonical serialized state and the current text projection SHALL include the installed descriptor registry plus every Patch capability identity, parameter assignment, and asset reference in descriptor order. Projection SHALL derive from descriptors and configs without capability-identity branches or duplicate engine-specific field lists.

#### Scenario: Maintainer inspects current state
- **WHEN** the current SoundFont fixture has been installed
- **THEN** StateTree and the text view show the exact registered descriptor and every Patch config value and asset from the same accepted generation

#### Scenario: Descriptor order changes
- **WHEN** the production descriptor order differs from a duplicated expected order
- **THEN** serialization, text projection, and schema-derived coverage follow the production descriptor and the duplicated expectation fails verification

### Requirement: Phase 2 increment boundary
This increment SHALL install only the current SoundFont capability and renderer while establishing the generic control model. It SHALL NOT add the prepared multi-engine rack, Braids C++ or FFI code, simultaneous mixed-engine rendering, engine selection, PATCH page, editable capability parameters, modulation, or per-Patch effects.

#### Scenario: Application starts after the capability refactor
- **WHEN** the normal, smoke, headless-demo, or live-demo path is launched
- **THEN** current SoundFont behavior remains available through exactly one installed capability and no unavailable engine or UI feature is presented
