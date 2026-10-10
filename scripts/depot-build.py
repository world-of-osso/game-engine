#!/usr/bin/env python3
"""Build the Linux Godot extension or run workspace tests on this host (native) or in the
bookworm container on desktop/local. Release artifacts always use the container."""

import argparse
import contextlib
import fcntl
import gzip
import hashlib
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from build_hosts import execute
from prepare_ui_icons import prepare_ui_icons

ROOT_NAME = "game-engine-godot-conversion"
SIBLINGS = (
    "asset-resolver",
    "ui-toolkit",
    "ui-toolkit-macros",
    "shared-protocol",
)
SOURCE_SUFFIXES = {".rs", ".c", ".h", ".cpp", ".hpp", ".wgsl"}
ROOT_PATHS = ("godot", "vendor")
EXCLUDED_DIRS = {".git", "target", "data", ".godot"}
ARTIFACT = "libgame_engine_godot.so"
CLI = "game-engine-cli"
FIXTURE_DIR = Path("godot/network/examples")
TEST_ASSETS = Path("godot/depot-test-assets.txt")
TEST_LOG = Path("target/depot-test.log")
HOSTS = ("desktop", "local", "native")
CONTAINER_HOSTS = ("desktop", "local")
# Concurrent native cargo runs. Each already uses every core, so this bounds memory.
NATIVE_SLOTS = int(os.environ.get("GAME_ENGINE_NATIVE_SLOTS", "3"))
FICLONE = 0x40049409
SUMMARY_PREFIXES = ("     Running ", "   Doc-tests ", "test result:", "error")


def phase(message, start):
    print(f"{message}: {time.monotonic() - start:.1f}s", flush=True)


def git_files(repo, paths, tracked):
    selection = ["--cached"] if tracked else ["--others", "--exclude-standard"]
    result = subprocess.run(
        ["git", "-C", str(repo), "ls-files", "-z", *selection, "--", *paths],
        check=True,
        capture_output=True,
    )
    return sorted(
        {Path(os.fsdecode(path)) for path in result.stdout.split(b"\0") if path}
    )


def allowed(repo_name, path, tracked):
    parts = path.parts
    if any(part in EXCLUDED_DIRS for part in parts) or any(
        part.startswith(".") for part in parts
    ):
        return False
    if repo_name == ROOT_NAME and parts[0] not in ROOT_PATHS:
        return False
    if path.suffix in SOURCE_SUFFIXES or path.name in {"Cargo.toml", "Cargo.lock"}:
        return True
    if tracked and repo_name == ROOT_NAME and path.suffix == ".png":
        return parts[:6] == (
            "godot",
            "rust",
            "src",
            "rendering",
            "ui",
            "nameplate_skins",
        )
    return (
        tracked and repo_name == ROOT_NAME and parts == ("vendor", "taffy", "README.md")
    )


def snapshot_repo(repo, destination, paths, name):
    tracked = set(git_files(repo, paths, True))
    # --cached includes staged deletions. The working tree, not the index, is authoritative.
    for relative in tracked:
        if not allowed(name, relative, True):
            continue
        source = repo / relative
        if source.is_symlink() or any(
            (repo / parent).is_symlink()
            for parent in relative.parents
            if parent != Path(".")
        ):
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
        if source.is_symlink() or any(
            (repo / parent).is_symlink()
            for parent in relative.parents
            if parent != Path(".")
        ):
            raise ValueError(f"unsupported source symlink: {source}")
        if source.is_file():
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)


def validate_sources(context):
    root = context / ROOT_NAME
    required = [root / "godot/Cargo.toml", root / "godot/Cargo.lock"]
    required.extend(
        root / "godot" / member / "Cargo.toml"
        for member in ("cli", "core", "network", "rust", "session", "ui-model")
    )
    required.extend(context / name / "Cargo.toml" for name in SIBLINGS)
    required.extend(
        root / "vendor" / name / "Cargo.toml" for name in ("taffy", "ktx2-rw")
    )
    for path in required:
        if not path.is_file():
            raise FileNotFoundError(
                f"missing build dependency: {path.relative_to(context)}"
            )


def install_artifact(compressed, destination, executable=False):
    if not compressed.is_file():
        raise FileNotFoundError(f"Build host did not produce {compressed.name}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            dir=destination.parent, prefix=f".{destination.name}-", delete=False
        ) as output:
            temporary = Path(output.name)
            with gzip.open(compressed, "rb") as source:
                shutil.copyfileobj(source, output)
            if executable:
                temporary.chmod(0o755)
        os.replace(temporary, destination)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def read_asset_manifest(root):
    entries = []
    for line in (root / TEST_ASSETS).read_text().splitlines():
        entry = line.split("#", 1)[0].strip()
        if not entry:
            continue
        relative = Path(entry)
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError(
                f"{TEST_ASSETS}: asset path must stay inside data/: {entry}"
            )
        if not (root / "data" / relative).is_file():
            raise FileNotFoundError(f"{TEST_ASSETS}: missing test asset data/{entry}")
        entries.append(relative)
    return sorted(set(entries))


