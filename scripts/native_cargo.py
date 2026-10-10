"""Shared host-wide admission and checkout-local Cargo execution."""
import contextlib
import fcntl
import os
from pathlib import Path
import subprocess
import time

NATIVE_SLOTS = int(os.environ.get("GAME_ENGINE_NATIVE_SLOTS", "3"))


@contextlib.contextmanager
def native_slot():
    """Share the engine's host-wide slots with every native server run."""
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


def run_cargo(root, arguments, output=None):
    environment = os.environ | {"CARGO_TARGET_DIR": str(root / "target")}
    environment.pop("CARGO_BUILD_JOBS", None)
    return subprocess.run(
        ["cargo", arguments[0], "--locked", *arguments[1:]],
        cwd=root,
        env=environment,
        stdout=output,
        stderr=subprocess.STDOUT if output else None,
        check=False,
    ).returncode
