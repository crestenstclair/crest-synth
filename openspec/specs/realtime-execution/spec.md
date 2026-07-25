# Real-Time Execution

## Purpose

Define the hard real-time rendering contract, lock-free control-to-audio handoff, control-side retirement, and bounded finite output guarantees.

## Requirements

### Requirement: Hard real-time callback contract
The audio callback SHALL use only preallocated fixed-capacity storage and bounded work and SHALL perform no allocation, deallocation, collection growth, locking, blocking, I/O, logging, formatting, panic, unwind, or owned-state destruction.

#### Scenario: Render under control and structural traffic
- **WHEN** parameter generations, MIDI commands, and one prepared graph replacement are published while audio is rendering
- **THEN** the callback renders from bounded preallocated storage with zero callback allocations and zero callback-owned destructions and without locks, blocking, I/O, logging, formatting, panic, or unwind

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
The real-time renderer SHALL route accepted audio commands through the capability-neutral prepared rack to their exact target Patches, render distinct bounded Patch stems, and combine them through the active graph's global mix into a finite bounded stereo output.

#### Scenario: Automatic MIDI renders through the callback
- **WHEN** accepted fixture commands target multiple configured Patches during rendering
- **THEN** each command reaches its exact prepared instrument, distinct Patch stems contribute to the final buffer, and the output has a finite nonzero bounded peak

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
