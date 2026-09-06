## Purpose

Defines functional Sample startup, file selection, waveform and editing presentation, and responsive controller operation, with Figma as a guide to hierarchy and interaction.

## ADDED Requirements

### Requirement: Sample works without manual launch configuration
Normal launch SHALL install Sample with an explicit bundled test WAV in a writable per-user library and start a bounded repeating test MIDI pattern through production channel fan-out. The user SHALL be able to stop and restart that pattern. Browser audition and session replacement SHALL suspend test notes. Explicit configured assets SHALL be validated without substitution.

#### Scenario: Launch and hear Sample
- **WHEN** the app starts without Sample environment variables
- **THEN** the Sample patch, waveform, and test MIDI controls SHALL be available and audio rendering SHALL produce nonzero output from the assigned sample

### Requirement: In-app file selection preserves control and asset ownership
Return on Sample File SHALL enter the shared in-app FileBrowser page. Navigate, Activate, Return, and Sample preview SHALL use the same semantic controls as the rest of the synth, with no platform asset dialog or pointer requirement. Shared file/folder identities, rows, and file-kind filters SHALL support WAV and SF2 clients; audio decoding and SoundFont bank/preset integration SHALL remain engine responsibilities. The browser SHALL expose library, Home, and available volume locations. An external WAV SHALL be validated and copied into the library without overwriting another asset; saved state SHALL contain only its stable relative identity. Cancellation, failure, and stale results SHALL preserve the acknowledged assignment and stable focus. Accepted choices SHALL use the existing structural preparation and acknowledgement path.

#### Scenario: Pick and reopen an external WAV
- **WHEN** the user chooses a supported external WAV and later saves and opens the session
- **THEN** Sample audio, waveform, and edited playback/loop landmarks SHALL refer to the imported asset without depending on its original external location

#### Scenario: Browse a nested folder
- **WHEN** the user enters a library subfolder or returns to its parent
- **THEN** its real contents SHALL load off the audio callback and window tick, with stable row focus and explicit failures

#### Scenario: Cancel or reject a file
- **WHEN** in-app selection is cancelled, decoding/import fails, or its correlated import has been superseded by session replacement
- **THEN** no unrelated Patch or saved asset SHALL change and no stale result SHALL activate a graph

#### Scenario: Dropbox audio has not been downloaded
- **WHEN** a selected or listed WAV is a zero-byte Dropbox placeholder
- **THEN** the browser and failed import SHALL report that the file needs downloading, with Make available offline guidance, rather than invalid audio; the current assignment SHALL remain unchanged and selection SHALL be retryable after downloading
- **AND** an empty local file without a provider marker SHALL report that it contains zero bytes, while a downloaded WAV SHALL be decoded even if provider metadata remains
- **AND** downloading cloud content SHALL remain the external storage provider’s responsibility; this workflow SHALL NOT require cloud-download integration

### Requirement: Sample visual content remains projection-owned
Sample Detail and Sample Browser SHALL paint only the canonical Sample subject, asset, browser, preview, focus, lifecycle, return, visualization, and action facts supplied by the immutable semantic projection. The renderer MUST NOT derive product state from DOM position, file labels, viewport geometry, or Figma fixture values.

#### Scenario: Registry and library content differ from Figma fixtures
- **WHEN** an installed Sample capability or asset library exposes labels, paths, metadata, control counts, or file counts different from the Figma examples
- **THEN** the authored composition SHALL arrange the exact projected production content without inventing, omitting, renaming, or substituting entries

#### Scenario: Repaint does not create a second Sample model
- **WHEN** the same Sample Detail or Sample Browser projection is painted repeatedly
- **THEN** asset identity, browser order, preview state, focus, return origin, valid actions, and generation SHALL remain identical and no renderer-owned Sample state SHALL persist between paints

### Requirement: Sample Detail follows the authored hierarchy
Sample Detail SHALL reproduce the hierarchy and interaction emphasis of live Figma node `39:92`: explicit Patch and Sample subject identity, selected asset and lifecycle status, projected waveform and landmark presentation, ordered editable Sample controls, persistent PATCH Utility, and one footer action guide. Visualizations SHALL remain non-focusable, and all control identities, values, units, ranges, and state readings SHALL come from the projection.

#### Scenario: Ready Sample asset
- **WHEN** Sample Detail renders an acknowledged ready asset
- **THEN** the native surface SHALL visibly correlate the Patch, Sample subject, asset, waveform and landmarks, ordered controls, single focused control, Utility, breadcrumb, and admitted actions in the authored hierarchy

#### Scenario: Selecting Sample as Engine
- **WHEN** Engine selection prepares and activates Sample with its configured default asset
- **THEN** activation acknowledgement SHALL retain the prepared waveform for Detail without requiring a subsequent browser assignment

