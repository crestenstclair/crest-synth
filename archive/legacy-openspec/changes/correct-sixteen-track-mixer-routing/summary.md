# Change Summary

## Outcome

- **Problem:** Patch count currently determines MIXER columns, and Patch owns controls that belong to persistent tracks.
- **Result:** Crest exposes exactly sixteen configurable tracks, T00–T0F; each Patch owns only a validated destination and trim.

## Change Outline

- **Adds:** Canonical track/output types, fixed snapshots and scratch, track semantic controls, meters, and deterministic/physical evidence.
- **Changes:** Patches may share a track before level/pan; mute wins, Solo gates other tracks, sends are post-gate, and meters are pre-gate.
- **Removes:** `ChannelParameters`, Patch-keyed MIXER columns, Patch-owned track controls/meters, and obsolete mixer targets without aliases.

## System Impact

- **Capabilities:** Adds `sixteen-track-mixer-routing`; modifies `global-mix`, `one-way-parameter-control`, `per-voice-envelope`, `schema-driven-patch-page`, `live-observable-demo`, and `observable-demo-scene`.
- **Architecture:** Centers the canonical Mixer value objects, `aggregate.Control.AppState`, `domainService.Control.StateProjector`, `domainService.Mixer.MixEngine`, `applicationService.RealTime.AudioRenderer`, and both snapshots.
- **Interfaces/data:** Breaks Patch construction and serialized/snapshot schemas; adds the retained `make demo-live-sixteen-track-mixer-routing` witness and alias.

## Delivery

- **Implementation:** Follow [tasks.md](tasks.md) through domain ownership, reducer, projections, real-time routing, eframe rendering, evidence, physical scene, and cleanup.
- **Validation:** Require focused acceptance, exact coverage, cross-track mutation failure, callback-safety measurements, Cargo gates, strict OpenSpec validation, and bounded teardown.

## Risks and Decisions

- **Key decisions:** Routes use compatible latest snapshots, all destinations are preallocated, meters remain separate observations, and summing has no hidden normalization.
- **Sequencing:** This is the next gate after Phase 2 and blocks Phase 3; do not archive the Patch-shaped MIXER contract first.
- **Risks:** Migration breadth, fixed scratch, clipping, and meter lag are mitigated by compile-time replacement, explicit gain, measurements, and generation tags.
- **Open questions:** None; later Phase 3, 6, and 7 decisions remain deferred.
