## ADDED Requirements

### Requirement: Canonical semantic focus contract
Reducer-owned `InteractionState` SHALL contain exactly one active `FocusPath`, remembered PATCH and MIXER main paths, one `InteractionMode`, and at most one `ReturnPath`. A focus path SHALL use the compatible `TopLevelContext`, one of `PatchMain | PatchUtility | MixerMain | MixerInspector`, and stable semantic Patch, capability, and control identities without collection indices, layout coordinates, labels, or widget identities. PATCH and MIXER SHALL remain the only top-level contexts; Navigate and Adjust SHALL be the only reachable Phase 2 modes, while Modal and MultiSelect remain named but unreachable.

#### Scenario: Initial semantic focus is projected
- **WHEN** the normal installed fixture reaches its initial accepted state
- **THEN** MIXER is active in Navigate mode, one stable enabled MixerMain control path is focused, the PATCH and MIXER main paths are remembered separately, and no return or modal path exists

#### Scenario: Context selection restores remembered focus
- **WHEN** the player moves to discriminating stable controls in PATCH and MIXER and switches between the two contexts
- **THEN** each context restores its exact remembered main path through `AppState::apply` without a window-owned index or parameter, graph, routing, or audio change

#### Scenario: PATCH Utility round trip
- **WHEN** PATCH Main has a focused control and the reducer accepts Right or `EnterSurface(PatchUtility)` followed by Return
- **THEN** Utility first receives its `SurfaceRoot` focus with the exact PATCH Main origin recorded, and Return atomically restores that origin, clears the return path, and selects Navigate mode

#### Scenario: MIXER Inspector round trip
- **WHEN** a passive MIXER control emits `EnterSurface(MixerInspector)` and then Return
- **THEN** Inspector receives its `SurfaceRoot` focus and Return restores the exact prior MixerMain path without creating another top-level context

#### Scenario: Reserved mode is requested
- **WHEN** a Phase 2 input or passive view attempts to enter Modal or MultiSelect
- **THEN** no valid semantic action exists for that transition and canonical state remains unchanged

### Requirement: Descriptor-driven graphical projection
`StateProjector` SHALL derive one immutable, host-neutral `SemanticGraphicalViewModel` from each accepted application generation. The model SHALL contain generation and state hash, context, active surface, focus path, interaction mode, optional return path, valid actions, typed lifecycle status, typed errors, and semantic surfaces. PATCH content SHALL derive from `PatchControlId` and installed instrument/effect descriptors; MIXER content SHALL derive from stable Patch/global parameter descriptors; Utility and Inspector SHALL contain only a `SurfaceRoot` plus canonical read-only summaries in Phase 2. The model SHALL contain no eframe/egui type, geometry, callback, mutable state, runtime owner, device, or audio buffer.

#### Scenario: Heterogeneous PATCH content is projected
- **WHEN** installed SoundFont and Braids Patches plus a configured Chorus effect are projected
- **THEN** each PatchMain control has the exact stable path, kind, label, typed value, unit, visibility, focusability, editability, status, and typed error dictated by its canonical config and descriptors, with no engine- or effect-specific projector branch

#### Scenario: Stable MIXER content is projected
- **WHEN** MIXER is projected for Patches with different instrument schemas
- **THEN** each Patch-owned target uses `PatchId` plus canonical `PatchEditableTarget`, each global target uses its typed global identity, and no row or column index becomes semantic identity

#### Scenario: Persistent side summaries are projected
- **WHEN** PATCH or MIXER is active
- **THEN** the model contains its main surface and matching persistent Utility or Inspector surface with stable identities, one side `SurfaceRoot`, and read-only canonical summary data rather than premature functional controls

#### Scenario: Lifecycle status or error is projected
- **WHEN** canonical engine selection is Ready, Preparing, Activating, or Failed
- **THEN** the semantic model projects the matching typed status and correlated typed error data for that generation without adapter-authored lifecycle strings, fallback, or prepared-object ownership

#### Scenario: Healthy state is projected
- **WHEN** no canonical failure applies to the accepted generation
- **THEN** the semantic model represents health with an explicit empty errors collection

### Requirement: Exact valid action contract
Every focused semantic model SHALL expose one ordered, duplicate-free set of `ValidAction` values equal to the `SemanticAction` values the reducer can accept for that exact context, surface, mode, typed bounds, dependency state, and structural lifecycle. Unavailable directions, modes, surfaces, and blocked edits SHALL be absent. Footer hints SHALL derive only from this set, and resolving availability SHALL neither mutate state nor predict worker success.

#### Scenario: Navigation actions are resolved
- **WHEN** a focusable main-surface control is projected in Navigate mode
- **THEN** valid actions contain exactly the context-compatible navigation, context selection, surface-entry, and mode actions currently accepted by the shared resolver

#### Scenario: Bounded adjustment is resolved
- **WHEN** an editable scalar is focused in Adjust mode at an interior value and then at a typed boundary
- **THEN** the applicable fine/coarse direction is present at the interior value and absent at the reached boundary while unrelated actions remain exact

#### Scenario: Structural adjustment is busy
- **WHEN** Engine is focused while a correlated selection is Preparing or Activating
- **THEN** Engine may remain focused but another structural adjustment is absent from valid actions and no availability check changes lifecycle state

#### Scenario: Footer is rendered
- **WHEN** the graphical shell paints an accepted semantic model
- **THEN** every action hint corresponds to one projected valid action and no adapter-only or stale action is displayed

#### Scenario: Projected action is dispatched
- **WHEN** a passive view emits any action from the projected set without an intervening state change
- **THEN** `AppLoop` maps it to the matching event and `AppState::apply` accepts it under the same resolver rules

