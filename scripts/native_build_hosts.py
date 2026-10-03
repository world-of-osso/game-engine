#!/usr/bin/env python3
"""Native Rust execution; caller owns source selection and runtime settings."""

from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import random
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import uuid

from build_hosts import (
    extract_directory,
    pack_directory,
    powershell,
    ps_literal,
    validate_key,
    windows_profile,
    wsl_path,
)

CACHE_ROOT = Path("/home/osso-test/.cache/native-builds")
AGENT_RUN = Path(__file__).resolve().parent / "agent/agent-run"
TOOLCHAIN = "1.98.1"
UPLOAD_TIMEOUT = 60
UPLOAD_BACKOFF = 1


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").digest()


def sync_source(snapshot: Path, source: Path, manifest: Path):
    """Only remove previously supplied files; runtime directories remain owned by app."""
    source.mkdir(parents=True, exist_ok=True)
    previous = json.loads(manifest.read_text()) if manifest.exists() else []
    owned = []
    for incoming in sorted(snapshot.rglob("*")):
        relative = incoming.relative_to(snapshot)
        destination = source / relative
        if incoming.is_symlink() or not (incoming.is_file() or incoming.is_dir()):
            raise ValueError(f"unsupported source: {incoming}")
        if any(p.is_symlink() for p in (destination, *destination.parents)):
            raise ValueError(f"symlink in persistent source: {destination}")
        if incoming.is_dir():
            destination.mkdir(parents=True, exist_ok=True)
            continue
        owned.append(str(relative))
        if not destination.exists() or digest(incoming) != digest(destination):
            old_time = destination.stat().st_mtime_ns if destination.exists() else 0
            shutil.copyfile(incoming, destination)
            fresh = max(time.time_ns(), old_time + 1)
            os.utime(destination, ns=(fresh, fresh))
        destination.chmod(incoming.stat().st_mode & 0o777)
    for name in set(previous) - set(owned):
        path = source / name
        if not path.resolve().is_relative_to(source.resolve()) or any(
            parent.is_symlink() for parent in (path, *path.parents)
        ):
            raise ValueError(f"unsafe owned source: {name}")
        path.unlink(missing_ok=True)
    manifest.parent.mkdir(parents=True, exist_ok=True)
    temporary = manifest.with_suffix(".new")
    temporary.write_text(json.dumps(owned))
    temporary.replace(manifest)


@contextmanager
def lifetime(monitor=False):
    """Signals and a bounded stdin lease govern every worker command."""
    stopped = threading.Event()
    reason = [0]

    def stop(number, _frame):
        reason[0] = number
        stopped.set()

    handlers = {
        number: signal.signal(number, stop)
        for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)
    }

    def watch():
        last = time.monotonic()
        while not stopped.is_set():
            ready, _, _ = select.select([sys.stdin.fileno()], [], [], 0.2)
            if ready:
                if not os.read(sys.stdin.fileno(), 1024):
                    reason[0] = signal.SIGHUP
                    stopped.set()
                    return
                last = time.monotonic()
            if time.monotonic() - last > 15:
                reason[0] = signal.SIGHUP
                stopped.set()

    thread = threading.Thread(target=watch, daemon=True) if monitor else None
    if thread:
        thread.start()
    try:
        yield stopped, reason
    finally:
        stopped.set()
        if thread:
            thread.join(timeout=1)
        for number, handler in handlers.items():
            signal.signal(number, handler)


def kill_group(process, number):
    try:
        os.killpg(process.pid, number)
    except ProcessLookupError:
        pass


def run_owned(
    command, cwd, environment, monitor=False, lease=None, relay=False, stdout=None
):
    if lease is None:
        with lifetime(monitor) as owned_lease:
            return run_owned(
                command, cwd, environment, lease=owned_lease, relay=relay, stdout=stdout
            )
    stopped, reason = lease
    if stopped.is_set():
        return 128 + reason[0]
    process = subprocess.Popen(
        command,
        cwd=cwd,
        env=environment,
        start_new_session=True,
        stdin=subprocess.PIPE if relay else subprocess.DEVNULL,
        stdout=stdout,
    )
    if relay:
        os.set_blocking(process.stdin.fileno(), False)
    try:
        while process.poll() is None and not stopped.is_set():
            if relay:
                try:
                    os.write(process.stdin.fileno(), b".\n")
                except BlockingIOError:
                    pass  # Worker lease expires if transport stops draining.
                except BrokenPipeError:
                    break
            stopped.wait(0.2)
        if stopped.is_set() and relay:
            # EOF reaches the WSL worker before terminating its Windows SSH relay.
            process.stdin.close()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
        if process.poll() is None:
            kill_group(process, signal.SIGTERM)
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                kill_group(process, signal.SIGKILL)
                process.wait()
        return 128 + reason[0] if stopped.is_set() else process.returncode
    finally:
        if relay and not process.stdin.closed:
            process.stdin.close()
        # Descendants belong to this invocation even if their parent exited first.
        kill_group(process, signal.SIGTERM)
        kill_group(process, signal.SIGKILL)
        process.wait()


