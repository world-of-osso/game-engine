#!/usr/bin/env python3
"""Stage immutable model owners from exact historical extraction receipts.

No install, extraction, source writes, owner refresh, or generic auditor fallback.
The request document names each source identity and actual companion destination.
"""

import argparse
import hashlib
import json
from pathlib import Path

from apply_mount_model_assets import index_hashes
from closure_nonmodel_receipts import is_hex, valid_file_identity
from closure_receipts import PRODUCTS, validate_receipt
from import_forever_skyborne import validate_magic
from import_model_asset_chains import create_asset_receipt, write_asset_files
from model_asset_publication import (
    index_asset_receipts,
    read_indexes,
    validate_staged_fragment,
)

FINDING = "HISTORICAL_ASSET_SOURCE_BUILD_MISMATCH"


def request_key(request):
    return (
        request["product"],
        request["fdid"],
        request["kind"],
        request["source_path"],
        request["source_build"],
        request["source_build_key"],
    )


def validate_request(request, metadata_build):
    product, fdid, kind, source, build, key = request_key(request)
    if (
        product not in PRODUCTS
        or kind not in {"m2", "blp"}
        or not isinstance(fdid, int)
        or fdid <= 0
    ):
        raise ValueError(
            f"Unsupported recovery source identity: {product} {fdid}.{kind}"
        )
    directory = "textures" if kind == "blp" else "models"
    if source != f"{directory}/{fdid}.{kind}" or not build or not is_hex(key, 32):
        raise ValueError(f"Invalid exact source receipt identity: {source}")
    if build == metadata_build:
        raise ValueError("Historical recovery requires a distinct actual source build")
    destination = request["requested_path"]
    if destination is None:
        return
    parts = Path(destination).parts
    if (
        kind != "blp"
        or len(parts) != 5
        or parts[:2] != ("products", product)
        or not is_hex(parts[2], 32)
        or parts[3:] != ("textures", f"{fdid}.blp")
    ):
        raise ValueError(f"Invalid requested companion namespace: {destination}")


def select_source_receipts(data, requests, expected_sha256):
    wanted = {request_key(request) for request in requests}
    selected = {}
    digest = hashlib.sha256()
    ledger = data / "provenance/closure-extract.jsonl"
    with ledger.open("rb") as stream:
        for line_number, line in enumerate(stream, 1):
            digest.update(line)
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(
                    f"Malformed extraction ledger line {line_number}"
                ) from error
            key = (
                row.get("product"),
                row.get("fdid"),
                row.get("type"),
                row.get("path"),
                row.get("actual_build"),
                row.get("build_config"),
            )
            if key not in wanted or not valid_file_identity(row):
                continue
            previous = selected.get(key)
            if previous and any(
                previous.get(field) != row.get(field)
                for field in ("size", "sha256", "content_key")
            ):
                raise ValueError(f"Conflicting source receipts: {key}")
            selected[key] = row
    if digest.hexdigest() != expected_sha256:
        raise ValueError("Extraction ledger differs from the pinned request document")
    missing = wanted - selected.keys()
    if missing:
        raise ValueError(
            f"No authenticated same-product/kind/path source receipt: {sorted(missing)[:5]}"
        )
    return selected


def make_owner(row, raw):
    actual = (len(raw), hashlib.sha256(raw).hexdigest(), hashlib.md5(raw).hexdigest())
    if actual != (row.get("size"), row.get("sha256"), row.get("content_key")):
        raise ValueError(
            f"Source payload size/SHA256/content-key mismatch: {row['path']}"
        )
    kind = row["type"]
    validate_magic(raw, kind, row["content_key"])
    identity = {
        "product": row["product"],
        "build": row["actual_build"],
        "build_key": row["build_config"],
    }
    directory = "textures" if kind == "blp" else "models"
    owner = create_asset_receipt(
        identity,
        row["fdid"],
        kind,
        f"{directory}/{row['fdid']}.{kind}",
        raw,
        row["content_key"],
    )
    validate_receipt(owner)
    return owner


def make_alias(owner, destination):
    alias = {
        **owner,
        "path": destination,
        "build_key": Path(destination).parts[2],
        "source_build": owner["build"],
        "source_build_key": owner["build_key"],
    }
    validate_receipt(alias, alias=True)
    return alias


