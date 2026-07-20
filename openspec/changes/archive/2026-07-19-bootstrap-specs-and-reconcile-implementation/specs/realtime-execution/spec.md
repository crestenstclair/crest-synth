## ADDED Requirements

### Requirement: Hard real-time callback contract
The audio callback SHALL use only preallocated fixed-capacity storage and SHALL perform no allocation, locking, blocking, I/O, logging, or destruction.

#### Scenario: Render under control traffic
- **WHEN** parameter generations and MIDI commands are published while audio is rendering
- **THEN** the callback renders from bounded preallocated storage with zero callback allocations and without locks, blocking, I/O, logging, or destruction

### Requirement: Lock-free command and parameter transfer
The audio side SHALL consume every ready bounded audio command and the newest complete parameter generation through a nonblocking boundary, without observing a partially published snapshot.

#### Scenario: Commands and snapshots are published concurrently
- **WHEN** the control side publishes several commands and replaces the current parameter snapshot
- **THEN** the audio side receives the ready commands in bounded order and observes either the previous or newest complete snapshot, never a partial generation

### Requirement: Control-side retirement
Engine-owned or snapshot-owned data replaced across the real-time boundary SHALL be retired and destroyed on the control side rather than inside the audio callback.

#### Scenario: Published state is replaced
- **WHEN** the audio side finishes using data superseded by a newer publication
- **THEN** ownership returns through the boundary and destruction occurs outside the audio callback

### Requirement: Finite continuous render result
The real-time renderer SHALL combine configured SoundFont Patch stems and the global mix into a finite bounded stereo output while delivering accepted audio commands to their exact target Patch.

#### Scenario: Automatic MIDI renders through the callback
- **WHEN** accepted fixture commands target multiple configured Patches during rendering
- **THEN** each command reaches its exact Patch, distinct Patch stems contribute to the final buffer, and the output has a finite nonzero bounded peak

