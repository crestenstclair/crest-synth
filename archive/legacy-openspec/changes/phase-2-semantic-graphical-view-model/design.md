## Context

Phase 1 established a production eframe/egui shell, but its interaction contract is still split between positional reducer fields, key-state interpretation, page-specific projections, and footer hints. That split cannot safely support later PATCH/MIXER components or alternative responsive compositions: a renderer can see what is drawn, but it cannot ask one canonical value what is focused, which actions are valid, why a control is unavailable, or where Return leads.

`DESIGN.md` requires physical input to become a semantic action/event before `AppState::apply`, with immutable view and audio projections derived only after acceptance. It also requires stable semantic focus, explicit Navigate/Adjust/Modal/MultiSelect modes, PATCH Main and Utility plus MIXER Main and Inspector surfaces, exact return behavior, descriptor-driven controls, and passive UI. The evaluated CUE architecture fixes the Phase 2 resource model and proof boundary. Existing hard-real-time transports, capability ports, prepared graph ownership, eframe/egui plus `egui_extras`, and the five-region shell must remain intact.

## Goals / Non-Goals

**Goals:**

- Establish one canonical, layout-neutral `SemanticGraphicalViewModel` for both PATCH and MIXER.
- Replace positional interaction state with stable reducer-owned focus, remembered roots, mode, and return state.
- Insert a closed `SemanticAction` boundary between physical/passive input and reducer events.
- Derive controls, status, errors, and exact valid actions from canonical state and typed descriptors.
- Recover focus deterministically after committed schema changes while making viewport changes focus-neutral.
- Prove the model through the production reducer/projector/render path and a cumulative physical live scene.

**Non-Goals:**

- Replacing eframe/egui, adding another GUI runtime, or publishing the Phase 4 component/token library.
- Making Utility or Inspector functionally editable before their later PATCH/MIXER phases.
- Adding effects, sends, returns, routing, or changing the later Phase 3 limits of three effect slots and eight bus returns.
- Implementing Modal or MultiSelect workflows, the Phase 7 Sample Browser, or its hover Start preview binding.
- Changing persistence, capability preparation, DSP, real-time transport, or callback behavior.

## Decisions

### 1. Insert `SemanticAction` without weakening the event/reducer boundary

`KeyboardInputTranslator` and future passive controls emit the closed union `SelectContext | Navigate | Adjust | SetInteractionMode | EnterSurface | Return`. `AppLoop.dispatchAction` maps each accepted user intent to exactly one corresponding `AppEvent`, then calls `AppState::apply`. MIDI, worker outcomes, graph acknowledgements, startup, and other system inputs continue to enter as typed `AppEvent` values directly.

```text
physical input / passive component
              │
              ▼
       SemanticAction
              │ AppLoop.dispatchAction
              ▼
          AppEvent ───────── system, MIDI, worker events
              │
              ▼
       AppState::apply
              │
              └─ accepted snapshot → view/audio projections
```

This preserves one mutation authority while preventing raw keys, widget callbacks, or adapter-local mode state from entering the domain. Mapping widgets directly to `AppEvent` was rejected because it would make the public UI contract depend on reducer internals; moving system events into `SemanticAction` was rejected because they are not user intents.

### 2. Make one stable `FocusPath` the interaction authority

`InteractionState` owns one active `FocusPath`, remembered PATCH and MIXER main-root paths, one `InteractionMode`, and at most one `ReturnPath`. A path combines `TopLevelContext`, one of `PatchMain | PatchUtility | MixerMain | MixerInspector`, stable Patch/capability/control identities where applicable, and an optional modal identity reserved for later work. MIXER targets use `MixerControlId` and the canonical descriptor-derived `PatchEditableTarget` rather than row/column indices.

Selecting a context restores its remembered main path. Entering Utility or Inspector records the exact main origin; Return atomically restores that origin, clears the return path, and restores Navigate mode. Paths never contain geometry, widget IDs, labels, values, or collection positions. Keeping the old PATCH focus and MIXER indices beside a new path was rejected because two authorities could diverge.

### 3. Project one host-neutral semantic model and embed it in the shell

`StateProjector` derives `SemanticGraphicalViewModel` from the same accepted snapshot as `GraphicalShellProjection`, `TextProjection`, `StateTree`, and `ParameterSnapshot`. It carries generation/hash, context, active surface, focus, mode, optional return path, ordered valid actions, typed lifecycle status, typed errors, and four semantic surface models. The shell embeds this value and derives its status/footer presentation from it; the retained diagnostic remains nested read-only content.

PATCH main controls derive from `PatchControlId` plus instrument/effect descriptors. MIXER main controls derive from stable Patch/global parameter descriptors. Utility and Inspector expose a focusable `SurfaceRoot` plus canonical read-only summaries so their identities and return behavior are real without claiming later functional controls. No semantic model contains egui types, rectangles, density rules, callbacks, prepared objects, devices, or audio buffers.

Page-specific models remain compatibility projections during migration, not interaction authorities. Letting each page or renderer compute its own focus/status/action data was rejected because responsive layouts and hosts would then disagree.

