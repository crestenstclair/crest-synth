# WP06 hardware evidence wall — webview shell cutover

Mission `webview-shell-cutover-01KZAC7Q`, WP06 (spec C-007, FR-003, NFR-001,
NFR-002; ROADMAP gate "retained evidence survives the cutover or the cutover
does not complete").

Rig: Mac15,6, macOS 26.5.2, arm64 — real window, physical audio output, no
other heavy load. All runs executed 2026-08-06 (UTC times below are log
mtimes / in-log headers). Every log is committed complete and untrimmed.

## C-007: these commits precede the egui deletion

The evidence wall (four scene logs, RT A/B logs + comparison, soak logs) was
committed as **`69fa5eb80ffbc7b71c403822b1a46ed45229857b`** on
`feat/webview-shell-cutover` (the mission's planning branch, where evidence
lives per the lane gate); this README follows in the immediately subsequent
commit on the same branch. The producing script
`scripts/rt_ab_measurement.sh` is committed on the WP06 lane branch
(`9b208c97718124af1e802fbff982c57ffa32af44` on
`kitty/mission-webview-shell-cutover-01KZAC7Q-lane-f`, which also carried the
first copy of these logs before the lane gate relocated them here). No egui
deletion commit exists yet anywhere in the mission — WP07's deletion is
forbidden until it links to these hashes and postdates them (spec C-007).

## T023 — four retained scenes on the webview shell (FR-003)

Every run: `make demo-live-<scene>` on this rig, process exit 0, report
`complete` per its `CREST_LIVE_SUMMARY`, EventLog `0 dropped` (lossless),
`callbackAllocations=0`, `callbackDestructions=0`, and clean teardown
(`cleanup=true`, `activeNotes=0`, `window_closed=true`, `stream_released=true`,
`owned_graphs_remaining=0`, `physical_audio_nonzero=true`). Qualifying webview
frames come from the WP02 forwarding path and correlate with the checkpoint
stream (nonzero on every run, ≥8 required by the report).

| Scene | Finished (UTC) | Exit | Report | Checkpoints | Qualifying webview frames | Events | audioUninterrupted false/true | Teardown | Log |
|---|---|---|---|---|---|---|---|---|---|
| graphical-shell | 06:58:44Z | 0 | complete (105/105 params, 3/3 engine) | 114 | 1868 | 14254, 0 dropped | absent (0 topology cp) | clean | `graphical-shell-live-run.log` |
| semantic-view-model | 07:00:28Z | 0 | complete (105/105 params, 3/3 engine) | 114 | 1872 | 14188, 0 dropped | absent (0 topology cp) | clean | `semantic-view-model-live-run.log` |
| sixteen-track-mixer-routing | 07:02:07Z | 0 | complete (105/105 params, 3/3 engine) | 114 | 1884 | 14196, 0 dropped | absent (0 topology cp) | clean | `sixteen-track-mixer-routing-live-run.log` |
| effects-and-buses | 07:03:44Z | 0 | complete (105/105 params, 3/3 engine) | 144 (30 topology) | 2198 | 15102, 0 dropped | **0 false / 30 true** | clean | `effects-and-buses-live-run.log` |

`audioUninterrupted` is emitted only by topology checkpoints, so for the
three scenes without topology transitions it is absent by schema (not a
measured 0); their per-checkpoint audio-continuity witness is
`audioPredicatePassed` on every parameter checkpoint, which a failed
predicate would have turned into a run failure — all four runs completed.

## T025 — identity comparison on the hardware logs (spec C-004, add-only)

Extracted from each committed log's emitted `CREST_LIVE_CHECKPOINT` stream
(`jq 'select(.kind=="engine" or .kind=="topology") | .checkpoint.transition'`
after stripping the marker), condensed to distinct identities in emission
order (each identity emits one checkpoint per lifecycle stage — engine
identities emit 3 checkpoints each), then checked against the frozen
baselines in `src/testing/live_demo_scene.rs`
(`FROZEN_ENGINE_TRANSITION_IDENTITY_BASELINE`, 3 identities) and
`src/testing/live_effects_and_buses_scene.rs`
(`FROZEN_TOPOLOGY_IDENTITY_BASELINE`, 17 identities).

| Scene | Baseline | Preserved (byte-identical, in order) | Modified | Removed | Pure insertions |
|---|---|---|---|---|---|
| graphical-shell | engine (3) | 3/3 | 0 | 0 | 0 |
| semantic-view-model | engine (3) | 3/3 | 0 | 0 | 0 |
| sixteen-track-mixer-routing | engine (3) | 3/3 | 0 | 0 | 0 |
| effects-and-buses | engine (3) | 3/3 | 0 | 0 | 0 |
| effects-and-buses | topology (17) | 17/17 | 0 | 0 | 13 |

The 13 topology insertions (all pure insertions between baseline members, in
emission order): `SlotOccupant.scalarEdited`, `SlotFill.secondCycle1`,
`SlotFill.thirdCycle1`, `SlotFill.thirdCycle2`, `Return.contentChangedCycle1`,
`Return.emptyOccupiedCycle1`, `Return.emptyOccupiedCycle2`,
`Topology.recoveredAfterRefusalCycle1`, `Slot.startupOccupantRestoredCycle1`,
`Slot.thirdClearedCycle1`, `Slot.thirdClearedCycle2`,
`Return.emptyRestoredCycle1`, `Return.emptyRestoredCycle2`.

## T022 — same-workload RT A/B, egui baseline vs webview (NFR-001, RISK-3)

`scripts/rt_ab_measurement.sh` ran the byte-identical workload
(`cargo run --release --bin crest-synth -- --demo-live-sixteen-track-mixer-routing`)
egui-hosted at pre-cutover baseline `d41e7bd` (throwaway git worktree,
07:04:07Z header) and webview-hosted at lane HEAD `b966da8`
(07:06:52Z header). Full field-by-field table: `rt-ab-comparison.md`; raw logs
`rt-ab-egui.log` / `rt-ab-webview.log`. Headline measured numbers
(egui / webview):

- process exit code: 0 / 0; report completeness: complete / complete
- callbackAllocations: **0 / 0**; callbackDestructions: **0 / 0** (RT
  callback global-allocator witness)
- EventLog dropped records: 0 / 0; audioPredicatePassed failures: 0 / 0
- checkpoints 114 / 114 (105 parameter, 9 engine each); audioObservation
  sequence monotonic: true / true
- whole-process (external `/usr/bin/time -l`, supplementary): real
  86.90 s / 81.23 s, user CPU 9.42 s / 6.84 s, sys 2.27 s / 1.64 s, max RSS
  661,504,000 / 674,021,376 bytes
- Every numerically comparable bound is within the egui baseline envelope
  (webview ≤ egui); per-thread audio-callback CPU is not carried by the
  production observation and is reported absent, not invented.

Tooling narration (nothing replaced silently): the first script invocation
ran both live sides successfully and wrote both logs, but its report
generator exited 2 under macOS bash 3.2 (`declare -A` unsupported). The
script was fixed for bash 3.2 and the comparison regenerated at 07:12:28Z
via `--reuse-logs` from the unmodified measured logs; the run phase of the
script was not changed.

## T024 — 300 s soak (NFR-002, RISK-4)

- `CREST_WEBVIEW_TESTS=1 CREST_WEBVIEW_FULL_SOAK=1 cargo test --test
  webview_projection_shell -- --nocapture` (log: `soak-300s.log`): meter
  cadence over 300 s = **29.43 Hz sustained** (declared 30 Hz pace,
  interval-quantized floor 29.0; first/last-third 29.43/29.43 Hz — no
  degradation), max gap 44.3 ms, **lost 0**, 239,013 observations coalesced
  into 8,830 emits (pending slot bounded at one; emit count ≤ pace bound
  9,002), page-side received **8,830/8,830** frames (lossless) at 29.43 Hz;
  process exit 0, `CREST_ACCEPTANCE webview_projection_shell passed
  (skipped: none)`.
- Leak metric (external sampler, `soak-300s-rss.samples.log`, 10 s cadence):
  the soak-window RSS plateau declined from 107,728 KiB to 103,904 KiB over
  the final three minutes and ended at 94,160 KiB — **no growth trend**
  (the spec declares no numeric leak bound; the measured series is committed
  for grading).
- Same run, NFR-001 measured on hardware: projection-to-paint over 150 paced
  reducer edits **p50 7.9 ms / p95 8.8 ms / max 10.9 ms** against the
  declared p95 ≤ 50 ms threshold — this is the measured figure for the
  plan's "reducer change visible ≤ 50 ms p95" goal.

## Artifact index

| Artifact | Produced by |
|---|---|
| `graphical-shell-live-run.log` | T023 `make demo-live-graphical-shell` |
| `semantic-view-model-live-run.log` | T023 `make demo-live-semantic-view-model` |
| `sixteen-track-mixer-routing-live-run.log` | T023 `make demo-live-sixteen-track-mixer-routing` |
| `effects-and-buses-live-run.log` | T023 `make demo-live-effects-and-buses` |
| `rt-ab-egui.log`, `rt-ab-webview.log`, `rt-ab-comparison.md` | T022 `scripts/rt_ab_measurement.sh` |
| `soak-300s.log`, `soak-300s-rss.samples.log` | T024 full soak + external RSS sampler |
| `README.md` | T025 (this file) |
