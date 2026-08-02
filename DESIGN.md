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

Crest Synth is a standalone, controller-first MIDI instrument host for desktop and compact handheld screens. A musician can navigate patches, edit an instrument and its post effects, mix sixteen output tracks, and perform without a pointer while the host produces stable, low-latency stereo audio.

The first executable slice can be narrower than the architecture. The current implemented baseline alternates HiDef SoundFont and Braids instruments using repository fixtures and routes the first fixture Patch through one statically configured Chorus insert. New engines and effects enter through separate capability registries and must never be represented by silent fallbacks or effect bypasses.

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
- `RuntimeState` contains device/load status plus the reducer-owned engine-selection
  lifecycle, never device handles, decoders, prepared graphs, or audio buffers.
- Invalid events return typed rejections and leave all state and generation unchanged.

Every accepted event follows one sequence:

1. Normalize physical input to a semantic `Action`.
2. Convert it to a closed `AppEvent`.
3. Reduce through `AppState::apply`.
4. Commit the accepted state.
5. Project the `ViewModel`, `ParameterSnapshot`, and any structural audio command.
6. Publish those projections through their owning ports.

Views receive an immutable view model and event sink, never mutable application state.

Engine selection and descriptor-declared structural choices use one asynchronous
structural-edit lifecycle through this path. An accepted request records a
monotonic request identity, typed intent, and `Preparing` status while leaving the
active `InstrumentConfig` and graph revision unchanged. A capacity-one worker
validates the exact target config and prepares a complete replacement graph off the
callback from one compatible provisional target snapshot. A correlated successful
result then passes through `AppState::apply`, commits the target config and
`Activating` status, projects the matching fixed snapshot, rebinds the graph to
that exact committed projection, and publishes the prepared graph. `Ready` is
restored only after block-boundary activation, retirement acknowledgement, and
control-side collection all agree. Failure, busy, early, stale, or mismatched
outcomes are typed, visible, and never select a fallback.

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

`ParameterSnapshot` is fixed-size and copyable. Instrument scalars, the three ordered Patch-effect scalar slots, the indexed track sends, and the bus-return scalars occupy separate fixed sections so an effect layout cannot alias an engine layout. It contains no `Vec`, `String`, path, mutex, decoder, or reference whose final drop could happen in the callback.

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
- active patch, effect, mixer track, and bus-return slots;
- graph handoff and retirement capacity.

The standalone binary is the production composition root. It selects and
injects capability providers, their separately registered preparers, the
discrete/scalar/structural transports, the audio-observation transport, the
device adapter, and the window adapter. The application service validates an
exact one-to-one provider/preparer registration before it freezes the registry
or publishes a graph; it does not import or construct concrete infrastructure
adapters.

Physical audio startup is explicitly two-stage. Control ownership negotiates a
validated PCM configuration without starting the stream, then prepares the
complete graph, every engine/effect, and all scratch from that configuration's
actual sample rate and bounded render capacity. Only that negotiated device
owner may then start the prepared renderer. A native callback larger than the
prepared capacity is rendered completely as consecutive bounded blocks; it is
never truncated or left with a silent tail.

When the physical device's default configuration is already usable PCM at the
preferred 48 kHz rate, negotiation accepts that reported default without also
enumerating optional format ranges. If optional range enumeration fails but the
reported default remains valid, that exact default is retained; Crest never
invents a device configuration or substitutes another device.

Post-start device failures cross back through a preallocated first-failure
atomic status path. The device callback maps only to a fixed-size typed error;
the control tick consumes it, retains the visible application error, and asks
the disposable window to close. Formatting, logging, recovery policy, and UI
behavior never run in the device callback.

Render complexity is bounded by explicit limits for active patches, voices, post-FX slots, tracks, bus returns, events, and frames. The product-level maxima are three ordered post-FX slots per Patch and eight bus returns; a current executable slice may impose narrower limits.

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

