#!/usr/bin/env python3
"""Build/export bookworm Linux server executables on desktop; never start them."""

import argparse
import importlib.util
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

from build_hosts import execute

SCRIPTS = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("depot_build", SCRIPTS / "depot-build.py")
depot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(depot)

BINARIES = ("game-server", "game-server-admin", "game-cli")
GROUND_GAPS = Path("crates/server/src/ground_gaps.tsv")


def snapshot(root, context):
    repositories = (
        ("game-server", root),
        ("shared-protocol", root.parent / "shared-protocol"),
    )
    for name, repo in repositories:
        if not repo.is_dir():
            raise FileNotFoundError(f"missing build dependency: {repo}")
        depot.snapshot_repo(repo, context / name, (".",), name)
    # This tracked include_str! input is not a source suffix in the engine helper.
    if GROUND_GAPS in depot.git_files(root, (".",), True) and not depot.allowed(
        "game-server", GROUND_GAPS, True
    ):
        source = root / GROUND_GAPS
        if source.is_symlink() or any(
            (root / parent).is_symlink() for parent in GROUND_GAPS.parents
        ):
            raise ValueError(f"unsupported source symlink: {source}")
        destination = context / "game-server" / GROUND_GAPS
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
    for name in ("game-server", "shared-protocol"):
        if not (context / name / "Cargo.toml").is_file():
            raise FileNotFoundError(f"missing build dependency: {name}/Cargo.toml")
    if not (context / "game-server/Cargo.lock").is_file():
        raise FileNotFoundError("missing build dependency: game-server/Cargo.lock")
    shutil.copyfile(SCRIPTS / "desktop-server/Dockerfile", context / "Dockerfile")
    shutil.copyfile(
        SCRIPTS / "depot/refresh-source-mtimes.py", context / "refresh-source-mtimes.py"
    )


def build(root):
    root = Path(root).resolve()
    lock, cache, checkout_key = depot.locked_checkout(root)
    key = "server-" + checkout_key
    with lock:
        target = root / "target/debug"
        if (root / "target").is_symlink() or target.is_symlink():
            raise ValueError(
                f"target symlink cannot guarantee checkout-local artifact: {target}"
            )
        with (
            depot.stable_context(cache, "server", checkout_key) as context,
            tempfile.TemporaryDirectory(
                prefix="server-output-", dir=cache
            ) as temporary,
        ):
            snapshot(root, context)
            output = Path(temporary)
            arguments = [
                "--build-arg",
                f"TARGET_CACHE=server-target-{key}",
                "--build-arg",
                f"SERVER_CACHE={key}",
            ]
            execute(context, output, key, "artifact", arguments, host="desktop")
            for name in BINARIES:
                depot.install_artifact(
                    output / (name + ".gz"), target / name, executable=True
                )
                print(target / name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=SCRIPTS.parents[1] / "game-server",
        help="originating server checkout",
    )
    args = parser.parse_args()
    try:
        build(args.root)
    except (OSError, EOFError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Desktop server build failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
