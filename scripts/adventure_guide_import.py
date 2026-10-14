#!/usr/bin/env python3
"""Import local Retail 12.1.0.69933 Journal DB2s into the client SQLite cache.

Extract the FDIDs below with casc-local first. Pass the corresponding WoWDBDefs
.dbd files in --definitions. The pinned layout hashes, not names, define fields.
Unreadable encrypted records are counted; never synthesize replacement records.
"""
import argparse
from contextlib import closing
import csv
import importlib.util
from pathlib import Path
import re
import sqlite3
import struct

SPEC = importlib.util.spec_from_file_location('journal_wdc5', Path(__file__).with_name('export_db2_csv.py'))
WDC = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(WDC)
from db2_source import source_identity
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
    'ItemSetSpell': (1314689, 0x2666A73F),
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
    import db2_dbd
    if kind in ('string', 'locstring'):
        value = db2_dbd.number(raw, 'int', 'u32', element)
        return WDC.read_string(data, offset, descriptor, value)
    value = db2_dbd.number(raw, kind, width, element)
    return float(value) if kind == 'float' else value


def read_table(table, path, definition):
    import db2_dbd
    data = path.read_bytes()
    layout = struct.unpack_from('<I', data, 156)[0]
    fields = db2_dbd.layout_fields(definition, layout)
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


# TrinityCore a352b1fa ObjectMgr::ItemSpecStats and DBCEnums::ItemSpecStat.
WEAPON_SPEC_STATS = {0: 7, 1: 8, 2: 16, 3: 15, 4: 11, 5: 12, 6: 19,
                     7: 9, 8: 10, 9: 28, 10: 18, 13: 14, 15: 13,
                     16: 20, 18: 17, 19: 21}
MODIFIER_SPEC_STATS = {3: (1,), 4: (2,), 5: (0,), 13: (5,), 14: (6,),
                       19: (24,), 20: (24,), 21: (24,), 32: (24,),
                       36: (25,), 31: (4,), 50: (26,),
                       71: (1, 2, 0), 72: (1, 2), 73: (1, 0), 74: (2, 0)}


def item_spec_traits(item_class, subclass, slot, modifiers):
    stats = set()
    item_type = 0
    if item_class == 2:
        item_type = 5
        if subclass in WEAPON_SPEC_STATS:
            stats.add(WEAPON_SPEC_STATS[subclass])
    elif item_class == 4:
        if subclass == 1 and slot == 16:
            stats.add(27)
        elif 1 <= subclass <= 4:
            item_type = subclass
        elif subclass == 6:
            item_type = 6
            stats.add(22)
        elif 6 < subclass <= 11:
            item_type = 6
            stats.add(23)
    for modifier in modifiers:
        stats.update(MODIFIER_SPEC_STATS.get(modifier, ()))
    return item_type, stats


def import_item_specializations(connection):
    overrides = {}
    for item, spec in connection.execute('SELECT ItemID,SpecID FROM ItemSpecOverride WHERE ItemID IS NOT NULL'):
        overrides.setdefault(item, set()).add(spec)
    rules = {}
    for item_type, primary, secondary, spec in connection.execute('SELECT ItemType,PrimaryStat,SecondaryStat,SpecializationID FROM ItemSpec'):
        rules.setdefault(item_type, []).append((primary, secondary, spec))
    specs = dict(connection.execute('SELECT ID,class_id FROM specialization'))
    connection.execute('CREATE TABLE item_specialization (item_id INTEGER, spec_id INTEGER, PRIMARY KEY(item_id,spec_id))')
    for item, item_class, subclass, slot, mask, modifiers in connection.execute('SELECT ID,item_class,subclass,slot,class_mask,stats FROM item WHERE ID IN (SELECT ItemID FROM JournalEncounterItem)'):
        if item in overrides:
            matched = overrides[item]
        else:
            item_type, traits = item_spec_traits(item_class, subclass, slot, map(int, modifiers.split(',')))
            matched = {spec for primary, secondary, spec in rules.get(item_type, ())
                       if (primary == 40 or primary in traits)
                       and (secondary == 40 or secondary in traits)
                       and spec in specs and mask & (1 << (specs[spec] - 1))}
        connection.executemany('INSERT INTO item_specialization VALUES(?,?)', ((item, spec) for spec in matched))