def stage_assets(root, staging):
    """Place listed data/ files in the build context, which Depot syncs incrementally by path and metadata."""
    wanted = read_asset_manifest(root)
    for relative in wanted:
        (staging / relative).parent.mkdir(parents=True, exist_ok=True)
        link_or_copy((root / "data" / relative).resolve(), staging / relative)
    return len(wanted)


def link_or_copy(source, target):
    """Reflink, else hardlink (data/ may be another filesystem), else copy; all keep the source mtime."""
    try:
        with source.open("rb") as src, target.open("wb") as dst:
            fcntl.ioctl(dst.fileno(), FICLONE, src.fileno())
        shutil.copystat(source, target)
        return
    except OSError:
        target.unlink(missing_ok=True)
    try:
        os.link(source, target)
    except OSError:
        shutil.copy2(source, target)


def sibling_repo(root, name):
    """`root`'s sibling checkout `name`, or `DEPOT_SIBLING_<NAME>` (for example
    DEPOT_SIBLING_UI_TOOLKIT for a toolkit branch worktree)."""
    override = os.environ.get("DEPOT_SIBLING_" + name.upper().replace("-", "_"))
    return Path(override).resolve() if override else root.parent / name


def snapshot(root, context):
    for name in (ROOT_NAME, *SIBLINGS):
        repo = root if name == ROOT_NAME else sibling_repo(root, name)
        if not repo.is_dir():
            raise FileNotFoundError(f"missing build dependency: {repo}")
        snapshot_repo(
            repo, context / name, ROOT_PATHS if name == ROOT_NAME else (".",), name
        )
    validate_sources(context)
    depot_scripts = Path(__file__).resolve().parent / "depot"
    shutil.copyfile(depot_scripts / "Dockerfile", context / "Dockerfile")
    shutil.copyfile(
        depot_scripts / "refresh-source-mtimes.py", context / "refresh-source-mtimes.py"
    )


def fixture_names(root):
    """Every top-level `game-engine-network` example; subdirectories hold their modules."""
    return sorted(
        path.stem for path in (root / FIXTURE_DIR).glob("*.rs") if path.is_file()
    )


def build_host_setting():
    # Runtime fixtures isolate XDG_CONFIG_HOME; this setting belongs to the build user.
    return Path.home() / ".config" / "game-engine" / "build-host"


def select_build_host(explicit):
    if explicit is not None:
        return explicit
    setting = build_host_setting()
    if not setting.is_file():
        raise ValueError(
            "choose --build-host desktop|local|native or --save-build-host desktop|local|native"
        )
    host = setting.read_text().strip()
    if host not in HOSTS:
        raise ValueError(
            f"invalid build-host {host!r} in {setting}; choose --build-host desktop|local|native"
        )
    return host


def save_build_host(host):
    setting = build_host_setting()
    setting.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w", dir=setting.parent, delete=False
    ) as output:
        temporary = Path(output.name)
        output.write(host + "\n")
    try:
        os.replace(temporary, setting)
    finally:
        temporary.unlink(missing_ok=True)
    print(f"Saved build host {host}: {setting}")


def prepare_checkout_cache(root):
    cache = (
        Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
        / "game-engine"
        / "depot-build"
    )
    cache.mkdir(parents=True, exist_ok=True)
    checkout_key = hashlib.sha256(os.fsencode(root)).hexdigest()[:20]
    return cache, checkout_key


def locked_checkout(root):
    cache, checkout_key = prepare_checkout_cache(root)
    lock = (cache / (checkout_key + ".lock")).open("w")
    fcntl.flock(lock, fcntl.LOCK_EX)
    return lock, cache, checkout_key


@contextlib.contextmanager
def stable_context(cache, mode, checkout_key):
    """A fresh build context at a fixed per-checkout path. BuildKit keys its incremental
    context transfer by path: a new temporary path re-sent every file, while a fixed path
    sends only files whose metadata changed, so unchanged reflinked assets stay remote."""
    context = cache / f"context-{mode}-{checkout_key}"
    shutil.rmtree(context, ignore_errors=True)
    context.mkdir()
    try:
        yield context
    finally:
        shutil.rmtree(context, ignore_errors=True)


