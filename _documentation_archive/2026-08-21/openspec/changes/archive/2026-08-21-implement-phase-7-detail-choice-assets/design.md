## Context

Phase 6 leaves Crest with a production Patch strip/detail projection and a complete sixteen-track Mixer, but the subordinate Patch interaction is still deliberately narrow: `PatchDetail` exists, while choice modals, the Sample capability, asset browsing, audition, and correlated assignment do not. Phase 7 crosses control state, capability schemas, filesystem adapters, preparation workers, prepared graph ownership, callback rendering, projections, the webview shell, persistence, tests, and physical-audio evidence.

`DESIGN.md` remains the master product and architecture source. Before behavior changes, implementation must move the durable decisions below into it. The [Crest Synth Figma file](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=0-1) is normative for composition, navigation, focus, and visual language. In particular, implementation must revisit [Instrument Detail (37:7)](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=37-7), [FX Detail (38:60)](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=38-60), [Sample Detail (39:92)](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=39-92), [Sample Browser (41:138)](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=41-138), [Engine Options (48:173)](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=48-173), [Post FX Options (48:207)](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=48-207), and the [Interaction Map (49:3)](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=49-3). The repository's `figma-functional-interpretation/` images are review aids, not a second design authority. Figma's concrete option names, asset names, counts, values, and example parameter sets are fixtures.

The hard boundaries remain unchanged: physical input becomes a semantic action/event, `AppState::apply` owns mutation, UI and audio are projections, structural state is completely prepared before block-boundary ownership transfer, scalar snapshots and discrete events use their own real-time transports, and nothing callback-owned may allocate, lock, block, perform I/O/logging, panic, or destruct.

## Goals / Non-Goals

**Goals:**

- Complete one capability-polymorphic detail and choice interaction model with stable semantic focus and exact return.
- Define and implement the deferred first Sample product contract rather than inferring it from a design fixture.
- Add library-scoped browsing, correlated preview and assignment, explicit lifecycle/error projection, persistence, and a prepared sixteen-voice Sample engine.
- Reuse the production component library and preserve shell anatomy at authored and compact densities.
- Produce deterministic tests and an optimized physical-audio scene that falsifiably prove UI, preparation, routing, audible behavior, and teardown.

**Non-Goals:**

- MP3, AIFF, FLAC, RF64/RIFX, compressed WAV, more than stereo, streaming-from-disk, recording, destructive sample editing, time stretching, slicing, multi-zone mapping, reverse or ping-pong loops.
- A general file manager, native file dialog, unrestricted filesystem access, cloud asset library, or broad session/preset library management.
- Adding the uninstalled Figma example engines/effects, treating its parameter count as an API, or adding capability-identity branches to the reducer/projector/renderer.
- Nested modal stacks, mouse-first interaction, hover authority, a third top-level context, final Phase 9 pixel polish, or Phase 8's exhaustive controller/content/resolution matrix.

## Decisions

### 1. Product authority is split deliberately

Durable behavior, limits, type ownership, and real-time rules live in `DESIGN.md`; this change's specs are temporary acceptance deltas. Figma owns the referenced surface composition and interaction language. Where they differ, the master design resolves behavior—for example, Start is hold-to-preview only in the Sample Browser—and fixture data never creates an installed capability.

Alternative considered: reproduce every value and option visible in Figma. Rejected because the master design explicitly classifies them as fixtures and because doing so would create unavailable placeholders and concrete identity branching.

### 2. One canonical subordinate-session union replaces special-case modal state

`InteractionState` will evolve its current detail-only invariant into one private canonical subordinate-session union:

```text
PatchSubordinateSession
├── Detail { subject, origin }
├── Choice { subject, origin, return_detail_subject? }
└── SampleBrowser { patch_id, asset_parameter_id, origin, return_detail_subject }
```

