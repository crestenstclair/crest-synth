## Context

See `proposal.md` for motivation and `specs/session-lifecycle/spec.md` for the behavioral contract. Today the standalone composition builds an empty `AppState`, lets `AutomaticMidiTest` install the bundled MIDI fixture's Patches, prepares that graph, starts the fixture source, and then opens the production window. `SavedSession` already captures version 2, decodes and migrates version 1, reconstructs a private candidate through `AppState::apply`, and returns candidate state only with its completely prepared graph. That boundary is tested but has no production file, dialog, document, or active-graph handoff owner.

The implementation must preserve one reducer mutation path, one application-wide structural request, block-boundary graph activation, off-thread preparation and retirement, immutable host-neutral projections, and the hard real-time callback contract. The linked Figma file remains authoritative for authored product and interaction surfaces, but it does not currently author New/Open/Save dialogs or document-error states. This phase therefore uses native application-menu/dialog conventions and the existing shell vocabulary without claiming a new Figma-parity slice.

## Goals / Non-Goals

**Goals:**

- Make normal startup, New, and Open produce the same validated candidate shape and the same callback-ready graph handoff.
- Keep persisted synth content canonical while placing document location, clean baseline, dialog state, filesystem operations, and lifecycle failures in a shell-owned boundary.
- Correlate whole-session reducer replacement with one prepared graph revision without exposing a half-installed session.
- Make dirty detection exact by saved content, including becoming clean when values return to the baseline.
- Retain autonomous fixture behavior only in explicit demo and witness paths.

**Non-Goals:**

- Empty-Patch creation, Patch-add workflows, autosave, crash recovery, recent documents, templates, cloud storage, or a session browser.
- A new saved-session version or a second session serialization model.
- Seamless preservation of sounding voices or effect tails across New/Open; the existing complete-graph replacement reset remains honest.
- Cross-platform packaging policy. Platform adapters must remain portable, but this phase validates the currently supported production host.
- Broad visual redesign or a claim that file lifecycle presentation closes existing Figma fidelity gaps.

## Decisions

### 1. A product-owned blueprint declares defaults; registry order never does

Add one default-session blueprint at the production composition boundary. Its production values are:

- Patch identity `1`, label `INIT`, MIDI channel `0` (shown as channel 1), output `T00`, and 0 dB Patch trim;
- the explicitly designated `instrument.soundfont.hidef` capability using its provider-authored default SoundFont asset and preset configuration;
- `VoiceEnvelope::default()`, the installed descriptor's seeded voice limit, and three empty post-effect slots;
- `MixerState::default()`, 0 dB master gain, and the existing exact production startup return composition (Reverb on return 0, Delay on return 1, both at their declared defaults, returns 2–7 empty).

The blueprint names capability identities; providers and registries still author configurations, availability, validation, and voice policy. A missing designated entry is an error rather than permission to use the first registry row. Tests inject alternate blueprints and registries without making production content name-enumerated throughout the reducer.

The factory constructs an initial candidate through canonical types and `AppState::apply`, captures it as `SavedSession`, and then submits it to the shared candidate preparer. This deliberately proves that the default is representable by the same public saved format and restore rules as an opened document. It does not introduce a second public `Session` type.

Alternatives considered:

- **Use the first enabled instrument descriptor.** Rejected because registry order would silently change product content and turn an unavailable default into fallback.
- **Reuse the multi-Patch MIDI fixture as the default.** Rejected because it preserves the demo-shaped startup this phase removes.
- **Start with zero Patches.** Rejected because empty-Patch implicit creation belongs to Phase 02 and would not start as a playable instrument.

### 2. Document lifecycle is a shell-owned state machine around the AppLoop

Introduce a `SessionLifecycleCoordinator` at the standalone application boundary. It owns:

- `DocumentIdentity` (`Untitled` or one exact platform path);
- the clean `SavedSession` baseline and derived dirty flag;
- the active lifecycle operation and any pending New/Open/close continuation;
- dialog requests/results and a typed visible lifecycle error;
- handles to candidate-preparation and file-write workers.

