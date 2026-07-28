## ADDED Requirements

### Requirement: Exact SF2 preset catalog

The system SHALL construct one immutable catalog from every effective playable preset in the fixed SoundFont, preserving the exact nonempty `achPresetName` content before its first NUL and the numeric `wBank`/`wPreset` address.

#### Scenario: Authored names and numeric order are preserved
- **WHEN** `./sf2/HiDef.sf2` is loaded
- **THEN** every selectable label equals its case-sensitive authored SF2 name
- **AND** entries are ordered by ascending numeric bank and then ascending numeric program
- **AND** neither raw PHDR order, alphabetical label order, nor a General MIDI name table defines the order or label

#### Scenario: Duplicate names remain distinct
- **WHEN** two playable presets have the same authored name at different numeric addresses
- **THEN** both entries remain selectable under different stable choice IDs
- **AND** the name is never used as identity or lookup input

#### Scenario: Duplicate coordinates follow SF2 precedence
- **WHEN** multiple playable PHDR records have the same bank and program
- **THEN** the first playable source record is the sole selectable entry for that address
- **AND** every shadowed source ordinal is retained in a typed collision diagnostic

#### Scenario: Invalid catalog content fails explicitly
- **WHEN** the parsed asset contains no playable presets, an invalid selectable coordinate, or an empty effective authored name
- **THEN** startup fails with a typed catalog error
- **AND** no synthetic label, adjacent preset, descriptor default, alternate asset, or engine is substituted

### Requirement: Stable numeric preset identity

The system SHALL represent a SoundFont preset with the canonical numeric identity `SoundFontPresetId { bank, program }` and encode generic Choice IDs as `sf2.bank-<decimal>.program-<decimal>`.

#### Scenario: Choice identity round-trips
- **WHEN** a catalog entry is encoded and decoded through its Choice ID
- **THEN** the resulting bank and program equal the original address exactly
- **AND** the ID contains no label, ordinal, registry index, or filesystem path

#### Scenario: Percussion is derived from the address
- **WHEN** a selected preset has bank 128
- **THEN** the preparer configures conventional SF2 percussion behavior from that bank
- **AND** no independent percussion assignment exists in `InstrumentConfig`

### Requirement: Catalog-backed HiDef schema

The system SHALL expose the HiDef capability as one `soundfont.preset` Structural Choice hydrated from the ordered catalog plus one required locked `soundfont.file` Structural Asset. Every `ParameterSpec` SHALL classify PATCH interaction as `ReadOnly` or `StructuralChoice`, and capability-neutral consumers SHALL use that classification instead of capability IDs.

#### Scenario: Descriptor is hydrated before registry freeze
- **WHEN** the production capability registry is constructed
- **THEN** the preset choices match the catalog IDs, order, and authored labels exactly
- **AND** the first sorted playable entry is the descriptor default
- **AND** the legacy bank, program, and percussion parameter specs are absent

#### Scenario: Malformed preset assignment is rejected
- **WHEN** a HiDef config omits the preset, names an undeclared choice, supplies a wrong parameter kind, or changes the fixed file
- **THEN** descriptor validation rejects the complete config without partial installation or fallback

#### Scenario: PATCH interaction classification is validated
- **WHEN** a parameter is classified `StructuralChoice`
- **THEN** it is a non-Asset Structural Choice with at least two declared values and its stable `ParameterId` may become a `PatchControlId::Capability` row
- **AND** descriptor construction rejects Asset, Scalar, one-choice, or empty-choice parameters with that classification before registry installation

#### Scenario: Provider and factory use the frozen catalog
- **WHEN** the provider returns its descriptor, validates a config, or the config factory replaces an adjacent structural choice
- **THEN** no file is read or parsed and only the catalog injected before registry freeze is used
- **AND** a valid replacement differs in exactly the targeted assignment while every other assignment, Patch field, and registry identity remains exact

### Requirement: Fixture SoundFont identities resolve exactly

Automatic MIDI initialization SHALL resolve every discovered SoundFont bank/program/percussion identity to one exact `SoundFontPresetCatalog` entry before installing Patches.

