# Crest Synth Product Roadmap

Status: **Active**

This roadmap sequences the next product increments after the completed foundation program. The historical foundation roadmap is preserved at [`archive/ROADMAP-foundation-2026-07-27.md`](archive/ROADMAP-foundation-2026-07-27.md).

`DESIGN.md` remains the product and technical authority, and the linked Figma file remains the visual and interaction reference. This document orders delivery; it does not replace either source.

## Live-demo requirement for every phase

Every numbered phase must add a separately named, retained live demo scene. Demo scenes are a phase-completion gate because they are the primary human-verification path for confirming that the integrated product actually works.

- Run the optimized standalone application with the real production window and physical audio output; a headless, silent, mocked, or dry-run substitute does not satisfy this gate.
- Play a real MIDI fixture through the production MIDI event source and normal routing/render path throughout the scene, following the established live-demo model. Direct state mutation, fabricated audio, and demo-only reducers or renderers are forbidden.
- Exercise the phase's new behavior through semantic actions, `AppState::apply`, immutable view/audio projections, and the same adapters used by ordinary execution.
- Pace the scene so the user can see the focused control, action, resulting state, and hear the corresponding musical consequence where the behavior affects sound.
- Emit structured phase-specific checkpoints that correlate semantic input, accepted generation, visible projection, graph or parameter state, MIDI activity, and measured audio observations.
- Finish with semantic all-notes-off, zero active notes, window close, stream release, worker shutdown, graph collection, and normal parent-process exit. A frozen window, timeout, dropped event, silent fallback, incomplete report, or teardown failure fails the phase.
- Preserve every completed phase scene under a stable phase-specific `make demo-live-<scene>` target so later work cannot replace earlier evidence. `make demo-live` points to the newest cumulative scene.
- A phase is not complete until its actual live target has been run successfully by the implementer and the resulting visible, audible, structured report covers every declared phase behavior.

## Phase 1 — Graphical application shell blockout

Establish the controller-first graphical shell around the existing application without moving mutation or navigation state into the UI.

- Preserve PATCH and MIXER as the only top-level contexts.
- Establish the context line, identity/header band, main workspace, persistent Utility/Inspector region, and footer.
- Keep the existing reducer, semantic input path, immutable projections, audio path, and live-demo lifecycle intact.
- Define the authored desktop composition and Steam Deck layout constraints without completing every product surface.
- Add `make demo-live-graphical-shell`: play the real MIDI fixture while the scene opens the production graphical shell, switches between PATCH and MIXER, verifies every structural band and persistent region, then completes the full live teardown contract.

## Phase 2 — Semantic graphical view model and focus contract

Project canonical application state into host-neutral graphical view models that can drive more than one layout without duplicating domain state.

- Represent context, surface, semantic focus path, interaction mode, return path, valid actions, status, and errors explicitly.
- Project instrument and effect content from capability descriptors rather than concrete engine or processor branches.
- Keep components passive: they receive immutable data and emit semantic actions through the existing reducer path.
- Prove deterministic focus recovery when responsive composition or schema changes alter visible placement.
- Add `make demo-live-semantic-view-model`: play the real MIDI fixture while the scene traverses semantic focus, interaction modes, valid actions, return paths, status, and error projections across both contexts, proving that visible focus and canonical state remain correlated.

## Corrective gate (completed) — Canonical sixteen-track mixer routing

This correction fixed the Patch-shaped transitional MIXER model before Phase 3
built any additional routing topology. It completed with its retained live
scene and remains a permanent regression gate.

- Introduce one canonical mixer bank with exactly sixteen persistent tracks, independent of Patch count and instrument schema.
- Give each Patch one validated output-track identity and Patch-local trim; allow multiple Patches to share a track without losing Patch identity.
- Move level, pan, mute, solo, current reverb/delay sends, and metering to mixer-track ownership. A track fader controls the combined output of every Patch routed to that track.
- Replace Patch-keyed MIXER focus and projection with stable track/control identities while keeping PATCH and MIXER as the only top-level contexts.
- Carry Patch routes, trims, sixteen track parameter sets, and sixteen meters through fixed bounded real-time snapshots and preallocated track accumulation.
- Add `make demo-live-sixteen-track-mixer-routing`: play real multi-Patch MIDI, display all sixteen tracks, route two Patches to one track, exercise track level/pan/mute/solo/sends and Patch rerouting through semantic actions, prove measured isolation and meter behavior, then complete the full live teardown contract.

This correction establishes the canonical domain, reducer, projection, and
rendering behavior. Phase 6 still owns assembly from the shared component
library, responsive density refinement, multi-select where specified, and final
functional Mixer composition.

## Phase 3 (delivered 2026-07-31; corrective gate closed 2026-08-01) — Expandable effects and bus topology

