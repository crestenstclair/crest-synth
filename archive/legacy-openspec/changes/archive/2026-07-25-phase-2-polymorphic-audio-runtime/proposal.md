## Why

Admitting Braids is safe only if heterogeneous preparation, graph-compatible scalar delivery, SoundFont voice ownership, and hard-real-time callback behavior remain explicit and measurable. This completed slice owns those audio-runtime contracts and their production witnesses.

## What Changes

- Extend prepared-instrument dispatch/render ports to receive only the matching fixed Patch parameter projection.
- Render simultaneous SoundFont and multiple Braids Patches through one bounded prepared rack with exact routing and graph/layout compatibility.
- Preserve separate discrete, latest-scalar, structural, retirement, and observation transports with no callback allocation, destruction, lock, block, I/O, log, or panic.
- Retain exactly one prepared SoundFont synthesizer per Patch and one shared parsed immutable bank while providing per-note envelope isolation.
- Measure worst-case mixed-engine 48 kHz/256-frame timing and require p99 below half the callback period.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `prepared-engine-rack`: Production mixed-engine routing and graph-compatible scalar snapshots.
- `realtime-execution`: Mixed FFI/callback safety, finite rendering, observations, and timing admission.
- `soundfont-audio`: Engine-managed SoundFont polyphony with one synthesizer per Patch and native per-note envelope isolation.

## Impact

This slice affects prepared instrument/rack/graph APIs, scalar snapshots, renderer ordering, lock-free boundaries, structural retirement, callback observations, the SoundFont backend seam, and the prepared-rack/runtime timing acceptance targets. It is one non-overlapping slice of the integrated Phase 2 architecture.
