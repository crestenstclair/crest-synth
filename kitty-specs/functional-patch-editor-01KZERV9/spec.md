# Mission Specification: Functional Patch Editor Blockout

**Mission Branch**: `feat/functional-patch-editor`
**Created**: 2026-08-07
**Status**: Draft
**Input**: ROADMAP.md "Phase 5 — Functional Patch editor blockout" and its entry
condition (LIMIT-1); DESIGN.md "Controller-first interface → Patch"; the Phase 4
carry-forward list transcribed at
`kitty-specs/crest-component-controls-and-compositions-01KZ25VX/cross-wp-findings.md`
§F-10.

## Crest-Spec Grounding

This mission derives from the crest-spec and cites its canonical IDs rather than
restating them. It advances `capability.schema_driven_patch_page` from a flat
descriptor-ordered row list into the authored Patch surface, extends
`capability.expandable_effects_and_bus_topology` onto the PATCH context, composes
that surface from `capability.component_vocabulary`, and adds one
`capability.live_observable_demo` scene.

Goals served: `goal.inspect_patch`, `goal.control_synth`, `goal.select_patch_engine`,
`goal.edit_patch_envelope`, `goal.shape_patch_with_effect`,
`goal.route_patches_through_sixteen_tracks`, `goal.build_from_component_vocabulary`,
`goal.observe_live_synth`.

Requirements this mission must not weaken: `requirement.one_way_loop`,
`requirement.hard_realtime_audio`, `requirement.canonical_semantic_focus_contract`,
`requirement.deterministic_focus_recovery`, `requirement.stable_patch_focus`,
`requirement.exact_valid_action_contract`,
`requirement.no_placeholder_values_in_production`,
`requirement.shell_composed_from_components`, `requirement.webview_projection_shell`,
`requirement.responsive_control_projection`, `requirement.ordered_patch_effect_slots`,
`requirement.patch_output_route_and_trim`, `requirement.explicit_state_rendering`,
`requirement.canonical_live_projection`, `requirement.live_current_surface`.

**Structure this mission needs that the crest-spec does not yet declare** (to be
authored in the `/spec-kitty.crest-spec` phase, before planning):

1. A **detail surface** identity. `SurfaceId` is a closed four-variant union
   (`PatchMain`, `PatchUtility`, `MixerMain`, `MixerInspector`) with no detail
   member, and `SemanticSurfaceRole` has no detail role. FR-012 needs both.
2. A **per-Patch voice limit** value on canonical state, its descriptor, its
   projection path, and its real-time enforcement point. FR-009 needs all four;
   nothing exists today (F-10 item 3: "no state, no descriptor, no path anywhere").
3. A **PATCH-side path to master gain**. Master gain is declared only as
   `MixerControlId::Global{MasterGainDb}` on the MIXER Inspector. FR-007 needs a
   PATCH Utility path to the same canonical value — one value, two surfaces, never
   a second copy.
4. A **MIDI input channel** control path. `Patch` carries `MidiChannel`;
   `patch_utility_paths` projects only `PatchOutputParameter`. FR-008 needs the
   channel declared as an editable Utility control.
5. A **stepped-parameter** control kind with a production producer. The kind is
   already selectable in the component vocabulary but no capability projects one;
   FR-009's voice limit becomes its first production producer.
6. **Per-row valid actions**. `validActions` are declared global to the focus;
   FR-011 needs them resolvable per row.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Reach the other fourteen instruments (Priority: P1)

The player has fifteen Patches installed, one per MIDI part. Today the controller
can edit exactly one of them. The player presses the patch-selection gesture and
the shell moves to the next installed Patch: the strip's identity, routing,
engine, envelope, and effect rows all re-project against the destination Patch's
own descriptor schema, and focus recovers to a valid row on that Patch.

**Why this priority**: This is the outstanding half of LIMIT-1 and the reason the
phase exists. The reducer half landed 2026-08-02; nothing on screen has ever shown
it. Without it every other story is confined to `patches.first()`.

