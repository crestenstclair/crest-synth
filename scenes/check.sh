#!/usr/bin/env bash
# scenes/check.sh
#
# Runs each starter scene in this directory through the `scene_run` binary
# (asset.SceneRunMain) and asserts MEASURED facts pulled from the resulting
# StateSnapshot JSON via jq, or from scene_run's own computed summary line
# -- never a fixed token the binary prints unconditionally. Exits non-zero
# if any scene fails to apply cleanly or any assertion fails, so a
# regression in the control plane fails this script.
#
# Usage: scenes/check.sh   (run from anywhere; resolves the project root
# from its own location so `cargo run --bin scene_run` always executes
# against this crate).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

FAILURES=0

fail() {
  echo "FAIL: $1" >&2
  FAILURES=$((FAILURES + 1))
}

pass() {
  echo "PASS: $1"
}

# run_scene <scene-file> <snapshot-out-file> [extra scene_run args...]
#
# Runs `scene_run` against <scene-file>, asking it to also write the FINAL
# StateSnapshot to <snapshot-out-file> as one JSON document (via --out), so
# assertions can jq that file directly instead of scraping stdout. With
# --out given, scene_run's own stdout carries only its one computed summary
# line (`events_applied=<N> rejections=<M> frames=<F> peak=<P>`), captured
# here for checks that assert against that line instead of the snapshot.
run_scene() {
  local scene_file="$1"
  local out_file="$2"
  shift 2
  cargo run --quiet --bin scene_run -- --scene "$scene_file" --out "$out_file" "$@" \
    > "$WORK_DIR/$(basename "$out_file").stdout" 2>&1
}

# summary_field <summary-file> <field-name>
#
# Extracts one `key=value` field from scene_run's computed summary line
# (e.g. `events_applied=3`), tolerant of the other fields around it.
summary_field() {
  local summary_file="$1"
  local field="$2"
  grep -oE "${field}=[^ ]+" "$summary_file" | tail -n1 | cut -d= -f2
}

# ---------------------------------------------------------------------------
# mixer-solo: soloing the strip at channels[1] must leave it audible
# (soloed=true, muted=false) while the other two strips exercised here
# (channels[0], channels[2]) report muted=true -- solo-in-place exclusivity
# (aggregate.Mixer.MixerController::is_audible).
# ---------------------------------------------------------------------------
mixer_solo_out="$WORK_DIR/mixer-solo.snapshot.json"
if run_scene scenes/mixer-solo.json "$mixer_solo_out"; then
  soloed_muted=$(jq -r '.channels[1].muted' "$mixer_solo_out")
  soloed_soloed=$(jq -r '.channels[1].soloed' "$mixer_solo_out")
  other_a_muted=$(jq -r '.channels[0].muted' "$mixer_solo_out")
  other_b_muted=$(jq -r '.channels[2].muted' "$mixer_solo_out")

  if [[ "$soloed_soloed" == "true" && "$soloed_muted" == "false" \
        && "$other_a_muted" == "true" && "$other_b_muted" == "true" ]]; then
    pass "mixer-solo: channels[1] soloed=true/muted=false, channels[0],[2] muted=true"
  else
    fail "mixer-solo: expected soloed[1]=true muted[1]=false muted[0]=true muted[2]=true; got soloed[1]=$soloed_soloed muted[1]=$soloed_muted muted[0]=$other_a_muted muted[2]=$other_b_muted"
  fi
else
  fail "scene_run exited non-zero for scenes/mixer-solo.json (see $WORK_DIR/$(basename "$mixer_solo_out").stdout)"
fi

# ---------------------------------------------------------------------------
# volume-edit: from the default 0.0 dB channel volume, one coarse-down nudge
# (-5.0 dB, MixerView::VOLUME_COARSE_STEP_DB) plus two fine-down nudges
# (-0.5 dB each, VOLUME_FINE_STEP_DB) land at exactly -6.0 dB.
# ---------------------------------------------------------------------------
volume_edit_out="$WORK_DIR/volume-edit.snapshot.json"
if run_scene scenes/volume-edit.json "$volume_edit_out"; then
  volume_db=$(jq -r '.channels[2].volume_db' "$volume_edit_out")
  if [[ "$volume_db" == "-6" || "$volume_db" == "-6.0" ]]; then
    pass "volume-edit: channels[2].volume_db == -6.0 dB exactly"
  else
    fail "volume-edit: expected channels[2].volume_db == -6.0, got $volume_db"
  fi
else
  fail "scene_run exited non-zero for scenes/volume-edit.json (see $WORK_DIR/$(basename "$volume_edit_out").stdout)"
fi

