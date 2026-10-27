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

# publish: changed legacy files first, then its manifest; immutable packs are
# uploaded and checked before publishing the v2 manifest. Keep removed legacy files
# and old packs for clients using an older manifest. Only changed legacy paths are
# uploaded: a full-tree
# rclone copy/sync of the 1.6M-file bundle spent most of a day listing and comparing
# (2026-10-14). no-cache because paths keep their names across deploys.
publish() {
    if [ ! -f "$FILE_SERVER_DIR/Cargo.toml" ]; then
        echo "file-server checkout not found at $FILE_SERVER_DIR (set FILE_SERVER_DIR)" >&2
        exit 1
    fi
    local top=""
    if [ "$dry_run" = 0 ] && ! top=$(rclone lsf "$DESTINATION" --max-depth 1); then
        echo "cannot list $DESTINATION; configure the rclone remote (docs/deploy.md)" >&2
        exit 1
    fi
    local upload=(--header-upload "Cache-Control: no-cache" --transfers 48 --checkers 16 --low-level-retries 20)
    local work
    work=$(mktemp -d)
    trap 'rm -rf "$work"' RETURN
    echo "=== Generating manifest ==="
    # The digest cache lives beside the bundle (assemble recreates the bundle itself) and
    # skips re-reading the unchanged ~197 GB on every publish.
    run cargo run --release --quiet --manifest-path "$FILE_SERVER_DIR/Cargo.toml" -- manifest "$BUNDLE" \
        --cache "$BUNDLE.digest-cache.json"
    echo "=== Diffing against the live manifest ==="
    if grep -qx manifest.json <<<"$top"; then
        rclone copyto "$DESTINATION/manifest.json" "$work/live-manifest.json"
    fi
    run python3 "$ROOT/scripts/manifest_diff.py" --new "$BUNDLE/manifest.json" \
        --live "$work/live-manifest.json" --upload "$work/upload.txt" --delete "$work/delete.txt"
    echo "=== Verifying staged bytes of the files to upload ==="
    run python3 "$ROOT/scripts/player_bundle.py" --root "$ROOT" --bundle "$BUNDLE" --platform "$PLATFORM" \
        --verify-manifest --only "$work/upload.txt"
    echo "=== Building immutable packs from the verified manifest ==="
    run cargo build --release --manifest-path "$ROOT/Cargo.toml" -p launcher-pack-chunks
    local pack_cache="$BUNDLE.pack-cache"
    run python3 "$ROOT/scripts/launcher_packs.py" "$BUNDLE" "$pack_cache" --manifest "$BUNDLE/manifest.json"
    echo "=== Uploading changed files to $DESTINATION ==="
    run rclone copy "$BUNDLE/" "$DESTINATION" --files-from "$work/upload.txt" --no-traverse \
        "${upload[@]}" --stats 60s --stats-one-line
    echo "=== Publishing manifest ==="
    run rclone copyto "$BUNDLE/manifest.json" "$DESTINATION/manifest.json" "${upload[@]}"
    echo "=== Publishing and checking immutable packs ==="
    run rclone copy "$pack_cache/packs/" "$DESTINATION/packs/" --immutable \
        --transfers 48 --checkers 16 --low-level-retries 20
    run rclone check "$pack_cache/packs/" "$DESTINATION/packs/" --one-way --size-only \
        --checkers 16 --low-level-retries 20
    echo "=== Publishing v2 manifest ==="
    run rclone copyto "$pack_cache/manifest-v2.json" "$DESTINATION/manifest-v2.json" "${upload[@]}"
    echo "=== Retaining old legacy files and packs for existing clients ==="
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
