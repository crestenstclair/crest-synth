# Crest Synth — Master Design

## Authority and scope

This is the single product and technical design for Crest Synth.

The [controller-first Figma design](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=0-1) is the visual and interaction reference. Its screens define composition, navigation, focus behavior, and visual language. Example patch names, engines, effects, and values are fixtures, not an exhaustive implementation checklist.

When sources disagree:

1. The real-time safety and domain contracts here are normative.
2. Figma is normative for interface composition and behavior.
3. Executable specifications and tests define a current implementation slice and must be reconciled to this design.
4. Code proves current behavior; it does not silently redefine the product.

Do not add another architecture, handoff, phase plan, or competing product design. Durable decisions belong here. Temporary work belongs in issues and commits.

## Product intent

Crest Synth is a standalone, controller-first MIDI instrument host for Steam Deck and desktop. A musician can navigate patches, edit an instrument and its post effects, mix sixteen output tracks, and perform without a pointer while the host produces stable, low-latency stereo audio.

The first executable slice can be narrower than the architecture. The current baseline is a SoundFont-backed instrument using repository fixtures. New engines and effects enter through capability registries and must never be represented by silent fallbacks.

This product is not:

- a DAW, arranger, or general-purpose sequencer;
- an exhaustive catalog of synth engines and effects;
- a native-file-dialog workflow with controller controls bolted on;
- a UI-owned copy of audio state;
- allowed to trade callback safety for implementation convenience.

## Design principles

1. **Audio is hard real-time.** Callback work is bounded, preallocated, lock-free, and free of I/O and destruction.
2. **One path owns mutation.** Keyboard, controller, MIDI, fixtures, and future inputs become semantic events and pass through one reducer.
3. **State has one owner.** UI, persistence, and audio project from canonical application state.
4. **Screens are schema-driven.** Instrument and effect details render capability-provided parameter descriptions.
5. **Controller behavior is semantic.** Product logic sees actions such as `Edit`, `Shift`, and `Navigate(Left)`, never raw key codes or button numbers.
6. **Focus is singular.** Exactly one target is focused. Cyan means focus; amber means active adjustment.
7. **Color is semantic and sparse.** Neutral surfaces dominate; color is paired with text or shape.
8. **Unavailable means explicit.** Missing engines, effects, assets, and devices produce typed, visible errors.

## System architecture

```text
 keyboard / controller / live MIDI / fixture
                       |
              platform input adapters
                       |
              semantic Action / AppEvent
                       |
             +---------v---------+
             | AppState::apply   |  only mutation path
             +---------+---------+
                       |
          +------------+-------------+
          |                          |
   ViewModel projector       AudioState projector
          |                          |
   tokenized egui UI        prepared graph + snapshots
                                     |
                            lock-free RT boundary
                                     |
                             CPAL audio callback
                                     |
                          bounded stereo render graph
```

The architecture is hexagonal:

- **Domain** owns patches, parameters, routing, mixer values, and invariants. It depends on no UI, filesystem, device, or async runtime.
- **Application** owns `AppState`, its reducer, projections, commands, and orchestration ports.
- **Audio** owns fixed real-time value types and the bounded render graph.
- **Adapters** integrate devices, MIDI, controller input, SoundFont/sample decoding, files, and serialization.
- **UI** renders an immutable, host-neutral view model and emits semantic actions.
- **Testing** drives the same reducer, ports, and render path as the standalone host.

There is one canonical public type for each concept. Adapters may have private transport types, but normalize them at the edge instead of exporting substitute `Patch`, `MidiEvent`, or `Parameter` models.

### Canonical state

```text
AppState
├── SessionState       patches, instruments, post FX, mixer, routes, asset refs
├── InteractionState   context, modal, focus path, edit/multi-select mode
└── RuntimeState       device/load status, recoverable errors, generation
```

