"""Measure private native players; never tune gameplay or infer unobserved effects."""
import argparse
import json
import re
import statistics
from collections import Counter
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
    findings = missing_casts(run["attempts"], successes)
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
    for event in window:
        if event["kind"] == "death":
            findings.append({"kind": "death", "actor": event["dst"], "at_ms": event["at_ms"]})
    for error in run.get("server_errors", []):
        findings.append({"kind": "server_error", "error": error})
    report = {
        "schema": 1, "scenario": run["scenario"], "spec": run["spec"],
        "actor": run["actor"], "target": run["target"], "duration_ms": duration,
        "damage": sum(by_spell.values()), "dps": sum(by_spell.values()) / (duration / 1000),
        "casts": dict(used), "damage_by_spell": dict(by_spell), "findings": findings,
        "unobserved": ["aura_effect", "cooldown_resets", "charges", "resource_spending"],
        "comparison": run.get("comparison"),
        "measurement_complete": run.get("measurement_complete", False),
    }
    return report


def balance(rows):
    """Compare only explicitly identical fixture groups; no unapproved tolerance."""
    if not rows or not rows[0].get("comparison"):
        raise ValueError("Balance requires comparable level, stats, dummy and duration receipts")
    if any(row.get("comparison") != rows[0]["comparison"] for row in rows):
        raise ValueError("Non-comparable fixture groups cannot be pooled")
    median = statistics.median(row["dps"] for row in rows)
    return {
        "median_dps": median,
        "max_over_median": max(row["dps"] for row in rows) / median if median else None,
        "min_over_median": min(row["dps"] for row in rows) / median if median else None,
        "specs": [{"spec": row["spec"], "dps": row["dps"],
                   "over_median": row["dps"] / median if median else None} for row in rows],
        "tolerance": None, "comparison": rows[0]["comparison"],
    }


def write_report(directory, report):
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    summary = [f"# Spec {report['spec']} — {report['scenario']}",
               f"{report['dps']:.2f} DPS; {report['damage']} damage / {report['duration_ms'] / 1000:g}s.",
               f"Measurement complete: {report['measurement_complete']}. No balance pass/fail tolerance set.",
               "## Findings"]
    summary.extend(f"- {json.dumps(finding)}" for finding in report["findings"])
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
