#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

shared_data_dir="${GAME_ENGINE_SHARED_DATA_DIR:-$ROOT/data}"
managed_repo_paths=(
  "data/casc/root.bin"
  "data/casc/encoding.bin"
  "data/casc/resolution.sqlite"
  "data/music_manifest.csv"
  "data/music_zone_links.csv"
  "data/music_zone_index.json"
  "data/textures/145513.blp"
  "data/textures/4219004.blp"
  "data/textures/4239595.blp"
  "data/textures/4226685.blp"
)

echo "Refreshing shared asset data in: $shared_data_dir"

cargo run --features casc-tools --bin casc_refresh
cargo run --features casc-tools --bin casc-local -- 145513 4219004 4239595 4226685 -o data/textures
GAME_ENGINE_SHARED_DATA_DIR="$shared_data_dir" python3 scripts/generate_music_manifest.py

echo
echo "Changed asset files:"
case "$shared_data_dir" in
  "$ROOT"/*)
    git status --short --ignored=matching -- "${managed_repo_paths[@]}" || true
    ;;
  *)
    echo "shared data dir is outside the repo checkout; skipping git status"
    ;;
esac
