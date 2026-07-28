## Why

Phase 3 already renders the focused Patch's common per-voice ADSR values, but they remain read-only there, forcing the player back to the transitional MIXER surface to edit values that conceptually belong on PATCH. This increment makes those existing canonical controls operable in their intended context before the separate SoundFont preset-discovery increment.

## What Changes

- Expand reducer-owned PATCH focus from only Engine to the ordered, nonwrapping surface Engine → Attack → Decay → Sustain → Release, using the existing `VoiceEnvelopeParameter` identities for the four ADSR rows.
- Make bare Up/Down navigate that surface; keep bare Left/Right unavailable, keep Edit+Left/Right as engine selection only on Engine, and apply the existing fine/coarse numeric adjustment semantics on ADSR rows.
- Route PATCH ADSR edits through `AppState::apply`, the canonical `Patch::VoiceEnvelope`, and the existing fixed `ParameterSnapshot`; focus changes and ADSR edits emit no audio command or structural graph work.
- Keep ADSR focus and editing available through Ready, Preparing, Activating, and recoverable Failed engine-selection states, preserving revision-compatible scalar publication and refreshing a prepared candidate from the latest committed values before publication.
- Project one explicit focused control plus stable control identities for envelope rows, and render exactly one selected text line even when the focused engine action is temporarily disabled.
- Extend the Patch-page, envelope, exhaustive headless, and paced live proofs so all four focused-Patch ADSR instances are exercised through PATCH without duplicating schema-derived coverage or DSP behavior.
- Harden the bounded physical live proof so a valid preferred default device configuration does not depend on optional CoreAudio range enumeration, each scalar checkpoint receives a semantic Patch-targeted audio probe independent of sparse fixture timing, and genuinely stalled audio/engine milestones close with a typed deadline instead of waiting indefinitely.
- **BREAKING (versioned observation schema):** expand serialized PATCH focus/projection leaves and the typed control surface; observation schema versions and exact-schema fixtures must advance together.
- Leave capability-provided parameters, SoundFont preset/asset selection, sibling-Patch navigation, graphical UI replacement, effects, and modulation out of scope.

## Capabilities

### New Capabilities

None. This increment exposes editing through existing product capabilities and does not introduce a second envelope capability.

### Modified Capabilities

- `schema-driven-patch-page`: PATCH gains a reducer-owned Engine-plus-ADSR focus order, editable canonical envelope rows, and exact focused-row projection.
- `one-way-parameter-control`: PATCH navigation and ADSR adjustment become valid context-sensitive uses of the existing `Navigate` and `Adjust` events.
- `per-voice-envelope`: every ADSR field must be editable through PATCH while retaining the same canonical state, fixed scalar projection, and independent SoundFont/Braids per-voice behavior.
- `asynchronous-engine-selection`: engine lifecycle and PATCH ADSR focus/scalar edits must coexist with revision-correct preparation and activation.
- `observable-demo-scene`: deterministic coverage must exercise the four focused PATCH ADSR controls, scalar/structural coexistence, and exact causal evidence.
- `live-observable-demo`: the focused Patch's four frozen envelope coverage instances must run visibly through PATCH exactly once before engine-transition proof and cleanup.

## Impact

- Control domain: `PatchControlId`, `InteractionState`, `AppState`, shared envelope adjustment resolution, and rejection/boundary cases.
- Projection and shell: `PatchPageProjection`, `StateProjector`, `TextProjection`, serialized leaf descriptors/schema versions, and selected-line rendering.
- Orchestration: revision-compatible `AppLoop` scalar publication while engine preparation or activation is in flight; no new real-time transport or graph workflow.
- Verification: existing `patch_page_projection`, `per_voice_envelope`, `exhaustive_demo_scene`, `live_demo_scene`, schema-surface, egui-context, and project gates become stricter.
- Dependencies and persistence: no new crate, engine, asset, saved-state migration, or fallback path.
