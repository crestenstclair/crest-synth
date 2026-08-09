# Cross-WP findings — functional-patch-editor-01KZERV9

Findings raised during implementation that outlive the work package that raised
them. Recorded here so none is silently inherited.

## F-01 — FR-009 must not be credited to WP01 alone

**Raised by**: WP01's review
**Owner**: WP02 (H1), then WP05 and the mission review

WP01 built `VoiceLimit`, enforced it in the callback, and proved the enforcement
falsifiable. It did **not** make the shipped product correct: `Patch::new` seeds
`VoicePolicy::EngineManaged` = 64 unconditionally, so in the running app every
Patch carries 64 — including the Braids Patches whose engine declares 16.

The crest-spec's `valueObject.Synth.VoiceLimit` invariant says installation seeds
from the active engine's ceiling "so no Patch starts with a limit its engine
could not honour". That is **unsatisfied** until WP02's H1 lands, because
`install_patches` is the only site with registry access and it belongs to WP02.

WP01's boundary was honest — the reviewer verified independently that neither
alternative was available inside WP01's ownership map. But FR-009 is not complete
on WP01, and neither WP05 nor the mission review should credit it there.

## F-02 — Six public APIs with no production caller

**Raised by**: WP01's review
**Owner**: WP02, WP03

`Patch::installed`, `seed_voice_limit`, `replace_instrument_config`,
`VoiceLimitCarryOver`, `with_voice_limit`, and `VoiceLimitDescriptor` /
`surface_descriptor` currently have zero production callers — only tests. They
exist for WP02 and WP03 to consume. If those packages do not consume them, they
are dead code at mission review, and the honest remedy then is deletion, not a
retroactive justification.

## F-03 — The leaf-descriptor enumeration gap was mine, and it is closed

**Raised by**: WP01's review
**Owner**: closed at crest-spec authoring; WP02 implements

The crest-spec widened `RtPatchParameters` to carry `VoiceLimit` without amending
the leaf-descriptor invariant's enumeration. WP01's prompt then demanded
enumeration the declaration did not require. The reviewer caught the disagreement
and ruled correctly that the declaration wins over a prompt.

Resolved deliberately rather than by letting the code decide: the enumeration in
`.kittify/crest-spec/contexts/realtime.yaml` now names `voiceLimit`, because a
canonical value that crosses the real-time boundary and changes what is audible
must be visible in the trace or no measured proof can correlate it. WP06's live
scene has to correlate exactly that.

## F-04 — One overclaiming test in `parameter_snapshot.rs`

**Raised by**: WP01's review
**Owner**: WP02 (it opens that file for H3)

`the_voice_limit_widens_the_entry_by_one_bounded_integer` asserts two tautologies
while its docstring claims the assertion is exact. The bullets it claims are
covered by two stronger pre-existing tests, so nothing is uncovered — but a false
rigor claim in a docstring is worse than no test. WP02 makes it exact or deletes
it.

## F-05 — The mission baseline's "1 pre-existing test failure" is malformed

**Raised by**: WP01, confirmed by its review

The baseline capture records one pre-existing failure. It is not a test: the
entry is `<declared-command>` with "For more information, try '--help'" — a CLI
usage error, the same capture defect a previous mission recorded as F-12. The
lane measured 734 passed / 0 failed at base. No WP should carry blame for it and
no WP should chase it.

## F-06 — Re-trigger at the limit is refused

**Raised by**: WP01's review
**Owner**: unowned; a deliberate ruling if re-trigger behaviour ever matters

The active-note bitset is per (patch, note), so re-triggering an already-sounding
note while at the limit is refused even though it would add no voice. This is
consistent with the declaration as written. Recorded rather than fixed, because
changing it is a product decision about what "sounding its limit" means.

## F-07 — The engine-swap carry-over asymmetry was undeclared. Now ruled.

**Raised by**: WP02's review
**Owner**: closed at declaration; WP02 rework and WP03 implement

WP01 invented the policy — clamp the voice limit on narrowing, preserve it on
widening — and the WP02 handoff H2 told WP02 to apply it. Neither was wrong to
act, but nobody had declared it, so the product behaviour was being settled by
whoever wrote the code first. That is precisely the inversion the crest-spec
phase exists to prevent, and it was mine to catch when I authored `VoiceLimit`.

**Ruling, now in `valueObject.Synth.VoiceLimit`:** the asymmetry stands and is
deliberate. Narrowing clamps, because a limit the engine cannot honour is not a
limit. Widening preserves the player's value rather than raising it, because the
limit is the player's setting and an engine change is not a request to change it.

