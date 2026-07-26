# schema-driven-patch-page Specification

## Purpose
TBD - created by archiving change phase-3-patch-page-projection. Update Purpose after archive.
## Requirements
### Requirement: Semantic two-context selection and stable focus
PATCH and MIXER SHALL be the only top-level contexts. Startup SHALL select MIXER, `1` SHALL directly select MIXER, and `2` SHALL directly select PATCH through normalized input, a semantic context event, and the canonical reducer. Reducer-owned interaction state SHALL retain the prior MIXER selection separately from one PATCH focus identified by stable `PatchId`; accepted Patch installation SHALL initialize that focus to the first Patch in stable installation order.

#### Scenario: Player opens PATCH
- **WHEN** the fixture Patches are installed and the player presses `2`
- **THEN** the input translates to semantic PATCH selection, `AppState::apply` accepts it, PATCH becomes the projected context, and focus resolves to the first installed stable PatchId

#### Scenario: Player returns to MIXER
- **WHEN** the player has a retained MIXER selection, opens PATCH, and presses `1`
- **THEN** MIXER becomes active and its exact prior selection is restored without deriving it from PATCH rows or a window-owned index

#### Scenario: PATCH is selected before installation
- **WHEN** PATCH selection is reduced while no Patch focus can exist
- **THEN** the event is rejected as no Patches installed, state and generation remain unchanged, and a later valid event remains processable

### Requirement: Exact descriptor-driven Patch projection
PATCH SHALL project the focused Patch identity, name, MIDI channel, active CapabilityId and label, every installed engine choice in registry order, the four canonical ADSR values, and every visible active CapabilityDescriptor section and parameter in descriptor order. Rows SHALL retain stable semantic IDs, labels, value kinds, update classes, current typed values or asset references, units, and dependency results. Projection SHALL NOT use a SoundFont-specific or Braids-specific field list or capability-identity branch.

#### Scenario: SoundFont Patch is projected
- **WHEN** the focused Patch uses `instrument.soundfont.hidef`
- **THEN** the page contains its exact identity/channel/envelope, both installed registry choices, and the active descriptor's bank, program, percussion, and fixed asset rows as Structural read-only data in descriptor order

#### Scenario: Braids Patch is projected
- **WHEN** the focused Patch uses `instrument.braids`
- **THEN** the same projection contract contains its exact identity/channel/envelope, both installed registry choices, and Model, Timbre, and Color from the Braids descriptor exactly once in descriptor order

#### Scenario: Descriptor shape changes
- **WHEN** a conforming installed descriptor adds, removes, renames, reorders, hides, or disables a parameter
- **THEN** PATCH follows the production descriptor and active config, and any duplicated stale expected row set fails exact verification

### Requirement: Read-only first Patch-page increment
Every PATCH identity, routing, engine, ADSR, Scalar, and Structural row SHALL be read-only in this increment. Navigate or Adjust received while PATCH is active SHALL be rejected as `ActionUnavailableInContext` without changing state or generation, preparing an engine, publishing a graph, or selecting a fallback; later context selection SHALL still succeed.

#### Scenario: Adjustment is attempted in PATCH
- **WHEN** PATCH is active and an Adjust event is reduced
- **THEN** the typed rejection is recorded, canonical state and every projection generation remain unchanged, no audio or structural effect is emitted, and a following MIXER selection is accepted

#### Scenario: Engine choice is inspected
- **WHEN** the player inspects the engine row and its installed choices
- **THEN** the active engine and complete registry are visible but no choice can request preparation, replace the instrument, or change the graph

### Requirement: Generation-coherent audio-neutral context projection
An accepted context selection SHALL change only reducer-owned interaction context, accepted generation, and the generation-coherent serialized/view projections. Patch/config/envelope/mixer/global values, stable Patch focus, fixed ParameterSnapshot values and layout, active GraphRevision, MIDI routing, prepared ownership, audio-command count, structural queues, and rendered behavior SHALL remain unchanged. The existing basic window SHALL render exactly one immutable active-context projection and SHALL own no context or Patch-page state.

#### Scenario: Context-only transition reaches projections
- **WHEN** a valid MIXER-to-PATCH event is accepted
- **THEN** StateSnapshot, PatchPageProjection, TextProjection, StateTree, EventRecord, and ParameterSnapshot agree on the new generation and context while the parameter snapshot retains the exact prior values and graph revision and no AudioCommand is emitted

#### Scenario: Audio consequence is compared
- **WHEN** before-context and after-context parameter values are rendered from identical prepared engine and mixer state
- **THEN** the stereo output is sample-identical and no structural graph is prepared, published, swapped, acknowledged, or retired

#### Scenario: Basic adapter paints each context
- **WHEN** the same eframe update callback receives `2` and then `1`
- **THEN** consecutive frames render the canonical PATCH and MIXER text projections from AppLoop without adapter-owned tabs, mutable AppState, or a second projector

