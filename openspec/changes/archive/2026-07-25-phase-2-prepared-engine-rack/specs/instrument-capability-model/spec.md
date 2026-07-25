## MODIFIED Requirements

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

### Requirement: Phase 2 increment boundary
This increment SHALL install only the current SoundFont capability and runtime preparer while establishing the generic prepared-instrument ports, bounded engine rack, and structural graph handoff. It SHALL NOT add Braids C++ or FFI code, a second production capability, user-selectable or simultaneous mixed-engine product configuration, engine selection, PATCH page, editable capability parameters, modulation, layering, or per-Patch effects. Deterministic heterogeneous prepared test instruments MAY prove the generic runtime contract without being installed as product capabilities.

#### Scenario: Application starts after the prepared-rack refactor
- **WHEN** the normal, smoke, headless-demo, or live-demo path is launched
- **THEN** current SoundFont behavior remains available through exactly one installed capability and preparer behind the generic rack, and no unavailable engine or UI feature is presented

#### Scenario: Runtime polymorphism is verified
- **WHEN** the prepared-rack contract uses two different deterministic prepared instrument implementations
- **THEN** they share the same bounded rack and renderer contract without appearing in the production capability registry or Patch projection

