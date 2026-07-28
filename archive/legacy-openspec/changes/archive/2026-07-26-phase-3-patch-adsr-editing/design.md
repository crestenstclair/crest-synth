## Context

Phase 3 currently has a descriptor-driven PATCH projection and asynchronous Engine editing, while the same Patch-owned `VoiceEnvelope` is editable only through the transitional MIXER selection. The DSP side is already complete: SoundFont and Braids consume the four canonical ADSR values per note from the fixed parameter snapshot. This change therefore adds control focus, reducer resolution, projection, and proof; it does not add an envelope model or renderer.

The reconciled CUE model adds required goal `goal.edit_patch_envelope` and requirements `patch_adsr_focus_surface`, `canonical_patch_adsr_adjustment`, `scalar_only_patch_adsr_publication`, `patch_adsr_structural_coexistence`, and `patch_adsr_behavioral_proof`. The pivotal resources remain `valueObject.Synth.VoiceEnvelope`, `valueObject.Control.PatchControlId`, `valueObject.Control.InteractionState`, `aggregate.Control.AppState`, `valueObject.Control.PatchPageProjection`, `domainService.Control.StateProjector`, `applicationService.Control.AppLoop`, and the existing real-time parameter/graph boundaries.

The physical-to-semantic boundary does not change: `1`, `2`, W, S, A, D, and K still normalize to the same `SelectContext`, `Navigate`, and `Adjust` events. Only `AppState::apply` interprets those events using current context and focus.

## Goals / Non-Goals

**Goals:**

- Let the player navigate Engine, Attack, Decay, Sustain, and Release on PATCH with exactly one reducer-owned focus.
- Edit all four ADSR fields using their existing canonical identities, bounds, fine/coarse steps, reducer transaction, fixed snapshot layout, and per-voice DSP.
- Keep focus and scalar editing coherent throughout Ready, Preparing, Activating, and recoverable Failed engine-selection states.
- Keep focus-only navigation audio-neutral and every ADSR edit scalar-only.
- Make page/text/state/parameter observations exact and versioned, then prove the path through production input, reducer, projections, both renderers, and both demos.

**Non-Goals:**

- Editing capability-provided Scalar or Structural rows on PATCH.
- SoundFont preset discovery, name display, or selection; asset editing; an engine-choice modal.
- Sibling-Patch navigation or adding Shift to the current keyboard vocabulary.
- A new envelope aggregate, UI cache, post-stem envelope, DSP implementation, audio command, real-time transport, or prepared-graph workflow.
- The Figma-derived graphical replacement, effects, modulation, persistence, or another engine.

## Decisions

### 1. Extend `PatchControlId` with the canonical envelope parameter

`PatchControlId` will represent either `Engine` or `Envelope(VoiceEnvelopeParameter)`. Stable serialized identities will be `patch.engine` and `patch.envelope.<VoiceEnvelopeParameter.name>`. Its ordered surface is built as Engine followed by the four entries from the canonical `VoiceEnvelope` surface descriptor.

This preserves one type per concept: control state wraps the existing envelope parameter identity instead of declaring Attack/Decay/Sustain/Release variants or strings again. `InteractionState.patchControlFocus` remains independent of `patchFocus` and the MIXER selection. Patch installation initializes Engine; context switches and engine lifecycle events preserve the current control identity.

Bare Up/Down moves one position without wrapping. Bare Left/Right is rejected as `ActionUnavailableInContext`; sibling-Patch movement remains a later Shift-based feature. Up at Engine and Down at Release receive the same unchanged rejection. Nonwrapping movement matches the spatial page and avoids treating the bottom and top rows as adjacent.

### 2. Resolve PATCH navigation and adjustment only in `AppState::apply`

The reducer will dispatch by context and then by `PatchControlId`:

