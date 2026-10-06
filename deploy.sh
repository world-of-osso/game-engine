#!/usr/bin/env bash
# Builds the Linux x86_64 Godot client bundle and publishes it to the public R2 bucket
# behind https://files.worldofosso.com, where game-launcher reads it.
# See docs/deploy.md for the bundle layout and the game-launcher contract.
#
# Usage: ./deploy.sh [--dry-run]
#   --dry-run  build and assemble the bundle, then print the publish commands instead of
#              running them.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")" && pwd)
PLATFORM=linux-x86_64
BUNDLE=${BUNDLE_DIR:-$ROOT/target/deploy/$PLATFORM}
FILE_SERVER_DIR=${FILE_SERVER_DIR:-$ROOT/../file-server}
# rclone remote with write access to the bucket (docs/deploy.md).
DESTINATION=${DESTINATION:-woo-r2:worldofosso-client}

# Runtime data: what the client reads from data/ that a player's WoW install cannot supply.
# An allowlist, because data/ also holds auth tokens and gigabytes of diagnostics.
DATA_DIRS=(cache campsite-ui db2 dbfilesclient fogs fonts forever-1.60.1.70205 glues item-models
    los minimap models music reference sounds terrain textures ui)
# Not tactkeys/: the third-party TACT key list stays off the public bucket; the resolver
# treats it as optional and falls back to the install's own keyring (casc_resolver.rs).
DATA_FILE_GLOBS=('*.csv' '*.ron' local-listfile-cache.sqlite)
# Inside those: negative-extraction markers, lock files, superseded resolver/NPC caches.
DATA_EXCLUDES=('*.missing' '*.lock' cache/casc 'cache/pre-retail-*')

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

# project_files: tracked Godot project files, without the Rust crates, tests, and the dev
# .gdextension that points into target/.
project_files() {
    local crates
    crates=$(git -C "$ROOT" ls-files 'godot/*/Cargo.toml' | sed 's|Cargo.toml$||')
    git -C "$ROOT" ls-files godot | grep -v -F "$crates" \
        | grep -v -E '^godot/(tests/|Cargo\.|\.gitignore$|depot-test-assets\.txt$|game_engine\.gdextension)'
}

write_launcher() {
    cat > "$BUNDLE/game-engine-$PLATFORM" <<'EOF'
#!/bin/sh
# World of Osso client. Every argument is a client option (--server, --screen, --char).
set -eu
here=$(dirname "$(readlink -f "$0")")
# game-launcher downloads files without their mode bits.
chmod +x "$here/runtime/godot"
exec "$here/runtime/godot" --path "$here/godot" -- "$@"
EOF
    chmod +x "$BUNDLE/game-engine-$PLATFORM"
}

write_extension_config() {
    cat > "$BUNDLE/godot/game_engine.gdextension" <<'EOF'
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = "4.7"
reloadable = false

[libraries]
linux.x86_64 = "res://../lib/libgame_engine_godot.so"
EOF
}

copy_project() {
    (cd "$ROOT" && project_files | xargs -d '\n' cp --parents -t "$BUNDLE")
    write_extension_config
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

# copy_data: -L because a worktree's data/ links into the canonical checkout's.
copy_data() {
    local entry pattern
    mkdir -p "$BUNDLE/data"
    for entry in "${DATA_DIRS[@]}"; do
        cp -RLp --reflink=auto "$ROOT/data/$entry" "$BUNDLE/data/"
    done
    shopt -s nullglob
    for pattern in "${DATA_FILE_GLOBS[@]}"; do
        for entry in "$ROOT"/data/$pattern; do
            cp -Lp --reflink=auto "$entry" "$BUNDLE/data/"
        done
    done
    shopt -u nullglob
    for pattern in "${DATA_EXCLUDES[@]}"; do
        case "$pattern" in
            */*) rm -rf "$BUNDLE"/data/$pattern ;;
            *) find "$BUNDLE/data" -name "$pattern" -delete ;;
        esac
    done
}

assemble() {
    rm -rf "$BUNDLE"
    mkdir -p "$BUNDLE/runtime" "$BUNDLE/lib"
    write_launcher
    cp "$(pinned_godot)" "$BUNDLE/runtime/godot"
    install -m 644 "$ROOT/target/release/libgame_engine_godot.so" "$BUNDLE/lib/"
    copy_project
    copy_data
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
    echo "=== Uploading bundle to $DESTINATION ==="
    run rclone copy "$BUNDLE/" "$DESTINATION" --exclude /manifest.json "${upload[@]}" --progress
    echo "=== Publishing manifest ==="
    run rclone copyto "$BUNDLE/manifest.json" "$DESTINATION/manifest.json" "${upload[@]}"
    echo "=== Deleting files the manifest no longer names ==="
    run rclone sync "$BUNDLE/" "$DESTINATION" "${upload[@]}"
}

echo "=== Building native extension (release, Depot) ==="
python3 "$ROOT/scripts/depot-build.py" --root "$ROOT" --release
echo "=== Assembling $BUNDLE ==="
assemble
echo "Bundle: $(du -sh --apparent-size "$BUNDLE" | cut -f1), $(find "$BUNDLE" -type f | wc -l) files"
publish
echo "=== Done ==="
