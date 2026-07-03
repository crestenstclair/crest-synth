# HANDOFF — crest-synth session bootstrap

_Last updated: 2026-07-03, end of the big autonomous run. Read this top-to-bottom
before doing anything; it is the state of the world._

## What this is

A fully **generated** Rust synthesizer. The CUE spec in `spec/` is the source of
truth; `src/` is written by the crest-spec generate→validate→retry loop. **Never
hand-write `.rs` files** — to change the product, change the spec (use the
`spec-authoring` skill) and run generation (`spec-generate` skill / the HTTP
drivers below). This rule has no exceptions.

Current state on `main` (through commit `a45800b` + possibly-uncommitted staged
spec edits, see "In flight"):

- **12 bounded contexts**, ~180 spec resources: Kernel, Engine, Sample, Effects,
  Mixer, Modulation, Patch, Preset, RealTime, Shell, MidiFile, Editor,
  DesignSystem, Loop, Plugin.
- **One-way event loop** (`Loop` context): `AppEvent` union → pure
  `AppState.apply` (the ONLY control-plane mutation path) → views render state;
  audio thread consumes `ParameterSnapshot`s via `StateProjector` →
  `ParameterBridge`. `synth_ui` runs on this loop.
- **Scenes**: serialized event sequences (`scenes/*.json`) executed through the
  same apply path (`scene_run`, `make demo-scenes`, `make scene FILE=...`).
  Deterministic JSON snapshots make the control plane evaluable by a human or
  an LLM without reading Rust.
- **Behavioral verification**: ~130 falsification-gated checks, board green at
  handoff time (every existing check graduated: real witness passes AND a
  degenerate stub fails). State lives in `.crest-spec/state.db` (gitignored).
- **Proof harness**: `make help` lists everything — demo binaries with measured
  outputs (`demo-voices` → `steals=N`, etc.), smokes with exit-code teeth,
  ~1200 generated unit tests.
- **Plugin context** (nih-plug): generated; crate builds `cdylib + rlib`.
  No `.clap`/`.vst3` bundle step yet (open decision, see backlog).

## How to run / test (human)

```bash
make ui FILE="./midi/Corridors of Time - Chrono Trigger.mid"  # window + gamepad + music
make play FILE=...      # headfirst playback
make watch [FILE=scenes/showcase.json]   # LIVE observation mode (after the staged
                        # live-scenes increment lands): scene drives the app,
                        # window open, audio on, captions streaming
make demo-scenes        # 4 scenes + jq state assertions
make smoke / tone / test / lint
make demo-voices demo-samples demo-effects demo-mod demo-patches demo-presets demo-midi
```

## The generation loop (how to run a session here)

The engine is the crest-spec repo's binary. `.mcp.json` wires it for normal
Claude sessions (stdio); for workflow-driven runs use the HTTP transport:

```bash
# 1. Launch the engine (keep stdin open or it exits):
cd ~/workspace/crest-synth
tail -f /dev/null | CREST_SPEC_SPEC_DIR=./spec CREST_SPEC_HTTP_ADDR=127.0.0.1:8792 \
  CREST_SPEC_GENERATE_MODEL=claude-sonnet-5 CREST_SPEC_MAX_RETRIES=3 \
  ~/workspace/crest-spec/bin/crest-spec > .crest-spec/server.log 2>&1 &
```

**CRITICAL: `CREST_SPEC_GENERATE_MODEL` must stay `claude-sonnet-5`** — the model
label feeds every effective hash; changing it re-plans the entire project.

All calls are JSON-RPC over `POST http://127.0.0.1:8792/mcp`
(`{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"spec/X","arguments":{...}}}`;
result JSON is in `.result.content[0].text`).

Session lifecycle: `spec/validate` → `spec/plan` (review the action list BEFORE
running — a correct increment is small) → `spec/begin` → run a driver workflow →
`spec/finish` (it REFUSES while behavioral checks are unresolved; `force` is a
human decision, never a default) → `git commit` spec + generated code together.