- MIXER `Navigate` and `Adjust` retain their existing resolver and wrap/clamp semantics.
- PATCH `Navigate` accepts only Up/Down and changes only `InteractionState.patchControlFocus`.
- PATCH `Adjust` on Engine accepts only Left/Right and enters the existing asynchronous selection path.
- PATCH `Adjust` on an envelope parameter uses Left/Right for fine decrement/increment and Down/Up for coarse decrement/increment.

The existing envelope mutation will be extracted into one reducer-owned operation addressed by stable Patch identity/index plus `VoiceEnvelopeParameter`. Both the MIXER `PatchEditableTarget::Envelope` branch and PATCH call it. It reads the same descriptor, applies the same adjustment and `ParameterAtBoundary` behavior, calls the same `VoiceEnvelope::with_value`, and changes exactly one canonical Patch field. No view, keyboard adapter, or demo owns adjustment arithmetic.

An Engine adjustment while Preparing or Activating remains `StructuralEditBusy`. ADSR adjustments are not structural and remain available in every lifecycle state. Edit+direction on an ADSR row never falls through to engine selection.

### 3. Separate focus from action availability in the PATCH projection

`PatchPageProjection` will add root `focusedControlId`. Each envelope row will add `controlId` derived from its `VoiceEnvelopeParameter` and change `editable` to true. Existing semantic `id`, label, value, bounds, fine/coarse steps, and unit continue to come from the envelope descriptor. Capability-provided rows remain read-only.

`engine.editable` continues to mean that a new structural choice can be requested: focused Patch exists, at least two registry choices exist, and status is Ready or Failed. It no longer doubles as selection state. The text renderer compares row control IDs with `focusedControlId`, emits one `>` marker, and calculates `selectedLine` from that match. Thus Engine stays visibly focused while temporarily disabled in Preparing or Activating.

The new `focusedControlId` and envelope `controlId` leaves intentionally change the versioned observation surface. The applicable StateTree/observation schema versions and the Patch-page production-owned serialized-leaf descriptor will advance together; exact-schema tests will reject either half changing alone. There is no persisted session migration in this increment.

### 4. Keep ADSR changes on the existing latest-scalar transport

Accepted focus movement follows the normal one-way dispatch and coherent projection path, so generation advances and the complete fixed snapshot is republished with identical values and graph revision. It emits no `AudioCommand` or `EngineSelectionEffect` and produces sample-identical audio.

Accepted ADSR adjustment commits the canonical value and publishes the complete fixed `ParameterSnapshot`; it emits no discrete command or structural effect. The renderer and both engines already consume that slot through the established per-voice contract.

Engine lifecycle determines which immutable graph revision a scalar snapshot targets:

| Lifecycle | Snapshot revision after ADSR edit | Observable behavior |
| --- | --- | --- |
| Ready | Active revision | Active graph consumes the newest value. |
| Failed | Preserved source revision | Preserved source graph consumes the newest value; no fallback exists. |
| Preparing | Source revision | Source remains audible with the edit; after candidate commit, the prepared graph's initial parameters are refreshed from the latest canonical target snapshot before publication. |
| Activating | Target revision | The staged/published replacement receives the newest value and starts exact; the still-active source continues with its last compatible source-revision snapshot until block-boundary swap. |

No scalar is relabeled across revisions and no second queue is introduced. Candidate refresh remains on control ownership. The hard real-time callback keeps its existing bounded, allocation-free, lock-free, nonblocking, no-I/O, no-log, no-panic, no-destruction behavior.

### 5. Route existing coverage through PATCH instead of inventing coverage

The production editable resolver remains the unique universe for mixer, envelope, engine Scalar, and global parameter instances. The focused first Patch's four envelope identifiers will be exercised through PATCH; the remaining instances keep their MIXER route. Coverage still contains each stable identifier exactly once.

Existing acceptance targets become stricter:

