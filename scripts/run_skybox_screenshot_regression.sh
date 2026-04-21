#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]] && command -v xvfb-run >/dev/null 2>&1; then
  xvfb-run -a cargo test --test skybox_screenshot_regression -- --ignored
else
  cargo test --test skybox_screenshot_regression -- --ignored
fi
