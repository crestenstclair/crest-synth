# WP03 review — cycle 1: changes requested

Most of this package is strong and stands. The `Arc` change is justified and I
reproduced its measurement. The gate is genuinely gone and I verified it on
Braids by execution. Both recorded falsifications are real and I reproduced both
verbatim. The lane is green in debug, clippy-clean, fmt-clean.

Two things must change before approval, and both are the same failure mode this
mission has now hit four times (F-13): **a guard whose fixture can only see one
variant, reported as satisfied.**

---

## B1 (blocker) — T016's guard does not cover `PatchPageProjection`, and I proved it

T016 names two production files and you fixed the defect in both:

- `src/control/semantic_graphical_view_model.rs:1762` — MIXER Inspector global row
- `src/control/patch_page_projection.rs:212` — the PATCH page's master-gain row

Your falsification covered the first one. I reproduced it verbatim:

```
Mixer(Global { parameter: MasterGainDb }) on MixerInspector is labelled with
the serialization key masterGainDb
```

I then ran the same falsification on the second one. I reverted
`src/control/patch_page_projection.rs:212` from `descriptor.label().to_owned()`
back to `descriptor.name().to_owned()` — reintroducing `masterGainDb` as the
PATCH page's on-screen master-gain label — and ran `cargo test --release
--no-fail-fast` across every target.

**Nothing failed.** 697 lib tests passed, every integration target passed. The
only failure in that run was `adapter::lock_free_structural_graph_boundary::
tests::status_is_latest_wins_coherent_and_never_backpressures_audio`, which is a
release-only pre-existing flake unrelated to this package (see N4).

So one of the two production sites T016 fixed has zero test coverage and can be
silently reverted. That fails three of T016's own bullets:

- [ ] "The guard fails when a key is deliberately reintroduced as a label —
  verify by doing it, then reverting" — verified on one site of two.
- [ ] "The guard covers PATCH, PATCH Utility, detail, MIXER, and MIXER
  Inspector, not just the row that was reported" — covered on
  `SemanticGraphicalViewModel` only. `PatchPageProjection` is never walked, and
  it is the projection with section labels, which nothing checks at all.
- Definition of Done: "No serialization key reaches any projected label, guarded
  by a set check."

Your own T019 rule applies verbatim here: a test that passes with and without
the code it claims to prove is not a proof.

**What to do.** Extend `no_projected_label_on_any_surface_is_a_serialization_key`
(or add a sibling) to walk `PatchPageProjection` over the same fixture set:
`page.output()[].label`, `page.sections()[].label` and its
`parameters()[].label`, `page.effects()[]` labels, and
`page.detail().sections()[].label` / `.parameters()[].label`. Then falsify it on
`patch_page_projection.rs:212` the way you falsified the other site, and record
the observed failure.

While you are there, the fixture only ever projects Patch 1 / SoundFont.
`mixed_state()` installs Braids and a chorus occupant, but neither test applies
`SelectPatch`, and no effect detail subject is ever opened — so no Braids label
and no effect-detail label is checked on any surface. The key set is wider than
the surfaces walked, which is the safe direction, but a Braids descriptor whose
`label()` equalled its `id()` would ship. Add the Braids patch and an effect
detail subject to the loop; you already have `mixed_state()` and it costs two
lines.

---

## B2 (blocker) — T015's read-only ruling rests on a factual claim that is wrong

You reported: *`CapabilitySection` has no read-only flag, so the concept does not
exist in the type system*, and concluded the bullet is vacuously satisfied.

The concept exists, and it is load-bearing:

- `PatchInteraction::ReadOnly` — `src/synth/instrument_capability.rs:146`
- Declared in the crest-spec: `valueObject.Synth.ParameterSpec.patchInteraction`
  (`.kittify/crest-spec/contexts/synth.yaml:168`), with the
  `CapabilityDescriptor` invariant "installed instrument descriptors in this
  increment use StructuralChoice or ReadOnly" (`synth.yaml:228`)
- It is the **default** of `ParameterSpec::new`
  (`src/synth/instrument_capability.rs:335`)
- The production fixture declares it: **all three Braids rows**
  (`src/adapter/braids_capability.rs:232`, `:312` both use `ParameterSpec::new`)
  and SoundFont's `file` row (`src/adapter/hidef_soundfont_capability.rs:57`,
  asserted at `:148`), while SoundFont's `preset` row is `StructuralChoice`. That
  is a discriminating pair on one descriptor.
- The same file you wrote the claim about already derives `editable` from it at
  three sites: `semantic_graphical_view_model.rs:1251`, `:1329`, `:1751`.
- **This same commit projects it.** `patch_page_projection.rs:148` sets
  `patch_interaction: spec.patch_interaction()` on every detail row, and it is in
  the leaf descriptor at `detail.sections[].parameters[].patchInteraction`.

I confirmed by execution against a production Braids detail projection:

```
PROBE detail row: section=oscillator id=braids.model label="Model" patchInteraction=ReadOnly editable=false
PROBE detail row: section=oscillator id=braids.timbre label="Timbre" patchInteraction=ReadOnly editable=false
PROBE detail row: section=oscillator id=braids.color  label="Color"  patchInteraction=ReadOnly editable=false
```

The code is fine. **The record is not**, and the record is what WP05 inherits.
T033 bullet 8 has an escape hatch — "if the fixture's installed capabilities
declare no read-only section, say so plainly and withdraw the claim from the
Scope Decisions table". Your write-up hands WP05 exactly the false premise that
would trigger that withdrawal. The fixture does declare read-only sections, the
claim must not be withdrawn, and the production producer is
`patchPage.detail.sections[].parameters[].patchInteraction` — not `editable`.

Separately, on `SemanticGraphicalViewModel` the detail surface's `editable` is a
hardcoded `false` (`semantic_graphical_view_model.rs:1510`). That is defensible
as a statement about what the reducer accepts, and I verified the refusal is
real. But it carries no information: a `StructuralChoice` row and a `ReadOnly`
row project identically, so
`detail_controls_project_editable_false_because_the_reducer_refuses_to_adjust_them`
cannot discriminate anything, and a page cannot mark a capability-declared
read-only section "in text or shape" from the semantic model alone.

**What to do** — either is acceptable, pick one:

1. Carry the declared interaction onto `SemanticControlViewModel` for detail rows
   so the semantic model is as informative as the page, and assert a Braids row
   (`ReadOnly`) and SoundFont's `preset` row (`StructuralChoice`) project
   differently; or
2. Leave the code exactly as it is and correct the written record: state that
   `editable: false` is a surface-level fact about the reducer, that the
   capability-declared read-only fact lives on `PatchPageProjection` at
   `detail.sections[].parameters[].patchInteraction`, and that WP05's T033
   bullet 8 is satisfiable there. Add a note to `cross-wp-findings.md` so WP04
   and WP05 read the right thing.

Do not leave "the concept does not exist in the type system" in the record.

---

## Accepted as submitted

**T013's substituted bullet.** "A disabled row's list excludes blocked actions"
has no reachable disabled row in the production fixtures, and your substitute —
the engine row offers `EnterSurface(PatchDetail)`, an envelope row does not,
neither focused — is a genuine per-row discrimination, not a weaker restatement.
I confirmed by execution that the engine row's own list on a Braids Patch carries
`EnterSurface(PatchDetail)`. `an_unfocused_row_at_its_boundary_excludes_the_
direction_that_would_exceed_it` is the stronger half and it is real: it drives a
MIXER track to its ceiling, moves focus off it, and asserts the *unfocused* row
excludes both increase directions and keeps the decrease. Substitution accepted.

**F-16's ruling.** I confirmed the mixed state is confined to the schema and
breaks nothing at runtime. The two leaves you renamed —
`interaction.detailSubject.{capabilityId,slotId}` and
`surfaces[].summary.subject.{capabilityId,slotId}` — have no reader in
`webview-page/page.js`; the page reads `summary.patch_name`,
`summary.capability_id`, `summary.patch_id`, all still snake_case and all
untouched. `typed_descriptors_and_discovered_serialized_leaves_are_bidirectionally_exact`
is green at version 15, so the declared descriptor matches emitted JSON exactly.
Serialize and Deserialize both carry `rename_all_fields`, so the in-process
round trip is symmetric. WP04 inherits the remaining two.

**F-07's transient report.** Accepted as satisfying "reported rather than left
for a player to discover". The voice-limit row is a PATCH Utility row and the
Utility panel is a persistent side region rendered beside PATCH Main, so the
`64 → 16` pair is on screen for the duration of preparation, announced *before*
the loss is applied. Extracting `VoiceLimitCarryOver::resolve` so the projection
and the commit read one pure function is the right shape and closes a real drift
risk. Your residual is correctly identified and correctly placed outside this WP
— see N1.

**Out-of-map edits.** Each one checked and each one forced:
- `app_loop.rs` (2 lines) — `const fn` is impossible through `Arc` deref. Forced.
- `global_parameters.rs` — the authored label had to exist somewhere; T016 asks
  for "the descriptor's authored label" by name. Forced and minimal.
- `tests/exhaustive_demo_scene.rs` — the seventh red test nobody enumerated. Real,
  and the fix is the honest one (restore `surface.detail`, not weaken the check).
- `tests/schema_surface.rs` — schema bump. Forced.
- WP02's five files and WP01's `patch.rs` — both packages approved and closed.
  `patch.rs` also cleared F-17's stale rustdoc links as a side effect, which is
  what F-17 asked for.
- **`make fmt-check` was genuinely red at the lane base.** I did not take your
  word for it: I extracted `src/testing/live_effects_and_buses_scene.rs` and
  `tests/topology_change_lifecycle.rs` at `838e1c8` and ran
  `rustfmt --check --edition 2021` on them. Both fail at base, in exactly the
  hunks you changed, and your edits are byte-identical to rustfmt's output. See
  N3 for the one caveat.

---

## Non-blocking notes to carry

**N1 — `VoiceLimitCarryOver::Clamped { previous }` still has no production
consumer.** F-02 item 4 is now *partially* closed: the discriminant is consumed
by the projection through `resolve`, but `previous` is read nowhere in
production. It is exactly the field a durable record of the loss would need.
Record this against F-02/F-07 rather than deleting the field.

**N2 — `state_projector.rs` (an owned file) still composes serialization keys
into a projected screen string.** T016 step 1 asked for an audit of every
label-producing site in the files you own. `ShellFooter::path_label`, built at
`state_projector.rs:572` and `:635`, was not audited. Executed:

```
"MIXER / GLOBAL / masterGainDb"        "PATCH / patch.voiceLimit"
"MIXER / RETURN B0 / returnLevel"      "PATCH / patch.output.trimGainDb"
"MIXER / T00 / send[1]"                "PATCH / patch.midiInput"
```

Not blocking, because `webview-page/page.js:1031` ignores `pathLabel` entirely
and composes its own breadcrumb from `focusIdentity(model)`, which reads the
authored `control.label`. So nothing reaches a player today. It does reach the
serialized projection at `graphicalShell.footer.pathLabel`. Flag it to WP04 so
the page is never re-coupled to it, and consider folding it into B1's widened
guard if it is cheap.

**N3 — the fmt fix touched an open package's file.**
`src/testing/live_effects_and_buses_scene.rs` is WP06's, and WP06 is not closed.
The change is four lines of pure rustfmt with zero semantic content and was
forced by your own lane gate, so I am not asking you to undo it — but WP06 must
be told, because it is the only out-of-map edit that lands in a live package's
map rather than a closed one.

**N4 — a release-only pre-existing flake, for the record.**
`adapter::lock_free_structural_graph_boundary::tests::status_is_latest_wins_
coherent_and_never_backpressures_audio` fails roughly 2 runs in 3 under
`cargo test --release` and passes in debug. WP03 touches neither that file nor
anything it depends on. The declared gate is debug and debug is green. Recorded
so the next reviewer does not chase it, alongside F-12.

**N5 — one comparand in T017's cross-projection check is a tautology.** At
`state_projector.rs:538`, `semantic.patch_identity()` resolves to
`state.interaction.active_focus.patch_id()` (via
`semantic_graphical_view_model.rs:783`, `:643`), which is the same expression as
`snapshot_patch` on the line above. That half can never fire. The
`page.patch().id()` half is real (the page derives from `patch_focus()`), and
`validate_data`'s set check over focus path + summaries + control paths is the
substantive guard and is genuinely non-trivial. Worth one line of comment
rather than a change.

**N6 — `with_counterfactual_focus`'s doc overstates one branch.** It says the
move "goes through the same `InteractionState` transitions the reducer itself
uses", but the persistent-side and detail branches assign
`candidate.interaction.active_focus` directly after `enter_surface`. That is
sound — the trailing `resolver.resolves(path)` and the `detail_invariant_holds`
assertion make an incoherent counterfactual unprojectable — but it is a third
direct-assignment site, and `interaction_state.rs:74` still says "the reducer
assigns `active_focus` directly at two sites". Fix the count or the sentence.

---

## What I verified by execution, for the record

- Full `cargo test --all-targets` (debug, the declared gate): **699 + 11 targets,
  0 failed**, 5:37 wall. `input_capture_witness` passed — F-12 did not fire.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean on the lane; **red at base `838e1c8`**,
  confirmed independently with `rustfmt --edition 2021`.
- **T019 falsification, reproduced verbatim.** Deleted the
  `retired_identities.remove(stale)` block from `ProjectionChannel::retire` and
  ran the T018 test:
  ```
  panicked at src/shell/webview/projection_channel.rs:1273:14:
  a verbatim ack for the re-pushed document must not be rejected:
  IdentityMismatch { generation: 1, field: "stateHash" }
  ```
  Identical to your Activity Log. The test is honest and well-built: it reaches
  the duplicate-generation state through the real public `push`/`forward_ack`
  API, asserts a *false rejection of a truthful ack* rather than a set being
  empty, and includes two guard-insensitive controls that close the "the window
  simply forgot generation 1" escape hatch. The diff is +85 lines, all inside
  `#[cfg(test)]`; no production line changed.