Grow the fixed first-effect foundation into the bounded effect and routing model required by the product interface.
Delivered by mission `expandable-effects-and-bus-topology-01KYNGX8` (all behaviors proven; post-merge review
PASS WITH NOTES). Its one HIGH finding — the retained scene drove slot and return occupancy by injected semantic
actions instead of the player's on-screen journey — was healed by mission
`demo-journey-fidelity-and-hygiene-01KYWVYG` and demonstrated on hardware; see the closed corrective gate below.
That gate also recorded LIMIT-1, a demo-scope bound carried forward as a Phase 5 entry condition.

- Support up to three descriptor-driven ordered effect slots per Patch without processor-specific UI structure.
- Introduce explicit buses, sends, returns, and routing identities, with at most eight bus returns in the prepared topology, while preserving Patch ownership and hard real-time preparation boundaries.
- Prepare and exchange complete structural graphs off the callback; never allocate, block, destroy, or silently bypass in real-time work.
- Keep topology changes semantic, validated, observable, and recoverable without replacing the active graph on failure.
- Add `make demo-live-effects-and-buses`: play real multi-Patch MIDI while the scene changes ordered Patch effects, sends, buses, and returns through production structural/scalar paths, audibly proves routing and isolation, exercises one controlled rejection, and verifies graph retirement and teardown.

## Corrective gate (CLOSED 2026-08-01) — Phase 3 demo journey fidelity and mission hygiene

The Phase 3 retained scene proved every declared behavior audibly but performed
slot and return occupancy changes by injecting semantic actions directly,
bypassing the player journey the phase exists to demonstrate. This gate healed
that gap and swept the post-merge review's open items before Phase 4 begins.
It was the gate for all later phases; it is now **closed** and remains here as
history and as a permanent regression gate. **Phase 4 is unblocked.**

- Rework `demo-live-effects-and-buses` so slot occupancy changes travel the player's PATCH journey on screen: focus moves to each effect-slot row, occupancy cycles by the adjacent-choice gesture, and at least one occupant parameter is edited audibly from the PATCH page; return occupancy likewise travels the MIXER return rows. The controlled rejection may keep direct injection — the UI cannot request an unknown entry by design — with that exception documented in the scene.
- Keep every existing checkpoint identity byte-identical (add-only), then re-run the scene on a physical device and refresh the recorded evidence; amend the mission's acceptance matrix and post-merge review addendum accordingly.
- Sweep the review's open items: retire the transitional `post_effects()` compact view (migrate callers to `effect_slots()`; the composition-root round-trip must stop re-compacting gapped chains); propagate default-return composition errors at the production root instead of `unwrap_or_default`; add the RETURN-clear held-note continuity twin test; make the live-report measurement fields distinguish absent evidence from zero; clean stale WP-numbered handoff comments, the `DESIGN.md` "aux buses" wording, and leftover `reverbSend` test-fixture literals; gate the name-enumeration guard script on its tool dependencies.
- Optional hardening if cheap while there: an end-to-end register-a-fourth-effect fixture converting SC-008's structural inference into a demonstration, and per-position engine-capability identity in the prepared-graph layout attestation.

### Gate status (2026-07-31) — code complete, OPEN pending hardware evidence

*Historical snapshot, retained verbatim. Superseded by "Gate CLOSED (2026-08-01)"
below; every statement here was true on 2026-07-31.*

Mission `demo-journey-fidelity-and-hygiene-01KYWVYG` has landed every code work
package (WP01–WP10 approved and merged). The gate is **not** closed: it closes
only on a physical `make demo-live-effects-and-buses` run, which has not been
performed. Nothing below claims a demonstration that has not happened.

Landed and verified deterministically on the merged tree:

- Journey rework — the scene declares 30 topology transitions, up from the 17 it
  froze pre-rework; every occupancy change now dispatches the adjacent-choice
  gesture behind a focus-verified journey to the exact row. The single remaining
  direct injection is the documented rejection (`Topology.refused`), asserted as
  the only one (`tests/effects_and_buses.rs`).
- Add-only identity contract — the 17 pre-journey identities are frozen in
  `FROZEN_TOPOLOGY_IDENTITY_BASELINE` (`tests/effects_and_buses.rs:59`) and
  asserted byte-identical and in order on both the declared and emitted
  surfaces; the 13 journey identities are pure insertions.
- Open items 1–7 — all seven dispositioned in code (compact view retired;
  default-return composition errors propagated at the production root; the
  RETURN-clear held-note twin test added; live-report measurements distinguish
  absent from zero; comment/wording/fixture residue cleared; guard script gated
  on its tools; the fourth-entry fixture and per-position capability identity
  both delivered).