- `SessionState` is versioned and persistent.
- `InteractionState` is reducer-owned so navigation is deterministic and testable; it need not be saved in presets.
- `RuntimeState` contains status values, never device handles, decoders, or audio buffers.
- Invalid events return typed rejections and leave all state and generation unchanged.

Every accepted event follows one sequence:

1. Normalize physical input to a semantic `Action`.
2. Convert it to a closed `AppEvent`.
3. Reduce through `AppState::apply`.
4. Commit the accepted state.
5. Project the `ViewModel`, `ParameterSnapshot`, and any structural audio command.
6. Publish those projections through their owning ports.

Views receive an immutable view model and event sink, never mutable application state.

## Low-latency audio

### Execution domains

| Domain | Owns | May allocate or block? |
| --- | --- | --- |
| UI/control loop | canonical state, reducer, view projection, orchestration | yes, outside latency-sensitive input callbacks |
| Audio callback | active render graph, voices, block scratch, event consumption | no |
| Workers | file I/O, parse/decode, resampling, graph preparation, destruction | yes |

MIDI APIs may invoke their own callbacks. Each producer gets its own SPSC queue, or events are merged on the control side. A single-producer queue must never be shared casually by UI, MIDI, and worker producers.

The control loop is outside the hard-real-time callback, but it is still a
performance path. Accepted performance MIDI must not deep-clone immutable
capability/Patch state, deserialize JSON produced by the same process, rebuild
an unchanged text body, or eagerly serialize a complete observation tree for
every note. It validates through `AppState::apply`, advances the canonical
generation, publishes the fixed parameter snapshot and discrete command, and
derives generation-consistent snapshot/text/tree values by sharing immutable
projection storage. If an observer requests deferred JSON, it must be
byte-for-byte equal to an eager projection of that same `AppState`.

### Real-time boundary

Different data has different delivery semantics:

| Data | Transport | Policy |
| --- | --- | --- |
| notes, expression, bounded commands | fixed-capacity SPSC event ring | ordered; carries block-relative sample offset |
| scalar parameters | triple-buffered immutable `ParameterSnapshot` | latest complete snapshot wins |
| structural changes | ownership-transfer queue of prepared graphs/assets | swap only at block boundaries; retire off-thread |
| meters and RT health | atomics or latest-value `MeterSnapshot` | decimated; UI polls |

`ParameterSnapshot` is fixed-size and copyable. It contains no `Vec`, `String`, path, mutex, decoder, or reference whose final drop could happen in the callback.

A worker builds a complete `PreparedGraph`: engines, effect state, sample zones, delay memory, routing, and scratch capacities are all ready before publication. The callback swaps it at a block boundary and moves the old graph into a bounded return queue for worker destruction. If the return queue is full, the callback retains the graph in a bounded retirement slot and retries; it never drops it.

### Callback contract

The audio callback must not:

- allocate, deallocate, or grow a collection;
- lock, wait, sleep, or call a potentially blocking API;
- perform file, console, network, MIDI-device, or UI I/O;
- parse formats, resolve paths, resample assets, or prepare effects;
- log, format strings, panic, unwind, or destroy owned state;
- perform unbounded traversal, recursion, or dynamic dispatch in the inner sample loop.

Before audio starts, preallocate:

- maximum voices per active instrument;
- stereo buses and per-block scratch;
- effect delay/convolution working memory;
- maximum events consumed per block;
- active patch, effect, mixer, and aux slots;
- graph handoff and retirement capacity.

Render complexity is bounded by explicit limits for active patches, voices, post-FX slots, tracks, aux buses, events, and frames.

### Overflow and recovery

Queue pressure is observable, never silent:

- reserve capacity or a dedicated path for note-off and all-notes-off;
- coalesce replaceable controller changes before losing note lifecycle events;
- increment lock-free overflow and late-event counters;
- recover an unrecoverable note-lifecycle overflow with a bounded all-notes-off on the next block;
- throttle structural edits until the prior graph handoff is acknowledged.

### Timing targets

