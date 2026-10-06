#!/usr/bin/env python3
"""Offline native launcher captures. Invoke under build-lock + agent-run launcherart."""
import json
import os
from pathlib import Path
import signal
import subprocess
import time

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "data/diagnostics/launcherart-2026-10-05"
RUNTIME = ROOT / "target/la-capture"
GODOT = ROOT / "target/party-preview/bin/godot"


def stop_process(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def capture_candidate(skin, style, env):
    name = f"{skin}-{style}"
    destination = OUT / f"{name}.png"
    env = dict(env, GODOT_LAUNCHER_SKIN=skin, GODOT_LAUNCHER_STYLE=style,
               GODOT_CAPTURE_PATH=str(destination))
    args = [str(GODOT), "--path", str(ROOT / "godot"), "--display-driver", "wayland",
            "--rendering-driver", "vulkan", "--audio-driver", "Dummy",
            "--resolution", "1920x1080", "--script", "res://tests/capture_launcher_candidates.gd"]
    with (OUT / f"{name}.log").open("w") as log:
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
    # Full metal-frame outsets retained, plus player-name context below it.
    crop = image.crop((620, 290, 1300, 855))
    crop.resize((crop.width * 2, crop.height * 2), Image.Resampling.NEAREST).save(
        OUT / f"{name}-2x.png")
    magnifier = image.crop((1580, 210, 1730, 305))
    magnifier.resize((300, 190), Image.Resampling.NEAREST).save(OUT / f"{name}-minimap-2x.png")
    print(f"PASS {name}: {destination}", flush=True)
    return {"pid": client.pid, "exit": code, "full": str(destination)}


def capture_candidates():
    OUT.mkdir(parents=True, exist_ok=True)
    RUNTIME.mkdir(exist_ok=True)
    RUNTIME.chmod(0o700)
    env = dict(os.environ, XDG_RUNTIME_DIR=str(RUNTIME), WAYLAND_DISPLAY="la",
               VK_DRIVER_FILES="/opt/game-engine/mesa-dzn/share/vulkan/icd.d/dzn_icd.x86_64.json",
               LD_LIBRARY_PATH="/usr/lib/wsl/lib")
    status = {}
    with (OUT / "weston.log").open("w") as log:
        weston = subprocess.Popen(
            ["weston", "--backend=headless", "--renderer=pixman", "--no-config",
             "--socket=la", "--width=1920", "--height=1080", "--idle-time=0"],
            env=env, stdout=log, stderr=subprocess.STDOUT)
        status["weston_pid"] = weston.pid
        try:
            for _ in range(100):
                if (RUNTIME / "la").exists():
                    break
                if weston.poll() is not None:
                    raise RuntimeError("Weston exited before socket")
                time.sleep(0.1)
            else:
                raise RuntimeError("Weston socket timeout")
            for skin in ["modern", "forever"]:
                for style in ["filled", "outline"]:
                    status[f"{skin}-{style}"] = capture_candidate(skin, style, env)
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
            (OUT / "capture-status.json").write_text(json.dumps(status, indent=2))


if __name__ == "__main__":
    capture_candidates()
