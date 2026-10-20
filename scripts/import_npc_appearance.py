#!/usr/bin/env python3
"""Import authored NPC appearance data from local fixed-record WDC5 tables."""

import argparse
import contextlib
import csv
import json
import sqlite3
import sys
import struct
from pathlib import Path

# Callers also load this module by path; resolve its sibling helper from here.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from server_db2 import db2_casc

# WoWDBDefs: only the three locally observed layouts, not a general DB2 reader.
LAYOUTS = {
    "extra": (0x4D9FE25C, 7, False),
    "option": (0x2F331C33, 2, True),
    "geoset": (0x5E539080, 2, True),
}


def unpack(data, fmt, offset):
    size = struct.calcsize("<" + fmt)
    if offset < 0 or offset + size > len(data):
        raise ValueError(f"truncated WDC5 at byte {offset}, need {size} bytes")
    return struct.unpack_from("<" + fmt, data, offset)


def block(data, offset, size):
    if offset < 0 or size < 0 or offset + size > len(data):
        raise ValueError(f"truncated WDC5 block at {offset}, size {size}")
    return data[offset : offset + size]


def validate_storage(data, count, sections, record_size, palette_size, common_size):
    """Reject storage this importer does not accept; return the end of field metadata.

    Values are decoded by db2_casc; these checks only refuse unsupported or
    inconsistent descriptors (pallet arrays, wide or out-of-record fields, odd or
    duplicate common entries, storage sizes that do not add up)."""
    start = 204 + sections * 40 + count * 4
    palette_start = start + count * 24
    block(data, palette_start, palette_size)
    common = block(data, palette_start + palette_size, common_size)
    pc = cc = 0
    for index in range(count):
        offset, width, additional, storage, *_ = unpack(data, "HH5I", start + index * 24)
        if storage not in (0, 1, 2, 3, 5):
            raise ValueError(f"unsupported storage {storage} in field {index}")
        if storage != 2 and (width > 32 or offset + width > record_size * 8):
            raise ValueError(f"unsupported storage width/offset in field {index}")
        if storage == 3:
            if additional % 4:
                raise ValueError("invalid palette storage size")
            pc += additional
        elif storage == 2:
            if additional % 8:
                raise ValueError("invalid common storage size")
            pairs = list(struct.iter_unpack("<II", block(common, cc, additional)))
            if len(dict(pairs)) != len(pairs):
                raise ValueError("duplicate common storage ID")
            cc += additional
        elif additional:
            raise ValueError("unexpected additional storage data")
    if pc != palette_size or cc != common_size:
        raise ValueError("inconsistent additional storage sizes")
    return palette_start + palette_size + common_size


def validate_relations(data, offset, size, count):
    """Record indexes of a relationship map whose entries are in range and unique."""
    indexes = set()
    if not size:
        return indexes
    entries, low, high = unpack(data, "3I", offset)
    if size != 12 + entries * 8:
        raise ValueError("invalid relationship block size")
    for parent, index in struct.iter_unpack(
        "<II", block(data, offset + 12, entries * 8)
    ):
        if index >= count or index in indexes or not low <= parent <= high:
            raise ValueError("invalid relationship record index or parent")
        indexes.add(index)
    return indexes


def read_skipped_sections(data, metadata_end, sections, record_size, noninline):
    keyed = []
    unknown = set()
    for section in range(sections):
        key, start, count, *_ = unpack(data, "Q8I", 204 + section * 40)
        if not key:
            continue
        keyed.append((section, key, count))
        if count and not any(block(data, start, count * record_size)):
            unknown.add(section)
    if not unknown:
        return {}
    skipped = {}
    position = metadata_end
    for section, key, count in keyed:
        id_count = unpack(data, "I", position)[0]
        position += 4
        if id_count != count:
            raise ValueError(f"invalid encrypted-ID metadata for section {section}")
        ids = list(unpack(data, f"{id_count}I", position))
        position += id_count * 4
        if section in unknown:
            skipped[section] = {
                "section": section,
                "key_name": f"{key:016X}",
                "record_ids": ids,
                "parent": (
                    "unknown (zero-filled)" if noninline
                    else "not applicable (inline Extra)"
                ),
            }
    return skipped