The consequence is stated rather than hidden: the carry is **lossy by design** —
narrow then widen does not restore the prior value — and that loss is reported
through the typed carry-over outcome rather than left for a player to discover.
This also settles F-02 item 4: `VoiceLimitCarryOver`'s `Preserved`/`Clamped`
discriminant now has a declared consumer, and it is WP03's engine-swap status
projection.

WP02's demo-scene fix — making the scene restore the limit through the Utility
row the way a player would, rather than excusing the field from the reversibility
check — was the honest response to an undeclared asymmetry and stands.

## F-08 — `detailSubject` in the StateTree interaction shape

**Raised by**: WP02's review
**Owner**: WP02 rework

The reviewer notes that `detailSubject` is absent from the crest-spec's
`StateTree.interaction` shape, so omitting it is currently correct — but that the
exact argument which got `voiceLimit` enumerated in F-03 applies here too.

It does. `detailSubject` is reducer-owned state that determines what is on
screen, and a trace that cannot show which capability a detail surface was opened
on cannot correlate a detail interaction with its consequence. Enumerate it on
the same reasoning. Folded into the WP02 rework alongside B2.

## F-09 — Two crest-spec bullets disagreed on SelectPatch focus recovery. Now one.

**Raised by**: WP02's review
**Owner**: closed at declaration

`aggregate.Control.InteractionState` said schema changes repair through the
"one deterministic next-before-previous sibling rule", and then said a SelectPatch
switch recovers "to its first valid control". Two rules, one of them called "the
one rule". My authoring error.

**Ruling:** the sibling rule, everywhere. There is exactly one focus-recovery rule
in this system and a patch switch is not an exception to it. Recovering to the
first valid control would make the destination's first row a special case no other
schema change has, and would throw away the operator's position for no reason.
WP02 implemented the sibling rule per T012, which was the strict improvement; the
declaration now says what WP02 already built.

## F-10 — `select_patch` reads the subordinate surface's control

**Raised by**: WP02's review
**Owner**: unowned; pre-existing

`select_patch` reads `patch_control_focus()`, which on a subordinate surface
returns that surface's control rather than the remembered main one. Pre-existing,
not introduced by this mission. Recorded so it is not discovered a third time.

## F-11 — `PatchDetail` is gated out of the offered vocabulary until WP03

**Raised by**: WP02 cycle 2, closing review blocker B1
**Owner**: WP03 removes the gate; WP06 restores what the gate costs

`SurfaceId::is_enterable` withholds `PatchDetail`, so `EnterSurface(PatchDetail)`
is not phase-two admitted, is absent from `SemanticAction::surface_descriptor`,
never reaches `validActions` or a footer hint, and is a typed unchanged rejection.
The reducer still owns, remembers, and leaves the surface — `ReturnPath::new` now
reads a new structural `is_return_target` predicate instead of `is_enterable`, so
a surface the vocabulary does not offer is still one the reducer can hold.

The gate exists because an accepted detail state cannot yet be projected, and
`app_loop.rs:347` `.expect()`s that every accepted state projects. Advertising an
action that panics the loop is worse than not offering it yet.

**Removing it is three coupled changes, all WP03's T015:**
1. the containment check at `patch_page_projection.rs::project`,
2. the detail page content plus `render_patch_text`'s selected line,
3. deleting the `PatchDetail` arm in `is_enterable`.

WP02 measured that (1) alone is not sufficient and the review's sizing of it was
wrong: it only moves the failure from `PatchPage(InvalidInstrumentConfig)` to
`render_patch_text`'s `InvalidSelection`, because the page projects
`control_id: None` for every `ScalarEdit` row, so a Braids `Capability(braids.model)`
detail focus has no line to be selected on. WP02 reverted its partial fix rather
than leave a half-fix that looks like the hole is closed.

**The gate cannot become permanent by forgetting.** Three tests fail the moment
(1) and (2) land and must be updated together with (3):
`app_state::the_entry_gate_exists_because_a_braids_detail_state_cannot_yet_be_projected`,
`semantic_focus::the_detail_surface_is_a_return_target_but_is_not_offered_until_wp03`,
and `semantic_action::semantic_action_descriptors_are_closed_unique_and_phase_two_safe`.

**What the gate costs, for WP06.** The demo scene loses its `surface.detail` steps
and coverage identifier, because a gated action is a refusal, not a demonstration.
WP02 made the expected-coverage builder skip a surface that is not `is_enterable`,
so WP03's gate removal restores the *expectation* automatically — but WP06 must
put the two scene steps and their checkpoints back by hand.

