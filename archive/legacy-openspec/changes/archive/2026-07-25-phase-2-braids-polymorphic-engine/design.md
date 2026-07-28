## Context

The prior Phase 2 increments made `Patch.instrument` generic, installed an immutable descriptor registry, and moved callback ownership behind `InstrumentPreparer`, `PreparedInstrument`, `PreparedEngineRack`, and complete `PreparedGraph` handoff. Production startup still constructs one HiDef SoundFont descriptor and one SoundFont preparer, capability values remain immutable, and the fixed real-time snapshot contains only Patch mixer and global values. The rack is therefore structurally polymorphic but has not yet carried two real engines or engine-owned live scalars.

This change must preserve the one-way `AppEvent` → `AppState::apply` → projection/publication path and the hard-real-time callback contract. It also responds to three binding product decisions: every Braids Patch owns its own sixteen voices, Braids voices are never pooled globally across Patches, and ADSR is a common configurable Patch concept applied independently to each note voice. SoundFont retains one synthesizer instance per SoundFont Patch with engine-managed polyphony rather than sixteen synthesizer instances per Patch. The physical and headless demos must install an alternating SoundFont/Braids scene and modify every value that is actually editable in this increment.

The upstream DSP is the official Mutable Instruments STM32F Braids implementation. The audited pins are `pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4` and its `stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec` submodule. The repository identifies STM32F code as MIT-licensed, and the required source files carry the MIT notice. Braids renders at 96 kHz in fixed blocks of at most 24 samples.

OpenSpec 1.6 enforces at most ten requirement deltas per change. The original integrated forty-delta plan is therefore represented by four non-overlapping completed slices: this change owns `braids-engine` and `instrument-capability-model`; `phase-2-polymorphic-control-demo` owns the fixture, reducer, and live proof; `phase-2-polymorphic-envelope-proof` owns canonical envelopes and the headless proof; and `phase-2-polymorphic-audio-runtime` owns prepared routing, callback contracts, and SoundFont conformance. This document retains the shared architectural rationale; `DESIGN.md` remains authoritative.

## Goals / Non-Goals

**Goals:**

- Install SoundFont and Braids as two real capability descriptors and preparers without an engine enum or capability branch in Patch, control projection, rack, renderer, or demo coverage.
- Give Braids an exact fixed-per-Patch sixteen-voice policy, make total admitted Braids capacity `16 × active Braids Patch count` with no global pool, retain engine-managed SoundFont polyphony, and apply one canonical Patch ADSR independently inside every note voice.
- Expose all 47 named upstream Braids models plus Timbre and Color through one descriptor; classify those three values as live Scalar updates.
- Carry envelope and engine-owned Scalar values through a fixed-size, copyable, graph-compatible latest snapshot and pass only the matching Patch projection to a prepared instrument.
- Alternate fixture Patch configs deterministically between the two installed providers while keeping the original discovered Patch/channel/event routing.
- Exercise every editable mixer, envelope, Braids, and global value in both demos, and prove mixed routing, parameter isolation, overlapping envelopes, finite audio, FFI lifecycle, source provenance, and callback performance.

**Non-Goals:**

- The PATCH page, engine selection, SoundFont preset browsing, user-triggered structural rebuilds, persistence migration, modulation, per-Patch effects, arbitrary graph editing, layering, or plugin hosting.
- Editing SoundFont bank, program, percussion, or asset fields. They remain descriptor-visible Structural configuration chosen during fixture preparation.
- Reimplementing, translating, or selectively approximating Braids algorithms in Rust.
- Supporting arbitrary host sample rates in the first Braids admission. A device configuration that cannot supply 48 kHz fails before graph publication.

## Decisions

### 1. ADSR is canonical Patch state, not an engine parameter or post-stem processor

`Patch` gains one `VoiceEnvelope` value with Attack milliseconds, Decay milliseconds, Sustain level, and Release milliseconds. `Patch::new` supplies the canonical default so existing callers do not invent local defaults. The value has a typed parameter descriptor surface, finite bounds, fine/coarse steps, and transactional `with_value` updates. It is serialized and projected beside the generic instrument config.

Each prepared engine owns one envelope state per note voice and latches Attack/Decay/Sustain at note-on plus Release at note-off. A changed snapshot affects subsequently started/released notes deterministically. Voice gains advance sample by sample; zero-time stages transition without division or unbounded loops. All-notes-off clears every slot with bounded work.

