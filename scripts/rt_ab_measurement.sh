#!/usr/bin/env bash
# scripts/rt_ab_measurement.sh — WP06 T022, mission webview-shell-cutover-01KZAC7Q.
#
# Same-workload RT A/B measurement across the shell cutover (spec NFR-001,
# foundation review RISK-3): run the identical live scene workload once
# egui-hosted and once webview-hosted, then extract the RT-relevant measured
# fields from each run's structured output and write a field-by-field
# comparison. No new RT instrumentation is added anywhere: every number is
# read from what the production live reports already measure (the
# callback-measuring global allocator, per-checkpoint audio observations,
# the lossless EventLog, and the teardown witness).
#
# Shell selection per side:
#   * egui side — the pre-cutover baseline commit (default: the merge-base
#     of HEAD and feat/webview-shell-cutover), where `run_live_demo_scene`
#     hosts the scene on the injected launch-time window and the launch
#     default is `ShellSelection::Egui`. The commit is checked out into a
#     throwaway `git worktree` and built there; the working tree is never
#     touched.
#   * webview side — the current working tree (post-WP03), where live mode
#     composes its own `TauriWebviewWindow`.
# The invocation is byte-identical on both sides (same scene flag, same
# committed fixture, same machine), so the only variable is the shell.
#
# Both runs open a REAL window and play PHYSICAL audio. Do not run headless;
# a silent or mocked substitute satisfies nothing.
#
# Requirements: bash 3.2+ (macOS default), git, cargo, jq, /usr/bin/time, a
# physical display and audio output device, and no other heavy load.
#
# Usage:
#   scripts/rt_ab_measurement.sh [--baseline-ref <ref>] [--out-dir <dir>]
#                                [--scene-flag <flag>] [--reuse-logs]
#
# --reuse-logs regenerates rt-ab-comparison.md from existing rt-ab-egui.log
# and rt-ab-webview.log without re-running the live scenes; the logs are the
# measurement of record and are never modified by this mode.
#
# Outputs (in --out-dir, default kitty-specs/webview-shell-cutover-01KZAC7Q/evidence):
#   rt-ab-egui.log        complete structured output of the egui-hosted run
#   rt-ab-webview.log     complete structured output of the webview-hosted run
#   rt-ab-comparison.md   field-by-field comparison, measured numbers only
#
# Exit code: 0 only when both runs exited 0. A failed run still leaves its
# complete log behind — commit it as a failed run; never trim or replace it
# silently.
set -euo pipefail

SCENE_FLAG="--demo-live-sixteen-track-mixer-routing"
OUT_DIR=""
BASELINE_REF=""
REUSE_LOGS=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --baseline-ref)
            BASELINE_REF="${2:?--baseline-ref requires a value}"
            shift 2
            ;;
        --out-dir)
            OUT_DIR="${2:?--out-dir requires a value}"
            shift 2
            ;;
        --scene-flag)
            SCENE_FLAG="${2:?--scene-flag requires a value}"
            shift 2
            ;;
        --reuse-logs)
            REUSE_LOGS=1
            shift
            ;;
        *)
            echo "unsupported option $1" >&2
            exit 2
            ;;
    esac
done

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"
if [ -z "$OUT_DIR" ]; then
    OUT_DIR="$REPO_ROOT/kitty-specs/webview-shell-cutover-01KZAC7Q/evidence"
fi
mkdir -p "$OUT_DIR"

SCRATCH="${RT_AB_SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/rt-ab.XXXXXX")}"
mkdir -p "$SCRATCH"
EGUI_TREE="$SCRATCH/egui-baseline-tree"

