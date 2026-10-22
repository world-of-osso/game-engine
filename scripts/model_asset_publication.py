"""Plan one receipt truth, then publish bytes and all owning indexes together."""

import hashlib
import json
import shutil
from pathlib import Path
import tempfile

from apply_db2_install import apply, fingerprint


def receipt_key(row):
    return row["product"], row["fdid"], row["kind"]


def relative_asset_path(row):
    return Path(row["path"]).relative_to(
        Path("products") / row["product"] / row["build_key"]
    )


def validate_refresh(previous, incoming):
    if previous["build_key"] == incoming["build_key"]:
        if any(
            previous[field] != incoming[field]
            for field in ["path", "bytes", "sha256", "content_key"]
        ):
            raise ValueError(f"conflicting existing receipt: {receipt_key(incoming)}")
    elif int(incoming["build"].rsplit(".", 1)[-1]) <= int(
        previous["build"].rsplit(".", 1)[-1]
    ):
        raise ValueError(
            f"stale asset receipt: {incoming['build']} cannot supersede {previous['build']}"
        )


def read_indexes(data):
    cache = data / "cache"
    documents = {}
    for path in [
        cache / "model-asset-index.json",
        *sorted(cache.glob("model-asset-index-*.json")),
    ]:
        if path.exists():
            document = json.loads(path.read_text())
            if document["version"] != 1:
                raise ValueError(
                    f"Unsupported model asset index version {document['version']}"
                )
            documents[path] = document
    return documents


def index_asset_receipts(documents):
    records = {}
    for path, document in documents.items():
        if document["version"] != 1:
            raise ValueError(f"Unsupported model asset index version in {path}")
        for row in document["assets"]:
            identity = receipt_key(row)
            if identity in records:
                raise ValueError(f"Duplicate asset receipt {identity} in {path}")
            records[identity] = path, row
    return records


def alias_metadata_key(row):
    return (
        row["product"],
        row["build_key"],
        relative_asset_path(row),
        row["fdid"],
        row["kind"],
        row["bytes"],
        row["sha256"],
    )


def existing_alias_metadata(documents):
    # Alias rows are provenance metadata, not ModelAssetIndex's unique asset map.
    return {
        alias_metadata_key(row)
        for document in documents.values()
        for row in document.get("aliases", [])
    }


def deduplicate_staged_fragment(data, fragment):
    """Keep one owner and preserve required namespaces as authenticated aliases."""
    documents = read_indexes(data)
    existing = index_asset_receipts(documents)
    incoming = index_asset_receipts({Path("<staged>"): fragment})
    assets = []
    relocated = []
    for identity, (_, row) in incoming.items():
        owner = existing.get(identity)
        if owner is None:
            assets.append(row)
            continue
        path, previous = owner
        if any(
            previous[name] != row[name]
            for name in ("fdid", "kind", "bytes", "sha256", "content_key")
        ):
            raise ValueError(
                f"conflicting staged receipt {identity} already owned by {path}"
            )
        if previous["path"] != row["path"]:
            relocated.append(row)
    known_aliases = existing_alias_metadata(documents)
    aliases = []
    for row in fragment.get("aliases", []) + relocated:
        key = alias_metadata_key(row)
        if key not in known_aliases:
            aliases.append(row)
            known_aliases.add(key)
    return {**fragment, "assets": assets, "aliases": aliases}


def validate_staged_fragment(data, fragment):
    """Fail if this fragment would make the canonical receipt union ambiguous."""
    documents = read_indexes(data)
    index_asset_receipts({**documents, Path("<staged>"): fragment})
    aliases = existing_alias_metadata(documents)
    for row in fragment.get("aliases", []):
        key = alias_metadata_key(row)
        if key in aliases:
            raise ValueError(f"Duplicate alias receipt {key}")
        aliases.add(key)


