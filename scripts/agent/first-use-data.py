#!/usr/bin/env python3
"""Make a worktree's next client run a first use of the given model and texture FDIDs
and of every spell sound. data/models and data/textures become local directories of
per-file symlinks to the canonical data (minus those FDIDs), and data/sounds gets an
empty local spells/, so the client extracts them from local CASC into the worktree only.

Usage: first-use-data.py <worktree> isolate [--models FDID...] [--textures FDID...]
       first-use-data.py <worktree> reset   (delete what a run extracted there)
"""
import argparse
import os

CANONICAL = "/syncthing/Sync/Projects/world-of-osso/game-engine/data"
DIRS = ("models", "textures", "sounds/spells")


def excluded(sub, name, models, textures):
    stem = name.split(".")[0]
    if sub == "models":
        # {fdid}.m2, {fdid}00.skin, {fdid}.skel
        return any(stem in (m, m + "00") for m in models)
    if sub == "textures":
        return stem in textures
    return False


def mirror(data, sub, models, textures, empty=()):
    local = os.path.join(data, sub)
    if os.path.islink(local):
        os.unlink(local)
    os.makedirs(local, exist_ok=True)
    source = os.path.join(CANONICAL, sub)
    for name in os.listdir(source):
        path = os.path.join(local, name)
        if os.path.lexists(path) or excluded(sub, name, models, textures):
            continue
        if name in empty:
            os.mkdir(path)
        else:
            os.symlink(os.path.join(source, name), path)


def reset(data):
    removed = 0
    for sub in DIRS:
        local = os.path.join(data, sub)
        for name in os.listdir(local):
            path = os.path.join(local, name)
            if os.path.isfile(path) and not os.path.islink(path):
                os.unlink(path)
                removed += 1
    print(f"removed {removed} extracted files")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("worktree")
    parser.add_argument("action", choices=("isolate", "reset"))
    parser.add_argument("--models", nargs="*", default=[])
    parser.add_argument("--textures", nargs="*", default=[])
    args = parser.parse_args()
    data = os.path.join(args.worktree, "data")
    if os.path.realpath(data) == os.path.realpath(CANONICAL):
        raise SystemExit("refusing to change the canonical data/")
    if args.action == "reset":
        reset(data)
        return
    mirror(data, "models", args.models, args.textures)
    mirror(data, "textures", args.models, args.textures)
    mirror(data, "sounds", args.models, args.textures, empty=("spells",))
    print("isolated")


if __name__ == "__main__":
    main()
