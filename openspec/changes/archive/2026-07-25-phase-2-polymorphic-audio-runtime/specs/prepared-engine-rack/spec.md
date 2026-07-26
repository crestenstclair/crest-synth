## MODIFIED Requirements

### Requirement: Bounded heterogeneous Patch routing
The prepared rack SHALL use fixed-capacity ordered storage, route each Patch-targeted audio command and only that Patch's matching fixed scalar/envelope projection to the prepared instrument with the exact `PatchId`, render each active instrument once per block into only its matching caller-owned stereo stem, and visit at most the fixed Patch capacity for a global all-notes-off. An unknown Patch identity or parameter-layout mismatch SHALL produce bounded observable failure without fallback, broadcast, or mutation of another instrument.

#### Scenario: MIDI targets one Patch
- **WHEN** a note or controller command names a non-first Patch in a rack containing SoundFont and Braids implementations
- **THEN** only that Patch's prepared instrument receives the command and matching parameters and every untargeted instrument and stem remains unchanged

#### Scenario: Several Patches render one block
- **WHEN** multiple prepared instruments are active for a bounded render block
- **THEN** each instrument is called once with its exact Patch parameter projection and fills only the stem whose index and `PatchId` match the canonical rack order

#### Scenario: Unknown Patch is targeted
- **WHEN** a command names a Patch not present in the active rack
- **THEN** no prepared instrument receives the command, no fallback or broadcast occurs, and a fixed-size routing failure is observable

### Requirement: Graph-compatible scalar snapshots
Every prepared graph and fixed scalar parameter snapshot SHALL carry a nonzero monotonic graph revision. The renderer SHALL consume a latest scalar snapshot only when its revision, active Patch count, ordered Patch identities, fixed capacities, and descriptor-ordered instrument scalar layouts are compatible with the active graph; otherwise it SHALL retain the last compatible snapshot. Every newly active graph SHALL contain compatible initial mixer, envelope, and instrument parameters so rendering never depends on the relative arrival order of structural and scalar publications.

#### Scenario: Matching scalar snapshot arrives
- **WHEN** the latest complete parameter snapshot targets the active graph revision, Patch order, and prepared capability layouts
- **THEN** the next rendered block uses that exact accepted mixer, envelope, and instrument-scalar generation

#### Scenario: Snapshot targets another graph
- **WHEN** the latest scalar snapshot is stale, belongs to a pending graph revision, or carries an incompatible instrument scalar layout
- **THEN** the renderer does not consume it and continues with the active graph's last compatible complete snapshot

#### Scenario: Graph activates before scalar publication
- **WHEN** a replacement graph becomes active before its separately published latest scalar snapshot is visible
- **THEN** the first block uses the complete compatible initial snapshot embedded during graph preparation

### Requirement: Prepared-rack Phase 2 boundary
This increment SHALL expose HiDef SoundFont and Braids together as the two production instrument capabilities and preparers. Replacement graphs SHALL retain the accepted PatchId and capability layout, and the application SHALL NOT expose engine selection, a PATCH page, user-triggered structural edits, modulation, layering, per-Patch effects, or fallback.

#### Scenario: Production application starts after the rack migration
- **WHEN** the normal, smoke, headless-demo, or live-demo path is launched
- **THEN** the generic prepared rack hosts alternating SoundFont and Braids Patches with exact isolated stems and no unavailable structural control is presented

## ADDED Requirements

### Requirement: Production mixed-engine rack proof
The named prepared-engine-rack acceptance SHALL include real SoundFont and real Braids preparers in one production graph and SHALL prove both implementation types respond to exact targeted MIDI, consume only matching scalar/envelope values, produce distinct nonzero finite stems, and retain zero callback allocation/destruction under bounded load.

#### Scenario: Mixed production graph renders
- **WHEN** one SoundFont Patch and one Braids Patch are prepared, sounded, edited, and rendered together
- **THEN** each engine's isolated stem and parameter consequences are measured before the acceptance marker is emitted

#### Scenario: Several Braids Patches share one rack
- **WHEN** `N` Braids Patches are admitted alongside SoundFont within the rack's fixed Patch capacity
- **THEN** the rack owns `N` separate prepared Braids instruments with `16 × N` total voices, including forty-eight for three Braids Patches, and routes commands, parameters, stealing, rendering, and all-notes-off without a global Braids voice pool or a Braids-specific Patch-count limit