def plan_assets(documents, target, verified):
    owners = {}
    for path, document in documents.items():
        for position, row in enumerate(document["assets"]):
            key = receipt_key(row)
            if key in owners:
                raise ValueError(f"Duplicate existing receipt: {key}")
            owners[key] = path, position, row
    for incoming, _ in verified:
        owner = owners.get(receipt_key(incoming))
        if owner is None:
            target["assets"].append(incoming)
        else:
            path, position, previous = owner
            validate_refresh(previous, incoming)
            documents[path]["assets"][position] = incoming


def plan_aliases(documents, target, verified, aliases):
    assets = {receipt_key(row): (row, raw) for row, raw in verified}
    incoming = {
        (row["product"], relative_asset_path(row)): (row, raw) for row, raw in aliases
    }
    consumed, refreshed = set(), []
    for document in documents.values():
        for position, previous in enumerate(document.get("aliases", [])):
            key = previous["product"], relative_asset_path(previous)
            replacement = incoming.get(key)
            if replacement is None and receipt_key(previous) in assets:
                asset, raw = assets[receipt_key(previous)]
                replacement = (
                    {
                        **asset,
                        "path": str(
                            Path("products")
                            / asset["product"]
                            / asset["build_key"]
                            / key[1]
                        ),
                    },
                    raw,
                )
            if replacement is None:
                continue
            receipt, raw = replacement
            validate_refresh(previous, receipt)
            document["aliases"][position] = receipt
            consumed.add(key)
            refreshed.append((receipt, raw))
    for key, pair in incoming.items():
        if key not in consumed:
            target["aliases"].append(pair[0])
            refreshed.append(pair)
    return refreshed


def merge_chain(document, identity, staged):
    namespace = identity["product"], identity["build_key"]
    chains = document.get("chains", [])
    previous = next(
        (row for row in chains if (row["product"], row["build_key"]) == namespace), {}
    )
    chain = {
        **identity,
        "dependencies": {**previous.get("dependencies", {}), **staged["dependencies"]},
        "metadata": [*previous.get("metadata", []), *staged.get("metadata", [])],
    }
    document["chains"] = [
        row for row in chains if (row["product"], row["build_key"]) != namespace
    ] + [chain]


def publish_documents(data, documents, payloads):
    directory = tempfile.mkdtemp(prefix=".model-asset-publication-", dir=data.parent)
    stage = Path(directory)
    try:
        files, destinations = [], {}

        def add(destination, raw):
            digest = hashlib.sha256(raw).hexdigest()
            if destination in destinations:
                if destinations[destination] != digest:
                    raise ValueError(f"conflicting staged asset: {destination}")
                return
            destinations[destination] = digest
            source = stage / str(len(files))
            source.write_bytes(raw)
            files.append(
                {
                    "source": str(source),
                    "destination": str(destination),
                    "before": fingerprint(destination),
                    "after": digest,
                }
            )

        for receipt, raw in payloads:
            path = data / receipt["path"]
            if path.exists() and path.read_bytes() != raw:
                raise ValueError(f"conflicting existing asset: {path}")
            add(path, raw)
        for path, document in documents.items():
            add(path, (json.dumps(document, indent=2) + "\n").encode())
        manifest = stage / "manifest.json"
        manifest.write_text(json.dumps({"files": files, "links": []}))
        apply(manifest, stage / "backup")
    except Exception as error:
        error.add_note(f"Publication staging/recovery retained at {stage}")
        raise
    else:
        shutil.rmtree(stage)


def publish_receipts(data, identity, staged, verified, aliases, fragment=None):
    verified, aliases = list(verified), list(aliases)
    documents = read_indexes(data)
    name = (
        "model-asset-index.json"
        if fragment is None
        else f"model-asset-index-{fragment}.json"
    )
    path = data / "cache" / name
    target = documents.setdefault(
        path, {"version": 1, "assets": [], "aliases": [], "chains": []}
    )
    plan_assets(documents, target, verified)
    alias_payloads = plan_aliases(documents, target, verified, aliases)
    merge_chain(target, identity, staged)
    target["assets"].sort(key=receipt_key)
    publish_documents(data, documents, [*verified, *alias_payloads])
    return target
