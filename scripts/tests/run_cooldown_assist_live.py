#!/usr/bin/env python3
"""Drive the real owned combat-assist fixture against an already-running private realm.

Uses the build-qualified Retail placement helper before login; never NPC legacy Z.
Starts/stops only its slot's private Cage client. Server/account creation stays external.
"""
import argparse
import json
import re
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


def run_owned(arguments, argv, environment):
    # The user scope connects through the host runtime; private GUI env belongs
    # after agent-run, on its child, not on systemd-run's user-bus connection.
    changes = [f"{key}={value}" for key, value in environment.items()
               if os.environ.get(key) != value]
    removals = [part for key in os.environ if key not in environment for part in ("-u", key)]
    result = subprocess.run(
        [str(arguments.engine / "scripts/agent/agent-run"), arguments.agent,
         "env", *removals, *changes, *argv],
        capture_output=True, text=True, check=False,
    )
    with (arguments.out / "driver.log").open("a") as log:
        log.write(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    return result.stdout


def client_log(arguments):
    path = arguments.engine / "target/headless-client.log"
    return path.read_text(errors="replace") if path.exists() else ""


def wait_for_marker(arguments, filename):
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        if (arguments.out / filename).exists():
            return
        output = client_log(arguments)
        if "SCRIPT ERROR" in output or "Cooldown assist fixture:" in output:
            raise RuntimeError(output[-2500:])
        time.sleep(0.25)
    raise RuntimeError(f"private fixture timed out waiting for {filename}")


def drive(arguments):
    if arguments.port == 5000 or not arguments.account.startswith("fb_"):
        raise ValueError("private port and fb_* account required")
    arguments.out.mkdir(parents=True, exist_ok=False)
    environment = os.environ.copy()
    environment["GAME_SERVER_ADMIN_SOCKET"] = f"/tmp/game-server-admin-{arguments.port}.sock"
    admin = arguments.server / "target/debug/game-server-admin"
    run_owned(arguments, ["python3", str(arguments.server / "scripts/place-private-character.py"),
                         "cooldown-assist", arguments.account, arguments.character,
                         "--admin", str(admin)], environment)
    for name in ["config", "userdata"]:
        path = arguments.out / name
        if arguments.restore_from is not None:
            shutil.copytree(arguments.restore_from / name, path)
        else:
            path.mkdir(mode=0o700)
    environment.update({
        "XDG_DATA_HOME": str(arguments.out / "userdata"),
        "ASSET_RESOLVER_CACHE_DIR": str(arguments.resolver_cache),
        "ASSET_RESOLVER_SHARED_DATA_DIR": str(arguments.engine / "data"),
        "COOLDOWN_ASSIST_RUN": str(arguments.out),
        "COOLDOWN_ASSIST_SERVER": f"127.0.0.1:{arguments.port}",
        "COOLDOWN_ASSIST_ACCOUNT": arguments.account,
        "COOLDOWN_ASSIST_ADMIN": "1",
        "COOLDOWN_ASSIST_SKIN": arguments.skin,
    })
    if arguments.restore_from is not None:
        environment["COOLDOWN_ASSIST_RESTORE"] = "1"
        placement = json.loads((arguments.restore_from / "viewer-placement.json").read_text())
        position = re.findall(r"-?\d+(?:\.\d+)?", placement["saved"])[:2]
        if len(position) != 2:
            raise ValueError("successful run has no saved viewer position")
        environment["COOLDOWN_ASSIST_EXPECTED_POSITION"] = ",".join(position)
    environment.pop("DISPLAY", None)
    helper = str(arguments.engine / "scripts/agent/headless-client.sh")
    with tempfile.TemporaryDirectory(prefix="cdassist-", dir="/tmp") as runtime:
        environment["XDG_RUNTIME_DIR"] = runtime
        started = False
        try:
            run_owned(arguments, [helper, "start-godot", str(arguments.engine),
                                  str(arguments.out / "config"), "--display-driver", "wayland",
                                  "--rendering-driver", "vulkan", "--audio-driver", "Dummy",
                                  "--resolution", "1920x1080", "--script",
                                  "res://tests/cooldown_assist_live.gd"], environment)
            started = True
            wait_for_marker(arguments, "online.ready")
            if arguments.restore_from is not None:
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    output = client_log(arguments)
                    if "RESTORE_PASS:" in output:
                        print("RESTORE_PASS: " + arguments.skin + " persisted manager choices/layout")
                        return 0
                    if "SCRIPT ERROR" in output or "Cooldown assist fixture:" in output:
                        raise RuntimeError(output[-2500:])
                    time.sleep(0.25)
                raise RuntimeError("persisted layout restoration timeout")
            for spell in [642, 31884]:
                run_owned(arguments, [str(admin), "learn-spell", arguments.character, str(spell)], environment)
            run_owned(arguments, [str(admin), "revive", arguments.character], environment)
            (arguments.out / "server-setup.done").write_text("private grants and revive complete")
            wait_for_marker(arguments, "combat.ready")
            run_owned(arguments, [str(admin), "revive", arguments.character], environment)
            (arguments.out / "combat-setup.done").write_text("private combat readiness complete")
            deadline = time.monotonic() + 90
            while time.monotonic() < deadline:
                output = client_log(arguments)
                if "PASS:" in output:
                    print("PASS: " + arguments.skin + " private combat assist")
                    return 0
                if "PARTIAL:" in output:
                    print(output[output.index("PARTIAL:"):][-2500:])
                    return 2
                if "SCRIPT ERROR" in output or "Cooldown assist fixture:" in output:
                    raise RuntimeError(output[-2500:])
                time.sleep(0.25)
            raise RuntimeError("private combat fixture final timeout")
        finally:
            if started:
                shutil.copy2(arguments.engine / "target/headless-client.log", arguments.out / "client.log")
                run_owned(arguments, [helper, "stop", str(arguments.engine / "target")], environment)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["engine", "server", "out", "resolver-cache"]:
        parser.add_argument("--" + name, type=Path, required=True)
    for name in ["account", "character", "agent"]:
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--skin", choices=["Forever"], default="Forever")
    parser.add_argument("--restore-from", type=Path, help="existing successful run whose config/userdata must survive a fresh client")
    arguments = parser.parse_args()
    for name in ["engine", "server", "out", "resolver_cache"]:
        setattr(arguments, name, getattr(arguments, name).resolve())
    try:
        return drive(arguments)
    except (OSError, RuntimeError, ValueError) as error:
        print(str(error))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
