# graphical-application-shell Specification

## Purpose
TBD - created by archiving change phase-1-graphical-application-shell. Update Purpose after archive.
## Requirements
### Requirement: Immutable five-region application shell
The production application SHALL render one immutable shell projection containing a context/status line, identity header, main workspace, persistent side region, and footer. `StateProjector` SHALL derive that shell, the retained diagnostic, the StateTree, and the parameter snapshot from the same accepted state snapshot, and the exact discovered schema SHALL include every graphical-shell leaf. The window SHALL consume only the immutable shell projection, emit semantic input and post-paint frame observations through injected ports, and SHALL NOT own or mutate context, focus, Patch values, graph state, audio state, or lifecycle state.

#### Scenario: PATCH shell is projected
- **WHEN** PATCH is the accepted top-level context
- **THEN** the shell projection identifies PATCH as the main workspace, Utility as the persistent side region, and one coherent generation and state hash for every region

#### Scenario: MIXER shell is projected
- **WHEN** MIXER is the accepted top-level context
- **THEN** the shell projection identifies MIXER as the main workspace, Inspector as the persistent side region, and no third top-level context or adapter-owned tab exists

#### Scenario: Projection fields disagree
- **WHEN** a shell region or retained diagnostic carries another context, generation, or state hash
- **THEN** exact projection verification fails rather than accepting or relabeling the inconsistent frame

#### Scenario: Graphical shell schema differs
- **WHEN** a required graphical-shell leaf is missing, unexpected, stale, or inconsistent with the accepted context or generation
- **THEN** exact schema and projection verification fail before an acceptance marker can pass

#### Scenario: Semantic input is rejected
- **WHEN** the production update callback dispatches an event rejected by a state invariant
- **THEN** the current shell and retained diagnostic remain unchanged, the window stays usable, and the next valid semantic input can still be emitted

### Requirement: Authored shell hierarchy remains visible at reference viewports
At the authored 1920×1080 viewport, the shell SHALL preserve the 48 px context line, 72 px identity header, 896 px workspace, approximately 1500/420 main-to-side workspace split, and 64 px footer. At 1280×800, the same five-region hierarchy SHALL remain visible, ordered, bounded, and non-overlapping; the persistent side region SHALL remain at least 320 px wide, and no required region SHALL be hidden to make the layout fit.

#### Scenario: Desktop composition is rendered
- **WHEN** the production rendering path receives a 1920×1080 viewport
- **THEN** all five named regions occupy the authored bands and split, remain within the viewport, and do not overlap

#### Scenario: Steam Deck composition is rendered
- **WHEN** the same shell projection is rendered at 1280×800
- **THEN** every named region and required label remains observable, Utility or Inspector remains at least 320 px wide, and the layout uses controlled density or local text handling without changing application state

#### Scenario: Required region would be lost
- **WHEN** responsive composition would hide, overlay, reorder, or collapse a required region
- **THEN** the shell acceptance test fails instead of treating that frame as a valid alternate product hierarchy

### Requirement: Retained diagnostic remains transitional read-only content
Phase One SHALL retain the complete deterministic context diagnostic inside the main workspace so every existing control value and reducer-owned selected line remains observable. The diagnostic SHALL NOT be the production window contract, a writable state copy, or permission to add functional Patch/Mixer screens, reusable components, waveforms, or sample behavior in this phase.

#### Scenario: Existing PATCH control is inspected
- **WHEN** PATCH is active and an existing reducer-owned control is focused or edited
- **THEN** the workspace diagnostic shows the exact canonical row, selection marker, value, and lifecycle status from the shell's accepted generation

#### Scenario: Existing MIXER state is inspected
- **WHEN** MIXER is active
- **THEN** the workspace diagnostic preserves the complete descriptor-derived Patch/global body and retained selection inside the graphical hierarchy

### Requirement: Context switching remains semantic and audio-neutral
Physical shell input SHALL continue through normalized semantic input and the canonical reducer before any new shell projection is rendered. A context-only transition SHALL change only reducer-owned interaction context and generation-coherent projections; session values, parameter values, graph revision, commands, prepared ownership, routing, and rendered audio SHALL remain unchanged.

#### Scenario: Player switches from MIXER to PATCH
- **WHEN** the supported context input is processed by the production control path
- **THEN** the next shell frame, event record, state tree, retained diagnostic, and parameter projection agree on PATCH and one accepted generation while no audio command or structural change occurs

#### Scenario: Player switches back to MIXER
- **WHEN** the supported MIXER input is accepted after a discriminating PATCH frame
- **THEN** the next frame maps the side region to Inspector, restores reducer-owned MIXER selection, and produces sample-identical audio from otherwise identical render state

### Requirement: Graphical shell evidence is non-vacuous and retained
Automated acceptance SHALL render both reference viewports through the production update path, dispatch real supported input, and assert named-region identity, visibility, bounds, order, non-overlap, and projection-generation coherence before printing its success marker. Phase completion SHALL additionally run retained `make demo-live-graphical-shell`, backed by the exclusive `--demo-live-graphical-shell` mode, using the release-mode production window, real MIDI fixture, physical audio output, both top-level contexts, semantic cleanup, and full ownership teardown. `make demo-live` and `--demo-live` SHALL remain compatibility aliases for the newest cumulative live scene. Idle native frames SHALL be scheduled after 16 ms, and success SHALL be emitted only after window return, stream release, worker shutdown, graph draining, and zero active notes.

#### Scenario: Headless graphical-shell acceptance passes
- **WHEN** the named graphical-shell integration target completes
- **THEN** it has assertion-bearing evidence for both viewports, all five regions, PATCH and MIXER mappings, real input dispatch, coherent projections, and audio-neutral context switching

#### Scenario: Physical graphical-shell scene completes
- **WHEN** the retained Phase One live command runs on a supported system
- **THEN** rendered post-paint observations correlate both contexts and every region with their exact shell generation and finite nonzero physical fixture audio, then all notes reach zero, the window closes, the stream is released, worker and graph ownership are drained off callback, one `CREST_GRAPHICAL_SHELL_LIVE_OBSERVATION` records those measured facts, and the command exits successfully

#### Scenario: Evidence is inferred or teardown is incomplete
- **WHEN** region credit comes only from expected labels, a fake frame, a layout helper, a stale or mismatched generation, overlapping geometry, or silent audio, or when the window, stream, worker, graph ownership, or parent command remains live or failed
- **THEN** Phase One acceptance is incomplete and SHALL NOT be reported as passing

#### Scenario: Live and headless modes are mixed
- **WHEN** the graphical-shell live option is combined with a headless, observation, exhaustive-demo, or controlled-negative option
- **THEN** argument validation rejects the invocation before application startup

