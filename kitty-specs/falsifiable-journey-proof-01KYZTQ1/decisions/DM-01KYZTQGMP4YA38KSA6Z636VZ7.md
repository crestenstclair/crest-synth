# Decision Moment `01KYZTQGMP4YA38KSA6Z636VZ7`

- **Mission:** `falsifiable-journey-proof-01KYZTQ1`
- **Origin flow:** `specify`
- **Slot key:** `specify.risk1.closure-shape`
- **Input key:** `risk1_closure_shape`
- **Status:** `resolved`
- **Created:** `2026-08-01T23:32:12.566715+00:00`
- **Resolved:** `2026-08-01T23:32:13.346030+00:00`
- **Opened by:** `cli`
- **Other answer:** `false`

## Question

How should RISK-1 be closed - binding an effect's slot_id to its position?

## Options

- Stamp from position
- Validate and reject
- Remove slot_id entirely

## Final answer

Stamp from position: set_slot_occupancy derives the occupant's slot_id from index.instance_identity() rather than accepting one, making the mismatch inexpressible at the sole chokepoint. Serialized slotId vocabulary and the real-time path stay untouched.

## Rationale

_(none)_

## Change log

- `2026-08-01T23:32:12.566715+00:00` — opened
- `2026-08-01T23:32:13.346030+00:00` — resolved (final_answer="Stamp from position: set_slot_occupancy derives the occupant's slot_id from index.instance_identity() rather than accepting one, making the mismatch inexpressible at the sole chokepoint. Serialized slotId vocabulary and the real-time path stay untouched.")