def import_presentation(connection, items):
    connection.execute('CREATE TABLE class (ID INTEGER PRIMARY KEY, name TEXT)')
    with (items / 'ChrClasses.csv').open(newline='') as handle:
        connection.executemany('INSERT INTO class VALUES(?,?)', ((int(row['ID']), row['Name_lang']) for row in csv.DictReader(handle)))
    connection.execute('CREATE TABLE difficulty (ID INTEGER PRIMARY KEY, name TEXT)')
    with (items / 'Difficulty.csv').open(newline='') as handle:
        connection.executemany('INSERT INTO difficulty VALUES(?,?)', ((int(row['ID']), row['Name_lang']) for row in csv.DictReader(handle)))
    connection.execute('CREATE TABLE map_difficulty (map_id INTEGER, difficulty_id INTEGER)')
    with (items / 'MapDifficulty.csv').open(newline='') as handle:
        connection.executemany('INSERT INTO map_difficulty VALUES(?,?)', ((int(row['MapID']), int(row['DifficultyID'])) for row in csv.DictReader(handle) if int(row['DifficultyID']) != 0))
    connection.execute('CREATE TABLE map_type (ID INTEGER PRIMARY KEY, instance_type INTEGER)')
    with (items / 'Map.csv').open(newline='') as handle:
        connection.executemany('INSERT INTO map_type VALUES(?,?)', ((int(row['ID']), int(row['InstanceType'])) for row in csv.DictReader(handle)))
    connection.execute('CREATE TABLE journal_text (tag TEXT PRIMARY KEY, text TEXT)')
    with (items / 'GlobalStrings.csv').open(newline='') as handle:
        connection.executemany('INSERT INTO journal_text VALUES(?,?)',
            ((row['BaseTag'], row['TagText_lang'].replace('|n', '\n')) for row in csv.DictReader(handle)
             if row['BaseTag'].startswith(('INVTYPE_', 'ITEM_SET_')) or row['BaseTag'] in ('RPE_TITLE', 'RPE_TUTORIAL_TEXT', 'RPE_START_EXPERIENCE')))


def read_challenge_dungeons(world):
    with closing(sqlite3.connect(f'file:{world.resolve()}?mode=ro', uri=True)) as connection:
        return connection.execute('SELECT ID,MapID,Name_lang,CriteriaCount_0 * 1000 FROM map_challenge_mode WHERE MaxMedals=3').fetchall()


def import_cache(db2, definitions, output, items, world):
    build = source_identity(db2)['build']
    # Decode every input before replacing an existing cache.
    decoded = {table: read_table(table, db2 / f'{fdid}.db2', definitions / f'{table}.dbd') for table, (fdid, _) in TABLES.items()}
    challenges = read_challenge_dungeons(world)
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = output.with_suffix('.staging.sqlite')
    if staging.exists():
        staging.unlink()
    with closing(sqlite3.connect(staging)) as connection, connection:
        connection.execute('CREATE TABLE source (build TEXT, table_name TEXT, layout TEXT, readable INTEGER, unreadable INTEGER)')
        for table, (columns, rows, dropped) in decoded.items():
            connection.execute(f'CREATE TABLE "{table}" ({", ".join(f"{name} {kind}" for name, kind in columns)})')
            connection.executemany(f'INSERT INTO "{table}" VALUES ({",".join("?" for _ in columns)})', rows)
            connection.execute(f'CREATE UNIQUE INDEX "{table}_id" ON "{table}" (ID)')
            connection.execute('INSERT INTO source VALUES(?,?,?,?,?)', (build, table, f'{struct.unpack_from("<I", (db2 / f"{TABLES[table][0]}.db2").read_bytes(), 156)[0]:08X}', len(rows), dropped))
            print(f'{table}: {len(rows)} readable, {dropped} encrypted/unreadable')
        import_items(connection, items)
        import_item_specializations(connection)
        import_presentation(connection, items)
        connection.execute('CREATE TABLE challenge_dungeon (ID INTEGER PRIMARY KEY, map_id INTEGER, name TEXT, time_limit_ms INTEGER)')
        connection.executemany('INSERT INTO challenge_dungeon VALUES(?,?,?,?)', challenges)
    staging.replace(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db2', type=Path, required=True)
    parser.add_argument('--definitions', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--items', type=Path, required=True, help='Pinned Item, ItemSparse, ChrSpecialization CSV directory')
    parser.add_argument('--world', type=Path, required=True, help='Read-only realm world.db with authentic MapChallengeMode data')
    args = parser.parse_args()
    import_cache(args.db2, args.definitions, args.output, args.items, args.world)


if __name__ == '__main__':
    main()