The host negotiates the best supported device configuration. The primary validation point is 48 kHz stereo at 128- or 256-frame buffers.

At supported buffer sizes:

- render-time p99 stays below 50% of the callback period;
- allocation instrumentation observes no heap allocation or destruction on the audio thread;
- a 30-minute performance-and-edit soak has no underruns on reference hardware;
- all output is finite, bounded, and denormal-safe;
- MIDI ordering is preserved and events use sample offsets inside the block.

The production control-path acceptance fixture installs fifteen Patches and
dispatches 512 MIDI events through the reducer, projector, journal, and audio
publication boundary in at most 50 ms in Cargo's unoptimized test profile. This
is a regression ceiling, not the desired operating point: the ordinary result
should be comfortably below it so one 256-event overdue fixture batch does not
stall a visible frame. The acceptance test must also force a deferred snapshot
and state tree to materialize and compare exactly with eager canonical output.

The callback updates only bounded counters for underruns, high-water render time, queue pressure, active voices, and clipped/non-finite samples. The UI polls those counters and decimated meters.

## Audio and domain model

### Patch and capability registry

```text
Patch
├── stable PatchId
├── label and MIDI mapping
├── InstrumentConfig { capability_id, values, asset references }
├── ordered PostFx slots
└── MixerRoute { track, gain, pan, mute, solo, sends }
```

The control-side patch namespace is sparse and effectively unlimited for sibling navigation. An empty sibling remains virtual until its first accepted modification, which initializes it atomically. The running graph still has explicit capacity limits. Materialization beyond them is rejected visibly instead of overcommitting the callback.

Instrument and effect implementations register descriptors:

```text
CapabilityDescriptor
├── stable id, label, semantic accent
├── ordered sections and ParameterSpec[]
├── asset requirements
├── voice limits and supported event semantics
├── off-thread preparation
└── real-time renderer factory

ParameterSpec
├── stable id and label
├── kind: continuous | stepped | choice | toggle | asset
├── range, default, fine step, coarse step, unit, formatter
├── update: scalar snapshot | prepared structural change
└── optional enabled/visible dependency
```

UI choices come from the installed registry. The Figma names are illustrative planned capabilities, not permission to expose placeholders. SoundFont is the first concrete registry entry. Braids is the second concrete engine and is wrapped around the pinned Mutable Instruments C++ macro-oscillator implementation; it exists to prove that Patch state, projection, preparation, MIDI routing, rendering, and verification are capability-polymorphic before the Patch page is built. Later sample, physical-model, wavetable, FM, and effect implementations must satisfy the same preparation and callback contracts.

### SoundFont and sample pipeline

SF2, WAV, and related formats are control-side assets:

1. resolve and validate the asset reference;
2. read and parse on a worker;
3. decode to normalized contiguous PCM and immutable zone metadata;
4. resample or precompute interpolation data off the callback;
5. build bounded voice state and warm the asset;
6. publish a prepared instrument through the structural handoff.

The real-time side receives stable numeric IDs, immutable PCM/zones, and bounded voice storage. It performs no path lookup, decode, zone allocation, or last-reference destruction.

SoundFont identity includes bank, program, and percussion status. Missing presets are load errors. A melodic preset cannot substitute for percussion with the same numeric program.

The Sample Browser is a nested Patch modal, not a native dialog. It supports controller navigation, metadata/waveform preview, hold-to-preview, assign, and cancel. Preview stops on release, focus change, navigation, assignment, or cancel. Preparation occurs off the callback.

### MIDI

```text
MidiEvent {
    source,
    channel,
    note_id,
    message,
    timestamp
}
```

`NoteId` keeps note-on, expression, and note-off attached to the same voice. Patch matching is exact; intentional layering produces one delivery per matching patch.

Hardware MIDI is performance input. Standard MIDI files are deterministic fixture/demo inputs, not a hidden sequencer domain. Parsing, tempo conversion, and instrument partitioning stay private to the adapter. The existing fixture may create one patch per bank/program/percussion identity and assign parts to sixteen output tracks deterministically.

