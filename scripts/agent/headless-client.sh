#!/usr/bin/env bash
# Usage: headless-client.sh start-godot <checkout> <xdg_config_home> [godot args...]
#        headless-client.sh stop <checkout>/target
# Runs the Godot client from <checkout>/godot with the pinned patched Godot
# (scripts/godot/pinned.sh, or GODOT_BIN) inside a headless cage compositor
# (nothing appears on the user's display).
set -euo pipefail
cmd=$1; T=$2; pidf="$T/headless-client.pid"
if [ "$cmd" = stop ]; then
  [ -f "$pidf" ] || exit 0
  cp=$(cat "$pidf"); kids=$(pgrep -P "$cp" 2>/dev/null || true)
  kill $kids $cp 2>/dev/null || true; sleep 2; kill -9 $kids $cp 2>/dev/null || true
  rm -f "$pidf"; exit 0
fi
[ "$cmd" = start-godot ] || { echo "unknown command: $cmd" >&2; exit 2; }
X=$3; shift 3
checkout=$T; T="$checkout/target"; pidf="$T/headless-client.pid"; mkdir -p "$T"
. "$(dirname "$0")/../godot/pinned.sh"
godot=${GODOT_BIN:-$GODOT_PINNED}
XDG_CONFIG_HOME="$X" WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=vulkan \
  setsid cage -- "$godot" --path "$checkout/godot" "$@" > "$T/headless-client.log" 2>&1 < /dev/null &
echo $! > "$pidf"; echo "cage pid $(cat $pidf); log $T/headless-client.log; IPC socket appears as /tmp/game-engine-<pid>.sock"
