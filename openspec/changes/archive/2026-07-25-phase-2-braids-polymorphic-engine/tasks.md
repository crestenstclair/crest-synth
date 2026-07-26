## 0. Validator-driven decomposition

- [x] 0.1 Partition the integrated forty-delta plan into four non-overlapping ten-delta changes without changing behavior, implementation ownership, or the authoritative `DESIGN.md`/CUE architecture.
- [x] 0.2 Retain `braids-engine` and `instrument-capability-model` in this core change and place the control/demo, envelope-proof, and audio-runtime capability deltas in their named companion changes.

## 1. Authoritative architecture

- [x] 1.1 Update `DESIGN.md` and `ROADMAP.md` so Phase 2 admits SoundFont plus Braids, gives every Braids Patch its own sixteen-voice bank, keeps SoundFont polyphony engine-managed with one synthesizer per Patch, pulls common per-voice ADSR and mixed demo proof into the current increment, and defers the PATCH page and structural editing.
- [x] 1.2 Reconcile the CUE project, capabilities, Synth/RealTime/Control/Testing resources, assets, requirements, evidence, and witnesses to capability-polymorphic voice policy plus scalar/envelope architecture; evaluate and validate the complete CUE package without invoking legacy crest-spec.

## 2. Canonical editable Patch state

- [x] 2.1 Add the canonical bounded `VoiceEnvelope` and typed ADSR descriptors/defaults to Patch state, serialization, event installation records, text projection, and StateTree/schema surfaces.
- [x] 2.2 Add one schema-derived Patch editable-target resolver covering mixer, common ADSR, and active descriptor Scalar values; make navigation and `AppState::apply(Adjust)` mutate candidates transactionally without engine-id branches or Structural edits.
- [x] 2.3 Enforce the sixteen-Scalar descriptor capacity and add canonical config replacement/conversion helpers for numeric, choice, and toggle values with typed no-fallback tests.

## 3. Fixed real-time scalar and envelope contract

- [x] 3.1 Extend `RtPatchParameters` with destructor-free ADSR and `[f32; 16]` descriptor-ordered instrument scalars, including choice-index encoding and graph/layout compatibility validation.
- [x] 3.2 Extend `PreparedInstrument`, rack dispatch/render, renderer ordering, StateProjector, StateTree equality, and all prepared test fixtures so each callback receives only its matching Patch parameter projection once per command/block.
- [x] 3.3 Implement the bounded sample-accurate per-voice ADSR contract with note-on/note-off latching, zero-time stages, capability-owned allocation/stealing, finite extremes, and allocation/destruction-free tests.

## 4. SoundFont engine-managed conformance

- [x] 4.1 Retain exactly one prepared synthesizer instance per SoundFont Patch, share the one parsed immutable bank across Patches, and expose an engine-native per-note envelope seam without creating a synthesizer per voice.
- [x] 4.2 Route common ADSR and MIDI lifecycle through the SoundFont backend's engine-managed voices and prove overlapping releases, all-notes-off, finite stems, and no callback allocation/destruction; extend or replace the backend if the existing implementation cannot conform.

## 5. Pinned Braids adapter

- [x] 5.1 Vendor the audited Braids/`stmlib` subset at the declared revisions with exact provenance hashes and MIT notices; add the fixed C++ build without firmware, driver, UI, bootloader, or runtime-download inputs.
- [x] 5.2 Add the exception-free opaque C wrapper and Rust RAII owner for exactly sixteen initialized `MacroOscillator` voices per prepared Braids Patch, including per-Patch lifecycle counters and typed create/index/model failures.
- [x] 5.3 Add `instrument.braids` provider/config construction with all 47 playable Model choices, Timbre, Color, Scalar classes, no assets, supported MIDI kinds, and exact `FixedPerPatch(16)` voice policy.
- [x] 5.4 Implement the prepared Braids renderer with deterministic idle/oldest voice assignment, velocity/expression/pitch behavior, independent ADSR, 24-sample 96 kHz chunks, bounded 2:1 decimation to 48 kHz, and explicit unsupported-rate failure.

## 6. Mixed production composition and demos

- [x] 6.1 Make automatic fixture configuration capability-neutral and install deterministic discovery-order alternation between exact SoundFont configs and default Braids configs while preserving Patch/channel/event identities and atomic no-fallback failure.
- [x] 6.2 Install both descriptors and preparers in standalone normal, smoke, headless-demo, and live-demo composition; update typed runtime observations to report and verify both engine kinds.
- [x] 6.3 Extend the headless scene, coverage derivation, checkpoints, schema equality, and MIDI probes to every mixer/ADSR/Braids/global value and each descriptor-supported MIDI kind, preserving exact restoration and two-run determinism.
- [x] 6.4 Extend the live scene/report to alternate visible/audible engine identities, adjust every derived editable instance with the existing dwell and reducer path, retain compact lossless output, and clean both engines to zero active notes.

## 7. Falsifiable mixed-engine acceptance

- [x] 7.1 Add a named Braids acceptance target proving source pins/hashes, descriptor/config shape, 47 models, one independent sixteen-voice bank per admitted Braids Patch, `16 × N` capacity for `N` active Braids Patches (with a concrete three-Patch/forty-eight-voice witness), no Braids-specific Patch-count limit below the general rack capacity, Patch-local seventeenth-note stealing, unsupported-rate/config failures, finite nonzero render, and native lifecycle ownership.
- [x] 7.2 Add a named per-voice-envelope target proving all ADSR controls audibly affect both production engines and two overlapping notes release independently; ensure a post-stem or ignored-control mutant cannot pass.
- [x] 7.3 Expand prepared-rack, real-time, capability-schema, schema-surface, GUI-context, mutation, control-performance, smoke, and demo witnesses for exact mixed routing, scalar isolation, no fallback, zero callback allocation/destruction, and complete projections.
- [x] 7.4 Add a release-profile 48 kHz/256-frame mixed-engine timing witness with engine-managed SoundFont load and sixteen active voices in every Braids Patch of the declared worst-case admitted graph; require measured p99 below half the callback period.

## 8. Completion gates

- [x] 8.1 Run focused unit/integration tests throughout implementation and resolve every regression without weakening a predicate, skipping a fixture, or adding an environment-dependent pass.
- [x] 8.2 Run strict OpenSpec validation, CUE evaluation/vetting, `cargo fmt --check`, `cargo check --all-targets`, Clippy with warnings denied, all tests, every named acceptance target, `make smoke`, and deterministic two-run `make demo`.
- [x] 8.3 Build and launch the optimized `make demo-live` path far enough to verify mixed-engine startup, supported device negotiation, responsive pacing, and non-churning render/control behavior; record any genuinely human listening-only remainder explicitly.
