#!/usr/bin/env bash
# Isolated native Linux test desktop. Audio reaches a PulseAudio monitor sink;
# this proves the CPAL/ALSA path, not physical speakers or a connected gamepad.
set -euo pipefail

if [[ "$(uname -s)" != Linux ]]; then
  echo 'Run this script on Linux, or in the scripts/linux/Dockerfile image.' >&2
  exit 2
fi
if [[ "${1:-}" != --inside-desktop ]]; then
  exec dbus-run-session -- xvfb-run -a -s '-screen 0 2560x1600x24 -nolisten tcp' \
    "$0" --inside-desktop "$@"
fi
shift
unset WAYLAND_DISPLAY
export GDK_BACKEND=x11
export XDG_CURRENT_DESKTOP=GNOME
# Xvfb has no hardware compositor. WebKit's accelerated path adds avoidable
# software-GPU round trips here; keep paint timing on its software path.
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
if [[ "$#" -eq 0 ]]; then
  echo 'usage: with-desktop.sh <command> [arguments...]' >&2
  exit 2
fi

runtime="$(mktemp -d /tmp/crest-linux-desktop.XXXXXX)"
export XDG_RUNTIME_DIR="$runtime"
export PULSE_SERVER="unix:$runtime/pulse/native"
export CREST_AUDIO_BUFFER_FRAMES="${CREST_AUDIO_BUFFER_FRAMES:-8192}"
export ALSA_CONFIG_PATH="$runtime/asound.conf"
# Portal backends activated by D-Bus need the display created by xvfb-run.
dbus-update-activation-environment DISPLAY XAUTHORITY XDG_CURRENT_DESKTOP XDG_RUNTIME_DIR
cat > "$ALSA_CONFIG_PATH" <<'EOF'
</usr/share/alsa/alsa.conf>
pcm.!default { type pulse }
ctl.!default { type pulse }
EOF
cleanup() {
  pulseaudio --kill >/dev/null 2>&1 || true
  if [[ -n "${window_manager:-}" ]]; then
    kill "$window_manager" 2>/dev/null || true
    wait "$window_manager" 2>/dev/null || true
  fi
  rm -rf "$runtime"
}
trap cleanup EXIT
openbox > "$runtime/openbox.log" 2>&1 &
window_manager=$!
pulseaudio --daemonize=yes --exit-idle-time=-1 \
  --load='module-null-sink sink_name=crest_test_sink rate=48000 channels=2'
pactl set-default-sink crest_test_sink
"$@"