## F-12 — `input_capture_witness` stalls intermittently under parallel load

**Raised by**: WP02 cycle 2
**Owner**: unowned; pre-existing

`tests/input_capture_witness.rs:428` ("the witness script stalled") fails roughly
one run in two under full parallel load. It is a windowed test contending for key
focus and passes 3/3 standalone. WP02 confirmed it pre-existing by running the
suite three times in a temporary worktree at the pre-rework base `f2bbd56` and
reproducing the identical failure on run 1 of 3. Not a regression. A reviewer
running the suite once may hit it.

## F-13 — Two review claims rested on partial enumerations

**Raised by**: WP02 cycle 2
**Owner**: recorded as a review-practice note

Both of WP02's rejection findings under-counted, in the same way the findings were
about:

- B2 named `subject.kind` and `subject.capability_id` as the undeclared leaves. An
  `Effect` subject also produces `subject.slot_id`. Declaring only the two the
  reviewer's instrument-subject fixture happened to discover would have reproduced
  the defect class exactly — three leaves, two fixtures.
- The `with_voice_limit` ruling verified two callers; there were five. All
  test-only, so the deletion ruling stands, but "verified zero non-test callers"
  rested on a partial list.

The lesson is not that the reviewer was careless — both findings were real and one
was a reproduced panic. It is that a fixture which can only see one variant is the
recurring failure mode in this mission, and it catches reviewers too.

## F-14 — F-11 under-stated the gate's removal cost; ownership widened

**Raised by**: WP02 cycle-2 review
**Owner**: closed by reassignment; WP03 executes

F-11 recorded three tests coupled to the gate. Removing it actually turns **six**
red, and two of the files needed were not WP03's:

- `src/control/semantic_focus.rs` holds `is_enterable` — WP02's file.
- Two demo-scene coverage tests go red with `missing: ["surface.detail"]` and can
  only be made green by restoring the two `surface.detail` steps in
  `src/testing/demo_scene.rs` — WP06's set.

WP03 could not have landed a green lane by removing the gate alone. Both files are
now WP03's: `semantic_focus.rs` transfers from WP02 (approved and closed, so no
contention), and `demo_scene.rs` is carved out of WP06's set, whose actual work is
the *live* scene rather than the headless exhaustive one. WP06's `src/testing/**`
glob is replaced by an explicit list so the carve-out is checkable rather than
implied. Ownership validation passes with no warnings.

The general lesson: a gate is only honest if some package can afford to remove it.
Recording the tripwires was necessary but not sufficient — the removal cost has to
land inside one package's map.

## F-15 — FR-012 is not user-reachable after WP02

**Raised by**: WP02 cycle-2 review
**Owner**: WP03; then WP05 and mission review

The detail surface is proved at the reducer seam, but the gate means no player can
enter it until WP03 lands the projection. Same shape as F-01: the work is real and
the package is done, but the requirement is not satisfied where a user stands.
Neither WP05 nor the mission review should credit FR-012 to WP02.

## F-16 — `PatchDetailSubject` casing, frozen at schema version 14

**Raised by**: WP02 cycle-2 review
**Owner**: WP03, folded in

`#[serde(tag, rename_all)]` renames variants, not fields, so `PatchDetailSubject`'s
fields serialize snake_case inside an otherwise camelCase schema. No declared
invariant mandates camelCase, and the shape was visible at cycle 1 where the
reviewer did not flag it, so it is not WP02's to redo. WP03 is already moving the
semantic leaf descriptor and bumping the version; folding the casing fix in there
costs one version bump instead of two.

## F-17 — Stale rustdoc links to the deleted `Patch::installed`

**Raised by**: WP02 cycle-2 review
**Owner**: whoever next touches `src/synth/patch.rs`

Two links at `src/synth/patch.rs:94` and `:106` point at the deleted
`Patch::installed`; `:106` describes a constructor that no longer exists. Two new
`broken_intra_doc_links` warnings. Rustdoc is not gated, so this is doc rot rather
than a break — fix on the next commit touching that file rather than opening one
for it.

## F-18 — The casing ruling was scoped to one of three instances. Rest goes to WP04.

**Raised by**: WP03, disagreeing with F-16 as written
**Owner**: WP04

F-16 told WP03 to fix `PatchDetailSubject`'s snake_case-in-camelCase field
serialization. WP03 complied and then said the ruling was wrong: the same
`#[serde(tag, rename_all)]` defect covers `SemanticSurfaceSummary` and
`MixerControlId` too, and fixing one of three leaves the schema **mixed** —
`summary.subject.capabilityId` beside `summary.capability_id` — which is harder to
reason about than uniformly wrong. It asked the mission to choose all or none
rather than complying silently.

