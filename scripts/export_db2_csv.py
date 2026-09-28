#!/usr/bin/env python3
"""Export the columns the client reads from local-CASC WDC5 tables to data/db2 CSVs.

Only the layouts listed in TABLES (build 12.1.0.69933) are accepted. Sections whose
TACT key is unknown arrive zero-filled from casc-local; their records are dropped and
counted on stderr.

Usage: export_db2_csv.py <table> <file.db2> <out.csv>
  Emotes                       FDID 1343602
  JournalInstance              FDID 1237438 (localized: pass its enUS copy)
  JournalInstanceEntrance      FDID 5228481
  NPCModelItemSlotDisplayInfo  FDID 1340661
  UiTextureKit                 FDID 939159
  ZoneLight                    FDID 1310253
  ZoneLightPoint               FDID 1310256
"""

import csv
import struct
import sys

# (layout hash, [(CSV column, source)]); source is "id", "parent", a field index,
# ("string", field index) for an inline string field of a single-section table, or
# ("float", field index, element) for 32-bit element `element` of a float field.
TABLES = {
    "Emotes": (0x0A598B68, [("ID", "id"), ("AnimID", 1)]),
    "JournalInstance": (
        0x6C5ED7F2,
        [("ID", "id"), ("Name_lang", ("string", 0)), ("MapID", 2), ("Flags", 7), ("AreaID", 8)],
    ),
    "JournalInstanceEntrance": (
        0x874E7CC2,
        [
            ("ID", "id"),
            ("Pos_0", ("float", 0, 0)),
            ("Pos_1", ("float", 0, 1)),
            ("Pos_2", ("float", 0, 2)),
            ("MapID", 1),
            ("AreaTableID", 2),
            ("Faction", 3),
            ("JournalInstanceID", "parent"),
        ],
    ),
    "NPCModelItemSlotDisplayInfo": (
        0xC2057F5B,
        [("ID", "id"), ("NpcModelID", "parent"), ("ItemDisplayInfoID", 0), ("ItemSlot", 1)],
    ),
    "UiTextureKit": (0x4740638A, [("ID", "id"), ("KitPrefix", ("string", 0))]),
    "ZoneLight": (
        0x94CE95E0,
        [
            ("ID", "id"),
            ("Name", ("string", 0)),
            ("MapID", 1),
            ("LightID", 2),
            ("Flags", 3),
            ("Zmin", ("float", 4, 0)),
            ("Zmax", ("float", 5, 0)),
            ("TransitionType", 6),
            ("PlayerConditionID", 7),
        ],
    ),
    "ZoneLightPoint": (
        0xDE2377FB,
        [
            ("ID", "id"),
            ("Pos_0", ("float", 0, 0)),
            ("Pos_1", ("float", 0, 1)),
            ("PointOrder", 1),
            ("ZoneLightID", "parent"),
        ],
    ),
}


def read_fields(data, field_count, sections):
    start = 204 + sections * 40 + field_count * 4
    fields = [struct.unpack_from("<HH5I", data, start + i * 24) for i in range(field_count)]
    palette_offsets, offset = [], 0
    for field in fields:
        palette_offsets.append(offset)
        offset += field[2]
    return fields, palette_offsets, start + field_count * 24


def decode_field(raw, field, palette, palette_offset):
    bit_offset, width, _, storage, _, _, array_count = field
    value = (raw >> bit_offset) & ((1 << width) - 1)
    if storage == 3:
        return struct.unpack_from("<I", palette, palette_offset + value * 4)[0]
    if storage == 4:
        return struct.unpack_from(f"<{array_count}I", palette, palette_offset + value * 4 * array_count)
    if storage == 5 and width and value & (1 << (width - 1)):
        return value - (1 << width)
    if storage not in (0, 1, 5):
        raise ValueError(f"unsupported field storage {storage}")
    return value


def read_relations(data, offset, size):
    if not size:
        return {}
    entries = struct.unpack_from("<I", data, offset)[0]
    return {index: parent for parent, index in struct.iter_unpack("<II", data[offset + 12 : offset + 12 + entries * 8])}


def read_wdc5(data, layout):
    if data[:4] != b"WDC5":
        raise ValueError("not a WDC5 file")
    _, field_count, record_size, _, _, actual_layout = struct.unpack_from("<6I", data, 136)
    flags, _, _, _, _, _, _, palette_size, sections = struct.unpack_from("<HH7I", data, 172)
    if actual_layout != layout:
        raise ValueError(f"layout {actual_layout:08X}, expected {layout:08X}")
    if flags & ~0x4:
        raise ValueError(f"unsupported WDC5 flags {flags:#x}")
    fields, palette_offsets, palette_start = read_fields(data, field_count, sections)
    palette = data[palette_start : palette_start + palette_size]
    rows, dropped = {}, 0
    for section in range(sections):
        key, start, count, string_size, _, id_size, relation_size, _, copies = struct.unpack_from(
            "<Q8I", data, 204 + section * 40
        )
        payload = data[start : start + count * record_size]
        if key and count and not any(payload):
            dropped += count
            continue
        id_start = start + count * record_size + string_size
        copy_start = id_start + id_size
        relations = read_relations(data, copy_start + copies * 8, relation_size)
        for i in range(count):
            raw = int.from_bytes(payload[i * record_size : (i + 1) * record_size], "little")
            values = [decode_field(raw, f, palette, o) for f, o in zip(fields, palette_offsets)]
            row_id = struct.unpack_from("<I", data, id_start + i * 4)[0] if id_size else values[0]
            rows[row_id] = (values, relations.get(i), start + i * record_size)
        for new_id, source in struct.iter_unpack("<II", data[copy_start : copy_start + copies * 8]):
            rows[new_id] = rows[source]
    return rows, dropped, fields, sections


def read_string(data, record_offset, field, value):
    """Inline string fields hold the string's offset from the field's own byte position."""
    start = record_offset + field[0] // 8 + value
    return data[start : data.index(b"\0", start)].decode("utf-8")


def main():
    table, db2_path, out_path = sys.argv[1:4]
    layout, columns = TABLES[table]
    data = open(db2_path, "rb").read()
    rows, dropped, fields, sections = read_wdc5(data, layout)
    if sections != 1 and any(isinstance(s, tuple) and s[0] == "string" for _, s in columns):
        raise ValueError(f"string columns need a single-section table, got {sections} sections")
    with open(out_path, "w", newline="") as handle:
        out = csv.writer(handle)
        out.writerow([name for name, _ in columns])
        for row_id in sorted(rows):
            values, parent, record_offset = rows[row_id]

            def column(s):
                if isinstance(s, tuple) and s[0] == "float":
                    bits = (values[s[1]] >> (32 * s[2])) & 0xFFFFFFFF
                    return "%.9g" % struct.unpack("<f", struct.pack("<I", bits))[0]
                if isinstance(s, tuple):
                    return read_string(data, record_offset, fields[s[1]], values[s[1]])
                return {"id": row_id, "parent": parent}[s] if isinstance(s, str) else values[s]

            out.writerow([column(s) for _, s in columns])
    print(f"{table}: {len(rows)} rows, {dropped} encrypted records dropped", file=sys.stderr)


if __name__ == "__main__":
    main()
