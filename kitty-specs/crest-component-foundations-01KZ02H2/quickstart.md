# Quickstart: Crest Component Foundations

**Date**: 2026-08-02 · **Branch**: `feat/crest-component-foundations`

How to see, run, and verify what this mission delivers.

## See it

```bash
make run                          # the production app, painted through the authored vocabulary
make demo-live-component-library  # the browsable gallery — digit keys 1-8 change page
```

`make run` is the P1 outcome: the window should show the authored dark palette, cyan focus, amber
adjustment, and Azeret Mono. Before this mission it showed a green-accented screen in a default font.

The gallery is browsable, not autonomous. It waits for you, digit keys page through it, and it exits when
you close the window. Every other `demo-live-*` target is the opposite — those run themselves and ignore
input on purpose.

### Gallery pages

| Key | Page | Shows |
|---|---|---|
| 1 | Colors | All 17 semantic colors with their canonical names |
| 2 | Type | All 8 type styles at their authored size, weight, line height, tracking |
| 3 | Spacing and geometry | The 6 spacing steps, radii, keyline widths, 48 px minimum target |
| 4 | Interaction states | Resting, focused, adjusting — keylines and the focus halo |
| 5 | Text and hairlines | Text roles and hairline separators |
| 6 | Values and status | Value displays, status marks, loading and error |
| 7 | Action hints | Footer hint treatments |
| 8 | Shell bands | The five structural bands at both viewport sizes |

Pressing a digit with no page bound leaves the current page alone.

## Verify it

```bash
make test                                    # whole suite
cargo test --test component_vocabulary       # this mission's acceptance target
make lint && make fmt-check
```

The acceptance target prints `CREST_ACCEPTANCE component_vocabulary passed` only after every declared
check holds. It proves values, not names — comparing what the production render path actually paints
against the authored table.

## Check nothing regressed

```bash
make demo-live      # the newest autonomous witness — audio and generation correlation
make smoke
```

This mission must not change what you hear. If `make demo-live` behaves differently, that is a
regression, not a side effect.

## What to look at when reviewing

1. **`make run`, side by side with the design file.** The P1 story is visual. Everything else exists to
   serve it.
2. **Page 8 at both sizes.** The Steam Deck density policy is authored rather than measured — there is no
   authored small-viewport design to check it against, so your eye is the acceptance test. This is the
   one place the mission genuinely needs you.
3. **Whether any state is legible only by color.** Every state must carry text or shape too.

## If the typeface is missing

Deleting `vendor/azeret-mono/` should produce a visible typed startup failure naming the unavailable
face — never a silent fallback to a default font. That is the intended behavior, and the acceptance
target proves it.

## Scope boundary

This mission ships the vocabulary, the primitives, the gallery, and the production repaint. It does **not**
ship the configurable controls (parameter rows, sliders, faders, meters, browser rows, modal options) or
the reusable compositions (shell, context switch, identity header, section, strip row, Utility/Inspector,
footer). Those are the follow-on Phase 4 mission. If you are looking for a fader in the gallery, that is
why it is not there.
