## 1. Patch Position and Capacity Foundations

- [x] 1.1 Define one dependency-neutral active-Patch capacity constant with value 16, replace control/RT/persistence duplicates with it, and verify focused capacity and compile-time array-size tests pass.
- [x] 1.2 Add the canonical `PatchPositionId::Created(PatchId) | TrailingEmpty` type and exports, including explicit created-ID accessors, and verify unit tests reject treating `TrailingEmpty` as a `PatchId`.
- [x] 1.3 Migrate PATCH `FocusPath`, remembered roots, return paths, subordinate sessions, and suspended MIDI Settings focus to `PatchPositionId` while preserving created-Patch serialization and access behavior; verify semantic-focus and MIDI Settings invariant tests pass.
- [x] 1.4 Extend `InteractionState` with one exhaustive empty-to-created rekey operation covering active, remembered, return, subordinate, and suspended paths, and verify exact-preservation, schema-repair, and user-navigated-away unit cases.
- [x] 1.5 Update event recording, state hashing, public surface descriptors, and static one-type guards for the new position identity, and verify exhaustive descriptor/schema tests contain no magic sentinel Patch ID.

## 2. Empty Navigation and Creation Blueprint

- [x] 2.1 Make semantic Patch-position order derive created identities plus exactly one trailing empty endpoint, update non-wrapping `SelectPatch` recovery for created↔empty transitions, and verify Shift+Right/Left, first/empty boundaries, subordinate-close, and heterogeneous-control tests pass through `AppState::apply`.
- [x] 2.2 Factor Phase 01's designated default capability/configuration into an injected immutable `PatchCreationBlueprint` shared by default-session and prospective creation paths, and verify reordered/augmented registries cannot change the exact designated default.
- [x] 2.3 Implement checked candidate construction (`max PatchId + 1`, `Patch {id}`, appended channel/track, 0 dB trim, neutral envelope, seeded voice limit, three empty slots), and verify one-Patch, sparse-ID, sixteenth-Patch, identity-exhaustion, and missing-default tests.
- [x] 2.4 Extend the semantic resolver with prospective Overview, default-Engine Detail, Choice, and Utility schemas backed by the blueprint rather than a placeholder `Patch`, and verify inspection/navigation changes no Patch vector, saved capture, parameter snapshot, or graph revision.
- [x] 2.5 Classify empty-position actions so accepted Patch-owned Engine/effect/Detail/Utility edits create one candidate while navigation, surface/mode changes, preview, MIDI, global-only edits, rejected boundaries, and unchanged `EMPTY` choices do not; verify one focused reducer matrix covers every action family.
- [x] 2.6 Add typed capacity, identity, default/capability, busy, stale, preparation, publication, and activation failure presentation for creation, and verify valid actions omit creation at 16 while direct stale commands reject without worker submission.

## 3. Reducer-Owned Structural Creation Lifecycle

- [x] 3.1 Extend the shared structural intent/correlation vocabulary with append-Patch identity and pending candidate ownership without adding a second lifecycle or coordinator, and verify exhaustive intent/context/effect/event descriptors pass.
- [x] 3.2 Add reducer transitions for creation Loading, Validating, Preparing, Activating, Failed/Unavailable, and matching acknowledgement, keeping `AppState::patches` unchanged until the final event; verify generation, requested/active, busy, duplicate, stale, and failure tests.
- [x] 3.3 Ensure the matching activation event is the only branch that appends the pending canonical `Patch` through `AppState::apply`, installs the target revision, rekeys focus, and reports `saved_session_changed`; verify precommit events are session-neutral and success appends exactly once.
- [x] 3.4 Integrate prospective Choice and Detail commit/close behavior with creation correlation and focus repair, and verify default Engine confirmation, alternate Engine selection, effect-slot choice, descriptor/envelope edit, exact return origin, and invalid-origin repair cases.
- [x] 3.5 Keep MIDI fan-out limited to created subscriptions until commit and preserve existing target order across append, and verify performance input while empty/pending neither creates nor targets the candidate while all prior shared-channel subscribers remain ordered.

## 4. Complete Graph Append and Real-Time Safety

- [x] 4.1 Add an append request constructor that validates source order, unique/capacity-valid candidate identity/config/effects/route, target revision, and full candidate parameters before worker submission; verify controlled invalid and graph-capacity cases return exact typed failures.
- [x] 4.2 Teach the existing graph preparation worker to build and warm `active Patches + candidate` with unchanged Mixer/global/returns and no partial publication, and verify successful and injected capability/asset/effect/allocation failures preserve candidate ownership for control-side retirement.
- [x] 4.3 Add `GraphReplacementScope::AppendPatch` layout admission requiring one exact final position and identical prior Patch/return/device layout, and verify insertion, replacement, removal, reorder, unrelated slot/return change, wrong identity, and seventeenth-Patch layouts are rejected.
- [x] 4.4 Carry live engine/effect/return state for existing identities across an admitted append while the new Patch starts silent, and verify focused renderer tests distinguish retained prior audio from the unvoiced candidate.
- [x] 4.5 Add control-side clone preflight of the future reducer commit and exact prepared-layout/parameter comparison before staging, and verify a mismatched state, candidate, identity, order, scalar shape, value, or revision never reaches the structural boundary.
- [x] 4.6 Refresh target-revision initial parameters from latest accepted source scalars plus the immutable candidate immediately before staging, and verify edits accepted during preparation remain exact while source-revision renderers ignore incompatible snapshots.
- [x] 4.7 Integrate prepared/failed append results with the existing worker poll, coordinator staging, block-boundary activation, matching retirement acknowledgement, and off-thread destruction, and verify busy/full/stale/incompatible/publication failures leave the prior graph usable.
- [x] 4.8 Exercise creation preparation, swap, render, failure, and retirement under callback instrumentation, and verify zero callback allocations, deallocations, locks, blocking, I/O, logging, formatting, panics, unwinding, and graph destruction.