- Declared proof — `cargo test --release --test expandable_effects_and_bus_topology`
  emits `schemaVersion: 2` with `fourthEntryEndToEndExercised: true`,
  `carryOverWrongEngineIdentityRefused: true`, `twoRunTraceEqual: true`; full
  suite 533 passed / 0 failed across 26 targets; clippy and fmt clean.

### Gate CLOSED (2026-08-01) — hardware evidence recorded

The four items above are discharged. `make demo-live-effects-and-buses` was run
on the physical rig (real window, physical audio device, real MIDI fixture) on
the fully merged lane. **Process exit 0.** The complete log is **committed**, not
merely cited, at
[`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/evidence/wp11-t044-live-run.log`](kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/evidence/wp11-t044-live-run.log)
(evidence commit `5238020`).

- Completeness — 144/144 checkpoints; 17,462 events with `droppedRecords=0` and
  `lossless=true`; 105/105 editable parameters; 3/3 engine transitions; 7,718
  qualifying shell frames; `graphRevision` 1 → 30; banks=1, instruments=15.
  Zero checkpoints with `audio_uninterrupted=false`. Clean teardown
  (`cleanup=true`, `activeNotes=0`, `window_closed=true`,
  `stream_released=true`, `owned_graphs_remaining=0`). The observation carries
  45 keys and **zero false values**.
- Measurements are MEASURED, not defaulted — `frames_to_projection_max=1`,
  `activation_sequence_gap_max=3`, `render_blocks_to_audible_max=46`. A
  defaulted zero would have been a WP06 regression; none is present.
- Exactly **three** product effects appeared on screen (`effect.chorus`,
  `effect.delay`, `effect.reverb`); the test-only fourth registry entry
  `witness-tilt` occurs 0 times in the log.
- Identity comparison (T045) against `FROZEN_TOPOLOGY_IDENTITY_BASELINE`
  (`tests/effects_and_buses.rs:59`) — **17/17 baseline identities preserved
  byte-identically and in order; 0 modified; 0 removed; 13 added; 30 total.**
  The add-only contract holds on hardware, not only in the deterministic twin.
- Parent record amended append-only: `acceptance-matrix.json` (8 added rows,
  pure insertions, all 35 original rows byte-identical) and `mission-review.md`
  (Addendum 2, 2026-08-01). The DRIFT-6 "superseded: inadequate for the player
  journey" note is resolved and the FR-019/C-010 grading is **restored to
  adequate**. All seven post-merge open items are **CLOSED**, none deferred;
  SC-008 is regraded from PARTIAL to a demonstration.

**One new finding — LIMIT-1 (DEMO-SCOPE BOUND, MEDIUM).** The retained scene
demonstrates the effect-slot and bus-return journey for **one instrument only**:
its subject is `patches.first()`
(`src/testing/live_effects_and_buses_scene.rs:284`), while all sixteen mixer
tracks and fifteen installed instruments are exercised for sends and routing.
The root cause is that **there is no patch-switching gesture in the semantic
vocabulary** — `SemanticAction` is a closed union of eight kinds;
`SelectContext` switches only PATCH↔MIXER, and although `FocusPath` and
`SetSlotOccupancy` both carry a `patch_id`, no `SelectPatch`/`NextPatch` action
exists to change which Patch is focused. The bound is a consequence of this
gate's own fix: requiring the on-screen journey structurally pins the
demonstration to the first Patch, where backstage injection never had to. Making
the demo honest is what exposed the missing patch selector. Accepted for Phase 3,
which closes on its declared behaviors, and **escalated to a Phase 5 entry
condition** below. Full finding: `mission-review.md` Addendum 2.

## Phase 4 — Component library blockout

Create the reusable component vocabulary before building the functional Patch and Mixer screens. This is an application component system, not a separate product and not a React runtime.

- Centralize semantic color, typography, spacing, geometry, focus, adjustment, disabled, loading, error, mute, solo, and selection tokens.
- Define responsive density and sizing policies for authored desktop and Steam Deck viewports; pages must compose from policies instead of scattering resolution-specific constants.
- Provide reusable primitives such as text roles, hairlines, keylines, focus frames, value displays, status marks, and action hints.
- Provide configurable controls such as parameter rows, choice rows, toggles, compact sliders, faders, meters, browser rows, and modal options.
- Provide reusable compositions such as the application shell, context switch, identity header, section, Patch strip row, Utility/Inspector panel, and footer.
- Accept immutable props/view data and return typed semantic UI intent; components do not own Patch values, focus, navigation, reducer state, or audio state.
- Permit carefully selected third-party egui utilities underneath the Crest layer, while Crest owns the stable component API, behavior, tokens, and visual contract.
- Add a component gallery that renders every meaningful behavioral state and representative content at desktop and Steam Deck sizes.
- Add `make demo-live-component-library`: ~~play the real MIDI fixture while the scene renders~~ **AMENDED 2026-08-03 — see below.** The scene renders the production shell through the shared components, traverses every currently applicable focus/edit/disabled/loading/error/status variant, ~~exercises representative controls through semantic actions~~ is browsed by hand, and visibly demonstrates both desktop and Steam Deck composition policies.

