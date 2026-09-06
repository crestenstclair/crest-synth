# Crest Synth — Master Design and As-Built Reference

The linked Figma file is the normative product, visual, and interaction source
of truth. This document is the repository's detailed as-built architecture,
invariant, and implementation-status authority. It records the system that
exists, the constraints that must survive changes, the available evidence, and
the known mismatch with the authored interface. `AGENTS.md` is the short
working contract and invariant summary; it does not replace this reference.
This file is not a roadmap, a phase plan, or a claim that the product is
finished.

Snapshot date: 2026-08-28. The implementation reviewed for this reset began at
commit `d2d257f`.

## Authority and maintenance

Read this file before changing product behavior or architecture.

The user's functionality-first direction governs visual acceptance: Figma
guides hierarchy and interaction, while readable controls, usable workflows,
truthful state, and stable focus determine completion. Exact typography,
spacing, and pixel correspondence are not completion gates. Previously accepted
resize behavior does not require another manual handoff unless a new change
or observed regression affects it.

- OpenSpec change artifacts may complement Figma with scoped intent,
  acceptance criteria, design reasoning, and implementation tasks. They are
  temporary planning material, not a normative product definition or proof of
  as-built behavior.
- Keep OpenSpec changes aligned with the live Figma contract and the
  architecture and invariants recorded here. If they conflict, Figma governs
  product, visual, and interaction intent, while this document governs the
  constraints and status of the production implementation until deliberately
  updated with evidence.
- Do not introduce a roadmap, CUE DSL, planning kit, parallel master design, or
  another competing source of truth into this repository.
- Use source code and production-path tests to describe current behavior. Use
  this file for durable architecture, product invariants, and explicit gaps.
- Use OpenSpec, issues, and commit messages for temporary intent, sequencing,
  acceptance notes, and handoffs.
- A narrow implementation slice must not redefine the product.
- Figma examples of engines, effects, patches, files, values, and option counts
  are fixtures. They demonstrate composition and interaction; installed
  capability registries determine production content.
- If durable behavior or architecture changes, update this file in the same
  commit and add falsifiable proof through the production reducer and render
  path.

## Current alignment status

The current interface is a functional webview projection/blockout. It is **not
a faithful implementation of the Figma design**, and presence of a component,
surface, DOM witness, or passing reducer test must not be presented as visual
or workflow parity.

The authored screens contain substantially more composition, hierarchy,
spacing, typography, state treatment, and workflow detail than the current UI.
The Figma responsive contract defines fluid, content-driven Wide, Standard,
and Compact compositions; its large frames are visual references, not fixed
canvases. Layout uses bounded tracks, intrinsic sizing, wrapping, and ordered
stacking while semantic focus and projection remain unchanged. The current
native responsive witness has measured Patch Overview and Instrument/FX Detail
at 1920×1080, 1440×900, 1280×800, 900×800, and 1280×800 with 1.25 text scale
through the shipped WKWebView. Those observations passed structural-mode,
48 px target-floor, document and required-content overflow/overlap,
scroll-endpoint reachability, singular focus identity/treatment,
paint-acknowledgement, resize-neutral projection, and repeat-render checks.
Readable native screenshots were captured and the enlarged-text Detail image
was inspected directly. This is native structural/render evidence, not broad
visual or workflow parity with Figma. The physical keyboard/controller handoff
for the Detail slice passed on 2026-08-26 through the production native app,
including entry, navigation, representative fine/coarse edits, exact
Engine/effect-slot return, resize invariance, and repeated Shift use; the app
then exited cleanly. Mixer multi-select has a type/state name but no defined
product semantics and is truthfully reported as not implemented.

The Instrument/FX Detail slice now uses one `detailShellHtml` composition for
both reducer-owned `PatchDetailSubject` variants. It reads Patch and subject
identity, exact return origin, canonical FX position, authored labels, ordered
sections and controls, scalar bounds/position, units, declared interaction,
valid actions, lifecycle/error/requested fields, and optional non-focusable
visualizations from the serialized semantic model. The only generic projection
addition is `focusRepair`, which names a removed Overview return origin and the
enabled sibling chosen by reducer-owned path repair. The shared header, section
hierarchy, flat indexed rows, state keylines/text, independently scrolling
Detail body, and bounded persistent Utility track were compared with the live
Instrument Detail `37:7`, FX Detail `38:60`, Responsive `98:2`, and Interaction
Map `49:3` hierarchy. This is a structural implementation comparison, not
visual parity.

Projected action guidance now has one visual owner: the persistent shell
footer. Per-control `validActions` remain in the immutable semantic document
for reducer admission and falsifiable correspondence, but the renderer no
longer repeats the same key legend in Overview rows, Detail rows, workspace
caption/mode bands, Utility/Inspector, or option rows. The non-focusable
Envelope visualization is a parameterized SVG whose horizontal display
coordinate is a stable `log1p` transform of milliseconds. Attack, decay, and
release retain their projected physical durations and canonical 10,000 ms
maximum, but each timed phase contributes `log1p(value) / log1p(10_000)` of one
equal graphical phase span. Sustain contributes one equal, explicitly
non-temporal preview span because it has level but no duration parameter. This
gives short and medium musical times readable space without normalizing against
the current envelope: 100 ms occupies about 12.5% and 5 s about 23.1% of the
complete four-span plot. Changing one phase therefore never rescales another,
zero-duration stages remain vertical, and the painted readout labels A/D/R in
`ms` and sustain as a percentage. The straight SVG segments match the
production envelope's linear sample-domain increments. The semantic
visualization projects each timed phase's maximum from its canonical envelope
descriptor; the page does not duplicate those physical bounds.

Numeric row presentation follows projected semantics rather than a fixed
three-decimal template. The renderer uses each control's `numericRange`,
`fineStep`, and authored unit to remove meaningless trailing zeroes while
preserving meaningful precision. Unitless normalized (`0..1`) and bipolar
(`-1..1`) controls render as percentages, millisecond controls remain in `ms`,
and gain controls render in `dB`. The global Master Volume descriptor owns its
`dB` unit so PATCH Utility and MIXER Inspector receive the same canonical unit.
Active values, requested values, and range endpoints share this formatter.

Deterministic evidence covers differently shaped instrument and effect
descriptors, every occupied FX position, duplicate effect capabilities, an
empty slot, long content, visible dependency-disabled controls, and distinct
Loading, Validating, Preparing, Activating, Unavailable, and Failed states
across 29 reducer/projector/serialization documents. Production keyboard
normalization proves Shift+Up entry and Shift+Down close with exact Engine/slot
return. Focused Rust witnesses prove descriptor order, exactly one visible
focus while arrows traverse every projected Detail section and switch between
Detail and Utility, fine/coarse editing, action admission, resize-neutral
semantic identity, visible return-origin repair, and AppKit `FlagsChanged`
safety. The headless webview path passes exact serialization, token, CSP, and
typed-startup checks. After the host restart, the real WKWebView witness passed
all Detail fixtures at Wide, Standard, Intermediate, Compact, and scaled-text
conditions, including deterministic repeated observations, exact focus and
state treatment, 48 px rows, complete independent scrolling, zero document or
required-content overlap, and six resize-only observations over one unchanged
serialized projection. That run also retained all sixteen Mixer tracks,
Inspector correlation, focus identity, and reachability. Visual inspection of
the first enlarged-text capture exposed a flex-shrink overlap that box-only
measurement missed; the Detail scroll-column blocks now retain natural wrapped
height, and the witness measures descendant-painted overlap to prevent its
return. A scoped `make test-webview-detail-native` target runs this real-window
Detail/Mixer proof and closes automatically, while honestly skipping unrelated
soak and deliberately uncloseable fault-injection scenes. Live physical
keyboard/controller operation for any later slice is not inferred from the
native render witness.