def native_environment(target, environment, local):
    env = dict(os.environ)
    env.update(environment or {})
    env["PATH"] = str(Path.home() / ".cargo/bin") + os.pathsep + env.get("PATH", "")
    env["CARGO_TARGET_DIR"] = str(target)
    if local:
        return env, [str(AGENT_RUN), "native-build"]
    bus = Path(f"/run/user/{os.getuid()}/bus")
    if not bus.exists():
        raise RuntimeError(f"native desktop requires user session bus: {bus}")
    env.setdefault("DBUS_SESSION_BUS_ADDRESS", f"unix:path={bus}")
    return env, [
        "systemd-run",
        "--user",
        "--scope",
        "--quiet",
        "-p",
        "MemoryMax=16G",
        "-p",
        "CPUQuota=800%",
    ]


def native_build(project, target, cargo_args, release, env, prefix, lease):
    if not cargo_args:
        raise ValueError("native Cargo subcommand is required")
    cargo = [
        "rustup",
        "run",
        TOOLCHAIN,
        "cargo",
        cargo_args[0],
        "--locked",
        "-j8",
        *cargo_args[1:],
    ]
    if release and "--release" not in cargo_args:
        cargo.append("--release")
    print(f"native source={project} target={target} scope={prefix}", flush=True)
    return run_owned([*prefix, *cargo], project, env, lease=lease)


def read_sysroot(project, env, prefix, lease):
    with tempfile.TemporaryFile() as output:
        status = run_owned(
            [*prefix, "rustup", "run", TOOLCHAIN, "rustc", "--print", "sysroot"],
            project,
            env,
            lease=lease,
            stdout=output,
        )
        if status:
            return status, None
        output.seek(0)
        sysroot = output.read().decode().strip()
        if not sysroot or not Path(sysroot).is_absolute():
            raise ValueError(f"invalid native Rust sysroot: {sysroot!r}")
        return 0, Path(sysroot)


def native_runtime(
    project, target, runtime_args, binary, release, env, prefix, lease, runtime_cwd=None
):
    if runtime_args is None:
        return 0
    if binary is None:
        raise ValueError("runtime_args requires binary")
    status, sysroot = read_sysroot(project, env, prefix, lease)
    if status:
        return status
    profile = "release" if release else "debug"
    env["LD_LIBRARY_PATH"] = ":".join(
        filter(
            None,
            [
                str(target / profile / "deps"),
                str(sysroot / "lib"),
                str(sysroot / "lib/rustlib/x86_64-unknown-linux-gnu/lib"),
                env.get("LD_LIBRARY_PATH", ""),
            ],
        )
    )
    executable = target / profile / binary
    if not executable.is_file():
        raise FileNotFoundError(f"missing native runtime binary: {executable}")
    print(f"native artifact={executable}", flush=True)
    return run_owned(
        [*prefix, str(executable), *runtime_args],
        project if runtime_cwd is None else runtime_cwd,
        env,
        lease=lease,
    )


def native_commands(
    project,
    target,
    cargo_args,
    runtime_args,
    binary,
    release,
    environment,
    lease,
    local,
    runtime_cwd=None,
    build=True,
):
    env, prefix = native_environment(target, environment, local)
    if build:
        status = native_build(project, target, cargo_args, release, env, prefix, lease)
        if status:
            return status
    return native_runtime(
        project, target, runtime_args, binary, release, env, prefix, lease, runtime_cwd
    )


def worker(
    archive,
    checkout_key,
    project_name,
    cargo_args,
    runtime_args,
    binary,
    release,
    environment,
    monitor=True,
    runtime_cwd=None,
    build=True,
):
    validate_key(checkout_key)
    validate_key(project_name)
    state = CACHE_ROOT / checkout_key
    state.mkdir(parents=True, exist_ok=True)
    project = state / "source" / project_name
    target = state / "target"
    with lifetime(monitor) as lease:
        env, prefix = native_environment(target, environment, False)
        with (state / "lock").open("a") as lock:
            while True:
                try:
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    if lease[0].wait(0.2):
                        return 128 + lease[1][0]
            with tempfile.TemporaryDirectory(
                prefix="incoming-", dir=state
            ) as temporary:
                snapshot = Path(temporary)
                extract_directory(archive, snapshot)
                sync_source(snapshot, state / "source", state / "manifest.json")
            if build:
                status = native_build(
                    project, target, cargo_args, release, env, prefix, lease
                )
                if status:
                    return status
        # Runtime owns its lease, not the source synchronization/build lock.
        return native_runtime(
            project,
            target,
            runtime_args,
            binary,
            release,
            env,
            prefix,
            lease,
            runtime_cwd,
        )


