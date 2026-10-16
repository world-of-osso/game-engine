"""Collect a fixed-duration dummy run from an already-owned native player.

Launch bot_harness_observer.gd via scripts/agent/headless-client.sh first.
All subprocesses inherit the caller's agent-run scope. Never auto-discover sockets.
"""
import argparse
import json
import re
import subprocess
import time
from pathlib import Path

from bot_harness import analyze, merge_polls, poll_has_gap, write_report
from bot_health import require_living_player
from bot_controller import PlayerController, requested_snapshot
from bot_rotation import authored_next, authored_order, choose, load_spells, read_rows


def invoke(cli, socket, output, sequence, *args):
    result = subprocess.run([str(cli), "--socket", str(socket), *args], capture_output=True, text=True, timeout=20)
    (output / f"ipc-{sequence:05d}.log").write_text(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(f"Native IPC {args}: {result.stderr or result.stdout}")
    return result.stdout


def read_state(path):
    state = json.loads(path.read_text())
    if int(time.time() * 1000) - state["at_ms"] > 3000:
        raise RuntimeError("Native observation stale for more than 3 seconds")
    if state.get("spec_request_error"):
        raise RuntimeError(state["spec_request_error"])
    if not state["account"].get("gameplay_input_allowed"):
        raise RuntimeError("Native player disconnected, dead or not in world")
    require_living_player(state["account"])
    return state


def observed_pet_owner(state):
    """local_pet is backed by replicated UnitIsPet + UnitSummonedBy, not bar state."""
    pet = state["pet"]["local_pet"]
    if pet is None:
        return None
    return {"unit": str(pet), "owner": str(state["account"]["local_player_id"]),
            "at_ms": state["at_ms"]}


def server_errors(path, offset):
    if path is None:
        return []
    with path.open() as source:
        source.seek(offset)
        return [line.strip() for line in source if re.search(r"\bERROR\b|panicked at", line)]


def collect(args, output):
    output.mkdir(parents=True, exist_ok=False)
    state = read_state(args.state)
    spells = state["spells"]
    order = authored_order(args.db2, spells["spec"])
    metadata = load_spells(args.db2, spells["known"], direct_only=False)
    candidates = [spell for spell in metadata if spell["id"] in order] if order else load_spells(args.db2, spells["known"])
    if not candidates:
        raise RuntimeError("No sourced rotation abilities for this observed kit")
    if order and state["target"].get("target") != args.target:
        raise RuntimeError("Authored evaluator target must equal the verified dummy entity")
    (output / "native-before.json").write_text(json.dumps(state, indent=2))
    role = next((int(row["Role"]) for row in read_rows(args.db2, "ChrSpecialization")
                 if int(row["ID"]) == spells["spec"]), None)
    if role is None:
        raise ValueError(f"No authored role for observed spec {spells['spec']}")
    start = int(time.time() * 1000)
    run = {
        "scenario": "dummy", "spec": spells["spec"], "role": role,
        "actor": str(state["account"]["local_player_id"]), "target": str(args.target),
        "start_ms": start, "end_ms": start + int(args.duration * 1000),
        "attempts": [], "spells": metadata if order else candidates, "measurement_complete": False,
        "rotation_rule": "native sourced AssistedCombatStep order/rules" if order else "generic highest available DB2 SP+AP direct damage coefficient; no authored order exists",
        "rotation_exclusions": "unsupported native predicates block the run" if order else "periodic/trigger-only, weapon, pet, AoE, utility and passive abilities",
        "authored_order": order,
        "owner_snapshot": [],
        "known_not_in_rotation": sorted(set(spells["known"]) - {s["id"] for s in candidates}),
    }
    if args.comparison:
        run["comparison"] = json.loads(args.comparison.read_text())
    offset = args.server_log.stat().st_size if args.server_log else 0
    polls = []
    observed_pets = set()
    unavailable = {}
    next_cast = 0
    last_sequence = spells["combat_log_seq"]
    observed_errors = list(spells["errors"])
    capture_complete = True
    failure = None
    sequence = 0
    def checked_command(*argv):
        nonlocal sequence
        # Record only actual submissions, including IPC failures, not rejected choices.
        run["attempts"].append(attempt)
        sequence += 1
        return invoke(args.cli, args.socket, output, sequence, *argv)
    controller = PlayerController(checked_command, lambda target: requested_snapshot(state, target),
                                  None, None)  # Dummy collection does not move or use party input.
    try:
        while int(time.time() * 1000) < run["end_ms"]:
            state = read_state(args.state)
            current = state["spells"]
            ownership = observed_pet_owner(state)
            if ownership and ownership["unit"] not in observed_pets:
                run["owner_snapshot"].append(ownership)
                observed_pets.add(ownership["unit"])
            sequence += 1
            poll = invoke(args.cli, args.socket, output, sequence, "combat", "log", "--lines", "1024")
            polls.append(poll)
            if poll_has_gap(last_sequence, current["combat_log_seq"], current["combat_log_count"]):
                capture_complete = False
            last_sequence = current["combat_log_seq"]
            errors = current["errors"]
            if errors != observed_errors:
                new_errors = errors[len(observed_errors):] if errors[:len(observed_errors)] == observed_errors else errors
                run.setdefault("native_errors", []).append({"at_ms": state["at_ms"], "errors": new_errors})
                observed_errors = list(errors)
            with (output / "native-samples.jsonl").open("a") as samples:
                samples.write(json.dumps(state) + "\n")
            now = int(time.time() * 1000)
            ready = now >= next_cast and not current["casting"] and current["gcd_ms"] == 0
            selected = None
            if ready and order:
                identity = authored_next(state["assisted"])
                if identity is not None:
                    selected = next((spell for spell in metadata if spell["id"] == identity), None)
                    if selected is None:
                        raise RuntimeError(f"Authored recommendation {identity} lacks known DB2 metadata")
            elif ready:
                selected = choose(candidates, unavailable, now)
            if selected and now + max(selected["cast_ms"], selected["gcd_ms"]) + 1000 < run["end_ms"]:
                attempt = {"spell": selected["id"], "at_ms": now,
                           "deadline_ms": min(run["end_ms"] - 1, now + max(selected["cast_ms"], 1500) + 2000)}
                action = controller.cast_next(args.target, lambda native, target: {"spell": selected["id"]})
                if action is None:
                    time.sleep(.2)
                    continue
                attempt["response"] = action["response"]
                next_cast = now + max(selected["cast_ms"], selected["gcd_ms"], 100)
                unavailable[selected["id"]] = now + selected["cooldown_ms"]
            time.sleep(.2)
        sequence += 1
        polls.append(invoke(args.cli, args.socket, output, sequence, "combat", "log", "--lines", "1024"))
        run["measurement_complete"] = capture_complete
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        failure = str(error)
        run["collection_error"] = failure
    finally:
        run["observed_until_ms"] = int(time.time() * 1000)
        run["server_errors"] = server_errors(args.server_log, offset)
        (output / "run.json").write_text(json.dumps(run, indent=2) + "\n")
        events = merge_polls(polls)
        (output / "combat.log").write_text("\n".join(e["text"] for e in events) + "\n")
        report = analyze(run, events)
        report["native_errors"] = run.get("native_errors", [])
        report["collection_error"] = failure
        report["rotation_rule"] = run["rotation_rule"]
        report["rotation_exclusions"] = run["rotation_exclusions"]
        write_report(output, report)
    if failure:
        raise RuntimeError(failure)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--socket", type=Path, required=True)
    parser.add_argument("--state", type=Path, required=True)
    parser.add_argument("--db2", type=Path, required=True)
    parser.add_argument("--target", type=int, required=True)
    parser.add_argument("--duration", type=float, default=60)
    parser.add_argument("--repeat", type=int, default=1)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--server-log", type=Path)
    parser.add_argument("--comparison", type=Path, help="Explicit common fixture receipt; omitted means no balance comparison")
    args = parser.parse_args()
    if args.duration <= 0 or args.repeat < 1:
        parser.error("Positive duration and repeat required")
    for index in range(args.repeat):
        print(json.dumps(collect(args, args.output / f"run-{index + 1:03d}")), flush=True)


if __name__ == "__main__":
    main()
