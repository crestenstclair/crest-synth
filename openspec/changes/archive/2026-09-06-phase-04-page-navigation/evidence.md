# Phase 04 implementation evidence

Date: 2026-09-06. Acceptance status is recorded per check below.
The worktree already contained SoundFont file-loading changes; those changes
remain in place. No commit was performed. Spec sync and archive are recorded below.

## Authored Settings return

The live normative Figma Page Layout is frame `153:184` in file
`kdQMw8dYUZtv2UxJPo0sXU`. Initial inspection found the return unmapped.
The user then selected **Shift+Right**, pairing it with Shift+Left entry.
Official Figma editing updated `155:208` to **SHIFT + RIGHT → RETURN**,
`155:205` to suspended PATCH/MIXER page and exact valid focus restoration or
nearest-enabled repair, and `157:199`–`157:200` to the paired Left/Right
bindings. The former warning badge `155:207` now uses the Settings accent.
Fonts were preserved, screenshots showed readable unclipped text, and a scoped
read of the entire Page Layout found no remaining unmapped/not-specified text.
This resolves task 1.1; the change artifacts now agree with those exact nodes.

## Implemented production path

- Native arrows normalize alongside WASD in the existing AppKit input adapter.
  Keyboard and controller Shift directions emit `NavigatePage(Direction)`.
  Page holds cannot dispatch again before release or cleanup; releasing Shift
  first cannot leak a held direction into navigation or editing. Q/E retain
  canonical non-wrapping Patch selection and require a fresh activation.
- Surface-first dispatch in `AppState::apply` reuses Detail, remembered
  context-root, Settings/discovery, and subordinate cancellation helpers. One
  accepted intent advances one generation and stops after its source edge.
- Source-specific admitted actions own footer guidance. StateTree schema 25
  and event-log schema 8 describe the expanded action/event vocabulary.
- Settings exposed a production StateTree coherence failure: the projector
  correctly removed the Patch page while retaining the PATCH context, but the
  tree required that page. Full and MIDI generation-only validation now admit
  its absence on a system surface. A negative test still rejects a missing
  Patch page outside Settings.
- Navigation and Q/E do not publish scalar snapshots. Production AppLoop,
  Braids, effect preparation, graph, and audio-render coverage preserves saved
  capture and audio values, graph identity, and finite nonzero sustained audio.
  The fixture uses sparse Patch IDs 7 and 42 with duplicate Chorus slots.

## Passing deterministic checks

```sh
cargo test --quiet --test page_navigation --lib
```

Result: 875 library tests passed, 2 ignored; all 9 page-navigation integrations
passed. Log: `/tmp/crest-page-library-final.log`. This includes translator
modifier/repeat/cleanup checks, native arrow aliases and redispatch checks,
unsafe Settings entry, exhaustive action consumers, exact Detail subjects,
empty-slot rejection, remembered roots, exposed repair, Settings discovery,
modal admission, sparse Patch endpoints, projected guidance, StateTree
validation, and the sustained production audio journey with MIDI in Settings.

```sh
cargo test --quiet --test semantic_focus_and_projection \
  --test shell_event_dispatch --test graphical_application_shell \
  --test patch_page_projection --test engine_post_fx_option_states \
  --test phase7_sample_workflows --test soundfont_file_loading \
  --test soundfont_preset_selection --test sample_detail_visual \
  --test schema_surface
```

Result: all 51 tests passed. Log: `/tmp/crest-page-affected-final.log`.
The earlier failure in `physical_shift_entry_and_close_restore_every_detail_origin`
was a stale expectation for `OpenRelated`/`Return`; the fixture now expects page
intent and releases the physical directions. Sample cancellation explicitly
uses Page Down and still stops active preview exactly once.

After authoring Shift+Right, the source-specific Settings reducer branch and
footer were updated, followed by:

```sh
cargo test --test page_navigation --test midi_device_contracts \
  --test semantic_focus_and_projection
```

Result: all 28 tests passed (11 page-navigation, 7 MIDI-device, 10 focus and
projection). Log: `/tmp/crest-page-right-focused.log`. This adds physical
Shift+D return to suspended Detail, Mixer Main, and Inspector; exposed repair
after removal of a suspended Overview origin; exact discovery state
preservation; Shift+Right footer guidance; and no accidental next-Patch step.
The sustained production audio journey now uses Shift+Right from Settings.
The existing Shift+Down escape remains covered separately as compatibility.
These results close task 4.1 without substituting the old escape as proof.

```sh
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
openspec validate phase-04-page-navigation --strict
git diff --check
```

