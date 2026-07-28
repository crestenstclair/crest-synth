## Context

Phase 2 established the production semantic path, reducer-owned focus, immutable projections, prepared audio graph, and eframe/egui shell, but its current mixer model conflates a Patch with a mixer channel. `Patch` owns `ChannelParameters`, MIXER columns are resolved from installed `PatchId` values, and `MixEngine` applies those values before the current shared effects. This contradicts `DESIGN.md`, the Figma fixture, and the evaluated CUE architecture, all of which require a persistent bank of exactly sixteen tracks.

This is a corrective gate after Phase 2 and before Phase 3. It deliberately changes the current Rust API and serialized proof schema rather than retaining two meanings of “channel.” It preserves the established control, graph-preparation, real-time, and GUI boundaries.

## Goals / Non-Goals

**Goals:**

- Make T00 through T0F one canonical, persistent mixer bank independent of Patch count and engine schema.
- Let every Patch select one track and one pre-track trim, including many-to-one Patch routing.
- Make Level, Pan, Mute, Solo, Reverb Send, Delay Send, and meters track-owned.
- Preserve the one-way physical-input to semantic-action/event to `AppState::apply` path for Patch-output and track edits.
- Preserve bounded, preallocated callback work and the existing discrete/latest-scalar/prepared-graph transport separation.
- Project all sixteen tracks through stable semantic identities in both reference layouts and provide causal deterministic and physical evidence.

**Non-Goals:**

- Phase 3's configurable Patch graph, its maximum three effect slots, or its maximum eight bus returns.
- Dynamic track counts, arbitrary buses, additional shared effects, inserts, EQ, compression, or a general routing matrix.
- Phase 6 component-library assembly, density refinement, multi-select behavior, or final visual polish.
- Phase 7 sample-preview behavior, which remains deferred to its own decision.
- A new GUI framework, audio engine, transport class, persistence format, or compatibility facade for the incorrect Patch-channel API.

## Decisions

### 1. Separate Patch output from persistent track state

The canonical ownership is:

| Concept | Canonical owner | Stable identity | Editable surface |
|---|---|---|---|
| Instrument, envelope, effects | `Patch` | `PatchId` + descriptor ids | PATCH Main |
| Output trim and destination | `Patch.output: PatchOutput` | `PatchId` + `PatchOutputParameter` | PATCH Utility |
| Level, pan, mute, solo, sends | `AppState.mixer: MixerState` | `MixerTrackId` + `MixerTrackParameter` | MIXER Main/Inspector |
| Track meter | latest audio observation | `MixerTrackId` | read-only MIXER display |
| Shared effect and master values | global mixer state | `GlobalParameter` | distinct MIXER globals |

`MixerTrackId` is a validated `u8` newtype for 0 through 15 and formats as T00 through T0F. `MixerState` stores `[MixerTrackParameters; 16]`. `PatchOutput` stores only `track_id` and finite `trim_gain_db`; it contains no fader, pan, gate, send, or meter. Patch construction requires an explicit validated output so routing is never silently inferred from `PatchId`, MIDI channel, or collection position. The Corridors fixture assigns its known parts deterministically across the fixed bank, while tests choose routes explicitly.

The old `ChannelParameters`, its `ChannelParameter` descriptor, and the mixer meaning of `PatchEditableTarget` are removed, not aliased. An alias would preserve the invalid ownership model and allow new call sites to keep treating Patches as tracks.

### 2. Use one exact signal order

The production callback uses this order for every render block:

```text
instrument -> Patch effects -> Patch trim -> destination accumulation
           -> track level/pan -> pre-gate meter -> mute/solo gate
           -> dry master sum + post-gate track sends
           -> one shared reverb + one shared delay -> master gain
```

All Patch stems routed to the same track are summed before any track parameter is applied. The solo set is computed once from the sixteen fixed track values. Mute always wins; when any Solo is active, only soloed and non-muted tracks contribute dry or send signals. Reverb and Delay sends are post-fader and post-gate. Each `TrackMeter` contains finite nonnegative left peak, right peak, and RMS measured after track Level/Pan but before the gate, so a sounding muted or solo-excluded track remains visible without exciting the output or shared effects. Empty tracks report zero.