**Independent Test**: Drive `SemanticAction::SelectPatch(Direction)` through the
production keyboard translator with the fixture installed and observe that the
projected `patchId`, patch name, MIDI channel, engine label, and effect row set
all change together in one projection generation, and that focus lands on a row
the destination Patch actually declares.

**Acceptance Scenarios**:

1. **Given** the fifteen-Patch fixture with focus on the first Patch's Engine row,
   **When** the player sends the rightward patch-selection gesture, **Then** the
   next projection names the second Patch, its own capability label and MIDI
   channel, and a focused row belonging to that Patch — in one generation, with no
   intermediate projection naming a Patch/schema mismatch.
2. **Given** focus on the last installed Patch, **When** the player sends the
   rightward patch-selection gesture, **Then** the reducer returns the typed
   unchanged rejection and the projection is unchanged — it refuses rather than
   wrapping.
3. **Given** a destination Patch whose descriptor declares fewer rows than the
   origin, **When** the switch lands, **Then** focus resolves deterministically
   against the destination's schema rather than carrying a stale row identity.

---

### User Story 2 — Edit a second instrument audibly (Priority: P1)

Having switched Patches, the player walks the destination Patch's effect slots,
places an occupant in a slot, focuses the occupant's parameter rows, and adjusts
one. The sound changes, and the change is observable as a correlated
patch-switch → focus → audible-consequence chain.

**Why this priority**: An on-screen switch that cannot then be edited proves
navigation, not reach. LIMIT-1 requires the full journey on the second instrument.

**Independent Test**: `make demo-live-patch-editor` runs end to end with a real
window and physical audio, and its report carries checkpoints correlating the
patch switch, the resulting focus path, and the measured audible consequence.

**Acceptance Scenarios**:

1. **Given** the second Patch focused after an on-screen switch, **When** the
   scene performs a full focus-verified effect-slot occupancy journey on it,
   **Then** each slot transition is checkpointed with its focus path before and
   after, and the graph revision advances.
2. **Given** an occupied slot on the second Patch, **When** the scene adjusts an
   occupant scalar parameter, **Then** the checkpoint records the parameter
   identity, the value change, and the measured audio observation on the
   destination Patch's own output — not the first Patch's.
3. **Given** the scene completes, **Then** the report shows zero dropped events,
   zero callback allocations, and a clean teardown.

---

### User Story 3 — The Patch surface reads as the authored strip (Priority: P2)

The player looks at PATCH and sees the designed structure: patch identity and
routing at the top, one instrument selector, the four envelope rows as a legible
group with their bounds and units, then the ordered effect slots with their
occupants' parameters nested beneath them — not one undifferentiated list of rows.

**Why this priority**: The strip is the phase's named deliverable and the surface
every later phase builds on. It is P2 rather than P1 because Stories 1 and 2 are
the gate; this is what makes the gate worth passing.

**Independent Test**: Render the production `SemanticGraphicalViewModel` for a
SoundFont Patch and a Braids Patch through the production webview path and assert
the composed structure — grouping, order, painted bounds and units — against the
authored composition, at both authored viewports.

**Acceptance Scenarios**:

1. **Given** a SoundFont Patch, **When** PATCH renders, **Then** identity and
   routing, the engine selector, the four envelope rows, the preset row, and the
   effect slots appear as distinguishable composed groups in the declared order.
2. **Given** any numeric row, **When** it renders, **Then** its declared range and
   unit are painted, not merely projected.
3. **Given** the compact authored viewport, **When** PATCH renders, **Then** the
   header/footer bands, the persistent Utility panel, and minimum interactive
   targets all survive, and no row is clipped or overlapped.

---

### User Story 4 — The Utility panel does its designed job (Priority: P2)

The player moves rightward from the strip into Utility and finds five live rows:
master volume, patch volume, MIDI input, output track, and voice limit. Every one
carries a real value from canonical state; none is a placeholder and none is
marked unavailable.

**Why this priority**: Four of the ten designed-but-undriven structures live on
this panel. It is the densest single closure available and the phase's declared
input.

