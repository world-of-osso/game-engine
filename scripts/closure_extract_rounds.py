#!/usr/bin/env python3
"""Extract local Retail closure in bounded batches, then traverse to a fixed point."""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import time

import closure_casc
from closure_extract import read_receipts, summarize_receipts
from closure_extract_batch import extract_batch, pending_assets


def count_retryable(statuses, failures):
    return sum(
        status == "local_index_present" and fdid not in failures
        for fdid, status in statuses.items()
    )


def fixed_point(manifest, extract, traverse, count, record):
    number = 1
    while True:
        before = count(manifest)
        extraction = extract(manifest, number)
        # Full-catalog graphs are multi-GB; release the prior round before replacement.
        manifest = None
        manifest = traverse()
        after = count(manifest)
        record(
            {
                "round": number,
                "before": before,
                "after": after,
                "extraction": extraction,
                "summary": manifest["summary"],
            }
        )
        if after == 0 or (extraction["files"] == 0 and after >= before):
            return manifest
        number += 1


class LocalInventory:
    def __init__(self, install, cache, version):
        rows = [
            r for r in closure_casc.installed_products(install) if r["Product"] == "wow"
        ]
        if len(rows) != 1 or rows[0]["Version"] != version:
            raise ValueError(
                "installed wow build differs from requested metadata build"
            )
        self.product = rows[0]
        self.build = self.product["Build Key"]
        config_path = (
            install / "Data/config" / self.build[:2] / self.build[2:4] / self.build
        )
        text = config_path.read_bytes()
        if hashlib.md5(text).hexdigest() != self.build:
            raise ValueError("build config content hash mismatch")
        config = dict(
            l.split(" = ", 1) for l in text.decode().splitlines() if " = " in l
        )
        self.cache = cache / "casc/wow" / self.build / "schema-2"
        self.root = (self.cache / "root.bin").read_bytes()
        self.encoding = (self.cache / "encoding.bin").read_bytes()
        for payload, expected in [
            (self.root, config["root"]),
            (self.encoding, config["encoding"].split()[0]),
        ]:
            if hashlib.md5(payload).hexdigest() != expected:
                raise ValueError(
                    "native resolution metadata differs from authenticated build config"
                )
        self.indices, self.archives, _ = closure_casc.load_indices(install)
        self.database = self.cache / "resolution.sqlite"
        self.identities = {}
        self.statuses = {}

    def describe(self, assets):
        wanted = {a["fdid"] for a in assets}
        new = wanted - self.statuses.keys()
        if new:
            roots = closure_casc.root_keys(self.root, new)
            encoded = closure_casc.encoding_keys(self.encoding, set(roots.values()))
            resolution = closure_casc.resolve_missing(
                new, roots, encoded, self.indices, self.archives
            )
            with sqlite3.connect(
                f"file:{self.database}?mode=ro", uri=True
            ) as connection:
                for row in resolution["files"]:
                    fdid = row["fdid"]
                    self.statuses[fdid] = row["status"]
                    if row["status"] != "local_index_present":
                        continue
                    content = roots[fdid]
                    size, keys = encoded[content]
                    native = connection.execute(
                        "SELECT content_key, encoding_key FROM resolution WHERE fdid=?",
                        (fdid,),
                    ).fetchone()
                    if native != (content, keys[0]):
                        raise ValueError(
                            f"native cache differs from authenticated root/encoding for FDID {fdid}"
                        )
                    if keys[0][:9] not in self.indices:
                        self.statuses[fdid] = "native_first_encoding_idx_missing"
                        continue
                    self.identities[fdid] = {
                        "fdid": fdid,
                        "product": "wow",
                        "actual_build": self.product["Version"],
                        "build_config": self.build,
                        "content_key": content.hex(),
                        "encoding_key": keys[0].hex(),
                        "size": size,
                    }
        return {fdid: self.statuses[fdid] for fdid in wanted}


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--install", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--extractor", type=Path, required=True)
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--path-resolution-cache", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--handoff", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--batch-size", type=int, default=10000)
    args = parser.parse_args()
    if not 1 <= args.workers <= 8 or args.batch_size < 1:
        parser.error("workers must be 1..8 and batch-size positive")
    args.output.mkdir(parents=True, exist_ok=True)
    config = json.loads(args.config.read_text())
    inventory = LocalInventory(args.install, args.cache, config["metadata_build"])
    receipts = read_receipts(args.data / "provenance/closure-extract.jsonl")
    failures_path = args.output / "failures.json"
    failures = (
        {int(k): v for k, v in json.loads(failures_path.read_text()).items()}
        if failures_path.exists()
        else {}
    )
    rounds = []
    current_round = [0]

    def count(manifest):
        missing = [a for a in manifest["assets"] if not a["present"]]
        statuses = inventory.describe(missing)
        return count_retryable(statuses, failures)

    def extract(manifest, number):
        current_round[0] = number
        started = time.monotonic()
        candidates = pending_assets(
            manifest["assets"], args.data, receipts, set(failures)
        )
        statuses = inventory.describe(candidates)
        indexed = []
        for asset in candidates:
            status = statuses[asset["fdid"]]
            if status == "local_index_present":
                indexed.append(asset)
            else:
                failures[asset["fdid"]] = status
        output = args.output / f"round-{number:02d}"
        output.mkdir(exist_ok=True)
        totals = {"files": 0, "bytes": 0, "by_type": {}, "attempted": len(indexed)}
        print(
            json.dumps(
                {"round": number, "attempted": len(indexed), "status": "extracting"}
            ),
            flush=True,
        )
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            futures = []
            for batch, offset in enumerate(range(0, len(indexed), args.batch_size)):
                futures.append(
                    pool.submit(
                        extract_batch,
                        indexed[offset : offset + args.batch_size],
                        inventory.identities,
                        args.extractor,
                        args.data,
                        output,
                        batch,
                        str(inventory.cache),
                        receipts,
                        {
                            "WOW_INSTALL_PATH": str(args.install),
                            "ASSET_RESOLVER_CACHE_DIR": str(args.cache),
                            "ASSET_RESOLVER_DATA_DIR": str(args.data),
                        },
                    )
                )
            for future in as_completed(futures):
                result = future.result()
                failures.update(result["failures"])
                totals["files"] += result["files"]
                totals["bytes"] += result["bytes"]
                for kind, values in result["by_type"].items():
                    target = totals["by_type"].setdefault(
                        kind, {"files": 0, "bytes": 0}
                    )
                    for key in target:
                        target[key] += values[key]
                write_json(failures_path, failures)
                write_json(output / "progress.json", totals)
                with args.handoff.open("a") as stream:
                    stream.write(
                        f"Round {number} batch progress: {totals['files']} paths, {totals['bytes']} bytes; {len(failures)} recorded failures.\n"
                    )
                print(
                    json.dumps(
                        {
                            "round": number,
                            "files": totals["files"],
                            "bytes": totals["bytes"],
                            "failures": len(failures),
                        }
                    ),
                    flush=True,
                )
        totals["seconds"] = round(time.monotonic() - started, 2)
        write_json(failures_path, failures)
        return totals

    def traverse():
        output = args.output / f"round-{current_round[0]:02d}"
        path = output / "manifest.json"
        command = [
            "python3",
            str(Path(__file__).with_name("asset_closure.py")),
            "--data",
            str(args.data),
            "--world-db",
            str(args.world_db),
            "--config",
            str(args.config),
            "--output",
            str(path),
        ]
        if args.path_resolution_cache is not None:
            command.extend(["--path-resolution-cache", str(args.path_resolution_cache)])
        print(f"Traversing round {current_round[0]}", flush=True)
        with (output / "traversal.log").open("w") as stream:
            result = subprocess.run(command, stdout=stream, stderr=stream, check=False)
        if result.returncode not in {0, 1} or not path.exists():
            raise RuntimeError(
                f"closure traversal failed; see {output / 'traversal.log'}"
            )
        return json.loads(path.read_text())

    def record(row):
        rounds.append(row)
        write_json(args.output / "rounds.json", rounds)
        with args.handoff.open("a") as stream:
            stream.write("Round complete: " + json.dumps(row, sort_keys=True) + "\n")
        print(json.dumps(row, sort_keys=True), flush=True)

    final = fixed_point(
        json.loads(args.manifest.read_text()), extract, traverse, count, record
    )
    final_path = args.output / f"round-{current_round[0]:02d}/manifest.json"
    summary = {
        "rounds": rounds,
        "final": final["summary"],
        "manifest": str(final_path),
        "manifest_sha256": hashlib.sha256(final_path.read_bytes()).hexdigest(),
        "extracted_by_type": summarize_receipts(
            read_receipts(args.data / "provenance/closure-extract.jsonl").values()
        ),
        "failure_reasons": dict(Counter(failures.values()).most_common()),
        "failed_fdids": len(failures),
        "remaining_unresolved": dict(
            Counter(r["code"] for r in final["unresolved"]).most_common()
        ),
        "remaining_missing_statuses": dict(
            Counter(
                inventory.describe(
                    [a for a in final["assets"] if not a["present"]]
                ).values()
            )
        ),
        "identity": {
            "product": "wow",
            "version": inventory.product["Version"],
            "build": inventory.build,
        },
    }
    write_json(args.output / "summary.json", summary)
    with args.handoff.open("a") as stream:
        stream.write(
            f"Fixed point finished; summary {args.output / 'summary.json'}; final manifest {final_path}.\n"
        )
    print("FIXED_POINT_EXIT=0", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
