#!/usr/bin/env python3
"""Publish the Forever map census assets: models, their closed dependencies and receipts.

The Forever-only maps place 3,482 distinct model FileDataIDs and 3,203 of them have no
`wow_classic_beta` receipt, so the client refuses to spawn the objects that reference them
("No verified asset receipt for wow_classic_beta FDID <n>.<kind>").

This script walks the M2/WMO dependency closure with `asset_closure.RawByteGraph` (the same
read-only parser graph the audit uses), extracts each missing FDID from the local CASC through
the installed `casc-local`, and repeats until a fixed point. It then publishes every acquired
asset under the product tree and writes exactly one index fragment whose receipts carry
product, build key, build, config hash, kind, path, size, sha256 and CASC content key.

Nothing contacts the network, nothing is renamed, and a FDID whose bytes the build does not
contain is reported by class instead of being substituted from another product.
"""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

from asset_closure import DIRECTORIES, RawByteGraph

PRODUCT = "wow_classic_beta"
BUILD = "1.60.1.70790"
BUILD_KEY = "9006ba6948388dd0d4a0578e64b8a92e"
EXPANDABLE_FROM_CENSUS = {"m2", "wmo"}


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def directory_for(kind: str) -> str:
    return DIRECTORIES.get(kind, "models")


def seed_kinds(root: Path) -> dict:
    """The census models already extracted: `<fdid>.m2` or `<fdid>.wmo` under models/."""
    seeds = {}
    for path in sorted((root / "models").glob("*")):
        if path.suffix[1:] in EXPANDABLE_FROM_CENSUS:
            seeds[int(path.stem)] = path.suffix[1:]
    return seeds


def closure(root: Path) -> dict:
    graph = RawByteGraph(root, {}, PRODUCT, BUILD)
    for fdid, kind in seed_kinds(root).items():
        graph.add(fdid, kind, "forever census seed")
    return graph.run()


def content_keys(cache: Path) -> dict:
    database = cache / "resolution.sqlite"
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        return {
            fdid: key.hex() if isinstance(key, bytes) else key
            for fdid, key in connection.execute("select fdid, content_key from resolution")
        }


def extract(extractor: Path, staging: Path, wanted: dict) -> dict:
    """Extract `wanted` (fdid -> kind) and file each byte where the graph looks for it."""
    failures = {}
    scratch = staging / ".extract"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    by_kind = defaultdict(list)
    for fdid, kind in wanted.items():
        by_kind[kind].append(fdid)
    for kind in sorted(by_kind):
        fds = sorted(by_kind[kind])
        for start in range(0, len(fds), 200):
            batch = [str(f) for f in fds[start:start + 200]]
            result = subprocess.run(
                [str(extractor), *batch, "-o", str(scratch)],
                capture_output=True, text=True,
            )
            output = f"{result.stdout}\n{result.stderr}"
            reported = [
                line for line in output.splitlines() if line.startswith("Failed FDID")
            ]
            if result.returncode != 0 and not reported:
                raise SystemExit(f"casc-local failed for {kind}: {result.stderr.strip()}")
            for line in reported:
                fdid = int(line.split()[2].rstrip(":"))
                failures[fdid] = f"{kind}: {line.split(':', 1)[1].strip()}"
        for fdid in fds:
            produced = [p for p in scratch.glob(f"{fdid}.*") if p.suffix != ".log"]
            if not produced:
                failures.setdefault(fdid, f"{kind}: no bytes produced")
                continue
            source = produced[0]
            target = staging / directory_for(kind) / f"{fdid}.{kind}"
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(source), target)
    return failures


def publish(staging: Path, products: Path, assets: list, keys: dict, config_sha: str) -> list:
    receipts = []
    for asset in assets:
        if not asset["present"]:
            continue
        kind, fdid = asset["type"], asset["fdid"]
        source = staging / asset["present_files"][0]["path"]
        relative = f"products/{PRODUCT}/{BUILD_KEY}/{directory_for(kind)}/{fdid}.{kind}"
        target = products / PRODUCT / BUILD_KEY / directory_for(kind) / f"{fdid}.{kind}"
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists():
            shutil.copy2(source, target)
        content_key = keys.get(fdid)
        if content_key is None:
            raise SystemExit(f"no CASC content key for FDID {fdid}")
        receipts.append(
            {
                "product": PRODUCT,
                "build_key": BUILD_KEY,
                "build": BUILD,
                "build_config_sha256": config_sha,
                "fdid": fdid,
                "kind": kind,
                "path": relative,
                "bytes": target.stat().st_size,
                "sha256": digest(target),
                "content_key": content_key,
            }
        )
    return receipts


