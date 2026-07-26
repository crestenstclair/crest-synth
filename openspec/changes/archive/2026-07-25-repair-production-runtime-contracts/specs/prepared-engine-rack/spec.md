## ADDED Requirements

### Requirement: Production structural handoff is injected
The standalone composition root SHALL construct and inject the structural graph boundary used by the production application. The application SHALL split and use only that injected boundary for initial graph status, replacement publication, and retirement collection and SHALL NOT construct a concrete structural adapter internally.

#### Scenario: Replaceable boundary is supplied
- **WHEN** a deterministic composition supplies a conforming structural-boundary fixture through the production constructor
- **THEN** startup splits and uses that exact fixture before rendering and no hidden structural boundary is created

### Requirement: Renderer preserves unknown-Patch routing status
The production renderer SHALL preserve an unknown-Patch dispatch as fixed-size callback-to-control observation data while leaving every prepared instrument, active-note observation, and untargeted stem unchanged. The observation SHALL identify the unknown Patch and advance a saturating failure count without fallback, broadcast, allocation, blocking, logging, or formatting.

#### Scenario: Unknown Patch reaches the renderer
- **WHEN** a bounded audio command names a Patch absent from the active parameter layout or prepared rack
- **THEN** no instrument receives the command and the next coherent audio observation reports exactly one additional routing failure and that Patch identity

#### Scenario: Known Patch follows a routing failure
- **WHEN** a later command names an installed Patch
- **THEN** it is dispatched normally to that Patch only and the prior failure does not change routing or active-note state
