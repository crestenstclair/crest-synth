# Mission Specification: Expandable Effects and Bus Topology

**Mission Branch**: `feat/expandable-effects-and-bus-topology`
**Created**: 2026-07-28
**Status**: Draft
**Input**: User description: "open up the phase 3 mission with the arch reconciliation section" — Roadmap Phase 3, "Expandable effects and bus topology".

## Overview

Crest Synth today permits at most one fixed effect on a Patch and sends wet signal to two hardcoded global processors. This mission grows that foundation into the bounded effect and routing model the product interface requires: up to three ordered effect slots per Patch, and explicit buses, sends, and returns with eight bus returns.

It delivers the **architecture**, not a catalogue of effects. The existing reverb and delay are generalized into ordinary bus returns drawn from the same effect registry that fills Patch slots, so the planned roster of additional effects (phaser, flanger, echo, tape delay, distortion, bitcrush, granular, convolution reverb, compressor, EQ8) can later be added as registry entries with no further architectural change. No new third-party audio processing is introduced here.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Build an ordered effect chain on a Patch (Priority: P1)

A player editing a Patch moves focus to an effect slot row and cycles it through the available effects. They fill a second and third slot, then hear the combined result while notes are still sounding. Reordering which effect sits in which slot audibly and measurably changes the result, because the chain is applied in slot order.

**Why this priority**: This is the core of the phase. Slot count, ordering, and per-instance independence are the foundation every later effect in the roster depends on. Without it, nothing else in the mission has a place to live.

**Independent Test**: Configure a Patch with three effects through ordinary editing, play the fixture, and confirm each addition is audible, that the chain applies in slot order, and that multiple instances of the same effect do not share internal state.

**Acceptance Scenarios**:

1. **Given** a Patch with no effects configured, **When** the player cycles the first slot to an available effect, **Then** that effect becomes audible on that Patch alone and its parameters appear for editing.
2. **Given** a Patch with one effect configured, **When** the player fills the second and third slots, **Then** all three process that Patch in slot order and no fourth slot can be reached.
3. **Given** a Patch with two different effects configured, **When** their slot positions are exchanged, **Then** the rendered output differs measurably from the original order.
4. **Given** the same effect configured in two slots, **When** notes sound through both, **Then** each instance carries its own internal state and neither shortens, doubles, or borrows the other's tail.
5. **Given** an effect configured in a slot, **When** the player cycles that slot back to empty, **Then** the effect stops processing without interrupting held notes or other Patches.

---

### User Story 2 - Route tracks to shared destinations through explicit buses (Priority: P2)

A player raises a send on a mixer track and hears that track feed a shared wet destination. Eight such destinations exist and are individually addressable. The reverb and delay that were previously fixed global processors now occupy two of these destinations as ordinary effects, and any destination's contents can be changed by the same editing gesture used for a Patch slot.

**Why this priority**: Explicit routing identity is the second half of the phase and what makes shared processing configurable rather than hardcoded. It depends on the registry established by Story 1 but is independently demonstrable.

**Independent Test**: Raise a send on one track toward one destination, confirm only that destination receives signal, confirm the other seven are unaffected, and change a destination's effect through the ordinary editing path.

**Acceptance Scenarios**:

1. **Given** the default configuration, **When** the player inspects the wet destinations, **Then** eight are addressable and the first two carry the familiar reverb and delay behavior.
2. **Given** a track with all sends at zero, **When** the player raises one send toward one destination, **Then** only that destination produces wet signal and the remaining destinations stay silent.
3. **Given** two tracks sending to the same destination, **When** both play, **Then** their contributions sum at that destination and each track's send level scales only its own contribution.
4. **Given** a track that is muted or excluded by another track's solo, **When** its send is raised, **Then** it contributes no wet signal, because sends remain after the fader and after the mute/solo gate.
5. **Given** a wet destination holding one effect, **When** the player cycles it to a different available effect, **Then** the destination's processing changes for every track sending to it, with no interruption to the dry mix.

---

### User Story 3 - Reject an invalid topology without losing the running sound (Priority: P3)

A player attempts a topology change that cannot be honored. The attempt is refused, the reason is visible, the audio that was already playing continues untouched, and a subsequent valid change succeeds normally.

