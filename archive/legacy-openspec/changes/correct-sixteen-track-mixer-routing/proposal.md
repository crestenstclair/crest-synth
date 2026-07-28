## Why

The current implementation treats each Patch as a mixer channel, so Patch count and instrument schema determine the MIXER surface and a Patch owns the gain/pan/send values that should belong to a channel. That contradicts the authored product: Crest needs exactly sixteen persistent, configurable mixer tracks, with each Patch routed to one track for shared volume control, before Phase 3 expands routing topology.

## What Changes

- **BREAKING** Replace Patch-owned `ChannelParameters` and Patch-keyed MIXER columns with one canonical `MixerState` containing exactly sixteen tracks, T00 through T0F.
- **BREAKING** Give each Patch one `PatchOutput` containing a validated `MixerTrackId` and Patch-local trim; a Patch no longer owns track level, pan, mute, solo, sends, or meters.
- Allow multiple Patches to route to one track. Apply Patch trim before accumulation, then apply the destination track's level, pan, mute/solo gate, and current reverb/delay sends to the combined signal.
- Make mute win over solo, make any active solo exclude non-soloed tracks, keep sends post-fader/post-gate, and meter every post-level/pan track before its audibility gate.
- Carry Patch outputs and all sixteen track parameter sets through one fixed latest `ParameterSnapshot`; carry all sixteen numeric meters through the separate fixed `AudioObservationSnapshot`. Route changes use the already-prepared destinations and do not rebuild the graph.
- Replace Patch-derived MIXER focus/projection with stable `MixerTrackId + MixerTrackParameter` identities. MIXER always projects all sixteen tracks; PATCH Utility owns Patch trim and output-track controls.
- Rework deterministic, mutation, schema, responsive-layout, and physical-live evidence around shared-track summing, rerouting, track isolation, mute/solo, sends, meters, invalid-route rejection, callback safety, and complete teardown.
- Add and retain `make demo-live-sixteen-track-mixer-routing`; point `make demo-live` to this newest cumulative scene while preserving the Phase 1 and Phase 2 targets.
- Keep Phase 3 capped at three Patch effect slots and eight bus returns, and leave Phase 6 component-library assembly, density refinement, multi-select, and visual finish for its roadmap phase.

## Capabilities

### New Capabilities

- `sixteen-track-mixer-routing`: Defines the fixed track bank, Patch output route/trim ownership, track mix and meter semantics, semantic controls, bounded transports, and production-path proof.

### Modified Capabilities

- `global-mix`: Changes gain/pan/mute/solo/send ownership and processing from individual Patches to post-accumulation mixer tracks.
- `one-way-parameter-control`: Replaces the Patch-derived MIXER editable resolver and fixed projection with distinct PATCH output and sixteen-track control paths.
- `per-voice-envelope`: Keeps ADSR Patch-owned and editable through PATCH, removing the obsolete claim that Patch envelopes are mixer-channel controls.
- `schema-driven-patch-page`: Makes PATCH Utility's canonical trim and output-track controls functional through the existing semantic reducer path.
- `live-observable-demo`: Freezes and proves focused-Patch output plus all fixed-track coverage in the newest retained physical scene instead of treating Patch values as mixer channels.
- `observable-demo-scene`: Serializes and causally verifies Patch outputs, all sixteen track states/meters, shared-track behavior, and the cross-track leakage mutant.

## Impact

- Domain and control: `Patch`, `AppState`, mixer parameter descriptors, semantic focus/control identities, reducers, serialization, text/state-tree/page projections, and demo coverage schemas.
- Real time and audio: `ParameterSnapshot`, `AudioObservationSnapshot`, prepared graph scratch, `AudioRenderer`, and `MixEngine`; no new callback transport class or callback allocation is permitted.
- UI and composition: the eframe/egui shell renders sixteen stable columns plus immutable fixed meter observations; no new GUI framework or component dependency is introduced.
- Tests and commands: existing mixer/envelope/demo/mutation assertions migrate to track semantics, a named `sixteen_track_mixer_routing` target is added, and prior live targets remain available.
- Change ordering: this corrective change follows the implemented Phase 2 semantic model and blocks Phase 3; the Patch-shaped Phase 2 MIXER contract must not be archived as final product behavior without this correction.
