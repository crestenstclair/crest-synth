# hound 3.5.1 dependency audit

- Purpose: Phase 7 RIFF/WAVE sample decode behind `SampleDecoderPort`.
- Pin: exact crates.io release `3.5.1`.
- License: Apache-2.0.
- Feature surface: the crate declares no optional Cargo features; Crest enables
  no feature flags.
- Types at boundaries: none. `hound` values remain inside the WAV adapter.
- Product admission remains narrower than the decoder: RIFF/WAVE only,
  mono/stereo PCM16/24/32 or IEEE float32 at 8–192 kHz. The adapter performs
  Crest's checked limits and typed rejection before returning domain PCM.
- Sources reviewed 2026-08-20: crates.io package metadata, docs.rs API metadata,
  and the repository/homepage linked by that package metadata.