- **T016 falsification, reproduced verbatim** (see B1).
- **B1's counter-falsification** — the page-projection site, green with the
  defect reintroduced (see B1).
- **The gate, on Braids, end to end.** `EnterSurface(PatchDetail)` is in the
  model-level `validActions` *and* in the engine row's own per-row list on a
  Braids Patch; it is accepted through `apply_semantic_action`; the resulting
  state projects. The discriminating shape is real — Braids' main order is
  `[Engine, Envelope×4, EffectSlot×3]` with no `Capability` row, and the detail
  focus lands on `Capability(braids.model)`, absent from that order. The text
  projection's selected line resolves to the detail row itself:
  `> DETAIL_PARAMETER {"controlId":"patch.capability.braids.model",...}`. The
  tree carries `interaction.detailSubject = {"capabilityId":"instrument.braids",
  "kind":"instrument"}`. All four coupled changes landed: containment check,
  page `detail` block + `render_patch_text`'s `DETAIL_PARAMETER` line,
  `is_enterable`, and `SEMANTIC_ACTION_SURFACE_DESCRIPTOR` 17 → 18.
- **The `Arc` measurement, reconstructed** (release, production fixture):
  ```
  AppState::clone (Arc, shared)            0.69 us
  CapabilityRegistry::clone               14.72 us   (91% of the old clone)
  EffectCapabilityRegistry::clone          0.82 us
  reconstructed pre-Arc AppState::clone   16.24 us   -> 23x
  ```
  Your 86% / 32x is the same claim on a different machine. **Sound.**
