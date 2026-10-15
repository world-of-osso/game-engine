"""Measure private native players; never tune gameplay or infer unobserved effects."""
import argparse
import json
import re
import statistics
from collections import Counter, defaultdict
from pathlib import Path

EVENT = re.compile(
    r"^(\w+) src=(\d+|-) dst=(\d+|-) spell=(\d+|-) amount=(-?\d+|-) "
    r".* timestamp_unix_ms=(\d+)\b"
)


def parse_combat(text):
    """Read the maintained native `combat log` text contract, preserving duplicates."""
    events = []
    for line in text.splitlines():
        match = EVENT.match(line)
        if not match:
            if "timestamp_unix_ms=" in line:
                raise ValueError(f"Unrecognized native combat event: {line}")
            continue
        kind, source, target, spell, amount, stamp = match.groups()
        events.append({
            "kind": kind, "src": source, "dst": target,
            "spell": None if spell == "-" else int(spell),
            "amount": 0 if amount == "-" else int(amount), "at_ms": int(stamp),
            "text": line,
        })
    return sorted(events, key=lambda event: event["at_ms"])


def poll_has_gap(previous_sequence, sequence, retained):
    return sequence - previous_sequence > retained


def merge_polls(polls):
    """Union overlapping ring-buffer snapshots while retaining true multiplicity."""
    counts = Counter()
    for poll in polls:
        counts |= Counter(event["text"] for event in parse_combat(poll))
    return parse_combat("\n".join(counts.elements()))


def missing_casts(attempts, successes):
    """Consume one server success per submitted request, not per spell identity."""
    unmatched = list(successes)
    findings = []
    for attempt in attempts:
        if attempt.get("error"):
            findings.append({"kind": "cast_error", "spell": attempt["spell"], "error": attempt["error"]})
            continue
        match = next((event for event in unmatched if event["spell"] == attempt["spell"]
                      and attempt["at_ms"] <= event["at_ms"] <= attempt["deadline_ms"]), None)
        if match is None:
            findings.append({"kind": "cast_never_fired", "spell": attempt["spell"], "at_ms": attempt["at_ms"]})
        else:
            unmatched.remove(match)
    return findings


def analyze(run, events):
    """Produce findings only where the evidence can demonstrate the observation."""
    duration = run["end_ms"] - run["start_ms"]
    if duration <= 0:
        raise ValueError("Run must have a positive fixed measurement duration")
    window = [e for e in events if run["start_ms"] <= e["at_ms"] < run["end_ms"]]
    own = [e for e in window if e["src"] == str(run["actor"])]
    successes = [e for e in own if e["kind"] == "cast_success"]
    observed_until = run.get("observed_until_ms", run["end_ms"])
    pending = [attempt for attempt in run["attempts"] if attempt["deadline_ms"] > observed_until and not attempt.get("error")]
    evaluated = [attempt for attempt in run["attempts"] if attempt not in pending]
    findings = missing_casts(evaluated, successes)
    used = Counter(e["spell"] for e in successes)
    damage = [e for e in own if e["kind"] == "damage" and e["dst"] == str(run["target"])]
    by_spell = Counter()
    for event in damage:
        by_spell[event["spell"]] += max(0, event["amount"])
    for spell in run["spells"]:
        identity = spell["id"]
        if not used[identity]:
            findings.append({"kind": "ability_never_used", "spell": identity})
        elif spell["damage"] and not sum(by_spell[value] for value in spell.get("damage_ids", [identity])):
            findings.append({"kind": "zero_damage_spell", "spell": identity})
    party = {str(actor) for actor in run.get("party_actors", [])}
    party.add(str(run["actor"]))
    for event in window:
        if event["kind"] == "death" and event["dst"] in party:
            findings.append({"kind": "death", "actor": event["dst"], "at_ms": event["at_ms"]})
    for error in run.get("server_errors", []):
        findings.append({"kind": "server_error", "error": error})
    for sample in run.get("native_errors", []):
        for error in sample["errors"]:
            findings.append({"kind": "native_cast_error", "at_ms": sample["at_ms"], "error": error, "spell": None})
    report = {
        "schema": 1, "scenario": run["scenario"], "spec": run["spec"], "role": run.get("role"),
        "actor": run["actor"], "target": run["target"], "duration_ms": duration,
        "damage": sum(by_spell.values()),
        "dps": sum(by_spell.values()) / (duration / 1000) if run.get("measurement_complete") else None,
        "casts": dict(used), "damage_by_spell": dict(by_spell), "findings": findings,
        "pending_casts": pending,
        "unobserved": ["aura_effect", "cooldown_resets", "charges", "resource_spending"],
        "comparison": run.get("comparison"),
        "measurement_complete": run.get("measurement_complete", False),
    }
    return report


def balance(rows):
    """Compare only explicitly identical fixture groups; no unapproved tolerance."""
    if any(row.get("role") not in (0, 1, 2) for row in rows):
        raise ValueError("Balance requires authored specialization roles")
    rows = [row for row in rows if row["role"] == 2]
    if not rows or not all(row.get("measurement_complete") for row in rows):
        raise ValueError("Balance requires complete fixed-duration DPS measurements")
    comparison = rows[0].get("comparison") or {}
    missing = [field for field in ("level", "stats", "dummy", "duration_ms") if not comparison.get(field)]
    if missing:
        raise ValueError("Balance comparison missing " + ", ".join(missing))
    if any(row.get("comparison") != rows[0]["comparison"] for row in rows):
        raise ValueError("Non-comparable fixture groups cannot be pooled")
    by_spec = defaultdict(list)
    for row in rows:
        by_spec[row["spec"]].append(row["dps"])
    specs = [{"spec": spec, "dps": statistics.mean(values), "runs": len(values)}
             for spec, values in sorted(by_spec.items())]
    median = statistics.median(row["dps"] for row in specs)
    for row in specs:
        row["over_median"] = row["dps"] / median if median else None
    return {
        "median_dps": median,
        "max_over_median": max(row["dps"] for row in specs) / median if median else None,
        "min_over_median": min(row["dps"] for row in specs) / median if median else None,
        "specs": specs, "tolerance": None, "comparison": rows[0]["comparison"],
    }


def write_report(directory, report):
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    measurement = f"{report['dps']:.2f} DPS" if report["dps"] is not None else "DPS unavailable: incomplete measurement"
    summary = [f"# Spec {report['spec']} — {report['scenario']}",
               f"{measurement}; {report['damage']} damage; planned {report['duration_ms'] / 1000:g}s.",
               f"Measurement complete: {report['measurement_complete']}. No balance pass/fail tolerance set.",
               "## Findings"]
    if report.get("collection_error"):
        summary.append("Collection blocked: " + report["collection_error"])
    summary.append(json.dumps(dict(Counter(finding["kind"] for finding in report["findings"]))))
    summary.append("Full findings and spell coverage: report.json.")
    summary.extend(["## Not observable", ", ".join(report["unobserved"])])
    (directory / "summary.md").write_text("\n\n".join(summary) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path, help="Run manifest including submitted casts and observation window")
    parser.add_argument("combat", type=Path, help="Native IPC combat log captured during that window")
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    write_report(args.output, analyze(json.loads(args.run.read_text()), parse_combat(args.combat.read_text())))


if __name__ == "__main__":
    main()