def upload(source, destination):
    """Retry only idempotent SCP writes; own every attempt's process group."""
    command = [
        "scp",
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=10",
        str(source),
        destination,
    ]
    transient = (
        "connection reset",
        "timed out",
        "lost connection",
        "broken pipe",
        "connection closed",
    )
    permanent = (
        "permission denied",
        "no such file",
        "not a directory",
        "authentication failed",
        "host key verification failed",
    )
    with lifetime() as (stopped, reason):
        for attempt in range(1, 4):
            if stopped.is_set():
                return 128 + reason[0]
            timed_out = False
            with tempfile.TemporaryFile() as errors:
                process = subprocess.Popen(
                    command,
                    start_new_session=True,
                    stdin=subprocess.DEVNULL,
                    stderr=errors,
                )
                deadline = time.monotonic() + UPLOAD_TIMEOUT
                try:
                    while process.poll() is None and not stopped.is_set():
                        remaining = deadline - time.monotonic()
                        if remaining <= 0:
                            timed_out = True
                            break
                        stopped.wait(min(0.1, remaining))
                finally:
                    kill_group(process, signal.SIGTERM)
                    try:
                        process.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        kill_group(process, signal.SIGKILL)
                        process.wait()
                    kill_group(process, signal.SIGKILL)
                errors.seek(0)
                stderr = errors.read().decode(errors="replace").strip()
            if stopped.is_set():
                return 128 + reason[0]
            if not timed_out and process.returncode == 0:
                return 0
            detail = (
                f"timed out after {UPLOAD_TIMEOUT}s"
                if timed_out
                else f"exit {process.returncode}"
            )
            message = f"native upload {source} -> {destination}, attempt {attempt}/3: {detail}: {stderr}"
            print(message, file=sys.stderr, flush=True)
            diagnostic = stderr.lower()
            if (
                attempt == 3
                or any(text in diagnostic for text in permanent)
                or not (timed_out or any(text in diagnostic for text in transient))
            ):
                if timed_out:
                    raise TimeoutError(message)
                raise subprocess.CalledProcessError(
                    process.returncode, command, stderr=stderr
                )
            stopped.wait(
                UPLOAD_BACKOFF * 2 ** (attempt - 1) + random.uniform(0, UPLOAD_BACKOFF)
            )
    return 128 + reason[0]


def desktop(
    context,
    key,
    project,
    cargo_args,
    runtime_args,
    binary,
    release,
    environment,
    runtime_cwd=None,
    build=True,
):
    transfer = windows_profile() / "data/build-host/transfers" / uuid.uuid4().hex
    with tempfile.TemporaryDirectory(
        prefix="native-transfer-", dir=context.parent
    ) as temporary:
        archive = Path(temporary) / "source.tar.gz"
        pack_directory(context, archive)
        powershell(
            f"New-Item -ItemType Directory -Path {ps_literal(transfer)} -ErrorAction Stop | Out-Null"
        )
        try:
            for source, name in (
                (archive, "source.tar.gz"),
                (Path(__file__), "native_build_hosts.py"),
                (Path(__file__).with_name("build_hosts.py"), "build_hosts.py"),
            ):
                status = upload(source, f"desktop:{(transfer / name).as_posix()}")
                if status:
                    return status
            request = json.dumps(
                [
                    key,
                    project,
                    cargo_args,
                    runtime_args,
                    binary,
                    release,
                    environment,
                    True,
                    str(runtime_cwd) if runtime_cwd is not None else None,
                    build,
                ]
            )
            command = subprocess.list2cmdline(
                [
                    "wsl.exe",
                    "-d",
                    "OssoBuild",
                    "-u",
                    "osso-test",
                    "--exec",
                    "python3",
                    wsl_path(transfer / "native_build_hosts.py"),
                    wsl_path(transfer / "source.tar.gz"),
                    request,
                ]
            )
            return run_owned(
                ["ssh", "desktop", command], context, dict(os.environ), relay=True
            )
        finally:
            powershell(
                f"Remove-Item -LiteralPath {ps_literal(transfer)} -Recurse -Force -ErrorAction Stop"
            )


def execute(
    context: Path,
    checkout_key: str,
    project_name: str,
    cargo_args: list[str],
    host: str,
    runtime_args: list[str] | None = None,
    binary: str | None = None,
    release: bool = False,
    environment: dict[str, str] | None = None,
    root: Path | None = None,
    runtime_cwd: Path | None = None,
    build: bool = True,
) -> int:
    """Optionally build/run on chosen host; never export artifacts or fall back."""
    validate_key(checkout_key)
    validate_key(project_name)
    if host not in {"local", "desktop"}:
        raise ValueError(f"unsupported native host: {host!r}")
    if runtime_args is not None and (binary is None or Path(binary).name != binary):
        raise ValueError("runtime requires a plain binary name")
    if host == "local":
        if root is None:
            raise ValueError("local native execution requires original project root")
        root = root.resolve(strict=True)
        with lifetime() as lease:
            return native_commands(
                root,
                root / "target",
                cargo_args,
                runtime_args,
                binary,
                release,
                environment,
                lease,
                True,
                runtime_cwd,
                build,
            )
    return desktop(
        context.resolve(strict=True),
        checkout_key,
        project_name,
        cargo_args,
        runtime_args,
        binary,
        release,
        environment,
        runtime_cwd,
        build,
    )


if __name__ == "__main__":
    try:
        sys.exit(worker(Path(sys.argv[1]), *json.loads(sys.argv[2])))
    except Exception as error:
        print(f"native worker failed: {error}", file=sys.stderr, flush=True)
        sys.exit(1)