`FocusPath` gains stable modal/browser row identities, never indices. Exactly one subordinate session is active. A choice or browser opened from Detail replaces the visible Detail surface while retaining the detail subject needed to restore the exact origin; it does not push an arbitrary surface stack. No modal can open another modal. The resolver repairs a missing origin to the nearest valid semantic sibling deterministically and exposes that recovery.

Alternative considered: add independent booleans and option indices for each modal. Rejected because they permit contradictory states, duplicate selection authority, and lose exact origin across schema changes.

### 3. Detail and choice content remains schema and registry driven

The existing instrument/effect descriptors and canonical `ParameterId`, `ParameterValue`, `AssetAssignment`, section, dependency, and update categories remain the only domain vocabulary. They may gain generic detail presentation metadata and non-focusable visualization declarations, but no `match Sample`, `match Braids`, or `match Chorus` is permitted in control or UI policy.

A control-side `ChoiceSource`/resolver maps generic source kinds to stable options:

- installed instruments from `InstrumentCapabilityProvider`;
- `EMPTY` plus installed effects from `EffectCapabilityProvider`;
- T00–T0F from the canonical Mixer track domain;
- descriptor choices or an adapter-provided correlated catalog.

Choosing dispatches the existing typed semantic intent appropriate to the control: engine/effect/asset changes use correlated structural preparation, while route and eligible scalar choices use their canonical parameter snapshot path. The modal itself never edits domain state.

Alternative considered: make the webview assemble option lists from labels it already renders. Rejected because that would make installed availability and choice identity UI-owned.

### 4. The first Sample descriptor is bounded and single-asset

The Sample capability declares `FixedPerPatch(16)` and one asset assignment. Its ordered schema exposes the asset, root note, normalized playback start and length, normalized loop start and length, loop mode (`OFF`, `FORWARD`), crossfade seconds, and the shared Patch `VoiceEnvelope`. Dependency predicates disable loop landmarks and crossfade while looping is off. Preparation converts normalized locations into validated frame landmarks for the selected asset. This keeps saved intent stable when an asset changes while making the prepared engine's addresses exact.

At render time, each voice reads shared prepared PCM using bounded linear interpolation and pitch ratio `2^((note-root)/12)`. Voice selection is inactive → oldest releasing → oldest active. A forward loop continues during release until the canonical envelope reaches silence. The engine owns no path, string, decoder, mutex, or callback-time allocation.

Alternative considered: adopt the Figma screen's apparent fifteen-parameter set verbatim. Rejected because the screen explicitly represents a fixture and does not establish complete playback semantics. Alternative considered: engine-managed or globally shared polyphony. Rejected because a fixed independent bank is simpler to bound, test, and correlate with Patch voice limits.

### 5. WAV admission and memory limits are exact product policy

Add a small, pinned Rust RIFF/WAVE decoder dependency behind a `SampleDecoderPort`; prefer `hound` over a general multimedia framework because Phase 7 admits only uncompressed WAV and does not need codecs or platform services. Decoder-library types stay inside the adapter. The product accepts mono/stereo signed PCM 16/24/32 and float32 at 8–192 kHz, a maximum 256 MiB source file, and a maximum 300-second duration. Preparation rejects non-finite float input rather than repairing it silently.

Decoded audio is normalized to finite interleaved `f32`, resampled off-thread to the negotiated device rate, and shared read-only by the sixteen voices. A prepared asset is limited to 28,800,000 scalar samples, and one complete graph is limited to 512 MiB of deduplicated Sample PCM. Deduplication keys on validated stable asset identity plus decode/preparation parameters and happens only on the worker. Prepared graph ownership ensures final destruction remains off the callback.

Alternative considered: streaming long files. Rejected for Phase 7 because it adds callback-visible buffer starvation, I/O scheduling, and a separate failure model. Alternative considered: unbounded full decode. Rejected because it prevents a falsifiable graph memory ceiling.

### 6. Filesystem access is a library-root capability port

