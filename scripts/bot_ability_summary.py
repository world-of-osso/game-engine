"""Read-only per-ability triage from retained native run/report artifacts."""
import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path

from bot_health import qualify_report_health
from bot_harness import parse_combat
from bot_combat_metrics import damage_outcomes, resource_net_change, sampled_usage, source_effects_by_spell, spell_key


def observed_spell_names(outcomes, samples):
    names = defaultdict(set)
    if outcomes is not None:
        for spell, observed in outcomes["names"].items():
            names[spell].update(observed)
    if samples:
        for row in samples[0]["spells"].get("spellbook", []):
            names[row["id"]].add(row["name"])
    return names


def source_auto_attack(source):
    if source is None:
        return None
    if any(int(row["Attributes_1"]) & 0x200 for row in source["SpellMisc"]):
        return "on_cast"
    if any(int(row["Attributes_2"]) & 0x100000 for row in source["SpellMisc"]):
        return "post_cast"
    return "none"


def describe_abilities(manifest, report, outcomes, samples):
    requests = Counter(attempt["spell"] for attempt in manifest["attempts"])
    successes = {spell_key(spell): count for spell, count in report["casts"].items()}
    damage = {spell_key(spell): amount for spell, amount in report["damage_by_spell"].items()}
    metadata = {spell["id"]: spell for spell in manifest["spells"]}
    effects = source_effects_by_spell(metadata)
    names = observed_spell_names(outcomes, samples)
    findings = Counter((finding["kind"], finding.get("spell")) for finding in report["findings"])
    identities = set(metadata) | set(requests) | set(successes) | set(damage)
    if outcomes is not None:
        identities.update(outcomes["by_spell"])
    abilities = []
    for spell in sorted(identities, key=lambda value: (-1 if value is None else value)):
        source = metadata.get(spell, {}).get("source")
        counts = {"hits": None, "crits": None, "misses": None, "miss_by_kind": None}
        if outcomes is not None:
            counts = outcomes["by_spell"].get(spell, {"hits": 0, "crits": 0, "misses": 0, "miss_by_kind": {}})
        abilities.append({"spell": spell, "names": sorted(names[spell]),
                          "requests": requests[spell], "successes": successes.get(spell, 0),
                          "event_damage": damage.get(spell, 0), **counts,
                          "damage_ids": metadata.get(spell, {}).get("damage_ids", []),
                          "missing_casts": findings["cast_never_fired", spell],
                          "zero_damage_findings": findings["zero_damage_spell", spell],
                          "unused": bool(findings["ability_never_used", spell]),
                          "source_effects": effects.get(spell), "source_auto_attack": source_auto_attack(source),
                          "raw_power_costs": source["SpellPower"] if source is not None else None,
                          "raw_cooldowns": source["SpellCooldowns"] if source is not None else None})
    return abilities


def summarize_run(manifest, report, events=None, samples=None):
    outcomes = damage_outcomes(manifest, events) if events is not None else None
    abilities = describe_abilities(manifest, report, outcomes, samples)
    unobserved = ["resource_spending", "final_caster_modified_damage_formula"]
    if outcomes is None:
        unobserved.extend(["combat_outcomes", "crit_outcomes"])
    elif any(row["crits"] is None for row in outcomes["by_spell"].values()):
        unobserved.append("crit_outcomes")
    sampling = sampled_usage(manifest, samples) if samples is not None and len(samples) >= 2 else None
    if sampling is None:
        unobserved.append("sampled_gcd_idle")
    net_change = resource_net_change(samples)
    if net_change is None:
        unobserved.append("resource_net_change")
    errors = Counter(error for sample in report["native_errors"] for error in sample["errors"])
    return {"spec": manifest["spec"], "role": manifest["role"], "authored_order": manifest.get("authored_order"),
            "duration_ms": report["duration_ms"], "measurement_complete": report["measurement_complete"],
            "damage": report["damage"], "dps": report["dps"], "abilities": abilities,
            "auto_attack": outcomes["auto_attack"] if outcomes is not None else {"damage": None, "share": None},
            "sampling": sampling, "resource_net_change": net_change, "resource_spent": None,
            "resource_spent_reason": "no per-cast power-debit record; net power change may include regeneration, gains and refunds",
            "unobserved": unobserved, "unattributed_native_errors": dict(errors)}


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
        combat_path, samples_path = directory / "combat.log", directory / "native-samples.jsonl"
        events = parse_combat(combat_path.read_text()) if combat_path.exists() else None
        samples = [json.loads(line) for line in samples_path.read_text().splitlines()] if samples_path.exists() else None
        result = summarize_run(json.loads(manifest_path.read_text()), report, events, samples)
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
                          "| Spell | Names | Requests | Successes | Hits | Crits | Misses | Event damage | Missing casts | Unused |",
                          "|---:|---|---:|---:|---:|---:|---:|---:|---:|:---:|"])
            for row in run["abilities"]:
                label = "swing" if row["spell"] is None else str(row["spell"])
                names = ", ".join(row["names"]).replace("|", "\\|")
                lines.append(f"| {label} | {names} | {row['requests']} | {row['successes']} | {row['hits']} | {row['crits']} | {row['misses']} | {row['event_damage']} | {row['missing_casts']} | {row['unused']} |")
            lines.extend(["", "Auto-attack: " + json.dumps(run["auto_attack"]),
                          "", "Sampled timing (not exact server occupancy): " + json.dumps(run["sampling"]),
                          "", "Resource net change (NOT spending): " + json.dumps(run["resource_net_change"]),
                          "", "Unobserved: " + ", ".join(run["unobserved"]),
                          "", "Unattributed native errors: " + json.dumps(run["unattributed_native_errors"])])
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
