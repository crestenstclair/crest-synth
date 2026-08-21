# sample-capability-and-browser Specification

## Purpose
TBD - created by archiving change implement-phase-7-detail-choice-assets. Update Purpose after archive.
## Requirements
### Requirement: Sample assets have a bounded admitted format
The first Sample capability SHALL admit only uncompressed RIFF/WAVE assets containing one or two channels of signed 16-, 24-, or 32-bit PCM or 32-bit IEEE floating-point samples at a source rate from 8,000 through 192,000 Hz. A source file MUST be no larger than 256 MiB and MUST describe no more than 300 seconds of audio. RF64, RIFX, compressed WAV, more than two channels, malformed chunks, non-finite floating-point samples, and every non-WAV format SHALL fail with a typed validation or unsupported-format result.

#### Scenario: Admit a stereo fixture
- **WHEN** the catalog resolves a 48 kHz stereo 24-bit PCM RIFF/WAVE asset inside all size and duration limits
- **THEN** the Sample preparation port accepts it for off-callback decoding and preparation

#### Scenario: Reject an unsupported asset
- **WHEN** the user assigns a compressed WAV, an MP3 file renamed with a `.wav` suffix, or an otherwise out-of-contract asset
- **THEN** validation returns the specific typed cause and no Sample graph or active asset changes

### Requirement: Sample references are stable and library-scoped
Canonical session state SHALL store a stable sample asset identifier relative to a configured library root, not decoded PCM, a UI row index, or an unrestricted absolute path. The filesystem adapter SHALL resolve and canonicalize each candidate, reject traversal and links that escape the root, and produce stable semantic identities for parent, folder, file, and cancel rows. Eligible entries SHALL sort folders before files and compare normalized names deterministically.

#### Scenario: Resolve a valid library asset
- **WHEN** a saved stable sample identifier resolves to an eligible file beneath the configured library root
- **THEN** the adapter returns that exact asset and its canonical metadata without exposing the absolute path as domain identity

#### Scenario: Reject library escape
- **WHEN** a relative path, parent traversal, or symbolic link would resolve outside the configured library root
- **THEN** the adapter returns a typed unavailable or validation failure and performs no decode or assignment

### Requirement: Sample playback has one explicit first-slice contract
Each Sample Patch SHALL own exactly one prepared asset and `FixedPerPatch(16)` voices. A note-on SHALL begin at the configured inclusive start frame and read until the exclusive end frame at the ratio `2^((midi_note - root_note) / 12)`, using bounded linear interpolation; `root_note` SHALL range from 0.00 through 127.00 semitones with default 60.00. Mono input SHALL duplicate to stereo and stereo input SHALL retain its two channels. The capability SHALL use the Patch's canonical `VoiceEnvelope`; note-off SHALL enter its release stage. Voice allocation SHALL choose the first inactive voice, then the oldest releasing voice, then the oldest active voice deterministically.

#### Scenario: Play at the root pitch
- **WHEN** MIDI note 60 starts a Sample whose root note is 60.00
- **THEN** the prepared asset advances at its prepared device-rate speed from the configured start frame through the configured playback range

#### Scenario: Exceed polyphony
- **WHEN** a seventeenth overlapping note arrives for one Sample Patch
- **THEN** the engine reuses exactly the deterministic oldest eligible voice without allocation or an engine-global voice pool

### Requirement: Sample looping is forward-only and validated
The first Sample capability SHALL support `OFF` and `FORWARD` loop modes only. Playback start, exclusive playback end, inclusive loop start, exclusive loop end, and crossfade length SHALL be canonical frame landmarks prepared from descriptor values. The invariant `start <= loop_start < loop_end <= end` SHALL hold whenever looping is on. Crossfade SHALL be between zero and the lesser of 200 milliseconds or half the loop length. A held voice in `FORWARD` mode SHALL wrap through the loop with a bounded linear crossfade; after note-off it SHALL continue the loop only until the canonical envelope release reaches silence. Reverse, ping-pong, time stretching, slicing, and multi-zone mapping are out of contract.

#### Scenario: Render a forward loop
- **WHEN** a held note crosses the exclusive loop-end landmark in `FORWARD` mode
- **THEN** rendering wraps to loop start using the prepared crossfade without reading beyond prepared PCM

#### Scenario: Reject invalid landmarks
- **WHEN** a requested loop has an empty or inverted range, lies outside the playback range, or requests an excessive crossfade
- **THEN** validation reports the exact invalid landmark and the active Sample configuration remains unchanged

### Requirement: Sample preparation is bounded and off the callback
The Sample preparation worker SHALL resolve and validate the reference, parse and decode PCM, replace non-finite input only by rejecting the asset, resample to the negotiated device rate, derive at most 2,048 waveform min/max pairs, validate and convert landmarks, allocate sixteen voice states, and warm the prepared engine before publication. A prepared asset SHALL contain at most 28,800,000 scalar `f32` samples, and the deduplicated Sample PCM owned by one complete `PreparedGraph` SHALL not exceed 512 MiB. The callback SHALL receive only prepared numeric state through the structural ownership-transfer boundary and SHALL perform bounded, preallocated work with no allocation, lock, blocking, I/O, logging, panic, or destruction.