def historical_finding(request, owner, metadata_build):
    return {
        "code": FINDING,
        "product": owner["product"],
        "fdid": owner["fdid"],
        "kind": owner["kind"],
        "metadata_build": metadata_build,
        "actual_build": owner["build"],
        "actual_build_key": owner["build_key"],
        "source_path": request["source_path"],
        "owner_path": owner["path"],
        "alias_destination": request["requested_path"],
        "bytes": owner["bytes"],
        "sha256": owner["sha256"],
        "content_key": owner["content_key"],
    }


def stage(data, output, document):
    data, output = Path(data), Path(output)
    if output.resolve().is_relative_to(data.resolve()):
        raise ValueError(
            "Private output must be outside the read-only source data tree"
        )
    metadata_build, name = document["metadata_build"], document["fragment_name"]
    if (
        not metadata_build
        or not name
        or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789-_" for c in name)
    ):
        raise ValueError("Explicit metadata build and safe fragment name are required")
    if not is_hex(document["ledger_sha256"], 64):
        raise ValueError("A pinned extraction ledger SHA256 is required")
    requests = document["assets"]
    if not requests:
        raise ValueError("No explicit recovery requests")
    existing = index_asset_receipts(read_indexes(data))
    graph_identities = set()
    for request in requests:
        validate_request(request, metadata_build)
        identity = request["product"], request["fdid"], request["kind"]
        graph_identity = (*identity, request["requested_path"])
        if graph_identity in graph_identities:
            raise ValueError(f"Duplicate recovery graph identity: {graph_identity}")
        graph_identities.add(graph_identity)
        if identity in existing:
            raise ValueError(
                f"Existing immutable owner cannot be refreshed: {identity}"
            )
    before = index_hashes(data)
    selected = select_source_receipts(data, requests, document["ledger_sha256"])
    output.mkdir(parents=True)
    owners, aliases, by_source = {}, {}, {}
    for request in requests:
        by_source.setdefault(request_key(request), []).append(request)
    payload_bytes = 0
    for key, group in by_source.items():
        row = selected[key]
        raw = (data / row["path"]).read_bytes()
        owner = make_owner(row, raw)
        identity = owner["product"], owner["fdid"], owner["kind"]
        previous = owners.get(identity)
        if previous and previous != owner:
            raise ValueError(f"Conflicting immutable source owner: {identity}")
        if previous is None:
            write_asset_files(output, [(owner, raw)])
            owners[identity] = owner
            payload_bytes += len(raw)
        for request in group:
            destination = request["requested_path"]
            if destination is None or destination == owner["path"]:
                continue
            alias = make_alias(owner, destination)
            previous_alias = aliases.get(destination)
            if previous_alias and previous_alias != alias:
                raise ValueError(f"Conflicting companion namespace: {destination}")
            if previous_alias is None:
                write_asset_files(output, [(alias, raw)])
                aliases[destination] = alias
                payload_bytes += len(raw)
    fragment = {
        "version": 1,
        "assets": [owners[key] for key in sorted(owners)],
        "aliases": [aliases[path] for path in sorted(aliases)],
        "chains": [],
    }
    validate_staged_fragment(data, fragment)
    if index_hashes(data) != before:
        raise ValueError(
            "Source receipt indexes changed while staging; package remains unpublished"
        )
    proof = output / "_proof"
    proof.mkdir()
    with (proof / "historical-source-findings.jsonl").open("x") as stream:
        for request in requests:
            owner = owners[request["product"], request["fdid"], request["kind"]]
            stream.write(
                json.dumps(
                    historical_finding(request, owner, metadata_build), sort_keys=True
                )
                + "\n"
            )
    (proof / "inputs.json").write_text(
        json.dumps(
            {
                "metadata_build": metadata_build,
                "ledger_sha256": document["ledger_sha256"],
                "source_index_hashes": before,
                "request_document_sha256": hashlib.sha256(
                    json.dumps(document, sort_keys=True).encode()
                ).hexdigest(),
            },
            indent=2,
        )
        + "\n"
    )
    cache = output / "cache"
    cache.mkdir()
    filename = f"model-asset-index-{name}.json"
    (cache / filename).write_text(json.dumps(fragment, indent=2) + "\n")
    result = {
        "owners": len(owners),
        "aliases": len(aliases),
        "findings": len(requests),
        "payload_bytes": payload_bytes,
        "fragment": filename,
        "boundary": "Private immutable source-qualified owners/aliases only; historical source identity retained,not net graph closure or P4. Canonical publication belongs to lead.",
    }
    (proof / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--requests", type=Path, required=True)
    args = parser.parse_args()
    result = stage(args.data, args.output, json.loads(args.requests.read_text()))
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