@contextlib.contextmanager
def native_slot():
    """Hold one of NATIVE_SLOTS host-wide slots for the duration of a native cargo run."""
    directory = Path.home() / ".cache" / "game-engine" / "native-slots"
    directory.mkdir(parents=True, exist_ok=True)
    waiting = False
    while True:
        for number in range(1, NATIVE_SLOTS + 1):
            handle = (directory / f"slot{number}").open("w")
            try:
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                handle.close()
                continue
            print(f"Native slot {number}/{NATIVE_SLOTS} acquired", flush=True)
            try:
                yield
            finally:
                handle.close()
            return
        if not waiting:
            print(f"All {NATIVE_SLOTS} native slots busy; waiting", flush=True)
            waiting = True
        time.sleep(5)


def native_source_root(root):
    """The checkout path Cargo should read. Without DEPOT_SIBLING_* overrides it is `root`;
    with any, a per-checkout directory of symlinks places `root` beside the overridden
    siblings, since Cargo resolves the relative path dependencies through it as written."""
    overrides = {name: sibling_repo(root, name) for name in SIBLINGS}
    if all(path == root.parent / name for name, path in overrides.items()):
        return root
    _, checkout_key = prepare_checkout_cache(root)
    tree = Path.home() / ".cache" / "game-engine" / "native-roots" / checkout_key
    tree.mkdir(parents=True, exist_ok=True)
    for name, target in {"game-engine": root, **overrides}.items():
        link = tree / name
        if link.is_symlink() or link.exists():
            link.unlink()
        link.symlink_to(target)
    return tree / "game-engine"


def native_cargo(root, command, arguments, output=None):
    """Run `cargo <command>` on this host for godot/, with artifacts in the checkout's
    target/ where game_engine.gdextension loads them."""
    environment = os.environ | {"CARGO_TARGET_DIR": str(root / "target")}
    # Size jobs from the cgroup CPU quota (agents.slice), not an inherited session value.
    environment.pop("CARGO_BUILD_JOBS", None)
    return subprocess.run(
        ["cargo", command, "--locked", "--manifest-path", str(native_source_root(root) / "godot/Cargo.toml"), *arguments],
        env=environment,
        stdout=output,
        stderr=subprocess.STDOUT if output else None,
        check=False,
    ).returncode


def build_native(root, fixture, cli):
    builds = [["-p", "game-engine-godot", "--lib"]]
    if fixture:
        builds.append(["-p", "game-engine-network", "--example", fixture])
    if cli:
        builds.append(["-p", "game-engine-cli", "--bin", CLI])
    start = time.monotonic()
    with native_slot():
        for arguments in builds:
            code = native_cargo(root, "build", arguments)
            if code != 0:
                raise subprocess.CalledProcessError(code, ["cargo", "build", *arguments])
    phase("native build", start)
    print(root / "target" / "debug" / ARTIFACT)
    if fixture:
        print(root / "target" / "debug" / "examples" / fixture)
    if cli:
        print(root / "target" / "debug" / CLI)


def run_tests_native(root, cargo_args):
    saved = root / TEST_LOG
    saved.parent.mkdir(parents=True, exist_ok=True)
    start = time.monotonic()
    with native_slot(), saved.open("w") as log:
        code = native_cargo(root, "test", cargo_args, output=log)
    phase("native test", start)
    print_test_summary(saved.read_text(errors="replace").splitlines())
    print(f"Full log: {saved}")
    print(f"cargo test {shlex.join(cargo_args)}: exit {code}", flush=True)
    return code