Engine selection now exposes the reducer-owned sequence Loading → Validating →
Preparing → Activating → Ready, with Unavailable and Failed as distinct typed
outcomes. The acknowledged source capability, configuration, and Overview/
Detail readings remain canonical during Activating; the complete prepared
candidate stays separate until graph activation acknowledgement. Compatible
scalar snapshots target the prepared revision during that interval, while the
source renderer ignores them by revision. No capability, asset, or value is
silently substituted.

Engine and Post FX option selection now use the existing reducer-owned generic
Choice session and one shared option DOM composition. Installed instrument and
effect descriptors provide stable row identity, authored label, registry order,
and generic enabled/unavailable metadata; the canonical Empty effect choice is
prepended independently for every slot. Visible membership includes unavailable
rows for truthful explanation, while focus membership and activation include
only enabled rows. Opening focuses the current enabled identity or the first
enabled row, `CURRENT` remains independent of focus, and choosing the current
row is an accepted no-op close. A changed choice emits exactly one correlated
Engine or slot request through `AppState::apply`; Shift+Down closes unchanged,
and return uses the exact Overview origin or the stable-order repair path.

The only new generic serialized presentation fact is the optional
`availabilityLabel` on an existing semantic control; state-tree schema version
18 covers that addition. The option projection otherwise reuses canonical
Choice subject/origin, stable controls, valid actions, active/requested
lifecycle, typed failure, revision, and focus-repair facts. The renderer reads
only that projection. It publishes an identity header, exact Engine or slot
source, ordered rows with independent structural focus/current markers,
explicit availability, origin-anchored lifecycle, and projected action
guidance. Sample Browser remains on its separate preview and file-navigation
composition.

The shared hierarchy and action grammar were compared directly with live Figma
Engine Options `48:173`, Post FX Options `48:207`, Interaction Map `49:3`, Patch
Overview `95:202`, and Responsive Contract `98:2`. The production renderer adds
projected lifecycle/revision/cause and per-row action facts that the compact
Figma fixtures do not enumerate, and its generic footer wording is denser than
the authored D-pad/Edit/Shift legend. Those are recorded scoped discrepancies;
the implementation is structural and responsive evidence, not a pixel-parity
claim or evidence for Sample or Mixer visual composition.

The real WKWebView option witness passed Wide, Standard, Intermediate, Compact,
1280×800, and enlarged-text fixtures with maximum registry content, long
labels, every option lifecycle, occupied and empty slots, and duplicate effect
capabilities. It measured one focus, independent current state, 48 px target
floors, reachable scroll endpoints and Utility, no required-content overlap or
document horizontal overflow, deterministic repeated paint, resize-neutral
semantic identity, normalized keyboard/controller journeys, exact and repaired
return, and clean shutdown; Detail and all sixteen Mixer tracks remained in the
same run as regressions. A bounded production-app physical handoff script exists,
but no interactive operator and controller were available in this session, so
physical option entry/navigation/choice/return and teardown remain unverified
and are not inferred from translation tests or native input injection.

Detail and Utility share one reducer-owned subordinate session. Unmodified
Right moves from Detail to Utility, Left restores the exact stable Detail
focus, and a subsequent Left returns to the exact Overview origin. If schema
change removes that origin while Detail is open, the reducer repairs the
return path to the nearest enabled stable Overview sibling and the projection
publishes the explicit `focusRepair` notice. Remaining visual slices are
Sample Detail/Browser, Mixer composition, and native polish. The Engine/Post FX
option slice still has the physical handoff and the scoped visual refinements
recorded above outstanding; neither its functional predecessor nor its new
structural witness implies broad Figma parity.

The old phase/spec system recorded 73 of 79 implementation tasks complete when
it was retired. The six incomplete items were the two native viewport
comparisons, native input/window/teardown proof, the aggregate deterministic
webview/gate run, the physical live target run, and final completion evidence.
Historical deterministic and physical-audio reports are evidence of functional
and real-time behavior only; they are not evidence of Figma fidelity.

## Visual and interaction reference

The normative product, visual, and interaction source of truth is the live
[Crest Synth — Controller-First UI Redesign](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=0-1).
The primary authored nodes are:

- [Responsive Front-End Contract — 98:2](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=98-2)
- [Patch Overview — 95:202](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=95-202)
- [Instrument Detail — 37:7](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=37-7)
- [FX Detail — 38:60](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=38-60)
- [Sample Detail — 39:92](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=39-92)
- [Sample Browser — 41:138](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=41-138)
- [Mixer — 42:3](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=42-3)
- [Engine Options — 48:173](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=48-173)
- [Post FX Options — 48:207](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=48-207)
- [Interaction Map — 49:3](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=49-3)

The inspected Figma nodes contain no prototype reactions. Their annotations,
screen states, component specimens, and interaction map remain valid design
evidence, but click-through behavior cannot resolve every edge case. In
particular, the file says Select enters multi-select without defining selection
accumulation, batch operations, cancellation, or a complete visual treatment.
Product behavior resolves Sample Browser preview as holding Start on a focused
sample row and stopping on release; Start is reserved elsewhere.

## Product as built

Crest Synth is a standalone, controller-first MIDI instrument host. The
production composition is a Rust application using a Tauri v2/WKWebView shell
and CPAL stereo audio output. Normal startup opens one clean, playable Sample
`INIT` document and starts a bounded repeating test MIDI pattern on channel 1.
`T` stops or restarts the pattern; the footer reports the available action.
Test playback pauses in Sample Browser and during session replacement. The
parsed `midi/Radiohead - Everything In Its Right Place - HiDef Compatible.mid`
fixture remains isolated to explicit demo and witness entry points. That
derived fixture differs from the retained source MIDI only by changing its
unavailable percussion bank-128/program-118 request to the installed
bank-128/program-0 drum kit; runtime capability fallback remains forbidden.
Crest Synth is not a DAW, arranger, or general-purpose sequencer.

The application currently provides:

- PATCH and MIXER as the only top-level contexts;
- sparse stable Patch identities with an active graph capacity of 16 Patches;
- HiDef SoundFont and Braids instrument capabilities in the default registry;
- Sample installed by default with `Test Tone.wav` in
  `~/Music/Crest Synth/Samples`; the bundled WAV seeds a new library without
  overwriting an existing asset. `CREST_SAMPLE_LIBRARY_ROOT` and
  `CREST_SAMPLE_DEFAULT_ASSET` remain paired, validated overrides;
