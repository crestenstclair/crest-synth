## MODIFIED Requirements

### Requirement: Hard real-time callback contract
The audio callback, including every SoundFont operation, Braids C++ FFI call, per-voice envelope transition, scalar application, 96-to-48 kHz conversion, and mixed-engine dispatch/render operation, SHALL use only preallocated fixed-capacity storage and bounded work and SHALL perform no allocation, deallocation, collection growth, locking, blocking, I/O, logging, formatting, panic, exception, unwind, or owned-state destruction.

#### Scenario: Render under control and structural traffic
- **WHEN** envelope/engine parameter generations, engine-managed SoundFont traffic, sixteen-voice traffic for every active Braids Patch, and one prepared graph replacement are published while audio is rendering
- **THEN** the callback renders from bounded preallocated storage with zero callback allocations, zero callback/native destructions, finite output, bounded timing, and no locks, blocking, I/O, logging, formatting, panic, exception, or unwind

### Requirement: Finite continuous render result
The real-time renderer SHALL route accepted audio commands and only the compatible matching Patch scalar/envelope projection through the capability-neutral prepared rack to exact target SoundFont or Braids Patches, render distinct bounded Patch stems, and combine them through the active graph's global mix into a finite bounded stereo output.

#### Scenario: Automatic MIDI renders through the callback
- **WHEN** accepted fixture commands target alternating SoundFont and Braids Patches during rendering
- **THEN** each command and parameter value reaches its exact prepared instrument, both engine types produce distinct nonzero Patch stems, and the output has a finite nonzero bounded peak

### Requirement: Bounded active-note observation
The renderer SHALL maintain a prepared fixed-capacity note-lifecycle observation updated only from MIDI commands it dispatches, and Patch-targeted or global all-notes-off SHALL clear the corresponding active-note state with bounded work. The observation SHALL NOT control or substitute SoundFont or Braids voice/envelope state.

#### Scenario: Note lifecycle is dispatched
- **WHEN** note-on, note-off, or all-notes-off commands pass through the renderer to either engine
- **THEN** the next audio observation reports the corresponding bounded active-note count while each engine retains ownership of its real voice and envelope lifecycle

## ADDED Requirements

### Requirement: Mixed-engine callback timing admission
At 48 kHz with 256-frame blocks, the production mixed-engine acceptance SHALL render a declared worst-case bounded scene repeatedly, record block durations without callback logging, and require p99 render time below half the callback period. The measurement SHALL include the declared engine-managed SoundFont load, sixteen active voices in every Braids Patch of the admitted test graph, envelope work, engine scalars, rack stems, global effects, and final mixing.

#### Scenario: Worst-case timing fixture runs
- **WHEN** the measured mixed-engine render loop completes on the reference development host
- **THEN** its declared sample count, maximum work bounds, and p99 duration are reported and p99 remains below 2.666 milliseconds
