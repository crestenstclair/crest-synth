## Why

The production standalone path currently bypasses declared composition ports, prepares audio for fixed defaults before learning the device configuration, drops post-start device failures, and erases unknown-Patch routing failures. Three CUE-declared test selectors also succeed without executing a test, so current acceptance can report success without proving those contracts.

## What Changes

- Make the standalone binary own and inject the installed capability providers, preparers, structural graph boundary, and audio observation boundary; reject missing, duplicate, unknown, and mismatched registrations before graph publication.
- Separate physical-device negotiation from stream start, validate a typed device configuration before preparation, build the complete graph for that exact configuration, and reject or fully service callback buffers without silent truncation.
- Add a bounded callback-to-control device-status path so post-start device failures become typed application-visible outcomes outside the real-time callback.
- Preserve fixed-size unknown-Patch routing failure observations through the production renderer without fallback or mutation.
- Repair the three declared CUE test selectors and add structured acceptance that fails when any test-bearing validation selects zero tests.
- Preserve the current installed-engine set and all later-phase exclusions; this repair adds no product engine, UI surface, effect, modulation, or graph-edit feature.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instrument-capability-model`: Require the production composition root to supply capability providers and preparers through the declared ports and make invalid registration combinations observable before publication.
- `prepared-engine-rack`: Require production injection of the structural handoff, exact negotiated configuration ownership, and observable unknown-Patch routing failure at the renderer boundary.
- `realtime-execution`: Require negotiate-before-prepare device startup, complete bounded callback servicing, typed post-start device failure delivery, and truthful targeted validation execution.
- `live-observable-demo`: Require physical-device runtime failures to leave the apparently healthy live state and become a typed visible application outcome.

## Impact

The change affects the Shell and RealTime CUE contracts, the standalone composition API, physical audio-output port and CPAL adapter, graph preparation and renderer observations, production startup orchestration, test fixtures, and validation declarations. It changes internal Rust construction and audio-output APIs but does not change the end-user command surface or add dependencies.