- Chorus, Reverb, and Delay in one effect registry;
- three ordered Patch post-effect slots and eight bus returns;
- one fixed bank of sixteen persistent Mixer tracks, T00 through T0F;
- descriptor-driven Patch Overview, Detail, generic Choice, Sample Browser,
  waveform/playhead projection, persistent Utility, and persistent Mixer
  Inspector surfaces;
- channel-based MIDI subscriptions in which any number of installed Patches
  may share one channel and each incoming message fans out to every matching
  Patch in stable installation order;
- physical MIDI input discovery, exact opaque-identity selection, connection
  lifecycle, per-user selected-input preference, and a temporary Settings ·
  MIDI Devices system surface that suspends and restores the exact PATCH or
  MIXER focus without adding a third top-level context;
- level, pan, mute, solo, indexed sends, return occupancy/parameters/levels,
  pre-gate meters, Patch route/trim, and master gain;
- versioned saved state containing stable asset references and normalized
  control values, never decoded PCM or device state;
- native File-menu New, Open, Save, Save As, and guarded Close workflows over
  one shell-owned Untitled-or-path document identity and content-based clean
  baseline;
- retained deterministic, headless, controlled-negative, and physical-demo
  commands listed later in this file.

Normal `make run` resolves the composition-root-designated Sample capability
exactly and constructs Patch 1 `INIT`: MIDI channel 1, T00 at 0 dB Patch trim,
neutral envelope, capability-seeded voice limit, and three empty post-effect
slots. It also installs the default sixteen-track Mixer, 0 dB master, and the
production return bank. The captured versioned session and its complete graph
are validated and prepared before audio or the window starts; a missing or
invalid designated capability is a typed fatal startup error, never registry
fallback. The initial graph receives the explicit test pattern through the same
control-side channel fan-out as physical MIDI. The pattern emits at most two
MIDI edges per window tick and never replays a catch-up burst. Its enabled flag
changes only through `AppState::apply`, stays outside saved-session content, and
pauses during browser audition and session replacement. `automaticMidi` remains
distinct from `physicalMidi` in input evidence. Neither source emits Patch targets.
The physical path uses `midir = 0.11.0`: CoreMIDI on Apple, ALSA on Linux, and
WinMM on Windows through default features. JACK and WinRT remain explicit
packaging-time opt-ins and are not enabled in the default build. Adapter
initialization, enumeration, port information, connection, retirement,
preference, and transport failures are typed; an unavailable backend leaves
the app open with truthful Settings state and never selects a similar name or
another port. The `demo-live-*` commands remain input-isolated bounded
autonomous witnesses and do not attach host MIDI devices.

## Architecture

```text
keyboard / controller / automatic or physical MIDI source
                       |
              physical input adapters
                       |
              SemanticAction / AppEvent
                       |
                 AppState::apply
                       |
          +------------+-------------+
          |                          |
 immutable semantic view      fixed audio projections
          |                          |
 Tauri webview projection    events / snapshots / graphs
                                     |
                              CPAL callback
                                     |
                           bounded stereo output
```

The non-negotiable mutation path is:

> physical input → semantic action/event → `AppState::apply` → view/audio
> projections

`AppState::apply` is the only product-state mutation path. Invalid events return
typed rejections and leave state and generation unchanged. UI, persistence,
MIDI, device, controller, asset, and serialization code are thin adapters.
Views receive immutable host-neutral data and emit semantic actions; they do
not own Patch, Mixer, focus, browser, or audio state.

The code is organized around hexagonal boundaries:

- domain types own Patches, capabilities, parameters, routing, Mixer state,
  assets, and their invariants;
- application/control code owns `AppState`, reducer transitions, semantic
  resolution, projection, orchestration ports, and correlation lifecycles;
- real-time code owns fixed destructor-free snapshots, prepared render graphs,
  queues, voices, buses, and bounded scratch;
- adapters own SoundFont/WAV parsing, filesystem access, MIDI fixtures,
  physical devices, native input, and Tauri integration;
- `webview-page/` paints the serialized semantic projection and emits no
  independent domain model;
- tests drive the same reducer, projection, preparation, and render seams used
  by the standalone binary.

There is one canonical public type per concept. Private transport values may
exist inside adapters, but duplicate public `Patch`, `MidiEvent`, `Parameter`,
focus, asset, or routing models are forbidden.

## Canonical state and interaction invariants

Canonical state is divided conceptually into session, interaction, and runtime
data. Session data contains Patches, instrument/effect configurations, routes,
Mixer values, and stable asset references. Interaction data owns the top-level
context, singular semantic focus, mode, return path, and subordinate session.
Runtime data owns device/load status, graph generation, and correlated
structural-edit lifecycle—not device handles, decoders, prepared graphs, or
audio buffers.

- Focus uses stable semantic IDs, never rectangle coordinates, DOM positions,
  collection indices, or labels.
- Exactly one target is focused.
- Presentation reflow dispatches no action and cannot change track, row,
  Patch, subordinate subject, or return identity.
- PATCH and MIXER remember their prior semantic roots.
- One `PatchSubordinateSession` union owns Detail, Choice, or Sample Browser;
  subordinate surfaces do not stack.
- Choice/browser rows use stable choice or library-relative identities.
- Closing a subordinate surface restores the exact origin when valid; if a
  schema change removed it, the reducer selects the nearest enabled semantic
  sibling deterministically and exposes the repair in status.
- Requested and active structural values remain distinct through Loading,
  Validating, Preparing, Activating, Ready, Unavailable, Invalid, Cancelled, or
  typed failure states.
- Only one structural request is in flight application-wide. Failure, busy,
  early, stale, mismatched, or cancelled results never select a fallback.
- A complete graph change may reset voices and effect tails; seamless migration
  is not claimed.

The installed capability registries own available choices and ordered schema.
Reducers, projectors, racks, renderers, and demo orchestration must not switch
on concrete capability names to define fields or availability.

## Hard real-time invariants

The audio callback performs bounded, preallocated work. It must never:

- allocate, deallocate, grow a collection, or drop the last reference to owned
  state;
- lock, wait, sleep, block, or invoke an API that may do so;
- perform file, console, network, MIDI-device, or UI I/O;
- parse, decode, resolve paths, resample, prepare effects/assets, or build a
  graph;
- log, format strings, panic, unwind, or destroy retired state;
- perform unbounded traversal, recursion, or capability discovery.

Different real-time data has different transport semantics:

| Data | Transport | Rule |
| --- | --- | --- |
| Notes, expression, preview, bounded commands | fixed-capacity SPSC event ring | ordered, block-relative offsets |
| Scalar parameters | triple-buffered fixed `ParameterSnapshot` | latest compatible complete snapshot wins |
| Structural changes | ownership-transfer queue of prepared graphs/assets | swap at block boundary; retire off-thread |
| Meters and RT health | atomics/latest `AudioObservationSnapshot` | decimated and polled by UI |
| Physical MIDI activity | latest compatible control-side snapshot | exact active revision, decimated to at most 30 Hz |

Each asynchronous producer owns its SPSC queue or is merged on the control
side. A single-producer queue is never shared casually among UI, MIDI, and
workers. Queue pressure is explicit; note-off/all-notes-off has reserved
recovery behavior, replaceable scalars may be coalesced, and silent loss is
forbidden.

