# Phase 03 — Figma Visual Fidelity

Status: Planned after Phase 02

## Outcome

The production UI advances from a functional blockout to a faithful
implementation of the normative Figma product across supported window sizes.

## Scope

- Complete the remaining Sample Detail and Sample Browser visual slices.
- Complete the Mixer responsive composition.
- Complete Engine and Post FX option visual gaps.
- Verify native scaling, typography, spacing, hierarchy, focus, status, and
  control-state presentation.
- Compare production native captures directly with the relevant Figma frames.
- Preserve semantic focus and interaction behavior across responsive density
  changes.
- Finish cohesive visual polish only after the named surfaces match their
  authored compositions.

## Out of scope

- Using visual work to redefine product behavior.
- New synthesizers, effects, session capabilities, or input devices.
- Multi-platform release work.
- Treating component presence or reducer/projection tests as visual acceptance.

## Completion signals

- Each remaining surface has a scoped native comparison against its normative
  Figma frame.
- The continuous supported width range has been exercised, not only named
  viewport snapshots.
- Text and shape communicate status and control state in addition to color.
- Responsive reflow never dispatches a semantic action or changes focus
  identity.
- `DESIGN.md` records measured as-built completion without claiming more than
  the evidence proves.

## Dependency

Follows the two workflow phases so final visual acceptance covers the intended
session and Patch behavior rather than the current fixture-driven blockout.