### 4. Use one pure resolver for focusability and valid actions

One descriptor-driven resolver defines ordered controls, path resolution, focus eligibility, and action acceptance. The projector uses it to expose a duplicate-free ordered `ValidAction` set; `AppState` uses the same rules before reducing the corresponding event. Footer hints are presentation metadata attached to those valid actions, never a second availability calculation.

Availability includes context, surface, current mode, value bounds, dependencies, and structural lifecycle state. It reports what the reducer can accept now but never predicts worker completion. A temporarily non-editable Engine can retain focus while its adjustment action is absent. Duplicated UI predicates or speculative worker-aware availability were rejected because they can drift from reducer truth.

### 5. Repair semantic paths only after committed schema changes

Viewport, density, wrapping, scrolling, and rectangle changes never alter semantic focus. After a committed descriptor/dependency change, the reducer validates active, remembered, and return paths. A still-valid stable identity is retained. Otherwise it searches the old surface order outward by distance, preferring the next surviving visible focusable sibling over the previous sibling on a tie. The projector and widget tree never repair state.

This policy is deterministic across hosts and preserves user locality without relying on indices. Resetting to the first control was rejected because it loses context; retaining an invalid path was rejected because the model could no longer satisfy its single-focus invariant.

### 6. Admit only the interaction states Phase 2 can prove

`InteractionMode` names Navigate, Adjust, Modal, and MultiSelect so later work has one canonical seam, but only Navigate and Adjust are reachable in Phase 2. Edit-key down emits `SetInteractionMode(Adjust)`; key up and physical focus loss emit `SetInteractionMode(Navigate)`. Directional input is then interpreted through the current mode. Modal and MultiSelect have no valid entry action until later workflows specify trapping, exit, and recovery.

PATCH Right from Main enters Utility and Left/Return restores the exact origin. Passive controls can emit `EnterSurface` for either Utility or Inspector, and both side surfaces expose Return. MIXER directional navigation remains available for its existing grid, so Inspector entry is not overloaded onto a conflicting bare direction.

### 7. Require deterministic and physical evidence at the production seams

`tests/semantic_graphical_view_model.rs` drives real `WindowInput` and passive-view actions through `AppLoop`, the production reducer/projector, and real egui frames at 1920×1080 and 1280×800. It proves exact action/event mapping, both contexts, four surfaces, two modes, two return round trips, descriptor polymorphism, typed failure/recovery, next-before-previous focus repair, schema exactness, layout invariance, and audio-neutral navigation.

`make demo-live-semantic-view-model` is the newest cumulative release-mode witness. It retains the physical window, device, fixture, scalar/structural/audio evidence, and teardown obligations, while adding measured semantic traversal and a healthy explicitly empty error set. `make demo-live-graphical-shell` remains stable and `make demo-live` aliases the new scene. Planned labels or self-reported markers cannot substitute for correlated post-paint, reducer, audio, cleanup, and process-exit evidence.

## Risks / Trade-offs

- **[Interaction-state migration touches many schemas and tests]** → Introduce canonical types and resolver first, migrate reducer/projector consumers together, and reject any temporary dual authority in exact schema checks.
- **[Projection and reducer availability could drift]** → Share one pure resolver and test every projected action both positively and at boundaries/blocked lifecycle states.
- **[Focus repair could vary with layout or collection representation]** → Define recovery only over canonical descriptor order and assert identical paths across both reference viewports.
- **[Edit release can be missed when the window loses focus]** → Normalize physical focus loss to Navigate and cover the transition through the production input callback.
- **[Side-surface anchors could be mistaken for final controls]** → Keep summaries read-only, name `SurfaceRoot` explicitly, and reserve functional Utility/Inspector controls for their roadmap phases.
- **[The semantic projection is larger than the Phase 1 shell value]** → Keep it immutable, descriptor-derived, and shared by generation; measure no new work on the audio callback.
- **[Physical evidence depends on host resources]** → Keep deterministic acceptance mandatory and require the separate live gate on a supported system without skips, fallbacks, or silent substitutes.

## Migration Plan

1. Add the stable semantic identity/action/mode/path/view-model types and the shared descriptor resolver.
2. Replace positional `InteractionState`, extend `AppEvent`, and implement action dispatch, surface transitions, mode transitions, and path repair in `AppState::apply`.
3. Derive and serialize `SemanticGraphicalViewModel`, embed it in the shell/StateTree, and migrate page/text/footer consumers to canonical fields.
4. Change the window/input port to emit `SemanticAction`, render semantic state passively, and preserve responsive geometry plus existing diagnostics.
5. Extend deterministic demos, schema/exhaustive tests, the new named acceptance target, live runner/report, CLI modes, and Make aliases.
6. Run formatting, build, lint, unit/integration/schema/demo gates, then complete the physical semantic-view-model witness and teardown check.

Rollback is the inverse control/view migration: restore the prior event sink and positional interaction schema, remove the semantic projection and new targets, and retain the Phase 1 shell. No persisted data or audio graph format changes, so no data migration is required.
