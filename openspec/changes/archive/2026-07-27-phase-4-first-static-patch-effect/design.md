## Context

The current product prepares one capability-polymorphic instrument per Patch, renders each instrument into an isolated `PatchAudioBlock`, and sends those dry stems directly to `MixEngine`. PATCH already derives Engine, common ADSR, and instrument structural controls from canonical state and immutable descriptors; scalar values cross through a fixed latest snapshot; complete engine/preset replacements cross a separate structural ownership boundary.

Phase 4 needs one real per-Patch effect without introducing the dynamic graph editor reserved for later roadmap work. The implementation must preserve `physical input → semantic AppEvent → AppState::apply → view/audio projections`, keep the callback bounded and destruction-free, expose no unavailable effect choices, and remain falsifiable through the production reducer and renderer. `DESIGN.md` and the evaluated CUE model define the resulting durable architecture.

The relevant owners are:

- Synth: canonical effect identities, descriptors, configs, provider/preparer ports, and off-thread rack construction.
- Control: Patch installation, focus, scalar mutation, serialization, and projection.
- RealTime: fixed effect scalars, prepared effect ownership, processing order, graph compatibility, and observations.
- Mixer: gain/pan/sends/global effects over already processed Patch stems.
- Shell/Testing: explicit composition, fixture setup, focused acceptance, deterministic demo, and the physical live demo.

## Goals / Non-Goals

### Goals

- Add one audible, Patch-local Chorus insert between instrument rendering and mixing.
- Make effect state capability-described and processor-neutral without adding voice or MIDI semantics to effect descriptors.
- Configure exactly one stable Chorus slot on the first production fixture Patch and zero slots on the others.
- Expose read-only Chorus identity plus editable Amount and Depth through PATCH and `AppState::apply`.
- Prepare every native processor, delay buffer, state object, layout, and capacity off callback and destroy it only after ownership returns off callback.
- Preserve the exact effect slot/config/layout through SoundFont preset and engine replacement.
- Prove source provenance, control coherence, signal order, audible stereo consequence, Patch isolation, independent instances/tails, typed failures, callback safety, and live-demo teardown/exit.

### Non-Goals

- Effect selection, bypass, wet/dry control beyond the upstream Amount parameter, reorder, removal, a second slot, parallel routing, feedback edges, or arbitrary graph editing.
- Modulation, automation, persistence migration, plugin hosting, a third top-level context, or the Figma-derived graphical interface.
- Additional effects, sample-rate conversion inside Chorus, seamless tail migration, or a silent pass-through fallback.
- Moving Braids instrument scalars from MIXER to PATCH.

## Decisions

### 1. Effects use a parallel capability family and shared parameter values

Add `EffectCapabilityId`, stable Patch-local `EffectSlotId`, `EffectCapabilityDescriptor`, `PostEffectConfig`, and `EffectCapabilityRegistry`. Add the control/worker boundaries `EffectCapabilityProvider`, `EffectPreparer`, and callback-facing `PreparedPostEffect`. `PreparedPostEffectRackBuilder` resolves every configured slot through exactly one registry descriptor and identity-matched preparer.

Effect descriptors reuse the canonical `ParameterId`, `ParameterSpec`, `ParameterAssignment`, `ParameterValue`, and `AssetReference` types. They do not reuse the whole instrument descriptor because instrument voice policy and supported MIDI kinds are nonsensical for post effects. Registry/config/provider/preparer errors are typed; no path manufactures a bypass processor or substitutes another capability.

Alternative considered: extend the existing instrument descriptor with optional effect fields. Rejected because it creates invalid states and makes the instrument preparation contract responsible for processors it does not own.

### 2. Patch state is ordered, but runtime topology is deliberately fixed

`Patch` gains an ordered `postEffects: Vec<PostEffectConfig>` with unique stable slot ids. The domain shape is ordered so a later roadmap increment can extend it, but this increment validates a capacity of zero or one. The fixture factory supplies one `effect.chorus` config on its first Patch only; general Patch installation may accept any schema-valid zero/one arrangement for focused tests.

The complete graph owns both aligned racks:

```text
PreparedGraph
├── PreparedEngineRack[PatchId]
├── PreparedPostEffectRack[PatchId, EffectSlotId]
├── initial ParameterSnapshot
└── mixer/global-effect state and bounded scratch

callback per Patch
  PreparedInstrument.render
        ↓
  PatchAudioBlock (isolated stereo stem)
        ↓
  PreparedPostEffectRack.process in place
        ↓
  MixEngine (gain/pan/sends/mute/solo)
        ↓
  shared reverb + delay → master stereo
```

