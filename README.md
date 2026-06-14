# crest-synth

A standalone, gamepad-friendly MIDI synthesizer written in Rust. Designed for the
Steam Deck but runs on any desktop. Hexagonal architecture with a hard real-time
audio thread at its core — the audio callback never allocates, locks, or blocks.

See [DESIGN.md](DESIGN.md) for the full architecture.

## Status

All 11 build phases are generated and green: builds clean, 1357 tests pass, and the
UI audio self-check (`make ui-smoke`) reports non-silent render.

| Phase | Domain |
|------:|--------|
| 1 | Kernel + MIDI-file playback spine |
| 2 | Polyphonic synth engine (voices, oscillator, filter, ADSR, stealing) |
| 3 | Real-time seam (rtrb / triple_buffer / basedrop) + cpal output |
| 4 | MIDI input + normalization |
| 5 | Patch model |
| 6 | Sample library |
| 7 | Effects (chorus, delay, reverb) |
| 8 | Presets (banks, codec, browser) |
| 9 | Shell adapters (cpal, midir, eframe, gilrs) |
| 10 | Plugin wrapper (nih-plug host) |
| 11 | Editor — keyboard/gamepad GUI, one-way event loop |

## Build & test

```sh
make build      # cargo build
make test       # cargo test
make lint       # cargo clippy --all-targets -- -D warnings
make fmt        # cargo fmt
make check      # build + test
```

## Run

```sh
make ui                 # launch the standalone gamepad/keyboard editor (live engine)
make ui-smoke           # headless audio self-check (asserts non-silent render)
make play-tone          # render a C-E-G arpeggio to tone-test.wav
make play-midi-live     # stream a .mid through the default output device via cpal
```

Other demos render to WAV: `make demo-voices`, `demo-patches`, `demo-mod`,
`demo-samples`, `demo-effects`, `demo-presets`. Device-free checks:
`make check-live`, `make check-gamepad`.

### The editor (phase 11)

The UI is a **patch/parameter editor**, not a performance surface — all notes come
from external MIDI gear; MIDI-file playback is a test fixture. It is fully
keyboard/gamepad driven (no mouse/touch):

- **WSAD** — up / down / left / right navigation
- **Hold J** — momentary edit mode; in edit mode **A/D** = ±1 (fine), **W/S** = ±10 (coarse)

State flows one way (Elm/Flux): widgets emit events → stores apply them in a reducer →
the new `ParameterSnapshot` is published across the lock-free seam → the audio thread
reads it. The audio model knows nothing about UI events.

## Spec-driven development

This crate is generated and maintained by the crest-spec declarative codegen loop.
The source of truth is the CUE spec under [`spec/`](spec/):

- `base.cue` — project config, shared Kernel/Shell contracts, whole-crate validations
- `phase-N.cue` — each phase's bounded contexts, aggregates, ports, and assets
- `phase-N.override-*.cue` — per-phase additions to `Cargo.toml` / `Makefile` / demos

To evolve the synth you edit the **spec**, not the Rust — the generate→validate→retry
loop writes and verifies the code. Each asset's `prompts:` describe what to generate;
each phase's `validations:` (compile, test, clippy `--all-targets`, custom make targets)
are the gate.
