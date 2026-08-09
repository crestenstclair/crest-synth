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

## F-19 — Read-only sections do not exist in the type system

**Raised by**: WP03
**Owner**: WP05, before it writes T033's bullet 8

T015's bullet said "a read-only section projects `editable: false`".
`CapabilitySection` has no read-only flag — the concept does not exist. WP03
derived `editable` from what the reducer actually accepts, and since
`adjust_patch_control` refuses every `Adjust` on `PatchDetail`, *every* detail
control projects `editable: false`. It called the bullet vacuously satisfied and
asserted the reducer's refusal instead, which is the honest reading.

**This bears directly on the analysis pass's finding C3.** WP05's T033 bullet 8
requires asserting that a production path produces a read-only surface summary,
and the spec's Scope Decisions table claims FR-012 closes that control kind. If
the concept does not exist in a descriptor, that assertion may be unsatisfiable —
in which case T033's own instruction applies: say so plainly and withdraw the
claim from the Scope Decisions table rather than manufacturing a fixture that
exists only to satisfy it.

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
