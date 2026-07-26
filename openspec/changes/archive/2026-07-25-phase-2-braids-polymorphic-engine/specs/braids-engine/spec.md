## ADDED Requirements

### Requirement: Pinned authoritative Braids source
The Braids capability SHALL generate sound by wrapping the pinned official Mutable Instruments C++ `MacroOscillator` implementation and its pinned `stmlib` dependency. The repository SHALL preserve all required copyright and MIT license notices, record exact source revisions and included files, and SHALL NOT substitute a reimplementation, another engine, or an unpinned network dependency.

#### Scenario: Source provenance is inspected
- **WHEN** the installed Braids build inputs are audited
- **THEN** every compiled upstream file is attributable to the declared immutable revisions and required notices are present

#### Scenario: Required source is missing or changed
- **WHEN** a compiled Braids file is absent, differs from its provenance manifest, or cannot be licensed and built as declared
- **THEN** validation fails before the capability can be claimed or a fallback renderer can be selected

### Requirement: Descriptor-owned Braids capability
The installed registry SHALL contain `instrument.braids` as a capability distinct from `instrument.soundfont.hidef`. Its ordered descriptor SHALL expose Model as the complete 47-choice upstream playable model set, Timbre and Color as bounded continuous values, classify all three as Scalar, declare no asset requirement, report `FixedPerPatch(16)`, and declare only MIDI message kinds the prepared adapter implements.

#### Scenario: Braids descriptor is projected
- **WHEN** canonical capability state is serialized or rendered
- **THEN** Model, Timbre, Color, their stable ids, complete choices, bounds, defaults, steps, update classes, fixed-per-Patch voice policy, and MIDI kinds come from the one registered descriptor

#### Scenario: A Braids config is malformed
- **WHEN** Model names an undeclared choice or Timbre or Color is missing, wrong-kind, non-finite, or out of range
- **THEN** provider, reducer installation, and preparation reject the config without selecting another model, value, capability, or renderer

### Requirement: Sixteen independent prepared Braids voices
Each prepared Braids Patch SHALL own a distinct bank of exactly sixteen fully initialized oscillator voices and bounded voice/envelope state. No oscillator, slot, envelope, note allocation, or voice limit SHALL be shared globally among Braids Patches. Note-on SHALL choose an idle voice or deterministically steal the oldest voice in the targeted Patch when all sixteen of that Patch's voices are occupied; note-off SHALL release the matching key voices in that Patch; velocity, pitch bend, and declared expression semantics SHALL remain Patch-local; and all-notes-off SHALL clear that Patch's voices with bounded work.

#### Scenario: Sixteen notes overlap
- **WHEN** sixteen distinct note-ons target one prepared Braids Patch before note-off
- **THEN** sixteen independent voices sound with separate note and envelope lifecycles and no note is silently collapsed into monophony

#### Scenario: A seventeenth note arrives
- **WHEN** every voice slot is occupied and another note-on targets the Patch
- **THEN** the oldest slot is reset and reused deterministically, active work remains bounded by sixteen, and no other Patch receives or renders the note

#### Scenario: Braids Patch count scales voice capacity
- **WHEN** the graph admits `N` active Braids Patches within its independently declared active-Patch capacity
- **THEN** it owns `N` distinct native banks and `16 × N` Braids voices (including forty-eight for three Patches), and activity or exhaustion in one bank does not consume, steal, or silence a voice in any sibling bank

#### Scenario: Overall Patch capacity is reached
- **WHEN** another Braids Patch would exceed the engine-agnostic prepared-rack Patch capacity
- **THEN** graph preparation rejects that Patch exactly as it would reject any other capability at the same rack limit, without imposing a lower Braids-specific Patch limit or reducing the sixteen voices of any admitted Braids Patch

### Requirement: Explicit 96-to-48 kHz rendering policy
Braids preparation SHALL accept a 48 kHz host configuration, run the pinned oscillators at their native 96 kHz calibration in chunks no larger than 24 samples, and convert each adjacent upstream pair into one finite host sample using bounded preallocated work. Any unsupported sample rate or invalid frame capacity SHALL fail before graph publication without retuning Braids, selecting SoundFont, or installing a silent renderer.

#### Scenario: Supported host block renders
- **WHEN** a 48 kHz graph asks a prepared Braids Patch to render any frame count up to its prepared maximum
- **THEN** the adapter covers the complete block through bounded 24-sample upstream chunks and produces finite stereo samples without allocation or out-of-bounds access

#### Scenario: Unsupported rate is requested
- **WHEN** Braids preparation receives a finite positive rate other than 48 kHz
- **THEN** it returns a typed unsupported-sample-rate failure and no partial instrument or fallback graph is published

### Requirement: Capability-isolated live scalar consumption
The prepared Braids renderer SHALL consume only the matching Patch's descriptor-ordered Model, Timbre, and Color scalar projection plus common Patch envelope and MIDI expression. A scalar change SHALL take effect without structural graph construction, and another Patch's parameters SHALL NOT alter its model, waveform, envelope, or stem.

#### Scenario: Each Braids control changes
- **WHEN** Model, Timbre, and Color are adjusted one at a time through the production reducer and latest snapshot path while a controlled note is rendered
- **THEN** each accepted value produces a measured finite waveform or energy difference in only that Braids Patch stem

#### Scenario: Another Patch changes
- **WHEN** a SoundFont or sibling Braids Patch parameter changes
- **THEN** the untargeted Braids Patch retains its exact scalar values and an identically initialized comparison render remains sample-identical

### Requirement: Exception-free hard-real-time FFI boundary
Braids creation, initialization, and destruction SHALL occur outside the audio callback. Callback dispatch and rendering across the native boundary SHALL be bounded and SHALL perform no allocation, deallocation, locking, blocking, I/O, logging, formatting, exception throwing, unwinding, or object destruction.

#### Scenario: Mixed-engine callback is measured
- **WHEN** the declared worst-case set of per-Patch sixteen-voice Braids banks, engine-managed SoundFont traffic, scalar updates, and bounded graph handoff are rendered through the production callback
- **THEN** output remains finite, callback allocation/destruction and native create/destroy deltas remain zero, and measured render time satisfies the declared callback budget

#### Scenario: Native ownership is retired
- **WHEN** a graph containing Braids oscillators is replaced
- **THEN** every Patch-owned native oscillator bank remains intact until the retired graph reaches control or worker ownership and only then is destroyed
