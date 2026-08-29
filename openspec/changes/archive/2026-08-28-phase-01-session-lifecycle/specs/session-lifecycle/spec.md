## Purpose

Defines the user-facing document lifecycle that creates, opens, saves, and safely replaces complete Crest Synth sessions without weakening canonical-state or real-time guarantees.

## ADDED Requirements

### Requirement: The product has one canonical default session
The product SHALL define one deterministic default session independently of demo fixtures. It SHALL contain exactly one Patch with stable identity 1, the product composition's explicitly designated default instrument and that provider's validated default configuration, MIDI channel 1, output track T00 at 0 dB Patch trim, the neutral default envelope, the capability-seeded voice limit, and three empty post-effect slots. It SHALL also contain the default sixteen-track Mixer, 0 dB master gain, and the production-declared default return bank. The default instrument and returns MUST resolve exactly through installed registries; absence or rejection SHALL be a typed failure and MUST NOT select the first, nearest, or similarly named capability.

#### Scenario: Default session is constructed
- **WHEN** the designated instrument, effect capabilities, and assets are installed and valid
- **THEN** default-session capture SHALL contain exactly the declared Patch, Mixer, master, return, routing, envelope, voice-limit, and empty-slot values

#### Scenario: Designated default capability is unavailable
- **WHEN** the explicitly designated default instrument or return capability is absent or rejects its authored default configuration
- **THEN** default-session construction SHALL fail with the exact capability cause and SHALL NOT create an empty Patch, choose another registry entry, or install partial state

#### Scenario: New capability registry order
- **WHEN** additional instrument or effect capabilities are installed before or after the designated defaults
- **THEN** the canonical default session SHALL retain the same declared capability identities and values rather than changing with registry position or Figma fixture content

### Requirement: Normal startup opens the default instrument without automatic playback
Normal product startup SHALL validate and completely prepare the canonical default session before starting its active audio graph and user-facing shell. It SHALL open that session as an untitled clean document in PATCH with one stable Engine focus and no subordinate surface. Normal startup MUST NOT initialize, start, install Patches from, or dispatch events from the bundled automatic MIDI fixture. Explicit demo and witness entry points MAY retain their isolated fixture behavior.

#### Scenario: Successful normal startup
- **WHEN** the normal product command starts with all designated capabilities and assets available
- **THEN** the first usable projection and active graph SHALL represent the canonical default session, the document SHALL be untitled and clean, and no note SHALL sound until user or physical MIDI input is accepted

#### Scenario: Automatic fixture is not consulted
- **WHEN** normal startup runs with a fixture source that would fail if prepared or started
- **THEN** startup SHALL still reach the default session without invoking that source or installing any fixture Patch

#### Scenario: Default preparation fails at startup
- **WHEN** default-session validation or complete graph preparation fails
- **THEN** startup SHALL expose the typed fatal cause and SHALL NOT launch an empty, substituted, partially prepared, or automatic-fixture session

### Requirement: Session files contain only the versioned canonical saved session
Save and Open SHALL use the existing versioned saved-session format and its supported migrations. Session files SHALL contain canonical Patch, Mixer, return, master, and stable relative asset-reference data only. Document paths, dialog state, dirty state, focus, subordinate interactions, runtime lifecycle, prepared graphs, decoded assets, device preferences, device handles, observations, and platform storage details MUST remain outside the saved session and canonical product state.

#### Scenario: Save captures a session
- **WHEN** a session is saved
- **THEN** the file SHALL decode as the current saved-session version and SHALL contain none of the document, dialog, interaction, runtime, graph, decoded-asset, or device fields excluded by this requirement

#### Scenario: Open migrates a supported older version
- **WHEN** Open selects a valid supported older saved-session version
- **THEN** the complete candidate SHALL be migrated to the current version before validation and preparation without modifying the source file implicitly

#### Scenario: Unsupported version is selected
- **WHEN** Open decodes a syntactically valid document with an unsupported version
- **THEN** it SHALL report the unsupported version explicitly and SHALL NOT infer, downgrade, or substitute session values

### Requirement: Document identity and unsaved changes are explicit shell state
The shell SHALL own either Untitled identity or one exact filesystem location for the active document, plus a clean baseline equal to a canonical saved-session capture. Dirty state SHALL mean the current canonical saved-session capture differs from that baseline. Navigation, focus, presentation reflow, MIDI performance events, device state, load lifecycle, observations, and other non-saved runtime changes MUST NOT mark the document dirty. Default New and successful Open SHALL establish a clean baseline; successful Save SHALL establish the exact written capture as the baseline.

