## Closure scope — 2026-09-05

Closed at the user's request before starting another OpenSpec change. The
accepted scope is the functional Sample slice: default test asset and MIDI,
Detail waveform and editing, the shared in-app file page, nested navigation,
validated import, cancellation/stale-result safety, and saved-session waveform
restoration. Resize acceptance remains closed. Figma guides hierarchy and
interaction; exact pixels, typography, spacing, and exhaustive visual comparison
are not completion gates. Cloud downloading remains the storage provider's job.

The original checklist records 24 completed tasks and 36 unchecked tasks.
Unchecked items in sections 2, 4, 5, 6, 7, and 8 retain their historical status:
they cover broader visual work and aggregate validation that were not completed
as specified. They are deferred from this closed functional scope, not reported
as implemented or proven. Existing checks are recorded below and in DESIGN.md;
this closure performs documentation validation, not another product acceptance
run. SoundFont bank loading remains unimplemented and belongs to a later change.

The four delta specs are reconciled with the functionality-first acceptance
policy and synchronized to the main specs before archival. Main specs describe
behavioral contracts; synchronization is not evidence that every broader visual
requirement has already been implemented. The historical plan below explains the
original work, and this closure scope governs any conflicting completion gate.

## Context

See `proposal.md` for motivation. The live Crest Synth Figma file is the normative product, visual, and interaction authority; `DESIGN.md` records the current production architecture and evidence. The relevant authored nodes are Responsive Contract `98:2`, Sample Detail `39:92`, Sample Browser `41:138`, Mixer `42:3`, Engine Options `48:173`, Post FX Options `48:207`, Interaction Map `49:3`, and Patch Overview `95:202`.

The production renderer already consumes one immutable semantic document and has functional Sample, Mixer, Inspector, and generic option surfaces. Current native witnesses prove semantic identity, reachability, target floors, deterministic paint, and selected workflow paths, but not faithful composition. Visual tokens originate in `src/shell/tokens.rs`, responsive bounds originate in `src/shell/density.rs`, generated CSS exposes those values, and `webview-page/` owns the committed native DOM presentation. Named Figma frames and existing Wide, Standard, and Compact observations are examples of the responsive system under particular content and window conditions; they are not separate fixed canvases to reproduce.

This design must preserve the physical-input → semantic action/event → `AppState::apply` → view/audio projection path, exactly one stable semantic focus, registry-owned content, the observation compatibility rules, and the hard real-time callback contract. Figma example labels, counts, files, values, and capabilities remain fixtures.

## Goals / Non-Goals

For the complete Sample handoff, functional usability takes precedence
over further visual refinement. Reuse accepted resize evidence and existing
native captures; do not reopen manual dragging or tune against exact Figma
pixels. Retain the existing library browser and waveform renderer. Correct
their normal entry paths and truthful action/state presentation, with focused
regressions through the production worker, reducer, projector, and renderer.

Return and OpenRelated on Sample File enter the same reducer-owned FileBrowser
surface. The existing semantic Navigate, Activate, Return, and PreviewStart/Stop
controls own folder movement, selection, cancellation, and Sample audition.
There is no native asset dialog or mouse-only path. Shared AssetFileId,
FileBrowserFolderId, row, and listing types are independent of WAV decoding;
the filesystem browser filters by AssetKind (WAV or SF2), and each engine adapter
owns metadata, import, validation, and preparation. The shared page can therefore
serve the later SoundFont loader without duplicating navigation or focus state.

The root lists the asset library plus Home and, on macOS, Volumes locations.
Location-relative browser identities remain transient. Selecting an external
Sample emits a correlated import request; the worker validates and copies it
into Imported without overwriting another file. Only the resulting stable
library-relative identity can be assigned or restored from a saved session.
Folder enumeration, import, decoding, and waveform preparation run off the
audio callback and off the window tick. Existing Sample assignment and graph
activation remain the only way to commit a chosen file. Cancellation and stale
results preserve the acknowledged assignment and exact origin focus.

Prepared session replacement carries the waveform summaries belonging to its
complete graph. This covers both startup and Open without serializing PCM or
waveform caches into the session document. Normal launch uses Sample with an
explicit bundled test asset and a bounded automatic MIDI pattern, routed through
the existing channel-based MIDI admission path. Test playback has a visible
start/stop action and pauses during browser audition and session replacement.

**Goals:**

- Make the named Sample, Mixer, and option surfaces visually faithful to their live Figma compositions through the existing production semantic path.
- Define a reproducible native review method that separates visual proof from functional and structural proof while evaluating relationships rather than pixel correspondence.
- Exercise fluid resizing through varied constrained, intermediate, and expanded conditions, including long content, text scale, and widths not represented by named Figma frames.
- Preserve semantic focus, return identity, projected content, reducer generation, and observation compatibility during every presentation-only resize.
- Keep shared tokens and primitives coherent, then apply final cross-surface polish only after each named surface has its own passing comparison.

**Non-Goals:**

