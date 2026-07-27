## ADDED Requirements

### Requirement: Instrument and effect capabilities have distinct contracts
The system SHALL keep instrument and Patch-effect capability identities, descriptors, registries, providers, preparers, and prepared callback ports distinct. Both families SHALL reuse the single canonical parameter/assignment/value/asset types. Instrument descriptors SHALL retain voice policy and supported MIDI semantics; effect descriptors SHALL contain neither and SHALL describe only effect configuration and parameter projection. Canonical `Patch` SHALL own one instrument config and an ordered zero-or-one effect-config list without concrete engine/effect fields or fallback state.

#### Scenario: Both registries are projected
- **WHEN** the production composition installs two instrument capabilities and `effect.chorus`
- **THEN** StateTree contains separate immutable instrument and effect registry schemas joined to Patch configs by stable identities while shared parameter types serialize only once per concept

#### Scenario: Effect metadata is treated as instrument metadata
- **WHEN** an effect descriptor declares voice policy or supported MIDI kinds, or an instrument descriptor is used as an effect registration
- **THEN** registry construction fails before Patch installation or preparation and no adapter is inferred from the other family

#### Scenario: A Patch is serialized
- **WHEN** one Patch has an instrument config and a Chorus config
- **THEN** canonical state contains both generic configs and stable identities but no Chorus-specific field, prepared processor, delay buffer, descriptor copy, UI state, bypass, or fallback