Introduce a `SampleAssetCatalogPort` whose production adapter is configured with one library root. Domain state stores a validated root-relative `SampleAssetId`; the adapter canonicalizes candidates, prevents traversal/symlink escape, filters eligible WAV files, and returns deterministic folder-first listings with stable row IDs and typed metadata state. Saved state contains only the stable ID and Sample config. Missing restore assets remain explicit and never substitute another file.

Alternative considered: native open dialogs or absolute paths in `AppState`. Rejected because they break controller focus, deterministic tests, portability, and the asset-adapter boundary.

### 7. Assignment extends the existing structural lifecycle

Sample assignment uses a correlation key containing Patch identity, monotonic request identity, source graph revision, and asset identity. `AppState` owns requested/active asset and the typed `Loading → Validating → Preparing → Activating → Ready` lifecycle plus unavailable, invalid, cancelled, and preparation-failure terminals. The worker always builds a complete candidate graph; activation alone commits the active reference. Older completions are rejected. The prior graph stays active on every pre-activation failure.

Instrument preparation errors will gain precise Sample causes while higher layers preserve typed categories rather than collapsing them to a string. The webview renders those categories, but does not own them.

Alternative considered: update the session asset immediately and repair it if preparation fails. Rejected because UI and audio would disagree and rollback could lose the prior valid configuration.

### 8. Preview uses prepared structural state plus the discrete-event transport

Browser focus may request preparation of one audition candidate. The worker adds a single `PreparedSampleAudition` slot, correlated to the browser session/row, to a complete candidate graph without changing the persisted Sample assignment. Once that graph activates, `PreviewStart` and `PreviewStop` travel as fixed-size discrete audio events. The audition slot mixes into the origin Patch stem before its post effects and trim, so normal track routing, level, pan, mute, solo, sends, and returns apply.

Start press/release and all implicit stops first become semantic events reduced by `AppState::apply`. A start is emitted only if the same row is still held when its compatible audition slot is active; stale completion cannot sound. One audition voice plays the raw asset at original pitch with no loop and a prepared 5 ms stop ramp. Preview state is transient, is absent from persistence, and never changes the committed asset.

Alternative considered: play preview from the control thread or a second audio device stream. Rejected because it bypasses production routing, timing, meters, device ownership, and callback safety. Alternative considered: swap the committed Sample engine temporarily. Rejected because audition must not mutate or interrupt the active instrument.

### 9. Waveform and playhead data use the correct transports

The preparation result owns a control-side waveform summary of at most 2,048 min/max pairs plus metadata and exact prepared landmarks. Projection copies only that bounded summary, not decoded PCM. The callback reports a decimated Sample playhead/preview observation keyed by Patch, graph revision, and parameter generation through the latest-observation snapshot path. The UI paints only compatible observations and shows stale/unavailable state explicitly; repaint never calls the reducer.

Alternative considered: poll the prepared engine or mutate application state each frame. Rejected because it crosses ownership boundaries and turns presentation cadence into domain events.

### 10. Webview composition reuses the component library and semantic projection

Add typed projections for detail sections, option-modal entries, browser entries/metadata, waveform landmarks, and lifecycle status. The renderer composes existing hairlines, CLI hints, parameter rows, asset rows, modal options, status treatments, and focus/edit/disabled variants. At 1920×1080 it matches the authored horizontal bands and 1500/420 workspace split. Compact density reflows the same regions and semantic IDs; it never dispatches an action or changes selection. Current, focused, editing, unavailable, invalid, loading, cancelled, and preview-playing states carry text or shape in addition to color.

Alternative considered: implement Sample Detail and Browser as one-off HTML. Rejected because it would fork focus behavior and the established vocabulary before Phase 9 cutover.

### 11. Acceptance starts below the UI and ends with physical evidence

Development begins with failing reducer, resolver, descriptor, decoder, preparation, engine, routing, projection, and render-path tests. Decoder/admission tests use small repository-owned valid and invalid WAV fixtures and deterministic generated buffers; they do not depend on Figma assets. Callback audit tests cover dispatch, render, audition start/stop, graph swap, and retirement. Screenshot/projection witnesses cover the referenced Figma surfaces at authored and compact densities.

