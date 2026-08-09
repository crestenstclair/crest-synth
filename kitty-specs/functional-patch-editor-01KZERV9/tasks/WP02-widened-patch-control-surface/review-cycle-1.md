# WP02 review — cycle 1: changes requested

Reviewer: reviewer-renata. Verified in the lane worktree
`/Users/crestenstclair/workspace/crest-synth/.worktrees/functional-patch-editor-01KZERV9-lane-b`
at commit `1ede2a2`.

**What I confirmed by running it:** `cargo test --all-targets` → exit 0, 771
passed, 0 failed (matches your report exactly). `cargo clippy --all-targets --
-D warnings` → clean. The crest-spec in the lane is byte-identical to the
repo-root copy and `contexts/realtime.yaml` names `voiceLimit` in the
leaf-descriptor enumeration.

Most of this package is right, and several parts are better than the prompt
asked for. The blocker below is one thing, found by execution, not by reading.

---

## BLOCKING — B1. Entering `PatchDetail` produces an accepted state the production projection cannot project, and `AppLoop` panics on it

This is the same rule you invoked to justify editing WP03's files —
"`app_loop.rs:347` panics on an accepted state without the projection". You
applied it to the five Utility rows and not to the surface you introduced.

Reproduced through the admitted semantic-action vocabulary only, on a single
Braids Patch with the production registry:

```
apply_semantic_action(SelectContext(Patch))          -> Ok
apply_semantic_action(EnterSurface(PatchDetail))     -> Ok   (accepted, generation advanced)
StateProjector::new().project_with_shell_tree(&state)
    -> Err(PatchPage(InvalidInstrumentConfig))
```

`src/control/app_loop.rs:347` does
`.expect("an accepted AppState must produce coherent projections")` on exactly
that call, so this is a panic in the production loop, not a returned error.

The mechanism is `src/control/patch_page_projection.rs:959-968`:

```rust
let resolved_controls = if focused_control_id.is_utility() {
    PatchControlId::utility_surface_descriptor().to_vec()
} else {
    state.focused_patch_controls()...
};
if !resolved_controls.contains(&focused_control_id) {
    return Err(PatchPageProjectionError::InvalidInstrumentConfig);
}
```

A `PatchDetail` focus is in neither order. Braids' PatchMain order is
`[Engine, Attack, Decay, Sustain, Release, EffectSlot(0..2)]` — no `Capability`
rows at all — while the detail order is
`[Capability(braids.model), Capability(braids.timbre), Capability(braids.color)]`.
Nothing overlaps, so the projection rejects.

