#!/usr/bin/env python3
"""Local/desktop Docker transport for an already prepared source-only context.

Builder provisioning and source selection belong to the caller. This file is
also uploaded as the fixed OssoBuild worker; no network/config fallback exists.
"""

import base64
import ctypes
import fcntl
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import uuid
from decimal import Decimal
from pathlib import Path, PureWindowsPath

CACHE_ROOT = Path("/root/.cache/game-engine")


def validate_key(checkout_key: str) -> None:
    if not re.fullmatch(r"[A-Za-z0-9_-]+", checkout_key):
        raise ValueError(f"invalid checkout cache key: {checkout_key!r}")


def docker_command(context, output, checkout_key, target, build_args):
    return [
        "docker",
        "buildx",
        "build",
        "--builder",
        "game-engine",
        "--platform",
        "linux/amd64",
        "--file",
        str(context / "Dockerfile"),
        "--target",
        target,
        "--build-arg",
        f"TARGET_CACHE=godot-target-{checkout_key}",
        "--build-arg",
        "JOBS=8",
        "--output",
        f"type=local,dest={output}",
        *build_args,
        str(context),
    ]


def run_build_command(command: list[str]) -> None:
    """Keep the client in the caller's cgroup, watched even after SIGKILL."""
    subprocess.run(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            "--owned-build",
            str(os.getpid()),
            json.dumps(command),
        ],
        check=True,
    )


def watch_build(parent_pid: int, command: list[str]) -> int:
    """Linux guardian: parent death/termination requests a graceful solve cancel."""
    cancelled = False

    def request_cancel(signum, frame):
        nonlocal cancelled
        cancelled = True

    for signum in (signal.SIGTERM, signal.SIGINT):
        signal.signal(signum, request_cancel)
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(1, signal.SIGTERM, 0, 0, 0) != 0:  # PR_SET_PDEATHSIG
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error))
    # Parent may have died before prctl armed the notification.
    if os.getppid() != parent_pid or cancelled:
        return 130
    with subprocess.Popen(command, start_new_session=True) as client:
        while not cancelled:
            try:
                return client.wait(timeout=0.1)
            except subprocess.TimeoutExpired:
                continue
        try:
            os.killpg(client.pid, signal.SIGINT)
        except ProcessLookupError:
            return client.wait()
        try:
            client.wait(timeout=20)
        except subprocess.TimeoutExpired:
            print(
                "build client ignored cancellation for 20s; killing owned group",
                file=sys.stderr,
            )
            os.killpg(client.pid, signal.SIGKILL)
            client.wait()
        return 130


def pack_directory(source: Path, archive: Path) -> None:
    with tarfile.open(archive, "w:gz", format=tarfile.PAX_FORMAT) as bundle:
        for path in sorted(source.rglob("*")):
            if path.is_symlink() or not (path.is_file() or path.is_dir()):
                raise ValueError(f"unsupported archive source: {path}")
            member = bundle.gettarinfo(str(path), str(path.relative_to(source)))
            member.pax_headers["mtime"] = str(
                Decimal(path.stat().st_mtime_ns) / Decimal(10**9)
            )
            if path.is_file():
                with path.open("rb") as data:
                    bundle.addfile(member, data)
            else:
                bundle.addfile(member)


def extract_directory(archive: Path, destination: Path) -> None:
    """Validate the entire archive before writing; reject links and devices."""
    root = destination.resolve()
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        for member in members:
            relative = Path(member.name)
            path = destination / relative
            if (
                relative.is_absolute()
                or ".." in relative.parts
                or not (member.isfile() or member.isdir())
                or not path.resolve().is_relative_to(root)
                or any(p.is_symlink() for p in (path, *path.parents))
            ):
                raise ValueError(f"unsafe archive member: {member.name!r}")
        destination.mkdir(parents=True, exist_ok=True)
        bundle.extractall(destination, members=members, filter="data")
        # PAX timestamps retain nanoseconds, whereas TarInfo.mtime is a float.
        for member in reversed(members):
            timestamp = Decimal(member.pax_headers.get("mtime", str(member.mtime)))
            nanoseconds = int(timestamp * Decimal(10**9))
            path = destination / member.name
            os.utime(path, ns=(nanoseconds, nanoseconds))