Drivers in `.crest-spec/` (run via Claude Code's Workflow tool, `{sessionId, endpoint}` args):

| Driver | Purpose |
|---|---|
| `spec-generate-http.js` | The wave loop: one sonnet generator per resource, whole-tree cargo gate per wave, triage. |
| `spec-behavioral-http.js` | design → tasks → verify for a context list (`contexts` arg). Verifiers prefer `scene_run` as witness. |
| `spec-reauthor-http.js` | Replace defective checks per resource (`resources: [{resource_id, feedback}]`) + verify fresh ones. |
| `spec-verify-pending-http.js` | Re-run verifiers for all pending checks. |

**One workflow at a time** against the engine (they share the tree and cargo).

Engine repo: `~/workspace/crest-spec` (pushed to origin/main). Rebuild with
`make build` there; restart the server here to pick it up.

## In flight at handoff

1. **Plugin behavioral polish** — a re-author round for 6 Plugin checks
   (5 incomplete verifiers + one symbolic-`member` check) may still be running
   or just finished. Reconcile with:
   `spec/sql: SELECT state, COUNT(*) FROM checks GROUP BY state` — expect all
   `graduated`. If stragglers remain: ONE sharper re-author round, then stop
   and report; never force.
2. **Live-scenes increment (STAGED, not yet generated)** — spec edits already
   in `spec/manifest.cue` (check `git status`): `synth_ui --scene` live playback
   with captions, the `scenes/showcase.json` observation scene, `make watch`,
   plus a fix for missing Loop dependency edges on SynthUiMain.
   `spec/plan` should show exactly 3 modifies (SynthUiMain, SceneLibrary,
   BuildMakefile). Run: begin → `spec-generate-http.js` → finish → commit.
   **This is the user's top priority: scenes that launch and PLAY so they can
   observe and identify issues.**
3. After both: push `main`.

## Backlog (in priority order)

- **User-observation feedback loop**: the user will run `make watch` / `make ui`
  and report what looks/sounds wrong. Each finding becomes a spec invariant or
  a scene + validation — fix through the loop, never patch by hand.
- **nih-plug bundle step** (user decision pending): conventionally an xtask
  bundler (`nih_plug_xtask`) produces the `.clap`/`.vst3`. The original spec
  never had one, so it was not invented. Ask before authoring.
- **Engine validation for symbolic `member` values**: twice now, checks were
  authored with `member:"SR"`-style back-references to witness arguments,
  compared literally. Candidate structural validation in crest-spec
  `internal/check` (like the existing field-name and missing-bounds gates).
- **Snapshot `frame` field placement**: top-level `frame` read as None when
  evaluating scenes externally; harmless (summary line carries frames) but
  worth normalizing in a future SnapshotCodec pass.
- Hardware MIDI-in and long-session RT stability are proven only indirectly.

## Hard-won rules (violate at your peril)

- **Validations and demo assets are the user's accumulated work product.** Port
  and preserve them verbatim; never "clean them up" away. (This was violated
  once and cost a full restoration effort.)
- **Iterate, don't regenerate**: the engine serves prior attempts back in
  UPDATE mode with `## Previous Errors` and `## Guidance`; a minor bug gets a
  minor fix. Blank-slate is only for a resource's very first attempt.
- **The gate polices design, not files**: `cargo fmt` auto-normalizes (never
  blocks), no drift policing, hand-edits to generated files are the user's
  right (but changes should flow through the spec).
- **Halts are loud**: non-convergence stops the run with the unresolved list.
  `spec/skip` requires quoting the actual spec contradiction. No auto-skips,
  no forcing gates.
- **Behavioral checks**: predicate field names are short measurable keys
  (engine-enforced); every numeric bound explicit (engine-enforced); `member`
  is always a concrete literal; every check must fail on a no-op stub.
- **crest-spec repo relationship**: this project is the flagship consumer of
  the engine at `~/workspace/crest-spec`. Engine bugs found here get fixed
  there (TDD), pushed, binary rebuilt, server restarted.

## Where deeper context lives

- `docs/superpowers/plans/2026-07-02-event-loop-demo-scenes.md` — the executed
  event-loop/scenes plan (tasks 1–6, all done except live-scenes staging above).
- ICM memory (`icm recall "crest-synth"` / topics `context-crest-spec`,
  `preferences`, `errors-resolved`) — the full decision and correction history.
- `DESIGN.md` — the product design document (architecture, contexts, phases).
- Old spec intent (editor tours, plugin, design system): git history on the
  pre-merge `main` side of commit `3ed889f`.