SoundFont cannot conform by multiplying its already mixed Patch stem. Each SoundFont Patch instead owns exactly one prepared synthesizer instance whose backend-managed note voices receive the common envelope through a per-note backend/adapter seam. The specification does not prescribe a RustySynth-internal mechanism: if the current backend cannot apply configurable ADSR independently to overlapping native voices, the adapter must be extended or the backend replaced before SoundFont exposes the common controls. Creating one full synthesizer per note is explicitly rejected.

Alternative considered: a single gain envelope after the SoundFont stem. Rejected because overlapping note-on/note-off lifecycles cannot be separated. Alternative considered: defer SoundFont ADSR while exposing it for Braids. Rejected because ADSR is common Patch semantics and an admitted engine may not display a control it does not audibly implement.

### 2. Voice policy is capability-polymorphic and Patch-owned

`CapabilityDescriptor` replaces a universal numeric voice-limit interpretation with a typed policy. `FixedPerPatch { voices }` declares an exact user-observable capacity owned anew by every prepared Patch; Braids uses `FixedPerPatch { voices: 16 }`. `EngineManaged` declares that the capability owns note allocation internally; SoundFont uses this policy and one synthesizer instance per SoundFont Patch. An engine-managed preparer still chooses and proves a finite internal real-time safety ceiling, but that operational bound is not presented as a shared product voice count.

Prepared instances are never shared between Patches. For any admitted active Braids Patch count `N`, the graph owns `N` independent oscillator banks and has `16 × N` Braids voices; three Braids Patches therefore own forty-eight voices. Braids declares no engine-specific Patch-count limit or shared voice budget. The graph's engine-agnostic active-Patch capacity bounds how many Patches of any type can be materialized concurrently for hard-real-time execution; it does not turn Braids voices into a global pool or reduce the sixteen voices owned by an admitted Patch.

Alternative considered: one global Braids bank routed among all Braids Patches. Rejected because Patch-local voice ownership, parameters, envelopes, MIDI lifecycle, stems, and scaling would be false. Alternative considered: force SoundFont into the same fixed policy. Rejected because its synthesizer already owns polyphony and duplicating a whole engine per voice is unnecessary and expensive.

### 3. The existing semantic Adjust event edits a schema-derived Patch surface

No engine-specific event variants are added. For a selected Patch, one canonical resolver produces the ordered editable targets:

1. the four existing Patch mixer values;
2. the four common envelope values;
3. every active capability parameter whose descriptor classifies it as Scalar, in descriptor order.

Structural parameters remain visible in descriptor projection but are skipped by navigation and rejected if a caller attempts to treat them as scalar. Numeric parameters use their declared bounds/steps; choice parameters adjust their stable choice index; toggles use a two-value domain. The reducer resolves the selected target against the immutable registry, creates a candidate config in canonical descriptor order, validates it, and commits only the accepted candidate. Projection and both demos use the same resolver rather than SoundFont/Braids field lists.

Alternative considered: new demo-only `SetBraidsParameter` events. Rejected because it would bypass the actual controller vocabulary and encode the second engine in the application event union. Alternative considered: make every descriptor value live. Rejected because SoundFont preset and asset changes require off-thread preparation and structural acknowledgement that remain a later increment.

### 4. Scalar instrument values use a fixed descriptor-ordered RT representation

The capability model declares a maximum of sixteen Scalar parameters per instrument descriptor. Registry construction rejects a descriptor above the bound. `RtPatchParameters` adds the Patch envelope and an `RtInstrumentParameters` value containing a count and `[f32; 16]`. The projector walks only Scalar specs in descriptor order and encodes continuous/stepped values numerically, toggles as 0/1, and choices as their descriptor index. No string, `Vec`, capability id, or destructor crosses in this value.

The active graph revision fixes the descriptor and scalar-slot interpretation. Compatibility still requires the same revision and ordered Patch ids. The rack gives a prepared instrument only its matching `RtPatchParameters` during targeted dispatch and its matching scalar/envelope projection during the once-per-Patch render call. The engine adapter owns interpretation of its scalar slots; the rack and renderer never match capability ids.

Alternative considered: put stable string ids in the callback snapshot. Rejected because that adds dynamic storage and string comparison to the hard-real-time boundary. Alternative considered: compile Braids fields directly into `ParameterSnapshot`. Rejected because it would make the shared snapshot a closed engine union.

### 5. Braids is an opaque, pinned C++ adapter with sixteen preinitialized oscillators per Patch

