## Why

Crest Synth's normal production path still boots and plays an automatic MIDI demo fixture, while its existing versioned session capture and transactional restore machinery is not connected to a user-facing document lifecycle. This phase establishes the production session boundary so the application starts as an instrument whose active work can be created, opened, saved, and preserved safely.

## What Changes

- Define one deterministic, registry-valid default session and make it the initial active session for normal startup.
- Remove automatic MIDI-fixture installation and playback from the normal product path while retaining explicitly invoked demo and witness paths.
- Add host-level New, Open, Save, and Save As commands with native file selection where a path is required.
- Connect `SavedSession` capture, version migration, validation, complete graph preparation, and atomic state/graph replacement to the production shell.
- Track active document identity and unsaved changes outside canonical synth state; protect dirty work before New, Open, and application close with explicit Save, Discard, or Cancel outcomes.
- Make Save write the current versioned canonical session atomically, and make Save As establish the chosen document location only after a successful write.
- Surface file-dialog, filesystem, decode, migration, validation, capability, graph-preparation, activation, and encoding/write failures explicitly without fallback or substitution.
- Preserve the prior active session, document identity, dirty status, focusable shell, and usable audio graph whenever New or Open cannot produce and activate a complete replacement.
- Add deterministic production-boundary tests for successful, cancelled, and failed startup, New, Open, Save, Save As, dirty-document, and atomic replacement paths.

## Capabilities

### New Capabilities

- `session-lifecycle`: Default-session construction, production startup, document identity and dirty tracking, New/Open/Save/Save As workflows, transactional state/graph replacement, explicit failure presentation, and production-path acceptance.

### Modified Capabilities

None.

## Impact

- Affected control/application areas include default-session construction, `AppState` event application, `AppLoop` session replacement and graph correlation, projection/status reporting, and shutdown coordination.
- Affected shell/adapter areas include the standalone composition root, native application commands and dialogs, document-path ownership, filesystem read/write/rename behavior, window title/status presentation, and removal of the automatic fixture from normal startup.
- Existing `SavedSession` version 2 capture, v1 migration, validation, and `PreparedSavedSession` boundaries will be reused and extended at the production composition seam rather than replaced by another persistence model.
- Demo and witness entry points remain fixture-driven and isolated from the normal product path.
- Implementation will update `DESIGN.md` with the durable as-built session boundary and falsifiable evidence; these temporary OpenSpec artifacts do not become a competing product authority.
