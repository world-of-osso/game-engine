#!/usr/bin/env python3
"""Build the Godot native library on Depot from a source-only checkout snapshot."""

import argparse
import fcntl
import gzip
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


ROOT_NAME = "game-engine-godot-conversion"
SIBLINGS = ("asset-resolver", "ui-toolkit-godot-conversion", "ui-toolkit-macros", "shared-protocol", "bevy-patches")
SOURCE_SUFFIXES = {".rs", ".c", ".h", ".cpp", ".hpp", ".wgsl"}
EXCLUDED_DIRS = {".git", "target", "data", ".godot"}
ARTIFACT = "libgame_engine_godot.so"
FIXTURES = ("native_input_fixture", "native_npc_visual_fixture")


def phase(message, start):
    print(f"{message}: {time.monotonic() - start:.1f}s", flush=True)


def git_files(repo, paths, tracked):
    selection = ["--cached"] if tracked else ["--others", "--exclude-standard"]
    result = subprocess.run(
        ["git", "-C", str(repo), "ls-files", "-z", *selection, "--", *paths],
        check=True, capture_output=True,
    )
    return sorted({Path(os.fsdecode(path)) for path in result.stdout.split(b"\0") if path})


def allowed(repo_name, path, tracked):
    parts = path.parts
    if any(part in EXCLUDED_DIRS for part in parts) or any(part.startswith(".") for part in parts):
        return False
    if repo_name == ROOT_NAME and parts[0] not in {"godot", "src"}:
        return False
    if path.suffix in SOURCE_SUFFIXES or path.name in {"Cargo.toml", "Cargo.lock"}:
        return True
    if tracked and repo_name == ROOT_NAME and path.suffix == ".png":
        return parts[:4] == ("src", "rendering", "ui", "nameplate_skins")
    return tracked and repo_name == "bevy-patches" and parts == ("taffy", "README.md")


def snapshot_repo(repo, destination, paths, name):
    tracked = set(git_files(repo, paths, True))
    # --cached includes staged deletions. The working tree, not the index, is authoritative.
    for relative in tracked:
        if not allowed(name, relative, True):
            continue
        source = repo / relative
        if source.is_symlink() or any((repo / parent).is_symlink() for parent in relative.parents if parent != Path(".")):
            raise ValueError(f"unsupported source symlink: {source}")
        if not source.exists():
            continue
        if not source.is_file():
            raise ValueError(f"unsupported source file: {source}")
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
    # Tracked root PNGs and the taffy README are never accepted from untracked files.
    for relative in git_files(repo, paths, False):
        if relative in tracked or not allowed(name, relative, False):
            continue
        source = repo / relative
        if source.is_symlink() or any((repo / parent).is_symlink() for parent in relative.parents if parent != Path(".")):
            raise ValueError(f"unsupported source symlink: {source}")
        if source.is_file():
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)


def validate_sources(context):
    root = context / ROOT_NAME
    required = [root / "godot/Cargo.toml", root / "godot/Cargo.lock"]
    required.extend(root / "godot" / member / "Cargo.toml" for member in ("core", "network", "rust", "session", "ui-model"))
    required.extend(context / name / "Cargo.toml" for name in SIBLINGS if name != "bevy-patches")
    required.extend(context / "bevy-patches" / name / "Cargo.toml" for name in ("taffy", "ktx2-rw"))
    for path in required:
        if not path.is_file():
            raise FileNotFoundError(f"missing build dependency: {path.relative_to(context)}")
    if not any((root / "src").rglob("*.rs")):
        raise FileNotFoundError("missing build dependency: game-engine-godot-conversion/src/*.rs")


def install_artifact(compressed, destination, executable=False):
    if not compressed.is_file():
        raise FileNotFoundError(f"Depot did not produce {compressed.name}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=destination.parent, prefix=f".{destination.name}-", delete=False) as output:
            temporary = Path(output.name)
            with gzip.open(compressed, "rb") as source:
                shutil.copyfileobj(source, output)
            if executable:
                temporary.chmod(0o755)
        os.replace(temporary, destination)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def build(root, fixture=None):
    cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "game-engine" / "depot-build"
    cache.mkdir(parents=True, exist_ok=True)
    lock_name = hashlib.sha256(os.fsencode(root)).hexdigest()[:20] + ".lock"
    with (cache / lock_name).open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        target = root / "target"
        if (target.is_symlink() or (target / "debug").is_symlink()
                or (fixture and (target / "debug" / "examples").is_symlink())):
            raise ValueError(f"target symlink cannot guarantee checkout-local artifact: {target}")
        start = time.monotonic()
        with tempfile.TemporaryDirectory(prefix="build-", dir=cache) as work:
            context = Path(work) / "context"
            context.mkdir()
            for name in (ROOT_NAME, *SIBLINGS):
                repo = root if name == ROOT_NAME else root.parent / name
                if not repo.is_dir():
                    raise FileNotFoundError(f"missing build dependency: {repo}")
                snapshot_repo(repo, context / name, ("godot", "src") if name == ROOT_NAME else (".",), name)
            validate_sources(context)
            depot_scripts = Path(__file__).resolve().parent / "depot"
            shutil.copyfile(depot_scripts / "Dockerfile", context / "Dockerfile")
            shutil.copyfile(depot_scripts / "refresh-source-mtimes.py", context / "refresh-source-mtimes.py")
            phase("Snapshot", start)
            output = Path(work) / "output"
            output.mkdir()
            command = [
                "depot", "build", "--project", os.environ.get("DEPOT_PROJECT_ID", "003c4ttwqh"),
                "--platform", "linux/amd64", "--file", str(context / "Dockerfile"), "--target", "artifact",
                "--output", f"type=local,dest={output}",
            ]
            if fixture:
                command.extend(["--build-arg", f"FIXTURE={fixture}"])
            subprocess.run([*command, str(context)], check=True)
            phase("Remote build", start)
            destination = root / "target" / "debug" / ARTIFACT
            if fixture:
                fixture_destination = root / "target" / "debug" / "examples" / fixture
                install_artifact(output / (fixture + ".gz"), fixture_destination, executable=True)
            install_artifact(output / (ARTIFACT + ".gz"), destination)
            phase("Installed", start)
            print(destination)
            if fixture:
                print(fixture_destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True, help="originating checkout")
    parser.add_argument("--fixture", choices=FIXTURES, help="also export one network fixture executable")
    args = parser.parse_args()
    try:
        build(args.root.resolve(), args.fixture)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Remote build failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
