# PARKED 2026-08-02 — before implementation

This mission is proof-about-proof. Its two real findings (the journey guard reads
the scene's declaration instead of the dispatched event; the occupant-edit
criterion is satisfied by the ambient probe note) are genuine and worth ~90
minutes of work inside a future mission that also delivers product.

It is parked because it delivers no synth. Phase 3's behaviors work on hardware.
The next thing the player actually needs is the ability to change which Patch
they are editing and playing — the gap the operator named directly.

Salvage when resumed: WP01 + WP02 only. WP03 (slot-identity stamping) is a real
one-line hardening that can ride along with any mission touching src/synth/patch.rs.
WP04-WP06 were ceremony scaffolding around those.

Do not resume this as a standalone mission.

---

## Closed 2026-08-02

Archived as `canceled` (all six work packages moved `planned` → `canceled`;
none was ever implemented). `target_branch` was repointed from
`feat/expandable-effects-and-bus-topology` — merged to main and deleted — to
the live integration branch, so the record resolves against a branch that
exists.

**WP04 is dead, not salvage.** Its deliverable was the acceptance-record
coverage gate. That shipped on 2026-08-02 as
`scripts/check_acceptance_matrix_coverage.sh`, written during the
crest-component-foundations accept gate because the crest-spec had declared
`validation.acceptance_matrix_covers_all_requirement_kinds` while the script
behind it never existed — so the validation had only ever exited 127, which
reads as "not passing" to a human and as nothing at all to a CI check that
only inspects the exit code of a missing file.

**Salvage, unchanged from above:** WP01 and WP02 — the journey guard reads the
scene's declaration instead of the dispatched event, and the occupant-edit
criterion is satisfied by the ambient probe note. WP03's slot-identity stamping
is a one-line hardening that can ride along with any mission touching
`src/synth/patch.rs`. Carry these into a future mission that also delivers
product; do not resurrect this mission to hold them.