None of these fields enters `AppState` or `SavedSession`. A small immutable document presentation is merged with the graphical shell projection only at the shell boundary so the window title/status can show the leaf document name, dirty marker, operation, and error. The full path stays in the coordinator and native dialogs. This is presentation composition, not an alternate product model.

The application window gains host-neutral session-command and dialog-result ports. The Tauri adapter maps native File menu items and conventional platform shortcuts to `New`, `Open`, `Save`, and `SaveAs`; tests use deterministic fakes. Controller bindings remain unchanged because the Figma interaction map defines no file-command grammar.

Alternatives considered:

- **Store the path and dirty bit in `AppState`.** Rejected because they are platform/document facts, not synth session content, and would leak into reducer serialization and persistence.
- **Let JavaScript own document state.** Rejected because reload/reprojection would lose authority and the DOM must remain a thin view.
- **Use only process-fatal errors or stderr.** Rejected because post-start file and preparation failures must be visible while the prior instrument remains usable.

### 3. Dirty tracking is content-based and reducer-observed

Extend accepted reducer outcomes with a private/control-side indication that a field represented by `SavedSession::capture` changed. The coordinator performs a new capture only after such an outcome, including correlated worker completions, and compares it to the clean baseline. Navigation, MIDI generation-only events, device lifecycle, observations, and projection activity therefore do not pay capture cost or mark the document dirty. Returning all saved values to the baseline clears dirty state by equality.

New/Open success replaces the baseline with the installed candidate capture. A save worker receives an immutable captured value and a content token. On success, the written capture becomes the baseline; the coordinator compares the current capture again, so edits accepted while I/O was running remain dirty. Save As changes identity only in the same success transition.

Alternatives considered:

- **Compare `AppState::generation`.** Rejected because focus, MIDI, runtime, and no-op accepted events also advance it.
- **Set dirty permanently after the first edit.** Rejected because it cannot truthfully recognize an exact return to saved values.
- **Serialize and hash every frame.** Rejected because it adds needless control-side work and obscures the existing typed equality boundary.

### 4. New and Open share one off-thread candidate pipeline

One bounded session-preparation worker accepts either a canonical default `SavedSession` or bytes read for Open. The worker performs, in order:

1. UTF-8/JSON decode for Open;
2. supported version migration;
3. saved shape, scalar, routing, capability, effect, return, voice-limit, and stable asset-reference validation;
4. reconstruction through canonical reducer application with fresh interaction/runtime state;
5. state projection for a reserved target graph revision;
6. complete engine, effect, return, asset, routing, voice, and scratch preparation for the negotiated device configuration.

The successful result remains one prepared candidate containing session content and graph. Error variants retain their stage and nested typed cause. Preparation never owns document paths and never performs dialog work. Only one replacement may be preparing or using the structural coordinator at a time; Save may snapshot and write independently because it does not change the graph.

During potentially long preparation the prior graph remains active. MIDI performance and non-mutating navigation may continue, but saved-field edits are explicitly unavailable once the user has authorized replacement, preventing new work from being discarded without another decision. When a candidate is ready, the coordinator temporarily gates new MIDI dispatch and sends bounded all-notes-off recovery before structural publication.

Alternatives considered:

- **Decode and prepare on the UI/control thread.** Rejected because file and asset work can block interaction and violates the established worker boundary.
- **Install decoded state before graph preparation.** Rejected because any later capability or asset failure would strand canonical state without a matching playable graph.
- **Maintain separate New and Open graph builders.** Rejected because their drift would undermine the production session boundary Phase 02 depends on.

### 5. Whole-session commit is one reducer event correlated with the shared structural coordinator