**Why this priority**: The roadmap requires topology changes to be "validated, observable, and recoverable without replacing the active graph on failure." This is the safety contract that makes the first two stories usable rather than dangerous, and it is verified last because it presupposes both.

**Independent Test**: Drive a change that fails validation or preparation while the fixture is playing, and confirm continuous audio, a visible reason, an unchanged prior configuration, and a successful retry.

**Acceptance Scenarios**:

1. **Given** the fixture is playing, **When** a topology change is refused, **Then** audio continues without dropout and the previously configured effects and routing remain exactly as they were.
2. **Given** a refused change, **When** the player inspects the surface, **Then** the reason is visible and attributable to the specific slot or destination that failed.
3. **Given** a refused change, **When** the player then makes a valid change, **Then** it is accepted and becomes audible, with no residue of the failed attempt.
4. **Given** an accepted topology change, **When** it becomes audible, **Then** the superseded configuration is disposed of away from the audio path and nothing is left owned at exit.

### Edge Cases

- A Patch is rerouted to a different mixer track while its effect chain is configured — the chain, its parameter values, and its per-instance state follow the Patch, not the track.
- Every Patch fills all three slots simultaneously, and every destination is occupied — the topology must remain within its declared bounds with no growth at render time.
- The player changes a slot while notes are still sounding through that Patch — the change must not tear, click, or drop the block in which it takes effect.
- Two topology changes are requested before the first has been acknowledged — the later request must not silently discard or reorder the earlier acknowledgment.
- A send is raised toward a destination that holds no effect — the destination contributes nothing rather than passing dry signal through unexpectedly.
- A slot's effect is removed while its parameters hold focus — focus must resolve deterministically to a valid neighbouring position rather than disappearing.
- Every track sends to the same destination at full level — summing must stay bounded and must not overflow or distort beyond the master safety stage.
- The visible surface changes shape because slots were added or removed — the semantic focus path must survive reprojection.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | User Story | Priority | Status |
|----|-------|------------|----------|--------|
| FR-001 | Three ordered effect slots | As a player, I want up to three ordered effect slots on a Patch so that I can shape its sound beyond a single fixed effect. | High | Open |
| FR-002 | Slot occupancy selection | As a player, I want to cycle an effect slot through the available effects and empty, using the same gesture that changes the engine, so that I do not learn a second vocabulary. | High | Open |
| FR-003 | Descriptor-driven slot parameters | As a player, I want each configured effect to present its own parameters for editing so that no effect requires bespoke screen logic. | High | Open |
| FR-004 | Order-faithful processing | As a player, I want effects applied in slot order so that rearranging the chain changes the result predictably. | High | Open |
| FR-005 | Independent instance state | As a player, I want each configured effect instance to own its internal state so that two instances of one effect never share or truncate each other's tails. | High | Open |
| FR-006 | Explicit bus identities | As a maintainer, I want buses, sends, and returns to be explicit validated identities so that routing is addressable rather than implied by hardcoded fields. | High | Open |
| FR-007 | Eight bus returns | As a player, I want eight shared wet destinations so that I can build more than the two fixed sends available today. | High | Open |
| FR-008 | Sends address bus identities | As a player, I want each track send to name the destination it feeds so that destinations can change without changing the send. | High | Open |
| FR-009 | Reverb and delay as registry effects | As a maintainer, I want the existing reverb and delay to become ordinary registry effects occupying the first two destinations so that one effect model governs both roles. | High | Open |
| FR-010 | Configurable return contents | As a player, I want to change which effect occupies a destination using the same gesture as a Patch slot so that shared processing is not fixed. | Medium | Open |
| FR-011 | Preserved send semantics | As a player, I want sends to remain after the fader and after the mute/solo gate so that a silenced track cannot keep feeding a wet destination. | High | Open |
| FR-012 | Prepared topology exchange | As a maintainer, I want every topology change prepared away from the audio path and exchanged complete, so that the render path never assembles or grows a graph. | High | Open |
| FR-013 | Validated rejection | As a player, I want an impossible topology change refused outright so that the running configuration is never left partly applied. | High | Open |
| FR-014 | Observable topology state | As a player, I want pending, accepted, and refused topology changes to be visible with their reason so that I can tell what the instrument is doing. | Medium | Open |
| FR-015 | Recovery after rejection | As a player, I want a valid change to succeed immediately after a refused one so that a mistake does not require a restart. | Medium | Open |
| FR-016 | Superseded graph retirement | As a maintainer, I want superseded topologies disposed of away from the audio path so that nothing is destroyed inside the render callback or leaked at exit. | High | Open |
| FR-017 | Focus survival across topology change | As a player, I want my place on the surface preserved when slots appear or disappear so that editing does not lose my position. | Medium | Open |
| FR-018 | Patch-owned chain across rerouting | As a player, I want a Patch's effect chain to follow the Patch when I reroute it so that routing and sound design stay independent. | Medium | Open |
| FR-019 | Retained live demo scene | As a maintainer, I want a retained named live demo for this phase so that its evidence cannot be replaced by later work. | High | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Render-path safety | Zero allocations, locks, blocking calls, file or network access, logging, panics, or destructor work occur on the audio render path, including during topology change. Measured occurrences must be exactly 0. | Reliability | High | Open |
| NFR-002 | Bounded topology capacity | Capacity for three slots per active Patch and eight returns is reserved ahead of rendering; 0 dynamic growth events occur at render time under any configuration reachable in this mission. | Performance | High | Open |
| NFR-003 | Atomic activation | A topology change becomes effective exactly at a block boundary; 0 rendered blocks may observe a partially applied topology. | Reliability | High | Open |
| NFR-004 | Audio continuity | 0 dropouts, underruns, or discontinuities attributable to a topology change across the full live demo scene. | Performance | High | Open |
| NFR-005 | Deterministic evidence | Two runs of the deterministic scene produce logically identical observations, with 0 differing checkpoints. | Reliability | High | Open |
| NFR-006 | Clean teardown | At scene end: 0 active notes, 0 retained topology owners, 0 leaked audio or worker resources, and a normal parent-process exit. | Reliability | High | Open |
| NFR-007 | Routing isolation | With one send raised toward one destination, every non-target destination measures below −60 dBFS attributable to that source. | Reliability | Medium | Open |
| NFR-008 | Edit responsiveness | An accepted edit is reflected in the visible surface within 1 frame of acceptance, and its audible effect within 1 render block of activation. | Performance | Medium | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | Slot ceiling | At most three ordered effect slots per Patch; the product maximum recorded in `DESIGN.md` must not be exceeded. | Technical | High | Open |
| C-002 | Return ceiling | At most eight bus returns in the prepared topology. | Technical | High | Open |
| C-003 | Two top-level contexts | PATCH and MIXER remain the only top-level contexts. | Technical | High | Open |
| C-004 | No new processing dependencies | This mission introduces no new third-party audio processing; it reuses existing processing in new roles. | Technical | High | Open |
| C-005 | Send position fixed | Sends remain post-fader and post-gate; mute always wins and solo exclusion suppresses wet contribution. | Technical | High | Open |
| C-006 | No arbitrary cycles | Feedback exists only inside bounded effect implementations, never as a routing cycle. | Technical | High | Open |
| C-007 | Sixteen fixed tracks | The canonical sixteen-track mixer bank and Patch route/trim ownership established by the corrective gate are preserved unchanged. | Technical | High | Open |
| C-008 | No choice modal | Topology is edited in place; modal choice surfaces belong to Phase 7 and are out of scope. | Technical | Medium | Open |
| C-009 | Spec reconciliation is not deferrable | The architecture spec and `DESIGN.md` must be updated during planning, before implementation, because this mission contradicts their current declarations. | Technical | High | Open |
| C-010 | Live demo gate | The phase is incomplete until its retained live scene has been run successfully with a real window, physical audio, and the real MIDI fixture. | Business | High | Open |
| C-011 | Roster deferred | The twelve-effect roster is out of scope; this mission must leave those additions as registry entries requiring no architectural change. | Business | High | Open |

