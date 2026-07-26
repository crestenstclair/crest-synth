## Why

Crest Synth has a polymorphic control model and prepared rack, but the production application still installs only SoundFont instruments. This core slice admits Braids as a materially different second capability and makes the canonical capability model express its descriptor, voice policy, scalar values, and explicit no-fallback failures.

## What Changes

- Vendor and pin the desktop-safe Mutable Instruments Braids `MacroOscillator` C++ subset and its pinned `stmlib` dependency, preserve their MIT notices, and wrap them behind an exception-free opaque C ABI.
- Install `instrument.braids` beside `instrument.soundfont.hidef`, with descriptor-owned Model, Timbre, and Color Scalars and an explicit sixteen-voice-per-Patch policy.
- Extend the immutable capability model with engine-managed versus fixed-per-Patch voice policy and generic Scalar adjustment while Structural values remain preparation-only.
- Prove independent Braids Patch voice banks, deterministic stealing, explicit 96-to-48 kHz adaptation, native lifetime ownership, and typed source/config/rate failures without fallback.
- Keep the integrated Phase 2 architecture in `DESIGN.md`; control/demo, envelope-proof, and audio-runtime requirements are carried by the three named companion changes created by the validator-driven decomposition.

## Capabilities

### New Capabilities

- `braids-engine`: Pinned Mutable Instruments C++ synthesis, explicit 48 kHz host/96 kHz oscillator adaptation, sixteen independently prepared voices for every Braids Patch, descriptor-owned parameters, typed failures, and hard-real-time proof.

### Modified Capabilities

- `instrument-capability-model`: Install two real descriptors, distinguish fixed-per-Patch from engine-managed voice policies, and allow descriptor-classified Scalar values to be adjusted generically while Structural values remain preparation-only.

## Impact

This core slice affects the evaluated capability/Synth declarations, generic instrument configuration, production provider composition, the Braids adapter, native wrapper, vendored source, and the named Braids acceptance target. The integrated implementation remains shared with `phase-2-polymorphic-control-demo`, `phase-2-polymorphic-envelope-proof`, and `phase-2-polymorphic-audio-runtime`. The C++ source is pinned to `pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4` with `stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec`; no runtime loading, plugin, networking, or async dependency is added.