**It is right, and the ruling is amended.** WP03 stopped where it did for a good
reason: `webview-page/page.js` reads `summary.patch_name`, `summary.capability_id`,
and `summary.patch_id` by name, and that file is WP04's. WP04 is rewriting that
render path wholesale for the composed strip, so it can move both sides in one
place. The remaining two casings and the page reads are now WP04's, recorded in
its prompt.

The mixed state exists only between WP03 and WP04 and ships in neither.

This is the second time an implementer has pushed back on a ruling of mine and
been right (WP02 did it twice). The pattern worth keeping: a ruling written from
one package's vantage point can be locally correct and globally wrong, and the
package holding the other half is the one positioned to notice.

## F-19 — WITHDRAWN. Read-only sections DO exist, and C3 must not be withdrawn.

**Originally raised by**: WP03
**Disproved by**: WP03's review, by execution
**Owner**: WP03 cycle 2; WP05 must read the correction before writing T033

**This finding was wrong and I propagated it without checking.** WP03 reported
that `CapabilitySection` has no read-only flag and that "the concept does not
exist in the type system", and I recorded that, and drew the conclusion that
WP05 might have to withdraw the analysis pass's C3 claim.

The reviewer disproved it: `PatchInteraction::ReadOnly` exists at
`instrument_capability.rs:146`, is crest-spec declared (`synth.yaml:168`, `:228`),
is the **default** of `ParameterSpec::new`, is what all three Braids rows and
SoundFont's `file` row actually declare, and is already consumed for `editable` at
three sites in the same file WP03 wrote the claim about. The same commit even
projects it at `patch_page_projection.rs:148`, and a production Braids detail
projection emits `"patchInteraction": "readOnly"`.

**Consequences, both directions:**

- **WP05 must NOT take T033's escape hatch.** The fixture does declare read-only
  sections. The claim stays in the spec's Scope Decisions table. The production
  producer is `patchPage.detail.sections[].parameters[].patchInteraction` — not
  `editable`, which is where WP03 looked.
- **WP03 cycle 2** either carries the declared interaction onto the semantic
  model, or corrects the written record. Not both — the block is on the record,
  not the code.

Worth keeping: an implementer's "this concept doesn't exist" is a claim about the
codebase, and I treated it as one about the world. Two earlier pushbacks were
right, which is exactly what made this one easy to accept without checking.

## F-20 — F-14's enumeration was also incomplete; a seventh test

**Raised by**: WP03

F-11 named three tests coupled to the gate. F-14 corrected it to six and widened
ownership. WP03 found a seventh: a hardcoded Contexts coverage list at
`tests/exhaustive_demo_scene.rs:110`. Three successive enumerations of the same
blast radius, each short.

Not a process failure to fix so much as a measurement to keep: in this codebase
the coupled-test set around a vocabulary change is reliably larger than it looks,
because several tests hardcode surface or coverage lists rather than deriving
them. That is worth knowing before the next vocabulary change, not after.

## F-21 — `make fmt-check` was already red at WP03's lane base

**Raised by**: WP03, verified by stashing

The tree was unformatted at HEAD before WP03 started. WP03 ran `cargo fmt --all`,
which touched four files outside its map as a side effect. The tree is now clean.
Recorded so the formatting churn in WP03's diff is not read as scope creep.

## F-22 — The voice-limit loss report is transient

**Raised by**: WP03
**Owner**: needs a ruling before mission review

F-07 required that a narrowing engine swap *report* the voice-limit loss rather
than leave a player to discover it. WP03 projects `requestedValue` on the
voice-limit row while the swap is `Preparing`. Once it commits, canonical state
already holds the clamped value, the same rule returns `Preserved`, and the row
falls silent.

That satisfies "a projection that shows the limit while a swap narrows it says
so" as literally written. Whether it satisfies the intent is a real question: a
player who looked away during preparation never learns their limit changed. A
durable record needs the carry-over outcome retained on `EngineSelectionStatus` —
canonical state plus a crest-spec field, outside WP03's scope and not something to
bolt on late.

## F-23 — Per-row action lists cost 2.92 ms per accepted event on MIXER

**Raised by**: WP03's review, measured independently in release
**Owner**: WP06 (NFR-004), and a ruling if it does not hold

The `Arc` change was upheld: `CapabilityRegistry::clone` was 91% of the pre-Arc
`AppState::clone`, and the reviewer reconstructed a 23× improvement on its own
machine. Cheaper designs were considered and ruled out — memoisation has no sound
key because only the reducer knows the focused control's identity, and a resolver
that avoids the clone is the one alternative T013 forbids by name, precisely so
the per-row answer cannot drift from the production reducer.

