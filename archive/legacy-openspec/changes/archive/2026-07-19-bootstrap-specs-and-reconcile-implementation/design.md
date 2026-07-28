## Context

crest-synth has an evaluated CUE architecture covering seven bounded contexts, six adapters, six observable capabilities, three required goals, implementation assets, validations, and behavioral witnesses. The repository also contains a substantial Rust implementation whose modules closely resemble those declared resources, but OpenSpec has no main capability specs and the implementation has not yet been reconciled against every CUE contract and completion check.

The evaluated `project` value and relationship index are authoritative. The existing implementation is evidence to inspect, not a substitute for that architecture and not disposable greenfield scaffolding. Reconciliation must preserve user-authored work, respect domain/application/infrastructure dependency direction, and distinguish an implementation defect from a genuine request to change architectural intent.

The relevant bounded-context flow is:

```text
Testing/Shell input
        |
        v
Control AppEvent -> AppState -> StateTree/TextProjection/ParameterSnapshot
        |                                      |
        +---------- AudioCommand --------------+
                                               v
                                    RealTime AudioBoundary
                                               |
                                               v
                                  Synth stems -> Mixer -> audio
```

## Goals / Non-Goals

**Goals:**

- Establish one OpenSpec behavior contract for each CUE capability.
- Determine which declared resources, assets, validations, and witnesses already conform and change only what does not.
- Preserve exact traceability from the three required goals through capabilities and requirements to measured evidence.
- Finish with every declared project check passing and every positive and controlled-negative witness producing its required observation.
- Leave a clean OpenSpec baseline that can be archived into main specs and used for later changes.

**Non-Goals:**

- Rewriting code that already conforms to its contract.
- Changing the CUE architecture, context map, dependency direction, or canonical resource ownership.
- Adding sequencing, transport, recording, alternate synthesis engines, extra effects, elaborate UI, persistence, live MIDI hardware, networking, database, or async-runtime behavior.
- Replacing measured behavioral evidence with construction checks, debug text, or self-reported success markers.

## Decisions

### 1. Treat evaluated CUE as the reconciliation authority

All audits and repairs will start from the evaluated CUE `project` and relationship index. A mismatch will be resolved in favor of CUE unless the desired product behavior itself has changed, in which case work stops for an explicit CUE amendment and coherent artifact update.

Alternative considered: infer the specification from current Rust behavior. Rejected because this would legitimize accidental drift, erase declared non-goals, and disconnect implementation from the existing acceptance and evidence model.

### 2. Audit before editing and preserve conforming implementation

Each capability slice will first map its canonical resources and declared assets to the existing files and executable checks. Repairs will be limited to missing behavior, boundary violations, incorrect observations, or incomplete proof. Passing behavior will not be reimplemented merely to match a hypothetical generated structure.

Alternative considered: regenerate the whole project from the architecture. Rejected because the implementation is already substantial and a rewrite would increase regression risk without improving the contract.

### 3. Organize behavior contracts by the six CUE capabilities

The OpenSpec baseline will contain `soundfont-audio`, `automatic-test-midi`, `global-mix`, `one-way-parameter-control`, `realtime-execution`, and `observable-demo-scene`. These are observable capability boundaries with explicit goal and evidence links; Rust modules and CUE contexts remain architectural implementation boundaries rather than becoming duplicate OpenSpec specs.

Alternative considered: create one spec per Rust module or bounded context. Rejected because modules are not independently observable user contracts and would fragment acceptance scenarios across implementation details.

### 4. Reconcile through existing ports and dependency direction

Sound generation, MIDI input, audio output, text rendering, effects processing, and the real-time boundary remain port-owned behaviors with replaceable infrastructure adapters. Domain code remains independent of application and infrastructure code; application services depend on domain behavior; infrastructure may depend on both. The CUE context map governs cross-context translation.

Alternative considered: close gaps with direct adapter-to-domain or cross-context calls. Rejected because expedient coupling would violate the declared anti-corruption and layer boundaries and make later adapter replacement unsafe.

### 5. Use evidence-backed completion rather than file presence

Local validations will be used while repairing a resource, followed by the complete project gates: format, clippy, tests, smoke, demo scene, schema surface, headless egui context, and mutation harness. Behavioral witnesses must assert their typed observations, and every controlled mutant must fail for its own causal seam while its matching healthy case passes.

Alternative considered: regard compilation, unit tests, or acceptance marker strings as completion. Rejected because those checks cannot prove correct routing, exact projections, nonzero audio, hard-real-time behavior, exhaustive coverage, or mutation resistance.

### 6. Bootstrap main OpenSpec specs through normal archive flow

The six files in this change are delta specs for new capabilities. They will remain change-local during reconciliation and become the initial main OpenSpec specifications only after implementation and validation are complete and the change is archived.

Alternative considered: write directly to `openspec/specs/`. Rejected because it would bypass change review and disconnect the baseline contracts from the work that establishes them.

## Risks / Trade-offs

- **The broad initial baseline can hide small gaps among already-complete areas** -> Begin with a capability/resource/check matrix and record concrete failures before editing.
- **Existing tests may share the same defect as the implementation** -> Require exact typed observations plus the six independent controlled-negative mutation cases.
- **The fixed SoundFont is an environmental prerequisite** -> Fail clearly when `./sf2/HiDef.sf2` is absent or invalid and run audio witnesses only with the declared fixture available.
- **Hard-real-time defects can be introduced by otherwise harmless repairs** -> Keep callback data fixed-capacity and preallocated, and rerun allocation and boundary validations after any RealTime, Synth, or Mixer change.
- **Current user work can be obscured by a large reconciliation diff** -> Inspect the dirty worktree before every edit, patch narrowly, and never overwrite or revert unrelated changes.
- **A real architecture correction may surface during apply** -> Do not silently adapt the plan; update the relevant CUE sources and reconcile the proposal, specs, design, and tasks before continuing.

## Migration Plan

1. Inventory existing files and run the cheapest declared checks to establish a concrete pass/fail baseline.
2. Reconcile the implementation capability by capability, following canonical resource relationships and repairing only demonstrated gaps.
3. Run each affected resource validation immediately after its repair, then run the complete project checks.
4. Execute every healthy and controlled-negative witness and confirm the required structured observations and exit behavior.
5. Review the final implementation and evidence against the six OpenSpec contracts, then archive the change to establish the main specs.

Rollback consists of reverting only the reconciliation edits associated with a failing capability while retaining the pre-change implementation and CUE architecture. No data migration or deployed compatibility transition is required because the product has no persistence or external service contract.