Refactor structural ownership so engine/effect changes and session replacement use the same application-wide `StructuralGraphCoordinator` and one revision sequence. A prepared session is staged only when no other structural request is active. Before staging, the control side proves that its one-shot private replacement payload applies to a clone and projects values exactly equal to the prepared graph's initial snapshot.

The active callback accepts the complete graph only at a block boundary. On matching activation acknowledgement, `AppLoop` applies one whole-session replacement event through `AppState::apply`; that event replaces only persisted Patch/Mixer/master/return content, resets PATCH interaction to the deterministic Engine focus, installs the target graph revision, and preserves current application preferences and device runtime. The projector then publishes the matching immutable shell, state tree, and parameter snapshot before MIDI admission resumes.

If staging is busy/full, activation fails, correlation is stale/mismatched, or the preflight reducer/projection proof fails, no replacement event is applied. The old state remains paired with the old graph, MIDI admission resumes, and the rejected candidate returns to the worker for destruction. The callback never drops either graph. This short input gate is preferable to allowing old Patch-target commands to reach a newly activated graph in the interval before control acknowledgement.

Alternatives considered:

- **Swap an entire candidate `AppState` directly into `AppLoop`.** Rejected because it bypasses the only mutation path and would overwrite live device/runtime state with restore defaults.
- **Give session lifecycle a second structural queue/coordinator.** Rejected because it permits competing graph revisions and violates the single in-flight structural invariant.
- **Apply session content before activation.** Rejected because a handoff failure would expose canonical state that the callback cannot render.

### 6. Save uses an immutable capture and an atomic filesystem port

`Save` captures the active `SavedSession`. With an existing identity it submits that capture and path to a bounded file worker; when Untitled it first requests Save As. `SaveAs` always asks for a destination. Dialog cancellation is a successful no-op result, not an error.

The filesystem port encodes the current pretty JSON format, creates a uniquely named temporary sibling in the destination directory, writes all bytes, flushes and syncs the file, and atomically replaces the destination. Where the platform requires it, the adapter also syncs the parent directory. A failed operation removes its temporary artifact when possible and returns the precise creation/write/flush/sync/replace cause. It never truncates the destination first. The path returned by the dialog remains tentative until the worker reports success.

No format version bump is required: this connects existing version 2 data to I/O without changing its schema. Open does not rewrite a migrated source implicitly; a later explicit Save writes the current version.

Alternatives considered:

- **Write directly to the destination.** Rejected because a partial write can destroy the last valid session.
- **Store paths or absolute asset roots inside the JSON.** Rejected by the existing persistence and capability boundary.
- **Mark clean when the write starts.** Rejected because cancellation/failure or a concurrent later edit would lie about recoverability.

### 7. Unsaved-change handling is one resumable command state machine

New, Open, and application close first inspect dirty state. A dirty operation records one pending continuation and asks for Save, Discard, or Cancel:

- Save runs Save or Save As and resumes the continuation once only after success;
- Discard resumes immediately;
- Cancel, Save As cancellation, or save failure clears the continuation and preserves the active document.

While a native dialog is outstanding, duplicate lifecycle commands are rejected as Busy with visible status rather than opening multiple dialogs. Window close is intercepted until this state machine resolves; successful close then performs the existing MIDI, audio, graph, and worker teardown.

Alternatives considered:

- **Prompt independently in each command handler.** Rejected because nested Save As and close paths can double-run or lose the original intent.
- **Auto-discard on quit.** Rejected because it defeats the requested unsaved-change behavior.
- **Autosave before replacement.** Rejected as explicitly out of scope.

### 8. Failure presentation remains typed and outside canonical session content

Define one `SessionLifecycleError` sum covering dialog startup, file read, decode/migration, validation, capability/asset, projection, preparation, structural stage/activation, encode, and each atomic-write stage. It keeps nested canonical and adapter errors rather than flattening them into fallback behavior. The shell projection maps the operation, stage, and actionable message to explicit text plus the existing warning shape/color treatment. Dismissal affects only shell lifecycle state.

