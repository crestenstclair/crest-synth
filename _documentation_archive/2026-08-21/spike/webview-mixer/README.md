# Webview mixer spike

**Op:** `01KZ9CN8E0YEMQEZQG8Q7VEESM` (architect-alphonso dispatch)
**Question:** does a webview fed by the existing serialized
`SemanticGraphicalViewModel` reproduce the authored MIXER screen with
drastically less layout effort than the hand-painted egui visual layer?

## What this is

- `view-model.json` — the exact JSON the production projector emits for the
  production fixture with the MIXER context selected. Produced by
  `tests/spike_webview_view_model_dump.rs` (run with
  `cargo test --test spike_webview_view_model_dump -- --ignored`).
  No second schema was invented; `SemanticGraphicalViewModel` already
  derives `Serialize` and DESIGN.md already declares the UI renders an
  immutable host-neutral view model.
- `index.html` — **244 lines**, one file: authored color/spacing/radius/type
  tokens as CSS custom properties (values verbatim from
  `src/shell/visual/token.rs`), the authored Azeret Mono faces, and the full
  MIXER screen — context bar, identity header, sixteen-column strip bank,
  focused-column keyline + halo, per-column fader/hex/pan line, persistent
  Inspector, and both hint rows — rendered from the JSON.
- `webview-render.jpg` / `side-by-side.png` — the render against the
  authored Figma design.

Serve with any static server (`python3 -m http.server` in this directory);
`file://` also works in a real wry/Tauri webview, which uses the same engine.

## Findings

1. **The sixteen-column layout is one CSS rule.**
   `grid-template-columns: repeat(16, minmax(48px, 82px))` plus a 4px gap
   replaces the fader-grid arithmetic of `ViewportDensityPolicy`
   (`density.rs`, 684 lines) for this surface. The compact-viewport floor is
   the `minmax` lower bound.
2. **The authored design carried over almost mechanically.** Tokens are the
   Figma variables as custom properties; the specimen's constants (82/86
   column/pitch, 14px track, 8px fill, 3px shoulder, 34×6 cap, keyline +
   halo focus) are each one declaration. First render was already correct;
   the only fix in the whole spike was filtering a null hint key in the
   footer.
3. **The egui equivalent of this screen** is spread across
   `application_shell.rs` (2276), `mixer_strip_bank.rs` (1196), `fader.rs`
   (789), plus the density policy (684) and shares of primitives — versus
   244 lines here. The token/typeface vocabulary itself stays in Rust and
   remains the single source; the CSS custom-property block would be
   generated from it in a real integration, not hand-copied.
4. **The view-model boundary held with zero changes.** Everything on screen
   — labels, hex values, focus, status word, footer hints — came from the
   serialized model. Hex readouts are `(value − min)/(max − min) × 127`;
   valid-action hints render the footer.

## What the spike deliberately does not prove

- Input: no semantic actions are emitted back (the return path would be a
  webview IPC bridge carrying the same `SemanticAction` values).
- Meters: no 30 Hz meter traffic over IPC was measured.
- Packaging: no wry/Tauri shell was built; Chrome stands in for the same
  WebKit/Blink engine class.

These are the risks the pivot mission must burn down next; layout cost —
the pain that motivated the pivot — is answered.
