## 1. Establish the Reconciliation Baseline

- [x] 1.1 Resolve the evaluated CUE project and relationship index, map the six declared implementation assets and their canonical target resources to the existing repository files, and identify concrete missing or contradictory items before editing code.
- [x] 1.2 Audit `Cargo.toml` for the one library, `crest-synth`, and verification-only `crest-synth-witness` targets plus the declared dependency allowlist; repair only mismatches and prove the result with `cargo metadata --no-deps --format-version 1`.
- [x] 1.3 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets` as the initial baseline, assigning each failure to its affected capability and canonical resources rather than performing broad rewrites.

## 2. Reconcile SoundFont Audio and Automatic MIDI

- [x] 2.1 Audit and reconcile Patch identity, MIDI channel/message values, instrument configuration, the SoundFont port, and the HiDef adapter so one shared `./sf2/HiDef.sf2` engine produces distinct finite bounded Patch stems and fails clearly on an invalid bank.
- [x] 2.2 Audit and reconcile instrument discovery, stable one-Patch-per-identity assignment, unique bounded channels, channel-exhaustion failure, and exact bank/program/percussion routing for the fixed Corridors of Time fixture.
- [x] 2.3 Audit and reconcile automatic fixture initialization and ticks so MIDI timing remains adapter-private, playback starts without transport input, and every emitted event traverses the production event, state, command, and Patch-routing path.

## 3. Reconcile Control and Mixing Behavior

- [x] 3.1 Audit and reconcile the AppState invariants and typed Patch/global parameter bounds, including nonfatal boundary rejections, section wrap, differing Patch/GLOBAL parameter-count clamps, and acceptance of a later valid edit.
- [x] 3.2 Audit and reconcile the control loop so accepted state commits before event logging, serialization, text projection, parameter publication, and audio-command emission, while rejected transitions preserve generations and exact before/after state hashes.
- [x] 3.3 Audit and reconcile state-tree, text, and parameter projection so every required property has an exact same-generation value and an edit changes only its targeted Patch or global field.
- [x] 3.4 Audit and reconcile W/S/A/D navigation, K-modified fine/coarse adjustment, the one scrollable complete text body, Patch separators, selection marker, and production GUI callback path without adding direct key-to-state mutation.
- [x] 3.5 Audit and reconcile the mixer and shared-effects adapter so each Patch retains an independent dry/send contribution and exactly one reverb plus one delay consume only their supplied sends across all seven global controls.

## 4. Reconcile Real-Time and Application Composition

- [x] 4.1 Audit and reconcile bounded audio commands, complete parameter snapshots, Patch audio blocks, and the lock-free boundary so ready commands and the newest complete generation cross without allocation, locks, blocking, or partial publication.
- [x] 4.2 Audit and reconcile ownership retirement so replaced engine or snapshot data is destroyed on the control side and never in the audio callback.
- [x] 4.3 Audit and reconcile the audio renderer so commands reach the exact Patch, independent stems feed the global mixer, and the callback produces finite nonzero bounded stereo output without allocation, locking, blocking, I/O, logging, or destruction.
- [x] 4.4 Audit and reconcile the standalone composition root and adapters so normal startup uses only the fixed SoundFont and MIDI fixture, while `--smoke`, `--observe`, `--demo-scene`, and the two verification-only degenerate flags obey their declared combinations, outputs, and exit behavior.

## 5. Reconcile Deterministic Behavioral Proof

- [x] 5.1 Audit and reconcile the exhaustive demo so it uses the production fixture, input translator, control loop, projections, real-time boundary, SoundFont path, and mixer and produces byte-identical event-log, state-tree, coverage, checkpoint, and report JSON across two fresh runs.
- [x] 5.2 Audit and reconcile current-surface discovery and coverage so production-owned input and typed parameter descriptors plus discovered serialized leaves exactly equal observed inputs, events, directions, MIDI kinds, editable values, properties, rejections, sources, commands, and effects with both missing and unexpected empty.
- [x] 5.3 Audit and reconcile causal observations so every scene step records exact tagged inputs, outcomes, generations, hashes, emitted effects, projections, and audio consequences and every reversible parameter, send, selection, and projection returns exactly to its captured baseline.
- [x] 5.4 Audit and reconcile faithful audio evidence so Patch isolation uses discriminating stems, wet comparisons use nonzero supplied sends and identical effect state, zero sends receive no dry-derived excitation, and output measurements cannot be replaced by success text.
- [x] 5.5 Audit and reconcile the headless egui integration path so real GUI key/focus input flows through the production update callback and the next frame, event record, accepted value, exact text projection, selection, and scroll target agree.

## 6. Reconcile Acceptance and Mutation Assets

- [x] 6.1 Audit and reconcile the four explicit integration targets—`exhaustive_demo_scene`, `schema_surface`, `eframe_context`, and `behavioral_mutation_harness`—so each calls public production seams, executes concrete assertions, and prints its exact acceptance marker only after passing.
- [x] 6.2 Audit and reconcile the mutation harness so dropped adjustment, cross-Patch parameter leak, Patch misroute, omitted state-tree leaf, dry-to-wet bypass, and zero renderer each alter only the named seam and produce measured schema-valid healthy and falsifying observations.
- [x] 6.3 Audit and reconcile `crest-synth-witness` argument validation and composition so it accepts only the six declared case/mutant pairs, emits exactly one `CREST_MUTATION_OBSERVATION` line, exits 0 for healthy cases and 1 for matching mutants, and remains unreachable from the interactive product.
- [x] 6.4 Audit and reconcile the Makefile default help and Cargo-backed `build`, `check`, `test`, `lint`, `fmt`, `fmt-check`, `run`, `play`, `ui`, `smoke`, `observe`, `demo`, and `clean` targets; prove the human entry points with `make -n ui`, `make -n observe`, and `make -n demo`.

## 7. Execute Completion Gates

- [x] 7.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets` and repair every failure without changing the CUE architecture or declared non-goals.
- [x] 7.2 Run `cargo test --test exhaustive_demo_scene -- --nocapture`, `cargo test --test schema_surface -- --nocapture`, `cargo test --test eframe_context -- --nocapture`, and `cargo test --test behavioral_mutation_harness -- --nocapture`; require exit 0 and each exact `CREST_ACCEPTANCE <target> passed` marker after behavioral assertions.
- [x] 7.3 With the fixed fixtures available, run `cargo run --bin crest-synth -- --smoke`, the running-synth and control-path observations, and the exhaustive demo positive/negative witness commands; verify their structured predicates and declared exit behavior rather than marker presence alone.
- [x] 7.4 Execute all six `crest-synth-witness` healthy/mutant pairs, requiring schema-valid measured observations, healthy exit 0, matching-mutant exit 1, and falsification at only the named causal seam.
- [x] 7.5 Run `openspec validate bootstrap-specs-and-reconcile-implementation`, recheck the evaluated CUE context for unintended architectural drift, and confirm all six capability specs and every required project check are complete before archive.
