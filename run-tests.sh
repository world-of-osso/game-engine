#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo test "$@"
cargo clippy -- -D warnings
dx fmt --check
