"""Opt-in native Stockade path trace, independent of harness decisions."""
import json
import time
from pathlib import Path


def stamp(at_ms, monotonic_ns=None, clock=None):
    return {"at_ms": at_ms, "simulation_s": None, "monotonic_ns": monotonic_ns,
            "monotonic_clock": clock}


def sample_rows(sample, names):
    clock = stamp(sample["at_ms"], sample["monotonic_ns"], "godot_process_ticks")
    return [{"event": "pose", "actor": actor, "name": names[actor], **pose, **clock}
            for actor, pose in sample["actors"].items() if actor in names]


def combat_rows(combat):
    return [{**row, "event": "death" if row["kind"] == "death" else "combat_event",
             "actor": row.get("dst"), **stamp(row["at_ms"])} for row in combat]


def compile_trace(samples, combat, run):
    """Transform actual observations, retaining unknown clocks rather than inventing ticks."""
    samples = [sample for sample in samples if sample["map_id"] == 34]
    names = run["party_names"]
    rows = []
    previous_combat = False
    for sample in samples:
        rows.extend(sample_rows(sample, names))
        engaged = sample.get("in_combat")
        if engaged is not None and engaged != previous_combat:
            rows.append({"event": "combat_start" if engaged else "combat_end",
                         "actor": run["actor"], "source": "owner CombatStatus",
                         **stamp(sample["at_ms"], sample["monotonic_ns"], "godot_process_ticks")})
            previous_combat = engaged
    rows.extend(combat_rows(combat))
    rows.extend(party_rows(run))
    rows.extend(boss_death_rows(combat, run))
    rows.sort(key=lambda row: row["at_ms"])
    start = {"event": "run_start", "schema": "stockade_path_map_v1", "map_id": 34,
             "run_id": run["run_id"], "revision": run["revision"],
             "cadence": "client timer 100ms; not simulation ticks",
             "capture_gaps": ["simulation clock unavailable", "overlap pulls not observed",
                              "mob spawn-to-entity identity unavailable outside captured bosses"],
             **stamp(run["start_ms"])}
    end = {"event": "run_end", "failure": run.get("failure"), **stamp(run["end_ms"])}
    return [start, *rows, end]


def boss_death_rows(combat, run):
    known = {str(boss["entity"]): boss["spawn_guid"] for boss in run.get("bosses", []) if "entity" in boss}
    return [{"event": "spawn_state", "spawn_guid": known[str(row["dst"])], "alive": False,
             "actor": str(row["dst"]), "source": "known boss identity and actual combat death",
             **stamp(row["at_ms"])} for row in combat
            if row["kind"] == "death" and str(row["dst"]) in known]


def party_rows(run):
    names = set(run["party_names"].values())
    rows = []
    wiped = False
    for observation in run.get("party_observations", []):
        members = observation["members"]
        clock = stamp(observation["at_ms"], observation["monotonic_ns"], "python_host_monotonic")
        rows.append({"event": "party_sample", "members": members, **clock})
        all_dead = names and {member["name"] for member in members} == names and all(
            member["health"] <= 0 and member["death"] != "Alive" for member in members)
        if all_dead and not wiped:
            rows.append({"event": "wipe", "actor": run["actor"],
                         "source": "complete native party roster simultaneously dead", **clock})
        wiped = bool(all_dead)
    return rows


def append_party(output, members):
    """Append already-read roster facts; never query or change the harness decisions."""
    row = {"at_ms": int(time.time() * 1000), "monotonic_ns": time.monotonic_ns(), "members": members}
    with (Path(output) / "bot-path-party.jsonl").open("a") as stream:
        stream.write(json.dumps(row) + "\n")


def write_trace(output, combat, run):
    """Read the explicitly enabled client capture and publish the manual renderer input."""
    path = Path(output) / "bot-path-samples.jsonl"
    samples = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    party_path = Path(output) / "bot-path-party.jsonl"
    if party_path.exists():
        party = [json.loads(line) for line in party_path.read_text().splitlines() if line.strip()]
        run = {**run, "party_observations": party}
    rows = compile_trace(samples, combat, run)
    destination = Path(output) / "bot-path-trace.jsonl"
    destination.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows))
    return destination
