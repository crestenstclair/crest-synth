# crest-synth design

## Control side

AppState owns installed Patches, global parameters, selection, and generation.
All inputs are translated to AppEvent; AppState.apply is the only mutation
operation. After an event is accepted, AppLoop commits state and then derives:

- canonical serialized state;
- the immutable text projection;
- the fixed-capacity real-time parameter snapshot;
- any discrete Patch-targeted MIDI command.

Views and input adapters never receive mutable state.

## Audio side

AudioBoundary is the only control/audio seam. Its adapter uses:

- an SPSC ring for bounded MIDI commands;
- latest-wins triple buffering for parameter snapshots;
- deferred destruction for retired audio state.

AudioRenderer drains ready commands, reads one complete latest snapshot,
renders the SoundFont Patches, mixes per-Patch gain and pan, processes the two
global effect sends, and writes stereo samples into the device-owned buffer.

The callback performs no allocation, locking, blocking, I/O, logging, collection
growth, string formatting, or owned-state destruction.

## Sound source

SoundFontEngine is the synthesis port. The application constructs exactly one
HiDefSoundFontEngine, which owns one synthesizer instance and loads
./sf2/HiDef.sf2 once on the control thread. Each Patch contains a SoundFont
instrument identity and an assigned MIDI channel that configure that shared
engine. There is no per-Patch engine, engine selector, or fallback.

## MIDI test input

MidiEventSource is the input port. CorridorsMidiEventSource reads
./midi/Corridors of Time - Chrono Trigger.mid, discovers instrument identities,
and assigns them to channels in first-sounding order using index modulo 16.
AutomaticMidiTest configures one Patch per identity and begins emitting due
events at startup.

File timing is private adapter behavior. No sequencer, transport, recording,
timeline, clip, pattern, or song-editing model is exposed.

## View

The view renders one stock monospace text label in one vertical scroll area.
Every Patch lists its instrument and editable values, Patch sections use the
same horizontal separator, and the global values appear last. Raw key handling
emits semantic navigation or adjustment events and owns only the K modifier.
