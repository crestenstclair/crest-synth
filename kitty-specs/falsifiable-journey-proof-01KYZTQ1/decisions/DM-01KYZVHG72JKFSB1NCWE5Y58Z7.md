# Decision Moment `01KYZVHG72JKFSB1NCWE5Y58Z7`

- **Mission:** `falsifiable-journey-proof-01KYZTQ1`
- **Origin flow:** `plan`
- **Slot key:** `plan.falsification.evidence-form`
- **Input key:** `falsification_evidence_form`
- **Status:** `resolved`
- **Created:** `2026-08-01T23:46:24.098268+00:00`
- **Resolved:** `2026-08-01T23:56:43.751362+00:00`
- **Opened by:** `cli`
- **Other answer:** `false`

## Question

How should falsification evidence be recorded?

## Options

- Committed artifact per guard
- WP notes only

## Final answer

Committed artifact per guard under evidence/falsification/: each records the mutation applied, the observed failure with command and exit code, the restoration, and the observed pass. Makes 'a guard with only one recorded outcome is unproven' independently checkable rather than self-reported prose.

## Rationale

_(none)_

## Change log

- `2026-08-01T23:46:24.098268+00:00` — opened
- `2026-08-01T23:56:43.751362+00:00` — resolved (final_answer="Committed artifact per guard under evidence/falsification/: each records the mutation applied, the observed failure with command and exit code, the restoration, and the observed pass. Makes 'a guard with only one recorded outcome is unproven' independently checkable rather than self-reported prose.")
