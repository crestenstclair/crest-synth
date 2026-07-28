## 1. Canonical Mixer Domain

- [x] 1.1 Add validated `MixerTrackId`, `PatchOutputParameter`, and `PatchOutput` modules with T00–T0F formatting, finite trim bounds, nonwrapping route choice, production-owned descriptors, and boundary/error unit tests.
- [x] 1.2 Add `MixerTrackParameter`, `MixerTrackParameters`, `MixerState`, and `TrackMeter` modules with exactly sixteen tracks, canonical defaults/descriptors, typed adjustment/toggle behavior, and unit tests for every bound and invalid value.
- [x] 1.3 Replace `Patch.parameters: ChannelParameters` with a required explicit `Patch.output: PatchOutput`, preserve output across instrument/effect replacement, and migrate constructors plus deterministic fixtures to declared routes.
- [x] 1.4 Add the fixed `MixerState` consistency boundary to `AppState` and prove Patch install, schema replacement, and rerouting never create, remove, reorder, or reset tracks.

## 2. Semantic Control and Reducer

- [x] 2.1 Extend `PatchControlId` and PATCH Utility resolution with Trim Gain and Output Track, then remove Patch-owned mixer controls from PATCH Main and all envelope controls from MIXER.
- [x] 2.2 Replace Patch-keyed `MixerControlId` with track/global identities and implement stable T00/Level startup focus, horizontal row-preserving navigation across all sixteen tracks, Inspector send focus, and distinct global focus.
- [x] 2.3 Route Patch-output and track adjustments through `SemanticAction` → `AppEvent` → `AppState::apply`, including toggle semantics, fine/coarse or adjacent choice behavior, and unchanged typed rejection for invalid route/boundary requests.
- [x] 2.4 Add reducer tests proving target-only mutation, shared-track ownership, exact retained PATCH/MIXER focus, audio-neutral navigation, no graph effect for route changes, and successful recovery after a rejected event.

## 3. Canonical Projections and Serialization

- [x] 3.1 Migrate `StateSnapshot` and serialized Patch/state structures to expose each Patch output and exactly sixteen mixer-track parameter sets without retaining Patch-channel fields.
- [x] 3.2 Migrate `StateTree`, `EventRecord`, state hashes, and exact-generation comparisons to Patch-output plus fixed-track paths, including typed invalid-route rejection payloads.
- [x] 3.3 Update `PatchPageProjection` and PATCH text projection to show canonical Utility Trim Gain and Output Track controls while keeping ADSR, instrument, and effect identities Patch-owned.
- [x] 3.4 Update `SemanticGraphicalViewModel`, shell/text projection, valid-action resolution, and routed-Patch summaries so MIXER always contains T00–T0F plus distinct globals and no Patch-derived column identity.
- [x] 3.5 Update projection/schema tests to require bidirectional leaf equality for Patch outputs, all track parameters, track focus paths, Utility/Inspector controls, and meter metadata while rejecting obsolete channel paths.

## 4. Fixed Real-Time Routing and Mixing

- [x] 4.1 Change `RtPatchParameters`/`ParameterSnapshot` to publish fixed Patch outputs and `[MixerTrackParameters; 16]` as one generation- and graph-compatible latest scalar snapshot.
- [x] 4.2 Prepare sixteen fixed-capacity stereo track scratch blocks with each graph and prove a route-only generation consumes the existing graph without structural submission, revision change, allocation, or fallback.
- [x] 4.3 Rewrite `MixEngine` to clear track scratch, apply Patch trim, accumulate many Patch stems per destination, apply track Level/Pan, measure pre-gate meters, apply mute-wins/any-solo gates, derive post-gate sends, and then run the existing shared effects/master.
- [x] 4.4 Extend `MixObservation` and `AudioObservationSnapshot` with `[TrackMeter; 16]`, publish them through the existing latest observation transport, and retain exact sequence/parameter-generation/graph-revision correlation.
- [x] 4.5 Update `AudioRenderer`, prepared graph builders/coordinator, and global-effect adapter call sites to preserve post-effect Patch identity before routing and to expose only measured track/send/wet/output observations.
- [x] 4.6 Add real-time contract tests for fixed snapshot equality, route isolation, all gate/send/meter stages, finite/clipping counts, maximum callback size, and zero callback allocation, deallocation, locking, blocking, or destruction.

