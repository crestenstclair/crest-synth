# Quickstart: Expandable Effects and Bus Topology

## Orientation

Read in this order before touching code:

1. `spec.md` — what the mission delivers and, in "Cross-cutting rename", what it retires
2. `plan.md` — Architecture Reconciliation and the Implementation Concern Map
3. `research.md` — why the design is generic rather than widened
4. `data-model.md` and `contracts/` — the shapes and obligations

## The one rule

**No name-enumerated effect or routing identity.** If you are about to write a type,
variant, field, or descriptor entry named after a specific effect or bus — `ReverbSend`,
`DelayReturn`, `reverb_input` — stop. Address it by index into a descriptor-driven
array instead.

The single exception is `MasterGainDb`, which is genuinely global rather than
per-effect.

This rule is enforced by a project check, not by review. It exists because the
expansion to three slots and eight returns was declared in DESIGN.md before the
closed code was written, and the closed code shipped anyway.

## Before the first implementation work package

Two gates block progress if skipped:

1. **Architecture reconciliation (IC-07, C-009).** The architecture spec declares
   this mission a non-goal. Narrow the three declarations in
   `.kittify/architecture/project.yaml`, restate the superseded `DESIGN.md`
   decisions, and add the new capability, goal, requirements, evidence, validation,
   witness, and invariant. Reload the spec afterwards. This is planning work that
   must land before or alongside the first WP — never after.

2. **Bulk-edit map.** `occurrence_map.yaml` is present and schema-valid. Read it
   before making cross-file changes. `cli_commands` and `logs_telemetry` are
   `do_not_change`: every earlier phase demo target and every existing checkpoint
   identity must stay byte-identical, or retained evidence stops being comparable.

## Suggested build order

Follows the concern dependencies in `plan.md`:

1. IC-01 registry generalization (reverb and delay become registry entries)
2. IC-02 slots and IC-03 buses — independent of each other, both depend on IC-01
3. IC-04 widened snapshot — the highest real-time risk; measure, do not assume
4. IC-05 lifecycle, then IC-06 focus and projection
5. IC-08 the retained scene last, since it proves the rest

IC-07 runs first or alongside, never last.

## Verification

```bash
make fmt-check && make lint          # format and clippy
make test                            # cargo test --all-targets
make demo                            # deterministic scene
make demo-live-effects-and-buses     # NEW retained scene — the phase gate
```

Earlier phase scenes must all still run:

```bash
make demo-live-graphical-shell
make demo-live-semantic-view-model
make demo-live-sixteen-track-mixer-routing
```

## The phase gate

Per `ROADMAP.md`, this phase is not complete until `make demo-live-effects-and-buses`
has been run by the implementer with a real window, physical audio output, and the
real MIDI fixture — and the visible, audible, structured report covers every declared
behavior. A headless, silent, mocked, or dry-run substitute does not satisfy it. The
scene must end with semantic all-notes-off, zero active notes, window close, stream
release, worker shutdown, graph collection, and normal parent-process exit.

`make demo-live` advances to this scene as the newest cumulative one.

## Traps

- `PreparedPostEffectRack::matches_parameters` proves exact Patch/slot correspondence. When widening it, keep it exact — a permissive version silently accepts mismatched layouts.
- Send position at `mix_engine.rs:164-167` is post-fader and post-gate. Generalizing to eight sends must not move it; a muted track feeding a wet return is a real regression that no unit test currently catches by accident.
- An unoccupied return must contribute silence, not pass-through.
- Clearing a slot while its parameters hold focus must resolve focus deterministically.
- Reverb and delay must sound the same after moving behind the generic boundary. That is a migration, not a redesign.
