## ADDED Requirements

### Requirement: Canonical sixteen-track ownership and Patch routing
The application SHALL own exactly sixteen persistent mixer tracks identified in stable order as T00 through T0F, independently of installed Patch count, order, route, and instrument schema. Each track SHALL own Level, Pan, Mute, Solo, Reverb Send, and Delay Send. Each Patch SHALL instead own exactly one validated output track and one Patch-local trim value, SHALL own none of the track controls or meters, and SHALL be routable to the same track as any other Patch. PATCH Utility SHALL expose the focused Patch's trim and output track, while MIXER SHALL expose all sixteen tracks, including empty tracks, through stable track-based semantic focus and the canonical reducer path.

#### Scenario: Mixer opens with any Patch population
- **WHEN** MIXER is projected with zero, one, or several installed Patches
- **THEN** exactly T00 through T0F appear in stable order with their own controls and meters, and Patch population or schema creates, removes, reorders, or resets no track

#### Scenario: Two Patches share a track
- **WHEN** two sounding Patches are routed to one track and that track's Level changes
- **THEN** their post-effect, post-trim stems are summed before the track controls, both contributions change together, and every unrelated track remains sample-identical

#### Scenario: Patch output is adjusted
- **WHEN** the player adjusts one Patch's trim or output track through PATCH Utility
- **THEN** normalized input becomes a semantic event accepted by `AppState::apply`, every same-generation projection agrees on the new Patch output, and no other Patch or track parameter changes

#### Scenario: Invalid destination is requested
- **WHEN** a Patch output candidate identifies a destination outside the sixteen-track bank
- **THEN** the reducer rejects it transactionally with a typed reason and publishes no state, parameter, command, or graph change

#### Scenario: Empty boundary tracks are navigated
- **WHEN** no Patch is routed to T00 or T0F and the player navigates across MIXER
- **THEN** both tracks remain visible and focusable by stable track identity, the focused control row is preserved horizontally, and no Patch or widget index defines the selection

### Requirement: Bounded track rendering, observation, and proof
Every complete real-time parameter generation SHALL carry each active Patch's validated output route and trim plus exactly sixteen track parameter entries. The renderer SHALL use fixed preallocated track accumulation, apply Patch effects and trim before destination summing, apply track Level and Pan after summing, capture each track's numeric meter before its audibility gate, let Mute override Solo, exclude non-soloed tracks whenever any Solo is active, and feed shared Reverb and Delay only from post-fader, post-gate track sends. Audio observations SHALL carry exactly sixteen generation-correlated numeric track meters separately from canonical UI state. The callback MUST perform no allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwind, or destruction, and assertion-bearing production-path evidence SHALL prove these behaviors in both deterministic and physical-live compositions.

#### Scenario: Muted track receives audio
- **WHEN** a sounding track is muted
- **THEN** its pre-gate meter remains nonzero while its dry contribution and both shared-effect send inputs are zero

#### Scenario: Solo and mute interact
- **WHEN** at least one track is soloed and a soloed track is also muted
- **THEN** every non-soloed track is excluded and the muted soloed track remains excluded because Mute wins

#### Scenario: Patch route changes during rendering
- **WHEN** a valid Patch destination changes between compatible parameter generations
- **THEN** the next consumed complete generation moves only that Patch contribution among the already-prepared sixteen destinations without rebuilding or substituting the structural graph

#### Scenario: Track observations reach the window
- **WHEN** the audio callback publishes a newer observation and the graphical window renders it
- **THEN** each meter is addressed by its matching `MixerTrackId`, stale or partial meter state cannot become canonical UI state, and the view performs no audio ownership or routing work

#### Scenario: Focused deterministic acceptance runs
- **WHEN** the named sixteen-track mixer acceptance target executes
- **THEN** it proves exact track persistence, shared-track summing, reroute and trim isolation, all six track controls, gate and send order, meters, invalid-route rejection, fixed snapshot equality, finite output, and zero callback allocation or destruction before printing its success marker

#### Scenario: Physical live acceptance completes
- **WHEN** `make demo-live-sixteen-track-mixer-routing` runs against the production window and physical audio stream
- **THEN** it reports all sixteen tracks and routing behaviors from measured production observations, performs semantic note cleanup, closes the window, releases the stream, shuts down the worker, collects all owned graphs, and exits normally within its declared bound
