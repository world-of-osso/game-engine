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


def read_fields(data, count, sections, record_size, palette_size, common_size):
    start = 204 + sections * 40 + count * 4
    palette_start = start + count * 24
    palette = block(data, palette_start, palette_size)
    common = block(data, palette_start + palette_size, common_size)
    fields = []
    pc = cc = 0
    for index in range(count):
        offset, width, additional, storage, a, b, c = unpack(
            data, "HH5I", start + index * 24
        )
        if storage not in (0, 1, 2, 3, 5):
            raise ValueError(f"unsupported storage {storage} in field {index}")
        if storage != 2 and (width > 32 or offset + width > record_size * 8):
            raise ValueError(f"unsupported storage width/offset in field {index}")
        values = None
        if storage == 3:
            if additional % 4:
                raise ValueError("invalid palette storage size")
            values = [
                v[0] for v in struct.iter_unpack("<I", block(palette, pc, additional))
            ]
            pc += additional
        elif storage == 2:
            if additional % 8:
                raise ValueError("invalid common storage size")
            pairs = list(struct.iter_unpack("<II", block(common, cc, additional)))
            values = dict(pairs)
            if len(values) != len(pairs):
                raise ValueError("duplicate common storage ID")
            cc += additional
        elif additional:
            raise ValueError("unexpected additional storage data")
        fields.append((offset, width, storage, a, values))
    if pc != palette_size or cc != common_size:
        raise ValueError("inconsistent additional storage sizes")
    return fields, palette_start + palette_size + common_size


def decode_field(raw, field, row_id):
    offset, width, storage, default, values = field
    if storage == 2:
        return values.get(row_id, default)
    value = (raw >> offset) & ((1 << width) - 1)
    if storage == 3:
        if value >= len(values):
            raise ValueError(f"palette storage index {value} out of bounds")
        return values[value]
    if storage == 5 and width and value & (1 << (width - 1)):
        value -= 1 << width
    return value


def read_relations(data, offset, size, count):
    relation = {}
    if not size:
        return relation
    entries, low, high = unpack(data, "3I", offset)
    if size != 12 + entries * 8:
        raise ValueError("invalid relationship block size")
    for parent, index in struct.iter_unpack(
        "<II", block(data, offset + 12, entries * 8)
    ):
        if index >= count or index in relation or not low <= parent <= high:
            raise ValueError("invalid relationship record index or parent")
        relation[index] = parent
    return relation


def read_wdc5(data, kind):
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
    fields, metadata_end = read_fields(
        data, count, sections, record_size, palette_size, common_size
    )
    if not noninline and fields[0][2] == 2:
        raise ValueError("unsupported common storage for inline ID")
    raw_rows = {}
    copies = []
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
        payload = block(data, start, n * record_size)
        if key and n and not any(payload):
            raise ValueError(f"missing encrypted section {section}, key {key:016X}")
        if id_size != (n * 4 if noninline else 0):
            raise ValueError("invalid section ID list size")
        id_start = start + n * record_size + string_size
        copy_start = id_start + id_size
        relation_start = copy_start + copy_count * 8
        block(
            data,
            start,
            n * record_size + string_size + id_size + copy_count * 8 + relation_size,
        )
        relations = read_relations(data, relation_start, relation_size, n)
        if noninline and len(relations) != n:
            raise ValueError("missing relationship for noninline record")
        if not noninline and relations:
            raise ValueError("unexpected relationship for Extra")
        for i in range(n):
            raw = int.from_bytes(
                payload[i * record_size : (i + 1) * record_size], "little"
            )
            rid = (
                unpack(data, "I", id_start + i * 4)[0]
                if noninline
                else decode_field(raw, fields[0], 0)
            )
            if rid <= 0 or rid in raw_rows:
                raise ValueError(f"invalid or duplicate record ID {rid}")
            raw_rows[rid] = (raw, relations.get(i))
        copies.extend(
            struct.iter_unpack("<II", block(data, copy_start, copy_count * 8))
        )
        seen_records += n
    if seen_records != records:
        raise ValueError("header/section record count mismatch")
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
            raw_rows[rid] = raw_rows[pending.pop(rid)]
    rows = {}
    for rid, (raw, parent) in raw_rows.items():
        values = [decode_field(raw, field, rid) for field in fields]
        if not noninline:
            values[0] = rid
        rows[rid] = (tuple(values), parent)
    return rows


def select_material(values, model_path):
    path = model_path.replace("\\", "/").lower()
    if not path.endswith(".m2"):
        raise ValueError(f"unsupported model path {path!r}")
    return values[6 if Path(path).stem.endswith("_hd") else 5]


def join_appearances(displays, extras, options, geosets, model_paths, textures):
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
        if display not in model_paths:
            raise ValueError(f"display {display}: missing model path")
        values = extras[extra_id][0]
        _, race, sex, klass, flags, sd, hd = values
        material = select_material(values, model_paths[display])
        if material != 0 and (material < 0 or textures.get(material, 0) <= 0):
            raise ValueError(
                f"display {display}: unresolved material {material} for {model_paths[display]}"
            )
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


def import_files(args):
    decoded = {}
    for kind, filename in [
        ("extra", "1264997.db2"),
        ("option", "3692043.db2"),
        ("geoset", "1720141.db2"),
    ]:
        path = args.db2_dir / filename
        try:
            decoded[kind] = read_wdc5(path.read_bytes(), kind)
        except ValueError as error:
            raise ValueError(f"{path}: {error}") from error
    displays = {}
    for row in read_csv(args.data_dir / "CreatureDisplayInfo.csv"):
        display = int(row["ID"])
        if display in displays:
            raise ValueError(f"duplicate display {display}")
        displays[display] = int(row["ExtendedDisplayInfoID"])
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
    write_database(args.output, rows)
    return {
        "output": str(args.output.resolve()),
        "decoded_records": {kind: len(records) for kind, records in decoded.items()},
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
                set(args.display_id or [13035, 13036, 130617]) & displays.keys()
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
        "--display-id",
        action="append",
        type=int,
        help="Limit output to these displays; repeatable, otherwise all displays",
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
