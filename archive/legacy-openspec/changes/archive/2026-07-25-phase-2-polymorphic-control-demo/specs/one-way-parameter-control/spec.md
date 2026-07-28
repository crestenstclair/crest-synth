## MODIFIED Requirements

### Requirement: Single complete text view
The user interface SHALL remain one scrollable text view listing both installed capability descriptors, the active generic instrument config for every Patch, every current editable Patch mixer value, every common ADSR value, every descriptor-classified Scalar engine value, and all global parameters, with Patch sections separated by `------------------------------------------------------------` and the selected editable line visibly identified. Structural capability values SHALL remain visible but not selectable as live adjustments.

#### Scenario: View is projected from current state
- **WHEN** installed alternating Patches, generic instrument configs, envelope/mixer values, Scalar engine values, or selection change
- **THEN** the one text body contains every exact current value, walks instrument fields in production descriptor order, includes the required Patch separators, and places one selection marker at the projected selected line

#### Scenario: Patch capabilities expose different shapes
- **WHEN** navigation moves between a SoundFont Patch and a Braids Patch
- **THEN** its editable selection count is derived from the active descriptor, SoundFont structural fields are skipped, and Braids Model, Timbre, and Color are present without a hard-coded engine field list

### Requirement: Cross-projection equality
Serialized state and text SHALL contain the exact immutable capability registry and generic Patch instrument configs accepted for the same state generation, and serialized state, text, and published real-time parameters SHALL contain the exact current Patch identity, mixer values, common envelope, descriptor-classified Scalar instrument values, global values, and selection for that generation. The StateTree parameters branch and real-time snapshot SHALL additionally agree on the target graph revision, ordered Patch identities, and fixed Scalar layouts.

#### Scenario: Inspect an accepted Patch installation
- **WHEN** generic SoundFont and Braids configs are installed through the production reducer and their initial graph is prepared
- **THEN** the state tree and text projection contain the same descriptor ids, parameter specs, capability ids, assignments, asset references, and envelopes in canonical order, and the tree's parameter branch targets the prepared graph revision

#### Scenario: Inspect an accepted edit
- **WHEN** any editable Patch or global parameter is adjusted
- **THEN** the state tree, selected text value, and corresponding real-time parameter contain the same exact value and target graph revision while every unrelated Patch/config/envelope/value remains unchanged

## ADDED Requirements

### Requirement: Descriptor-derived Patch editable surface
The ordered editable surface for each Patch SHALL be derived from one production-owned resolver containing the four Patch mixer values, four common ADSR values, and every active descriptor parameter classified Scalar in descriptor order. Reducer navigation, adjustment, text selection, deterministic demo coverage, and live-demo coverage SHALL use this same surface.

#### Scenario: SoundFont Patch surface is derived
- **WHEN** a SoundFont Patch is selected
- **THEN** its mixer and ADSR values are editable while bank, program, percussion, and file remain visible Structural values outside the live selection cycle

#### Scenario: Braids Patch surface is derived
- **WHEN** a Braids Patch is selected
- **THEN** its mixer, ADSR, Model, Timbre, and Color values are each reachable exactly once in the declared order

### Requirement: Fixed engine-scalar audio projection
Each real-time Patch parameter value SHALL carry its canonical ADSR plus at most sixteen destructor-free engine Scalar values encoded in descriptor order. The projection SHALL contain no string, vector, capability-specific union, asset, or engine object, and choice values SHALL use only the index fixed by the active graph's immutable descriptor revision.

#### Scenario: Braids scalar snapshot is published
- **WHEN** a Braids value is accepted
- **THEN** the newest complete snapshot carries its matching finite encoded value in the graph-compatible Braids layout and no SoundFont Patch receives that slot

#### Scenario: Scalar capacity is exceeded
- **WHEN** an installed descriptor declares more than sixteen Scalar values
- **THEN** descriptor/registry construction fails before Patch installation or audio publication
