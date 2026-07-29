# Decision Moment `01KYNPJG8V4WVGJTRS393EPF4P`

- **Mission:** `expandable-effects-and-bus-topology-01KYNGX8`
- **Origin flow:** `plan`
- **Slot key:** `plan.realtime.snapshot-growth`
- **Input key:** `snapshot_growth`
- **Status:** `resolved`
- **Created:** `2026-07-29T01:07:09.723607+00:00`
- **Resolved:** `2026-07-29T01:14:43.921159+00:00`
- **Opened by:** `cli`
- **Other answer:** `false`

## Question

How should the fixed real-time parameter snapshot absorb 3 effect slots per Patch, 8 bus returns, and 16x8 track sends?

## Options

- grow-monolithic
- split-transports
- indexed-sparse
- Other

## Final answer

grow-monolithic: keep one fixed latest-value ParameterSnapshot and widen it to patches[16] x effects[3] x scalars[8], tracks[16] with sends[8], and returns[8]. One transport, one revision, fixed layout preserved.

## Rationale

_(none)_

## Change log

- `2026-07-29T01:07:09.723607+00:00` — opened
- `2026-07-29T01:14:43.921159+00:00` — resolved (final_answer="grow-monolithic: keep one fixed latest-value ParameterSnapshot and widen it to patches[16] x effects[3] x scalars[8], tracks[16] with sends[8], and returns[8]. One transport, one revision, fixed layout preserved.")
