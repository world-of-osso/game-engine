#!/usr/bin/env python3
"""Offline native filled launcher captures. Invoke under build-lock + agent-run."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "target/launcherhdr-capture"


def stop_process(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def capture_candidate(skin, env, godot, out):
    name = f"{skin}-filled-final"
    destination = out / f"{name}.png"
    env = dict(env, GODOT_LAUNCHER_SKIN=skin, GODOT_CAPTURE_PATH=str(destination))
    args = [str(godot), "--path", str(ROOT / "godot"), "--display-driver", "wayland",
            "--rendering-driver", "vulkan", "--audio-driver", "Dummy",
            "--resolution", "1920x1080", "--script", "res://tests/capture_launcher_candidates.gd"]
    with (out / f"{name}.log").open("w") as log:
        client = subprocess.Popen(args, env=env, stdout=log, stderr=subprocess.STDOUT)
        try:
            code = client.wait(timeout=120)
        finally:
            stop_process(client)
    if code != 0:
        raise RuntimeError(f"{name}: client exit {code}; see {name}.log")
    image = Image.open(destination)
    if image.size != (1920, 1080):
        raise RuntimeError(f"{name}: invalid capture size {image.size}")
    # The offline canvas is native 1:1. Include the metal-frame outsets.
    left, top = (1920 - 526) // 2, (1080 - 474) // 2
    crops = {
        "launcher": image.crop((left - 8, top - 16, left + 530, top + 482)),
        "header": image.crop((left - 8, top - 16, left + 530, top + 36)),
    }
    for label, crop in crops.items():
        crop.save(out / f"{skin}-{label}-1x.png")
        crop.resize((crop.width * 2, crop.height * 2), Image.Resampling.NEAREST).save(
            out / f"{skin}-{label}-2x.png")
    print(f"PASS {name}: {destination}", flush=True)
    return {"pid": client.pid, "exit": code, "full": str(destination)}


def capture_candidates(godot, out):
    if not godot.is_file():
        raise FileNotFoundError(f"Godot executable missing: {godot}")
    out.mkdir(parents=True, exist_ok=True)
    RUNTIME.mkdir(exist_ok=True)
    RUNTIME.chmod(0o700)
    env = dict(os.environ, XDG_RUNTIME_DIR=str(RUNTIME), WAYLAND_DISPLAY="launcherhdr",
               VK_DRIVER_FILES="/opt/game-engine/mesa-dzn/share/vulkan/icd.d/dzn_icd.x86_64.json",
               LD_LIBRARY_PATH="/usr/lib/wsl/lib")
    status = {}
    with (out / "weston-final.log").open("w") as log:
        weston = subprocess.Popen(
            ["weston", "--backend=headless", "--renderer=pixman", "--no-config",
             "--socket=launcherhdr", "--width=1920", "--height=1080", "--idle-time=0"],
            env=env, stdout=log, stderr=subprocess.STDOUT)
        status["weston_pid"] = weston.pid
        try:
            for _ in range(100):
                if (RUNTIME / "launcherhdr").exists():
                    break
                if weston.poll() is not None:
                    raise RuntimeError("Weston exited before socket")
                time.sleep(0.1)
            else:
                raise RuntimeError("Weston socket timeout")
            for skin in ["modern", "forever"]:
                status[skin] = capture_candidate(skin, env, godot, out)
        finally:
            children = Path(f"/proc/{weston.pid}/task/{weston.pid}/children")
            pids = [int(pid) for pid in children.read_text().split()] if children.exists() else []
            stop_process(weston)
            for pid in pids:
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            status["weston_exit"] = weston.returncode
            status["weston_children"] = pids
            (out / "capture-status-final.json").write_text(json.dumps(status, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    capture_candidates(arguments.godot.resolve(), arguments.output.resolve())
