## ADDED Requirements

### Requirement: The first static Patch effect is canonical, fixed, prepared, controllable, and falsifiably verified
Every installed Patch effect SHALL have a stable `EffectCapabilityId`, a separate immutable `EffectCapabilityDescriptor`, and exactly one identity-matched control-side provider and worker-side preparer. Every configured instance SHALL have a stable Patch-local `EffectSlotId` and canonical `PostEffectConfig`. Effect descriptors SHALL reuse the canonical parameter and asset value types but SHALL NOT contain instrument voice policy, supported MIDI semantics, engine state, prepared DSP, delay memory, or UI state. Unknown, duplicate, missing, mismatched, invalid, or unavailable registration and config cases SHALL fail with typed errors without fallback or bypass.

#### Scenario: Valid effect registry and config are installed
- **WHEN** `effect.chorus` has one unique descriptor, provider, and preparer and a Patch supplies one matching descriptor-ordered config
- **THEN** registry/config validation succeeds by stable identities without label, registry-index, or processor-specific matching

#### Scenario: Effect registration or config is invalid
- **WHEN** an effect id or slot is unknown or duplicated, provider/preparer registration is missing or mismatched, or an assignment is missing, undeclared, wrong-kind, non-finite, out of range, or dependency-invalid
- **THEN** installation or preparation fails atomically and no descriptor, config, processor, bypass, dry substitute, or alternate effect is selected

**Contract facet — fixed ordered Patch effect topology.**
Each Patch SHALL own an ordered post-effect config list with a current capacity of zero or one. The production fixture SHALL configure exactly one `effect.chorus` slot on its first Patch and no effect on every other Patch. The callback SHALL process each configured Patch in the fixed order `PreparedEngineRack → PatchAudioBlock → PreparedPostEffectRack → MixEngine`. This increment SHALL expose no effect selector, bypass, reorder, removal, second slot, placeholder, arbitrary edge, parallel route, or external feedback path.

#### Scenario: Production fixture graph is prepared
- **WHEN** normal, smoke, deterministic-demo, or live-demo startup prepares the accepted production Patches
- **THEN** the first Patch owns one stable Chorus config and prepared instance, all other fixture Patches own zero effect slots, and the complete graph is not published until both aligned racks are ready

#### Scenario: Configured and unconfigured Patches render together
- **WHEN** the first and another fixture Patch produce simultaneous nonzero stems
- **THEN** Chorus mutates only the first Patch stem before its gain, pan, sends, mute/solo, and shared effects while the unconfigured Patch crosses the explicit effect stage sample-exactly

**Contract facet — pinned Chorus capability and preparation.**
The first effect SHALL be the MIT-licensed Mutable Instruments Rings Chorus pinned at `pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4` and `stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec`, vendored as an audited minimal required source/header/table/license/provenance subset with a SHA-256 manifest and exposed to the product only as `Chorus`. Its descriptor SHALL contain exactly Amount then Depth as continuous normalized `Scalar`/`ScalarEdit` parameters with range `0..=1`, default `0.5`, fine step `0.01`, and coarse step `0.1`. Each prepared instance SHALL own a distinct initialized processor, 2,048-sample 16-bit external delay buffer, and LFO/tail state. This first adapter SHALL accept exactly 48,000 Hz.

#### Scenario: Chorus source and schema are inspected
- **WHEN** the source provenance and installed effect descriptor are validated
- **THEN** both exact revisions, every vendored hash, MIT notice, product label, two parameter identities, order, kinds, bounds, defaults, and steps match the declared contract

#### Scenario: Two Chorus instances are prepared
- **WHEN** a focused deterministic graph configures Chorus on two Patches
- **THEN** it owns two distinct native processors, delay buffers, LFO states, and tails and activity in either instance cannot advance or overwrite state in the other

#### Scenario: Unsupported rate is requested
- **WHEN** Chorus preparation receives any rate other than exactly 48,000 Hz or a malformed scalar layout/capacity
- **THEN** it returns a typed preparation error before publication and does not resample, bypass, construct a null processor, or select another effect

