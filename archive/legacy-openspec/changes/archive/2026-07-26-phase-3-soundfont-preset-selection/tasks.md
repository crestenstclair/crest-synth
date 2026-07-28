## 1. Canonical preset catalog and parse-once asset

- [x] 1.1 Add `SoundFontPresetId`, its strict reversible `sf2.bank-<bank>.program-<program>` codec, `SoundFontPresetCatalogEntry`, collision diagnostics, and `SoundFontPresetCatalog` in the Synth context; cover valid limits, bank 128, malformed IDs, duplicate labels, and lookup errors with unit tests.
- [x] 1.2 Implement catalog construction in source PHDR order using exact RustySynth-authored names, accept only effective playable addresses, retain the first playable duplicate coordinate with typed shadow diagnostics, and sort final choices numerically by bank then program without a GM-name table.
- [x] 1.3 Introduce `HiDefSoundFontAsset` to open and parse `./sf2/HiDef.sf2` once and produce the immutable catalog plus a separate numeric `PreparedSoundFontBank`; return typed file/parse/metadata/sample/region/catalog errors without partial output or fallback.
- [x] 1.4 Refactor `PreparedSoundFontBank` to own copied immutable PCM, regions, loop metadata, and numeric preset addresses only; remove `Arc<rustysynth::SoundFont>` and prove raw parser/name-bearing ownership is dropped before callback-reachable graph ownership.
- [x] 1.5 Change `HiDefSoundFontCapability` and `HiDefSoundFontPreparer` constructors to accept the shared loader outputs and prohibit either adapter from reopening or reparsing the asset.
- [x] 1.6 Update `src/bin/crest_synth.rs` and test composition helpers to construct the asset once, inject its catalog into provider/fixture paths and its catalog plus numeric bank into the preparer, preserving startup-before-audio failure ordering.

## 2. Catalog-backed capability and fixture configs

- [x] 2.1 Extend `ParameterSpec` with canonical `ReadOnly | StructuralChoice` PATCH interaction metadata and validation that permits `StructuralChoice` only on non-Asset Structural Choice schemas with at least two choices; update equality/serialization/schema descriptors and all provider fixtures.
- [x] 2.2 Replace HiDef bank/program/percussion specs and assignments with ordered catalog-backed `soundfont.preset` plus locked `soundfont.file`; use the first sorted playable preset as the descriptor default and derive percussion from bank 128 during preparation.
- [x] 2.3 Add `DescriptorDefaultConfigFactory::replace_structural_choice` (or the equivalent canonical method) that validates the source and returns a candidate differing in exactly one declared Choice assignment; reject wrong capability/parameter/kind/choice without fallback.
- [x] 2.4 Update `SoundFontInstrument`, `AutomaticMidiTest`, `CorridorsMidiEventSource` integration, smoke fixtures, and test builders so normalized MIDI addresses resolve to exact catalog choice IDs before Patch installation and no second bank/program/percussion identity remains in `InstrumentConfig`.
- [x] 2.5 Advance every frozen descriptor/config/state/projection surface version affected by the breaking schema migration and update bidirectional schema-surface assertions; do not add compatibility reads for nonexistent persisted sessions.

## 3. Descriptor-derived PATCH focus and projection

- [x] 3.1 Extend `PatchControlId` with `Capability(ParameterId)` and implement one canonical resolver returning Engine, Attack, Decay, Sustain, Release, then active descriptor `StructuralChoice` parameters in descriptor order.
- [x] 3.2 Make `AppState::apply` use the resolver for nonwrapping navigation, action availability, Patch/capability changes, and focus clamping; SoundFont must have Preset after Release while Braids must still end at Release.
- [x] 3.3 Extend `PatchPageProjection`, `StateTree`, `TextProjection`, and `StateProjector` so structural rows carry control/parameter IDs, stable selected/requested choice IDs, exact authored labels, ordered choices, editability, lifecycle/failure, selected line, and graph revision from one accepted generation.
- [x] 3.4 Keep `EframeTextWindow` thin: render the exact projected Preset row and selected line, route input only through `KeyboardInputTranslator`, and add no SoundFont branch, private focus, choice parsing, widget state, modal, or browser.
- [x] 3.5 Update `patch_page_projection`, `schema_surface`, `eframe_context`, capability-schema, and reducer tests for dynamic differently shaped Patch surfaces, exact authored labels/order, focus clamping, boundary behavior, and audio neutrality of navigation.