No automatic normalization is introduced when several Patches share a track. Existing finite/clipping observations remain the proof seam, while the track and master levels provide explicit gain control.

### 3. Treat route changes as compatible latest-snapshot data

`ParameterSnapshot` carries each active Patch's `PatchOutput` beside its existing fixed engine, envelope, and effect scalar layout, plus `[MixerTrackParameters; 16]` and globals. The snapshot remains one complete immutable generation matched to the active prepared graph revision.

Every prepared graph allocates sixteen destination scratch buffers before publication. A valid route edit therefore changes only the next complete latest-value parameter generation; it does not submit structural work, change `GraphRevision`, or create a fourth transport. Invalid track ids are rejected before publication and are never clamped, wrapped, dropped, or replaced.

`AudioRenderer` continues to produce one identity-preserving post-effect stem per prepared Patch. `MixEngine` clears the fixed track scratch, applies each Patch trim, accumulates by validated `MixerTrackId`, then runs the track and global stages. Its callback path retains the hard constraints: no allocation/deallocation, locks, blocking, I/O, logging, formatting, panic/unwind, or destruction.

### 4. Keep meters in the separate latest observation transport

`MixObservation` and `AudioObservationSnapshot` gain `[TrackMeter; 16]` in canonical track order. The callback computes these values from its actual track buffers and publishes them through the existing latest numeric observation boundary. The observation is sequence-, parameter-generation-, and graph-revision-correlated.

Meters do not enter `AppState`, `StateSnapshot`, `ParameterSnapshot`, EventLog, or a mutable widget cache. The window receives one immutable audio observation alongside the canonical graphical projection and joins meters to controls only by `MixerTrackId`. A stale observation may be displayed only as stale diagnostic data; it cannot change focus, values, routing, or acceptance evidence for a newer generation.

### 5. Replace Patch-keyed MIXER semantics with track-keyed semantics

`MixerControlId` becomes `Track(MixerTrackId, MixerTrackParameter) | Global(GlobalParameter)` and contains no `PatchId`. PATCH uses `PatchControlId` for Engine, Envelope, capability/effect descriptors, and `Output(PatchOutputParameter)`; the obsolete cross-context `PatchEditableTarget` model is removed.

Startup focus is MIXER/T00/Level. Horizontal MIXER navigation changes `MixerTrackId` while preserving the compatible control row, including across empty tracks; globals remain a distinct focus branch. PATCH Utility resolves Trim Gain then Output Track for the focused Patch. Mixer Inspector resolves the selected track's sends and a read-only routed-Patch summary. All changes still flow through normalized input, `SemanticAction`, `AppEvent`, `AppState::apply`, and then same-generation projections and audio publication.

Patch install, engine replacement, effect replacement, and rerouting must preserve all sixteen track values. Context, surface, focus, and viewport changes remain audio-neutral. Responsive composition may scroll or reflow the fixed bank, but it cannot omit tracks or substitute layout/widget indices for semantic identity.

### 6. Keep the GUI correction functional and framework-stable

The existing eframe/egui adapter renders the canonical sixteen-track model and the separate immutable meters. `egui_extras` may provide the existing table/strip primitives, but this change adds no GUI dependency and no reusable visual component layer. Desktop and Steam Deck reference viewports must keep all sixteen tracks addressable, with horizontal scrolling or compact composition as needed. Styling beyond what is necessary to make routing and track state legible remains Phase 6 work.

### 7. Make the correction falsifiable through production seams

A focused `tests/sixteen_track_mixer_routing.rs` target exercises the production `Patch`, reducer, semantic resolver/projector, fixed snapshots, prepared renderer, mixer, and audio observation boundary. It proves exactly sixteen tracks with empty/populated cases, shared-track summing before the fader, trim/reroute isolation, all six track parameter classes, mute-wins and any-solo behavior, post-gate sends, pre-gate muted meters, invalid-route rejection, responsive stable focus, finite audio, and zero callback allocation/destruction.

