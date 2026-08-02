# Decision Moment `01KYNPJJMKR8PY4H9N4FTZ5K2V`

- **Mission:** `expandable-effects-and-bus-topology-01KYNGX8`
- **Origin flow:** `plan`
- **Slot key:** `plan.mixer.global-parameters-fate`
- **Input key:** `global_parameters_fate`
- **Status:** `resolved`
- **Created:** `2026-07-29T01:07:12.147448+00:00`
- **Resolved:** `2026-07-29T01:14:46.000387+00:00`
- **Opened by:** `cli`
- **Other answer:** `false`

## Question

What happens to the reverb and delay fields inside GlobalParameters once those effects live on bus returns?

## Options

- dissolve-into-return-scalars
- keep-globals-add-returns
- Other

## Final answer

dissolve-into-return-scalars: GlobalParameter retains only MasterGainDb. Reverb/delay tuning values become descriptor scalars of their registry entries; return level is owned per bus return.

## Rationale

_(none)_

## Change log

- `2026-07-29T01:07:12.147448+00:00` — opened
- `2026-07-29T01:14:46.000387+00:00` — resolved (final_answer="dissolve-into-return-scalars: GlobalParameter retains only MasterGainDb. Reverb/delay tuning values become descriptor scalars of their registry entries; return level is owned per bus return.")
