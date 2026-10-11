#!/usr/bin/env python3
"""Publish portrait-black's missing Retail chains from authenticated local archives.

No runtime fallback: validate frozen source receipts and publish scoped files through
import_model_asset_chains. Starter cloth textures are explicit replacement inputs,
not dependencies embedded in the Human/Orc M2s.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess

from import_forever_skyborne import asset_references, validate_magic
from import_model_asset_chains import import_staged_chain, read_frozen_identity

# Human/Orc warlock starter cloth's purple/yellow replacement sets (local listfile)
# and Imp display4449. Include the whole two sets: loading stops at the first miss.
CLOTH_TEXTURES = [
    3946776, 3946777, 3946778, 3946779, 3946780, 3946781, 3946783, 3946784,
    3946786, 3946787, 3946789, 3946790, 3946794, 3946795, 3946796, 3946797,
    3946798, 3946799, 3946803, 3946804, 3946805, 3946806, 3946807, 3946808,
    3946809, 3946813, 3946814, 3946815, 3946816, 3946817, 3946820, 3946821,
    3946822, 3946823,
]
ROOTS = {(1098889, "m2")} | {(fdid, "blp") for fdid in CLOTH_TEXTURES}


def stage_assets(source_manifest, resolution, archives, extractor, keys, output):
    source = json.loads(source_manifest.read_text())
    identity = dict(read_frozen_identity(source, "wow"))
    snapshot = resolution.read_bytes()
    if hashlib.sha256(snapshot).hexdigest() != identity["resolution_sha256"]:
        raise ValueError("Frozen resolution differs from authenticated source receipt")
    output.mkdir(parents=True, exist_ok=True)
    frozen = output / "resolution.sqlite"
    frozen.write_bytes(snapshot)
    rawdir = output / "raw"
    rawdir.mkdir(exist_ok=True)
    stage = {"source_identity": identity, "assets": [], "dependencies": {}, "failures": []}
    pending = set(ROOTS)
    seen = set()
    connection = sqlite3.connect(frozen.resolve().as_uri() + "?mode=ro", uri=True)
    try:
        while pending:
            batch = sorted(pending - seen)
            if not batch:
                break
            pending.clear()
            command = [str(extractor), str(archives), str(frozen), str(keys), str(rawdir),
                       *[str(fdid) for fdid, _ in batch]]
            subprocess.run(command, check=True)
            for fdid, kind in batch:
                path = rawdir / f"{fdid}.dat"
                raw = path.read_bytes()
                row = connection.execute("SELECT content_key FROM resolution WHERE fdid=?", (fdid,)).fetchone()
                if row is None:
                    raise ValueError(f"FDID {fdid}: absent frozen resolution")
                content_key = bytes(row[0]).hex()
                validate_magic(raw, kind, content_key)
                if hashlib.md5(raw).hexdigest() != content_key:
                    raise ValueError(f"FDID {fdid}: bytes differ from frozen content key")
                seen.add((fdid, kind))
                stage["assets"].append({"fdid": fdid, "kind": kind, "product": "wow",
                    "build_key": identity["build_key"], "source_path": str(path),
                    "content_key": content_key, "sha256": hashlib.sha256(raw).hexdigest(),
                    "bytes": len(raw)})
                if kind in {"m2", "skel"}:
                    references = asset_references(raw)
                    stage["dependencies"][f"{fdid}.{kind}"] = references
                    pending.update((child, child_kind) for child_kind, ids in references.items()
                                   for child in ids if (child, child_kind) not in seen)
    finally:
        connection.close()
    manifest = output / "chain-receipts.json"
    manifest.write_text(json.dumps(stage, indent=2) + "\n")
    return manifest, frozen


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["data", "source-manifest", "resolution", "archives", "extractor", "keys", "output"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    manifest, frozen = stage_assets(args.source_manifest, args.resolution, args.archives,
                                    args.extractor, args.keys, args.output)
    result = import_staged_chain(args.data, "wow", manifest, frozen, fragment="portrait-black")
    print(f"Published {len(result['assets'])} portrait asset receipts")


if __name__ == "__main__":
    main()
