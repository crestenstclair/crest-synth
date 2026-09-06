## Closure scope — 2026-09-05

Closed at the user's request before starting another OpenSpec change. The
accepted scope is the functional Sample slice: default test asset and MIDI,
Detail waveform and editing, the shared in-app file page, nested navigation,
validated import, cancellation/stale-result safety, and saved-session waveform
restoration. Resize acceptance remains closed. Figma guides hierarchy and
interaction; exact pixels, typography, spacing, and exhaustive visual comparison
are not completion gates. Cloud downloading remains the storage provider's job.

The original checklist records 24 completed tasks and 36 unchecked tasks.
Unchecked items in sections 2, 4, 5, 6, 7, and 8 retain their historical status:
they cover broader visual work and aggregate validation that were not completed
as specified. They are deferred from this closed functional scope, not reported
as implemented or proven. Existing checks are recorded below and in DESIGN.md;
this closure performs documentation validation, not another product acceptance
run. SoundFont bank loading remains unimplemented and belongs to a later change.

The four delta specs are reconciled with the functionality-first acceptance
policy and synchronized to the main specs before archival. Main specs describe
behavioral contracts; synchronization is not evidence that every broader visual
requirement has already been implemented. The historical plan below explains the
original work, and this closure scope governs any conflicting completion gate.

## 1. Normative Measurements and Evidence Infrastructure

- [x] 1.1 Re-read live Figma nodes `98:2`, `39:92`, `41:138`, `42:3`, `48:173`, `48:207`, `49:3`, and `95:202`; create the bounded composition ledger and verify it records retrieval identity/time, hierarchy, relative emphasis, type roles, spacing rhythm, flexible region relationships, state markers, focus, and fixture-versus-production distinctions for every Phase 03 surface.
- [x] 1.2 Export representative normative frames and define a review manifest linking Figma node, retrieval time, observed window condition, device/text scale, fixture identity, semantic-document hash, and native paint acknowledgement; verify a self-test rejects missing identity or correspondence fields without treating the frame resolution as an acceptance target.
- [x] 1.3 Add deterministic side-by-side and annotated composition-review output for paired Figma/native captures; verify the review detects a controlled hierarchy/state regression while ignoring harmless coordinate differences caused by responsive reflow.
- [x] 1.4 Audit the existing Sample, browser, Mixer, Inspector, option, shell-token, computed-style, and native-observation projections against the ledger; record every missing generic presentation fact and verify the audit contains no capability-name, fixture-count, or renderer-owned-state proposal.
- [x] 1.5 Extend the production native witness to resize one unchanged serialized document through a varied generated sequence of constrained, intermediate, and expanded widths plus live manual dragging; verify it exercises widths outside named Figma examples, crosses every observed reflow, and fails on a missing paint acknowledgement without encoding a resolution table.
- [x] 1.6 Extend native observations to record document hash, generation, focus, subject, return identity, valid actions, device scale, minimum-target compliance, overlap, overflow, scroll endpoints, observation compatibility, and input/reducer event counts; verify malformed or incomplete evidence is rejected and geometry is used only to falsify layout defects.

## 2. Shared Projection and Visual Vocabulary

- [ ] 2.1 If ledger-backed authored distinctions are absent, add only the audited generic projector/serializer facts and advance the serialized schema version; verify reducer-to-projection-to-JSON correspondence, backward fixture handling, and absence of concrete capability-name branches.
- [ ] 2.2 Add any ledger-required shared color, type, spacing, radius, keyline, focus, fader, meter, or responsive values to the Rust-owned token vocabulary and generated CSS; verify token export, canonical naming, literal guards, and component-vocabulary tests pass.
- [ ] 2.3 Extend computed-style and responsive-relationship observation for authored type roles, grouping, state shapes, focus/adjustment treatments, faders, meters, waveforms, and footer ownership; verify controlled hierarchy, clipping, and overflow negatives are detected without comparing against fixed Figma coordinates.
- [ ] 2.4 Add maximum-content, long-label, explicit unavailable/failure, stale observation, and enlarged-text fixture builders shared by the Phase 03 witnesses; verify every fixture is produced through `AppState::apply`, projection, and serialization rather than hand-authored DOM state.

