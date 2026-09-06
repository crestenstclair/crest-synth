## Context

See proposal.md. Existing parsing already produces separate control preset metadata and callback-safe numeric banks. The fixed descriptor and preparer are the obstruction: preset choices are global despite each Patch storing its own asset reference. `DESIGN.md` remains the as-built authority.

## Goals / Non-Goals

**Goals:** Reuse the shared browser, immutable descriptors, provider validation, and complete-graph activation. Keep file I/O, parsing, metadata, and retirement off the callback and window tick. Validate each Patch against its exact bank.

**Non-Goals:** Cloud hydration, SoundFont format conversion, new synthesis semantics, native dialogs, gamepad-driver integration, resize acceptance, and pixel matching.

## Decisions

1. Extend the immutable capability registry with asset-scoped descriptors. Installed engine entries stay stable; active config lookup resolves its asset-specific choices. Worker-produced descriptors enter product state only through the reducer. A global descriptor replacement would corrupt other Patches and is rejected.
2. Reuse the current correlated file import worker and directory adapter. SoundFonts use a per-user library under `Music/Crest Synth/SoundFonts`; external selections validate before copying, never overwrite, and save library-relative references. Legacy `./sf2/HiDef.sf2` remains the explicit bundled default.
3. Share prepared numeric banks through worker-side asset ownership while keeping preset names and paths out of callback objects. Existing numeric SoundFont rendering remains unchanged.
4. Asset replacement uses the target descriptor's initial structural choices; normal preset edits preserve the selected bank. Candidate-delta validation names and enforces this behavior at reducer and worker boundaries. Missing saved presets fail rather than selecting another preset.
5. Session preparation resolves asset descriptors before validating and preparing its private candidate. The active session changes only after graph acknowledgement. No new saved-session version or serialized PCM/catalog cache is needed.

## Risks / Trade-offs

- Large or malformed SF2 files can consume substantial preparation resources. Bound source admission, reuse the existing parser/region validation, and share active banks without retaining every visited bank indefinitely.
- Dynamic metadata must not become mutable global product state. Carry immutable asset descriptors with canonical state and preparation requests; workers own only resource caches.
- Existing Sample-only names in shared import plumbing must be generalized where SoundFont now shares ownership, without changing Sample decoding or audition behavior.

## Migration Plan

Preserve existing capability IDs and bundled references. Advance projection schema for added asset-dependent metadata. Saved sessions continue storing only asset references and parameter values; unsupported or unavailable banks produce explicit failures. Imported copies are user assets and are never deleted as part of rollback.
