# Working in Crest Synth

Read `DESIGN.md` before changing product behavior or architecture. It is the master design; its linked Figma file is the visual and interaction reference.

The CUE files in `spec/` are executable implementation declarations. Reconcile them to `DESIGN.md` before generation. A deliberately narrow implementation slice must not redefine the product. Generate governed Rust through crest-spec instead of hand-writing competing implementation paths.

Preserve these boundaries:

- physical input → semantic action/event → `AppState::apply` → view/audio projections;
- a hard real-time callback with bounded, preallocated work and no allocation, locking, blocking, I/O, logging, panic, or destruction;
- separate RT transports for discrete events, latest scalar snapshots, and prepared structural graph changes;
- SoundFont and other engines behind capability ports, with no silent fallback;
- a schema-driven controller UI with PATCH and MIXER as the only top-level contexts;
- one canonical type per concept and thin UI, MIDI, device, controller, asset, and persistence adapters;
- measured, falsifiable proofs using the production reducer and render path.

Figma example engines, effects, patches, and values are design fixtures, not an exhaustive feature list. Put durable decisions in `DESIGN.md`; use issues or commits for temporary plans and handoffs.