Startup errors that prevent any valid default graph are fatal but still pass through the native shell error boundary rather than silently launching a fixture or empty graph. After startup, every failure keeps the prior projection interactive and audible where applicable.

### 9. Proof is layered but crosses production seams

Add focused deterministic evidence for:

- exact default capture under production and reordered/augmented registries, plus missing-default failures;
- a spy fixture source proving normal startup neither prepares nor starts automatic MIDI;
- document baseline/dirty equality across every persisted and representative non-persisted event;
- unsaved Save/Discard/Cancel continuations, including cancelled Save As and save failure;
- fake dialog/filesystem ports covering read and every atomic-write stage without touching canonical state;
- v1 migration, v2 Open, unsupported version, malformed JSON, invalid shape/value/capability/asset, projection, preparation, stage, and activation failures;
- successful New/Open activation through the production reducer, projector, worker, structural boundary, renderer, and shell projection;
- preserved prior state hash, capture, document identity, dirty flag, revision, and audible render after every controlled replacement failure;
- matching candidate state/parameter/graph revisions at commit, input gating/recovery, block-boundary activation, off-thread retirement, and zero callback allocations/deallocations/destruction;
- successful Save/Save As bytes that decode to the exact captured version 2 value, including an edit accepted while a write is outstanding.

Native acceptance confirms menu/shortcut availability, dialog cancellation, visible dirty/error treatment, an Open/Save As round trip, and clean shutdown. This is a lifecycle handoff, not a broad visual-parity claim.

## Risks / Trade-offs

- **[Risk] Whole-session activation and reducer acknowledgement occur on different threads.** → Gate MIDI around structural publication, correlate one revision, apply only on matching acknowledgement, and retain the old pair on every non-success outcome.
- **[Risk] Refactoring structural ownership could regress engine/effect changes.** → Keep one coordinator contract and run existing topology, option, Detail, Mixer, Sample, callback-safety, and native-shell regressions in addition to lifecycle tests.
- **[Risk] A slow Open makes the application appear frozen or lets new edits be lost.** → Keep audio/MIDI performance and safe navigation active during preparation, show explicit progress, and disable only saved-field edits after replacement authorization.
- **[Risk] Platform rename semantics differ.** → Put atomic replacement and directory synchronization behind a typed filesystem port with controlled failure tests; do not claim untested packaging targets.
- **[Risk] Pretty JSON equality or field ordering could be mistaken for dirty state.** → Compare typed `SavedSession` values, never encoded byte strings.
- **[Risk] The production default capability or asset is unavailable.** → Fail truthfully at startup/New; never choose another registry entry. The prior session remains active for a failed New.
- **[Trade-off] New/Open reset voices and tails.** → State this explicitly and use all-notes-off recovery; seamless graph migration remains out of scope.
- **[Trade-off] Native file UI is not currently authored in Figma.** → Use platform-standard menu/dialog behavior and existing shell status vocabulary, record the gap, and make no parity claim.

## Migration Plan

1. Add default-session, document-state, dialog/filesystem, worker, and typed-error boundaries with deterministic fakes while leaving the current production entry path intact.
2. Centralize structural coordinator ownership and add whole-session preflight/stage/activation/reducer-commit tests before wiring user commands.
3. Add the lifecycle coordinator, dirty baseline, atomic save worker, unsaved continuation state machine, and shell document/error projection.
4. Split standalone preparation into a fixture-free production path and the retained explicit demo/witness path; switch normal `make run` only after the startup and no-fixture proofs pass.
5. Wire native menu items, shortcuts, dialogs, close interception, and bounded native handoff evidence.
6. Run focused and broad deterministic/native gates, then update `DESIGN.md` with the actual production boundary and measured evidence.

No persisted-data migration or saved-session version bump is introduced. Rollback reverts the production shell wiring and default startup selection; any files written by this phase remain valid version 2 `SavedSession` documents even if the reverted application lacks user-facing commands to open them.
