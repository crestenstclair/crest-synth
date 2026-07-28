## Why

Phase 2 cannot safely add a PATCH page or Braids while `Patch` and its projections assume that every instrument is a SoundFont preset. The first Phase 2 increment must make SoundFont one explicit capability behind a schema-driven configuration contract, preserving current audio behavior while creating the polymorphic seam later engine-rack and Braids work will use.

## What Changes

- Add stable instrument capability, parameter, value, asset-reference, descriptor, config, and registry domain types owned by the Synth context.
- Add an `InstrumentCapabilityProvider` port and a `HiDefSoundFontCapability` adapter that declare `instrument.soundfont.hidef` and validate generic assignments/assets without exposing a SoundFont-specific method on the polymorphic port.
- **BREAKING**: replace the SoundFont-specific instrument shape owned by `Patch` and serialized control state with a validated generic `InstrumentConfig`.
- Construct one immutable descriptor registry in `AppState`; validate every installed Patch against it and fail explicitly on unknown, duplicate, incomplete, or invalid configurations without fallback.
- Project installed descriptors and generic Patch configs into `StateTree` and the existing text view by walking descriptor order rather than matching engine-specific fields.
- Adapt the current SoundFont renderer to consume the generic config while retaining identical fixed-HiDef, one-engine, one-bank, bounded-stem, MIDI, mixer, UI, and hard-real-time behavior.
- Extend schema-derived demo coverage and add a named `capability_schema` acceptance test for exact descriptors/configs and controlled invalid cases.
- Keep high-rate MIDI responsive by validating it read-only in `AppState`, sharing unchanged immutable projection storage, deferring large JSON materialization, and proving eager/deferred projection equivalence plus a fifteen-Patch dispatch ceiling.
- Keep the complete live EventLog in the typed report while replacing the interactive terminal's multi-megabyte record dump with one compact lossless summary and chain endpoints.
- Pace idle eframe redraws at 16 ms and run `make demo-live` with the optimized release binary so the physical demo does not churn a CPU core in an immediate debug repaint loop.
- Keep the prepared multi-engine rack, Braids C++/FFI wrapper, simultaneous SoundFont/Braids proof, PATCH page, engine selection, effects, and modulation out of this increment.

## Capabilities

### New Capabilities

- `instrument-capability-model`: Defines canonical descriptors, typed parameter metadata, generic Patch-owned configs, immutable capability registration, provider conversion, generic projection, and explicit no-fallback failure.

### Modified Capabilities

- `soundfont-audio`: Reframes SoundFont as the only installed renderer in this increment and configures it from the generic capability contract instead of a SoundFont-shaped Patch.
- `one-way-parameter-control`: Extends canonical state and projection equality to the installed capability registry and generic Patch instrument configs.
- `automatic-test-midi`: Converts discovered SoundFont identities through the installed capability provider before Patch installation.
- `observable-demo-scene`: Derives coverage from installed capability/parameter descriptors and proves exact registry/config serialization, projection, invalid-config rejection, and bounded interactive evidence output.
- `live-observable-demo`: Retains complete live evidence for verification while emitting a compact final journal summary instead of every performance MIDI record.

## Impact

Affected canonical resources include `aggregate.Synth.Patch`, `aggregate.Control.AppState`, `domainService.Control.StateProjector`, `applicationService.Control.AppLoop`, `applicationService.Testing.AutomaticMidiTest`, `applicationService.Testing.ExhaustiveGuiDemo`, `applicationService.Shell.StandaloneApplication`, `port.Synth.SoundFontEngine`, `adapter.HiDefSoundFontEngine`, the new Synth capability value objects and provider port, the new `adapter.HiDefSoundFontCapability`, and the Rust library, composition root, and named integration-test assets.

Serialized `StateTree`/state snapshot instrument fields and their schema-derived coverage universe change. Generation-only production projections become lazy but materialize to the same canonical JSON. The live terminal marker changes from `CREST_LIVE_EVENT_LOG` to `CREST_LIVE_EVENT_LOG_SUMMARY`; the complete EventLog remains part of `LiveDemoReport`. The live eframe loop uses a scheduled 16 ms idle repaint and the Make target selects Cargo's release profile. No dependency, audio callback transport, effect topology, top-level UI context, MIDI fixture, or user-editable parameter behavior changes in this increment.
