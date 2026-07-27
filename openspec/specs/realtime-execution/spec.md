# Real-Time Execution

## Purpose

Define the hard real-time rendering contract, lock-free control-to-audio handoff, control-side retirement, and bounded finite output guarantees.
## Requirements
### Requirement: Hard real-time callback contract
The audio callback, including every SoundFont operation, Braids C++ FFI call, per-voice envelope transition, scalar application, 96-to-48 kHz conversion, and mixed-engine dispatch/render operation, SHALL use only preallocated fixed-capacity storage and bounded work and SHALL perform no allocation, deallocation, collection growth, locking, blocking, I/O, logging, formatting, panic, exception, unwind, or owned-state destruction.

#### Scenario: Render under control and structural traffic
- **WHEN** envelope/engine parameter generations, engine-managed SoundFont traffic, sixteen-voice traffic for every active Braids Patch, and one prepared graph replacement are published while audio is rendering
- **THEN** the callback renders from bounded preallocated storage with zero callback allocations, zero callback/native destructions, finite output, bounded timing, and no locks, blocking, I/O, logging, formatting, panic, exception, or unwind

### Requirement: Separate structural graph transfer
Complete prepared graph ownership SHALL cross from control or worker ownership to audio ownership through a preallocated bounded nonblocking structural transport, and replaced graph ownership SHALL return through a different preallocated bounded nonblocking transport. Neither direction SHALL reuse discrete command storage, latest scalar storage, or callback-observation storage.

#### Scenario: Discrete, scalar, structural, and observation traffic overlap
- **WHEN** MIDI commands, a latest parameter snapshot, one prepared graph, and callback measurements are all exchanged
- **THEN** each value uses its declared transport and no traffic type overwrites, blocks, aliases, or partially publishes another

### Requirement: Compatible graph and parameter generations
The renderer SHALL associate each active graph with a nonzero graph revision and SHALL render only from a complete parameter snapshot whose revision and ordered Patch identities match that graph. A graph SHALL carry its own compatible initial snapshot, and an incompatible latest scalar publication SHALL NOT replace the active compatible values.

#### Scenario: Structural and scalar publication order differs
- **WHEN** a candidate graph and its separately published scalar snapshot become visible in either order
- **THEN** every rendered block uses one complete graph-compatible snapshot and never combines the Patch layout of one graph with values from another

### Requirement: Lock-free command and parameter transfer
The audio side SHALL consume every ready bounded audio command and the newest complete graph-compatible parameter generation through distinct nonblocking boundaries, without observing a partially published snapshot or carrying structural graph ownership in either transport.

#### Scenario: Commands and snapshots are published concurrently
- **WHEN** the control side publishes several commands and replaces the current parameter snapshot
- **THEN** the audio side receives the ready commands in bounded order and observes either the previous or newest complete compatible snapshot, never a partial generation or a snapshot for another active graph

### Requirement: Control-side retirement
Every complete graph replaced across the real-time boundary SHALL return through bounded ownership transfer and SHALL be destroyed on the control or worker side rather than inside the audio callback. A full return transport SHALL cause the callback to retain the graph in fixed storage and retry without accepting another replacement or dropping owned state.

#### Scenario: Published graph is replaced
- **WHEN** the audio side activates a newer prepared graph at a block boundary
- **THEN** ownership of the prior graph returns for off-callback destruction or remains intact in the callback's bounded retirement slot until return succeeds

#### Scenario: Retirement return is backpressured
- **WHEN** the graph return transport is full
- **THEN** rendering continues on the active graph, retirement pressure is observable, callback destruction remains zero, and no further graph is activated until the retained graph has returned

### Requirement: Finite continuous render result
The real-time renderer SHALL route accepted audio commands and only the compatible matching Patch scalar/envelope projection through the capability-neutral prepared rack to exact target SoundFont or Braids Patches, render distinct bounded Patch stems, and combine them through the active graph's global mix into a finite bounded stereo output.

#### Scenario: Automatic MIDI renders through the callback
- **WHEN** accepted fixture commands target alternating SoundFont and Braids Patches during rendering
- **THEN** each command and parameter value reaches its exact prepared instrument, both engine types produce distinct nonzero Patch stems, and the output has a finite nonzero bounded peak

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
The renderer SHALL maintain a prepared fixed-capacity note-lifecycle observation updated only from MIDI commands it dispatches, and Patch-targeted or global all-notes-off SHALL clear the corresponding active-note state with bounded work. The observation SHALL NOT control or substitute SoundFont or Braids voice/envelope state.