The repository vendors only the upstream files needed by `MacroOscillator`: macro, analog, and digital oscillator sources/headers; generated resources; required header-only helpers; `stmlib` fixed-point/random helpers; and their notices. A provenance manifest records repository pins, original paths, and hashes. `build.rs` uses the `cc` build dependency to compile this fixed list as C++ with exceptions and RTTI disabled where the compiler supports those flags. Hardware drivers, UI, bootloader, settings persistence, test I/O, and firmware entry points are not built.

A small Crest-owned C++ wrapper exports an opaque `extern "C"` bank. Creation and destruction occur only during control/worker preparation and retirement. Every prepared Braids Patch owns a distinct bank containing exactly sixteen `MacroOscillator` instances, initializes each before publication, validates every voice/model index, and exposes reset/configure/render operations declared `noexcept`. Rust owns each opaque pointer through one RAII adapter that is `Send`; no C++ type enters domain, rack, graph, or snapshot APIs, and no bank or voice is shared between Patches.

The descriptor id is `instrument.braids`. Its Model choice contains the 47 upstream named algorithms from CSAW through QPSK, excluding the firmware's question-mark sentinel. Timbre and Color are continuous 0..1 controls. Model, Timbre, and Color are all Scalar. The descriptor reports `FixedPerPatch { voices: 16 }` and the MIDI kinds the adapter actually handles.

Alternative considered: a Rust port or an existing third-party port. Rejected because the requested behavior is specifically to defer synthesis to the Mutable Instruments C++ implementation and because a second translation would weaken source provenance. Alternative considered: a shared monophonic oscillator. Rejected by the product decision and by the need for independent overlapping note envelopes.

### 6. Braids runs internally at 96 kHz and admits a 48 kHz host

Preparation accepts exactly 48,000 Hz for a Braids Patch. Every active voice renders upstream chunks of 24 samples, producing 12 host frames per chunk. Each adjacent pair is averaged as a bounded 2:1 low-pass decimation step. A short final chunk handles a host block not divisible by 12 without reading or writing outside prepared scratch. The host block may contain up to the graph's prepared maximum frames; all temporary storage is embedded in the oscillator/wrapper or caller-owned Patch stem.

This policy is intentionally narrow and explicit. It preserves the upstream pitch calibration and 24-sample scratch assumptions and matches Crest's primary validation configuration. Unsupported rates return `InvalidSampleRate` before graph publication; they never select SoundFont or a resampling fallback.

Alternative considered: call Braids at the host rate and retune pitch. Rejected because it changes upstream DSP timing and model behavior. Alternative considered: admit arbitrary rates with a general resampler now. Rejected because it increases callback state and the acceptance matrix before the first engine proof.

### 7. Voice assignment and MIDI expression are capability-owned, bounded, and deterministic

Within each Braids Patch, note-on uses an idle slot first and otherwise steals that Patch's oldest slot, resetting only that Patch-local voice before reuse. Note-off releases every active slot for the matching MIDI key in the targeted Patch; all-notes-off clears that Patch's sixteen voices immediately. Velocity scales only its note voice. Braids pitch starts from MIDI note in 1/128-semitone units; pitch bend applies a bounded ±2-semitone offset. SoundFont delegates allocation and stealing to its one Patch-local synthesizer and never borrows capacity from another SoundFont Patch. Supported controller/pressure expression is cached in fixed numeric fields and applied without changing canonical config. Unsupported message kinds return fixed-size typed status; descriptor-declared support and the exhaustive demo's per-Patch MIDI probes must agree.

Repeated-key MIDI cannot yet distinguish two same-key note instances because the existing canonical `MidiMessage` does not carry `NoteId`. Releasing all same-key slots is the explicit bounded policy for this increment; adding end-to-end MIDI 2.0/note-id semantics is not smuggled into the Braids adapter.

### 8. Production fixture assignment alternates providers at the testing edge

`AutomaticMidiTest` retains source discovery and stable Patch/channel mapping but accepts a capability-neutral config factory for each discovered part. Production composition supplies an alternating factory: zero-based even parts become their exact HiDef SoundFont config; odd parts become the default Braids config. This branch belongs only to the fixed demo-fixture adapter. The resulting `Patch` values, reducer, registry, projector, graph builder, rack, renderer, and observations see only `InstrumentConfig` and `CapabilityId` matching.

Standalone composition installs both descriptors and both preparers. Missing Braids preparation cannot silently make a Patch SoundFont-backed, and missing SoundFont cannot silently make it Braids-backed. Unit fixtures may still install a one-capability registry when that is the behavior under test.