But the price is not only test time, and neither WP03 nor I measured the
production half. The reviewer did: **MIXER reprojection is 2.92 ms across 89 rows
per accepted event on the control thread** (PATCH: 630 µs over 17 rows). It scales
as rows × actions × clone.

NFR-004 says "projection throughput unchanged". WP06 inherits this with a number
attached, and must measure rather than assume. If the live scene's frame or
generation evidence degrades, the honest options are to narrow what carries a
per-row list or to grade NFR-004 with the number stated — not to quietly loosen
the bar.

## F-24 — WP03 edited a file belonging to an open package

**Raised by**: WP03's review
**Owner**: WP06 must be told; not a violation to undo

Every out-of-map edit WP03 made was into WP01's or WP02's files — both approved
and closed, so no contention — except one:
`src/testing/live_effects_and_buses_scene.rs` is **WP06's, and WP06 is open**.
Four lines, pure formatting, forced by `cargo fmt --all` which WP03's own lane
gate required and which was already red at its base (F-21).

The reviewer let it stand. It is recorded so WP06 is not surprised by four lines
of formatting churn in a file it has not opened yet.

## F-25 — The footer path label still composes serialization keys

**Raised by**: WP03's review
**Owner**: WP04 must not re-couple; the composition itself is unfixed

`state_projector.rs` still builds `graphicalShell.footer.pathLabel` from
serialization keys — `"MIXER / GLOBAL / masterGainDb"`, `"PATCH / patch.voiceLimit"`.
This is the same class of defect T016 exists to close, one layer up.

It is non-blocking only by accident: `page.js:1031` ignores `pathLabel` entirely
and builds its own breadcrumb from `control.label`. So a serialization key is
composed into a projection field and then not read.

**WP04 must not re-couple to `pathLabel`** while recomposing the footer. If it
does, the keys reach the screen and T016's guard — which does not walk this path
either — will not notice.

## F-26 — A release-mode flake, distinct from F-12

**Raised by**: WP03's review
**Owner**: unowned; pre-existing

`adapter::lock_free_structural_graph_boundary::tests::status_is_latest_wins_...`
fails roughly 2 runs in 3 under `cargo test --release` and passes in debug.
Untouched by WP03; the declared gate is debug and debug is green. Recorded
alongside F-12 so nobody chases it during the live phases, where release builds
are the norm.

## F-27 — Two agents wrote lane-c concurrently. Orchestrator error.

**Raised by**: WP03's cycle-2 implementer, which noticed and said so
**Owner**: orchestrator; no code consequence, but a process one

I dispatched WP03's cycle-2 implementer while cycle 1's agent was still live in the
same lane worktree. Both wrote `lane-c`. The cycle-2 report noted it explicitly:
files changed between its own consecutive commands (`app_state.rs`,
`interaction_state.rs`, `patch.rs` mid-verification; `semantic_graphical_view_model.rs`
twice more), one commit was authored by something other than itself, and one test
run failed against a half-written file and passed on re-run.

It did the right thing: it identified the stale-binary artifact as an artifact
rather than a real failure, and re-verified everything on the committed tree
before reporting.

**No code consequence.** I stopped the second agent, then independently ran the
gates on the committed tree rather than trusting either report: 701 lib tests plus
every integration target green, with only F-12's known flake firing. The lane's
own commits touch no protected path.

**The process lesson.** A rejection returns a work package to `planned`, but it
does not terminate the agent that was working it. Dispatching the rework is not
safe until the previous agent is confirmed stopped. Nothing in the loop enforced
that, and I did not check. For the remaining packages: stop the prior agent
explicitly before re-dispatching a rejected package into the same lane.

That this cost nothing is luck plus an implementer paying attention, not a
property of the arrangement.

## F-28 — The label guard walks every projection but cannot fail on 7 of 17 sites

**Raised by**: WP03's cycle-2 review, which located its own cycle-1 misdiagnosis
**Owner**: needs one — WP04 cannot reach it

Cycle 1 diagnosed the label guard's blindness as a *fixture* gap and prescribed
adding a Braids fixture and an effect detail subject. WP03 did exactly that and
falsified exactly what it was asked to. The fixture set is now genuinely wide: all
five surfaces asserted against `SurfaceId::ALL`, both engines, both detail
subjects, master gain focused on each of the two surfaces that host it.

The blindness moved rather than closed. A 17-site mutation sweep over every
label-producing site caught 10 and **missed 7**:

