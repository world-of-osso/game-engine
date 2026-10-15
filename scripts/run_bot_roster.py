"""Provision/run owned native dummy bots against an already-running private :5871.

The caller owns server lifetime. This driver owns at most one Cage/Godot client,
uses an isolated XDG directory per bot, and preserves each private login token.
"""
import argparse
import json
import os
import re
import shutil
import signal
import subprocess
import time
from pathlib import Path
from types import SimpleNamespace

from bot_roster import build_roster
from bot_rotation import authored_order
from bot_talents import load_plan
from run_dummy_bots import collect


def command(output, argv, env=None):
    result = subprocess.run([str(value) for value in argv], env=env, capture_output=True, text=True, timeout=60)
    with (output / "driver.log").open("a") as log:
        log.write(f"{time.time()} {argv[0]} {argv[1]}\n" + result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(result.stderr or result.stdout)
    return result.stdout


def wait_capacity(args, gpu=False):
    while True:
        result = command(args.output, ["sh", args.capacity])
        if "ADD:" in result:
            if not gpu:
                return
            memory = command(args.output, ["nvidia-smi", "--query-gpu=memory.used,memory.total", "--format=csv,noheader,nounits"])
            used, total = [int(value.strip()) for value in memory.splitlines()[0].split(",")]
            if total - used >= 1536:
                return
        time.sleep(60)


def admin(args, *words):
    env = dict(os.environ, GAME_SERVER_ADMIN_SOCKET="/tmp/game-server-admin-5871.sock")
    return command(args.output, [args.server / "target/debug/game-server-admin", *words], env)


def provision(args, bot):
    record = admin(args, "check-account", bot["account"])
    if "exists=false" in record:
        admin(args, "create-account", bot["account"], "fbtest")
    characters = admin(args, "list-characters", bot["account"])
    if f"name={bot['name']} " not in characters:
        admin(args, "create-character", bot["account"], bot["name"], str(bot["race"]), str(bot["class"]))
    owner = admin(args, "find-character", bot["name"])
    if not owner.startswith(f"username={bot['account']} "):
        raise RuntimeError("Character ownership differs from the private roster")
    admin(args, "set-level", bot["name"], str(bot["level"]))
    # Existing regression-tested Retail terrain destination; no NPC legacy Z.
    env = dict(os.environ, GAME_SERVER_ADMIN_SOCKET="/tmp/game-server-admin-5871.sock")
    command(args.output, ["python3", args.server / "scripts/place-private-character.py", "cooldown-assist", bot["account"], bot["name"], "--admin", args.server / "target/debug/game-server-admin"], env)


def process_children(pid):
    try:
        children = Path(f"/proc/{pid}/task/{pid}/children").read_text().split()
    except FileNotFoundError:
        return []
    result = []
    for child in children:
        result.extend(process_children(int(child)))
        result.append(int(child))
    return result


def stop_client(pid, output):
    identities = process_children(pid) + [pid]
    stopped = []
    for owned in identities:
        try:
            os.kill(owned, signal.SIGTERM)
            stopped.append(owned)
        except ProcessLookupError:
            pass
    time.sleep(2)
    forced = []
    for owned in stopped:
        stat = Path(f"/proc/{owned}/stat")
        if stat.exists() and stat.read_text().split(") ", 1)[1].split()[0] != "Z":
            try:
                os.kill(owned, signal.SIGKILL)
                forced.append(owned)
            except ProcessLookupError:
                pass
    (output / "stopped-pids.json").write_text(json.dumps({"terminated": stopped, "forced": forced}) + "\n")


def verified_dummy_target(state, npc_info):
    target = state["target"]["target"]
    if f"entity {target})" not in npc_info:
        raise ValueError("Selected target is not verified Training Dummy entry44548")
    return target


def native_ready(state, spec, requires_authored):
    player_ready = state["account"].get("gameplay_input_allowed") and state["spells"]["spec"] == spec
    evaluator_ready = not requires_authored or (state["assisted"].get("available", False) and state["spells"].get("catalog_ready", False))
    target = state["target"]
    talent_ready = state.get("talent_status") == "observed"
    return bool(player_ready and evaluator_ready and talent_ready and target.get("target_name") == "Training Dummy")


def wait_native(args, output, bot):
    deadline = time.monotonic() + 180
    requires_authored = bool(authored_order(args.engine / "data/db2/active", bot["spec"]))
    while time.monotonic() < deadline:
        log = (args.engine / "target/headless-client.log").read_text(errors="replace")
        if "SCRIPT ERROR" in log:
            raise RuntimeError(log[-3000:])
        path = output / "native-state.json"
        if path.exists():
            state = json.loads(path.read_text())
            if state["account"].get("selected_character_name") not in ("", bot["name"]):
                raise RuntimeError("Native logged into a different character")
            if native_ready(state, bot["spec"], requires_authored):
                return state
            if state.get("target_attempts", 0) >= 32:
                raise RuntimeError("Native Tab cycling found no Training Dummy after 32 targets")
        time.sleep(.25)
    raise RuntimeError("Native readiness/Training Dummy selection exceeded 180 seconds")


def run_bot(args, bot):
    output = args.output / str(bot["spec"])
    output.mkdir(mode=0o700)
    (output / "bot.json").write_text(json.dumps(bot, indent=2) + "\n")
    talent_plan = load_plan(args.engine / "data/db2/active", bot["spec"])
    (output / "talent-plan.json").write_text(json.dumps(talent_plan, indent=2) + "\n")
    provision(args, bot)
    token = args.engine / "data/auth_token.127.0.0.1_5871"
    if token.is_symlink():
        raise RuntimeError("Private token unexpectedly links outside the owned data root")
    if token.exists():
        raise RuntimeError("Private port has an unclaimed saved token; preserve/identify it before login")
    for name in ("config", "userdata", "cache", "runtime"):
        (output / name).mkdir(mode=0o700)
    credentials = output / "config/world-of-osso/credentials.ron"
    credentials.parent.mkdir()
    credentials.write_text(f'(username:"{bot["account"]}",password:"fbtest")\n')
    credentials.chmod(0o600)
    env = {
        "XDG_RUNTIME_DIR": str(output / "runtime"), "XDG_DATA_HOME": str(output / "userdata"),
        "XDG_CACHE_HOME": str(output / "cache"), "BOT_HARNESS_OUTPUT": str(output),
        "BOT_HARNESS_SPEC": str(bot["spec"]), "ASSET_RESOLVER_SHARED_DATA_DIR": str(args.engine / "data"),
        "BOT_HARNESS_TALENTS": str(output / "talent-plan.json"),
        "GODOT_BIN": str(args.godot), "ASSET_RESOLVER_CACHE_DIR": str(args.output / "resolver"),
        "GAME_ENGINE_ASSET_MODE": "extracted-only",
    }
    wait_capacity(args, gpu=True)
    pid = None
    try:
        argv = [args.engine / "scripts/agent/agent-run", "bot-harness", "env", "-u", "DISPLAY",
                *[f"{key}={value}" for key, value in env.items()],
                args.engine / "scripts/agent/headless-client.sh", "start-godot", args.engine,
                output / "config", "--audio-driver", "Dummy", "--resolution", "1280x720",
                "--script", "res://tests/bot_harness_observer.gd", "--", "--server", "127.0.0.1:5871",
                "--screen", "inworld", "--char", bot["name"]]
        reply = command(args.output, argv)
        pid = int(re.search(r"cage pid (\d+)", reply)[1])
        state = wait_native(args, output, bot)
        npcs = admin(args, "npc-info", "0", "0", "44548")
        (output / "dummy-identity.txt").write_text(npcs)
        target = verified_dummy_target(state, npcs)
        children = process_children(pid)
        sockets = [Path(f"/tmp/game-engine-{child}.sock") for child in children]
        sockets = [path for path in sockets if path.exists()]
        if len(sockets) != 1:
            raise RuntimeError(f"Expected one owned native IPC socket, found {sockets}")
        options = SimpleNamespace(cli=args.engine / "target/debug/game-engine-cli", socket=sockets[0],
                                  state=output / "native-state.json", db2=args.engine / "data/db2/active",
                                  target=target, duration=args.duration, comparison=None, server_log=args.server_log)
        stats = command(args.output, [options.cli, "--socket", sockets[0], "status", "character-stats"])
        (output / "character-stats.txt").write_text(stats)
        for index in range(args.repeat):
            collect(options, output / f"run-{index + 1:03d}")
    finally:
        if pid is not None:
            stop_client(pid, output)
            time.sleep(2)
            shutil.copy2(args.engine / "target/headless-client.log", output / "client.log")
            if token.exists():
                token.rename(output / token.name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("engine", "server", "output", "capacity", "server-log", "godot"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--specs", type=int, nargs="+", default=[62, 63, 64])
    parser.add_argument("--all-specs", action="store_true")
    parser.add_argument("--level", type=int, default=32)
    parser.add_argument("--duration", type=float, default=60)
    parser.add_argument("--repeat", type=int, default=2)
    args = parser.parse_args()
    if args.duration <= 0 or args.repeat < 1:
        parser.error("Positive duration and repeat required")
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    admin(args, "ping")
    bots = build_roster(args.engine / "data/db2/active", args.level)
    bots = [bot for bot in bots if args.all_specs or bot["spec"] in args.specs]
    (args.output / "roster.json").write_text(json.dumps(bots, indent=2) + "\n")
    failures = []
    for bot in bots:
        try:
            run_bot(args, bot)
        except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
            failure = {"spec": bot["spec"], "error": str(error), "measurement_complete": False}
            failures.append(failure)
            directory = args.output / str(bot["spec"])
            (directory / "setup-error.json").write_text(json.dumps(failure, indent=2) + "\n")
            (directory / "summary.md").write_text(f"# Spec {bot['spec']} — setup blocked\n\n{error}\n\nNo complete DPS measurement or balance claim.\n")
    (args.output / "failures.json").write_text(json.dumps(failures, indent=2) + "\n")
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
