## ADDED Requirements

### Requirement: Live demo exercises and retains configured Chorus through physical completion
The autonomous live scene SHALL derive Chorus Amount and Depth from the production PATCH resolver, edit each exactly once through semantic PATCH input with a visible dwell, and bracket each edit with matching bounded Patch-targeted NoteOn/NoteOff probes. Each accepted edit SHALL wait for a newer physical `AudioObservationSnapshot` whose parameter generation is exact and whose real `PatchEffectObservation` identifies the configured Patch, reports finite nonzero input, causal pre/post difference, and stereo side energy. Fixture probes SHALL use `AppState::apply` and SHALL NOT earn scalar coverage.

#### Scenario: Amount and Depth checkpoints complete
- **WHEN** `make demo-live` reaches the configured first Patch's effect rows
- **THEN** both semantic adjustments are accepted, page/text/state/fixed snapshot values agree, their values dwell visibly, exact-generation physical effect observations pass, and matching NoteOff events are dispatched afterward

#### Scenario: Unrelated audio advances
- **WHEN** the master output changes because another Patch or a time-varying tail sounds while the awaited configured effect observation has not advanced
- **THEN** the runner does not credit that block or advance the Chorus checkpoint

**Contract facet — structural and lifecycle completion retains Chorus.**
After scalar coverage, the live demo SHALL complete one adjacent SoundFont preset replacement and SoundFont→Braids→descriptor-default-SoundFont for the focused first Patch through the production threaded worker. Every Preparing, Activating, and Ready checkpoint SHALL retain the exact Chorus slot/config/layout, wait for a newer acknowledged graph revision, and require finite nonzero targeted physical output through the effect stage. Completion SHALL include semantic all-notes-off, a later zero-active-note observation, final report emission, window close, stream release, off-callback graph/effect cleanup, worker shutdown, and parent-process success.

#### Scenario: Live structural sequence succeeds
- **WHEN** all three structural requests activate normally
- **THEN** the final Ready Patch uses descriptor-default SoundFont and its default preset with the original Chorus config, exact scalar/structural coverage is complete, fallback/callback-destruction counts are zero, and `make demo-live` exits zero after teardown

#### Scenario: Live effect checkpoint makes no progress
- **WHEN** no semantic, effect-observation, structural, or cleanup milestone advances for ten seconds, total time exceeds 120 seconds, or the user closes the window before the completed report
- **THEN** the run retains a typed stage-specific failure, performs available semantic note cleanup, closes/releases/drains ownership off callback, exits nonzero, and emits no completed report

#### Scenario: Success text appears before external teardown
- **WHEN** checkpoints or a summary are printed but the native window, stream, worker, graph/effect ownership, or parent process remains live or failed
- **THEN** physical live acceptance is incomplete and apply SHALL NOT mark the change done