| missed site | reverted to | why it cannot fail |
|---|---|---|
| `patch_page_projection.rs:857` detail section label | `section.id()` | `CapabilitySection::id()` absent from the key set |
| `:1336` PATCH Main section label | `section.id()` | same |
| `:1386` effect-slot section label | `section.id()` | same |
| `:1192` engine choice label | `descriptor.id()` | `CapabilityId` absent from the key set |
| `:1451` engine active label | `descriptor.id()` | same |
| `:1349` occupancy choice label | `descriptor.id()` | `EffectCapabilityId` absent |
| `:1395` slot occupancy label | `effect_descriptor.id()` | same |

`serialization_keys()` collects descriptor names, parameter ids, leaf-descriptor
names, and `PatchControlId::as_str()` forms — but not capability or section
identifiers. So the guard *walks* these labels and still cannot *fail* on them.
The exact case cycle 1 named — a Braids descriptor whose `label()` equalled its
`id()` — remains shippable, and the added fixture bought nothing for it, because
the key set cannot express it.

This sits inside a declared invariant, not a nice-to-have:
`contexts/control.yaml` defines a serialization key as "the name a value carries
in the state tree, the parameter snapshot, or a leaf descriptor", and
`patchPage.sections[].id`, `patchPage.engine.activeCapabilityId`, and
`patchPage.effects[].capabilityId` are all such names.

**The fix is ~8 lines** in `serialization_keys` (`semantic_graphical_view_model.rs:2893`):
add each installed instrument and effect descriptor's `id()` and each of its
sections' `id()` to the key set. Re-falsification is mechanical — all seven rows
above must flip from MISSED to CAUGHT.

**Assigned to WP05**, which is the package that owns proving this mission's claims
and already writes the deterministic acceptance target. `semantic_graphical_view_model.rs`
is added to its map for this purpose alone.

The pattern to carry: cycle 1 was right that the guard was blind, and wrong about
where. A prescription attached to a correct diagnosis can still send the fix to
the wrong place, and "they did what I asked and it did not help" is the signal.

## F-29 — Halve the per-row availability clones before narrowing anything

**Raised by**: WP03's cycle-2 review; recovered from a commit whose content was lost
**Owner**: WP06, to measure against NFR-004

Each availability probe clones `AppState` **twice**, and the second is redundant.
`accepts_semantic_action` clones to obtain a `&mut` (`app_state.rs:659`), and the
reducer it calls is already transactional — `apply` does
`let mut next = self.clone(); next.reduce(event)?; *self = next` — so a refused
action never touches the state it was given.

A row pays `1 + 2·|vocabulary|` clones where `1 + |vocabulary| + |accepted|` would
do. Hoist one scratch clone per row, reuse it across refused probes, take a fresh
one only after an action is accepted. At a MIXER row most of the vocabulary is
refused, so the dominant term roughly halves.

It is behaviour-preserving by construction — same production reducer, same
question, which is the property T013 forbids trading away — but it is unmeasured,
its correctness rests on the transactional guarantee being total rather than
incidental, and it touches a reducer seam in a closed package.

**WP06 should measure this before reaching for anything that narrows what carries
a per-row list**, because unlike narrowing it costs no fidelity.

## F-30 — NFR-004's real number is 3.00 ms, not 2.92

**Raised by**: WP03's cycle-2 review, re-measured
**Owner**: WP06

