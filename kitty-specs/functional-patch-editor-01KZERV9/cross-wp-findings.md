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

## F-39 — The accept gate requires executed evidence, not reasoned-about evidence

**Raised by**: WP04's review
**Owner**: the accept gate; binding on WP04, WP05, WP06

Thirty-eight findings in, the mission's recurring failure has a single shape, and
it is not carelessness: **unexecuted evidence keeps being treated as evidence.**

It has now appeared at every level of this mission:

- a fixture that could see only one variant (the detail-entry panic hid because
  every test site used SoundFont);
- a guard that walked a projection it could not fail on (the label guard, twice —
  the PATCH page, then seven sites the key set cannot express);
- a leaf-exactness test whose fixture never opened the state it guarded;
- two reviewer enumerations that undercounted the thing they were enumerating;
- a ruling written from one package's vantage point that was locally right and
  globally wrong;
- a claim about a producer "already reaching the screen" that three parties
  accepted without checking which projection the page receives;
- and a substitute runtime that measured confidently in a frame 62 px taller than
  the product, which is the one that looked most like verification.

Four of those were caught only by someone deliberately mutating code and watching
what *failed to fail*. None was caught by a suite going red.

**So the accept gate for this mission requires the live sections to have actually
run.** Specifically, before acceptance:

1. `CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell` executed with
   the display awake, output recorded — not skipped, not substituted.
2. `make demo-live-patch-editor` executed on hardware, its observation recorded
   with actual field values.
3. Every falsification this mission claims — F-28's seven label sites, F-33's
   choice-value guard, the voice-limit defeat, the `--defeat-patch-selection`
   negative — demonstrated by performing it, with the observed failure text.

A `CREST_WEBVIEW_SKIP` line in the acceptance output is not a pass. Per F-35 the
display is present and wakes with `caffeinate -u`, so there is no environmental
excuse available.

The standard is the mission's own: a test that passes with and without the code it
claims to prove is not a proof, and a test that never ran is not a test.

## F-40 — The session is locked, and that IS a real blocker

**Raised by**: WP04 cycle 2
**Confirmed independently**: `ioreg -n Root -d1 -a` reports `IOConsoleLocked: true`
**Owner**: the operator; blocks the mission's exit gate

F-35 corrected "the display is absent" to "the display is asleep". That was right
and incomplete. There are two separate conditions:

| condition | state | fix |
|---|---|---|
| display attached and seating 1920×1080 | yes | — |
| display awake | wakes with `caffeinate -u` | automatable |
| **console session unlocked** | **NO** | **requires the operator's password** |

The paint acknowledgment is emitted from `requestAnimationFrame`, which macOS
suspends for a window behind the lock screen. Hence `only 0 of 150 projections were
acked as painted within 15s`. The evidence screenshot WP04 captured is a picture of
the lock screen, which confirms it directly.

**What still runs locked** (and did, this cycle): T022, T023, T010, T014, T025, and
crucially **T024 page-render determinism and T011 painted-geometry fidelity** —
because `window.crest.render` is synchronous and layout is computed regardless of
occlusion. That is how WP04 cycle 2 measured every viewport number it reports.

**What cannot run locked**: NFR-001 projection-to-paint, T012, T013, the T015 ack
audit — and **`make demo-live-patch-editor`, the mission's declared exit gate**,
which depends on the same ack path.

I previously told the operator the exit gate was producible without them. That was
wrong, and the error is the same shape as everything else this mission has found: I
corrected "absent" to "asleep", verified the display, and stopped — without checking
whether waking it was *sufficient*. A necessary condition satisfied is not a
sufficient condition met.

**This does not stop the mission.** Every deterministic gate, the full webview
geometry suite, and WP05's acceptance target all run locked. The mission can reach
`accept` with everything except the live witness, and park exactly there.

## F-41 — Compact cannot seat the strip, structurally. NFR-003 graded, not passed.

**Raised by**: WP04, quantified by its review
**Owner**: mission review — a product decision about the compact band budget

| document | rows | composition | band | scrolls by |
|---|---|---|---|---|
| patch-navigate | 12 (576 px) | 753 | 520 | 233 |
| patch-adjust | 12 (610 px) | 787 | 520 | 267 |
| patch-braids | 11 (528 px) | 705 | 520 | 185 |

