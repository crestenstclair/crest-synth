---
affected_files: []
cycle_number: 3
mission_slug: functional-patch-editor-01KZERV9
reproduction_command: cargo test --all-targets
reviewed_at: '2026-08-09T05:40:00Z'
reviewer_agent: reviewer-renata
verdict: approved
wp_id: WP02
---

# WP02 review — cycle 2: approved

Reviewer: reviewer-renata. Verified in the lane worktree
`/Users/crestenstclair/workspace/crest-synth/.worktrees/functional-patch-editor-01KZERV9-lane-b`
at `2a9e545` (rework commit `f39a0d7` plus the coordination merge).

**Measured, not read:** `cargo test --all-targets` → exit 0, 777 passed / 0
failed / 2 ignored. `cargo clippy --all-targets -- -D warnings` → clean. The
lane's `.kittify/crest-spec` is byte-identical to the repo root and to the
mission branch; `spec-kitty crest-spec doctor` → OK (7 contexts / 135
resources). `tests/input_capture_witness.rs` did not stall on this run.

Every cycle-1 blocker is closed, and the pushback is right on all four points.

---

## B1 — the entry gate is airtight

Verified through a throwaway integration probe driven entirely by the public
API on a production-registry Braids Patch (probe deleted; lane left clean):

- `SemanticAction::surface_descriptor()` has 17 entries and does not contain
  `EnterSurface(PatchDetail)`; `is_phase_two_admitted()` is `false`.
- `accepts_semantic_action` is `false`; `SemanticResolver::valid_actions()`
  yields `[SelectContext(Patch), SelectContext(Mixer), Navigate(Down),
  Navigate(Right), SetInteractionMode(Navigate), SetInteractionMode(Adjust),
  EnterSurface(PatchUtility)]` — no detail entry.
- The projected `GraphicalShellProjection` footer reads
  `["2 OPEN PATCH", "1 OPEN MIXER", "S MOVE DOWN", "D MOVE RIGHT",
  "RELEASE K NAVIGATE MODE", "HOLD K ADJUST MODE", "D OPEN UTILITY"]` — no
  detail hint — and the projected `validActions` list matches the resolver's.
  The chain is structurally closed: `action_hints` derives from
  `valid_actions()`, which filters `surface_descriptor()`, which no longer
  lists the action.
- `apply_semantic_action(EnterSurface(PatchDetail))` →
  `Err(EventRejection::ActionUnavailableInContext)`, generation unchanged,
  state compares equal to its pre-call clone.

And the reducer still owns the surface: `AppEvent::EnterSurface(PatchDetail)`
lands `active_surface == PatchDetail` with `detail_subject == Some` and a
return path whose `entered_surface` is `PatchDetail`; `AppEvent::Return`
restores the exact origin and clears the subject. Splitting `is_return_target`
(structural: every non-main surface) from `is_enterable` (admission) is the
right seam, and `is_enterable` is consulted at exactly two production points —
`SemanticAction::is_phase_two_admitted` and the demo scene's expected-coverage
builder.

## The gate cannot become permanent

Falsified by flipping the `Self::PatchDetail => false` arm to `true` and
running `cargo test --lib`. Six tests fail, including all three the rework
names:

- `control::app_state::the_entry_gate_exists_because_a_braids_detail_state_cannot_yet_be_projected`
  (fails at the `!accepts_semantic_action` assertion; its closing
  `project_with_shell_tree(...).is_err()` is the assertion that inverts when
  WP03 makes a detail focus projectable),
- `control::semantic_focus::the_detail_surface_is_a_return_target_but_is_not_offered_until_wp03`,
- `control::semantic_action::semantic_action_descriptors_are_closed_unique_and_phase_two_safe`,
- plus `control::app_state::the_detail_surface_is_reducer_owned_but_not_offered_until_wp03`,
- plus `testing::exhaustive_gui_demo::exhaustive_gui_demo_scene_uses_production_seams_and_has_no_coverage_gaps`
  and `shell::standalone_application::standalone_exhaustive_gui_demo_composes_a_complete_production_trace`,
  which fail with `missing: ["surface.detail"]`.

All three named tests are genuinely coupled, not decorative.

## B1a / B1b — falsified

Removing `self.leave_stale_detail_surface();` from `repair_semantic_paths`
fails exactly two tests and no others:
`an_engine_swap_under_an_open_detail_entry_leaves_the_surface` and
`clearing_the_subject_slot_under_an_open_detail_entry_leaves_the_surface`,
both at the `detail_subject() == None` assertion. `detail_subject_is_live` asks
the right question — `resolves()` cannot, because the detail order resolves
from the *subject's* capability id. Coverage is complete: `repair_semantic_paths`
is reached from both structural commit arms that can invalidate a PATCH detail
subject (`ReplaceCapability`/`ReplaceParameterChoice` at `app_state.rs:1272`,
`SetSlotOccupancy` at `:1479`), and the repaired origin is computed before the
surface is left, so leaving never lands on a row the new schema cannot host.

## B2 / B3 — closed

Deleting `"surfaces[].summary.subject.slot_id"` from
`SERIALIZED_LEAF_DESCRIPTOR` fails the bidirectional exactness test with
`discovered-only=["graphicalShell.semanticModel.surfaces[].summary.subject.slot_id"]`,
so the third leaf and the Effect-subject fixture are both load-bearing.

