# Quickstart: Webview Shell Cutover

**Mission**: `webview-shell-cutover-01KZAC7Q`

## Run the shell

```bash
make run                     # production app — webview shell (sole shell after IC-05)
make demo-live               # newest cumulative live scene, real window + physical audio
```

## Deterministic proof (no hardware needed)

```bash
cargo test --test webview_projection_shell -- --nocapture   # serialized fidelity, tokens, determinism
cargo test --test shell_event_dispatch -- --nocapture       # headless event → document coherence
cargo test --test component_vocabulary -- --nocapture       # authored values through the webview path
cargo test --test component_composition -- --nocapture
spec-kitty accept --mission webview-shell-cutover-01KZAC7Q  # all 32 declared checks
```

## Hardware evidence (operator, on the rig — order matters)

1. While both shells still exist: RT A/B same-workload measurement (NFR-001).
2. `make demo-live-graphical-shell`, `make demo-live-semantic-view-model`,
   `make demo-live-sixteen-track-mixer-routing`, `make demo-live-effects-and-buses`
   — each through the webview shell, exit 0, logs committed (FR-003).
3. `CREST_WEBVIEW_FULL_SOAK=1` 300 s soak, recorded (NFR-002).
4. Only after those evidence commits: the egui deletion lands (C-007).

## Gallery

```bash
make demo-live-component-library   # 15 pages, digits 1-9/0 + [ ] stepping, both densities
```

## What must never happen

- The page registering a key handler (asserted by test).
- A second serialization of `SemanticGraphicalViewModel` (byte-identity proof).
- A hand-edited `webview-page/tokens.css` (generation freshness proof).
- A silent fallback window after webview init failure (typed-error test).
