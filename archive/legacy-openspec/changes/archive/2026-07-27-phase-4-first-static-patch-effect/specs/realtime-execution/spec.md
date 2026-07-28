## ADDED Requirements

### Requirement: Effect scalar storage, processing, observation, and retirement remain hard-real-time safe
`ParameterSnapshot` SHALL contain a separate fixed Patch-aligned post-effect section with zero or one active slot per Patch, stable slot identity, at most eight finite descriptor-ordered scalars, and no String, Vec, asset, config, reference, processor, or destructor. The renderer SHALL use an effect scalar entry only when its graph revision, Patch order, slot id, and scalar layout exactly match the active complete graph.

#### Scenario: Latest effect edit reaches audio
- **WHEN** control publishes a newer Amount or Depth generation for the active graph
- **THEN** the renderer consumes the newest complete compatible effect section without changing instrument scalar layout or observing a partial assignment

#### Scenario: Effect snapshot targets another layout
- **WHEN** the latest snapshot has a different revision, Patch order, slot id, or scalar count
- **THEN** the renderer retains the prior compatible complete values and records the mismatch without combining layouts or selecting defaults

**Contract facet — processing and observation obey the callback contract.**
For every bounded block, `AudioRenderer` SHALL render each instrument stem, measure the exact configured Patch immediately before and after `PreparedPostEffectRack` processing, then pass the processed stems to `MixEngine`. `PatchEffectObservation` SHALL contain only fixed-size finite numeric Patch identity, pre-RMS, post-RMS, difference-RMS, and side-RMS data and SHALL never control processing. Effect processing, measurement, graph swap, and retirement SHALL preserve the full hard-real-time prohibition on allocation, deallocation, growth, locking, blocking, I/O, logging, formatting, panic, exception, unwind, and destruction.

#### Scenario: Configured Chorus produces a block
- **WHEN** the first Patch supplies finite nonzero input with nonzero Chorus settings
- **THEN** one coherent observation reports the exact Patch, finite pre/post values, nonzero causal difference and stereo side energy before mixer gain/pan/sends are applied

#### Scenario: Other Patches render in the same block
- **WHEN** configured and unconfigured Patches sound simultaneously
- **THEN** effect processing changes only the matching stem, mixer routing consumes all post-effect stems, and no observation field is derived from an unrelated Patch or final master output

#### Scenario: Complete graph retires
- **WHEN** a graph containing native Chorus ownership is replaced
- **THEN** no native or Rust effect owner is destroyed in the callback and all returned effect ownership is collected and destroyed on worker/control ownership