Each physical MIDI candidate owns a separate 1,024-event `rtrb` SPSC ingress,
immutable connection revision, atomic enable gate, and fixed diagnostic
counters. The midir callback performs bounded status-byte classification and
normalizes only supported channel messages through `midly`; it never owns or
reads `AppState`, allocates, locks, logs, formats, blocks, or traverses a SysEx
payload. Control drains at most 64 events per tick and rejects every event
whose revision is not the reducer-acknowledged active revision. Activity is a
presentation-only latest snapshot (count, last message/timestamps, diagnostics,
overflow epoch, revision); it never mutates or serializes into product state.

Connection preparation, discovery, retirement, and preference I/O run on one
bounded device worker. A candidate ingress stays disabled until matching
reducer acknowledgement. Switch, disconnect, loss, ingress overflow, audio
command saturation, and shutdown disable/discard ingress before global
all-notes-off recovery; the audio command ring reserves one slot for that
coalesced recovery command. Backend handles move to the device worker for
consuming close/destruction, including shutdown-owned and stale prepared
handles.

`ParameterSnapshot` is fixed-size and destructor-free. A worker constructs and
warms a complete graph—engines, effects, sample PCM/zones, routing, delay
memory, return topology, and scratch—before publication. The callback swaps at
a block boundary and transfers old graphs to a bounded return queue for worker
destruction. If retirement is back-pressured, callback-owned bounded retention
retries; the callback never destroys the graph.

Physical audio startup is two-stage: negotiate and validate a device config
without starting the stream, prepare the complete graph for its actual sample
rate and bounded block capacity, then let the owning device adapter start the
prepared renderer. Oversized native callbacks are rendered as consecutive
bounded blocks, never truncated. Device errors cross back through fixed atomic
status and are formatted/handled off callback.

## Audio/domain invariants

- A Patch owns a stable `PatchId`, label/MIDI mapping, instrument config,
  per-note `VoiceEnvelope`, up to three ordered post-effect slots, output
  `MixerTrackId`, Patch-local trim, and voice limit.
- A Patch MIDI channel is a subscription, never an exclusive owner. Multiple
  Patches may subscribe to the same channel; one incoming message fans out to
  every current subscriber in stable installation order. Changing a
  subscription is immediately effective control state and does not rebuild the
  structural graph. A channel with no subscribers emits no audio command.
- Mixer state owns exactly sixteen persistent tracks independently of the
  Patch collection. Multiple Patches may share a track and empty tracks remain
  configurable.
- Patch trim and route are not track controls. A Patch never owns track level,
  pan, mute, solo, send, or meter state.
- Master gain has one canonical owner, projected in PATCH Utility and MIXER
  Inspector.
- The maximum active prepared Patch count is 16. Each Patch has three effect
  slots; each effect slot has at most eight scalar parameters; each instrument
  has at most sixteen scalar parameters; there are eight bus returns.
- Effect slot order is render order. Topology edits prepare and exchange a
  complete graph; there is no silent bypass or render-time graph mutation.
- Reverb occupies return 0 and Delay return 1 by default; returns 2–7 start
  empty. Registry failure is typed rather than substituted.
- A Patch voice limit is 1–64 and refuses a new note at capacity rather than
  stealing an existing voice. Engine-native stealing rules still apply inside
  the admitted limit.
- Every engine applies the Patch-owned ADSR to independent native note voices;
  one post-stem envelope is nonconforming.
- SoundFont uses one synthesizer per Patch with engine-managed polyphony.
- Each Braids Patch owns exactly 16 voices and 47 named models; voice capacity
  scales as `16 × active Braids Patch count` with no global Braids pool.
- Each Sample Patch owns one asset and exactly 16 voices. Allocation order is
  inactive, oldest releasing, then oldest active.
- Unsupported assets, engines, effects, devices, rates, and topology states
  produce typed visible errors. No engine/effect/asset silently substitutes or
  falls back.

The signal flow is fixed:

```text
MIDI
  ↓
Patch instrument
  ↓
ordered Patch post FX
  ↓
Patch trim
  ↓
route and sum into one of 16 tracks
  ↓
track level / pan
  ├──→ pre-gate meter
  ↓
mute / solo gate
  ├──→ post-gate sends[0..7] → returns[0..7] ─┐
  ↓                                           │
16-track dry mix ←────────────────────────────┘
  ↓
master gain / safety limiter
  ↓
stereo device
```

Mute always wins. If any track is soloed, only soloed, non-muted tracks
contribute dry signal or sends. Sends are post-fader and post-gate. Meters are
post-level/pan but pre-gate so muted tracks remain diagnosable. Feedback may
exist only inside bounded effect implementations, never as an arbitrary graph
cycle.

## Asset and persistence boundaries

The selected physical MIDI identity is an application preference, not synth
session content. The shell resolves `midi-input.json` beneath the per-user
configuration directory (`~/Library/Application Support/crest-synth` on
macOS, `$XDG_CONFIG_HOME/crest-synth` or `~/.config/crest-synth` on Linux, and
`%APPDATA%\crest-synth` on Windows). Version 1 stores only the opaque identity
schema/value and last-known display name, using a temporary file plus rename.
Manual disconnect, runtime connection state, descriptors, handles, queues,
timestamps, diagnostics, and activity are never persisted. `SavedSession`
remains version 2 and contains none of those device fields.

SoundFont and Sample file work is off callback: resolve, validate, read, parse,
decode, resample/precompute, allocate/warm voices, then publish a complete
prepared graph. Prepared callback ownership contains numeric PCM, zones,
addresses, landmarks, and bounded voice state—not paths, names, parsers, or
decoders.

SoundFont preset identity is numeric bank/program; labels are exact authored
SF2 names. Ordering is bank then program, with source ordinal only for malformed
coordinate collisions. The application does not invent General MIDI names or
choose a nearby preset.

The Sample adapter uses exact `hound = 3.5.1` (Apache-2.0, no optional
features). It accepts uncompressed RIFF/WAVE only: mono/stereo signed
PCM16/24/32 or IEEE float32 at 8,000–192,000 Hz. Limits are 256 MiB and 300
seconds per source, 28,800,000 scalar `f32` samples per prepared asset, 512 MiB
of deduplicated Sample PCM per complete graph, and at most 2,048 waveform
min/max pairs. RF64, RIFX, compression, non-WAV data, extra channels, malformed
chunk arithmetic, non-finite samples, and over-limit assets are typed failures.

Sample configuration stores a relative asset ID, root note 0–127 (default 60),
normalized playback/loop landmarks, OFF/FORWARD loop mode, crossfade capped by
200 ms or half the loop, and the shared envelope. Preview is original-pitch,
file-start, no-loop audio through the origin Patch's post-FX, trim, Mixer,
sends, returns, and gates. Stop uses a prepared 5 ms de-click. Preview never
commits an asset or mutates saved state.

Return or Shift+W on Sample File opens the same in-app FileBrowser page.
The page uses the existing semantic navigation, confirm, back, and preview
controls; no platform asset dialog is involved. W/S navigates rows, Return
enters folders or selects a file, Shift+S returns unchanged, and holding Space
on a Sample file auditions it. Parent and Cancel are reachable list rows.
`FileBrowserState` owns the exact origin and focus; the page remains a PATCH
subordinate surface, never a new top-level context.

