#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]] && command -v xvfb-run >/dev/null 2>&1; then
  xvfb-run -a cargo test --test screenshot_regression renders_known_model_matches_golden -- --ignored --exact
else
  cargo test --test screenshot_regression renders_known_model_matches_golden -- --ignored --exact
fi
