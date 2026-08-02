# Decision Moment `01KYNPJGZE4SHHASMVDCY2ET42`

- **Mission:** `expandable-effects-and-bus-topology-01KYNGX8`
- **Origin flow:** `plan`
- **Slot key:** `plan.mixer.global-effects-port-fate`
- **Input key:** `global_effects_port_fate`
- **Status:** `resolved`
- **Created:** `2026-07-29T01:07:10.446754+00:00`
- **Resolved:** `2026-07-29T01:14:44.605916+00:00`
- **Opened by:** `cli`
- **Other answer:** `false`

## Question

What happens to port.Mixer.GlobalEffectsProcessor when reverb and delay become registry effects on bus returns?

## Options

- retire-port
- widen-port
- Other

## Final answer

retire-port: delete port.Mixer.GlobalEffectsProcessor entirely. Reverb and delay become ordinary registry effect capabilities and preparers using the same generic prepared-effect boundary as Chorus. Rationale from user: the port was closed against extension that was always declared, violating open-closed; one effect model must exist, not two.

## Rationale

_(none)_

## Change log

- `2026-07-29T01:07:10.446754+00:00` — opened
- `2026-07-29T01:14:44.605916+00:00` — resolved (final_answer="retire-port: delete port.Mixer.GlobalEffectsProcessor entirely. Reverb and delay become ordinary registry effect capabilities and preparers using the same generic prepared-effect boundary as Chorus. Rationale from user: the port was closed against extension that was always declared, violating open-closed; one effect model must exist, not two.")
