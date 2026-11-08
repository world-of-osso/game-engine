#!/usr/bin/env python3
"""Run the P4 no-install certification poses (docs/specs/shipped-assets.md)."""

import argparse
import json
from pathlib import Path
import re
import sqlite3
import subprocess

ZONES = [
    {"name": "northshire", "map": "azeroth", "product": "wow", "point": (-8914.57, 80.5, 133.909), "snap": False},
    {"name": "stormwind", "map": "azeroth", "product": "wow", "point": (-8841.06, 109.6, -489.656), "snap": False},
    {"name": "dunmorogh", "map": "azeroth", "product": "wow", "point": (-6240.0, 382.8, -331.0), "snap": False},
    {"name": "orgrimmar", "map": "kalimdor", "product": "wow", "point": (1798.27, 102.5, 4363.27), "snap": False},
    {"name": "mulgore", "map": "kalimdor", "product": "wow", "point": (-2915.55, 59.45, 257.347), "snap": False},
    {"name": "durotar", "map": "kalimdor", "product": "wow", "point": (-618.52, 38.7, 4251.67), "snap": False},
    {"name": "stockades", "map": "StormwindJail", "product": "wow", "point": (56.68, -19.27, -0.62), "snap": False, "mode": "global_wmo"},
    {"name": "zephras", "map": "2991", "product": "wow_classic_beta", "point": (0.0, 0.0, 0.0), "snap": True},
    {"name": "dalaran-2959", "map": "2959", "product": "wow_classic_beta", "point": (0.0, 0.0, 0.0), "snap": True},
    {"name": "dalaran-2980", "map": "2980", "product": "wow_classic_beta", "point": (0.0, 0.0, 0.0), "snap": True},
    {"name": "nightmare-grove", "map": "2832", "product": "wow_classic_beta", "point": (0.0, 0.0, 0.0), "snap": True},
]

FLAT_MISS = re.compile(r"missing extracted-only asset FDID (\d+)")
RECEIPT_MISS = re.compile(r"No verified asset receipt for (\S+) FDID (\d+)\.(\w+)")
FORBIDDEN = re.compile(r"forbids CASC access")


def run_pose(args, zone, output):
    log = output / f"{zone['name']}.log"
    evidence = output / zone["name"]
    evidence.mkdir()
    result_path = evidence / "result.json"
    shot = evidence / "capture.png"
    env = {
        "GAME_ENGINE_ASSET_MODE": "extracted-only",
        "WOW_INSTALL_PATH": str(args.empty_install),
        "WOW_DATA_PATH": str(args.empty_install / "Data"),
        "CERT_MAP": zone["map"],
        "CERT_POINT": ",".join(str(v) for v in zone["point"]),
        "CERT_SNAP": "1" if zone["snap"] else "0",
        "CERT_MODE": zone.get("mode", "terrain"),
        "CERT_OUTPUT": str(result_path),
        "CERT_SHOT": str(shot),
        "CERT_TIMEOUT": str(args.timeout_ms),
    }
    script = f"""
set -e
export {'; export '.join(f'{key}={value}' for key, value in env.items())}
cd {args.root}
scripts/agent/headless-client.sh start-godot {args.root} {args.xdg} -s res://tests/noinstall_certification.gd
cp=$(cat {args.root}/target/headless-client.pid)
child=""
for i in $(seq 1 100); do child=$(pgrep -P "$cp" || true); [ -n "$child" ] && break; sleep 0.1; done
[ -n "$child" ] && tail --pid="$child" -f /dev/null || true
scripts/agent/headless-client.sh stop {args.root}/target
cp {args.root}/target/headless-client.log {log}
"""
    completed = subprocess.run(["bash", "-lc", script], text=True, capture_output=True)
    (evidence / "runner.log").write_text(completed.stdout + completed.stderr)
    facts = json.loads(result_path.read_text()) if result_path.exists() else {}
    lines = log.read_text(errors="replace").splitlines() if log.exists() else []
    flat = sorted({int(m.group(1)) for m in map(FLAT_MISS.search, lines) if m})
    receipts = sorted({(m.group(1), int(m.group(2)), m.group(3)) for m in map(RECEIPT_MISS.search, lines) if m})
    return {
        "zone": zone,
        "exit": completed.returncode,
        "facts": facts,
        "flat_misses": flat,
        "receipt_misses": receipts,
        "forbidden_casc": sum(1 for line in lines if FORBIDDEN.search(line)),
        "fallbacks": sorted({line.strip() for line in lines if "fallback" in line.lower()}),
        "log": str(log),
        "capture": str(shot),
    }


