# Decision Moment `01KYZTQV8MSZXP5S604GM9QFHW`

- **Mission:** `falsifiable-journey-proof-01KYZTQ1`
- **Origin flow:** `specify`
- **Slot key:** `specify.acceptance-matrix.omission-fix`
- **Input key:** `acceptance_matrix_omission_fix`
- **Status:** `resolved`
- **Created:** `2026-08-01T23:32:23.444277+00:00`
- **Resolved:** `2026-08-01T23:32:24.194401+00:00`
- **Opened by:** `cli`
- **Other answer:** `false`

## Question

How should the acceptance-matrix NFR/constraint omission be fixed?

## Options

- Declared validation gate
- Fix the template only

## Final answer

Declared validation gate: a crest-spec validation fails acceptance when a mission acceptance matrix lacks NFR and constraint rows, so the omission that occurred in both this mission line's prior missions cannot recur. Precedent: asset.ValidationScripts declared by the parent mission.

## Rationale

_(none)_

## Change log

- `2026-08-01T23:32:23.444277+00:00` — opened
- `2026-08-01T23:32:24.194401+00:00` — resolved (final_answer="Declared validation gate: a crest-spec validation fails acceptance when a mission acceptance matrix lacks NFR and constraint rows, so the omission that occurred in both this mission line's prior missions cannot recur. Precedent: asset.ValidationScripts declared by the parent mission.")