## 5. Projection and Webview Rendering

- [x] 5.1 Add explicit created/empty Patch-page and semantic-shell projection variants with no Patch ID on empty, prospective versus active ownership, active count/capacity, lifecycle/failure, focus/return, and reducer-derived actions; verify projector tests reject identity or ownership disagreement.
- [x] 5.2 Advance the state-tree schema once for the tagged empty shape and update Rust/JavaScript serialization consumers, exact leaf descriptors, fixtures, and mismatch rejection; verify exact serialization and CSP/headless policy tests pass.
- [x] 5.3 Render empty Overview, prospective default-Engine Detail, Choice, and Utility through the existing projected section/control compositions, including explicit NEW/DEFAULT, pending phase, failure, and CAPACITY 16/16 text/shape; verify JavaScript contains no Patch-count inference, fake identity, or capability-name switch.
- [x] 5.4 Extend headless DOM correspondence tests for empty, prospective inspection, every pending phase, failed/unavailable, capacity, and newly created states, and verify one focus, exact valid actions, footer-only guidance, and removal of empty treatment after commit.
- [x] 5.5 Extend the native responsive witness across Wide, Standard, Intermediate, Compact, and enlarged-text empty-state fixtures, and verify 48 px target floors, complete scroll endpoints, no required-content overlap or document horizontal overflow, deterministic repaint, and resize-neutral semantic identity.

## 6. Phase 01 Persistence and Lifecycle Integration

- [x] 6.1 Keep `SavedSession` version 2 capture limited to created Patches and add negative serialization checks for position/focus/prospective/pending/capacity/graph fields; verify Save while resting on empty decodes to the exact prior created set.
- [x] 6.2 Connect reducer acceptance to document dirty tracking so empty navigation and precommit/failure events are neutral while acknowledged append is saved content, and verify exact clean-baseline equality before, during, after, and on retry.
- [x] 6.3 Cover Save/Save As during creation preparation and after later acknowledgement, and verify the written baseline contains only the acknowledged set while subsequent successful creation remains visibly dirty.
- [x] 6.4 Add one structural-exclusion policy for pending creation versus authorized New/Open/guarded Close and existing Engine/effect edits, and verify Busy/defer behavior prevents dual candidates and refuses new saved-field actions after replacement authorization.
- [x] 6.5 Reject and retire append results whose source revision was superseded by session replacement, and verify they cannot alter the replacement state, document identity, baseline, graph revision, focus, or audio.
- [x] 6.6 Add a production Save→close→Open round trip for implicitly created Patches, and verify exact identities/order/config/routing survive, no sentinel is encoded, Phase 01 focus reset occurs, and a fresh trailing empty position is derived.
- [x] 6.7 Open and activate an exact 16-Patch saved session, then verify all 16 render, the fresh trailing empty position reports capacity, and no seventeenth Patch or audio-inactive model is synthesized.

## 7. Production Acceptance and Documentation

- [x] 7.1 Add focused production-path acceptance for first edits from Engine confirmation/selection, Post FX choice, default-Engine Detail, and each Patch-owned Utility family, and verify each journey emits one semantic request, one prepared append, one activation commit, one stable identity, and matching projection/snapshot/graph revisions.
- [x] 7.2 Add controlled failure/retry acceptance for identity/default/capability/asset/effect/allocation/worker/publication/stale/capacity causes, and verify the prior saved capture, dirty state, focus, graph revision, rendered audio, and off-thread ownership remain recoverable.
- [x] 7.3 Extend production keyboard normalization and the bounded native handoff with Shift+Right → empty → non-creating inspection → first edit → created → Shift+Left, and verify AppKit Shift handling, one focus, exact return identity, resize invariance, and clean teardown without adding physical gamepad scope.
- [x] 7.4 Compare the rendered empty/pending/failure/capacity/created states directly with the live Figma Patch Overview and Interaction Map nodes, and record measured alignment plus the explicit 16-Patch unlimited-workflow mismatch without claiming broad visual parity.
- [x] 7.5 Run formatting, warnings-denied lint, exact-selector/no-name guards, focused OpenSpec acceptance, `cargo test --all-targets`, and the scoped native webview target; verify every deterministic command passes and environmental native skips remain explicitly incomplete rather than accepted.
- [x] 7.6 Update `DESIGN.md` in the implementation commit with the verified Patch-position identity, shared blueprint, append graph scope, persistence/capacity behavior, known Figma mismatch, commands, and measured evidence, and verify it makes no roadmap or unproved parity claim.
