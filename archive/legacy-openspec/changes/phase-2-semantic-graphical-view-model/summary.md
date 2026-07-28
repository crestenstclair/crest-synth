# Change Summary

## Outcome

- **Problem:** Phase 1 renders the authored shell, but positional focus, key-state interpretation, page projections, and hints do not form one safe multi-layout interaction contract.
- **Result:** PATCH and MIXER gain one immutable semantic model with stable focus, explicit Navigate/Adjust state, exact actions, side-surface returns, typed status/errors, and deterministic recovery.

## Change Outline

- **Adds:** `SemanticAction`, four `SurfaceId` values, stable `FocusPath`/`ReturnPath`, typed control/surface view models, one shared resolver, and `SemanticGraphicalViewModel`.
- **Changes:** `InteractionState`, `AppEvent`, `AppLoop`, `StateProjector`, shell/window input, schema surfaces, demos, and live evidence use the canonical action/path contract.
- **Removes:** Positional MIXER identity, parallel PATCH focus authority, adapter-inferred mode/action state, and `make demo-live` pointing at the Phase 1 scene.

## System Impact

- **Capabilities:** Adds `semantic-graphical-view-model`; modifies `one-way-parameter-control`, `graphical-application-shell`, `schema-driven-patch-page`, and `live-observable-demo`.
- **Architecture:** Adds `goal.use_semantic_graphical_view_model`, `valueObject.Control.{SemanticAction,FocusPath,ReturnPath,ValidAction,SemanticGraphicalViewModel}`, and `valueObject.Synth.PatchEditableTarget`; extends `aggregate.Control.AppState`, `domainService.Control.StateProjector`, and `applicationService.Control.AppLoop` relationships.
- **Interfaces/data:** The window emits actions instead of reducer events; exact event/state/shell/tree schemas advance. Audio transports, prepared graphs, DSP, eframe/egui, and `egui_extras` remain unchanged.

## Delivery

- **Implementation:** Build identities/resolver, migrate reducer/action flow, add semantic projection/schemas, adapt passive eframe rendering, then extend deterministic/live composition.
- **Validation:** Require all build/lint/test gates, a production-path semantic integration target, exact schema and responsive egui proofs, cumulative demos, and physical `make demo-live-semantic-view-model` teardown evidence.

## Risks and Decisions

- **Key decisions:** One resolver governs focus and valid actions; recovery uses canonical order with next-before-previous ties; Utility/Inspector are read-only roots; only Navigate/Adjust are reachable.
- **Risks/open questions:** Schema migration breadth, missed Edit release, resolver drift, and environment-dependent physical proof are mitigated by exact schemas, focus-loss normalization, shared logic, and a mandatory separate live gate; no open questions remain.