Shared `AssetFileId`, `FileBrowserFolderId`, row, and listing types are defined
in `synth::file_browser`. `FilesystemFileBrowser` owns directory navigation and
WAV/SF2 filtering by `AssetKind`; engine adapters own decoding and metadata.
This is the shared picker foundation for Sample and SoundFont. The current
SoundFont engine still uses its fixed HiDef bank; arbitrary SF2 loading and
asset-dependent preset catalogs are not claimed by this picker correction.
The serialized projection schema is version 23 (`fileBrowser` surface identity,
with unavailable metadata distinct from invalid audio).

The library root also offers Home and, on macOS, Volumes locations. External
location identities are transient and cannot be restored as saved assets.
Selecting an external Sample emits a reducer-owned import request correlated
to its origin, generation, and graph revision. A capacity-one shell filesystem
worker validates it and copies it into `Imported/` with content reuse and no
overwrite. Only its resulting library-relative identity enters structural
assignment; cancellation, failure, and stale completion preserve the active
asset. Browsing and importing never block the callback or window tick. Nested
folder results select their first row initially, then preserve stable focus
on refresh. The projected asset row shows `BROWSE`; it is not a scalar edit.
The footer remains the sole keyboard-guide owner.

On macOS, the filesystem adapter checks zero-byte files for Dropbox's legacy
placeholder attribute before WAV parsing. These report `DownloadRequired` and
show Make available offline guidance in the browser and after failed import;
the active asset remains unchanged. Empty local files report `EmptyFile` rather
than an unsupported container. Downloaded WAV bytes take precedence over any
retained placeholder attribute. Selecting again re-reads the file; leaving and
re-entering its folder refreshes metadata. Crest does not initiate cloud
downloads or change Dropbox storage settings. This correction addresses legacy
zero-byte Dropbox placeholders; other cloud-provider hydration is not claimed.
The regression sets the real macOS Dropbox attribute on a temporary empty
file, checks listing/read/import failures, then replaces it with WAV bytes and
verifies successful import even with retained metadata. Production-catalog
inspection of an existing Dropbox tracker WAV confirmed the typed result.
Focused Sample tests and native browser paint verified actionable text,
unavailable state, retry, and unchanged saved assignment/focus on failure.

Sample Detail renders bounded min/max waveform bars with HTML/CSS, including
both source channels, and current descriptor-linked playback/loop landmarks.
Missing or asset-incompatible summaries produce `WAVEFORM UNAVAILABLE` without
invented geometry. Prepared summaries become acknowledged control-side data
on both Engine selection and asset assignment; the previous active waveform
remains visible while replacement is pending. Scalar landmark edits do not
replace the decoded summary. The engine-choice regression in
`tests/phase7_sample_workflows.rs` exercises options, worker preparation,
audio block activation, acknowledgement, and Detail projection without first
assigning a file in the browser. Prepared session replacement also carries the
waveform summaries belonging to its complete graph, hydrating startup and Open through `AppState::apply` without
storing waveform caches in the saved document. Newly appended Sample Patches
retain the summary supplied by their topology preparation.

Saved state is versioned and stores canonical control state and stable relative
asset references. It never stores decoded PCM, absolute library roots, browser
or preview sessions, document paths, dialog or dirty state, prepared graphs, or
device state. The shell owns `Untitled` or one exact path, an exact typed
`SavedSession` clean baseline, pending continuations, operation status, and
typed lifecycle failures. Its immutable window projection exposes only the
leaf document name, dirty marker, operation/status text, and error marker/text;
full paths remain inside native dialog and filesystem boundaries.

New and Open share one capacity-one candidate worker. Open reads bytes before
the worker performs decode → supported migration → validation → canonical
reconstruction/projection → complete graph preparation. The control side
admits navigation and performance during that work but refuses saved-field
actions once New, Open, or guarded Close has been authorized, preventing a
later edit from being discarded without another decision. Direct Save and
Save As continue to admit edits so the exact captured baseline can complete
while later work remains visibly dirty. The control side then
preflights the private one-shot persisted-content replacement against a cloned
state and the prepared graph's initial parameters. Session replacement then
uses the same application-wide structural coordinator as engine and topology
changes: MIDI admission is gated with bounded all-notes-off recovery, the
complete graph activates at a block boundary, and one correlated
`ReplacePersistedSession` event commits through `AppState::apply`. Failed,
busy, mismatched, or stale candidates are retired on the control side; the
prior capture, state projection, graph revision, render output, and input path
remain usable. No callback destroys a graph.

Save workers receive an immutable typed capture and a content token. The
filesystem adapter encodes version 2 JSON into a unique same-directory sibling,
writes, flushes, syncs, atomically renames, and syncs the parent directory.
Identity and baseline change only after correlated success, so a later edit
remains dirty against the exact written capture. Create/write/flush/sync/rename
and encoding failures stay typed and do not truncate an existing destination.
Dirty New, Open, and Close use one resumable Save/Discard/Cancel state machine;
cancellation is a non-error, while failures remain visible without fallback.

PATCH interaction now derives one trailing empty position after the created
Patch order. `PatchPositionId::Created(PatchId) | TrailingEmpty` is the
canonical interaction identity for focus, remembered roots, subordinate
sessions, return paths, and focus suspended by MIDI Settings. `TrailingEmpty`
is not a `Patch`, has no numeric ID, route, MIDI subscription, parameter entry,
or graph slot, and never enters `SavedSession`. Shift+Right reaches it through
the same non-wrapping semantic Patch-navigation action used between created
Patches; Shift+Left returns to the final created Patch. Navigation and
prospective Overview/Detail/Choice/Utility inspection do not change saved
capture, graph revision, or audio.

One injected immutable `PatchCreationBlueprint` resolves the designated
provider-authored Engine default used by both the initial-session factory and
prospective creation. A first accepted Patch-owned edit reserves the greatest
created `PatchId` plus one, derives `Patch {id}`, appended MIDI channel and
Mixer track, 0 dB trim,
neutral envelope, seeded voice limit, and three empty effect positions, then
applies only the triggering edit. The pending candidate stays outside the
created Patch vector. Loading, validation, preparation, failure, activation,
and the final commit all pass through `AppState::apply`; only a matching graph
activation acknowledgement appends the candidate, rekeys any still-live empty
focus/return identity, advances the graph revision, and changes saved content.

Creation reuses the capacity-one structural worker and coordinator.
`GraphReplacementScope::AppendPatch` admits only an identical prior Patch,
return, and device layout plus one exact final candidate. The worker prepares
the complete graph off callback. Before staging, the control side applies the
future commit to a clone, compares exact revision/order/identity/scalar shape
and candidate values with the prepared graph, and refreshes prior scalar
values accepted during preparation. Existing live engines/effects/returns are
carried across the block-boundary swap where identities and layouts agree; the
new engine starts silent. Failure leaves the prior session and graph usable,
and prepared ownership retires off callback.