#### Scenario: Persisted control changes
- **WHEN** an accepted reducer event changes any field represented by the saved-session capture
- **THEN** the document SHALL visibly become dirty without placing its path or dirty flag in canonical product state

#### Scenario: Non-session activity
- **WHEN** the user navigates, resizes, opens or closes a subordinate surface, performs MIDI notes, changes device connection state, or receives observations without changing a saved field
- **THEN** clean or dirty status SHALL remain unchanged

#### Scenario: Values return to the clean baseline
- **WHEN** canonical saved values are edited and later exactly equal the current clean baseline again
- **THEN** the document SHALL return to clean status

#### Scenario: Save completes after a later edit
- **WHEN** a captured session is written successfully but canonical saved values changed after that capture began
- **THEN** the exact written capture SHALL become the baseline and document identity SHALL update as applicable, while the active document SHALL remain dirty

### Requirement: New replaces the session through the prepared graph path
New SHALL construct, validate, and completely prepare the canonical default session using the same candidate and graph path used by Open. It SHALL not expose partial canonical state. Successful activation SHALL replace only persisted session content and reset performance interaction to the default PATCH Engine focus while retaining shell-owned document infrastructure and application preferences.

#### Scenario: New succeeds
- **WHEN** New is requested for a clean document and the default candidate validates, prepares, and activates
- **THEN** the active state and graph SHALL change together to the canonical default session and the document SHALL become clean and Untitled

#### Scenario: New preparation fails
- **WHEN** New cannot validate, prepare, stage, or activate a complete default candidate
- **THEN** the prior session, graph, document identity, clean baseline, dirty state, and usable shell SHALL remain active and the exact failure SHALL be visible

#### Scenario: New is cancelled by the user
- **WHEN** a prerequisite unsaved-change prompt is cancelled
- **THEN** New SHALL perform no candidate preparation or active-session change

### Requirement: Open is a transactional complete-session replacement
Open SHALL use a native file-selection boundary and, after a path is selected, perform file read, decode, supported migration, shape and capability validation, asset resolution, projection, and complete graph preparation off the audio callback. The active session and graph SHALL remain usable until one candidate is ready. Candidate activation SHALL occur only at a block boundary and SHALL be correlated with one canonical replacement so no mixed candidate/active projection, parameter snapshot, or revision becomes user-visible.

#### Scenario: Open succeeds
- **WHEN** the selected file reads, decodes, migrates, validates, prepares, stages, and activates successfully
- **THEN** the complete candidate state and graph SHALL replace the prior pair, the selected exact location SHALL become document identity, the document SHALL be clean, and focus SHALL reset deterministically to the active Patch's Engine in PATCH

#### Scenario: Open dialog is cancelled
- **WHEN** the user cancels file selection
- **THEN** Open SHALL be a non-error no-op that preserves the active session, graph, document identity, baseline, dirty state, and focus

#### Scenario: Open fails before activation
- **WHEN** file read, decode, migration, validation, capability or asset resolution, projection, or complete graph preparation fails
- **THEN** the candidate SHALL be discarded off callback, the prior valid session and graph SHALL remain usable, document identity and dirty state SHALL remain unchanged, and the exact failure category and cause SHALL be visible

#### Scenario: Open cannot stage or activate
- **WHEN** the complete candidate cannot be handed off or acknowledged for block-boundary activation
- **THEN** the prior canonical session and active graph SHALL remain correlated and usable, the candidate SHALL retire off callback, and the handoff failure SHALL be visible

### Requirement: Save and Save As write atomically without lying about identity
Save SHALL capture and encode the active canonical session in the current version. If the document has an exact location, Save SHALL target it; if it is Untitled, Save SHALL execute the Save As selection flow. Save As SHALL request a location even when one already exists. A write SHALL use a same-destination temporary file and atomic replacement so failure does not truncate or partially replace a prior valid file. Save As MUST establish the new document location only after replacement succeeds.

#### Scenario: Save writes an identified document
- **WHEN** Save is requested for a document with an existing location and encoding, temporary write, flush, and replacement all succeed
- **THEN** that exact location SHALL contain the captured current-version saved session and the corresponding baseline SHALL be clean unless later canonical edits exist

