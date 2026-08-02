Kick off Phase 4 — Component library blockout — for crest-synth.

## Where things stand

- Repo: /Users/crestenstclair/workspace/crest-synth
- Branch: `main`. Phase 3 was merged in as commit `60996f4` (209 commits from
  `feat/expandable-effects-and-bus-topology`). Start Phase 4 from `main`.
  `main` is 216 commits ahead of `origin/main` and has NOT been pushed.
- Phase 3 is delivered and demonstrated on physical hardware: three ordered
  descriptor-driven effect slots per Patch, eight bounded bus returns, one
  role-independent effect registry with no effect or routing identity named in
  any type.
- Just landed: `SemanticAction::SelectPatch(Direction)`. The fixture installs one
  Patch per MIDI part — the rig loads 15 instruments — and until now the
  controller could reach only the first. Q/E now step between them. This closed
  Phase 5's first entry condition. The second (`make demo-live-patch-editor` on
  2+ instruments) is still open and is NOT Phase 4's job.

## What to build

Phase 4 is specified in `ROADMAP.md:170-184`. Read it verbatim — do not
paraphrase it back as the scope. In short: the reusable component vocabulary the
real Patch and Mixer screens get assembled from. Semantic tokens, primitives,
configurable controls, compositions, a gallery rendering every meaningful
behavioral state at desktop and Steam Deck sizes, and a new
`make demo-live-component-library` scene. It is an application component system,
not a separate product and not a React runtime.

Read `DESIGN.md` first — it is the product authority, and its linked Figma file
is the visual and interaction reference. Phase 4 is where visual fidelity
actually starts to matter, so lean on Figma hard.

## How to work

Read `CLAUDE.md`, then note:

- The crest-spec at `.kittify/crest-spec/` is the bedrock. Author it FIRST for
  any new structure, before planning. `spec-kitty crest-spec doctor` must stay
  green.
- Use the Spec Kitty mission workflow.

## What went wrong last session — do not repeat it

1. An entire session went to proof-about-proof: a mission to verify a demo, then
   a mission to verify that verification, then an analysis blocking on verifying
   the verification. Three levels of meta, zero synth. The operator was rightly
   furious. **If a mission's deliverable is not something a player can hear or
   see, stop and re-scope it.** spec-kitty drives architecture and invariants; it
   is not the work.
2. Don't hand the program back as homework. Run `make run` and the live demos
   yourself, then ask the operator only for the visual judgment a log cannot
   carry.
3. Never pipe cargo/test/demo output through `head` or `tail` — the pipe reports
   the pager's exit code, so a "green" recorded that way is a lie. Redirect to a
   file instead.
4. Verify before asserting. Two claims last session were stated without checking
   (a type that did exist, a defect count that was wrong) and both had to be
   corrected. Grep and run first.
5. Closed unions are the good thing here: adding one `SemanticAction` variant
   made the compiler and the coverage guards name all 12 sites that had to
   change. Trust that, and add the exhaustiveness assertion when you add a
   surface.

## Start by

Reading `DESIGN.md` and `ROADMAP.md:170-184`, then telling me how you would slice
Phase 4 and what you need from Figma. Ask before running `/spec-kitty.specify`.
