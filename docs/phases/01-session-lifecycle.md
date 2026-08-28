# Phase 01 — Session Lifecycle

Status: Next

## Outcome

Crest Synth starts as a real user-facing instrument rather than an automatic
demo fixture. A user can create a default session, create a new session, open
an existing session, and save the active session.

## Scope

- Define the canonical default session.
- Replace automatic fixture startup in the normal product path.
- Add New, Open, Save, and Save As workflows.
- Connect the existing versioned `SavedSession` capture, migration,
  validation, preparation, and atomic restore behavior to the production
  shell.
- Preserve the active session and audio graph if opening a session fails.
- Surface file, decode, validation, capability, preparation, and save failures
  explicitly without fallback or substitution.
- Keep file dialogs, filesystem paths, and platform-specific storage behavior
  outside canonical product state.
- Establish document identity and unsaved-change behavior for the active
  session.

## Out of scope

- Empty-patch implicit creation, which is Phase 02.
- Multi-platform packaging or handheld-specific work.
- Autosave, cloud synchronization, recent-file history, templates, and session
  browsing beyond the basic lifecycle.
- New synthesizer or effect capabilities.

## Completion signals

- Normal startup no longer depends on or automatically plays the bundled MIDI
  fixture.
- New produces the defined default session through the canonical state and
  prepared-graph path.
- Open validates and prepares a complete candidate before atomically replacing
  the active session and graph.
- Save writes the active canonical session in the versioned format, and Save
  As establishes a new document location.
- Failures are visible and leave the prior valid session usable.
- Deterministic tests cover successful and failed New, Open, Save, and Save As
  paths through production boundaries.

## Dependency

This phase establishes the production session boundary used by Phase 02.

