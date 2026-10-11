#!/usr/bin/env python3
"""Record-scoped local-CASC NPC metadata authentication; no missing-key recovery.

A complete profile also needs proof of child membership. An encrypted relationship
block with unknown parents cannot prove either relevance or irrelevance to a display.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

import closure_casc as casc
import import_npc_appearance as npc
from verify_legacy_product_assets import authenticate_root

TABLES = {"extra": 1264997, "option": 3692043, "geoset": 1720141}


def range_chunks(receipt, start, size):
    end = start + size
    if start < 0 or size < 0 or end > receipt["decoded_bytes"]:
        raise ValueError("record dependency outside authenticated BLTE bounds")
    return (
        {
            c["index"]
            for c in receipt["chunks"]
            if c["decoded_start"] < end and start < c["decoded_end"]
        }
        if size
        else set()
    )


def authenticate_ranges(receipt, ranges):
    chunks = set()
    for start, size in ranges:
        chunks.update(range_chunks(receipt, start, size))
    return {
        "chunks": sorted(chunks),
        "authenticated": all(
            receipt["chunks"][i]["authenticated_plaintext"] for i in chunks
        ),
    }


def map_table(data, kind, receipt, *, wanted=None, parents=None):
    if not authenticate_ranges(receipt, [(0, 204)])["authenticated"]:
        raise ValueError("WDC5 header is not authenticated plaintext")
    record_size = npc.unpack(data, "I", 144)[0]
    count = npc.unpack(data, "I", 140)[0]
    common, palette, sections = npc.unpack(data, "3I", 192)
    fields, metadata_end = npc.read_fields(
        data, count, sections, record_size, palette, common
    )
    keyed_counts = [
        npc.unpack(data, "Q8I", 204 + s * 40)[2]
        for s in range(sections)
        if npc.unpack(data, "Q", 204 + s * 40)[0]
    ]
    directory_end = metadata_end + sum(4 + n * 4 for n in keyed_counts)
    metadata = authenticate_ranges(receipt, [(0, directory_end)])
    if not metadata["authenticated"]:
        raise ValueError(
            "WDC5 field/palette/common/encrypted-ID metadata unauthenticated"
        )
    omitted = []
    rows = npc.read_wdc5(data, kind, skipped=omitted)
    selected = {
        rid
        for rid, (_, parent) in rows.items()
        if (wanted is None or rid in wanted) and (parents is None or parent in parents)
    }
    records, section_map, blocked = {}, [], []
    omitted_by_section = {s["section"]: s for s in omitted}
    for section in range(sections):
        key, start, n, strings, sparse, ids, relation, offsets, copies = npc.unpack(
            data, "Q8I", 204 + section * 40
        )
        id_start = start + n * record_size + strings
        copy_start = id_start + ids
        relation_start = copy_start + copies * 8
        size = n * record_size + strings + ids + copies * 8 + relation
        section_proof = authenticate_ranges(receipt, [(start, size)])
        section_map.append(
            dict(
                section=section,
                start=start,
                end=start + size,
                record_count=n,
                key_name=f"{key:016X}" if key else None,
                **section_proof,
            )
        )
        if section in omitted_by_section:
            omission = omitted_by_section[section]
            parent_ids = None
            if (
                kind != "extra"
                and authenticate_ranges(receipt, [(relation_start, relation)])[
                    "authenticated"
                ]
            ):
                parent_ids = sorted(
                    set(npc.read_relations(data, relation_start, relation, n).values())
                )
            blocked.append(dict(omission, parents=parent_ids))
            continue
        for i in range(n):
            raw = int.from_bytes(
                npc.block(data, start + i * record_size, record_size), "little"
            )
            rid = (
                npc.unpack(data, "I", id_start + i * 4)[0]
                if ids
                else npc.decode_field(raw, fields[0], 0)
            )
            if rid not in selected:
                continue
            ranges = [(0, directory_end), (start + i * record_size, record_size)]
            if ids:
                ranges.extend([(id_start + i * 4, 4), (relation_start, relation)])
            records[rid] = dict(
                section=section, ranges=ranges, **authenticate_ranges(receipt, ranges)
            )
        for rid, source in struct.iter_unpack(
            "<II", npc.block(data, copy_start, copies * 8)
        ):
            if rid in selected:
                # Copy semantics can transitively depend on another section. Only a
                # fully authenticated table proves this bounded import's copy chain.
                proof = authenticate_ranges(receipt, [(0, len(data))])
                records[rid] = dict(
                    section=section,
                    copy_source=source,
                    ranges=[(0, len(data))],
                    **proof,
                )
    if selected - records.keys():
        raise ValueError("selected WDC5 record has no byte dependency proof")
    return {
        "rows": {rid: rows[rid] for rid in sorted(selected)},
        "records": records,
        "sections": section_map,
        "blocked_sections": blocked,
    }


def certify_profile(display, extra, tables):
    required = {
        "extra": [extra] if extra else [],
        "option": sorted(
            rid
            for rid, (_, parent) in tables["option"]["rows"].items()
            if extra and parent == extra
        ),
        "geoset": sorted(
            rid
            for rid, (_, parent) in tables["geoset"]["rows"].items()
            if parent == display
        ),
    }
    gaps = []
    for kind, ids in required.items():
        for rid in ids:
            if not tables[kind]["records"].get(rid, {}).get("authenticated", False):
                gaps.append(
                    {
                        "table": kind,
                        "record_ids": [rid],
                        "known_required": True,
                        "reason": "required_record_not_authenticated",
                    }
                )
    for kind, parent in [("option", extra), ("geoset", display)]:
        if kind == "option" and not extra:
            continue
        for section in tables[kind]["blocked_sections"]:
            parents = section["parents"]
            if parents is not None and parent not in parents:
                continue
            gaps.append(
                {
                    "table": kind,
                    "section": section["section"],
                    "record_ids": section["record_ids"],
                    "known_required": parents is not None,
                    "reason": "encrypted_parent_membership_unknown"
                    if parents is None
                    else "required_encrypted_section",
                }
            )
    status = (
        "missing_required_records"
        if any(g["known_required"] for g in gaps)
        else "membership_unresolved"
        if gaps
        else "certified"
    )
    return {
        "display": display,
        "extra_id": extra,
        "status": status,
        "required_records": required,
        "gaps": gaps,
    }


def read_local_table(install, indices, encoding, root_keys, fdid):
    content_key = root_keys[fdid]
    size, keys = casc.encoding_keys(encoding, {content_key})[content_key]
    key = next((k for k in keys if k[:9] in indices), None)
    if key is None:
        raise ValueError(f"FDID {fdid}: no local encoding")
    archive, offset, length = indices[key[:9]]
    with (install / "Data/data" / f"data.{archive:03d}").open("rb") as stream:
        stream.seek(offset)
        raw = stream.read(length)
    if len(raw) != length:
        raise ValueError("truncated local archive")
    data, receipt = casc.authenticate_blte_chunks(raw[30:], key.hex(), size)
    complete = all(c["authenticated_plaintext"] for c in receipt["chunks"])
    if complete and hashlib.md5(data).digest() != content_key:
        raise ValueError("fully readable DB2 content key mismatch")
    receipt.update(
        fdid=fdid,
        root_content_key=content_key.hex(),
        decoded_sha256=hashlib.sha256(data).hexdigest(),
        full_content_verified=complete,
        archive=archive,
        archive_offset=offset,
    )
    return data, receipt


def certify(args):
    args.output.mkdir(parents=True, exist_ok=True)
    indices, _, _ = casc.load_indices(args.install)
    product = next(
        p for p in casc.installed_products(args.install) if p["Product"] == "wow"
    )
    identity, roots = authenticate_root(
        args.install, product, set(TABLES.values()), indices, args.output
    )
    build = identity["build_key"]
    config = (args.install / "Data/config" / build[:2] / build[2:4] / build).read_text()
    config = dict(line.split(" = ", 1) for line in config.splitlines() if " = " in line)
    ck, ek = config["encoding"].split()
    encoding = casc.read_metadata(args.install, indices, ek, ck)
    displays = {
        int(row["ID"]): int(row["ExtendedDisplayInfoID"])
        for row in npc.read_csv(args.data / "CreatureDisplayInfo.csv")
        if int(row["ID"]) in args.display_id
    }
    if set(args.display_id) - displays.keys():
        raise ValueError("requested display absent from CreatureDisplayInfo inputs")
    extras = {extra for extra in displays.values() if extra}
    tables, receipts = {}, {}
    for kind, fdid in TABLES.items():
        data, receipt = read_local_table(args.install, indices, encoding, roots, fdid)
        (args.output / f"{fdid}.db2").write_bytes(data)
        table = map_table(
            data,
            kind,
            receipt,
            wanted=extras if kind == "extra" else None,
            parents=extras
            if kind == "option"
            else set(displays)
            if kind == "geoset"
            else None,
        )
        tables[kind] = table
        receipts[kind] = dict(receipt, **table)
    profiles = [
        certify_profile(display, extra, tables)
        for display, extra in sorted(displays.items())
    ]
    report = {
        "version": 2,
        "scope": "NPC required DB2 records and child-membership completeness",
        "identity": identity,
        "input_display_csv_sha256": hashlib.sha256(
            (args.data / "CreatureDisplayInfo.csv").read_bytes()
        ).hexdigest(),
        "input_display_csv_authenticated": False,
        "status": "PASS"
        if all(p["status"] == "certified" for p in profiles)
        else "GAPS",
        "tables": receipts,
        "profiles": profiles,
        "limits": [
            "Display/texture/model joins are existing cache inputs, not authenticated DB2 record receipts.",
            "Unknown encrypted child parents do not prove that any requested display needs those records.",
        ],
    }
    (args.output / "certification.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def publish_cache(cache, rows, profiles, backup):
    """Publish positive records, but add coverage only for complete profiles."""
    cache, backup = Path(cache), Path(backup)
    before = cache.read_bytes()
    eligible = {
        p["display"] for p in profiles if p["status"] != "missing_required_records"
    }
    complete = {p["display"] for p in profiles if p["status"] == "certified"}
    imported = tuple(
        [row for row in table if row[0] in (complete if i == 3 else eligible)]
        for i, table in enumerate(rows)
    )
    merged = npc.merge_existing_rows(cache, imported)
    staged = cache.with_name(cache.name + ".chunk-staged")
    npc.write_database(staged, merged)
    try:
        with backup.open("xb") as stream:
            stream.write(before)
        if cache.read_bytes() != before:
            raise ValueError("NPC cache changed during backed-up publication")
        staged.replace(cache)
    finally:
        if staged.exists():
            staged.unlink()
    return {
        "path": str(cache),
        "backup": str(backup),
        "before_sha256": hashlib.sha256(before).hexdigest(),
        "after_sha256": hashlib.sha256(cache.read_bytes()).hexdigest(),
        "imported_counts": dict(
            zip(
                ("appearances", "choices", "geosets", "display_coverage"),
                map(len, imported),
            )
        ),
        "coverage_added_only_for": sorted(complete),
    }


def apply_report(args, report):
    tables = report["tables"]
    eligible = {
        p["display"]: p["extra_id"]
        for p in report["profiles"]
        if p["status"] != "missing_required_records"
    }
    models = npc.load_model_paths(
        args.data / "cache/creature_display.sqlite",
        args.data / "community-listfile.csv",
        eligible,
    )
    extras = tables["extra"]["rows"]
    materials = {
        npc.select_material(extras[e][0], models[d]) for d, e in eligible.items() if e
    }
    textures = npc.load_textures(
        args.data / "TextureFileData.csv",
        args.data / "cache/outfit_links.sqlite",
        materials - {0},
    )
    rows = npc.join_appearances(
        eligible,
        extras,
        tables["option"]["rows"],
        tables["geoset"]["rows"],
        models,
        textures,
    )
    report["publication"] = publish_cache(
        args.apply_cache, rows, report["profiles"], args.backup
    )
    (args.output / "certification.json").write_text(json.dumps(report, indent=2) + "\n")
    # The certificate accompanies the actual installed cache, not a staging cache.
    sidecar = args.apply_cache.with_name("npc-appearance-authentication.json")
    if sidecar.exists():
        with (args.output / "previous-npc-appearance-authentication.json").open(
            "xb"
        ) as stream:
            stream.write(sidecar.read_bytes())
    staged = sidecar.with_name(sidecar.name + ".chunk-staged")
    with staged.open("x") as stream:
        stream.write(json.dumps(report, indent=2) + "\n")
    staged.replace(sidecar)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--install", type=Path, required=True)
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--display-id", type=int, action="append", required=True)
    parser.add_argument(
        "--apply-cache", type=Path, help="Back up and replace this canonical NPC cache"
    )
    parser.add_argument(
        "--backup",
        type=Path,
        help="New exclusive backup path; required with --apply-cache",
    )
    args = parser.parse_args()
    if bool(args.apply_cache) != bool(args.backup):
        parser.error("--apply-cache and --backup must be specified together")
    report = certify(args)
    if args.apply_cache:
        apply_report(args, report)
    print(
        json.dumps(
            {
                "status": report["status"],
                "profiles": len(report["profiles"]),
                "certified": sum(
                    p["status"] == "certified" for p in report["profiles"]
                ),
            }
        )
    )


if __name__ == "__main__":
    main()