Existing schema, page, semantic-view-model, envelope, prepared-rack, static-effect, exhaustive-demo, live-demo, runtime, and shell tests migrate from Patch-channel expectations to Patch-output plus track expectations. The old cross-Patch parameter-leak mutant becomes a cross-track parameter-leak mutant at the snapshot-to-mixer ownership seam.

`make demo-live-sixteen-track-mixer-routing` is a retained cumulative physical witness. `make demo-live` aliases it, while the Phase 1 and Phase 2 named live targets remain available. The final marker is emitted only after the production window and stream have demonstrated the declared behavior, semantic note cleanup has completed, the window has closed, the stream and worker have shut down, all graphs have been collected off callback, and the parent exits successfully.

## Risks / Trade-offs

- **Broad breaking surface:** `ChannelParameters` appears across domain, projection, renderer, demos, and tests. Mitigation: introduce the canonical types first, migrate ownership from the reducer outward, and remove the old type only after all production call sites use track semantics.
- **Transient mixed models:** Partial migration could publish Patch-shaped state to a track-shaped renderer. Mitigation: change the complete snapshot as one compile-time contract and do not retain conversion shims.
- **Callback memory growth:** Sixteen stereo scratch blocks cost more fixed memory than the current Patch-direct mix. Mitigation: size storage from the already-negotiated maximum callback frame count, allocate before graph publication, and retain measured callback timing/allocation gates.
- **Incorrect gate or send ordering:** A plausible implementation can sound correct while leaking muted tracks into effects or hiding their meters. Mitigation: measure the declared buffers separately with discriminating inputs and paired state, including muted-sounding and active-solo cases.
- **Meter/state incoherence:** Latest audio observations can lag accepted UI generations. Mitigation: retain sequence, parameter generation, and graph revision tags and disallow meter data from reducer-owned state or acceptance for an unmatched generation.
- **Higher summed levels:** Multiple Patches on one track can clip where separately attenuated Patch channels did not. Mitigation: do not add hidden normalization; preserve explicit trim/track/master gain and measured clipping evidence.
- **Overlap with the active Phase 2 change:** Archiving the older Patch-shaped contract first would make the wrong model appear durable. Mitigation: land and validate this corrective change as the next gate and archive specs only with the corrected deltas applied.

## Migration Plan

1. Add validated `MixerTrackId`, `PatchOutput`, `MixerTrackParameters`, `MixerState`, and `TrackMeter` types and their single production-owned descriptors.
2. Require `PatchOutput` in Patch construction, add the fixed `MixerState` to `AppState`, and migrate fixtures to explicit deterministic routes.
3. Replace Patch-keyed mixer focus and adjustment with `PatchControlId::Output` and track/global `MixerControlId`, preserving the reducer-only mutation path.
4. Migrate serialization, page/text/state-tree/semantic projections, coverage descriptors, and exact-generation equality to Patch-output and sixteen-track state.
5. Change the complete parameter and audio-observation snapshot layouts, then update prepared graph storage, `AudioRenderer`, and `MixEngine` together so no mixed contract compiles.
6. Render the sixteen canonical tracks and immutable meters in the existing eframe adapter at both reference viewports.
7. Migrate deterministic fixtures, mutation seams, exhaustive/live runners, existing affected tests, the new focused acceptance target, and retained Make targets.
8. Remove `ChannelParameters`, Patch-derived mixer targets, obsolete serialized fields, and all compatibility helpers; run formatting, lint, complete tests, strict OpenSpec validation, and the bounded physical witness.

No persisted user-project format currently requires an on-disk data migration. If persistence lands before this correction is applied, implementation must stop and propose an explicit versioned migration rather than infer routes from Patch order.

## Open Questions

- None for this gate. Phase 3 routing expansion, Phase 6 visual composition, and Phase 7 sample preview remain governed by their later roadmap decisions.