## 5. eframe/egui Mixer and Patch Utility

- [x] 5.1 Pass one immutable `AudioObservationSnapshot` into the existing eframe window update path without storing meters in reducer or widget-owned state.
- [x] 5.2 Render all sixteen stable mixer tracks, Level/Pan/Mute/Solo controls, track meters, selected-track sends, routed-Patch summary, and distinct globals from the semantic projection plus matching observation.
- [x] 5.3 Render PATCH Utility Trim Gain and Output Track through the same semantic dispatch callback, with no direct state mutation or duplicate route/value cache.
- [x] 5.4 Extend real egui-context and graphical-shell tests at desktop and Steam Deck reference viewports to prove every track remains visible or scroll-addressable, focus is identity-stable, rectangles remain valid, and accepted actions appear on the next frame.

## 6. Deterministic and Mutation Evidence

- [x] 6.1 Migrate shared test support and all affected capability, envelope, engine, effect, rack, runtime, page, shell, and semantic tests from `ChannelParameters` to explicit Patch outputs plus fixed mixer state.
- [x] 6.2 Add `tests/sixteen_track_mixer_routing.rs` using production reducer/projector/snapshot/renderer/mixer seams to assert sixteen-track persistence, shared-track summing, trim/reroute isolation, all six controls, gates, sends, meters, invalid routes, focus, and callback safety before its marker.
- [x] 6.3 Replace the cross-Patch parameter-leak mutant with a cross-track parameter-leak at the `ParameterSnapshot`-to-`MixEngine` seam and prove the healthy/mutant pair uses identical production observations and expected exits.
- [x] 6.4 Rebuild exhaustive demo and schema coverage from canonical Patch-output and track descriptors, exercise both Patch output fields plus six parameters on all sixteen tracks, and require exact missing/unexpected equality and exact baseline restoration.
- [x] 6.5 Update faithful-effects evidence to establish nonzero post-gate track sends, paired identical effect state, zero dry-to-wet bypass, and exact restoration of Patch outputs, tracks, sends, globals, focus, and projections.

## 7. Physical Scene and Commands

- [x] 7.1 Extend the paced live scene and report with Patch Utility output coverage, all sixteen track controls/meters, two Patches sharing one track, one compatible reroute, generation-correlated physical observations, and existing semantic/engine/preset/effect coverage.
- [x] 7.2 Add the `--demo-live-sixteen-track-mixer-routing` CLI option and `make demo-live-sixteen-track-mixer-routing`, retain prior Phase 1/2 live targets, and point `make demo-live` at the new cumulative scene.
- [x] 7.3 Emit `CREST_SIXTEEN_TRACK_MIXER_ROUTING_LIVE_OBSERVATION` only after measured routing acceptance, semantic note cleanup, zero active notes, window close, stream release, worker shutdown, graph collection, and successful bounded completion.
- [x] 7.4 Run the release physical witness and verify its structured predicates, finite nonzero device output, timeout/error cleanup, normal parent exit, and zero remaining owned graphs.

## 8. Removal and Acceptance Gates

- [x] 8.1 Remove `ChannelParameters`, `ChannelParameter`, obsolete `PatchEditableTarget` mixer paths, old serialized channel fields, and conversion/compatibility helpers; use repository-wide search to prove no production or test reference remains.
- [x] 8.2 Run `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, all focused acceptance targets, and `cargo test --all-targets`, fixing every regression without weakening assertions.
- [x] 8.3 Re-evaluate the complete CUE package, confirm `openspec context --json` has no status errors, and run strict validation for this change and the complete OpenSpec workspace.
- [x] 8.4 Confirm `DESIGN.md`, `ROADMAP.md`, evaluated CUE, proposal, specs, design, implemented behavior, and retained live commands all describe the same sixteen-track model before marking the change complete.
