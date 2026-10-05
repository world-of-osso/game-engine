#!/bin/sh
# (Re)create the shared `game-engine` buildx builder with the checked-in BuildKit config.
# Keeps its cache volume (--keep-state). Run while holding the build lock so no build is
# in flight: ~/.worktrees/build-lock.sh scripts/setup-builder.sh
set -eu
here=$(cd "$(dirname "$0")" && pwd)
docker buildx rm --keep-state game-engine 2>/dev/null || true
docker buildx create --name game-engine --driver docker-container \
  --driver-opt cpu-period=100000 --driver-opt cpu-quota=800000 --driver-opt memory=10g \
  --buildkitd-config "$here/depot/buildkitd.toml" --bootstrap
docker buildx inspect game-engine | sed -n '/GC Policy/,$p'