Unknown Patch routing is part of that bounded observation surface: the
renderer increments a saturating routing-failure counter and retains the last
unknown `PatchId` while leaving every prepared instrument unchanged. A direct
rack error is not sufficient evidence if the production renderer discards it.

## Audio and domain model

### Patch and capability registry

```text
Patch
├── stable PatchId
├── label and MIDI mapping
├── InstrumentConfig { capability_id, values, asset references }
├── VoiceEnvelope { attack_ms, decay_ms, sustain, release_ms }
├── up to three ordered PostFx slots
└── PatchOutput { track: MixerTrackId, trim_gain_db }

MixerState
└── tracks: [MixerTrack; 16]
    ├── stable MixerTrackId (0..15, displayed T00..T0F)
    └── MixerTrackParameters { level_db, pan, mute, solo, sends }
```

Patch output routing and mixer-channel control are different concepts. A Patch
owns one validated output track and a Patch-local trim. The mixer owns exactly
sixteen persistent tracks independently of the Patch collection. Multiple
Patches may target the same track, empty tracks remain addressable, and changing
a track fader changes the combined contribution of every Patch routed there.
Patch mute, solo, pan, sends, or mixer-fader state do not exist.

The control-side patch namespace is sparse and effectively unlimited for sibling navigation. An empty sibling remains virtual until its first accepted modification, which initializes it atomically. The running graph still has explicit capacity limits. Materialization beyond them is rejected visibly instead of overcommitting the callback.

Instrument and effect implementations share canonical parameter types but register distinct descriptors because voice policy and MIDI semantics do not apply to post effects:

```text
InstrumentCapabilityDescriptor
├── stable id, label, semantic accent
├── ordered sections and ParameterSpec[]
├── asset requirements
├── voice policy: fixed per Patch | engine managed
├── supported event semantics
├── off-thread preparation
└── real-time renderer factory

EffectCapabilityDescriptor
├── stable id, label, semantic accent
├── ordered sections and ParameterSpec[]
├── asset requirements
├── off-thread preparation
└── real-time processor factory

ParameterSpec
├── stable id and label
├── kind: continuous | stepped | choice | toggle | asset
├── range, default, fine step, coarse step, unit, formatter
├── update: scalar snapshot | prepared structural change
├── PATCH interaction: read-only | scalar edit | structural choice
└── optional enabled/visible dependency
```

UI choices come from the installed registries. The Figma names are illustrative planned capabilities, not permission to expose placeholders. SoundFont is the first concrete instrument entry. Braids is the second concrete engine and is wrapped around the pinned Mutable Instruments C++ macro-oscillator implementation; it exists to prove that Patch state, projection, preparation, MIDI routing, rendering, and verification are capability-polymorphic before the graphical Patch page is built. Neither the reducer, projector, rack, renderer, nor demo coverage may switch on concrete capability identities to define fields. Later sample, physical-model, wavetable, FM, and effect implementations must satisfy the same preparation and callback contracts.

Chorus is the first concrete effect entry. Crest uses the MIT-licensed Mutable Instruments Rings Chorus pinned at `pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4` and `stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec`, vendoring only the audited minimal source, table, header, license, and provenance subset behind an opaque exception-free C ABI. The product label is `Chorus`; upstream project names are provenance, not Crest branding. Each prepared instance owns its own 2,048-sample 16-bit delay buffer and LFO/tail state. This first adapter admits exactly 48 kHz and fails unsupported rates during preparation without bypass or substitution. Its descriptor exposes exactly `Amount` then `Depth`, each normalized to `0..=1`, default `0.5`, fine step `0.01`, and coarse step `0.1`.