The effect rack is Patch-aligned with the engine rack and fixed snapshot. It invokes at most one processor-neutral dynamic dispatch per configured slot per block, never inside the sample loop. A zero-slot Patch passes through at this explicit stage; a configured-slot identity/layout mismatch is a render fault and is observed, not silently bypassed.

Alternative considered: put Chorus inside `MixEngine`. Rejected because the mixer would own per-Patch processors, obscure instrument→effect→mix order, and conflate Patch inserts with its one global reverb and delay.

Alternative considered: implement a general node/edge graph now. Rejected because routing, capacity, cycle, editing, and persistence semantics are outside this increment and would weaken the bounded proof.

### 3. Effect editing extends the existing PATCH resolver and scalar transport

`PatchControlId` gains an effect target carrying `EffectSlotId` and `ParameterId`. The single canonical nonwrapping PATCH order becomes:

```text
Engine → Attack → Decay → Sustain → Release
       → visible instrument StructuralChoice rows
       → each configured effect's visible ScalarEdit rows
```

The page projects read-only effect identity and generic descriptor sections/rows. Chorus Amount and Depth are not named in the reducer, projector, window, focus resolver, or demo coverage. Edit+Left/Right applies descriptor fine decrement/increment; Edit+Down/Up applies coarse decrement/increment. An accepted edit changes exactly one assignment in canonical `PostEffectConfig`, advances generation, and publishes the complete latest scalar snapshot without an `AudioCommand`, preparation request, or structural graph.

`ParameterSnapshot` gains a separate fixed Patch-aligned effect section: zero or one active slot per Patch, stable slot id, scalar count, and at most eight descriptor-ordered finite scalars. Keeping it separate from the existing at-most-sixteen instrument scalar section prevents engine/effect layout aliasing. `StateTree`, `PatchPageProjection`, `TextProjection`, `ParameterSnapshot`, `AudioObservationSnapshot`, demo coverage, and live report schemas advance their versions together.

Alternative considered: add Chorus-specific fields to `Patch`, `AppState`, or `ParameterSnapshot`. Rejected because the first effect would become the canonical data model and every later effect would require another branch.

### 4. Chorus is a pinned, minimal, exact-rate native adapter

`ChorusCapability` exposes `effect.chorus`, product label `Chorus`, and exactly two normalized continuous `Scalar`/`ScalarEdit` parameters in order:

| Parameter | Range | Default | Fine | Coarse |
| --- | ---: | ---: | ---: | ---: |
| Amount | 0–1 | 0.5 | 0.01 | 0.1 |
| Depth | 0–1 | 0.5 | 0.01 | 0.1 |

`ChorusPreparer` wraps the MIT-licensed Mutable Instruments Rings Chorus pinned at `pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4` and `stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec`. `vendor/chorus` contains only the audited required Chorus/FxEngine/resource/stmlib sources, upstream notices, provenance, and a SHA-256 manifest. The build uses the existing `cc` mechanism and a small opaque extern-C wrapper with exceptions and RTTI disabled.

Each successful preparation creates a distinct initialized native Chorus, 2,048-sample 16-bit external delay buffer, LFO/tail state, and bounded scratch requirements. The first admission accepts exactly 48,000 Hz. Every other rate fails before publication; the adapter does not resample or choose another effect. Native construction and destruction occur only on worker/control ownership.

Alternative considered: write a Crest-specific approximation. Rejected because the roadmap calls for the selected upstream implementation and source/license/timing behavior must be auditable rather than visually similar.

### 5. Structural replacement preserves effect identity/config/layout

Every engine- or preset-selection request and candidate carries the Patch's exact ordered `postEffects`. Candidate validation rejects any effect slot, capability, layout, or untargeted value difference. The complete replacement prepares a new independent effect instance for the candidate graph, refreshes its compatible initial scalar snapshot from the latest accepted generation before publication, and swaps both racks atomically at the existing block boundary.

Scalar effect edits remain available while preparation or activation is in progress. They follow the same graph-revision compatibility rules as mixer/envelope scalars and cannot be reverted by activation. A complete swap may reset the native Chorus tail; seamless state migration remains outside the contract.

