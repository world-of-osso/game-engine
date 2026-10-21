#!/usr/bin/env python3
"""Authenticate legacy bytes against local product roots before scoped publication.

Consumes a recorded no-install misses.json. Never extracts, downloads, substitutes
another product, overwrites asset bytes, or treats a filename as provenance.
"""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

import closure_casc
from closure_extract import runtime_path
import import_model_asset_chains as chains
from import_forever_skyborne import asset_references, validate_magic


def parse_build_pins(values):
    builds = {}
    for value in values:
        product, separator, build = value.partition("=")
        if not separator or not product or not build or product in builds:
            raise ValueError(f"Expected one unique PRODUCT=VERSION pin: {value!r}")
        builds[product] = build
    if not builds:
        raise ValueError("At least one explicit PRODUCT=VERSION pin is required")
    return builds


def authenticate_root(install, product, wanted, indices, output, *, expected_build):
    if product["Version"] != expected_build:
        raise ValueError(
            f"Unexpected actual build: {product['Product']} {product['Version']}"
        )
    build = product["Build Key"]
    config_path = install / "Data/config" / build[:2] / build[2:4] / build
    config_bytes = config_path.read_bytes()
    if hashlib.md5(config_bytes).hexdigest() != build:
        raise ValueError("build config content hash mismatch")
    config = dict(
        line.split(" = ", 1)
        for line in config_bytes.decode().splitlines()
        if " = " in line
    )
    encoding_ck, encoding_ek = config["encoding"].split()
    encoding = closure_casc.read_metadata(install, indices, encoding_ek, encoding_ck)
    root_ck = config["root"]
    record = closure_casc.encoding_keys(encoding, {bytes.fromhex(root_ck)})[
        bytes.fromhex(root_ck)
    ]
    root_ek = next(key for key in record[1] if key[:9] in indices)
    root = closure_casc.read_metadata(install, indices, root_ek.hex(), root_ck)
    roots = closure_casc.root_keys(root, wanted)
    snapshot = output / (product["Product"] + "-root-keys.json")
    snapshot.write_text(
        json.dumps(
            {
                "build_key": build,
                "root_content_key": root_ck,
                "encoding_content_key": encoding_ck,
                "roots": {str(k): v.hex() for k, v in sorted(roots.items())},
            },
            indent=2,
        )
        + "\n"
    )
    identity = {
        "product": product["Product"],
        "build_key": build,
        "build": product["Version"],
        "build_config_sha256": hashlib.sha256(config_bytes).hexdigest(),
        "resolution_sha256": hashlib.sha256(snapshot.read_bytes()).hexdigest(),
    }
    return identity, roots


def find_fragment_receipt(data, receipt, field):
    mappings = {}
    for path in sorted((data / "cache").glob("model-asset-index-*.json")):
        fragment = json.loads(path.read_text())
        if fragment["version"] != 1:
            raise ValueError(f"Unsupported receipt fragment version: {path}")
        for record in fragment.get(field, []):
            key = (
                (record["product"], record["fdid"], record["kind"])
                if field == "assets"
                else record["path"]
            )
            if field == "assets" and key in mappings:
                raise ValueError(f"Duplicate fragment mapping: {key}")
            mappings.setdefault(key, []).append((record, path))
    key = (
        (receipt["product"], receipt["fdid"], receipt["kind"])
        if field == "assets"
        else receipt["path"]
    )
    existing = mappings.get(key)
    if existing is None:
        return None
    fields = (
        "product",
        "build_key",
        "build",
        "fdid",
        "kind",
        "path",
        "bytes",
        "sha256",
        "content_key",
    )
    for record, path in existing:
        if any(record[name] != receipt[name] for name in fields):
            raise ValueError(f"conflicting fragment mapping: {key} in {path}")
    return existing[0]


def publish_legacy(data, identity, asset, roots):
    if asset["product"] != identity["product"]:
        raise ValueError("requested product differs from authenticated root")
    fdid, kind = asset["fdid"], asset["extension"]
    row = dict(asset)
    content = roots.get(fdid)
    if content is None:
        return dict(row, status="root_absent")
    source = runtime_path(data, asset["legacy_candidate_path"])
    if not source.is_file():
        return dict(row, status="bytes_absent")
    raw = source.read_bytes()
    digest = hashlib.md5(raw).hexdigest()
    row.update(legacy_content_key=digest, expected_content_key=content.hex())
    if digest != content.hex():
        return dict(row, status="content_mismatch")
    validate_magic(raw, kind, content.hex())
    directory = "textures" if kind == "blp" else "models"
    receipt = chains.create_asset_receipt(
        identity, fdid, kind, f"{directory}/{fdid}.{kind}", raw, content.hex()
    )
    try:
        fragment = find_fragment_receipt(data, receipt, "assets")
        chains.write_asset_files(data, [(receipt, raw)])
    except ValueError as error:
        return dict(row, status="scoped_conflict", reason=str(error))
    if fragment is not None:
        record, path = fragment
        return dict(row, status="verified", receipt=record, receipt_location=str(path))
    chains.write_asset_index(
        data, identity, {"dependencies": {}, "metadata": []}, [(receipt, raw)], []
    )
    return dict(row, status="verified", receipt=receipt)


