#!/usr/bin/env python3
"""Publish authenticated local-CASC model chains without inferring legacy ownership.

Input is a staged extraction receipt set, not an unqualified models directory.
Frozen extraction identity and resolution records authenticate every asset;
publication never re-selects an active install. Metadata hashes remain separate.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import sqlite3

from import_forever_skyborne import validate_magic


def read_installed_identity(install, product):
    with (install / ".build.info").open(newline="") as stream:
        reader = csv.DictReader(stream, delimiter="|")
        rows = [{key.split("!")[0]: value for key, value in row.items()} for row in reader]
    active = [row for row in rows if row["Product"] == product and row["Active"] == "1"]
    if len(active) != 1:
        raise ValueError(f"{product}: expected one active installed build, found {len(active)}")
    row = active[0]
    key = row["Build Key"]
    config = install / "Data/config" / key[:2] / key[2:4] / key
    return {"product": product, "build_key": key, "build": row["Version"],
            "build_config_sha256": hashlib.sha256(config.read_bytes()).hexdigest()}


def create_asset_receipt(identity, fdid, kind, relative_path, raw, content_key):
    if hashlib.md5(raw).hexdigest() != content_key:
        raise ValueError(f"FDID {fdid}: bytes do not match source content key {content_key}")
    path = f"products/{identity['product']}/{identity['build_key']}/{relative_path}"
    return {**identity, "fdid": fdid, "kind": kind, "path": path,
            "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest(),
            "content_key": content_key}


def read_verified_asset(identity, staged, connection):
    if (staged["product"], staged["build_key"]) != (identity["product"], identity["build_key"]):
        raise ValueError(f"FDID {staged['fdid']}: staged identity differs from frozen extraction identity")
    fdid, kind = staged["fdid"], staged["kind"]
    record = connection.execute("SELECT content_key FROM resolution WHERE fdid=?", (fdid,)).fetchone()
    if record is None:
        raise ValueError(f"FDID {fdid}: no source resolution record")
    raw = Path(staged["source_path"]).read_bytes()
    content_key = bytes(record[0]).hex()
    validate_magic(raw, kind, content_key)
    if staged["content_key"] != content_key or staged["sha256"] != hashlib.sha256(raw).hexdigest():
        raise ValueError(f"FDID {fdid}: staged receipt differs from verified source bytes")
    if staged["bytes"] != len(raw):
        raise ValueError(f"FDID {fdid}: staged byte count differs from source")
    directory = "textures" if kind == "blp" else "models"
    receipt = create_asset_receipt(identity, fdid, kind, f"{directory}/{fdid}.{kind}", raw, content_key)
    return receipt, raw


def prepare_verified_assets(identity, staged, resolution_path):
    expected_resolution = identity["resolution_sha256"]
    if hashlib.sha256(resolution_path.read_bytes()).hexdigest() != expected_resolution:
        raise ValueError("Source resolution snapshot differs from frozen extraction receipt")
    if staged["failures"]:
        raise ValueError(f"Cannot publish incomplete chain: {staged['failures']}")
    connection = sqlite3.connect(f"file:{resolution_path}?mode=ro", uri=True)
    try:
        verified = [read_verified_asset(identity, asset, connection) for asset in staged["assets"]]
    finally:
        connection.close()
    keys = [(receipt["fdid"], receipt["kind"]) for receipt, _ in verified]
    if len(keys) != len(set(keys)):
        raise ValueError("Duplicate staged asset mappings")
    return dict(zip(keys, verified))


def prepare_companion_aliases(identity, dependencies, verified):
    aliases = []
    for parent, references in dependencies.items():
        model_fdid, kind = parent.split(".")
        required = {(fdid, child_kind) for child_kind, fdids in references.items() for fdid in fdids}
        missing = required - verified.keys()
        if missing:
            raise ValueError(f"{parent}: missing required assets {sorted(missing)}")
        if kind != "m2":
            continue
        for index, fdid in enumerate(references.get("skin", [])):
            receipt, raw = verified[fdid, "skin"]
            alias = create_asset_receipt(identity, fdid, "skin", f"models/{model_fdid}{index:02}.skin", raw, receipt["content_key"])
            aliases.append((alias, raw))
        for fdid in references.get("skel", []):
            receipt, raw = verified[fdid, "skel"]
            alias = create_asset_receipt(identity, fdid, "skel", f"models/{model_fdid}.skel", raw, receipt["content_key"])
            aliases.append((alias, raw))
    return aliases


def write_asset_files(data, verified):
    for receipt, raw in verified:
        path = data / receipt["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(path.suffix + ".importing")
        temporary.write_bytes(raw)
        temporary.replace(path)


def write_asset_index(data, identity, staged, verified, aliases):
    path = data / "cache/model-asset-index.json"
    existing = json.loads(path.read_text()) if path.exists() else {"version": 1, "assets": [], "aliases": [], "chains": []}
    if existing["version"] != 1:
        raise ValueError(f"Unsupported model asset index version {existing['version']}")
    assets = {(r["product"], r["fdid"], r["kind"]): r for r in existing["assets"]}
    assets.update({(r["product"], r["fdid"], r["kind"]): r for r, _ in verified})
    alias_records = {r["path"]: r for r in existing.get("aliases", [])}
    alias_records.update({r["path"]: r for r, _ in aliases})
    chain = {**identity, "dependencies": staged["dependencies"], "metadata": staged.get("metadata", [])}
    chains = [c for c in existing.get("chains", []) if (c["product"], c["build_key"]) != (identity["product"], identity["build_key"])]
    result = {"version": 1, "assets": [assets[key] for key in sorted(assets)],
              "aliases": [alias_records[key] for key in sorted(alias_records)], "chains": [*chains, chain]}
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".json.importing")
    temporary.write_text(json.dumps(result, indent=2) + "\n")
    temporary.replace(path)
    return result


def read_frozen_identity(staged, product):
    identity = staged["source_identity"]
    if identity["product"] != product or not identity["build"]:
        raise ValueError("Staged source identity does not match requested product")
    key = identity["build_key"]
    if len(key) != 32 or any(char not in "0123456789abcdef" for char in key):
        raise ValueError("Frozen source build key must be 32 hexadecimal characters")
    return identity


def import_staged_chain(data, product, staged_manifest, resolution_path):
    staged = json.loads(staged_manifest.read_text())
    identity = read_frozen_identity(staged, product)
    verified = prepare_verified_assets(identity, staged, resolution_path)
    aliases = prepare_companion_aliases(identity, staged["dependencies"], verified)
    write_asset_files(data, [*verified.values(), *aliases])
    return write_asset_index(data, identity, staged, verified.values(), aliases)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--product", choices=["wow", "wow_classic_beta"], required=True)
    parser.add_argument("--staged-manifest", type=Path, required=True)
    parser.add_argument("--resolution", type=Path, required=True)
    args = parser.parse_args()
    result = import_staged_chain(args.data, args.product, args.staged_manifest, args.resolution)
    print(json.dumps({"index": str(args.data / "cache/model-asset-index.json"), "assets": len(result["assets"]), "aliases": len(result["aliases"])}))


if __name__ == "__main__":
    main()
