## ADDED Requirements

### Requirement: Correlated asynchronous Patch engine selection
The focused PATCH engine row SHALL replace only the adjacent installed capability through semantic events, `AppState::apply`, a capacity-one off-callback worker, and the complete structural handoff. The source SHALL remain active until a descriptor-default target commits and activates. Every outcome SHALL be correlated and typed; Ready SHALL require off-callback retirement collection, and no path SHALL choose a fallback.

#### Scenario: Request begins while the source remains active
- **WHEN** Edit+Left/Right targets an adjacent installed engine while status is Ready or recoverable Failed
- **THEN** the reducer records a monotonic request and Preparing, emits one preparation effect, and leaves the active config and graph revision exact while the source graph remains audible

#### Scenario: Prepared target commits and activates
- **WHEN** the worker returns a complete candidate matching the request, Patch, capabilities, config, revisions, Patch layout, and device bounds
- **THEN** the candidate commits through `AppState::apply`, the exact target-revision snapshot is published before graph ownership, audio swaps only at a block boundary, and Ready follows exact retirement collection

#### Scenario: Structural publication is temporarily full
- **WHEN** the target config has committed but the prepared graph queue cannot accept ownership
- **THEN** control retains exactly one staged graph, status remains Activating, later ticks retry, and no rollback, drop, second request, or fallback occurs

#### Scenario: Preparation or correlation fails
- **WHEN** preparation fails or a result or acknowledgement is busy, early, stale, duplicate, or mismatched
- **THEN** the applicable transition records visible source-preserving failure or rejects unchanged, no invalid graph or fallback is published, and every rejected candidate is destroyed outside the callback

#### Scenario: Headless proof completes both directions
- **WHEN** the deterministic scene and named workflow run SoundFont to Braids to descriptor-default SoundFont through production providers, preparers, reducer, graph builder, handoff, renderer, observation, and retirement
- **THEN** they prove pending source audio, target-only mutation, finite nonzero target-stem output, failure preservation, busy and stale rejection, compatible snapshots, zero callback allocation or destruction, no fallback, and byte-identical fresh-run evidence before emitting success

#### Scenario: Paced live proof completes both directions
- **WHEN** `make demo-live` requests both adjacent choices through semantic events and the production threaded worker
- **THEN** Preparing, Activating, and Ready are visible on increasing acknowledged revisions, targeted physical output is finite and nonzero for each target, cleanup is semantic, and the final first-Patch config is descriptor-default HiDef SoundFont

#### Scenario: Claimed success is not the target engine
- **WHEN** preparation or publication is missing, the old or unrelated engine supplies measured output, output is silent or non-finite, a fallback appears, or callback ownership is violated
- **THEN** the structured observation fails and neither the named acceptance marker nor a complete demo report is emitted