def worker(
    archive: Path,
    output_archive: Path,
    checkout_key: str,
    target: str,
    build_args: list[str],
) -> None:
    validate_key(checkout_key)
    CACHE_ROOT.mkdir(parents=True, exist_ok=True)
    with (CACHE_ROOT / f"{checkout_key}.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        context = CACHE_ROOT / f"context-{checkout_key}"
        # Keep the path stable, but never leak removed sources into a later build.
        if context.exists():
            shutil.rmtree(context)
        try:
            extract_directory(archive, context)
            with tempfile.TemporaryDirectory(
                prefix=f"output-{checkout_key}-", dir=CACHE_ROOT
            ) as temporary:
                output = Path(temporary)
                run_build_command(
                    docker_command(context, output, checkout_key, target, build_args)
                )
                pack_directory(output, output_archive)
        finally:
            if context.exists():
                shutil.rmtree(context)
        # Lock files and Docker-managed caches deliberately survive requests.


def powershell(script: str, capture=False):
    encoded = base64.b64encode(script.encode("utf-16-le")).decode("ascii")
    command = f"powershell.exe -NoProfile -NonInteractive -EncodedCommand {encoded}"
    return subprocess.run(
        ["ssh", "desktop", command], check=True, capture_output=capture
    )


def windows_profile() -> PureWindowsPath:
    result = powershell("$env:USERPROFILE", capture=True)
    encoding = "utf-16-le" if b"\0" in result.stdout else "utf-8-sig"
    text = result.stdout.decode(encoding).replace("\0", "").strip().lstrip("\ufeff")
    profile = PureWindowsPath(text)
    if not profile.is_absolute() or not re.fullmatch(r"[A-Za-z]:", profile.drive):
        raise ValueError(f"unsupported Windows USERPROFILE: {text!r}")
    return profile


def wsl_path(path: PureWindowsPath) -> str:
    return f"/mnt/{path.drive[0].lower()}/" + "/".join(path.parts[1:])


def ps_literal(value) -> str:
    return "'" + str(value).replace("'", "''") + "'"


def desktop(context, output, checkout_key, target, build_args):
    transfer = windows_profile() / "data/build-host/transfers" / uuid.uuid4().hex
    remote_source = transfer / "source.tar.gz"
    remote_worker = transfer / "worker.py"
    remote_output = transfer / "output.tar.gz"
    # Transfer scratch stays in data/, never /tmp; the caller owns the context.
    with tempfile.TemporaryDirectory(
        prefix="build-host-", dir=context.parent
    ) as temporary:
        scratch = Path(temporary)
        archive = scratch / "source.tar.gz"
        result = scratch / "output.tar.gz"
        pack_directory(context, archive)
        powershell(
            f"New-Item -ItemType Directory -Path {ps_literal(transfer)} -ErrorAction Stop | Out-Null"
        )
        try:
            for source, destination in (
                (archive, remote_source),
                (Path(__file__).resolve(), remote_worker),
            ):
                subprocess.run(
                    ["scp", str(source), f"desktop:{destination.as_posix()}"],
                    check=True,
                )
            command = subprocess.list2cmdline(
                [
                    "wsl.exe",
                    "-d",
                    "OssoBuild",
                    "-u",
                    "root",
                    "--exec",
                    "python3",
                    wsl_path(remote_worker),
                    wsl_path(remote_source),
                    wsl_path(remote_output),
                    checkout_key,
                    target,
                    json.dumps(build_args),
                ]
            )
            subprocess.run(["ssh", "desktop", command], check=True)
            subprocess.run(
                ["scp", f"desktop:{remote_output.as_posix()}", str(result)], check=True
            )
            extract_directory(result, output)
        finally:
            powershell(
                f"Remove-Item -LiteralPath {ps_literal(transfer)} -Recurse -Force -ErrorAction Stop"
            )


def execute(
    context: Path,
    output: Path,
    checkout_key: str,
    target: str,
    build_args: list[str],
    host: str,
) -> None:
    """Build or raise; errors never select another host or create a builder."""
    validate_key(checkout_key)
    if host not in {"local", "desktop"}:
        raise ValueError(f"unsupported build host: {host!r}")
    context, output = context.resolve(), output.resolve()
    if host == "local":
        run_build_command(
            docker_command(context, output, checkout_key, target, build_args)
        )
    else:
        desktop(context, output, checkout_key, target, build_args)


if __name__ == "__main__":
    if sys.argv[1] == "--owned-build":
        status = watch_build(int(sys.argv[2]), json.loads(sys.argv[3]))
        sys.exit(status if status >= 0 else 128 - status)
    archive, output, key, target, arguments = sys.argv[1:]
    worker(Path(archive), Path(output), key, target, json.loads(arguments))