`MAX_ACTIVE_PATCHES` is the single control, persistence, graph-layout, and RT
array authority and remains 16. At capacity, the trailing empty interaction
position is still reachable and projects `CAPACITY 16/16`, but creating actions
are unavailable and direct stale attempts reject before worker submission. No
audio-inactive, dormant, paged, or seventeenth Patch model exists. This is an
explicit scoped mismatch with Figma's effectively unlimited Patch workflow,
not a redefinition of that workflow or a weakening of callback bounds.

Direct inspection of the live Figma file measured Patch Overview node `95:202`
at 1920×1080 with 48/72/896/64 px shell bands, Engine plus three ordered Post
FX positions, the persistent Utility rows, and `SHIFT+L/R:patch` guidance.
Interaction Map node `49:3` measured 3000×2600 and explicitly sequences
existing Patch → Shift+Right → virtual empty slot → modify parameter →
initialized Patch, with no explicit New Patch command. The production
projection follows that semantic sequence and reports the 16-Patch boundary;
the current blockout has not established broad visual parity. The dedicated
native empty-Patch witness passed on 2026-08-28. It painted empty, prospective
Detail/Choice/Utility, all pending phases, failure, capacity, and created
documents at requested Wide and Standard windows, then reused the exact empty
projection at Intermediate, Compact, and enlarged-text conditions. The real
scroll owners reached both endpoints and their first/last semantic targets;
all interactive targets retained a 48 px floor, document horizontal overflow
and required-content overlap measured zero, repeated paints were identical,
and Compact Overview truthfully scrolled by 177 px. These measurements prove
the scoped interaction blockout, not broad visual parity.

## Controller and focus contract

Physical bindings normalize to semantic actions before product logic:

| Input | Meaning |
| --- | --- |
| unmodified arrows/D-pad | spatial semantic focus movement |
| Edit + Left/Right | fine decrement/increment or adjacent valid choice |
| Edit + Up/Down | coarse increment/decrement; Up opens a choice for choice controls |
| Edit | toggle/confirm |
| Shift + Up | open related Detail/Browser; MIXER → PATCH |
| Shift + Down | return/close; PATCH → MIXER |
| Shift + Left/Right | previous/next created Patch or trailing empty endpoint |
| Shift + Start | open temporary Settings · MIDI Devices (non-repeating) |
| Select | multi-select only when reducer semantics exist; currently unavailable |
| Start press/release | hold-to-preview in Sample Browser; reserved elsewhere |

PATCH Main is the non-wrapping Overview order: Engine, then the three canonical
effect-slot occupancy controls. Sibling order is every created Patch followed
by exactly one trailing empty position. The visual reading order is the same
single vertical sequence at every viewport width; wider compositions expand the row
interiors and the persistent Utility region rather than turning focus movement
into a horizontal card scan. Instrument, envelope, and configured-effect
parameters remain on descriptor-driven Detail surfaces. PATCH Utility contains
exactly master volume, Patch volume, MIDI input, output track, and voice limit.
Main and Utility remain mutually reachable, and subordinate return restores the
stable semantic origin or the nearest enabled sibling after schema change.

Settings · MIDI Devices is a system surface, not a top-level context. Entry
suspends the exact performance focus, interaction mode, return path, and Patch
subordinate session; Shift+Down restores that identity after schema repair.
The projected list is registry-driven and carries display name, opaque identity
facts, Available/Connecting/Connected/Unavailable/Disconnected/Failed text,
and a structural marker in addition to color. The inspector is pinned only to
the reducer-acknowledged active revision. Its activity card accepts only a
matching latest snapshot and otherwise paints Waiting. The list and inspector
start near an 80/20 fraction, clamp to content minima, and stack only under a
container-width constraint; resize emits no semantic action and uses no
aspect-ratio or named-viewport switch. This as-built structure has deterministic
projection/renderer coverage; direct native comparison with Figma node `116:2`
and the physical-device visual handoff remain incomplete evidence.

On macOS, Shift arrives through AppKit's `FlagsChanged` event rather than a key
down/up pair. The native input adapter treats modifier transitions as
non-repeatable and never queries key-repeat state from a modifier event; a UI
input callback must not unwind through the Objective-C event boundary. The
local monitor retains a bounded 128-signature window so WebKit's delayed
unhandled-key redispatch cannot double-feed the translator after a full input
burst. The strict native witness delivered 68 scripted transitions exactly
once and produced 20 byte-exact semantic actions, including repeated Shift
flags, Shift held across D/A, and held-K focus-loss cleanup. Its paired
real-window journey painted Shift+Right to empty, non-creating Detail
inspection, first edit, visible preparation failure, fresh retry, acknowledged
creation, exact return identity, and Shift+Left through production keyboard
normalization, `AppState::apply`, projection, and native paint.

MIXER Main uses one stable `(MixerTrackId, MixerTrackParameter)` path. Left/Right
changes T00–T0F while preserving Level/Pan/Mute/Solo row; Up/Down changes row
while preserving track. Inspector correlation is pinned to the selected track
and control, and its send → return → global body may scroll without using
scroll position as selection authority. The webview renders every visible
Inspector control through the shared parameter row, including sends; it must
not suppress a control kind that remains visible and focusable in the semantic
projection. Inspector entry therefore paints and reveals the reducer-selected
first send. The native webview witness checks the complete visible control
order, send labels, and exactly one painted target for the semantic focus path.
Compatible meter data must match both the painted parameter generation and
graph revision; missing/stale data paints
an explicit zero/stale state and cannot mutate application state.

Exactly one focused control is shown. Focus, adjust, active/current,
disabled, loading, error, mute, solo, and selection states use text or shape in
addition to color. The footer shows the current semantic path and only valid
actions. It is the only visual action-legend owner; rows and panels do not
repeat its guidance.

## As-built visual vocabulary

The current token source is `src/shell/tokens.rs`; `webview-page/tokens.css` is
generated from it. The interface is dark-only and uses Azeret Mono.

### Colors

| Token | Value |
| --- | --- |
| canvas | `#0c1015` |
| surface | `#121821` |
| panel | `#17202a` |
| elevated | `#1d2733` |
| selected | `#2a3745` |
| border-default | `#2a3745` |
| border-strong | `#415166` |
| text-primary | `#f2f6f8` |
| text-secondary | `#b8c4d1` |
| text-muted | `#6f8095` |
| focus | `#65e5ff` |
| adjust | `#ffb454` |
| positive | `#58e887` |
| warning | `#ff6868` |
| instrument | `#b894ff` |
| patch | `#ff6fbe` |
| chorus | `#f6f178` |

### Type and geometry

| Style | Size / line | Weight | Tracking |
| --- | --- | --- | --- |
| Display/Screen | 32 / 40 | SemiBold | 0.4 |
| Heading/Section | 18 / 24 | SemiBold | 1.4 |
| Heading/Panel | 14 / 20 | Bold | 1.2 |
| Body/Default | 15 / 22 | Regular | 0 |
| Body/Compact | 13 / 18 | Regular | 0 |
| Label/Control | 12 / 16 | Medium | 0.8 |
| Code/Value | 14 / 20 | SemiBold | 0.2 |
| Instruction/Hint | 11 / 16 | Medium | 0.8 |

