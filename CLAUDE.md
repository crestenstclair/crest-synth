# Working in crest-synth

The CUE files in spec/ are the source of truth. Generate implementation through
crest-spec; do not hand-write Rust outside a governed crest-spec candidate
attempt.

Preserve these boundaries:

- one-way input -> AppEvent -> AppState.apply -> projections/effects;
- SoundFont-only synthesis using ./sf2/HiDef.sf2;
- automatic test input from Corridors of Time, with no sequencer or transport;
- one global reverb and one global delay, with no other effects;
- one plain text parameter view with W/S/A/D and K-modified editing;
- no allocation, locks, blocking, I/O, logging, or destruction in the audio
  callback;
- ports and adapters at SoundFont, MIDI input, audio output, view, and
  control/audio boundaries.