#### Scenario: Sample File admits browsing
- **WHEN** an asset row admits the canonical related-surface action
- **THEN** it SHALL expose browsing and SHALL NOT be presented as read-only or disabled merely because it is not scalar-editable

#### Scenario: Requested and active asset states differ
- **WHEN** a Sample asset or structural value is loading, validating, preparing, activating, unavailable, or failed
- **THEN** acknowledged and requested readings, the exact lifecycle word, and any typed cause SHALL remain distinct and visible without replacing the acknowledged waveform, asset, or configuration with fallback content

#### Scenario: Waveform data is unavailable
- **WHEN** the projection has no compatible waveform or landmark visualization
- **THEN** Sample Detail SHALL show an explicit unavailable or stale text-and-shape treatment in the authored visualization region and SHALL NOT fabricate a flat waveform or implied landmark value

### Requirement: Sample Browser follows the authored hierarchy
Sample Browser SHALL reproduce the hierarchy and interaction emphasis of live Figma node `41:138`: library and current-folder identity, ordered projected browser rows, selected and focused row state, asset metadata, waveform or explicit unavailable state, preview status and playhead, and one footer action guide. The browser SHALL retain its canonical stable row identities and exact return origin.

#### Scenario: Browsing a populated folder
- **WHEN** Sample Browser projects parent, folder, file, and cancel rows for a populated location
- **THEN** the native surface SHALL paint them in canonical order with exactly one semantic focus, explicit row kind and availability, readable metadata, the correlated preview region, and only the projected actions

#### Scenario: Empty, loading, and failed browser states
- **WHEN** the current location is empty, loading, unavailable, or failed
- **THEN** the browser SHALL preserve its authored regions and show the exact state and typed cause through text and shape rather than hiding the list, manufacturing an entry, or silently navigating elsewhere

#### Scenario: Hold-to-preview state is visible
- **WHEN** Start press and release drive the existing preview workflow for a focused sample row
- **THEN** preparing, held, playing, stopping, stopped, stale, and failed states that occur SHALL be correlated to that row and rendered explicitly without committing the previewed asset or moving semantic focus

### Requirement: Sample composition is fluid and presentation-only
Sample Detail and Sample Browser SHALL use one semantic DOM per surface across varied constrained, intermediate, and expanded native widths and representative height and text-scale conditions. Intrinsic sizing, flexible tracks, wrapping, clamping, and bounded scrolling SHALL adapt the authored hierarchy without viewport-specific markup or fixed frame coordinates. Reflow SHALL preserve exactly one semantic focus and the same subject, browser location, row order, return origin, preview correlation, projected actions, and product generation.

#### Scenario: Fluid resize without input
- **WHEN** one serialized Sample Detail or Sample Browser document is resized repeatedly through varied widths without physical or semantic input
- **THEN** every observed paint SHALL retain the same semantic document and no resize-derived action or `AppState::apply` invocation SHALL occur

#### Scenario: Compact and enlarged content remains reachable
- **WHEN** Compact composition or enlarged text renders maximum acceptance content and long labels
- **THEN** the Sample workspace and persistent Utility SHALL stack in semantic order, every interactive target SHALL retain the canonical minimum target size, and first and last required content SHALL remain reachable without required-content overlap or document-level horizontal overflow

### Requirement: Sample functionality is accepted through native review
Sample completion SHALL be established through the production reducer, projector, serialization channel, committed renderer, and native WKWebView paint. Figma nodes `39:92`, `41:138`, and responsive contract `98:2` guide hierarchy and interaction; exact typography, spacing, and pixel correspondence are not completion gates. Component presence, hidden-DOM output, reducer tests, or planning artifacts MUST NOT be treated as native workflow acceptance. Previously accepted resize behavior SHALL NOT require a repeated manual handoff for Sample Detail.

#### Scenario: Representative native comparisons
- **WHEN** normative Sample Detail and Sample Browser states are captured at representative native conditions
- **THEN** native review SHALL verify readable identity, waveform, controls, focus, values, status, and admitted action guidance using Figma as a hierarchy guide, and SHALL identify remaining functional defects

#### Scenario: Responsive native evidence
- **WHEN** Sample surfaces are resized fluidly through varied widths and exercised with representative constrained, intermediate, expanded, and enlarged-text conditions
- **THEN** evidence SHALL record the observed responsive compositions, native scale, minimum-target compliance, singular focus identity, overflow and overlap results, scroll endpoints, paint acknowledgement, and unchanged semantic generation

#### Scenario: Completion claim is bounded by evidence
- **WHEN** a Sample workflow has an unresolved functional defect or lacks production-path evidence
- **THEN** that workflow SHALL remain reported as incomplete rather than inferring functionality from fixture presence or visual appearance