def classify(row, resolution):
    product = row["zone"].get("product", "wow")
    identities = list(row["flat_misses"]) + [fdid for _, fdid, _ in row["receipt_misses"]]
    result = {"locally_fixable": [], "absent_from_build": []}
    database = resolution.get(product)
    for fdid in identities:
        kind = "absent_from_build"
        if database and database.execute("select 1 from resolution where fdid=?", (fdid,)).fetchone():
            kind = "locally_fixable"
        result[kind].append({"product": product, "fdid": fdid})
    return result


def pose_pass(row):
    facts = row["facts"]
    if row["exit"] != 0 or not facts:
        return False
    if facts.get("runtime", {}).get("forbidden_casc_accesses") != 0:
        return False
    if row["flat_misses"] or row["receipt_misses"] or row["forbidden_casc"]:
        return False
    if facts.get("mode") == "global_wmo":
        return facts.get("global_wmo_fdid", 0) > 0 and facts.get("terrain_failures", 1) == 0 and facts.get("visible_meshes", 0) > 0
    return (
        facts.get("drained") is True
        and facts.get("tiles", 0) > 0
        and facts.get("terrain_failures", 1) == 0
        and facts.get("spawned", 0) > 0
        and facts.get("pending", 1) == 0
        and facts.get("object_failures", 1) == 0
    )


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True, help="game-engine checkout")
    parser.add_argument("--output", type=Path, required=True, help="new diagnostics directory")
    parser.add_argument("--xdg", type=Path, required=True)
    parser.add_argument("--timeout-ms", type=int, default=180000)
    parser.add_argument("--only", help="comma-separated zone names")
    parser.add_argument(
        "--resolution", action="append", default=[],
        metavar="PRODUCT=SQLITE",
        help="product resolution.sqlite for miss classification",
    )
    args = parser.parse_args(argv)
    args.output.mkdir(parents=True, exist_ok=False)
    empty = args.output / "empty-install"
    (empty / "Data" / "data").mkdir(parents=True)
    args.empty_install = empty
    args.xdg.mkdir(parents=True, exist_ok=True)
    resolution = {}
    for item in args.resolution:
        product, path = item.split("=", 1)
        resolution[product] = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    selected = set(args.only.split(",")) if args.only else None
    rows = []
    for zone in ZONES:
        if selected and zone["name"] not in selected:
            continue
        row = run_pose(args, zone, args.output)
        row["pass"] = pose_pass(row)
        row["classification"] = classify(row, resolution)
        rows.append(row)
        print(f"CERT {zone['name']}: {'PASS' if row['pass'] else 'FAIL'} "
              f"spawned={row['facts'].get('spawned')} tiles={row['facts'].get('tiles')} "
              f"mesh={row['facts'].get('visible_meshes')} flat={row['flat_misses']} "
              f"receipt={row['receipt_misses']}", flush=True)
    report = {
        "criteria": "docs/specs/shipped-assets.md P4 no-install certification run",
        "generated": "noinstall_certification.py",
        "pass": all(row["pass"] for row in rows),
        "zones": [{key: value for key, value in row.items()} for row in rows],
    }
    (args.output / "report.json").write_text(json.dumps(report, indent=2))
    return 0 if report["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