## 3. Sample Detail Visual Slice

The completed Detail presentation slice includes the normal-entry regression
in 3.7. The current functional expansion is tracked in S1–S6 below. The user has
accepted resize behavior and explicitly directed that the manual handoff not
be repeated. Task 1.5 records that acceptance alongside the passing generated
native witness; it does not claim a new automated manual-drag PASS. Sample
Detail's programmatic layout evidence already passed in
`make test-webview-sample-native` (13 generated widths, enlarged text, exact
focus, target floor, reachability, and no required overlap or horizontal
overflow). Preserve that acceptance rather than reopening it.

Current implementation correspondence: `detailShellHtml` and `waveformHtml`
cover 3.1–3.2; `AppState::sample_visualization` and the canonical visualization
projector own 3.4; `tests/sample_detail_visual.rs` and the shared Sample native
fixtures cover 3.5. The 2026-09-05 functional acceptance supersedes further
cosmetic comparison: Figma guides hierarchy and interaction, without exact
pixel, type, or spacing matching. Native captures and checks cover incompatible
data, alternate descriptors, long paths, current scalar landmarks, and visible
browser admission. The subsequent user request authorizes in-app selection/import, nested browsing,
waveform restoration, and startup test MIDI in S1–S6. Further cosmetic work
and Mixer/Options changes remain outside this handoff.

Verification: `cargo test --test phase7_sample_workflows --test sample_detail_visual`
passes 18 tests, including Engine options through worker preparation, audio
activation, and the first Detail waveform. `make test-webview-sample-native`
passes renderer correspondence and owned native teardown. Native ready,
adjusting, failed, incompatible, and long-asset captures were inspected.
The expanded functional tests also cover restored waveforms and edited landmarks;
native Save/Open confirms the imported file and waveform survive replacement.

- [x] 3.1 Implement the Sample-specific Detail hierarchy from projected Patch/Sample identity, asset and lifecycle readings, waveform/landmarks, ordered controls, persistent Utility, breadcrumb, and footer guidance; verify renderer correspondence tests cover ready, requested-versus-active, unavailable, and failed states.
- [x] 3.2 Implement readable Sample Detail type roles, grouping, keylines, visualization anatomy, focus/adjustment, disabled, loading, unavailable, and failure treatments through shared tokens; verify computed-style role assertions and text-or-shape state checks pass, with Figma as a guide rather than an exact visual gate.
- [x] 3.3 Make Sample Detail fluid through varied constrained, intermediate, and expanded widths plus enlarged text with bounded main/Utility scrolling; verify the canonical minimum target, reachable first/last controls, zero required overlap, and zero document horizontal overflow without viewport-specific markup.
- [x] 3.4 Render missing or incompatible waveform/landmark data as an explicit unavailable or stale state without fabricated geometry; verify positive compatible data and controlled stale/missing negatives through the production visualization projection.
- [x] 3.5 Add deterministic Sample Detail state and repeat-paint coverage for differently shaped Sample descriptors, long assets, all structural lifecycle phases, and exact stable focus/return identity; verify focused reducer/projector/serialization/headless-renderer tests pass.
- [x] 3.6 Capture Sample Detail natively at representative conditions and inspect functional clarity, using Figma `39:92` as a hierarchy guide; verify readable waveform, controls, state, and action presentation while retaining already accepted resize evidence without a repeated manual handoff.
- [x] 3.7 Retain the prepared waveform after choosing Sample as Engine and show the existing browser affordance on Sample File; verify the normal options/worker/audio-activation/Detail journey and native `BROWSE` presentation without first assigning another file.

## 4. Sample Browser Visual Slice

The user expanded this handoff to the complete functional Sample slice.
The following functional tasks take precedence over further visual polish:

