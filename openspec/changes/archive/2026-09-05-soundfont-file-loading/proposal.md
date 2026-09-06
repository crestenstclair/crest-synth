## Why

The shared file page can filter SF2 files, but the SoundFont engine still binds every Patch to the bundled HiDef bank. Users need to select their own local banks, hear them, and retain the selection in saved sessions.

## What Changes

- Use the existing controller-navigable file page for SoundFont File, with SF2 filtering and validated library import.
- Resolve preset choices and numeric audio data per selected bank, preserving independent banks across Patches.
- Start an explicitly selected new bank at its first playable preset; preserve exact later preset selections.
- Restore imported bank references and preset catalogs off-thread when opening sessions.
- Preserve Sample workflows, test MIDI, failure/cancellation behavior, and graph-acknowledgement ownership.

## Capabilities

### New Capabilities
- `soundfont-file-loading`: In-app bank selection, independent preset catalogs, audible playback, and saved-session restore.

### Modified Capabilities

None.

## Impact

SoundFont adapters, asset-dependent capability metadata, shared file browser, structural preparation, and session restore. No new audio dependency, cloud-download integration, native asset dialog, or visual-fidelity gate. Existing saved capability identity and bundled asset references remain compatible.
