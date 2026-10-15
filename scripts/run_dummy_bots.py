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

from bot_harness import analyze, merge_polls, write_report
from bot_rotation import choose, load_spells


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
    return state


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
    candidates = load_spells(args.db2, spells["known"])
    if not candidates:
        raise RuntimeError("No sourced direct-target damage abilities for this observed kit")
    (output / "native-before.json").write_text(json.dumps(state, indent=2))
    start = int(time.time() * 1000)
    run = {
        "scenario": "dummy", "spec": spells["spec"],
        "actor": str(state["account"]["local_player_id"]), "target": str(args.target),
        "start_ms": start, "end_ms": start + int(args.duration * 1000),
        "attempts": [], "spells": candidates, "measurement_complete": False,
        "rotation_rule": "generic highest available DB2 SP+AP direct damage coefficient; not a sourced Retail priority",
        "rotation_exclusions": "periodic/trigger-only, weapon, pet, AoE, utility and passive abilities",
        "known_not_in_rotation": sorted(set(spells["known"]) - {s["id"] for s in candidates}),
    }
    if args.comparison:
        run["comparison"] = json.loads(args.comparison.read_text())
    offset = args.server_log.stat().st_size if args.server_log else 0
    polls = []
    unavailable = {}
    next_cast = 0
    last_sequence = spells["combat_log_seq"]
    observed_errors = list(spells["errors"])
    capture_complete = True
    failure = None
    sequence = 0
    try:
        while int(time.time() * 1000) < run["end_ms"]:
            state = read_state(args.state)
            current = state["spells"]
            sequence += 1
            poll = invoke(args.cli, args.socket, output, sequence, "combat", "log", "--lines", "1024")
            polls.append(poll)
            if current["combat_log_seq"] - last_sequence > 1024:
                capture_complete = False
            last_sequence = current["combat_log_seq"]
            errors = current["errors"]
            if errors != observed_errors:
                run.setdefault("native_errors", []).append({"at_ms": state["at_ms"], "errors": errors})
                observed_errors = list(errors)
            with (output / "native-samples.jsonl").open("a") as samples:
                samples.write(json.dumps(state) + "\n")
            now = int(time.time() * 1000)
            ready = now >= next_cast and not current["casting"] and current["gcd_ms"] == 0
            selected = choose(candidates, unavailable, now) if ready else None
            if selected and now + max(selected["cast_ms"], selected["gcd_ms"]) + 1000 < run["end_ms"]:
                attempt = {"spell": selected["id"], "at_ms": now,
                           "deadline_ms": min(run["end_ms"] - 1, now + max(selected["cast_ms"], 1500) + 2000)}
                run["attempts"].append(attempt)
                sequence += 1
                response = invoke(args.cli, args.socket, output, sequence, "spell", "cast", "--spell", str(selected["id"]), "--target", str(args.target))
                attempt["response"] = response
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
