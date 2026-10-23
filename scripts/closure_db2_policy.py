"""Readable-section proof for pinned DB2s with demonstrably unavailable keys."""

import csv
import hashlib
import io
from pathlib import Path
import re
import struct

from closure_nonmodel_receipts import is_hex, record_input

POLICY = "DB2_ENCRYPTED_SECTION_KEY_UNAVAILABLE"


def read_proof_input(graph, relative, expected_sha256):
    relative = Path(relative)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValueError(f"proof input must remain in data: {relative}")
    path = graph.data / relative
    entry = record_input(graph, path)
    if entry is None or entry["sha256"] != expected_sha256:
        raise ValueError(f"proof input hash differs: {relative}")
    return path.read_bytes()


def read_census_input(graph, descriptor):
    return read_proof_input(graph, descriptor["path"], descriptor["sha256"])


def parse_external_key_ids(raw):
    keys = set()
    for line in raw.decode().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split()
        if len(fields) != 2 or not is_hex(fields[0], 16) or not is_hex(fields[1], 32):
            raise ValueError("invalid external TACT key row")
        keys.add(fields[0].upper())
    return keys


def parse_keyring_ids(raw):
    keys = set()
    for line in raw.decode().splitlines():
        if not line.startswith("key-"):
            continue
        name, value = line.split(" = ", 1)
        if not is_hex(name[4:], 16) or not is_hex(value, 32):
            raise ValueError("invalid keyring TACT key row")
        keys.add(bytes.fromhex(name[4:])[::-1].hex().upper())
    return keys


def read_key_census(graph, census, source):
    info = read_census_input(graph, census["build_info"])
    ring = read_census_input(graph, census["keyring"])
    external = read_census_input(graph, census["external"])
    rows = [
        {key.split("!")[0]: value for key, value in row.items()}
        for row in csv.DictReader(io.StringIO(info.decode()), delimiter="|")
    ]
    active = [
        row
        for row in rows
        if row["Product"] == source["product"] and row["Active"] == "1"
    ]
    if len(active) != 1:
        raise ValueError("key census has no unique active source product")
    row = active[0]
    if any(
        row[field] != census[key]
        for field, key in [
            ("Product", "product"),
            ("Version", "build"),
            ("Build Key", "build_key"),
        ]
    ):
        raise ValueError("key census identity differs from its build-info snapshot")
    if row["Version"].split(".")[:-1] != source["build"].split(".")[:-1] or int(
        row["Version"].split(".")[-1]
    ) < int(source["build"].split(".")[-1]):
        raise ValueError("key census must be the same or later source build")
    if hashlib.md5(ring).hexdigest() != row["KeyRing"]:
        raise ValueError("keyring bytes differ from build-info content key")
    return parse_keyring_ids(ring) | parse_external_key_ids(external)


def prove_missing_key(table, section, available_keys):
    key = section["key_id"]
    if key in available_keys:
        raise ValueError(
            f"section {section['index']} key {key} is available; decrypting re-export required"
        )
    warnings = "\n".join(table.get("extraction_warnings", []))
    chunks = [
        (int(size), int(offset))
        for size, offset in re.findall(
            key + r" \(chunk \d+, (\d+) bytes at offset (\d+)\)", warnings
        )
    ]
    if not any(
        offset == section["start"] and offset + size >= section["end"]
        for size, offset in chunks
    ):
        raise ValueError(
            f"section {section['index']} key {key} has no matching encrypted-chunk evidence"
        )


def read_excluded_sections(raw, table, available_keys):
    # WDC5 metadata offsets/Q8I section layout match game-server db2_casc.read_header.
    # This inspects encryption boundaries only; the existing decoder owns row export.
    if raw[:4] != b"WDC5" or len(raw) < 204:
        raise ValueError("expected a complete pinned WDC5 header")
    record_size = struct.unpack_from("<I", raw, 144)[0]
    layout = struct.unpack_from("<I", raw, 156)[0]
    flags = struct.unpack_from("<H", raw, 172)[0]
    section_count = struct.unpack_from("<I", raw, 200)[0]
    if f"{layout:08X}" != table["layout"]:
        raise ValueError("pinned DB2 layout differs from its export receipt")
    if flags not in {0, 4, 5, 7}:
        raise ValueError(f"unsupported DB2 section flags {flags:#x}")
    excluded = []
    for index in range(section_count):
        key, start, count, _, end, *_ = struct.unpack_from(
            "<Q8I", raw, 204 + index * 40
        )
        if flags in {0, 4}:
            end = start + count * record_size
        if not 0 <= start <= end <= len(raw):
            raise ValueError(f"section {index} record span exceeds pinned bytes")
        if not count or any(raw[start:end]):
            continue
        if not key or end == start:
            raise ValueError(
                f"section {index} has unreadable bytes without proven encryption"
            )
        section = {
            "index": index,
            "key_id": f"{key:016X}",
            "records": count,
            "start": start,
            "end": end,
        }
        prove_missing_key(table, section, available_keys)
        excluded.append(
            {name: section[name] for name in ["index", "key_id", "records"]}
        )
    count = sum(section["records"] for section in excluded)
    if not excluded or count != table["unreadable_records"]:
        raise ValueError(
            "not every unreadable export row has unavailable-key section proof"
        )
    return excluded


def verify_readable_sections(graph, path, table, source):
    policy = table["readable_section_policy"]
    if policy["name"] != POLICY:
        raise ValueError(f"unknown policy {policy['name']}")
    raw = read_proof_input(graph, policy["raw_path"], table["db2_sha256"])
    available = read_key_census(graph, policy["key_census"], source)
    sections = read_excluded_sections(raw, table, available)
    return {
        "path": str(path.relative_to(graph.data)),
        "policy": POLICY,
        "source_product": source["product"],
        "source_build": source["build"],
        "key_census_build": policy["key_census"]["build"],
        "key_availability_premise": "monotonic pushed-key availability; disclosed later census, not source-build substitution",
        "excluded_records": sum(section["records"] for section in sections),
        "sections": sections,
    }
