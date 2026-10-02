#!/usr/bin/env python3
"""Run godot/tests/frame_benchmark.gd in a headless cage and profile each segment.

Usage: frame-benchmark.py --checkout DIR --server HOST:PORT --account A --character C
           --log FILE [--agent NAME] [--perf [--call-graph]]
           [--place X Y Z --admin game-server-admin --admin-socket SOCK] [KEY=VALUE ...]

--place puts the offline character back at WoW position X Y Z first (its facing is kept;
the walk segment only runs forward and backpedals, so it never turns), so every run starts
from the same spot.

The client runs under scripts/agent/agent-run (when --agent is given) inside cage. With
--perf, `perf record` samples the Godot main thread during every BENCH_MARK segment and
writes <log>.<segment>.perf.data; `perf report --no-children --sort dso,sym` splits it.
Godot is killed 20 s after its final FIXTURE line (its exit can hang in a thread join).
"""

import argparse
import os
from pathlib import Path
import re
import signal
import subprocess
import time

GODOT = Path.home() / ".cache/game-engine/godot/4.7.2-pr123946/godot-4.7.2-pr123946"
AGENT_RUN = "/syncthing/Sync/Projects/world-of-osso/game-engine/scripts/agent/agent-run"


def find_godot(checkout):
    marker = f"--path {checkout}/godot -s res://tests/frame_benchmark.gd"
    for proc in Path("/proc").iterdir():
        if not proc.name.isdigit():
            continue
        try:
            cmdline = (proc / "cmdline").read_bytes().replace(b"\0", b" ").decode()
        except OSError:
            continue
        if cmdline.startswith(str(GODOT)) and marker in cmdline:
            return int(proc.name)
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkout", required=True)
    parser.add_argument("--server", required=True)
    parser.add_argument("--account", required=True)
    parser.add_argument("--character", required=True)
    parser.add_argument("--log", required=True)
    parser.add_argument("--agent")
    parser.add_argument("--perf", action="store_true")
    parser.add_argument("--call-graph", action="store_true",
                        help="with --perf, record DWARF call graphs (inclusive cost per caller)")
    parser.add_argument("--timeout", type=int, default=1500)
    parser.add_argument("--place", nargs=3)
    parser.add_argument("--admin")
    parser.add_argument("--admin-socket")
    parser.add_argument("env", nargs="*", help="extra KEY=VALUE environment")
    args = parser.parse_args()

    checkout = os.path.realpath(args.checkout)
    log = Path(args.log)
    env = dict(os.environ)
    env.pop("DISPLAY", None)
    env.update(WLR_BACKENDS="headless", WLR_LIBINPUT_NO_DEVICES="1", WLR_RENDERER="vulkan",
               GODOT_TEST_SERVER=args.server, BENCH_ACCOUNT=args.account,
               BENCH_CHARACTER=args.character)
    env.setdefault("XDG_CONFIG_HOME", str(Path.home() / ".cache/frame-benchmark/xdg"))
    Path(env["XDG_CONFIG_HOME"]).mkdir(parents=True, exist_ok=True)
    env.update(item.split("=", 1) for item in args.env)
    godot = f"{GODOT} --path {checkout}/godot -s res://tests/frame_benchmark.gd"
    command = ["cage", "--", "sh", "-c", f"exec {godot} >> {log} 2>&1"]
    if args.agent:
        command = [AGENT_RUN, args.agent] + command
    if args.place:
        subprocess.run([args.admin, "set-position", args.character, *args.place], check=True,
                       env=dict(os.environ, GAME_SERVER_ADMIN_SOCKET=args.admin_socket))
    log.write_text(f"load: {Path('/proc/loadavg').read_text()}")
    runner = subprocess.Popen(command, env=env, stdin=subprocess.DEVNULL)
    deadline = time.time() + args.timeout
    perf = None
    seen = 0
    done_at = None
    while runner.poll() is None and time.time() < deadline:
        time.sleep(0.2)
        lines = log.read_text(errors="replace").splitlines()
        for line in lines[seen:]:
            mark = re.match(r"BENCH_MARK (\w+) (start|end)", line)
            if mark and args.perf:
                if mark[2] == "start":
                    pid = find_godot(checkout)
                    perf = subprocess.Popen(
                        ["perf", "record", "-q", "-F", "997", "-t", str(pid),
                         "-o", f"{log}.{mark[1]}.perf.data"]
                        + (["-e", "cycles:u", "--call-graph", "dwarf,32768", "--user-callchains", "-F", "199"] if args.call_graph else []),
                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                elif perf is not None:
                    perf.send_signal(signal.SIGINT)
                    perf.wait()
                    perf = None
            if line.startswith("FIXTURE ") and done_at is None:
                done_at = time.time()
        seen = len(lines)
        if done_at is not None and time.time() - done_at > 20:
            pid = find_godot(checkout)
            if pid:
                os.kill(pid, signal.SIGKILL)
                with log.open("a") as out:
                    out.write(f"killed hung Godot {pid}\n")
            break
    if runner.poll() is None:
        pid = find_godot(checkout)
        if pid:
            os.kill(pid, signal.SIGKILL)
    runner.wait()
    with log.open("a") as out:
        out.write(f"load: {Path('/proc/loadavg').read_text()}")
    for line in log.read_text(errors="replace").splitlines():
        if line.startswith(("BENCH_", "FIXTURE", "load:", "killed")):
            print(line[:600])


if __name__ == "__main__":
    main()
