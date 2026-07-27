# Change Summary

## Outcome

- **Problem:** Patches currently flow directly from prepared instruments into the mixer, leaving Phase 4's per-Patch effect boundary unproven.
- **Result:** The first fixture Patch gains one audible, editable, fallback-free Chorus before mix/routing; other fixture Patches remain effect-free.

## Change Outline

- **Adds:** Separate effect capability/config/provider/preparer contracts, stable effect slots, a Patch-aligned prepared effect rack, fixed effect scalars, and causal stage observations.
- **Changes:** PATCH appends read-only Chorus identity plus descriptor-derived Amount/Depth; complete preset/engine replacements preserve effect config/layout.
- **Excludes:** Effect selection, bypass, reorder, multiple slots, modulation, arbitrary routing, plugins, and seamless tail migration.

## System Impact

- **Capabilities:** Adds `static-patch-effect`; modifies instrument capability, PATCH, one-way control, prepared rack, realtime, global mix, structural selection, and both demo contracts.
- **Architecture:** Centers `valueObject.Synth.PostEffectConfig`, `port.Synth.EffectPreparer`, `aggregate.RealTime.PreparedPostEffectRack`, `applicationService.RealTime.AudioRenderer`, `domainService.Mixer.MixEngine`, and `adapter.ChorusPreparer`.
- **Interfaces/data:** Versioned state/page/text/parameter/audio/demo schemas gain effect registry, slot, scalar, focus, and measurement fields.

## Delivery

- **Implementation:** Domain/ports → PATCH/projection → prepared rack/renderer → pinned native adapter → structural composition → demos/reports → acceptance gates.
- **Source policy:** Uses the minimal MIT-licensed Rings Chorus subset at the declared eurorack/stmlib pins, exact hashes, independent 2,048-sample buffers, and 48 kHz-only admission.
- **Validation:** Requires the focused release target, affected named tests, all-target test/clippy/format gates, smoke, deterministic demo, and an actual physical `make demo-live` through teardown and exit 0.

## Risks and Decisions

- **Key decisions:** Effects parallel instrument capability ownership but share canonical parameter types; processing order is engine → effect → mixer; complete graphs retire effects off callback.
- **Main risks:** Vendored-source completeness, exact-rate device compatibility, tail-sensitive comparisons, stale scalar activation, and live no-progress hangs have explicit provenance, rejection, causal-measurement, refresh, and timeout mitigations.
- **Open questions:** None; `DESIGN.md`, all delta specs, tasks, `DESIGN.md`/`ROADMAP.md`, and the evaluated CUE package agree on the final plan.