### Requirement: Deterministic semantic focus recovery
Responsive layout changes SHALL NOT alter any semantic path. After a committed descriptor or dependency change, `AppState` SHALL retain every active, remembered, and return-path target that still resolves, otherwise search the prior canonical surface order outward for the nearest surviving visible focusable sibling with next-before-previous tie breaking. A projector, layout, or widget SHALL NOT repair focus.

#### Scenario: Viewport changes
- **WHEN** the same immutable semantic model is rendered at 1920×1080 and 1280×800 with different rectangles, density, wrapping, or scrolling
- **THEN** focus, remembered paths, mode, return path, valid actions, generation, and state hash remain identical

#### Scenario: Focused descriptor control is removed
- **WHEN** a committed SoundFont-to-Braids replacement removes the focused SoundFont-only control
- **THEN** the reducer selects the nearest surviving visible focusable sibling in prior surface order, choosing the next sibling before the previous sibling on an equal-distance tie

#### Scenario: Stable target survives a schema change
- **WHEN** a descriptor commit changes neighboring controls but the focused stable identity still resolves
- **THEN** the exact focus path is retained rather than reconstructed from its former position

#### Scenario: Remembered or return origin becomes invalid
- **WHEN** a committed schema change removes a target stored in a remembered main path or active return path
- **THEN** the reducer repairs that stored path by the same resolver before context restoration or Return can use it

#### Scenario: Adapter proposes focus repair
- **WHEN** a layout or widget cannot paint the focused target at its preferred position
- **THEN** canonical focus remains unchanged and no adapter-originated repair mutation is accepted

### Requirement: Passive semantic action boundary
Keyboard, controller, MIDI-control, and future graphical component adapters SHALL normalize user intent to the closed `SemanticAction` union and SHALL NOT read or mutate application state to interpret it. `AppLoop` SHALL convert each user action to one typed `AppEvent` before `AppState::apply`; startup, MIDI performance, worker outcome, and system events SHALL retain their direct typed event entry. Views, demos, and adapters SHALL NOT mutate focus, mode, return, session, runtime, graph, or audio state directly.

#### Scenario: Edit key changes mode
- **WHEN** physical Edit is pressed, released, or interrupted by window focus loss
- **THEN** the translator emits `SetInteractionMode(Adjust)`, `SetInteractionMode(Navigate)`, or Navigate respectively, and only the reducer owns the resulting mode transition

#### Scenario: Direction is interpreted by mode
- **WHEN** a direction action reaches the reducer in Navigate and then Adjust mode
- **THEN** it moves semantic focus in Navigate or attempts the focused target's typed adjustment in Adjust without the translator inspecting AppState

#### Scenario: Passive control emits an intent
- **WHEN** a graphical control is activated using its projected action
- **THEN** it emits that immutable `SemanticAction` through the injected sink and retains no writable domain or interaction copy

#### Scenario: System outcome arrives
- **WHEN** a worker result, graph acknowledgement, MIDI performance event, or startup event enters the application
- **THEN** it remains a correlated `AppEvent` and is not mislabeled as a user semantic action

#### Scenario: Action is rejected
- **WHEN** state changes after projection make an emitted action unavailable
- **THEN** the reducer records the typed unchanged rejection, every publication remains unchanged, and a later valid action remains processable

### Requirement: Semantic view-model evidence is non-vacuous and cumulative
Automated acceptance SHALL drive real normalized input and passive-view actions through `AppLoop`, `AppState::apply`, `StateProjector`, and the production egui render path. It SHALL prove exact action/event mapping, focus, modes, returns, valid actions, descriptor polymorphism, lifecycle status/errors, deterministic schema recovery, responsive invariance, generation coherence, and audio neutrality before emitting `CREST_ACCEPTANCE semantic_graphical_view_model passed`. Phase completion SHALL additionally run release-mode `make demo-live-semantic-view-model` with the production window, real MIDI fixture, physical output, cumulative prior live obligations, semantic cleanup, full ownership teardown, and normal parent exit.

#### Scenario: Deterministic semantic acceptance passes
- **WHEN** the named semantic-view-model integration target completes
- **THEN** assertion-bearing evidence covers both contexts, all four surfaces, Navigate and Adjust, both return round trips, SoundFont/Braids/Chorus descriptor differences, typed Failed and recovery, next-before-previous path repair, both reference viewports, exact schemas, and unchanged audio state

#### Scenario: Physical semantic scene completes
- **WHEN** `make demo-live-semantic-view-model` runs on a supported physical system
- **THEN** correlated post-paint, reducer, projection, and audio observations cover two contexts, four surfaces, two reachable modes, two return round trips, exact valid actions, healthy explicitly empty errors, at least one real focus recovery, responsive focus invariance, finite nonzero audio, zero active notes, closed window, released stream, drained worker/graphs, and successful process exit

#### Scenario: Typed failure is required only in deterministic proof
- **WHEN** the healthy physical scene has no genuine structural failure
- **THEN** it proves the explicit empty error set while deterministic production-path acceptance separately proves Failed projection and later recovery without injecting a fake physical failure

#### Scenario: Evidence is inferred or incomplete
- **WHEN** credit comes from planned labels, supplied models, mock-only reducers, helper-only layout, stale generations, silent audio, a success marker before teardown, or an unfinished parent process
- **THEN** Phase 2 acceptance is incomplete and SHALL NOT be reported as passing

#### Scenario: Older live shell command is run
- **WHEN** `make demo-live-graphical-shell` is invoked after Phase 2
- **THEN** its retained Phase 1 behavior remains available while `make demo-live` aliases the newest semantic-view-model scene
