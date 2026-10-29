#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo test --workspace "$@"
cargo clippy --workspace --all-targets -- -D warnings
# --all would also format game-engine-core's godot workspace, whose tests run through scripts/engine-build.py.
cargo fmt -p game-engine-launcher -p game-engine-tools --check
