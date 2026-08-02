# Phase 0 Research: Falsifiable Journey Proof

No `[NEEDS CLARIFICATION]` markers survived specification; `spec-kitty agent decision verify`
reports clean. This document records the three investigations that actually changed a design
decision, and one correction they forced on the predecessor mission's record.

---

## R-001 — Does deriving slot identity from position contradict the declared model?

**Decision**: No. Stamp the position's derived identity onto an installed occupant
(`set_slot_occupancy`), making a mismatched identity inexpressible.

**Why this was in doubt**: `contexts/synth.yaml` declares that `EffectSlotIndex` "names a
position, not an instance; it identifies which of the three ordered slots is addressed while
`EffectSlotId` identifies the configured instance currently occupying one." Read alone, that
separation suggests an instance identity independent of position — which stamping would
collapse. The `/spec-kitty.crest-spec` phase forbids silently editing a declaration to permit
already-planned work, so this had to be resolved before authoring, not after.

**Investigation**:

1. `SemanticAction` (`src/control/semantic_action.rs:54-70`) has exactly 8 variants. Occupancy
   is expressed as `SetSlotOccupancy { patch_id, slot, entry }` — *which registry entry occupies
   a position*. There is no move, swap, or exchange variant.
2. The capability's `ordered_patch_effect_chain` acceptance says "occupy all three slots on one
   Patch and exchange two of them." With no move operation in the union, an exchange is
   performed as two occupancy sets. No instance object is relocated.
3. `EffectSlotIndex::instance_identity()` (`src/synth/effect_slot_id.rs:98-101`) already
   documents the intended contract verbatim: "the stable instance identity an occupancy change
   derives for this position: positions map one-to-one onto non-zero slot ids (position + 1),
   exactly as bus returns derive theirs, so the identity is deterministic and unique per position
   **by construction**."
4. Production already relies on it — `app_state.rs:1202,1281` construct occupants via
   `default_config(slot.instance_identity())`, and `standalone_application.rs:1516` recovers a
   position by matching `instance_identity() == config.slot_id()`.

**Conclusion**: the declared separation is about *addressing* (a position is not an instance),
not about independent provenance of the identity value. Since instances are never relocated,
"the instance currently occupying a position" and "the identity derived from that position"
denote the same thing. Stamping makes the code match its own documented contract; today
`set_slot_occupancy` merely *trusts* a caller-supplied identity and checks uniqueness.

**Alternatives considered**:
- *Validate and reject* (add a `MismatchedSlotId` error): smallest diff, but leaves the wrong
  value constructible and closes the path rather than the class. Rejected under DIRECTIVE_043.
- *Remove `slot_id` from `PostEffectConfig` entirely*: strongest guarantee, but reaches into
  real-time snapshot construction and the 10+ serialized `slotId` paths the predecessor mission
  froze. Rejected under spec C-003 and DIRECTIVE_024.

**Action taken**: the crest-spec wording was sharpened (`ad9960b`) so the next reader does not
have to re-derive this.

---

## R-002 — Should the permitted-injection assertion be "exactly one" or "at most one"?

**Decision**: At most one — `directInjectionsRecorded: lte 1`.

**Rationale**: the documented rejection is the only *permitted* direct injection because the UI
cannot express an unknown registry entry. It is not a *required* one. `eq 1` would encode
today's limitation as a rule and silently fail a future scene that expresses the rejection by
gesture — locking in a constraint that nothing actually requires.

**Alternatives considered**: `eq 1` mirrors the current test's
`assert_eq!(injected, ["Topology.refused"])` most literally. Rejected: a predicate should
forbid what is wrong, not freeze what is currently true.

---

## R-003 — What scope should the acceptance-record coverage gate scan?

**Decision**: the mission being accepted, only.

**Rationale**: the spec's aim is that the omission "is not repeatable" — that is a
forward-looking property. Scanning siblings would make this mission's acceptance depend on
backfilling another mission's record, which it was not chartered to do.

**Investigation** (see R-004 — this is where the predecessor's record turned out to be
misdescribed): a sibling-scanning gate would fail today on
`expandable-effects-and-bus-topology-01KYNGX8`, which grades 1 constraint row against roughly
eleven declared. That is a real gap, but it is the parent's, and folding it in here would grow
scope on a mission about proof adequacy.

**Alternatives considered**:
- *All non-archived missions + backfill*: complete, but imports unbudgeted work.
- *All missions with a grandfather allowlist*: no backfill, but an allowlist is a silence
  mechanism — exactly the shape that lets a future author opt out instead of grading.

---

## R-004 — Correction forced on the predecessor's record

While grounding R-003, counting the graded rows in both acceptance matrices disproved a finding
this mission's charter inherited.

| Mission | Graded rows | Breakdown |
|---|---|---|
| `expandable-effects-and-bus-topology-01KYNGX8` | 43 | FR 21, **NFR 10**, SC 11, C 1 |
| `demo-journey-fidelity-and-hygiene-01KYWVYG` | 16 | FR 16, NFR 0, C 0 |

The predecessor's review and retrospective claimed both missions "produced acceptance matrices
covering functional requirements only" and concluded that a repeat across two authors "indicts
the artifact template, not the author." The parent graded its NFRs. **The NFR omission belongs
to `demo-journey` alone.** What genuinely recurs is constraint under-grading — which is what the
parent's own review sentence, quoted in that finding, actually said.

The remediation is unaffected: 12 of 28 declared rows went ungraded in `demo-journey` and
nothing caught it. Only the recurrence reasoning was wrong, and it mattered enough to correct
because it misassigns cause from author to template.

Corrected in commit `25444d7`, with the correction marked rather than silently rewritten:
`mission-review.md` DRIFT-3 carries a dated correction block, and `retrospective.yaml` n-007 was
rewritten with the measured counts and a new evidence reference (e-025).

**Method note**: the original claim was filed without counting the parent's rows. Filing a
recurrence claim without checking the second instance is the same class of error — a claim
outrunning its evidence — that this mission line exists to close. It is recorded here rather
than quietly fixed.
