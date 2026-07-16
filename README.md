# crest-synth

crest-synth is a standalone SoundFont synthesizer written in Rust. It expects:

- ./sf2/HiDef.sf2
- ./midi/Corridors of Time - Chrono Trigger.mid

On startup it reads the MIDI fixture, creates one Patch for each instrument,
configures each Patch with the matching SoundFont bank/program or percussion
instrument, assigns Patches to MIDI channels round-robin, and starts playback.
The file is test input; crest-synth has no sequencer or transport.

The application has one keyboard-controlled screen: a scrollable text listing
of every Patch parameter followed by the global parameters. Patch sections are
separated by a horizontal line.

- W / S: previous or next parameter
- A / D: previous or next Patch section
- hold K and press a direction: adjust the selected value

The current signal path is:

    SoundFont Patch
      -> Patch gain and pan
      -> global reverb send
      -> global delay send
      -> master gain
      -> stereo output

There is one synthesis engine type, one shared reverb, and one shared delay.

## Architecture

crest-synth uses ports and adapters around a one-way control loop and a hard
real-time audio callback:

    keyboard / MIDI fixture
      -> AppEvent
      -> AppState.apply
      -> committed AppState
      -> serialized state + text projection + ParameterSnapshot + AudioCommand
      -> lock-free AudioBoundary
      -> SoundFont engine + global mixer
      -> audio device

The audio callback uses preallocated bounded storage and never allocates,
locks, blocks, performs I/O, logs, or destroys owned state.

The CUE declaration under spec/ is the source of truth. Generated Rust
implementation is reconciled through crest-spec.
