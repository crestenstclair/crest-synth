# RT A/B same-workload comparison — egui vs webview shell

Mission webview-shell-cutover-01KZAC7Q, WP06 T022 (spec NFR-001; foundation review RISK-3).

- Comparison generated: 2026-08-06T07:12:28Z
- Host: Mac15,6, macOS 26.5.2, arm64 (physical audio device, real window, no other heavy load)
- Workload (byte-identical on both sides): `cargo run --release --bin crest-synth -- --demo-live-sixteen-track-mixer-routing`
- egui side: commit `d41e7bd85b74c8eb283a3485a40476a98171d65e` (pre-cutover baseline; live scene hosted on the injected default-egui window)
- webview side: commit `b966da82b244309d0c147d8daeb483cc6c81bb37` (live mode composes its own TauriWebviewWindow)
- Raw logs: `rt-ab-egui.log`, `rt-ab-webview.log` (complete, untrimmed; run dates in their headers)
- Method: no new RT instrumentation; every number below is read from the production live report output. "absent" means the scene does not emit the field (distinct from a measured 0).

| Field | egui | webview |
|---|---|---|
| commit | d41e7bd85b74c8eb283a3485a40476a98171d65e | b966da82b244309d0c147d8daeb483cc6c81bb37 |
| process exit code | 0 | 0 |
| report completeness | complete | complete |
| checkpoints total | 114 | 114 |
| checkpoints: parameter | 105 | 105 |
| checkpoints: engine | 9 | 9 |
| checkpoints: topology | 0 | 0 |
| callbackAllocations (RT callback, global-allocator witness) | 0 | 0 |
| callbackDestructions (RT callback, global-allocator witness) | 0 | 0 |
| EventLog dropped records | 0 | 0 |
| audioPredicatePassed=false checkpoints | 0 | 0 |
| audioUninterrupted=false checkpoints (topology-only field) | absent | absent |
| audioUninterrupted=true checkpoints (topology-only field) | absent | absent |
| framesToProjection max (topology-only field) | absent | absent |
| renderBlocksToAudible max (topology-only field) | absent | absent |
| observedActivationSequenceGap max (topology-only field) | absent | absent |
| engine checkpoints with silent source audio | 8 | 8 |
| engine checkpoints with silent target audio | 6 | 6 |
| engine transition identities (distinct, sorted) | BraidsToDescriptorDefaultSoundFont,SoundFontPresetToNext,SoundFontToBraids | BraidsToDescriptorDefaultSoundFont,SoundFontPresetToNext,SoundFontToBraids |
| audioObservation.sequence first checkpoint | 54 | 57 |
| audioObservation.sequence last checkpoint | 11343 | 10265 |
| audioObservation.sequence monotonic across checkpoints | true | true |
| renderedBlocks at last parameter checkpoint | 11343 | 10265 |
| renderedFrames at last parameter checkpoint | 2903808 | 2627840 |
| qualifying shell frames | 6141 | 1898 |
| cleanup | true | true |
| activeNotes after cleanup (summary) | 0 | 0 |
| teardown: window_closed | true | true |
| teardown: stream_released | true | true |
| teardown: owned_graphs_remaining | 0 | 0 |
| teardown: active_notes_after_cleanup | 0 | 0 |
| teardown: physical_audio_nonzero | true | true |
| whole-process real seconds (external /usr/bin/time) | 86.90 | 81.23 |
| whole-process user CPU seconds (external /usr/bin/time) | 9.42 | 6.84 |
| whole-process sys CPU seconds (external /usr/bin/time) | 2.27 | 1.64 |
| whole-process max RSS bytes (external /usr/bin/time) | 661504000 | 674021376 |

Per-thread audio-callback CPU is not carried by the production
observation, so it is reported as absent rather than invented; the
whole-process rows above are supplementary external measurements of
the full cargo-run process tree.

## Acceptance bar (WP06 T022), stated as measured numbers

- audioUninterrupted=false count: egui=absent, webview=absent (bar: zero in both; the field is emitted only by topology checkpoints — this scene emitted egui=0, webview=0 topology checkpoints, so the per-checkpoint audio-continuity witness for this scene is audioPredicatePassed failures: egui=0, webview=0)
- RT bounds within the egui baseline envelope (webview <= egui on every measured bound):
- callbackAllocations: webview 0 <= egui 0 — within the baseline envelope
- callbackDestructions: webview 0 <= egui 0 — within the baseline envelope
- EventLog dropped records: webview 0 <= egui 0 — within the baseline envelope
- audioPredicatePassed failures: webview 0 <= egui 0 — within the baseline envelope
- engine silent-source count: webview 8 <= egui 8 — within the baseline envelope
- engine silent-target count: webview 6 <= egui 6 — within the baseline envelope
- framesToProjection max: egui=absent webview=absent (not numerically comparable: absent on at least one side)
- renderBlocksToAudible max: egui=absent webview=absent (not numerically comparable: absent on at least one side)
- process exit codes: egui=0, webview=0 (bar: 0 and 0)

## Raw summary lines

- egui: `live demo complete: 105/105 editable parameters, 3/3 engine transitions, 6141 qualifying shell frames, 114 checkpoints, 15934 events, 0 dropped, banks=1, instruments=15, soundfontPatches=8, braidsPatches=7, alternatingCapabilities=true, initialGraphRevision=1, graphRevision=4, engineSwitches=3, fallbacks=0, callbackAllocations=0, callbackDestructions=0, cleanup=true, activeNotes=0`
- webview: `live demo complete: 105/105 editable parameters, 3/3 engine transitions, 1898 qualifying shell frames, 114 checkpoints, 14222 events, 0 dropped, banks=1, instruments=15, soundfontPatches=8, braidsPatches=7, alternatingCapabilities=true, initialGraphRevision=1, graphRevision=4, engineSwitches=3, fallbacks=0, callbackAllocations=0, callbackDestructions=0, cleanup=true, activeNotes=0`
