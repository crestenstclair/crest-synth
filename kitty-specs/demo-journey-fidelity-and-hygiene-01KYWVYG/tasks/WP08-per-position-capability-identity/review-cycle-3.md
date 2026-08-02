---
affected_files: []
cycle_number: 3
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
reproduction_command:
reviewed_at: '2026-07-31T23:40:00Z'
reviewer_agent: reviewer
verdict: approved
wp_id: WP08
---

# WP08 review — cycle 2: approved

Cycle 1's single blocking issue (the truncation-refusal path had no exercising
test) is closed. The fix commit `0fc4aad` is genuinely tests-only, the boundary
is pinned on both sides with two distinct capabilities that share a
64-byte prefix, the whole-graph refusal is driven through the production
builder, and nothing cycle 1 verified regressed.

## Delta verification

**Tests-only claim — confirmed.** For each of the three files touched by
`0fc4aad`, the production portion (everything above the single top-level
`#[cfg(test)]`) is byte-identical to `f680a2c`:

| file | prod lines | diff vs f680a2c |
|---|---|---|
| `src/real_time/graph_preparation_worker.rs` | 880 | identical |
| `src/real_time/prepared_graph.rs` | 602 | identical |
| `src/real_time/prepared_graph_builder.rs` | 370 | identical |

Each file has exactly one `#[cfg(test)]` (881 / 603 / 371), so no production
code sits below the cut. `from_identifier` is unchanged — the temporary
mutation was reverted. No re-verification of the mechanism was required.

**Boundary pinned on both sides, for both constructors, with a shared prefix.**
`carry_over_capability_identity_records_at_capacity_and_refuses_one_byte_beyond`
builds `at_capacity` (64 B), `first_beyond = at_capacity + "b"` and
`second_beyond = at_capacity + "c"` (65 B each), then asserts
`assert_ne!(first_beyond, second_beyond)` *and*
`assert_eq!(first_beyond[..64], second_beyond[..64])`. That is the real hazard
encoded, not a length check: under truncation the two distinct capabilities
would compare equal. Both are refused by `from_capability_id` **and**
`from_effect_capability_id`, and `at_capacity` records with
`as_str() == at_capacity` through both. Empty is refused too
(`assert_eq!(identity(""), None)`), so an unoccupied position's `None` can
never collide with a recorded identity.

**Whole-graph refusal through the production builder.**
`carry_over_capability_identity_beyond_the_record_refuses_the_whole_graph`
calls `PreparedGraphBuilder::new(...).build(...)`. At 64 B the graph builds and
*both* read paths carry the complete identity —
`engine_rack().capability_identity(0)` and
`layout().engine_capability_identity(0)`. At 65 B `build` returns
`Err(GraphPreparationError::UnrecordableCapabilityIdentity)`; the `Result`
shape means no partially-built graph can exist. The `Display` text is pinned.

The `build_with_engine_capability` helper does not weaken production
validation: it calls the real builder, derives its descriptor from the
production SoundFont descriptor (label, accent, sections, asset requirements,
voice policy, MIDI kinds), and builds its config through
`descriptor.create_config`. Between the at-capacity and beyond-capacity runs
the **only** difference is the identifier string — same revision, patch,
registry, and preparer. `FixturePreparer` is a pre-existing test double; the
commit only extracted `boxed_for` from `boxed`, leaving existing callers
behavior-identical.

**Worker mapping and Display arm.**
`carry_over_capability_identity_refusal_reports_an_invalid_candidate_config`
asserts `map_graph_preparation_failure(&UnrecordableCapabilityIdentity) ==
EngineSelectionFailure::InvalidDefaultConfig`; the `Display` string is asserted
in the builder test above.

## Reviewer-run non-vacuity proof

Replaced the length refusal in `from_identifier` with a truncating clamp:

```rust
if raw.is_empty() { return None; }
let take = raw.len().min(MAX_CAPABILITY_IDENTITY_BYTES);
```

`cargo test carry_over_capability_identity` → **9 passed, 2 FAILED**, and the
two failures are exactly the new boundary tests:

- `prepared_graph::tests::carry_over_capability_identity_records_at_capacity_and_refuses_one_byte_beyond`
  — `left: Some(PositionCapabilityIdentity("capability.aaa…"))  right: None`,
  i.e. the 65-byte identity collapsing onto the 64-byte record;
