# Crest Synth — Figma interpretation

This is a product reading derived from the Figma file alone. It does not compare the design with `DESIGN.md`, CUE declarations, OpenSpec artifacts, or source code.

## Core interpretation

Crest Synth is designed as a hardware instrument rendered through a sparse, text-led interface. Its defining idea is a consistent physical command grammar:

- the D-pad moves focus;
- Edit modifies, adjusts, chooses, or toggles;
- Shift changes context or opens a related surface;
- Select enters multi-select;
- Start is reserved in the general command grammar; the product resolution below defines one Sample Browser exception.

The design depends on spatial memory. PATCH and MIXER are peer contexts. Detail views, option lists, Utility, and the Sample Browser are subordinate surfaces that preserve where the user came from.

## Evidence profile

- Five Figma pages inspected: Documentation, Components, Screens, Mixer, and Interaction Map.
- Eight primary screen/state frames inspected: Patch Strip, Instrument Detail, FX Detail, Sample Detail, Sample Browser, Mixer, Engine Options, and Post FX Options.
- Five component sets containing 31 variants inspected.
- No prototype reactions were present on the inspected screen or state nodes.
- Interaction evidence therefore comes from rendered screens, layer structure, on-canvas annotations, visible control hints, and the authored interaction map.

## Product resolution added after Figma inspection

Product authority resolves the Sample Browser's undefined “X” preview annotation as follows: while a sample row is focused, holding Start previews that sample and releasing Start stops preview. Start remains reserved outside the Sample Browser. This resolution comes from the master product design rather than the Figma evidence.

## Product model inferred from Figma

### Context

The interface has two peer contexts:

1. **PATCH** — composition, routing, engine/effect selection, and detailed parameter editing.
2. **MIXER** — 16-track level, pan, mute, and solo control.

### PATCH surfaces

- **Patch Strip** is the home and orchestration surface.
- **Detail** is one stable polymorphic parameter shell used by instruments and effects.
- **Utility** is a persistent right-side region entered spatially with D-pad Right.
- **Choice modals** expose all engine or effect options at once.
- **Sample Browser** is a nested PATCH subflow, not a peer application context.

### Interaction state

The screen can be understood as the projection of five layers:

1. current context;
2. current surface;
3. focused region and control;
4. interaction mode, such as navigate, adjust, choose, toggle, or multi-select;
5. remembered origin used to return to the exact row that opened a subordinate view.

This is an inference from repeated layouts, return annotations, and the interaction map.

## Stable screen anatomy

Every 1920×1080 PATCH screen uses the same horizontal bands:

| Region | Figma geometry | Purpose |
|---|---:|---|
| Context line | 48 px | Product, PATCH/MIXER context, readiness |
| Patch identity | 72 px | Patch number, name, and current surface metadata |
| Workspace | 896 px | Main task area plus Utility or Preview |
| Controls | 64 px | Current location and live hardware hints |

The workspace is split into 1500 px of main content and a 420 px right-side panel.

## Screen interpretations

### [Patch Strip — node 36:3](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=36-3)

The Patch Strip is a high-level orchestration surface. Instrument and post-FX rows expose direct actions for changing a selection, opening a full option list, or entering Detail. The waveform area is explicitly reserved and omitted from focus order. This keeps the home screen focused on structure rather than editing.

### [Instrument Detail — node 37:7](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=37-7)

Instrument Detail is a stable editor populated by an instrument-specific ordered parameter schema. The Figma note says the instrument supplies its label, accent, ranges, units, defaults, and disabled dependencies. Group headings provide hierarchy without introducing tabs or nested cards.

### [FX Detail — node 38:60](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=38-60)

FX Detail reuses the same shell and adjustment grammar. Its origin remains visible as a slot plus effect name, and Shift+Down returns to that same FX row. The design treats instrument and effect diversity as content variation rather than navigation variation.

### [Sample Detail — node 39:92](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=39-92)

Sample is another instrument schema with a specialized waveform. Start, loop-in, play, loop-out, and end landmarks update as values change, but the waveform itself is not a focus target. Ten rows are visible, including a pinned ADSR group, while the header reports 15 parameters overall.

### [Sample Browser — node 41:138](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=41-138)

The Sample Browser is a controller-native file picker. Parent, folder, file, and cancel rows share a compact CLI grammar. A right-side Preview region supplies file metadata and waveform context. The Figma note says the browser remains inside PATCH and returns to the exact Sample control that opened it.

### [Mixer — node 42:3](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=42-3)

The Mixer keeps all 16 tracks visible. Left/Right changes track, Up/Down changes control, Edit adjusts or toggles, and Select enters multi-select. The focused track is reinforced in the fader field, header, inspector, and footer so dense presentation does not sacrifice orientation.

### Choice modals

[Engine Options](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=48-173) and [Post FX Options](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=48-207) use the same 920×680 list model. The current value is marked, D-pad Up/Down moves, Edit chooses, and Shift+Down closes. Figma shows concrete option lists but does not say whether they are exhaustive.

## Component grammar

| Component set | Variant model | Count | Product role |
|---|---|---:|---|
| Context Switch | State | 3 | Separates default, selected, and focused context states |
| CLI Hint | Tone | 4 | Distinguishes neutral, focus, adjust, and back commands |
| CLI Browser Line | Type × state | 8 | Represents parent, folder, file, and cancel rows |
| Compact Parameter Slider | Tone × state | 12 | Crosses engine/sample/effect identity with default/focused/editing/disabled |
| Compact Mixer Fader | State | 4 | Represents default, focused, muted, and solo |

The component library favors behavioral variants over decorative ones. Hairlines establish grouping; a `>` prefix and cyan line indicate focus; an amber state indicates editing; semantic accent colors distinguish engine, effect, sample, status, and context.

## Visual interpretation

- Azeret Mono is used throughout for aligned labels, values, and commands.
- Surfaces remain nearly neutral; color communicates state or domain rather than decoration.
- Focus is the primary interaction state, not hover.
- Rows and faders are separated by hairlines rather than cards.
- A compact spacing rhythm and large focusable rows make a visually dense interface controller-readable.
- Repeated location cues in the header, section title, inspector, and footer reduce disorientation.

## Remaining unresolved Figma questions

1. **Selector change conflict.** Patch rows say `L/R:change`, while the interaction map says the unmodified D-pad moves focus and `Edit + Left/Right` makes fine adjustments. The exact direct-change binding is ambiguous.
2. **Incomplete multi-select behavior.** Select enters multi-select, but selection accumulation, batch actions, cancellation, and visual treatment are not demonstrated.
3. **No executable prototype path.** The interaction model is well annotated, but zero prototype reactions were present on the inspected screen and state frames.

## Primary interaction source

The complete authored flow is in [Interaction Map — node 49:3](https://www.figma.com/design/kdQMw8dYUZtv2UxJPo0sXU/Crest-Synth-%E2%80%94-Controller-First-UI-Redesign?node-id=49-3).
