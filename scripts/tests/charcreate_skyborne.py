#!/usr/bin/env python3
"""Run offline Skyborne Vulkan acceptance in cage; retain child exit and captures.

Requires host Pillow solely to write the independent raw-BLP icon oracle PNG.
Build the native extension with depot-build.py before invoking this runner.
Invoke through scripts/agent/agent-run skyborne-charcreate.
"""

from __future__ import annotations

import json
import os
import shutil
import struct
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CAPTURES = ROOT / "data/diagnostics/skyborne-charcreate"
RESULT = CAPTURES / "child-result.json"
PINNED = (
    Path.home()
    / ".cache/game-engine/godot/4.7.2-pr123946-pr123546/godot-4.7.2-pr123946-pr123546"
)


def run_godot_child() -> int:
    """Called as GODOT_BIN by headless-client.sh; preserve the real exit code."""
    environment = dict(os.environ)
    environment.pop("SKYBORNE_GODOT_CHILD")
    result = subprocess.run([str(PINNED), *sys.argv[1:]], env=environment, check=False)
    RESULT.write_text(json.dumps({"exit_code": result.returncode}) + "\n")
    return result.returncode


def decode_atlas_pixels(data: bytes) -> tuple[int, int, bytes]:
    """Decode this atlas's BLP2 raw BGRA mip independently of the native decoder."""
    if len(data) < 148 or data[:4] != b"BLP2" or data[8] != 3:
        raise ValueError("FDID 8200220 must be BLP2 raw BGRA")
    width, height = struct.unpack_from("<II", data, 12)
    offset = struct.unpack_from("<I", data, 20)[0]
    size = struct.unpack_from("<I", data, 84)[0]
    if (
        not width
        or not height
        or size != width * height * 4
        or offset + size > len(data)
    ):
        raise ValueError("Invalid raw atlas mip dimensions/length")
    pixels = bytearray(data[offset : offset + size])
    pixels[0::4], pixels[2::4] = pixels[2::4], pixels[0::4]
    return width, height, bytes(pixels)


def prepare_inputs() -> dict[str, str]:
    from PIL import Image

    CAPTURES.mkdir(parents=True, exist_ok=True)
    RESULT.unlink(missing_ok=True)
    for image in CAPTURES.glob("race-*-sex-*.png"):
        image.unlink()
    width, height, pixels = decode_atlas_pixels(
        (ROOT / "data/textures/8200220.blp").read_bytes()
    )
    Image.frombytes("RGBA", (width, height), pixels).save(
        CAPTURES / "atlas-8200220.png"
    )
    config = CAPTURES / "config"
    settings = config / "world-of-osso/options_settings.ron"
    settings.parent.mkdir(parents=True, exist_ok=True)
    settings.write_text(
        "(graphics:(antiAlias:None,bloomEnabled:false,renderScale:1.0,"
        "uiScale:1.0,frameRateLimitEnabled:false,))\n"
    )
    return dict(
        os.environ,
        GODOT_BIN=str(Path(__file__).resolve()),
        SKYBORNE_GODOT_CHILD="1",
        GODOT_TEST_CAPTURE_DIR=str(CAPTURES),
        XDG_CONFIG_HOME=str(config),
    )


def wait_for_child(timeout: float) -> int:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if RESULT.exists():
            return int(json.loads(RESULT.read_text())["exit_code"])
        time.sleep(1)
    raise TimeoutError(
        f"No normal Godot exit within {timeout}s; inspect headless-client.log"
    )


def run_fixture() -> int:
    if not PINNED.is_file():
        raise FileNotFoundError(PINNED)
    environment = prepare_inputs()
    helper = ROOT / "scripts/agent/headless-client.sh"
    # This helper owns one cage PID/log per checkout; never stop someone else's cage.
    pidfile = ROOT / "target/headless-client.pid"
    if pidfile.exists():
        raise RuntimeError(
            f"Existing cage ownership at {pidfile}; resolve before running"
        )
    log = ROOT / "target/skyborne-native-flow.log"
    try:
        subprocess.run(
            [
                str(helper),
                "start-godot",
                str(ROOT),
                environment["XDG_CONFIG_HOME"],
                "-s",
                "res://tests/charcreate_skyborne_flow.gd",
                "--",
                "--screen",
                "charcreate",
            ],
            env=environment,
            check=True,
        )
        code = wait_for_child(240)
        source = ROOT / "target/headless-client.log"
        shutil.copyfile(source, log)
        output = log.read_text()
        print(output, end="")
        print(f"Native child exit={code}; log={log}; screenshots={CAPTURES}")
        if code != 0:
            return 1
        required = [
            CAPTURES / f"race-{race}-sex-{sex}{suffix}.png"
            for race in (95, 96)
            for sex in (0, 1)
            for suffix in ("", "-control", "-empty", "-restored")
        ]
        if "SKYBORNE RESULT variants=4/4 failures=false engine_errors=0" not in output:
            raise RuntimeError("Native child did not report four successful variants")
        if any(not path.is_file() for path in required):
            raise RuntimeError("Missing native capture/control image")
        if (
            "ERROR:" in output
            or "were leaked" in output
            or "still in use at exit" in output
        ):
            raise RuntimeError(
                "Native acceptance log retains engine errors/resource leaks"
            )
        return 0
    finally:
        source = ROOT / "target/headless-client.log"
        if source.exists():
            shutil.copyfile(source, log)
        subprocess.run([str(helper), "stop", str(ROOT / "target")], check=True)


if __name__ == "__main__":
    if os.environ.get("SKYBORNE_GODOT_CHILD") == "1":
        sys.exit(run_godot_child())
    sys.exit(run_fixture())
