#!/usr/bin/env python3
"""Build the Godot native library, or run godot/ workspace tests, on Depot from a source-only checkout snapshot."""

import argparse
import fcntl
import gzip
import hashlib
import os
import shlex
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


ROOT_NAME = "game-engine-godot-conversion"
SIBLINGS = ("asset-resolver", "ui-toolkit-godot-conversion", "ui-toolkit-macros", "shared-protocol", "bevy-patches")
SOURCE_SUFFIXES = {".rs", ".c", ".h", ".cpp", ".hpp", ".wgsl"}
ROOT_PATHS = ("godot", "src", "tests")  # tests/unit is compiled into crates through #[path]
EXCLUDED_DIRS = {".git", "target", "data", ".godot"}
ARTIFACT = "libgame_engine_godot.so"
FIXTURE_DIR = Path("godot/network/examples")
TEST_ASSETS = Path("godot/depot-test-assets.txt")
TEST_LOG = Path("target/depot-test.log")
FICLONE = 0x40049409
SUMMARY_PREFIXES = ("     Running ", "   Doc-tests ", "test result:", "error")


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
    if repo_name == ROOT_NAME and parts[0] not in ROOT_PATHS:
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


def read_asset_manifest(root):
    entries = []
    for line in (root / TEST_ASSETS).read_text().splitlines():
        entry = line.split("#", 1)[0].strip()
        if not entry:
            continue
        relative = Path(entry)
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError(f"{TEST_ASSETS}: asset path must stay inside data/: {entry}")
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
    DEPOT_SIBLING_SHARED_PROTOCOL for a protocol branch worktree)."""
    override = os.environ.get("DEPOT_SIBLING_" + name.upper().replace("-", "_"))
    return Path(override).resolve() if override else root.parent / name


def snapshot(root, context):
    for name in (ROOT_NAME, *SIBLINGS):
        repo = root if name == ROOT_NAME else sibling_repo(root, name)
        if not repo.is_dir():
            raise FileNotFoundError(f"missing build dependency: {repo}")
        snapshot_repo(repo, context / name, ROOT_PATHS if name == ROOT_NAME else (".",), name)
    validate_sources(context)
    depot_scripts = Path(__file__).resolve().parent / "depot"
    shutil.copyfile(depot_scripts / "Dockerfile", context / "Dockerfile")
    shutil.copyfile(depot_scripts / "refresh-source-mtimes.py", context / "refresh-source-mtimes.py")


def fixture_names(root):
    """Every top-level `game-engine-network` example; subdirectories hold their modules."""
    return sorted(path.stem for path in (root / FIXTURE_DIR).glob("*.rs") if path.is_file())


def depot_environment():
    """Depot reads its login from $XDG_CONFIG_HOME/depot/depot.yaml. Fixtures isolate
    XDG_CONFIG_HOME for Godot, so the login also resolves from the user's ~/.config."""
    environment = dict(os.environ)
    if environment.get("DEPOT_TOKEN"):
        return environment
    home_config = Path.home() / ".config"
    configs = [Path(environment["XDG_CONFIG_HOME"])] if environment.get("XDG_CONFIG_HOME") else []
    for config in (*configs, home_config):
        if (config / "depot" / "depot.yaml").is_file():
            environment["XDG_CONFIG_HOME"] = str(config)
            return environment
    searched = ", ".join(str(config / "depot" / "depot.yaml") for config in (*configs, home_config))
    raise FileNotFoundError(f"Depot login not found in {searched}; run `depot login` or set DEPOT_TOKEN")


def depot_command(context, output, checkout_key, target):
    return [
        "depot", "build", "--project", os.environ.get("DEPOT_PROJECT_ID", "003c4ttwqh"),
        "--platform", "linux/amd64", "--file", str(context / "Dockerfile"), "--target", target,
        "--build-arg", f"TARGET_CACHE=godot-target-{checkout_key}",
        "--output", f"type=local,dest={output}",
    ]


def locked_checkout(root):
    cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "game-engine" / "depot-build"
    cache.mkdir(parents=True, exist_ok=True)
    checkout_key = hashlib.sha256(os.fsencode(root)).hexdigest()[:20]
    lock = (cache / (checkout_key + ".lock")).open("w")
    fcntl.flock(lock, fcntl.LOCK_EX)
    return lock, cache, checkout_key


def build(root, fixture=None):
    if fixture and fixture not in fixture_names(root):
        raise ValueError(f"unknown fixture {fixture!r}; choose from {', '.join(fixture_names(root))}")
    environment = depot_environment()
    lock, cache, checkout_key = locked_checkout(root)
    with lock:
        target = root / "target"
        if (target.is_symlink() or (target / "debug").is_symlink()
                or (fixture and (target / "debug" / "examples").is_symlink())):
            raise ValueError(f"target symlink cannot guarantee checkout-local artifact: {target}")
        start = time.monotonic()
        with tempfile.TemporaryDirectory(prefix="build-", dir=cache) as work:
            context = Path(work) / "context"
            context.mkdir()
            snapshot(root, context)
            phase("Snapshot", start)
            output = Path(work) / "output"
            output.mkdir()
            command = depot_command(context, output, checkout_key, "artifact")
            if fixture:
                command.extend(["--build-arg", f"FIXTURE={fixture}"])
            subprocess.run([*command, str(context)], check=True, env=environment)
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


def run_tests(root, cargo_args):
    """Run `cargo test` remotely and return cargo's exit status."""
    environment = depot_environment()
    lock, cache, checkout_key = locked_checkout(root)
    with lock:
        start = time.monotonic()
        with tempfile.TemporaryDirectory(prefix="test-", dir=cache) as work:
            context = Path(work) / "context"
            context.mkdir()
            snapshot(root, context)
            count = stage_assets(root, context / "test-assets")
            phase(f"Snapshot ({count} test assets)", start)
            output = Path(work) / "output"
            output.mkdir()
            command = depot_command(context, output, checkout_key, "test-result")
            command.extend(["--build-arg", f"TEST_ARGS={shlex.join(cargo_args)}",
                            "--build-arg", f"TEST_RUN={time.time_ns()}"])
            subprocess.run([*command, str(context)], check=True, env=environment)
            phase("Remote test", start)
            log, status = output / "test.log", output / "status"
            if not log.is_file() or not status.is_file():
                raise FileNotFoundError("Depot did not produce test.log and status")
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
        if line.startswith(SUMMARY_PREFIXES) or (line.startswith("test ") and line.endswith("FAILED")):
            print(line)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True, help="originating checkout")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--fixture", help=f"also export one {FIXTURE_DIR} executable, named by file stem")
    mode.add_argument("--test", action="store_true",
                      help="run `cargo test --locked` in godot/ with every following argument; must be last")
    argv = sys.argv[1:]
    # argparse drops `--`, which cargo needs to separate test-binary arguments.
    split = argv.index("--test") + 1 if "--test" in argv else len(argv)
    args = parser.parse_args(argv[:split])
    try:
        if args.test:
            return run_tests(args.root.resolve(), argv[split:])
        build(args.root.resolve(), args.fixture)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Remote build failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
