#!/usr/bin/env python3
"""Stage a complete model closure against an authenticated frozen CASC snapshot."""

import argparse
from contextlib import closing
import hashlib
import json
from pathlib import Path
import sqlite3
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_closure import RawByteGraph
import import_model_asset_chains as chains
import model_asset_publication as publication


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_snapshot(manifest, root, resolution):
    identity = json.loads(manifest.read_text())["source_identity"]
    for path, key in ((root, "root_sha256"), (resolution, "resolution_sha256")):
        if sha256(path) != identity[key]:
            raise ValueError(f"Frozen snapshot hash mismatch: {path}")
    return identity


def expand_owned_roots(graph, roots, owners, index_data, identity, connection):
    for fdid in roots:
        owner = owners.get((identity["product"], fdid, "m2"))
        if owner is None or owner[1]["build_key"] != identity["build_key"]:
            raise ValueError(f"M2 {fdid} is not owned by the selected frozen build")
        row = owner[1]
        raw = (index_data / row["path"]).read_bytes()
        content = connection.execute(
            "SELECT content_key FROM resolution WHERE fdid = ?", (fdid,)
        ).fetchone()
        if (
            content is None
            or bytes(content[0]).hex() != row["content_key"]
            or len(raw) != row["bytes"]
            or hashlib.sha256(raw).hexdigest() != row["sha256"]
            or hashlib.md5(raw).hexdigest() != row["content_key"]
        ):
            raise ValueError(f"M2 {fdid} does not match its receipt and frozen root")
        graph.expand_model(fdid, "m2", raw)


def select_asset(row, source_data, index_data, owners, identity, connection):
    fdid, kind = row["fdid"], row["type"]
    if kind not in ("m2", "wmo", "skin", "skel", "anim", "bone", "blp"):
        raise ValueError(f"Unsupported model closure asset {fdid}.{kind}")
    content = connection.execute(
        "SELECT content_key FROM resolution WHERE fdid = ?", (fdid,)
    ).fetchone()
    if content is None:
        raise ValueError(f"FDID {fdid}: absent from frozen resolution")
    content_key = bytes(content[0]).hex()
    candidates = [source_data / file["path"] for file in row["present_files"]]
    owner = owners.get((identity["product"], fdid, kind))
    if owner is not None:
        candidates.append(index_data / owner[1]["path"])
    if kind in ("m2", "skel") and (
        not row["present_files"]
        or hashlib.md5(candidates[0].read_bytes()).hexdigest() != content_key
    ):
        raise ValueError(f"FDID {fdid}: expanded bytes differ from frozen resolution")
    for candidate in candidates:
        if candidate.is_file():
            raw = candidate.read_bytes()
            if hashlib.md5(raw).hexdigest() == content_key:
                return {
                    "fdid": fdid,
                    "kind": kind,
                    "product": identity["product"],
                    "build_key": identity["build_key"],
                    "source_path": str(candidate),
                    "content_key": content_key,
                    "sha256": hashlib.sha256(raw).hexdigest(),
                    "bytes": len(raw),
                }
    raise ValueError(f"FDID {fdid}.{kind}: authenticated source bytes unavailable")


def stage_closure(
    source_data, index_data, roots, manifest, root, resolution, output, name
):
    if not name or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789-_" for c in name):
        raise ValueError("Invalid fragment name")
    if not roots or any(type(fdid) is not int or fdid <= 0 for fdid in roots):
        raise ValueError("Roots must be a nonempty array of positive FDIDs")
    if output.exists() or any(
        output.resolve().is_relative_to(path.resolve())
        for path in (source_data, index_data)
    ):
        raise ValueError("Output must be a new directory outside the input data roots")
    identity = load_snapshot(manifest, root, resolution)
    documents = publication.read_indexes(index_data)
    owners = publication.index_asset_receipts(documents)
    claimed_paths = {
        row["path"]
        for document in documents.values()
        for row in document["assets"] + document.get("aliases", [])
    }
    graph = RawByteGraph(
        source_data,
        {},
        identity["product"],
        identity["build"],
        path_cache=source_data / "local-listfile-cache.sqlite",
    )
    with closing(
        sqlite3.connect(resolution.resolve().as_uri() + "?mode=ro", uri=True)
    ) as connection:
        expand_owned_roots(
            graph, sorted(set(roots)), owners, index_data, identity, connection
        )
        result = graph.run()
        if result["unresolved"]:
            raise ValueError(f"Unresolved model closure: {result['unresolved']}")
        assets = [
            select_asset(row, source_data, index_data, owners, identity, connection)
            for row in result["assets"]
        ]
    staged = {"source_identity": identity, "assets": assets, "failures": []}
    verified = chains.prepare_verified_assets(identity, staged, resolution)
    allowed = {
        "product", "build", "build_key", "fdid", "kind", "path", "bytes", "sha256", "content_key"
    }
    pairs = [
        ({key: value for key, value in receipt.items() if key in allowed}, raw)
        for receipt, raw in verified.values()
    ]
    for receipt, raw in pairs:
        target = index_data / receipt["path"]
        if receipt["path"] in claimed_paths and not target.is_file():
            raise ValueError(f"Existing receipt payload missing: {receipt['path']}")
        if target.is_file() and target.read_bytes() != raw:
            raise ValueError(f"Existing payload conflicts: {receipt['path']}")
    fragment = publication.deduplicate_staged_fragment(
        index_data,
        {"version": 1, "assets": [row for row, _ in pairs], "aliases": [], "chains": []},
    )
    publication.validate_staged_fragment(index_data, fragment)
    output.mkdir(parents=True)
    chains.write_asset_files(output, pairs)
    path = output / "cache" / f"model-asset-index-{name}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(fragment, indent=2))
    (output / "frozen-manifest.json").write_text(json.dumps(staged, indent=2))
    (output / "graph.json").write_text(json.dumps(result, indent=2))
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in (
        "source-data", "index-data", "roots", "manifest", "root", "resolution", "output"
    ):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--name", required=True)
    args = parser.parse_args()
    path = stage_closure(
        args.source_data,
        args.index_data,
        json.loads(args.roots.read_text()),
        args.manifest,
        args.root,
        args.resolution,
        args.output,
        args.name,
    )
    print(f"STAGED {path} sha256={sha256(path)}")


if __name__ == "__main__":
    main()
