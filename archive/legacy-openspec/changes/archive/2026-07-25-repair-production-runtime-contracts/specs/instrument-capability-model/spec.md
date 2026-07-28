## ADDED Requirements

### Requirement: Production capability ports are composed explicitly
The production composition root SHALL construct and inject every installed instrument capability provider and its separate preparer into the standalone application. Before fixture initialization or graph publication, the application SHALL build the immutable registry from the injected provider descriptors and SHALL require exactly one provider and exactly one preparer for every installed capability identity. Missing, duplicate, unknown, or mismatched registrations SHALL return a typed startup failure without selecting a fallback.

#### Scenario: Matching production registrations start
- **WHEN** the production constructor receives the declared providers and one identity-matched preparer for each provider
- **THEN** it accepts the composition and later graph preparation uses only those injected ports

#### Scenario: Provider or preparer registration is invalid
- **WHEN** a provider or preparer is missing, duplicated, unknown to the opposite collection, or identity-mismatched
- **THEN** construction fails with the corresponding typed error before a graph is prepared or published

#### Scenario: Concrete selection is replaced in a witness
- **WHEN** a deterministic witness supplies conforming provider and preparer fixtures through the production constructor
- **THEN** the standalone application uses those fixtures without constructing a built-in capability or preparer internally
