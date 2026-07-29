# Contract: Bus Routing, Sends, and Returns

## Identity

| Rule | Statement |
|---|---|
| B-1 | `BusId` is positional, bounded to `0..8`, and validated before publication — never clamped, never defaulted |
| B-2 | A `BusId` is stable across changes to what occupies its return |
| B-3 | No type names a bus. `BusId::Reverb` and equivalents are prohibited by the no-name-enumeration invariant |

## Send stage

| # | Obligation | Verified by |
|---|---|---|
| C-BR-1 | Sends are taken after the fader and after the mute/solo gate | Sample-exact routing test at the gate boundary |
| C-BR-2 | Mute always wins; when any track is soloed, only soloed non-muted tracks contribute dry signal **or** sends | Existing gate proof, extended across 8 sends |
| C-BR-3 | A send scales only its own track's contribution to its own destination | Isolation measurement, NFR-007 |
| C-BR-4 | Two tracks sending to one bus sum at that bus | Accumulation test |
| C-BR-5 | Raising one send leaves the other seven destinations below −60 dBFS from that source | SC-004 measurement |

## Return stage

| # | Obligation | Verified by |
|---|---|---|
| C-BR-6 | An unoccupied return contributes silence; it never passes its input through | Negative test on an empty return |
| C-BR-7 | Wet excitation derives exclusively from the return's declared input; samples already in the output are never an implicit send | Inherited from the retired port's contract |
| C-BR-8 | Zero input cannot produce a wet return | Inherited from the retired port's contract |
| C-BR-9 | A return sums into the dry mix and cannot feed another return; no routing cycle is expressible | Structural test — the topology has no return-to-send edge |
| C-BR-10 | Return level is owned by the return, not by the effect | Type-level; return level survives changing the occupying effect |

## Preserved from the sixteen-track gate

Unchanged by this mission and re-proved after it:

- exactly sixteen persistent tracks
- a Patch owns only its output `MixerTrackId` and pre-track trim
- multiple Patches may share a track; no Patch is silently rerouted or lost
- meters observe post-level/pan, pre-gate, so muted tracks stay diagnosable
- a Patch route change is a validated fixed-size snapshot value, not a graph rebuild