## 4. Shared structural edit lifecycle

- [x] 4.1 Add `StructuralEditIntent::{ReplaceCapability, ReplaceParameterChoice}` and include it in every request/status/effect/event/worker result/graph correlation record needed to prove the permitted delta; retain historical `EngineSelection*` names only where the design explicitly permits bounded compatibility naming.
- [x] 4.2 Generalize PATCH Adjust handling so a valid adjacent preset enters the same app-wide request sequence and busy guard as engine selection, remains source-preserving in Preparing, and emits exactly one preparation effect through `AppState::apply`.
- [x] 4.3 Update `AppLoop`, `GraphPreparationWorker`, deterministic/threaded worker adapters, `PreparedGraphBuilder`, and `StructuralGraphCoordinator` to build, revalidate, publish, activate, acknowledge, and retire either intent without a second queue or lifecycle.
- [x] 4.4 On prepared preset commit, prove only the targeted Patch's `soundfont.preset` assignment changes, retain every other Patch/config/route/mixer/envelope/device value, and merge compatible scalar edits accepted during Preparing/Activating into the correct candidate generation.
- [x] 4.5 Add exact unchanged handling for first/last choice boundaries, unsupported directions, busy requests, missing presets, preparation failure, worker pressure, early/duplicate/stale/mismatched results and acknowledgements, and subsequent valid recovery with no preset/asset/config/engine fallback.
- [x] 4.6 Update engine-selection workflow tests so engine and preset requests contend through one guard, all structural correlation fields serialize exactly, candidates/retired graphs are destroyed only off callback, and both original engine directions still pass.

## 5. Numeric SoundFont preparation and real-time safety

- [x] 5.1 Resolve `soundfont.preset` on control/worker ownership from stable choice ID to exact numeric prepared-bank address; reject missing/malformed/catalog-bank disagreement before producing a prepared instrument.
- [x] 5.2 Make `SoundFontVoiceEngine` select melodic and bank-128 percussion presets from numeric prepared data without labels, raw SoundFont access, default preset selection, or nearest-address fallback.
- [x] 5.3 Instrument the production callback path to prove preset preparation/activation adds zero allocation and zero destruction and that callback-reachable SoundFont ownership contains zero strings, paths, catalog entries, or parser structures.
- [x] 5.4 Re-run prepared-rack and production runtime contracts for exact Patch targeting, complete block-boundary swap, fixed-size acknowledgement, bounded queue-pressure retention, scalar-layout compatibility, and control-side retirement.

## 6. Dedicated real-SF2 acceptance

- [x] 6.1 Add `tests/soundfont_preset_selection.rs` with ordinary assertion-bearing tests and a structured `CREST_SOUNDFONT_PRESET_OBSERVATION`; emit `CREST_ACCEPTANCE soundfont_preset_selection passed` only after every declared predicate succeeds.
- [x] 6.2 Against the real fixed asset, compare the complete effective catalog with RustySynth source entries, assert exact names and numeric order, demonstrate that raw and alphabetical order are rejected where the fixture discriminates, round-trip every choice ID, and prove no GM names are synthesized.
- [x] 6.3 Add synthetic catalog cases for reordered PHDR records, duplicate labels, duplicate coordinates/first-record precedence, bank 128, empty names, invalid program/bank values, empty zones/catalog, and typed collision/error behavior.
- [x] 6.4 Drive both adjacent directions and both boundaries through `KeyboardInputTranslator` and `AppLoop`, then exercise source audio during preparation, exact candidate commit, newer-revision activation, target-only finite nonzero distinct audio, controlled failure/stale/busy cases, and descriptor-default restoration.
- [x] 6.5 Run `cargo test --release --test soundfont_preset_selection -- --nocapture` and require exit 0 plus the exact acceptance marker and all structured predicates; a zero-test match, skipped real asset, construction-only assertion, or success-on-missing-fixture branch does not count.

