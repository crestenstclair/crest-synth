# Phase 7 Figma reference review — 2026-08-20

Authority remains the live [Crest Synth controller-first Figma file](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=0-1). The checked-in PNGs named below are reference captures used to make review repeatable; they do not replace the live file or `DESIGN.md`.

| Live node | Checked-in capture | Phase 7 evidence retained |
| --- | --- | --- |
| [Instrument Detail 37:7](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=37-7) | `assets/instrument-detail.png` | Shared shell, descriptor-owned title/sections/ranges/units/dependencies, non-focusable visualization, persistent Utility, exact return cue. |
| [FX Detail 38:60](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=38-60) | `assets/fx-detail.png` | Same shell and adjustment grammar, slot identity, four horizontal bands, Utility correlation, return to exact effect row. |
| [Sample Detail 39:92](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=39-92) | `assets/sample-detail.png` | Asset row, start/play/loop/end waveform landmarks, non-focusable waveform, root/playback/loop and pinned ADSR grouping. |
| [Sample Browser 41:138](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=41-138) | `assets/sample-browser.png` | Parent/folder/file/cancel CLI rows, stable path context, right Preview region, metadata/waveform, trapped controller flow, exact Sample-origin return. |
| [Engine Options 48:173](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=48-173) | `assets/engine-options.png` | 920×680 list grammar, current marker, Up/Down focus, Edit choose, Shift+Down close, visible origin. |
| [Post FX Options 48:207](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=48-207) | `assets/post-fx-options.png` | The same generic modal composition populated from a different installed registry. |
| [Interaction Map 49:3](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=49-3) | `assets/interaction-map.png` | PATCH/MIXER peer contexts, subordinate return relationships, focus/edit/choice grammar, and the product-owned Start-preview resolution. |

At 1920×1080, acceptance keeps the authored 48 px context line, 72 px identity header, 896 px workspace with 1500/420 task/Utility-or-Preview split, and 64 px footer. The compact 1280×800 policy is derived from those desktop frames and `DESIGN.md` minimums because Figma contains no authored compact frame.

Fixture boundary: the visible engine/effect lists, patch and sample names, file durations and sizes, parameter counts, labels, and numeric values demonstrate composition only. Production lists come from installed registries/catalogs and Sample behavior comes from `DESIGN.md`. In particular, the master design—not the Figma annotation—defines Start as hold-to-preview, release/focus/navigation/assign/cancel as stop, the admitted WAV contract, root pitch, voice count, loop modes, preparation limits, and no-fallback behavior.

## Implemented comparison and retained differences

The production document preserves the Figma shell anatomy from shared tokens: the desktop policy resolves the context/header/workspace/footer bands to 48/72/896/64 px and the workspace to the existing 1500/420 split. The option and browser surfaces share one `ModalOption` row renderer and the authored 920×680 outer bound. Instrument, effect, and Sample detail share one detail renderer; Utility remains the persistent side surface. Focus, current, adjustment, disabled, loading, invalid, unavailable, cancelled, and preview states each carry a text label or a distinct border/marker in addition to semantic color. No Phase 7 stylesheet color, pixel extent, or density split is authored outside the existing generated vocabulary.

The 1280×800 composition is intentionally derived, because the Figma file has no authored compact Phase 7 frame. Intrinsic modal bounds, minimum-target tokens, the Steam Deck 960/320 split, and the same stable semantic focus IDs perform reflow; resizing is paint-only and dispatches no density event.

Behavior-owned differences retained intentionally:

- installed registry/catalog entries and active values replace the example engine, effect, patch, and file fixtures shown in Figma;
- Start is a physical hold with a distinct key-up stop, and focus/navigation/assign/cancel also stop preview, as `DESIGN.md` requires;
- invalid, unavailable, preparing, cancelled, routing, mute, solo, and preview-release causes remain visibly explicit even where a static Figma specimen shows only the nominal state;
- the browser waveform/playhead is a bounded prepared summary and compatible latest observation, never decoded PCM or a UI animation pretending to be audio state.

## Native visual-gate status

The headless projection-shell run passed its serialized schema, token freshness, protocol parity, superseded-ack, and typed-startup sections (T022/T023/T010/T014/T025). It correctly reported the real DOM/geometry/window sections as skipped without `CREST_WEBVIEW_TESTS=1`.

The required native rerun with `CREST_WEBVIEW_TESTS=1` did **not** reach comparison capture: it timed out waiting for the production harness phase `animation-frame-ready` after the serialized sections passed. No browser session was available in this environment for an independent production-page capture. Therefore this note records the implemented source/token comparison and the exact environmental failure, but does not claim the 1920×1080 screenshot comparison, compact live geometry, native input-capture witness, or clean native window teardown as accepted.
