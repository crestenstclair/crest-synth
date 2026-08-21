# Working in Crest Synth

Read `DESIGN.md` before changing product behavior or architecture. It is the
master as-built design and status reference; its linked Figma file is the visual
and interaction reference.

The current UI is a functional blockout, not a faithful Figma implementation.
Do not infer visual or workflow acceptance from the presence of surfaces,
components, or passing reducer/projection tests.

Preserve these invariants:

- physical input → semantic action/event → `AppState::apply` → view/audio
  projections;
- `AppState::apply` is the only product-state mutation path;
- a hard real-time callback with bounded, preallocated work and no allocation,
  deallocation, locking, blocking, I/O, logging, formatting, panic, unwinding,
  or destruction;
- separate RT transports for ordered discrete events, latest compatible scalar
  snapshots, prepared structural graph changes, and decimated observations;
- complete graph preparation off callback, block-boundary activation, and
  off-thread retirement/destruction;
- SoundFont, Sample, Braids, effects, assets, and devices behind capability
  ports, with typed failure and no silent fallback or substitution;
- a schema-driven controller UI with PATCH and MIXER as the only top-level
  contexts;
- exactly one stable semantic focus, with stable return identity across
  navigation, reprojection, and density changes;
- one canonical public type per concept and thin UI, MIDI, device, controller,
  asset, persistence, and serialization adapters;
- explicit status and control states in text or shape as well as color;
- measured, falsifiable proof through the production reducer, projector,
  worker, graph, and render path.

Figma example engines, effects, patches, files, values, and counts are design
fixtures, not an exhaustive feature list. Installed registries determine
production content.

Do not invoke or reintroduce the retired OpenSpec tooling, CUE DSL, roadmap,
planning kit, generated specification system, or a competing design document.
Put durable architecture and product decisions in `DESIGN.md`; use issues and
commits for temporary plans, sequencing, acceptance notes, and handoffs.