**Contract facet — canonical PATCH effect control.**
PATCH SHALL project configured effects from the canonical Patch config and effect registry after Engine, common ADSR, and visible instrument `StructuralChoice` rows. Effect identity SHALL be visible and read-only; descriptor `ScalarEdit` rows SHALL use stable slot/parameter control identities. Edit+Left/Right SHALL apply the descriptor fine decrement/increment and Edit+Down/Up SHALL apply the coarse decrement/increment through `AppState::apply`. An accepted edit SHALL commit exactly one config assignment and publish only a complete same-layout latest scalar snapshot.

#### Scenario: First Patch Chorus is projected
- **WHEN** the configured first Patch is focused in PATCH
- **THEN** the page and text projection show read-only Chorus identity followed by exact Amount and Depth values after its instrument controls with one selected row and no processor-specific projector branch

#### Scenario: Amount or Depth is adjusted
- **WHEN** either effect row receives a valid fine or coarse semantic adjustment
- **THEN** exactly that canonical assignment changes by the declared step, StateTree/page/text/fixed effect snapshot share the accepted generation and value, and no audio command, preparation request, structural publication, or graph revision change occurs

#### Scenario: Effect identity or a boundary is adjusted
- **WHEN** the player attempts to edit the read-only identity or move a scalar beyond its bound
- **THEN** the reducer returns the applicable typed unchanged rejection, emits no effect or graph work, and accepts a later valid input

**Contract facet — hard-real-time prepared effect ownership.**
Every effect instance, external buffer, LFO/tail state, scalar layout, Patch-aligned rack slot, observation value, and scratch capacity SHALL be fully prepared before audio ownership. Callback processing SHALL be bounded and SHALL perform no allocation, deallocation, collection growth, locking, blocking, I/O, logging, formatting, panic, exception, unwind, or owned-state destruction. Replaced effect state SHALL return with the complete graph and be destroyed only on worker/control ownership.

#### Scenario: Chorus renders under bounded load
- **WHEN** the configured Patch renders 256-frame blocks at 48 kHz while scalar and structural traffic overlap
- **THEN** output remains finite, p99 complete-render time remains below 2.666 ms, callback allocation/deallocation/destruction/string counts remain zero, and every processor invocation stays within prepared capacity

#### Scenario: A complete graph is replaced under return pressure
- **WHEN** a graph containing Chorus is retired while the ownership-return transport is temporarily full
- **THEN** the callback retains complete ownership in bounded retirement storage, continues rendering, destroys no effect state, and activates no further graph until return succeeds

**Contract facet — falsifiable static effect acceptance.**
The repository SHALL provide a release-mode `static_patch_effect` target that emits `CREST_PATCH_EFFECT_OBSERVATION` and its success marker only after measuring exact source/license, schema/config, focus/edit, scalar-only publication, processor order, target-only audible difference, stereo side energy, Patch isolation, independent instances/tails, structural preservation, exact-rate/missing-registration failures, finite output, zero fallback, callback safety, and timing through production seams. The deterministic and physical demos SHALL also exercise both configured effect scalars; the focused target SHALL NOT replace either demo gate.

#### Scenario: Focused static-effect target passes
- **WHEN** `cargo test --release --test static_patch_effect -- --nocapture` executes
- **THEN** every structured predicate passes before `CREST_ACCEPTANCE static_patch_effect passed` is emitted

#### Scenario: Effect output is inferred rather than measured
- **WHEN** a test changes only a label/value, observes unrelated final output, shares instance state, omits source proof, bypasses the production renderer, or reports success before assertions
- **THEN** the named acceptance fails and static Patch-effect completion is not claimed

#### Scenario: Physical live acceptance runs
- **WHEN** apply acceptance reaches the external-runtime gate
- **THEN** a real release-mode `make demo-live` audibly edits Amount and Depth, completes its structural sequence with the Chorus config intact, cleans notes, closes the window, releases the stream, drains ownership, and returns parent-process success