- `patch_page_projection` proves the five-control order, nonwrapping navigation, exact marker/selected line, fine/coarse edits, boundaries, lifecycle visibility, and empty audio/structural effects.
- `per_voice_envelope` proves both MIXER and PATCH resolve the same four fields and that PATCH edits produce target-only measurable per-voice consequences in real SoundFont and Braids renderers.
- `exhaustive_demo_scene` uses normalized keyboard input for all four PATCH ADSR controls, edits during Preparing and Activating, checks exact revision behavior, and preserves deterministic two-run evidence.
- `live_demo_scene` visibly exercises the focused Patch's four frozen ADSR instances through PATCH for at least 500 ms each, then returns focus to Engine for the existing two-direction structural proof.
- schema-surface and egui-context checks cover the new stable identities, leaves, focus marker, and scroll target.

The existing `CREST_ENVELOPE_OBSERVATION` and `CREST_OBSERVATION` records gain measured predicates declared by the reconciled CUE witness model. Success markers remain insufficient without those structured values.

### 6. Keep physical demo startup and progress bounded

The CPAL adapter will accept an already valid preferred-rate default output configuration before asking the backend to enumerate optional supported ranges. If enumeration fails but the reported default remains valid, negotiation uses that exact default; it does not invent a format, change devices, or weaken graph preparation against the actual configuration.

`LiveDemoRunner` will track only meaningful autonomous milestones: a scene dispatch, exact-generation checkpoint capture, engine lifecycle advance, or cleanup completion. Ten seconds without one of those milestones and 120 seconds total are typed failures. The existing standalone fatal-tick path then closes the disposable window, performs semantic cleanup, releases the stream, shuts down graph ownership off callback, and exits nonzero without emitting a completed report. The binary prints one concise startup status so an input-isolated autonomous run is not mistaken for an interactive freeze.

Fixture time is intentionally frozen while a parameter checkpoint awaits its exact accepted generation; therefore a sparse or silent Corridors passage cannot be the checkpoint's only signal source. The frozen scene brackets every accepted scalar checkpoint with a Patch-targeted semantic NoteOn before the edit and its matching NoteOff after capture and dwell. Patch parameters target their owning Patch and global parameters use the focused first Patch. These events travel through `AppLoop` and the normal discrete command boundary, never earn scalar coverage, and keep the deadline meaningful: failure to observe the planned probe is a real production-path stall rather than ordinary song timing.

## Risks / Trade-offs

- **Observation compatibility:** adding focus leaves breaks exact JSON consumers. Mitigation: bump the applicable schema version and leaf descriptor atomically and make stale versions fail tests.
- **Focus/editability conflation:** the current engine marker is based on editability. Mitigation: introduce one explicit projected focus identity and test Engine while disabled.
- **Duplicated adjustment behavior:** a second PATCH-specific numeric implementation could drift from MIXER. Mitigation: both contexts call one canonical envelope adjustment operation and compare identical-state results.
- **Revision timing during activation:** the old source cannot consume a target-revision edit after candidate commit. Mitigation: make that boundary explicit, require the replacement to start with the latest value, and verify source continuity plus target exactness.
- **Demo duplication and duration:** replaying focused ADSR in both contexts would inflate coverage and live time. Mitigation: route the same four frozen identifiers through PATCH exactly once and skip their old MIXER steps.
- **Physical-device metadata and stalled progress:** optional CoreAudio enumeration may fail despite a valid default, sparse fixture timing can strand an exact-generation scalar predicate while fixture advancement is frozen, and a silent callback/worker can otherwise leave a checkpoint pending forever. Mitigation: prefer the already valid default, bracket scalar edits with bounded semantic target probes, retain typed errors, and enforce stage and total deadlines through the existing teardown owner.
- **Scope pressure toward engine parameters or presets:** the focus mechanism could technically admit more rows. Mitigation: `PatchControlId` is closed to Engine plus canonical ADSR for this increment; capability rows stay projected read-only and preset discovery remains the next roadmap change.
