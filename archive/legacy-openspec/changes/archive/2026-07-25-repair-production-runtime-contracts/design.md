## Context

The production binary already selects concrete adapters, but `StandaloneApplication` still creates or reaches concrete capability, structural-handoff, and observation adapters internally. Physical output is currently a one-step `open` operation: the CPAL adapter learns its sample rate and buffer shape only after the application has prepared a graph from fixed defaults. The renderer truncates buffers beyond that capacity, CPAL discards asynchronous stream errors, and the renderer reduces rack dispatch to a Boolean. The CUE architecture already declares the intended ports and fixed-size observation path; this change makes the production wiring conform to them.

The working tree also contains the completed Braids increment. The repair preserves that installed-engine set and its exact 48 kHz policy while remaining testable with a conforming single-capability fixture at another supported sample rate.

## Goals / Non-Goals

**Goals:**

- Make all provider, preparer, structural-handoff, and observation implementations explicit constructor inputs owned by the composition root.
- Validate provider/preparer registration atomically before any Patch or graph can be published.
- Negotiate and validate an exact physical device configuration before preparing the graph and starting the stream.
- Fully render every physical callback buffer with bounded chunks no larger than the graph capacity.
- Deliver post-start device errors and unknown-Patch routing failures through fixed-size, nonblocking status values.
- Make every declared test selector execute its own witness and make zero-selection evidence fail acceptance.

**Non-Goals:**

- Adding or removing an instrument engine, exposing engine selection, or changing Braids' exact-rate policy.
- Adding the Patch page, effects, modulation, arbitrary graph editing, recovery/reopen policy, or a new persistence model.
- Treating a broad test suite as evidence for a narrower validation selector.

## Decisions

### Composition validates injected registrations

`StandaloneApplication::new` accepts the provider collection, preparer collection, structural graph boundary, and audio observation boundary. It builds the immutable registry from provider descriptors and validates an exact one-to-one identity match with preparers before storing the composed values. The constructor returns typed duplicate, missing, unknown, or mismatched registration errors; it does not create a concrete adapter or fallback.

The standalone binary explicitly constructs the installed HiDef SoundFont and Braids providers and preparers plus the lock-free structural and atomic observation adapters. Deterministic tests use the same constructor with replaceable fixtures. This keeps concrete selection in the infrastructure composition root while preserving generic application orchestration.

Alternative considered: keep accepting a prebuilt `CapabilityRegistry`. That would continue bypassing `InstrumentCapabilityProvider` and could not prove provider/registry disagreement through production construction.

### Physical output is a two-stage port

`AudioOutput::negotiate` consumes the output adapter and returns a negotiated output owner. That owner exposes a validated `AudioDeviceConfig` containing sample rate, PCM sample format, channel count, stereo channel mapping, and prepared render capacity. Only after the application builds the complete graph for those exact values does it call `start` with the render and runtime-status callbacks.

The negotiated owner retains the selected device and stream configuration, so `start` cannot silently rediscover a different default device or configuration. CPAL prefers the design's 48 kHz validation point when available; an engine or device incompatibility remains a typed pre-start failure.

Alternative considered: query and then reopen the stateless default adapter. A device/configuration can change between those operations, defeating the exact preparation proof.

### Oversized callbacks are chunked by the renderer

`AudioRenderer::render` clears the caller buffer and processes it in consecutive stereo chunks of at most `PreparedGraph::max_frames`. Each chunk is a bounded render block using the same preallocated graph scratch. This covers native stereo and mapped device paths and makes a large device callback fully rendered rather than silently zeroing its tail.

Alternative considered: reject every callback larger than a negotiated maximum. Several CPAL backends report an unknown default buffer size, so bounded chunking gives a portable, falsifiable policy without unbounded allocation.

### Runtime device failure uses an atomic first-failure latch

The output adapter receives a prebuilt callback that accepts a small `Copy` error enum. CPAL maps its error kind without formatting, logging, allocation, locking, or UI calls. The callback stores the first failure in an atomic latch. The control-side window tick consumes that typed value, records `ApplicationError::AudioDeviceRuntime`, and asks the window loop to close; the application then returns the exact typed failure on control ownership.

This prevents a dead stream from leaving an apparently healthy window running. Recovery and device reopening remain future behavior.

Alternative considered: append an error string directly to the text view. That would bypass canonical projection ownership and perform presentation work at the adapter seam.

### Routing failure extends the existing audio observation

`AudioObservationSnapshot` gains a saturating unknown-Patch routing-failure count and the most recent unknown `PatchId`. `AudioRenderer` updates those fixed-size fields whenever either parameter lookup or rack dispatch reports the target absent, and publishes them with the next coherent block observation. It does not mutate active-note state or another instrument.

Alternative considered: a new queue for each failure. Unknown-Patch status is diagnostic latest-value data; the existing dedicated callback observation has the correct overwrite and non-backpressure semantics.

### Validation selectors are exact and assertion-bearing

The three CUE resource validations target exact named tests in an explicit integration target, run with `--exact --nocapture`, and require a post-assertion marker. The OpenSpec acceptance report must record a nonzero executed count for each invocation. A controlled acceptance check presents a successful broad-suite record alongside a zero-selection targeted record and proves the latter cannot satisfy the declaration.

Every CUE validation also has a stable globally unique `validation.*` identity. Project checks declare an explicit working directory, bounded process limits, and acceptance-supported assertion shapes. The completion inventory names the production-runtime target, all three exact selectors, and the zero-selection guard, so deterministic acceptance executes and records each proof independently.

Alternative considered: point all three declarations at `cargo test --all-targets`. That would erase selector-level provenance and preserve the proof gap.

## Risks / Trade-offs

- [More generic parameters in standalone orchestration] → Keep handle types inferred at the composition root and put validation in one constructor.
- [A CPAL backend reports unknown callback size] → Prepare a declared chunk capacity and fully iterate any larger buffer with no allocation.
- [Multiple asynchronous device errors arrive] → Latch the first typed failure, which is sufficient to stop the unhealthy runtime deterministically; later recovery can add a bounded history if required.
- [Existing tests construct the old one-step output or standalone API] → Migrate fixtures mechanically and add compile-time port contract tests.
- [The active Braids change and this repair touch the same files] → Preserve its engine behavior and verify both its named acceptance targets and this change's production-runtime target.

## Migration Plan

1. Update the CUE port contracts, stable validation inventory, and exact validation declarations.
2. Introduce the canonical negotiated device/status values and migrate CPAL plus test outputs.
3. Replace the standalone constructor and inject every declared port from the binary.
4. Add routing observation fields and bounded renderer chunking.
5. Add the production-runtime integration target and run targeted, broad, CUE, and OpenSpec acceptance checks.

The change is internal and requires no saved-state migration. Reverting the commit restores the prior constructor and output port, but no partial mixture of the old one-step output and new preparation order is supported.