cleanup() {
    if [ -d "$EGUI_TREE" ]; then
        git worktree remove --force "$EGUI_TREE" >/dev/null 2>&1 || true
        git worktree prune >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

HOST_MODEL="$(sysctl -n hw.model 2>/dev/null || uname -m)"
HOST_OS="$(sw_vers -productName 2>/dev/null || uname -s) $(sw_vers -productVersion 2>/dev/null || uname -r)"
HOST_ARCH="$(uname -m)"

EGUI_LOG="$OUT_DIR/rt-ab-egui.log"
WEBVIEW_LOG="$OUT_DIR/rt-ab-webview.log"
COMPARISON="$OUT_DIR/rt-ab-comparison.md"

# ---------------------------------------------------------------------------
# One measured side: header, warm build, timed run, complete log.
# ---------------------------------------------------------------------------
run_side() {
    side="$1"    # egui | webview
    workdir="$2" # tree to build and run in
    sha="$3"     # commit that tree is at
    log="$4"     # evidence log path

    if [ "$side" = egui ]; then
        shell_note="pre-cutover baseline; live scene hosted on the injected default-egui window"
    else
        shell_note="current tree; live mode composes its own TauriWebviewWindow"
    fi
    {
        echo "# RT A/B measurement — side: $side"
        echo "# mission: webview-shell-cutover-01KZAC7Q WP06 T022 (NFR-001, RISK-3)"
        echo "# date: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
        echo "# host: $HOST_MODEL, $HOST_OS, $HOST_ARCH"
        echo "# commit: $sha ($(git log -1 --format=%s "$sha"))"
        echo "# shell: $side ($shell_note)"
        echo "# workload: cargo run --release --bin crest-synth -- $SCENE_FLAG"
        echo "# note: /usr/bin/time -l footer measures the whole cargo-run process tree"
        echo "#       (external, supplementary); RT evidence is the in-report fields."
    } > "$log"

    echo "[$side] building release binary at $sha ..." >&2
    (cd "$workdir" && cargo build --release --bin crest-synth) \
        > "$SCRATCH/build-$side.log" 2>&1 \
        || {
            echo "# BUILD FAILED — see appended build log" >> "$log"
            cat "$SCRATCH/build-$side.log" >> "$log"
            return 1
        }

    echo "[$side] running live scene (real window, physical audio) ..." >&2
    set +e
    (cd "$workdir" && /usr/bin/time -l \
        cargo run --release --bin crest-synth -- "$SCENE_FLAG") \
        >> "$log" 2>&1
    code=$?
    set -e
    echo "# RT_AB_EXIT_CODE=$code" >> "$log"
    return "$code"
}

# ---------------------------------------------------------------------------
# Field extraction: only what the production reports already measure.
# Absent-vs-zero is preserved: a field a scene does not emit is reported as
# "absent", never as 0. Emits shell-sourceable <PREFIX>KEY='VALUE' lines
# (bash 3.2 has no associative arrays, so sides are prefixed variables).
# ---------------------------------------------------------------------------
extract() {
    log="$1"
    out="$2"
    prefix="$3" # E_ | W_

    summary="$(grep '^CREST_LIVE_SUMMARY ' "$log" | head -1 | sed 's/^CREST_LIVE_SUMMARY //' | tr -d "'")"
    grep '^CREST_LIVE_CHECKPOINT ' "$log" | sed 's/^CREST_LIVE_CHECKPOINT //' \
        > "$SCRATCH/checkpoints.jsonl" || true

    jq -s '
        def topo: map(select(.kind == "topology"));
        def engine: map(select(.kind == "engine"));
        def param: map(select(.kind == "parameter"));
        def opt(v): if (v | length) == 0 then "absent" else (v | max) end;
        {
            checkpoints_total: length,
            checkpoints_parameter: (param | length),
            checkpoints_engine: (engine | length),
            checkpoints_topology: (topo | length),
            audio_predicate_failed:
                (param | map(select(.checkpoint.audioPredicatePassed == false)) | length),
            audio_uninterrupted_false:
                (if (topo | length) == 0 then "absent"
                 else (topo | map(select(.checkpoint.audioUninterrupted == false)) | length)
                 end),
            audio_uninterrupted_true:
                (if (topo | length) == 0 then "absent"
                 else (topo | map(select(.checkpoint.audioUninterrupted == true)) | length)
                 end),
            frames_to_projection_max:
                opt([topo[].checkpoint.framesToProjection | select(. != null)]),
            render_blocks_to_audible_max:
                opt([topo[].checkpoint.renderBlocksToAudible | select(. != null)]),
            activation_sequence_gap_max:
                opt([topo[].checkpoint.observedActivationSequenceGap | select(. != null)]),
            engine_source_audio_silent:
                (engine | map(select(.checkpoint.sourceAudioNonzero == false)) | length),
            engine_target_audio_silent:
                (engine | map(select(.checkpoint.targetAudioNonzero == false)) | length),
            engine_transitions:
                ([engine[].checkpoint.transition] | unique | sort | join(",")),
            audio_sequence_first:
                ([param[].checkpoint.audioObservation.sequence] | if length == 0 then "absent" else first end),
            audio_sequence_last:
                ([param[].checkpoint.audioObservation.sequence] | if length == 0 then "absent" else last end),
            audio_sequence_monotonic:
                ([param[].checkpoint.audioObservation.sequence] as $s | $s == ($s | sort)),
            rendered_blocks_last:
                ([param[].checkpoint.audioObservation.renderedBlocks] | if length == 0 then "absent" else last end),
            rendered_frames_last:
                ([param[].checkpoint.audioObservation.renderedFrames] | if length == 0 then "absent" else last end)
        }
        | to_entries[] | "CK_" + (.key | ascii_upcase) + "=" + (.value | tostring)
    ' -r "$SCRATCH/checkpoints.jsonl" \
        | sed -e "s/^/$prefix/" -e "s/=\(.*\)\$/='\1'/" > "$out"

    # Summary-line scalars (the production one-line report).
    {
        echo "${prefix}SUMMARY_LINE='$summary'"
        echo "${prefix}QUALIFYING_FRAMES='$(sed -En 's/.* ([0-9]+) qualifying shell frames.*/\1/p' <<< "$summary")'"
        echo "${prefix}EVENTS_DROPPED='$(sed -En 's/.* [0-9]+ events, ([0-9]+) dropped.*/\1/p' <<< "$summary")'"
        echo "${prefix}CALLBACK_ALLOCATIONS='$(sed -En 's/.*callbackAllocations=([0-9]+).*/\1/p' <<< "$summary")'"
        echo "${prefix}CALLBACK_DESTRUCTIONS='$(sed -En 's/.*callbackDestructions=([0-9]+).*/\1/p' <<< "$summary")'"
        echo "${prefix}CLEANUP='$(sed -En 's/.*cleanup=(true|false).*/\1/p' <<< "$summary")'"
        echo "${prefix}ACTIVE_NOTES='$(sed -En 's/.*activeNotes=([0-9]+).*/\1/p' <<< "$summary")'"
        echo "${prefix}COMPLETENESS='$(sed -En 's/^live demo (complete|incomplete):.*/\1/p' <<< "$summary")'"
    } >> "$out"

    # Teardown witness (final observation line; marker is scene-specific).
    teardown="$(grep -E '^CREST_[A-Z_]+_LIVE_OBSERVATION ' "$log" | head -1 | sed -E 's/^CREST_[A-Z_]+_LIVE_OBSERVATION //')"
    if [ -n "$teardown" ]; then
        jq -r '
            {
                window_closed: (.window_closed // "absent"),
                stream_released: (.stream_released // "absent"),
                owned_graphs_remaining: (.owned_graphs_remaining // "absent"),
                active_notes_after_cleanup: (.active_notes_after_cleanup // "absent"),
                physical_audio_nonzero: (.physical_audio_nonzero // "absent")
            }
            | to_entries[] | "TD_" + (.key | ascii_upcase) + "=" + (.value | tostring)
        ' <<< "$teardown" | sed -e "s/^/$prefix/" -e "s/=\(.*\)\$/='\1'/" >> "$out"
    else
        for key in TD_WINDOW_CLOSED TD_STREAM_RELEASED TD_OWNED_GRAPHS_REMAINING \
            TD_ACTIVE_NOTES_AFTER_CLEANUP TD_PHYSICAL_AUDIO_NONZERO; do
            echo "${prefix}${key}='absent'"
        done >> "$out"
    fi

    # Whole-process externals from /usr/bin/time -l (supplementary; the
    # report does not carry per-thread audio CPU, so none is invented).
    {
        echo "${prefix}PROC_REAL_S='$(awk '/ real /{print $1; exit}' "$log")'"
        echo "${prefix}PROC_USER_S='$(awk '/ real /{print $3; exit}' "$log")'"
        echo "${prefix}PROC_SYS_S='$(awk '/ real /{print $5; exit}' "$log")'"
        echo "${prefix}PROC_MAX_RSS_BYTES='$(awk '/maximum resident set size/{print $1; exit}' "$log")'"
        echo "${prefix}EXIT_CODE='$(sed -En 's/^# RT_AB_EXIT_CODE=([0-9]+)$/\1/p' "$log")'"
        echo "${prefix}COMMIT='$(sed -En 's/^# commit: ([0-9a-f]+) .*/\1/p' "$log" | head -1)'"
    } >> "$out"
}

# ---------------------------------------------------------------------------
# Sides
# ---------------------------------------------------------------------------
EGUI_EXIT=0
WEBVIEW_EXIT=0

if [ "$REUSE_LOGS" -eq 1 ]; then
    for log in "$EGUI_LOG" "$WEBVIEW_LOG"; do
        if [ ! -s "$log" ]; then
            echo "--reuse-logs: $log is missing or empty" >&2
            exit 2
        fi
    done
    echo "RT A/B: regenerating comparison from existing logs (runs untouched)" >&2
else
    if [ -z "$BASELINE_REF" ]; then
        BASELINE_REF="$(git merge-base HEAD feat/webview-shell-cutover)"
    fi
    BASELINE_SHA="$(git rev-parse --verify "${BASELINE_REF}^{commit}")"
    HEAD_SHA="$(git rev-parse HEAD)"
    if [ "$BASELINE_SHA" = "$HEAD_SHA" ]; then
        echo "baseline ($BASELINE_SHA) equals HEAD: both sides would run the same shell" >&2
        exit 2
    fi

    echo "RT A/B: egui baseline $BASELINE_SHA vs webview HEAD $HEAD_SHA" >&2
    echo "RT A/B: workload $SCENE_FLAG, evidence in $OUT_DIR" >&2

    git worktree add --detach "$EGUI_TREE" "$BASELINE_SHA" >/dev/null

    run_side egui "$EGUI_TREE" "$BASELINE_SHA" "$EGUI_LOG" || EGUI_EXIT=$?
    run_side webview "$REPO_ROOT" "$HEAD_SHA" "$WEBVIEW_LOG" || WEBVIEW_EXIT=$?
fi

extract "$EGUI_LOG" "$SCRATCH/egui.fields" "E_"
extract "$WEBVIEW_LOG" "$SCRATCH/webview.fields" "W_"
# shellcheck disable=SC1091
. "$SCRATCH/egui.fields"
# shellcheck disable=SC1091
. "$SCRATCH/webview.fields"

# The logs are the measurement of record: commits and exit codes come from
# their headers, so --reuse-logs reports exactly what was run.
EGUI_SHA="${E_COMMIT:-unknown}"
WEBVIEW_SHA="${W_COMMIT:-unknown}"
EGUI_EXIT="${E_EXIT_CODE:-$EGUI_EXIT}"
WEBVIEW_EXIT="${W_EXIT_CODE:-$WEBVIEW_EXIT}"

# bash 3.2-portable two-side lookup via ${!var} indirection.
side_value() {
    var="$1$2"
    if [ -n "${!var:-}" ]; then
        echo "${!var}"
    else
        echo "absent"
    fi
}

row() {
    printf '| %s | %s | %s |\n' "$1" "$(side_value E_ "$2")" "$(side_value W_ "$2")"
}

# Numeric within-envelope check: webview <= egui, only when both measured.
envelope() {
    label="$1"
    a="$(side_value E_ "$2")"
    b="$(side_value W_ "$2")"
    case "$a$b" in
        *[!0-9]*)
            echo "- $label: egui=$a webview=$b (not numerically comparable: absent on at least one side)"
            return
            ;;
    esac
    if [ "$b" -le "$a" ]; then
        echo "- $label: webview $b <= egui $a — within the baseline envelope"
    else
        echo "- $label: webview $b > egui $a — OUTSIDE the baseline envelope"
    fi
}

{
    echo "# RT A/B same-workload comparison — egui vs webview shell"
    echo
    echo "Mission webview-shell-cutover-01KZAC7Q, WP06 T022 (spec NFR-001; foundation review RISK-3)."
    echo
    echo "- Comparison generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    echo "- Host: $HOST_MODEL, $HOST_OS, $HOST_ARCH (physical audio device, real window, no other heavy load)"
    echo "- Workload (byte-identical on both sides): \`cargo run --release --bin crest-synth -- $SCENE_FLAG\`"
    echo "- egui side: commit \`$EGUI_SHA\` (pre-cutover baseline; live scene hosted on the injected default-egui window)"
    echo "- webview side: commit \`$WEBVIEW_SHA\` (live mode composes its own TauriWebviewWindow)"
    echo "- Raw logs: \`rt-ab-egui.log\`, \`rt-ab-webview.log\` (complete, untrimmed; run dates in their headers)"
    echo "- Method: no new RT instrumentation; every number below is read from the production live report output. \"absent\" means the scene does not emit the field (distinct from a measured 0)."
    echo
    echo "| Field | egui | webview |"
    echo "|---|---|---|"
    row "commit" COMMIT
    row "process exit code" EXIT_CODE
    row "report completeness" COMPLETENESS
    row "checkpoints total" CK_CHECKPOINTS_TOTAL
    row "checkpoints: parameter" CK_CHECKPOINTS_PARAMETER
    row "checkpoints: engine" CK_CHECKPOINTS_ENGINE
    row "checkpoints: topology" CK_CHECKPOINTS_TOPOLOGY
    row "callbackAllocations (RT callback, global-allocator witness)" CALLBACK_ALLOCATIONS
    row "callbackDestructions (RT callback, global-allocator witness)" CALLBACK_DESTRUCTIONS
    row "EventLog dropped records" EVENTS_DROPPED
    row "audioPredicatePassed=false checkpoints" CK_AUDIO_PREDICATE_FAILED
    row "audioUninterrupted=false checkpoints (topology-only field)" CK_AUDIO_UNINTERRUPTED_FALSE
    row "audioUninterrupted=true checkpoints (topology-only field)" CK_AUDIO_UNINTERRUPTED_TRUE
    row "framesToProjection max (topology-only field)" CK_FRAMES_TO_PROJECTION_MAX
    row "renderBlocksToAudible max (topology-only field)" CK_RENDER_BLOCKS_TO_AUDIBLE_MAX
    row "observedActivationSequenceGap max (topology-only field)" CK_ACTIVATION_SEQUENCE_GAP_MAX
    row "engine checkpoints with silent source audio" CK_ENGINE_SOURCE_AUDIO_SILENT
    row "engine checkpoints with silent target audio" CK_ENGINE_TARGET_AUDIO_SILENT
    row "engine transition identities (distinct, sorted)" CK_ENGINE_TRANSITIONS
    row "audioObservation.sequence first checkpoint" CK_AUDIO_SEQUENCE_FIRST
    row "audioObservation.sequence last checkpoint" CK_AUDIO_SEQUENCE_LAST
    row "audioObservation.sequence monotonic across checkpoints" CK_AUDIO_SEQUENCE_MONOTONIC
    row "renderedBlocks at last parameter checkpoint" CK_RENDERED_BLOCKS_LAST
    row "renderedFrames at last parameter checkpoint" CK_RENDERED_FRAMES_LAST
    row "qualifying shell frames" QUALIFYING_FRAMES
    row "cleanup" CLEANUP
    row "activeNotes after cleanup (summary)" ACTIVE_NOTES
    row "teardown: window_closed" TD_WINDOW_CLOSED
    row "teardown: stream_released" TD_STREAM_RELEASED
    row "teardown: owned_graphs_remaining" TD_OWNED_GRAPHS_REMAINING
    row "teardown: active_notes_after_cleanup" TD_ACTIVE_NOTES_AFTER_CLEANUP
    row "teardown: physical_audio_nonzero" TD_PHYSICAL_AUDIO_NONZERO
    row "whole-process real seconds (external /usr/bin/time)" PROC_REAL_S
    row "whole-process user CPU seconds (external /usr/bin/time)" PROC_USER_S
    row "whole-process sys CPU seconds (external /usr/bin/time)" PROC_SYS_S
    row "whole-process max RSS bytes (external /usr/bin/time)" PROC_MAX_RSS_BYTES
    echo
    echo "Per-thread audio-callback CPU is not carried by the production"
    echo "observation, so it is reported as absent rather than invented; the"
    echo "whole-process rows above are supplementary external measurements of"
    echo "the full cargo-run process tree."
    echo
    echo "## Acceptance bar (WP06 T022), stated as measured numbers"
    echo
    echo "- audioUninterrupted=false count: egui=$(side_value E_ CK_AUDIO_UNINTERRUPTED_FALSE), webview=$(side_value W_ CK_AUDIO_UNINTERRUPTED_FALSE) (bar: zero in both; the field is emitted only by topology checkpoints — this scene emitted egui=$(side_value E_ CK_CHECKPOINTS_TOPOLOGY), webview=$(side_value W_ CK_CHECKPOINTS_TOPOLOGY) topology checkpoints, so the per-checkpoint audio-continuity witness for this scene is audioPredicatePassed failures: egui=$(side_value E_ CK_AUDIO_PREDICATE_FAILED), webview=$(side_value W_ CK_AUDIO_PREDICATE_FAILED))"
    echo "- RT bounds within the egui baseline envelope (webview <= egui on every measured bound):"
    envelope "callbackAllocations" CALLBACK_ALLOCATIONS
    envelope "callbackDestructions" CALLBACK_DESTRUCTIONS
    envelope "EventLog dropped records" EVENTS_DROPPED
    envelope "audioPredicatePassed failures" CK_AUDIO_PREDICATE_FAILED
    envelope "engine silent-source count" CK_ENGINE_SOURCE_AUDIO_SILENT
    envelope "engine silent-target count" CK_ENGINE_TARGET_AUDIO_SILENT
    envelope "framesToProjection max" CK_FRAMES_TO_PROJECTION_MAX
    envelope "renderBlocksToAudible max" CK_RENDER_BLOCKS_TO_AUDIBLE_MAX
    echo "- process exit codes: egui=$EGUI_EXIT, webview=$WEBVIEW_EXIT (bar: 0 and 0)"
    echo
    echo "## Raw summary lines"
    echo
    echo "- egui: \`${E_SUMMARY_LINE:-<missing>}\`"
    echo "- webview: \`${W_SUMMARY_LINE:-<missing>}\`"
} > "$COMPARISON"

echo "RT A/B: egui exit=$EGUI_EXIT webview exit=$WEBVIEW_EXIT" >&2
echo "RT A/B: wrote $EGUI_LOG, $WEBVIEW_LOG, $COMPARISON" >&2

if [ "$EGUI_EXIT" -ne 0 ] || [ "$WEBVIEW_EXIT" -ne 0 ]; then
    exit 1
fi