Fixture polling uses caller-owned fixed-capacity batches. If control-side work delays a poll so that more events are due than one batch can hold, the adapter fills the available capacity and retains the remaining overdue events for later polls in source order; elapsed-time catch-up must not drop events, allocate in the audio callback, or fail merely because one control tick spans a dense interval.

### Signal flow

```text
MIDI
  ↓
patch instrument
  ↓
ordered patch post FX
  ↓
patch gain / pan
  ├────────────→ aux sends → shared aux FX ─┐
  ↓                                         │
track mute / solo gate                      │
  ↓                                         │
16-track mix bus ←──────────────────────────┘
  ↓
master gain / safety limiter
  ↓
stereo device output
```

Meters observe each track before the mute/solo audibility gate so muted tracks remain diagnosable. Effects run in declared order. Feedback exists only inside bounded effect implementations, never as an arbitrary graph cycle.

The interface has sixteen mixer tracks. More than sixteen patches may share tracks deterministically, but no patch may disappear or be silently rerouted.

### Persistence

Saved state is versioned and contains complete control state plus stable asset references, not decoded PCM or device state. Restore is decode → migrate → validate → prepare replacement audio state → atomically commit and hand off. A failure before commit leaves the active session and graph untouched.

## Controller-first interface

### Information architecture

There are two top-level contexts:

- **PATCH** — patch strip, instrument detail, effect detail, sample detail, option modals, and nested Sample Browser.
- **MIXER** — sixteen tracks, level/pan/mute/solo, routing summary, and selection inspector.

At the 1920×1080 authored size:

```text
48 px  context line: product · PATCH/MIXER · status
72 px  identity/header
896 px workspace
        ├── 1500 px main surface
        └──  420 px Utility/Inspector, always visible
64 px  current path and valid control hints
```

The shell is structural, not a collection of cards. Separators are hairlines. Waveforms are informative and omitted from the focus order.

The layout must also be verified at the Steam Deck viewport. Preserve the header/footer bands, visible Utility/Inspector, minimum targets, and hierarchy using proportional widths and controlled density rather than hiding required context.

### Patch

The strip contains patch identity/routing, one instrument selector, ordered post-effect selectors, and a persistent Utility panel for global/patch volume, MIDI input, output track, and voice limit.

Instrument and effect detail views reuse the shell. The active capability supplies the title, accent, sections, values, ranges, units, and dependency rules. Sample detail adds an asset selector and a non-focusable waveform/loop visualization.

Option selection is a nested modal showing installed choices. Focus is trapped until choose/cancel and then returns to the originating control.

### Mixer

All sixteen faders remain visible at 1920×1080. Channels are compact columns with hairline separators, not cards.

- Left/Right changes track while preserving the control row.
- Up/Down moves Volume → Pan → Mute → Solo.
- The Inspector identifies cursor, value/range, mute/solo, and route/sends.
- Shift+Up returns to Patch.
- Focus uses cyan; mute and solo pair warning/positive color with explicit `M ON` or `S ON` text.

### Semantic controls

Physical bindings are adapter configuration:

| Input | Numeric | Choice/toggle | Navigation |
| --- | --- | --- | --- |
| unmodified D-pad/arrows | move focus | move focus | spatial movement |
| Edit + Left/Right | fine decrement/increment | previous/next quick choice where valid | — |
| Edit + Up/Down | coarse increment/decrement | Up opens choices | — |
| Edit | — | toggle or choose | confirm |
| Shift + Up | open detail/browser | open related view | Mixer → Patch |
| Shift + Down | return | close/lower level | Patch → Mixer |
| Shift + Left/Right | — | — | previous/next sibling patch |
| Select | — | — | enter/leave multi-select |
| Start | — | — | reserved; no action |

The focused control kind resolves overloaded chords: Edit+Up is coarse increment on a number and opens the modal on a choice.

