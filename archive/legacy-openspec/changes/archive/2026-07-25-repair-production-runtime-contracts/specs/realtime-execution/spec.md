## ADDED Requirements

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