# ---------------------------------------------------------------------------
# midi-note-burst (scenes/voice-steal.json): three NoteOn events, each
# followed by 2 rendered blocks, scripted using ONLY the pinned Scene
# vocabulary (Midi/Mixer/Editor/Gamepad -- src/loop/scene_step.rs AppEvent).
# Asserts scene_run's own MEASURED summary line: all 3 events applied with
# zero rejections, all 6 requested blocks (3 steps x 2 render_blocks) were
# actually rendered, and the real oscillator-backed render produced a
# nonzero peak -- so a regression that silently drops a MIDI event, under-
# renders blocks, or leaves the tone renderer silent fails this check.
# (Does not assert `frame`/`active_voice_count`/`.patch` -- those fields do
# not exist on the pinned StateSnapshot schema; see check.sh history.)
# ---------------------------------------------------------------------------
midi_burst_out="$WORK_DIR/midi-note-burst.snapshot.json"
if run_scene scenes/voice-steal.json "$midi_burst_out"; then
  summary_file="$WORK_DIR/$(basename "$midi_burst_out").stdout"
  events_applied=$(summary_field "$summary_file" "events_applied")
  rejections=$(summary_field "$summary_file" "rejections")
  frames=$(summary_field "$summary_file" "frames")
  peak=$(summary_field "$summary_file" "peak")

  if [[ -z "$events_applied" || -z "$rejections" || -z "$frames" || -z "$peak" ]]; then
    fail "midi-note-burst: could not parse scene_run's summary line from $summary_file"
  elif [[ "$events_applied" == "3" && "$rejections" == "0" && "$frames" == "6" ]] \
      && awk -v p="$peak" 'BEGIN { exit !(p > 0) }'; then
    pass "midi-note-burst: events_applied=3 rejections=0 frames=6 peak=$peak (> 0)"
  else
    fail "midi-note-burst: expected events_applied=3 rejections=0 frames=6 peak>0; got events_applied=$events_applied rejections=$rejections frames=$frames peak=$peak"
  fi
else
  fail "scene_run exited non-zero for scenes/voice-steal.json (see $WORK_DIR/$(basename "$midi_burst_out").stdout)"
fi

# ---------------------------------------------------------------------------
# vocabulary-sweep (scenes/preset-roundtrip.json): one event from each of
# the four kinds the pinned Scene vocabulary closes over (Mixer, Editor,
# Gamepad Navigate, Gamepad unit action), proving all four apply uniformly
# through the same reducer path with zero rejections. Asserts scene_run's
# MEASURED summary line rather than `.patch`/preset fields the pinned
# StateSnapshot schema does not carry.
# ---------------------------------------------------------------------------
vocab_sweep_out="$WORK_DIR/vocabulary-sweep.snapshot.json"
if run_scene scenes/preset-roundtrip.json "$vocab_sweep_out"; then
  summary_file="$WORK_DIR/$(basename "$vocab_sweep_out").stdout"
  events_applied=$(summary_field "$summary_file" "events_applied")
  rejections=$(summary_field "$summary_file" "rejections")
  frames=$(summary_field "$summary_file" "frames")
  peak=$(summary_field "$summary_file" "peak")

  if [[ -z "$events_applied" || -z "$rejections" || -z "$frames" || -z "$peak" ]]; then
    fail "vocabulary-sweep: could not parse scene_run's summary line from $summary_file"
  elif [[ "$events_applied" == "6" && "$rejections" == "0" && "$frames" == "1" ]] \
      && awk -v p="$peak" 'BEGIN { exit !(p > 0) }'; then
    pass "vocabulary-sweep: events_applied=6 rejections=0 frames=1 peak=$peak (> 0)"
  else
    fail "vocabulary-sweep: expected events_applied=6 rejections=0 frames=1 peak>0; got events_applied=$events_applied rejections=$rejections frames=$frames peak=$peak"
  fi
else
  fail "scene_run exited non-zero for scenes/preset-roundtrip.json (see $WORK_DIR/$(basename "$vocab_sweep_out").stdout)"
fi

# ---------------------------------------------------------------------------
# showcase (scenes/showcase.json): the ~30-second, human-watchable demo
# scene (solo strip 1 then unsolo, pan hard left then hard right, a volume
# dip and return, a mute toggle -- see scenes/showcase.json's step comments
# in this file's sibling documentation). Headlessly asserts scene_run's
# MEASURED summary line: every one of the scene's 35 scripted steps applied
# with zero rejections, and the real oscillator-backed render produced a
# nonzero peak across all of its paced render_blocks -- so a regression that
# drops a step, rejects an event, or silences the renderer fails this check
# even though the scene's real purpose is to be watched, not just graded.
# ---------------------------------------------------------------------------
showcase_out="$WORK_DIR/showcase.snapshot.json"
if run_scene scenes/showcase.json "$showcase_out"; then
  summary_file="$WORK_DIR/$(basename "$showcase_out").stdout"
  events_applied=$(summary_field "$summary_file" "events_applied")
  rejections=$(summary_field "$summary_file" "rejections")
  frames=$(summary_field "$summary_file" "frames")
  peak=$(summary_field "$summary_file" "peak")

  if [[ -z "$events_applied" || -z "$rejections" || -z "$frames" || -z "$peak" ]]; then
    fail "showcase: could not parse scene_run's summary line from $summary_file"
  elif [[ "$events_applied" == "35" && "$rejections" == "0" && "$frames" == "5175" ]] \
      && awk -v p="$peak" 'BEGIN { exit !(p > 0) }'; then
    pass "showcase: events_applied=35 rejections=0 frames=5175 peak=$peak (> 0)"
  else
    fail "showcase: expected events_applied=35 rejections=0 frames=5175 peak>0; got events_applied=$events_applied rejections=$rejections frames=$frames peak=$peak"
  fi
else
  fail "scene_run exited non-zero for scenes/showcase.json (see $WORK_DIR/$(basename "$showcase_out").stdout)"
fi

echo
if [[ "$FAILURES" -eq 0 ]]; then
  echo "All scene checks passed."
  exit 0
else
  echo "$FAILURES scene check(s) failed."
  exit 1
fi