def publish_model_companion(data, identity, asset, roots):
    """Authenticate the parent SFID/SKID relation as well as the payload."""
    if asset["actual_build"] != identity["build_key"]:
        return dict(asset, status="build_mismatch")
    filename = Path(asset["legacy_candidate_path"]).name
    kind = asset["extension"]
    model = int(Path(filename).stem[:-2] if kind == "skin" else Path(filename).stem)
    slot = int(Path(filename).stem[-2:]) if kind == "skin" else 0
    parent = runtime_path(
        data,
        f"products/{identity['product']}/{identity['build_key']}/models/{model}.m2",
    )
    if not parent.is_file():
        return dict(asset, status="parent_bytes_absent")
    raw = parent.read_bytes()
    if hashlib.md5(raw).digest() != roots.get(model):
        return dict(asset, status="parent_content_mismatch")
    dependencies = asset_references(raw)
    references = dependencies.get(kind, [])
    if slot >= len(references) or references[slot] != asset["fdid"]:
        return dict(asset, status="companion_mismatch")
    row = publish_legacy(data, identity, asset, roots)
    if row["status"] != "verified":
        return row
    receipt = row["receipt"]
    payload = (data / receipt["path"]).read_bytes()
    alias = chains.create_asset_receipt(
        identity,
        asset["fdid"],
        kind,
        f"models/{filename}",
        payload,
        receipt["content_key"],
    )
    try:
        fragment = find_fragment_receipt(data, alias, "aliases")
        chains.write_asset_files(data, [(alias, payload)])
    except ValueError as error:
        return dict(row, status="scoped_conflict", reason=str(error))
    if fragment is not None:
        record, path = fragment
        return dict(row, alias=record, alias_location=str(path))
    staged = {"dependencies": {f"{model}.m2": dependencies}, "metadata": []}
    chains.write_asset_index(data, identity, staged, [], [(alias, payload)])
    return dict(row, alias=alias)


def is_model_alias(asset):
    return (
        asset["extension"] in ("skin", "skel")
        and Path(asset["legacy_candidate_path"]).name
        != f"{asset['fdid']}.{asset['extension']}"
    )


def companion_parent(asset):
    stem = Path(asset["legacy_candidate_path"]).stem
    return int(stem[:-2] if asset["extension"] == "skin" else stem)


def publish_miss(data, identity, asset, roots):
    if asset["kind"] == "missing_scoped_bytes":
        if asset["actual_build"] != identity["build_key"]:
            return dict(asset, status="build_mismatch")
        if is_model_alias(asset):
            return publish_model_companion(data, identity, asset, roots)
    return publish_legacy(data, identity, asset, roots)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--misses", type=Path, required=True)
    parser.add_argument("--install", type=Path, required=True)
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--build",
        action="append",
        required=True,
        metavar="PRODUCT=VERSION",
        help="explicit actual-build pin for each requested product",
    )
    args = parser.parse_args()
    try:
        builds = parse_build_pins(args.build)
    except ValueError as error:
        parser.error(str(error))
    assets = [
        row
        for row in json.loads(args.misses.read_text())["asset_misses"]
        if row["kind"] in ("missing_verified_receipt", "missing_scoped_bytes")
    ]
    requested = {row["product"] for row in assets}
    if requested != builds.keys():
        parser.error(
            f"Build pins must match requested products exactly: {sorted(requested)}"
        )
    args.output.mkdir(parents=True, exist_ok=True)
    products = {p["Product"]: p for p in closure_casc.installed_products(args.install)}
    indices, _, sources = closure_casc.load_indices(args.install)
    wanted = {row["fdid"] for row in assets}
    wanted.update(
        companion_parent(row)
        for row in assets
        if row["kind"] == "missing_scoped_bytes" and is_model_alias(row)
    )
    authenticated = {
        name: authenticate_root(
            args.install,
            products[name],
            wanted,
            indices,
            args.output,
            expected_build=builds[name],
        )
        for name in sorted(requested)
    }
    results = []
    for asset in assets:
        identity, roots = authenticated[asset["product"]]
        row = publish_miss(args.data, identity, asset, roots)
        if row["status"] == "content_mismatch":
            row["other_authenticated_matches"] = [
                name
                for name, (_, keys) in authenticated.items()
                if keys.get(asset["fdid"], b"").hex() == row["legacy_content_key"]
            ]
        results.append(row)
    summary = {
        "counts": dict(Counter(r["status"] for r in results)),
        "idx_sources": sources,
        "builds": {name: identity for name, (identity, _) in authenticated.items()},
    }
    (args.output / "results.json").write_text(
        json.dumps({"summary": summary, "assets": results}, indent=2) + "\n"
    )
    print(json.dumps(summary["counts"]))


if __name__ == "__main__":
    main()