### Key Entities

- **Effect slot**: An ordered position on a Patch that is either empty or occupied by one effect, carrying that effect's own parameter values. Three exist per Patch.
- **Effect registry entry**: One available effect, describing its identity, its editable parameters, their bounds and units, and what it requires to be prepared. The same entry can occupy a Patch slot or a bus return.
- **Bus**: A named routing destination that tracks may feed. Its identity is stable and independent of what currently occupies it.
- **Send**: A track-owned amount directed at one named bus, taken after the fader and after the mute/solo gate.
- **Bus return**: The processing and output stage of a bus, occupied by zero or one effect from the registry, summing into the mix. Eight exist.
- **Prepared topology**: A complete, bounded, ready-to-render arrangement of engines, slots, routes, buses, and returns, exchanged as a whole and never assembled on the render path.
- **Topology change outcome**: The pending, accepted, or refused status of a requested change, with the reason and the position it applies to.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A player can configure any Patch with three ordered effects and hear each addition take effect with 0 audible dropouts of the surrounding mix and 0 torn or truncated blocks. Clearing a slot leaves held notes uninterrupted; installing or changing an occupant may end sounding notes at the boundary (operator ruling 2026-07-31: mid-performance effect additions are rare enough that note preservation is required only for clearing and for value edits).
- **SC-002**: Exchanging the positions of two different effects produces a measurably different rendered output, confirming order is honored in 100% of trials.
- **SC-003**: Two instances of the same effect on one Patch produce independent tails, with neither instance's output altered by the other's presence.
- **SC-004**: Eight wet destinations are individually addressable, and a send raised toward one leaves the other seven below −60 dBFS from that source.
- **SC-005**: A muted or solo-excluded track contributes 0 measurable wet signal to any destination.
- **SC-006**: 100% of refused topology changes leave audio uninterrupted and the prior configuration audibly and visibly intact, and a valid change immediately afterwards succeeds.
- **SC-007**: The retained live scene completes with every declared behavior demonstrated, ending in 0 active notes, released resources, and a normal exit.
- **SC-008**: Adding a further effect to the registry after this mission requires 0 changes to slot, routing, preparation, projection, or render structure.

