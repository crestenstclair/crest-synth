## MODIFIED Requirements

### Requirement: Immutable five-region application shell
The production application SHALL render one immutable shell projection containing a context/status line, identity header, main workspace, persistent side region, and footer, and that projection SHALL embed the canonical `SemanticGraphicalViewModel`. `StateProjector` SHALL derive the semantic model, shell, retained diagnostic, StateTree, and parameter snapshot from the same accepted state snapshot; shell context, surface, status, footer actions, generation, and state hash SHALL agree exactly with the embedded semantic model; and exact discovered schemas SHALL include every semantic and shell leaf. The window SHALL consume only this immutable projection, emit `SemanticAction` values and post-paint frame observations through injected ports, and SHALL NOT own or mutate context, surface, focus, mode, return path, valid actions, Patch values, graph state, audio state, or lifecycle state.

#### Scenario: PATCH shell is projected
- **WHEN** PATCH is the accepted top-level context
- **THEN** the embedded semantic model identifies PatchMain or PatchUtility as active, the shell maps PATCH to the main workspace and Utility to the persistent side region, and every region agrees on one generation and state hash

#### Scenario: MIXER shell is projected
- **WHEN** MIXER is the accepted top-level context
- **THEN** the embedded semantic model identifies MixerMain or MixerInspector as active, the shell maps MIXER to the main workspace and Inspector to the persistent side region, and no third top-level context or adapter-owned tab exists

#### Scenario: Projection fields disagree
- **WHEN** a shell region, status, footer hint, retained diagnostic, or embedded semantic field carries another context, surface, generation, state hash, or action availability
- **THEN** exact projection verification fails rather than accepting, relabeling, or repairing the inconsistent frame

#### Scenario: Graphical shell schema differs
- **WHEN** a required semantic/shell leaf is missing, unexpected, stale, or inconsistent with the accepted context or generation
- **THEN** exact schema and projection verification fail before an acceptance marker can pass

#### Scenario: Semantic input is rejected
- **WHEN** the production update callback dispatches an action rejected by a state invariant
- **THEN** the current semantic model, shell, retained diagnostic, and audio publications remain unchanged, the window stays usable, and the next valid semantic action can still be emitted

#### Scenario: Responsive shell placement changes
- **WHEN** the same semantic model is rendered at the desktop and Steam Deck reference viewports
- **THEN** named rectangles may differ but the renderer paints the same context, active surface, focus, mode, return, status, errors, values, and valid-action hints without mutating canonical interaction state