**Independent Test**: Project PATCH Utility from canonical state and assert five
controls with real typed values, each editable through the production reducer, with
master gain resolving to the same canonical value the MIXER Inspector edits.

**Acceptance Scenarios**:

1. **Given** master gain adjusted on PATCH Utility, **When** MIXER is selected,
   **Then** the Inspector shows the same value — one canonical value, two surfaces.
2. **Given** the voice limit set below the number of notes the fixture sustains,
   **When** the fixture plays, **Then** notes beyond the limit do not sound, the
   limit is observable in the audio, and the callback still allocates nothing.
3. **Given** the MIDI input row adjusted, **When** the fixture sends on the newly
   selected channel, **Then** that Patch responds and the previously selected
   channel no longer drives it.
4. **Given** any Utility row, **When** it renders, **Then** its label is the
   authored label, never a serialization key such as `masterGainDb`.

---

### User Story 5 — One detail shell, two subjects (Priority: P3)

From a focused instrument row or a focused effect row, the player opens detail and
gets the same shell either way — title, accent, sections, values, ranges, units,
status, and errors all supplied by the active capability — and closing returns to
the exact originating row.

**Why this priority**: The polymorphic shell is Phase 5's named deliverable, but
its *content* completion (sample detail, waveform landmarks, choice modals) is
Phase 7's. P3 marks that this mission builds the mechanism, not the full content.

**Independent Test**: Enter detail from an instrument row and from an effect row on
the same Patch; assert one surface implementation serves both, that sections and
controls come from the capability descriptor, and that return lands on the exact
origin path after reprojection.

**Acceptance Scenarios**:

1. **Given** a focused instrument row, **When** detail is opened and closed,
   **Then** focus returns to that exact row, not to a recomputed default.
2. **Given** a focused effect-occupant row, **When** detail is opened, **Then** the
   same shell renders with the effect capability's own title, accent, and sections.
3. **Given** a capability that declares a read-only section, **When** detail
   renders it, **Then** it is explicitly marked read-only in text or shape, not by
   color alone.

### Edge Cases

- The patch-selection gesture at either end of the installed order: refuses, does
  not wrap, and leaves the projection byte-identical.
- A patch switch while a structural edit is in flight on the origin Patch: the
  in-flight work stays correlated to its own Patch and does not follow focus.
- A destination Patch with zero configured effect slots: the strip shows the
  declared empty slots as addressable positions, not as absent structure.
- Voice limit set to its minimum while notes are already sounding: existing notes
  are not truncated mid-callback; the limit applies at the next note-on.
- A capability whose descriptor declares more rows than the Utility panel band can
  seat: the panel's row set is bounded by declaration, so this cannot arise on
  PATCH — the equivalent MIXER Inspector overflow stays open and deferred (C-003).
- Detail opened on a row whose capability is mid-preparation: the shell shows the
  typed lifecycle status rather than an empty or stale section set.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | User Story | Priority | Status |
