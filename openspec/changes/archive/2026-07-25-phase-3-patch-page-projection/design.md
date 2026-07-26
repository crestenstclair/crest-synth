## Context

Phase 2 leaves Crest Synth with one capability-polymorphic state and audio path, but its UI is still a single diagnostic text wall. `AppState` owns a flat mixer selection; `AppEvent` has no page event; `WindowInput` knows only W/S/A/D/K; and `StateProjector` renders one `TextProjection`. The completed capability model already provides the stable IDs, ordered sections, parameter metadata, generic configs, asset references, and common Patch ADSR needed for a Patch page.

This change is the first Phase 3 slice. It must preserve the basic adapter and all production audio behavior while introducing the durable semantic seam later engine-selection, ADSR-edit, and SoundFont-preset changes will use. `DESIGN.md` fixes PATCH and MIXER as the only top-level contexts, reducer ownership of interaction state, immutable host-neutral view models, schema-derived capability detail, and the physical-input → semantic-event → `AppState::apply` → projection path.

Canonical resources affected are `valueObject.Control.TopLevelContext`, `valueObject.Control.InteractionState`, `valueObject.Control.AppEvent`, `valueObject.Control.PatchPageProjection`, `valueObject.Control.TextProjection`, `aggregate.Control.AppState`, `domainService.Control.StateProjector`, `applicationService.Control.AppLoop`, `valueObject.Shell.WindowInput`, `applicationService.Shell.KeyboardInputTranslator`, `adapter.EframeTextWindow`, and the Testing/verification resources that derive the current surface.

## Goals / Non-Goals

**Goals:**

- Make `1` select MIXER and `2` select PATCH through the canonical semantic event path.
- Keep the current diagnostic wall as the transitional MIXER projection and preserve its prior selection across page switches.
- Own PATCH focus by stable `PatchId` in reducer state and project the first installed Patch deterministically.
- Project Patch identity, MIDI channel, active engine, complete installed engine choices, canonical ADSR, and the active descriptor/config without engine-specific page branches.
- Keep every PATCH row read-only and prove a context event is generation-coherent but audio- and graph-neutral.
- Extend exact schema, GUI, demo, and named acceptance evidence without weakening any Phase 2 gate.

**Non-Goals:**

- Engine replacement, background preparation, pending/error state, graph publication, or acknowledgement.
- Editing ADSR, Scalars, Structural values, MIDI channel, or Patch identity from PATCH.
- SoundFont preset discovery or selection, asset browsing, per-Patch effects, modulation, persistence, or the Figma-derived graphical interface.
- A third top-level context, adapter-owned tabs, mouse interaction, or changes to DSP, mixer, real-time transports, or dependencies.

## Decisions

### 1. Put context and per-context focus in one canonical InteractionState

Add `TopLevelContext::{Patch, Mixer}` and replace `AppState`'s flat selection field with `InteractionState { context, mixer_selection, patch_focus }`. Startup defaults to MIXER. Successful Patch installation initializes `patch_focus` to the first installed stable `PatchId`; it is `None` before installation. Direct context selection retains both focus values. Selecting PATCH without an installed focus uses the existing `NoPatchesInstalled` rejection, while selecting the already active context is an idempotent accepted event whose only logical change is generation.

MIXER continues to apply existing navigation and adjustment rules against `mixer_selection`. PATCH is intentionally read-only: Navigate and Adjust return `ActionUnavailableInContext` without changing generation or state. This prevents invisible edits to a hidden MIXER selection and provides an explicit recovery path for later input.

Alternative considered: infer PATCH focus from the current vector index in the MIXER selection. Rejected because GLOBAL selection loses Patch identity and because the durable design requires semantic IDs rather than widget or collection positions.

Alternative considered: store the selected page in `EframeTextWindow`. Rejected because deterministic navigation, serialization, event records, and future controller adapters must observe the same canonical reducer state.

### 2. Add a host-neutral PatchPageProjection, then render it through the existing text shell

`StateProjector` resolves `InteractionState.patch_focus` against canonical Patches and creates one immutable `PatchPageProjection`. The projection includes:

- stable Patch identity, name, and MIDI channel;
- active CapabilityId/label and every installed registry entry as an ordered read-only choice;
- four envelope rows from the canonical `VoiceEnvelope` surface descriptor;
- active descriptor sections and parameter rows in exact descriptor order, including stable IDs, labels, kinds, update classes, values/assets, units, dependency results, and `editable = false`.

Envelope descriptor metadata gains presentation labels and units so the page does not own a second ADSR field list. Capability rows are produced by one generic descriptor/config walk; presentation code may not match SoundFont or Braids identities or their field names.

