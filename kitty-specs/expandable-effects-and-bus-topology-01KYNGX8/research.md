# Phase 0 Research: Expandable Effects and Bus Topology

Every open question from planning is resolved. No `[NEEDS CLARIFICATION]` markers
remain; `spec-kitty agent decision verify` reports clean.

## R-01 — Real-time snapshot growth strategy

**Decision**: Keep one monolithic fixed-layout `ParameterSnapshot` and widen it in
place: `patches[16] x effects[3] x scalars[8]`, `tracks[16]` each with `sends[8]`,
and `returns[8]`.

**Rationale**: The architecture declares three distinct real-time transports — one
for discrete events, one for latest scalar snapshots, one for prepared structural
graphs. Splitting scalars into three snapshots would multiply the *number* of
transports without adding a category, and would introduce cross-snapshot
correlation questions that the current design does not have to answer. The existing
`PreparedPostEffectRack::matches_parameters` exactness proof depends on a fixed
layout; keeping one block preserves it. Copy cost rises but stays bounded and
allocation-free, which is what the callback contract actually constrains.

**Alternatives considered**:
- *Split transports* — smaller copies per edit, but three revisions to correlate and three staleness windows to prove. Rejected: pays real proof complexity for a copy-size problem that is not yet measured as a bottleneck.
- *Sparse indexed layout* — smallest payload, but index resolution moves into the render loop and the fixed-layout guarantee behind exact matching weakens. Rejected: trades a proof for a micro-optimization.

**Follow-through**: NFR-002 requires demonstrating zero dynamic growth at render
time under any reachable configuration; the widened layout must be measured, not
assumed, since the block roughly triples.

## R-02 — Fate of `GlobalEffectsProcessor`

**Decision**: Retire the port entirely. Reverb and delay become ordinary
`EffectCapabilityProvider` + `EffectPreparer` pairs and process through the same
generic prepared-effect boundary as Chorus.

**Rationale**: `process(reverb_input, delay_input, output, parameters)` encodes the
number *and identity* of returns in a type signature. No amount of downstream
generality survives that. Retiring it leaves exactly one prepared-effect boundary,
which is what makes FR-009 and SC-008 achievable.

The user's stated rationale is recorded verbatim in the decision record: the port
was closed against an expansion that DESIGN.md declared from the outset, which is
an open-closed violation rather than a missing feature. This plan treats that as a
design finding, not a preference — see R-05.

**Alternatives considered**:
- *Widen `process()` to a slice of inputs* — far smaller diff, existing DSP nearly untouched. Rejected by the user: bus returns would still process through a different boundary than Patch slots, preserving the two-model split FR-009 exists to remove, and leaving the next roster addition equally expensive.

## R-03 — Send and parameter addressing

**Decision**: No name-enumerated addressing anywhere. A track owns `sends` as a
fixed array of one generic send value type addressed by `BusId`. Parameter identity
is descriptor-driven and index-addressed rather than one enum variant per
destination. The same rule governs Patch effect slots and bus returns.

**Rationale**: This was the user's explicit correction to all three options offered
during planning, and it is the correct domain-driven reading. `ReverbSend` and
`DelaySend` are not two kinds of parameter; they are one kind of parameter
(a send level) pointing at two different destinations. Encoding the destination in
the parameter's *name* conflates identity with addressing. Once separated, eight
sends cost no more type surface than two, and the twelfth roster effect costs none.

**Alternatives considered**:
- *`Send(BusId)` parameterized variant* — compact but keeps sends inside an enum that must still be matched exhaustively everywhere.
- *Flat `Send1..Send8` variants* — smallest departure from shipped code, but hardcodes the bound into the type and reproduces the original fault at a larger N.
- Both rejected: they generalize the count while leaving the naming pattern intact.

## R-04 — Fate of `GlobalParameters`

**Decision**: `GlobalParameter` retains only `MasterGainDb`. `ReverbRoomSize`,
`ReverbDamping`, `DelayMilliseconds`, and `DelayFeedback` become descriptor scalars
of their registry entries. `ReverbReturn` and `DelayReturn` become a per-return
output level owned by each bus return.

**Rationale**: Once a return can hold any registry effect, keeping reverb-specific
fields in a globals struct means the same value exists in two places with different
lifetimes. Master gain is genuinely global — it belongs to the master stage, not to
any effect — so it stays.

**Consequence for MIXER**: DESIGN.md:309's "sixteen track-owned controls and
distinct globals" remains true; the set of distinct globals shrinks to one, and
return controls become return-owned. This is captured in the reconciliation table.

## R-05 — Preventing recurrence of the closed design

**Decision**: Add a proof-enforced architecture invariant — *no name-enumerated
effect or routing identity* in Synth, Mixer, RealTime, or Control — with a static
project check that fails on effect-specific identifiers in those contexts.

**Rationale**: The expansion to three slots and eight returns was declared in
DESIGN.md before the closed code was written, and the closed code shipped anyway.
That is evidence that a prose constraint in the design document is not a sufficient
control. The architecture spec's own model already treats `validations` as
executable gates and warns against replacing measured proof with self-reported
text; this applies that principle to the design property itself.

**Alternatives considered**:
- *Reviewer convention / checklist item* — rejected: indistinguishable from the control that already failed.
- *Runtime assertion* — rejected: the property is structural and knowable at build time; a render-path assertion would also violate the callback contract.

**Scope note**: The check is seeded with the concrete strings this mission retires
and the contexts it touches. It is deliberately narrow — it constrains effect and
routing identity, not all naming — so it can be enforced without false positives on
`MasterGainDb` or on genuinely singular concepts.

## R-06 — Order-sensitivity evidence without new DSP

**Decision**: Prove slot ordering with the reverb, delay, and chorus entries the
registry holds after R-02, rather than adding a new effect type.

**Rationale**: C-004 forbids new third-party processing and C-011 defers the roster.
Once reverb and delay are registry entries, three genuinely different processors are
available, so `A→B ≠ B→A` is demonstrable both audibly and by measurement. This is
what makes the "architecture only" scope decision coherent: it does not weaken the
ordering proof, because generalizing the existing effects supplies the variety.

**Alternatives considered**:
- *Chorus-only ordering proof* — would have made ordering measurable but not audible, weakening the live-demo gate. Avoided as a consequence of R-02 rather than by adding scope.
