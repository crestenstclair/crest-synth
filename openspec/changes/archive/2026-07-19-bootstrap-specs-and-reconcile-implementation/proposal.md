## Why

crest-synth already has a substantial Rust implementation and an authoritative evaluated CUE architecture, but it has no OpenSpec capability specifications or active change history connecting the two. This change bootstraps those behavioral contracts and reconciles only the implementation gaps that prevent the required goals and evidence-backed completion checks from passing.

## What Changes

- Create OpenSpec specifications for all six observable capabilities declared by the CUE architecture, preserving their goal, requirement, acceptance, invariant, and evidence traceability.
- Audit the existing Rust implementation against the evaluated CUE resources, boundaries, dependency directions, assets, validations, and witnesses instead of treating the project as greenfield work.
- Preserve conforming implementation and repair only missing, contradictory, or insufficiently verified behavior.
- Require measured proof from the declared project checks and behavioral witnesses, including controlled negative mutation cases; success markers without the required observations are insufficient.
- Keep the CUE sources unchanged unless reconciliation reveals that architectural intent itself must change; any such change requires an explicit CUE edit rather than a silent planning workaround.
- Preserve the declared non-goals: no sequencer or transport model, alternate synthesis engine, additional effects, elaborate GUI, persistence, physical MIDI adapter, networking, database, or async runtime.

## Capabilities

### New Capabilities

- `soundfont-audio`: Load the fixed HiDef.sf2 bank once, configure distinct instrument Patches, and produce bounded stereo SoundFont audio.
- `automatic-test-midi`: Start the fixed Corridors of Time MIDI fixture automatically, discover instrument parts, and route them through distinct Patch channels without adding sequencing concepts to the domain.
- `global-mix`: Mix independent Patch stems with gain, pan, and sends through exactly one shared reverb and one shared delay.
- `one-way-parameter-control`: Translate keyboard input into events, reduce accepted state, serialize and project that state, and publish matching parameters and commands through a single one-way control loop.
- `realtime-execution`: Render audio through fixed-capacity lock-free boundaries while satisfying the hard real-time callback contract.
- `observable-demo-scene`: Produce deterministic, exhaustive, machine-readable behavioral evidence and reject controlled defects at the declared production seams.

### Modified Capabilities

None. This is the initial OpenSpec capability baseline.

## Impact

- Establishes the initial specifications under `openspec/specs/` when this change is completed and archived.
- Audits the existing `src/` modules across the Kernel, Synth, Mixer, Control, RealTime, Shell, and Testing bounded contexts plus their infrastructure adapters.
- Audits `Cargo.toml`, the product and witness binaries, the Makefile entry points, and the integration tests as implementation assets declared by CUE.
- Uses the existing CUE canonical resources and relationship index as the architectural source of truth; no architectural boundary change is proposed.
- Completion is gated by the declared format, lint, test, smoke, demo-scene, schema-surface, egui-context, and mutation-harness checks.