From the main Patch surface, unmodified Right crosses into Utility and Left returns. Each surface remembers its prior row. Details and modals store a stable return path so closing returns to the exact origin after reprojection.

The Sample Browser alone adds a hold-to-preview action. It does not commit an asset until confirm assigns it.

### Focus state

```text
FocusPath {
    context,
    surface,
    patch_id?,
    capability_id?,
    control_id?,
    modal_id?
}
```

Focus uses semantic IDs, never widget indices. If a schema change removes the target, the reducer selects the nearest valid sibling deterministically. Disabled targets are skipped. Exactly one target is focused.

Explicit modes are:

- `Navigate` — cyan 3 px keyline and restrained halo;
- `Adjust` — amber 3 px keyline while editing;
- `Modal` — focus trapped in a choice/browser surface;
- `MultiSelect` — visibly distinct and never color-only.

The footer echoes the current context/path and only actions valid at the focused target.

## Visual system

```text
semantic tokens
      ↓
pure behavior reducer → host-neutral ViewModel
      ↓
skin / primitives
      ↓
composed views
```

Views own composition and traversal. Primitives receive explicit focus/edit/disabled/mute/solo/selection state. Neither chooses domain behavior.

### Colors

The interface is dark-only. Raw colors stay private to the theme.

| Token | Value | Use |
| --- | --- | --- |
| canvas | `#0c1015` | application background |
| surface | `#121821` | primary surfaces |
| panel | `#17202a` | grouped regions |
| elevated | `#1d2733` | controls/modals |
| border/default | `#2a3745` | resting hairlines |
| border/strong | `#415166` | structural separation |
| text/primary | `#f2f6f8` | primary content |
| text/secondary | `#b8c4d1` | secondary content |
| text/muted | `#6f8095` | labels/inactive state |
| focus | `#65e5ff` | focus only |
| adjust | `#ffb454` | active adjustment and labeled sample identity |
| positive | `#58e887` | ready/solo/positive |
| warning | `#ff6868` | error/mute/destructive |
| instrument | `#b894ff` | instrument identity |
| patch | `#ff6fbe` | patch identity |
| chorus | `#f6f178` | chorus identity |

### Type and geometry

Use Azeret Mono throughout:

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

- Spacing: 4, 8, 12, 16, 24, 32 px.
- Radius: 0, 4, 8 px.
- Minimum interactive target: 48 px.
- Resting keyline: 1 px.
- Focus: 3 px cyan keyline; halo radius 8, spread 1, opacity 0.28.
- Adjustment: 3 px amber keyline.

Focus, mute, solo, loading, error, and selection always have text or shape in addition to color. Keyboard and controller actions are behaviorally identical after normalization.

## Library candidates

Versions belong in `Cargo.toml`, not here.