The declared compact shell bands take 156 px, so the largest band the strip could
*ever* hold is 612 px in the shipped 768 px window (644 px at a true authored 800).
The composition needs 753 px, and stripping every gap and inset still leaves six
required group titles on top of 576 px of rows. `overflow-y: auto` on
`#workspace .strip` is pre-existing; compact was already 708/504 at base.

Seating compact requires painting fewer rows, going below the declared 48 px
interactive minimum, or enlarging the bands in `src/shell/density.rs`. None is
inside any package's map, and the first two are worse than scrolling.

**NFR-003 is graded met-with-qualification, not passed silently and not failed.**
All four of its stated criteria hold at both viewports: five shell bands intact,
Inspector at or above 320 px, every interactive target at or above the minimum, no
clipped or overlapped row — a scrolling container reaches every row, and the
no-scroll rule the crest-spec declares is scoped to the Utility panel, which does
not scroll.

What fails is NFR-003's *title* — "Both authored viewports seat the surface" — at
compact, structurally and permanently. The honest grade is met, with 233/267/185 px
attached, and a mission-review decision about whether the title or the criteria is
wrong. Desktop seats with 17 px of headroom (753 in 770), up from zero at base.

## F-42 — A threshold guard is only as discriminating as the widest fixture reaching it

**Raised by**: WP04 cycle 2, on its own work
**Owner**: WP05, as method

WP04's first T024 rail guard used a `rail > 5.0` threshold. It **passed against the
live defect**, because the T024 fixtures' rails were 319–421 px — comfortably above
it — while the failing T011 row was 4.28 px. Its review confirmed this by
neutralizing only the structural assertion and leaving the threshold in place: T024
passed. The guard as first written would have shipped the very defect it was added
for.

WP04 found it by running the mutation rather than by reasoning, rewrote the guard
structurally — the hint run's top edge must sit at or below the label's bottom edge
— and the structural form fails on the first document's first row, `patch.engine`,
which has *no rail at all*. That case no threshold can reach on any fixture.

This is the mission's recurring failure caught by an implementer on its own work
before review saw it, which is the first time that has happened here. The method
generalizes and WP05 should carry it: **falsify by mutation, not by fixture.** A
threshold passes whatever is comfortably above it, and the fixture set silently
decides what that is.

## F-43 — A NUL byte ships in `page.js`

**Raised by**: WP04's review
**Owner**: the merge step — no open package owns `webview-page/` after WP04

`webview-page/page.js` line 1384 uses a literal U+0000 as a dedup key separator in
the hint-run de-duplication.

The code is correct and the page is byte-identical through the production seam. But
it is the only file in the repository containing a NUL, and `grep`/`rg` therefore
classify the most-audited file in this mission as **binary** and return no line
matches. The reviewer's own searches came back empty until it noticed.

In a mission whose recurring failure is a tool that walks something it cannot report
on, a silent search failure over `page.js` is worth closing. One character — replace
with a printable separator such as `|` or a unit-separator that greps cleanly.

WP04's review approved rather than opening a third cycle, which was proportionate:
blocking two packages over a delimiter would have cost more than it bought. Fixed on
the consolidated tree at merge, the same way the previous mission handled its
inherited formatting gate.

## F-44 — I authored a witness with four predicates nothing can produce

**Raised by**: WP06, which stopped rather than build a scene against them
**Owner**: closed at declaration; two producers assigned to WP06

The `witness.functional_patch_editor` I wrote declares 44 predicates. WP06 traced
each to a production producer and found four that have none. The live ack payload
(`webview-page/page.js` `paintedEvidence`) carries exactly six identity fields,
`window.innerWidth/innerHeight`, and five shell-region rects with labels. Nothing
else reaches Rust.

| predicate | why it cannot be produced | ruling |
|---|---|---|
| `stripGroupsPainted > 1` | grouping is `stripGroups()` in `page.js`, page-side only. `grep group src/control/semantic_graphical_view_model.rs` is empty | **carry it in the ack** |
| `stripFlatControlRun == false` | same | **carry it in the ack** |
| `steamDeckViewportPainted == true` | `window.rs:522` opens at the desktop viewport and never calls `set_size`. There is no control-side resize seam; `TickCallback` returns only `bool`. Only the test harness resizes, and that path drives `renderObservation`, not `LiveDemoReport` | **withdrawn** |
| `clippedOrOverlappingRows == 0` | row geometry exists only in the harness-only `window.crest.renderObservation`. At the ack's region granularity, overlap is already validated in `ShellFrameObservation::try_new_semantic`, so any count is tautologically 0 | **withdrawn** |