|----|-------|------------|----------|--------|
| FR-001 | Patch identity and routing header | As a player, I want the focused Patch's name, MIDI channel, and output track composed as a header above the strip so that I always know which instrument I am editing and where it goes. | High | Open |
| FR-002 | On-screen patch-to-patch navigation | As a player, I want the patch-selection gesture to visibly move the whole PATCH surface to the next installed Patch so that all fifteen instruments are reachable from the controller. | High | Open |
| FR-003 | Instrument selection from the strip | As a player, I want to change the focused Patch's engine from the strip and see the active value, the requested value, and the lifecycle status while the change is in flight. | High | Open |
| FR-004 | Ordered effect slots on PATCH | As a player, I want every declared effect slot addressable in order, with occupancy settable from the PATCH surface and each occupant's parameter rows nested beneath its slot. | High | Open |
| FR-005 | Visible ADSR group | As a player, I want the four envelope rows presented as one legible group with their declared bounds and units so that shaping a voice does not mean reading four unrelated rows. | High | Open |
| FR-006 | Persistent Utility panel | As a player, I want a Utility panel that stays visible on PATCH and carries master volume, patch volume, MIDI input, output track, and voice limit. | High | Open |
| FR-007 | Master volume on PATCH Utility | As a player, I want master volume editable from PATCH Utility, resolving to the same canonical value the MIXER Inspector edits, with no second copy. | High | Open |
| FR-008 | MIDI input on PATCH Utility | As a player, I want the focused Patch's MIDI input channel projected and editable so that I can re-assign which incoming part drives this instrument. | Medium | Open |
| FR-009 | Voice limit | As a player, I want a per-Patch voice limit that is canonical state, projected as a stepped control, and enforced in the audio path so that the row means something. | Medium | Open |
| FR-010 | Requested value during a structural edit | As a player, I want a row mid-structural-edit to show both its active and its requested value so that I can see what the system is moving toward. | Medium | Open |
| FR-011 | Per-row action hints | As a player, I want each row to carry the actions valid at that row so that the strip tells me what I can do without my having to try. | Medium | Open |
| FR-012 | Polymorphic detail shell | As a player, I want one detail surface reused for instruments and effects, with capability-supplied title, sections, values, ranges, units, status, and errors, returning to the exact originating row. | Medium | Open |
| FR-013 | Painted ranges and units | As a player, I want every numeric row's projected range and unit rendered on screen so that adjustment has visible context. | Medium | Open |
| FR-014 | Authored labels, not serialization keys | As a player, I want every control labeled by its authored label so that no serialization key such as `masterGainDb` reaches the screen. | Medium | Open |
| FR-015 | Utility hint line | As a player, I want the Utility panel's authored hint line rendered so that the panel's own affordances are stated, not dropped. | Low | Open |
| FR-016 | `make demo-live-patch-editor` | As a maintainer, I want one retained live scene that navigates Patch-to-Patch on screen through the patch-selection gesture, then performs a full focus-verified effect-slot occupancy journey and an audible occupant parameter edit on the second instrument, with checkpoints correlating switch, focus, and audible consequence. | High | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Callback contract under voice limiting | The audio callback reports `callbackAllocations = 0` across the full `demo-live-patch-editor` run with voice limiting active, and performs no locking, blocking, I/O, logging, panic, or destruction. | Real-time safety | High | Open |
| NFR-002 | Live scene completion bar | `make demo-live-patch-editor` exits 0 with a complete report: 0 dropped events, all declared checkpoints satisfied, and a clean teardown with owned-resource cleanup, on a real window with physical audio. | Reliability | High | Open |
| NFR-003 | Both authored viewports seat the surface | At 1920×1080 and at 1280×800 the full PATCH surface renders with the five shell bands intact, the Utility panel at ≥ 320 px, every interactive target at or above the declared minimum, and no clipped or overlapped row. | Usability | High | Open |
| NFR-004 | Projection throughput unchanged | The live scene produces > 0 qualifying webview frames with monotonic projection generations and no generation gap, matching the bar the retained scenes already hold. | Performance | Medium | Open |
| NFR-005 | Patch switch is one generation | A patch-selection gesture produces exactly one advanced projection generation; no intermediate projection pairs one Patch's identity with another's schema. | Correctness | High | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | One-way loop only | Every edit travels physical input → semantic action/event → `AppState::apply` → projection. No UI-local state, no view-owned domain copy, no backstage mutation. | Technical | High | Open |
| C-002 | No choice modals, no asset surfaces | Engine/effect/route choice modals with trapped focus, sample detail, waveform landmarks, and the Sample Browser are Phase 7. This mission adds the detail *shell*, adjacent-choice editing, and nothing modal. | Scope | High | Open |
| C-003 | MIXER-side carry-forwards stay deferred | The undrivable mixer meter, `MixerControlId::Track` inspector reachability, the `M`/`S` label divergence, the multi-select help block, and the MIXER Inspector's lost scroll remain open and belong to Phase 6. | Scope | High | Open |
| C-004 | Crest-spec first | The crest-spec is authored before planning; `plan.md` records a `## Crest-Spec Derivation` section; no `data-model.md` and no `contracts/` are produced. | Process | High | Open |
| C-005 | Gated live suite needs the external display | `make demo-live-patch-editor` runs only on a display seating 1920×1080; the harness refuses rather than degrading. Evidence is captured on that rig. | Technical | High | Open |
| C-006 | No placeholder values | A structure with no data behind it is marked unavailable or omitted, never filled with a plausible value. Every row this mission claims to drive must carry a real canonical value. | Technical | High | Open |