def read_wdc5(data, kind, *, skipped=None):
    """{record id: (field values, parent)} decoded by db2_casc.read_fixed after strict
    shape checks. Copy rows resolve common-storage fields by their own ID."""
    layout, count, noninline = LAYOUTS[kind]
    if block(data, 0, 4) != b"WDC5" or unpack(data, "I", 4)[0] != 5:
        raise ValueError("expected WDC5 version 5")
    (
        records,
        field_count,
        record_size,
        strings,
        table,
        actual_layout,
        low,
        high,
        locale,
    ) = unpack(data, "9I", 136)
    (
        flags,
        id_index,
        total,
        bitpacked,
        lookup,
        storage_size,
        common_size,
        palette_size,
        sections,
    ) = unpack(data, "HH7I", 172)
    if actual_layout != layout or field_count != count or total != count:
        raise ValueError(
            f"unsupported {kind} layout {actual_layout:08X}, fields {field_count}/{total}"
        )
    if flags != (4 if noninline else 0):
        raise ValueError(
            f"unsupported WDC5 flags {flags:#x} (sparse records unsupported)"
        )
    if (
        storage_size != count * 24
        or not record_size
        or not sections
        or (not noninline and id_index != 0)
    ):
        raise ValueError("unsupported WDC5 header/storage shape")
    metadata_end = validate_storage(
        data, count, sections, record_size, palette_size, common_size
    )
    omitted = read_skipped_sections(data, metadata_end, sections, record_size, noninline)
    if not noninline and unpack(data, "HH5I", 204 + sections * 40 + count * 4)[3] == 2:
        raise ValueError("unsupported common storage for inline ID")
    seen_records = 0
    for section in range(sections):
        (
            key,
            start,
            n,
            string_size,
            sparse_end,
            id_size,
            relation_size,
            offsets,
            copy_count,
        ) = unpack(data, "Q8I", 204 + section * 40)
        if sparse_end or offsets:
            raise ValueError("unsupported sparse section storage")
        if n and (start < metadata_end or start >= len(data)):
            raise ValueError(f"missing {'encrypted ' if key else ''}section {section}")
        block(data, start, n * record_size)
        seen_records += n
        if section in omitted:
            continue
        if id_size != (n * 4 if noninline else 0):
            raise ValueError("invalid section ID list size")
        id_start = start + n * record_size + string_size
        copy_start = id_start + id_size
        block(
            data,
            start,
            n * record_size + string_size + id_size + copy_count * 8 + relation_size,
        )
        related = validate_relations(data, copy_start + copy_count * 8, relation_size, n)
        if noninline and len(related) != n:
            raise ValueError("missing relationship for noninline record")
        if not noninline and related:
            raise ValueError("unexpected relationship for Extra")
    if seen_records != records:
        raise ValueError("header/section record count mismatch")
    try:
        header, infos, table_sections = db2_casc.read_fixed(data)
        _, commons = db2_casc.read_field_storage(
            data, header, infos, metadata_end - palette_size - common_size
        )
    except struct.error as error:
        raise ValueError(f"corrupt WDC5 storage: {error}") from error
    raw_rows = {}
    copies = []
    for section in table_sections:
        if section.records is None:
            continue
        for record in section.records:
            if record.id <= 0 or record.id in raw_rows:
                raise ValueError(f"invalid or duplicate record ID {record.id}")
            raw_rows[record.id] = (record.values, record.parent)
        copies.extend(section.copies)
    pending = dict(copies)
    if len(pending) != len(copies):
        raise ValueError("duplicate copy ID")
    while pending:
        resolved = [rid for rid, source in pending.items() if source in raw_rows]
        if not resolved:
            raise ValueError("missing or cyclic copy source")
        for rid in resolved:
            if rid <= 0 or rid in raw_rows:
                raise ValueError(f"duplicate or invalid copy ID {rid}")
            values, parent = raw_rows[pending.pop(rid)]
            values = list(values)
            for field, common in commons.items():
                values[field] = common.get(rid, infos[field][4])
            raw_rows[rid] = (values, parent)
    rows = {}
    for rid, (values, parent) in raw_rows.items():
        values = list(values)
        if not noninline:
            values[0] = rid
        rows[rid] = (tuple(values), parent)
    if skipped is not None:
        skipped.extend(omitted.values())
    return rows


