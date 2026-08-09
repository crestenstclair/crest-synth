---
affected_files: []
cycle_number: 2
mission_slug: functional-patch-editor-01KZERV9
reviewed_at: '2026-08-09T07:00:00Z'
reviewer_agent: reviewer-renata
verdict: approved
wp_id: WP03
---

# WP03 review — cycle 2: approved

Both cycle-1 blockers are closed and I reproduced each falsification by execution.

**Note on the record.** The file previously at this path was a frontmatter-wrapped
duplicate of the cycle-1 rejection body, `reviewer_agent: unknown`, written
2026-08-09T05:31Z — inside the window of the concurrent-writer incident recorded as
F-27. It recorded the rejection this cycle answers, not a cycle-2 verdict. The
review-artifact gate was blocking on that corrupt duplicate, which is why approval
required `--skip-review-artifact-check`. Implementer and reviewer are distinct
agents; this was not a self-approval. This file replaces the duplicate with the
genuine cycle-2 record.

## B1 — the label guard now fails on the PATCH page

Reverted `patch_page_projection.rs:212` from `descriptor.label()` to
`descriptor.name()`:

```
panicked at src/control/semantic_graphical_view_model.rs:3187:17:
soundfont PATCH Main: page output masterGainDb is labelled with the serialization key masterGainDb
```

Cycle 1's guard could not fail on that site. This one does. The guard walks all
three projections across ten fixtures — all five surfaces asserted against
`SurfaceId::ALL`, both engines, both detail subjects, master gain focused on each
of the two surfaces that host it.

**It is still not complete, and that is my error rather than the implementer's.**
See F-28: a 17-site mutation sweep catches 10 and misses 7, because
`serialization_keys()` omits capability and section identifiers. Cycle 1 diagnosed
the blindness as a fixture gap and prescribed a fixture fix; the implementer did
exactly that and falsified exactly what was asked. The prescription was attached to
a correct diagnosis and sent the fix to the wrong place. Assigned to WP05 with the
~8-line remedy and a mechanical re-falsification.

## B2 — the read-only record, corrected rather than the model widened

The implementer chose to correct the written record rather than carry
`patchInteraction` onto `SemanticControlViewModel`, reasoning that adding a field
to the aggregate mid-implementation to permit code already written is the inversion
F-07 was raised about, especially when the same declaration already carries the
fact on `sections[].parameters[]`.

The discrimination is real. Hardcoding `patch_interaction` in `detail_sections` to
each of the three variants fails, each on the row that disagrees:

| hardcoded | fails on | left / right |
|---|---|---|
| `ReadOnly` | `soundfont.preset` | `Some(ReadOnly)` / `Some(StructuralChoice)` |
| `StructuralChoice` | `soundfont.file` | `Some(StructuralChoice)` / `Some(ReadOnly)` |
| `ScalarEdit` | `soundfont.preset` | `Some(ScalarEdit)` / `Some(StructuralChoice)` |

The test asserts every projected detail row against the installed descriptor's own
declaration first, so it is not a two-row special case.

**The downstream consequence is not what cycle 1 said, and F-19 needed a second
correction.** `patchInteraction` produces the read-only *fact*; it does not produce
the read-only surface-summary *control kind*. `SemanticControlKind::Surface` has one
construction site, called only from a projection builder whose own documentation
says production never uses it. WP05's bullet 7 is satisfiable; bullet 8 was not, and
the claim has since been withdrawn from the spec with its reasoning.

## F-25 — the footer, fixed at the composition

Both falsifications reproduced verbatim. The Debug form leaves the set check green
and only the derivation test fires, which confirms the implementer's reason for
adding the second guard:

```
Debug form:    left: "PATCH / Patch(Engine)"   right: "PATCH / Engine"
as_str() form: left: "PATCH / patch.engine"    right: "PATCH / Engine"
```

The per-variant match in `state_projector.rs` is deleted: one composition, one
source, no arm left in which to spell a key. Breadcrumbs read `PATCH / Master
Volume`, `MIXER / T00 Level`, `PATCH / Model`, `MIXER / T00 Send B0`.

## Other verification

- **T017's removed tautology is a genuine removal.** The retained comparand
  `page.patch().id()` derives from `remembered_patch_main.patch_id()` — a different
  `InteractionState` field — so it can fire.
- **`VoiceLimitCarryOver::Clamped { previous }`** is not consumed, and the
  explanation is correct: the projection resolves the carry-over before the commit,
  so the row's projected value already *is* the previous limit. It earns a consumer
  only in F-22's durable report. F-02 item 4: closed.
- **Gates**: 701 lib tests plus every integration target, 0 failed; clippy clean;
  fmt clean. F-12 did not fire this run.
- **F-23 re-measured**: unchanged by cycle 2. See F-30 for the full-pipeline number.

## Inherited

WP04: `footer.pathLabel` is now safe to read; F-16's two remaining casings are
yours. WP05: bullet 7 ready, bullet 8 withdrawn, F-28's key-set fix assigned.
WP06: F-29's clone halving to measure first, F-30's 3.00 ms, and cycle 1's
four-line rustfmt change in `live_effects_and_buses_scene.rs` to rebase past.
