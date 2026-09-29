#!/usr/bin/env python3
"""Make staged compile inputs newer than a shared Cargo target after acquiring its lock."""

import os
from pathlib import Path
import sys


EXCLUDED_DIRS = {".git", ".godot", "data", "target"}


def refresh_sources(context):
    inputs = []
    for repo in context.iterdir():
        if not repo.is_dir() or repo.name in EXCLUDED_DIRS or repo.name.startswith("."):
            continue
        if repo.is_symlink():
            raise ValueError(f"unsupported source symlink: {repo}")
        for directory, dirs, files in os.walk(repo):
            base = Path(directory)
            for name in dirs[:]:
                if name in EXCLUDED_DIRS or name.startswith("."):
                    dirs.remove(name)
                elif (base / name).is_symlink():
                    raise ValueError(f"unsupported source symlink: {base / name}")
            for name in files:
                path = base / name
                if path.is_symlink():
                    raise ValueError(f"unsupported source symlink: {path}")
                inputs.append(path)
    for path in inputs:
        os.utime(path, None, follow_symlinks=False)


if __name__ == "__main__":
    try:
        refresh_sources(Path(sys.argv[1]))
    except (OSError, ValueError) as error:
        sys.exit(str(error))
