# One-Way Event Loop + LLM-Evaluable Demo Scenes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **This is a crest-spec project**: every task authors CUE declarations and runs the
> generate→validate→retry loop. NEVER hand-write `.rs` files. "Failing test first"
> maps to "validation declared with the asset before generation runs."

**Goal:** Restore the original crest-synth's totally event-driven, one-way data loop across the whole app (not just the editor), and add executable **demo scenes**: ordered event sequences a harness — human, script, or LLM — fires through the real dispatch path, then evaluates by reading serialized state snapshots.

**Architecture:** Elm/Flux. One `AppEvent` union is the *only* thing that mutates control-plane state; one pure `apply(state, event) -> state` reducer owns all transitions; views and adapters read state and emit events, never mutate. The RealTime boundary is unchanged — the audio thread consumes `ParameterSnapshot`s *projected from* `AppState`, keeping the existing RT invariants intact. Scenes are serialized event sequences + snapshot dumps over that loop, which makes the control plane hermetically evaluable: same events in, same state out, every time.

**Tech stack:** CUE spec + crest-spec loop (sonnet generators). Serialization rides the existing serde dependency (crate names stay confined to `manifest.cue`/adapter `framework:` fields).

**Sequencing:** AFTER the currently-queued work lands: editor increment (its `EditorState` reducer is the seed this plan generalizes), the behavioral verification pass, and the `respec/clean-base` → `main` merge.

## Global Constraints

- Spec declarations are behavioral: no Rust types beyond the established primitive vocabulary, no crate names outside `manifest.cue` + adapter `framework:`.
- Iterate, don't regenerate: every task is a small incremental plan (verify expected action count with `spec/plan` BEFORE running); existing settled resources must not cascade except where a declaration genuinely changes.
- Determinism is load-bearing: a scene replayed twice must produce byte-identical snapshots (fixed sample rate, no wall-clock, no unseeded randomness in the control plane). This is what makes LLM evaluation trustworthy.
- Every new behavior ships with a measured-value validation (a number computed from real state), never a token print.

---

### Task 1: The Loop bounded context — AppEvent, AppState, the reducer

**Spec files:**
- Create: `spec/loop.cue`
- Modify: `spec/project.cue` (contextMap additions)

**Declarations (complete, paste-adapt into `spec/loop.cue`):**

```cue
package crestsynth

// Loop — the one-way (Elm/Flux) data loop that owns the entire control plane.
// Events in, state out. Views render state; adapters emit events; NOTHING
// else mutates. The audio thread is downstream: it consumes ParameterSnapshot
// projections of AppState across the RealTime boundary, unchanged.

project: contexts: Loop: purpose: "the app-wide one-way event loop: a single AppEvent union, a single AppState store, and a pure reducer that is the only mutation path in the control plane"

project: contexts: Loop: ubiquitousLanguage: {
	AppEvent: "a semantic event — MIDI, gamepad, editor, mixer, patch, preset, scene control — the only thing that mutates AppState"
	AppState: "the single source of truth for the control plane: patches, mixer, editor view state, active preset/session"
	Reducer:  "apply(state, event) -> state; pure, total, allocation-free"
}

project: contexts: Loop: valueObjects: {
	AppEvent: {
		description: "closed union over every control-plane event: normalized MidiEvent, GamepadAction, editor navigation/edit events, mixer strip changes, patch commands, preset load/save; each variant carries its aggregate's existing command payload"
		invariants: ["every variant is serializable and deserializable losslessly (scenes are files)"]
	}
	EventRejection: {
		state: {reason: "string"}
		description: "why an event was not applicable in the current state (unknown target, out-of-range value); rejections are values, never panics"
	}
}

project: contexts: Loop: aggregates: AppState: {
	root:    true
	purpose: "the whole control-plane state as one value: patch set, mixer, editor view state, active session"
	state: {frame: "u64"}
	commands: {
		Apply: {event: "AppEvent"}
	}
	events: {
		Applied: {frame: "u64"}
		Rejected: {reason: "string"}
	}
	invariants: [
		"apply(event) is the ONLY way to mutate AppState — no setters, no direct field mutation anywhere in the control plane",
		"apply is pure and deterministic: the same initial state and the same event sequence always produce an identical AppState",
		"apply never panics: an inapplicable event yields an EventRejection value and leaves state unchanged",
		"frame increments by exactly one per applied event — it is the event-sequence clock, not wall-clock",
	]
}

project: contexts: Loop: domainServices: {
	StateProjector: {
		purpose: "projects AppState into the ParameterSnapshot the audio thread reads — the one bridge from the loop to the RealTime boundary"
		uses: ["aggregate.Loop.AppState", "port.RealTime.ParameterBridge"]
	}
}
```

**contextMap additions in `spec/project.cue`:**