- [x] S1 Install Sample with a bundled test asset and a writable per-user library when no explicit environment configuration is supplied; preserve typed override failures.
- [x] S2 Add Return-to-open in-app file selection, bounded validated import, cancellation/failure/stale-result handling, stable focus, and assignment only after graph acknowledgement.
- [x] S3 Load nested browser folders through an off-thread catalog adapter; preserve stable focus, cancellation, and hold-to-preview behavior.
- [x] S4 Hydrate waveform summaries from prepared graphs on startup and saved-session replacement; verify file identity and current landmark edits survive Save/Open.
- [x] S5 Launch an audible repeating test MIDI pattern through production MIDI fan-out; expose start/stop and suspend test notes during browser audition and session replacement.
- [x] S6 Exercise native file selection, browser navigation/preview, scalar and loop editing, save/reopen, audible render output, failures, and clean teardown; update the as-built record without cosmetic or manual-resize work.

Picker correction (2026-09-05): the user rejected the native asset dialog.
Earlier native-dialog evidence does not establish the requested picker workflow.
SoundFont file switching is not claimed: the shared browser supports SF2 listing,
while the current engine remains pinned to HiDef pending its loader integration.

- [x] F1 Route Return and OpenRelated to one in-app file page; remove the native asset-dialog future and preserve canonical confirm/back/focus controls.
- [x] F2 Extract shared file/folder/row/listing types and filesystem navigation with WAV/SF2 filters; retain Sample-specific decoding and hold-to-preview.
- [x] F3 Expose Home/Volumes alongside the library and import external Sample selections through a correlated worker before structural assignment; reject transient references in saved sessions.
- [x] F4 Verify native in-app navigation, folder entry/back/cancel, Sample selection, waveform/audio, generic SF2 filtering, stale imports, and owned close; update DESIGN without reopening resize acceptance.
- [x] F5 Report zero-byte Dropbox placeholders as download-required with actionable browser/Detail text, preserve the current asset and focus, and prove import succeeds after download while empty local files remain a distinct failure.

F5 evidence: the native Dropbox-attribute regression reproduced the old
unsupported-container failure before the fix and passed afterward, including
retry with downloaded bytes and retained provider metadata. A read-only check
through the production catalog reported DownloadRequired for an existing WAV
in the reported Dropbox tracker folder. The 35 focused Sample library tests,
20 Sample/Detail integration tests, serialized-schema test, and native Sample
witness passed; the witness verifies unavailable row text and rejects an
INVALID marker for missing audio. Actual Dropbox downloading is user-managed.

Correction evidence: 868 library tests passed (two existing measurement-only
tests ignored), 19 Sample/Detail tests passed, shared WAV/SF2 filtering and
transient-reference persistence rejection passed, serialized schema and
warnings-denied Clippy passed. Native production execution verified Return
entry, nested folders, Home/Music navigation, Cancel preserving the asset and
origin, library assignment, external Home-based WAV import, READY waveform,
and owned close. The imported copy remained readable after its original source
was removed. The final native renderer witness and strict OpenSpec validation
passed. No new manual resize approval or SoundFont bank-switching claim.

- [ ] 4.1 Implement the authored Sample Browser hierarchy from projected library/folder identity, canonical rows, metadata, waveform, preview/playhead, status, and footer guidance; verify populated, empty, loading, unavailable, failed, parent, folder, file, and cancel correspondence fixtures.
- [ ] 4.2 Implement ledger-backed browser row rhythm, kind markers, focus/selection independence, metadata wrapping, preview geometry, and text-or-shape lifecycle states; verify computed-style and structural-state assertions pass without deriving identity from labels or row indices.
- [ ] 4.3 Preserve the existing hold-to-preview observation path while painting preparing, held, playing, stopping, stopped, stale, and failed states; verify Start press/release changes no saved asset, semantic focus, browser location, or product generation beyond canonical preview events.
- [ ] 4.4 Make Sample Browser and persistent Utility fluid through varied widths with bounded row/preview scrolling; verify the canonical minimum target, reachable row and Utility endpoints, zero required overlap, and zero document horizontal overflow under maximum content, long labels, and enlarged text.
- [ ] 4.5 Add deterministic browser reprojection, exact return-origin, sibling-Patch, empty-Patch prospective, and preview-correlation regressions; verify normalized input journeys paint the same reducer-owned focus and return identities before and after resize.
- [ ] 4.6 Capture Sample Browser natively at representative conditions, review it side by side with Figma `41:138`, and exercise fluid live/programmatic resizing; verify hierarchy, rhythm, state clarity, responsive behavior, and the meaningful-discrepancy ledger pass or leave Sample Browser explicitly incomplete.