Alternative considered: keep the old effect object and replace only the instrument slot. Rejected because it complicates cross-graph ownership, retirement, and revision compatibility and would no longer publish one complete prepared graph.

### 6. Effect observations are causal and verification is layered

The renderer computes one fixed-size `PatchEffectObservation` at the actual stage, including configured Patch identity plus pre-effect RMS, post-effect RMS, difference RMS, and stereo side RMS. It is measurement-only and joins the existing latest audio observation without exposing buffers or controlling DSP.

The release-mode `static_patch_effect` target is the focused witness. It verifies exact pins/hashes/license, descriptor/config failures, first-Patch production config, fine/coarse reducer edits, fixed scalar publication, instrument→effect→mix order, target-only difference, side energy, unconfigured-Patch identity, two independently stateful test instances and tails, structural preservation, exact-rate rejection, no fallback, finite output, zero callback allocations/deallocations/destructions/strings, and p99 below 2.666 ms at 48 kHz/256 frames.

The exhaustive demo adds bidirectionally exact schema and effect coverage and retains controlled/focused independent-instance proof. The live demo edits Amount and Depth through semantic PATCH inputs, brackets them with the existing bounded semantic note probes, waits for a newer exact-generation physical callback observation at the real effect stage, completes the preset and two engine transitions with the Chorus config intact, performs note cleanup, closes the real window, releases the stream, drains structural ownership, and returns process success. A printed marker without teardown and parent exit is not acceptance.

Alternative considered: infer effect success from a changed label, parameter value, or final output RMS. Rejected because those observations cannot establish that Chorus ran on the correct stem at the declared location.

## Risks / Trade-offs

- [The pinned source subset may have hidden header/table dependencies] → Record every vendored file and SHA-256 hash, compile only the manifest, and make provenance/source completeness part of the focused target.
- [The upstream processor may assume a specific sample rate or buffer semantics] → Admit exactly 48 kHz, bind the declared 2,048-sample 16-bit delay buffer per instance, and reject unsupported preparation before graph publication.
- [A time-varying tail can make naïve audio comparisons flaky] → Reset fixtures to identical state, use pre/post measurements at the actual effect seam, bound render windows, and separate production first-Patch coverage from the two-instance independence fixture.
- [Adding effect fields can accidentally make high-rate projection expensive] → Keep callback storage fixed/copyable, preserve generation-only immutable sharing, and advance/verify serialized schemas explicitly.
- [Structural swaps can restore stale effect scalar values] → Preserve slot/layout in the request, refresh candidate initial parameters from the committed generation, and assert scalar edits during preparation and activation remain exact.
- [A failed prepared effect could be mistaken for valid dry pass-through] → Treat missing/mismatched preparation and runtime slot/layout mismatch as typed failure with zero fallback/bypass count.
- [The physical demo can hang while waiting for sparse audio] → Reuse semantic note probes, stage-specific ten-second no-progress and 120-second total limits, and require cleanup, stream release, window close, and parent success.
- [Exact 48 kHz narrows device compatibility] → Surface an explicit unsupported-device/preparation error now; rate adaptation is a later independently proven change.

## Migration Plan

1. Land the reconciled CUE model and master-design/roadmap updates before implementation so the new ownership and proof contracts remain authoritative.
2. Add effect domain types, registry/provider/preparer ports, Patch config storage, typed validation/rejection, and versioned serialization without composing a processor yet.
3. Extend PATCH focus/projection and the fixed effect scalar snapshot; update schema equality and reducer tests before audio processing.
4. Add `PreparedPostEffectRack`, the complete-graph builder relation, renderer ordering, effect observations, and off-callback retirement tests using deterministic prepared-effect fixtures.
5. Vendor and hash the minimal pinned Chorus subset, add the opaque adapter, and pass focused source/lifecycle/rate/DSP/timing tests.
6. Compose `effect.chorus` and the first-Patch slot in normal, smoke, deterministic-demo, and live-demo startup; retain zero effects on other production fixture Patches.
7. Run the named focused target, all affected acceptance targets, the complete test/lint/format gates, `make demo`, and finally a real release-mode `make demo-live` to normal process exit.

Rollback is one coherent change rollback before persistence is introduced: remove the production effect registration/config, restore the prior schema versions and direct engine→mixer graph, and remove the vendored bundle. There is no saved-session migration or partial runtime compatibility promise in this increment.
