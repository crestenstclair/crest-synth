## Why

Phase 2 now proves that SoundFont and Braids share one capability-polymorphic Patch, reducer, projection, prepared-rack, and render path. Phase 3 can therefore expose that real schema to the player, beginning with direct PATCH/MIXER selection and a read-only Patch page before any structural engine or preset workflow is admitted.

## What Changes

- Add `PATCH` and `MIXER` as the only reducer-owned top-level contexts; the existing diagnostic wall becomes the transitional MIXER projection.
- Map `1` and `2` through normalized `WindowInput` and `KeyboardInputTranslator` to semantic `SelectContext(MIXER|PATCH)` events handled only by `AppState::apply`.
- Add canonical `InteractionState` with a preserved MIXER selection and stable `PatchId` focus, plus an immutable `PatchPageProjection` derived from the focused Patch, installed registry, active descriptor/config, and common envelope.
- Render Patch identity, MIDI channel, active engine and installed engine choices, ADSR, and every descriptor-provided field with stable IDs and update metadata, without SoundFont/Braids page branches. All PATCH controls remain read-only in this slice.
- Prove that context selection advances coherent state/view generations but leaves Patch/config/mixer/envelope/global values, graph revision, parameter values, audio commands, prepared ownership, and rendered behavior unchanged.
- Expand production-owned input/event/schema descriptors, headless coverage, eframe-context verification, and named `patch_page_projection` acceptance for both installed capabilities and typed `ActionUnavailableInContext` recovery.
- **BREAKING**: extend the public semantic event/input vocabulary and versioned control/projection schema with context, focus, and Patch-page data.

## Capabilities

### New Capabilities

- `schema-driven-patch-page`: Direct semantic page selection, stable reducer-owned Patch focus, exact descriptor-driven read-only Patch projection, preserved MIXER projection, and audio-neutral switching.

### Modified Capabilities

- `instrument-capability-model`: Replace the Phase 2 “no PATCH page” boundary with generic read-only projection of the two installed descriptors while retaining the ban on runtime engine replacement and Structural editing.
- `one-way-parameter-control`: Extend the exact keyboard vocabulary with `1`/`2` context events and replace the single complete text view with two reducer-selected projections in the same basic adapter.
- `observable-demo-scene`: Add both page keys, both semantic contexts, `SelectContext`, interaction/page serialization, typed PATCH read-only rejection, and exact Patch projection to the production-derived exhaustive surface.

## Impact

The change affects Control value objects and serialization, `AppState`, `StateProjector`, `AppLoop`, Shell input normalization/translation, the eframe text adapter, deterministic/live observation compatibility, schema discovery, and test fixtures. It adds one named integration target and updates existing schema, GUI-context, demo, mutation-compatibility, smoke, and all-target gates. It changes no DSP, prepared engine, mixer, real-time transport, dependency, asset, or graph-replacement behavior.
