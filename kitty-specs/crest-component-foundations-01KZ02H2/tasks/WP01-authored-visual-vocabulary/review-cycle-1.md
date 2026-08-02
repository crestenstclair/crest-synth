# WP01 Review — Cycle 1

**Verdict**: Changes requested. One blocking finding.

Reviewed `kitty/mission-crest-component-foundations-01KZ02H2..kitty/mission-crest-component-foundations-01KZ02H2-lane-a`.

## Blocking

### R1 — NFR-006 is declared in `requirement_refs` and has zero assertion

`NFR-006` is in this WP's frontmatter and in `tasks.md`'s coverage table. It requires the typeface to be
vendored "with its verbatim license, upstream revision, a byte-exact hash manifest, and a reproducible
derivation procedure for any weight not shipped upstream." All four artifacts are present and correct at
`vendor/azeret-mono/` — and nothing verifies any of them. If a face is silently replaced, no test fails.

This repository already treats vendored provenance as test-enforced, not documented. `verify_source_bundle`
at `tests/static_patch_effect.rs:269-296` reads `SHA256SUMS`, recomputes every hash with `sha2`, asserts the
manifest's path set equals the directory's actual contents, asserts `PROVENANCE.md` names the upstream
revisions, and asserts the license text. `tests/braids_engine.rs:363` does the same for Braids. Azeret Mono
is the only vendored asset without it.

**Fix**: add a test to `src/shell/visual/typeface.rs` (a file this WP owns) mirroring that pattern:

1. Read `vendor/azeret-mono/SHA256SUMS`, recompute each listed file's SHA-256 with the `sha2` crate
   (already a dependency, `Cargo.toml:34`), and assert every hash matches.
2. Assert the manifest's path set equals the directory's actual contents plus `PROVENANCE.md` and
   `SHA256SUMS`, so an unlisted face added later fails.
3. Assert `OFL.txt` contains the verbatim OFL 1.1 grant, and that `PROVENANCE.md` names the upstream
   source and the derivation procedure for the four static weights.

That closes NFR-006 with the same mechanism the rest of the repository already uses.

## Non-blocking notes

### N1 — `src/shell/visual/*` has no production caller

`grep -rn "shell::visual\|visual::" src --include="*.rs"` returns nothing outside the module itself. By the
letter of anti-pattern item 1 this is dead code. It is not being treated as a FAIL because the mission's
declared dependency graph (`tasks.md:216`) makes WP04 the consumer and sequences WP01 → WP04 deliberately;
`plan.md` IC-01 names this concern "the root; every other concern consumes it." **This stops being
acceptable if WP04 lands without deleting the seven adapter constants** — which still exist at
`src/adapter/eframe_graphical_window.rs:28-34`. WP04's reviewer should re-run this grep and require a hit.

### N2 — the three stubs are outside `owned_files`

`density.rs`, `state.rs`, and `primitives.rs` are in `create_intent` but not `owned_files`, so the commit
guard logged `ACTIVE_WP_SCOPE_VIOLATION` three times. T001 mandates creating them, so the frontmatter is
what is wrong, not the commit. Add the three paths to `owned_files` when the WP is next edited.

### N3 — `src/shell/visual/mod.rs` is a shared surface

WP02 and WP03 fill in modules this WP declared, and re-exporting their public surface means editing
`mod.rs`, which WP01 owns. T001's note covers the module declarations (`pub mod primitives;` works for both
`primitives.rs` and `primitives/mod.rs`), but not re-exports. WP02/WP03 can use full paths
(`crate::shell::visual::density::…`) and avoid touching it. Coordination note recorded.

## Anti-pattern checklist

| # | Item | Result |
|---|---|---|
| 1 | Dead code | **N/A — sequenced foundation**, see N1 |
| 2 | Synthetic-fixture test | PASS — deleting `resolve()`/`load()` breaks compilation; no test asserts a literal it also constructs |
| 3 | Silent empty return | PASS — the one `.ok()` (`typeface.rs:292`) is test-only cleanup |
| 4 | FR coverage | **FAIL — NFR-006, see R1.** FR-001, FR-002, FR-010 each covered |
| 5 | Frozen surface | N/A — no file marked frozen |
| 6 | Locked decision | PASS — no `Color32::from_rgb` outside `token.rs`; the seven adapter constants are WP04's T020 |
| 7 | Shared-file ownership | PASS with notes N2, N3 |
| 8 | Production fragility | PASS — no new panic in a production path; `load()` returns `Result` |

## Review guidance responses

- **Values exact?** Yes. `accent/focus` = `#65e5ff` (`token.rs:106`), `Display/Screen` = 32/40 SemiBold 0.4
  (`token.rs:230`). Spot-checked all 17 colors and all 8 styles against `DESIGN.md:534-573`.
- **Raw values private?** Yes. Every RGB triple lives in `SemanticColor::resolve`; no literal color, size,
  or spacing constant exists outside `token.rs`.
- **Typed missing-typeface error?** Yes, and proven negatively — `an_unavailable_face_is_a_typed_error_naming_it`
  asserts `FaceUnavailable` naming the Regular face, and `an_empty_face_is_unreadable_rather_than_silently_accepted`
  covers a present-but-unusable file. Defaulting `FontFamily::Proportional` and `::Monospace` to the authored
  Regular face closes the fallthrough egui would otherwise absorb.
- **T006 expected values independent?** Yes. Every expected value is spelled out as a literal in the test;
  no loop compares the vocabulary to a table derived from it.
- **`DESIGN.md` clean?** Yes. Three decisions appended plus the `selected` row. No collateral edits.

## Verification run

- `cargo test --lib shell::visual` — 15 passed, 0 failed
- `make test` — 482 lib + every integration target, 0 failures
- `make lint`, `make fmt-check` — clean

Two lane defects were repaired before any of this could run and are worth carrying forward: lane-a was
missing `vendor/azeret-mono/` and the six new `.kittify/crest-spec/proof/*` files (the lane branch was cut
from the last merge commit, not the feature tip), and `sf2/` fixtures were not linked, which failed 30+
unrelated tests with `FileOpen`.
