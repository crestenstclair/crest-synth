# Phase 04 — Page Navigation

Status: Planned after Phase 03

## Outcome

Page-level navigation follows the normative Figma Page Layout: Patch Overview
is the default hub, Detail is subordinate to its highlighted subject, Mixer is
the other top-level context, and Settings is a temporary system surface.

## Figma source

- Page: `05 · Page Layout`
- Frame: [Crest Synth · Page Layout — 153:184](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=153-184)

Figma remains authoritative for the page hierarchy, connections, labels, and
input meanings summarized here.

## Page hierarchy

- Patch Overview is the default main page in the PATCH context.
- Detail opens for the Instrument or effect slot highlighted in Patch Overview.
- Detail returns to the exact Patch Overview origin.
- Mixer is the MIXER top-level context; PATCH and MIXER retain their semantic
  roots.
- Settings · MIDI Devices is a temporary system surface entered without losing
  the preserved performance identity.
- Page movement is independent from in-page focus movement and editing.

## Mapped page inputs

- `Q`: previous Patch while remaining in Patch Overview.
- `E`: next Patch while remaining in Patch Overview.
- `Shift + Up`: Patch Overview → highlighted Detail; Mixer → Patch Overview.
- `Shift + Down`: Detail → Patch Overview; Patch Overview → Mixer.
- `Shift + Left`: Patch Overview → Settings · MIDI Devices.

Physical inputs must normalize to semantic page actions before product logic
and continue through `AppState::apply` before view or audio projection.

## Unresolved Figma boundary

The Page Layout currently marks the return input from Settings as unmapped.
This phase must not invent that binding. The Figma contract should define it
before page-navigation implementation is considered complete.

## Out of scope

- In-page focus movement and parameter editing.
- Physical gamepad integration.
- Multi-select.
- Multi-platform release work.
- New synthesizer or effect capabilities.

## Completion signals

- Every mapped Page Layout connection is reachable through the production
  semantic action and reducer path.
- Patch cycling remains in Patch Overview and preserves stable Patch identity.
- Detail entry follows the highlighted subject and return restores its exact
  valid origin or the documented deterministic repair.
- PATCH and MIXER retain their prior semantic roots across context movement.
- Settings entry suspends the exact performance identity, and its return
  behavior matches the completed Figma contract.
- Page transitions do not masquerade as or alter in-page focus movement.
- Deterministic and native acceptance evidence covers the authored page graph.

## Dependency

Follows Phase 03 so the navigation proof exercises the intended Figma-faithful
surfaces.
