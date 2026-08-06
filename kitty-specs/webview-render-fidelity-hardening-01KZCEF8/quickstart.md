# Quickstart: Webview Render Fidelity and Error-Path Hardening

## See the defect (before the fix)

```bash
make run
# Set any track level (e.g. hex 73 ≈ 90%): readout shows the value,
# fader fill paints empty — RISK-1 live.
```

## Run the affected proofs

```bash
cargo test --test webview_projection_shell -- --nocapture   # primary: policy parity, paint fidelity, forced throw, determinism, latency
cargo test --test component_vocabulary -- --nocapture       # style-literal scan (now incl. gallery.js/gallery.css)
cargo test --test component_composition -- --nocapture      # no-input-handler scan (now incl. gallery.js)
```

Each target must print its `CREST_ACCEPTANCE <target> passed` marker.

## Verify the fixes by hand

1. `make run` — fader fills and position indicators match their readouts at every level, including 0 and max.
2. Forced render throw (test-only seam in `tests/webview_projection_shell.rs`) — shell exits nonzero with the typed `PageRenderFailed` error; no stale window.
3. Inspect the served policy: the document response carries `base-uri 'none'; form-action 'none'` and no `unsafe-inline` anywhere.

## Evidence

Re-run artifacts land in `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/evidence/` (named logs + README index, same conventions as the cutover mission). The live demo target remains `make demo-live-graphical-shell`.