#### Scenario: Prepare within limits
- **WHEN** a valid candidate and the complete candidate graph fit all per-asset and aggregate limits
- **THEN** the worker publishes a warmed prepared graph correlated to the request and the callback swaps it only at a block boundary

#### Scenario: Exceed the graph PCM budget
- **WHEN** adding a valid decoded asset would exceed the 512 MiB deduplicated PCM budget for the complete graph
- **THEN** preparation returns a typed capacity failure, retains the prior graph, and never partially publishes the candidate asset

### Requirement: Sample assignment is correlated and never falls back
Confirming a Sample Browser file SHALL close the browser to the exact originating asset control and create a correlated structural assignment request. Canonical state SHALL distinguish requested from active asset and expose `Loading`, `Validating`, `Preparing`, `Activating`, `Ready`, `Unavailable`, `Invalid`, `Cancelled`, and typed failure states. Only activation of the compatible prepared graph SHALL commit the active asset reference. A failure or cancellation SHALL retain the prior asset and graph and MUST NOT substitute a fixture, silence engine, SoundFont, or other fallback.

#### Scenario: Commit a valid sample
- **WHEN** a selected asset validates, prepares, and activates for the current request and source revision
- **THEN** the active Sample asset changes exactly once at activation and the origin row reports `Ready`

#### Scenario: A newer request supersedes preparation
- **WHEN** an older asset preparation completes after a newer request or cancellation for the same Sample Patch
- **THEN** correlation rejects the stale result, the active graph does not regress, and the current state remains explicit

#### Scenario: Cancel the browser
- **WHEN** the user exits without confirming a file
- **THEN** no assignment request is created, active state remains unchanged, preview stops, and the origin reports `Cancelled — unchanged`

### Requirement: The Sample Browser is a controller-native trapped modal
The Sample Browser SHALL remain inside PATCH, trap focus in reducer-owned `Modal` mode, and project parent, folder, eligible file, and cancel rows from canonical catalog state. Up and Down SHALL move non-wrapping among enabled semantic row identities; Edit SHALL enter a folder, move to the parent, assign an eligible file, or activate cancel according to the row kind; Shift+Down SHALL cancel. The preview region SHALL show the focused file's stable name, admitted format metadata, duration, channel count, preparation/status text, and control-side waveform summary without becoming focusable. The browser MUST NOT invoke a native file dialog or nest another modal.

#### Scenario: Navigate a folder
- **WHEN** the user focuses a folder row and presses Edit
- **THEN** the reducer requests that library-relative listing and deterministically focuses the first enabled semantic row when it arrives

#### Scenario: Catalog refresh preserves focus
- **WHEN** metadata or availability changes without removing the focused stable row
- **THEN** the browser preserves that row's semantic focus across reprojection and density changes

### Requirement: Start provides cancellable prepared preview only in the browser
Pressing Start on an eligible focused file row SHALL create a semantic preview-hold request; releasing Start, moving focus, navigating, assigning, cancelling, or closing the browser SHALL create the matching semantic stop/cancel request. Preview preparation SHALL occur off the callback and be correlation-safe. If preparation completes while Start remains held on the same row, one preallocated audition voice SHALL play the raw asset once at its original pitch, from file start with no loop, through the origin Patch's post-effect, trim, Mixer track, send, return, mute, solo, level, and pan path. Release SHALL apply a bounded 5 ms de-click stop. Preview MUST NOT alter the active Sample reference or persisted session, and Start SHALL remain unavailable outside the Sample Browser.

#### Scenario: Hold after preview is ready
- **WHEN** Start is pressed and held on a prepared eligible file row
- **THEN** only that correlated file begins audible preview through the origin Patch route and the UI marks preview as `PLAYING` using text or shape as well as color

#### Scenario: Release before preview is ready
- **WHEN** Start is released while the focused file is still preparing for preview
- **THEN** the pending request is cancelled and a later completion cannot start audio

#### Scenario: Focus changes during preview
- **WHEN** Up or Down changes the focused row while preview is audible
- **THEN** the audition voice stops through the bounded de-click path before any new row can preview

### Requirement: Sample detail correlates controls and waveform landmarks
Sample detail SHALL use the shared `PatchDetail` shell and descriptor ordering while projecting the active/requested asset control, root pitch, playback and loop controls, and the canonical Patch envelope. Its non-focusable waveform SHALL label start, play position, loop start, loop end, and end using the prepared 2,048-pair-or-smaller summary and current canonical landmarks. The visible control count, asset names, parameter examples, and values shown in Figma SHALL remain fixtures; the installed descriptor and active asset SHALL determine production content.

#### Scenario: Edit a landmark
- **WHEN** a valid Sample playback or loop parameter is edited through the semantic action path
- **THEN** the reducer updates canonical requested state and the projected text values and waveform landmarks correlate to the same revision

#### Scenario: Asset is unavailable on restore
- **WHEN** a saved Sample reference cannot be resolved during restore
- **THEN** Sample detail shows the stable missing reference and explicit `Unavailable` state while the system avoids decoding, substituting, or presenting the asset as ready
