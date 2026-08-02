# Phase 1 Data Model: Expandable Effects and Bus Topology

Shapes are indicative of structure and boundedness, not final signatures. The
binding rule throughout: **no type below names a specific effect or bus**.

## Constants

| Name | Value | Source of truth |
|---|---|---|
| `MAX_PATCHES` | 16 | existing |
| `MAX_EFFECT_SLOTS` | 3 | DESIGN.md:690 product maximum, C-001 |
| `MAX_BUS_RETURNS` | 8 | DESIGN.md:690 product maximum, C-002 |
| `MAX_EFFECT_SCALAR_PARAMETERS` | 8 | existing |
| `MixerTrackId::COUNT` | 16 | existing, unchanged |

## Value objects

### `BusId`

Bounded routing destination identity. `0..MAX_BUS_RETURNS`.

- **Validation**: constructed only from an in-range index; out-of-range is rejected before publication, never clamped
- **Invariant**: stable and independent of what currently occupies the return — rerouting a return's effect does not change any `BusId`
- **Note**: identity is positional, not nominal. There is no `BusId::Reverb`

### `EffectSlotIndex`

Ordered position on a Patch. `0..MAX_EFFECT_SLOTS`. Peer of the existing
`EffectSlotId` (which identifies a *configured instance*, not a position).

### `BusSendLevel`

One send amount. Range `0.0..=1.0`, finite, descriptor-bounded — the same bounds
the current `ReverbSend`/`DelaySend` descriptors carry.

### `EffectRegistryEntry` (extended)

Existing effect capability descriptor. Gains a role-independence property: the same
entry is admissible in a Patch slot and in a bus return. No entry declares which
role it belongs to.

## Aggregates and state

### `Patch` (modified)

```
Patch {
  id, engine config, envelope, output: PatchOutput,
  effects: [Option<EffectConfig>; MAX_EFFECT_SLOTS],   // was post_effects: Vec<PostEffectConfig>
}
```

- **Invariant**: slot order is render order (FR-004)
- **Invariant**: two occupied slots holding the same registry entry are independent instances with disjoint internal state (FR-005)
- **Invariant**: the effect array is Patch-owned and follows the Patch across rerouting (FR-018)
- **Transition**: `SetSlotOccupancy(EffectSlotIndex, Option<RegistryEntryId>)` — structural, correlated, validated

### `MixerTrackParameters` (modified)

```
MixerTrackParameters {
  level_db, pan, mute, solo,
  sends: [BusSendLevel; MAX_BUS_RETURNS],   // was reverb_send, delay_send
}
```

- **Invariant**: sends are taken post-fader and post-gate; a muted or solo-excluded track contributes no wet signal (FR-011, C-005)
- **Invariant**: mute always wins over solo
- **Addressing**: parameter identity is `(MixerTrackId, control)` where a send control carries its `BusId` as data, not as a variant name

### `BusReturn` (new)

```
BusReturn {
  id: BusId,
  effect: Option<EffectConfig>,   // from the same registry as Patch slots
  return_level: f32,
}
```

- **Invariant**: an unoccupied return contributes silence — it never passes its input through (spec Edge Cases)
- **Invariant**: a return's output sums into the mix and cannot feed another return; no routing cycle can be expressed (C-006)
- **Transition**: `SetReturnOccupancy(BusId, Option<RegistryEntryId>)` — same lifecycle as slot occupancy

### `GlobalParameters` (reduced)

```
GlobalParameters { master_gain_db }
```

Six reverb and delay fields dissolve per R-04.

## Real-time projections

### `RtPatchParameters` (modified)

```
RtPatchParameters {
  patch_id, output, envelope,
  instrument: [f32; MAX_INSTRUMENT_SCALAR_PARAMETERS],
  effects: [RtPostEffectParameters; MAX_EFFECT_SLOTS],   // was one
}
```

### `RtMixerTrackParameters` (modified)

Gains `sends: [f32; MAX_BUS_RETURNS]`.

### `RtBusReturnParameters` (new)

```
RtBusReturnParameters {
  active: bool,
  slot_id: Option<EffectSlotId>,
  scalar_count: usize,
  scalars: [f32; MAX_EFFECT_SCALAR_PARAMETERS],
  return_level: f32,
}
```

### `ParameterSnapshot` (widened)

One fixed latest-value block (R-01): `patches[16]`, `mixer_tracks[16]`,
`returns[8]`, `global`.

- **Invariant**: fixed layout, no dynamic growth, destructor-free (NFR-002)
- **Invariant**: exact structural matching against the prepared racks survives widening — `matches_parameters` stays exact, never permissive

## Prepared graph

### `PreparedPostEffectRack` (modified)

```
slots: [[Option<PreparedSlot>; MAX_EFFECT_SLOTS]; MAX_PATCHES]   // was [Option<_>; MAX_PATCHES]
```

Processes each occupied slot in index order, in place on the matching stem.

### `PreparedBusReturnRack` (new)

Peer of the post-effect rack, owned by `PreparedGraph`. Fixed capacity
`[Option<PreparedReturn>; MAX_BUS_RETURNS]`, each with its own preallocated input
scratch. Replaces the retired `GlobalEffectsProcessor` call site.

## Signal flow (revised from DESIGN.md:396-415)

```
patch instrument
  → ordered patch post FX  (slots 0..2, in index order)
  → patch trim
  → route and sum into one of 16 tracks
  → track level / pan
      ├──→ pre-gate track meter
  → track mute / solo gate
      ├──→ post-gate sends[0..7] ──→ bus returns[0..7] ──┐
  → 16-track dry mix ←──────────────────────────────────┘
  → master gain / safety limiter
  → stereo device output
```

Structurally identical to today; the two named aux paths become eight indexed ones.
Meter position, gate order, and send position are unchanged.

## Lifecycle and events

| Change | Kind | Path |
|---|---|---|
| Slot / return occupancy | Structural | Off-callback preparation, complete graph exchange at block boundary, correlated acknowledgement |
| Slot / return scalar value | Scalar | Latest-value snapshot |
| Send level, track controls | Scalar | Latest-value snapshot |
| Patch route change | Validated scalar | Existing fixed-size snapshot value — the prepared graph already owns all 16 destinations |

**Rejection**: a refused structural change publishes no graph and leaves the active
one untouched (FR-013); the outcome and its position are projected (FR-014); a
subsequent valid change proceeds normally (FR-015); superseded graphs are retired
off-callback (FR-016).
