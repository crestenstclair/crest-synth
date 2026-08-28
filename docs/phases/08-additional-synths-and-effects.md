# Phase 08 — Additional Synthesizers and Effects

Status: Deferred content expansion

## Outcome

Crest Synth gains additional open-source synthesizer engines and effects
through its existing capability and registry architecture.

## Scope

- Evaluate candidates individually for license, build, platform, real-time,
  memory, parameter-schema, and maintenance fit.
- Integrate accepted synthesizers behind instrument capability providers and
  preparers.
- Integrate accepted effects behind effect capability providers and preparers.
- Expose installed content through registries and the existing generic UI.
- Preserve typed failures and prohibit silent fallback or substitution.
- Prove bounded callback behavior and complete off-callback preparation for
  every added capability.

## Out of scope

- Treating Figma fixture engines or effects as an exhaustive production list.
- Hard-coding capability-specific product UI when descriptors can express the
  required controls.
- Selecting candidates before a dedicated research pass.

## Current decision

Additional synthesizers and effects are desirable future breadth, not a missing
foundation required before the current product workflows are complete.

## Completion signals

Defined per capability. Each accepted engine or effect must satisfy the same
production state, preparation, graph, rendering, error, and evidence standards
as existing capabilities.
