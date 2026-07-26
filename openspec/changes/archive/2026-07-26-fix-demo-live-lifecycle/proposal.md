## Why

The physical `make demo-live` scene emits a complete report and then leaves the window, audio stream, and Make process resident indefinitely; an accepted key event before completion can also overwrite the checkpoint generation in the latest-snapshot transport and abort the run. The live demo needs a bounded, unattended lifecycle whose proof cannot be corrupted by interactive input.

## What Changes

- **BREAKING**: make successful `make demo-live` completion close the live window, stop the physical stream through normal ownership, and return exit code zero immediately after the four final report records are emitted.
- Make the active autonomous live-demo window observational: keyboard input does not dispatch semantic edits while the scene is running, while the native window close action remains available for early cancellation and normal interactive mode remains unchanged.
- Preserve exact generation-correlated checkpoints, semantic per-Patch all-notes-off cleanup, the complete typed event log, and the existing physical production reducer/render path.
- Add deterministic regression proof that injected window input cannot advance state or invalidate a pending checkpoint and that successful completion requests window shutdown instead of entering an unbounded post-completion loop.
- Reconcile the master design, evaluated CUE architecture, and executable OpenSpec contract with the bounded lifecycle.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `live-observable-demo`: Replace persistent post-completion window ownership with successful automatic shutdown and isolate the autonomous scene from semantic window input until it completes or is cancelled.

## Impact

This change affects the live-demo declarations in `DESIGN.md`, Shell/Testing/goal CUE resources, the `StandaloneApplication` live callbacks, and deterministic standalone/live-demo tests. It changes only `--demo-live`; normal interactive input, headless `make demo`, engine preparation, audio transports, and hard real-time callback behavior remain unchanged, and no dependency is added.