def select_material(values, model_path):
    path = model_path.replace("\\", "/").lower()
    if not path.endswith(".m2"):
        raise ValueError(f"unsupported model path {path!r}")
    return values[6 if Path(path).stem.endswith("_hd") else 5]


def join_appearances(displays, extras, options, geosets, model_paths, textures):
    materials = {}
    for display, extra in displays.items():
        if not extra:
            continue
        if extra not in extras:
            raise ValueError(f"display {display}: missing Extra {extra}")
        if display not in model_paths:
            raise ValueError(f"display {display}: missing model path")
        materials[display] = select_material(extras[extra][0], model_paths[display])
    return join_appearance_materials(
        displays, extras, options, geosets, materials, textures
    )


def join_appearance_materials(displays, extras, options, geosets, materials, textures):
    choices_by_extra = {}
    for (_, choice), parent in options.values():
        choices_by_extra.setdefault(parent, set()).add(choice)
    appearances = []
    choices = []
    for display, extra_id in sorted(displays.items()):
        if not extra_id:
            continue
        if extra_id not in extras:
            raise ValueError(f"display {display}: missing Extra {extra_id}")
        values = extras[extra_id][0]
        _, race, sex, klass, flags, sd, hd = values
        material = materials[display]
        if material != 0 and (material < 0 or textures.get(material, 0) <= 0):
            raise ValueError(f"display {display}: unresolved material {material}")
        baked_texture = 0 if material == 0 else textures[material]
        appearances.append((display, race, sex, klass, baked_texture))
        choices.extend(
            (display, choice) for choice in sorted(choices_by_extra.get(extra_id, ()))
        )
    overrides = {}
    for (index, value), display in geosets.values():
        if display not in displays:
            continue
        key = (display, index)
        if key in overrides and overrides[key] != value:
            raise ValueError(f"conflicting geoset values for {key}")
        overrides[key] = value
    coverage = [
        (display, int(extra_id != 0)) for display, extra_id in sorted(displays.items())
    ]
    return (
        appearances,
        choices,
        [(d, i, v) for (d, i), v in sorted(overrides.items())],
        coverage,
    )


def write_database(path, rows):
    path = Path(path)
    if path.exists():
        raise ValueError(f"output exists: {path}; choose a new output path")
    # Exclusive creation prevents accidental replacement of a production cache.
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb"):
        pass
    try:
        with contextlib.closing(sqlite3.connect(path)) as conn, conn:
            conn.executescript("""
                CREATE TABLE display_coverage (
                    display_id INTEGER PRIMARY KEY,
                    requires_appearance INTEGER NOT NULL CHECK (requires_appearance IN (0, 1))
                );
                CREATE TABLE appearances (display_id INTEGER PRIMARY KEY, race INTEGER NOT NULL,
                    sex INTEGER NOT NULL, class INTEGER NOT NULL, baked_texture_fdid INTEGER NOT NULL);
                CREATE TABLE choices (display_id INTEGER NOT NULL, choice_id INTEGER NOT NULL,
                    PRIMARY KEY (display_id, choice_id));
                CREATE TABLE geosets (display_id INTEGER NOT NULL, geoset_index INTEGER NOT NULL,
                    geoset_value INTEGER NOT NULL, PRIMARY KEY (display_id, geoset_index));
            """)
            conn.executemany("INSERT INTO appearances VALUES (?, ?, ?, ?, ?)", rows[0])
            conn.executemany("INSERT INTO choices VALUES (?, ?)", rows[1])
            conn.executemany("INSERT INTO geosets VALUES (?, ?, ?)", rows[2])
            conn.executemany("INSERT INTO display_coverage VALUES (?, ?)", rows[3])
    except Exception:
        path.unlink()
        raise


