# Working in Crest Synth

Read `DESIGN.md` before changing product behavior or architecture. It is the master design; its linked Figma file is the visual and interaction reference.

The architecture spec in `.kittify/architecture/` (terse YAML: intent, bounded contexts, proof model) is the executable implementation declaration. Read it via `spec-kitty context architecture` and reconcile it to `DESIGN.md` before planning; plans must record an `## Architecture Reconciliation` section. A deliberately narrow implementation slice must not redefine the product. Use Spec Kitty missions (`/spec-kitty.specify` → plan → tasks → implement → review → accept) for all changes; `spec-kitty accept` runs the declared deterministic validations and both acceptance layers must pass. Do not invoke legacy crest-spec, CUE, or OpenSpec tooling (retired to `archive/`).

Preserve these boundaries:

- physical input → semantic action/event → `AppState::apply` → view/audio projections;
- a hard real-time callback with bounded, preallocated work and no allocation, locking, blocking, I/O, logging, panic, or destruction;
- separate RT transports for discrete events, latest scalar snapshots, and prepared structural graph changes;
- SoundFont and other engines behind capability ports, with no silent fallback;
- a schema-driven controller UI with PATCH and MIXER as the only top-level contexts;
- one canonical type per concept and thin UI, MIDI, device, controller, asset, and persistence adapters;
- measured, falsifiable proofs using the production reducer and render path.

Figma example engines, effects, patches, and values are design fixtures, not an exhaustive feature list. Put durable decisions in `DESIGN.md`; use issues or commits for temporary plans and handoffs.

<!-- spec-kitty:orientation -->
**Spec Kitty v3.2.6** — project: unknown (healthy)

Two usage patterns:
- **Full mission** (spec → plan → tasks → implement → review → merge):
  trigger: "spec out", "create a mission", "write a spec", "plan this"
  → run `/spec-kitty.specify`
- **Lightweight dispatch** (ad-hoc fix, question, or advice — no mission created):
  trigger: "hey spec kitty", "use spec kitty to", "spec kitty <anything>"
  → **ALWAYS run `spec-kitty dispatch "<request verbatim>"` — do NOT answer directly.**
  If you know the right profile, pass it to skip routing:
  `spec-kitty dispatch "<request verbatim>" --profile <profile-id>`
  Reason: `spec-kitty dispatch` loads governance context, routes the request,
  and opens the Op. Skipping it produces ungoverned, untracked responses.
  After finishing the work, close the Op with the command printed in the capsule
  (`spec-kitty profile-invocation complete --invocation-id <id> --outcome <done|failed|abandoned>`).
<!-- /spec-kitty:orientation -->
