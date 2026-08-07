# Semantic Acceptance — webview-render-fidelity-hardening-01KZCEF8

**Date**: 2026-08-06 · **Layer**: execution-acceptance contract (obligation ledger + crest-spec drift guards), complementing `deterministic-acceptance.json` (31/31 passed, `passed: true`)
**Judged by**: claude (orchestrator), from four independent reviewer-renata cycle-1 reviews with disable-the-mechanism probes (WP01–WP04) and the committed `evidence/` re-run.

## Obligation ledger — changed crest-spec declarations (commit `07cf450`) → implementation → proof

| Declaration (tightened) | Implemented by | Proof / evidence |
|---|---|---|
| `requirement.webview_projection_shell`: dynamic geometry paints under the production CSP without JS-built inline style attributes | WP01 `22e3c50` — data attributes + CSSOM `setProperty` pass (research D1) | T011 painted-geometry proof at both viewports under served `PAGE_CSP`; zero `style="` in page.js; kill test fails with RISK-1 signature when the pass is neutered (run twice: implementer + reviewer) |
| `requirement.webview_projection_shell`: render throw / unhandled rejection ends the process typed and nonzero | WP01 emitter + latch; WP02 `9a48772` `RenderError → PageRenderFailed` exit path | T012: first-render throw (subprocess, exit 1, typed payload), update-render throw after credited ack (analysis C1), rejection variant, healthy-page negative control |
| `requirement.serialized_projection_transport`: 50 ms p95 measured with the production policy served | WP03 `d30250f` harness cutover | 150-edit paced workload under `PAGE_CSP`: p50 8.1 / p95 8.9 / max 11.7 ms (`evidence/acceptance-live-run.log`); reviewer reproduction p95 9.0 ms |
| `requirement.graphical_shell_behavioral_proof`: harness serves the identical policy from the single source; painted-geometry and forced-throw proofs named | WP02 exported seam (`protocol_response`/`PAGE_CSP` pub, documented two-caller single source); WP03 `prove_protocol_policy_parity` | Served header asserted equal to the exported constant; no CSP literal restated in the test; 9 assets byte-identical; 404 on unknown paths |
| `validation.webview_projection_shell`: description extended to policy parity, painted geometry, typed render-failure exit | WP03 new sections inside the existing named target (command surface unchanged) | Full live run `CREST_ACCEPTANCE webview_projection_shell passed (skipped: none)` — implementer run + independent reviewer re-run |

## Drift guards

1. **Forbidden derived artifacts**: no `data-model.md`, no `contracts/` anywhere in the mission dir (accept lists them as absent-optional). PASS
2. **Crest-spec authored first, never reconciled after the fact**: declarations tightened at `07cf450` during the crest-spec phase, before any implementation commit; no crest-spec edit made during implement. PASS
3. **Asset ownership**: every lane diff confined to the declared asset files (`WebviewProjectionPage` → page.js; `WebviewShellModules` → window.rs/mod.rs; `WebviewProjectionShellAcceptanceTests` → webview_projection_shell.rs; vocabulary/composition test assets → the two scan files). Enforced by the implement claim gate; confirmed per-review. PASS
4. **No silent fallback**: unknown protocol paths 404 with no fallback page; render failure is typed and fatal, never stale-silent; scan hits get narrated allowances, never silent carve-outs. PASS
5. **Validation command surfaces unchanged**: `validations.yaml` untouched by the mission; assertions deepened inside the same named targets, so declared commands are identical. PASS
6. **Frozen baselines / proof rules not loosened**: WP03 diff's deletion side is only the bare no-CSP closure + plumbing; no baseline file changed; pre-review regression gates reported "no new failures" on all four WPs. PASS
7. **Hard real-time and reducer boundaries untouched** (C-001): no reducer/RT/projection-schema change in any lane; `validation.audio_renderer_realtime_contract`, `validation.prepared_graph_handoff_contract`, `validation.audio_observation_realtime_contract` all passed deterministically. PASS
8. **Declared evidence satisfied**: `evidence.component_vocabulary_contract` (validation.component_vocabulary + component_composition, deterministic PASS); `evidence.graphical_application_shell_contract` (validation.graphical_application_shell deterministic PASS + the live webview run committed under `evidence/`). PASS
9. **CSP never weakened** (C-004/NFR-001): programmatic old-vs-new comparison — every pre-existing directive byte-identical, exactly `base-uri 'none'; form-action 'none'` appended; T009 pins the string and rejects `unsafe-inline`/`unsafe-eval`/wildcards per directive. PASS

## Known scope note (honest gap, resolved at merge)

`validation.webview_projection_shell` is the one project validation outside the 31 deterministic completion checks (its live layer requires a real WKWebView on the macOS main thread). Its WP03-deepened content lives on lane-c until `spec-kitty merge` consolidates lanes; the deterministic run therefore exercised the pre-WP03 suite on feat. Compensating evidence: two full live runs of the deepened suite (implementer + independent reviewer), both `skipped: none`, artifacts committed at `evidence/` (`89df0ca`). **Follow-through: re-run the headless suite (and accept if desired) after merge on the consolidated branch.**

## Verdict

Semantic layer **PASS** — both acceptance layers satisfied; mission ready for `spec-kitty merge`.
