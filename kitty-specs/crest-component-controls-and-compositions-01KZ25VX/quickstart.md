# Quickstart: Crest Component Controls and Compositions

How to run this mission's result and judge whether it is right.

## See the library

```bash
make demo-live-component-library
```

A real window opens on the gallery. Digit keys `1`–`9` and `0` jump to the first ten pages; `[` and `]` step through all fifteen. Close the window to exit.

Pages 1–8 are Phase 4a's and are unchanged. What this mission adds:

| Page | Key | Shows |
|---|---|---|
| 9. ParameterAndChoiceRows | `9` | parameter rows and choice rows, every applicable state |
| 10. TogglesAndSliders | `0` | toggles and compact sliders |
| 11. FadersAndMeters | `]` from 10 | faders and meters |
| 12. BrowserAndModalOptions | `]` | browser rows and modal options |
| 13. ShellAndContextSwitch | `]` | the application shell and context switch |
| 14. HeadersAndSections | `]` | identity header and section |
| 15. StripPanelAndFooter | `]` | Patch strip row, Utility/Inspector panel, footer |

Every page renders at both the desktop (1920×1080) and Steam Deck (1280×800) sizes.

**Nothing sounds.** The scene opens no audio device and loads no MIDI fixture. If you hear anything, that is a defect (NFR-006, C-001).

## See the shipped product

```bash
make run
```

Every region on screen — context line, identity header, main workspace, side region, footer — is now painted by a composition, and every control within them by a control. Put it beside the Figma file and compare.

## What to look at

1. **Does each control look like its Figma specimen?** This is the judgment a log cannot carry and the reason the gallery exists.
2. **Is every state legible without color?** Cover the color channel mentally — focused, adjusting, disabled, loading, error, muted, soloed, and selected must each still be distinguishable by text or shape.
3. **Does the compact viewport still hold everything?** Both bands, the persistent side region, and 48 px minimum targets must survive at 1280×800. Nothing hidden to fit.
4. **Is anything on screen a lie?** If the shipped shell shows a value, real view data produced it. A placeholder in the production window is a C-003 violation — the gallery is where representative content belongs.

## Run the proof

```bash
cargo test --test component_composition -- --nocapture > /tmp/cc.log 2>&1; echo "exit=$?"
cargo test --test component_vocabulary  -- --nocapture > /tmp/cv.log 2>&1; echo "exit=$?"
cargo test > /tmp/all.log 2>&1; echo "exit=$?"
cargo clippy --all-targets -- -D warnings; cargo fmt --check
```

Redirect to a file — never pipe test output through `head` or `tail`, because the pipe reports the pager's exit code and a "green" recorded that way is a lie.

Expect `CREST_ACCEPTANCE component_composition passed` and `CREST_ACCEPTANCE component_vocabulary passed`, the full suite green, and **no existing test modified** (NFR-005). A shell, projection, or focus test that needed editing means the recomposition changed behavior — fix the recomposition, not the test.

## Check the boundaries held

```bash
# Adapter reduced to plumbing: must be <= 512 lines (NFR-003)
wc -l src/adapter/eframe_graphical_window.rs

# No visual literals outside the vocabulary (NFR-004) — the declared guard
scripts/<literal-guard> 2>&1; echo "exit=$?"
```

## Scope reminders

- No `SemanticAction` variant, no focus target, no reducer change was added. This mission changed how state is rendered, never what state exists.
- `make demo-live-component-library` is browsable by design. It accepts input and asserts no exact-generation claim, unlike the autonomous `demo-live-*` witness scenes. That is deliberate and does not weaken their contract.