Voice policy is capability-polymorphic. Braids declares `FixedPerPatch(16)`: every admitted Braids Patch owns a distinct sixteen-oscillator bank, so `N` active Braids Patches own `16 × N` voices with no engine-global pool; three Braids Patches therefore own forty-eight voices. There is no Braids-specific Patch-count limit or shared Braids voice budget. SoundFont declares `EngineManaged`: every SoundFont Patch owns one synthesizer instance whose backend manages polyphony under a finite prepared real-time safety ceiling. The engine-agnostic active graph capacity bounds how many Patches of any type can be materialized concurrently for hard-real-time execution; it never changes a Braids Patch's sixteen voices or turns them into shared capacity.

Every admitted engine applies the Patch-owned ADSR independently inside each native note voice. Attack, Decay, and Sustain are latched at note-on; Release is latched at note-off. Applying one gain envelope after a mixed Patch stem is nonconforming because it cannot preserve independent overlapping note lifecycles. Braids assigns an idle voice first and otherwise steals the oldest voice within the targeted Patch only. SoundFont delegates allocation to its one Patch-local synthesizer. All-notes-off remains Patch-targeted and bounded by the prepared engine policy.

Braids is built from the official Mutable Instruments source pinned at `pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4` and `stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec`. Crest vendors only the audited DSP subset and license/provenance files, compiles it behind a small exception-free opaque C ABI, and owns exactly sixteen fully initialized `MacroOscillator` instances per prepared Braids Patch. The descriptor exposes the 47 named playable upstream models plus scalar Timbre and Color. Braids retains its 96 kHz, 24-sample internal contract; the first admitted host format is exactly 48 kHz and uses a bounded 2:1 adapter. Unsupported rates fail during preparation and never select another engine.

Scalar capability parameters use descriptor order within the immutable active graph revision. Control projection encodes at most sixteen instrument scalar values and, separately, at most eight values for each of the three ordered effect slots into fixed destructor-free real-time storage; choices use their descriptor index. PATCH has one reducer-owned ordered focus surface resolved from both registries: Engine, Attack, Decay, Sustain, Release, visible instrument parameters classified `StructuralChoice`, then each configured ordered effect's visible parameters classified `ScalarEdit`. Bare Up/Down moves through that nonwrapping order; bare Left/Right remains unavailable until sibling-Patch navigation is introduced. On the engine row, Edit+Left/Right requests the adjacent installed capability without wrapping and Edit+Up/Down is unavailable. On ADSR and effect-scalar rows, Edit+Left/Right applies the canonical fine decrement/increment and Edit+Down/Up applies the canonical coarse decrement/increment. On a descriptor-declared structural-choice row, Edit+Left/Right requests the adjacent declared choice without wrapping and Edit+Up/Down is unavailable. Engine targets are rebuilt from the selected descriptor's ordered defaults and required assets; structural-choice targets replace exactly one assignment in the active config. Both paths use the same correlated structural-edit lifecycle, keep no inactive-config cache, and never translate or substitute values. The HiDef SoundFont surface therefore adds Preset after Release while its asset stays visible and locked; Braids instrument Scalar values remain descriptor-projected and read-only on PATCH in this slice; each of the three ordered effect slots appends one occupancy choice row — cycling empty and every installed registry entry through the same adjacent-choice gesture as the engine row — followed by the configured effect's editable parameters, so the first fixture Patch shows Chorus with editable Amount and Depth in its first slot by default. MIXER contains the sixteen track-owned controls with sends addressed by bus, the eight bus returns with their occupancy, parameters, and return levels, and master gain as the only distinct global.

Only one structural request is in flight application-wide, regardless of whether
its intent selects an engine or a capability-owned structural choice. MIDI, context
selection, focus navigation, and valid scalar edits from MIXER or the focused PATCH
ADSR/effect rows may continue while preparation or activation runs. An ADSR or effect edit during
preparation updates the audible source revision and the candidate is refreshed from
the latest committed snapshot before publication. An edit after candidate commit
targets the newer graph revision and is exact when that graph activates; the source
remains audible with its last compatible snapshot until the swap. The old graph
stays audible until the complete replacement swaps at a block boundary. A full
structural publish queue retains exactly one staged graph on control ownership and
retries; it does not roll back committed control state, drop the graph, or choose a
substitute. A complete structural candidate preserves each Patch's ordered effect
slot identity, capability, configuration, and scalar layout byte-for-byte. A complete
structural change may reset active voices and effect tails;
seamless migration is not claimed.

