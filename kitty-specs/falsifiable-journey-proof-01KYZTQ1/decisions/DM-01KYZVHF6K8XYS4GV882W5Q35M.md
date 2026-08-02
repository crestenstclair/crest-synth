# Decision Moment `01KYZVHF6K8XYS4GV882W5Q35M`

- **Mission:** `falsifiable-journey-proof-01KYZTQ1`
- **Origin flow:** `plan`
- **Slot key:** `plan.coverage-gate.scan-scope`
- **Input key:** `coverage_gate_scan_scope`
- **Status:** `resolved`
- **Created:** `2026-08-01T23:46:23.059960+00:00`
- **Resolved:** `2026-08-01T23:56:42.874453+00:00`
- **Opened by:** `cli`
- **Other answer:** `false`

## Question

Which missions should the acceptance-matrix coverage gate scan?

## Options

- Current mission only
- All non-archived missions with backfill
- All missions, grandfather existing

## Final answer

Current mission only: the coverage gate grades the mission being accepted, preventing recurrence going forward without reopening archived history or forcing a backfill of the parent's constraint rows. Scope stays exactly the spec's 'so the omission is not repeatable'; the prior gaps remain documented in the parent review.

## Rationale

_(none)_

## Change log

- `2026-08-01T23:46:23.059960+00:00` — opened
- `2026-08-01T23:56:42.874453+00:00` — resolved (final_answer="Current mission only: the coverage gate grades the mission being accepted, preventing recurrence going forward without reopening archived history or forcing a backfill of the parent's constraint rows. Scope stays exactly the spec's 'so the omission is not repeatable'; the prior gaps remain documented in the parent review.")
