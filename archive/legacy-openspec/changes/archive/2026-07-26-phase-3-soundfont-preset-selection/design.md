## Context

Crest Synth currently describes HiDef SoundFont patches with separate `soundfont.bank`, `soundfont.program`, `soundfont.percussion`, and `soundfont.file` assignments. The capability provider is constructed independently from the preparer, while the preparer parses `HiDef.sf2` and keeps an `Arc<rustysynth::SoundFont>` reachable from `PreparedSoundFontBank`. PATCH focus is currently Engine plus the four canonical ADSR controls, and the one-in-flight prepared replacement protocol is publicly named for engine selection.

That shape cannot present the names authored in the SoundFont, does not give bank/program one coherent choice identity, and would tempt either a second selection workflow or string-bearing SF2 objects into callback ownership. The change spans Synth, Control, RealTime, Testing, and Shell, but it must preserve the existing physical input → semantic event → `AppState::apply` → projection/publication flow and the hard-real-time callback contract.

The format decision is grounded in section 7.2.1 of the [SoundFont 2.04 specification](https://musescore.org/sites/musescore.org/files/2023-01/sfspec24.pdf): a preset header contains `achPresetName[20]`, `wPreset`, and `wBank`; names are case-sensitive; preset records are not ordered; duplicate numeric coordinates make the first PHDR record active; bank 128 conventionally denotes General MIDI percussion; and presets without zones are ignored. The [MIDI Association](https://midi.org/about-midi-part-3midi-messages) defines Program Change and Bank Select as numeric selection messages, while [General MIDI](https://midi.org/general-midi) standardizes a sound set for compliant devices. GM therefore informs coordinate interpretation; it does not authorize replacing arbitrary SF2-authored names.

The fixed local asset discriminates these rules: its raw preset order begins with authored entries such as `Fiddle`, `Violin`, and `Hard FM EP`, whereas sorting bank 0 numerically yields program-coordinate order. The implementation must derive expectations from the parsed fixed asset rather than hard-code a copied name table.

## Goals / Non-Goals

Goals:

- Expose every effective playable HiDef preset under its exact authored name in numeric bank/program order.
- Make one preset choice a stable, validated part of `InstrumentConfig` and a descriptor-derived PATCH structural control.
- Reuse the existing reducer-owned, one-in-flight, prepared complete-graph replacement lifecycle for preset and engine edits.
- Parse the fixed SF2 once and keep all strings/parser ownership off the audio callback.
- Resolve the automatic MIDI fixture to exact catalog identities without fallback.
- Prove catalog fidelity, control correlation, graph activation, audible target output, callback safety, and live completion through production seams.

Non-goals:

- A preset browser, modal, search, categories, favorites, persistence, session migration, alternate SoundFont selection, or a third top-level UI context.
- Renaming every existing `EngineSelection*` public type in this increment; the lifecycle is generalized semantically while historical names remain where a broad rename would add no behavior.
- Treating General MIDI instrument names as SF2 metadata or alphabetically sorting authored labels.
- Adding a second structural queue, callback command, scalar snapshot field, direct engine mutation, or silent fallback.

## Decisions

### 1. Preset identity is numeric; its authored name is presentation

Add canonical Synth values:

- `SoundFontPresetId { bank: u16, program: u8 }`
- `SoundFontPresetCatalogEntry { id, name, source_ordinal }`
- `SoundFontPresetCatalog { entries, coordinate_collisions }`

The generic Choice ID is the reversible ASCII form `sf2.bank-<decimal>.program-<decimal>`. It contains no name, ordinal, registry index, or file path. Entries sort by `(bank, program)`, never by PHDR order or label. `name` is the exact nonempty content before the first NUL as exposed from the fixed 20-byte SF2 field, preserving case and authored spaces. Duplicate labels at different coordinates remain distinct choices.

For duplicate coordinates, catalog and prepared-bank construction retain the first playable PHDR record and record every shadowed ordinal as a typed collision diagnostic. This is deliberately implemented before any map insertion: RustySynth 1.3.6 exposes presets in source order, but its synthesizer lookup inserts repeated numeric IDs into a hash map and would otherwise make a later record active, contrary to the SF2 rule. Invalid program/bank values, empty names, empty catalogs, and unplayable entries are typed load failures or ignored exactly as specified; none selects a neighbor or default.

Alternative rejected: use a General MIDI name array keyed by program. That would erase the SoundFont author's presentation, collide across banks, and misrepresent non-GM/extended presets. Alternative rejected: use the label or raw ordinal as identity. Both are unstable when metadata or file order changes and do not match playback addressing.

### 2. One asset load produces two ownership projections

Introduce the infrastructure adapter `HiDefSoundFontAsset`. The composition root opens and parses `./sf2/HiDef.sf2` once, before descriptor freeze, fixture installation, graph preparation, or audio startup. From that one parsed value it constructs:

```text
HiDef.sf2
   │ one control-side parse
   ├── SoundFontPresetCatalog ──► capability schema / fixture resolution / projection
   └── PreparedSoundFontBank ───► preparer ─► prepared instruments ─► callback
                                      numeric PCM, zones, loops, addresses only
```

`PreparedSoundFontBank` copies the PCM and immutable region/loop/address data needed by `SoundFontVoiceEngine`; it no longer retains `Arc<rustysynth::SoundFont>`, preset names, parser structures, paths, or catalog entries. The raw `SoundFont` is dropped on control ownership before the numeric bank can enter callback-reachable graph ownership.

`HiDefSoundFontCapability` is constructed from the immutable catalog. `HiDefSoundFontPreparer` is constructed from that same catalog and numeric bank and never reopens the file. The catalog validates/decodes stable choice IDs on the worker side; prepared instruments receive only numeric resolved preset data.

Alternative rejected: parse once in the provider and again in the preparer. It can drift, doubles I/O/allocation, and weakens the one-asset observation. Alternative rejected: retain the RustySynth `SoundFont` merely because it is convenient. Its strings and nested parser-owned allocations would remain callback-reachable and be destructible on the wrong owner.

### 3. HiDef exposes one structural Choice

Extend `ParameterSpec` with `patch_interaction: ReadOnly | StructuralChoice`. `StructuralChoice` is valid only for non-Asset Structural Choice parameters with at least two choices. Existing scalar and asset parameters default to/read as `ReadOnly` on PATCH unless explicitly classified.

The hydrated HiDef descriptor contains, in order:

1. `soundfont.preset`: Structural Choice, choices from the sorted catalog, defaulting to its first entry, `patch_interaction = StructuralChoice`.
2. `soundfont.file`: required Structural Asset locked to `./sf2/HiDef.sf2`, `patch_interaction = ReadOnly`.

The old bank/program/percussion assignments are removed. Percussion is derived from `SoundFontPresetId.bank == 128` only when configuring numeric engine state. Braids retains its existing three scalar parameters and contributes no structural parameter control.

All frozen serialized descriptor/config/state/projection schemas advance versions together. There is no persisted session format, so migration is a cold reconstruction through the new providers rather than dual-read compatibility.

### 4. PATCH focus is resolved once from canonical types

Extend `PatchControlId` to:

- `Engine`
- `Envelope(VoiceEnvelopeParameter)`
- `Capability(ParameterId)`

One canonical resolver returns Engine, Attack, Decay, Sustain, Release, then active descriptor parameters whose `patch_interaction` is `StructuralChoice`, preserving descriptor order. Reducer navigation, `PatchPageProjection`, `StateProjector`, text rendering, schema discovery, deterministic demo generation, and live scene generation all consume this resolver. SoundFont therefore has six focus rows and Braids five; selection clamps when the focused Patch changes shape.

The capability row projects its schema label (`Preset`), stable choice ID, exact authored label, ordered choices, requested choice, lifecycle status/failure, and revision. The UI remains a thin text projection and does not parse choice IDs or own focus.

Alternative rejected: hard-code a SoundFont row in the page/projector. That would violate the descriptor-owned schema and create a capability branch across every consumer.

### 5. Preset and engine edits share one structural intent and lifecycle

Add `StructuralEditIntent`:

- `ReplaceCapability { target_capability_id }`
- `ReplaceParameterChoice { capability_id, parameter_id, choice_id }`

The existing app-wide request counter, `EngineSelectionStatus`, effects, capacity-one `GraphPreparationWorker`, complete `PreparedGraph` build, `StructuralGraphBoundary`, acknowledgement, and off-callback retirement remain the only lifecycle. Historical `EngineSelection*` names are retained for bounded migration, but every correlation record now includes the structural intent.

On Edit+Left/Right over Preset, `AppState::apply` resolves the adjacent declared Choice. A boundary is an unchanged typed rejection. A valid request records the intent and enters Preparing without changing the source `InstrumentConfig` or active graph. `DescriptorDefaultConfigFactory::replace_structural_choice` creates a candidate by replacing exactly one assignment in the current config, then validates it against the immutable descriptor. Preparation resolves the choice ID to the exact numeric preset and builds a complete graph.

Only a matching prepared result may commit, and revalidation proves that capability, Patch identity, and every assignment except the targeted preset remain exact. Scalar edits accepted during Preparing or Activating are refreshed into the candidate snapshot/layout as they are today. Ready requires block-boundary activation, the matching newer graph revision acknowledgement, and control-side collection of retired ownership. Busy, preparation failure, missing preset, stale/mismatched result, early/duplicate acknowledgement, queue pressure, or absent target audio preserves the source config/graph and never falls back.

Alternative rejected: publish bank/program through scalar snapshots. Preset identity changes region topology and engine setup, so treating it as a scalar risks incoherent half-updates and violates prepared structural ownership. Alternative rejected: add a preset-specific worker/status. Two app-wide structural requests could then race and commit against incompatible source revisions.

### 6. Fixture identity is resolved through the catalog

`CorridorsMidiEventSource` continues to discover normalized bank/program/percussion source identities. `AutomaticMidiTest` uses the injected catalog/config factory to map each even SoundFont part to the exact `SoundFontPresetId` and stable choice ID before installing Patches. The stored config has one preset assignment; it does not retain a second `SoundFontInstrument` identity.

An absent or ambiguous coordinate is a typed initialization error before graph publication or audio startup. Label matching, General MIDI name matching, nearest-neighbor selection, and descriptor-default fallback are forbidden.

### 7. Proof uses production paths and a real live run

Add `tests/soundfont_preset_selection.rs`, run in release mode against the fixed real SF2. It compares the catalog with the parser's complete effective preset sequence, proves exact names, numeric order, rejection of raw/alphabetical order where the fixture discriminates, stable ID round trips, duplicate handling with synthetic fixtures, exact fixture mapping, both adjacent directions/boundaries, one-assignment candidate deltas, pending source audio, activation, target-only finite nonzero distinct audio, restoration, zero fallback, zero callback allocation/destruction, and zero callback-reachable strings.

The exhaustive headless demo derives Preset from the descriptor, exercises success, busy, boundary, failure, stale/mismatched, scalar/structural coexistence, and deterministic restoration. The live scene adds exactly one adjacent preset transition before SoundFont → Braids → descriptor-default SoundFont. Each transition must show Preparing, Activating, Ready, a newer acknowledged revision, the exact target identity, and a fresh targeted physical-audio observation. Its no-progress timeout is reset only by semantic dispatch, checkpoint capture, lifecycle advancement, or cleanup—not by unrelated render polling.

Implementation acceptance includes running `make demo` twice for byte-identical deterministic evidence and running `make demo-live` directly on the production eframe/CPAL path. The implementing agent owns that execution and diagnosis; the user is not asked to validate it. A usable physical device must yield exit 0, final report records, semantic note cleanup, window close, stream release, and process exit.

## Risks / Trade-offs

- [RustySynth duplicate-coordinate behavior differs from SF2 first-record precedence] → Build catalog/prepared lookup explicitly in PHDR order, keep first playable, emit collision diagnostics, and test synthetic duplicates independently of `Synthesizer` lookup.
- [Copying all sample/region data may increase startup memory or time] → Perform one measured startup conversion, share the immutable numeric bank across preparers/instruments, and prohibit per-Patch copies or parses.
- [A dynamic focus count can expose stale indices when changing Patch or engine] → Resolve and clamp focus inside `AppState::apply` after every accepted structural commit and use the same resolver for projection/tests.
- [Historical `EngineSelection*` names can obscure generalized semantics] → Put `StructuralEditIntent` in every public correlation record and describe the compatibility naming explicitly; defer wholesale renaming until it has independent value.
- [Audible differences can be note/preset dependent] → Render source and target from identical fresh state with a bounded discriminating note set; require target identity, nonzero output, and at least one measured sample/energy difference rather than relying on a label.
- [Live fixture sparsity can repeat the prior no-progress failure] → Bracket the preset checkpoint with semantic targeted NoteOn/NoteOff, correlate the observation to the active revision/generation, keep timeout stages typed, and run the real demo before completion.

## Migration Plan

1. Introduce catalog/ID types and the parse-once asset adapter; make the numeric prepared bank independent of raw `SoundFont` ownership.
2. Hydrate the HiDef descriptor from the catalog, replace old assignments with `soundfont.preset`, update fixture config construction, and advance schema versions.
3. Add descriptor-derived structural focus and generalized structural intent while preserving the existing engine lifecycle transport/ownership protocol.
4. Extend the preparer, graph correlation, deterministic/live scenes, reports, and named acceptance targets.
5. Run focused tests, release real-SF2 acceptance, all-target tests/lints, `make demo` twice, and the real `make demo-live`; only then mark tasks complete and advance the roadmap.

Rollback is a source-level revert of this change before any release. There is no persisted user/session data to down-migrate. A partial rollback that restores bank/program fields while retaining catalog-aware configs or generalized schema versions is not supported.