Phase 4 completes when later pages can be assembled from the shared vocabulary without copying paint, layout, focus, or state-visualization logic.

### Amendment (2026-08-03) — what the component-library demo delivered, and what was scoped out

The bullet above is amended in place rather than rewritten, because a gate is closed by recorded evidence and never by quietly removing the requirement it stated.

**Delivered.** `make demo-live-component-library` opens a browsable gallery window: fifteen pages covering the vocabulary, the eight controls, the eight compositions, and the nine component states at both authored viewport densities, painted through the production render path. Digits `1`–`9` and `0` select the first ten pages, `[` and `]` step through all fifteen, and closing the window finishes. The operator drives it; nothing drives the operator.

**Scoped out, by operator directive (C-001, 2026-08-02).** The MIDI fixture, the audio device, and the traversal-by-semantic-action. The demo constructs no audio output and no MIDI event source, so it plays nothing — a consequence of what was built, not a property proved about it. The requirement is not withdrawn: a scene that plays the fixture while exercising controls through semantic actions is worth having, and it lands when a later phase has a reason to build it.

**Why hand-browsing rather than an automated traversal.** Phase 4's deliverable is a component library a person can look at. A scripted traversal proves a script ran; a window a reviewer opens proves the components compose. The declared deterministic proofs — `component_vocabulary` and `component_composition` — carry the machine-checkable half, and both drive the production render path.

**This diverges from "Live-demo requirement for every phase" above, and the divergence is left visible.** That section requires every phase's retained scene to play a real MIDI fixture through the production audio path. Phase 4's does not, by the operator's directive. Amending a program-level rule from inside one phase's entry would be the wrong place to settle it, so the contradiction is recorded here rather than resolved by editing the rule: whoever next reads the gate should decide whether it means *every* phase or every phase that touches sound.

### Phase 4 completion note (2026-08-03)

**What closes it.** The token, type, spacing, geometry, density, and state vocabularies; the eight controls and eight compositions; the render adapter reduced to window plumbing and event translation, with every shipped region produced by a composition; the browsable gallery; and two declared project validations that measure all of it through the production render path rather than through a parallel one.

**What carries forward.**

- **Ten designed structures the projection does not drive** — master volume on the PATCH Utility surface, MIDI input, voice limit, per-row action hints, and the rest. Each is marked unavailable or omitted today rather than filled with a plausible value; Phase 5 supplies the data behind them. This list is Phase 5's declared input.
- **Two control kinds no production capability projects.** Nothing declares a stepped parameter, and the read-only surface summary is reachable only on a projection path no application state produces. Both select real controls; neither has a real value to select for yet.
- **The mixer meter is not drivable in this slice.** The meter control exists and is proven, but no path carries an audio observation to a composition, so the shipped mixer shows one reading painted by the render adapter rather than sixteen painted by the bank.
- **The persistent side region lost its scroll.** The Inspector's lower rows — the bus returns' levels and master gain — paint past the bottom of the panel with no gesture that brings them back. Reachable before the recomposition, not reachable now, and the remedy is a decision about that surface rather than a reflex to restore a scroll area.
- **Display fidelity cleanup.** Compact mixer cell labels are trimmed at their column edge, the disabled `Locked` word overruns its row inset, and two derived band heights sit a few pixels off their authored extents. Recorded, bounded by the deterministic proof so none can grow, and not a gate at this stage.
- **Three controls have no authored specimen.** The design file defines no toggle, no choice-row adjacency affordance, and no meter. All three shipped as flagged minimums; whether the design file is extended or the minimums stand is a product call.

## Webview shell cutover gate (added 2026-08-05) — retire the egui visual layer before Phase 5

**Operator decision (2026-08-05): all current shell work moves to the Tauri
webview shell before any further product implementation.** Mission
`webview-shell-foundation-01KZ9DN7` (merged via PR #2, effective verdict PASS
WITH NOTES) proved the approach: the production MIXER renders from the same
`SemanticGraphicalViewModel` through 244 lines of HTML/CSS where the
hand-painted egui layer needed a 684-line density policy. This gate **blocks
Phase 5**; Phases 5–9 assemble their surfaces in the webview shell.

