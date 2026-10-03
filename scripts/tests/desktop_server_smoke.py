#!/usr/bin/env python3
"""Run an isolated desktop server/admin/UDP-login smoke test on staged Linux artifacts.

Run inside OssoBuild: python3 desktop_server_smoke.py /root/data/game-engine-trial/server
The staged root must contain bin/{game-server,game-server-admin,game-cli} and
read-only data/world.db, data/gametables, and data/economy/config.json.
Never copies existing player storage.
"""
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import uuid


def run_checked(command, root, environment, timeout=20):
    result = subprocess.run(command, cwd=root, env=environment, text=True,
                            capture_output=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"{Path(command[0]).name} failed ({result.returncode}): {result.stdout}\n{result.stderr}")
    print(result.stdout, end="", flush=True)
    return result.stdout


def smoke(root):
    root = root.resolve()
    for relative in ("data/world.db", "data/economy/config.json"):
        if not (root / relative).is_file():
            raise FileNotFoundError(root / relative)
    owned = root / "data/smoke"
    owned.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="run-", dir=owned) as directory:
        scratch = Path(directory)
        environment = dict(os.environ, GAME_SERVER_DB_PATH=str(scratch / "game.redb"),
                           GAME_SERVER_BIND_IP="127.0.0.1", GAME_SERVER_PORT="15001",
                           GAME_SERVER_ADMIN_SOCKET=str(scratch / "admin.sock"),
                           XDG_CONFIG_HOME=str(scratch / "config"), HOME=str(scratch))
        admin = root / "bin/game-server-admin"
        command = [str(admin), "ping"]
        log_path = owned / "server.log"
        with log_path.open("w") as log:
            process = subprocess.Popen([str(root / "bin/game-server")], cwd=root,
                                       env=environment, stdout=log, stderr=log,
                                       start_new_session=True)
            try:
                deadline = time.monotonic() + 60
                while not (scratch / "admin.sock").exists():
                    if process.poll() is not None:
                        raise RuntimeError(f"server exited {process.returncode}: {log_path.read_text()[-4000:]}")
                    if time.monotonic() >= deadline:
                        raise TimeoutError(f"admin socket not ready; inspect {log_path}")
                    time.sleep(0.2)
                # The socket is bound before world/economy startup finishes.
                # Readiness is the response, within the original startup deadline.
                remaining = max(0.1, deadline - time.monotonic())
                if run_checked(command, root, environment, timeout=remaining).strip() != "pong":
                    raise RuntimeError("admin ping did not return pong")
                username = "hosttrial" + uuid.uuid4().hex[:8]
                # Disposable credentials exist only in this owned, deleted test database.
                password = uuid.uuid4().hex
                run_checked([str(admin), "create-account", username, password], root, environment)
                output = run_checked([str(root / "bin/game-cli"), "login", "--server", "127.0.0.1:15001",
                                      "--username", username, "--password", password], root, environment)
                if "[login] Characters:" not in output:
                    raise RuntimeError("UDP client did not receive an authenticated roster")
                print("PASS desktop server: admin pong + disposable-account UDP login", flush=True)
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
                print(f"Owned server stopped; log: {log_path}", flush=True)


if __name__ == "__main__":
    smoke(Path(sys.argv[1]))