def read_csv(path):
    with path.open(newline="", encoding="utf-8-sig") as handle:
        yield from csv.DictReader(handle)


def read_sqlite(path, query):
    with contextlib.closing(
        sqlite3.connect(Path(path).resolve().as_uri() + "?mode=ro", uri=True)
    ) as conn:
        return conn.execute(query).fetchall()


def merge_existing_rows(base_cache, rows):
    """Preserve authored rows while extending coverage with validated new imports."""
    tables = [
        ("appearances", "display_id, race, sex, class, baked_texture_fdid", 1),
        ("choices", "display_id, choice_id", 2),
        ("geosets", "display_id, geoset_index, geoset_value", 2),
        ("display_coverage", "display_id, requires_appearance", 1),
    ]
    merged = []
    for (table, columns, key_size), imported in zip(tables, rows):
        existing = read_sqlite(base_cache, f"SELECT {columns} FROM {table}")
        by_key = {row[:key_size]: tuple(row) for row in existing}
        for row in imported:
            row = tuple(row)
            key = row[:key_size]
            if key in by_key and by_key[key] != row:
                raise ValueError(f"conflicting existing {table} row {key}")
            by_key[key] = row
        merged.append(sorted(by_key.values()))
    return tuple(merged)


def load_model_paths(cache, listfile, displays):
    fdids = {}
    for display, fdid in read_sqlite(
        cache, "SELECT display_id, model_fdid FROM creature_displays"
    ):
        if displays.get(display, 0):
            if display in fdids:
                raise ValueError(f"duplicate model cache display {display}")
            fdids[display] = fdid
    needed = set(fdids.values())
    paths = {}
    with listfile.open(newline="", encoding="utf-8-sig") as handle:
        for fdid, path in csv.reader(handle, delimiter=";"):
            fdid = int(fdid)
            if fdid in needed:
                if fdid in paths and paths[fdid] != path:
                    raise ValueError(f"ambiguous model path for FDID {fdid}")
                paths[fdid] = path
    return {display: paths[fdid] for display, fdid in fdids.items() if fdid in paths}


def load_textures(csv_path, outfit_cache, needed):
    if outfit_cache:
        rows = read_sqlite(
            outfit_cache,
            "SELECT material_resource_id, texture_fdid FROM material_to_texture",
        )
    else:
        rows = (
            (int(row["MaterialResourcesID"]), int(row["FileDataID"]))
            for row in read_csv(csv_path)
            if int(row["UsageType"]) == 0
        )
    textures = {}
    for material, fdid in rows:
        if material not in needed:
            continue
        if material in textures and textures[material] != fdid:
            raise ValueError(
                f"ambiguous material {material}: {textures[material]} / {fdid}"
            )
        textures[material] = fdid
    return textures


def referenced_world_displays(path):
    """All content-spawn alternatives and overrides, including inactive/map-copy roots."""
    rows = read_sqlite(path, """
        WITH entries AS (
            SELECT id1 AS entry FROM content_creature WHERE id1 != 0
            UNION SELECT id2 FROM content_creature WHERE id2 != 0
            UNION SELECT id3 FROM content_creature WHERE id3 != 0
        )
        SELECT model.CreatureDisplayID
        FROM content_creature_template_model model
        JOIN entries ON entries.entry = model.CreatureID
        WHERE model.CreatureDisplayID != 0
        UNION SELECT modelid FROM content_creature WHERE modelid != 0
    """)
    return {int(row[0]) for row in rows}


