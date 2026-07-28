# Change Summary

## Outcome

- **Problem:** PATCH lists SoundFont and Braids but cannot select them; the demos do not prove runtime replacement.
- **Result:** Edit+Left/Right selects an adjacent engine through semantic events, reducer state, off-callback preparation, graph handoff, and acknowledged target audio.

## Change Outline

- **Adds:** Canonical request identity, lifecycle status, typed failures, and descriptor-default candidate construction without fallback.
- **Adds:** A capacity-one preparation worker with deterministic and threaded adapters; prepared ownership never enters `AppState`.
- **Changes:** The engine row becomes PATCH's only editable control; ADSR and capability parameters remain read-only.
- **Headless:** `make demo` covers both directions, busy/failure/stale/mismatch cases, isolation, callback safety, and identical two-run evidence.
- **Live:** `make demo-live` covers SoundFont → Braids → default SoundFont through the threaded worker, acknowledged revisions, and targeted finite physical audio.

## System Impact

- **Capabilities:** Adds `asynchronous-engine-selection` and modifies the PATCH, capability, rack, control, and demo contracts.
- **Architecture:** Input → `AppState::apply` → `AppLoop` → worker → structural coordinator → renderer, with commit before graph publication.
- **Data:** Versioned events, state, projections, logs, and demo schemas gain lifecycle and revision fields.

## Delivery

- **Implementation:** Land types/reducer, projections, worker/handoff, orchestration/composition, then both demo scenes.
- **Validation:** Run the named workflow, regressions, two headless runs, physical live demo, real-time gates, evaluated CUE, and strict OpenSpec checks.

## Risks and Decisions

- **Decisions:** One structural request at a time; commit before publication; returning uses descriptor defaults; failure never substitutes an engine.
- **Proof split:** Live owns threaded successes; headless owns controlled negatives and repeatability.
- **Risks:** Replacement resets voices/tails, timing varies, and publication can stall; block-boundary swaps, state-based waits, staged retry, and callback counters bound them.
