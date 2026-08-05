# Decision Moment `01KZ9DNTZE7TQFMH1Z7YQTKT01`

- **Mission:** `webview-shell-foundation-01KZ9DN7`
- **Origin flow:** `specify`
- **Slot key:** `specify.input.capture-ownership`
- **Input key:** `input_capture`
- **Status:** `resolved`
- **Created:** `2026-08-05T16:56:30.446219+00:00`
- **Resolved:** `2026-08-05T16:56:31.235585+00:00`
- **Opened by:** `cli`
- **Other answer:** `false`

## Question

Where does controller/keyboard input get captured?

## Options

_(none)_

## Final answer

Rust side: existing physical-input translators keep owning input; the webview is a pure projection surface

## Rationale

_(none)_

## Change log

- `2026-08-05T16:56:30.446219+00:00` — opened
- `2026-08-05T16:56:31.235585+00:00` — resolved (final_answer="Rust side: existing physical-input translators keep owning input; the webview is a pure projection surface")
