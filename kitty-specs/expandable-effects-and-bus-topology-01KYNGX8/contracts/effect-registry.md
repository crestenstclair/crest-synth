# Contract: Effect Registry (role-independent)

There are no HTTP or GraphQL surfaces in this project. Contracts here are port and
boundary contracts — the same thing the architecture spec means by `ports`.

## Ports

`port.Synth.EffectCapabilityProvider` and `port.Synth.EffectPreparer` are unchanged
in shape and gain one obligation:

> An entry declares identity, visible parameters, bounds, units, and preparation
> requirements. It does **not** declare whether it may occupy a Patch slot or a bus
> return. Role admissibility is decided by the caller, not the entry.

## Obligations

| # | Obligation | Verified by |
|---|---|---|
| C-ER-1 | The same registry entry can be prepared into a Patch slot and into a bus return, producing independent instances | Contract test preparing one entry into both roles and asserting disjoint state |
| C-ER-2 | Two instances of one entry never share delay lines, LFO phase, or tails | Existing two-Chorus independence proof, extended to reverb and delay |
| C-ER-3 | Preparation is off-callback; no instance allocates, locks, or blocks during processing | Existing real-time contract validation |
| C-ER-4 | An entry that fails preparation yields a refusal; no partially prepared instance is published | Controlled-negative witness command |
| C-ER-5 | Adding a registry entry requires no change to slot, routing, snapshot, preparation, projection, or render structure | SC-008 — a test that adds a synthetic entry and asserts zero structural change |

## Retired contract

`port.Mixer.GlobalEffectsProcessor` is deleted, including
`process(reverb_input, delay_input, output, parameters)`. Its two obligations that
must survive, now carried by the return rack:

- wet excitation derives exclusively from declared inputs; samples already in `output` are never an implicit send
- zero effect input cannot produce a wet return

These are preserved verbatim as return-rack obligations in `bus-routing.md`.

## Migration of existing entries

| Was | Becomes | Descriptor scalars |
|---|---|---|
| Reverb inside `GlobalReverbDelay` | registry entry + preparer pair | room size, damping |
| Delay inside `GlobalReverbDelay` | registry entry + preparer pair | milliseconds, feedback |
| Chorus | unchanged | amount, depth |

Return level is **not** a descriptor scalar — it belongs to the `BusReturn`, since
it is a property of the destination rather than of the effect (R-04).
