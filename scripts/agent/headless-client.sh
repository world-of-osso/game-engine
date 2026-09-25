#!/usr/bin/env bash
# Usage: headless-client.sh start <target_dir> <xdg_config_home> [game args...]  |  headless-client.sh stop <target_dir>
# Runs the client inside a headless cage compositor (nothing appears on the user's display).
set -euo pipefail
cmd=$1; T=$2; pidf="$T/headless-client.pid"
if [ "$cmd" = stop ]; then
  [ -f "$pidf" ] || exit 0
  cp=$(cat "$pidf"); kids=$(pgrep -P "$cp" 2>/dev/null || true)
  kill $kids $cp 2>/dev/null || true; sleep 2; kill -9 $kids $cp 2>/dev/null || true
  # the client may have been reparented; kill any leftover client from this target dir
  for p in $(ls /proc | grep -E '^[0-9]+$'); do [ "$(readlink /proc/$p/exe 2>/dev/null)" = "$T/debug/game-engine" ] && kill -9 $p 2>/dev/null || true; done
  rm -f "$pidf"; exit 0
fi
X=$3; shift 3
export LD_LIBRARY_PATH="$T/debug/deps:$(rustc --print sysroot)/lib"
XDG_CONFIG_HOME="$X" WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=vulkan \
  setsid cage -- "$T/debug/game-engine" "$@" > "$T/headless-client.log" 2>&1 < /dev/null &
echo $! > "$pidf"; echo "cage pid $(cat $pidf); log $T/headless-client.log; IPC socket appears as /tmp/game-engine-<pid>.sock"