- Author the retirement in the crest-spec FIRST: replace
  `requirement.selected_egui_stack` with a webview-default declaration, retire
  the egui adapter and any validations that drive the egui render path
  (Phase 4's `component_vocabulary` / `component_composition` proofs must be
  re-declared against the webview path or deliberately retired, not silently
  dropped), and record the pivot in `DESIGN.md`. This is a deliberate
  declaration, never a side effect of deletion.
- Render the PATCH context and every shipped shell surface through the webview
  shell from the canonical view model. Keyboard input stays Rust-side; the
  one-way loop, immutable projections, and owned-shutdown path are unchanged.
- Migrate the retained live scenes before deleting the egui window: every
  `make demo-live-<scene>` target must run through the webview shell with
  refreshed hardware evidence. Retained evidence survives the cutover or the
  cutover does not complete. The component-library gallery either re-renders
  through the webview vocabulary or its retirement is recorded here — a
  decision for specify, not a silent casualty.
- Delete the egui visual layer (`src/shell/visual/`, ~17k lines), the eframe
  window adapter, and the `eframe`/`egui_extras` dependencies. The Phase 4
  token/type/spacing/state vocabulary carries forward as the authored CSS
  vocabulary under FR-002's single-source contract; the component *contracts*
  survive the renderer they were first painted with.
- Discharge the foundation review's successor items: painted-ack →
  `ShellFrameObservation` forwarding with an explicit owner, the same-workload
  RT A/B measurement, the 300 s soak, a restrictive CSP,
  `cfg(debug_assertions)` on `CREST_WEBVIEW_PAGE`, and the automated
  key-injection witness.

The gate closes on a hardware run of the cumulative `make demo-live` target
through the webview shell — real window, physical audio, real MIDI fixture,
full teardown contract — with the egui visual layer no longer in the tree.

**Gate closed (2026-08-06, mission `webview-shell-cutover-01KZAC7Q`).**
Closure evidence, in the order the C-007 constraint demands:

- *Retained evidence first.* The WP06 hardware evidence wall —
  `kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/README.md` with the
  four retained scene logs, the same-workload RT A/B comparison against the
  egui baseline, and the 300 s soak — was committed on
  `feat/webview-shell-cutover` at `69fa5eb` (logs) and `b57bf9d` (README),
  strictly before any deletion.
- *Deletion last.* WP07's deletion commit (`ceef87b` on lane
  `kitty/mission-webview-shell-cutover-01KZAC7Q-lane-g`, "delete the egui
  layer") removed `src/shell/visual/`, the eframe window adapter,
  `tests/eframe_context.rs`, the `webview_input_probe` binary, and the
  `eframe`/`egui_extras` dependencies in one deletion-only change:
  39 files, +212/−18,353 lines. The authored vocabulary survived it,
  relocated to `src/shell/tokens.rs`, `typeface.rs`, `density.rs`,
  `component_state.rs`, and `component_vocabulary.rs`.
- *Probe decision.* The WP01 foundation probe binary
  (`webview_input_probe`, 484 lines) is deleted with its `[[bin]]` entry:
  its evidence is the committed probe verdict
  (`kitty-specs/webview-shell-foundation-01KZ9DN7/research/input-capture-probe.md`),
  and the automated key-injection witness (`tests/input_capture_witness.rs`,
  WP05) carries the living input-capture contract.
- *SC-003, zero references.* `grep -riE '\b(egui|eframe)\b' src/ tests/
  webview-page/ Cargo.toml Cargo.lock` returns zero matches. (Two
  pre-existing identifiers — `ExhaustiveGuiDemo` and the `WholeFrame`
  shell region — contain incidental case-insensitive substrings and do not
  reference the retired stack.)
- *SC-004, net shell-code reduction.* Measured on code paths only —
  committed evidence logs and planning documents under `kitty-specs/` are
  deliberately excluded: `git diff --shortstat d41e7bd..HEAD -- src tests
  webview-page Cargo.toml Cargo.lock` (from the pre-mission planning tip
  `d41e7bd` of `feat/webview-shell-cutover`) reports +13,368/−27,907 =
  **net −14,539 lines** (−13,506 restricted to `src/` and `tests/`),
  clearing the ≥10,000-line bar.
- *Post-deletion hardware close.* `make demo-live` (cumulative
  effects-and-buses scene) after the deletion: exit 0, report complete
  (105/105 parameters, 3/3 engine transitions, 2,158 qualifying webview
  frames, 15,214 events, 0 dropped, `callbackAllocations=0`, clean
  teardown), real window and physical audio on the same rig as the WP06
  wall. The forced init failure (`CREST_WEBVIEW_PAGE` to an unloadable
  path) exits 1 with the typed `PageLoadFailed` error and no alternate
  window.

## Phase 5 — Functional Patch editor blockout