#### Scenario: Fixture preset exists
- **WHEN** an even fixture part identifies a SoundFont bank and program
- **THEN** its `InstrumentConfig` contains the matching stable `soundfont.preset` choice ID and the locked file assignment
- **AND** no separate bank/program/percussion identity is retained in the Patch config

#### Scenario: Fixture preset is missing
- **WHEN** no catalog entry has the discovered numeric address
- **THEN** initialization returns a typed error before Patch installation, graph publication, MIDI start, or audio startup
- **AND** it does not match by authored or General MIDI name or choose a default/neighbor

### Requirement: Descriptor-derived PATCH preset control

The system SHALL derive the focused Patch control sequence as Engine, Attack, Decay, Sustain, Release, followed by active descriptor parameters classified `StructuralChoice` in descriptor order. Reducer navigation, action availability, page/state/text projection, schema coverage, and deterministic/live scene generation SHALL consume one canonical resolver.

#### Scenario: SoundFont exposes Preset after Release
- **WHEN** PATCH focuses a HiDef SoundFont Patch
- **THEN** `PatchControlId::Capability(soundfont.preset)` follows Release
- **AND** the row projects `Preset`, its `ParameterId`, the stable selected and requested choice IDs, exact authored labels, ordered choices, editability, correlated lifecycle/failure, selected line, and graph revision from one accepted generation

#### Scenario: Braids does not gain a preset row
- **WHEN** PATCH focuses a Braids Patch
- **THEN** its control sequence ends at Release because its descriptor declares no structural Choice
- **AND** navigation and selection clamping use the same canonical resolver as SoundFont, with reducer-owned focus clamped when a destination Patch has fewer controls

#### Scenario: Preset request is pending
- **WHEN** a preset replacement is Preparing or Activating
- **THEN** the row shows the active authored label and correlated requested authored label while the source config remains the projected active value until candidate commit
- **AND** a parameter classified `ReadOnly` is absent from the focus/edit resolver even when it is displayed as descriptor data

### Requirement: Correlated prepared preset replacement

The system SHALL apply an adjacent preset selection as one `ReplaceParameterChoice` structural intent through `AppState::apply`. Engine and parameter-choice replacement SHALL share one app-wide request sequence, busy guard, lifecycle status/effects, capacity-one worker, complete graph handoff, acknowledgement, and off-callback retirement path, with the complete intent carried by every correlation record.

#### Scenario: Adjacent preset activates successfully
- **WHEN** Edit+Right or Edit+Left selects a declared adjacent preset
- **THEN** semantic input reaches only `AppState::apply`, which records one request ID, Patch ID, complete intent, source/target capability IDs, source revision, and candidate config and enters Preparing while retaining the source config and graph
- **AND** the worker builds a complete candidate whose targeted Patch differs only in `soundfont.preset`
- **AND** the config commits only after request, Patch, intent, capability, source/target revision, and exact one-assignment delta all match
- **AND** Ready occurs only after block-boundary activation, newer revision acknowledgement, and off-callback retirement collection
- **AND** no UI, MIDI, asset, provider, or audio adapter mutates config directly

#### Scenario: Structural request is already active
- **WHEN** an engine or preset request is Preparing or Activating and another structural edit is attempted
- **THEN** the new edit is rejected as busy without changing generation, config, graph revision, or audio commands

#### Scenario: Preset boundary does not wrap
- **WHEN** Edit+Left targets before the first catalog entry or Edit+Right targets after the last
- **THEN** the event is an unchanged typed structural-selection rejection
- **AND** no parameter snapshot, audio command, preparation request, or graph is published and a later valid adjacent edit remains accepted

#### Scenario: Structural and scalar edits coexist
- **WHEN** a declared scalar edit is accepted while a preset request is Preparing or Activating
- **THEN** the scalar uses the existing latest-snapshot path while the preset retains exact correlation
- **AND** the activated candidate consumes the newest compatible scalar state rather than encoding preset identity as bank/program snapshots

