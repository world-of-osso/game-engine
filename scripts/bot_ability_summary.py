"""Read-only per-ability triage from retained native run/report artifacts."""
import argparse
import json
from collections import Counter
from pathlib import Path

from bot_health import qualify_report_health


def summarize_run(manifest, report):
    requests = Counter(attempt["spell"] for attempt in manifest["attempts"])
    successes = {int(spell): count for spell, count in report["casts"].items()}
    damage = {int(spell): amount for spell, amount in report["damage_by_spell"].items()}
    metadata = {spell["id"]: spell for spell in manifest["spells"]}
    findings = Counter((finding["kind"], finding.get("spell")) for finding in report["findings"])
    identities = set(metadata) | set(requests) | set(successes) | set(damage)
    abilities = [{"spell": spell, "requests": requests[spell], "successes": successes.get(spell, 0),
                  "event_damage": damage.get(spell, 0),
                  "damage_ids": metadata.get(spell, {}).get("damage_ids", []),
                  "missing_casts": findings["cast_never_fired", spell],
                  "zero_damage_findings": findings["zero_damage_spell", spell],
                  "unused": bool(findings["ability_never_used", spell])}
                 for spell in sorted(identities)]
    errors = Counter(error for sample in report["native_errors"] for error in sample["errors"])
    return {"spec": manifest["spec"], "role": manifest["role"],
            "duration_ms": report["duration_ms"], "measurement_complete": report["measurement_complete"],
            "damage": report["damage"], "dps": report["dps"], "abilities": abilities,
            "unattributed_native_errors": dict(errors)}


def read_spec_runs(root, spec, repeats):
    runs, missing, partial = [], [], []
    for index in range(1, repeats + 1):
        name = f"run-{index:03d}"
        directory = root / str(spec) / name
        manifest_path, report_path = directory / "run.json", directory / "report.json"
        if not manifest_path.exists() or not report_path.exists():
            missing.append(name)
            if manifest_path.exists() or report_path.exists():
                partial.append(name)
            continue
        report = qualify_report_health(directory, json.loads(report_path.read_text()))
        result = summarize_run(json.loads(manifest_path.read_text()), report)
        result["health_qualification"] = report["health_qualification"]
        if "reported_dps" in report:
            result["reported_dps"] = report["reported_dps"]
            result["reported_measurement_complete"] = report["reported_measurement_complete"]
        result["run"] = name
        runs.append(result)
    return runs, missing, partial


def summarize_roster(root, roster, repeats):
    specs = []
    for bot in roster:
        spec = bot["spec"]
        runs, missing, partial = read_spec_runs(root, spec, repeats)
        failure = root / str(spec) / "setup-error.json"
        specs.append({"spec": spec, "planned_runs": repeats, "runs": runs,
                      "missing_runs": missing, "partial_artifacts": partial,
                      "setup_error": json.loads(failure.read_text()) if failure.exists() else None})
    return {"schema": 1, "source": str(root.resolve()), "specs": specs,
            "damage_policy": "event_damage retains actual combat spell IDs; damage_ids are source relationships, not duplicate root credit",
            "error_policy": "native errors lack spell IDs and remain unattributed; missing_casts are not added to them as a new failure count"}


def render_markdown(summary):
    lines = ["# Per-ability native diagnostics", "", summary["damage_policy"], "", summary["error_policy"]]
    for spec in summary["specs"]:
        lines.extend(["", f"## Spec {spec['spec']}"])
        if spec["setup_error"]:
            lines.append("Setup/collection blocked: " + spec["setup_error"]["error"])
        if spec["missing_runs"]:
            lines.append("Missing runs: " + ", ".join(spec["missing_runs"]))
        if spec["partial_artifacts"]:
            lines.append("Incomplete artifact pairs: " + ", ".join(spec["partial_artifacts"]))
        for run in spec["runs"]:
            if not run["health_qualification"]["valid"]:
                lines.extend(["", "Health qualification failed: " + run["health_qualification"]["error"]])
            lines.extend(["", f"### {run['run']} — complete={run['measurement_complete']}; DPS={run['dps']}", "",
                          "| Spell | Requests | Successes | Event damage | Missing casts | Zero damage | Unused |",
                          "|---:|---:|---:|---:|---:|---:|:---:|"])
            for row in run["abilities"]:
                lines.append(f"| {row['spell']} | {row['requests']} | {row['successes']} | {row['event_damage']} | {row['missing_casts']} | {row['zero_damage_findings']} | {row['unused']} |")
            lines.extend(["", "Unattributed native errors: " + json.dumps(run["unattributed_native_errors"])])
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--repeat", type=int, default=2)
    args = parser.parse_args()
    if args.repeat < 1:
        parser.error("Positive repeat required")
    roster = json.loads((args.source / "roster.json").read_text())
    summary = summarize_roster(args.source, roster, args.repeat)
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "abilities.json").write_text(json.dumps(summary, indent=2) + "\n")
    (args.output / "abilities.md").write_text(render_markdown(summary))


if __name__ == "__main__":
    main()