**Blocked by the webview shell cutover gate above (2026-08-05).** All Phase 5
surfaces are assembled in the webview shell; references to the egui window in
the entry condition below are the historical record of how LIMIT-1 was found
and remain accurate as history.

### Entry condition — close LIMIT-1: the journey must reach more than one instrument

Phase 3's live gate proved the effect-slot and bus-return journey **for a single
Patch only**. The retained scene's subject is `patches.first()`
(`src/testing/live_effects_and_buses_scene.rs:284`), and it is pinned there
because **the semantic vocabulary has no patch-switching gesture**:
`SemanticAction` (`src/control/semantic_action.rs:54`) is a closed union of
eight kinds, `SelectContext` switches only PATCH↔MIXER, and while `FocusPath`
(`src/control/semantic_focus.rs:205`) and `SetSlotOccupancy` both carry a
`patch_id`, nothing lets the player change which Patch is focused. Phase 3 could
only make the journey honest, not make it reach further; requiring the on-screen
path is precisely what exposed the missing selector.

Phase 5 does not close until both of these hold:

- ~~**A patch-selection gesture exists in the semantic vocabulary**~~ —
  **CLOSED 2026-08-02.** `SemanticAction::SelectPatch(Direction)` moves the
  focused Patch one position along the installed order, reduced by
  `AppState::select_patch`. It travels the production physical input → semantic
  action → `AppState::apply` → projection path (Q/E through the egui window and
  the keyboard translator), refuses rather than wraps at either end, is
  available only in the Patch context in Navigate mode, and recovers focus
  against the destination Patch's own descriptor schema. Declared first in the
  crest-spec (`SemanticAction`, `InteractionState` invariants). Falsified: the
  reducer test fails when the move is defeated.

  This mattered more than "a demo bound" made it sound. The fixture installs
  one Patch per MIDI part — the last hardware run loaded **15** (8 SoundFont,
  7 Braids), all rendering — and the controller could reach exactly one. The
  gesture is what makes the other fourteen playable.
- **`make demo-live-patch-editor` demonstrates the effect-slot journey on more
  than one instrument** — the scene navigates from one Patch to another *through
  that gesture on screen*, then performs a full focus-verified effect-slot
  occupancy journey and an audible occupant parameter edit on the second
  instrument, not only the first. Checkpoints must correlate the patch switch,
  the resulting focus, and the audible consequence.

  **STILL OPEN (2026-08-09) — BUILT AND UNRUN.** The target, the scene, the
  observation, and the controlled negative all exist and are green under
  `cargo test`. The live run has never completed, so this bullet is not struck.
  A bullet struck against a run that did not happen would be a claim, not a
  record. See the Phase 5 status note below for exactly what is measured and
  what is not.

Meeting both closes LIMIT-1 (recorded in
`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/mission-review.md`,
Addendum 2). Until then, no scene may claim the effects journey is demonstrated
across the instrument roster.

### Phase 5 status note (2026-08-09) — the exit gate is built and unrun

This is not a completion note. Phase 5 does not close here, and the note says so
rather than reporting an adjective where a number belongs.

**What is built.** `make demo-live-patch-editor` exists and resolves
(`Makefile`, `--demo-live-patch-editor` in `src/bin/crest_synth.rs`). `demo-live`
is unchanged and still points at `demo-live-effects-and-buses`; this scene is
additive. The scene
(`src/testing/live_patch_editor_scene.rs`) reaches the **second** installed Patch
by dispatching `SemanticAction::SelectPatch` through `AppLoop` and the production
reducer, then derives every effect-slot position, the audible occupant edit, the
voice-limit ceiling walk, and the end-of-order boundary from *that Patch's own*
descriptors and published state. The observation
(`src/testing/functional_patch_editor_observation.rs`) carries all 42 fields the
witness declares, and each second-Patch counter is keyed by the PatchId resolved
from the **final state's installed order**, never from the scene's own subject.
The declared controlled negative `--defeat-patch-selection` removes the gesture
and leaves the journey on the first instrument.

**What is measured, deterministically, without a window.**

- 15 installed Patches (8 SoundFont, 7 Braids) — the fixture the reach claim is
  made against; the controller could formerly reach 1 of them.
- The switch script, replayed through the production `AppLoop`, leaves the
  canonical projection speaking for installed Patch **2**, distinct from Patch 1.
- All **3** declared slot positions are targeted on Patch 2 by id, each with a
  verified landing on its own occupancy row, and all 3 are restored to the exact
  occupancy the run found.