All twelve `pub(super)` mutators on `InteractionState` end in
`assert_detail_invariant` (audited mechanically, not by eye).
`replace_return_origin` returns `FocusPathError` rather than assigning `None`.
The type docstring's narrowed claim is accurate: `active_focus` is assigned by
the reducer at exactly two sites, `navigate_side_nonwrapping` and
`repair_inspector_focus`, and both are guarded by the surface they operate on.
The `include_str!` scan now calls itself a text tripwire and names the
reducer-driven test as the proof.

## The four pushbacks — all upheld

1. **Option 1 was mis-sized. Confirmed by execution.** I applied the
   containment fix alone (resolving the detail order for the check) and
   re-projected a Braids detail state: the failure moved from
   `PatchPage(InvalidInstrumentConfig)` to `StateProjectionError::InvalidSelection`,
   raised by `render_patch_text`'s `selected_line.ok_or(...)`. The mechanism is
   `patch_page_projection.rs`: `control_id` is `Some` only when
   `patch_interaction() == StructuralChoice && enabled && visible`, and Braids'
   three rows are `ParameterUpdate::Scalar`, so the page carries no line the
   detail focus can select. WP03's T015 is correctly scoped as three coupled
   changes, and reverting the partial fix was the right call.
2. **B2's leaf list was incomplete. Confirmed.** Three leaves, two fixtures.
   My instrument-subject fixture could not have discovered `subject.slot_id`.
3. **The `with_voice_limit` enumeration was partial. Confirmed.** At the
   cycle-1 base `1ede2a2`, `Patch::with_voice_limit` had five test-only callers
   (`patch.rs:750`, `:762`, `audio_renderer.rs:1472`, `:1500`,
   `parameter_snapshot.rs:1242`), not two. The deletion ruling stands; the
   enumeration did not. `RtPatchParameters::with_voice_limit` is correctly
   untouched and still production-called.
4. **The gate costs the demo scene its `surface.detail` steps. Confirmed** by
   the two coverage failures above.

---

## Carry-forward — not blocking, for the orchestrator and WP03/WP06

- **F-11 under-states what gate removal breaks.** It says WP03's removal
  "restores the expectation automatically". Measured, removing the gate turns
  six tests red, two of which (`exhaustive_gui_demo`, `standalone_application`)
  can only be made green by restoring the two `surface.detail` scene steps in
  `src/testing/demo_scene.rs` — which is **WP06's** owned surface, not WP03's.
  And `is_enterable` itself lives in `src/control/semantic_focus.rs`, which is
  **WP02's** owned file, not WP03's either. WP03 cannot land a green lane by
  removing the gate alone. Settle the ownership before WP03 starts: either
  widen WP03's map to cover `semantic_focus.rs` and the two scene steps, or
  sequence the gate removal into a package that owns both.
- **FR-012 is not user-reachable after WP02.** The detail surface exists at
  the reducer seam and is proved there, but no player can enter it until WP03
  removes the gate. Same shape as F-01: WP05 and the mission review must not
  credit FR-012 to WP02.
- **Two stale rustdoc links.** `src/synth/patch.rs:94` and `:106` still
  reference the deleted `Patch::installed`; `:106` reads "`Self::installed`
  does both at once", describing a constructor that no longer exists.
  `cargo doc` now emits two new `broken_intra_doc_links` warnings. Rustdoc is
  not gated, so this is doc rot rather than a build break — fix it in the next
  commit that touches `patch.rs`.
- **`PatchDetailSubject` serializes snake_case field names into an otherwise
  camelCase schema.** `#[serde(tag = "kind", rename_all = "camelCase")]`
  renames variants, not fields, so the trace now carries
  `interaction.detailSubject.capability_id` and `.slot_id` beside `patchId`,
  `modalId`, `enteredSurface`. No declared invariant mandates camelCase and no
  guard enforces it, and the shape was already visible at cycle 1 in
  `surfaces[].summary.subject.capability_id` where I did not flag it — so this
  is not a goalpost move and not this package's to redo. But it is now frozen
  at `StateTree::SCHEMA_VERSION = 14`. WP03 is already going to move the
  semantic leaf descriptor and bump the version; fold the casing fix in there
  rather than paying a second version for it later.

## Anti-pattern checklist

1. Dead code — **PASS** (`Patch::installed` and `Patch::with_voice_limit`
   deleted; `is_return_target`, `detail_subject_is_live`,
   `leave_detail_to_origin` all have production callers;
   `VoiceLimit::surface_descriptor` kept under an explicit cycle-1 ruling with
   its reason now in code).
2. Synthetic-fixture test — **PASS** (every new test drives `AppState::apply`
   and the production `StateProjector`; B1a/B1b and the leaf-descriptor
   exactness test were each falsified against the implementation).
3. Silent empty return — **PASS** (`replace_return_origin` no longer swallows
   its failure; `leave_stale_detail_surface`'s early return is the documented
   no-detail-open case).
4. FR coverage — **PASS**, with the FR-012 caveat recorded above.
5. Frozen surface — **N/A**.
6. Locked decision — **PASS**.
7. Shared-file ownership — **PASS** (out-of-map edits ruled in cycle 1;
   `.kittify/crest-spec` was authored in-lane, the implementer flagged it, and
   the orchestrator has since adopted the same content at the repo root — lane,
   repo root, and mission branch are byte-identical and `doctor` passes).
8. Production fragility — **PASS** (`leave_detail_to_origin`'s `expect` is
   guarded by the detail invariant that the same type's twelve mutators all
   assert; `AppState::apply` reduces into a clone and only commits on success,
   so no partial mutation can escape a rejected event).

**Verdict: approved.** WP03 unblocks.
