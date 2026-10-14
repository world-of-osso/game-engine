#!/usr/bin/env bash
# Builds the combined Linux/Windows x86_64 Godot player bundle and publishes it to R2
# behind https://files.worldofosso.com, where game-launcher reads it.
# See docs/deploy.md for the bundle layout and the game-launcher contract.
#
# Usage: ./deploy.sh [--dry-run]
#   DEPLOY_BUILD_WRAPPER  optional command prefix for the release build step only.
#   --dry-run  build and assemble the bundle, then print the publish commands instead of
#              running them.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")" && pwd)
PLATFORM=linux-x86_64
BUNDLE=${BUNDLE_DIR:-$ROOT/target/deploy/player}
FILE_SERVER_DIR=${FILE_SERVER_DIR:-$ROOT/../file-server}
# rclone remote with write access to the bucket (docs/deploy.md).
DESTINATION=${DESTINATION:-woo-r2:worldofosso-client}

# Runtime project/data allowlist is shared with Windows in scripts/player_bundle.py.

dry_run=0
case "${1:-}" in
    --dry-run) dry_run=1 ;;
    "") ;;
    *) echo "usage: $0 [--dry-run]" >&2; exit 2 ;;
esac

if [ "$(uname -s)-$(uname -m)" != Linux-x86_64 ]; then
    echo "deploy.sh builds the $PLATFORM bundle only; see docs/deploy.md for other platforms" >&2
    exit 1
fi

# pinned_godot: the patched Godot the launcher pins, checksum-verified.
pinned_godot() {
    . "$ROOT/scripts/godot/pinned.sh"
    local expected
    expected=$(cat "$ROOT/scripts/godot/godot-$GODOT_PINNED_VERSION.sha512")
    if [ ! -x "$GODOT_PINNED" ] || [ "$(sha512sum "$GODOT_PINNED" | cut -d' ' -f1)" != "$expected" ]; then
        echo "Godot $GODOT_PINNED_VERSION missing or not the pinned build at $GODOT_PINNED;" \
            "run scripts/godot/build-patched-godot.sh" >&2
        exit 1
    fi
    echo "$GODOT_PINNED"
}

write_launcher() {
    cat > "$BUNDLE/game-engine-$PLATFORM" <<'EOF'
#!/bin/sh
# World of Osso client. Every argument is a client option (--server, --screen, --char).
set -eu
here=$(dirname "$(readlink -f "$0")")
# game-launcher downloads files without their mode bits.
chmod +x "$here/runtime/godot"
export GAME_ENGINE_ASSET_MODE=extracted-only
exec "$here/runtime/godot" --path "$here/godot" -- "$@"
EOF
    chmod +x "$BUNDLE/game-engine-$PLATFORM"
}

# import_project: run Godot's one-time import in the bundle so players never import.
# Isolated XDG dirs keep the import off the builder's Godot settings. The extension is
# registered first so Godot loads it at startup. The 134 aborts seen here were Godot's
# ClassDB race, fixed by the pin (docs/wiki/investigations/godot-cold-import-crash.md).
import_project() {
    local home
    mkdir -p "$BUNDLE/godot/.godot"
    echo res://game_engine.gdextension > "$BUNDLE/godot/.godot/extension_list.cfg"
    home=$(mktemp -d)
    XDG_CONFIG_HOME=$home/config XDG_DATA_HOME=$home/data XDG_CACHE_HOME=$home/cache \
        "$BUNDLE/runtime/godot" --headless --path "$BUNDLE/godot" --import
    rm -rf "$home" "$BUNDLE/godot/.godot/editor"
}

assemble() {
    rm -rf "$BUNDLE"
    mkdir -p "$BUNDLE/runtime" "$BUNDLE/lib"
    write_launcher
    cp "$(pinned_godot)" "$BUNDLE/runtime/godot"
    install -m 644 "$ROOT/target/release/libgame_engine_godot.so" "$BUNDLE/lib/"
    python3 "$ROOT/scripts/player_bundle.py" --root "$ROOT" --bundle "$BUNDLE" --platform "$PLATFORM" \
        --windows-artifacts "$ROOT/target/windows-release/artifacts"
    import_project
}

# run: execute, or under --dry-run print the exact command.
run() {
    if [ "$dry_run" = 1 ]; then
        printf '+'
        printf ' %q' "$@"
        printf '\n'
    else
        "$@"
    fi
}

# publish: files first, then the manifest, then deletions, so the live manifest never
# names a missing file. no-cache because paths keep their names across deploys.
publish() {
    if [ ! -f "$FILE_SERVER_DIR/Cargo.toml" ]; then
        echo "file-server checkout not found at $FILE_SERVER_DIR (set FILE_SERVER_DIR)" >&2
        exit 1
    fi
    if [ "$dry_run" = 0 ] && ! rclone lsf "$DESTINATION" --max-depth 1 >/dev/null; then
        echo "cannot list $DESTINATION; configure the rclone remote (docs/deploy.md)" >&2
        exit 1
    fi
    local upload=(--header-upload "Cache-Control: no-cache" --transfers 8 --checkers 16)
    echo "=== Generating manifest ==="
    run cargo run --release --quiet --manifest-path "$FILE_SERVER_DIR/Cargo.toml" -- manifest "$BUNDLE"
    echo "=== Verifying staged bytes ==="
    run python3 "$ROOT/scripts/player_bundle.py" --root "$ROOT" --bundle "$BUNDLE" --platform "$PLATFORM" --verify-manifest
    echo "=== Uploading bundle to $DESTINATION ==="
    run rclone copy "$BUNDLE/" "$DESTINATION" --exclude /manifest.json "${upload[@]}" --progress
    echo "=== Publishing manifest ==="
    run rclone copyto "$BUNDLE/manifest.json" "$DESTINATION/manifest.json" "${upload[@]}"
    echo "=== Deleting files the manifest no longer names ==="
    run rclone sync "$BUNDLE/" "$DESTINATION" "${upload[@]}"
}

echo "=== Building Linux native extension (release, bookworm) ==="
# DEPLOY_BUILD_WRAPPER (optional): a command prefix for the build only, e.g. a host build
# lock, so uploads never hold it.
${DEPLOY_BUILD_WRAPPER:-} python3 "$ROOT/scripts/depot-build.py" --root "$ROOT" --release --build-host local
echo "=== Building Windows runtime, extension and entry (native desktop) ==="
python3 "$ROOT/scripts/windows-release.py" --root "$ROOT" --phase build
echo "=== Assembling $BUNDLE ==="
assemble
echo "Bundle: $(du -sh --apparent-size "$BUNDLE" | cut -f1), $(find "$BUNDLE" -type f | wc -l) files"
publish
echo "=== Done ==="
