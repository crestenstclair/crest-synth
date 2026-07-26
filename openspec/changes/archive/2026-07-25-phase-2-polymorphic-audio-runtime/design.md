## Context

The second engine adds native C++ DSP, fixed-per-Patch voice banks, descriptor Scalars, and a sample-rate adapter to the same callback that already owns SoundFont rendering and graph handoff. The callback contract must remain bounded and capability-neutral.

## Decisions

### Prepared slots receive matching fixed parameters

Control projects descriptor-ordered Scalars and common ADSR into fixed `RtPatchParameters`. Graph revision and layout identity are validated before activation, and each rack slot receives only its Patch projection during targeted dispatch and block rendering.

### Runtime transports remain separated

Discrete MIDI commands, latest scalar snapshots, prepared structural graphs, retired ownership, and lossy observations retain distinct bounded transports. Native construction, parsing, allocation, and destruction remain off the callback.

### Engine voice policies remain capability-owned

Every Braids Patch owns sixteen prepared voices. SoundFont remains engine-managed with one synthesizer per Patch and one shared parsed bank. Unsupported configuration or sample rate fails visibly before processing; neither engine substitutes for the other.

### Admission is measured

The release-profile witness renders the declared worst-case mixed graph at 48 kHz in 256-frame blocks and reports p99 against half the physical callback period while allocator and native lifecycle counters remain unchanged.

## Verification

Prepared-rack, production-runtime, Braids, SoundFont/envelope, callback, structural-handoff, finite-render, no-allocation/destruction, and timing witnesses execute through the production reducer and renderer.
