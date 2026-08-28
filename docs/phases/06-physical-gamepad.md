# Phase 06 — Physical Gamepad Integration

Status: Deferred

## Outcome

Physical gamepad input drives Crest Synth's existing controller-first semantic
interaction model.

## Scope

- Add a production physical-gamepad input adapter.
- Normalize device input into the existing semantic controller vocabulary.
- Preserve the physical input → semantic action/event → `AppState::apply` →
  view/audio projection invariant.
- Verify connection, disconnection, repeat behavior, and the supported control
  mappings on target hardware.

## Out of scope

- Changing the product interaction model to match a specific controller.
- Multi-select behavior for the reserved Select button.
- Emulator or unrelated controller configuration.

## Current decision

This is intentionally deferred. Keyboard and normalized controller paths can
continue supporting development and acceptance of the preceding phases.

## Completion signals

- A connected supported controller can operate the complete semantic workflow
  assigned to its buttons and D-pad.
- Disconnects and unsupported mappings fail visibly without corrupting product
  state.
- Physical acceptance complements, rather than replaces, deterministic
  semantic input coverage.