## Domain Language

Canonical terms for this mission; avoid the listed alternatives so later work does not drift.

| Canonical term | Meaning | Avoid |
|----------------|---------|-------|
| effect slot | An ordered position on a Patch holding zero or one effect | insert, FX unit, chain link |
| bus | The stable routing destination identity | aux, group, submix |
| send | Track-owned amount directed at one bus | aux send level, wet knob |
| bus return | The processing and output stage of a bus | global effect, shared FX, master FX, wet destination (in code) |
| registry entry | One available effect and its declared schema | plugin, module, processor type |
| prepared topology | The complete ready-to-render arrangement | graph rebuild, patch graph, live graph |

"Global reverb" and "global delay" are retired as concepts once they become bus returns; refer to them as the reverb return and the delay return. "Destination" remains acceptable in player-facing prose (as in the user stories above), but code identifiers use the bus / bus return vocabulary.

### Cross-cutting rename

This mission renames name-enumerated routing and effect concepts into generic,
index-addressed ones across roughly 25 files. The per-category rules are captured
in `occurrence_map.yaml` and must be satisfied by the implementation:

- `GlobalEffectsProcessor` (a two-input port) is retired in favour of a bounded bus return rack.
- `MixerTrackParameter::ReverbSend` and `::DelaySend` become one indexed send array addressed by bus.
- `GlobalParameter`'s six reverb and delay fields become descriptor scalars of registry effects; only master gain remains global.
- `GlobalReverbDelay` splits into ordinary registry effect capability and preparer adapters.

The renames are structural, not cosmetic: in each case a hardcoded per-effect
name is replaced by an index into a descriptor-driven array. An implementation
that preserves the old names has not satisfied FR-006 or FR-009.

## Architecture Reconciliation

The architecture spec currently declares this mission's subject matter a **non-goal**. Per the spec's own rule — "Never silently plan around a conflict with the architecture spec" — these declarations must be edited during planning and the sources reloaded. This section names what planning must reconcile; the plan carries the authoritative, expanded version.

### Declarations that contradict this mission

