## 1. Fixed prepared contracts

- [x] 1.1 Extend real-time Patch parameters with bounded ADSR and descriptor-ordered `[f32; 16]` Scalars plus graph/layout compatibility.
- [x] 1.2 Pass only matching Patch parameters through prepared dispatch/render and preserve exact unknown-Patch routing status.

## 2. Engine and ownership conformance

- [x] 2.1 Prepare independent sixteen-voice Braids banks per Patch with off-callback native ownership and explicit rate/config failures.
- [x] 2.2 Retain one engine-managed SoundFont synthesizer per Patch and one shared parsed bank with native per-note envelope isolation.

## 3. Hard-real-time execution

- [x] 3.1 Preserve separate bounded discrete, scalar, structural, retirement, and observation transports.
- [x] 3.2 Prove finite mixed rendering with zero callback allocation, destruction, lock, block, I/O, logging, or panic.

## 4. Verification

- [x] 4.1 Expand prepared-rack and production-runtime targets for routing, scalar isolation, graph handoff, and no-fallback behavior.
- [x] 4.2 Run the release-profile worst-case mixed timing witness and require p99 below half the 48 kHz/256-frame callback period.
