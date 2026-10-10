#!/usr/bin/env bash
# Seed a new agent CARGO_TARGET_DIR as a btrfs reflink (copy-on-write) clone of a warm one,
# so the agent recompiles only what its branch changes instead of all of Bevy.
# Usage: seed-target.sh <repo: game-engine|game-server|shared-protocol> <new-target-dir> [source-target]
set -euo pipefail
repo="$1"; dest="$2"
[ -e "$dest" ] && { echo "exists: $dest"; exit 1; }
# Warmest = most recently modified existing target for the same repo.
src=${3:-$(ls -dt /home/osso/.worktrees/.target-"$repo"-* 2>/dev/null | grep -v -- '-client-' | head -1 || true)}
[ -n "$src" ] || { echo "no warm $repo target to seed from; build fresh"; exit 2; }
# Refuse to clone a target that a cargo process is writing right now (lock held).
if [ -e "$src/debug/.cargo-lock" ] && fuser "$src/debug/.cargo-lock" >/dev/null 2>&1; then
  echo "warm target busy: $src (wait for its build to finish)"; exit 3
fi
cp -a --reflink=always "$src" "$dest"
echo "seeded $dest from $src"