- **The residual price, measured** (release, per accepted event, control thread):
  ```
  PATCH  SemanticGraphicalViewModel::project    630 us over 17 rows
  MIXER  SemanticGraphicalViewModel::project   2920 us over 89 rows
  ```
  See "for WP06" below.
- Single-resolver claim: `SemanticResolver::valid_actions`
  (`semantic_resolver.rs:387`) is the only computation site, filtering through
  `AppState::accepts_semantic_action`; `valid_actions` and `requested_value` are
  assigned in exactly one place (`project_control_intent`, svm.rs:949-950).
  `shell_frame_observation.rs:253` reads the model, it does not recompute. The
  bullet holds.
- Registries are never reassigned: no `self.capabilities =`, no `self.effects =`,
  no `Arc::get_mut`, no `Arc::make_mut` anywhere in `src/`. Two `Arc::new` sites,
  both at construction. No stale-`Arc`-outliving-a-swap path exists because there
  is no swap.
- `AppState` never reaches the real-time side (no reference in `src/real_time/`;
  "PreparedGraph never enters AppState"), so the atomic refcount drop cannot
  occur in the audio callback.

## Ruling on the `Arc` change

**It stands.** The crest-spec's operative invariant for
`aggregate.Control.AppState` is that each registry is "supplied at construction,
remains immutable" (`control.yaml:943`, `:945`). `Arc` honours both exactly:
construction-only, no interior mutability, no `make_mut`. `Arc<T>: PartialEq`
still compares contents, so every "a refused action leaves state identical"
assertion in this codebase means what it meant before. The word "own" in the
aggregate's purpose line is an aggregate-boundary statement about who validates
against the registries, not a Rust move-semantics requirement.