### 9. Demo coverage and real-time admission are expanded together

The headless and live scenes derive their Patch-editable lists from the canonical target resolver. With the current fifteen-part fixture, the expected set includes mixer and ADSR values for every Patch, Model/Timbre/Color only for Braids Patches, and the seven globals. MIDI coverage is the union of installed descriptors and probes each Patch only with kinds its descriptor declares.

New acceptance renders isolated SoundFont and multiple Braids Patches together through the production graph, then changes each Braids scalar and each ADSR field through `AppState::apply`. It requires exact snapshot/state/text equality, parameter isolation, nonzero finite stems, a changed waveform/RMS for every engine control, independently releasing overlapping notes, sixteen simultaneous voices in every exercised Braids Patch, deterministic Patch-local seventeenth-note stealing, `16 × N` capacity for `N` admitted Braids Patches, and no fallback for bad model/rate/config. Callback instrumentation covers the production reducer/renderer path; native lifecycle counters plus source audit prove one C++ bank per Braids Patch and no C++ construction/destruction during dispatch/render. A bounded mixed-engine timing loop reports p99 against half the 48 kHz/256-frame callback period.

## Risks / Trade-offs

- [The current SoundFont backend may not expose configurable per-note ADSR for its engine-managed voices] → Keep exactly one synthesizer per SoundFont Patch, require the backend seam to prove independent overlapping envelopes, and extend or replace the backend rather than multiplying synthesizer instances or accepting a post-stem approximation.
- [Braids capacity grows linearly with the number of active Braids Patches] → Keep the separate active-Patch graph bound explicit, prepare every sixteen-voice bank off callback, reject graph over-capacity visibly, and measure the declared worst-case admitted graph without introducing a global voice pool.
- [Old Braids code can rely on compiler/embedded assumptions] → Compile only the audited subset behind a tiny C ABI, pin both repositories, use exact 24-sample chunks, disable exceptions/RTTI, run C++ smoke tests under sanitizers when available, and fail unsupported targets/rates explicitly.
- [A simple 2:1 average is not a mastering-grade resampler] → Treat it as the admitted, measured first policy for the exact 96→48 kHz ratio; keep resampling private so a later bounded FIR can replace it without changing Patch or capability contracts.
- [Choice-index encoding is positional inside one graph revision] → Preserve descriptor order in canonical registry/state, bind the projection to the graph revision, validate count/layout at preparation, and never persist or expose the RT index as semantic identity.
- [Changing ADSR while a note is already active is not retroactive] → Define note-on/note-off latching explicitly and make demo proofs trigger fresh overlapping notes after each configuration; future modulation may define continuous envelope modulation separately.
- [The fixture's repeated-key events lack NoteId] → Use the explicit release-all-same-key policy and keep the deficiency visible; do not invent adapter-local identities that the reducer and audio command cannot carry.
- [Exhaustive live coverage roughly doubles the number of checkpoints] → Retain the 500 ms observability dwell, keep one event per tick and compact terminal output, run release mode, and ensure fixture batching/projection remains below the existing responsiveness ceiling.

## Migration Plan

1. Update `DESIGN.md`, `ROADMAP.md`, and the evaluated CUE resources/capabilities/requirements/evidence to make the two-engine, per-Patch Braids voice policy, engine-managed SoundFont policy, and per-voice-envelope slice authoritative and remove SoundFont-only increment exclusions.
2. Add canonical envelope/editable-target values and fixed scalar projection; migrate reducer, text/state schemas, demos, and fixtures while the SoundFont renderer still satisfies the new port.
3. Retain one synthesizer per SoundFont Patch, add or select a backend per-note envelope seam, and prove common overlapping ADSR behavior without per-note synthesizer duplication.
4. Vendor/pin/build the Braids subset, add the opaque wrapper, provider, preparer, one sixteen-voice bank per Braids Patch, and focused native/Rust tests.
5. Install both providers/preparers in standalone composition, alternate fixture configs, and add mixed-engine demo and acceptance evidence.
6. Run strict OpenSpec/CUE validation plus format, check, lint, all tests, named acceptance targets, smoke, headless two-run demo, optimized live startup, and mixed-engine timing/allocation/destruction gates.

Rollback is removal of this entire increment before archival: restore the prior one-capability CUE/spec boundary, constructor/port signatures, SoundFont prepared adapter, and demo expectations together. Partial rollback that leaves Braids descriptors without a preparer, scalar state without callback consumption, or ADSR fields without per-voice behavior is not permitted.