#### Scenario: Complete graph replacement uses existing ports
- **WHEN** a valid preset candidate is built, published, or encounters temporary prepared/retired queue pressure
- **THEN** Patch IDs/order/count, routes, mixer values, envelopes, device bounds, rack capacity, effects, and untargeted configs remain exact while only the selected preset assignment and numeric regions may differ
- **AND** bounded staging/retry preserves owners, the generic structural boundary swaps only at a render-block boundary, and no preset-specific transport or renderer branch is introduced

#### Scenario: Failure preserves source state
- **WHEN** the target is missing, preparation fails, correlation is stale or mismatched, acknowledgement is early or mismatched, or target audio cannot be established
- **THEN** the source config and active graph remain authoritative
- **AND** the failure is visible, mismatched candidate ownership is destroyed off callback, and no preset, asset, config, or engine fallback occurs

### Requirement: SoundFont metadata stays off the callback

The system SHALL parse `./sf2/HiDef.sf2` exactly once before registry freeze and graph preparation, derive the control-side catalog and shared numeric prepared bank from that parse, and make callback-reachable SoundFont storage contain only immutable numeric PCM, prepared zones, loop data, preset addresses, and fixed engine state.

#### Scenario: Raw parser ownership is released
- **WHEN** normal, smoke, deterministic-demo, or live-demo startup finishes constructing the catalog and prepared bank
- **THEN** the raw RustySynth SoundFont and every parser/name-bearing structure are dropped on control ownership
- **AND** provider and preparer receive their injected projections without reopening the file, and neither a String, catalog entry, path, nor raw SoundFont is reachable from a prepared instrument or graph

#### Scenario: Exact numeric preset renders
- **WHEN** a valid melodic or bank-128 percussion choice is prepared
- **THEN** its stable choice ID resolves to the exact numeric bank/program regions and produces finite stereo output for a supported note, deriving percussion only from bank 128
- **AND** an address absent from either catalog or prepared bank fails visibly without a default or nearest-preset fallback

#### Scenario: Preset render remains hard real-time safe
- **WHEN** the callback renders the selected preset or swaps to its prepared graph
- **THEN** it performs no allocation, destruction, label lookup, parsing, locking, blocking, I/O, logging, formatting, panic, unwind, or fallback
- **AND** intent/effects use the discrete path, compatible scalar parameters use the latest-snapshot path, the complete graph uses the structural ownership path, and bounded observations contain only numeric/fixed-size correlation and audio measurements

### Requirement: Measured preset-selection proof

The system SHALL prove preset selection through the fixed real SF2, production reducer, worker, structural boundary, and renderer rather than through construction success or self-reported labels.

#### Scenario: Named acceptance target passes
- **WHEN** `cargo test --release --test soundfont_preset_selection -- --nocapture` succeeds
- **THEN** its structured observation proves one parse, exact names/order, fixture resolution, both adjacent directions and boundaries, descriptor-derived focus, exact target-only config mutation, shared busy/failure/stale correlation, scalar/structural coexistence, newer revision activation, finite nonzero distinct target audio, restoration, zero fallbacks, zero callback allocations/destructions, and zero callback-reachable strings

#### Scenario: Deterministic production-path demo repeats exactly
- **WHEN** `make demo` runs twice from fresh identical services
- **THEN** both runs produce byte-identical complete event log, state tree, coverage, checkpoints, and observation JSON
- **AND** missing, unexpected, duplicated, reordered, renamed, unexercised, or nondeterministic preset evidence makes the report incomplete

#### Scenario: Physical live preset transition completes
- **WHEN** `make demo-live` runs with a usable physical output device
- **THEN** one adjacent authored-name preset transition reaches Preparing, Activating, and Ready before the two engine transitions
- **AND** ordered coverage contains preset-next, SoundFont-to-Braids, and Braids-to-descriptor-default-SoundFont with no missing or unexpected transition
- **AND** it produces a fresh targeted physical-audio observation, descriptor-default preset restoration, semantic note cleanup, zero active notes, one window close, stream release, worker/graph shutdown, and successful parent-process exit
- **AND** a ten-second preset-stage stall returns a typed non-progress error and cleanup without a successful report; dry runs, deterministic harnesses, or user confirmation cannot substitute for the implementing agent's physical run