def build(root, fixture=None, cli=False, release=False, host=None):
    host = select_build_host(host)
    if release and host == "native":
        # glibc portability: a host (Arch) link needs the host's newest glibc.
        raise ValueError("release artifacts use the bookworm container: --build-host local|desktop")
    if fixture and fixture not in fixture_names(root):
        raise ValueError(
            f"unknown fixture {fixture!r}; choose from {', '.join(fixture_names(root))}"
        )
    if host == "native":
        lock, _, _ = locked_checkout(root)
        with lock:
            return build_native(root, fixture, cli)
    if release:
        # Shipping requires prepared UI art; debug/CPU work can diagnose gaps without
        # publishing an incomplete asset set. Runtime never calls this developer step.
        prepare_ui_icons(root, sibling_repo(root, "asset-resolver") / "target/debug/casc-local")
    lock, cache, checkout_key = locked_checkout(root)
    with lock:
        target = root / "target"
        if (
            target.is_symlink()
            or (target / "debug").is_symlink()
            or (target / "release").is_symlink()
            or (fixture and (target / "debug" / "examples").is_symlink())
        ):
            raise ValueError(
                f"target symlink cannot guarantee checkout-local artifact: {target}"
            )
        start = time.monotonic()
        with (
            tempfile.TemporaryDirectory(prefix="build-", dir=cache) as work,
            stable_context(cache, "build", checkout_key) as context,
        ):
            snapshot(root, context)
            phase("Snapshot", start)
            output = Path(work) / "output"
            output.mkdir()
            command = []
            if fixture:
                command.extend(["--build-arg", f"FIXTURE={fixture}"])
            if cli:
                command.extend(["--build-arg", "CLI=1"])
            if release:
                command.extend(["--build-arg", "RELEASE=1"])
            execute(context, output, checkout_key, "artifact", command, host)
            phase(f"{host} build", start)
            destination = (
                root / "target" / ("release" if release else "debug") / ARTIFACT
            )
            if fixture:
                fixture_destination = root / "target" / "debug" / "examples" / fixture
                install_artifact(
                    output / (fixture + ".gz"), fixture_destination, executable=True
                )
            if cli:
                cli_destination = root / "target" / "debug" / CLI
                install_artifact(
                    output / (CLI + ".gz"), cli_destination, executable=True
                )
            install_artifact(output / (ARTIFACT + ".gz"), destination)
            phase("Installed", start)
            print(destination)
            if fixture:
                print(fixture_destination)
            if cli:
                print(cli_destination)


def run_tests(root, cargo_args, host=None):
    """Run `cargo test` on the selected host and return cargo's exit status."""
    host = select_build_host(host)
    if host == "native":
        lock, _, _ = locked_checkout(root)
        with lock:
            return run_tests_native(root, cargo_args)
    lock, cache, checkout_key = locked_checkout(root)
    with lock:
        start = time.monotonic()
        with (
            tempfile.TemporaryDirectory(prefix="test-", dir=cache) as work,
            stable_context(cache, "test", checkout_key) as context,
        ):
            snapshot(root, context)
            count = stage_assets(root, context / "test-assets")
            phase(f"Snapshot ({count} test assets)", start)
            output = Path(work) / "output"
            output.mkdir()
            command = [
                "--build-arg",
                f"TEST_ARGS={shlex.join(cargo_args)}",
                "--build-arg",
                f"TEST_RUN={time.time_ns()}",
            ]
            execute(context, output, checkout_key, "test-result", command, host)
            phase(f"{host} test", start)
            log, status = output / "test.log", output / "status"
            if not log.is_file() or not status.is_file():
                raise FileNotFoundError(
                    "Build host did not produce test.log and status"
                )
            code = int(status.read_text())
            saved = root / TEST_LOG
            saved.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(log, saved)
            print_test_summary(log.read_text(errors="replace").splitlines())
            print(f"Full log: {saved}")
            print(f"cargo test {shlex.join(cargo_args)}: exit {code}", flush=True)
            return code


def print_test_summary(lines):
    print("--- log tail ---", *lines[-20:], "--- test summary ---", sep="\n")
    for line in lines:
        if line.startswith(SUMMARY_PREFIXES) or (
            line.startswith("test ") and line.endswith("FAILED")
        ):
            print(line)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="originating checkout",
    )
    parser.add_argument(
        "--build-host",
        choices=HOSTS,
        help="override saved build host for this run",
    )
    parser.add_argument(
        "--save-build-host",
        choices=HOSTS,
        help="save default host and exit without building",
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--fixture",
        help=f"also export one {FIXTURE_DIR} executable, named by file stem",
    )
    parser.add_argument(
        "--cli", action="store_true", help=f"also export target/debug/{CLI}"
    )
    mode.add_argument(
        "--release",
        action="store_true",
        help=f"build only the optimized library into target/release/{ARTIFACT}",
    )
    mode.add_argument(
        "--test",
        action="store_true",
        help="run `cargo test --locked` in godot/ with every following argument; must be last",
    )
    argv = sys.argv[1:]
    # argparse drops `--`, which cargo needs to separate test-binary arguments.
    split = argv.index("--test") + 1 if "--test" in argv else len(argv)
    args = parser.parse_args(argv[:split])
    if args.cli and (args.test or args.release):
        parser.error(
            f"argument --cli: not allowed with argument {'--test' if args.test else '--release'}"
        )
    try:
        if args.save_build_host:
            if args.build_host or args.fixture or args.cli or args.release or args.test:
                parser.error("--save-build-host cannot combine with build options")
            save_build_host(args.save_build_host)
            return 0
        if args.test:
            return run_tests(args.root.resolve(), argv[split:], args.build_host)
        build(
            args.root.resolve(), args.fixture, args.cli, args.release, args.build_host
        )
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Remote build failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