def existing_receipts(cache: Path, exclude: Path | None = None) -> set:
    seen = set()
    base = cache / "model-asset-index.json"
    paths = ([base] if base.is_file() else []) + sorted(
        cache.glob("model-asset-index-*.json")
    )
    for path in paths:
        if exclude is not None and path == exclude:
            continue
        try:
            document = json.loads(path.read_text())
        except (OSError, ValueError):
            continue
        for row in document.get("assets", []):
            seen.add((row.get("product"), row.get("fdid"), row.get("kind")))
    return seen


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--staging", type=Path, required=True)
    parser.add_argument("--products", type=Path, required=True, help="data/products")
    parser.add_argument("--cache", type=Path, required=True, help="data/cache")
    parser.add_argument("--install", type=Path, required=True)
    parser.add_argument("--extractor", type=Path, required=True)
    parser.add_argument("--index", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--casc-cache", type=Path, required=True)
    parser.add_argument("--rounds", type=int, default=8)
    parser.add_argument("--estimate-only", action="store_true")
    args = parser.parse_args()

    os.environ["WOW_INSTALL_PATH"] = str(args.install)
    os.environ["WOW_PRODUCT"] = PRODUCT
    config = args.install / "Data/config" / BUILD_KEY[:2] / BUILD_KEY[2:4] / BUILD_KEY
    config_sha = hashlib.sha256(config.read_bytes()).hexdigest()

    history = []
    for number in range(1, args.rounds + 1):
        manifest = closure(args.staging)
        summary = manifest["summary"]
        wanted = {
            asset["fdid"]: asset["type"]
            for asset in manifest["assets"]
            if not asset["present"]
        }
        print(f"round {number}: {summary['files']} assets, {len(wanted)} missing, "
              f"{summary['unresolved_by_code']}", flush=True)
        history.append({"round": number, "missing": len(wanted), "summary": summary})
        if args.estimate_only or not wanted:
            break
        failures = extract(args.extractor, args.staging, wanted)
        if failures:
            history[-1]["failures"] = {str(k): v for k, v in sorted(failures.items())}
            print(f"round {number}: {len(failures)} extraction failures", flush=True)
        if not any((args.staging / directory_for(k) / f"{f}.{k}").exists()
                   for f, k in wanted.items()):
            break

    manifest = closure(args.staging)
    present = [a for a in manifest["assets"] if a["present"]]
    bytes_total = sum(a["size"] or 0 for a in present)
    print(f"closure fixed point: {manifest['summary']['files']} assets, "
          f"{len(present)} present, {bytes_total / 2**20:.1f} MiB", flush=True)
    report = {
        "product": PRODUCT,
        "build": BUILD,
        "build_key": BUILD_KEY,
        "history": history,
        "closure": manifest["summary"],
        "unresolved": manifest["unresolved"][:500],
        "bytes_present": bytes_total,
        "estimate_only": args.estimate_only,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    if args.estimate_only:
        return 0

    keys = content_keys(args.casc_cache)
    published = existing_receipts(args.cache, exclude=args.index)
    receipts = [
        row
        for row in publish(args.staging, args.products, present, keys, config_sha)
        if (row["product"], row["fdid"], row["kind"]) not in published
    ]
    skipped = len(present) - len(receipts)
    receipts.sort(key=lambda row: (row["kind"], row["fdid"]))
    args.index.write_text(
        json.dumps({"version": 1, "assets": receipts}, indent=2) + "\n"
    )
    report["receipts"] = Counter(row["kind"] for row in receipts)
    report["receipts"]["_total"] = len(receipts)
    report["receipts_bytes"] = sum(row["bytes"] for row in receipts)
    report["skipped_already_published"] = skipped
    report["unresolved_by_code"] = manifest["summary"]["unresolved_by_code"]
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True, default=dict) + "\n")
    print(json.dumps(report["receipts"], sort_keys=True, default=dict))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
