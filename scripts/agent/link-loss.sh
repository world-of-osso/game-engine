#!/usr/bin/env bash
# Usage: link-loss.sh <checkout> <server addr> <server pid> <account> <character> [rounds]
# Runs godot/tests/link_loss_live.gd in a headless client. Each time it prints
# FIXTURE LINK_READY, stops the (private) server process for 13 s, past the client's 10 s
# netcode timeout, so the client loses its link and reconnects. Fails if the client log has
# a godot-rust "[panic" or the fixture does not finish.
set -uo pipefail
checkout=$1 server=$2 pid=$3 account=$4 character=$5 rounds=${6:-1}
log="$checkout/target/headless-client.log"
xdg=$(mktemp -d)
GODOT_TEST_SERVER=$server LINKLOSS_ACCOUNT=$account LINKLOSS_CHARACTER=$character \
  LINKLOSS_ROUNDS=$rounds RUST_BACKTRACE=1 \
  "$checkout/scripts/agent/headless-client.sh" start-godot "$checkout" "$xdg" \
  --display-driver wayland --rendering-driver vulkan --audio-driver Dummy \
  --script res://tests/link_loss_live.gd
status=1
stalled=0
for _ in $(seq 1 $((200 * rounds))); do
  sleep 1
  ready=$(grep -c "FIXTURE LINK_READY" "$log")
  if [ "$ready" -gt "$stalled" ]; then
    kill -STOP "$pid"; sleep 13; kill -CONT "$pid"
    stalled=$ready
  fi
  if grep -q "FIXTURE LINKLOSS_DONE" "$log"; then status=0; break; fi
  grep -q "FIXTURE FAIL" "$log" && break
  pgrep -F "$checkout/target/headless-client.pid" >/dev/null || break
done
"$checkout/scripts/agent/headless-client.sh" stop "$checkout/target"
rm -rf "$xdg"
if grep -q "\[panic" "$log"; then
  echo "FAIL: client panicked:"; grep -A12 "\[panic" "$log" | head -40; exit 1
fi
grep -E "FIXTURE" "$log"
exit $status
