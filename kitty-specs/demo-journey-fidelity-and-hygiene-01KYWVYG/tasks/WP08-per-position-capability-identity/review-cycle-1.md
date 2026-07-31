# WP08 review — cycle 1: changes requested

The mechanism is correct and the C-004 boundary is clean. One blocking gap: the
truncation-refusal path — the single behaviour that keeps this WP's premise
("two different capabilities must never compare equal") sound — has no test.

## What passed (do not redo)

- Diff scope: exactly the six owned `src/real_time/` files, one commit `f680a2c`.
- **C-004**: verified by reading `render`/`dispatch`/`process_return`/
  `process_slots_in_place` — zero identity code in any per-block path. The only
  production touch points per rack are the field, its `None` init, the accessor,
  the recorder, and one conjunct in `carry_live_*`. `PositionCapabilityIdentity`
  is `{ length: u8, bytes: [u8; 64] }`, `Copy`, no `String`/`Vec`/`Box`, no
  `Drop`, no panic. `PreparedGraphLayout`'s ~4.7 KB growth is confined to
  `StructuralGraphCoordinator` (control/worker side) — no callback-path storage.
- Fail-safe direction: all three racks extend the existing guard with one
  equality conjunct on the existing `continue` — mismatch keeps the fresh
  instance; never bypasses, never panics, never partially adopts.
- Teeth-proof (reviewer-run): removing each rack's conjunct in turn fails
  exactly that rack's mismatch test (3 removed → 3 failures). Tree restored.
- Agreement still carries over: `cargo test --test topology_change_lifecycle`
  10/10, including both held-note carry-over proofs — non-vacuous, since every
  production graph is stamped through `PreparedGraphBuilder::build`
  (`PreparedGraph::new` is `pub(crate)` with that one call site).
- `permits_replacement` is identity-exact for every position OUTSIDE the
  declared scope in all three scopes, with assertions for each.
- Unoccupied positions record `Option::None`, structurally distinct from any
  recorded identity (`from_identifier` rejects empty) — no sentinel collision.
- Hygiene: `grep -n "post_effects()" graph_preparation_worker.rs` → zero;
  `grep "\bWP[0-9]"` over owned files → zero.
- Gates: `cargo test carry_over_capability_identity` 8/8;
  `cargo test --all-targets` all green; `cargo clippy --all-targets -D warnings`
  clean; `cargo fmt --check` clean.
- The `control_dispatch_performance.rs` diagnosis is correct: that test drives
  `AppLoop::dispatch` over a `NoopBoundary` and never reaches graph preparation,
  so this prepare-time-only change adds nothing to what it measures. It also
  passed in this review run.

## Issue 1 (blocking) — the truncation-refusal path is entirely untested

`GraphPreparationError::UnrecordableCapabilityIdentity`, its `Display` arm, and
its `map_graph_preparation_failure` mapping to
`EngineSelectionFailure::InvalidDefaultConfig` have **zero** exercising callers
outside the three `.ok_or(...)?` sites:

```
src/real_time/prepared_graph_builder.rs:119,148,231   # producers
src/real_time/prepared_graph_builder.rs:279,339       # variant + Display
src/real_time/graph_preparation_worker.rs:795         # worker mapping
```

Reading confirms the code is right: `from_identifier` length-checks *before*
`copy_from_slice`, and there is no truncation primitive anywhere. But nothing
pins that. A future edit that changed `from_identifier` to truncate instead of
refuse would keep the whole suite green while silently making two distinct
capabilities compare equal — precisely the hole this WP exists to close.

The path is reachable: `CapabilityId`/`EffectCapabilityId` enforce kebab-case
segments but impose **no length bound**, so a >64-byte id is constructible.

**Fix**: add a test under the declared selector (name it so
`cargo test carry_over_capability_identity` runs it) that asserts:

1. `PositionCapabilityIdentity::from_capability_id` returns `None` for an
   identity of `MAX_CAPABILITY_IDENTITY_BYTES + 1` bytes, and `Some` at exactly
   `MAX_CAPABILITY_IDENTITY_BYTES` (pin the boundary, not just the overflow).
2. `PreparedGraphBuilder::build` with such a capability at one position refuses
   the **whole graph** with `GraphPreparationError::UnrecordableCapabilityIdentity`
   — no partially-built graph, nothing truncated.

Asserting the boundary in both directions is what makes truncation
non-reintroducible; asserting only the overflow would still pass under a
truncating implementation that happened to reject overlong input elsewhere.

## Note 2 (non-blocking) — three layout accessors have no production caller

`PreparedGraphLayout::engine_capability_identity`,
`effect_capability_identity`, and `return_capability_identity`
(`prepared_graph.rs:430,435,447`) are `pub` but called only from tests —
`permits_replacement` compares the backing arrays directly. Strictly this trips
the dead-code check.

Not blocking: they are read-only observers on a production type, and the WP
explicitly requires the mechanism to stay "reachable from production types (no
test-only backdoor seams)" so WP09 can measure
`carryOverWrongEngineIdentityRefused`. Leave them; if WP09 ends up reading the
racks rather than the layout, retire them there rather than growing a second
seam.
