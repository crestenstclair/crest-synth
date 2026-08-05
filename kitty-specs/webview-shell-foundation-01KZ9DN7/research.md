# Research: Webview Shell Foundation

## R-01 — Can eframe and Tauri v2 coexist in one binary?

**Decision**: Yes, with launch-time exclusivity. The shell is selected before
either event loop starts; exactly one of `eframe::run_native` (winit) or
`tauri::Builder::run` (tao) ever runs. They never share a thread or a frame.

**Rationale**: eframe's only entry point creates its own winit event loop and
one event loop may exist per thread ([egui#2875](https://github.com/emilk/egui/issues/2875),
[egui#1489](https://github.com/emilk/egui/discussions/1489)). Concurrent
windows from both stacks are therefore impossible — but the mission never
needs them: shell selection is an explicit launch-time decision
(`requirement.webview_projection_shell`), so the unselected stack is dead
code for that process. cpal owns its stream on its own audio thread and is
independent of either event loop; the existing stream/worker ownership and
shutdown model transfers unchanged.

**Alternatives considered**: `tauri-egui` (egui inside a Tauri window via
glutin — obsolete, wrong direction: it embeds egui in Tauri rather than
keeping two peer shells); separate binaries per shell (splits the composition
root and the demo/proof harness for no benefit).

**Residual risk**: binary size and dependency-tree weight of carrying both
stacks; measured, not blocking. Retirement of the egui stack in the successor
mission removes it.

## R-02 — Rust-side keyboard capture under a WKWebView

**Decision**: Capture keys natively before the webview interprets them.
Primary path: tao/Tauri window-level key events (`on_window_event`). Known
fallback if the focused WKWebView swallows events before tao surfaces them
(macOS first-responder routing): an `NSEvent`
`addLocalMonitorForEventsMatchingMask` local monitor — in-process, Rust/native
side, sees every key event before dispatch regardless of first responder.
Either way the events feed the existing `KeyboardInputTranslator`; the page
registers no key handler (crest-spec adapter rule).

**Rationale**: FR-003 and `requirement.webview_projection_shell` require
input ownership on the Rust side so the two shells cannot drift in
vocabulary. A DOM-side handler would re-create the translator in JS.
Web-search evidence on tao's keyboard surfacing under WKWebView is
inconclusive ([tauri#5464](https://github.com/tauri-apps/tauri/issues/5464)
documents webview key-event quirks), so this is the mission's riskiest
mechanism and is burned down first (IC-01) with a disposable probe before any
dependent work.

**Alternatives considered**: `tauri-plugin-global-shortcut` (system-global,
wrong scope — captures keys when the app is not focused); DOM keydown
forwarding over IPC (violates the declared adapter rule; only reconsidered by
returning to `/spec-kitty.crest-spec` if both native paths fail).

## R-03 — Serialized projection transport

**Decision**: Push-based: Rust serializes the accepted
`GraphicalShellProjection`'s embedded `SemanticGraphicalViewModel` with the
existing `serde_json` derives and emits it to the page over Tauri's event
channel (`Emitter::emit`); the page listens and repaints. Meters are a second
channel at 30 Hz carrying the latest `AudioObservationSnapshot`, coalesced —
never queued — on the Rust side.

**Rationale**: the spike proved the serialization is already page-sufficient
(84,587 bytes for the production MIXER fixture; well under IPC budgets at
interactive rates). Latest-value coalescing preserves the DESIGN.md meter
transport semantics (`snapshots, decimated; UI polls` → push at the same
decimation is equivalent loss-tolerant traffic). One schema, no page DTO
(`requirement.serialized_projection_transport`).

**Alternatives considered**: page-side polling via `invoke` (adds a request
path for no gain); WebSocket sidecar (second server surface, pointless
in-process); binary encoding (premature — 85 KB JSON at ≤60 Hz is ~5 MB/s
worst case, and NFR-001/NFR-002 will measure actual cost).

## R-04 — Token-table generation

**Decision**: a `build.rs`-invoked generator (or explicit `cargo` bin run by
the build) emits `webview-page/tokens.css` from the authored Rust vocabulary
(`SemanticVisualToken`, `TypeStyle`, `SpacingStep`, `Radius`, keyline/halo
constants), one custom property per canonical authored name. An acceptance
check regenerates and diffs, failing on drift (declared in
`WebviewProjectionShellAcceptanceTests`).

**Rationale**: keeps the Rust vocabulary the single source
(`goal.build_from_component_vocabulary`) while the page consumes plain CSS;
freshness is proven, not trusted.

**Alternatives considered**: hand-copied CSS (the spike did this; forbidden
for the product by the crest-spec adapter rule); runtime injection of tokens
over IPC (couples page boot to a round-trip and hides drift until runtime).

## R-05 — Page determinism proof without a browser in CI

**Decision**: two-layer proof. Layer 1 (headless, deterministic, in the
declared validation): the page's render logic is a pure
`document → DOM-structure` function; prove it by executing the page's JS
against recorded view-model documents in a DOM-less harness (the render
function emits a declared observation — structure + values, not pixels — that
is asserted stable and correct at both authored viewports). Layer 2 (live):
the paced live demo through the real Tauri window, per
`success criterion 3` and the existing live-demo discipline.

**Rationale**: matches the existing proof posture (production reducer and
render path, falsifiable observations, no golden-pixel fragility across
webview engine versions) — `requirement.graphical_shell_behavioral_proof`.

**Alternatives considered**: WebDriver/pixel goldens (engine-version
fragility, heavy CI); proving only the live path (loses determinism and
two-run comparability).