```cue
	{from: "Kernel", to: "Loop", kind: "shared-kernel"},
	{from: "Loop", to: "RealTime", kind: "anti-corruption", direction: "downstream"},
	{from: "Patch", to: "Loop", kind: "customer-supplier", direction: "upstream"},
	{from: "Mixer", to: "Loop", kind: "customer-supplier", direction: "upstream"},
	{from: "Preset", to: "Loop", kind: "customer-supplier", direction: "upstream"},
```

**Project-level invariant addition (`spec/project.cue`, with the others):**

```cue
	{text: "all control-plane state mutation flows through the Loop reducer; views and adapters read state and emit events, never mutate", meta: rationale: "one-way data flow is what makes the control plane hermetically testable: feed events, read state"},
```

Steps:
- [ ] Author the declarations above (spec-authoring skill loaded first).
- [ ] `spec/validate` clean; `spec/plan` shows ONLY the new Loop resources as creates (+ the project-invariant guidance edit regenerating nothing else — verify no cascade).
- [ ] Run the generate driver; commit gate green.
- [ ] `git commit` spec + generated code.

### Task 2: Snapshots — serialized state an LLM can read

**Spec files:**
- Create: append to `spec/loop.cue`
- Modify: `spec/manifest.cue` (nothing new in deps — serde already present)

**Declarations:**

```cue
project: contexts: Loop: valueObjects: {
	StateSnapshot: {
		description: "a complete, deterministic serialization of AppState: stable field order, stable map ordering, no wall-clock timestamps — byte-identical for identical states"
		invariants: [
			"serializing the same AppState twice yields byte-identical output",
			"a snapshot round-trips: deserialize(serialize(state)) equals the original state",
		]
	}
}

project: contexts: Loop: ports: {
	SnapshotCodec: {
		contract: {
			encode: "(state: AppState) -> StateSnapshot"
			decode: "(snapshot: StateSnapshot) -> result<AppState, CodecError>"
		}
		meta: notes: "human- and LLM-readable text format (JSON); field names are the ubiquitous language, values are plain (dB as numbers, booleans as booleans)"
	}
}

project: adapters: SerdeSnapshotCodec: {
	implements: "port.Loop.SnapshotCodec"
	layer:      "infrastructure"
	meta: framework: "serde"
}
```

Steps:
- [ ] Author; `spec/plan` = 3 creates, no cascade.
- [ ] Generate; the round-trip and determinism invariants must be proven by generated unit tests (add `{kind: "test", command: ["cargo", "test", "state_snapshot"], ...}` validation on StateSnapshot).
- [ ] Commit.

### Task 3: Scenes — ordered event sequences as data

**Declarations (append to `spec/loop.cue`):**

```cue
project: contexts: Loop: valueObjects: {
	SceneStep: {
		state: {event: "AppEvent", renderBlocks: "u32"}
		description: "one step: apply an event, then optionally render N audio blocks headlessly (so audible consequences accrue between events)"
	}
	Scene: {
		state: {name: "string", steps: "list<SceneStep>"}
		description: "a named, ordered, serializable event sequence — the unit of control-plane demonstration and evaluation"
		invariants: ["a scene file round-trips losslessly through the SnapshotCodec's format"]
	}
}

project: contexts: Loop: domainServices: {
	SceneRunner: {
		purpose: "executes a Scene against a fresh AppState through the SAME apply path the live app uses: per step, apply the event, render the requested blocks, and (when asked) emit a StateSnapshot; produces the final snapshot plus per-step snapshots on demand"
		uses: ["aggregate.Loop.AppState", "valueObject.Loop.Scene", "port.Loop.SnapshotCodec", "domainService.Loop.StateProjector", "domainService.Engine.EngineRenderer"]
	}
}
```

Steps:
- [ ] Author; plan = 3 creates; generate; commit.

### Task 4: The `scene_run` binary + starter scene library

**Spec files:**
- Modify: `spec/manifest.cue` (two assets)

**Declarations:**

