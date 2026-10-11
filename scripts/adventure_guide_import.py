#!/usr/bin/env python3
"""Import local Retail 12.1.0.69933 Journal DB2s into the client SQLite cache.

Extract the FDIDs below with casc-local first. Pass the corresponding WoWDBDefs
.dbd files in --definitions. The pinned layout hashes, not names, define fields.
Unreadable encrypted records are counted; never synthesize replacement records.
"""
import argparse
import csv
import importlib.util
from pathlib import Path
import re
import sqlite3
import struct

SPEC = importlib.util.spec_from_file_location('journal_wdc5', Path(__file__).with_name('export_db2_csv.py'))
WDC = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(WDC)
BUILD = '12.1.0.69933'
TABLES = {
    'JournalTier': (1349056, 0xF4AFE752),
    'JournalTierXInstance': (1285009, 0xBFCBF1F0),
    'JournalInstance': (1237438, 0x6C5ED7F2),
    'JournalEncounter': (1240336, 0xB88D11CC),
    'JournalEncounterCreature': (1301155, 0x81A3C769),
    'JournalEncounterSection': (1134413, 0xCB88312B),
    'JournalEncounterItem': (1344467, 0x8ED8EAEA),
    'JournalItemXDifficulty': (1277744, 0x0AD35082),
    'JournalSectionXDifficulty': (1121900, 0x9A12141A),
    'JournalEncounterXDifficulty': (1121901, 0xC038156D),
    'ItemSet': (1343609, 0xF79068A4),
    'ItemSpec': (1135120, 0x83F3D113),
    'ItemSpecOverride': (1134576, 0xB292998C),
}


def layout_fields(definition, layout):
    lines = definition.read_text().splitlines()
    types = {}
    for line in lines[1:]:
        if line.startswith(('LAYOUT', 'BUILD')):
            break
        match = re.match(r'(int|float|string|locstring)(?:<[^>]*>)?\s+(\w+)', line)
        if match:
            types[match[2]] = match[1]
    start = next((i for i, line in enumerate(lines) if line.startswith('LAYOUT') and f'{layout:08X}' in line), None)
    if start is None:
        raise ValueError(f'{definition}: layout {layout:08X} absent')
    fields = []
    for line in lines[start + 1:]:
        if line.startswith('LAYOUT') or (line.startswith('BUILD') and fields):
            break
        if not line.strip() or line.startswith('BUILD'):
            continue
        match = re.fullmatch(r'(?:\$([^$]+)\$)?(\w+)(?:<([^>]+)>)?(?:\[(\d+)\])?', line.strip())
        if match is None:
            raise ValueError(f'Unsupported DBD field {line}')
        tags, name, width, count = match.groups()
        fields.append((name, types[name], (tags or '').split(','), width, int(count or 1)))
    return fields


def decode_value(data, offset, descriptor, raw, kind, width, element):
    bits = int((width or '32').lstrip('u'))
    value = raw[element] if isinstance(raw, tuple) else (raw >> (bits * element)) & ((1 << bits) - 1)
    if kind in ('string', 'locstring'):
        return WDC.read_string(data, offset, descriptor, value)
    if kind == 'float':
        return struct.unpack('<f', struct.pack('<I', value))[0]
    if width and not width.startswith('u') and value & (1 << (bits - 1)):
        value -= 1 << bits
    return value


def read_table(table, path, definition):
    _, layout = TABLES[table]
    data = path.read_bytes()
    fields = layout_fields(definition, layout)
    inline = [field for field in fields if 'noninline' not in field[2]]
    id_field = next((i for i, field in enumerate(inline) if 'id' in field[2]), 0)
    records, dropped, descriptors, _ = WDC.read_wdc5(data, layout, id_field)
    columns = [(name if count == 1 else f'{name}_{i}', 'TEXT' if kind in ('string', 'locstring') else 'REAL' if kind == 'float' else 'INTEGER')
               for name, kind, _, _, count in fields for i in range(count)]
    rows = []
    for row_id, (values, parent, offset) in sorted(records.items()):
        row, index = [], 0
        for name, kind, tags, width, count in fields:
            if 'noninline' in tags:
                if 'id' in tags:
                    row.append(row_id)
                elif 'relation' in tags:
                    row.append(parent)
                else:
                    raise ValueError(f'{table}: unsupported noninline field {name}')
                continue
            if 'id' in tags:
                row.append(row_id)
            else:
                row.extend(decode_value(data, offset, descriptors[index], values[index], kind, width, i) for i in range(count))
            index += 1
        rows.append(row)
    return columns, rows, dropped


def import_items(connection, items):
    connection.execute('CREATE TABLE item (ID INTEGER PRIMARY KEY, name TEXT, icon INTEGER, slot INTEGER, class_mask INTEGER, quality INTEGER, item_level INTEGER, item_class INTEGER, subclass INTEGER, stats TEXT)')
    with (items / 'Item.csv').open(newline='') as handle:
        base = {int(row['ID']): row for row in csv.DictReader(handle)}
    with (items / 'ItemSparse.csv').open(newline='') as handle:
        for row in csv.DictReader(handle):
            item = base.get(int(row['ID']))
            if item is None:
                continue
            stats = ','.join(row[f'StatModifier_bonusStat_{i}'] for i in range(10))
            connection.execute('INSERT INTO item VALUES(?,?,?,?,?,?,?,?,?,?)', (int(row['ID']), row['Display_lang'], int(item['IconFileDataID']), int(row['InventoryType']), int(row['AllowableClass']), int(row['OverallQualityID']), int(row['ItemLevel']), int(item['ClassID']), int(item['SubclassID']), stats))
    connection.execute('CREATE TABLE specialization (ID INTEGER PRIMARY KEY, name TEXT, class_id INTEGER)')
    with (items / 'ChrSpecialization.csv').open(newline='') as handle:
        connection.executemany('INSERT INTO specialization VALUES(?,?,?)', ((int(row['ID']), row['Name_lang'], int(row['ClassID'])) for row in csv.DictReader(handle)))


def import_cache(db2, definitions, output, items):
    # Decode every input before replacing an existing cache.
    decoded = {table: read_table(table, db2 / f'{fdid}.db2', definitions / f'{table}.dbd') for table, (fdid, _) in TABLES.items()}
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = output.with_suffix('.staging.sqlite')
    if staging.exists():
        staging.unlink()
    with sqlite3.connect(staging) as connection:
        connection.execute('CREATE TABLE source (build TEXT, table_name TEXT, layout TEXT, readable INTEGER, unreadable INTEGER)')
        for table, (columns, rows, dropped) in decoded.items():
            connection.execute(f'CREATE TABLE "{table}" ({", ".join(f"{name} {kind}" for name, kind in columns)})')
            connection.executemany(f'INSERT INTO "{table}" VALUES ({",".join("?" for _ in columns)})', rows)
            connection.execute(f'CREATE UNIQUE INDEX "{table}_id" ON "{table}" (ID)')
            connection.execute('INSERT INTO source VALUES(?,?,?,?,?)', (BUILD, table, f'{TABLES[table][1]:08X}', len(rows), dropped))
            print(f'{table}: {len(rows)} readable, {dropped} encrypted/unreadable')
        import_items(connection, items)
    staging.replace(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db2', type=Path, required=True)
    parser.add_argument('--definitions', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--items', type=Path, required=True, help='Pinned Item, ItemSparse, ChrSpecialization CSV directory')
    args = parser.parse_args()
    import_cache(args.db2, args.definitions, args.output, args.items)


if __name__ == '__main__':
    main()
