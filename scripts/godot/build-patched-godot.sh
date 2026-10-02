#!/usr/bin/env bash
# Builds the Godot the launcher pins: 4.7.2-stable plus upstream PR #123946 (Wayland exit
# hang, issue #123059; docs/wiki/investigations/godot-wayland-exit-hang.md), installed as
# $GODOT_PINNED (scripts/godot/pinned.sh) only when its SHA-512 matches
# scripts/godot/godot-$GODOT_PINNED_VERSION.sha512.
#
# Toolchain and options follow the official 4.7.2 Linux editor release:
# godot-build-scripts 213defb build-linux/build.sh (production=yes, accesskit-c 0.22.3)
# in build-containers aaef187 (buildroot SDK godot-2023.08.x-4, GCC 13.2, SCons 4.10.1).
#
# Retire this script, the patch and the pin once the launcher moves to an official Godot
# release that contains #123946.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
. "$here/pinned.sh"
tag=4.7.2-stable
tag_commit=ed1daf0bf001b61586d9930840f2f1394092c079
sdk_url=https://github.com/godotengine/buildroot/releases/download/godot-2023.08.x-4/x86_64-godot-linux-gnu_sdk-buildroot.tar.bz2
sdk_sha256=b89f173b1f2f2f35f3090bc4efad732f1c0c1c1206e5480cfcb141977010bc94
accesskit_url=https://github.com/godotengine/godot-accesskit-c-static/releases/download/0.22.3/accesskit-c-0.22.3.zip
accesskit_sha256=41b30b1382d03b225b6f7e8721269a32557a68065804f3797bdffc34a3273498
expected=$(cat "$here/godot-$GODOT_PINNED_VERSION.sha512")
work=${XDG_CACHE_HOME:-$HOME/.cache}/game-engine/godot-build
mkdir -p "$work/downloads"

# fetch <url> <sha256>: download once into $work/downloads, verified.
fetch() {
  local file="$work/downloads/${1##*/}"
  if [ ! -f "$file" ]; then
    curl --fail --location --silent --show-error --output "$file.partial" "$1"
    mv "$file.partial" "$file"
  fi
  echo "$2  $file" | sha256sum --check --quiet >&2
  echo "$file"
}

sdk=$work/x86_64-godot-linux-gnu_sdk-buildroot
if [ ! -f "$sdk/.relocated" ]; then
  rm -rf "$sdk"
  tar -xjf "$(fetch "$sdk_url" "$sdk_sha256")" -C "$work"
  (cd "$sdk" && ./relocate-sdk.sh && ln -sf x86_64-godot-linux-gnu-objcopy bin/objcopy \
    && ln -sf x86_64-godot-linux-gnu-strip bin/strip)
  touch "$sdk/.relocated"
fi

accesskit=$work/accesskit/accesskit-c
if [ ! -d "$accesskit" ]; then
  rm -rf "$work/accesskit"
  mkdir -p "$work/accesskit"
  unzip -q "$(fetch "$accesskit_url" "$accesskit_sha256")" -d "$work/accesskit"
  mv "$work"/accesskit/accesskit-c-* "$accesskit"
fi

src=$work/godot-$tag
rm -rf "$src"
git clone --quiet --depth 1 --branch "$tag" https://github.com/godotengine/godot.git "$src"
[ "$(git -C "$src" rev-parse HEAD)" = "$tag_commit" ] || {
  echo "$tag is not $tag_commit" >&2
  exit 1
}
git -C "$src" apply "$here/pr123946-wayland-exit-hang.patch"

# PYTHONHASHSEED: editor/editor_builders.py embeds Python hash() of the docs, randomized
# per process otherwise, so the binary would differ on every build.
(cd "$src" && PATH="$sdk/bin:$PATH" BUILD_NAME=pr123946 PYTHONHASHSEED=0 uvx --from scons==4.10.1 scons \
  -j"$(nproc)" verbose=yes warnings=no progress=no redirect_build_objects=no \
  platform=linuxbsd arch=x86_64 production=yes accesskit_sdk_path="$accesskit" target=editor)

built=$src/bin/godot.linuxbsd.editor.x86_64
actual=$(sha512sum "$built" | cut -d' ' -f1)
if [ "$actual" != "$expected" ]; then
  echo "built $built has SHA-512 $actual, pinned $expected; not installed" >&2
  exit 1
fi
install -D --mode=755 "$built" "$GODOT_PINNED"
rm -rf "$src"
echo "installed $GODOT_PINNED"
