# Phase 7 WAV fixture provenance

The bounded decoder fixtures in `src/adapter/wav_sample_decoder.rs` are generated in memory during
tests. Valid mono/stereo PCM16, PCM24, PCM32, and IEEE float32 files are emitted by the repository's
pinned `hound 3.5.1` adapter dependency, so the repository does not carry large binary audio blobs.

Hostile headers are repository-authored byte fixtures. They cover RF64/RIFX/non-WAV signatures,
unsupported compression/depth/channel/rate, non-finite float PCM, malformed chunks, declared source
overflow, declared duration overflow, and decoded-scalar overflow. Their data chunks are deliberately
header-only where admission can be refused from metadata, keeping the complete fixture set below a
kilobyte while exercising the same checked preflight used by the production adapter.