`make demo-live-detail-and-assets` is cumulative and becomes `make demo-live`. It uses the real parsed MIDI fixture, multiple installed capabilities, the production webview and physical device, production Sample fixtures, bounded named checkpoints, per-Patch/track audio measurements, a controlled negative, and full teardown. Earlier targets remain unchanged and callable.

Alternative considered: prove browser/asset behavior only with a headless unit test. Rejected because Phase 7 explicitly requires audible isolation, routing, visible focus correlation, and real preparation/window/device evidence.

## Risks / Trade-offs

- [A five-minute stereo asset can consume about 115 MiB after decode at 48 kHz] → Enforce per-asset scalar and 512 MiB graph budgets before publication, deduplicate on the worker, and report capacity failure without fallback.
- [Preparing a newly focused preview can have visible latency] → Show `PREPARING`, prefetch only the focused candidate, start only while the hold remains correlated, and never conceal the delay with control-thread audio.
- [Adding modal/browser origins can weaken the current detail invariant] → Replace it with one private subordinate-session union, enumerate all transitions, mutation-test invalid combinations, and keep arbitrary modal stacks impossible.
- [Malformed WAV and path escape are hostile adapter inputs] → Keep parsing/canonicalization outside the callback, cap bytes/duration/channels/rate before allocation where possible, validate decoded counts with checked arithmetic, and test traversal, symlink, chunk, and non-finite cases.
- [Routing audition through the origin Patch can make preview silent when that route is muted or solo-excluded] → Treat that as truthful production behavior and project the route/mute/solo cause explicitly in preview status and the live report.
- [Linear interpolation and loop crossfade trade quality for boundedness] → Keep the algorithm explicit and deterministic in Phase 7; a future resampler or loop mode requires its own measured capability change.
- [The Phase 7 slice is broad] → Land it in dependency order with production-path gates after each package; do not build parallel UI-only domain models or weaken the live scene to compensate.
- [Figma may evolve after the captured node references] → Review the linked live nodes at implementation and acceptance time, record the node IDs/date in visual evidence, and update `DESIGN.md` for any durable resolution rather than silently following fixture drift.

## Migration Plan

1. Update `DESIGN.md` with the Sample contract, subordinate-session invariant, preview transport, and Figma node references; record Phase 6 closeout/Phase 7 evidence expectations in `ROADMAP.md`.
2. Add failing acceptance tests and canonical types/actions while retaining current serialized-session compatibility and existing live targets.
3. Replace the detail-only interaction invariant atomically with the subordinate-session union; migrate projector and serialization tests in the same change so no contradictory intermediate state is exposed.
4. Add the catalog/decoder adapters, Sample capability descriptor, prepared engine, structural assignment/preview paths, callback audits, and repository-owned fixtures. Register Sample only after a valid production preparer is composed; never expose a placeholder.
5. Add typed projections and webview compositions, compare the referenced Figma surfaces at 1920×1080 and compact density, then enable choice/browser gestures.
6. Version saved state for Sample assignments. Older sessions migrate with no Sample config. A new session whose Sample asset is missing restores as explicit `Unavailable`; decoded PCM and transient preview state are never serialized.
7. Add the optimized live scene, controlled negative, Make target, and cumulative alias; run deterministic validation before the required physical window/audio evidence.

Rollback removes Sample from the installed registry and restores the prior cumulative alias, while leaving the previous active graph/session untouched. A saved newer Sample capability is reported as unsupported/unavailable on an older build; rollback MUST NOT substitute SoundFont or another asset. Earlier retained demo targets remain the regression floor.

## Open Questions

None for Phase 7 planning. Additional formats, streaming, alternative loop modes, higher-quality interpolation, multi-zone mapping, or different limits require a later explicit capability change with new real-time and memory evidence.
