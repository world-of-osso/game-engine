#!/usr/bin/env bash
# Usage: cross-map-teleport.sh <checkout> <server addr> <admin socket> <admin binary> \
#          <account> <character> <map> <x> <y> <z>
# Runs godot/tests/cross_map_teleport.gd in a headless client, teleports the character
# with game-server-admin once it is in the world, and fails if the client log has a
# godot-rust "[panic" or the fixture does not finish.
set -uo pipefail
checkout=$1 server=$2 socket=$3 admin=$4 account=$5 character=$6 map=$7 x=$8 y=$9 z=${10}
log="$checkout/target/headless-client.log"
xdg=$(mktemp -d)
GODOT_TEST_SERVER=$server XMAP_ACCOUNT=$account XMAP_CHARACTER=$character \
  "$checkout/scripts/agent/headless-client.sh" start-godot "$checkout" "$xdg" \
  --display-driver wayland --rendering-driver vulkan --audio-driver Dummy \
  --script res://tests/cross_map_teleport.gd
status=1
teleported=0
for _ in $(seq 1 300); do
  sleep 2
  if [ $teleported = 0 ] && grep -q "FIXTURE AT_START" "$log"; then
    GAME_SERVER_ADMIN_SOCKET=$socket "$admin" teleport "$character" "$map" "$x" "$y" "$z"
    teleported=1
  fi
  if grep -q "FIXTURE XMAP_DONE" "$log"; then status=0; break; fi
  grep -q "FIXTURE FAIL" "$log" && break
  pgrep -F "$checkout/target/headless-client.pid" >/dev/null || break
done
"$checkout/scripts/agent/headless-client.sh" stop "$checkout/target"
rm -rf "$xdg"
if grep -q "\[panic" "$log"; then
  echo "FAIL: client panicked:"; grep -A2 "\[panic" "$log" | head -20; exit 1
fi
grep -E "FIXTURE" "$log"
exit $status
