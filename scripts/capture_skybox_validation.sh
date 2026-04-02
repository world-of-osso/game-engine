#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

output_dir="${1:-data/skybox-validation}"
mkdir -p "$output_dir"

capture_case() {
  local light_skybox_id="$1"
  local filename="$2"
  local description="$3"
  local output_path="$output_dir/$filename"

  echo "Capturing $description -> $output_path"
  cargo run --bin game-engine -- \
    --screen skyboxdebug \
    --light-skybox-id "$light_skybox_id" \
    screenshot "$output_path"
}

capture_case 628 "skyboxdebug-ohnahran-628.webp" \
  "alternate-slot authored skybox"
capture_case 81 "skyboxdebug-global-81.webp" \
  "global/default authored skybox"

echo
echo "Saved validation captures in $output_dir"
