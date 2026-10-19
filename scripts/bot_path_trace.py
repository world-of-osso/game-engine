"""Opt-in native Stockade path trace, independent of harness decisions."""
import json
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
    rows.sort(key=lambda row: row["at_ms"])
    start = {"event": "run_start", "schema": "stockade_path_map_v1", "map_id": 34,
             "run_id": run["run_id"], "revision": run["revision"],
             "cadence": "client timer 100ms; not simulation ticks",
             "capture_gaps": ["simulation clock unavailable", "overlap pulls not observed",
                              "mob spawn-to-entity identity unavailable outside captured bosses"],
             **stamp(run["start_ms"])}
    end = {"event": "run_end", "failure": run.get("failure"), **stamp(run["end_ms"])}
    return [start, *rows, end]


def write_trace(output, combat, run):
    """Read the explicitly enabled client capture and publish the manual renderer input."""
    path = Path(output) / "bot-path-samples.jsonl"
    samples = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    rows = compile_trace(samples, combat, run)
    destination = Path(output) / "bot-path-trace.jsonl"
    destination.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows))
    return destination