`TextProjection` gains an explicit context and renders exactly one active projection in the existing vertical-scroll shell. MIXER retains the complete diagnostic content and selection semantics, with the direct page bindings added to its header. PATCH is a deterministic text rendering of `PatchPageProjection`, with one stable selected identity line for scroll compatibility. `StateTree` contains InteractionState and an optional PatchPageProjection that is present exactly in PATCH.

Alternative considered: format the Patch page directly from `AppState` inside the eframe adapter. Rejected because it would duplicate projection behavior and prevent a later graphical adapter from consuming the same host-neutral model.

Alternative considered: add only a second string body. Rejected because exact semantic rows, registry choices, metadata, and stable focus would then be recoverable only by parsing presentation text.

### 3. Keep page selection on the one-way publication path without creating audio work

Extend the closed semantic event with `SelectContext(TopLevelContext)` and its production-owned surface descriptor. Extend normalized window keys with Digit1/Digit2; key-down translates to MIXER/PATCH selection and key-up is inert, independent of K state. The eframe adapter only normalizes these keys and dispatches the resulting event.

`AppLoop` keeps its existing order: reduce, commit, serialize/project, publish the latest ParameterSnapshot, then enqueue an optional discrete command. A context event advances the accepted generation and therefore publishes a snapshot with the new generation, but its Patch ordering, parameter values, Scalar layouts, and GraphRevision are exact copies of the prior snapshot. It emits no AudioCommand and never touches the structural boundary. Acceptance compares production renders from identical prepared state using the before/after values and requires sample identity.

Alternative considered: skip parameter publication for context events. Rejected because it would create a second accepted-event projection policy and break the existing invariant that logical projections share one accepted generation.

### 4. Treat the schema expansion as an explicit versioned migration

The closed input surface grows from 13 to 17 normalized values, the AppEvent surface from four to five variants, top-level context coverage becomes exactly two, and rejection coverage grows from ten to eleven variants. Serialized state, StateTree, TextProjection, EventRecord payload discovery, and leaf descriptors add interaction/context/page fields and increment their schema versions where versioned.

The exhaustive headless scene exercises both page keys, exact context events, both production capability projections, PATCH rejection/recovery, and return to the captured MIXER baseline. The existing live scene remains in default MIXER because this slice adds no editable PATCH controls; it must still tolerate and serialize the expanded projection schema. A named `patch_page_projection` integration target proves the focused SoundFont and Braids shapes, eframe callback path, audio-boundary effects, and sample-identical render consequence before printing its marker.

Alternative considered: hard-code the expected SoundFont and Braids row names in the new test. Rejected because a test-owned duplicate list could pass after the production schema drifts.

## Risks / Trade-offs

- [Public enum and serialized-schema expansion breaks exhaustive consumers] → Update all matches, typed surface descriptors, schema versions, discovery checks, and exact tests in the same atomic change.
- [A hidden MIXER adjustment could occur while PATCH is visible] → Gate Navigate/Adjust in `AppState::apply` by context and require `ActionUnavailableInContext` with unchanged state.
- [Page projection drifts from capability metadata] → Generate rows only from the installed registry, active descriptor/config, and canonical envelope descriptor; compare exact production-derived sets in both directions.
- [Context-only events accidentally trigger audio or structural work] → Assert no command/graph publication, exact parameter values and revision, and sample-identical output from identical prepared state.
- [Generation-only MIDI optimization accidentally reuses the wrong context body] → Include InteractionState in snapshot identity and add eager-versus-deferred equivalence cases for both contexts while keeping MIDI's unchanged-context sharing path.
- [The transitional MIXER name overstates the diagnostic wall] → Keep its content intentionally unchanged for compatibility and document that the Phase 5 interface replaces this adapter; do not create a third “diagnostic” context.

## Migration Plan

1. Add context/interaction types, semantic input/event variants, descriptor entries, typed rejection, and versioned serialization while defaulting existing construction to MIXER.
2. Migrate `AppState` selection access through InteractionState, initialize stable Patch focus during installation, and gate context-specific actions.
3. Add envelope presentation metadata, PatchPageProjection, context-aware TextProjection/StateTree, and AppLoop accessors/publication invariants.
4. Normalize Digit1/Digit2 in eframe and the shared translator; keep the window contract immutable and page-agnostic.
5. Expand demo/schema/GUI fixtures and add the named production-path acceptance target, then run every declared project gate.

There is no persisted user-state migration in this slice. Rollback is the whole change: restore the prior enum/schema versions and single MIXER projection together; no audio asset or prepared graph data needs conversion.