Cycle 2 did not change the cost (2.83 ms vs cycle 1's 2.92 ms is noise). But the
earlier figure measured `SemanticGraphicalViewModel::project` alone. The **full**
projection pipeline per accepted event, release, 89 MIXER rows:

| surface | rows | `svm::project` | full `project_with_shell` |
|---|---|---|---|
| PATCH Main | 17 | 462 µs | 641 µs |
| MIXER Main | 89 | 2829 µs | 2997 µs |
| MIXER Inspector | 89 | 2820 µs | 2957 µs |

NFR-004 says "projection throughput unchanged". WP06 measures against 3.00 ms,
not 2.92, and grades honestly rather than loosening the bar.

## F-31 — `SemanticControlViewModel.state` is declared field-by-field and is not exhaustive

**Raised by**: WP03's cycle-2 review
**Owner**: mission review

`aggregate.Control.SemanticControlViewModel.state` (`contexts/control.yaml:806`)
enumerates its fields one by one and lists neither `numericRange` nor `focusable`,
both of which the code has carried since before this mission.

This weakens one of WP03's three reasons for declining to carry `patchInteraction`
onto the semantic model — "the declaration is field-by-field, so adding one means
editing the bedrock to permit code" — because the declaration is evidently not
exhaustive in practice. Its other two reasons stand on their own: one declared
fact should have one producer, and that producer already exists and already
reaches the screen.

Not WP03's to fix. A mission-review line: either the declaration is exhaustive and
two fields are missing from it, or it is illustrative and should say so.

## F-32 — Two rulings assumed things reach the page that never do

**Raised by**: WP04, disputing both
**Owner**: closed here; the projection consequence goes to WP05

The webview transport emits `projection.semantic_model()` only, and
`requirement.serialized_projection_transport` declares the page consumes exactly
the serde serialization of `SemanticGraphicalViewModel`. Two of my rulings ignored
that.

**F-25's lift was meaningless.** I told WP04 it could read
`graphicalShell.footer.pathLabel` now that WP03 had cleaned it. `pathLabel` lives
on `GraphicalShellProjection`, which the page never receives. The footer still
builds its breadcrumb from `control.label`, as it always did. F-25's real status is
not "fixed" but "a serialization key is composed into a projection field that
nobody reads" — better than a key on screen, and not the same as closed.

**WP03's B2 resolution rested on a false premise, and both the reviewer and I
accepted it.** WP03 declined to carry `patchInteraction` onto the semantic model,
reasoning in part that "the producer already exists and already reaches the
screen". It reaches the *StateTree* at
`patchPage.detail.sections[].parameters[].patchInteraction`. The reviewer verified
that leaf is emitted — true, and beside the point, because the page never receives
the StateTree. `TextProjection` reaches the webview nowhere either.

So for the shipped screen there was no read-only discriminator at all: `editable`
is uniformly `false` on every detail row, because the reducer refuses every
`Adjust` on `PatchDetail` in this phase. T028 had nothing to paint.

WP04 added `patch_interaction: Option<PatchInteraction>` to
`SemanticControlViewModel`, projected from the same single producer
(`ParameterSpec::patch_interaction()`), folded into the unshipped schema version
15. That is **outside its map** — the file is WP05's — and it flagged it rather
than letting it pass.

The lesson is specific and worth keeping: "the producer already reaches the screen"
is a claim about a transport, and neither WP03 nor its reviewer nor I checked which
projection the page actually receives. Three parties, one unverified premise, two
cycles of reasoning built on it.

## F-33 — A choice row paints a choice id, not an authored name

**Raised by**: WP04
**Owner**: WP05 — folded into this mission

The Preset row paints `sf2.bank-0.program-40`; Braids' Model row paints
`braids.model.csaw`. `SemanticControlValue::Choice` carries the stored config
string and the semantic model has no option-label vocabulary.

WP04 argued this is arguably outside FR-014, which is worded about *labels*. That
reading is defensible, and I am overriding it, because DESIGN.md calls this exact
row "the **authored-name** Preset row" and declares that SoundFont presets are
"labeled with exact authored SF2 names". An identifier is visibly on screen, on a
row the design authority says carries a name. Closing FR-014 while that ships would
be closing it on a technicality.

It is a projection gap rather than a page one — `SemanticControlValue::Choice` must
carry the descriptor's authored option label alongside the stored id, the way every
other row already carries `label`. WP05 owns
`src/control/semantic_graphical_view_model.rs` and is the package that proves this
mission's label claims.

WP04 was right not to invent a label page-side. That would have been the same
defect one layer over.

## F-34 — The semantic model drops `CapabilitySection`

**Raised by**: WP04
**Owner**: deferred, deliberately

FR-012 asks for a "capability-supplied title, sections". The semantic model
flattens `descriptor.parameters()` and carries no `CapabilitySection` — no section
identity, title, or accent. Every production capability declares exactly one
section, so nothing is lost today and "section order matches the projection" is
satisfied trivially. A two-section capability would render as one undifferentiated
run.

WP04 did not extend the projection for this, and that restraint was right: no
installed capability needs it, and building a section vocabulary nothing produces
is the placeholder rule in reverse.

It found a branch-free route for the title instead — the detail shell names its
subject by the projected *value* of the strip row that owns the detail rows (engine
row → "HiDef SoundFont", slot occupancy row → "Chorus"), so `detailShellHtml`
contains zero references to `summary.subject`. Painting `subject.capabilityId`
would have been exactly the defect FR-014 closes.

Deferred with the gap named: the declaration and the code disagree, and the first
two-section capability makes it visible.

## F-35 — CORRECTED. The display exists; it was asleep.

**Originally raised by**: WP04, from its own environment failure
**Corrected by**: WP04's review, which ran the gated tests
**Owner**: closed

I recorded that T028's painted assertions and `make demo-live-patch-editor` could
not run on this rig, and treated it as a hardware gap the mission would have to
carry to the accept gate.

Wrong. The LS28AG700N is attached — 3840×2160, "UI Looks like: 1920×1080", main
display. It was **asleep**. `caffeinate -u` wakes it and the gate passes. WP04's
review hit the same honest refusal (`no attached display seats the authored
1920x1080 viewport`), ran `caffeinate -u`, and executed the live sections — which
is how it found the three defects that rejected the package.

Verified independently: `system_profiler SPDisplaysDataType` reports the display
attached and seating 1920×1080.

**The consequence is large.** The gated evidence is producible: T028's painted
assertions, and `make demo-live-patch-editor` — the mission's exit gate — can both
run. There is no hardware gap to declare at accept.

The lesson: a harness that refuses is not the same as a capability that is absent,
and I took an implementer's environment failure as a property of the rig. The
harness was behaving exactly as designed — it refuses rather than degrading — and
the correct response to that refusal is to satisfy it, not to record it as a limit.

## F-36 — The compact viewport already scrolled, and now scrolls more

**Raised by**: WP04, measured against base rather than asserted
**Owner**: judged at review; recorded either way

Desktop: 816/816 content-to-viewport at base, 825/816 with grouping. WP04 tightened
the inter-group rhythm one token step to `--space-8` and returned it to 816/816.

Compact (1280×800): **708/536 at base** — it already overflowed before this
mission — and 846/536 now. WP04 calls that a degree rather than a kind, and
inherited.

NFR-003 requires both authored viewports to seat the surface "with no clipped or
overlapped row". Scrolling is neither clipping nor overlapping, and the no-scroll
rule this mission declares is scoped to the Utility panel, which does not scroll.
So NFR-003 is arguably satisfied on its own words while the compact viewport is
visibly worse than it was.

Recorded rather than resolved: whether "seats the surface" tolerates a scrolling
main workspace is a product question, and the honest answer at mission review is
the two numbers rather than a verdict dressed as a measurement.

## F-37 — A browser is not the shipped window, and the difference is 62 pixels

**Raised by**: WP04's review, measured in both
**Owner**: closed by the rejection; the lesson is general

WP04 could not run the gated tests, so it substituted Chrome rendering the
committed page inside an iframe sized to the authored viewports, and measured
rather than eyeballed. That was a reasonable substitution and it was diligent.

It was also systematically wrong in one dimension: **the shipped window gives the
page `innerHeight` 1018, not 1080** (768, not 800, at compact). The iframe granted
62 px of vertical room the product does not have. So WP04's measured claim — "the
desktop strip seating without scroll exactly as it did before the grouping" — was
false in the shipped window: `#strip` is 754/754 at base and 811/754 after.

Desktop went from seating to scrolling. That is a new fact, not an inherited one,
and it was invisible in the substitute.

The general form is worth keeping past this mission: a substitute runtime that
differs from the real one in any measured dimension will produce confident,
precise, wrong numbers. WP04's evidence was better than eyeballing and worse than
nothing in one specific way — it looked like verification.

## F-38 — Two assertions below the headless gate: one regression, one unsatisfiable

**Raised by**: WP04's review, by execution behind `CREST_WEBVIEW_TESTS=1`
**Owner**: WP04 cycle 2

The declared gate `cargo test --all-targets` is green: 701 lib tests plus every
integration target, 0 failed, F-12 quiet. Both defects sit entirely below it.

**A measured regression of a pre-existing assertion.**
`T011 … patch.envelope.attackMilliseconds position rail must have width (got 4.28px)`.
The `rail > 5.0` assertion is not WP04's — it predates the package. Running the same
live section at base `a380235` (neutralising only the Utility assertion WP04 exists
to change) **passes**. Rails went 1256–1305 px at base → 319–421 px, and 4.28 px on
the focused row. Cause: `.prow{flex-wrap:wrap}` with `.prow-position{flex:1}` — a
zero-basis item never forces a wrap, so the hint run eats the rail.

**A new assertion that can never pass.** T025's own check compares
`split_whitespace()` of the hint run's `textContent` against a `join(" ")` model
string, but `hintRun` emits adjacent `<span>`s with no separating text. Unsatisfiable
with more than one hint. This is **not** a Blink/WebKit difference — `textContent`
is identical in both, so the Chrome harness would have failed it too. It was never
exercised anywhere.

Both are the same shape as everything else this mission has found: the failure was
not that a test went red, but that no test ran.