## 7. Exhaustive deterministic demo

- [x] 7.1 Extend typed demo surface discovery, checkpoints, reports, and observations with catalog choices, `Capability(soundfont.preset)`, structural intents/transitions, authored-name/order equality, preset boundaries/failures, target audio, callback metadata counts, and scalar/structural coexistence.
- [x] 7.2 Add one adjacent preset success path before the existing engine switches, including pending source audio, busy rejection, exact one-assignment commit, Activating/Ready acknowledgement, targeted audible difference, restoration, and later controlled failure/stale/mismatched recovery.
- [x] 7.3 Update all deterministic coverage oracles from production descriptors/serializers rather than hand-maintained preset lists, preserve every existing ADSR/Braids/mixer/effect/mutation assertion, and require no missing or unexpected identifiers.
- [x] 7.4 Run `make demo` twice from fresh state, require both commands to exit 0, and compare the complete `CREST_EVENT_LOG`, `CREST_STATE_TREE`, coverage, checkpoints, and observation records byte-for-byte; diagnose and fix any nondeterminism before marking this task complete.

## 8. Live physical demo and lockup prevention

- [x] 8.1 Extend `LiveDemoScene`, checkpoint/report schemas, and exact coverage with ordered structural transitions `[SoundFontPresetToNext, SoundFontToBraids, BraidsToDescriptorDefaultSoundFont]`, preserving the frozen scalar coverage set.
- [x] 8.2 After scalar coverage, focus Preset semantically, dispatch one adjacent choice, and require correlated Preparing/Activating/Ready plus a newer acknowledged revision and exact active choice ID before proceeding to Engine and the two existing directions.
- [x] 8.3 Bracket the preset transition with bounded Patch-targeted semantic NoteOn/NoteOff and wait for a fresh matching-generation/revision finite nonzero physical observation that cannot be satisfied by source or unrelated Patch audio.
- [x] 8.4 Keep the UI tick nonblocking and make progress accounting stage-specific: only semantic dispatch, exact checkpoint capture, structural lifecycle advancement, or completed cleanup resets the ten-second watchdog; return typed preset/audio/whole-run errors with cleanup instead of hanging.
- [x] 8.5 Expand deterministic `live_demo` tests for preset success, no-audio/no-progress timeout, mapped-input isolation, final descriptor-default preset restoration, zero active notes, exactly one completion/close, stream-before-worker teardown, and inert post-completion behavior.
- [x] 8.6 Run the real `make demo-live` yourself on the configured physical window/audio path; do not ask the user to validate it, and do not substitute `make -n`, a headless harness, or log inspection. If it locks up, times out, or exits nonzero, diagnose the exact stage, fix it, and rerun until a usable device yields exit 0 after all three structural transitions, final records, note cleanup, window close, stream release, worker/graph shutdown, and parent process exit.

## 9. Full gates and roadmap completion

- [x] 9.1 Run `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`; fix every failure without weakening or skipping existing tests.
- [x] 9.2 Run the declared smoke, capability schema, PATCH projection, engine workflow, prepared rack, per-voice envelope, schema surface, eframe context, exhaustive demo, live harness, mutation, and production real-time contract gates in addition to the dedicated release preset target.
- [x] 9.3 Re-evaluate `openspec context --json`, run `git diff --check`, and strictly validate this change and the complete OpenSpec workspace; reconcile any DESIGN/CUE/OpenSpec drift before completion.
- [x] 9.4 Only after the deterministic and real live acceptance runs succeed, update `ROADMAP.md` to mark `phase-3-soundfont-preset-selection` and Phase 3 complete, record that Phase 3 has no remaining planned increment, and identify the already-declared next roadmap increment without redefining the product architecture.
