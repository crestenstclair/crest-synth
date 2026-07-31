# WP05 Review — Cycle 1 (reviewer-renata)

Verdict: CHANGES REQUESTED — one concrete issue. The real-time core of this WP
(T026–T028, T030, T031, exact matching, cap lift, bridge deletion, measurements)
verified clean; do not rework any of it. The single failure is an incomplete
mirror of the WP05 leaf rename inside `src/bin/crest_synth.rs`.

## Issue 1: `make demo` observation now misreports four health fields as false

WP05 renamed the serialized mixer-track send leaves to the indexed `sends`
array and reduced the snapshot mirror's `global` object to `masterGainDb`
(both correct, T027/T029). The commit message claims "demo leaf paths follow
(T029)", and `final_tree_values_are_exact` in `src/bin/crest_synth.rs` was
updated — but two sibling check functions in the same file were missed:

1. `fixed_fixture_baseline_restored` (src/bin/crest_synth.rs:783-784) still
   requires `track.pointer("/reverbSend") == Some(0.0)` and
   `track.pointer("/delaySend") == Some(0.0)`. Mixer tracks in the state tree
   now serialize `{levelDb, pan, mute, solo, sends[8]}` (verified against the
   live CREST_STATE_TREE output), so this function can never return true.

2. `parameter_projection_matches_state` (src/bin/crest_synth.rs:988) still
   requires `parameters.get("global") == Some(global)`. The tree's top-level
   `global` keeps the seven-key retired surface until WP06, while
   `parameters.global` now carries only `masterGainDb` (deliberate, T029), so
   this comparison can never return true either.

Measured evidence (both runs of `cargo run --bin crest-synth -- --smoke
--observe --demo-scene`, exit 0 both times):

- At WP05 base (49ae07b^): false fields =
  `[all_parameter_boundaries_exercised, selection_clamps_exact]` (pre-existing).
- At WP05 HEAD: false fields additionally include
  `baseline_restored`, `parameter_projection_matches_state`,
  `exact_state_values`, `exact_projection_values` — all four are WP05
  regressions cascading from the two stale functions above.

Why this is material: the exhaustive demo observation is part of the mission's
falsifiable-proof surface, and WP06/WP07/WP08 will run `make demo` against
their own changes. A diagnostic channel that permanently reports
`baseline_restored:false` and `exact_state_values:false` while exiting 0 is
exactly the silent-under-reporting class T029/C-RT-6 exists to prevent, and it
destroys the signal later WPs need. This file was explicitly flagged for
manual review with the expectation that ONLY the internal leaf checks changed —
they changed incompletely.

### Fix (small, one file)

In `src/bin/crest_synth.rs`:

- `fixed_fixture_baseline_restored`: replace the two named-send pointer checks
  with a check that `/sends` is an 8-element array of exact `0.0` values
  (mirror of the updated `final_tree_values_are_exact` shape, but with the
  baseline's exact-zero expectation).
- `parameter_projection_matches_state`: compare only the shared leaf, i.e.
  `tree.pointer("/global/masterGainDb") == tree.pointer("/parameters/global/masterGainDb")`
  — the same fix WP05 already applied correctly in
  `src/testing/live_demo_scene.rs` (see the "/global/masterGainDb" comparison
  added there), with a comment noting WP06 retires the remaining global
  reverb/delay leaves.

Acceptance for the re-review: run
`cargo run --bin crest-synth -- --smoke --observe --demo-scene` and confirm the
CREST_OBSERVATION false set is back to exactly the pre-existing pair
`[all_parameter_boundaries_exercised, selection_clamps_exact]` (those two
belong to later WPs, do not chase them). Then `make lint`, `make fmt-check`,
`cargo test --all-targets` all green as before.

## Verified clean — do not touch

- T026: `effects: [RtPostEffectParameters; MAX_EFFECT_SLOTS]`; all four
  construction sites (`new`, `projected`, `projected_with_effects`,
  `inactive`) initialize every slot; fixed `[f32;N]` storage; `Copy` +
  `needs_drop == false` asserted in tests.
- T027/T028: indexed `sends: [f32; 8]`, `RtBusReturnParameters` +
  `returns[8]`, reused error variants, non-finite/capacity negative tests.
- Central hazard: `matches_parameters` is per-position exact on BOTH racks
  (position zip; empty↔inactive and occupied↔attesting-entry both directions);
  the WP03 position-agnostic `flatten()` arm is gone; the first-occupied
  `slot_id(index)`/`scalar_count(index)` accessors are deleted;
  `process_with_slot_observations` feeds exactly one entry per position.
  Negative tests cover wrong-position, partial attestation, cross-position
  exchange, wrong slot_id, wrong scalar_count, both directions, on both racks.
- The three caps changed atomically in 49ae07b: `MAX_POST_EFFECTS_PER_PATCH =
  MAX_EFFECT_SLOTS`, widened `PreparedGraphLayout` (per-position grid + return
  topology) compared in `permits_selected_replacement` with negative tests.
- Double-wet: `src/adapter/global_reverb_delay.rs` deleted; production graphs
  run `MixEngine<NullGlobalEffects>`; the wet path exists once through the
  return rack; migration proof
  `racked_default_returns_match_the_retired_processor_sample_for_sample` is a
  verbatim transcription (checked against `git show 2b6617f^:src/adapter/
  global_reverb_delay.rs` — constants, damping filter, feedback formula, clamp,
  ceil all identical), noise input, 8 blocks x 4 parameter sets, `assert_eq!`
  exact.
- T029: descriptor completeness test enumerates recursively from the real
  serde output, not a hand-copied list; serde pin test updated byte-precisely.
- T031 measured live in release: snapshot = 4632 bytes, publish mean = 65 ns,
  read mean = 52 ns over 10,000 triple-buffer iterations, ceilings asserted
  (16 KiB / 100 µs); 256 fully-occupied steady-state blocks + activation block
  with 0 allocations, 0 deallocations, 0 drops; activation atomic (replacement
  snapshot observed whole); retirement off-callback with graph returned intact.
- `cargo test --all-targets` exit 0; `make lint` and `make fmt-check` clean.
- Adjacent files (state_tree.rs, testing/*, mixer files, effect_capability.rs,
  static_patch_effect.rs, graphical_application_shell.rs) are mechanical
  mirrors of the widened shape / sanctioned cap lift — no later-WP behavior
  preempted. Coordination note: state_tree.rs (WP06-owned) and testing/*
  (WP08-owned) were touched only as shape mirrors.
