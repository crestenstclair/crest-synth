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

## Phase 3 (delivered 2026-07-31, with one open gate) — Expandable effects and bus topology

Grow the fixed first-effect foundation into the bounded effect and routing model required by the product interface.
Delivered by mission `expandable-effects-and-bus-topology-01KYNGX8` (all behaviors proven; post-merge review
PASS WITH NOTES). One HIGH finding remains and is the current corrective gate below: the retained scene drives
slot and return occupancy by injected semantic actions instead of the player's on-screen journey.

- Support up to three descriptor-driven ordered effect slots per Patch without processor-specific UI structure.
- Introduce explicit buses, sends, returns, and routing identities, with at most eight bus returns in the prepared topology, while preserving Patch ownership and hard real-time preparation boundaries.
- Prepare and exchange complete structural graphs off the callback; never allocate, block, destroy, or silently bypass in real-time work.
- Keep topology changes semantic, validated, observable, and recoverable without replacing the active graph on failure.
- Add `make demo-live-effects-and-buses`: play real multi-Patch MIDI while the scene changes ordered Patch effects, sends, buses, and returns through production structural/scalar paths, audibly proves routing and isolation, exercises one controlled rejection, and verifies graph retirement and teardown.

## Current corrective gate — Phase 3 demo journey fidelity and mission hygiene

The Phase 3 retained scene proves every declared behavior audibly but performs
slot and return occupancy changes by injecting semantic actions directly,
bypassing the player journey the phase exists to demonstrate. This gate heals
that gap and sweeps the post-merge review's open items before Phase 4 begins.
It is the current gate for all later phases.

- Rework `demo-live-effects-and-buses` so slot occupancy changes travel the player's PATCH journey on screen: focus moves to each effect-slot row, occupancy cycles by the adjacent-choice gesture, and at least one occupant parameter is edited audibly from the PATCH page; return occupancy likewise travels the MIXER return rows. The controlled rejection may keep direct injection — the UI cannot request an unknown entry by design — with that exception documented in the scene.
- Keep every existing checkpoint identity byte-identical (add-only), then re-run the scene on a physical device and refresh the recorded evidence; amend the mission's acceptance matrix and post-merge review addendum accordingly.
- Sweep the review's open items: retire the transitional `post_effects()` compact view (migrate callers to `effect_slots()`; the composition-root round-trip must stop re-compacting gapped chains); propagate default-return composition errors at the production root instead of `unwrap_or_default`; add the RETURN-clear held-note continuity twin test; make the live-report measurement fields distinguish absent evidence from zero; clean stale WP-numbered handoff comments, the `DESIGN.md` "aux buses" wording, and leftover `reverbSend` test-fixture literals; gate the name-enumeration guard script on its tool dependencies.
- Optional hardening if cheap while there: an end-to-end register-a-fourth-effect fixture converting SC-008's structural inference into a demonstration, and per-position engine-capability identity in the prepared-graph layout attestation.

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
- Add `make demo-live-component-library`: play the real MIDI fixture while the scene renders the production shell through the shared components, traverses every currently applicable focus/edit/disabled/loading/error/status variant, exercises representative controls through semantic actions, and visibly demonstrates both desktop and Steam Deck composition policies.

Phase 4 completes when later pages can be assembled from the shared vocabulary without copying paint, layout, focus, or state-visualization logic.

## Phase 5 — Functional Patch editor blockout

Assemble the Patch experience from the component library and semantic view models.

- Implement Patch strip, Patch identity and routing, instrument selection, ordered effects, visible ADSR, and persistent Utility behavior.
- Reuse one polymorphic detail shell for instruments and effects.
- Render capability-provided sections, controls, bounds, units, choices, dependencies, status, and errors.
- Keep every edit on the physical input → semantic action/event → reducer → projection path.
- Favor functional completeness, hierarchy, focus, and responsive composition over final pixel polish.
- Add `make demo-live-patch-editor`: play real MIDI while the scene navigates the Patch strip and polymorphic detail shell, changes the engine and effect topology where supported, edits ADSR and scalar parameters, verifies Utility behavior and exact return focus, and makes every audible edit observable.

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
