"""Encode genuine exported CSV values as explicit, uncompressed WDC5 test records.

These are reconstructed fixtures, not original CASC snapshots. Arrays retain their
DBD grouping; strings use field-relative offsets and IDs use a non-inline list.
"""

import csv
import io
import re
import struct


FIXTURE_LAYOUT = 0x12345678


def csv_wdc5(content, float_columns=(), string_columns=(), row_ids=None):
    reader = csv.DictReader(io.StringIO(content.decode()))
    rows = list(reader)
    names = [name for name in reader.fieldnames if name != "ID"]
    fields = []
    for name in names:
        match = re.fullmatch(r"(.+)_(\d+)", name)
        base = match[1] if match else name
        if match and int(match[2]) > 0:
            if not fields or fields[-1][0] != base:
                raise ValueError(f"noncontiguous array column: {name}")
            fields[-1][1].append(name)
        else:
            fields.append((base, [name]))
    definitions = ["COLUMNS", "int ID"]
    layout_fields = ["$noninline,id$ID<32>"]
    for base, columns in fields:
        kind = "float" if columns[0] in float_columns else "int"
        if columns[0] in string_columns:
            kind = "string"
        definitions.append(f"{kind} {base}")
        width = "" if kind in ("string", "float") else "<32>"
        array = f"[{len(columns)}]" if len(columns) > 1 else ""
        layout_fields.append(f"{base}{width}{array}")
    definition = (
        "\n".join(definitions)
        + "\n\nLAYOUT 12345678\nBUILD 1.60.1.70205\n"
        + "\n".join(layout_fields) + "\n"
    ).encode()
    record_size = 4 * len(names)
    record_start = 244 + len(fields) * 28
    records, strings, ids = bytearray(), bytearray(), bytearray()
    for index, row in enumerate(rows):
        row_id = (
            row_ids[index] if row_ids is not None
            else int(row["ID"] if "ID" in row else row["ItemLevel"])
        )
        ids += struct.pack("<I", row_id)
        for column_index, name in enumerate(names):
            value = row[name]
            if name in string_columns:
                offset = (
                    (len(rows) - index) * record_size
                    - column_index * 4 + len(strings)
                )
                records += struct.pack("<I", offset)
                strings += value.encode() + b"\0"
            elif name in float_columns:
                records += struct.pack("<f", float(value))
            else:
                records += struct.pack("<i", int(value))
    raw = bytearray(record_start)
    raw[:4] = b"WDC5"
    struct.pack_into("<6I", raw, 136, len(rows), len(fields), record_size,
                     len(strings), 0, FIXTURE_LAYOUT)
    struct.pack_into("<HH7I", raw, 172, 4, 0, len(fields), 0,
                     len(fields) * 24, 0, 0, 0, 1)
    struct.pack_into("<Q8I", raw, 204, 0, record_start, len(rows), len(strings),
                     record_start + len(records), len(ids), 0, 0, 0)
    bit_offset = 0
    for index, (_, columns) in enumerate(fields):
        width = len(columns) * 32
        struct.pack_into("<HH5I", raw, 244 + len(fields) * 4 + index * 24,
                         bit_offset, width, 0, 0, 0, 0, 0)
        bit_offset += width
    return bytes(raw + records + strings + ids), definition
