# Decision Moment `01KYNPJHVFZ0Y9EDEHCQ7CHHYK`

- **Mission:** `expandable-effects-and-bus-topology-01KYNGX8`
- **Origin flow:** `plan`
- **Slot key:** `plan.mixer.send-addressing`
- **Input key:** `send_addressing`
- **Status:** `resolved`
- **Created:** `2026-07-29T01:07:11.344038+00:00`
- **Resolved:** `2026-07-29T01:14:45.303488+00:00`
- **Opened by:** `cli`
- **Other answer:** `true`

## Question

How should the 8 per-track sends be addressed now that MixerTrackParameter has hardcoded ReverbSend and DelaySend variants?

## Options

- parameterized-send-variant
- flat-send-variants
- separate-send-address
- Other

## Final answer

Polymorphic generic addressing, not enumeration. Nothing in the mixer track is hardcoded per effect: a track owns sends as a fixed array of one generic send value type addressed by BusId. Parameter identity is descriptor-driven and index-addressed rather than a named enum variant per destination. Same rule applies to Patch effect slots and bus returns.

## Rationale

_(none)_

## Change log

- `2026-07-29T01:07:11.344038+00:00` — opened
- `2026-07-29T01:14:45.303488+00:00` — resolved (final_answer="Polymorphic generic addressing, not enumeration. Nothing in the mixer track is hardcoded per effect: a track owns sends as a fixed array of one generic send value type addressed by BusId. Parameter identity is descriptor-driven and index-addressed rather than a named enum variant per destination. Same rule applies to Patch effect slots and bus returns.")
