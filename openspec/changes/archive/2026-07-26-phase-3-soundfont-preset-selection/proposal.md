## Why

The fixed SoundFont already contains authored preset names and numeric bank/program identities, but Crest Synth currently models them as separate low-level fields and exposes no coherent PATCH control for choosing a preset. This phase makes those embedded presets visible and selectable in their deterministic SF2/General MIDI coordinate order while preserving the existing reducer, prepared-graph, and hard-real-time boundaries.

## What Changes

- Parse `./sf2/HiDef.sf2` once at startup into an immutable control-side catalog and separate numeric, callback-safe prepared data.
- Preserve each playable SF2 preset's exact authored `achPresetName`; identify and sort choices by numeric `wBank`, then `wPreset`, without synthesizing General MIDI names or trusting raw file order.
- **BREAKING** Replace the HiDef descriptor's independent bank/program/percussion assignments with one catalog-backed `soundfont.preset` structural Choice plus the locked `soundfont.file` asset.
- Add an explicit PATCH interaction classification to parameter schemas and derive focus as Engine, Attack, Decay, Sustain, Release, then descriptor-declared structural choices. SoundFont gains Preset; Braids remains unchanged.
- Generalize the existing one-in-flight engine-selection workflow with a typed structural-edit intent so an adjacent preset change uses the same reducer, capacity-one worker, complete graph preparation, block-boundary activation, and off-callback retirement path.
- Resolve automatic MIDI fixture instruments to exact catalog identities and fail visibly when an address is absent; never substitute a preset, label, asset, or engine.
- Extend deterministic and live observable demos with a correlated, visible, audible preset transition and real callback ownership measurements.

## Capabilities

### New Capabilities

- `soundfont-preset-selection`: Exact SF2 preset discovery, authored-name presentation, numeric ordering/identity, descriptor-derived PATCH selection, prepared structural replacement, and behavioral proof.

## Impact

- Affects SoundFont loading/preparation, capability descriptors/config validation, fixture configuration, PATCH control IDs/projection, structural request correlation, demo/report schemas, the standalone composition root, and acceptance tests.
- Integrates with the existing capability-model, SoundFont-audio, automatic-MIDI, PATCH/control, prepared-rack, real-time, deterministic-demo, and live-demo contracts; the new capability owns their added cross-cutting preset-selection behavior.
- Changes serialized descriptor, InstrumentConfig, StateTree, TextProjection, and demo coverage surfaces; schema versions must advance where those shapes are externally observed.
- Adds no new runtime dependency, top-level UI context, alternate SoundFont asset, preset browser/modal, persistence, callback transport, or fallback behavior.
- Requires successful CUE/OpenSpec validation, the named release-mode real-SF2 acceptance target, deterministic `make demo`, and an agent-run `make demo-live` on the production window/audio path.
