# Contract: Real-Time Transport and Topology Lifecycle

## Callback contract (unchanged, re-proved at the new size)

| # | Obligation |
|---|---|
| C-RT-1 | No allocation, locking, blocking, I/O, logging, panic, or destruction on the render path — including during topology activation |
| C-RT-2 | All capacity is reserved before rendering: `16 patches x 3 slots x 8 scalars`, `16 tracks x 8 sends`, `8 returns x 8 scalars` |
| C-RT-3 | Zero dynamic growth events at render time under any configuration reachable in this mission (NFR-002) |

## Snapshot

| # | Obligation | Verified by |
|---|---|---|
| C-RT-4 | One fixed-layout latest-value snapshot carries all scalar state (R-01) | Layout test |
| C-RT-5 | Structural matching between snapshot and prepared racks stays **exact**, not permissive, at the widened size | Extended `matches_parameters` proof — a mismatched layout must still be rejected |
| C-RT-6 | `SERIALIZED_LEAF_DESCRIPTOR` enumerates every leaf of the widened block | Descriptor completeness test |
| C-RT-7 | Publish cost stays bounded and destructor-free despite the roughly tripled block | Measured, not assumed — the highest RT risk in the mission |

## Structural change lifecycle

| # | Obligation | Verified by |
|---|---|---|
| C-RT-8 | Slot and return occupancy changes are prepared off-callback and exchanged as a complete graph | Existing handoff contract, extended |
| C-RT-9 | Activation occurs exactly at a block boundary; no rendered block observes a partially applied topology (NFR-003) | Block-boundary activation test |
| C-RT-10 | A refused change publishes no graph; the active graph is untouched and remains audible (FR-013) | Controlled-negative witness |
| C-RT-11 | The outcome — pending, accepted, refused — is projected with its reason and its position (FR-014) | Projection test |
| C-RT-12 | A valid change immediately after a refused one succeeds with no residue (FR-015) | Recovery test |
| C-RT-13 | Two changes requested before the first is acknowledged neither reorder nor drop acknowledgements | Correlation test |
| C-RT-14 | Superseded graphs are retired off-callback; nothing is owned at exit (FR-016, NFR-006) | Existing retirement proof, extended |

## Scalar vs structural

| Change | Kind |
|---|---|
| Slot or return occupancy | Structural — prepared, exchanged, acknowledged |
| Slot or return scalar value | Scalar — latest-value snapshot |
| Send level, track level/pan/mute/solo | Scalar — latest-value snapshot |
| Return level | Scalar — latest-value snapshot |
| Patch route change | Validated fixed-size scalar; the prepared graph already owns all 16 destinations |

Scalar and structural changes must coexist within one block without either being
lost — an existing proven property that must survive the widening.
