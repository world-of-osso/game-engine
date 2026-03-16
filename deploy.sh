#!/usr/bin/env bash
set -euo pipefail

SERVER="sakuin"
REMOTE_DIR="/docker-volumes/file-server/data"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Building game-engine (release) ==="
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

echo "=== Syncing game-engine binary ==="
scp "$SCRIPT_DIR/target/release/game-engine" "$SERVER:$REMOTE_DIR/game-engine"

echo "=== Syncing data directory ==="
rsync -avz --delete \
    --exclude='*.lock' \
    --exclude='debug_*.webp' \
    "$SCRIPT_DIR/data/" "$SERVER:$REMOTE_DIR/data/"

echo "=== Generating manifest on server ==="
ssh "$SERVER" "cd /repos/sakuin/ops && docker compose run --rm file-server manifest /data"

echo "=== Done ==="
echo "Files synced to $SERVER:$REMOTE_DIR"
ssh "$SERVER" "cat $REMOTE_DIR/manifest.json | head -5"