### SoundFont and sample pipeline

SF2, WAV, and related formats are control-side assets:

1. resolve and validate the asset reference;
2. read and parse on a worker;
3. decode to normalized contiguous PCM and immutable zone metadata;
4. resample or precompute interpolation data off the callback;
5. build bounded voice state and warm the asset;
6. publish a prepared instrument through the structural handoff.

The fixed SoundFont is parsed exactly once outside the callback into two immutable
projections. `SoundFontPresetCatalog` is control-side metadata containing stable
numeric preset identities and authored names. `PreparedSoundFontBank` contains only
the numeric PCM, zones, loop data, and preset addresses required by rendering. The
raw parsed SF2 object is released before a prepared graph can reach audio ownership,
so preset, instrument, and sample names never enter the callback graph. Each
SoundFont Patch still owns exactly one synthesizer instance and all such instances
share the numeric prepared bank. Common ADSR must reach that synthesizer's
independent native note voices through a conforming backend seam; Crest does not
create one synthesizer per voice or accept a post-stem envelope. If the selected
backend cannot satisfy that proof, the adapter must be extended or replaced before
the controls are exposed.

`SoundFontPresetId` is the stable playback identity formed from the SF2 `wBank` and
`wPreset` coordinates; the name is presentation, not identity. The catalog reads
each playable preset's `achPresetName` exactly through the SF2 fixed-field
NUL-termination rule, preserves case and duplicate authored names, and never
replaces or augments a label with a General MIDI name table. SF2 preset records are
not assumed to be stored in display order. Selectable entries are ordered by
`wBank`, then `wPreset`, with source ordinal used only to make malformed coordinate
collisions deterministic; the first playable record for a duplicate coordinate has
SF2 lookup precedence and the collision remains a typed control-side diagnostic.
This yields bank 0 in General MIDI program-number order when present, followed by
extension banks and conventional bank 128 percussion, without claiming that General
MIDI names arbitrary SoundFont presets. The descriptor exposes those entries as one
`soundfont.preset` Structural Choice whose stable choice ID encodes only the numeric
address and whose label is the authored name, plus the locked `soundfont.file`
asset. The descriptor default is the first sorted playable entry. An empty catalog,
missing fixture address, malformed metadata, or unavailable selected address is a
typed load/configuration failure; no nearby preset, synthesized name, melodic bank,
or other engine is substituted.

The Sample Browser is a nested Patch modal, not a native dialog. It supports controller navigation, metadata/waveform preview, hold-to-preview, assign, and cancel. Holding Start while a sample row is focused previews that sample. Preview stops when Start is released or on focus change, navigation, assignment, or cancel. Preparation occurs off the callback.

The detailed Sample capability contract—including admitted formats, playback and loop semantics, polyphony, root-pitch behavior, and preparation limits—is intentionally deferred until Phase 7 planning. Earlier phases neither depend on nor infer those choices from Figma fixtures.

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
patch trim
  ↓
route and sum into one of 16 tracks
  ↓
track level / pan
  ├──→ pre-gate track meter
  ↓
track mute / solo gate
  ├──→ post-gate sends[0..7] ──→ bus returns[0..7] ──┐
  ↓                                                  │
16-track dry mix ←──────────────────────────────────┘
  ↓
master gain / safety limiter
  ↓