| Concern | Candidate | Decision |
| --- | --- | --- |
| device audio | [CPAL](https://github.com/RustAudio/cpal) | Preferred low-level cross-platform host. High-level playback APIs do not replace the render architecture. |
| live MIDI | [midir](https://github.com/Boddlnagg/midir) | Normalize callback bytes immediately and keep protocol types out of the domain. |
| desktop UI | [eframe/egui](https://github.com/emilk/egui) | Preferred for a custom-painted controller UI. Consider winit + wgpu only if measured fidelity/performance requires it. |
| controller input | [gilrs](https://gitlab.com/gilrs-project/gilrs) | Map buttons/axes to semantic actions at the edge. |
| RT event queues | [rtrb](https://github.com/mgeier/rtrb) | Fixed-capacity SPSC; one producer per queue. |
| latest snapshots | [triple-buffer](https://github.com/HadrienG2/triple-buffer) | Suitable for newest-complete parameters/meters after destruction behavior is audited. |
| SoundFont baseline | [RustySynth](https://github.com/sinshu/rustysynth) | Current-spec candidate behind `SoundFontBackend`; requires allocation, voice-bound, preset, percussion, and timing audits. |
| SoundFont alternative | [OxiSynth](https://github.com/PolyMeilex/OxiSynth) | Evaluate as a pure-Rust alternative. [FluidSynth](https://github.com/FluidSynth/fluidsynth) is the mature FFI fallback when compatibility outweighs deployment cost. |
| audio decode | [Symphonia](https://github.com/pdeljanov/Symphonia) | Decode/demux on workers into the prepared sample model. |
| MIDI files | [midly](https://github.com/kovaxis/midly) | Fixture/demo adapter only; parser types stay private. |
| persistence | [Serde](https://github.com/serde-rs/serde) + JSON | Versioned control-side schema with explicit migrations. |
| destruction | ownership return ring | Prepared graphs return through their dedicated bounded structural queue and are destroyed only on control/worker ownership. |

No third-party synth/DSP is assumed real-time safe merely because it renders into a slice. Admission requires proof of:

- zero callback allocation and deallocation during note dispatch and render;
- bounded voices/events and stable timing under overload;
- correct preset, percussion, sample-loop, and channel behavior;
- no locks, logging, I/O, panic path, or last-reference drop in callback use;
- acceptable license, platforms, and maintenance health.

If a candidate fails, replace the adapter without weakening the port or callback contract.

## Verification

A completed behavior must be distinguishable from a no-op:

- **Reducer:** exact accepted/rejected mutations, stable focus, implicit patch creation, input parity.
- **Projection:** one state generation produces matching view and audio snapshots.
- **DSP:** finite measured peak/RMS, stereo routing, mute/solo, effect order, bounded voice behavior.
- **RT:** allocator instrumentation, callback timing, overflow recovery, graph swap, off-thread destruction.
- **Assets:** real SF2/sample fixtures, preset identity, malformed input, loop bounds, atomic replacement.
- **UI:** golden images at 1920×1080 and Steam Deck size, semantic tokens, single focus, complete controller navigation, modal return.
- **Integration:** standalone, fixture, and synthetic inputs use the production reducer and render path.

Offline render is the deterministic audio proof. Device smoke tests separately validate negotiation and underruns. Construction-only tests, success-token logs, and silent output are not evidence.

`make demo-live` keeps the complete lossless `EventLog` in its typed report for
verification, but terminal output emits a compact
`CREST_LIVE_EVENT_LOG_SUMMARY` with counts and chain endpoints. Dumping every
performance MIDI record into an interactive terminal is not part of the live
experience; deterministic tests inspect the complete retained journal in
memory.

The interactive window schedules its next idle frame after 16 ms instead of
requesting an immediate perpetual repaint. Input and native window events may
still wake it sooner. `make demo-live` uses the optimized release profile so a
physical listening demo measures product behavior rather than debug-build
overhead; deterministic acceptance remains in the unoptimized test profile.

An architecture change must preserve the one-way state path and callback contract, use canonical types, update this document when a durable decision changes, add falsifiable proof, and remove the superseded path in the same change.

## Durable decisions

- The Figma redesign replaces the diagnostic text view as the product interface.
- PATCH and MIXER are the only top-level contexts.
- The UI is schema-driven; Figma example names are not a feature list.
- SoundFont is the first concrete engine, not a reason to couple the domain to one library.
- Braids is the second concrete engine; its C++ DSP remains behind the generic capability and prepared-renderer boundaries.
- Live input, fixtures, and UI share the canonical reducer/projector path.
- High-rate MIDI projections share immutable generation-only state and
  materialize large JSON only when observed; the materialized form remains
  exactly equal to eager canonical serialization.
- Interactive rendering is event-driven with a 16 ms idle-frame cadence, and
  the physical `demo-live` target runs the optimized binary.
- Discrete events, scalar snapshots, and structural graphs cross the RT boundary differently.
- Structural audio state is prepared and destroyed off the callback.
- The authored mixer has sixteen tracks; patch and voice capacity remain explicit runtime bounds.
- This file is the repository's only master design.
