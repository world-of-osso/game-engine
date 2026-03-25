#!/usr/bin/env bash
set -euo pipefail

SERVER="sakuin"
REMOTE_DIR="/docker-volumes/file-server/data"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Building game-engine (release) ==="
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

PLATFORM="linux-x86_64"
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)   PLATFORM="linux-x86_64" ;;
  Darwin-arm64)   PLATFORM="macos-aarch64" ;;
  Darwin-x86_64)  PLATFORM="macos-x86_64" ;;
  MINGW*|MSYS*|CYGWIN*)  PLATFORM="windows-x86_64" ;;
esac

echo "=== Syncing game-engine binary ($PLATFORM) ==="
ssh "$SERVER" "rm -f $REMOTE_DIR/game-engine-${PLATFORM}*"
scp "$SCRIPT_DIR/target/release/game-engine" "$SERVER:$REMOTE_DIR/game-engine-${PLATFORM}"

echo "=== Syncing data directory ==="
rsync -avz --delete \
    --exclude='*.lock' \
    --exclude='*.webp' \
    --exclude='*.png' \
    --exclude='screenshots/' \
    "$SCRIPT_DIR/data/" "$SERVER:$REMOTE_DIR/data/"

echo "=== Generating manifest on server ==="
ssh "$SERVER" "cd /repos/sakuin/ops && docker compose run --rm file-server manifest /data"

echo "=== Done ==="
echo "Files synced to $SERVER:$REMOTE_DIR"
ssh "$SERVER" "cat $REMOTE_DIR/manifest.json | head -5"
