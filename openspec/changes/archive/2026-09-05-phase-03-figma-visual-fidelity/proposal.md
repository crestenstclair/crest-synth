## Closure scope — 2026-09-05

Closed at the user's request before starting another OpenSpec change. The
accepted scope is the functional Sample slice: default test asset and MIDI,
Detail waveform and editing, the shared in-app file page, nested navigation,
validated import, cancellation/stale-result safety, and saved-session waveform
restoration. Resize acceptance remains closed. Figma guides hierarchy and
interaction; exact pixels, typography, spacing, and exhaustive visual comparison
are not completion gates. Cloud downloading remains the storage provider's job.

The original checklist records 24 completed tasks and 36 unchecked tasks.
Unchecked items in sections 2, 4, 5, 6, 7, and 8 retain their historical status:
they cover broader visual work and aggregate validation that were not completed
as specified. They are deferred from this closed functional scope, not reported
as implemented or proven. Existing checks are recorded below and in DESIGN.md;
this closure performs documentation validation, not another product acceptance
run. SoundFont bank loading remains unimplemented and belongs to a later change.

The four delta specs are reconciled with the functionality-first acceptance
policy and synchronized to the main specs before archival. Main specs describe
behavioral contracts; synchronization is not evidence that every broader visual
requirement has already been implemented. The historical plan below explains the
original work, and this closure scope governs any conflicting completion gate.

## Why

The production UI is still a functional blockout: current structural and reducer-path evidence does not establish that the remaining Sample, Mixer, and option surfaces faithfully realize the normative Figma compositions. With the session lifecycle and implicit empty-Patch workflows complete, visual acceptance can now cover the intended production workflows rather than obsolete fixture-only states.

## What Changes

The current authorized handoff is the complete functional Sample slice. It
includes a controller-navigable in-app file page on Sample File, safe library import, nested
library browsing, waveform restoration with saved sessions, and an audible
test MIDI pattern at normal launch with an explicit start/stop action. Sample
is available without environment setup through a bundled test sample copied
into the per-user library. Explicit environment overrides remain validated.
The in-app browser supersedes the rejected native WAV dialog. Shared file and
folder identities, rows, filtering, and presentation support WAV and SF2
clients; new SoundFont bank loading and preset-catalog replacement remain a
separate engine integration, not a reason to build a second picker.
These additions supersede the earlier restriction to Detail presentation.
Figma guides
hierarchy and interaction; exact typography, spacing, or pixel matching is not
a completion gate. Resize is already accepted and its manual handoff must not
be repeated. Existing Engine selection, file browsing, waveform, and editing
paths must work before expanding to the remaining surfaces below.

- Complete the authored Sample Detail and Sample Browser compositions while retaining the canonical Sample asset, browser, preview, focus, and return semantics.
- Replace the Mixer blockout with the authored responsive sixteen-track Mixer and correlated Inspector composition.
- Close the remaining Engine Options and Post FX Options hierarchy, typography, spacing, state-treatment, and action-guidance gaps against their live Figma frames.
- Refine the shared native shell vocabulary—scaling, typography, spacing, hierarchy, focus, status, and control-state presentation—without making viewport or density a product-state input.
- Add production-native visual comparison evidence for every named surface, including live resize exploration across varied widths, representative height and text-scale conditions, deterministic repaint, bounded reachability, and direct comparison of hierarchy and responsive relationships with the relevant live Figma frames.
- Require status and control state to remain understandable through text or shape as well as color, and require resize-only runs to prove unchanged semantic focus, return identity, projected content, and reducer generation.
- Treat Figma frames as authored composition and interaction references, not fixed-resolution pages: do not reproduce frame coordinates, introduce viewport-specific markup, require every width to match a snapshot, or use pixel-difference scores as visual acceptance.
- Update `DESIGN.md` only after measured production-path evidence establishes the exact as-built completion and any remaining discrepancies.
- Beyond the authorized Sample completion above, do not add product workflows, capabilities, instruments, effects, devices, fixed fixture counts, or multi-platform release work.

## Capabilities

### New Capabilities

- `sample-visual-composition`: Defines the normative Sample Detail and Sample Browser hierarchy, responsive composition, state treatment, and native Figma-comparison evidence while preserving existing Sample behavior.
- `mixer-visual-composition`: Defines the responsive sixteen-track Mixer and correlated Inspector composition, explicit control and meter states, focus stability, and native Figma-comparison evidence.

### Modified Capabilities

- `engine-post-fx-option-states`: Strengthens visual acceptance from structural correspondence to responsive native fidelity for Engine and Post FX Options without changing their reducer-owned workflows.
- `responsive-shell-composition`: Extends presentation-only responsive acceptance beyond named snapshots through fluid resize exploration and defines shared native scaling and visual-fidelity evidence for the Phase 03 surfaces.

## Impact

- Affects the Rust-owned visual tokens and responsive projection facts only where authored presentation cannot be expressed from the existing immutable semantic document.
- Affects the committed `webview-page` HTML/CSS/JavaScript compositions for Sample Detail, Sample Browser, Mixer, Inspector, Engine Options, Post FX Options, and shared shell chrome.
- Extends production reducer/projector/serialization fixtures, native WKWebView responsive-behavior witnesses, representative captures, direct live-Figma composition reviews, and visual regression evidence.
- Preserves `AppState::apply` as the only product-state mutation path, canonical semantic focus and return identity, registry-driven production content, separate real-time transports, and the hard real-time callback contract.
- Adds no external runtime dependency, no public product capability, and no breaking session or interaction change.