## 5. Mixer and Inspector Visual Slice

- [ ] 5.1 Implement the authored sixteen-track Mixer Main anatomy—Track Header, Level Fader, Level Readout, Pan Readout, State Line, and passive meter—from canonical projected controls; verify T00 through T0F remain in stable order with exactly one semantic focus.
- [ ] 5.2 Implement ledger-backed track rhythm, fader and meter anatomy, level/pan readouts, focus/adjustment, track correlation, mute, solo, global gate, disabled, stale, and error treatments; verify every state has text or shape in addition to color and computed-style role checks pass.
- [ ] 5.3 Complete the correlated Inspector hierarchy for focused identity/value/range, meter, mute/solo, routed Patches, sends, return occupancy/levels/parameters, and global controls; verify exact track/control correlation and canonical Inspector focus order for empty, maximum, unavailable, and failed content.
- [ ] 5.4 Keep meter painting compatible and observation-only for both track and Inspector readings; verify matching generation/revision updates both, stale or mismatched observations paint explicit zero/stale treatment, and no observation mutates `AppState` or saved state.
- [ ] 5.5 Make the sixteen-column bank and bounded Inspector adapt from available space and content, placing or stacking regions in semantic order as their intrinsic constraints require; verify T00/T0F and Inspector endpoints remain reachable with the canonical minimum target, bounded region scrolling, zero workspace/Inspector overlap, and zero document horizontal overflow.
- [ ] 5.6 Add deterministic Mixer/Inspector renderer coverage for all four main parameters on all sixteen tracks, routing variants, sends/returns/global controls, meter compatibility, long labels, maximum registries, and repeat paint; verify reducer/projector/serialization/headless tests pass.
- [ ] 5.7 Drive normalized keyboard/controller Mixer navigation, adjustment, Main↔Inspector movement, PATCH↔MIXER return, and resize-only sequences through the production path; verify exact focus/correlation, unchanged resize generation, and no newly available multi-select action.
- [ ] 5.8 Capture Mixer and Inspector natively at representative states, review them side by side with Figma `42:3`, and exercise fluid live/programmatic resizing; verify all sixteen identities, hierarchy, rhythm, state clarity, responsive relationships, and the meaningful-discrepancy ledger pass or leave Mixer explicitly incomplete.

## 6. Engine and Post FX Option Visual Closure

- [ ] 6.1 Reconcile the shared option identity header, origin annotation, list rhythm, current-versus-focus markers, Utility, and single footer guide with ledger measurements; verify Engine and every occupied/empty Post FX slot use the same generic renderer without changing option order or reducer workflow.
- [ ] 6.2 Reconcile disabled, unavailable, loading, validating, preparing, activating, ready, and failed option treatments with text and structural shape; verify active/requested/current/focus facts remain distinct for long labels, duplicates, maximum registries, and typed causes.
- [ ] 6.3 Make option hierarchy and list scrolling fluid through varied widths, heights, long content, and enlarged text; verify every observed reflow retains one focus, the canonical minimum target, reachable list/Utility endpoints, zero required overlap, and zero document horizontal overflow.
- [ ] 6.4 Rerun normalized Engine and every Post FX origin entry, navigation, choose-current no-op, choose-different request, close-unchanged, exact/repaired return, and Shift regression journeys; verify visual changes preserve the existing `AppState::apply` event sequence and structural lifecycle correlations.
- [ ] 6.5 Capture Engine Options states natively, review them side by side with Figma `48:173`, and exercise fluid live/programmatic resizing; verify hierarchy, rhythm, state clarity, responsive behavior, and its meaningful-discrepancy ledger pass or leave Engine Options explicitly incomplete.
- [ ] 6.6 Capture Post FX Options occupied, empty, duplicate, focus/current-separated, unavailable, loading, and failed states natively, review them side by side with Figma `48:207`, and exercise fluid live/programmatic resizing; verify hierarchy, rhythm, state clarity, responsive behavior, and its meaningful-discrepancy ledger pass or leave Post FX Options explicitly incomplete.

