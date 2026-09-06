## Why

The production synth has the required pages and stable focus machinery, but its physical page bindings do not yet implement the authored Page Layout graph. Phase 04 will make Patch Overview the default hub, connect it to highlighted Detail, Mixer, and temporary MIDI Settings, and prove those transitions through the existing production path.

## What Changes

- Follow Figma page `05 · Page Layout`, frame [Crest Synth · Page Layout — 153:184](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=153-184), inspected during proposal preparation.
- Keep Q/E as previous/next Patch navigation landing in Patch Overview, including the existing non-wrapping trailing empty position and stable Patch identities.
- Map Shift+Up from Overview to its highlighted Instrument/occupied effect Detail, and from Mixer to remembered Patch Overview.
- Map Shift+Down from Detail to its exact Overview origin, and from Overview to remembered Mixer.
- **BREAKING:** Shift+Left from Patch Overview opens Settings · MIDI Devices instead of selecting the previous Patch. Q becomes the authored previous-Patch input, including return from the trailing empty position.
- Keep page actions distinct from in-page focus movement and editing. Normalize physical arrow keys and existing WASD directional equivalents before the reducer; update projected guidance and retain non-conflicting existing shortcuts as compatibility behavior.
- Reuse temporary Settings suspension and deterministic focus repair. The user selected Shift+Right on 2026-09-06, now authored in Page Layout `153:184` at `155:208` and `157:200`. It restores the suspended PATCH/MIXER page and exact valid focus or the existing nearest-enabled repair. Shift+Down remains a compatibility escape.
- Add deterministic edge coverage and a bounded native keyboard journey for page identity, subject, focus, return, and session/audio neutrality. Completion requires every authored edge, including Shift+Right from Settings, to be implemented and verified.

## Capabilities

### New Capabilities

- `page-navigation`: The authored page hierarchy, physical-to-semantic page inputs, remembered roots, navigation isolation, and production acceptance.

### Modified Capabilities

- `midi-device-settings`: Align primary Settings entry with Shift+Left from Overview and return with Shift+Right; preserve suspended performance identity and deterministic repair.
- `empty-patch-implicit-creation`: Use Q/E in navigation scenarios so returning from the trailing empty position does not conflict with the new Settings chord. Empty-position semantics and the 16-Patch capacity remain unchanged.

## Impact

Keyboard/native input normalization, the host-neutral controller translator, semantic action/event admission, reducer-owned interaction transitions, projected valid-action hints, and focused navigation/native tests. Reuse the current contexts, stable focus paths, subordinate sessions, Settings suspension, and repair policy; introduce no parallel navigation state or audio engine path.

Phase 03 and SoundFont loading are archived and synced. Phase 03 closed with accepted functional scope and explicit visual deferrals; this dependency does not reopen its resize acceptance or require its deferred pixel-fidelity work. Figma is authoritative for hierarchy, labels, connections, and input meanings; functionality and readable, truthful navigation determine acceptance. In-page editing redesign, physical gamepad integration, multi-select, platform releases, and new instrument/effect capabilities are outside this change. No implementation or Figma edits are authorized by this proposal workflow.
