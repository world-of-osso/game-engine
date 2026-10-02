#!/usr/bin/env bash
# Usage: quit-hang-loop.sh <checkout> <runs> <out_dir> <script res path> [godot args...]
# Runs one Godot test script <runs> times in a headless cage and counts runs whose process
# is still alive after RUN_LIMIT_SECS (default 90). On a hang it writes every
# thread's stack (eu-stack) of the Godot and cage processes to <out_dir>/hang-<n>.stack.
# Exits 1 when any run hung. GODOT_BIN overrides the pinned patched Godot.
set -uo pipefail
checkout=$1; runs=$2; out=$3; script=$4; shift 4
. "$(dirname "$0")/../godot/pinned.sh"
godot=${GODOT_BIN:-$GODOT_PINNED}
mkdir -p "$out/xdg/config" "$out/xdg/data"
if [ ! -f "$checkout/godot/.godot/extension_list.cfg" ]; then
  XDG_CONFIG_HOME="$out/xdg/config" XDG_DATA_HOME="$out/xdg/data" \
    "$godot" --headless --path "$checkout/godot" --import > "$out/import.log" 2>&1
fi
limit=${RUN_LIMIT_SECS:-90}
hangs=0
for n in $(seq 1 "$runs"); do
  log="$out/run-$n.log"
  XDG_CONFIG_HOME="$out/xdg/config" XDG_DATA_HOME="$out/xdg/data" WLR_BACKENDS=headless \
    WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=vulkan setsid cage -- "$godot" --path "$checkout/godot" \
    --display-driver wayland --rendering-driver vulkan --audio-driver Dummy --script "$script" "$@" \
    > "$log" 2>&1 < /dev/null &
  cage=$!
  waited=0
  while kill -0 "$cage" 2>/dev/null && [ "$waited" -lt "$limit" ]; do sleep 1; waited=$((waited + 1)); done
  if kill -0 "$cage" 2>/dev/null; then
    hangs=$((hangs + 1))
    for pid in $(pgrep -P "$cage") "$cage"; do
      { echo "== pid $pid $(cat /proc/$pid/comm)"
        for t in /proc/$pid/task/*; do echo "tid ${t##*/} $(cat $t/comm) state/utime/stime $(sed 's/.*) //' $t/stat | cut -d' ' -f1,12,13)"; done
        # A main thread blocked in pthread_join waits on a futex inside the joined thread's
        # struct pthread; match its address against the pthread_t values gdb lists.
        futex=$(cut -d' ' -f2 /proc/$pid/syscall)
        echo "main syscall: $(cat /proc/$pid/syscall)"
        gdb -p "$pid" -batch -ex "x/4dw $futex" -ex "info threads" 2>/dev/null | grep -E '^0x|Thread 0x'
        eu-stack -m -p "$pid" 2>&1; } >> "$out/hang-$n.stack"
    done
    echo "run $n: HANG after ${waited}s; stacks in $out/hang-$n.stack"
    kill -9 $(pgrep -P "$cage") "$cage" 2>/dev/null
  else
    wait "$cage"; echo "run $n: exit $? after ${waited}s"
  fi
done
echo "hangs: $hangs/$runs"
[ "$hangs" -eq 0 ]