#### Scenario: Note lifecycle is dispatched
- **WHEN** note-on, note-off, or all-notes-off commands pass through the renderer to either engine
- **THEN** the next audio observation reports the corresponding bounded active-note count while each engine retains ownership of its real voice and envelope lifecycle

### Requirement: Device negotiation precedes graph preparation
The physical-audio path SHALL select one output device and validate its sample rate, PCM sample format, channel count, stereo channel mapping, and bounded render capacity before preparing any engine, effect, scratch storage, or complete graph. The graph SHALL be prepared from that exact accepted configuration, and the retained negotiated device owner SHALL start its stream only after the compatible renderer exists. Unsupported configurations SHALL fail before rendering or MIDI start.

#### Scenario: Supported non-default rate is negotiated
- **WHEN** a conforming output and installed engine set negotiate a supported sample rate different from application defaults
- **THEN** every preparer and the complete graph receive that negotiated rate and the stream renders only after preparation succeeds

#### Scenario: Exact capacity is negotiated
- **WHEN** the negotiated device configuration declares a bounded render capacity
- **THEN** instrument, effect, stem, mixer, and graph scratch are prepared for exactly that capacity before stream start

#### Scenario: Configuration is unsupported
- **WHEN** the selected sample rate, sample format, channel mapping, or capacity is invalid or unsupported by a required component
- **THEN** startup returns a typed failure and neither the render callback nor MIDI source starts

### Requirement: Every physical callback buffer is fully bounded
Every physical output path, including native stereo, SHALL service the complete device buffer as consecutive render blocks no larger than the active graph's prepared frame capacity. It SHALL NOT truncate, leave an oversized tail silent, allocate replacement scratch, or perform unbounded work in any individual render block.

#### Scenario: Callback equals prepared capacity
- **WHEN** a device callback contains exactly the graph's prepared frame capacity
- **THEN** one bounded block renders the complete buffer

#### Scenario: Callback exceeds prepared capacity
- **WHEN** a device callback contains more frames than the graph's prepared capacity
- **THEN** ordered bounded chunks render every complete frame and no tail remains silent because of capacity truncation

### Requirement: Runtime device failures are typed control outcomes
After a stream starts, every device error callback SHALL map its error to a fixed-size typed status and publish the first failure through a bounded nonblocking callback-to-control path without allocation, locking, blocking, I/O, logging, formatting, panic, or UI work. The control loop SHALL consume that status, stop presenting the runtime as healthy, and return the exact typed application failure. No recovery or silent substitute SHALL be implied.

#### Scenario: Device fails after successful start
- **WHEN** a controlled output starts successfully and later reports device unavailability
- **THEN** the next control tick observes `DeviceNotAvailable`, terminates the unhealthy window lifetime, and returns the corresponding typed application error

#### Scenario: Several errors race before control polls
- **WHEN** more than one runtime device error arrives before the control loop reads status
- **THEN** the first failure remains intact and callback publication remains bounded and nonblocking

### Requirement: Test-bearing validation evidence is selector-specific
Every declared test-bearing validation SHALL execute at least one test under its own exact selector and SHALL provide selector-matched structured evidence of the nonzero executed count and required assertions. A successful broad suite or another selector SHALL NOT satisfy, replace, or lend execution counts to that validation.

#### Scenario: Exact renderer validation executes
- **WHEN** each declared renderer, graph-handoff, or audio-observation selector is evaluated
- **THEN** its own invocation executes its named witness and satisfies its post-assertion marker

#### Scenario: Targeted selector matches zero tests
- **WHEN** a broad suite passes but one independently declared targeted selector executes zero tests
- **THEN** acceptance fails that targeted validation despite its process exit code being zero

### Requirement: Mixed-engine callback timing admission
At 48 kHz with 256-frame blocks, the production mixed-engine acceptance SHALL render a declared worst-case bounded scene repeatedly, record block durations without callback logging, and require p99 render time below half the callback period. The measurement SHALL include the declared engine-managed SoundFont load, sixteen active voices in every Braids Patch of the admitted test graph, envelope work, engine scalars, rack stems, global effects, and final mixing.

#### Scenario: Worst-case timing fixture runs
- **WHEN** the measured mixed-engine render loop completes on the reference development host
- **THEN** its declared sample count, maximum work bounds, and p99 duration are reported and p99 remains below 2.666 milliseconds

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
