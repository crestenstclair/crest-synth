## ADDED Requirements

### Requirement: Bounded callback-to-control audio observations
The audio callback SHALL publish fixed-size numeric observations through a dedicated coherent latest-value transport from the callback to the control side. Publication SHALL be bounded, lock-free, nonblocking, allocation-free, and free of logging, formatting, I/O, panic, and owned-state destruction; a slow reader SHALL NOT backpressure rendering.

#### Scenario: Callback publishes a completed block
- **WHEN** a block is rendered from a complete parameter generation
- **THEN** the callback publishes one coherent observation tagged with that generation and a monotonically increasing sequence without changing the rendered output

#### Scenario: Control reader is slower than audio
- **WHEN** several callback observations are produced before the control side reads
- **THEN** the reader receives one complete latest observation, may skip stale intermediate observations, and never combines fields from different blocks

### Requirement: Mixer-stage measurement ownership
Measurements of reverb input, delay input, wet output, final stereo peak/RMS, clipping, and non-finite samples SHALL be calculated from the actual mixer-owned callback buffers as a fixed-size value and SHALL NOT expose those private buffers or feed observation data back into mixing decisions.

#### Scenario: Mixer processes a normal block
- **WHEN** dry stems, sends, shared effects, and master gain are processed
- **THEN** the returned measurements describe their declared signal stages and the output samples are identical to rendering with observation disabled

#### Scenario: Invalid output sample is encountered
- **WHEN** a mixed sample is clipped or non-finite
- **THEN** the corresponding bounded counter is updated without callback logging, formatting, panic, or I/O

### Requirement: Bounded active-note observation
The renderer SHALL maintain a prepared fixed-capacity note-lifecycle observation updated only from MIDI commands it dispatches, and Patch-targeted or global all-notes-off SHALL clear the corresponding active-note state with bounded work. The observation SHALL NOT control or substitute engine voice state.

#### Scenario: Note lifecycle is dispatched
- **WHEN** note-on, note-off, or all-notes-off commands pass through the renderer
- **THEN** the next audio observation reports the corresponding bounded active-note count while SoundFont behavior remains owned by the engine