### Key Entities

- **Patch** — the focused instrument. Gains a canonical voice limit; already
  carries identity, MIDI channel, instrument config, envelope, effect slots, and
  output routing.
- **Voice limit** — a bounded per-Patch count of simultaneously sounding notes,
  owned by canonical state, published to the real-time domain on the existing
  latest-scalar-snapshot transport, and enforced at note-on.
- **Detail surface** — a new semantic surface identity, entered from a focused
  instrument or effect row, carrying an exact return path to its origin.
- **Patch Utility control set** — the five-row bounded set: master volume (an
  alias path onto the canonical master gain), patch volume, MIDI input channel,
  output track, and voice limit.

## Scope Decisions on the Phase 4 Carry-Forward List

The carry-forward list is this mission's declared input. Each item is dispositioned
here rather than silently inherited.

**Supplied by this mission:**

| Carry-forward item | Requirement |
|---|---|
| 1. PATCH Utility `MASTER VOLUME` has no path | FR-007 |
| 2. MIDI INPUT projects nothing | FR-008 |
| 3. VOICE LIMIT has no state, descriptor, or path | FR-009 |
| 4. Requested value of a row mid-structural-edit | FR-010 |
| 5. Per-row action hints do not exist | FR-011 |
| 8. Global rows label themselves with serialization keys | FR-014 |
| 11. The Utility panel's authored hint line is dropped | FR-015 |
| 13. `numeric_range` and `unit` projected but never painted | FR-013 |
| Control kind: stepped parameter has no production producer | FR-009 (the voice limit becomes it) |
| ~~Control kind: read-only surface summary has no production path~~ | **WITHDRAWN 2026-08-09 — see below** |
| The persistent side region lost its scroll — **PATCH half** | FR-006: the Utility row set is bounded by declaration to five rows, which seat within the panel at both authored viewports. The remedy is the bounded declaration, not a restored scroll area. |

**Withdrawn during implementation (2026-08-09):**

**The read-only surface-summary control kind is not closed by this mission.** The
claim above was wrong when I wrote it, and WP03's review established why by
execution.

Two different things were conflated. FR-012 does supply the capability-declared
read-only **fact**: `PatchInteraction::ReadOnly` reaches the screen at
`patchPage.detail.sections[].parameters[].patchInteraction`, it is discriminating
(SoundFont declares two different interactions in one section), and detail rows
are marked read-only in text or shape. That half is real and stays.

But the carry-forward item was about a **control kind** —
`SemanticControlKind::Surface` carrying a `SemanticControlValue::Summary` — stated
in parallel with the stepped-parameter kind that FR-009 genuinely closes. That
kind still has exactly one construction site, reachable only from a projection
builder whose own documentation says production never uses it. Detail rows take
their kind from `ParameterKind` and project `Continuous`, `Choice`, or `Asset` —
never `Surface`.

Closing it would mean inventing a surface-summary control the product design does
not call for, purely so a checkbox could be ticked. That is precisely what C-006
forbids: a designed structure with nothing behind it is marked unavailable, not
filled with a plausible value. The same rule that governs the UI governs the
mission's own claims about itself.

So: **deferred, with the reason stated.** Whoever next needs a read-only surface
summary should design one because a surface needs it, not because this table once
said it was handled.

**Deliberately deferred:**

