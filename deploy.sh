#!/usr/bin/env bash
# Build the combined Linux/Windows player manifests and publish them to R2.
# --dry-run performs every local step without reading or writing the live bucket.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")" && pwd)
PLATFORM=linux-x86_64
BUNDLE=${BUNDLE_DIR:-$ROOT/target/deploy/player}
GENERATED="$BUNDLE.generated"
OUTPUT="$BUNDLE.manifests"
PACK_CACHE="$BUNDLE.pack-cache"
DESTINATION=${DESTINATION:-woo-r2:worldofosso-client}

dry_run=0
case "${1:-}" in
    --dry-run|--no-publish) dry_run=1 ;;
    "") ;;
    *) echo "usage: $0 [--dry-run|--no-publish]" >&2; exit 2 ;;
esac

if [ "$(uname -s)-$(uname -m)" != Linux-x86_64 ]; then
    echo "deploy.sh builds the $PLATFORM bundle only; see docs/deploy.md for other platforms" >&2
    exit 1
fi

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
    cat > "$GENERATED/game-engine-$PLATFORM" <<'EOF'
#!/bin/sh
# World of Osso client. Every argument is a client option (--server, --screen, --char).
set -eu
here=$(dirname "$(readlink -f "$0")")
# game-launcher downloads files without their mode bits.
chmod +x "$here/runtime/godot"
export GAME_ENGINE_ASSET_MODE=extracted-only
exec "$here/runtime/godot" --path "$here/godot" -- "$@"
EOF
    chmod +x "$GENERATED/game-engine-$PLATFORM"
}

import_project() {
    python3 "$ROOT/scripts/import_project.py" --root "$ROOT" --generated "$GENERATED"
}

assemble_generated() {
    rm -rf "$GENERATED"
    mkdir -p "$GENERATED/runtime" "$GENERATED/lib"
    write_launcher
    cp "$(pinned_godot)" "$GENERATED/runtime/godot"
    install -m 644 "$ROOT/target/release/libgame_engine_godot.so" "$GENERATED/lib/"
    python3 "$ROOT/scripts/player_bundle.py" --root "$ROOT" --bundle "$GENERATED" \
        --platform "$PLATFORM" --project-only
    import_project
}

echo "=== Building Linux native extension (release, bookworm) ==="
${DEPLOY_BUILD_WRAPPER:-} python3 "$ROOT/scripts/depot-build.py" --root "$ROOT" --release --build-host local
echo "=== Building Windows runtime, extension and entry (native desktop) ==="
python3 "$ROOT/scripts/windows-release.py" --root "$ROOT" --phase build
echo "=== Preparing project import ==="
assemble_generated
echo "=== Building launcher pack chunker ==="
cargo build --release --manifest-path "$ROOT/Cargo.toml" -p launcher-pack-chunks

args=(--root "$ROOT" --generated "$GENERATED" --windows-artifacts "$ROOT/target/windows-release/artifacts"
      --output "$OUTPUT" --pack-cache "$PACK_CACHE")
if [ "$dry_run" = 0 ]; then
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
    if ! top=$(rclone lsf "$DESTINATION" --max-depth 1); then
        echo "cannot list $DESTINATION; configure the rclone remote (docs/deploy.md)" >&2
        exit 1
    fi
    if grep -qx manifest.json <<<"$top"; then
        rclone copyto "$DESTINATION/manifest.json" "$work/live-manifest.json"
    fi
    args+=(--destination "$DESTINATION" --live-manifest "$work/live-manifest.json")
fi
python3 "$ROOT/scripts/mapped_deploy.py" "${args[@]}"
echo "=== Done ==="