- Change Sample browsing, preview, asset selection, Mixer control, option selection, Patch creation, session, or navigation semantics.
- Add capability-specific renderer branches, fixed Figma fixture counts, a second visual state model, or viewport-derived application state.
- Change audio graph behavior, real-time transports, callback work, persistence format, installed registries, or input devices.
- Establish release or visual acceptance on another platform, or claim parity for Settings or later roadmap surfaces not named by this change.
- Treat an OpenSpec artifact, DOM presence, reducer test, or geometry-only witness as proof of final visual fidelity.
- Build viewport-specific page variants, reproduce absolute frame coordinates, tune isolated pixel offsets to match one screenshot, or gate acceptance on image-difference scores.

## Decisions

### 1. Establish a live-Figma measurement ledger before visual edits

Implementation begins by reading the live nodes listed in Context and recording a bounded ledger for each named state: node identity and retrieval time, region hierarchy, relative emphasis, type roles, spacing rhythm, flexible region relationships, keylines, radii, state markers, focus treatment, control anatomy, footer guidance, and representative fixture content. The ledger distinguishes authored composition rules from fixture-only labels, counts, values, frame dimensions, and coordinates.

The same ledger drives representative reference review, native capture names, computed-style role assertions, responsive-behavior checks, and the final discrepancy report. Stale screenshots or values copied from the current blockout are not accepted as design input.

Alternatives considered:

- Treating existing CSS or `DESIGN.md` token tables as the complete visual source was rejected because the repository explicitly records the UI as a blockout and Figma remains normative.
- Pixel-copying fixed frame coordinates was rejected because the responsive contract is content-driven and must survive production registry content and reflow.

### 2. Preserve one semantic document and add only generic presentation facts when unavoidable

Sample Detail, Sample Browser, Mixer, Inspector, Engine Options, and Post FX Options continue to render from their existing canonical surfaces, stable paths, lifecycle facts, return paths, and valid actions. JavaScript remains a pure projection-to-DOM adapter plus compatible decimated observation painting; it does not remember a selected row, active asset, Mixer track, scroll-derived focus, or layout mode.

Most fidelity work belongs in shared HTML primitives and CSS. If an authored distinction cannot be expressed from existing projected facts, the smallest generic presentation fact is added to the canonical projector and serializer, with a schema-version update and reducer/projector/serialization correspondence tests. It must describe a reusable concept such as an explicit state label or visualization annotation, not a named capability or Figma fixture.

Sample preview playhead and Mixer meter updates remain latest compatible observations keyed to the projected correlation. They do not enter product state or the serialized session.

Alternatives considered:

- DOM-local modal, browser, or Mixer state was rejected because it would create a second mutation path and break deterministic reprojection.
- Capability-name and effect-name branches were rejected because installed registries own production content.

### 3. Build each surface with content-driven authored primitives

Sample Detail uses the existing Detail subject and ordered control/visualization projection, but receives its own authored Sample hierarchy: Patch/Sample identity, asset and lifecycle readings, waveform/landmark region, ordered controls, and persistent Utility. Sample Browser retains the separate browser composition with folder identity, canonical rows, metadata, waveform, preview/playhead region, and the single shell footer.

Mixer Main remains a sixteen-column stable bank. Each column uses the authored Track Header → Level Fader → Level Readout → Pan Readout → State Line anatomy with a passive compatible meter. The persistent Inspector is correlated to the exact focused track/control and renders the projected routed Patches, sends, returns, return parameters, and global controls. At constrained widths the track bank scrolls inside its region; it does not compress below target or become document-level horizontal overflow.

Engine and Post FX Options continue through the shared generic choice composition. Their remaining work is hierarchy and visual-state reconciliation—identity/origin, list rhythm, focus versus current, disabled/unavailable/progress/failure shapes and text, Utility, and footer—not a new modal or selection workflow.

All surfaces use the Rust-owned color, typography, spacing, radius, keyline, focus, fader, and responsive tokens exported to CSS. Composition uses Grid, Flexbox, intrinsic sizing, `minmax()`, `clamp()`, wrapping, and bounded scrolling. A new value is added to the shared authored vocabulary only when it represents a durable reusable token; it is never introduced merely to nudge one viewport toward a screenshot.

Alternatives considered:

- One universal Detail/modal template for Sample and registry choices was rejected because the authored Sample waveform/preview hierarchy is materially distinct while the semantic ownership remains shared.
- Hiding tracks, Inspector content, or browser rows at narrow widths was rejected because production content and focus targets must remain reachable.

### 4. Exercise responsive behavior without encoding a resolution matrix

The native witness reuses one unchanged serialized projection while the window is resized through a deliberately varied sequence of constrained, intermediate, and expanded conditions. The sequence includes named reference conditions as orientation points, widths between them, content-driven reflows discovered during the run, long labels, maximum acceptance content, and enlarged text. It is not an exhaustive list of pixel widths and does not make any sampled resolution a product mode.

Resize observations record the serialized-document hash, generation, focus, subject, return identity, valid actions, layout mode, viewport, device scale factor, minimum-target compliance, required overlap, document/region overflow, reachable endpoints, observation compatibility key, and native paint acknowledgement. The input and reducer-event ledgers remain empty for resize-only activity. Geometry may be measured to falsify clipping, overlap, inaccessible controls, or unstable reflow; it is not compared to fixed Figma coordinates.

