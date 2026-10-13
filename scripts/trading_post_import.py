#!/usr/bin/env python3
"""Prepare layout-qualified local Retail Trading Post SQL, never mutate world.db.

Use local casc-local to extract these FDIDs. --definitions supplies exact pinned
WoWDBDefs layouts. Apply the resulting SQL only to an explicitly owned copy.
"""
import argparse
import hashlib
import importlib.util
import json
import struct
from pathlib import Path
from db2_source import source_identity
TABLES = {
    'PerksVendorCategory': ('perks_vendor_category', 4281310, 0x00ECA515),
    'PerksVendorItem': ('perks_vendor_item', 4281311, 0x96A2B1EB),
    'PerksVendorItemUIInfo': ('perks_vendor_item_ui_info', 4281312, 0x75DC10F3),
    'PerksVendorItemXInterval': ('perks_vendor_item_x_interval', 4872513, 0xF901FDA4),
    'PerksVendorItemUIGroup': ('perks_vendor_item_ui_group', 5386256, 0xF67D8F83),
}
SPEC = importlib.util.spec_from_file_location('trading_post_decoder', Path(__file__).with_name('adventure_guide_import.py'))
DECODER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DECODER)
DECODER.TABLES.update({name: (fdid, layout) for name, (_, fdid, layout) in TABLES.items()})
read_table = DECODER.read_table


def decode_catalog(db2, definitions):
    sources = {}
    for name, (table, fdid, layout) in TABLES.items():
        binary = db2 / f'{fdid}.db2'
        definition = definitions / f'{name}.dbd'
        columns, rows, dropped = read_table(name, binary, definition)
        layout = struct.unpack_from('<I', binary.read_bytes(), 156)[0]
        sources[table] = {
            'name': name, 'fdid': fdid, 'layout': f'{layout:08X}',
            'db2_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
            'schema_sha256': hashlib.sha256(definition.read_bytes()).hexdigest(),
            'unreadable': dropped,
            'records': [dict(zip([field for field, _ in columns], row)) for row in rows],
        }
    return sources


def quote_sql(value):
    return "'" + value.replace("'", "''") + "'"


def prepare_sql(sources, build):
    sql = ['BEGIN TRANSACTION;', 'CREATE TABLE IF NOT EXISTS trading_post_source (name TEXT PRIMARY KEY,build TEXT NOT NULL,fdid INTEGER NOT NULL,layout TEXT NOT NULL,readable INTEGER NOT NULL,unreadable INTEGER NOT NULL,db2_sha256 TEXT NOT NULL,schema_sha256 TEXT NOT NULL);', 'DELETE FROM trading_post_source;']
    report = {'build': build, 'sources': {}, 'gaps': [
        'Monthly featured rotation: PerksVendorItemXInterval has no verified calendar epoch.',
        'Unknown vendor fields retained verbatim; no availability/discount meaning inferred.',
        'Seasonal activity threshold group/calendar binding unavailable; Osso uses sourced group 1 by explicit policy.',
        'Refund duration unavailable; disabled unless an explicit realm policy is supplied.',
    ]}
    for table, source in sources.items():
        rows = source['records']
        report['sources'][source['name']] = {key: value for key, value in source.items() if key != 'records'}
        report['sources'][source['name']]['readable'] = len(rows)
        sql.extend([f'DROP TABLE IF EXISTS {table};', f'CREATE TABLE {table} (ID INTEGER PRIMARY KEY,payload TEXT NOT NULL);'])
        for row in rows:
            sql.append(f'INSERT INTO {table} VALUES({row["ID"]},{quote_sql(json.dumps(row, ensure_ascii=False, separators=(",", ":")))});')
        values = [quote_sql(source['name']), quote_sql(build), str(source['fdid']), quote_sql(source['layout']), str(len(rows)), str(source['unreadable']), quote_sql(source['db2_sha256']), quote_sql(source['schema_sha256'])]
        sql.append('INSERT INTO trading_post_source VALUES(' + ','.join(values) + ');')
    sql.append('COMMIT;')
    return '\n'.join(sql) + '\n', report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('db2', 'definitions', 'output-sql', 'report'):
        parser.add_argument('--' + flag, type=Path, required=True)
    args = parser.parse_args()
    sql, report = prepare_sql(decode_catalog(args.db2, args.definitions), source_identity(args.db2)['build'])
    args.output_sql.parent.mkdir(parents=True, exist_ok=True)
    args.output_sql.write_text(sql)
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({name: row['readable'] for name, row in report['sources'].items()}))


if __name__ == '__main__':
    main()