stereo device output
```

The fixed Phase 4 callback seam is `PreparedEngineRack → PatchAudioBlock → PreparedPostEffectRack → MixEngine`. The effect rack is Patch-aligned with the engine rack and fixed parameter snapshot; it mutates only the matching interleaved stereo stem in place. `MixEngine` applies Patch trim, accumulates each stem into the selected fixed track scratch buffer, and then applies that track's parameters. The production bound is the product maximum itself: three ordered effect slots per Patch and eight bus returns, with one Chorus configured on the first fixture Patch by default. Focused tests may configure two Chorus instances to prove their delay, LFO, and tail ownership is independent.

Meters observe each post-level/pan track before the mute/solo audibility gate so muted tracks remain diagnosable. Mute always wins. When any track is soloed, only soloed, non-muted tracks contribute dry signal or sends. Sends are post-fader and post-gate, so a muted or solo-excluded track cannot continue feeding a wet return. Effects run in declared order. Feedback exists only inside bounded effect implementations, never as an arbitrary graph cycle.

The interface and canonical `MixerState` have exactly sixteen mixer tracks. More than sixteen patches may share tracks deterministically, but no patch may disappear or be silently rerouted. A Patch route change is a validated fixed-size latest-snapshot value, not a graph rebuild: the prepared graph already owns all sixteen bounded destinations. Invalid track identities are rejected before publication.

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

The layout must also be verified at the compact viewport. Preserve the header/footer bands, visible Utility/Inspector, minimum targets, and hierarchy using proportional widths and controlled density rather than hiding required context.

### Patch

The strip contains patch identity/routing, one instrument selector, ordered post-effect selectors, and a persistent Utility panel for master/patch volume, MIDI input, output track, and voice limit. Patch volume is Patch-local trim before track accumulation; it is not a mixer fader. Output track is a validated `MixerTrackId`, and changing it moves only that Patch's post-effect, post-trim contribution on the next compatible parameter snapshot. In the current bounded slice PATCH resolves Engine, Attack, Decay, Sustain, Release, descriptor-declared instrument `StructuralChoice` rows, then configured effect `ScalarEdit` rows. Bare Up/Down moves focus without wrapping. The engine row accepts Edit+Left/Right to request the adjacent installed engine. The four ADSR rows edit the focused Patch's existing canonical `VoiceEnvelope` with its descriptor-owned bounds and fine/coarse steps; they do not own UI values or a second DSP path. A SoundFont Patch adds the authored-name Preset row after Release and Edit+Left/Right requests its adjacent catalog entry; its file row remains locked. The targeted structural row displays its active and requested value plus `Preparing`, `Activating`, or a typed failure while the active graph remains explicit. Braids instrument capability rows remain read-only on PATCH. The production fixture's first Patch then shows read-only `Chorus` identity and editable `Amount` and `Depth`; Edit+Left/Right uses their descriptor fine step and Edit+Down/Up uses their coarse step. Other fixture Patches show no effect rows. This slice adds no engine/preset/effect modal, effect selector, bypass, reorder, second slot, or placeholder.

Instrument and effect detail views reuse the shell. The active capability supplies the title, accent, sections, values, ranges, units, and dependency rules. Non-Sample synths that expose the shared Patch `VoiceEnvelope` render a live, non-focusable ADSR preview in the strip visualization slot and instrument detail. Sample detail keeps its asset selector and non-focusable waveform/loop visualization alongside the same ADSR control contract.

Option selection is a nested modal showing installed choices. Focus is trapped until choose/cancel and then returns to the originating control.

### Mixer

All sixteen faders remain visible at 1920×1080. The persistent mixer tracks are T00 through T0F, never Patch-derived columns. They are compact columns with hairline separators, not cards; empty tracks remain visible and configurable.

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
| Start | — | Sample Browser: preview focused sample while held | reserved elsewhere |

The focused control kind resolves overloaded chords: Edit+Up is coarse increment on a number and opens the modal on a choice.

From the main Patch surface, unmodified Right crosses into Utility and Left returns. Each surface remembers its prior row. Details and modals store a stable return path so closing returns to the exact origin after reprojection.

The Sample Browser alone assigns Start as a hold-to-preview action for the focused sample row; Start remains reserved everywhere else. Preview does not commit an asset; only confirm assigns it.

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
| selected | `#2a3745` | selected row background |
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
| desktop UI | [eframe/egui](https://github.com/emilk/egui) | Selected application and rendering stack for the controller-first product UI. |
| UI utilities | [egui_extras](https://github.com/emilk/egui/tree/main/crates/egui_extras) | Selected companion utilities for egui layout and image/SVG loading. Crest still owns the stable component, behavior, token, and visual contracts. |
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
- **DSP:** finite measured peak/RMS, stereo routing, mute/solo, synthesis → Patch effect → mix order, target-only Chorus difference and stereo side energy, independent effect instances/tails, `16 × N` Patch-local Braids scaling, deterministic Patch-local stealing, engine-managed SoundFont polyphony, and independent overlapping-note envelopes in both engines.
- **RT:** allocator instrumentation, callback timing, overflow recovery, graph swap, off-thread destruction.
- **Assets:** real SF2/sample fixtures, exact authored preset names, bank/program ordering, preset identity, malformed input, loop bounds, and atomic replacement.
- **UI:** golden images at 1920×1080 and 1280×800, semantic tokens, single focus, complete controller navigation, modal return.
- **Integration:** standalone, fixture, and synthetic inputs use the production reducer and render path; the production fixture alternates SoundFont and Braids Patches and configures Chorus only on its first Patch. The deterministic demo modifies both focused-Patch output fields, all six parameters on every one of the sixteen tracks, every focused-Patch ADSR value, both Chorus scalars, and every global value through canonical navigation and adjustment; proves shared-track accumulation, trim/reroute isolation, exact pre/post-effect and gate/send/meter order, target isolation, independent instances, stereo output, and no fallback; selects adjacent SoundFont presets by exact catalog identity; then selects SoundFont → Braids → descriptor-default SoundFont through the same worker, reducer, complete graph handoff, and renderer while preserving Patch outputs, MixerState, and effect config/layout. It also proves catalog order/name fidelity, pending/busy/failure/stale handling, scalar/structural coexistence, block-boundary activation, off-callback retirement, target-only mutation, finite distinct output, and two-run logical determinism. The paced physical-device demo sounds two Patches through one track, exercises all sixteen track identities and both Patch-output controls with exact-generation observations, visibly and audibly edits both Chorus scalars before the structural sequence, waits for every acknowledged graph revision, restores descriptor-default SoundFont with Chorus still configured, performs semantic note cleanup, closes the window, releases the stream and worker, collects graph ownership, and exits normally; exhaustive negative-path, sample-exact routing, independent-instance, and two-run proof remains headless.

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

`make demo-live` is a bounded autonomous verification command, not an
open-ended interactive session. While its scene is active, the window renders
only the canonical `AppLoop` projection and native close remains available,
but mapped semantic key input is not dispatched into `AppState`; this prevents
an asynchronous user edit from replacing the exact generation awaited by a
checkpoint. After semantic all-notes-off cleanup and the completed report, the
standalone owner emits the four final records synchronously, asks the window to
close on that same control tick, releases the physical stream on control
ownership, and returns success. Closing the native window before that report
remains a typed incomplete-demo failure. Open-ended keyboard control belongs to
the normal `make run` application mode.

Scalar checkpoint audibility does not depend on which Corridors fixture part
happens to be sounding at that wall-clock instant. Immediately before each
accepted parameter edit, the live plan dispatches one bounded semantic NoteOn
probe through `AppLoop` to the owning Patch (or the focused first Patch for a
global edit); immediately after the exact-generation audio checkpoint and its
visible dwell, it dispatches the matching NoteOff. Fixture advancement remains
frozen only across that exact-generation interval and resumes afterward. Probe
events receive ordinary reducer, command, routing, and callback treatment but
never earn editable-parameter coverage, bypass `AppState::apply`, or introduce
a sequencer/transport model. Final Patch-targeted all-notes-off remains the
authoritative cleanup backstop.

The command announces that autonomous/input-isolated lifecycle before device
startup. Once the window tick begins, ten seconds without a runner milestone
or 120 seconds of total scene time is a typed live-demo timeout. A timeout asks
the same disposable window to close, performs semantic note cleanup while the
control loop is available, releases the physical stream, shuts down structural
work off callback, and exits nonzero with the stalled stage; it never remains
pending indefinitely or fabricates a completed report.

After completing its frozen editable-scalar coverage, the autonomous live scene
navigates the focused first SoundFont Patch to Preset, selects one adjacent named
catalog entry, and then selects SoundFont to Braids and back to descriptor-default
SoundFont. Each request is a semantic `AppEvent`; the runner waits without blocking
for canonical `Preparing`, `Activating`, and `Ready` observations while the
standalone control tick advances the production worker and structural coordinator.
After each acknowledged revision it dispatches targeted MIDI and requires a newer
finite, nonzero physical render observation before proceeding. The preset and
engine transitions share one coverage/status vocabulary and one in-flight limit.
The live scene does not inject worker failures or stale results; those exhaustive
controlled negatives remain in the deterministic headless scene.

An architecture change must preserve the one-way state path and callback contract, use canonical types, update this document when a durable decision changes, add falsifiable proof, and remove the superseded path in the same change.

## Durable decisions

- The Figma redesign replaces the diagnostic text view as the product interface.
- The product UI uses eframe/egui with egui_extras as its selected Rust GUI stack. Crest owns the application shell and component contracts; third-party utilities do not own product state, navigation, or audio behavior.
- PATCH and MIXER are the only top-level contexts.
- The UI is schema-driven; Figma example names are not a feature list.
- SoundFont is the first concrete engine, not a reason to couple the domain to one library.
- Braids is the second concrete engine; its C++ DSP remains behind the generic capability and prepared-renderer boundaries.
- Instruments and effects have separate descriptor/registry/provider/preparer families while sharing canonical parameter ids, specs, assignments, values, and asset references; effect descriptors never inherit instrument voice or MIDI semantics.
- Chorus, Reverb, and Delay are the three installed registry effects, peers in one role-independent registry whose entries may occupy a Patch effect slot or a bus return; the formerly mixer-owned reverb and delay join that registry as ordinary entries occupying the first two bus returns by default. Chorus uses the pinned MIT-licensed Mutable Instruments Rings Chorus subset, is admitted only at exactly 48 kHz in this slice, and owns an independent 2,048-sample 16-bit delay buffer plus LFO/tail state per prepared instance.
- The product topology is capped at three ordered post-effect slots per Patch and eight bus returns. A bounded implementation slice may support fewer but may not exceed these limits without updating this design.
- The production topology provides three ordered post-effect slots per Patch, each independently empty or occupied from the registry, with occupancy selected through the correlated prepared structural lifecycle: `PreparedEngineRack → PatchAudioBlock → PreparedPostEffectRack → MixEngine`. Slot order is render order; slots may be emptied, filled, or exchanged through that lifecycle, and the first fixture Patch holds one Chorus by default. There is no bypass control and no dynamic render-time graph editing — every topology change is prepared off the audio path and exchanged complete.
- The mixer is one fixed bank of exactly sixteen persistent tracks. A Patch owns only its output `MixerTrackId` and pre-track trim; a track owns level, pan, mute, solo, sends, and its meter. Multiple Patches may share a track, and no Patch-owned mixer-fader state exists.
- The detailed Sample capability contract is intentionally deferred until Phase 7 planning and is not inferred from Figma fixtures or treated as a prerequisite for earlier phases.
- In the Sample Browser, holding Start while a sample row is focused previews that sample; releasing Start or leaving the row stops preview, and Start remains reserved elsewhere.
- Every Braids Patch owns exactly sixteen voices; Braids capacity scales as `16 × active Braids Patch count` and is never pooled globally.
- Every SoundFont Patch owns one synthesizer with engine-managed polyphony; SoundFont is not artificially capped at sixteen and is never split into one synthesizer per note.
- SoundFont and Braids share the same Patch-owned per-note ADSR contract despite their distinct voice policies.
- The production fixture and both demos intentionally mix the two engines; engine-specific editable fields come only from capability descriptors.
- PATCH structural editability is descriptor-owned. The engine row and parameters classified `StructuralChoice` share one reducer/application-coordinator lifecycle, one in-flight limit, and one prepared-graph handoff; locked assets and Braids Scalar rows remain read-only on PATCH.
- PATCH focus is the reducer-owned nonwrapping order Engine → Attack → Decay → Sustain → Release → active instrument `StructuralChoice` controls → configured effect `ScalarEdit` controls. The four ADSR rows reuse `VoiceEnvelopeParameter`, and instrument/effect controls reuse `ParameterId` plus stable effect-slot identity; none creates UI-owned state or duplicate field enums.
- SoundFont exposes one `soundfont.preset` structural Choice ordered by numeric SF2 bank/program coordinates and labeled with exact authored SF2 names, plus the locked fixed asset. General MIDI supplies coordinate conventions, not replacement names or raw-file ordering.
- SoundFont preset names and raw SF2 metadata remain control-side; prepared callback ownership contains only numeric PCM, zones, addresses, and bounded voice state.
- Structural engine selection constructs the target from descriptor defaults and required default assets; structural parameter selection changes exactly one active assignment. Both keep no inactive-config cache, leave the old graph audible until block-boundary activation, and never fall back after failure.
- A complete structural graph swap may reset voices and effect tails; seamless migration is outside the current contract.
- Both autonomous demos edit first-Patch Chorus Amount and Depth, select an adjacent authored-name SoundFont preset, and select the focused first Patch SoundFont → Braids → descriptor-default SoundFont through production seams while preserving the effect config/layout; the headless demo owns exhaustive deterministic, independent-instance, and controlled-negative proof, while `demo-live` owns paced visible and physical-audio confirmation through the threaded worker.
- Normal live input, fixtures, and UI share the canonical reducer/projector
  path.
- The autonomous `demo-live` witness isolates mapped semantic input while its
  generation-correlated scene runs, then closes and returns after final report
  emission and control-owned stream teardown.
- Each `demo-live` scalar checkpoint is bracketed by a bounded semantic
  Patch-targeted NoteOn/NoteOff probe so sparse fixture timing cannot strand an
  exact-generation audible observation; probes never receive scalar coverage.
- High-rate MIDI projections share immutable generation-only state and
  materialize large JSON only when observed; the materialized form remains
  exactly equal to eager canonical serialization.
- Interactive rendering is event-driven with a 16 ms idle-frame cadence, and
  the physical `demo-live` target runs the optimized binary.
- Discrete events, scalar snapshots, and structural graphs cross the RT boundary differently.
- Structural audio state is prepared and destroyed off the callback.
- The authored mixer has sixteen tracks; patch and voice capacity remain explicit runtime bounds.
- The authored color set is the union of the design file's published variables and this document's table. The design file publishes a selected-row background this document omitted; this document declares elevated, strong border, patch, and chorus accents the design file does not publish as variables. Neither source is trimmed to match the other, so the vocabulary holds seventeen semantic colors.
- The Steam Deck density policy is authored from the desktop frames and the declared minimums, not measured from an authored small-viewport design, because no such design exists.
- Loading and error appearances reuse the structural-edit vocabulary this document already declares — the `Preparing`/`Activating` treatment and the typed-failure text — rather than inventing a second visual language.
- This file is the repository's only master design.