As-built geometry uses spacing 4/8/12/16/24/32 px, radii 0/4/8 px, a 48 px
minimum target, a 1 px resting keyline, a 3 px cyan focus keyline with 8 px
halo radius/1 px spread/0.28 opacity, and a 3 px amber adjustment keyline. The
Mixer fader specimen is a 14 px track, 8 px fill, 3 px bottom shoulder, 34×6 px
cap, and 2 px rounding.

Shell geometry is a bounded responsive contract rather than a pair of fixed
canvases. Rust owns the minimum, preferred, and maximum track/spacing tokens
and the ordered Wide, Standard, and Compact thresholds; generated CSS exposes
them to one Grid/Flex DOM. Wide and Standard use a flexible main track with a
bounded persistent Utility/Inspector track. Compact stacks those same regions
in document order. `minmax()`, `clamp()`, intrinsic sizing, wrapping, and
scroll-to-focus keep required content reachable, and every interactive target
retains the 48 px floor. The 1920×1080 and 1280×800 sizes are representative
witness fixtures only and do not select product state or authorize fixed
coordinates.

## Evidence retained in code

The focused Mixer visibility and scroll-observation repair passed
`make test-webview-detail-native`, including repeat-render and focus checks,
the generated width sweep across all three layout modes, and owned shutdown.
Workspace reachability now measures the modal's inner option list when one is
present. Scroll probes check the absolute region endpoints separately from
revealing the first and last controls, since non-focusable headings and
visualizations can extend beyond those controls. This run excluded manual
dragging, and native screen capture returned `could not create image from
display`; the structural pass supplies no new screenshot or visual-fidelity
evidence.

The repository's tests and live-scene report types are the surviving detailed
evidence. At the documentation reset and the 2026-08-21 responsive Patch
Overview slice:

- formatting, Clippy with warnings denied, JavaScript syntax validation, and
  `cargo test --all-targets` passed; the library aggregate reported 744 passed
  and two measurement-only tests ignored, and every integration target
  completed without a deterministic failure;
- exact selector, no-name-enumeration, graph/callback, reducer/projection,
  Sample, and headless webview witnesses existed in the production path;
- the production webview witness measured Wide, Standard, Intermediate,
  Compact, and scaled-text Patch Overview compositions with one serialized
  projection, stable focus, all three slot controls, persistent Utility,
  48 px target floors, bounded independent scrolling, no required-content
  overlap, no document-level horizontal overflow, and deterministic repeated
  rendering;
- the same witness kept all sixteen Mixer tracks and Inspector reachable under
  responsive composition and retained projection-to-paint identity and meter
  correlation; the Mixer remains a functional blockout pending its own visual
  composition slice;
- the 2026-08-20 Patch-editor live report recorded 15 focused Patches, 105/105
  editable parameters, all three engine transitions, nonzero isolated audio,
  zero callback allocations/destructions, and clean note/stream/graph teardown;
- the 2026-08-20 Mixer live report recorded all 64 track/row focus pairs,
  32/32 send edits, exact level/pan/mute/solo edits, routing/isolation/meter
  predicates, nonzero physical audio, zero callback allocations/destructions,
  and clean teardown; it explicitly recorded multi-select as not implemented;
- the detail/assets implementation has deterministic positive and controlled
  negative tests, and its interactive audio path was manually heard and
  verified; Instrument/FX Detail now also has the shared descriptor-driven DOM
  composition, 29-state exact serialization matrix, and current native
  responsive/render measurements described above; its production native
  physical keyboard/controller handoff also passed as described above.
- the 2026-08-26 Engine/Post FX option run passed the focused reducer,
  registry, lifecycle, serialization, semantic-action, Sample regression,
  formatting, warnings-denied lint, exact-validation self-test, no-name guard,
  and broad deterministic suite; the library aggregate reported 750 passed and
  two measurement-only tests ignored;
- the scoped real-window option target passed the full viewport/text-scale
  matrix, maximum-registry scroll and wrapping, repeated paint, resize-only
  sequence, normalized input journeys, every occupied/empty/duplicate slot,
  Detail/Mixer regressions, and clean shutdown described above; the separate
  physical option handoff is still incomplete evidence.
- the 2026-08-27 scoped real-window Detail/Mixer witness measured zero action
  hints outside the footer across Overview, Detail, Utility/Inspector, Choice,
  Sample Browser, and Mixer fixtures; it also reconciled the Envelope SVG's
  four projected ADSR values, descriptor-owned timed-phase maxima, approved
  100 ms/5 s display proportions, and six painted curve points; retained the
  48 px target floor and complete scroll reachability, captured readable Wide,
  Standard, Intermediate, Compact, and scaled-text screenshots, and closed
  cleanly.
- the 2026-08-27 shared-channel MIDI correction passed focused production
  reducer, AppLoop, fixture-source, projection, and functional tests: duplicate
  subscriptions install and edit successfully, one normalized channel message
  emits ordered commands for both matching Patches, and an unsubscribed channel
  leaves generation and command output unchanged. The production smoke run
  reported automatic MIDI delivery with zero callback allocations and
  destructions;
- the 2026-08-27 physical MIDI implementation has focused deterministic proof
  for canonical IDs/preferences, one reducer mutation path, ordered scans and
  exact restart matching, bounded worker ownership, callback normalization and
  a 1 MiB SysEx negative, zero callback allocations/deallocations, 64-event
  control draining, shared-channel fan-out, reserved recovery, overflow
  invalidation/fresh revision, worker-side shutdown retirement, exact
  observation revision/30 Hz coalescing, Settings projection, schema version
  19, and committed renderer structure. The 2026-08-27 macOS CoreMIDI host
  seam enumerated three real ports under the `midir-v1` identity schema; the
  seam also treats zero ports as truthful success or reports one typed
  initialization failure.
- on 2026-08-28, the operator completed the bounded physical MIDI handoff with
  an attached device through the production CoreMIDI adapter and audio graph:
  Shift+Start entry, device navigation, Connect/Disconnect/Retry, correlated
  Receiving activity and audible supported messages, switching with the old
  input rejected and no stuck note, hot unplug/Unavailable/exact-identity
  return, Shift+Down focus restoration, and clean exit all passed. The native
  Settings page was also confirmed readable. Combined with the controlled
  malformed, unsupported, overflow, fresh-revision, and retirement tests above,
  this closes the physical-path handoff without claiming those synthetic edge
  packets were produced by the attached controller. Cross-platform native
  builds, the continuous native Settings width sweep, and direct screenshot
  comparison with Figma node `116:2` remain incomplete acceptance.
- on 2026-08-28, the session-lifecycle implementation passed formatting,
  Clippy with warnings denied, the exact-selector self-test, the static
  no-name-enumeration guard, 41 focused session library tests, and
  `cargo test --all-targets`. The library aggregate reported 834 passed and two
  measurement-only tests ignored; every binary and integration target passed.
  Production-seam evidence covered the exact `INIT` default, a fixture source
  that panics if normal startup consults it, initial silence and physical-MIDI
  playability, New/Open prepared replacement, Save/Save As exact round trips,
  dirty guards, typed failure preservation, block-boundary graph activation,
  off-callback candidate retirement, and zero measured callback allocations or
  destructions. The adapter-level native lifecycle report completed all five
  menu shortcuts, New cancellation, failed and successful Open, Save As, and
  dirty Close Cancel/Save/Discard with clean worker/graph teardown. The broad
  webview target passed its headless policy and serialization checks but
  truthfully skipped its environment-gated real-window/DOM groups because
  `CREST_WEBVIEW_TESTS=1` was absent. This lifecycle evidence does not establish
  broad Figma parity, packaged cross-platform storage semantics, or a manual OS
  dialog visual handoff.