## 7. Cohesive Native Polish and Regression Gate

- [ ] 7.1 Confirm Sample Detail, Sample Browser, Mixer, Engine Options, and Post FX Options each have passing direct-comparison ledgers before changing shared polish; verify the gate refuses to proceed when any named surface is incomplete.
- [ ] 7.2 Apply the final shared shell, typography, spacing, hierarchy, focus, status, control-state, Utility/Inspector, and footer refinements using only authored tokens; verify no duplicated action guidance, ad hoc literals, color-only states, or local semantic state remains.
- [ ] 7.3 Rerun the Phase 03 representative capture and responsive-behavior reviews after shared polish and regenerate every review manifest; verify prior meaningful discrepancies do not return and all manifests reference the final paint/document identities.
- [ ] 7.4 Rerun Patch Overview, empty-Patch creation, Instrument/FX Detail, session lifecycle, and MIDI Settings structural/native regressions affected by shared shell changes; verify focus, return, state truth, reachability, and existing evidence boundaries remain intact without claiming new fidelity for out-of-scope surfaces.
- [ ] 7.5 Run the aggregate varied-width resize-only exploration for every Phase 03 surface, combining generated window conditions, live dragging, long content, and expansion stress; verify deterministic paint with zero semantic input/reducer events, minimum-target failures, required overlaps, or document horizontal overflow and no resolution-specific renderer branch.
- [ ] 7.6 Exercise native device-scale reporting and enlarged-text conditions for every Phase 03 surface; verify manifests record the actual host conditions and any unavailable scale remains an explicit evidence limitation rather than an inferred pass.

## 8. Final Validation and As-Built Record

- [ ] 8.1 Run formatting, generated-token freshness, JavaScript syntax, warnings-denied lint, exact-selector self-test, no-name-enumeration guard, focused Phase 03 suites, and `cargo test --all-targets`; verify every required command passes without silently skipped deterministic groups.
- [ ] 8.2 Run the scoped production native WKWebView target to completion in an interactive host; verify real-window creation, all required captures/sweeps, paint acknowledgements, normalized input regressions, and owned teardown pass, with environmental skips reported as incomplete.
- [ ] 8.3 Verify the real-time boundary and compatible observation regressions after visual work; confirm callback allocation, destruction, locking, blocking, I/O, logging, formatting, panic, and render/layout work remain absent and separate transports are unchanged.
- [ ] 8.4 Review every readable native capture, composition ledger, responsive-behavior report, and meaningful-discrepancy entry against the live Figma nodes; verify completion is withheld for unexplained hierarchy, state, rhythm, reachability, or reflow failures while harmless coordinate differences are not treated as defects.
- [ ] 8.5 Update `DESIGN.md` with only measured final as-built results, exact production-path commands, Figma nodes, responsive conditions and reflows exercised, native/text-scale evidence, generic projection/token changes, and honest remaining gaps; verify it does not cite OpenSpec as proof, publish a fixed resolution matrix, or broaden claims to unmeasured surfaces/platforms.
- [ ] 8.6 Run `openspec validate phase-03-figma-visual-fidelity --strict` and inspect the final change diff; verify all implementation tasks, delta specs, evidence references, and `DESIGN.md` claims are coherent and no unrelated user worktree changes were modified.