| Source | Declaration | Conflict |
|--------|-------------|----------|
| `project.yaml:17-19` | `nonGoals.additional_effects` — "does not provide another insert type, more than one slot per Patch, bypass, selection, reordering, effect chains" | FR-001, FR-002, FR-004 require multiple ordered slots and selection |
| `project.yaml:32-35` | `nonGoals.later_roadmap_phases` — excludes "Phase 3 expansion beyond at most three effect slots per Patch and eight bus returns" | This mission *is* that expansion, up to exactly those bounds |
| `project.yaml:47-52` | `meta.avoid` — "effect selection, bypass, multiple slots, reordering, or arbitrary routing"; "arbitrary buses" | FR-002, FR-006, FR-007 require selection and explicit buses; note the mission's buses are bounded and validated, not arbitrary |
| `DESIGN.md:689` | "Chorus is the first concrete Patch effect" | Reverb and delay join the same registry (FR-009) |
| `DESIGN.md:691` | "statically bounded to zero or one post effect per Patch… no selector, bypass, reorder, placeholder, or dynamic graph editing yet" | Directly superseded by FR-001 through FR-004 |

`DESIGN.md:418` and `DESIGN.md:690` already sanction the target state — three ordered post-FX slots and eight bus returns as the product maximum — so the reconciliation raises the executable slice to a ceiling the master design already authorizes. This is not a product redefinition.

### Canonical resources this mission affects

- **Contexts**: `context.Synth` (effect slots and registry), `context.Mixer` (buses, sends, returns), `context.RealTime` (bounded prepared topology and snapshot capacity), `context.Control` (slot and destination focus, valid actions, status, errors), `context.Testing` (the retained live scene and its measurements)
- **Ports**: `port.Mixer.GlobalEffectsProcessor` is reframed as a bus return; `port.Synth.EffectCapabilityProvider` and `port.Synth.EffectPreparer` extend to serve both slot and return roles
- **Adapters**: `adapter.GlobalReverbDelay` becomes registry-backed return processing; `adapter.ChorusCapability` and `adapter.ChorusPreparer` gain multi-instance and return-role coverage; `adapter.EframeGraphicalWindow` renders slot and destination rows without owning their state
- **Intent model**: a new capability for expandable effects and bus topology, a new goal for `completion.requiredGoals`, and new requirement entries; the two non-goal entries above must be narrowed rather than deleted, since the roster and later phases remain excluded
- **Proof**: new validation and witness entries plus evidence for the phase, and a new project check alongside the existing `static_patch_effect` and `sixteen_track_mixer_routing` gates

### Open reconciliation judgment

If reframing the global effects port and its adapter becomes a genuine cross-file rename rather than a contained change, the mission adopts bulk-edit handling with an occurrence map. This is decided during planning, when the actual footprint is known, rather than pre-committed here.

## Assumptions

- Because bus returns draw from the same registry as Patch slots, the existing reverb and delay processing is lifted into that registry as ordinary entries. This is a reframing of processing that already exists, not new processing (C-004).
- Returns three through eight are addressable and bounded from the start; they begin unoccupied, and occupying them uses the same gesture as any other slot.
- "Available effects" during this mission means the registry as it stands — the reframed reverb and delay plus the existing chorus. Slot ordering is therefore demonstrable with genuinely different processing, without adding any dependency.
- The mixer's sixteen tracks, Patch route ownership, and Patch trim established by the corrective gate carry forward untouched.
- The retained scene for this phase is named `make demo-live-effects-and-buses`, and `make demo-live` advances to it as the newest cumulative scene while every earlier phase scene remains runnable.

## Out of Scope

- The twelve-effect roster: phaser, flanger, echo, tape delay, distortion, bitcrush, granular, convolution reverb, compressor, and EQ8. These follow as registry entries in later missions.
- Asset-backed effects requiring impulse responses or sample material; the asset contract remains deferred to Phase 7.
- Modal choice surfaces for effects or routes (Phase 7).
- The reusable component library (Phase 4), the complete Patch editor (Phase 5), and component-polished Mixer composition (Phase 6).
- Modulation sources, modulation routing, and any modulation matrix.
- Arbitrary graph editing, plugin hosting, preset or session persistence.
- More than three slots per Patch or more than eight returns.