The review actively drags and programmatically resizes the native window so intermediate behavior is visible. If a composition changes abruptly, clips, or creates excess dead space, the implementation is corrected through intrinsic constraints and flexible relationships rather than another viewport-specific exception.

Alternatives considered:

- Testing only named viewports was rejected because transition defects occur between reference frames.
- Exercising every integer width was rejected because it rewards resolution tables instead of responsive reasoning and adds volume without improving the product contract.
- Declaring an arbitrary visual-fidelity ceiling was rejected because the layout should remain content-driven as the available space changes.

### 5. Compare native captures as compositions, not pixel replicas

For each normative comparison, inspect the live Figma frame beside representative production WKWebView captures that express the same state intent. Store a manifest tying the review to the Figma node, retrieval time, observed window condition, native device scale, text scale, fixture identity, serialized-document hash, and paint acknowledgement.

Acceptance evaluates:

- exact projected content, state, focus, and action correspondence;
- shared type, color, spacing, keyline, and state-role usage;
- hierarchy, relative emphasis, reading order, grouping, balance, and responsive region relationships;
- minimum-target compliance, reachability, wrapping, scrolling, overlap, and overflow behavior;
- behavior while resizing between representative captures; and
- an empty meaningful-discrepancy ledger, or explicit remaining discrepancies that keep the affected surface incomplete.

Side-by-side captures support review, but fixed-resolution reproduction, absolute-coordinate matching, opacity overlays, pixel diffs, and perceptual similarity scores are not acceptance gates. A responsive composition may legitimately place or size content differently from a static Figma example while remaining faithful to its hierarchy and interaction priorities.

Alternatives considered:

- A pixel-difference score was rejected because it optimizes raster coincidence rather than responsive product quality.
- Manual review without semantic, reachability, and resize evidence was rejected because it would not be reproducible or falsifiable.

### 6. Finish slices before shared polish

Work proceeds through Sample Detail/Browser, Mixer/Inspector, then Engine/Post FX Options. Each slice first closes its live-Figma comparison, state matrix, responsive sweep, and existing workflow regressions. Only then may shared shell chrome and token polish be applied across the full set.

Any shared-token or shell change reruns the affected Sample, Mixer, option, Patch Overview, and Instrument/FX Detail native witnesses. Patch Overview and Instrument/FX Detail are regressions in this phase, not newly claimed fidelity surfaces unless their own direct comparison evidence is deliberately refreshed and recorded.

Alternatives considered:

- Global polish first was rejected because it obscures which surface owns a discrepancy and causes repeated cross-surface rework.

### 7. Keep acceptance on the production path and update as-built status last

Functional correspondence fixtures drive `AppState::apply`, semantic resolution, projection, serialization, the committed page, native paint, normalized keyboard/controller input where the existing workflow requires it, and compatible observations. Controlled negatives cover stale meter/playhead data, unavailable waveform, long labels, maximum acceptance registries, empty content, typed failure, and resize with no semantic event.

The audio callback is not entered for visual work beyond existing production observation behavior, and no test-only reducer or renderer may stand in for the shipped path. `DESIGN.md` is updated only after the final evidence run, naming what actually passed, the tested interval and native scale, any generic projection additions, and every remaining limitation. OpenSpec is never cited as proof.

## Risks / Trade-offs

- [Risk] The live Figma file can change during implementation. → Record node identity and retrieval time in the ledger and capture manifest; re-read affected nodes before final comparison and invalidate stale baselines.
- [Risk] Reviewers may still treat a static Figma frame as a coordinate template. → Put the responsive guidance in both the live Figma contract and the evidence manifest, and reject viewport-specific exceptions introduced only to match a screenshot.
- [Risk] Varied-width sampling can miss an intermediate layout defect. → Combine automated varied conditions with live manual resizing, content stress, overflow/overlap instrumentation, and focused regression cases for any discovered reflow.
- [Risk] Shared CSS/token changes can regress already completed surfaces. → Gate shared polish on per-surface completion and rerun Patch Overview and Instrument/FX Detail native regressions after every affected shared change.
- [Risk] Long production content may require scrolling unlike compact Figma fixtures. → Preserve authored hierarchy and target rhythm while using bounded region scrolling; record the fixture/content distinction in the comparison ledger.
- [Risk] A missing projected visual fact could tempt renderer-owned state. → Require a generic canonical projector addition, schema/version proof, and resize-neutral byte correspondence before using the fact.
- [Trade-off] Representative captures cannot depict every possible size. → Treat captures as review evidence, keep layout rules intrinsic, and report the resize/content conditions actually exercised without converting them into a fixed support matrix.

## Migration Plan

No persisted data, capability registry, product workflow, or audio graph migration is required. Land the work as independently verifiable visual slices: measurement/capture infrastructure, Sample, Mixer, options, then shared polish and final evidence. Each slice keeps the prior semantic projection contract or introduces a separately tested compatible schema addition. A slice can be rolled back by reverting its renderer/token/harness changes without session migration; `DESIGN.md` completion status is written only after the full evidence gate passes.