#### Scenario: Save on an untitled document
- **WHEN** Save is requested for an Untitled document
- **THEN** the shell SHALL run Save As and SHALL establish the selected exact location only after a successful atomic write

#### Scenario: Save As changes location
- **WHEN** Save As selects a different location and atomic writing succeeds
- **THEN** the new exact location SHALL become document identity while any former file remains unchanged

#### Scenario: Save As selection is cancelled
- **WHEN** the user cancels Save As selection
- **THEN** the active document location, baseline, dirty state, session, graph, and existing files SHALL remain unchanged without reporting a failure

#### Scenario: Encoding or writing fails
- **WHEN** encoding, temporary creation, byte writing, flushing, syncing, or atomic replacement fails
- **THEN** the active document identity and clean baseline SHALL remain unchanged, dirty work SHALL remain recoverable in memory, any prior valid destination SHALL remain untruncated, and the exact save stage and cause SHALL be visible

### Requirement: Dirty work is guarded before replacement or close
When the active document is dirty, New, Open, and application close SHALL require an explicit Save, Discard, or Cancel decision. Save SHALL continue the pending action only after the required save succeeds. Discard SHALL continue without saving. Cancel, a cancelled Save As dialog, or any save failure SHALL abort the pending action and preserve the active document.

#### Scenario: Save then continue
- **WHEN** the user chooses Save from the dirty-document guard and the save succeeds
- **THEN** the originally requested New, Open, or close action SHALL resume exactly once

#### Scenario: Discard then continue
- **WHEN** the user chooses Discard from the dirty-document guard
- **THEN** the requested New, Open, or close action SHALL continue without writing the dirty session

#### Scenario: Cancel preserves work
- **WHEN** the user chooses Cancel, cancels the required Save As selection, or the required save fails
- **THEN** the pending New, Open, or close action SHALL stop and the dirty active session, graph, document identity, and focus SHALL remain usable

#### Scenario: Clean document needs no guard
- **WHEN** New, Open, or close is requested while the document is clean
- **THEN** the shell SHALL proceed without showing an unsaved-change decision

### Requirement: Lifecycle status and failures are truthful and non-canonical
The shell SHALL present in-progress lifecycle status and every file-dialog, filesystem, decode, migration, validation, capability, asset, projection, preparation, activation, encoding, and save failure in text or shape as well as color. Presentation SHALL identify the failed operation and stage without inventing a fallback. Dialog cancellations SHALL be distinguished from failures. Status and failure presentation MUST NOT mutate or serialize canonical session content.

#### Scenario: Typed Open failure is shown
- **WHEN** Open fails at a known stage
- **THEN** the shell SHALL identify Open, the failed stage, and its actionable typed cause while continuing to project the prior usable session

#### Scenario: Typed Save failure is shown
- **WHEN** Save or Save As fails at a known stage
- **THEN** the shell SHALL identify the save operation, failed stage, and its actionable typed cause and SHALL continue to mark unsaved canonical work accurately

#### Scenario: Cancellation is not an error
- **WHEN** a file dialog or unsaved-change guard is cancelled
- **THEN** the shell SHALL return to the prior usable state without a fabricated error status

### Requirement: Production evidence proves lifecycle behavior and real-time safety
Deterministic acceptance SHALL drive normal startup and New, Open, Save, and Save As through the production command, reducer, projector, persistence, preparation-worker, structural-graph, audio-render, and shell boundaries. Evidence SHALL cover success, cancellation, and each controlled failure family, assert exact state/document/graph preservation on failure, and retain callback safety and off-thread graph retirement.

#### Scenario: Successful lifecycle matrix
- **WHEN** deterministic production-boundary tests execute successful startup, New, Open, Save, and Save As journeys
- **THEN** each SHALL prove exact canonical capture, document identity, dirty state, graph revision, projection correlation, and renderability rather than only constructing a fixture or printing a success token

#### Scenario: Failed replacement matrix
- **WHEN** controlled file, decode, migration, validation, capability, preparation, handoff, and activation failures are injected
- **THEN** evidence SHALL distinguish each negative path and prove the prior state hash, saved capture, document identity, dirty status, graph revision, and usable audio path remain unchanged

#### Scenario: Real-time invariants survive session operations
- **WHEN** session preparation, activation, save, failure, and candidate retirement are exercised while the renderer runs
- **THEN** the audio callback SHALL perform no allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwinding, or graph destruction and SHALL activate complete graphs only at block boundaries
