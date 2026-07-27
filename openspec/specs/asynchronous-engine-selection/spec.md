# asynchronous-engine-selection Specification

## Purpose
TBD - created by archiving change phase-3-asynchronous-engine-selection. Update Purpose after archive.
## Requirements
### Requirement: Correlated asynchronous Patch engine selection
The focused PATCH Engine row SHALL replace only the adjacent installed capability through semantic events, `AppState::apply`, a capacity-one off-callback worker, and the complete structural handoff. The source SHALL remain active until a descriptor-default target commits and activates. Every outcome SHALL be correlated and typed; Ready SHALL require off-callback retirement collection, and no path SHALL choose a fallback. Engine lifecycle transitions SHALL preserve PATCH control focus and the canonical Patch envelope. PATCH focus navigation and ADSR scalar edits SHALL remain valid while structural work is Preparing or Activating, but another Engine adjustment SHALL remain busy.

#### Scenario: Request begins while the source remains active
- **WHEN** Edit+Left/Right targets an adjacent installed engine while Engine is focused and status is Ready or recoverable Failed
- **THEN** the reducer records a monotonic request and Preparing, emits one preparation effect, and leaves the active config, graph revision, canonical envelope, and control focus exact while the source graph remains audible

#### Scenario: Engine adjustment is attempted from an ADSR row
- **WHEN** Edit plus a direction is received while an ADSR row rather than Engine is focused
- **THEN** the event is resolved only as that envelope row's bounded scalar adjustment and starts no engine request

#### Scenario: ADSR changes during preparation
- **WHEN** a valid PATCH ADSR adjustment is accepted while an engine request is Preparing
- **THEN** the canonical value is published against the active source revision, the audible source can consume it, no second structural request is emitted, and the prepared candidate is refreshed from the latest committed snapshot before graph ownership is published

#### Scenario: Prepared target commits and activates
- **WHEN** the worker returns a complete candidate matching the request, Patch, capabilities, config, revisions, Patch layout, and device bounds
- **THEN** the candidate commits through `AppState::apply`, the exact latest target-revision snapshot is published before graph ownership, audio swaps only at a block boundary, PATCH focus and envelope remain exact, and Ready follows exact retirement collection

#### Scenario: ADSR changes during activation
- **WHEN** a valid PATCH ADSR adjustment is accepted after candidate commit while status is Activating
- **THEN** the canonical value is published against the target revision for the replacement graph to consume on activation, the source stays audible with its last compatible source-revision snapshot, and no rollback or additional graph is created

#### Scenario: Structural publication is temporarily full
- **WHEN** the target config has committed but the prepared graph queue cannot accept ownership
- **THEN** control retains exactly one staged graph, status remains Activating, PATCH ADSR edits continue to target its revision, later ticks retry, and no rollback, drop, second request, or fallback occurs

#### Scenario: Preparation or correlation fails
- **WHEN** preparation fails or a result or acknowledgement is busy, early, stale, duplicate, or mismatched
- **THEN** the applicable transition records visible source-preserving failure or rejects unchanged, preserves focus and envelope, publishes no invalid graph or fallback, destroys every rejected candidate outside the callback, and leaves valid ADSR editing available

#### Scenario: Headless proof completes both directions
- **WHEN** the deterministic scene and named workflow run SoundFont to Braids to descriptor-default SoundFont through production providers, preparers, reducer, graph builder, handoff, renderer, observation, and retirement
- **THEN** they prove pending source audio, PATCH ADSR edits during preparation and activation, target-only mutation, finite nonzero target-stem output, failure preservation, busy and stale rejection, compatible latest snapshots, zero callback allocation or destruction, no fallback, and byte-identical fresh-run evidence before emitting success

#### Scenario: Paced live proof completes both directions
- **WHEN** `make demo-live` completes focused-Patch ADSR coverage, returns focus to Engine, and requests both adjacent choices through semantic events and the production threaded worker
- **THEN** Preparing, Activating, and Ready are visible on increasing acknowledged revisions, targeted physical output is finite and nonzero for each target, cleanup is semantic, and the final first-Patch config is descriptor-default HiDef SoundFont

#### Scenario: Claimed success is not the target engine
- **WHEN** preparation or publication is missing, the old or unrelated engine supplies measured target output, an accepted ADSR value is lost, output is silent or non-finite, a fallback appears, or callback ownership is violated
- **THEN** the structured observation fails and neither the named acceptance marker nor a complete demo report is emitted