- Under `--defeat-patch-selection` the identical journey runs, targets **3**
  slots and makes an audible edit — all of it on Patch **1** — and the reach
  counters therefore read `patchesFocused = 1`, `secondPatchIdDistinct = false`,
  `secondPatchSlotsVisited = 0`. That is the keying rule doing the work it
  exists for. **Two halves, proven separately.** That the defeated plan stays on
  Patch 1 is proven by the plan-level test; that such a measurement credits the
  second Patch nothing is proven by the unit test, which feeds the resolver a
  synthetic three-slot-and-an-edit measurement recorded against Patch 1 and gets
  `secondPatchAudibleEditDelta = 0.0` back. **That `0.0` is a synthetic
  measurement, not a live number** — it belongs to the unit test, not to a run,
  and the live audible deltas remain unmeasured below. The plan→runner→resolve
  join is exercised only live, so the negative's falsifying power today is "the
  keying rule provably discriminates on a measurement of the shape the runner
  produces", not "the negative was observed to fail on reach".
- The end-of-order refusal is a genuine `parameterAtBoundary` in **both** modes,
  so the negative fails on reach rather than on its own scaffold.
- The voice-limit walk lands exactly on the declared minimum (**1**), one further
  step is a boundary refusal, and the asymmetric return walk restores the exact
  starting ceiling (**16** on the Braids subject) rather than overshooting it.
- Detail entry from an empty occupancy row on Patch 2 is refused
  `actionUnavailableInContext`.
- The page's strip-paint evidence survives the ack round trip verbatim, and a
  half-formed `strip` object is a typed malformed ack rather than a zero.
- The emitted observation's keys are **exactly** the witness's declared 42, in
  both directions — every declared field present, and nothing beyond
  `schema_version` — pinned in `the_emitted_schema_matches_the_declared_witness_fields`
  and falsified both ways (a camelCase `serde(rename)` and a stale 41-entry
  array each fail it).
- F-49's scene-name gate — which decides whether a scene's topology checkpoints
  are graded against the effects-and-buses bus contract — is now pinned in both
  directions, both mutations run rather than argued. Narrowing it to a
  never-matching literal was **already** caught, by `tests/effects_and_buses.rs`
  rather than by any suite a general sweep would reach. **Widening** it to
  always-true was caught by nothing: the `effects_and_buses`,
  `live_demo_scene`, `topology_change_lifecycle` and `live_patch_editor_scene`
  suites all stayed green while an always-true gate graded this scene's
  effect-slot occupancy walk against a contract it never claimed to meet. That
  is the direction that was genuinely open, and it is now closed.
- NFR-004: full `project_with_shell` per accepted event, release, median of 15,
  89 MIXER rows — **2382 µs** against the 3.00 ms bar (down from 2960 µs).

**What is not measured, and why.** Every predicate that needs a painted frame
remains unexecuted: `stripGroupsPainted`, `stripFlatControlRun`,
`qualifyingWebviewFrames`, `desktopViewportPainted`, `physicalAudioNonzero`, both
audible-edit deltas and the `audibleEditIsolatedToSecondPatch` verdict drawn from
them, `checkpointsCorrelatingSwitchFocusAudio`,
`voiceLimitRefusals`, `projectionGenerationGaps`, and the teardown quartet. The
run was attempted on 2026-08-09 and failed after 10 s with
`no progress ... awaiting parameter projection paint confirmation at step 3`:
no paint acknowledgment ever arrives. **The already-shipped
`demo-live-effects-and-buses` scene fails identically — same step, same
predicate, same timeout** — which is what establishes the blocker as the
environment rather than this scene. Neither run reaches its own phase.

**One declared threshold is unvalidated.** `AUDIBLE_EDIT_DELTA_MARGIN`
(1.0e-3 RMS) is the margin by which the second Patch's measured delta must exceed
the first Patch's. It is a declaration, not a measurement; the first completed
live run is what confirms or moves it.

**The witness now asserts the bounded verdict directly, and the observation
carries it.** F-47 ruled the exact-zero first-Patch delta unattainable on a live
decaying voice and replaced it with the bounded comparison; that ruling has since
landed in the declaration. `witness.functional_patch_editor` declares a 42nd
field, `audible_edit_isolated_to_second_patch`, and predicates *that* — while
still reporting both raw deltas beside it, because a verdict without its inputs
cannot be argued with. `first_patch_audible_edit_delta` remains in the schema as
a reported number and no longer carries a predicate of its own. The observation
carries the field, computes it in `resolve()` as
`secondPatchAudibleEditDelta - firstPatchAudibleEditDelta >=
AUDIBLE_EDIT_DELTA_MARGIN`, and names it as the shortfall when it fails, so the
controlled negative's recorded failing set names a predicate that exists. A run
that made no edit on the second Patch reports both deltas at zero, and zero does
not clear the margin — absent evidence reads as "not isolated" rather than as
isolation by default. Recorded as mission finding F-57.

