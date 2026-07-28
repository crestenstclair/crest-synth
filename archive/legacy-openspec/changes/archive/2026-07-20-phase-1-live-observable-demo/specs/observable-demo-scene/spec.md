## ADDED Requirements

### Requirement: Headless demo remains an independent deterministic proof
Adding the live observable demo SHALL NOT change the `make demo` command, its headless/no-device/no-window execution, deterministic timing, two-run equality, structured output markers, expected coverage universe, mutation cases, or acceptance predicates.

#### Scenario: Existing headless demo runs after live mode is added
- **WHEN** the maintainer runs `make demo`
- **THEN** the existing exhaustive scene produces its current deterministic event log, state tree, observation, exact coverage, and controlled-negative behavior without opening a native window or physical device

#### Scenario: Live-only observation fields are introduced
- **WHEN** live checkpoints or callback-to-control observations are added to the implementation
- **THEN** they do not expand, satisfy, or alter the headless demo's production-derived coverage universe or completed report schema

### Requirement: Existing verification gates remain required
Phase 1 completion SHALL require the existing exhaustive-demo, schema-surface, GUI-context, mutation, real-time, smoke, format, lint, and all-target checks in addition to the new live-demo contract test.

#### Scenario: Live behavior passes but an existing gate fails
- **WHEN** the live-demo test passes and any existing required headless or project check fails
- **THEN** the Phase 1 change is incomplete and cannot be accepted