- `prepared_graph_builder::tests::carry_over_capability_identity_beyond_the_record_refuses_the_whole_graph`
  — the whole-graph refusal no longer fires.

The other nine remained green, confirming the two new tests are the ones
carrying the truncation contract. Mutation reverted with `git checkout --`;
`git diff HEAD` empty afterwards.

## Regression gates (all run in the lane worktree)

- `cargo test carry_over_capability_identity` → **11 passed** (was 8).
- `cargo test --test topology_change_lifecycle` → **10 passed**, including
  `held_voices_carry_over_a_slot_occupancy_activation_sample_continuously` and
  `held_voices_carry_over_a_slot_clear_activation_sample_continuously` — a
  refuse-everything guard would have broken these.
- `cargo test --all-targets` → green (442 unit tests plus every integration
  target; zero failures).
- `cargo clippy --all-targets -- -D warnings` → clean.
- `cargo fmt --all -- --check` → clean.
- Hygiene: `grep -n "post_effects()" graph_preparation_worker.rs` → empty;
  `grep -n "\bWP[0-9]"` over the six owned files → empty.
- Diff scope vs the mission base is exactly the six owned `src/real_time/`
  files across two commits.

## Accessor-seam judgment — WP09 has one unambiguous seam

The implementer kept the three `PreparedGraphLayout` identity accessors as
WP09's read seam. Each supporting claim checks out in the code:

- **The rack accessors are production-composed inputs, not a competing proof
  seam.** `PreparedGraph::layout()` calls all three —
  `engine_rack.capability_identity(index)` (`prepared_graph.rs:226`),
  `effect_rack.capability_identity_at(index, position)` (`:236`),
  `mixer.bus_returns().capability_identity(bus)` (`:255`) — and `layout()`
  itself has three production callers in `structural_graph_coordinator.rs`
  (`:22`, `:52`, `:108`). They compose the layout; they are live production
  code either way.
- **The layout returns exactly what `permits_replacement` consumes.**
  `permits_replacement` compares `engine_capability_identities`,
  `effect_capability_identities`, and `return_capability_identities` — the same
  three arrays the accessors read — and the coordinator's admission check is
  `graph.layout() != self.required_layout`.
- **`PreparedGraphLayout` is `Copy`** (`#[derive(Clone, Copy, Debug, Eq,
  PartialEq)]`, all fields arrays of `Copy`), so a proof can capture before and
  compare after without holding two live graphs.

One further point strengthens the decision beyond the implementer's argument:
`PreparedGraph` exposes `engine_rack()` and `effect_rack()` publicly but has
**no** public mixer/bus-returns accessor, so the rack path cannot observe
return-position identity from outside the crate at all. Only `layout()` gives
complete coverage of all three position kinds. `prepared_graph` is a `pub mod`
and `PositionCapabilityIdentity` is `pub` + `Copy` + `PartialEq` with
`as_str()`, so the seam is reachable from an integration target. WP09 has one
surface, not two.

## Carry-forward (non-blocking, for WP09)

Cycle 1's Note 2 still stands: the three layout identity accessors have zero
production callers — every call site is inside a `#[cfg(test)]` module. That is
deliberate (they exist so WP09 can measure
`carryOverWrongEngineIdentityRefused` without a test-only backdoor). If WP09
ends up comparing whole `PreparedGraphLayout` values rather than reading
per-position, retire the accessors there rather than growing a second seam.

## Anti-pattern checklist (delta)

1. Dead code — N/A for the delta (no new production code). Pre-existing
   accessor note carried forward above, non-blocking.
2. Synthetic-fixture test — PASS, proven by the reviewer-run mutation.
3. Silent empty return — N/A; no new production paths. `from_identifier`'s
   `None` is the documented fail-loud refusal.
4. FR coverage (FR-007, FR-016) — PASS; 11 tests under the declared selector,
   the refusal path now has three exercising tests.
5. Frozen surface — PASS; only the six owned files.
6. Locked decision — PASS; C-004 untouched (production byte-identical to the
   cycle-1-verified `f680a2c`).
7. Shared-file ownership — PASS; lane-h is WP08's alone and all six files are
   its declared `owned_files`.
8. Production fragility — N/A for the delta; the `.ok_or(...)?` refusals are
   control-side and documented.

**Verdict: approved.**
