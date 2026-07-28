## 1. Architecture declarations

- [x] 1.1 Update the Shell CUE device, window-control, standalone injection, CPAL runtime-status, and production-composition contracts without changing product scope.
- [x] 1.2 Update the RealTime CUE observation, renderer chunking/routing-status rules, stable acceptance validation inventory, and exact assertion-bearing validation selectors.
- [x] 1.3 Evaluate the CUE project and strictly validate the new OpenSpec change before implementation proceeds.

## 2. Negotiated audio output

- [x] 2.1 Add validated canonical device configuration, PCM format, stereo mapping, negotiated-output, render-callback, runtime-error, and status-callback types to the Shell audio-output port.
- [x] 2.2 Implement a fixed-size atomic first-runtime-failure channel whose callback writer is nonblocking and whose control reader consumes the exact typed failure.
- [x] 2.3 Refactor the CPAL adapter to retain one negotiated device/configuration, prefer the supported 48 kHz validation point, start only after preparation, map runtime error kinds without formatting, and preserve bounded sample/channel adaptation.
- [x] 2.4 Refactor `AudioRenderer::render` to process exact-capacity and oversized stereo buffers completely in prepared-capacity chunks without silent truncation.

## 3. Explicit production composition

- [x] 3.1 Add typed provider/preparer composition validation for matching, missing, duplicate, unknown, and identity-mismatched registrations.
- [x] 3.2 Replace the standalone constructor with explicit provider, preparer, structural-boundary, observation-boundary, source, window, output, and configuration injection and remove concrete adapter construction/imports from the application service.
- [x] 3.3 Negotiate physical output before `prepare_startup`, pass the exact sample rate and capacity into every graph component, and start the retained stream and MIDI only after compatible preparation succeeds.
- [x] 3.4 Poll runtime device status on the control tick, end the unhealthy window lifetime, and return the exact typed `ApplicationError` in normal and live modes.
- [x] 3.5 Construct the production providers, preparers, lock-free structural boundary, and atomic audio observation explicitly in `src/bin/crest_synth.rs`.

## 4. Observable routing failure

- [x] 4.1 Extend the fixed-size audio observation snapshot and atomic adapter with a saturating routing-failure count and most recent unknown Patch identity.
- [x] 4.2 Preserve both missing-parameter-projection and rack-level unknown-Patch detection in `AudioRenderer` while proving no fallback, active-note mutation, or untargeted dispatch.

## 5. Production-path proof

- [x] 5.1 Add a named production-runtime integration target covering injected matching and invalid registrations plus a replaceable structural and observation boundary.
- [x] 5.2 Add production-path device witnesses for a supported non-default sample rate, exact graph capacity, a fully rendered oversized callback, and unsupported configuration rejection before stream/MIDI start.
- [x] 5.3 Add a controlled post-start device-failure witness that observes the exact typed control/application result.
- [x] 5.4 Extend the renderer witness to assert both unknown-Patch routing isolation and exact fixed-size observable failure.
- [x] 5.5 Add exact named tests and post-assertion markers for `audio_renderer_realtime_contract`, `prepared_graph_handoff`, and `audio_observation_realtime_contract`, plus controlled zero-selection acceptance evidence.

## 6. Verification

- [x] 6.1 Run Rust formatting and Clippy with warnings denied.
- [x] 6.2 Run every new exact validation selector and the named production-runtime integration target, requiring nonzero executed counts and markers.
- [x] 6.3 Run all Cargo targets and the existing capability, rack, Braids, envelope, demo, schema, eframe, mutation, and live acceptance targets.
- [x] 6.4 Evaluate CUE, run strict OpenSpec validation for all changes/specifications, and run deterministic OpenSpec acceptance with selector-specific execution provenance.
