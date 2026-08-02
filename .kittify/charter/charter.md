# Project Charter — curated companion

<!--
This file is a hand-authored, display-only companion to the authoritative
`.kittify/charter/charter.yaml`. It holds the human *rationale* behind your
project's governance and doctrine.

It is NEVER a charter-resolving input, and `charter generate` NEVER overwrites
it — edit it freely.
-->

## What this project is

Crest Synth is a controller-first software synthesizer in Rust: a SoundFont
engine behind capability ports, a fixed sixteen-track mixer, and a bounded,
descriptor-driven effect and bus topology. `DESIGN.md` is the product
authority; the crest-spec at `.kittify/crest-spec/` is the executable
declaration of what the system is and the single source of implementation
intent. Everything else — planning, work packages, code, proof — derives from
those two, in that order of authority.

## Governance posture (from the 2026-07-31 charter interview)

**Full mission rigor is the standard, not the exception.** Every change goes
through the complete Spec Kitty mission workflow (specify → crest-spec → plan
→ analyze → tasks → implement → review → accept), with measured, falsifiable
proof on the production reducer and render path, retained live-demo scenes as
phase gates, and bulk-edit occurrence maps where edits fan out.

**The costliest failure is silent design drift.** Reviews and gates
prioritize fidelity between code, the crest-spec, and `DESIGN.md` above all
else — above speed, above polish, above even the (co-critical) hard real-time
contract. The crest-spec is authored first and never edited after the fact to
permit code that was already planned. Prose constraints that failed once are
replaced by proof-enforced invariants (see the no-name-enumeration project
check).

**Exceptions are self-service but never silent.** An agent blocked by a
process or tooling gate may waive or work around it only by committing the
waiver in-repo with its rationale and flagging it in the next human-visible
report. Product and proof gates — acceptance validations, physical-device
live-demo gates, real-time contract proofs, `spec-kitty crest-spec doctor` —
are never waived autonomously: those stop and ask the human.

## Doctrine selection rationale

The deliberate selections are narrow: **domain-driven-design** (the
crest-spec *is* a DDD declaration over seven bounded contexts) and directives
DIRECTIVE_001 (Architectural Integrity), DIRECTIVE_003 (Decision
Documentation), DIRECTIVE_010 (Specification Fidelity), DIRECTIVE_024
(Locality of Change — held in deliberate, documented tension whenever a
mission's blast radius is the point), and DIRECTIVE_025 (Boy Scout Rule).
The generated catalog lists the full built-in library for reference; listing
is not activation, and near-matches (BDD, atomic design, git-flow) were not
adopted. The interview answers at
`.kittify/charter/interview/answers.yaml` (surfaced as
`USER:PROJECT_PROFILE`) carry the full governance detail: testing
requirements, quality gates, review policy, performance targets, risk
boundaries, amendment and exception policy.
