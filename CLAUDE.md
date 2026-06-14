# crest-synth — working in this repo

This is a **crest-spec** project. The source of truth is the CUE spec in
[`spec/`](spec/); the Rust crate in `src/` is **generated** from it by the
crest-spec generate→validate→retry loop.

## The rules (non-negotiable)

1. **Author the spec, never hand-write the code.** To add or change a feature,
   edit the relevant domain `.cue` in `spec/` (describe intent in an asset's
   `prompts`). Do NOT write `.rs` by hand — the loop generates and validates it.
2. **Use the skills.** `spec-authoring` for writing/modifying the CUE (vision +
   methodology); `spec-generate` for running generation. Invoke `spec-authoring`
   before editing the spec.
3. **Spec layout is domain-grouped.** One `.cue` per bounded context (plus
   `project.cue`, `manifest.cue`). No phases, no `override-*` files — those were
   a test-harness artifact. crest-spec loads the whole folder as one CUE package.
4. **The gate is whole-tree validation.** `cargo fmt`, `clippy --all-targets`,
   build, and test run across the entire crate (`project: validations` in
   `project.cue`). Do not introduce per-file resource→file ownership or scope
   validations to single files — that breaks crate-wide tooling (formatters,
   lints) and is explicitly unwanted.

## Iterating

1. `spec-authoring` → edit the right `spec/*.cue` (add/tighten the value object,
   aggregate, port, adapter, or asset + its `invariants`/`validations`).
2. `spec-generate` → the planner regenerates only what changed, validates the
   whole tree, and commits.
3. `git commit` the spec change + regenerated code together.

State lives in `.crest-spec/state.db` (gitignored); the baseline is already
adopted, so edits regenerate incrementally. The crest-spec MCP server is wired
via `.mcp.json` pointed at `./spec`.
