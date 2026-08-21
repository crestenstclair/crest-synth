# Documentation reset archive — 2026-08-21

This temporary directory is the recoverable first step of the documentation
reset. It contains the superseded roadmap, master design, README/instruction
duplicates, OpenSpec state, legacy CUE/spec workflows, planning and acceptance
notes, Figma capture report/assets, webview spike, and spec conversion tooling.

The repository now keeps one living reference at the root: `AGENTS.md`. That
file consolidates the as-built architecture, invariants, current capabilities,
known gaps, live Figma links, evidence boundaries, and commands without a future
phase plan.

The archive commit is intentionally followed by a second commit deleting this
entire directory. Git history is the recovery mechanism.

The workspace also contained three ignored absolute symlinks into the retired
external `crest-spec` workflow. They are not copied into Git because they point
outside this repository; the reset removes only the links, not their external
targets:

- `.claude/skills/spec-authoring` →
  `/Users/crestenstclair/workspace/crest-spec/.claude/skills/spec-authoring`
- `.claude/skills/spec-generate` →
  `/Users/crestenstclair/workspace/crest-spec/.claude/skills/spec-generate`
- `.claude/workflows/spec-generate.js` →
  `/Users/crestenstclair/workspace/crest-spec/.claude/workflows/spec-generate.js`