```cue
project: assets: SceneRunMain: {
	kind:        "rust-bin-target"
	description: "src/bin/scene_run.rs: execute a scene file and emit snapshots for evaluation"
	uses: ["domainService.Loop.SceneRunner", "port.Loop.SnapshotCodec"]
	prompts: [
		"File path: src/bin/scene_run.rs",
		"scene_run --scene <FILE> [--dump-every-step] [--out <FILE>]: load the scene, run it through SceneRunner, print the FINAL StateSnapshot to stdout as one JSON document; with --dump-every-step print one snapshot JSON per step first.",
		#"After the snapshot, print exactly one summary line: `events_applied=<N> rejections=<M> frames=<F> peak=<final rendered peak>` — measured from the run, and exit non-zero if any event was rejected (a scene that doesn't fully apply is a failed scene)."#,
	]
	validations: [
		{kind: "integration", command: ["make", "demo-scenes"], description: "the starter scene library applies cleanly and asserts state facts", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "events_applied="},
		]},
	]
}

project: assets: SceneLibrary: {
	kind:        "scene-library"
	description: "scenes/: starter scenes proving one behavior each, plus scenes/check.sh asserting state facts from the snapshots"
	uses: ["asset.SceneRunMain"]
	prompts: [
		"Directory: scenes/. Author FOUR scene files in the SnapshotCodec format plus a scenes/check.sh that runs each through scene_run and asserts snapshot facts with jq.",
		"mixer-solo: solo one of three strips, assert the snapshot shows the other two muted=true and the soloed one muted=false (solo exclusivity).",
		"volume-edit: navigate to a strip, enter edit mode, adjust volume down 6 dB, assert the snapshot volume field equals the expected value exactly.",
		"voice-steal: configure polyphony 2 with oldest-steal, fire 3 note-ons with renders between, assert active voice count is 2 and the frame clock equals the step count.",
		"preset-roundtrip: edit a patch, save preset, mutate again, load the preset, assert the snapshot's patch state equals the post-save snapshot's patch state.",
		"Every assertion reads a MEASURED value from the snapshot JSON — never a token the binary prints unconditionally.",
	]
}
```

Also in `spec/project.cue` assetKinds: `"scene-library": {description: "scene data files + their assertion script", filePattern: "scenes/*"}`, and add to the BuildMakefile prompt list: `demo-scenes (run scenes/check.sh via scene_run)` and `scene (run scene_run --scene "$(FILE)" --dump-every-step)`.

Steps:
- [ ] Author; plan = 2 creates + BuildMakefile modify.
- [ ] Generate. The gate proves the library: `make demo-scenes` must pass all four scenes' jq assertions.
- [ ] By-hand LLM check (the point of the feature): run `scene_run --scene scenes/mixer-solo.json --dump-every-step`, read the JSON as an agent would, confirm the state story is followable without reading any Rust.
- [ ] Commit.

### Task 5: Rewire `synth_ui` onto the loop

The app currently wires inputs to services directly. Move it onto the loop so live behavior and scene behavior are the same code path — that's what makes scene evaluation representative.

**Spec change:** on `asset.SynthUiMain`, add to `uses`: `"aggregate.Loop.AppState"`, `"domainService.Loop.StateProjector"`; add prompt:

```cue
		"ONE-WAY LOOP: every input path (MIDI, gamepad, editor keys) emits AppEvents into the single apply loop; views render from AppState only; the audio thread receives changes exclusively via StateProjector -> ParameterBridge. No input handler mutates state directly.",
```

Steps:
- [ ] Author; plan = 1 modify (synth_ui regenerates in UPDATE mode; BuildMakefile may cascade — expect ≤2 actions).
- [ ] Existing proofs are the regression net: `make smoke` (Megalovania + Corridors) and the editor increment's ui-smoke/tour validations must all still pass at commit.
- [ ] Commit.

### Task 6: Fold scenes into behavioral verification

Scenes make witnesses trivial: a verifier no longer authors a bespoke harness crate — it authors a *scene* and reads the snapshot. Predicate fields map directly onto snapshot fields.

Steps:
- [ ] In crest-spec (engine repo): update the verifier prompt in `.claude/workflows/spec-generate.js` (and the resume/verify-pending drivers) with: "If the project ships a scene runner (`scene_run`), PREFER it as the witness: author a scene exercising the behavior, run it, and map CREST_OBS fields from the snapshot JSON; the stub baseline is the same scene against a no-op reducer."
- [ ] Run a scoped behavioral pass over the Loop context itself: contracts for apply/purity/rejection semantics; verify with scene-based witnesses; graduate what passes.
- [ ] Author the graduated checks back into `spec/loop.cue` invariants (the graduation loop, closed end-to-end for the first time).
- [ ] Commit both repos.

## Self-Review Notes

- Coverage vs the ask: event-driven ✓ (Task 1 + 5 make the loop total, not editor-only); one-way data loop ✓ (project invariant + AppState invariants); demo scenes executed in order ✓ (Task 3-4); LLM evaluation by looking at state ✓ (snapshots are deterministic JSON; Task 4 step 3 proves an agent can follow them; Task 6 makes the engine's own verifiers those agents).
- The old spec's hermetic-editor idea ("feed an event sequence, assert on state") is quoted architecture: this plan is that idea, app-wide, with the file format and runner it needed to be evaluable from outside the process.
- Type consistency: `AppEvent`/`AppState`/`StateSnapshot`/`Scene`/`SceneRunner`/`StateProjector` names are used identically across tasks; `scene_run` is the binary, `demo-scenes`/`scene` the make targets.
- Risk noted: Task 5 is the only task touching a settled, load-bearing asset (synth_ui); it is sequenced last-but-one deliberately, after scenes exist to catch regressions, and its gate includes every existing smoke.
