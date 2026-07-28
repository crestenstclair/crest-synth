## MODIFIED Requirements

### Requirement: Descriptor-derived Patch editable surface
The ordered semantic surface for the focused Patch SHALL be derived from production-owned resolvers containing its Patch-output trim and destination track in PATCH Utility, its four common ADSR values and active descriptor parameters classified StructuralChoice in PATCH Main, followed by configured Patch-effect parameters classified ScalarEdit, and no mixer-track value. Descriptor-classified instrument Scalar values remain projected and fixed-snapshot encoded but read-only in the current bounded PATCH slice. A distinct production-owned MIXER resolver SHALL contain Level, Pan, Mute, Solo, Reverb Send, and Delay Send for each of exactly sixteen `MixerTrackId` values plus the editable globals. Reducer navigation, adjustment, text selection, deterministic demo coverage, and live-demo coverage SHALL use these same resolvers.

#### Scenario: SoundFont Patch surface is derived
- **WHEN** a SoundFont Patch is selected
- **THEN** its output trim, destination track, and ADSR values are editable through PATCH while bank, program, percussion, and file remain visible Structural values outside the scalar selection cycle, and no track control is attributed to the Patch

#### Scenario: Braids Patch surface is derived
- **WHEN** a Braids Patch is selected
- **THEN** its output trim, destination track, and ADSR controls use stable PATCH identities, Model, Timbre, and Color remain descriptor-projected read-only values in this slice, and no track control is attributed to the Patch

#### Scenario: Mixer surface is derived
- **WHEN** MIXER is selected with any installed Patch population
- **THEN** every one of the six canonical track parameters is reachable for T00 through T0F in stable track order, globals remain distinct, and neither Patch identities nor instrument descriptors determine mixer columns
