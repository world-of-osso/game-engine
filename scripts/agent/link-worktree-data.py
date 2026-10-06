#!/usr/bin/env python3
"""Share untracked data directories; repair idle slots without losing local assets."""

import argparse
import filecmp
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


PROC_ROOT = Path("/proc")


def is_private(name):
    # Root-level runtime state remains slot-owned. Sidecars inside shared cache/
    # must stay beside the shared SQLite database.
    return name.startswith("auth_token") or name.endswith(("-wal", "-shm", "-journal"))


def tracked_data(worktree):
    result = subprocess.run(
        ["git", "-C", str(worktree), "ls-files", "-z", "--", "data/"],
        check=True,
        capture_output=True,
    )
    return {Path(os.fsdecode(name)) for name in result.stdout.split(b"\0") if name}


def plan_links(source, destination, relative, tracked):
    """Recurse only into directory ancestors of tracked files."""
    actions = []
    names = {entry.name for entry in source.iterdir()} if source.exists() else set()
    if destination.exists():
        names.update(entry.name for entry in destination.iterdir())
    for name in sorted(names):
        if relative == Path("data") and is_private(name):
            continue
        src, dst, rel = source / name, destination / name, relative / name
        if rel in tracked:
            continue
        if any(rel in path.parents for path in tracked):
            if dst.is_symlink():
                raise ValueError(f"tracked subtree must be local: {dst}")
            actions.extend(plan_links(src, dst, rel, tracked))
            continue
        if dst.is_symlink():
            if dst.resolve() != src.resolve():
                raise ValueError(f"link points elsewhere: {dst} -> {dst.readlink()}")
            continue
        if src.is_dir() or dst.is_dir():
            if src.exists() and not src.is_dir():
                raise ValueError(f"canonical directory conflicts with file: {src}")
            if dst.exists() and not dst.is_dir():
                raise ValueError(f"slot directory conflicts with file: {dst}")
            actions.append((src, dst, True))
        else:
            actions.append((src, dst, False))
    return actions


def refuse_open_files(directories):
    """Fail closed when descriptors cannot be inspected; ignore exited processes."""
    roots = [str(path) for path in directories]
    for process in PROC_ROOT.iterdir():
        if not process.name.isdigit():
            continue
        try:
            descriptors = list((process / "fd").iterdir())
            for descriptor in descriptors:
                try:
                    target = os.readlink(descriptor).removesuffix(" (deleted)")
                except FileNotFoundError:
                    continue
                if any(
                    target == root or target.startswith(root + "/") for root in roots
                ):
                    raise ValueError(
                        f"refusing repair: PID {process.name} holds {target} open"
                    )
        except (FileNotFoundError, ProcessLookupError):
            continue
        except PermissionError as error:
            raise ValueError(
                f"cannot inspect PID {process.name} descriptors; repair requires "
                "permission to read every /proc/*/fd"
            ) from error


def move_without_overwrite(source, destination):
    """Copy exclusively (also across filesystems), then remove the slot file."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        output = destination.open("xb")
    except FileExistsError:
        if destination.is_file() and filecmp.cmp(source, destination, shallow=False):
            source.unlink()
        else:
            print(f"CONFLICT: {source} differs from {destination}")
        return
    try:
        with output, source.open("rb") as input_file:
            shutil.copyfileobj(input_file, output)
        shutil.copystat(source, destination)
    except BaseException:
        destination.unlink()
        raise
    source.unlink()
    print(f"moved: {source} -> {destination}")


def merge_directory(source, destination):
    destination.mkdir(parents=True, exist_ok=True)
    for local in sorted(source.iterdir()):
        canonical = destination / local.name
        if local.is_symlink() and local.resolve() == canonical.resolve():
            local.unlink()
        elif local.is_dir():
            if local.is_symlink() or (canonical.exists() and not canonical.is_dir()):
                print(f"CONFLICT: {local} differs from {canonical}")
                continue
            merge_directory(local, canonical)
            if not any(local.iterdir()):
                local.rmdir()
        elif local.is_file():
            move_without_overwrite(local, canonical)
        else:
            raise ValueError(f"unsupported asset entry: {local}")


def apply_links(actions, worktree, repair):
    conflicts = None
    for source, destination, directory in actions:
        destination.parent.mkdir(parents=True, exist_ok=True)
        if directory:
            source.mkdir(parents=True, exist_ok=True)
            if destination.exists():
                merge_directory(destination, source)
                if any(destination.iterdir()):
                    if conflicts is None:
                        conflicts = Path(
                            tempfile.mkdtemp(prefix="data-repair-conflicts-", dir=worktree)
                        )
                    saved = conflicts / destination.relative_to(worktree / "data")
                    saved.parent.mkdir(parents=True, exist_ok=True)
                    destination.rename(saved)
                    print(f"conflicts preserved: {saved}")
                else:
                    destination.rmdir()
        elif destination.exists():
            if not repair:
                continue
            move_without_overwrite(destination, source)
            if destination.exists():
                continue
        destination.symlink_to(source, target_is_directory=directory)
        print(f"linked: {destination} -> {source}")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("canonical", type=Path)
    parser.add_argument("worktree", type=Path)
    parser.add_argument(
        "--repair", action="store_true",
        help="merge idle real directories before linking",
    )
    args = parser.parse_args(argv)
    canonical, worktree = args.canonical.resolve(), args.worktree.resolve()
    try:
        if canonical == worktree or (worktree / "data").is_symlink():
            raise ValueError(
                "worktree must have its own data/ root, distinct from canonical"
            )
        if not (canonical / "data").is_dir():
            raise ValueError(f"canonical data/ is missing: {canonical}")
        actions = plan_links(
            canonical / "data", worktree / "data", Path("data"), tracked_data(worktree)
        )
        real_directories = [
            dst for _, dst, directory in actions if directory and dst.is_dir()
        ]
        if real_directories and not args.repair:
            raise ValueError(
                f"real asset directory requires --repair: {real_directories[0]}"
            )
        if args.repair:
            local_files = [
                dst for _, dst, directory in actions if not directory and dst.exists()
            ]
            if real_directories or local_files:
                refuse_open_files(real_directories + local_files)
        apply_links(actions, worktree, args.repair)
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"data linking failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
