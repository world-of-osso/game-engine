#!/usr/bin/env bash
# Seed a new agent CARGO_TARGET_DIR as a reflink (copy-on-write) clone of a warm one,
# so the agent recompiles only what its branch changes. Only works where the filesystem
# supports reflinks (btrfs, XFS). On others (ext4) it exits 4 without copying: a full copy
# costs 50-130 GiB per target and filled the agent host's disk on 2026-10-10.
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
probe_dir=$(dirname "$dest")
probe=$(mktemp "$probe_dir/.reflink-probe.XXXXXX")
trap 'rm -f "$probe" "$probe.clone"' EXIT
if ! cp --reflink=always "$probe" "$probe.clone" 2>/dev/null; then
  echo "no reflink support on $(df --output=target "$probe_dir" | tail -1) ($(findmnt -no FSTYPE -T "$probe_dir")):" \
    "build cold in $dest instead; never copy a target with cp or --reflink=auto" >&2
  exit 4
fi
cp -a --reflink=always "$src" "$dest"
echo "seeded $dest from $src"
