#!/usr/bin/env python3
"""Lead-run immutable fragment publication; default dry-run never writes canonical.

Stop canonical writers before --apply. Payloads stage atomically before the merged
index; the existing DB2/assets transaction retains recovery data and rolls back
published files on failure. No install or CASC is consulted.
"""

import argparse
from contextlib import redirect_stdout
from dataclasses import dataclass
from copy import deepcopy
import hashlib
import io
import json
from pathlib import Path
import shutil

from apply_db2_install import apply, fingerprint
from apply_mount_model_assets import index_hashes
from closure_receipts import validate_receipt
from model_asset_publication import (
    deduplicate_staged_fragment,
    index_asset_receipts,
    read_indexes,
    receipt_key,
    validate_staged_fragment,
)


@dataclass
class PublicationPlan:
    report: dict
    payloads: list
    index_path: Path
    index_bytes: bytes | None
    index_inputs: dict


def checked_path(root, relative):
    path = root / relative
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"Path escapes data root: {path}")
    return path


def payload_identity(path):
    before = path.stat()
    sha, md5 = hashlib.sha256(), hashlib.md5()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            sha.update(block)
            md5.update(block)
    after = path.stat()
    if (before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (
        after.st_ino,
        after.st_size,
        after.st_mtime_ns,
        after.st_ctime_ns,
    ):
        raise ValueError(f"Payload changed while reading: {path}")
    return before.st_size, sha.hexdigest(), md5.hexdigest()


def receipt_identity(row):
    return row["bytes"], row["sha256"], row["content_key"]


def verify_sources(package, fragment, owners):
    paths = {}
    for alias, rows in [
        (False, fragment["assets"]),
        (True, fragment.get("aliases", [])),
    ]:
        for row in rows:
            validate_receipt(row, alias=alias)
            if alias:
                owner = owners.get(receipt_key(row))
                if owner is None or receipt_identity(owner) != receipt_identity(row):
                    raise ValueError(f"Alias differs from unique owner: {row['path']}")
            previous = paths.get(row["path"])
            if previous and receipt_identity(previous) != receipt_identity(row):
                raise ValueError(f"Staged path conflict: {row['path']}")
            if previous is None:
                source = checked_path(package, row["path"])
                if payload_identity(source) != receipt_identity(row):
                    raise ValueError(f"Source payload identity mismatch: {source}")
            paths[row["path"]] = row
    return paths


def plan_payloads(data, package, rows, documents):
    claimed = {}
    for document in documents.values():
        for row in document["assets"] + document.get("aliases", []):
            claimed.setdefault(row["path"], []).append(row)
    files, identical = [], 0
    for relative, row in sorted(rows.items()):
        destination = checked_path(data, relative)
        for previous in claimed.get(relative, []):
            if any(previous.get(k) != row[k] for k in ("bytes", "sha256")):
                raise ValueError(f"Canonical receipt path conflict: {destination}")
        if destination.exists() or destination.is_symlink():
            if not destination.is_file() or payload_identity(
                destination
            ) != receipt_identity(row):
                raise ValueError(f"Canonical path conflict: {destination}")
            identical += 1
            continue
        files.append(
            {
                "source": str(checked_path(package, relative)),
                "destination": str(destination),
                "before": None,
                "after": row["sha256"],
                "bytes": row["bytes"],
            }
        )
    return files, identical


def merge_fragment_index(documents, index_path, prepared):
    previous = documents.get(index_path)
    target = (
        deepcopy(previous)
        if previous is not None
        else {"version": 1, "assets": [], "aliases": [], "chains": []}
    )
    for field in ("assets", "aliases"):
        target.setdefault(field, []).extend(prepared.get(field, []))
    for row in prepared.get("chains", []):
        if row not in target.setdefault("chains", []):
            target["chains"].append(row)
    if target == previous or (
        previous is None
        and not any(target[field] for field in ("assets", "aliases", "chains"))
    ):
        return None
    index_asset_receipts({**documents, index_path: target})
    return (json.dumps(target, indent=2) + "\n").encode()


def plan_publication(data, package, fragment_relative, backup):
    data, package, backup = data.resolve(), package.resolve(), backup.resolve()
    if fragment_relative.parts != ("cache", fragment_relative.name) or not (
        fragment_relative.name.startswith("model-asset-index-")
        and fragment_relative.suffix == ".json"
    ):
        raise ValueError("Fragment must name cache/model-asset-index-<name>.json")
    if not data.is_dir() or backup.is_relative_to(package):
        raise ValueError(
            "Canonical data must exist and backup must be outside the source package"
        )
    inputs = index_hashes(data)
    documents = read_indexes(data)
    existing = index_asset_receipts(documents)
    fragment = json.loads(checked_path(package, fragment_relative).read_text())
    incoming = index_asset_receipts({fragment_relative: fragment})
    owners = {key: row for key, (_, row) in existing.items()}
    owners.update({key: row for key, (_, row) in incoming.items()})
    rows = verify_sources(package, fragment, owners)
    prepared = deduplicate_staged_fragment(data, fragment)
    validate_staged_fragment(data, prepared)
    payloads, identical = plan_payloads(data, package, rows, documents)
    index_path = checked_path(data, fragment_relative)
    index_raw = merge_fragment_index(documents, index_path, prepared)
    changed = index_raw is not None
    previous_bytes = index_path.stat().st_size if index_path.exists() and changed else 0
    index_files = []
    if changed:
        index_files.append(
            {
                "path": str(index_path),
                "bytes": len(index_raw),
                "before_sha256": inputs.get(index_path.name),
                "after_sha256": hashlib.sha256(index_raw).hexdigest(),
                "pre_backup": str(
                    backup
                    / (
                        f"pre-{index_path.name}"
                        if index_path.exists()
                        else f"pre-{index_path.name}.absent.json"
                    )
                ),
            }
        )
    manifest_estimate = len(
        json.dumps({"files": payloads, "indexes": index_files}).encode()
    )
    payload_bytes = sum(row["bytes"] for row in payloads)
    # Staged payloads, merged-index source+temporary, two retained old-index copies,
    # transaction manifests/recovery records, and small-file allocation overhead.
    estimate = (
        payload_bytes
        + 2 * len(index_raw or b"")
        + 2 * previous_bytes
        + 5 * manifest_estimate
        + 4096 * (len(payloads) + 1)
    )
    if index_hashes(data) != inputs:
        raise ValueError(
            "Current canonical indexes changed during preflight; rerun dry-run"
        )
    report = {
        "mode": "dry-run",
        "noop": not payloads and not changed,
        "canonical_data": str(data),
        "package": str(package),
        "backup": str(backup),
        "fragment": str(fragment_relative),
        "source_fragment_sha256": fingerprint(package / fragment_relative),
        "canonical_index_inputs": inputs,
        "new_owners": len(prepared["assets"]),
        "new_aliases": len(prepared.get("aliases", [])),
        "payload_files": payloads,
        "identical_payload_files": identical,
        "payload_bytes_to_copy": payload_bytes,
        "index_files": index_files,
        "canonical_write_paths": [row["destination"] for row in payloads]
        + [row["path"] for row in index_files],
        "space_required_bytes": estimate,
        "disk_free_bytes": shutil.disk_usage(data).free,
        "transaction_directory": str(backup / "transaction"),
        "boundary": "Lead-only apply; immutable owners retained; identical collisions no-op. No source relabeling or P4 claim.",
    }
    return PublicationPlan(report, payloads, index_path, index_raw, inputs)


def apply_publication(data, plan):
    report = {**plan.report, "mode": "apply"}
    if report["noop"]:
        return report
    if index_hashes(data) != plan.index_inputs:
        raise ValueError("Canonical index drift before apply; no writes applied")
    if shutil.disk_usage(data).free < report["space_required_bytes"]:
        raise ValueError("Insufficient free space for publication estimate")
    backup = Path(report["backup"])
    backup.mkdir(parents=True, exist_ok=False)
    files = list(plan.payloads)
    if plan.index_bytes is not None:
        entry = report["index_files"][0]
        saved = Path(entry["pre_backup"])
        if entry["before_sha256"] is None:
            saved.write_text(
                json.dumps({"path": str(plan.index_path), "previously_absent": True})
                + "\n"
            )
        else:
            shutil.copy2(plan.index_path, saved)
            if fingerprint(saved) != entry["before_sha256"]:
                raise ValueError(
                    "Index changed during pre-* backup; no publication applied"
                )
        merged = backup / f"merged-{plan.index_path.name}"
        merged.write_bytes(plan.index_bytes)
        files.append(
            {
                "source": str(merged),
                "destination": str(plan.index_path),
                "before": entry["before_sha256"],
                "after": entry["after_sha256"],
            }
        )
    manifest = backup / "planned-publication.json"
    manifest.write_text(json.dumps({"files": files, "links": []}, indent=2) + "\n")
    with redirect_stdout(io.StringIO()) as output:
        apply(manifest, backup / "transaction")
    report["transaction_result"] = output.getvalue().strip()
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canonical-data", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--fragment", type=Path, required=True)
    parser.add_argument("--backup", type=Path, required=True)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--dry-run", action="store_true", help="Default: read-only preflight"
    )
    mode.add_argument(
        "--apply", action="store_true", help="Lead only: atomic copies and merged index"
    )
    args = parser.parse_args()
    try:
        plan = plan_publication(
            args.canonical_data, args.package, args.fragment, args.backup
        )
        report = (
            apply_publication(args.canonical_data.resolve(), plan)
            if args.apply
            else plan.report
        )
    except (ValueError, OSError, KeyError) as error:
        parser.exit(1, f"Publication refused: {error}\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