**The two withdrawals.** A compact-viewport paint needs a resize seam that does not
exist, and building one is resolution work — ROADMAP Phase 8 is "Controller and
resolution hardening", which is where it belongs. The row-overlap count is already
proven *deterministically* and far better: WP04's T024 and T011 measure every row's
height, rail, hint run, and label/hint edges at both authored viewports in the
shipped WebKit. Asserting a tautological zero in the live witness would have added
the appearance of coverage over evidence that already exists elsewhere.

**The two producers.** Grouping is the substance of FR-001 and T030, so it stays —
but measured where it is known. ~6 lines in `paintedEvidence` and ~15 in
`projection_channel.rs` / `ShellFrameObservation` carry what the page painted to
Rust. WP06 owns this.

The alternative — reimplementing the page's grouping rule Rust-side to measure it —
is the defect F-25 and F-33 both rejected: a second producer for one fact. That is
not proof, it is agreement between two copies, and the first divergence is silent.

**This is the fifth crest-spec authoring error this mission has found in my work**
(F-03, F-07, F-09, F-19's propagation, now this). The pattern in all five: I declared
what should be true without tracing whether anything could make it true. A witness
is executable by definition, and I wrote predicates the way one writes prose.

## F-45 — `apply` is transactional but not uniformly so

**Raised by**: WP06, proving rather than assuming F-29's premise
**Owner**: closed, pinned by tests

F-29's clone halving rests on `AppState::apply` being transactional — a refused
action must never touch the state it was given. WP06 proved it instead of assuming
it, and found the guarantee is **total but not uniform**: `apply` has an early MIDI
branch that mutates `self.generation` *without* cloning. It never does so on a
rejection, so the property holds — but it holds by case analysis, not by
construction, and nothing was pinning it.

Two tests now do: one requires every refused action over every reachable fixture
state to leave its candidate byte-for-byte identical, and asserts refusals actually
occurred so it cannot pass vacuously; one requires the swept and one-shot
availability answers to agree everywhere.

That second test matters more than it looks. `accepts_semantic_action` and the
sweep are now literally the same code, so the per-row answer cannot drift from the
production reducer — which is the property T013 forbids trading away, preserved
through an optimization that halves its cost.

## F-46 — F-29 measured: 2960 µs → 2382 µs, and NFR-004 is met

**Raised by**: WP06
**Owner**: recorded

Release, median of 15, full `project_with_shell` per accepted event:

| surface | rows | before | after |
|---|---|---|---|
| MIXER Main | 89 | 2960 µs | **2382 µs** |
| PATCH Main | 15 | 529 µs | 470 µs |

Per-row sweep alone: 25.79 µs → 20.33 µs. The "before" is a real measurement — WP06
reverted only the resolver hoist, rebuilt, measured, restored — and it reproduces
F-30's independently measured 2997 µs within noise, which is what makes the fixture
credible rather than self-confirming.

**NFR-004 is met at 2382 µs against F-30's 3.00 ms.** F-29 bought 578 µs at no
fidelity cost, so nothing that narrows what carries a per-row list is needed. The
option that would have traded proof for speed is not required, which is the outcome
worth having.

## F-47 — `firstPatchAudibleEditDelta == 0` is unattainable exactly

**Raised by**: WP06, before writing the scene rather than during
**Owner**: ruled here

An exact zero on a live decaying voice is unattainable — natural RMS drift between
two observation blocks is nonzero. The only way to force exact zero is to keep the
first Patch silent across the measured window, which makes that half of the pair
weak evidence.

**Ruling: keep the pair, and make it a bounded comparison rather than an exact
zero.** The second Patch's delta must exceed the first Patch's by a declared margin,
with both measured on their own outputs over the same window. That is the claim the
gate actually cares about — the edit moved *this* instrument and not *that* one —
and it is stable against decay in a way an exact zero is not.

The discrimination survives intact, which is the point: under
`--defeat-patch-selection` the first Patch is the sounding subject, so its delta
goes nonzero *and* the second Patch's track is silent. The margin inverts, and
`patchesFocused`, `secondPatchIdDistinct`, and `secondPatchSlotsVisited` fire
alongside it.

**And the counters must be keyed by PatchId resolved from the final state's
installed order, never from the scene's own subject.** WP06 caught this and it is
the sharpest detail in its report: otherwise `secondPatchSlotsVisited == 3` passes
under defeat, because a defeated scene still visits three slots — just on the wrong
instrument. A predicate that counts work without checking where the work landed is
exactly the shape of everything else this mission has found.

## F-48 — F-11/F-14's instruction to WP06 was stale

**Raised by**: WP06, checking rather than doing

F-11 and F-14 told WP06 to restore the two `surface.detail` demo-scene steps by
hand after WP03 removed the gate. WP03 already put them back
(`src/testing/demo_scene.rs:665-672`), and `tests/exhaustive_demo_scene.rs:126`
asserts the coverage identifier. It passes.

Recorded because a stale instruction that says "do this" is worse than none: the
obedient response is duplicated work, and WP06 checked instead. F-24's warnings
about inherited churn were likewise already resolved in its base, and T040's
"correct the stale Makefile alias wording" did not apply — the comments were
already right.

## F-49 — Two live-runner couplings any new scene must satisfy

**Raised by**: WP06, verified
**Owner**: WP06

- `LiveDemoRunner::advance_engine` hard-errors via `current_engine_transition()?`
  (`live_demo_runner.rs:599`) if a scene declares zero engine transitions.
- `LiveDemoReport::new`'s `complete` requires `runtime_audio.engine_switches() == 3`
  (`live_demo_report.rs:1147`).

So a new live scene must either perform the base scene's three engine transitions or
both must be relaxed. Neither is a defect; both are undeclared assumptions that a
scene author meets by discovering them. Worth knowing before writing 800 lines.

## F-50 — The shipped observation adapter drops `voiceLimitRefusals`

**Raised by**: WP05
**Owner**: WP06

`AtomicObservationFields` in `src/adapter/atomic_audio_observation.rs` has no atomic
for the refusal counter, so a count read through the production transport is zero
regardless of what the callback counted. Nothing on the control side reads it today.

WP05 did not fix it — the file is WP06's, and WP06 is the package that must
correlate a refused note with the limit that refused it on hardware. Its own target
counts where production counts (`AudioRenderer::render`) and publishes through a
local two-atomic transport, documented in place.

Its reasoning is the right one and worth stating: reading FR-009's central claim
through an adapter that discards it would have been this mission's signature defect
— a guard walking something it cannot report on. WP06 was already instructed to add
the field; this confirms it is load-bearing rather than tidy-up.

## F-51 — `SemanticControlViewModel.state` is now five fields short

**Raised by**: WP05, extending F-31
**Owner**: mission review

F-31 recorded `numericRange` and `focusable` missing from the crest-spec's
field-by-field declaration. Add `patchInteraction` (WP04), `selectedLabel` and
`requestedLabel` (WP05): five.

Both WP04 and WP05 followed the same precedent — do not edit the bedrock to permit
code already written — which is correct and has now produced a declaration that is
visibly not what the code carries.

Either the declaration is exhaustive and five fields are missing from it, or it is
illustrative and should say so. Not a package's to decide; it is a question about
what that block of the crest-spec means, and answering it by adding five fields
would settle it in the direction that happens to match today's code rather than the
direction that is right.

## F-52 — The transcription ruling, and the boundary it rests on

**Raised by**: WP05, disputing F-44's reach
**Ruled**: the transcription stays

WP05's `page_strip_groups` is a Rust transcription of the page's `stripGroups`
rule. F-44 ruled that reimplementing the page's grouping rule Rust-side is a second
producer for one fact. WP05 kept the transcription and made its case rather than
complying silently.

**It is right, and the boundary is worth stating precisely so this is not misread
later.**

- **F-44 is about the witness**, which proves *what a run painted*. There, a Rust
  reimplementation would be two producers of one fact and the first divergence would
  be silent. WP06 carrying `stripGroupsPainted` from the page's own acknowledgment is
  the correct producer, and that ruling stands unchanged.
- **WP05's target proves the page's *rule* against the committed source.** It has no
  DOM and no acknowledgment, so it cannot carry the page's answer. Without the
  transcription, T030's grouping claim has no executed proof at all while F-40 blocks
  the live layer — strictly worse than a pinned transcription.

Different claims, different producers, both legitimate. `tests/component_composition.rs`
already established this pattern for the deterministic layer; WP05 did not invent it.

**The ruling rests entirely on the pins being real**, and WP05 proved that is a live
constraint rather than a formality: its own range pin anchored on a string appearing
three times in `page.js`, so emptying `rangeHtml` left the pin satisfied and the
guard passed the defect it exists for. It found this by mutation, not by reading.

So the transcription is acceptable **only while every rule it copies is pinned to
the committed source, and every pin is falsified by mutating that source.** A pin
that matches a comment, or matches in more places than it claims, is not a pin. If a
later mission finds a copied rule no pin covers, the correct response is to add the
pin or delete the transcription — not to trust it because it was once allowed.

## F-53 — F-42 recurred one mission later, in the work of the agent that was told about it

**Raised by**: WP05, on its own work

WP05 was briefed on F-42 explicitly — a threshold guard that passed the very defect
it was added for — and then wrote a pin with the same shape: an anchor string
(`control && control.numericRange`) that appears three times in `page.js`, twice in
position-indicator helpers. Emptying `rangeHtml` left it satisfied.

It found it by running the mutation. It also found a second, the same way: mutating
the page's declared-group re-insertion changed nothing, because its own transcription
implements that rule.

**This is the strongest evidence the mission has produced that F-39 and F-42 belong
in doctrine rather than in a findings file.** Knowing about the failure mode did not
prevent it. Running the mutation did — twice, in the same package, by an agent that
had been told what to watch for and still needed the mutation to see it.

The mission's own record now shows the practice catching the failure at every level:
in implementation, in review, in an implementer's self-check, and now in an
implementer who had read the warning. Nothing else in this mission caught these.

## F-54 — F-27 recurred, in the work of the agent that recorded it

**Raised by**: the orchestrator, on itself
**Owner**: the orchestrator; the note did not work, so the practice must

F-27 recorded that a rejection returns a work package to `planned` but does not
terminate the agent working it, and that dispatching the rework into the same lane
before stopping the previous agent is unsafe. I wrote that finding, and then did it
again: WP06's first session was still live in lane-f when I dispatched its
continuation there.

No damage — the first session was only re-reporting a stale wait loop, I stopped it
on noticing, and `95f600a`'s work is intact with the continuation's uncommitted
changes untouched. But it cost nothing by luck, twice.

**This is F-53's shape applied to process rather than to tests.** Knowing about the
failure mode did not prevent it. What would prevent it is a step that cannot be
skipped: stop the prior agent as part of re-claiming a lane, not as something to
remember before dispatching.

For the rest of this mission: `TaskStop` on the previous agent is the first action
of any re-dispatch, before the claim, unconditionally — including when the previous
agent has reported `completed`, because `completed` is what both of these had
reported.

The generalizable lesson is the one the whole mission keeps producing: a recorded
warning is not a control. F-39 and F-42 earned their place by being enforced at a
gate; F-27 was written down and read and did nothing.

## F-55 — The transcription ruling's condition was tested and failed

**Raised by**: WP05's review
**Owner**: WP05 cycle 2

F-52 ruled WP05's Rust transcription of the page's grouping rule acceptable **only
while every rule it copies is pinned to the committed source, and every pin is
falsified by mutating that source.** The reviewer tested that condition directly.
Six mutations to `page.js`, each breaking a rule the transcription copies, all
**MISSED** — the acceptance target stayed green:

- `stripGroupKey`'s `patch.envelope.` prefix altered
- `stripGroupKey`'s `patch.effect.` arm returning `null` instead of `openSlot`
- `stripGroups`' `if (!control.visible) continue` neutralized
- `groupHeadControlId`'s `slot.N` → `patch.engine`
- `stripGroups`' `openSlot` assignment nulled
- unknown identities dropped instead of marked `?group`

`page_strip_group_key` is a line-for-line copy of `stripGroupKey`, and **no pin
mentions any of its five prefix rules.** The `DESIGNED_STRIP_GROUPS` pin covers the
group *names*; the identity→group *mapping* — which is the substance of T030 — was
tied to nothing. Two further copied rules are also unpinned: `rangeEndpointText`'s
`toFixed(3)` and `controlValueText`'s `"ON"/"OFF"`.

**The ruling stands; the package fails its condition.** This is what a conditional
ruling is for. F-52 named the exact failure mode — "a pin that matches a comment, or
matches in more places than it claims, is not a pin" — and the reviewer found the
larger version: rules with no pin at all.

It is live rather than hypothetical. WP06 is in `page.js` now, F-44's
`stripGroupsPainted` producer has not landed, and F-40 blocks the live layer — so
this file is currently the mission's **only** executed grouping proof.

One sibling blind pin also found, the same shape WP05 caught in its own work:
`data-role="row-range"` occurs twice — the painting site in `rangeHtml` and a CSS
selector in `renderObservation` — so renaming only the painting site MISSES. The pin
set is not blind overall (the two `rangeEndpointText` pins independently caught a
fully-emptied `rangeHtml`), but that entry guards nothing it claims to.

## F-56 — Fourth instance, and the first where the reviewer had to run it

**Raised by**: WP05's review

Counting only this mission: the failure mode was visible solely by running a
mutation in WP04's implementation, in WP04's review, twice in WP05's own self-check,
and now in WP05's review. Four levels, one practice.

F-53 already argued F-39 and F-42 belong in project doctrine rather than a findings
file. This adds the case that matters most for how the doctrine should be worded:
**the reviewer had to run the mutation too.** Reading the transcription against the
page and satisfying itself they agreed would have approved a target whose central
claim was tied to nothing — and reading is what review normally is.

So the doctrine is not "implementers should falsify". It is: **a guard is unproven
until someone has watched it fail, and that obligation does not transfer by being
reviewed.**

## F-57 — Two declarations I amended in one place and not the other

**Raised by**: WP06
**Owner**: closed

Both would have failed `spec-kitty accept`, and WP06 found them by implementing
against the declaration rather than around it.

**The observation's `state:` block was stale.** F-44 withdrew
`steamDeckViewportPainted` and `clippedOrOverlappingRows`, and I amended the
witness schema and wrote the reasoning into the invariants — but left both fields in
the `state:` list. The invariant described a withdrawal the declaration beside it
did not perform. WP06 followed the witness (41 fields) and said so.

**F-47's ruling and the witness disagreed.** I ruled the exact-zero first-Patch
delta unattainable and replaced it with a bounded comparison, recorded that in the
findings, and never amended the predicate. The witness still said
`first_patch_audible_edit_delta == 0`. WP06 implemented the ruling, reported that
acceptance would fail on the YAML, and **did not lower the predicate or silence it**
— which is exactly right, and is the behaviour that made the gap visible instead of
absorbed.

Both are now fixed, and the second is fixed properly rather than by deletion: the
witness predicates a new `audible_edit_isolated_to_second_patch` boolean carrying the
bounded verdict, with both raw deltas still reported — because a verdict without its
inputs cannot be argued with. **WP06 must add that one field**; it already computes
the comparison in `is_complete()`.

The pattern is now familiar enough to name: I keep amending the half of a
declaration I am looking at. F-03, F-07, F-09, F-44 and now this — five times, always
the prose and not the machine-checkable list, or the reverse. The findings file is
not a substitute for the declaration, and a ruling recorded only in findings is a
ruling that will fail a gate.

## F-58 — The blocker was falsified, not asserted

**Raised by**: WP06

WP06 ran `make demo-live-patch-editor` rather than inheriting F-40's verdict. It
failed at `awaiting parameter projection paint confirmation at step 3` after 10 s,
zero paint acks. It then ran the **shipped** `demo-live-effects-and-buses` — a scene
that has passed on this rig before — and it fails **identically**: same step, same
predicate, same timeout. Neither reaches its own phase.

That is the difference between "our new scene does not work" and "nothing that needs
a painted frame works on a locked console", and it is the strongest falsification
available without the password. It is also the first time in this mission that an
environmental claim was tested by finding a known-good control rather than by
reasoning about the mechanism.

## F-59 — What the exit gate cannot yet prove, stated plainly

**Raised by**: WP06
**Owner**: the accept gate

Two honest limitations, both self-reported rather than discovered:

**The declared margin is unmeasured.** `AUDIBLE_EDIT_DELTA_MARGIN = 1.0e-3` is set
above the expected block-to-block drift of a silent stem, but no completed live run
has confirmed it. It is documented as a threshold in code and in ROADMAP. The first
live run confirms it or moves it.

**Under F-40 the controlled negative proves nothing.** Both the positive and the
negative exit 1, for the same environmental reason. `--defeat-patch-selection`'s
falsifying power currently rests entirely on the headless keying tests — which are
real and which caught four defects in WP06's own first plan — but the negative's
*exit code* is not evidence today. The `shortfalls()` list is emitted by name so a
completed live run distinguishes the two cases immediately.

Both belong in the acceptance record as stated limits, not as passing predicates.

## F-60 — `midi_input_rechannelled` measures projection, not editing

**Raised by**: WP06
**Owner**: the accept gate, to grade deliberately

The fixture packs fifteen Patches onto channels 0–14, so an adjacent-step rechannel
on Patch 2 is a `DuplicateMidiChannel` refusal. WP06 therefore measures the field as
"the row projected more than one channel across the run" rather than as a completed
edit.

It discriminates for what the live scene claims — the row is Patch-local and follows
the switch, so a defeated run projects one value — and FR-008's editability is proven
in WP05's deterministic target against a fixture that permits it. But the field's
name promises more than it measures, and WP06 flagged that rather than letting the
name carry it. Grade it for what it measures.

## F-61 — F-49's finding was half wrong; the dangerous direction was the other one

**Raised by**: WP06 cycle 2, correcting the review that raised it

The review found that mutating the effects-and-buses scene-name gate to a
never-matching literal left 715 lib tests and the topology, mixer and shell suites
green while dropping an entire evidence block. WP06 tested both directions and the
picture is different:

**Narrowing was already caught.** With the literal mutated to never match,
`tests/effects_and_buses.rs` fails at `the cumulative scene retains
effects-and-buses evidence`. The suites the finding listed do stay green — but the
suite that owns the phase was not among them, and it fails. That direction was
covered before anyone touched it.

**Widening was caught by nothing.** With the gate replaced by always-true,
`effects_and_buses`, `live_demo_scene`, `topology_change_lifecycle` and
`live_patch_editor_scene` all pass. That is the real gap, and it is the direction
that breaks *this* mission: the patch-editor scene declares topology transitions, so
an always-true gate grades its effect-slot occupancy walk against the
eight-destination bus contract it never claimed to meet.

Both directions are now pinned, with a second test showing *why* the gate is
load-bearing rather than tidy.

The lesson is narrow and useful: a mutation proves what it proves. Testing one
direction of a boolean gate and reporting "unprotected" named a real gap and got its
polarity backwards — and the polarity was the part that mattered.

## F-62 — The same half-amendment, in miniature, two hours later

**Raised by**: WP06 cycle 2
**Owner**: closed

I amended the witness to add `audible_edit_isolated_to_second_patch` after
`first_patch_audible_edit_delta`, and amended the observation's `state:` block to add
it after `desktopViewportPainted`. Content agreed exactly, so nothing failed — but
the two declarations of one thing listed it in different places, which is F-57's
pattern repeating within the same working session that recorded F-57.

WP06 followed the witness as instructed, flagged the discrepancy rather than
silently picking one, and noted it would read as a defect to the next person who
diffs them. Now aligned.

Six instances. The through-line is not carelessness about any one edit — it is that
I treat a declaration as prose to be updated rather than as a machine-checkable
artifact with more than one face. `crest-spec doctor` passes on both orderings,
which is exactly why it kept happening.

## F-63 — The margin's exposure changed when the predicate did

**Raised by**: WP06 cycle 2
**Owner**: the accept gate

`AUDIBLE_EDIT_DELTA_MARGIN = 1.0e-3` was an inline comparison inside
`is_complete()`. After the F-57 amendment it stands behind a declared witness
predicate. The ruling that documenting is sufficient still holds and WP06 did not
invent a bound — but the exposure is different: the first completed live run now
either confirms the margin or **moves a predicate**.

One property worth keeping, which WP06 asserted rather than left true by accident: a
run with no edit on the second Patch reports both deltas at zero, and zero does not
clear the margin — so absent evidence reads as "not isolated" rather than as
isolation by default.

## F-64 — The completeness claim stopped being a claim about care

**Raised by**: WP05 cycle 2
**Owner**: doctrine

This is the most useful thing the mission produced, and it came from an agent
being honest about its own first attempt.

WP05 was told to pin every rule its transcription copies. It did a careful
line-by-line pass, produced 55 pins, and — in its own words — **would have
submitted that**. Then it wrote the audit that became
`check_every_line_of_a_transcribed_page_rule_carries_a_pin`, and the audit named
**ten body lines the careful pass had missed** — every one a *discriminator* rather
than an arm body: `value.kind === "scalar"`, `parameter.kind === "choice"`,
`if (key === null)`, `var openSlot = null`, `controls[c].validActions`, and so on.
Pinning the choice arm's body while leaving `=== "choice"` unpinned is F-55 in
miniature, one level down.

So the completeness claim is no longer "I walked it carefully". It is: **the set is
closed under a check that runs on every test run, and that check has already been
falsified twice** — a new unpinned arm fails, a stale scaffolding entry fails.

Two enforcement rules now live in the acceptance target rather than in a reviewer's
head:

- **every anchor must occur exactly once** — the review's own cycle-1 note ("assert
  each new anchor is unique so the next entry cannot be added blind") turned from a
  note into an assertion, and it immediately caught one of WP05's own new anchors;
- **every line of every wholly-transcribed function must carry a pin**, with a
  13-entry scaffolding list whose *unused* entries also fail, so the list cannot rot
  into a blanket permit.

F-56 said a guard is unproven until someone has watched it fail, and that the
obligation does not transfer by being reviewed. This is that lesson taken literally:
the omission that caused the rejection now fails at the moment it is committed,
rather than waiting for a reviewer to think of the right mutation.

67 mutations, 67 caught, `page.js` restored byte-for-byte each time.

## F-65 — Two defects the review did not find, and one is a shape worth sweeping

**Raised by**: WP05 cycle 2's own audit

**`page_side_hint_line` was not a transcription at all.** It joined with
`HINT_SEPARATOR` (` · `) and read `action.label` raw; the page joins hint spans with
a text space and puts labels through `hintLabel` (lowercase, leading `open`/`move`
stripped, trailing `mode` stripped). So it copied two rules from nowhere, and the
`HINT_SEPARATOR` pin defended a rule this file did not transcribe — F-55's condition
pointing the *other* way, at a pin with no copied rule behind it.

**Its assertion passed for the wrong reason.** `contains("Return")` matched the
physical hint `A / Return`, not the leave action's label, so it held whatever that
label said. The painted line is `D:utility A / Return:return`; it now checks the
`:return` half.

That second one is a shape rather than an instance: **an assertion that passes
because it matched the wrong half of a composed string.** It is the same family as
the anchor matching in more places than it names, and it is invisible to every check
this mission has built, because the assertion does fail when the whole string
changes — just not when the half it claims to test does.

## F-66 — F-43's NUL is load-bearing, not cosmetic

**Corrected by**: WP05 cycle 2
**Owner**: the merge step

F-43 recorded the NUL byte in `page.js` as a grep nuisance and assigned "replace with
a printable separator" to merge. That under-described it.

It is the **dedup-key separator** in `sideRegionHintLine`, and it works precisely
because no hint or label can contain it. Replacing it is therefore a semantic choice,
not a cosmetic one: the replacement must be a character no projected hint or label
can contain, or two distinct hint pairs could collide into one key and a hint would
silently vanish.

WP05's pins sit either side of the separator and the transcription dedups on the pair,
so the repair fires neither pin and needs no test update. But whoever makes the change
at merge must pick the character deliberately rather than reaching for `|`.

## F-67 — The unit half of T032 is thinner than the range half

**Raised by**: WP05 cycle 2, declining to hide it

`check_ranges_and_units_are_rendered` reads `control.unit` off the projection and
pins only the painting site. The unit's *text* is not read back through a transcribed
rule the way ranges and values are, because the page paints `String(control.unit)`
unmodified — there is no rule to transcribe.

That is honest and it is also asymmetric, and WP05 said so rather than manufacturing
a rule to make the two halves look alike. Recorded so the asymmetry is a known
property rather than something a later reader mistakes for thoroughness.
