#!/usr/bin/env python3
"""Owned desktop Vulkan/login/input/screenshot smoke; run as osso-test in OssoBuild.

Requires staged root/{bin/godot,bin/game-engine-cli,godot/,target/debug/,data/},
Weston, and test-only Dozen ICD. No window appears on the Windows desktop.
"""
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time


def stop_process(process):
    if process.poll() is None:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()


def read_cli(root, socket, environment, *args):
    result = subprocess.run([str(root / "bin/game-engine-cli"), "--socket", str(socket), *args],
                            cwd=root, env=environment, text=True, capture_output=True, timeout=10)
    if result.returncode:
        raise RuntimeError(f"IPC {args[0]} failed: {result.stdout}\n{result.stderr}")
    return result.stdout


def smoke(root):
    root = root.resolve()
    output = root / "target/desktop-gpu-smoke"
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="runtime-", dir=output) as directory:
        runtime = Path(directory)
        environment = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), WAYLAND_DISPLAY="owned-gpu-test",
                           XDG_CONFIG_HOME=str(runtime / "config"), XDG_DATA_HOME=str(runtime / "data"),
                           VK_DRIVER_FILES="/opt/game-engine/mesa-dzn/share/vulkan/icd.d/dzn_icd.x86_64.json",
                           LD_LIBRARY_PATH="/usr/lib/wsl/lib")
        environment.pop("DISPLAY", None)
        script = runtime / "login-input.js"
        script.write_text('ui.waitForFrame("UsernameInput", 15.0);\nui.click("UsernameInput");\nui.type("desktop-probe");\nui.dumpUiTree();\n')
        with (output / "weston.log").open("w") as compositor_log, (output / "godot.log").open("w") as client_log:
            weston = subprocess.Popen(["weston", "--backend=headless", "--renderer=pixman", "--no-config",
                                       "--socket=owned-gpu-test", "--width=1280", "--height=720"],
                                      env=environment, stdout=compositor_log, stderr=compositor_log,
                                      start_new_session=True)
            client = None
            try:
                deadline = time.monotonic() + 60
                while not (runtime / "owned-gpu-test").exists():
                    if weston.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError(f"owned compositor not ready; inspect {output / 'weston.log'}")
                    time.sleep(0.1)
                client = subprocess.Popen([str(root / "bin/godot"), "--path", str(root / "godot"),
                                           "--display-driver", "wayland", "--rendering-driver", "vulkan", "--",
                                           "--screen", "login", "--run-js-ui-script", str(script)],
                                          cwd=root, env=environment, stdout=client_log, stderr=client_log,
                                          start_new_session=True)
                socket = Path(f"/tmp/game-engine-{client.pid}.sock")
                while not socket.exists():
                    if client.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError(f"client IPC not ready; inspect {output / 'godot.log'}")
                    time.sleep(0.1)
                while True:
                    tree = read_cli(root, socket, environment, "dump-ui-tree")
                    if "desktop-probe" in tree:
                        break
                    if time.monotonic() >= deadline:
                        raise TimeoutError("login click/type did not update observable editbox state")
                    time.sleep(0.2)
                (output / "ui-tree.txt").write_text(tree)
                performance = read_cli(root, socket, environment, "performance")
                print(performance, flush=True)
                print(read_cli(root, socket, environment, "screenshot", str(output / "login.webp")), flush=True)
                if "Microsoft Direct3D12 (NVIDIA GeForce RTX 4070 Ti)" not in (output / "godot.log").read_text():
                    raise RuntimeError("Godot did not report RTX-backed Vulkan")
                from PIL import Image
                with Image.open(output / "login.webp") as image:
                    if min(image.size) < 100 or all(low == high for low, high in image.getextrema()):
                        raise RuntimeError("screenshot is empty or uniform")
                print(f"PASS desktop RTX Vulkan: login click/type + IPC screenshot; evidence {output}", flush=True)
            finally:
                if client is not None:
                    stop_process(client)
                stop_process(weston)


if __name__ == "__main__":
    smoke(Path(sys.argv[1]))
