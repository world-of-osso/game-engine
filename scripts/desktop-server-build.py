#!/usr/bin/env python3
"""Build/export bookworm Linux server executables, or run the server workspace tests, on
desktop or local; never start the server."""

import argparse
import fcntl
import importlib.util
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import time

from build_hosts import CACHE_ROOT, execute

SCRIPTS = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("depot_build", SCRIPTS / "depot-build.py")
depot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(depot)

BINARIES = ("game-server", "game-server-admin", "game-cli")
GROUND_GAPS = Path("crates/server/src/ground_gaps.tsv")
TEST_DATA = SCRIPTS / "desktop-server/test-data.txt"
TEST_LOG = Path("target/server-test.log")
# Synced data mirror on each host, shared by every checkout under one lock.
TEST_DATA_MIRROR = "server-test-data"
DESKTOP_RSH = "ssh desktop wsl.exe -d OssoBuild -u root --exec"


def snapshot(root, context):
    repositories = (
        ("game-server", root),
        ("shared-protocol", depot.sibling_repo(root, "shared-protocol")),
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


def build(root, host=None, release=False, binary=None):
    if binary is not None and binary not in BINARIES:
        raise ValueError(f"unsupported server binary: {binary!r}")
    binaries = (binary,) if binary else BINARIES
    root = Path(root).resolve()
    host = depot.select_build_host(host)
    lock, cache, checkout_key = depot.locked_checkout(root)
    key = "server-" + checkout_key
    with lock:
        target = root / "target" / ("release" if release else "debug")
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
                "--build-arg",
                f"BUILD_ROOT={root}",
                "--build-arg",
                f"BUILD_PARENT={root.parent}",
                "--build-arg",
                f"RELEASE={str(release).lower()}",
                "--build-arg",
                f"BINARY={binary or ''}",
            ]
            execute(context, output, key, "artifact", arguments, host=host)
            for name in binaries:
                depot.install_artifact(
                    output / (name + ".gz"), target / name, executable=True
                )
                print(target / name)


def test_data_filters(root):
    """`{repo: (data dir, rsync filter rules)}` from TEST_DATA; an entry matching
    nothing fails."""
    repos = {"game-server": root, "game-engine": depot.sibling_repo(root, "game-engine")}
    rules = {name: [] for name in repos}
    for line in TEST_DATA.read_text().splitlines():
        entry = line.split("#", 1)[0].strip()
        if not entry:
            continue
        name, _, relative = entry.partition("/")
        path = Path(relative.rstrip("/"))
        if name not in repos or not relative or path.is_absolute() or ".." in path.parts:
            raise ValueError(f"{TEST_DATA.name}: expected game-server/ or game-engine/<path>: {entry}")
        directory = relative.endswith("/")
        data = repos[name] / "data"
        if not any(match.is_dir() if directory else match.is_file() for match in data.glob(str(path))):
            raise FileNotFoundError(f"{TEST_DATA.name}: missing test data {data / relative}")
        rules[name].extend(f"+ /{parent}/" for parent in reversed(path.parents[:-1]))
        rules[name].append(f"+ /{path}/***" if directory else f"+ /{path}")
    return {name: (repos[name] / "data", rules[name] + ["- *"]) for name in repos}


def sync_test_data(root, host, cache):
    """Mirror the TEST_DATA files to `host`; returns the mirror path there. Unchanged files
    (size and mtime) are not re-sent; files no longer listed are deleted."""
    if host == "desktop":
        mirror = CACHE_ROOT / TEST_DATA_MIRROR
        # rsync runs `<rsh> <host> rsync --server ...`; the host `env` passes the rest through.
        destination, transport = f"env:{mirror}", ["-z", "-e", DESKTOP_RSH]
    else:
        mirror = destination = cache / TEST_DATA_MIRROR
        transport = []
    for name, (data, rules) in test_data_filters(root).items():
        subprocess.run(
            ["rsync", "-rtL", "--mkpath", "--delete", "--delete-excluded", *transport,
             *(f"--filter={rule}" for rule in rules), f"{data}/", f"{destination}/{name}/"],
            check=True,
        )
    return mirror


def run_tests(root, cargo_args, host=None):
    """Run `cargo test --locked` for the server workspace on `host` and return cargo's
    exit status."""
    root = Path(root).resolve()
    host = depot.select_build_host(host)
    lock, cache, checkout_key = depot.locked_checkout(root)
    key = "server-" + checkout_key
    start = time.monotonic()
    # Checkouts share the mirror, so it stays locked until the tests have read it.
    with lock, (cache / f"{TEST_DATA_MIRROR}-{host}.lock").open("w") as data_lock:
        fcntl.flock(data_lock, fcntl.LOCK_EX)
        mirror = sync_test_data(root, host, cache)
        depot.phase(f"Test data synced to {host}", start)
        with (
            depot.stable_context(cache, "server-test", checkout_key) as context,
            tempfile.TemporaryDirectory(prefix="server-output-", dir=cache) as temporary,
        ):
            snapshot(root, context)
            output = Path(temporary)
            arguments = [
                "--build-context", f"test-data={mirror}",
                "--build-arg", f"TARGET_CACHE=server-target-{key}",
                "--build-arg", f"SERVER_CACHE={key}",
                "--build-arg", f"BUILD_ROOT={root}",
                "--build-arg", f"BUILD_PARENT={root.parent}",
                "--build-arg", f"TEST_ARGS={shlex.join(cargo_args)}",
                "--build-arg", f"TEST_RUN={time.time_ns()}",
            ]
            execute(context, output, key, "test-result", arguments, host=host)
            depot.phase(f"{host} test", start)
            log, status = output / "test.log", output / "status"
            if not log.is_file() or not status.is_file():
                raise FileNotFoundError("Build host did not produce test.log and status")
            code = int(status.read_text())
            saved = root / TEST_LOG
            saved.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(log, saved)
    depot.print_test_summary(saved.read_text(errors="replace").splitlines())
    print(f"Full log: {saved}")
    print(f"cargo test {shlex.join(cargo_args)}: exit {code}", flush=True)
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=SCRIPTS.parents[1] / "game-server",
        help="originating server checkout",
    )
    parser.add_argument("--build-host", choices=("desktop", "local"))
    parser.add_argument("--save-build-host", choices=("desktop", "local"))
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--bin", choices=BINARIES, dest="binary")
    parser.add_argument(
        "--test",
        action="store_true",
        help="run `cargo test --locked` with every following argument; must be last",
    )
    argv = sys.argv[1:]
    # argparse drops `--`, which cargo needs to separate test-binary arguments.
    split = argv.index("--test") + 1 if "--test" in argv else len(argv)
    args = parser.parse_args(argv[:split])
    if args.test and (args.save_build_host or args.release or args.binary):
        parser.error("--test cannot combine with --save-build-host, --release or --bin")
    try:
        if args.save_build_host:
            if args.build_host or args.release or args.binary:
                parser.error("--save-build-host cannot combine with build options")
            depot.save_build_host(args.save_build_host)
            return 0
        if args.test:
            return run_tests(args.root, argv[split:], args.build_host)
        build(args.root, host=args.build_host, release=args.release, binary=args.binary)
    except (OSError, EOFError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Desktop server build failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
