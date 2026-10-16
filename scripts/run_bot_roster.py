"""Provision/run owned native dummy bots against an already-running private :5871.

The caller owns server lifetime. This driver owns at most one Cage/Godot client,
uses an isolated XDG directory per bot, and preserves each private login token.
"""
import argparse
import hashlib
import json
import math
import os
import re
import shutil
import signal
import subprocess
import time
from pathlib import Path
from types import SimpleNamespace

from bot_roster import build_roster
from bot_harness import balance
from bot_health import qualify_report_health, require_living_player
from bot_forms import form_is_observed, load_plan as load_form_plan
from bot_gear import equipment_receipt, load_plan as load_gear_plan, required_grants
from bot_rotation import authored_order
from bot_talents import load_plan
from run_dummy_bots import collect, read_state


def command(output, argv, env=None):
    result = subprocess.run([str(value) for value in argv], env=env, capture_output=True, text=True, timeout=60)
    with (output / "driver.log").open("a") as log:
        log.write(f"{time.time()} {argv[0]} {argv[1]}\n" + result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(result.stderr or result.stdout)
    return result.stdout


def record_inputs(args):
    files = [args.engine / "scripts" / f"{name}.py" for name in
             ("bot_harness", "bot_rotation", "bot_roster", "bot_talents", "bot_gear", "bot_health", "bot_forms", "run_dummy_bots", "run_bot_roster")]
    files += [args.engine / "godot/tests" / f"{name}.gd" for name in ("bot_harness_observer", "bot_harness_talents", "bot_harness_gear")]
    files.append(args.world_db)
    tables = ("ChrSpecialization", "ChrRaces", "ChrClasses", "CharBaseInfo", "SpellEffect", "SpellMisc",
              "SpellPower", "SpellCooldowns", "SpellCastTimes", "AssistedCombat", "AssistedCombatStep",
              "AssistedCombatRule", "TraitTreeLoadout", "TraitTreeLoadoutEntry", "TraitNodeXTraitNodeEntry",
              "TraitNodeEntry", "TraitDefinition", "SpellShapeshiftForm")
    files += [args.engine / "data/db2/active" / f"{table}.csv" for table in tables]
    files += [args.engine / "target/debug/libgame_engine_godot.so",
              args.engine / "target/debug/game-engine-cli", args.server / "target/debug/game-server"]
    hashes = {}
    for path in files:
        with path.open("rb") as source:
            hashes[str(path)] = hashlib.file_digest(source, "sha256").hexdigest()
    revision = command(args.output, ["git", "-C", args.engine, "rev-parse", "HEAD"]).strip()
    receipt = {"at_ms": int(time.time() * 1000), "driver_source_revision": revision, "sha256": hashes,
               "db2_directory": str((args.engine / "data/db2/active").resolve()),
               "binary_source": "Binary hashes identify running artifacts; driver revision does not establish their Rust build revision."}
    (args.output / "inputs.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


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


def approach_step(here, target):
    dx, dz = target[0] - here[0], target[2] - here[2]
    distance = math.hypot(dx, dz)
    if distance <= 2.5:
        return None
    return {"seconds": min(.4, (distance - 2.5) / 7),
            "yaw_degrees": math.degrees(math.atan2(dx, dz))}


def approach_dummy(args, output, socket, target):
    observations = []
    for tick in range(100):
        state = read_state(output / "native-state.json")
        if state["target"].get("target") != target:
            raise RuntimeError("Native target changed during physical approach")
        here = [float(value) for value in state["account"]["local_player_position"].strip("()").split(",")]
        goal = state["target_position"]
        if len(goal) != 3:
            raise RuntimeError("Native dummy position not observed")
        step = approach_step(here, goal)
        observations.append({"position": here, "target": goal, "step": step})
        (output / "approach.json").write_text(json.dumps(observations, indent=2) + "\n")
        if step is None:
            return
        command(args.output, [args.engine / "target/debug/game-engine-cli", "--socket", socket,
                              "movement", "forward", "--seconds", str(step["seconds"]),
                              "--yaw-degrees", str(step["yaw_degrees"])])
        time.sleep(step["seconds"] + .2)
    raise RuntimeError("Native physical approach did not reach2.5yd after100 observations")


def verified_dummy_target(state, npc_info):
    target = state["target"]["target"]
    if f"entity {target})" not in npc_info:
        raise ValueError("Selected target is not verified Training Dummy entry44548")
    return target


def dummy_fixture(npc_info, target):
    pattern = r"Training Dummy \(entity (\d+)\) level (\d+) health [\d.]+/([\d.]+) difficulty (\d+) combat=(\{.*\})"
    for line in npc_info.splitlines():
        match = re.fullmatch(pattern, line)
        if match and int(match[1]) == target:
            combat = json.loads(match[5])
            if "creature_armor" not in combat:
                raise ValueError("Native dummy combat metadata omits creature_armor")
            return {"entry": 44548, "level": int(match[2]),
                    "max_health": float(match[3]), "difficulty": int(match[4]),
                    "creature_armor": combat["creature_armor"]}
    raise ValueError(f"Live dummy fixture for entity {target} was not observed")


def read_roster_reports(output, specs, repeat):
    rows, missing = [], []
    for spec in specs:
        for index in range(1, repeat + 1):
            relative = Path(str(spec)) / f"run-{index:03d}/report.json"
            path = output / relative
            if not path.exists():
                missing.append(str(relative))
                continue
            row = qualify_report_health(path.parent, json.loads(path.read_text()))
            row["source_report"] = str(relative)
            rows.append(row)
            if not row.get("measurement_complete"):
                missing.append(str(relative))
    return rows, missing


def compare_roster_reports(rows, specs, missing):
    if len(specs) < 3:
        return {"available": False, "reason": "Three complete DPS specs are required for the MVP comparison"}
    if missing:
        return {"available": False, "reason": "Expected DPS runs are missing or incomplete"}
    try:
        return {"available": True, **balance(rows)}
    except ValueError as error:
        return {"available": False, "reason": str(error)}


def write_roster_comparison(output, bots, repeat):
    specs = [bot["spec"] for bot in bots if bot["role"] == 2]
    rows, missing = read_roster_reports(output, specs, repeat)
    result = compare_roster_reports(rows, specs, missing)
    health_failures = [{"report": row["source_report"], "error": row["health_qualification"]["error"]}
                       for row in rows if not row["health_qualification"]["valid"]]
    result.update(expected_specs=specs, missing_runs=missing, health_failures=health_failures)
    (output / "balance.json").write_text(json.dumps(result, indent=2) + "\n")
    if result["available"]:
        summary = (f"Median {result['median_dps']:.2f} DPS; "
                   f"max/median {result['max_over_median']:.3f}; min/median {result['min_over_median']:.3f}. "
                   "No tolerance or balance pass/fail.")
    else:
        summary = "Comparison unavailable: " + result["reason"]
    (output / "comparison.md").write_text("# Dummy comparison\n\n" + summary + "\n\nFull receipts: balance.json and each spec's fixture-receipt.json.\n")


def native_ready(state, spec, requires_authored):
    player_ready = state["account"].get("gameplay_input_allowed") and state["spells"]["spec"] == spec
    evaluator_ready = not requires_authored or (state["assisted"].get("available", False) and state["spells"].get("catalog_ready", False))
    target = state["target"]
    talent_ready = state.get("talent_status") == "observed"
    gear_ready = state.get("gear_status") == "observed"
    health = state["account"].get("local_player_health")
    alive = health is not None and health > 0
    target_ready = target.get("target_name") == "Training Dummy"
    return all((player_ready, alive, evaluator_ready, talent_ready, gear_ready, target_ready))


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
            for phase in ("gear", "talent"):
                status = state.get(phase + "_status")
                if status not in ("pending", "applying", "observed"):
                    receipt = output / ("native-gear.json" if phase == "gear" else "native-talents.json")
                    detail = json.loads(receipt.read_text()).get("error") if receipt.exists() else str(status)
                    raise RuntimeError(f"Native {phase} setup blocked: {detail}; receipt={receipt}")
            setup_observed = all(state.get(phase + "_status") == "observed" for phase in ("talent", "gear"))
            if setup_observed and state["account"].get("gameplay_input_allowed"):
                require_living_player(state["account"])
            if native_ready(state, bot["spec"], requires_authored):
                return state
            if state.get("target_attempts", 0) >= 32:
                raise RuntimeError("Native Tab cycling found no Training Dummy after 32 targets")
        time.sleep(.25)
    raise RuntimeError("Native readiness/Training Dummy selection exceeded 180 seconds")


def prepare_native_form(args, output, socket, plan):
    receipt = {"plan": plan, "cast_requested": False}
    if not plan["required"]:
        (output / "native-form.json").write_text(json.dumps(receipt, indent=2) + "\n")
        return receipt
    state = read_state(output / "native-state.json")
    (output / "form-before.json").write_text(json.dumps(state) + "\n")
    if plan["spell"] not in state["spells"]["known"]:
        raise RuntimeError(f"Sourced fixture form spell {plan['spell']} is not natively known")
    if not form_is_observed(state, plan):
        actor = state["account"]["local_player_id"]
        command(args.output, [args.engine / "target/debug/game-engine-cli", "--socket", socket,
                              "spell", "cast", "--spell", str(plan["spell"]), "--target", str(actor)])
        receipt["cast_requested"] = True
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        state = read_state(output / "native-state.json")
        if form_is_observed(state, plan):
            receipt.update(observed_at_ms=state["at_ms"], observed_aura=plan["spell"],
                           proof="ordinary native cast and replicated own form aura; combat effects remain measured separately")
            (output / "form-after.json").write_text(json.dumps(state) + "\n")
            (output / "native-form.json").write_text(json.dumps(receipt, indent=2) + "\n")
            return receipt
        time.sleep(.1)
    raise RuntimeError(f"Native source form spell {plan['spell']} aura not confirmed; no repeat/toggle or alternate form")


def grant_missing_native_gear(args, output, bot, plan, state):
    (output / "inventory-before-grants.json").write_text(json.dumps(state, indent=2) + "\n")
    equipment = admin(args, "list-equipment", bot["name"])
    (output / "equipment-before-grants.txt").write_text(equipment)
    grants = required_grants(plan, equipment, state["inventory"]["bags"])
    for item, count in grants.items():
        admin(args, "grant-item", bot["name"], str(item), str(count))
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        state = read_state(output / "native-state.json")
        if not required_grants(plan, equipment, state["inventory"]["bags"]):
            (output / "inventory-after-grants.json").write_text(json.dumps(state, indent=2) + "\n")
            receipt = {"grants": grants, "proof": "authenticated owned bags confirmed before ordinary gear input"}
            (output / "gear-provisioned.json").write_text(json.dumps(receipt, indent=2) + "\n")
            return
        time.sleep(.1)
    raise RuntimeError("Granted source items not confirmed in native owned bags within10s")


def provision_native_gear(args, output, bot, gear_plan):
    deadline = time.monotonic() + 180
    while time.monotonic() < deadline:
        path = output / "native-state.json"
        if path.exists():
            state = read_state(path)
            account, inventory = state["account"], state["inventory"]
            native_owner = account.get("selected_character_name") == bot["name"]
            inworld = account.get("screen") == "InWorld" and account.get("gameplay_input_allowed")
            if native_owner and inworld and inventory["equipment"] and inventory["item_catalog_loaded"]:
                require_living_player(account)
                grant_missing_native_gear(args, output, bot, gear_plan, state)
                return
        time.sleep(.25)
    raise RuntimeError("Authenticated owned inventory not observed before gear provisioning within180s")


def run_bot(args, bot):
    output = args.output / str(bot["spec"])
    output.mkdir(mode=0o700)
    (output / "bot.json").write_text(json.dumps(bot, indent=2) + "\n")
    talent_plan = load_plan(args.engine / "data/db2/active", bot["spec"])
    (output / "talent-plan.json").write_text(json.dumps(talent_plan, indent=2) + "\n")
    gear_plan = load_gear_plan(args.world_db, args.engine / "data/db2/active", bot["spec"], bot["level"])
    (output / "gear-plan.json").write_text(json.dumps(gear_plan, indent=2) + "\n")
    form_plan = load_form_plan(args.engine / "data/db2/active", bot["spec"])
    (output / "form-plan.json").write_text(json.dumps(form_plan, indent=2) + "\n")
    provision(args, bot)
    equipment = admin(args, "list-equipment", bot["name"])
    (output / "equipment-before.txt").write_text(equipment)
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
        "BOT_HARNESS_GEAR": str(output / "gear-plan.json"),
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
        provision_native_gear(args, output, bot, gear_plan)
        state = wait_native(args, output, bot)
        talent_receipt = json.loads((output / "native-talents.json").read_text())
        fixture = equipment_receipt(gear_plan, state["inventory"], talent_receipt["sheet_stats"])
        (output / "fixture-receipt.json").write_text(json.dumps(fixture, indent=2) + "\n")
        npcs = admin(args, "npc-info", "0", "0", "44548")
        (output / "dummy-identity.txt").write_text(npcs)
        target = verified_dummy_target(state, npcs)
        children = process_children(pid)
        sockets = [Path(f"/tmp/game-engine-{child}.sock") for child in children]
        sockets = [path for path in sockets if path.exists()]
        if len(sockets) != 1:
            raise RuntimeError(f"Expected one owned native IPC socket, found {sockets}")
        fixture["precombat_form"] = prepare_native_form(args, output, sockets[0], form_plan)
        fixture["native_stats_phase"] = "after native talents; before precombat form"
        (output / "fixture-receipt.json").write_text(json.dumps(fixture, indent=2) + "\n")
        approach_dummy(args, output, sockets[0], target)
        options = SimpleNamespace(cli=args.engine / "target/debug/game-engine-cli", socket=sockets[0],
                                  state=output / "native-state.json", db2=args.engine / "data/db2/active",
                                  target=target, duration=args.duration, comparison=None, server_log=args.server_log)
        stats = command(args.output, [options.cli, "--socket", sockets[0], "status", "character-stats"])
        (output / "character-stats.txt").write_text(stats)
        native_level = re.search(r"(?m)^level: (\d+)$", stats)
        if native_level is None or int(native_level[1]) != bot["level"]:
            raise ValueError("Native character level does not match the fixed fixture")
        dummy = dummy_fixture(npcs, target)
        dummy["world_sha256"] = args.input_receipt["sha256"][str(args.world_db)]
        comparison = {"level": int(native_level[1]), "stats": fixture["budget"],
                      "dummy": dummy, "duration_ms": int(args.duration * 1000)}
        options.comparison = output / "comparison.json"
        options.comparison.write_text(json.dumps(comparison, indent=2) + "\n")
        for index in range(args.repeat):
            collect(options, output / f"run-{index + 1:03d}")
        (output / "equipment-after.txt").write_text(admin(args, "list-equipment", bot["name"]))
    finally:
        if pid is not None:
            stop_client(pid, output)
            time.sleep(2)
            shutil.copy2(args.engine / "target/headless-client.log", output / "client.log")
            if token.exists():
                token.rename(output / token.name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("engine", "server", "output", "capacity", "server-log", "godot", "world-db"):
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
    args.input_receipt = record_inputs(args)
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
    write_roster_comparison(args.output, bots, args.repeat)
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
