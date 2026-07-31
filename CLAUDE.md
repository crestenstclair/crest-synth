# Working in Crest Synth

Read `DESIGN.md` before changing product behavior or architecture. It is the master design; its linked Figma file is the visual and interaction reference.

The **crest-spec** at `.kittify/crest-spec/` (terse YAML: intent, bounded contexts, resources, assets, proof model) is the bedrock — the executable declaration of what the system is and the single source of implementation intent. Planning, work packages, code, and proof derive FROM it; it is never reconciled to other documents after the fact. A mission that adds, changes, or retires structure authors the crest-spec FIRST (`/spec-kitty.crest-spec`, after specify, before plan); `plan.md` derives from it and records a `## Crest-Spec Derivation` section; work packages derive from its `assets[]`. Read it via `spec-kitty crest-spec context`; validate with `spec-kitty crest-spec doctor`. `DESIGN.md` remains the product authority for what the product should be — when it and the crest-spec disagree, resolve that deliberately during crest-spec authoring; never silently edit the crest-spec to permit code that was already planned, and never produce `data-model.md`/`contracts/` (they fork the canonical resources and fail acceptance). A deliberately narrow implementation slice must not redefine the product. Use Spec Kitty missions (`/spec-kitty.specify` → crest-spec → plan → tasks → implement → review → accept) for all changes; `spec-kitty accept` runs the declared deterministic validations and both acceptance layers must pass. Do not invoke the retired CUE DSL or OpenSpec tooling (immutable snapshots in `archive/legacy-cue-dsl/`, `archive/legacy-cue-workflows/`, `archive/legacy-openspec/`) — the crest-spec means exactly one thing: the live DDD declaration at `.kittify/crest-spec/`.

Preserve these boundaries:

- physical input → semantic action/event → `AppState::apply` → view/audio projections;
- a hard real-time callback with bounded, preallocated work and no allocation, locking, blocking, I/O, logging, panic, or destruction;
- separate RT transports for discrete events, latest scalar snapshots, and prepared structural graph changes;
- SoundFont and other engines behind capability ports, with no silent fallback;
- a schema-driven controller UI with PATCH and MIXER as the only top-level contexts;
- one canonical type per concept and thin UI, MIDI, device, controller, asset, and persistence adapters;
- measured, falsifiable proofs using the production reducer and render path.

Figma example engines, effects, patches, and values are design fixtures, not an exhaustive feature list. Put durable decisions in `DESIGN.md`; use issues or commits for temporary plans and handoffs.
