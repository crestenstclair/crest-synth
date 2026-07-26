## ADDED Requirements

### Requirement: Bounded complete engine-selection graph replacement
Engine selection SHALL use one injected capacity-one nonblocking worker and one complete structural handoff. A candidate SHALL preserve ordered Patch identities, device and routing capacities, and every untargeted config while permitting only the selected capability and scalar layout to change on a newer revision. Commit SHALL precede publication; Ready SHALL follow exact activation and off-callback retirement, and all failures SHALL preserve ownership without fallback.

#### Scenario: Selection request is submitted
- **WHEN** the reducer accepts one request and no structural work is pending
- **THEN** the exact immutable candidate and runtime inputs are submitted once without blocking and the active graph continues rendering

#### Scenario: A second request arrives before completion
- **WHEN** one request is preparing, staged, published, activating, or awaiting retirement collection
- **THEN** the second request is rejected as busy rather than queued, overwritten, merged, or sent to another preparer

#### Scenario: Worker preparation fails
- **WHEN** validation, capability matching, asset loading, preparation, graph construction, or worker delivery fails
- **THEN** candidate ownership is destroyed outside the callback, no graph reaches structural input, and the old graph remains active without fallback

#### Scenario: Selected slot changes capability layout
- **WHEN** a complete candidate replaces one SoundFont or Braids slot with the other validated default config
- **THEN** the same ordered PatchId and route own the new prepared implementation and scalar layout while every other slot, config, capacity, and route remains exact

#### Scenario: Scalar edits occur during preparation
- **WHEN** mixer, envelope, or scalar edits are accepted before candidate publication
- **THEN** the graph's initial snapshot is refreshed from the exact committed completion generation and compatible target revision so activation cannot revert them

#### Scenario: Candidate differs outside the selected layout
- **WHEN** Patch identity/order/count, callback bounds, route, request metadata, device configuration, or any untargeted config differs
- **THEN** control rejects the candidate, preserves or destroys it off callback, and leaves the active graph unchanged

#### Scenario: Prepared boundary is momentarily full
- **WHEN** the target config has committed but graph ownership cannot immediately cross the queue
- **THEN** control retains exactly one staged graph and retries while status remains Activating without rollback, drop, second request, or fallback

#### Scenario: Replacement completes
- **WHEN** audio activates the candidate, returns the source graph, and control collects the exact retired revision
- **THEN** one correlated acknowledgement marks Ready, adopts the new required layout, and admits the next request

#### Scenario: Named acceptance is measured
- **WHEN** `engine_selection_workflow` runs both real directions plus controlled failure, busy, and stale cases through production reducer, providers, preparers, graph builder, handoff, renderer, and observation
- **THEN** success is emitted only after target-only mutation, finite nonzero target output, compatible snapshots, zero callback allocation/destruction, exact acknowledgements, and off-callback graph collection pass

## REMOVED Requirements

### Requirement: Prepared-rack Phase 2 boundary
This increment SHALL expose HiDef SoundFont and Braids together as the two production instrument capabilities and preparers. Replacement graphs SHALL retain the accepted PatchId and capability layout, and the application SHALL NOT expose engine selection, a PATCH page, user-triggered structural edits, modulation, layering, per-Patch effects, or fallback.

#### Scenario: Production application starts after the rack migration
- **WHEN** the normal, smoke, headless-demo, or live-demo path is launched
- **THEN** the generic prepared rack hosts alternating SoundFont and Braids Patches with exact isolated stems and no unavailable structural control is presented

**Reason**: The prepared rack and read-only PATCH projection are complete; Phase 3 now admits one selected capability-layout change.

**Migration**: Preserve stable Patch identities, capacities, complete graph ownership, one-in-flight acknowledgement, and no fallback while allowing only the selected Patch's capability and scalar layout to change.