**Your tests and the demo scene pass only by coincidence.** Every place that
opens the detail surface today does so on the SoundFont engine row, whose
subject's first row is `Capability(soundfont.preset)` — which *is* a
StructuralChoice and therefore *is* in the PatchMain order. Swap the fixture to
Braids and the same code path fails. That coincidence is precisely why "read
test bodies, don't accept a validation bullet because a test with a matching
name exists" matters here: `detail_entry_is_accepted_only_from_a_row_that_
resolves_a_subject` never projects, and the one place that does project
(`demo_scene.rs` `surface.detail.entered`) runs on the lucky engine.

Two further reachable variants of the same hole, both also reproduced:

**B1a — engine swap commits under an open detail entry.** Request an engine
change on the Engine row, enter `PatchDetail`, then let `EnginePrepared`
arrive. Result: `subject = Instrument(instrument.soundfont.hidef)` while the
Patch's active capability is `instrument.braids`. `detail_invariant_holds()`
returns `true` (all three fields still agree), `resolves()` returns `true`
(because `patch_detail_paths` looks the descriptor up by the *subject's* id,
not the Patch's), and the projection fails.

**B1b — effect slot cleared under an open detail entry.** Focus an occupied
slot row, request clearing it, enter `PatchDetail`, then let `TopologyPrepared`
arrive. Result: `subject = Effect { slot_id: 1, capability_id: "effect.chorus" }`
with slot 0 now empty, `resolves()` false, projection fails.

B1a and B1b are also a direct violation of the declaration. From
`valueObject.Control.PatchDetailSubject` in
`.kittify/crest-spec/contexts/control.yaml`:

> a subject whose capability leaves the registry, or whose slot is cleared, is
> not repaired into a neighbouring subject; the detail surface is left through
> the deterministic focus resolver back to its origin, because silently
> retargeting a detail view would show one capability's values under another's
> title

`repair_semantic_paths` (`app_state.rs:2440`) repairs the two remembered roots
and one return *origin* — all main paths by construction — and never touches an
open detail entry. It must leave the detail surface when the subject stops
naming a live capability or slot.

**What I want, minimally:**

1. `patch_page_projection.rs::project` must handle a `PatchDetail` focus —
   resolve the detail order for the containment check the same way you made it
   resolve the Utility order. This is the same size and shape as the edit you
   already made in that file and stays inside the rationale you already used.
2. `repair_semantic_paths` (or the structural-commit arms that call it) must
   leave the detail surface when the subject no longer resolves, restoring the
   origin through the deterministic resolver, per the declaration quoted above.
3. Add a test that opens the detail surface on a **Braids** Patch and projects
   it, and tests for B1a and B1b that assert the surface was left. A test that
   only ever enters detail on SoundFont does not discriminate.

If you judge that (1) properly belongs to WP03's detail projection, the
alternative is to hold `SurfaceId::PatchDetail` out of `is_enterable()` (and so
out of the admitted `SemanticAction` vocabulary) until WP03 lands. What is not
acceptable is shipping an action the reducer advertises as valid
(`accepts_semantic_action(EnterSurface(PatchDetail))` returns `true`, and it
reaches the projected `validActions` and footer hints) that panics the loop
when taken. Pick one and say which in the commit message.

---

## SHOULD FIX IN THIS PACKAGE

### B2. The detail surface's serialized leaves are unenumerated

With a detail entry open, a recursive leaf sweep of the projected
`SemanticGraphicalViewModel` discovers two leaves that
`SemanticGraphicalViewModel::SERIALIZED_LEAF_DESCRIPTOR`
(`src/control/semantic_graphical_view_model.rs:496-509`) does not declare:

```
surfaces[].summary.subject.capability_id
surfaces[].summary.subject.kind
```

The crest-spec's `StateTree` invariant requires
"serializedLeafDescriptor exactly equals recursively discovered JSON leaf paths
in both directions for a discriminating multi-Patch tree". The existing
exactness test passes only because the discriminating tree never opens a detail
surface. This is the same defect class as the mission's own F-03 — an
enumeration that silently stopped covering the value it describes — so please
close it here rather than let it reach mission review.

### B3. The invariant docstring overclaims, and one mutator can break it

`src/control/interaction_state.rs:72-73` says "Every mutator ends by asserting
[`Self::detail_invariant_holds`]." Six of the eleven `pub(super)` mutators do
not: `set_mode`, `initialize_patch_focus`, `leave_subordinate`,
`replace_remembered_patch_main`, `replace_remembered_mixer_main`, and
`replace_return_origin`.

`replace_return_origin` is the one that matters:

```rust
pub(super) fn replace_return_origin(&mut self, origin: FocusPath) {
    if let Some(return_path) = self.return_path.as_ref() {
        self.return_path = ReturnPath::new(origin, return_path.entered_surface()).ok();
    }
}
```

`.ok()` turns a construction failure into `return_path = None` while
`detail_subject` stays `Some` — an invariant violation with no assertion to
catch it. I could not reach it (origins are always valid main paths in the same
context), so this is latent, not live. But a false rigor claim in a docstring
is exactly what this mission recorded as F-04 and made you delete a test over.
Either assert in every mutator, or narrow the claim to what is true.

Separately, on the T011 claim itself: making `detail_subject` private while its
siblings are `pub(super)` is a real and good structural improvement, and I
could not construct a violating state through any reachable path — I tried
patch switch with detail open, context switch with detail open, both nesting
directions, engine swap under detail, and slot clear under detail. But
`active_focus` is still assigned directly from the reducer at
`app_state.rs:2128` and `:2534`, so "the reducer cannot assign it" is true of
one field of three. Both of those writes are currently safe (each is guarded by
the surface it operates on); please say so accurately rather than claiming more
than the code enforces.

---

## RULINGS ON F-02 (dead API disposition)

1. **`Patch::installed` — delete now.** Verified zero non-test callers; the only
   one is `src/synth/patch.rs:798`, inside `mod tests`. `install_patches`
   composes `Patch::new` + `seed_voice_limit` because it needs the registry
   lookup *between* them, so `installed()` cannot serve that site and never
   will. Two ways to build an installed Patch, one of which the reducer cannot
   use, is worse than one.
2. **`Patch::with_voice_limit` — delete now.** Verified zero non-test callers
   (`patch.rs:750`, `:762`, both in `mod tests`). `set_voice_limit` is
   `pub(crate)` and is the real path; the app_state tests already use it
   directly (`app_state.rs:4646`). Note the identically named method on
   `RtPatchParameters` (`parameter_snapshot.rs:382`) *is* production-called —
   do not touch that one.
3. **`VoiceLimit::surface_descriptor()` — keep, and state the reason in the
   code.** It exists so the value satisfies the descriptor contract the
   crest-spec declares for it: "the descriptor ... enumerates the field exactly
   once ... the same descriptor contract the VoiceEnvelope and GlobalParameters
   surfaces already hold." Both of those have production callers of their
   `surface_descriptor()`; `VoiceLimit`'s does not only because it has one
   field. `VoiceLimit::descriptor()` is now consumed in three production sites
   (`app_state.rs:1908`, `patch_page_projection.rs:236`,
   `semantic_graphical_view_model.rs:1172`), which is what F-02 asked for. Add
   a one-line doc comment saying why the single-element array stays, so mission
   review does not re-litigate it.
4. **`VoiceLimitCarryOver`'s `Preserved`/`Clamped` discriminant — assign it to a
   later WP by name, or delete it.** Today `app_state.rs:1265` binds the
   carry-over and uses it only inside a `debug_assert_eq!`; the discriminant is
   discarded. The fact it carries — "the engine you just chose narrowed your
   limit from N to M" — is user-visible and belongs on a projection. Name the
   package that will consume it (WP03's engine-swap status presentation is the
   natural home) in `cross-wp-findings.md`, or change the return type to
   `VoiceLimit` and delete the enum. Do not let it survive to mission review as
   "exists for a future consumer" a third time.
5. **`seed_voice_limit` and `replace_instrument_config` — F-02 closed.** Both
   now have real production callers (`app_state.rs:964` and `:1265`), and H1/H2
   prove them against both real production descriptors.

---

## VERIFIED AND ACCEPTED — no action needed

- **T009 single master-gain owner.** Read both call sites. `app_state.rs:1730`
  (PATCH Utility arm) and `app_state.rs:2170` (MIXER Inspector arm) both call
  `self.adjust_global(parameter, direction)`; `adjust_global` (`:2419`) reads
  and writes `self.global` and nothing else. No PATCH-side field exists: only
  `GlobalParameters` declares `master_gain_db`, `Patch` does not,
  `RtPatchParameters` does not, and `StateTree`/`serialized_state` carry it only
  under `global`. `master_gain_is_one_canonical_value_reached_from_patch_and_mixer`
  drives both edits through `AppState::apply` and asserts each moved by the
  descriptor's own fine step. This is the claim I expected to be silently wrong
  and it is not. One note: the companion test at `:4191` counts source lines
  with `include_str!` — that is a text grep, not a structural check, so do not
  cite it as the proof; the reducer-driven test above is the proof.
- **T007/T008.** The five-row order is written once
  (`PatchControlId::UTILITY`), `is_utility()` is the single split predicate and
  `FocusPath::validate` and both resolvers all read it, and
  `tests/schema_surface.rs:405-412` round-trips all three new variants against
  their declared paths. `focus_utility_row` navigates through the production
  reducer rather than assigning focus.
- **H3 schema bump and the offset bug.** The fix is real, not a second latent
  form. `MidiTreeTemplate::from_json` now parses the version out of the document
  and checks it, and derives `root_start` from the found offsets. I read the
  rest of that function: every other marker was already `.find()`-derived, so no
  length coupling remains anywhere in it. The 12→13 bump is justified — the tree
  genuinely gains a leaf.
- **F-04.** Deleting the tautological test and naming the two stronger
  neighbours in a comment at the deletion site is the right call.
- **The four rebased baseline captures are genuine latent defects, not tests
  bent to fit.** `tests/engine_selection_workflow.rs` captured
  `untargeted_patch.clone()` *before* install; what that observation is about is
  "the engine swap leaves the untargeted Patch alone", and installation
  legitimately reseeds the limit. `tests/per_voice_envelope.rs` has the same
  shape. Both now capture from `state.patches()`. Correct, and correctly
  explained in the comments.
- **The one-way clamp and the demo-scene restoration.** Restoring the limit
  through the Utility row rather than excusing the field is the honest choice —
  see the carry-forward item below for the part that is not yours.

---

## OUT-OF-MAP EDITS — ruling

Forced and minimal, no action: `state_projector.rs` (test-only line-offset
derivations), `webview-page/page.js` (three `driver: null` → real driver paths,
now true and required by `AUTHORED_UTILITY_ENTRIES`), `src/testing/*` (the
scenes assumed Utility adjacency; `utility_row_distance` derived from the
declaration is the right fix).

Forced but **incomplete**: `patch_page_projection.rs` and
`semantic_graphical_view_model.rs`. The Utility half was genuinely forced. The
detail half was not — nothing forced you to project the detail surface, and
projecting it in the semantic model but not the patch page is what produced B1.
Close B1 and B2 and this becomes a clean handoff.

Things WP03/WP04/WP06 must now reconcile rather than build fresh — please record
these in `cross-wp-findings.md`:

- **WP03** inherits `SemanticSurfaceRole::Detail`, `SemanticSurfaceSummary::PatchDetail`,
  the `project_patch_surfaces` detail branch, and the `is_utility()`-based
  `resolved_controls` split in `patch_page_projection.rs`. Its detail projection
  must extend these, not replace them, or the leaf descriptor moves twice.
- **WP06** inherits `push_patch_utility_scalar_steps`,
  `push_voice_limit_restoration_steps`, and the `surface.detail` scene step. The
  restoration helper hardcodes `BRAIDS_FIXED_VOICES` knowledge of the engine
  journey; revisit when WP06 owns the scene. The `surface.detail` step is only
  safe today because of the SoundFont coincidence described in B1 — it must not
  be repointed at a Braids Patch until B1 is closed.
- **WP04** inherits nothing it must redo.

---

## CARRY FORWARD — not blocking, record in `cross-wp-findings.md`

- **F-07: the engine-swap carry-over asymmetry is undeclared.** Clamp on
  narrowing, preserve on widening, so the narrowing is one-way. I searched the
  crest-spec: `valueObject.Synth.VoiceLimit` declares the bound, the installation
  seed, and the refusal semantics; `aggregate.Synth.Patch` declares the
  installation seed. **Nothing declares what happens to the limit on an engine
  swap.** `replace_instrument_config`'s clamp was WP01's invention and your H2
  prompt told you to apply it in canonical state, so this is not yours to fix —
  but the demo scene now works around a product behaviour the declaration never
  took a position on. Record it for deliberate resolution before mission review.
- **F-08: `detailSubject` is invisible in the trace.** The crest-spec's
  `StateTree` declares `interaction: '{focusPath, rememberedPatchPath,
  rememberedMixerPath, mode, returnPath}'` — no `detailSubject` — so
  `SerializedInteractionState` omitting it is *correct against the declaration*
  and you were right not to invent it. But the exact argument that got
  `voiceLimit` enumerated in F-03 applies: the subject decides what the surface
  shows, and no measured proof can correlate it. Amend the declaration
  deliberately or record why not.
- **F-09: two crest-spec bullets disagree about SelectPatch focus recovery.**
  `aggregate.Control.InteractionState` says a switch recovers "deterministically
  to its first valid control", while the preceding bullet and T012 both say the
  next-before-previous sibling rule. You implemented the sibling rule with
  first-row as final fallback, which is what T012 demanded and is a strict
  improvement. Resolve the disagreement at crest-spec level rather than in code.
- **F-10 (pre-existing, newly more visible):** `select_patch` reads
  `patch_control_focus()`, which on `PatchUtility` or `PatchDetail` returns the
  *subordinate* surface's control rather than the remembered main one, so a
  switch made from Utility recovers against a control the main order never
  hosts and falls through to the first row. Unchanged by WP02, but the five-row
  Utility panel and the detail surface make it reachable in more ways.