**One witness field's name is broader than what it measures.**
`midiInputRechannelled` is implemented as "the projected MIDI-input row took more
than one distinct value across the run" — that is, *the row is Patch-local and
re-projects across a switch*, so a defeated run that never leaves the first
instrument projects one value and fails it. It is **not** a completed
re-channelling edit: the fixture packs 15 Patches onto channels 0-14, so every
adjacent channel is a `DuplicateMidiChannel` refusal and no such edit is
measurable on this roster. FR-008 editability is proven in WP05's target. The
field is graded for what it measures and the name is left alone rather than
renamed mid-mission; the gap is recorded here and carries into acceptance.

Assemble the Patch experience from the component library and semantic view models.

- Implement Patch strip, Patch identity and routing, instrument selection, ordered effects, visible ADSR, and persistent Utility behavior.
- Reuse one polymorphic detail shell for instruments and effects.
- Render capability-provided sections, controls, bounds, units, choices, dependencies, status, and errors.
- Keep every edit on the physical input → semantic action/event → reducer → projection path.
- Favor functional completeness, hierarchy, focus, and responsive composition over final pixel polish.
- Add `make demo-live-patch-editor`: play real MIDI while the scene navigates the Patch strip and polymorphic detail shell, changes the engine and effect topology where supported, edits ADSR and scalar parameters, verifies Utility behavior and exact return focus, and makes every audible edit observable. Per the entry condition above, the scene must switch the focused Patch through the new semantic gesture and demonstrate the effect-slot journey on more than one instrument.

## Phase 6 — Functional Mixer blockout

Assemble the sixteen-track Mixer from the same component library and interaction model.

- Keep all sixteen tracks addressable with stable semantic track/control focus.
- Implement level, pan, mute, solo, routing summary, sends, meters, and the persistent Inspector.
- Preserve row and track position across navigation and responsive density changes.
- Keep status and control states explicit in text or shape as well as color.
- Add `make demo-live-mixer`: play real MIDI across multiple Patches while the scene traverses all sixteen tracks, exercises level, pan, mute, solo, sends, meters, multi-select where implemented, and Inspector correlation, and audibly proves target isolation and routing.

## Phase 7 — Detail, choice, and asset workflows

Complete the subordinate PATCH surfaces using the same shell and component vocabulary.

- Instrument detail and effect detail.
- Engine, effect, route, and other choice modals with trapped focus and exact return paths.
- Sample detail, waveform landmarks, and the controller-native Sample Browser, where holding Start on the focused sample row previews that sample and releasing Start stops preview.
- Typed loading, unavailable, validation, and cancellation states without UI-owned domain copies.
- Add `make demo-live-detail-and-assets`: play real MIDI while the scene opens instrument/effect details and choice surfaces, navigates the Sample Browser, previews and commits a valid asset through production preparation, exercises cancel/error paths, and returns focus to each exact origin.

The detailed Sample capability contract—including admitted formats, playback and loop semantics, polyphony, root-pitch behavior, and preparation limits—is intentionally deferred until Phase 7 planning. It is not an unresolved prerequisite for Phases 1–6 and must not be inferred from Figma fixtures.

## Phase 8 — Controller and resolution hardening

Exercise the complete interface across supported input devices, viewport classes, content extremes, and lifecycle states.

- Verify keyboard and controller parity after input normalization.
- Verify desktop and Steam Deck layouts, minimum targets, hierarchy, persistent context, and bounded density.
- Test long labels, large registries, disabled dependencies, loading/failure states, rapid navigation, and schema changes.
- Extend deterministic and physical live evidence to the graphical projections without weakening audio or teardown proofs.
- Add `make demo-live-controller-resolution`: play real MIDI while the scene repeats a representative end-to-end workflow at desktop and Steam Deck viewports, verifies normalized keyboard/controller parity, stresses content and navigation boundaries, and proves focus, audio, and teardown remain coherent.

## Phase 9 — Visual completion and product UI cutover

Bring the functional interface to the authored visual standard and retire the diagnostic text view as the normal product surface.

- Reconcile composition, typography, spacing, colors, focus, adjustment, status, and responsive behavior with the Figma reference.
- Remove accidental one-off styling and layout now covered by the shared component system.
- Complete visual, behavioral, accessibility, performance, startup, shutdown, and physical-device acceptance.
- Retain diagnostic projections only where they remain useful as explicit verification or support tools.
- Add `make demo-live-product-ui`: run the final cumulative real-MIDI performance scene through PATCH, details, choices, assets, MIXER, effects, buses, responsive layouts, and production cleanup; this becomes the `make demo-live` target used for final product acceptance.

## Deferred beyond this roadmap

- modulation sources and modulation routing;
- a modulation matrix;
- arbitrary plugin hosting;
- additional engines and effects introduced without an individually bounded product need;
- broad preset, session, and library management beyond the controller-first workflows above.
