---
affected_files: []
cycle_number: 3
mission_slug: crest-component-foundations-01KZ02H2
reproduction_command: cargo test --lib visual::typeface
reviewed_at: '2026-08-02T20:32:15Z'
reviewer_agent: claude
verdict: approved
wp_id: WP01
---

# WP01 – Authored visual vocabulary and typeface — review cycle 3

Reviewer: claude · lane-a · `kitty/mission-crest-component-foundations-01KZ02H2-lane-a` @ `5b61d39`

**Verdict**: Approved.

## Why this cycle exists

Cycle 1 rejected on one blocking finding (R1). `review-cycle-2.md` is not a second
review: its body is byte-identical to cycle 1's, still titled "Cycle 1", carrying the
same rejection with `reviewer_agent: unknown` — a frontmatter wrapper written over the
cycle-1 content at 2026-08-02T03:37:51Z, not a judgement of the fix. R1 was in fact
fixed in `db34bb2` and the work package was moved to `approved`, so its lane state and
its latest review artifact disagreed, and the merge gate refused on
`terminal_wp_latest_review_artifact_must_not_be_rejected`.

This cycle is the judgement of the fix that was never recorded. It is a real review,
not an arbiter override — the runtime offered `--skip-review-artifact-check` and it was
declined, because the disagreement was a missing verdict rather than a wrong one.

## R1 — NFR-006 declared in `requirement_refs` with zero assertion — **discharged**

`db34bb2` adds four provenance tests to `src/shell/visual/typeface.rs`, mirroring the
pattern this repository already uses for its other vendored assets
(`tests/static_patch_effect.rs:269`, `tests/braids_engine.rs:363`):

| Test | Line | Claim |
|---|---|---|
| `every_vendored_file_matches_its_recorded_hash` | 204 | every file `SHA256SUMS` lists still hashes to its recorded value |
| `the_manifest_covers_every_vendored_file` | 232 | the manifest's path set equals the directory's contents, so a face added without a hash fails rather than passing by omission |
| `the_open_font_license_ships_verbatim` | 267 | `OFL.txt` carries the verbatim OFL 1.1 header, permission grant, and upstream copyright |
| `the_provenance_record_names_the_upstream_source_and_derivation` | 287 | `PROVENANCE.md` names the upstream source and the derivation procedure |

`vendor/azeret-mono/` carries all four registered weights, the variable source,
`OFL.txt`, `PROVENANCE.md`, and `SHA256SUMS`.

Verified by running, on lane-a: `cargo test --lib visual::typeface` → **10 passed,
0 failed**.

## The assertion was falsified, not just observed passing

A passing test is not evidence it catches anything. `SHA256SUMS` line 1 was rewritten
with a zeroed hash and the central assertion re-run:

```
test result: FAILED. 0 passed; 1 failed
  shell::visual::typeface::tests::every_vendored_file_matches_its_recorded_hash
```

The manifest was restored and `git status vendor/azeret-mono/` is clean. A silently
replaced face now fails a test, which is precisely what R1 said it did not.

## Scope and ownership

The diff against the mission branch is 8 files, 1022 insertions, and nothing outside
the work package's surface:

- `src/shell/visual/token.rs` (578) and `src/shell/visual/typeface.rs` (413) — the
  vocabulary and the typeface;
- `src/shell/visual/mod.rs` (23) and `src/shell/mod.rs` (1) — module wiring;
- `src/shell/visual/{density,primitives,state}.rs` (1 each) — the stubs cycle 1 noted
  as N2, later filled by WP02 and WP03;
- `DESIGN.md` (+4) — the three durable decisions and the `selected` row, no collateral
  edits.

All four declared `requirement_refs` now carry assertions: FR-001 and FR-002 through
`token.rs` (9 tests) and `typeface.rs` (10 tests), FR-010 through
`an_unavailable_face_is_a_typed_error_naming_it` and its two companions, and NFR-006
through the four provenance tests above.

## Non-blocking notes from cycle 1

N1 (`src/shell/visual/*` has no production caller), N2 (the three stubs sit outside
`owned_files`), and N3 (`mod.rs` is a shared surface) were all correct at cycle 1 and
are all resolved by the mission as delivered: WP04 repainted the production shell
through the vocabulary, and WP02/WP03 filled the stubs. Nothing carries forward.

## Verification run

- `cargo test --lib visual::typeface` (lane-a) → 10 passed, 0 failed.
- Falsification of `every_vendored_file_matches_its_recorded_hash` → FAILED as
  required, manifest restored, tree clean.
- Mission-level, on lane-f: `make fmt-check`, `make lint`, and `make test` all exit 0
  (587 lib tests plus every integration target, 0 failures), and
  `cargo test --test component_vocabulary` prints
  `CREST_ACCEPTANCE component_vocabulary passed` with `colors=17 type_styles=8
  spacing_steps=6 radii=3`, measured through the production render path.