- on 2026-08-28, empty-Patch implicit creation passed formatting, Clippy with
  warnings denied, exact-selector and no-name guards, focused reducer,
  projector, worker, coordinator, renderer, persistence, and session-lifecycle
  tests, and `cargo test --all-targets`. The library aggregate reported 860
  passed and two measurement-only tests ignored; every binary and integration
  target passed. The headless webview witness covered 40 serialized states,
  including prospective Overview, Detail, Choice, Utility, every creation
  phase, typed failure, 16/16 capacity, and acknowledged creation. Production
  evidence covered all declared first-edit families converging on one append
  correlation, exact graph preparation and preflight, prior live-audio carry,
  a silent new engine, activation-only saved commit, failure/retry, Save and
  fresh Open, exact 16-Patch restore, and zero measured callback allocations
  or destructions. Direct live-Figma inspection measured Patch Overview
  `95:202` and Interaction Map `49:3` as recorded above. The scoped native
  empty-Patch target passed its Wide, Standard, Intermediate, Compact, and
  enlarged-text matrix with singular focus, active scroll-endpoint
  reachability, 48 px target floors, zero required overlap/horizontal
  overflow, deterministic repaint, resize-neutral identity, and owned
  teardown. The strict native key monitor delivered all 68 scripted AppKit
  transitions exactly once into 20 byte-exact actions, and the real window
  painted the complete normalized empty → inspection → failed first edit →
  retry → created → exact return → Shift+Left sequence. This scoped evidence
  does not establish broad Figma parity or physical gamepad integration.

Instrument and FX Detail have completed their scoped native manual handoff.
The native responsive DOM measurements and handoff do not claim full Figma
parity. Sample Detail and Browser have the functional completion recorded in
the Sample section; further pixel matching is not an acceptance gate. Remaining
visual work includes Mixer composition, native scaling, and shared polish. Engine/Post FX
options retain the physical-input and scoped visual gaps recorded above. Their
existing functional surfaces are not evidence that those visual slices are
complete.

These measurements are historical observations, not a roadmap and not a
waiver for missing Figma fidelity. A test that only constructs objects, prints
a success token, uses a parallel reducer/render path, or produces silent output
is not behavioral proof.

## Commands

```sh
make build
make check
make test
make lint
make fmt-check
make test-engine-post-fx-options
make test-midi-devices
make test-midi-host
make test-webview-options-native
make test-webview-empty-patch-native

# Open Sample INIT with test MIDI playing. T stops/restarts; Return opens Detail.
make run

# Interactive attached-device Settings/audio handoff. The script traps
# interruption for safe teardown and exits incomplete unless the operator
# confirms every printed physical-path check.
make midi-device-handoff

# Run the current cumulative bounded live witness.
make demo-live
make demo-live-detail-and-assets

# Retained focused live witnesses.
make demo-live-mixer
make demo-live-effects-and-buses
make demo-live-patch-editor
make demo-live-sixteen-track-mixer-routing
make demo-live-semantic-view-model
make demo-live-graphical-shell

# Manual component browser.
make demo-live-component-library
```

Sample works with plain `make run`. Return opens Detail from Engine, then
Return on Sample File opens the in-app file page. Shift+W reaches the same page.
W/S navigates, Return enters/assigns, Space held on a file auditions it, and
Shift+S or the Cancel row returns unchanged. Hold K while using W/S/A/D to
adjust editable controls; loop mode opens its canonical choice list. Cmd+S and
Cmd+O retain the native session Save/Open dialogs.

The 2026-09-05 picker correction replaces the rejected native asset dialog.
Its focused checks cover common WAV/SF2 directory filtering, nested folders,
canonical keyboard confirm/back, import failure and stale-result correlation,
Sample audio/preview, and restored waveforms. The 868-test library run, 19
Sample/Detail tests, serialized-schema test, and warnings-denied Clippy passed.
Native production execution confirmed in-app entry, nested and Home navigation,
Cancel with the same asset/focus, library WAV selection, and Home-based external
WAV import returning to a READY Detail with an Imported-relative asset. The
imported copy remained readable after removal of its original source. The
final native renderer witness and owned close passed.
Existing resize acceptance remains closed. Physical gamepad mapping and new
SoundFont bank loading are separate integrations; neither is claimed here.

To use an existing library and its initial asset, configure both overrides
before the process starts:

```sh
CREST_SAMPLE_LIBRARY_ROOT=/absolute/path/to/library \
CREST_SAMPLE_DEFAULT_ASSET=relative/path/to/sample.wav \
make run
```

Useful validation entry points are
`bash scripts/run_exact_test_validation.sh --self-test`,
`bash scripts/run_exact_test_validation.sh <target> <exact-test> <marker>`,
`scripts/check_no_name_enumerated_identity.sh`,
`cargo test --test webview_projection_shell -- --nocapture`, and
`make test-webview-detail-native` for the bounded real-window Detail/Mixer
witness. `make test-webview-empty-patch-native` scopes the real-window
empty/pending/capacity/newly-created Patch fixtures, the normalized
failure/retry journey, and the strict production AppKit input-capture witness.
It requires an interactive macOS session; an unavailable window, missing
first animation-frame paint, non-exact native transition log, or missing
focus-loss edge fails the target and remains incomplete evidence. The option handoff checklist is
`scripts/run_engine_post_fx_option_handoff.sh`; the physical MIDI checklist is
`scripts/run_midi_device_handoff.sh`. `make test-midi-host` is safe with no
attached input: zero ports is truthful success and backend initialization is a
typed report. Linux builds require ALSA development headers for midir's default
backend; optional JACK needs an explicit packaging feature. Windows defaults
to WinMM; optional WinRT likewise needs an explicit packaging feature. Native
window, physical-input, and physical-audio sections require an interactive
host; a typed environmental skip is incomplete evidence, not acceptance.

## Change checklist

Every product or architecture change must preserve or deliberately amend the
following in one coherent commit:

1. physical input normalizes to semantic intent and mutates only through
   `AppState::apply`;
2. UI and audio are projections of canonical state;
3. the callback remains bounded, preallocated, lock-free, nonblocking,
   allocation/destruction/I/O/log/panic-free;
4. discrete events, latest scalars, and prepared structural graphs retain
   separate transports;
5. capability ports remain generic and failures never silently fall back;
6. PATCH and MIXER remain the only top-level contexts;
7. focus and return paths remain stable semantic identities across reflow;
8. control/status state remains explicit in text or shape as well as color;
9. proof uses the production reducer, projector, worker, graph, and render
   seams and includes a measurable negative/no-op distinction;
10. actual rendered output is compared directly with the live Figma reference
    before claiming interface fidelity.

Do not treat OpenSpec change artifacts or any other planning documentation as a
competing product authority. This file should change only when the as-built
architecture, durable invariant, verified capability, known gap, or reference
contract actually changes.