Result: all passed. Clippy log: `/tmp/crest-page-clippy-final.log`.
OpenSpec validation establishes artifact coherence only; it does not imply
the unchecked contract and native tasks are complete.

Earlier affected coverage also passed all 7 `midi_device_contracts` and all 10
`production_runtime_contracts` tests; retained log:
`/tmp/crest-page-affected.log`. That earlier command later stopped at the stale
physical assertion above, so its aggregate run is not claimed as passing.

## Earlier native attempts

```sh
make test-webview-page-navigation-native
```

The target enables `CREST_WEBVIEW_TESTS=1` and
`CREST_WEBVIEW_PAGE_NAVIGATION_WITNESS=1` in the existing webview harness.
The bounded journey posts actual AppKit events to its owned window, feeds the
production native capture and keyboard translator, then applies only the
captured action through the reducer. Native paint observations compare exact
focus, one visible focused control, page, subject, return, generation, and
footer guidance. It does not use direct reducer injection as native input proof
or run unrelated resize/soak scenes. The original Settings exit was tagged
`page-settings-compatibility-escape`; the updated journey uses an actual
Shift+Right capture tagged `page-settings-return`.

An initial harness error locating the transient key window was fixed by
selecting the exact owned window title and making that window key. Subsequent
runs still failed strict input checks:

- `/tmp/crest-page-native.log`: E was accepted and painted, then the witness
  rejected `page-e-next: extra native activation PreviewStart`.
- `/tmp/crest-page-native-final.log`: startup Overview painted, then the witness
  rejected `page-e-next: expected SelectPatch(Right), captured Navigate(Right)`.

The source of the unexpected native input was not established. No unexpected
action was discarded to obtain a pass. Both commands exited unsuccessfully.
The driver requests owned-window closure on error; those failed runs did not
prove complete native journey and closure acceptance. Partial paint JSON is retained under
`/tmp/crest-wp06-webview-acceptance/page-*.json`; files from different attempts
must not be combined into a complete journey.

Both attempts passed the harness's serialized-schema fidelity, token freshness,
production protocol/CSP parity, late-ack identity, and typed startup-failure
checks before native failure. Those headless checks do not close task 5.2.

## Passing authored native journey

```sh
make test-webview-page-navigation-native
```

Result: passed. Log: `/tmp/crest-page-right-native.log`. All 17 scripted
activations passed with no extra native actions, including actual Q/E,
Shift+arrows with repeat injection, Instrument and effect Detail entry/return,
remembered PATCH/MIXER roots, Shift+Left Settings entry, authored Shift+Right
return, and WASD equivalence. Every accepted action advanced one generation;
painted page, subject, exact focus, return, and footer matched the production
projection. Saved content and audio-command/structural-effect checks passed
throughout. `T026 harness window owned shutdown` passed with `run_return = 0`.
The same run passed the headless checks listed above. Unrelated resize,
geometry, fault-injection, and soak scenes were explicitly outside this focused
target; they are not required by this change and are not claimed as new proof.

The 18 observations (startup plus 17 activations) from this successful run are
`/tmp/crest-wp06-webview-acceptance/page-*.json`; use the successful log's exact
ordered tags to identify this run's files. The current Settings return
observation is `page-settings-return.json`. The prior unsuccessful attempts are
retained above, not reclassified as passes. This passing run closes task 5.2.

## Final acceptance

The Settings contract and implementation now agree, and the complete bounded
native journey passed. Final affected-code checks also passed:

```sh
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
openspec validate phase-04-page-navigation --strict
git diff --check
```

Clippy log: `/tmp/crest-page-right-clippy.log`. `DESIGN.md` records actual
bindings, source-first reducer ownership, compatibility behavior, and measured
proof without a physical-gamepad or broad visual-parity claim. All 11 tasks
are implemented and verified.

## Spec sync and archive

On 2026-09-06 the user requested spec sync and archive. The sync added all seven
`page-navigation` requirements, updated three `midi-device-settings`
requirements and two `empty-patch-implicit-creation` requirements, and preserved
existing Purpose sections and unrelated requirements/scenarios. Comparison
against all three delta specs confirmed no outstanding sync differences.
`openspec validate --specs` passed all 11 specs, strict change validation passed,
and `git diff --check` passed before the move.

The completed spec-driven change was archived to
`openspec/changes/archive/2026-09-06-phase-04-page-navigation/`, with all 11 tasks
complete and `.openspec.yaml` preserved. The moved file inventory matched the
original byte-for-byte before this lifecycle note was added.
