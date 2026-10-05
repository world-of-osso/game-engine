#!/usr/bin/env python3
"""Restore content-aware source timestamps under the locked Cargo target cache."""

import hashlib
import json
import os
from pathlib import Path
import sys
import time


EXCLUDED_DIRS = {".git", ".godot", "data", "target"}


def refresh_sources(context, target):
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
                inputs.append((repo, path))
    state = target / "source-mtimes.json"
    previous = json.loads(state.read_text()) if state.exists() else {}
    # Restaged files may have older mtimes even when their contents changed.
    # Remember both content and timestamp so unchanged layers remain Cargo-fresh.
    changed_mtime = max(
        time.time_ns(),
        max((item[1] for item in previous.values()), default=0) + 1,
    )
    current = {}
    for repo, path in inputs:
        # Relative to the repo's parent: the same keys as relative to a single context
        # root, and also valid when the caller lists repos individually (desktop-server).
        key = str(path.relative_to(repo.parent))
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        saved = previous.get(key)
        mtime = saved[1] if saved is not None and saved[0] == digest else changed_mtime
        os.utime(path, ns=(mtime, mtime), follow_symlinks=False)
        current[key] = [digest, mtime]
    target.mkdir(parents=True, exist_ok=True)
    temporary = state.with_suffix(".tmp")
    temporary.write_text(json.dumps(current, sort_keys=True))
    temporary.replace(state)


if __name__ == "__main__":
    try:
        refresh_sources(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError) as error:
        sys.exit(str(error))
