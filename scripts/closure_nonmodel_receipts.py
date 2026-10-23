"""Qualify selected native flat bytes and exported DB2s from existing receipts.

Generic byte receipts are provenance/closure-extract.jsonl, not ModelAssetIndex.
Exported tables use their own directory's receipt.json. Never extract, select an
alternate path/product, or promote hash-only UI inventories into source identity.
"""

from collections import defaultdict
import hashlib
import json
import struct


SOUND_KINDS = {"ogg", "mp3", "wav", "audio"}


def is_hex(value, size):
    return (
        isinstance(value, str)
        and len(value) == size
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def family(kind):
    if kind in SOUND_KINDS:
        return "sound"
    if kind in {"adt", "wmo"}:
        return kind
    return "texture" if kind in {"blp", "blob"} else None


def record_input(graph, path):
    relative = str(path.relative_to(graph.data))
    with path.open("rb") as stream:
        sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
    entry = graph.inputs.get(relative)
    if entry is not None and entry["sha256"] != sha256:
        entry.update(
            identity_status="unverified",
            source_product=None,
            actual_build=None,
            changed_sha256=sha256,
        )
        graph.issue(
            "receipt_input_changed",
            f"{relative}: input changed inside the audit cohort",
        )
        return None
    return graph.inputs.setdefault(
        relative, {"path": relative, "size": path.stat().st_size, "sha256": sha256}
    )


def verify_table(graph, path, name):
    relative = str(path.relative_to(graph.data))
    graph.db2_policy_exclusions.pop(relative, None)
    entry = record_input(graph, path)
    if entry is None:
        return False
    entry.update(identity_status="unverified", source_product=None, actual_build=None)
    receipt_path = path.parent / "receipt.json"
    if not receipt_path.is_file():
        graph.issue(
            "missing_metadata_receipt",
            f"{path.relative_to(graph.data)}: expected {receipt_path.relative_to(graph.data)}",
        )
        return False
    if record_input(graph, receipt_path) is None:
        return False
    document = json.loads(receipt_path.read_text())
    identity = document.get("identity", {})
    if identity.get("product") != graph.product:
        graph.issue(
            "metadata_product_mismatch",
            f"{path.relative_to(graph.data)}: requested {graph.product}, receipt {identity.get('product')!r}",
        )
        return False
    if (
        not is_hex(identity.get("build_key"), 32)
        or not isinstance(identity.get("build"), str)
        or not identity["build"]
    ):
        graph.issue(
            "metadata_receipt_mismatch",
            f"{path.relative_to(graph.data)}: invalid export build identity",
        )
        return False
    table = document.get("tables", {}).get(name)
    if not table or not identity.get("build") or not identity.get("build_key"):
        graph.issue(
            "missing_metadata_receipt",
            f"{path.relative_to(graph.data)}: no authenticated {name} table export receipt",
        )
        return False
    if entry["sha256"] != table.get("csv_sha256"):
        graph.issue(
            "metadata_receipt_mismatch",
            f"{path.relative_to(graph.data)}: CSV SHA256 differs from export receipt",
        )
        return False
    unreadable = table.get("unreadable_records", 0)
    if not isinstance(unreadable, int) or unreadable < 0:
        graph.issue(
            "metadata_receipt_mismatch",
            f"{path.relative_to(graph.data)}: invalid unreadable-record count",
        )
        return False
    if table.get("content_authenticated") is not True:
        if not table.get("readable_section_policy"):
            graph.issue(
                "missing_metadata_receipt",
                f"{relative}: no authenticated {name} table export receipt",
            )
            return False
        from closure_db2_policy import POLICY, verify_readable_sections

        try:
            proof = verify_readable_sections(graph, path, table, identity)
        except (ValueError, KeyError, OSError, struct.error) as error:
            graph.issue("db2_policy_refused", f"{relative}: {POLICY}: {error}")
            return False
        graph.db2_policy_exclusions[relative] = proof
        entry.update(
            source_product=identity["product"],
            actual_build=identity["build"],
            identity_status="authenticated_readable_sections",
            excluded_records=proof["excluded_records"],
            excluded_copy_records=proof["excluded_copy_records"],
            excluded_rows=proof["excluded_rows"],
            db2_policy=POLICY,
            export_receipt=str(receipt_path.relative_to(graph.data)),
        )
        return True
    entry.update(
        source_product=identity["product"],
        actual_build=identity["build"],
        identity_status="authenticated_partial_export"
        if unreadable
        else "verified_export",
        unreadable_records=unreadable,
        export_receipt=str(receipt_path.relative_to(graph.data)),
    )
    if unreadable:
        graph.issue(
            "metadata_unreadable_records",
            f"{path.relative_to(graph.data)}: {unreadable} records unavailable in authenticated export",
        )
    return True


def flat_assets(result):
    return [
        row
        for row in result["assets"]
        if not row["receipt_selected"] and family(row["type"])
    ]


def file_md5(data, relative, cache):
    if relative not in cache:
        with (data / relative).open("rb") as stream:
            cache[relative] = hashlib.file_digest(stream, "md5").hexdigest()
    return cache[relative]


def matches_payload(graph, asset, row, md5):
    actual = next(
        (item for item in asset["present_files"] if item["path"] == row["path"]), None
    )
    if actual is None or len({item["sha256"] for item in asset["present_files"]}) != 1:
        return False
    valid = actual["size"] == row.get("size") and actual["sha256"] == row.get("sha256")
    return valid and file_md5(graph.data, row["path"], md5) == row.get("content_key")


def requested_receipts(ledger, wanted):
    with ledger.open() as stream:
        for line_number, line in enumerate(stream, 1):
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(
                    f"{ledger}:{line_number}: invalid extraction receipt"
                ) from error
            key = (
                row.get("product"),
                row.get("fdid"),
                family(row.get("type")),
                row.get("path"),
            )
            for index in wanted.get(key, []):
                yield row, index


def valid_file_identity(row):
    return (
        isinstance(row.get("actual_build"), str)
        and bool(row["actual_build"])
        and is_hex(row.get("build_config"), 32)
        and is_hex(row.get("content_key"), 32)
        and row.get("disposition") in {"extracted", "verified_existing"}
    )


def qualify_flat_assets(graph, result):
    assets = flat_assets(result)
    if not assets:
        return
    wanted = defaultdict(list)
    matched = [False] * len(assets)
    verified = [set() for _ in assets]
    for index, asset in enumerate(assets):
        for relative in asset["locations"]:
            wanted[
                (
                    asset["requested_product"],
                    asset["fdid"],
                    family(asset["type"]),
                    relative,
                )
            ].append(index)
    ledger = graph.data / "provenance/closure-extract.jsonl"
    md5 = {}
    if ledger.is_file() and record_input(graph, ledger) is not None:
        for row, index in requested_receipts(ledger, wanted):
            matched[index] = True
            if valid_file_identity(row) and matches_payload(
                graph, assets[index], row, md5
            ):
                verified[index].add((row["actual_build"], row["build_config"]))
    for index, asset in enumerate(assets):
        proofs = sorted(verified[index])
        if proofs:
            builds = sorted({build for build, _ in proofs})
            asset.update(
                identity_status="verified_receipt",
                source_product=asset["requested_product"],
                actual_build=builds[0] if len(builds) == 1 else None,
                verified_builds=builds,
                file_receipt_build_keys=sorted({key for _, key in proofs}),
                receipt_family="closure-extract",
            )
        else:
            code = "file_receipt_mismatch" if matched[index] else "missing_file_receipt"
            graph.issue(
                code,
                f"{asset['requested_product']} {asset['fdid']}.{asset['type']}: actual flat paths {asset['locations']}; no matching size/SHA256/content-key source receipt",
                asset["fdid"],
            )
