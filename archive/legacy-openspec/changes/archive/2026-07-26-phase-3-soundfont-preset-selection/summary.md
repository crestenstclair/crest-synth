# Change Summary

## Outcome

- **Problem:** HiDef presets are unnamed low-level bank/program/percussion fields with no coherent PATCH selection control.
- **Result:** Players see exact SF2-authored preset names in numeric bank/program order and select them through the production prepared structural path.

## Change Outline

- **Adds:** `soundfont-preset-selection`, a parse-once catalog, stable `SoundFontPresetId`, `StructuralEditIntent`, descriptor-derived Preset focus, and measured preset evidence.
- **Changes:** HiDef exposes one `soundfont.preset` Choice plus locked file; engine and preset edits share one app-wide lifecycle.
- **Removes:** Independent SoundFont bank/program/percussion config assignments and callback ownership of raw RustySynth metadata.

## System Impact

- **Capabilities:** Adds `soundfont-preset-selection`; modifies capability model, SoundFont audio, automatic MIDI, PATCH/control, structural rack/RT, and both demos.
- **Architecture:** Centers `valueObject.Synth.SoundFontPresetId`, `valueObject.Synth.SoundFontPresetCatalog`, `adapter.HiDefSoundFontAsset`, `valueObject.Control.StructuralEditIntent`, and existing worker/graph ports.
- **Interfaces/data:** Advances frozen descriptor/config/state/projection schemas; PATCH focus becomes Engine → ADSR → descriptor `StructuralChoice` rows.

## Delivery

- **Implementation:** Build catalog/loader, migrate capability and fixture configs, add dynamic focus/projection, generalize structural correlation, then extend tests and demos; see [tasks](tasks.md).
- **Validation:** Require release real-SF2 acceptance, all existing gates, two byte-identical `make demo` runs, and an agent-run successful real `make demo-live`.
- **CUE:** All 15 source declarations evaluate with the new required goal, capability, resources, evidence, and completion gate.

## Risks and Decisions

- **Key decisions:** Numeric coordinates are identity/order; exact authored names are presentation; first playable duplicate coordinate wins; GM names are never synthesized.
- **Ownership:** Catalog strings stay control-side; callback storage is numeric/preallocated; no new transport, lifecycle, fallback, modal, persistence, or top-level context.
- **Risks:** Duplicate-coordinate library behavior, dynamic focus clamping, startup conversion cost, and live audio stalls are covered by explicit diagnostics, shared resolvers, ownership tests, and stage-specific watchdogs; see [design](design.md).