def import_files(args):
    if args.pet_catalog:
        catalog = json.loads(args.pet_catalog.read_text())
        pet_displays = sorted({int(pet["display_id"]) for pet in catalog if int(pet["display_id"]) > 0})
        if not pet_displays:
            raise ValueError("pet catalog has no resolvable display roots")
        args.display_id = list(args.display_id or []) + pet_displays
    decoded = {}
    skipped = {}
    for kind, table, filename in [
        ("extra", "CreatureDisplayInfoExtra", "1264997.db2"),
        ("option", "CreatureDisplayInfoOption", "3692043.db2"),
        ("geoset", "CreatureDisplayInfoGeosetData", "1720141.db2"),
    ]:
        path = args.db2_dir / filename
        try:
            skipped[table] = []
            decoded[kind] = read_wdc5(path.read_bytes(), kind, skipped=skipped[table])
        except ValueError as error:
            raise ValueError(f"{path}: {error}") from error
    displays = {}
    for row in read_csv(args.data_dir / "CreatureDisplayInfo.csv"):
        display = int(row["ID"])
        if display in displays:
            raise ValueError(f"duplicate display {display}")
        displays[display] = int(row["ExtendedDisplayInfoID"])
    world_displays = referenced_world_displays(args.world_db) if args.world_db else None
    world_missing = sorted(world_displays - displays.keys()) if world_displays is not None else []
    if world_displays is not None:
        selected = world_displays & displays.keys()
        if args.display_id:
            selected &= set(args.display_id)
        displays = {display: displays[display] for display in sorted(selected)}
    if args.display_id:
        missing = set(args.display_id) - displays.keys()
        if missing:
            raise ValueError(f"missing requested displays: {sorted(missing)}")
        displays = {d: displays[d] for d in sorted(set(args.display_id))}
    models = load_model_paths(
        args.model_cache, args.data_dir / "community-listfile.csv", displays
    )
    materials = set()
    for display, extra in displays.items():
        if extra in decoded["extra"] and display in models:
            material = select_material(decoded["extra"][extra][0], models[display])
            if material != 0:
                materials.add(material)
    textures = load_textures(
        args.data_dir / "TextureFileData.csv", args.outfit_cache, materials
    )
    rows = join_appearances(
        displays,
        decoded["extra"],
        decoded["option"],
        decoded["geoset"],
        models,
        textures,
    )
    if args.base_cache is not None:
        rows = merge_existing_rows(args.base_cache, rows)
    write_database(args.output, rows)
    return {
        "output": str(args.output.resolve()),
        "world_reference_count": len(world_displays) if world_displays is not None else None,
        "world_displays_absent_from_source": world_missing,
        "decoded_records": {kind: len(records) for kind, records in decoded.items()},
        "skipped_sections": skipped,
        "counts": dict(
            zip(
                ("appearances", "choices", "geosets", "display_coverage"),
                map(len, rows),
            )
        ),
        "fixtures": {
            str(display): {
                "extra_id": displays[display],
                "model_path": models.get(display),
                "appearance": [row for row in rows[0] if row[0] == display],
                "choices": [row[1] for row in rows[1] if row[0] == display],
                "geosets": [row[1:] for row in rows[2] if row[0] == display],
            }
            for display in sorted(
                set(args.display_id or [19177, 19178, 3167]) & displays.keys()
            )
        },
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--db2-dir",
        required=True,
        type=Path,
        help="Local directory containing the three FDID-named WDC5 files",
    )
    parser.add_argument(
        "--data-dir",
        required=True,
        type=Path,
        help="Existing CreatureDisplayInfo/TextureFileData CSVs and community listfile",
    )
    parser.add_argument(
        "--model-cache",
        required=True,
        type=Path,
        help="Existing creature_display.sqlite (read-only)",
    )
    parser.add_argument(
        "--outfit-cache",
        type=Path,
        help="Use cached material_to_texture instead of TextureFileData.csv",
    )
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="New SQLite path; existing files are never overwritten",
    )
    parser.add_argument(
        "--base-cache",
        type=Path,
        help="Preserve this read-only authored cache in the new output; conflicts fail",
    )
    parser.add_argument(
        "--display-id",
        action="append",
        type=int,
        help="Limit output to these displays; repeatable, otherwise all displays",
    )
    parser.add_argument(
        "--world-db", type=Path,
        help="Read-only canonical content spawn roots: all maps, model alternatives and overrides",
    )
    parser.add_argument(
        "--pet-catalog", type=Path,
        help="Add every resolved companion display from export_pet_catalog.py output",
    )
    args = parser.parse_args(argv)
    try:
        report = import_files(args)
    except (ValueError, KeyError, OSError, sqlite3.Error, csv.Error) as error:
        print(f"NPC appearance import failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
