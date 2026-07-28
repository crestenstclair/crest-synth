## Context

One post-mix envelope cannot prove per-note behavior, and a demo that merely visits controls cannot prove audible production consequences. Canonical envelope state and its exhaustive witness therefore form one bounded implementation slice.

## Decisions

### ADSR is Patch state and voice-local runtime state

Attack, decay, sustain, and release are bounded canonical values projected into copyable Patch parameters. Each allocated note voice latches and advances its own envelope; note-off releases only matching voices and all-notes-off clears bounded state.

### Both engines conform at their native voice seams

Braids owns sixteen independent oscillator/envelope slots per Patch. SoundFont retains one synthesizer per Patch and exposes engine-native per-note envelope control; no synthesizer-per-voice or post-stem approximation is accepted.

### The headless scene is an exact causal proof

Coverage is derived from installed schemas, actions flow through `AppState::apply`, state/text/scalars are compared, audio consequences are measured per control and engine, baseline restoration is exact, and two runs must serialize identically. Controlled mutants must falsify their named seam.

## Verification

Unit and integration tests cover envelope extremes, overlapping releases, both engines, exact coverage, mutation sensitivity, deterministic restoration, and audible parameter consequences.