| Carry-forward item | Deferred to | Why |
|---|---|---|
| 6. Inspector three-line help block, incl. "SELECT enters multi-select" | Phase 8 | The reducer has no multi-select; the help line cannot be made true here. |
| 7. `MixerControlId::Track` unreachable from `mixer_inspector_paths` | Phase 6 | MIXER-owned; this mission does not touch the Inspector's path set. |
| 9. The mixer meter is not drivable | Phase 6 | Requires an audio-observation path to a MIXER composition. |
| 10. Sub-band constants without a density accessor | **Retired (confirmed 2026-08-07)** | `WORKSPACE_TITLE_ROW_PX` and `MIXER_TRACK_MIN_WIDTH_PX` no longer exist anywhere in `src/`; they went out with the egui adapter at the webview cutover. `ViewportDensityPolicy` remains declared. There is nothing left to give an accessor to, so the item is closed rather than carried. |
| 12. The `M`/`S` label divergence | Phase 6 | MIXER-owned. |
| The persistent side region lost its scroll — **MIXER half** | Phase 6 | The Inspector's row set is unbounded (eight sends, eight returns, occupant rows, globals); the remedy is a decision about that surface, and that decision belongs with the phase that owns it. |

**Folded in from the previous mission's LOW follow-ups** (settled 2026-08-07 against
`kitty-specs/shell-hygiene-01KZD0KR/semantic-acceptance.md`, "New findings"):

| Follow-up | Disposition |
|---|---|
| WP02 OBS-A — `retire()`'s de-duplication guard is load-bearing but unproven (`src/shell/webview/projection_channel.rs:564`); the reviewer left the proving test ready to paste | **Folded in.** This mission drives far more projection churn through that channel — a whole-surface reprojection per patch switch, plus the detail surface appearing and disappearing — so a stale identity shadowing a current one is squarely in this mission's blast radius. The guard gets its test. |
| WP04 — `PAINTED_ACK_IDENTITY_FIELDS`'s doc claims two consumers, has one | **Closed.** The constant no longer exists in `src/`; it went out with the frame-stream residue removal. Nothing to correct. |
| WP03 — the two unconsumed residues above | **Closed** by the two rows above. |
| WP01 F4 — `std::env::set_var` in `window.rs` tests races `var_os` in the same test binary | **Stays filed.** This mission does not change `src/shell/webview/window.rs`; folding it in would be a change outside the mission's domain, which is what the deferral rule exists to prevent. |
| Pre-existing soak flake — `receive_phase("meters")` fails roughly 1 run in 5 under load | **Stays filed.** Mixer-side and established as pre-existing; this mission touches neither the meters phase nor the soak harness. |

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All fifteen installed instruments are reachable and editable from the
  controller, up from one.
- **SC-002**: `make demo-live-patch-editor` completes on hardware with a report
  that correlates, in order, a patch switch, the resulting focus, a full effect-slot
  occupancy journey on the second instrument, and a measured audible parameter edit
  on that second instrument.
- **SC-003**: Zero rows on the shipped PATCH surface are marked unavailable for
  want of data — every designed PATCH structure this mission claims either carries a
  real canonical value or is explicitly deferred in the table above. The read-only
  surface-summary control kind is one such deferral (withdrawn 2026-08-09), so an
  assertion of this criterion names it as a declared exception rather than counting
  it as a failure or quietly widening the bound.
- **SC-004**: The audio callback allocates nothing and drops nothing across the full
  live run with voice limiting active.
- **SC-005**: The complete PATCH surface renders without clipping or overlap at both
  authored viewports.

## Assumptions

- The gesture half of LIMIT-1 (`SemanticAction::SelectPatch`) is landed and
  correct; this mission consumes it and does not redesign it.
- Phase 5 builds the detail *shell*; Phase 7 completes detail *content*. ROADMAP
  lists "Reuse one polymorphic detail shell" under Phase 5 and "Instrument detail
  and effect detail" under Phase 7; this reading is the deliberate resolution of
  that overlap and is recorded here rather than settled by editing the roadmap.
- Voice-limit enforcement refuses notes beyond the limit rather than stealing
  voices. This matches the codebase's existing "refuse rather than wrap" idiom and
  is the more falsifiable of the two policies. If crest-spec authoring finds a
  stronger reason for stealing, it is decided there, before planning.
- The fifteen-Patch fixture (8 SoundFont, 7 Braids) remains the live subject.
