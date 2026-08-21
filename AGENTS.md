# Crest Synth — As-Built Architecture and Invariants

This is the repository's only living product/architecture document. It records
the system that exists, the invariants that must survive changes, the available
evidence, and the known mismatch with the authored interface. It is not a
roadmap, a phase plan, or a claim that the product is finished.

Snapshot date: 2026-08-21. The implementation reviewed for this reset began at
commit `d2d257f`.

## Working rule

Read this file before changing product behavior or architecture.

- Do not introduce a roadmap, OpenSpec change, CUE DSL, planning kit, parallel
  master design, or generated specification system into this repository.
- Use source code and production-path tests to describe current behavior. Use
  this file for durable architecture, product invariants, and explicit gaps.
- Use issues and commit messages for temporary intent, sequencing, acceptance
  notes, and handoffs.
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
The native 1920×1080 comparison, native 1280×800 reflow comparison, complete
native input/window witness, and clean native visual acceptance were not
completed. The last native projection attempt passed serialized checks but
timed out before the first `animation-frame-ready` acknowledgement. There is no
authored compact Figma frame; the current 1280×800 policy is derived from the
desktop design and declared minimums. Mixer multi-select has a type/state name
but no defined product semantics and is truthfully reported as not implemented.

The old phase/spec system recorded 73 of 79 implementation tasks complete when
it was retired. The six incomplete items were the two native viewport
comparisons, native input/window/teardown proof, the aggregate deterministic
webview/gate run, the physical live target run, and final completion evidence.
Historical deterministic and physical-audio reports are evidence of functional
and real-time behavior only; they are not evidence of Figma fidelity.

## Visual and interaction reference

The normative visual/workflow reference is the live
[Crest Synth — Controller-First UI Redesign](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=0-1).
The primary authored nodes are:

- [Patch Strip — 36:3](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=36-3)
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
production composition is a Rust application using a Tauri v2/WKWebView shell,
CPAL stereo audio output, and the parsed `midi/Corridors of Time - Chrono
Trigger.mid` fixture for automatic playback. It is not a DAW, arranger, or
general-purpose sequencer.

The application currently provides:

- PATCH and MIXER as the only top-level contexts;
- sparse stable Patch identities with an active graph capacity of 16 Patches;
- HiDef SoundFont and Braids instrument capabilities in the default registry;
- an optional Sample capability installed only when
  `CREST_SAMPLE_LIBRARY_ROOT` and `CREST_SAMPLE_DEFAULT_ASSET` are both set and
  the real catalog/WAV adapters validate the configured asset;
- Chorus, Reverb, and Delay in one effect registry;
- three ordered Patch post-effect slots and eight bus returns;
- one fixed bank of sixteen persistent Mixer tracks, T00 through T0F;
- descriptor-driven Patch rows, Detail, generic Choice, Sample Browser,
  waveform/playhead projection, persistent Utility, and persistent Mixer
  Inspector surfaces;
- level, pan, mute, solo, indexed sends, return occupancy/parameters/levels,
  pre-gate meters, Patch route/trim, and master gain;
- versioned saved state containing stable asset references and normalized
  control values, never decoded PCM or device state;
- retained deterministic, headless, controlled-negative, and physical-demo
  commands listed later in this file.

Normal `make run` opens the real application and automatically plays the fixed
MIDI fixture. The `demo-live-*` commands are bounded autonomous witnesses, not
open-ended performance sessions; their mapped semantic input is isolated while
the generation-correlated scene runs.

## Architecture

```text
keyboard / controller / live MIDI / MIDI fixture
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
- Presentation density dispatches no action and cannot change track, row,
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

Each asynchronous producer owns its SPSC queue or is merged on the control
side. A single-producer queue is never shared casually among UI, MIDI, and
workers. Queue pressure is explicit; note-off/all-notes-off has reserved
recovery behavior, replaceable scalars may be coalesced, and silent loss is
forbidden.

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

Saved state is versioned and stores canonical control state and stable relative
asset references. It never stores decoded PCM, absolute library roots, browser
or preview sessions, prepared graphs, or device state. Restore is decode →
migrate → validate → prepare → atomic commit/handoff. Failure before commit
leaves the active session and graph untouched; a missing Sample restores as
explicit Unavailable.

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
| Shift + Left/Right | previous/next installed Patch |
| Select | multi-select only when reducer semantics exist; currently unavailable |
| Start press/release | hold-to-preview in Sample Browser; reserved elsewhere |

PATCH focus is descriptor-driven and non-wrapping: Engine, Attack, Decay,
Sustain, Release, visible instrument structural choices, then each effect-slot
occupancy and the configured effect's scalar rows. PATCH Utility contains
exactly master volume, Patch volume, MIDI input, output track, and voice limit.

MIXER Main uses one stable `(MixerTrackId, MixerTrackParameter)` path. Left/Right
changes T00–T0F while preserving Level/Pan/Mute/Solo row; Up/Down changes row
while preserving track. Inspector correlation is pinned to the selected track
and control, and its send → return → global body may scroll without using
scroll position as selection authority. Compatible meter data must match both
the painted parameter generation and graph revision; missing/stale data paints
an explicit zero/stale state and cannot mutate application state.

Exactly one focused control is shown. Focus, adjust, active/current,
disabled, loading, error, mute, solo, and selection states use text or shape in
addition to color. The footer shows the current semantic path and only valid
actions.

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

The authored 1920×1080 shell is 48 px context line, 72 px identity header, 896
px workspace split 1500/420 between main and persistent Utility/Inspector, and
a 64 px path/action footer. This is a reference geometry, not a statement that
the current output matches it.

## Evidence retained in code

The repository's tests and live-scene report types are the surviving detailed
evidence. At the documentation reset:

- formatting, Clippy with warnings denied, and all-target tests had passed;
- exact selector, no-name-enumeration, graph/callback, reducer/projection,
  Sample, and headless webview witnesses existed in the production path;
- the 2026-08-20 Patch-editor live report recorded 15 focused Patches, 105/105
  editable parameters, all three engine transitions, nonzero isolated audio,
  zero callback allocations/destructions, and clean note/stream/graph teardown;
- the 2026-08-20 Mixer live report recorded all 64 track/row focus pairs,
  32/32 send edits, exact level/pan/mute/solo edits, routing/isolation/meter
  predicates, nonzero physical audio, zero callback allocations/destructions,
  and clean teardown; it explicitly recorded multi-select as not implemented;
- the detail/assets implementation has deterministic positive and controlled
  negative tests, and its interactive audio path was manually heard and
  verified, but the required native visual comparisons and structured final
  acceptance were not completed.

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

# Open the production window and automatic MIDI/audio path.
make run

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

To install Sample in the production registry, configure both values before the
process starts:

```sh
CREST_SAMPLE_LIBRARY_ROOT=/absolute/path/to/library \
CREST_SAMPLE_DEFAULT_ASSET=relative/path/to/sample.wav \
make run
```

Useful validation entry points are `scripts/run_exact_test_validation.sh`,
`scripts/check_no_name_enumerated_identity.sh`, and
`cargo test --test webview_projection_shell -- --nocapture`. Native window and
physical-audio sections may require an interactive macOS host; a typed
environmental skip is incomplete evidence, not acceptance.

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

Do not reintroduce a competing documentation tree. This file should change only
when the as-built architecture, durable invariant, verified capability, known
gap, or reference contract actually changes.