On the cheaper alternatives: memoisation has no sound key, because a row's list
depends on the focused control's identity and the resolver's only honest answer
runs the reducer. "Visible rows only" saves nothing — every projected row is a
visible row. A resolver that does not need a full state clone is the one real
alternative, and it is the one T013 step 2 forbids by name: `accepts_semantic_
action` clones and runs the *production* reducer precisely so the answer cannot
drift, and re-expressing it as a borrow-only predicate reintroduces exactly that
drift. Given a clone-the-reducer design was mandated, making the clone cheap was
the correct lever and the registry was the correct thing to share.

The 45% suite-time increase is the right price for the feature, but it is not
free and it is not only test time — see below.

## What WP04, WP05 and WP06 inherit

**WP04** — the read-only fact for the detail surface lives at
`patchPage.detail.sections[].parameters[].patchInteraction` (`"readOnly"` /
`"structuralChoice"` / `"scalarEdit"`), **not** at `editable`, which is
uniformly `false` on every detail row. FR-012's AC-3 ("explicitly marked
read-only in text or shape, not by colour alone") is satisfiable only from
`patchInteraction`. F-16's remaining two casing defects are yours:
`SemanticSurfaceSummary` and `MixerControlId` still emit `summary.patch_id`,
`summary.patch_name`, `summary.capability_id`, `summary.effect_count`,
`summary.patch_count`, `summary.global_parameter_count`,
`summary.focused_control.*`, `summary.focused_track`, and `...id.track_id`. The
page currently reads three of them. Do not couple the page to
`graphicalShell.footer.pathLabel` (N2).

**WP05** — T033 bullet 8's escape hatch does **not** apply: the fixture's
installed capabilities do declare read-only sections (all three Braids rows;
SoundFont's `file`). Do not withdraw the claim from the Scope Decisions table.
The production producer is the `patchInteraction` leaf above, and a discriminating
assertion is available on one descriptor (SoundFont's `preset` is
`StructuralChoice` while its `file` is `ReadOnly`). T033 bullet 7's read-only
half must assert against that, not against `editable`. FR-010, FR-011, FR-012
and NFR-005 are now user-reachable, closing F-15.

**WP06** — two things. (1) `src/testing/live_effects_and_buses_scene.rs` took a
four-line rustfmt-only change from this lane; rebase rather than conflict.
(2) **NFR-004, "projection throughput unchanged", now has a number against it:**
a MIXER reprojection costs 2.92 ms in release over 89 rows, and it scales as
rows x actions x reducer-clone. That is paid on the control thread per accepted
event. The live scene is where this becomes observable. Measure it deliberately
rather than discovering it.
